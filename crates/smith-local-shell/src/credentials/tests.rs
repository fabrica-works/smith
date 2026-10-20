use super::*;
use crate::settings::{Origin, Sourced};
use std::os::unix::fs::DirBuilderExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);
pub(crate) struct Files(pub(crate) PathBuf);
impl Files {
    pub(crate) fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("smith-credential-inspection-{}-{serial}", std::process::id()));
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&path).expect("scratch directory");
        Self(path)
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove scratch directory");
    }
}
fn wall() -> Wall {
    Wall::from_nanos(100_000_000_000)
}
#[expect(
    clippy::zero_sized_map_values,
    reason = "The fixture retains adapter names even while Claude Code is the sole adapter."
)]
fn resolved(files: &Files) -> settings::resolve::Resolved {
    let (local, lower) = smith_local_process_world::process::configuration(skein_io::kernel::Fd::new(70), false);
    let mut values = BTreeMap::new();
    for (name, source) in [("codex", "none"), ("unused", "env")] {
        for (field, value) in [("source", source), ("variable", "SMITH_UNSET_FIXTURE")] {
            values.insert(
                format!("accounts.{name}.{field}"),
                Sourced { value: toml::Value::String(value.into()), origin: Origin::BuiltIn },
            );
        }
    }
    values.insert("environment.inherit".into(), Sourced { value: toml::Value::Array(vec![]), origin: Origin::BuiltIn });
    let fields = values
        .iter()
        .filter_map(|(key, value)| key.strip_prefix("accounts.codex.").map(|field| (field.into(), value.clone())))
        .collect();
    settings::resolve::Resolved {
        settings: settings::Settings { paths: settings::Paths::default(), workspace: files.0.clone(), values },
        accounts: vec![Account { name: "codex".into(), number: 0, fields }],
        agent: smith_agent_shell::config::Configuration {
            programs: BTreeMap::new(),
            endpoint_programs: BTreeMap::new(),
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
fn the_credential_catalog_uses_the_resolvers_needed_accounts_and_omits_values() {
    let files = Files::new();
    let resolved = resolved(&files);
    let accounts = report(&resolved, wall(), &ProgramStates::new());
    assert!(accounts["codex"].required && accounts["codex"].credential.usable(), "the selected account is usable");
    assert!(
        !accounts["unused"].required && !accounts["unused"].credential.usable(),
        "the unset unused account is only a warning"
    );
    let json = serde_json::to_string(&accounts).expect("redacted accounts");
    assert!(!json.contains("signature") && !json.contains("UNISSUED"), "no login value is a report field");
}

#[test]
fn an_env_value_is_access_only_and_an_invalid_header_value_is_refused() {
    let record = env_record(b"UNISSUED_ACCESS", false, wall()).expect("synthetic API value");
    assert!(record.record.access_token.as_ref() == b"UNISSUED_ACCESS", "the variable is the access value");
    assert!(
        record.record.refresh_token.is_none() && record.account_id.is_empty(),
        "an API variable holds no refresh or Codex identity"
    );
    assert!(env_record(b"UNISSUED\nHEADER", false, wall()).is_none(), "a variable cannot inject a header");
    assert!(env_record(b"UNISSUED_ACCESS", true, wall()).is_none(), "Codex's variable must supply its account claim");
}

pub(crate) fn resolved_source(files: &Files, source: &str) -> settings::resolve::Resolved {
    let mut resolved = resolved(files);
    let value = Sourced { value: toml::Value::String(source.into()), origin: Origin::BuiltIn };
    resolved.accounts[0].fields.insert("source".into(), value.clone());
    resolved.settings.values.insert("accounts.codex.source".into(), value);
    resolved.settings.values.insert(
        "credentials".into(),
        Sourced { value: toml::Value::String(files.0.display().to_string()), origin: Origin::BuiltIn },
    );
    resolved
}

pub(crate) fn private_key(files: &Files) {
    use std::os::unix::fs::PermissionsExt;
    let key = files.0.join("keys/codex");
    std::fs::create_dir_all(key.parent().expect("keys parent")).expect("keys directory");
    std::fs::write(&key, b"key-for-work").expect("scratch key");
    std::fs::set_permissions(key, std::fs::Permissions::from_mode(0o600)).expect("private key");
}

#[test]
fn preparing_refuses_a_key_account_without_a_key() {
    let files = Files::new();
    for interactive in [false, true] {
        let refusal = prepare(&resolved_source(&files, "key"), wall(), interactive, &ProgramStates::new())
            .err()
            .expect("missing key");
        assert_eq!(refusal.exit_code(), 3, "credentials refusal");
        assert!(refusal.line().contains("codex") && refusal.line().contains("smith login"), "account and next action");
    }
}

fn no_source(source: &str) {
    let files = Files::new();
    if source == "key" {
        private_key(&files);
    }
    let prepared =
        prepare(&resolved_source(&files, source), wall(), false, &ProgramStates::new()).expect("usable source");
    assert!(prepared.sources.is_empty(), "the shell owns this source");
    assert_eq!(prepared.locations.len(), 1, "one selected location");
    assert_eq!(prepared.locations.iter().next().expect("location").name, "codex", "selected account");
}

#[test]
fn preparing_a_key_account_hands_no_source() {
    no_source("key");
}

#[test]
fn preparing_a_program_account_hands_no_source() {
    let files = Files::new();
    let programs = BTreeMap::from([("codex".into(), Credential::Usable { until: None })]);
    let prepared = prepare(&resolved_source(&files, "program"), wall(), false, &programs).expect("usable program");
    assert!(prepared.sources.is_empty(), "the shell lends the program name");
}

#[test]
fn preparing_a_none_account_hands_no_source() {
    no_source("none");
}
