//! Provider declarations are checked without credentials or a running issuer.
use super::*;
use crate::{
    command::Shared,
    presets::{self, Preset},
};

fn settings(text: &str) -> Settings {
    from_sources(
        &Shared::default(),
        "/unused".into(),
        &[Source { layer: Layer::User, path: "providers.toml".into(), text: text.into() }],
    )
    .unwrap()
}

#[test]
fn the_preset_names_include_openai_beside_codex() {
    for (preset, endpoint) in [(Preset::OpenAi, "openai"), (Preset::Codex, "codex"), (Preset::Anthropic, "anthropic")] {
        let table: toml::Table = toml::from_str(presets::built_in(preset)).unwrap();
        assert!(table["endpoints"].get(endpoint).is_some());
    }
    let catalog = settings("");
    assert_eq!(catalog.values["endpoints.openai.address"].value.as_str(), Some("api.openai.com:443"));
    assert!(!catalog.values.contains_key("accounts.openai.source"));
}

#[test]
fn the_openai_models_are_priced_under_openai() {
    let catalog = settings("");
    for (model, key, input, output) in [
        ("gpt-6-astra", "models.openai/gpt-6-astra.window", 10_000_000, 50_000_000),
        ("gpt-6-luna", "models.openai/gpt-6-luna.window", 100_000, 500_000),
        ("gpt-6.1-sol", "models.\"openai/gpt-6.1-sol\".window", 2_000_000, 10_000_000),
    ] {
        let rates = presets::list_prices("openai", model, 272_000).unwrap();
        assert_eq!(rates.input, input);
        assert_eq!(rates.output, output);
        assert_eq!(catalog.values[key].value.as_integer(), Some(272_000));
    }
}

#[test]
fn a_sign_in_account_resolves_its_provider() {
    let selected =
        resolve::select(&settings("model='openai/gpt-6-luna'\n[accounts.work]\nprovider='openai'\nsource='sign_in'\n"))
            .unwrap();
    assert_eq!(selected.accounts[0].name, "work");
    assert_eq!(selected.models.len(), 3);
}

#[test]
fn provider_registration_keys_are_refused() {
    for key in ["client_id", "redirect_uri", "scope", "token_format"] {
        let value = if key == "token_format" { "form" } else { "value" };
        let refusal = resolve::checked(&settings(&format!(
            "[accounts.work]\nsource='sign_in'\nprovider='openai'\n{key}='{value}'\n"
        )))
        .unwrap_err();
        assert_eq!(refusal.key, format!("accounts.work.{key}"));
        assert!(
            matches!(refusal.problem, Problem::Input(ref message) if message == "the provider's preset gives the registration")
        );
    }
}

#[test]
fn the_new_keys_need_a_provider() {
    for (key, value) in [
        ("issuer", "'https://issuer.test'"),
        ("keys_url", "'https://issuer.test/keys'"),
        ("revocation_url", "'https://issuer.test/revoke'"),
        ("redirect_port", "1455"),
    ] {
        let refusal =
            resolve::checked(&settings(&format!("[accounts.work]\nsource='sign_in'\n{key}={value}\n"))).unwrap_err();
        assert_eq!(refusal.key, format!("accounts.work.{key}"));
        assert!(matches!(refusal.problem, Problem::Input(ref message) if message == "needs `provider`"));
    }
}

#[test]
fn two_accounts_need_the_endpoints_account() {
    let refusal = resolve::select(&settings("model='openai/gpt-6-luna'\n[accounts.work]\nprovider='openai'\nsource='key'\n[accounts.home]\nprovider='openai'\nsource='key'\n")).unwrap_err();
    assert_eq!(refusal.key, "endpoints.openai.account");
    assert!(refusal.line().contains("home, work"));
}

#[test]
fn an_http_override_outside_loopback_names_its_key() {
    for key in ["issuer", "keys_url", "revocation_url", "authorization_url", "token_url"] {
        let refusal = resolve::checked(&settings(&format!(
            "[accounts.work]\nprovider='openai'\nsource='sign_in'\n{key}='http://192.0.2.1/token'\n"
        )))
        .unwrap_err();
        assert_eq!(refusal.key, format!("accounts.work.{key}"));
    }
}

#[test]
fn the_anthropic_preset_has_no_registration() {
    let refusal = resolve::checked(&settings("[accounts.work]\nprovider='anthropic'\nsource='sign_in'\n")).unwrap_err();
    assert_eq!(refusal.key, "accounts.work.source");
}
