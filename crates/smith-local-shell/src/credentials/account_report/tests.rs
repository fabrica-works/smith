use super::*;
use crate::credentials::tests::{Files, resolved_source};
use crate::settings::{Origin, Sourced};
use serde_json::json;
use std::fmt::Write;

#[test]
fn the_report_names_provider_source_number_place() {
    let files = Files::new();
    let mut resolved = resolved_source(&files, "none");
    resolved.accounts.clear();
    let states = ProgramStates::from([(
        "work".into(),
        Credential::ProgramNotSignedIn { program: smith_agent_shell::config::ProgramName::ClaudeCode },
    )]);
    let rows = [
        ("sign_in", Some(json!({"kind":"file","at":files.0.join("tokens/work")}))),
        ("key", Some(json!({"kind":"file","at":files.0.join("keys/work")}))),
        ("env", Some(json!({"kind":"variable","at":"SMITH_REPORT_TEST_KEY"}))),
        ("program", Some(json!({"kind":"directory","at":files.0.join("programs/work")}))),
        ("none", None),
    ];
    for (source, kept) in rows {
        resolved.settings.values.retain(|key, _| !key.starts_with("accounts."));
        for (field, value) in [("provider", "anthropic"), ("source", source), ("variable", "SMITH_REPORT_TEST_KEY")] {
            resolved.settings.values.insert(
                format!("accounts.work.{field}"),
                Sourced { value: toml::Value::String(value.into()), origin: Origin::BuiltIn },
            );
        }
        let report = report(&resolved, Wall::from_nanos(0), &states);
        let account = serde_json::to_value(&report["work"]).expect("redacted account");
        assert_eq!(account["provider"], "anthropic", "the reported state matches the contract");
        assert_eq!(account["source"], source, "the reported state matches the contract");
        // printf work | sha256sum => 00e13ed7..., first four bytes big endian.
        assert_eq!(account["number"], 0x00e1_3ed7, "the reported state matches the contract");
        assert_eq!(account["required"], false, "the reported state matches the contract");
        assert_eq!(account.get("kept"), kept.as_ref(), "the reported state matches the contract");
        let mut line = format!("account work: provider anthropic, source {source}, number 14761687");
        if let Some(kept) = &kept {
            write!(line, ", kept at {}", kept["at"].as_str().expect("location")).expect("String formatting");
        }
        assert_eq!(report["work"].line("work"), line, "the reported state matches the contract");
    }
    resolved.settings.values.remove("accounts.work.provider");
    let report = report(&resolved, Wall::from_nanos(0), &states);
    let account = serde_json::to_value(&report["work"]).expect("account");
    assert!(account.get("provider").is_none(), "the reported state matches the contract");
    assert_eq!(
        report["work"].line("work"),
        "account work: source none, number 14761687",
        "the reported state matches the contract"
    );
}

#[test]
fn the_report_omits_a_place_without_a_credentials_directory() {
    let files = Files::new();
    let mut places = BTreeMap::new();
    for source in ["sign_in", "key", "program"] {
        let mut resolved = resolved_source(&files, source);
        resolved.settings.values.remove("credentials");
        let catalog = report(&resolved, Wall::from_nanos(0), &ProgramStates::new());
        let account = serde_json::to_value(&catalog["codex"]).expect("redacted account");
        places.insert(source, account.get("kept").cloned());
    }
    assert_eq!(
        places,
        BTreeMap::from([("sign_in", None), ("key", None), ("program", None)]),
        "no absolute base means no kept location for any owned source"
    );
}
