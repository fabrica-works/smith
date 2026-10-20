use super::*;
use crate::credentials::tests::Files;
use std::os::unix::fs::PermissionsExt;

pub(super) fn configured(files: &Files, source_name: &str, program: bool) -> super::super::Settings {
    let certificate =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/protocol-llm/fixtures/root.der");
    let program = if program { "program='claude_code'" } else { "" };
    settings(&[source(Layer::User, &format!(
        "model='codex/gpt-6-luna'\ncredentials='{}'\n[programs]\nclaude_code='/bin/true'\n[accounts.codex]\nsource='{source_name}'\n[endpoints.codex]\naddress='127.0.0.1:443'\ntrust.system=false\ntrust.der=['{}']\n{program}\n",
        files.0.display(), certificate.display()
    ))]).expect("settings")
}

fn resolved(settings: &super::super::Settings) -> resolve::Resolved {
    resolve::resolve_startup(settings).unwrap_or_else(|why| panic!("{}", why.line())).0
}

#[test]
fn a_none_accounts_models_name_no_account_in_the_charter() {
    let files = Files::new();
    let resolved = resolved(&configured(&files, "none", false));
    assert!(resolved.local.models.iter().all(|model| model.account.is_none()), "no charter account");
    assert!(resolved.local.accounts.is_empty(), "no configured grant account");
    assert_eq!(resolved.accounts[0].number, 1_474_186_484, "none retains its number");
}

#[test]
fn a_key_accounts_models_name_its_number() {
    let files = Files::new();
    let resolved = resolved(&configured(&files, "key", false));
    assert!(resolved.local.models.iter().all(|model| model.account == Some(1_474_186_484)), "stable charter account");
    assert_eq!(resolved.local.accounts.as_ref(), [1_474_186_484], "configured grant account");
}

#[test]
fn the_generated_document_names_the_program_directory() {
    let files = Files::new();
    let (_, bytes) =
        resolve::resolve_startup(&configured(&files, "program", true)).unwrap_or_else(|why| panic!("{}", why.line()));
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("agent document");
    assert_eq!(document["programs"]["claude_code"]["path"], "/bin/true", "configured executable");
    assert!(
        document["programs"]["claude_code"]["directory"] == files.0.join("programs").to_str().expect("UTF-8"),
        "programs directory"
    );
    assert_eq!(document["endpoints"][0]["program"], "claude_code", "endpoint program");
    let text = String::from_utf8(bytes.into_vec()).expect("UTF-8 document");
    for secret in ["\"account\"", "key-for-work"] {
        assert!(!text.contains(secret), "no credential in configuration");
    }
    assert!(!files.0.join("programs").exists(), "resolution creates no program directory");
}

#[test]
fn a_document_without_a_program_endpoint_holds_no_programs() {
    let files = Files::new();
    let (_, bytes) =
        resolve::resolve_startup(&configured(&files, "key", false)).unwrap_or_else(|why| panic!("{}", why.line()));
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("agent document");
    assert!(document.get("programs").is_none(), "unused program settings omitted");
}

#[test]
fn locating_a_program_searches_the_given_path() {
    let files = Files::new();
    let executable = files.0.join("claude");
    std::fs::write(&executable, b"#!/bin/sh\nexit 0\n").expect("scratch executable");
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).expect("executable mode");
    assert_eq!(resolve::programs::locate(None, Some(files.0.as_os_str())), Ok(executable), "given PATH");
}

#[test]
fn a_missing_or_non_executable_program_is_refused() {
    let files = Files::new();
    assert!(resolve::programs::locate(None, Some(files.0.as_os_str())).is_err(), "absent executable");
    std::fs::write(files.0.join("claude"), b"script").expect("scratch non-executable");
    std::fs::set_permissions(files.0.join("claude"), std::fs::Permissions::from_mode(0o600)).expect("file mode");
    assert!(resolve::programs::locate(None, Some(files.0.as_os_str())).is_err(), "non-executable");
    assert!(resolve::programs::locate(None, None).is_err(), "absent PATH");
}

#[test]
fn a_configured_program_path_must_be_absolute() {
    assert!(
        resolve::programs::locate(Some("claude"), Some(std::ffi::OsStr::new("/bin"))).is_err(),
        "absolute path required"
    );
}
