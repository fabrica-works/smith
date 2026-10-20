use super::tests::Files;
use super::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

#[test]
fn each_unusable_state_says_its_exit_3_words() {
    let program = ProgramName::ClaudeCode;
    let rows = [
        (Credential::NotSignedIn, "account work has no credential: run `smith login work`"),
        (Credential::SignInAgain, "account work must sign in again: run `smith login work`"),
        (
            Credential::WithoutScope { scope: "chatgpt.tokens.use.direct".into() },
            "account work was signed in without chatgpt.tokens.use.direct: run `smith login work` again and allow it, or use a key",
        ),
        (
            Credential::ProgramNotSignedIn { program },
            "account work: Claude Code is not signed in: run `smith login work`",
        ),
        (
            Credential::ProgramSignedInOtherwise { program, method: "api".into() },
            "account work: Claude Code is signed in with api, not a subscription: run `smith login work`",
        ),
        (
            Credential::ProgramFailed { program, detail: "unreadable".into() },
            "account work: Claude Code could not be checked: unreadable",
        ),
        (Credential::Unset { variable: "WORK_KEY".into() }, "account work: WORK_KEY is not set"),
        (
            Credential::ExpiredVariable { at: 60_000_000_000, variable: "WORK_KEY".into() },
            "account work: variable WORK_KEY expired at 00:01",
        ),
        (Credential::Unreadable { file: "/tokens/work".into() }, "account work: unreadable login at /tokens/work"),
        (Credential::KeyUnreadable { problem: KeyProblem::Mode }, "account work: key has unsafe mode"),
        (Credential::KeyUnreadable { problem: KeyProblem::Size }, "account work: key has invalid size"),
        (Credential::KeyUnreadable { problem: KeyProblem::Unreadable }, "account work: key is unreadable"),
    ];
    for (credential, expected) in rows {
        assert!(!credential.usable(), "the reported state matches the contract");
        assert_eq!(credential.line("work"), expected, "the reported state matches the contract");
        assert_eq!(
            Refusal::Credentials { account: "work".into(), credential }.line(),
            format!("smith: {expected}"),
            "the reported state matches the contract"
        );
    }
}

fn record(files: &Files, scopes: Option<&[u8]>) -> PathBuf {
    let mut record = env_record(b"key-for-work", false, Wall::from_nanos(0)).expect("synthetic record").record;
    record.expires_at = Wall::from_nanos(60_000_000_000);
    record.granted_scopes = scopes.map(Box::from);
    let bytes = skein_oauth::encode_record(&record, &sign_in::document_limits()).expect("bounded record");
    let path = files.0.join("record");
    let mut file =
        std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).expect("private record");
    file.write_all(&bytes).expect("write record");
    path
}

#[test]
fn an_expired_record_without_a_refresh_token_must_sign_in_again() {
    let files = Files::new();
    let path = record(&files, Some(b"chatgpt.tokens.use.direct"));
    let state = inspect(
        &Source::SignIn { file: path, required_scopes: Some(b"chatgpt.tokens.use.direct".as_slice().into()) },
        Wall::from_nanos(60_000_000_000),
        &ProgramStates::new(),
    );
    assert!(matches!(state, Credential::SignInAgain), "the reported state matches the contract");
}

#[test]
fn a_record_without_the_plan_scope_is_signed_in_without_it() {
    let files = Files::new();
    let path = record(&files, Some(b"openid profile"));
    let fields = BTreeMap::from([(
        "provider".into(),
        Sourced { value: toml::Value::String("openai".into()), origin: settings::Origin::BuiltIn },
    )]);
    let settings =
        settings::Settings { paths: settings::Paths::default(), workspace: files.0.clone(), values: BTreeMap::new() };
    let descriptor = source(
        &fields,
        &settings,
        false,
        "work",
        Some(&Directory::from_base(files.0.clone()).expect("absolute scratch")),
    );
    let descriptor = match descriptor {
        Source::SignIn { required_scopes, .. } => Source::SignIn { file: path.clone(), required_scopes },
        Source::Env { .. } | Source::Key { .. } | Source::Program { .. } | Source::Nothing => {
            panic!("sign-in descriptor")
        }
    };
    let state = inspect(&descriptor, Wall::from_nanos(0), &ProgramStates::new());
    assert!(
        matches!(&state, Credential::WithoutScope { scope } if scope == "chatgpt.tokens.use.direct"),
        "the reported state matches the contract"
    );
    assert!(
        !serde_json::to_string(&state).expect("redacted state").contains("key-for-work"),
        "the reported state matches the contract"
    );
    assert!(
        inspect(&Source::SignIn { file: path, required_scopes: None }, Wall::from_nanos(0), &ProgramStates::new())
            .usable(),
        "the reported state matches the contract"
    );
}

#[test]
fn a_record_is_checked_against_the_supplied_required_scopes() {
    let files = Files::new();
    let file = record(&files, Some(b"read"));
    let source = Source::SignIn { file, required_scopes: Some(b"read upload".as_slice().into()) };
    let credential = inspect(&source, Wall::from_nanos(0), &ProgramStates::new());
    assert_eq!(
        serde_json::to_value(credential).expect("redacted state"),
        serde_json::json!({"state":"without_scope","scope":"upload"}),
        "inspection uses the source's scopes rather than a provider name"
    );
}

#[test]
fn an_omitted_grant_scope_keeps_the_record_usable() {
    let files = Files::new();
    let file = record(&files, None);
    let source = Source::SignIn { file, required_scopes: Some(b"read upload".as_slice().into()) };
    let credential = inspect(&source, Wall::from_nanos(0), &ProgramStates::new());
    assert_eq!(
        serde_json::to_value(credential).expect("redacted state"),
        serde_json::json!({"state":"usable","until":60_000_000_000_u64}),
        "omission grants the requested scopes"
    );
}
