//! Negative controls for the session's append-only window contract (domain/session.md, section 10).
use smith_domain_session::llm::{Block, Endpoint, Message, Prompt, Role};
use smith_session_world::assert_prompt_extends;

fn prompt() -> Prompt {
    Prompt {
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
