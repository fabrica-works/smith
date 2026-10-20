//! Offline command controls use synthetic credentials and an unissued replay arm.

pub mod common;

use smith_bench::arm::ArmRef;
use smith_bench::probe::{Order, Source};
use smith_bench::{Agent, Judgment, Measure};
use std::os::unix::fs::DirBuilderExt;
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
    std::fs::DirBuilder::new().mode(0o700).create(&directory).expect("private fixture root");
    std::fs::create_dir_all(directory.join("state/smith-bench/logins/smith/tokens")).expect("synthetic home");
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
        assert!(request.configuration.expect("pin").ends_with("smith/everything.pin.toml"));
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
    let login = directory.join("state/smith-bench/logins/smith");
    std::fs::create_dir_all(login.join("config/smith")).expect("synthetic configuration");
    std::fs::write(login.join("config/smith/config.toml"), []).expect("setup marker");
    std::fs::write(login.join("host-id"), []).expect("host marker");
    std::fs::write(login.join("tokens/chatgpt"), []).expect("record marker");
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
    assert!(login.join("tokens/chatgpt").is_file());
    std::fs::remove_dir_all(directory).expect("cleanup");
}

fn replay_arm(directory: &Path) -> PathBuf {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    std::fs::DirBuilder::new().mode(0o700).create(directory).expect("private replay root");
    std::fs::create_dir_all(directory.join("state/smith-bench/logins/smith/tokens")).expect("synthetic home");
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

fn calibration_files(name: &str) -> (PathBuf, PathBuf) {
    let directory = root(name);
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(&directory).expect("calibration directory");

    let file = directory.join("task.toml");
    std::fs::copy(repository.join("tasks/probes/smoke-tools/task.toml"), &file).expect("task");
    (directory, file)
}

#[test]
fn a_calibration_block_names_its_summary_and_failing_control() {
    let (directory, file) = calibration_files("block");
    let task = smith_bench::read_task(&file).expect("task");
    let block = common::calibration(&task);
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
    let block = common::calibration(&task);
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
    deviation.outcome.deciding.push("answer".into());
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
    results[0].outcome.deciding.push("answer".into());
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
fn an_unavailable_advisory_check_leaves_a_model_deviation() {
    let mut result = recorded_passes().remove(0);
    result.outcome.deciding = vec!["file-content:first.txt".into()];
    result.outcome.checks = vec![smith_bench::CheckVerdict {
        check: "file-content:first.txt".into(),
        passed: Measure::Observed { value: false },
        evidence: vec!["No such file or directory (os error 2)".into()],
    }];
    for check in ["teardown", "headroom/request-bytes", "headroom/first-byte", "headroom/largest-gap"] {
        result.outcome.checks.push(smith_bench::CheckVerdict {
            check: check.into(),
            passed: Measure::unavailable("not yet: passive observation"),
            evidence: vec![],
        });
    }
    assert!(smith_bench::probe::model_deviation(&result));
}

#[test]
fn a_failed_deciding_contract_check_is_no_model_deviation() {
    let mut result = recorded_passes().remove(0);
    result.outcome.deciding = vec!["file-content:first.txt".into(), "run-completed".into()];
    result.outcome.checks = ["file-content:first.txt", "run-completed"]
        .into_iter()
        .map(|check| smith_bench::CheckVerdict {
            check: check.into(),
            passed: Measure::Observed { value: false },
            evidence: vec![],
        })
        .collect();
    assert!(!smith_bench::probe::model_deviation(&result));
}

#[test]
fn a_block_whose_prompt_digest_differs_gates_nothing() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut task = smith_bench::read_task(&repository.join("tasks/probes/smoke-tools/task.toml")).expect("task");
    task.calibration = Some(common::calibration(&task));
    assert!(task.calibrated().expect("unchanged inputs"));
    task.prompt.push_str("Read another file.\n");
    assert!(!task.calibrated().expect("changed prompt"));
}

#[test]
fn a_failed_call_count_with_smiths_checks_passing_is_a_model_deviation() {
    let mut result = recorded_passes().remove(0);
    result.outcome.deciding = vec!["call-count".into(), "compaction-carried".into(), "tool-call".into()];
    result.outcome.checks = vec![
        smith_bench::CheckVerdict {
            check: "call-count".into(),
            passed: Measure::Observed { value: false },
            evidence: vec![],
        },
        smith_bench::CheckVerdict {
            check: "compaction-carried".into(),
            passed: Measure::Observed { value: true },
            evidence: vec![],
        },
        smith_bench::CheckVerdict {
            check: "tool-call".into(),
            passed: Measure::Observed { value: true },
            evidence: vec![],
        },
    ];
    let counted = smith_bench::probe::count(&[result], 10, 12);
    assert_eq!((counted.passed, counted.failed, counted.deviations, counted.unjudged), (0, 0, 1, 0));
}

#[test]
fn a_model_failure_beside_an_unavailable_contract_check_is_unjudged() {
    let mut result = recorded_passes().remove(0);
    result.outcome.deciding = vec!["answer".into(), "compaction-carried".into()];
    result.outcome.checks = vec![
        smith_bench::CheckVerdict {
            check: "answer".into(),
            passed: Measure::Observed { value: false },
            evidence: vec![],
        },
        smith_bench::CheckVerdict {
            check: "compaction-carried".into(),
            passed: Measure::unavailable("prompt was not captured"),
            evidence: vec![],
        },
    ];
    let counted = smith_bench::probe::count(&[result], 10, 12);
    assert_eq!((counted.passed, counted.failed, counted.deviations, counted.unjudged), (0, 0, 0, 1));
}

#[test]
fn an_early_finish_before_the_threshold_is_a_model_deviation() {
    let mut result = recorded_passes().remove(0);
    result.outcome.deciding =
        vec!["answer".into(), "call-count".into(), "conversation-opened".into(), "compaction-choice".into()];
    result.outcome.checks = ["answer", "call-count"]
        .into_iter()
        .map(|check| smith_bench::CheckVerdict {
            check: check.into(),
            passed: Measure::Observed { value: false },
            evidence: vec![],
        })
        .collect();
    for check in ["conversation-opened", "compaction-choice"] {
        result.outcome.checks.push(smith_bench::CheckVerdict {
            check: check.into(),
            passed: Measure::unavailable("not exercised: no request passed the compaction line"),
            evidence: vec![],
        });
    }
    let counted = smith_bench::probe::count(&[result], 10, 12);
    assert_eq!((counted.passed, counted.failed, counted.deviations, counted.unjudged), (0, 0, 1, 0));
}

#[test]
fn the_program_order_keeps_its_own_arm_identity() {
    let name = "the_program_order_keeps_its_own_arm_identity";
    let Some(directory) = isolated(name) else {
        return;
    };
    std::fs::create_dir_all(&directory).expect("private fixture root");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repository");
    let arm = ArmRef::Built(common::unexecuted_arm(&directory, repository));
    let order =
        Order { probe: "probes/smoke-tools".into(), agent: Agent::ClaudeCode, attempts: 1, source: Source::Head };
    let planned = smith_bench::probe::plan(repository, &order, &arm).expect("program plan");
    let request = &planned.requests[0];
    assert_eq!(request.identity.agent, Agent::Smith);
    assert_eq!(request.identity.provider, smith_bench::Provider::Anthropic);
    assert_eq!(request.identity.model.requested_model, "claude-haiku-5-5");
    assert!(request.identity.arm.starts_with("smith-claude-code-"));
    assert!(request.configuration.as_ref().expect("program pin").ends_with("smith/smith-claude-code.pin.toml"));
}

#[test]
fn a_missing_smoke_login_refuses_before_calibration() {
    let directory = root("missing-smoke-login");
    std::fs::DirBuilder::new().mode(0o700).create(&directory).expect("private root");
    let output = Command::new(env!("CARGO_BIN_EXE_smith-bench"))
        .args(["suite", "smoke"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &directory)
        .env("XDG_CONFIG_HOME", directory.join("config"))
        .env("XDG_STATE_HOME", directory.join("state"))
        .output()
        .expect("offline smoke refusal");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).expect("diagnostic").trim(),
        format!(
            "no smith login for the harness: run `XDG_CONFIG_HOME={} smith login chatgpt`",
            directory.join("state/smith-bench/logins/smith/config").display()
        )
    );
    std::fs::remove_dir_all(directory).expect("synthetic cleanup");
}
