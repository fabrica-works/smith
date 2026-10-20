use super::{
    lifecycle_tests::{backend, env, request},
    *,
};

fn active() -> Backend {
    let mut backend = backend();
    backend.phase = Phase::Running;
    backend.child = Some(backend.token(4));
    backend.owner = Some(Token::new(80));
    let request = request(&backend);
    let FromDomain::Complete { prompt, .. } = &request else { panic!("complete") };
    let session::Opening::New { id } = session::opening(prompt, None, None) else { panic!("new window") };
    backend.running = Some(id);
    backend.request = Some(request);
    backend
}

#[test]
fn relay_eof_waits_for_the_programs_exit() {
    let mut backend = active();
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.mcp_ended(&env(0), &mut up, &mut below);
    assert!(up.is_empty());
    assert_eq!(backend.phase, Phase::Running);
    backend.from_io(
        &env(0),
        BelowEvent::Io(io::Event::Exited { owner: backend.token(4), exit: kernel::Exit::Code(7) }),
        &mut up,
        &mut below,
    );
    backend.from_io(&env(0), BelowEvent::Io(io::Event::Closed { owner: backend.token(4) }), &mut up, &mut below);
    let Some(ToDomain::Failed { failure, detail, .. }) = up.pop() else { panic!("program terminal") };
    assert_eq!(failure, llm::Failure::Program { exit: Some(7) });
    assert!(detail.is_empty());
}

#[test]
fn a_relay_without_a_program_exit_fails_at_its_grace() {
    let mut backend = active();
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    let until = env(0).now.saturating_add(backend.settings.grace);
    backend.mcp_ended(&env(0), &mut up, &mut below);
    assert!(up.is_empty(), "EOF must not fail before its grace");
    let mut before = env(0);
    before.now = Time::from_nanos(until.as_nanos().checked_sub(1).expect("positive grace"));
    backend.completion_deadline(&before, false);
    assert_eq!(backend.next_deadline(), Some(until));
    backend.fire(&before, &mut up, &mut below);
    assert!(up.is_empty(), "the grace has not expired");
    let mut due = env(0);
    due.now = until;
    backend.fire(&due, &mut up, &mut below);
    let Some(ToDomain::Failed { failure, detail, .. }) = up.pop() else { panic!("program terminal") };
    assert_eq!(failure, llm::Failure::Program { exit: None });
    assert_eq!(detail.as_ref(), b"smith: the MCP server ended");
    backend.fire(&due, &mut up, &mut below);
    assert!(up.is_empty(), "exactly one terminal");
}

#[test]
fn buffered_stdout_cannot_clear_the_relay_eof_grace() {
    let mut backend = active();
    backend.pipes[0] = Some(Token::new(31));
    let until = env(0).now.saturating_add(backend.settings.grace);
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.mcp_ended(&env(0), &mut up, &mut below);
    assert!(up.is_empty(), "EOF must not fail before its grace");
    let mut buffered = env(0);
    buffered.now = Time::from_nanos(until.as_nanos().checked_div(2).expect("mid grace"));
    backend.line(
        &buffered,
        br#"{"type":"assistant","message":{"id":"text","content":[{"type":"text","text":"buffered"}]}}"#,
        &mut up,
        &mut below,
    );
    backend.line(&buffered, br#"{"type":"result","is_error":false}"#, &mut up, &mut below);
    let Some(ToDomain::Text { bytes, .. }) = up.pop() else { panic!("buffered text") };
    assert_eq!(bytes, 8);
    let Some(ToDomain::Completed { .. }) = up.pop() else { panic!("buffered completion") };
    assert!(up.is_empty());
    assert!(backend.deadline.is_none(), "delivery cleared only the completion deadline");
    backend.from_domain(&buffered, request(&backend), &mut up, &mut below);
    assert!(up.is_empty());
    assert_eq!(backend.next_deadline(), Some(until));
    let mut before = env(0);
    before.now = Time::from_nanos(until.as_nanos().checked_sub(1).expect("positive grace"));
    backend.fire(&before, &mut up, &mut below);
    assert!(up.is_empty(), "a new completion cannot advance or clear EOF grace");
    let mut due = env(0);
    due.now = until;
    backend.fire(&due, &mut up, &mut below);
    let Some(ToDomain::Failed { failure, detail, .. }) = up.pop() else { panic!("program terminal") };
    assert_eq!(failure, llm::Failure::Program { exit: None });
    assert_eq!(detail.as_ref(), b"smith: the MCP server ended");
    backend.fire(&due, &mut up, &mut below);
    assert!(up.is_empty(), "exactly one terminal after buffered delivery");
}
