#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "programming-model.md section 10.2: deterministic test harnesses use ordinary Rust"
)]

use super::*;

fn builtin() -> Said {
    Said::ToolCall {
        id: bytes(b"read"),
        name: bytes(b"Read"),
        input: bytes(br#"{"file_path":"file"}"#),
        call: Decoded::Builtin { effect: tools::Effect::Read },
        replay: None,
    }
}

#[test]
fn a_builtin_result_reaches_the_session() {
    let mut harness = Harness::new();
    let (_, owner, _) = harness.admit(7, charter());
    assert!(harness.answer(owner, Box::new([builtin()])).is_empty());
    let (again, _) = completing(harness.next());
    assert_eq!(again, owner);
    assert!(harness.turns.is_empty());
    assert!(
        harness
            .step(Event::BuiltinResult { owner, id: bytes(b"read"), text: bytes(b"contents"), error: false })
            .is_empty()
    );
    assert_eq!(harness.turns.len(), 1);
    let turn = harness.turns.last().unwrap();
    let results: Vec<_> = turn
        .messages
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            session::llm::Block::ToolResult { id, result } => Some((id.as_ref(), result)),
            session::llm::Block::ToolCall { .. }
            | session::llm::Block::Text { .. }
            | session::llm::Block::Refusal { .. }
            | session::llm::Block::Opaque { .. } => None,
        })
        .collect();
    assert_eq!(
        results,
        vec![(
            b"read".as_slice(),
            &session::llm::Returned::Text { text: bytes(b"contents"), error: false, replay: None }
        )]
    );
}

#[test]
fn a_builtin_result_for_a_gone_session_is_dropped() {
    let mut harness = Harness::new();
    let (run, owner, _) = harness.admit(7, charter());
    assert!(harness.answer(owner, Box::new([builtin()])).is_empty());
    drop(completing(harness.next()));
    assert!(harness.step(Event::Cancel { run }).is_empty());
    let cancelled = harness.next();
    assert!(cancelled.iter().any(|request| request == &Request::Cancel { owner }));
    drop(harness.step(Event::Cancelled { owner }));
    drop(harness.next());
    assert_eq!(harness.domain.sessions.len(), 0);
    harness.keep_facts();
    let count = harness.turns.len();
    assert!(
        harness.step(Event::BuiltinResult { owner, id: bytes(b"read"), text: bytes(b"late"), error: false }).is_empty()
    );
    assert_eq!(harness.turns.len(), count);
    assert_eq!(harness.domain.pop_fact(), None);
}
