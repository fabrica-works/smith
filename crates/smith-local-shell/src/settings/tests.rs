use super::{Layer, Origin, Source, layers::merge};
use crate::command::Shared;
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
