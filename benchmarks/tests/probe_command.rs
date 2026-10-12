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
    let settings = directory.join("records");
    std::fs::write(&settings, common::events::bytes(&common::events::balanced())).expect("wire recording");
    let script = format!(
        "#!/usr/bin/python3\nimport json,sys\ns=json.load(open(sys.argv[2]))\np=s['agent']['trace']['path']\nopen(p,'wb').write(open({:?},'rb').read())\n",
        settings.to_str().expect("path")
    );
    let program = build.directory.join("smith");
    std::fs::write(&program, script.as_bytes()).expect("replay executable");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).expect("execute");
    let digest = format!("{:x}", Sha256::digest(script.as_bytes()));
    build.arm.sha256.clone_from(&digest);
    build.arm.binaries[0].sha256.clone_from(&digest);
    std::fs::write(build.directory.join("arm.json"), serde_json::to_vec(&build.arm).expect("manifest"))
        .expect("manifest");
    let arm = directory.join(smith_bench::arm::arm_name(&build.arm.commit, &digest).expect("arm name"));
    std::fs::rename(&build.directory, &arm).expect("rename frozen arm");
    arm
}
