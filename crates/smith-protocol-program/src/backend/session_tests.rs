#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    clippy::match_like_matches_macro,
    clippy::wildcard_enum_match_arm,
    reason = "ordinary deterministic harness, programming-model.md 10.2"
)]
use super::{
    lifecycle_tests::{backend, env, request},
    *,
};
use serde_json::json;

fn message(role: llm::Role, text: &[u8]) -> llm::Message {
    llm::Message { role, content: Box::new([llm::Block::Text { text: text.into(), replay: None }]) }
}

fn ordinary() -> llm::Prompt {
    let backend = backend();
    let FromDomain::Complete { mut prompt, .. } = request(&backend) else { panic!("complete") };
    prompt.messages = Box::new([message(llm::Role::User, b"task\n\"quoted\"\t\x01")]);
    prompt
}

#[test]
fn the_session_id_follows_the_window() {
    let mut prompt = ordinary();
    let session::Opening::New { id } = session::opening(&prompt, None, None) else { panic!("new") };
    let first = stream::user_message(&prompt, 4096).expect("first input");
    assert_eq!(id, session::session_id(prompt.affinity, &first));
    prompt.messages = Box::new([
        message(llm::Role::User, b"task\n\"quoted\"\t\x01"),
        message(llm::Role::Assistant, b"answer"),
        message(llm::Role::User, b"next"),
    ]);
    assert_eq!(session::opening(&prompt, Some(id), None), session::Opening::Running);
    assert_eq!(session::opening(&prompt, None, None), session::Opening::Resume { id });
    prompt.messages[0] = message(llm::Role::User, b"new window");
    assert_ne!(session::opening(&prompt, Some(id), None), session::Opening::Running);
    prompt = ordinary();
    prompt.affinity.thread = 1;
    assert_ne!(session::opening(&prompt, Some(id), None), session::Opening::Running);
}

#[test]
fn the_session_id_is_a_v4_uuid() {
    let session::Opening::New { id } = session::opening(&ordinary(), None, None) else { panic!("new") };
    assert_eq!(id[6] >> 4_u32, 4_u8);
    assert_eq!(id[8] >> 6_u32, 2_u8);
    assert_eq!(session::uuid(id).len(), 36);
}

#[test]
fn a_new_window_stops_the_old_session() {
    let mut backend = backend();
    backend.phase = Phase::Running;
    backend.child = Some(backend.token(4));
    backend.pipes = [Some(Token::new(31)), Some(Token::new(32)), Some(Token::new(33))];
    let session::Opening::New { id } = session::opening(&ordinary(), None, None) else { panic!("new") };
    backend.running = Some(id);
    backend.owner = Some(Token::new(80));
    backend.request = Some(request(&backend));
    let mut next = request(&backend);
    let FromDomain::Complete { prompt, .. } = &mut next else { panic!("complete") };
    *prompt = ordinary();
    prompt.messages[0] = message(llm::Role::User, b"next window");
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.from_domain(&env(0), next, &mut up, &mut below);
    assert!(backend.switching);
    assert!(below.iter().any(|effect| match effect {
        Below::Io(io::Request::Signal { signal: kernel::Signal::Interrupt, .. }) => true,
        _ => false,
    }));
    assert!(!below.iter().any(|effect| match effect {
        Below::Io(io::Request::Spawn { .. }) => true,
        _ => false,
    }));
    backend.line(&env(0), br#"{"type":"result","is_error":true}"#, &mut up, &mut below);
    backend.flush(&mut below);
    assert!(below.iter().any(|effect| match effect {
        Below::Io(io::Request::Close { entity }) => *entity == Token::new(31),
        _ => false,
    }));
    assert!(up.is_empty(), "old interrupted result has no terminal for the new owner");
}

#[test]
fn a_compaction_runs_without_tools() {
    let mut prompt = ordinary();
    prompt.purpose = llm::Purpose::Compaction { room: 1000 };
    let args = compact::arguments(&prompt, b"instructions.md", b"session").expect("compact arguments");
    assert!(args.iter().any(|argument| argument.as_ref() == b"--strict-mcp-config"));
    for forbidden in [b"--mcp-config".as_slice(), b"--permission-prompt-tool", b"--json-schema", b"--resume"] {
        assert!(!args.iter().any(|argument| argument.as_ref() == forbidden));
    }
    assert!(args.windows(2).any(|pair| pair[0].as_ref() == b"--tools" && pair[1].is_empty()));
    assert!(args.windows(2).any(|pair| pair[0].as_ref() == b"--max-turns" && pair[1].as_ref() == b"1"));
    assert!(
        args.windows(2)
            .any(|pair| pair[0].as_ref() == b"--append-system-prompt-file" && pair[1].as_ref() == b"instructions.md")
    );
}

#[test]
fn a_compaction_renders_the_window_as_one_message() {
    let mut backend = backend();
    let mut next = request(&backend);
    let FromDomain::Complete { prompt, observation, .. } = &mut next else { panic!("complete") };
    prompt.purpose = llm::Purpose::Compaction { room: 1000 };
    prompt.messages = Box::new([
        message(llm::Role::User, b"task"),
        llm::Message {
            role: llm::Role::Assistant,
            content: Box::new([llm::Block::ToolCall {
                id: b"id".as_slice().into(),
                name: b"read".as_slice().into(),
                input: br#"{"path":"x"}"#.as_slice().into(),
                replay: None,
            }]),
        },
        llm::Message {
            role: llm::Role::User,
            content: Box::new([llm::Block::ToolResult {
                id: b"id".as_slice().into(),
                result: llm::Returned::Text { text: b"file contents".as_slice().into(), error: false, replay: None },
            }]),
        },
        message(llm::Role::User, b"summarise"),
    ]);
    *observation = Some(protocol::Observation { first: true, messages: 0 });
    backend.begin_prompt(&next);
    backend.request = Some(next);
    backend.compaction_input().expect("compact input");
    let value: serde_json::Value =
        serde_json::from_slice(backend.outgoing.as_ref().expect("outgoing")).expect("user line");
    assert_eq!(
        value["message"]["content"],
        "user:\ntask\nassistant:\nread: {\"path\":\"x\"}\nuser:\nresult of id: file contents\nuser:\nsummarise\n"
    );
    backend.flush_prompt_measure(Time::ZERO);
    let Some(protocol::Measure::Prompt { prompt, .. }) = backend.pop_measure() else { panic!("capture") };
    assert!(prompt.tools.expect("inventory").is_empty());
    let smith_events::Block::Text { text } = &prompt.messages.get(0).expect("message").blocks.get(0).expect("block")
    else {
        panic!("text")
    };
    assert_eq!(text.as_ref(), value["message"]["content"].as_str().expect("text").as_bytes());
}

#[test]
fn a_compaction_s_text_is_its_summary() {
    let mut backend = backend();
    backend.phase = Phase::Running;
    backend.owner = Some(Token::new(80));
    backend.child = Some(backend.token(4));
    backend.pipes[0] = Some(Token::new(31));
    let mut request = request(&backend);
    let FromDomain::Complete { prompt, .. } = &mut request else { panic!("complete") };
    prompt.purpose = llm::Purpose::Compaction { room: 1000 };
    backend.request = Some(request);
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.line(&env(0),&serde_json::to_vec(&json!({"type":"assistant","message":{"id":"summary","content":[{"type":"text","text":"short summary"}],"usage":{"input_tokens":17_u32,"output_tokens":3_u32}}})).expect("json"),&mut up,&mut below);
    backend.line(&env(0), br#"{"type":"result","is_error":false}"#, &mut up, &mut below);
    let Some(ToDomain::Text { .. }) = up.pop() else { panic!("text") };
    let Some(ToDomain::Completed { completion, .. }) = up.pop() else { panic!("summary") };
    assert_eq!(completion.usage.input_tokens, Some(17));
    assert_eq!(completion.usage.output_tokens, Some(3));
    let llm::Said::Text { text, .. } = &completion.content[0] else { panic!("text") };
    assert_eq!(text.as_ref(), b"short summary");
    backend.flush(&mut below);
    assert!(below.iter().any(|effect| match effect {
        Below::Io(io::Request::Close { entity }) => *entity == Token::new(31),
        _ => false,
    }));
}

#[test]
fn the_program_s_own_compaction_passes() {
    let mut backend = backend();
    backend.phase = Phase::Running;
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.line(&env(0), br#"{"type":"system","subtype":"compact_boundary"}"#, &mut up, &mut below);
    assert!(up.is_empty());
    assert_eq!(backend.phase, Phase::Running);
}

#[test]
fn a_pending_window_keeps_the_stop_grace() {
    let mut backend = backend();
    backend.phase = Phase::Running;
    backend.switching = true;
    backend.child = Some(backend.token(4));
    backend.pipes[0] = Some(Token::new(31));
    backend.turns.result = true;
    backend.deadline = Some(env(1).now);
    let mut up = Queue::with_capacity(32);
    let mut below = Queue::with_capacity(64);
    backend.from_domain(&env(0), request(&backend), &mut up, &mut below);
    assert_eq!(backend.deadline, Some(env(1).now));
    assert!(below.is_empty(), "pending window starts nothing until the stopped invocation settles");
    backend.fire(&env(1), &mut up, &mut below);
    assert!(below.iter().any(|effect| match effect {
        Below::Io(io::Request::Signal { signal: kernel::Signal::Terminate, .. }) => true,
        _ => false,
    }));
}
