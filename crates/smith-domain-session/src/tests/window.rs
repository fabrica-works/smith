//! Measured window estimates and compaction room (domain/session.md, 8.1–8.2).

use super::*;

const CODEX_WINDOW: u32 = 272_000;
const CODEX_OUTPUT: u32 = 32_000;

#[test]
fn the_check_reserves_no_output_below_the_threshold() {
    use crate::window::{Admission, Tokens, Window, check};
    const PROMPT: u64 = 217_000;
    const LAST_OUTPUT: u64 = 224;
    const APPENDED_BYTES: u64 = 376;
    let mut window = Window::fresh(1, 0);
    window.completed(
        Usage { input_tokens: Some(PROMPT), output_tokens: Some(LAST_OUTPUT), ..Usage::NONE },
        CODEX_OUTPUT,
        2,
        PROMPT,
    );
    window.appended = Tokens(APPENDED_BYTES);
    assert_eq!(check(&window, 0, 0, LIMITS.token_bytes, CODEX_WINDOW, [4, 5]), Admission::Fits);
}

#[test]
fn appended_bytes_count_at_the_windows_measured_ratio() {
    use crate::window::{Tokens, Window, bound};
    const PROMPT: u64 = 152_588;
    const OUTPUT: u64 = 224;
    const APPENDED: u64 = 34_469;
    const PROMPT_BYTES: u64 = 534_058;
    let mut window = Window::fresh(1, 0);
    window.completed(
        Usage { input_tokens: Some(PROMPT), output_tokens: Some(OUTPUT), ..Usage::NONE },
        CODEX_OUTPUT,
        2,
        PROMPT_BYTES,
    );
    window.appended = Tokens(APPENDED);
    // PROMPT + OUTPUT + ceil(APPENDED / 3.5).
    assert_eq!(bound(&window, 0, 0, LIMITS.token_bytes), Some(162_661));
}

#[test]
fn a_ratio_below_one_byte_a_token_is_taken_at_one() {
    use crate::window::{Tokens, Window, bound};
    const PROMPT: u64 = 200;
    const OUTPUT: u64 = 10;
    const PROMPT_BYTES: u64 = 100;
    const APPENDED_BYTES: u64 = 31;
    let mut window = Window::fresh(1, 0);
    window.completed(
        Usage { input_tokens: Some(PROMPT), output_tokens: Some(OUTPUT), ..Usage::NONE },
        CODEX_OUTPUT,
        2,
        PROMPT_BYTES,
    );
    window.appended = Tokens(APPENDED_BYTES);
    assert_eq!(bound(&window, 0, 0, LIMITS.token_bytes), Some(241));
}

#[test]
fn a_ratio_above_token_bytes_is_taken_at_token_bytes() {
    use crate::window::{Tokens, Window, bound};
    const PROMPT: u64 = 100;
    const OUTPUT: u64 = 10;
    const PROMPT_BYTES: u64 = 1_600;
    const APPENDED_BYTES: u64 = 31;
    let mut window = Window::fresh(1, 0);
    window.completed(
        Usage { input_tokens: Some(PROMPT), output_tokens: Some(OUTPUT), ..Usage::NONE },
        CODEX_OUTPUT,
        2,
        PROMPT_BYTES,
    );
    window.appended = Tokens(APPENDED_BYTES);
    // 100 + 10 + ceil(31 / 8).
    assert_eq!(bound(&window, 0, 0, LIMITS.token_bytes), Some(114));
}

#[test]
fn without_a_ratio_the_bound_counts_payload_bytes_and_the_allowance() {
    const ALLOWANCE: u64 = 100;
    const WINDOW: u32 = 155;
    const OUTPUT: u32 = 16;
    let mut selected = spec();
    selected.protocol_allowance = ALLOWANCE;
    // (model 5 + system 8 + prompt 11 + ALLOWANCE) / threshold 4/5.
    selected.window = WINDOW;
    selected.output = OUTPUT;
    let mut harness = Harness::new(&Limits { protocol_allowance: ALLOWANCE, ..LIMITS });
    let (_, prompt) = harness.open_with(1, selected);
    assert_eq!(&*prompt.messages[0].content, &[Block::Text { text: bytes(b"fix the bug"), replay: None }]);
}

#[test]
fn a_compactions_room_keeps_no_output_for_its_summary() {
    const PROMPT: u64 = 200_000;
    const OUTPUT: u64 = 2_000;
    let mut selected = spec();
    selected.window = CODEX_WINDOW;
    selected.output = CODEX_OUTPUT;
    let mut answer = done();
    answer.usage = Usage { input_tokens: Some(PROMPT), output_tokens: Some(OUTPUT), ..Usage::NONE };
    let mut harness = Harness::new(&LIMITS);
    let owner = harness.yielded_with(selected, answer);
    let (_, prompt) = calling(harness.step(Event::Continue { session: owner, content: bytes(b"continue") }));
    let (_, summary) = calling(harness.step(Event::Failed {
        retry_ok: true,
        owner,
        failure: Failure::ContextTooLong,
        evidence: crate::llm::Evidence::Unsent,
        detail: bytes(b"past the line"),
    }));
    assert_eq!(
        prompt.purpose,
        crate::llm::Purpose::Ordinary {
            guard: Some(crate::llm::Guard { line: 64_512, last_prompt: 200_000, from: 1 })
        }
    );
    // 272_000 - 200_000 - 2_000 - 26-byte summary instruction.
    assert_eq!(summary.purpose, crate::llm::Purpose::Compaction { room: 69_974 });
}

#[test]
fn a_compactions_room_past_the_threshold_keeps_the_window() {
    const PROMPT: u64 = 230_000;
    const OUTPUT: u64 = 2_000;
    let mut selected = spec();
    selected.window = CODEX_WINDOW;
    selected.output = CODEX_OUTPUT;
    let mut answer = done();
    answer.usage = Usage { input_tokens: Some(PROMPT), output_tokens: Some(OUTPUT), ..Usage::NONE };
    let mut harness = Harness::new(&LIMITS);
    let owner = harness.yielded_with(selected, answer);
    let (_, summary) = calling(harness.step(Event::Continue { session: owner, content: bytes(b"continue") }));
    // 272_000 - 230_000 - 2_000 - 26-byte summary instruction.
    assert_eq!(summary.purpose, crate::llm::Purpose::Compaction { room: 39_974 });
}
