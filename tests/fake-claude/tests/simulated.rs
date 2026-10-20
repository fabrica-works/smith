//! Hosted sign-in stories over skein's fake filesystem (protocol/programs.md,
//! section 8; testing.md, section 4).

mod support;

use skein_io::kernel::Exit;
use smith_fake_claude::{Login, Logout, Observation, Status, Story, observations};
use support::{Invocation, run};

#[global_allocator]
static HEAP: skein_heap::Counting = skein_heap::Counting;

fn status(record: &support::Record) -> serde_json::Value {
    assert!(record.stderr.is_empty());
    serde_json::from_slice(&record.stdout).expect("status JSON")
}

#[test]
fn a_status_without_a_login_is_not_signed_in() {
    let result = run(None, None, vec![Invocation::auth("status")]);
    assert_eq!(result.records[0].exit, Some(Exit::Code(1)));
    assert_eq!(
        status(&result.records[0]),
        serde_json::json!({
            "loggedIn": false, "authMethod": "none", "apiProvider": "firstParty",
        })
    );
}

#[test]
fn a_login_then_a_status_is_signed_in_through_the_subscription() {
    let result = run(None, None, vec![Invocation::auth("login"), Invocation::auth("status")]);
    assert_eq!(result.records[0].exit, Some(Exit::Code(0)));
    assert_eq!(result.records[0].stdout, b"Signed in.\n");
    assert_eq!(result.records[1].exit, Some(Exit::Code(0)));
    assert_eq!(
        status(&result.records[1]),
        serde_json::json!({
            "loggedIn": true, "authMethod": "claude.ai", "apiProvider": "firstParty",
        })
    );
    assert_eq!(result.login.as_deref(), Some(b"claude.ai".as_slice()));
}

#[test]
fn a_login_as_another_method_is_signed_in_otherwise() {
    let method = "api\"key\\method";
    let result = run(
        Some(Story { login: Login::Immediate(method.into()), ..Story::default() }),
        Some(b"old-method-longer"),
        vec![Invocation::auth("login"), Invocation::auth("status")],
    );
    assert_eq!(result.records[1].exit, Some(Exit::Code(0)));
    assert_eq!(status(&result.records[1])["authMethod"], method);
    assert_eq!(result.login.as_deref(), Some(method.as_bytes()));
}

#[test]
fn a_login_by_code_reads_the_pasted_line() {
    let result = run(
        Some(Story { login: Login::Code("claude.ai".into()), ..Story::default() }),
        None,
        vec![Invocation::auth("login").input(b"pasted-code\n"), Invocation::auth("status")],
    );
    assert_eq!(result.records[0].stdout, b"Paste the code from the browser:\n");
    assert_eq!(result.records[0].exit, Some(Exit::Code(0)));
    assert_eq!(status(&result.records[1])["authMethod"], "claude.ai");
    let bytes = result.observations.expect("observations");
    assert!(!String::from_utf8_lossy(&bytes).contains("pasted-code"));
}

#[test]
fn a_login_by_code_refuses_the_inputs_end() {
    let result = run(
        Some(Story { login: Login::Code("claude.ai".into()), ..Story::default() }),
        None,
        vec![Invocation::auth("login")],
    );
    assert_eq!(result.records[0].exit, Some(Exit::Code(1)));
    assert_eq!(result.login, None);
}

#[test]
fn a_login_by_code_refuses_an_unterminated_line() {
    let result = run(
        Some(Story { login: Login::Code("claude.ai".into()), ..Story::default() }),
        None,
        vec![Invocation::auth("login").input(b"unfinished-code")],
    );
    assert_eq!(result.records[0].exit, Some(Exit::Code(1)));
    assert_eq!(result.login, None);
}

#[test]
fn a_login_by_code_refuses_an_empty_line() {
    let result = run(
        Some(Story { login: Login::Code("claude.ai".into()), ..Story::default() }),
        Some(b"seed"),
        vec![Invocation::auth("login").input(b"\n")],
    );
    assert_eq!(result.records[0].exit, Some(Exit::Code(1)));
    assert_eq!(result.login.as_deref(), Some(b"seed".as_slice()));
}

#[test]
fn a_refused_login_keeps_the_login() {
    let result =
        run(Some(Story { login: Login::Refused, ..Story::default() }), Some(b"seed"), vec![Invocation::auth("login")]);
    assert_eq!(result.records[0].exit, Some(Exit::Code(1)));
    assert!(!result.records[0].stderr.is_empty());
    assert_eq!(result.login.as_deref(), Some(b"seed".as_slice()));
}

#[test]
fn a_logout_removes_the_login() {
    let result = run(None, Some(b"claude.ai"), vec![Invocation::auth("logout"), Invocation::auth("status")]);
    assert_eq!(result.records[0].exit, Some(Exit::Code(0)));
    assert_eq!(result.records[0].stdout, b"Signed out.\n");
    assert_eq!(result.records[1].exit, Some(Exit::Code(1)));
    assert_eq!(status(&result.records[1])["loggedIn"], false);
    assert_eq!(result.login, None);
}

#[test]
fn a_logout_without_a_login_succeeds() {
    let result = run(None, None, vec![Invocation::auth("logout")]);
    assert_eq!(result.records[0].exit, Some(Exit::Code(0)));
    assert_eq!(result.records[0].stdout, b"Signed out.\n");
}

#[test]
fn a_failing_logout_keeps_the_login() {
    let result = run(
        Some(Story { logout: Logout::Fails, ..Story::default() }),
        Some(b"claude.ai"),
        vec![Invocation::auth("logout")],
    );
    assert_eq!(result.records[0].exit, Some(Exit::Code(1)));
    assert!(!result.records[0].stderr.is_empty());
    assert_eq!(result.login.as_deref(), Some(b"claude.ai".as_slice()));
}

#[test]
fn an_unreadable_status_is_not_json() {
    let result =
        run(Some(Story { status: Status::Unreadable, ..Story::default() }), None, vec![Invocation::auth("status")]);
    assert_eq!(result.records[0].exit, Some(Exit::Code(0)));
    assert_eq!(result.records[0].stdout, b"not json\n");
    assert!(serde_json::from_slice::<serde_json::Value>(&result.records[0].stdout).is_err());
}

#[test]
fn a_silent_status_never_exits_until_signalled() {
    let result = run(
        Some(Story { status: Status::Silent, ..Story::default() }),
        None,
        vec![Invocation::auth("status").terminate()],
    );
    assert!(result.signalled);
    assert_eq!(result.records[0].exit, Some(Exit::Code(143)));
    assert!(result.records[0].stdout.is_empty());
    assert!(result.records[0].stderr.is_empty());
    assert_eq!(observations(&result.observations.expect("observations")).len(), 1);
}

#[test]
fn without_a_configuration_directory_it_refuses() {
    let result = run(
        None,
        Some(b"seed"),
        vec![
            Invocation::auth("status").environment(&["HOME=home"]),
            Invocation::auth("login").environment(&["HOME=home", "CLAUDE_CONFIG_DIR="]),
            Invocation::auth("logout").environment(&["HOME=home"]),
        ],
    );
    for record in &result.records {
        assert_eq!(record.exit, Some(Exit::Code(2)));
        assert!(record.stdout.is_empty());
        assert_eq!(record.stderr, b"fake claude: CLAUDE_CONFIG_DIR is not set\n");
    }
    assert_eq!(result.observations, None);
    assert_eq!(result.login.as_deref(), Some(b"seed".as_slice()));
}

#[test]
fn each_invocation_is_observed_by_its_environment_s_names() {
    let environment = ["HOME=home", "CLAUDE_CONFIG_DIR=config", "SECRET=never-record-this-value"];
    let result = run(
        None,
        None,
        vec![Invocation::auth("login").environment(&environment), Invocation::auth("status").environment(&environment)],
    );
    let bytes = result.observations.expect("observations");
    assert!(!String::from_utf8_lossy(&bytes).contains("never-record-this-value"));
    assert_eq!(
        observations(&bytes),
        vec![
            Observation {
                arguments: vec!["auth".into(), "login".into()],
                environment: vec!["HOME".into(), "CLAUDE_CONFIG_DIR".into(), "SECRET".into()]
            },
            Observation {
                arguments: vec!["auth".into(), "status".into()],
                environment: vec!["HOME".into(), "CLAUDE_CONFIG_DIR".into(), "SECRET".into()]
            },
        ]
    );
}

#[test]
fn unknown_commands_are_observed_then_refused() {
    let result = run(
        None,
        None,
        vec![
            Invocation::auth("status").arguments(&["auth", "status", "--json"]),
            Invocation::auth("status").arguments(&["-p"]),
        ],
    );
    for record in &result.records {
        assert_eq!(record.exit, Some(Exit::Code(2)));
        assert!(!record.stderr.is_empty());
    }
    assert_eq!(observations(&result.observations.expect("observations")).len(), 2);
}
