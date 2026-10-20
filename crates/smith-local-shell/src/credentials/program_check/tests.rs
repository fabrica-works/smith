use super::*;
use crate::credentials::{
    Source, prepare,
    tests::{Files, resolved_source},
};
use skein_lib::Wall;
use smith_host_domain::{FailureReason, ProgramFailure, ProgramMethod};

#[test]
fn a_program_answer_becomes_its_state() {
    let rows = [
        (Ok(SourceAnswer::Check(ProgramCheck::Usable)), serde_json::json!({"state":"usable","until":null})),
        (
            Ok(SourceAnswer::Check(ProgramCheck::NotSignedIn)),
            serde_json::json!({"state":"program_not_signed_in","program":"claude_code"}),
        ),
        (
            Ok(SourceAnswer::Check(ProgramCheck::SignedInOtherwise(ProgramMethod::new(b"api\\key").expect("method")))),
            serde_json::json!({"state":"program_signed_in_otherwise","program":"claude_code","method":r"api\\key"}),
        ),
        (
            Ok(SourceAnswer::Check(ProgramCheck::Failed(ProgramFailure {
                reason: FailureReason::Unreadable,
                detail: b"bad\x1b\n".as_slice().into(),
            }))),
            serde_json::json!({"state":"program_failed","program":"claude_code","detail":r"bad\x1b\n"}),
        ),
        (
            Err("cannot\rcheck".into()),
            serde_json::json!({"state":"program_failed","program":"claude_code","detail":r"cannot\rcheck"}),
        ),
    ];
    for (answer, expected) in rows {
        assert_eq!(
            serde_json::to_value(state(ProgramName::ClaudeCode, answer)).expect("redacted state"),
            expected,
            "the reported state matches the contract"
        );
    }
}

#[test]
fn an_absent_program_directory_is_not_signed_in_without_a_run() {
    let files = Files::new();
    let mut resolved = resolved_source(&files, "program");
    resolved.settings.values.insert(
        "programs.claude_code".into(),
        crate::settings::Sourced {
            value: toml::Value::String(files.0.join("absent-program").display().to_string()),
            origin: crate::settings::Origin::BuiltIn,
        },
    );
    let states = check(&resolved, ["codex"].into_iter(), &Clock::new());
    assert!(
        matches!(states["codex"], Credential::ProgramNotSignedIn { .. }),
        "the reported state matches the contract"
    );
    assert!(!files.0.join("programs").exists(), "check creates no directory");
}

#[test]
fn startup_refuses_a_needed_program_account_not_signed_in() {
    let files = Files::new();
    let resolved = resolved_source(&files, "program");
    for credential in [
        Credential::ProgramNotSignedIn { program: ProgramName::ClaudeCode },
        Credential::ProgramSignedInOtherwise { program: ProgramName::ClaudeCode, method: "api".into() },
        Credential::ProgramFailed { program: ProgramName::ClaudeCode, detail: "unreadable".into() },
    ] {
        let states = ProgramStates::from([("codex".into(), credential)]);
        let refusal = prepare(&resolved, Wall::from_nanos(0), false, &states).err().expect("refused source");
        assert_eq!(refusal.exit_code(), 3, "the reported state matches the contract");
    }
}

#[test]
fn startup_lends_a_usable_program_account() {
    let files = Files::new();
    let resolved = resolved_source(&files, "program");
    let states = ProgramStates::from([("codex".into(), Credential::Usable { until: None })]);
    let prepared = prepare(&resolved, Wall::from_nanos(0), false, &states).expect("usable source");
    assert!(
        matches!(prepared.lendable.get(&0), Some((0, smith_protocol_channel::GrantValue::Program(name))) if name.as_ref() == b"codex"),
        "the reported state matches the contract"
    );
}

#[test]
fn startup_checks_only_the_needed_program_accounts() {
    let files = Files::new();
    let mut resolved = resolved_source(&files, "program");
    resolved.settings.values.insert(
        "accounts.unused.source".into(),
        crate::settings::Sourced {
            value: toml::Value::String("program".into()),
            origin: crate::settings::Origin::BuiltIn,
        },
    );
    let states = needed(&resolved, &Clock::new());
    assert_eq!(
        states.keys().map(String::as_str).collect::<Vec<_>>(),
        ["codex"],
        "the reported state matches the contract"
    );
    assert!(
        crate::credentials::unused_warnings(&resolved, Wall::from_nanos(0), &states).is_empty(),
        "the reported state matches the contract"
    );
    assert_eq!(
        crate::credentials::inspect(&Source::Program { name: "unused".into() }, Wall::from_nanos(0), &states)
            .line("unused"),
        "account unused: Claude Code could not be checked: not checked",
        "the reported state matches the contract"
    );
}
