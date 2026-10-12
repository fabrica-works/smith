//! Actual requests and compaction turns authorise whole-window replacement.
//! This referee knows the opener's window and accepted usage, never domain state.

use super::{Block, Prompt, Session, agent};

/// Check two actual requests of one window (domain/session.md, sections 3 and 10).
pub fn assert_prompt_extends(previous: &Prompt, current: &Prompt) {
    assert_prompt_boundary(previous, current, None);
}

/// A replacement needs the exact opening observed in a compaction turn.
pub fn assert_prompt_boundary(previous: &Prompt, current: &Prompt, opening: Option<&[agent::llm::Message]>) {
    assert_eq!(current.affinity, previous.affinity, "a window keeps its affinity");
    assert_eq!(current.system, previous.system, "a window keeps its system text");
    assert_eq!(current.tools, previous.tools, "a window keeps its tool grants");
    assert_eq!(current.delegated, previous.delegated, "a window keeps its tool descriptors");
    if let Some(opening) = opening {
        assert!(current.messages.starts_with(opening), "a window begins with its recorded opening");
    } else {
        assert!(current.messages.starts_with(&previous.messages), "a window extends its history");
    }
}

/// No actual request may exceed the supplied model window.
pub fn assert_prompt_bound(bound: u64, window: u32) {
    assert!(bound <= u64::from(window), "no prompt whose bound exceeds its window");
}

pub(super) fn prompt(session: &mut Session, prompt: &Prompt) {
    if let Some(previous) = &session.previous_prompt {
        if session.compacting.is_some() {
            assert_compaction_prompt(previous, prompt, session);
        } else if session.opening.is_some() {
            assert_prompt_boundary(previous, prompt, session.opening.as_deref());
        } else {
            assert_prompt_extends(previous, prompt);
        }
    }
    session.opening = None;
    if session.compacting.is_none() {
        let bytes = prompt.input_bytes().expect("bounded prompt");
        let input = match session.ordinary {
            Some((usage, following)) if usage.input_tokens.is_some() => {
                let mut prefix = prompt.clone();
                prefix.messages = prompt.messages[..following].into();
                let appended = &prompt.messages[following..];
                let cells = appended
                    .iter()
                    .map(|message| {
                        std::mem::size_of::<agent::llm::Message>()
                            + message.content.len() * std::mem::size_of::<Block>()
                    })
                    .sum::<usize>();
                usage.input_tokens.unwrap_or(0)
                    + usage.cache_read_tokens.unwrap_or(0)
                    + usage.cache_write_tokens.unwrap_or(0)
                    + usage.output_tokens.unwrap_or(u64::from(session.max_tokens))
                    + bytes
                    - prefix.input_bytes().expect("bounded prefix")
                    - u64::try_from(cells).expect("small cells")
            }
            _ => session.summary.map_or(bytes, |(summary_bytes, tokens)| bytes - summary_bytes + tokens),
        };
        assert_prompt_bound(input + u64::from(prompt.max_tokens), session.window);
    }
    session.previous_prompt = Some(prompt.clone());
}

pub(super) fn recorded_window(session: &mut Session, turn: &agent::record::Turn, index: usize) {
    assert!(session.compacting.is_some(), "a window boundary follows one compaction");
    assert_eq!(turn.messages.len(), 1);
    assert_eq!(turn.messages[0].role, agent::llm::Role::User);
    let text: Vec<u8> = session.accepted_content[index]
        .iter()
        .flat_map(|block| match block {
            Block::Text { text, .. } => text.to_vec(),
            Block::Opaque { .. } | Block::Refusal { .. } | Block::ToolCall { .. } | Block::ToolResult { .. } => {
                Vec::new()
            }
        })
        .collect();
    assert_eq!(turn.messages[0].content[1], Block::Text { text: text.clone().into(), replay: None });
    session.compacting = None;
    session.opening = Some(turn.messages.clone());
    session.ordinary = None;
    session.summary = Some((
        u64::try_from(text.len()).expect("small summary"),
        turn.usage.output_tokens.unwrap_or(u64::from(session.max_tokens)),
    ));
}

fn assert_compaction_prompt(previous: &Prompt, current: &Prompt, session: &Session) {
    let following = session.ordinary.map_or(0, |(_, following)| following);
    let mut preserved = previous.clone();
    preserved.messages = previous
        .messages
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
            let mut message = message.clone();
            if index >= following && message.role != agent::llm::Role::Assistant {
                message.content =
                    message.content.into_iter().filter(|block| !matches!(block, Block::Text { .. })).collect();
            }
            (!message.content.is_empty()).then_some(message)
        })
        .collect();
    assert_prompt_extends(&preserved, current);
    assert_eq!(
        current.messages.last().expect("summary request").content,
        Box::new([Block::Text { text: session.summary_instruction.clone(), replay: None }]) as Box<[Block]>
    );
}
