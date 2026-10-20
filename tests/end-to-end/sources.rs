//! Configuration refusals through the shipped login command (shell.md, 5.3).
use super::{Scratch, support};

#[test]
fn login_refuses_the_same_invalid_declarations_as_startup() {
    for (text, key) in [
        ("credentials='relative'\n[accounts.work]\nsource='key'\n", "credentials"),
        ("[accounts.acct-78235]\nsource='none'\n[accounts.acct-124172]\nsource='none'\n", "accounts.acct-78235"),
        ("[accounts.'\"']\nsource='none'\n[accounts.work]\nsource='none'\n", "account name"),
        ("[accounts.work]\nsource='program'\n[endpoints.extra]\naccount='work'\n", "accounts.work.source"),
    ] {
        let scratch = Scratch::new();
        let _file = support::user_layer(&scratch, text.as_bytes());
        let ran = support::run_product(&scratch, &["login", "work"], b"", None);
        assert_eq!(ran.code, 2, "configuration refusal");
        assert!(String::from_utf8_lossy(&ran.stderr).contains(key), "refusal names the broken declaration");
    }
}

#[test]
fn login_checks_an_account_without_selecting_a_main_model() {
    let scratch = Scratch::new();
    let _file = support::user_layer(
        &scratch,
        b"[accounts.work]\nsource='none'\n[endpoints.custom]\npreset='codex'\naccount='work'\n",
    );
    let ran = support::run_product(&scratch, &["login", "work"], b"", None);
    assert_eq!(ran.code, 0, "usable account without a run");
}

#[test]
fn an_unusable_key_report_contains_no_private_data() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    let mut settings: toml::Value =
        toml::from_str(std::str::from_utf8(&support::fake_provider_settings(&scratch)).expect("fixture UTF-8"))
            .expect("fixture TOML");
    settings["accounts"]["codex"] = toml::Value::try_from(serde_json::json!({"source":"key"})).expect("key account");
    let _file = support::user_layer(&scratch, toml::to_string(&settings).expect("settings TOML").as_bytes());
    let key = scratch.path().join("credentials/keys/codex");
    std::fs::create_dir_all(key.parent().expect("key parent")).expect("scratch keys directory");
    std::fs::write(&key, b"key-for-work").expect("scratch key");
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).expect("unusable key mode");
    let ran = support::smith(&scratch, &["check", "--json"], b"");
    assert_eq!(ran.code, 3, "key unavailable");
    let report: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("check document");
    assert!(report["credentials"]["codex"]["state"] == "key_unreadable", "key failure required");
    assert!(report["credentials"]["codex"]["problem"] == "mode", "mode problem required");
    assert!(report["settings"]["values"]["credentials"]["value"] == "<redacted>", "private base redacted");
    assert_eq!(report["credentials"]["codex"]["kept"], serde_json::json!({"kind":"file","at":key}));
    for text in [&ran.stdout, &ran.stderr] {
        let text = std::str::from_utf8(text).expect("report UTF-8");
        assert!(!text.contains("key-for-work"), "no key value");
    }
}
