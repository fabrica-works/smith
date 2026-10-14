# Session 07: compaction

2026-10-14. Every compaction on record was asked with the conversation's
own tool choice, `auto`: 9 of 37 on the small tier answered with a call,
and the 8 with no text failed their run as malformed. The summary is its
text blocks glued with nothing between them, opens the new window with no
framing, and its instruction asks it to keep the task, which the new
window already holds (findings K1 to K4). This session asks every
compaction with `ToolChoice::None`, frames the summary with the run's
preamble, and rewrites the instruction; `history` version 2's
calibration proves it.

## Read first

- smith `docs/design/domain/session.md`, sections 5 (tool choice) and
  8.1 to 8.2; `docs/design/domain/run.md`, section 3.3 ("The summary
  instruction and its preamble"); `docs/design/protocol/llm.md`, sections
  2.2 and 3 (tool choice); `docs/design/protocol/limits.md`, section 3.9;
  `docs/design/protocol/transcript.md`, section 2 (the window turn).
- skein `docs/design/fake-llm.md`, section 2.1.
- `findings/compaction.md` K1 to K6; `decisions.md` 4.1 to 4.4.
- skein `programming-model.md`, sections 3 and 4.5; `testing-strategy.md`,
  sections 2.2 and 2.5.

## What the design asks

- **A compaction keeps the window's system text and tools, and is asked
  with `ToolChoice::None`,** whatever the conversation's choice; the new
  window asks with the conversation's choice again (session.md 8.2;
  llm.md 2.2). Calls in its answer are not run.
- **The new window opens with one user message of separate text blocks:**
  the task, the summary preamble, the summary, then the held-back
  messages, one block each (session.md 8.2; transcript.md 2).
- **The summary** is the answer's text blocks, in order, separated by a
  blank line (session.md 8.2).
- **The instruction must say** that the work is not finished and a new
  window continues it, opening with the task's own words; to answer with
  text and call no tool; to write what the next window needs that the task
  does not say (what was done and found, with exact values, names and
  paths as read; what remains; the decisions taken); and not to restate
  the task or its required answer. **The preamble** says that what follows
  is the model's own summary of an earlier window, and that the work goes
  on. Both count in smith's own part of a window (run.md 3.3; limits.md
  3.9).

## Choices this plan makes

- A result answered `not run` for being outside the tool choice keeps,
  in the session's history, the choice in force when it was answered (the
  transcript's `not_run` record is unchanged, and a resumed window renders
  it as it does today), so that a compaction asked
  with `None` after a narrowing to `Only` renders earlier history
  unchanged (`prompt.rs:457` today renders it with the current request's
  choice; findings `planning-audit.md`, LLM protocol, point 3).
- The window turn's preamble is a text block like the others, and the
  summary moves to the third block: a layout a reader notices, so 07.2
  lands under the transcript's version 4 (11.1; README, rule 12), and no
  version-3 transcript is read with it.

## Rules that bind it

- **Upstream first** (development.md 9.5): 07.1 repins skein in a commit
  of its own, at its start, for skein 01's prefix.
- **The live budget** (README, rule 10): 07.3's line is 28 attempts.
- **Hotspots** (rule 15): `smith-domain-session/src/session.rs` is shared
  with 09.2 and 10.3.

## Increments

### 07.1 A compaction asked with `None`
Scope: session.md 5 and 8.2; llm.md 2.2 and 3 (findings K1).
Needs: smith 08's vocabulary [08.1] (07.3's control is this increment's
  parent, and must write the events the harness reads); skein 01's prefix
  [skein 01.1], repinned in this increment's first commit (`heavy cargo update -p skein-fake-llm-domain`, or the
  package of skein's git source that moves it).
Files: `Cargo.lock`; `crates/smith-domain-session/src/session.rs`
  (`:2186`, `:2201`), `crates/smith-domain-session/src/compaction.rs`
  (`:107`, `:256`), `crates/smith-protocol-llm/src/prompt.rs` (`:98-110`,
  `result_text` `:457`, `outside_choice` `:181-210`), the `NotRun` result
  type it renders, `tests/session/tests/compaction.rs`,
  `tests/protocol-llm/tests/production_render.rs` (`:371`),
  `tests/protocol-llm/tests/` (a compaction in each dialect), and the
  tests that assert the fake's cache counts (`tests/protocol-llm/tests/brief.rs`
  `:387`, `:550`; `tests/protocol-llm/tests/budget.rs` `:397`, `:683`;
  `tests/agent/tests/budget.rs` `:338`), which the new prefix may change:
  their new expected values are derived by hand from fake-llm.md 2.1, and
  the commit body gives each old and new value.
Shape: the compaction's `Prompt` carries `ToolChoice::None` for
  `Purpose::Compaction`, and its priced choice bytes are `None`'s; the
  next window's first request carries `conversation.choice`. A `not run`
  result outside the choice keeps the choice it was answered under.
Test shape: `tests/session/tests/compaction.rs`:
  `a_compaction_is_asked_with_none_and_the_windows_tools`,
  `a_conversation_narrowed_to_only_asks_only_again_after_its_compaction`,
  `a_childs_final_turn_falling_due_during_a_compaction_waits_for_it`;
  `tests/protocol-llm/tests/`: `a_compaction_asks_none_in_each_dialect_and_the_next_window_its_choice`;
  `production_render.rs`'s `a_compactions_owed_results_are_cut_and_its_history_is_unchanged`
  extended to a conversation narrowed to `Only`. Under 0.5 s each.
Proof: world, those tests; the live before and after are 07.3's.
Don't: drop the tools from a compaction, or retry or nudge one
  (decisions.md 4.1, 4.4).
Size: 80 lines.
Removes: the compaction's use of the conversation's choice.

### 07.2 The summary, framed
Scope: session.md 8.2 (the new window, the summary); run.md 3.3 (the
  preamble given at open); limits.md 3.9; transcript.md 2 (findings K3).
Needs: 07.1; smith 11's transcript version 4 [11.1].
Files: `crates/smith-domain-session/src/session.rs` (`text_of`,
  `:2239`), `crates/smith-domain-session/src/compaction.rs` (`open_window`,
  `:209-216`; `restore`, `:424`), `crates/smith-domain-session/src/boundary.rs`
  (`Spec.summary`, `:281`), `crates/smith-domain-run/src/boundary.rs`
  (`Open.summary`, `:673`), `crates/smith-domain-run/src/opening.rs`
  (`:60`), `crates/smith-domain-run/src/run.rs` (`child_opening`,
  `:2036`), `crates/smith-domain-run/src/prompt.rs` (`render_fixed`,
  `:84`; `summary_bytes`, `:34`), `crates/smith-agent-service/src/profile.rs`
  (`:486`), `tests/session/tests/compaction.rs` (`:59`), the session and
  run worlds' fixtures.
Shape: `Spec` and `Open` gain `preamble: Box<[u8]>` beside `summary`; the
  run gives `PREAMBLE` (in `prompt.rs`, beside `SUMMARY`) with each session
  it opens; the new window's message is `[task, preamble, summary,
  held-back…]`; `text_of` joins text blocks with `b"\n\n"` and takes no
  refusal text; `restore` reads the summary at index 2; the preamble's
  bytes count in smith's own part.
Test shape: `a_summary_of_several_text_blocks_is_joined_by_blank_lines`,
  `the_new_window_opens_with_the_task_the_preamble_and_the_summary`,
  `a_restored_window_reads_its_summary_after_the_preamble`. The test at
  `tests/session/tests/compaction.rs:59` expects the new order (the case
  the Shape changes). Under 0.5 s each.
Proof: world, those tests.
Don't: put the preamble in the summary's block.
Size: 120 lines.
Removes: the summary glued with nothing between its blocks.

### 07.3 The instruction's words, proven by `history`
Scope: run.md 3.3's four requirements and the preamble's words
  (findings K2, K4; decisions.md 4.3); benchmarks.md 4.2's before and
  after.
Needs: 07.2; smith 04's calibration blocks and `history` version 2 [04.1,
  04.2]; smith 03's summary version 3 [03.1].
Files: `crates/smith-domain-run/src/prompt.rs` (`SUMMARY`, `:30`;
  `PREAMBLE`), its tests, `benchmarks/tasks/probes/history/task.toml`
  (the two blocks), `benchmarks/summaries/`.
Shape: `SUMMARY` and `PREAMBLE` rewritten to say, in that order, what
  run.md 3.3 lists; `SUMMARY` no longer says "Keep the task".
Test shape: `the_summary_instruction_and_its_preamble_fit_smiths_own_part`
  (their bytes within the bound the profile measures), under 20 ms.
Proof: live, once each, through `heavy --gate`, with the control 07.1's
  parent, built first: `calibrate probes/history/small-window --control
  <07.1's parent> --arm <dir>` (the control fails `compaction-choice`;
  the arm passes 10 within 12) and `calibrate probes/history --control
  <07.1's parent> --arm <dir>`. Each writes its block (04.1). At most 28
  attempts, $1.50, 60 minutes. A malformed end under `None` is smith's,
  and stops on a question (decisions.md 4.4).
Don't: change the threshold (4/5; decisions.md 4.5).
Size: 30 lines.
Removes: "Keep the task, decisions, …" from the instruction.

## Done when

- The README's "Done when" item 4 prints at least 2 (`history`,
  `history/small-window`).
- `rg -n 'Keep the task' crates/smith-domain-run/src/prompt.rs` prints
  nothing.
