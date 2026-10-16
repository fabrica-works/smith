# Session 03: the line, and the history it bounds

2026-10-16. The design now holds each prompt sent within a line: the
window less one `output`, the summary instruction and a cut result's
marker for each of `calls_per_response` calls, below which the
compaction after any prompt always fits. Between the threshold and the
line lies the spare. Nothing in the code computes either: the summary
instruction's size and the marker's reach only the room's check
(findings A8); the charter's entrance checks the room, not the spare
(A9); the history's bytes are derived from the threshold, not the line
(A10); and the `history` probe judges a compaction due by the old
reservation (A13). This session derives the line, checks the spare at
the entrance, gives each conversation its line, moves the event
vocabulary to version 3, derives the history from the line, raises
`memory` if what that adds passes it, and moves the `history` probe's
condition to the line. It changes nothing the model is sent: with today's check,
no prompt reaches the line.

## Read first

- smith `docs/design/protocol/limits.md`, sections 2.1, 3 (rule 4), 3.2
  (the history's bytes; the prompt's allowance), 3.9, 4
  (`compaction-headroom`, `window-first`), 5, 9.
- smith `docs/design/protocol/charter.md`, sections 6 and 8.
- smith `docs/design/protocol/events.md`, sections 3.2 and 6.
- smith `docs/design/shell.md`, sections 6.1 (the room and the spare)
  and 6.3 (`memory`).
- smith `docs/design/benchmarks.md`, section 11.1 (the `history` row).
- `decisions.md`, sections 2.4, 2.6, 9 and 12; `findings/planning-audit.md`
  A8 to A13; `findings/planning-commands.md` (the memory margin at the
  base).
- skein `programming-model.md`, sections 6.1 and 6.3.

## What the design asks

- **The line** is a model's `window` less one `output`, the summary
  instruction and a marker for each of `calls_per_response` calls; **the
  spare** is what lies between the threshold and the line. A charter's
  effective `window` and `output` give the line and the spare its
  session uses (limits.md 3.9).
- **The entrance refuses a negative spare:** each model entry's window
  above its threshold against one `output`, the summary instruction and
  the markers (charter.md 6 and 8; limits.md 4).
- **The history's bytes** are the line's tokens at `TOKEN_BYTES` bytes
  each, plus one completion's whole reserve, its answer included, plus
  one batch's results; `window-first` checks it (limits.md 3.2, 4).
- **Room is made by raising the cap:** a change whose worst case passes
  `memory` raises it, and its commit names the part that grew and by how
  many bytes; it never shaves another part (limits.md 5; shell.md 6.3).
- **The `history` probe:** a compaction is due when a request's prompt,
  as its usage counts it, is above the line, from the effective limits
  `session.started` carries; a prompt between the threshold and the line
  is no failure (benchmarks.md 11.1).

## Choices this plan makes

- The material above the line is one derived number, the run's
  `compaction_material`: the summary instruction's bytes plus
  `calls_per_response` times the cut marker's, a byte a token (390 + 16
  × 37 = 982 for the standard profile). The run checks each charter
  model's spare with it at the entrance, and gives each conversation its
  line, `window - output - compaction_material`, from the charter's
  effective values, when it opens it. The session takes the line from
  its open; it derives nothing of its own (decisions.md 12.4, question
  4).
- `run.compaction_material` is a new name among the effective limits,
  so the event vocabulary moves to version 3 (events.md 6; README, rule
  14).
- A negative spare is refused as the existing `Invalid::Llm`, whose doc
  line names it, so that the channel's version stays 2 (decisions.md 12.4, question
  5).
- `memory` stays 2 GiB while the larger of the two worst cases the
  profile's memory tests print (`PRODUCT_TOTAL`, `PRODUCT_COMPLETE`)
  fits it after 03.2's derivation; past it, it is raised to the smallest
  multiple of 64 MiB at or above that worst case. limits.md 5: "One whose
  worst case still fits leaves `memory` as it is, with no margin of its
  own kept above the worst case." At the base the larger,
  `PRODUCT_TOTAL`, leaves 120,442,167 bytes
  (`findings/planning-commands.md`; decisions.md 12.4, question 8).
  The worst case is also each shipped startup's own `service.worst_case`,
  which the startup check refuses past `declared.memory`: a face or
  model the two totals do not cover (the `exec` faces on `gpt-6-luna`,
  the pinned product check) counts, and `memory` is raised to the
  smallest multiple of 64 MiB at or above the largest of them all.

## Rules that bind it

- **Formats move once** (README, rule 14): the event vocabulary moves to
  3 in 03.1; 03.2 and 03.3 change it under 3.
- **The harness follows the binary** (README, rule 16): 03.1 moves
  `smith-bench`'s smith adapter to read version 3.
- **No shaving of a part's memory accounting** (`code-organization.md`,
  section 7): 03.2 raises `memory`.

## Increments

### 03.1 The line, the spare's check, and each conversation's line
Scope: limits.md 3.9 ("The line", "The spare") and 4's entrance;
  charter.md 6 and 8; events.md 6 (findings A8, A9).
Needs: nothing.
Files: `crates/smith-agent-service/src/profile/room.rs` (`:41-115`, the
  marker at `:80-82`), `crates/smith-agent-service/src/profile.rs`
  (where the run's limits are built, beside `schema_costs`),
  `crates/smith-domain-run/src/limits.rs` (`:52-55`),
  `crates/smith-domain-run/src/charter.rs` (`check_window_parts`,
  `:302-368`; `has_room`, `:493-507`), `crates/smith-domain-run/src/boundary.rs`
  (`Invalid::Llm`'s doc line, `:1013-1015`), the session's open: the
  struct the run opens a conversation with and its model entry, found by
  `rg -n 'compaction_threshold' crates/smith-domain-run/src crates/smith-domain-session/src`,
  `crates/smith-events/src/lib.rs` (`VERSION`, `:34`), the effective
  limits' list (`crates/smith-agent-service/src/profile/` `effective`),
  the smith adapter's version check in `benchmarks/src/agent/smith/`,
  the recorded streams `rg -l '"v":2,' benchmarks/tests/recorded tests`
  lists (7 at the base), `crates/smith-events/golden/v2/` (216 goldens,
  moved to `golden/v3/`, and the 18 files that read them), and
  `crates/smith-events/golden/unsupported-v3.jsonl`, which becomes
  `unsupported-v4.jsonl` for `a_frozen_line_of_another_version_is_refused_by_name`.
Shape: `run::Limits` gains `compaction_material: u64`, set by the profile
  from `run::summary_bytes()` and `calls_per_response *
  CUT_MARKER_BYTES`. The entrance refuses a charter model entry whose
  `window - threshold(window) < output + compaction_material` as
  `Invalid::Llm`, for the main model and each sub-agent model, beside
  `has_room`. The conversation's model entry at open gains `line: u64`,
  `window - output - compaction_material` of the charter's effective
  values; the session's `Spec` (`boundary.rs:261`) takes it as `line`.
  The session's `Conversation` does not keep it in 03.1: its first
  reader, 04.1, adds it as a named field. `model_window` and its reads,
  in `compaction.rs` among them, stay as they are. The effective
  limits name it `run.compaction_material`; `smith_events::VERSION` = 3.
Test shape: `crates/smith-domain-run/src/tests.rs`, beside the room's
  refusal tests (`rg -n 'Invalid::Room' crates/smith-domain-run/src/tests.rs`
  gives the one to copy): `a_charter_model_with_a_negative_spare_is_refused`,
  `a_charter_model_with_a_spare_of_zero_is_admitted`,
  `a_sub_agent_model_with_a_negative_spare_is_refused`,
  `each_conversation_opens_with_its_line` (the standard Codex model:
  239,018 as a literal). `crates/smith-agent-service/src/profile/room/tests.rs`:
  `the_compaction_material_is_the_summary_instruction_and_the_markers`
  (982). In `smith-events`' tests: `a_stream_of_version_two_is_refused`.
  Under 20 ms each.
Proof: process: smith's `check`, `heavy cargo run -q -p smith -- check -m codex/gpt-6.1-sol -c 'accounts.codex.source="borrow"' -c "accounts.codex.file=\"$HOME/.codex/auth.json\""`,
  prints `run.compaction_material` with 982, and the smoke (README, rule
  8) passes on version 3: the check's `run.compaction_material` line and
  the smoke's summary line go in the commit body.
Don't: add a refusal variant or a channel version; let the session
  derive the line from anything but its open.
Size: 150 lines.
Removes: nothing.

### 03.2 The history's bytes from the line, and `memory` raised
Scope: limits.md 3.2 (the history's bytes), 4 (`window-first`), 5 ("Room
  is made by raising the cap"); shell.md 6.3's `memory` row (findings
  A10 to A12).
Needs: 03.1 (each model's line).
Files: `crates/smith-domain-session/src/limits.rs` (`derive`,
  `:296-311`; the worst case, `:150-172`),
  `crates/smith-agent-service/src/profile.rs` (`threshold_bytes`,
  `:438-454`), `crates/smith-agent-service/src/profile/validate.rs`
  (`window_first`, `:413-482`), `crates/smith-agent-service/src/profile/quantities.rs`
  (`memory`, `:179`), `crates/smith-domain-session/src/tests.rs`
  (`:2769-2812`), `crates/smith-agent-service/src/profile/tests.rs`
  (`:519-552`), `tests/protocol-llm/tests/history_window.rs` (`:54`),
  `tests/session/tests/startup_receiving.rs` (`:38-50`),
  `crates/smith-local-shell/src/settings/tests.rs` (`:497-513`),
  `crates/smith-agent-shell/src/config.rs` (`:1430-1552`, `:1604`), and
  every user `rg -n 'threshold_bytes' crates tests` lists.
Shape: `threshold_bytes()` becomes `line_bytes()`, the largest over the
  declared models of `line * TOKEN_BYTES`, and the session's
  `Derivation` field likewise. `history_bytes = line_bytes +
  completion_reserve + batch`: the whole reserve, its answer no longer
  subtracted. `history_messages` and `history_blocks` follow from it as
  today; so do the resume limit, the transcript's ceilings and the LLM's
  history items. `window_first` checks the line at `TOKEN_BYTES` plus
  the whole reserve and one batch within `history_bytes`, its refusal
  naming `line`. `memory` follows 03's "Choices"; the commit body names
  each part whose worst case grew, with its bytes before and after, from
  `memory_parts` (`crates/smith-local-shell/src/local_host/memory.rs:68-93`),
  and what is left under `memory`.
  The expected values of the four derivation tests change to the line's
  arithmetic, listed in the commit body (README, rule 3):
  `derived_history_secures_threshold_and_full_receiving_before_effects`
  (`crates/smith-domain-session/src/tests.rs:2770`),
  `window_first_holds_owned_and_delegated_batches_at_the_exact_byte_edge`
  (`crates/smith-agent-service/src/profile/tests.rs:519`),
  `a_derived_history_at_its_bound_is_sent_and_one_byte_more_is_refused_typed`
  (`tests/protocol-llm/tests/history_window.rs:14`), and
  `the_frozen_control_is_refused_at_startup_or_services_finish_after_four_turns`
  (`tests/session/tests/startup_receiving.rs:10`).
Test shape: `crates/smith-domain-session/src/tests.rs`, beside the test
  at `:2769`: `the_history_holds_the_line_and_one_whole_completion` (a
  literal for a window of 1,000 tokens, output 100, material 50: line
  850 at 8 bytes, plus the reserve and the batch as literals);
  `crates/smith-agent-service/src/profile/tests.rs`:
  `window_first_refuses_a_history_one_byte_short_of_the_line`. Under 20
  ms each.
Proof: process: smith's `check` (03.1's command) passes, and prints
  `session.history_bytes` (6,331,914 at the base); the two memory tests'
  printed totals (`heavy cargo nextest run -p smith-local-shell -p
  smith-agent-shell -E 'test(/fit/)' --no-capture`), run once on
  `origin/main` and once on `HEAD`, go in the commit body.
Don't: lower `conversations`, `jobs` or any part to fit; derive
  `conversations` from the settings (findings `causes.md`, "The memory
  ceiling": not decided).
Size: 80 lines.
Removes: `threshold_bytes` and its field.

### 03.3 The `history` probe's condition at the line
Scope: benchmarks.md 11.1's `history` row, its condition (findings A13;
  decisions.md 12.4, question 6).
Needs: 03.1 (`run.compaction_material` in `session.started`).
Files: `benchmarks/src/agent/smith/events/threshold.rs` (`reached`,
  `:13-74`), `benchmarks/src/agent/smith/events/checks.rs` (`:321-326`),
  `benchmarks/tests/events/compaction_choice.rs`, the recorded stream
  `benchmarks/tests/recorded/compaction-threshold-8a5bb9ec-03.jsonl`.
Shape: `threshold::reached` becomes `threshold::passed_line`: a request
  has passed the line when its prompt (input, cache reads and cache
  writes) is above the line: the conversation's `window_tokens` less its
  `output_tokens`, as `conversation.opened` gives them, less
  `run.compaction_material` from `session.started`'s effective limits. Without `run.compaction_material` in the
  stream, the condition is unavailable, and `conversation-opened` and
  `compaction-choice` are "not exercised", with that reason.
Test shape: `benchmarks/tests/events/compaction_choice.rs`:
  `a_prompt_past_the_line_without_a_compaction_fails`,
  `a_prompt_between_the_threshold_and_the_line_is_no_failure`,
  `a_stream_without_the_material_leaves_the_condition_unexercised`. Under
  20 ms each.
Proof: offline, those tests on recorded streams. No live run: the
  `history` probe is not in the `parity` suites, and its recalibration
  after 07.1 is `later.md`'s.
Don't: count the output reservation in the condition.
Size: 60 lines.
Removes: `threshold::reached`.

## Questions to raise when reached

- None known; decisions.md 12.4's questions 4, 5, 6 and 8 are answered
  in the design (smith `f1b33a17`).

## Done when

- `cd ~/src/rust/smith && rg -n 'pub const VERSION: u64 = 3' crates/smith-events/src/lib.rs` prints one line.
- `cd ~/src/rust/smith && rg -n 'threshold_bytes|fn reached' crates tests benchmarks/src` prints nothing.
- `cd ~/src/rust/smith && heavy cargo run -q -p smith -- check -m codex/gpt-6.1-sol -c 'accounts.codex.source="borrow"' -c "accounts.codex.file=\"$HOME/.codex/auth.json\""` exits 0.
