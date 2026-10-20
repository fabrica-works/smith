//! Offline controls for live configuration and the credential-directory guard.
#[path = "support/live.rs"]
pub mod live;

#[test]
fn the_live_arguments_resolve_only_the_requested_account() {
    assert_account("claude");
}

#[test]
fn chatgpt_live_arguments_select_only_chatgpt() {
    assert_account("chatgpt");
}

#[test]
fn key_live_arguments_select_only_the_key() {
    assert_account("anthropic-key");
}

fn assert_account(account: &str) {
    use smith_local_shell::{command, settings};
    let directory = std::path::Path::new("/unused/smith-live-story");
    let command::Command::Exec(execution) =
        command::parse(live::arguments(account, directory, true, false)).expect("live command parses")
    else {
        panic!("exec command")
    };
    let source = settings::Source { layer: settings::Layer::User, path: "/unused/live/config/smith/config.toml".into(), text: "credentials='/unused/live'\n[accounts.chatgpt]\nprovider='openai'\nsource='sign_in'\n[accounts.claude]\nprovider='anthropic'\nsource='program'\n[accounts.anthropic-key]\nprovider='anthropic'\nsource='env'\nvariable='ANTHROPIC_API_KEY'\n".into() };
    let settings = settings::from_sources(&execution.shared, directory.into(), &[source])
        .unwrap_or_else(|why| panic!("{}", why.line()));
    let resolved = settings::resolve::select(&settings).unwrap_or_else(|why| panic!("{}", why.line()));
    assert!(
        resolved.accounts.len() == 1 && resolved.accounts.first().is_some_and(|selected| selected.name == account),
        "only this story's account is selected"
    );
}

#[test]
fn the_live_arguments_carry_private_program_homes() {
    let directory = std::path::Path::new("/unused/smith-live-story");
    let command = smith_local_shell::command::parse(live::arguments("claude", directory, false, false))
        .expect("live command parses");
    let smith_local_shell::command::Command::Exec(execution) = command else { panic!("exec command") };
    for (name, path) in [("HOME", directory.join("home")), ("XDG_STATE_HOME", directory.join("state"))] {
        let expected =
            format!("environment.set.{name}={}", toml::Value::String(path.to_str().expect("synthetic path").into()));
        assert!(
            execution.shared.overrides.contains(&expected),
            "the program receives its scratch home by an explicit setting"
        );
    }
}
