use super::{Layer, Origin, Source, layers::merge};
use crate::command::Shared;
use std::fmt::Write;
use std::path::PathBuf;

fn source(layer: Layer, text: &str) -> Source {
    Source { layer, path: PathBuf::from("fixture.toml"), text: text.into() }
}

fn settings(sources: &[Source]) -> Result<super::Settings, super::Refusal> {
    merge(&Shared::default(), PathBuf::from("/workspace"), sources)
}

#[test]
fn layers_merge_tables_and_keep_the_winning_origin() {
    let shared =
        Shared { model: Some("flag/model".into()), overrides: vec!["effort='high'".into()], ..Shared::default() };
    let decoded = merge(
        &shared,
        "/workspace".into(),
        &[
            source(Layer::User, "model='user/model'\n[environment.set]\nFIRST='user'\n"),
            source(Layer::Repository, "model='repo/model'\n[environment.set]\nSECOND='repo'\n[build]\npreset='rust'\n"),
        ],
    )
    .expect("merged layers");
    assert_eq!(decoded.values["model"].value.as_str(), Some("flag/model"));
    assert_eq!(decoded.values["model"].origin, Origin::Flag { flag: "-m".into() });
    assert_eq!(decoded.values["effort"].origin, Origin::Override { index: 1 });
    assert_eq!(decoded.values["environment.set.FIRST"].value.as_str(), Some("user"));
    assert_eq!(decoded.values["build.preset"].value.as_str(), Some("rust"));
    assert_eq!(
        decoded.values["environment.set.SECOND"].origin,
        Origin::File { layer: Layer::Repository, path: "fixture.toml".into(), line: 3 }
    );
}

#[test]
fn an_unknown_key_names_its_file_line_and_path() {
    let refusal = settings(&[source(Layer::User, "model='test/model'\nmodle=1\n")]).expect_err("unknown key");
    assert_eq!(refusal.key, "modle");
    assert!(refusal.line().contains("fixture.toml:2"));
    assert_eq!(refusal.problem, super::Problem::UnknownKey);
}

#[test]
fn a_wrong_type_names_the_nested_key_and_its_line() {
    let refusal = settings(&[source(Layer::User, "[accounts.test]\nsource=false\n")]).expect_err("source type");
    assert_eq!(refusal.key, "accounts.test.source");
    assert!(refusal.line().contains("fixture.toml:2"));
}

#[test]
fn an_integer_past_its_record_width_is_refused_at_the_key() {
    let refusal = settings(&[source(Layer::User, "[budget]\nturns=4294967296\n")]).expect_err("count bound");
    assert_eq!(refusal.key, "budget.turns");
    assert!(refusal.line().contains("fixture.toml:2"));
    assert_eq!(refusal.problem, super::Problem::WrongValue);
}

#[test]
fn a_file_past_one_mib_is_refused_before_decoding() {
    let text = " ".repeat(1_048_577);
    let refusal = settings(&[source(Layer::User, &text)]).expect_err("file bound");
    assert!(refusal.line().contains("1 MiB"));
}

#[test]
fn repository_inherited_names_are_authority_and_are_refused() {
    let refusal = settings(&[source(Layer::Repository, "[environment]\ninherit=['TOKEN']\n")]).expect_err("authority");
    assert_eq!(refusal.key, "environment.inherit");
    assert!(refusal.line().contains("fixture.toml:2"));
}

#[test]
fn a_repository_cannot_raise_the_users_budget() {
    let refusal =
        settings(&[source(Layer::User, "[budget]\nturns=12\n"), source(Layer::Repository, "[budget]\nturns=13\n")])
            .expect_err("repository budget");
    assert_eq!(refusal.key, "budget.turns");
    assert_eq!(refusal.problem, super::Problem::RepositoryBudget);
}

#[test]
fn an_unknown_array_field_names_its_element_and_exact_line() {
    let refusal = settings(&[source(Layer::User, "[[workspace.directories]]\nname='main'\npatth='.'\n")])
        .expect_err("directory key");
    assert_eq!(refusal.key, "workspace.directories[0].patth");
    assert!(refusal.line().contains("fixture.toml:3"));
}

#[test]
fn the_built_in_layer_matches_its_golden() {
    let settings = settings(&[]).expect("built-in layer");
    let mut bytes = String::new();
    for (key, sourced) in settings.values {
        assert_eq!(sourced.origin, Origin::BuiltIn);
        writeln!(bytes, "{key}={}", serde_json::to_string(&sourced.value).expect("value JSON")).expect("golden line");
    }
    assert_eq!(bytes, include_str!("built-in.golden"));
}

#[test]
fn haiku_takes_its_long_prompt_rates_only_past_a_100000_token_window() {
    let defaults = settings(&[]).expect("preset");
    let declared = settings(&[source(Layer::User, "[models.'anthropic/claude-haiku-5-5']\nwindow=100000\n")])
        .expect("declared window");
    let key = "models.anthropic/claude-haiku-5-5.input_price";
    assert_eq!(defaults.values["models.anthropic/claude-haiku-5-5.window"].value.as_integer(), Some(200_000));
    assert_eq!(defaults.values[key].value.as_float(), Some(0.5));
    assert_eq!(declared.values[key].value.as_float(), Some(0.1));
    let past = settings(&[source(Layer::User, "[models.'anthropic/claude-haiku-5-5']\nwindow=100001\n")])
        .expect("window past the tier");
    assert_eq!(past.values[key].value.as_float(), Some(0.5));
}

#[test]
fn rates_a_person_sets_stay_as_set_when_the_window_changes() {
    let decoded = settings(&[source(Layer::User,
        "[models.'anthropic/claude-haiku-5-5']\nwindow=100001\ninput_price=0.2\ncached_price=0.03\ncache_write_price=0.4\noutput_price=0.6\n"
    )]).expect("person's prices");
    for (field, expected) in
        [("input_price", 0.2), ("cached_price", 0.03), ("cache_write_price", 0.4), ("output_price", 0.6)]
    {
        let value = &decoded.values[&format!("models.anthropic/claude-haiku-5-5.{field}")];
        assert_eq!(value.value.as_float(), Some(expected));
        assert!(matches!(value.origin, Origin::File { layer: Layer::User, .. }));
    }
}

#[test]
fn a_preset_rate_uses_the_declared_price_unit() {
    let decoded =
        settings(&[source(Layer::User, "[models.'codex/gpt-6-astra']\nprice_unit=1000\n")]).expect("price unit");
    assert_eq!(decoded.values["models.codex/gpt-6-astra.input_price"].value.as_float(), Some(0.01));
}

#[test]
fn codex_takes_long_prompt_rates_only_past_272000_tokens() {
    let decoded =
        settings(&[source(Layer::User, "[models.'codex/gpt-6-astra']\nwindow=272001\n")]).expect("long prompt window");
    assert_eq!(decoded.values["models.codex/gpt-6-astra.input_price"].value.as_float(), Some(20.0));
    assert_eq!(decoded.values["models.codex/gpt-6-astra.output_price"].value.as_float(), Some(75.0));
}

fn selection(text: &str) -> Result<super::resolve::Selection, super::Refusal> {
    let settings = settings(&[source(Layer::User, text)])?;
    super::resolve::select(&settings)
}

#[test]
fn a_codex_only_first_run_needs_no_anthropic_credential() {
    let selected =
        selection("model='codex/gpt-6-astra'\n[accounts.codex]\nsource='borrow'\n").expect("one account's run");
    assert_eq!(selected.models, ["codex/gpt-6-astra"]);
    assert_eq!(selected.accounts.len(), 1);
    assert_eq!(selected.accounts[0].name, "codex");
    assert_eq!(selected.accounts[0].fields["source"].value.as_str(), Some("borrow"));
    assert!(!selected.settings.values.contains_key("accounts.anthropic.source"));
}

#[test]
fn sub_agents_on_adds_the_models_of_every_account_with_a_source() {
    let selected = selection("model='codex/gpt-6-astra'\nsub_agents=true\n[accounts.codex]\nsource='borrow'\n[accounts.anthropic]\nsource='env'\n")
        .expect("accounts with sources");
    assert_eq!(
        selected.models,
        [
            "codex/gpt-6-astra",
            "anthropic/claude-haiku-5-5",
            "anthropic/claude-opus-5-5",
            "anthropic/claude-sonnet-5-5",
            "codex/gpt-6-luna"
        ]
    );
    assert_eq!(
        selected.accounts.iter().map(|account| (account.name.as_str(), account.number)).collect::<Vec<_>>(),
        [("codex", 0), ("anthropic", 1)]
    );
}

#[test]
fn mains_account_without_a_source_is_refused_naming_the_key() {
    let refusal = selection("model='codex/gpt-6-astra'\n").expect_err("main account has no source");
    assert_eq!(refusal.key, "accounts.codex.source");
    assert_eq!(refusal.line(), "smith: built-in settings: accounts.codex.source: required value is absent");
}

#[test]
fn sub_agents_on_keeps_an_unset_accounts_models_out() {
    let selected = selection("model='codex/gpt-6-astra'\nsub_agents=true\n[accounts.codex]\nsource='borrow'\n")
        .expect("codex delegation");
    assert_eq!(selected.models, ["codex/gpt-6-astra", "codex/gpt-6-luna"]);
    assert_eq!(selected.accounts.len(), 1);
}

#[test]
fn a_custom_endpoint_inherits_its_preset_and_keeps_supplied_values() {
    let selected = selection("model='custom/gpt-6-astra'\n[endpoints.custom]\npreset='codex'\naddress='example.test:443'\n[models.'custom/gpt-6-astra']\nwindow=100000\ninput_price=1.0\n[accounts.codex]\nsource='borrow'\n")
        .expect("custom preset");
    let values = selected.settings.values;
    assert_eq!(values["endpoints.custom.address"].value.as_str(), Some("example.test:443"));
    assert!(matches!(values["endpoints.custom.address"].origin, Origin::File { layer: Layer::User, .. }));
    assert_eq!(values["endpoints.custom.server_name"].value.as_str(), Some("chatgpt.com"));
    assert_eq!(values["endpoints.custom.server_name"].origin, Origin::BuiltIn);
    assert_eq!(values["models.custom/gpt-6-astra.window"].value.as_integer(), Some(100_000));
    assert_eq!(values["models.custom/gpt-6-astra.output"].value.as_integer(), Some(32_000));
    assert_eq!(values["models.custom/gpt-6-astra.input_price"].value.as_float(), Some(1.0));
    assert_eq!(values["models.custom/gpt-6-astra.output_price"].value.as_float(), Some(50.0));
}

#[test]
fn dotted_custom_names_keep_their_account_and_model_bindings() {
    let selected = selection("model='custom.endpoint/gpt-6-luna'\n[endpoints.'custom.endpoint']\npreset='codex'\naccount='named.account'\n[accounts.'named.account']\nsource='env'\nvariable='TEST_CREDENTIAL'\n")
        .expect("dotted names");
    assert_eq!(selected.models, ["custom.endpoint/gpt-6-luna"]);
    assert_eq!(selected.accounts[0].name, "named.account");
    assert_eq!(
        selected.settings.values["models.\"custom.endpoint/gpt-6-luna\".window"].value.as_integer(),
        Some(272_000)
    );
}

#[test]
fn an_unknown_endpoint_preset_is_refused_at_its_source() {
    let refusal =
        selection("model='custom/model'\n[endpoints.custom]\npreset='unknown'\n").expect_err("unknown preset");
    assert_eq!(refusal.key, "endpoints.custom.preset");
    assert_eq!(refusal.origin, Origin::File { layer: Layer::User, path: "fixture.toml".into(), line: 3 });
}

#[test]
fn a_declared_model_with_a_source_is_available_to_sub_agents() {
    let selected = selection("model='codex/gpt-6-astra'\nsub_agents=true\n[accounts.codex]\nsource='borrow'\n[models.'codex/declared']\nwindow=100000\noutput=1000\ninput_price=1.0\ncached_price=0.1\noutput_price=2.0\n")
        .expect("declared child model");
    assert_eq!(selected.models, ["codex/gpt-6-astra", "codex/declared", "codex/gpt-6-luna"]);
    assert_eq!(selected.accounts.len(), 1);
}

#[test]
fn choosing_another_preset_replaces_only_the_endpoints_default_values() {
    let selected = selection("model='codex/claude-haiku-5-5'\n[endpoints.codex]\npreset='anthropic'\naddress='proxy.test:443'\n[accounts.anthropic]\nsource='env'\n")
        .expect("preset over defaults");
    assert_eq!(selected.accounts[0].name, "anthropic");
    assert_eq!(selected.settings.values["endpoints.codex.server_name"].value.as_str(), Some("api.anthropic.com"));
    assert_eq!(selected.settings.values["endpoints.codex.address"].value.as_str(), Some("proxy.test:443"));
    assert_eq!(selected.settings.values["models.codex/claude-haiku-5-5.window"].value.as_integer(), Some(200_000));
}
