//! Program source startup and check through the shipped binary (shell.md, 4.4, 5.3).
use super::support;
use smith_real_world::Scratch;
use std::os::unix::fs::DirBuilderExt;

pub(super) fn setup(method: Option<&str>, status: &str) -> (Scratch, std::path::PathBuf) {
    let scratch = Scratch::new();
    let executable = std::env::current_exe()
        .expect("test executable")
        .parent()
        .expect("deps")
        .parent()
        .expect("target")
        .join("fake-claude");
    let settings = format!(
        "model='anthropic/claude-haiku-5-5'\nsub_agents=false\n[programs]\nclaude_code='{}'\n[accounts.work]\nprovider='anthropic'\nsource='program'\n[endpoints.anthropic]\npreset='anthropic'\naccount='work'\nprogram='claude_code'\n",
        executable.display()
    );
    let _file = support::user_layer(&scratch, settings.as_bytes());
    let configuration = scratch.path().join("state/smith/programs/work");
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&configuration).expect("private configuration");
    if let Some(method) = method {
        std::fs::write(configuration.join("fake-claude.login"), method).expect("fake method");
    }
    std::fs::write(
        configuration.join("fake-claude.story"),
        serde_json::to_vec(&serde_json::json!({
            "status":status,"login":{"Immediate":"claude.ai"},"logout":"Succeeds"
        }))
        .expect("story JSON"),
    )
    .expect("fake story");
    (scratch, executable)
}

fn observations(scratch: &Scratch, account: &str) -> Vec<serde_json::Value> {
    let bytes =
        std::fs::read(scratch.path().join("state/smith/programs").join(account).join("fake-claude.observations"))
            .expect("observations");
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("observation JSON"))
        .collect()
}

fn one_status(scratch: &Scratch, account: &str) {
    let arguments: Vec<_> = observations(scratch, account)
        .into_iter()
        .map(|record| record.get("arguments").expect("invocation arguments").clone())
        .collect();
    assert_eq!(arguments, vec![serde_json::json!(["auth", "status"])], "the reported state matches the contract");
}

#[test]
fn check_reports_a_signed_in_program_account_as_usable() {
    let (scratch, executable) = setup(Some("claude.ai"), "Truthful");
    let ran = support::smith(&scratch, &["check", "--json"], b"");
    assert_eq!(ran.code, 0, "{}", String::from_utf8_lossy(&ran.stderr));
    let report: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("check JSON");
    assert_eq!(report["version"], 2, "the reported state matches the contract");
    assert_eq!(
        report["credentials"]["work"],
        serde_json::json!({
            "required":true,"provider":"anthropic","source":"program","number":0x00e1_3ed7,
            "kept":{"kind":"directory","at":scratch.path().join("state/smith/programs/work")},
            "state":"usable","until":null
        }),
        "the reported state matches the contract"
    );
    assert_eq!(
        report["programs"]["claude_code"],
        executable.to_str().expect("path UTF-8"),
        "the reported state matches the contract"
    );
    one_status(&scratch, "work");
}

#[test]
fn check_exits_3_for_a_program_account_not_signed_in() {
    let (scratch, _) = setup(None, "Truthful");
    let ran = support::smith(&scratch, &["check"], b"");
    assert_eq!(ran.code, 3, "{}", String::from_utf8_lossy(&ran.stderr));
    assert!(
        String::from_utf8_lossy(&ran.stdout)
            .lines()
            .any(|line| line == "account work: Claude Code is not signed in: run `smith login work`"),
        "the reported state matches the contract"
    );
    one_status(&scratch, "work");
}

#[test]
fn check_says_a_program_signed_in_otherwise() {
    let (scratch, _) = setup(Some("api"), "Truthful");
    // The exec projection uses the same catalog check.
    let ran = support::smith(&scratch, &["check", "--json", "exec"], b"");
    assert_eq!(ran.code, 3, "{}", String::from_utf8_lossy(&ran.stderr));
    let report: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("check JSON");
    assert_eq!(
        report["credentials"]["work"]["state"], "program_signed_in_otherwise",
        "the reported state matches the contract"
    );
    assert_eq!(report["credentials"]["work"]["method"], "api", "the reported state matches the contract");
    one_status(&scratch, "work");
}

#[test]
fn a_failed_program_check_is_exit_3() {
    let (scratch, _) = setup(Some("claude.ai"), "Unreadable");
    let ran = support::smith(&scratch, &["check", "--json"], b"");
    assert_eq!(ran.code, 3, "{}", String::from_utf8_lossy(&ran.stderr));
    let report: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("check JSON");
    assert_eq!(report["credentials"]["work"]["state"], "program_failed", "the reported state matches the contract");
    one_status(&scratch, "work");
}

#[test]
fn exec_refuses_a_program_account_not_signed_in_before_its_run() {
    let (scratch, _) = setup(None, "Truthful");
    let ran = support::smith(&scratch, &["exec", "--json", "Do the work"], b"");
    assert_eq!(ran.code, 3, "{}", String::from_utf8_lossy(&ran.stderr));
    assert_eq!(
        String::from_utf8_lossy(&ran.stderr).lines().last(),
        Some("smith: account work: Claude Code is not signed in: run `smith login work`"),
        "the reported state matches the contract"
    );
    assert!(ran.stdout.is_empty(), "the reported state matches the contract");
    one_status(&scratch, "work");
}

#[test]
fn check_inspects_an_unused_program_account_without_refusing_startup() {
    let (scratch, _) = setup(Some("claude.ai"), "Truthful");
    let path = scratch.path().join("config/smith/config.toml");
    let mut settings = std::fs::read_to_string(&path).expect("scratch settings");
    settings.push_str("\n[accounts.spare]\nprovider='anthropic'\nsource='program'\n");
    let _file = support::user_layer(&scratch, settings.as_bytes());
    let configuration = scratch.path().join("state/smith/programs/spare");
    std::fs::DirBuilder::new().mode(0o700).create(&configuration).expect("private unused source");
    let ran = support::smith(&scratch, &["check", "--json"], b"");
    assert_eq!(ran.code, 0, "only needed credentials decide the exit: {}", String::from_utf8_lossy(&ran.stderr));
    let report: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("check JSON");
    // printf spare | sha256sum => cf2d9706..., first four bytes big endian.
    assert_eq!(
        report["credentials"]["spare"],
        serde_json::json!({
            "required":false,"provider":"anthropic","source":"program","number":0xcf2d_9706_u32,
            "kept":{"kind":"directory","at":configuration},
            "state":"program_not_signed_in","program":"claude_code"
        }),
        "the unused account retains its metadata and checked state"
    );
    let warning = report["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .find(|warning| warning.get("account").is_some_and(|account| account == "spare"))
        .expect("unused account warning");
    assert_eq!(
        warning,
        &serde_json::json!({
            "kind":"credential","account":"spare","credential":{"state":"program_not_signed_in","program":"claude_code"}
        }),
        "the warning identifies the unusable unused account"
    );
    one_status(&scratch, "work");
    one_status(&scratch, "spare");
}
