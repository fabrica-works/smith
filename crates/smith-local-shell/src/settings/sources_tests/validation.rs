use super::*;
use crate::credentials::{Directory, tests::Files};
use std::os::unix::fs::PermissionsExt;

#[test]
fn malformed_account_names_are_refused_without_disappearing() {
    for name in ["\"", "\\", "\"broken", "", "bad.name", "bad name"] {
        let encoded = serde_json::to_string(name).expect("encoded name");
        let text = run(&format!("[accounts.{encoded}]\nsource='none'\n"));
        let refusal = selection(&text).expect_err("invalid account name");
        assert!(refusal.line().contains("account name"), "named refusal");
    }
}

#[test]
fn an_invalid_encoded_account_key_is_refused() {
    let mut settings = settings(&[]).expect("settings");
    settings.values.insert(
        "accounts.\".source".into(),
        super::super::Sourced { value: toml::Value::String("none".into()), origin: super::super::Origin::BuiltIn },
    );
    assert!(resolve::checked(&settings).is_err(), "malformed encoding must be refused");
}

#[test]
fn account_names_round_trip_through_setting_key_parts() {
    for name in ["work", "\"", "\\", "", "named.account"] {
        let key = format!("{}.source", super::super::name(name));
        let (decoded, field) = super::super::named_field(&key).expect("encoded key part");
        assert_eq!(decoded, name, "name preserved for validation");
        assert_eq!(field, "source", "field preserved");
    }
}

#[test]
fn credential_paths_use_only_the_supplied_environment() {
    let mut settings = settings(&[]).expect("settings");
    settings.paths.home = Some("/supplied-home".into());
    settings.paths.state_home = Some("/supplied-state".into());
    let directory = Directory::of(&settings).expect("supplied default");
    assert!(directory.key("work") == std::path::Path::new("/supplied-state/smith/keys/work"), "supplied state base");
}

#[test]
fn program_documents_use_the_supplied_path() {
    let files = Files::new();
    let executable = files.0.join("claude");
    std::fs::write(&executable, b"#!/bin/sh\nexit 0\n").expect("scratch executable");
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).expect("executable mode");
    let mut settings = resolution::configured(&files, "program", true);
    settings.values.remove("programs.claude_code");
    settings.paths.path = Some(files.0.clone().into());
    let (_, bytes) = resolve::resolve_startup(&settings).unwrap_or_else(|why| panic!("{}", why.line()));
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("agent document");
    assert!(document["programs"]["claude_code"]["path"].as_str() == executable.to_str(), "supplied PATH used");
}

#[test]
fn withdrawn_source_settings_name_the_replacement() {
    let mut base = settings(&[source(
        Layer::User,
        "[accounts.work]\nsource='borrow'\nfile='auth.json'\nformat='withdrawn'\nprogram='codex'\n",
    )])
    .expect("withdrawn keys reach source validation");
    base.values.retain(|key, _| key.starts_with("accounts.work."));
    for (source_name, retained, key) in [
        ("borrow", &[][..], "source"),
        ("borrow", &["file", "format"][..], "file"),
        ("env", &["format"][..], "format"),
        ("env", &["program"][..], "program"),
    ] {
        let mut settings = base.clone();
        for field in ["file", "format", "program"] {
            if !retained.contains(&field) {
                settings.values.remove(&format!("accounts.work.{field}"));
            }
        }
        settings.values.get_mut("accounts.work.source").expect("account source").value =
            toml::Value::String(source_name.into());
        let refusal = resolve::checked(&settings).expect_err("withdrawn settings");
        assert_eq!(refusal.key, format!("accounts.work.{key}"));
        assert!(
            refusal
                .line()
                .contains("declare `provider = \"openai\"` and `source = \"sign_in\"`, then run `smith login work`"),
            "replacement names the account"
        );
    }
}
