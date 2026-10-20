//! The same fake as an executable (protocol/programs.md, section 8;
//! testing.md, sections 2.2, 2.3 and 4). All process state stays in scratch.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

use skein_scratch::Scratch;
use smith_fake_claude::{CONFIG_DIR, LOGIN_FILE, OBSERVATIONS_FILE, observations};

fn invoke(home: &Path, config: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fake-claude"));
    command.env_clear().env("HOME", home).current_dir(home).args(args);
    if let Some(config) = config {
        command.env(CONFIG_DIR, config);
    }
    command.output().expect("fake binary runs")
}

#[test]
fn the_binary_signs_in_and_reports_its_status() {
    let scratch = Scratch::new("fake-claude-login");
    let config = scratch.path().join("config");
    fs::create_dir(&config).expect("configuration directory");
    fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).expect("private configuration");
    let login = invoke(scratch.path(), Some(&config), &["auth", "login"]);
    assert_eq!(login.status.code(), Some(0));
    assert_eq!(login.stdout, b"Signed in.\n");
    assert!(login.stderr.is_empty());
    assert_eq!(fs::read(config.join(LOGIN_FILE)).expect("login"), b"claude.ai");
    let status = invoke(scratch.path(), Some(&config), &["auth", "status"]);
    assert_eq!(status.status.code(), Some(0));
    assert!(status.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&status.stdout).expect("status JSON"),
        serde_json::json!({"loggedIn": true, "authMethod": "claude.ai", "apiProvider": "firstParty"})
    );
    let records = observations(&fs::read(config.join(OBSERVATIONS_FILE)).expect("observations"));
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].arguments, ["auth", "login"]);
    assert_eq!(records[1].arguments, ["auth", "status"]);
    for record in records {
        assert_eq!(record.environment, ["CLAUDE_CONFIG_DIR", "HOME"]);
    }
    assert!(!scratch.path().join(".claude").exists());
}

#[test]
fn the_binary_refuses_without_a_configuration_directory() {
    let scratch = Scratch::new("fake-claude-no-config");
    let output = invoke(scratch.path(), None, &["auth", "login"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"fake claude: CLAUDE_CONFIG_DIR is not set\n");
    assert_eq!(fs::read_dir(scratch.path()).expect("scratch").count(), 0);
}

#[test]
fn the_binary_without_its_directory_is_not_signed_in() {
    let scratch = Scratch::new("fake-claude-absent-status");
    let config = scratch.path().join("absent");
    let output = invoke(scratch.path(), Some(&config), &["auth", "status"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("status JSON"),
        serde_json::json!({"loggedIn": false, "authMethod": "none", "apiProvider": "firstParty"})
    );
    assert!(!config.exists());
}

#[test]
fn the_binary_login_refuses_without_its_directory() {
    let scratch = Scratch::new("fake-claude-absent-login");
    let config = scratch.path().join("absent");
    let output = invoke(scratch.path(), Some(&config), &["auth", "login"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!output.stderr.is_empty());
    assert!(output.stdout.is_empty());
    assert!(!config.exists());
}

#[test]
fn the_binary_logout_accepts_an_absent_directory() {
    let scratch = Scratch::new("fake-claude-absent-logout");
    let config = scratch.path().join("absent");
    let output = invoke(scratch.path(), Some(&config), &["auth", "logout"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"Signed out.\n");
    assert!(!config.exists());
}
