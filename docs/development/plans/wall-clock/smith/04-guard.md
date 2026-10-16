# Session 04: the guard

2026-10-16. The window's check is to become an estimate (07.1), so
something must bound what it misses. The design puts the bound in the
protocol layer: before an ordinary request with an earlier prompt in its
window is sent, the layer measures the request's new part as it renders
it, a byte a token, and refuses the request unsent, as *context too
long*, when the last prompt's reported size plus that measure passes the
line. Today the domain's prompt carries nothing a guard needs, skein
gives only a whole request's size, and nothing produces an unsent
*context too long* (findings A6, A7). This session builds the guard and
the session world's provider that counts tokens from bytes. With
today's check no prompt reaches the line, so the guard changes no
request the suites make; it lands before the baseline, so that the
baseline measures it (decisions.md, 12.1).

## Read first

- smith `docs/design/domain/session.md`, sections 4 ("A request past
  the line"), 8.1 ("The line holds a compaction"), 8.2 (its trigger),
  10 ("The provider counts tokens from bytes", the fuzzy suite, the
  invariants).
- smith `docs/design/protocol/llm.md`, section 6 ("The guard") and its
  tests' "the guard" bullet; `protocol/limits.md`, sections 3.2 (the
  prompt's allowance) and 3.9 ("The line").
- `decisions.md`, sections 2.4, 2.5 and 12; `findings/planning-audit.md`
  A4 to A7.
- skein `programming-model.md`, sections 3 and 4 (step machines);
  `testing-strategy.md`, sections 3 and 5 (worlds, the referee).

## What the design asks

- **The session gives the guard** with each ordinary completion that has
  an earlier prompt in its window: the line, the last prompt's reported
  size, and where that completion's answer starts in the history
  (session.md 8.1).
- **The protocol layer measures and refuses:** the last answer and every
  block after it, as it renders them, a byte a token; past the line, the
  request is not sent, and the call fails as *context too long*, with
  evidence `unsent`, its detail naming the line and the measure. The
  same rendering serves the measure and the request (llm.md 6).
- **The session takes it as *context too long*:** one compaction; a
  second in the same window fails the session (session.md 4).
- **The worlds:** the session world's provider gives each block a
  rendered size, drawn at least its counted bytes, counts its tokens at
  a ratio drawn between one byte a token and `TOKEN_BYTES`, and applies
  the guard from that size; the fuzzy suite draws them, and no request
  sent passes the line (session.md 10). The protocol world: a request
  within the line sent; one a byte past it failed, unsent, naming the
  line; short lines and escaped bytes measured as they render (llm.md's
  tests).

## Choices this plan makes

- The guard's measure is the request's encoded body, less the encoded
  body of the same request with its messages cut where the new part
  starts, both through skein's `client::measure` (`client.rs:312-325`),
  which the request's own encoding uses: a block's framing is counted as
  encoded, a byte a token (decisions.md 12.4, question 3).
- `Purpose::Ordinary` carries the guard, so the domain's prompt and its
  re-export in `smith-domain` change in place, and a compaction carries
  none.

## Rules that bind it

- **Users outside Files** (README, rule 3): `Purpose::Ordinary` becomes
  `Purpose::Ordinary { guard }`; each construction outside Files takes
  `guard: None`.
- **No second mechanism** (`code-organization.md`, section 7): the guard
  is the only check of a request's rendered size against the line.
- **The smoke** (README, rule 8): the guard runs on each of its ordinary
  requests after the first.

## Increments

### 04.1 The guard
Scope: session.md 4 and 8.1 ("the guard"); llm.md 6 ("The guard") and
  its tests' guard bullet (findings A6, A7).
Needs: smith 03's line in each conversation [03.1].
Files: `crates/smith-domain-session/src/llm.rs` (`Purpose`, `:242-247`;
  `Prompt`, `:252-276`), `crates/smith-domain-session/src/session.rs`
  (`complete`, `:2196-2254`, the purpose at `:2221`),
  `crates/smith-domain-session/src/window.rs` (`following`, read),
  `crates/smith-domain/src/llm.rs` (`:20-23`),
  `crates/smith-domain/src/peer.rs` (`:245-289`),
  `crates/smith-protocol-llm/src/prompt.rs` (`:76`, `:143-146`),
  `crates/smith-protocol-llm/src/component.rs` (`start`, `:384-442`),
  `crates/smith-protocol-llm/src/failure.rs` (`refusal`, `:70-84`), a
  new `crates/smith-protocol-llm/src/guard.rs`,
  `crates/smith-domain-session/src/tests.rs`, `tests/protocol-llm/`,
  `tests/session/tests/compaction.rs`, `tests/session/src/recorded.rs`.
Shape: the sketch below. `complete` sets `guard: Some(..)` for an
  ordinary completion whose window has a reported last prompt, with the
  conversation's line (03.1), that prompt's size, and `from` at the
  index where the last answer starts (`following - 1`); `None` only for
  a window with no reported last prompt: a window's first completion,
  and the first after a resume from a window turn. A resume from an
  ordinary turn restores the last prompt (`compaction.rs:468-473`), and
  its next completion carries the guard. In `start`, after
  the request is prepared and its output ceiling set, and before it is
  observed and submitted, `guard::measure` gives the new part's bytes;
  when `last_prompt + measure > line`, `start` answers through
  `refusal` with `Failure::ContextTooLong`, evidence `Unsent`, detail
  `"past the line: last prompt <p> + new part <m> > line <l>"`, and
  submits nothing. The session's `context_failure` already takes it as
  *context too long*.
Test shape: `tests/protocol-llm/tests/`, beside
  `a_derived_history_at_its_bound_is_sent_and_one_byte_more_is_refused_typed`
  (`history_window.rs:14`), the reference to copy:
  `a_request_within_the_line_is_sent`,
  `a_request_one_byte_past_the_line_fails_unsent`,
  `the_guards_failure_names_the_line_and_the_measure`,
  `short_lines_are_measured_as_they_render`,
  `escaped_bytes_are_measured_as_they_render` (control bytes, each
  rendered `\u00XX`, so the measure is six times their count),
  `a_request_without_a_guard_is_sent_unmeasured`, each in both dialects
  as rows of one table where the case is one behaviour. In
  `crates/smith-domain-session/src/tests.rs`:
  `an_ordinary_completion_after_a_prompt_carries_the_guard`,
  `a_windows_first_completion_carries_no_guard`,
  `a_compaction_carries_no_guard`,
  `a_resumed_ordinary_window_carries_the_guard`. In `tests/session/tests/compaction.rs`,
  beside `a_context_too_long_compacts_a_used_window` (`:141`):
  `the_guards_refusal_compacts_the_window_once`. Limits from the
  standard profile; the line set by its test window as 03.1 derives it.
  Unit tests under 20 ms; world tests under 0.5 s.
Proof: process: the smoke (README, rule 8) passes with the guard on its
  requests, and its stream has no `response.failed` of class
  `context_too_long`: `rg -c '"context_too_long"'
  ~/.local/state/smith-bench/runs/<the smoke's run>/` prints nothing, in
  the commit body. The suite's tests are the joint
  and world layers; no story can reach the line in a live run with
  today's check.
Don't: estimate the new part from the session's counts; refuse on the
  provider's side; add a refusal class.
Size: 200 lines.
Removes: nothing.

```rust
// crates/smith-domain-session/src/llm.rs
/// Why a completion is asked: an ordinary turn, or a compaction with its room.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Purpose {
    /// An ordinary completion; `guard` is set when its window has a reported last prompt.
    Ordinary { guard: Option<Guard> },
    /// A compaction, its owed results rendered within `room`.
    Compaction { room: u64 },
}

/// What the protocol layer checks before it sends an ordinary request (domain/session.md, 8.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Guard {
    /// The conversation's line, in tokens.
    pub line: u64,
    /// The window's last prompt, as its usage reported it, in tokens.
    pub last_prompt: u64,
    /// The index in the prompt's messages where the last answer starts.
    pub from: usize,
}

// crates/smith-protocol-llm/src/guard.rs
/// The new part's encoded bytes: the whole request's body less the body cut at `from`.
pub(crate) fn measure(request: &skein_llm::Prompt, from: usize) -> Result<u64, Failure>;
```

### 04.2 The session world counts tokens from bytes
Scope: session.md 10 ("The provider counts tokens from bytes", the
  fuzzy suite, "none sent passes the line") (findings A4, A5).
Needs: 04.1's guard [04.1].
Files: `tests/session/src/world.rs` (its LLM effect),
  `tests/session/src/translate.rs` (`:160-169`),
  `tests/session/src/window_referee.rs` (`:30-64`),
  `tests/session/tests/fuzzy_simulation.rs`,
  `tests/session/tests/simulation.rs`.
Shape: the world's LLM effect draws, for each block of a request, a
  rendered size between its counted bytes and six times them (every
  byte escaped) and a ratio of one to
  `TOKEN_BYTES` bytes a token, and reports the request's input as the
  sum of each block's rendered size over its ratio, rounded up; for a
  request with a guard, it applies the guard from those rendered sizes
  and answers past the line as the protocol layer does. The referee
  adds: no ordinary request sent whose reported prompt passes its line.
  Its other assertions stay until 07.1.
Test shape: `tests/session/tests/simulation.rs`:
  `the_worlds_provider_counts_blocks_at_their_drawn_ratios`,
  `a_guarded_request_past_the_line_is_refused_by_the_world`.
  `tests/session/tests/fuzzy_simulation.rs`:
  `drawn_ratios_and_rendered_sizes_send_nothing_past_the_line`, a sweep
  of seeds. Focused tests under 0.5 s; the fuzzy one about 5 s.
Proof: world, those tests: the world is the highest layer at which
  ratios can be drawn; 07.1's stories run on it.
Don't: change a production crate.
Size: none (test code).
Removes: nothing.

## Questions to raise when reached

- None known; decisions.md 12.4's question 3 is answered in the design
  (smith `f1b33a17`).

## Done when

- `cd ~/src/rust/smith && rg -n 'pub struct Guard' crates/smith-domain-session/src/llm.rs` prints one line.
- `cd ~/src/rust/smith && heavy cargo nextest run -p smith-protocol-llm-world -p smith-session-world -E 'test(a_request_one_byte_past_the_line_fails_unsent) | test(the_guards_refusal_compacts_the_window_once) | test(a_guarded_request_past_the_line_is_refused_by_the_world)'` runs three tests per dialect row, and they pass.
- `cd ~/src/rust/smith && heavy cargo nextest run --profile fuzzy -p smith-session-world -E 'test(drawn_ratios_and_rendered_sizes_send_nothing_past_the_line)'` passes.
