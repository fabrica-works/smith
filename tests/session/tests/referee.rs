//! Negative controls for the session's append-only window contract (domain/session.md, section 10).
use smith_domain_session::llm::{Endpoint, Prompt, Message, Block, Role};
use smith_session_world::assert_prompt_extends;

fn prompt() -> Prompt {
    Prompt { endpoint: Endpoint(0), model: b"model".as_slice().into(), system: b"system".as_slice().into(), tools: smith_domain_tools::Grants { inspect: true, modify: false, shell: false },
        delegated: Box::new([]), messages: Box::new([Message { role: Role::User, content: Box::new([Block::Text { text: b"first".as_slice().into(), replay: None }]) }]), max_tokens: 1 }
}

#[test]
#[should_panic(expected = "a window keeps its system text")]
fn changed_window_system_is_rejected() {
    let mut changed = prompt(); changed.system = b"different".as_slice().into();
    assert_prompt_extends(&prompt(), &changed);
}

#[test]
#[should_panic(expected = "a window extends its history")]
fn edited_window_history_is_rejected() {
    let mut changed = prompt(); changed.messages[0].content = Box::new([]);
    assert_prompt_extends(&prompt(), &changed);
}
