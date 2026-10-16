//! Negative controls for the session's append-only window contract (domain/session.md, section 10).
use smith_domain_session::llm::{Block, Endpoint, Message, Prompt, Role};
use smith_session_world::assert_prompt_extends;

fn prompt() -> Prompt {
    Prompt {
        effort: None,
        purpose: smith_domain_session::llm::Purpose::Ordinary { guard: None },
        choice: smith_domain_session::llm::ToolChoice::Auto,
        protocol_allowance: 0,
        namespace: smith_domain_tools::Namespace::One,
        affinity: smith_domain_session::llm::Affinity { key: [0; 16], thread: 0 },
        endpoint: Endpoint(0),
        model: b"model".as_slice().into(),
        system: b"system".as_slice().into(),
        tools: smith_domain_tools::Grants { inspect: true, modify: false, shell: false },
        delegated: Box::new([]),
        messages: Box::new([Message {
            role: Role::User,
            content: Box::new([Block::Text { text: b"first".as_slice().into(), replay: None }]),
        }]),
        max_tokens: 1,
    }
}

#[test]
#[should_panic(expected = "a window keeps its system text")]
fn changed_window_system_is_rejected() {
    let mut changed = prompt();
    changed.system = b"different".as_slice().into();
    assert_prompt_extends(&prompt(), &changed);
}

#[test]
#[should_panic(expected = "a window extends its history")]
fn edited_window_history_is_rejected() {
    let mut changed = prompt();
    changed.messages[0].content = Box::new([]);
    assert_prompt_extends(&prompt(), &changed);
}

#[test]
#[should_panic(expected = "a window keeps its affinity")]
fn changed_window_affinity_is_rejected() {
    let previous = prompt();
    let mut changed = prompt();
    changed.affinity.thread = 1;
    assert_prompt_extends(&previous, &changed);
}

#[test]
#[should_panic(expected = "a window extends its history")]
fn a_window_boundary_without_a_compaction_is_rejected() {
    let previous = prompt();
    let mut changed = prompt();
    changed.messages[0].content = Box::new([Block::Text { text: b"summary".as_slice().into(), replay: None }]);
    smith_session_world::assert_prompt_boundary(&previous, &changed, None);
}

#[test]
#[should_panic(expected = "no prompt whose bound exceeds its window")]
fn a_prompt_past_its_window_is_rejected() {
    smith_session_world::assert_prompt_bound(1_001, 1_000);
}
