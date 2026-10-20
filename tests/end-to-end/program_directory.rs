//! Program checks refuse unsafe source directories before spawning (shell.md, 7.2).
use super::{program_check::setup, support};
use smith_real_world::Scratch;
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

fn directory(scratch: &Scratch, programs_root: bool) -> PathBuf {
    let programs = scratch.path().join("state/smith/programs");
    if programs_root { programs } else { programs.join("work") }
}

fn refuses(scratch: &Scratch, path: &Path, reason: &str) {
    let ran = support::smith(scratch, &["check", "--json"], b"");
    let report: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("check JSON");
    assert!(
        !scratch.path().join("state/smith/programs/work/fake-claude.observations").exists(),
        "the fake must receive no auth status for a refused directory"
    );
    assert_eq!(ran.code, 3, "an unsafe source refuses the needed account");
    let account = report.pointer("/credentials/work").expect("work account");
    assert_eq!(
        account.get("state").and_then(serde_json::Value::as_str),
        Some("program_failed"),
        "an unsafe directory cannot be usable"
    );
    let detail = format!("{}: {reason}", path.display());
    assert_eq!(
        account.get("detail").and_then(serde_json::Value::as_str),
        Some(detail.as_str()),
        "the refusal names the path and its problem"
    );
}

fn linked(programs_root: bool) {
    let (scratch, _) = setup(Some("claude.ai"), "Truthful");
    let path = directory(&scratch, programs_root);
    let target = scratch.path().join("signed-in-directory");
    std::fs::rename(&path, &target).expect("move the scratch login");
    symlink(&target, &path).expect("link the scratch login");
    refuses(&scratch, &path, "symbolic link");
}

fn public(programs_root: bool) {
    let (scratch, _) = setup(Some("claude.ai"), "Truthful");
    let path = directory(&scratch, programs_root);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("public source mode");
    refuses(&scratch, &path, "nonprivate mode bits 0o55");
}

#[test]
fn check_refuses_a_linked_program_account_directory() {
    linked(false);
}

#[test]
fn check_refuses_a_public_program_account_directory() {
    public(false);
}

#[test]
fn check_refuses_a_linked_programs_directory() {
    linked(true);
}

#[test]
fn check_refuses_a_public_programs_directory() {
    public(true);
}
