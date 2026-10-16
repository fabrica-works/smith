# Session 07: the window's check, an estimate of the next prompt

2026-10-16. Cause 2 of `findings/causes.md`: smith compacted in 14 of
its 15 `shell-cwd` attempts over the parity plan's three runs, at real
prompts of 120,000 to 172,000 tokens, 44 to 63% of its 272,000 window,
for 74 seconds an attempt and re-reads after. The check fired because it
counts each appended byte as a token and reserves a whole completion's
32,000 output tokens below the threshold (findings A1): attempt 49 counted
219,281 against 217,600. The design makes the check an estimate of the
next prompt, appended bytes at the window's measured ratio, with no
output reserved below the threshold; the guard (smith 04) bounds what
the estimate misses. This step changes when smith compacts, so it is
proven by its `parity-change` run (benchmarks.md 16.6).

## Read first

- smith `docs/design/domain/session.md`, sections 8.1 (what the session
  tracks, the appended count, the check, the line), 8.2 (the
  compaction's results), 10 (the appended count's stories, the
  invariants, the fuzzy suite).
- smith `docs/design/protocol/limits.md`, sections 2.1 ("A window is a
  prompt's"), 3 (rule 4), 3.9 (the room; the compaction's room).
- smith `docs/design/benchmarks.md`, section 16.6.
- `decisions.md`, section 2; `findings/causes.md`, "The compaction
  trigger"; `findings/planning-audit.md` A1 to A4.
- skein `programming-model.md`, sections 3 and 4; `testing-strategy.md`,
  sections 3 and 5.

## What the design asks

- **What the session tracks:** the last prompt's size from its usage;
  that completion's output as reported, or the model's most output; the
  payload bytes the session counts in that prompt; and those it counts
  in what was appended since. One measure throughout: each block's
  payload bytes as the session holds them, never their cells, recounted
  after a resume (8.1).
- **The appended count:** those appended bytes in tokens at the window's
  measured ratio, the last prompt's counted bytes over its reported
  size, at least one byte a token and at most `TOKEN_BYTES`. Without a
  ratio (a window's first completion, the first after a resume from a
  window turn, usage with no input count), the whole prompt's bytes plus
  the allowance, a byte a token, as today (8.1).
- **The check:** the last prompt, plus its output, plus the appended
  count, at most the window times its threshold. No output is reserved
  below the threshold (8.1).
- **A compaction's results** are rendered within the window less the
  history before them and the summary instruction; its summary is its
  output, beside its prompt (8.2; limits.md 3.9).
- **The stories** (10): a window whose prompts the provider counts at
  several bytes a token, then a turn whose results it counts at one
  byte a token, admitted by the check and refused unsent by the guard,
  then compacted within its window; a turn whose new part measures just
  within the line, sent; a completion of the most output just within
  the line, then its compaction fitting; a ratio below one byte a token,
  or above `TOKEN_BYTES`, taken at that bound. **The invariants:** no
  prompt, a compaction's included, whose reported size passes its
  window; no ordinary completion made whose check passes the threshold;
  none sent whose reported prompt passes the line.

## Choices this plan makes

- The ratio is kept as the two numbers it is made of, the last prompt's
  counted bytes and reported tokens, and the appended count is
  `ceil(appended_bytes * last_prompt_tokens / last_prompt_bytes)`,
  clamped to between `ceil(appended_bytes / TOKEN_BYTES)` and
  `appended_bytes`, in checked arithmetic.

## Rules that bind it

- **One change at a time** (README, rule 11): this step lands after
  06.1's summary is on `main`, and nothing else that changes a prompt
  lands until it has.
- **The stop** (README, rule 12): `suite parity-change` exiting 1 or 2
  stops the step on a question.
- **No split** (README, rule 1): past 1.5 times its Size, it asks.

## Increments

### 07.1 The check estimates the next prompt
Scope: session.md 8.1 and 8.2's results; limits.md 2.1 and 3.9's
  compaction room; session.md 10's appended-count stories and invariants
  (findings A1 to A4; cause 2).
Needs: the baseline's summary and the change pipeline's small run on
  `main` [06.1]; smith 04's world counting tokens from bytes [04.2];
  smith 03's `history` condition at the line [03.3].
Files: `crates/smith-domain-session/src/window.rs` (`Window`, `:23-33`;
  `completed`, `:55-62`; `check`, `:65-77`; `bound`, `:88-99`),
  `crates/smith-domain-session/src/compaction.rs` (`due`, `:66-84`;
  `appended_bytes`, `:86-94`; `prompt_bytes`, `:97-113`),
  `crates/smith-domain-session/src/session.rs` (`:2202-2222`, the
  compaction's room, its cells at `:2203-2207`; `receive_completion`'s
  `window.completed`, `:2519-2523`; `spec_cost`, `:2547-2548`, and
  `choice_bytes`, `:753-761`, as `prompt_bytes` counts them),
  `compaction.rs`'s resume (`restore`, `:426`; `Window::fresh`, `:457`;
  its `completed`, `:467-471`), `crates/smith-domain-session/src/limits.rs`
  (a `token_bytes` field beside `threshold`, `:26`, `:221`, `:269`), set
  by `crates/smith-agent-service/src/profile.rs` (`:550`) from
  `shared::TOKEN_BYTES`, as `line_bytes` is at `:448`, and the other
  `Limits` literals (`crates/smith-domain/src/tests.rs:109`,
  `crates/smith-domain-session/src/tests.rs:59`,
  `tests/session/src/world.rs:207`), its unit tests (`:2912`; the three
  window tests at `:2912-2947`, and `a_compaction_carries_no_guard`,
  `:3023`, whose room becomes 65,490),
  `tests/session/src/window_referee.rs` (`:30-64`),
  `tests/session/tests/compaction.rs` (its `usage()`, `:9-11`, takes
  `input_tokens: Some(1_300)` so that it still compacts),
  `tests/session/tests/fuzzy_simulation.rs` (`:114`, the `+512` at
  `:135`), `tests/agent/tests/compaction.rs` (`data.txt`, `:47`) and
  `tests/agent/tests/memory_returns.rs` (`fill_window`, `:69`, `:77`),
  whose sizes grow until they compact as before (the check found 3,700
  bytes, and `0..7` with `turn < 6`, on its own sketch), and
  `benchmarks/summaries/` (its `parity-change` summary).
Shape: `Window` gains `last_prompt_bytes: u64`, the payload bytes counted
  in the prompt `completed` records. `bound` adds `last_prompt +
  last_output + appended`, where `appended` is the appended count above,
  and no longer adds `output`. `prompt_bytes`, the bound without a
  ratio, counts payload bytes, not `size_of::<Prompt>()`, `Message`,
  `Block`, `Descriptor` (`spec_cost`) or `Token` (`choice_bytes`) cells.
  The session crate does not depend on skein-llm: the clamp's
  `TOKEN_BYTES` is the session `Limits`' `token_bytes`, set by the
  service. The compaction's room is the window less its history
  before the owed results (the last prompt and its output, or, without
  a ratio, the counted payload bytes and the allowance, with no
  `content_cost` or `size_of::<Message>()` cells, `session.rs:2203-2207`)
  and less `instruction_bytes`: no `min` with the threshold, no floor,
  and no `output` subtracted (limits.md 3.9). After a resume the counted bytes are recounted
  from the loaded history. The referee asserts each reported prompt
  within its window (no `max_tokens` added) and each ordinary completion
  made within the threshold as 8.1 counts it: it walks the request's
  payload bytes itself for the ratio, in place of the cells it subtracts
  today (`window_referee.rs:47-53`, `:60`); `Prompt::input_bytes`
  (`llm.rs:294`) stays, for `route.rs:701`. The fuzzy test's `+512`
  goes. The profile's startup room (`smith-agent-service/src/profile/room.rs`,
  two outputs below the threshold, `:14`, `:78`, `:98`) stays as it is.
Test shape: `crates/smith-domain-session/src/tests.rs`, beside
  `the_check_counts_reported_usage` (`:2912`):
  `the_check_reserves_no_output_below_the_threshold`,
  `appended_bytes_count_at_the_windows_measured_ratio`,
  `a_ratio_below_one_byte_a_token_is_taken_at_one`,
  `a_ratio_above_token_bytes_is_taken_at_token_bytes`,
  `without_a_ratio_the_bound_counts_payload_bytes_and_the_allowance`,
  `a_compactions_room_keeps_no_output_for_its_summary`,
  `a_compactions_room_past_the_threshold_keeps_the_window` (a last
  prompt of 230,000 and an output of 2,000 on the standard Codex model,
  with the unit harness's 26-byte instruction, `tests.rs:307`: a room of
  39,974, so the compaction's prompt stays within 272,000);
  each with its
  numbers as literals (attempt 49's: 152,588 reported, 224 output,
  34,469 bytes appended at a ratio of 3.5 bytes a token: 162,661, which
  fits 217,600). `tests/session/tests/compaction.rs`, beside
  `a_continue_past_the_threshold_compacts_and_holds_its_message_back`
  (`:117`): `a_turn_counted_a_byte_a_token_is_refused_by_the_guard_then_compacted`,
  `a_new_part_just_within_the_line_is_sent`,
  `the_most_output_just_within_the_line_then_compacts_within_the_window`.
  Unit tests under 20 ms; world tests under 0.5 s; the fuzzy test about
  5 s.
Proof: live, through the stop: `heavy cargo run -p smith-bench -- build
  HEAD` and `build origin/main`, then `heavy --gate cargo run -p
  smith-bench -- suite parity-change --arm <HEAD's dir> --parent-arm
  <origin/main's dir> --baseline-run <the file README rule 13's first
  command prints> --landed <each file its second prints>`: 40 attempts,
  about 3 hours, run once; its exit decides (README, rule 12). Its
  summary is amended into the step's commit and named in
  `Change-Summary:`; the commit body gives its criteria, projection and
  stop lines, README rule 13's counts beside, and names cause 2.
Don't: change the threshold, the window or `output`; count cells
  anywhere in the check.
Size: 150 lines.
Removes: the check's output reservation; the cells in `prompt_bytes`.

## Questions to raise when reached

- None known.

## Done when

- `cd ~/src/rust/smith && git log origin/main --grep='^Step: smith-07.1$' --format='%(trailers:key=Change-Summary,valueonly)' | grep -c .` prints 1, or `awk -F'\t' '$1 == "smith-07.1" { print $4 }' ~/src/rust/tmp/smith/wall-clock/ledger.tsv` prints `dropped`.
