//! Batch result caps below the guard's line (domain/session.md, section 8.1).

#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "programming-model.md, section 10.2: deterministic test harnesses use ordinary Rust"
)]

use super::*;

// Two 4 KiB reads at once, with 64 bytes as the smallest answer (TOOLS.path_bytes).
const READS: u32 = 2;
const SHARE: u64 = 512;
const MINIMUM: u64 = 64;
const PROMPT: u64 = 100;
const PROMPT_BYTES: u64 = 24; // model + system + opening text.

fn scene(calls: u32, room: u64, history_room: Option<u64>, usage: Usage) -> (Harness, Token, Completion) {
    let content: Box<[Block]> = (0..calls)
        .map(|index| tool_call(&[b'a'.checked_add(u8::try_from(index).unwrap()).unwrap()], cat(b"file")))
        .collect();
    let skeleton = u64::try_from(size_of::<Block>()).unwrap().checked_add(1).unwrap();
    let held = initial_bytes()
        .checked_add(crate::session::content_cost(&content).unwrap())
        .unwrap()
        .checked_add(u64::from(calls).checked_mul(skeleton).unwrap())
        .unwrap();
    let mut limits = LIMITS;
    if let Some(room) = history_room {
        limits.history_bytes =
            held.checked_add(room).unwrap().checked_add(u64::try_from(size_of::<Block>()).unwrap()).unwrap();
        // Admission must fit even when the concrete completion fills the history.
        limits.completion_bytes = crate::session::content_cost(&content).unwrap();
        limits.completion_blocks = calls;
        limits.answer_bytes = u64::from(calls);
    }
    let mut selected = spec();
    // Inverse of the design's 5/4 allowance, rounded up for this fixture.
    selected.line = PROMPT
        .checked_add(
            held.checked_sub(PROMPT_BYTES).unwrap().checked_add(room).unwrap().checked_mul(5).unwrap().div_ceil(4),
        )
        .unwrap();
    let mut harness = Harness::new(&limits);
    let (owner, _) = harness.open_with(1, selected);
    crate::reserve_completion(&mut harness.domain, owner, PROMPT_BYTES, 0, 1024).unwrap();
    let completion = Completion { usage, ..completion(content, Stop::ToolUse) };
    (harness, owner, completion)
}

fn reported() -> Usage {
    Usage { input_tokens: Some(PROMPT), output_tokens: Some(5), ..Usage::NONE }
}

fn finish_reads(harness: &mut Harness, runs: List<Token>) -> Prompt {
    let mut prompt = None;
    for run in &runs {
        let request = harness.step(ran(*run, &[b'x'; 4096]));
        if request.is_some() {
            prompt = Some(calling(request).1);
        }
    }
    prompt.expect("the batch issues the next completion")
}

fn result_caps(messages: &[Message]) -> Vec<u64> {
    messages
        .iter()
        .flat_map(|message| message.content.iter())
        .filter_map(|block| match block {
            Block::ToolResult {
                result: Returned::Owned { outcome: Outcome::Read { content, window, cut, .. } },
                ..
            } => {
                assert_eq!(
                    *window,
                    tools::BytesClamp::Clamped {
                        asked: TOOLS.read_bytes,
                        ceiling: u32::try_from(content.len()).unwrap()
                    }
                );
                assert_eq!(*cut, tools::Cut::Line);
                Some(u64::try_from(content.len()).unwrap())
            }
            Block::Opaque { .. }
            | Block::Refusal { .. }
            | Block::Text { .. }
            | Block::ToolCall { .. }
            | Block::ToolResult { .. } => None,
        })
        .collect()
}

#[test]
fn the_first_batch_shares_the_smaller_line_room() {
    let (mut harness, owner, completion) = scene(READS, u64::from(READS) * SHARE, None, reported());
    let runs = harness.batch(Event::Completed { owner, completion });
    let prompt = finish_reads(&mut harness, runs);
    assert_eq!(result_caps(&prompt.messages), [SHARE, SHARE]);
    ordinary(&prompt);
}

#[test]
fn a_later_batch_shares_what_the_first_left() {
    const CALLS: u32 = READS * 2;
    const ROOM: u64 = 2048;
    // The first pair leaves 64 per future call: (2048 - 128) / 2 = 960.
    const FIRST: u64 = 960;
    let (mut harness, owner, completion) = scene(CALLS, ROOM, None, reported());
    let runs = harness.batch(Event::Completed { owner, completion });
    // Finish the first pair without consuming the next pair's requests together.
    let mut runs = runs.into_boxed().into_iter();
    drop(harness.step(ran(runs.next().unwrap(), &[b'x'; 4096])));
    let later = harness.batch(ran(runs.next().unwrap(), &[b'x'; 4096]));
    let prompt = finish_reads(&mut harness, later);
    assert_eq!(result_caps(&prompt.messages), [FIRST, FIRST, MINIMUM, MINIMUM]);
    ordinary(&prompt);
}

#[test]
fn the_batch_shares_the_smaller_history_room() {
    const LINE_ROOM: u64 = 8192;
    let (mut harness, owner, completion) = scene(READS, LINE_ROOM, Some(u64::from(READS) * SHARE), reported());
    let runs = harness.batch(Event::Completed { owner, completion });
    let prompt = finish_reads(&mut harness, runs);
    assert_eq!(result_caps(&prompt.messages), [SHARE, SHARE]);
}

#[test]
fn without_a_reported_prompt_reads_keep_their_declared_caps() {
    let (mut harness, owner, completion) = scene(READS, u64::from(READS) * MINIMUM, None, Usage::NONE);
    let runs = harness.batch(Event::Completed { owner, completion });
    let prompt = finish_reads(&mut harness, runs);
    assert_eq!(prompt.purpose, crate::llm::Purpose::Ordinary { guard: None });
    let results: Vec<_> = prompt
        .messages
        .iter()
        .flat_map(|message| message.content.iter())
        .filter_map(|block| match block {
            Block::ToolResult {
                result: Returned::Owned { outcome: Outcome::Read { content, window, .. } }, ..
            } => Some((content.len(), *window)),
            Block::Opaque { .. }
            | Block::Refusal { .. }
            | Block::Text { .. }
            | Block::ToolCall { .. }
            | Block::ToolResult { .. } => None,
        })
        .collect();
    assert_eq!(results, [(4096, tools::BytesClamp::Within); 2]);
}

#[test]
fn the_line_holds_exact_minima_and_falls_back_one_byte_short() {
    for (room, expected) in [(u64::from(READS) * MINIMUM, MINIMUM), (u64::from(READS) * MINIMUM - 1, 4096)] {
        let (mut harness, owner, completion) = scene(READS, room, None, reported());
        let runs = harness.batch(Event::Completed { owner, completion });
        let prompt = finish_reads(&mut harness, runs);
        let lengths: Vec<_> = prompt
            .messages
            .iter()
            .flat_map(|message| message.content.iter())
            .filter_map(|block| match block {
                Block::ToolResult { result: Returned::Owned { outcome: Outcome::Read { content, .. } }, .. } => {
                    Some(u64::try_from(content.len()).unwrap())
                }
                Block::Opaque { .. }
                | Block::Refusal { .. }
                | Block::Text { .. }
                | Block::ToolCall { .. }
                | Block::ToolResult { .. } => None,
            })
            .collect();
        assert_eq!(lengths, [expected, expected], "room {room}");
    }
}

fn ordinary(prompt: &Prompt) {
    match prompt.purpose {
        crate::llm::Purpose::Ordinary { guard: Some(_) } => {}
        crate::llm::Purpose::Ordinary { guard: None } | crate::llm::Purpose::Compaction { .. } => {
            panic!("guarded ordinary completion")
        }
    }
}
