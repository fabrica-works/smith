use super::{
    Layer, resolve,
    tests::{selection, settings, source},
};

mod resolution;
mod validation;

fn run(extra: &str) -> String {
    format!("model='codex/gpt-6-luna'\n{extra}\n[accounts.codex]\nsource='key'\n")
}

#[test]
fn selected_accounts_are_numbered_by_name_not_order() {
    for model in ["codex/gpt-6-luna", "anthropic/claude-haiku-5-5"] {
        let selected = selection(&format!(
            "model='{model}'\nsub_agents=true\n[accounts.codex]\nsource='key'\n[accounts.anthropic]\nsource='key'\n"
        ))
        .expect("selection");
        for account in selected.accounts {
            let expected = match account.name.as_str() {
                "codex" => 1_474_186_484,
                "anthropic" => 3_339_635_307,
                _ => panic!("fixture account"),
            };
            assert_eq!(account.number, expected, "{}", account.name);
        }
    }
}

fn collision(source: &str, selected: bool) -> super::Refusal {
    let extra = if selected { "[endpoints.codex]\naccount='acct-78235'\n" } else { "" };
    selection(&run(&format!("{extra}[accounts.acct-78235]\nsource='key'\n[accounts.acct-124172]\nsource='{source}'\n")))
        .expect_err("collision")
}

fn names(refusal: &super::Refusal) {
    for text in ["acct-78235", "acct-124172", "2996738092"] {
        assert!(refusal.line().contains(text), "collision names {text}");
    }
    assert_eq!(refusal.key, "accounts.acct-78235", "later sorted name");
}

#[test]
fn the_known_prefix_collision_is_refused_at_selection() {
    names(&collision("key", true));
}

#[test]
fn a_collision_between_unselected_accounts_is_refused() {
    names(&collision("key", false));
}

#[test]
fn a_none_account_takes_part_in_the_collision_check() {
    names(&collision("none", false));
}

#[test]
fn a_program_account_on_an_endpoint_without_a_program_is_refused() {
    let refusal =
        selection("model='codex/gpt-6-luna'\n[accounts.codex]\nsource='program'\n").expect_err("no endpoint program");
    assert_eq!(refusal.key, "accounts.codex.source", "source key");
    for text in ["accounts.codex", "endpoints.codex.program"] {
        assert!(refusal.line().contains(text), "binding names {text}");
    }
}

#[test]
fn a_program_account_on_a_program_endpoint_resolves() {
    let selected = selection(
        "model='codex/gpt-6-luna'\n[accounts.codex]\nsource='program'\n[endpoints.codex]\nprogram='claude_code'\n",
    )
    .expect("program selection");
    assert_eq!(selected.accounts[0].source(), "program", "selected source");
}

#[test]
fn sub_agents_leave_out_the_models_of_a_program_account() {
    let selected = selection(&run(
        "sub_agents=true\n[accounts.anthropic]\nsource='program'\n[endpoints.anthropic]\nprogram='claude_code'\n",
    ))
    .expect("selection");
    assert!(selected.models.iter().all(|name| name.starts_with("codex/")), "program models excluded");
}

#[test]
fn the_main_endpoint_keeps_its_models_on_a_program_account() {
    let selected = selection(
        "model='codex/gpt-6-luna'\n[accounts.codex]\nsource='program'\n[endpoints.codex]\nprogram='claude_code'\n",
    )
    .expect("selection");
    assert_eq!(selected.models, ["codex/gpt-6-luna", "codex/gpt-6-astra", "codex/gpt-6.1-sol"], "main endpoint models");
}

#[test]
fn a_repository_layer_cannot_set_credentials_or_programs() {
    for (text, key) in
        [("credentials='/tmp/private'\n", "credentials"), ("[programs]\nclaude_code='/bin/true'\n", "programs")]
    {
        let refusal = settings(&[source(Layer::Repository, text)]).expect_err("authority");
        assert_eq!(refusal.key, key, "authority key");
        assert_eq!(refusal.problem, super::Problem::RepositoryAuthority, "repository refusal");
    }
}

#[test]
fn a_relative_credentials_setting_is_refused() {
    let refusal = selection(&run("credentials='relative'\n")).expect_err("relative directory");
    assert_eq!(refusal.key, "credentials", "directory key");
}
