use super::lifecycle_tests::{backend, env, request};
use super::*;

fn active() -> Backend {
    let mut backend = backend();
    backend.phase = Phase::Running;
    backend.child = Some(Token::new(30));
    backend.pipes[0] = Some(Token::new(31));
    backend.request = Some(request(&backend));
    backend.owner = Some(Token::new(80));
    backend
}

#[test]
fn an_unframed_tool_message_completes_without_any_partial() {
    let mut backend = active();
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.line(&env(0),br#"{"type":"assistant","message":{"id":"unframed","content":[{"type":"tool_use","id":"t","name":"mcp__smith__finish","input":{}}],"usage":{"input_tokens":17,"output_tokens":3}}}"#,&mut up,&mut below);
    let Some(ToDomain::Completed { owner, completion }) = up.pop() else {
        panic!("unframed completion");
    };
    assert_eq!(owner, Token::new(80));
    assert_eq!(completion.usage.input_tokens, Some(17));
    assert!(backend.turns.message.is_none());
}

#[test]
fn an_open_next_message_does_not_receive_a_duplicate_nudge() {
    let mut backend = active();
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.line(
        &env(0),
        br#"{"type":"assistant","message":{"id":"one","content":[{"type":"text","text":"thinking"}]}}"#,
        &mut up,
        &mut below,
    );
    backend.line(
        &env(0),
        br#"{"type":"stream_event","event":{"type":"message_start","message":{"id":"next"}}}"#,
        &mut up,
        &mut below,
    );
    assert!(backend.owner.is_none(), "the first message was delivered");
    let mut next = request(&backend);
    let FromDomain::Complete { prompt, owner, .. } = &mut next else {
        panic!("next completion");
    };
    *owner = Token::new(81);
    prompt.messages = Box::new([llm::Message {
        role: llm::Role::User,
        content: Box::new([llm::Block::Text { text: b"finish now".as_slice().into(), replay: None }]),
    }]);
    backend.resume(&env(0), next, &mut up, &mut below);
    assert!(backend.outgoing.is_none());
    assert_eq!(backend.owner, Some(Token::new(81)));
    assert!(below.is_empty());
}

#[test]
fn a_line_outside_the_stream_grammar_is_a_program_failure() {
    let mut backend = active();
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.line(&env(0), br#"{"other":true}"#, &mut up, &mut below);
    let Some(ToDomain::Failed { failure, .. }) = up.pop() else {
        panic!("program failure");
    };
    assert_eq!(failure, llm::Failure::Program { exit: None });
}
#[test]
fn a_result_without_an_assistant_hands_the_nudge_to_the_domain() {
    let mut backend = active();
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.line(&env(0), br#"{"type":"result","subtype":"success","is_error":false}"#, &mut up, &mut below);
    let Some(ToDomain::Completed { completion, .. }) = up.pop() else {
        panic!("no-tool completion");
    };
    assert!(completion.content.is_empty());
    assert_eq!(completion.stop, llm::Stop::EndTurn);
    assert_eq!(completion.usage, llm::Usage::ZERO);
}
#[test]
fn the_unoffered_sub_agent_tool_is_not_executed() {
    let mut backend = active();
    let Some(FromDomain::Complete { prompt, .. }) = &mut backend.request else {
        panic!("completion");
    };
    prompt.served = Box::new([llm::Served::SubAgent]);
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.line(&env(0),br#"{"type":"assistant","message":{"id":"unframed","content":[{"type":"tool_use","id":"t","name":"mcp__smith__sub_agent","input":{}}]}}"#,&mut up,&mut below);
    let Some(ToDomain::Completed { completion, .. }) = up.pop() else {
        panic!("completion");
    };
    let llm::Said::ToolCall { call, .. } = &completion.content[0] else {
        panic!("tool call");
    };
    assert_eq!(*call, llm::Decoded::Invalid { problem: llm::Problem::UnknownTool });
}
