use super::*;
use crate::settings::{Origin, Sourced};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("smith-credential-inspection-{}-{serial}", std::process::id()));
        std::fs::create_dir_all(&path).expect("scratch directory");
        Self(path)
    }
    fn login(&self, name: &str) -> PathBuf {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../smith-local-protocol/src/login-fixtures").join(name);
        let to = self.0.join(name);
        std::fs::write(&to, std::fs::read(path).expect("synthetic fixture")).expect("scratch login");
        to
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove scratch directory");
    }
}
fn descriptor(file: PathBuf) -> Source {
    Source::Borrow { file, format: LoginFormat::Codex, program: "codex".into() }
}
fn wall() -> Wall {
    Wall::from_nanos(100_000_000_000)
}
fn resolved(files: &Files, needed_file: PathBuf) -> settings::resolve::Resolved {
    let (local, lower) = smith_local_process_world::process::configuration(skein_io::kernel::Fd::new(70), false);
    let mut values = BTreeMap::new();
    for (name, file) in [("codex", needed_file), ("unused", files.login("expired.json"))] {
        for (field, value) in [
            ("source", "borrow".into()),
            ("format", "codex".into()),
            ("program", "codex".into()),
            ("file", file.display().to_string()),
        ] {
            values.insert(
                format!("accounts.{name}.{field}"),
                Sourced { value: toml::Value::String(value), origin: Origin::BuiltIn },
            );
        }
    }
    values.insert("environment.inherit".into(), Sourced { value: toml::Value::Array(vec![]), origin: Origin::BuiltIn });
    let fields = values
        .iter()
        .filter_map(|(key, value)| key.strip_prefix("accounts.codex.").map(|field| (field.into(), value.clone())))
        .collect();
    settings::resolve::Resolved {
        settings: settings::Settings { workspace: files.0.clone(), values },
        accounts: vec![Account { name: "codex".into(), number: 0, fields }],
        agent: smith_agent_shell::config::Configuration {
            service: lower,
            memory: u64::MAX,
            trace: None,
            profile: smith_agent_service::profile::standard(),
            declarations: smith_agent_process_world::declarations(0),
        },
        limits: local.limits.local,
        local: local.local,
        outside_models: BTreeMap::new(),
    }
}

#[test]
fn a_usable_borrowed_login_reports_expiry_without_its_access_value() {
    let files = Files::new();
    let state = inspect(&descriptor(files.login("valid.json")), wall());
    assert!(state.usable(), "the valid access-only record is usable");
    let json = serde_json::to_string(&state).expect("redacted state");
    assert!(json.contains("120000000000"), "the state keeps its expiry");
    assert!(!json.contains("signature"), "the access value is not a reported field");
    assert!(!json.contains("UNISSUED"), "the refresh value is never reported");
}

#[test]
fn an_expired_login_inspection_names_the_file_time_and_program() {
    let files = Files::new();
    let file = files.login("expired.json");
    let state = inspect(&descriptor(file.clone()), wall());
    assert!(!state.usable(), "an expired record cannot serve the run");
    let line = state.line("codex");
    assert!(line.contains(&file.display().to_string()), "the notice names its source");
    assert!(line.contains("expired at 00:01. Run `codex` once"), "the source's renewal action is exact");
}

#[test]
fn a_malformed_login_inspection_reports_the_file_without_its_bytes() {
    let files = Files::new();
    let file = files.0.join("broken.json");
    std::fs::write(&file, b"UNISSUED_BAD_DOCUMENT").expect("scratch input");
    let state = inspect(&descriptor(file.clone()), wall());
    assert!(!state.usable(), "a malformed record supplies no access value");
    assert_eq!(
        state.line("codex"),
        format!("account codex: unreadable login at {}", file.display()),
        "the refusal reports the source only"
    );
}

#[test]
fn the_credential_catalog_uses_the_resolvers_needed_accounts_and_omits_values() {
    let files = Files::new();
    let resolved = resolved(&files, files.login("valid.json"));
    let accounts = report(&resolved, wall());
    assert!(accounts["codex"].required && accounts["codex"].credential.usable(), "the selected account is usable");
    assert!(
        !accounts["unused"].required && !accounts["unused"].credential.usable(),
        "the expired unused account is only a warning"
    );
    let json = serde_json::to_string(&accounts).expect("redacted accounts");
    assert!(!json.contains("signature") && !json.contains("UNISSUED"), "no login value is a report field");
}

#[test]
fn an_expired_selected_login_refuses_composition_with_exit_three() {
    let files = Files::new();
    let resolved = resolved(&files, files.login("expired.json"));
    let Err(failure) = prepare(&resolved, wall(), false) else {
        panic!("the expired source must refuse before its directory is adopted");
    };
    assert_eq!(failure.exit_code(), 3, "selected credentials have their own refusal code");
    assert!(failure.line().contains("Run `codex` once"), "the compositor gives the source's renewal action");
}

#[test]
fn an_env_value_is_access_only_and_an_invalid_header_value_is_refused() {
    let record = env_record(b"UNISSUED_ACCESS", false, wall()).expect("synthetic API value");
    assert_eq!(record.record.access_token.as_ref(), b"UNISSUED_ACCESS", "the variable is the access value");
    assert!(
        record.record.refresh_token.is_none() && record.account_id.is_empty(),
        "an API variable holds no refresh or Codex identity"
    );
    assert!(env_record(b"UNISSUED\nHEADER", false, wall()).is_none(), "a variable cannot inject a header");
    assert!(env_record(b"UNISSUED_ACCESS", true, wall()).is_none(), "Codex's variable must supply its account claim");
}

#[test]
fn an_identifier_past_the_drivers_receiving_bound_is_not_reported_usable() {
    let files = Files::new();
    let file = files.login("valid.json");
    let mut login: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("synthetic login")).expect("fixture shape");
    login["tokens"]["account_id"] =
        serde_json::Value::String("a".repeat(smith_local_service::Source::account_id_bytes() + 1));
    std::fs::write(&file, serde_json::to_vec(&login).expect("bounded fixture")).expect("scratch login");
    assert!(!inspect(&descriptor(file), wall()).usable(), "inspection uses the driver's account-id bound");
}
