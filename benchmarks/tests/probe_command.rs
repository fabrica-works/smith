//! Offline command controls use synthetic credentials and an unissued replay arm.

pub mod common;

use base64::Engine;
use smith_bench::arm::ArmRef;
use smith_bench::probe::{Order, Source};
use smith_bench::{Agent, Judgment, Measure};
use std::path::{Path, PathBuf};
use std::process::Command;

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("smith-probe-command-{name}-{}", std::process::id()))
}

fn isolated(name: &str) -> Option<PathBuf> {
    if std::env::var_os("SMITH_PROBE_OFFLINE_CHILD").is_some() {
        return Some(root(name));
    }
    let directory = root(name);
    std::fs::create_dir_all(directory.join(".codex")).expect("synthetic home");
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", name, "--nocapture"])
        .env("SMITH_PROBE_OFFLINE_CHILD", "1")
        .env("HOME", &directory)
        .env("XDG_CONFIG_HOME", directory.join("config"))
        .env("XDG_STATE_HOME", directory.join("state"))
        .output()
        .expect("isolated test process");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(directory).expect("remove private fixture");
    None
}

#[test]
fn a_probe_order_writes_one_request_per_attempt_with_its_pin_and_tier() {
    let name = "a_probe_order_writes_one_request_per_attempt_with_its_pin_and_tier";
    let Some(directory) = isolated(name) else {
        return;
    };
    std::fs::create_dir_all(&directory).expect("fixture");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repository");
    let arm = ArmRef::Built(common::unexecuted_arm(&directory, repository));
    let order = Order { probe: "probes/smoke-tools".into(), agent: Agent::Smith, attempts: 2, source: Source::Head };
    let planned = smith_bench::probe::plan(repository, &order, &arm).expect("prepare");
    assert_eq!(planned.requests.len(), 2);
    for request in planned.requests {
        assert_eq!(request.identity.model.requested_model, "gpt-6-luna");
        assert_eq!(request.identity.model.requested_effort, "low");
        assert!(request.configuration.expect("pin").ends_with("smith/smoke.pin.toml"));
        assert!(planned.directory.join(format!("{}.json", request.identity.attempt)).exists());
    }
    assert_eq!(smith_bench::read_suite(&planned.suite).expect("suite").rerun_failed, 0);
}

#[test]
fn a_codex_order_takes_the_standard_codex_pin() {
    let name = "a_codex_order_takes_the_standard_codex_pin";
    let Some(directory) = isolated(name) else {
        return;
    };
    std::fs::create_dir_all(&directory).expect("fixture");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repository");
    let order = Order { probe: "probes/smoke-tools".into(), agent: Agent::Codex, attempts: 1, source: Source::Head };
    let planned =
        smith_bench::probe::plan(repository, &order, &ArmRef::Installed { program: "/bin/true".into() }).expect("plan");
    let request = &planned.requests[0];
    assert_eq!(request.identity.agent, Agent::Codex);
    assert!(request.configuration.as_ref().expect("pin").ends_with("codex/standard.pin.toml"));
}

#[test]
fn a_line_prints_an_unavailable_spend_as_a_dash() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut result = smith_bench::read_result(&root.join("tests/recorded/results/attempt.json")).expect("recorded");
    result.spend.usd = Measure::unavailable("no prices for this model");
    let text = smith_bench::probe::line(&result).to_string();
    assert!(text.starts_with(&format!("{}/{} ", result.identity.run, result.identity.attempt)), "{text}");
    assert!(text.ends_with(" spend=-"), "{text}");
}

#[test]
fn calibration_stops_at_ten_passes_or_once_ten_are_out_of_reach() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let result = smith_bench::read_result(&root.join("tests/recorded/results/attempt.json")).expect("recorded");
    assert_eq!(smith_bench::judge(&result), Judgment::Pass);
    let results = vec![result; 10];
    let count = smith_bench::probe::count(&results, 10, 12);
    assert!(count.calibrated());
    let mut results = results[..3].to_vec();
    for result in &mut results {
        result.outcome.checks[0].passed = Measure::Observed { value: false };
    }
    let count = smith_bench::probe::count(&results, 10, 12);
    assert!(!count.calibrated());
    assert!(!count.reachable());
}

#[test]
fn a_replay_arm_runs_through_the_probe_command_and_prints_one_line() {
    let directory = root("replay");
    let arm = replay_arm(&directory);
    let mut auth: serde_json::Value = serde_json::from_slice(&common::unissued_auth()).expect("unissued login");
    let expiry = skein_shell::Clock::new().now().wall.as_nanos() / 1_000_000_000 + 3600;
    let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("{{\"exp\":{expiry}}}"));
    auth["tokens"]["access_token"] = format!("header.{claims}.signature").into();
    let auth = serde_json::to_vec(&auth).expect("synthetic login");
    std::fs::write(directory.join(".codex/auth.json"), &auth).expect("login");
    let output = Command::new(env!("CARGO_BIN_EXE_smith-bench"))
        .args(["probe", "probes/smoke-tools", "--arm"])
        .arg(&arm)
        .env("HOME", &directory)
        .env("XDG_CONFIG_HOME", directory.join("config"))
        .env("XDG_STATE_HOME", directory.join("state"))
        .output()
        .expect("probe process");
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        text.lines().filter(|line| line.contains("/01 end=completed ")).count(),
        1,
        "{text} {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let verdict = match text.lines().last().expect("summary").split("verdict=").last().expect("verdict") {
        "pass" => 0,
        "fail" => 1,
        "incomplete" => 2,
        other => panic!("unknown verdict {other}"),
    };
    assert_eq!(output.status.code(), Some(verdict));
    assert_eq!(std::fs::read(directory.join(".codex/auth.json")).expect("source login"), auth);
    std::fs::remove_dir_all(directory).expect("cleanup");
}

fn replay_arm(directory: &Path) -> PathBuf {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(directory.join(".codex")).expect("synthetic home");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repository");
    let mut build = common::unexecuted_arm(directory, repository);
    let settings = build.directory.join("recording.jsonl");
    std::fs::write(&settings, common::events::bytes(&common::events::balanced())).expect("wire recording");
    let executable = std::fs::read(env!("CARGO_BIN_EXE_test-peer")).expect("Rust replay peer");
    let program = build.directory.join("smith");
    std::fs::write(&program, &executable).expect("replay executable");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).expect("execute");
    let digest = format!("{:x}", Sha256::digest(&executable));
    build.arm.sha256.clone_from(&digest);
    build.arm.binaries[0].sha256.clone_from(&digest);
    std::fs::write(build.directory.join("arm.json"), serde_json::to_vec(&build.arm).expect("manifest"))
        .expect("manifest");
    let arm = directory.join(smith_bench::arm::arm_name(&build.arm.commit, &digest).expect("arm name"));
    std::fs::rename(&build.directory, &arm).expect("rename frozen arm");
    arm
}

fn recorded_passes() -> Vec<smith_bench::AttemptResult> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let result = smith_bench::read_result(&root.join("tests/recorded/results/attempt.json")).expect("recorded");
    (1..=10)
        .map(|index| {
            let mut result = result.clone();
            result.identity.attempt = format!("{index:02}");
            result
        })
        .collect()
}

fn block(task: &smith_bench::Task) -> smith_bench::Calibration {
    use sha2::{Digest, Sha256};
    smith_bench::Calibration {
        binary: "a".repeat(40),
        binary_sha256: "b".repeat(64),
        date: "2026-10-14".into(),
        summary: "summaries/calibration.json".into(),
        attempts: 10,
        passes: (1..=10).map(|index| format!("recorded-run/{index:02}")).collect(),
        deviations: vec![],
        control: "c".repeat(40),
        failed_before: vec!["control-run/01".into()],
        prompt_sha256: format!("{:x}", Sha256::digest(task.prompt.as_bytes())),
        seed_sha256: task.seed_sha256.clone(),
        checks_sha256: task.checks_sha256().expect("checks digest"),
    }
}

fn calibration_files(name: &str) -> (PathBuf, PathBuf) {
    let directory = root(name);
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(directory.join("benchmarks/agents")).expect("prices directory");
    std::fs::copy(repository.join("agents/prices.toml"), directory.join("benchmarks/agents/prices.toml"))
        .expect("recorded prices");
    let file = directory.join("task.toml");
    std::fs::copy(repository.join("tasks/probes/smoke-tools/task.toml"), &file).expect("task");
    (directory, file)
}

#[test]
fn a_calibration_block_names_its_summary_and_failing_control() {
    let (directory, file) = calibration_files("block");
    let task = smith_bench::read_task(&file).expect("task");
    let block = block(&task);
    smith_bench::probe::record_calibration(&directory, &file, "probes/smoke-tools", &block, &recorded_passes(), 7)
        .expect("record calibration");
    let task = smith_bench::read_task(&file).expect("persisted task");
    assert_eq!(task.calibration, Some(block.clone()));
    assert_eq!(block.failed_before, ["control-run/01"]);
    assert_eq!(block.control, "c".repeat(40));
    let summary = smith_bench::read_summary(&directory.join("benchmarks").join(&block.summary)).expect("summary");
    assert_eq!(summary.groups[0].attempts.len(), 10);
    assert!(task.calibrated().expect("current inputs"));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn a_variant_carries_its_own_calibration() {
    let (directory, file) = calibration_files("variant");
    let task = smith_bench::read_task(&file).expect("task").with_variant("child").expect("child");
    let block = block(&task);
    smith_bench::probe::record_calibration(
        &directory,
        &file,
        "probes/smoke-tools/child",
        &block,
        &recorded_passes(),
        7,
    )
    .expect("variant calibration");
    let task = smith_bench::read_task(&file).expect("persisted task");
    assert_eq!(task.calibration, None);
    let variant = task.with_variant("child").expect("child");
    assert_eq!(variant.calibration, Some(block));
    assert!(variant.calibrated().expect("current inputs"));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn ten_passes_within_twelve_attempts_is_calibrated() {
    let mut results = recorded_passes();
    let mut deviation = results[0].clone();
    deviation.outcome.checks.push(smith_bench::CheckVerdict {
        check: "answer".into(),
        passed: Measure::Observed { value: false },
        evidence: vec!["wrong sum".into()],
    });
    results.extend([deviation.clone(), deviation]);
    let counted = smith_bench::probe::count(&results, 10, 12);
    assert_eq!((counted.passed, counted.failed, counted.deviations, counted.unjudged), (10, 0, 2, 0));
    assert!(counted.calibrated());
}

#[test]
fn a_model_deviation_is_counted_apart_from_smiths_failures() {
    let mut results = recorded_passes();
    results.truncate(2);
    results[0].outcome.checks.push(smith_bench::CheckVerdict {
        check: "answer".into(),
        passed: Measure::Observed { value: false },
        evidence: vec!["wrong sum".into()],
    });
    results[1].outcome.end =
        smith_bench::End::Failed { reason: smith_bench::FailureReason::Model, detail: "malformed compaction".into() };
    let counted = smith_bench::probe::count(&results, 10, 12);
    assert_eq!((counted.passed, counted.failed, counted.deviations, counted.unjudged), (0, 1, 1, 0));
    assert!(!counted.calibrated());
}

#[test]
fn a_block_whose_prompt_digest_differs_gates_nothing() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut task = smith_bench::read_task(&repository.join("tasks/probes/smoke-tools/task.toml")).expect("task");
    task.calibration = Some(block(&task));
    assert!(task.calibrated().expect("unchanged inputs"));
    task.prompt.push_str("Read another file.\n");
    assert!(!task.calibrated().expect("changed prompt"));
}
