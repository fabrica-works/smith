# Session 07: compaction

2026-10-14. Every compaction on record was asked with the conversation's
own tool choice, `auto`: 9 of 37 on the small tier answered with a call,
and the 8 with no text failed their run as malformed. The summary is its
text blocks glued with nothing between them, opens the new window with no
framing, and its instruction asks it to keep the task, which the new
window already holds (findings K1 to K4). This session asks every
compaction with `ToolChoice::None`, frames the summary with the run's
preamble, and rewrites the instruction; `history` version 3's
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
  version-3 transcript is read with it. A version-4 window turn written
  between 11.1 and 07.2, with the summary at index 1, comes only from the
  run's own builds: no smith that writes version 4 has shipped, and no
  step restores one, so 07.2 reads the summary at index 2 and handles
  nothing else (ready check of 07.1).

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
  (`:2186`, `:2201`; `answer_unchosen`, `:2102`, which makes
  `NotRun::OutsideChoice` and stores `conversation.choice` with it),
  `crates/smith-domain-session/src/llm.rs` (`NotRun`; `ToolChoice::Only`
  holds tickets, `:168`), `crates/smith-domain-session/src/compaction.rs`
  (`prompt_bytes`, `:107`, only), `crates/smith-domain/src/llm.rs` (`:21`
  re-exports the session's `NotRun`; the renderer needs the choice as
  served names, so smith-domain takes its own `NotRun`),
  `crates/smith-domain/src/peer.rs` (`:339`, which translates the kept
  choice as `:267-274` translates the request's),
  `crates/smith-protocol-llm/src/prompt.rs` (`:98-110`, `result_text`
  `:457`, `outside_choice` `:181-210`, and `:436`, which passes the
  request's choice to `result_text`), the tests that build or match the
  variant (`crates/smith-domain-session/src/tests.rs` `:2538`, `:2670`;
  `tests/protocol-llm/tests/client.rs` `:603`;
  `tests/session/tests/child_final.rs` `:51`;
  `tests/session/tests/wind_down.rs` `:48`; `tests/session/src/translate.rs`
  `:293`), `tests/session/tests/compaction.rs`,
  `tests/protocol-llm/tests/production_render.rs` (`:371`),
  `tests/protocol-llm/tests/` (a compaction in each dialect), and the
  tests that assert the fake's cache counts (`tests/protocol-llm/tests/brief.rs`
  `:387`, `:550`; `tests/protocol-llm/tests/budget.rs` `:397`, `:683`;
  `tests/agent/tests/budget.rs` `:338`). The repin changes none of those
  counts (the ready check ran the three worlds after it: 333 tests passed
  unchanged); only the `None` compaction may, and that commit's body
  gives each old and new value, derived by hand from fake-llm.md 2.1.
Shape: the compaction's `Prompt` carries `ToolChoice::None` for
  `Purpose::Compaction`; `prompt_bytes` (`compaction.rs:107`, which also
  serves `due`, `:71`, for ordinary requests) prices `None`'s choice bytes
  only while `compaction.asking()`, the state `session.rs:2167` calls it
  from; `new_bytes` (`:256`) keeps `conversation.choice`, which the new
  window asks with. The next window's first request carries
  `conversation.choice`. A `not run` result outside the choice keeps the
  choice it was answered under; `NotRun` loses `Copy`.
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
Size: 120 lines.
Removes: the compaction's use of the conversation's choice.

### 07.2 The summary, framed
Scope: session.md 8.2 (the new window, the summary); run.md 3.3 (the
  preamble given at open); limits.md 3.9; transcript.md 2 (findings K3).
Needs: 07.1; smith 11's transcript version 4 [11.1]; 08.s1 (ruling 115).
Files: `crates/smith-domain-session/src/session.rs` (the conversation's
  `summary`, `:186`; the opening's bytes, `:2025`, `:2055`),
  `crates/smith-domain-session/src/compaction.rs` (`open_window`,
  `:209-216`, where `held + 2` becomes `+ 3`; `new_bytes`, `:259`, which
  counts the preamble; the summary's own join at `:341`; `restore`,
  `:423`), `crates/smith-domain-session/src/boundary.rs` (`Spec.summary`,
  `:281`), `crates/smith-domain-run/src/boundary.rs` (`Opening.summary`,
  `:643`, `:673`), `crates/smith-domain/src/translate.rs` (`:67`, `:121`,
  `Opening` into `Spec`), `crates/smith-domain-run/src/opening.rs`
  (`:60`), `crates/smith-domain-run/src/run.rs` (`child_opening`,
  `:2036`), `crates/smith-domain-run/src/prompt.rs` (`render_fixed`,
  `:84`; `summary_bytes`, `:34`), `crates/smith-agent-service/src/profile.rs`
  (`:490`), the struct literals and byte checks of `Spec` and `Opening`
  (`crates/smith-domain-session/src/tests.rs` `:298`, its expected bytes
  `:491`; `crates/smith-domain-run/src/tests.rs` `:403`;
  `tests/session/tests/memory.rs` `:414`, `:508`, `:593`, `:696`;
  `tests/session/tests/recorded.rs` `:470`, `initial_owned`), the tests
  that read the old layout (`tests/session/tests/compaction.rs` `:59` and
  `:138`; `tests/session/src/window_referee.rs` `:73-82`), and the session
  and run worlds' fixtures.
Shape: `Spec` and `Opening` gain `preamble: Box<[u8]>` beside `summary`;
  the run gives `PREAMBLE` (in `prompt.rs`, beside `SUMMARY`) with each
  session it opens; the new window's message is `[task, preamble, summary,
  held-back…]`; the summary has its own join, at `compaction.rs:341`, its
  text blocks separated by `b"\n\n"`; `text_of` and its other users
  (`session.rs:1585`, the child's final text; `:1642`, the `Yielded` text)
  stay as they are, and a refusal block already ends the compaction
  `NoSummary::Refused` (`compaction.rs:311-326`); `restore` reads the
  summary at index 2; the preamble's bytes count in smith's own part.
Test shape: `a_summary_of_several_text_blocks_is_joined_by_blank_lines`,
  `the_new_window_opens_with_the_task_the_preamble_and_the_summary`,
  `a_restored_window_reads_its_summary_after_the_preamble`. The tests at
  `tests/session/tests/compaction.rs:59` and `:138`, and the referee at
  `tests/session/src/window_referee.rs:73-82`, expect the new order and
  join (the case the Shape changes). Under 0.5 s each.
Proof: world, those tests.
Don't: put the preamble in the summary's block.
Size: 160 lines.
Removes: the summary glued with nothing between its blocks.

### 07.3 The instruction's words, proven by `history`
Scope: run.md 3.3's four requirements and the preamble's words
  (findings K2, K4; decisions.md 4.3); benchmarks.md 4.2's before and
  after; `history` version 3 (benchmarks.md 3.2 and 11.1, as the
  designer's answers to the questions of 2026-10-14T20:18:45Z and
  21:00:40Z amended them).
Needs: 07.2; smith 04's calibration blocks and `history` version 2 [04.1,
  04.2]; 04.s2's `call-count` and `compaction-carried`; 04.s3's
  compaction checks not exercised before the threshold; smith 03's
  summary version 3 [03.1].
Files: `crates/smith-domain-run/src/prompt.rs` (`SUMMARY`, `:30`;
  `PREAMBLE`), its tests, `tests/local-process/src/world.rs` (`:1089`,
  a fake cue that matches "Summarise the work so far": the new `SUMMARY`
  keeps that opening, or the cue matches its new one),
  `benchmarks/tasks/probes/history/task.toml` (version 3, the two
  blocks, and `effort = "medium"` with its reason, ruling 118),
  `benchmarks/summaries/`.
Shape: `SUMMARY` and `PREAMBLE` rewritten to say, in that order, what
  run.md 3.3 lists; `SUMMARY` no longer says "Keep the task". `history`
  `version = 3`:
  - the prompt's sentence "Keep a running sum and your next filename as
    you go, including across any summary." is followed by "In each
    completion, before its read call, write one line of text: the sum of
    the values read so far, and the file you are reading."; nothing else
    in the prompt, and nothing in the seed, changes;
  - `tool-call` keeps `tool = "read"` and `verdict = "read"`, without its
    `count`; a new `[[event]]` `check = "call-count"`, `tool = "read"`,
    `count = { exactly = 32 }`; a new `[[event]]` `check =
    "compaction-carried"`; `completions` goes: 32 reads, at most one per
    response, imply its 33 completions;
  - `deciding = ["end", "exit-code", "answer", "tool-call", "call-count",
    "calls-per-response", "prompt-within-window", "compaction-carried"]`;
    `small-window`'s own `deciding`, checks and window stay as they
    are.
Test shape: `the_summary_instruction_and_its_preamble_fit_smiths_own_part`
  (their bytes within the bound the profile measures), under 20 ms.
Proof: live, once each, through `heavy --gate`, on the branch rebased
  on 04.s3, with the version 3 arm already built (`8a5bb9ec`: 04.s3
  changes only `benchmarks/`, so it is not rebuilt), and the control
  07.1's parent:
  `calibrate probes/history/small-window --control <07.1's parent> --arm
  <dir>` (the control fails `compaction-choice`; the arm passes 10 within
  12), then `calibrate probes/history --arm <dir>`, with no control: base
  `history` never compacts, so 07.1's parent runs nothing of it that the
  arm changed, and can fail it only by the model's errors. Its block's
  `control` reads "none: no runnable control was named", and the commit
  body says why (benchmarks.md 3.1). Each writes its block (04.1).
  - A failed check that is smith's (benchmarks.md 3.2), or an unjudged
    attempt, stops on a question quoting the attempt's failed checks and
    their evidence. A malformed end under `None` is smith's, and stops on
    a question (decisions.md 4.4). An attempt whose compaction checks are
    "not exercised" (benchmarks.md 3.2, 11.1), with its other failed
    checks the model's, is a model deviation, and does not stop.
  - The small-window command's first version 3 run
    (`probes-history-small-window-8a5bb9ec-20261014T205416Z`) stopped
    before 04.s3 and is spent: `calibrate` has no resume, so the command
    runs again whole, control included. A base `history` block written
    before 04.s3 stands: base `history` decides neither compaction
    check.
  - If either command stops on model deviations alone (ten passes out of
    reach), the task names `effort = "high"`, its reason in the comment
    above it, and both commands run once more. An attempt that ends on its
    budget at high, or a second such stop, stops on a question.
  - The attempts and spend are README row 07.3's.
Don't: change the threshold (4/5; decisions.md 4.5); change `SUMMARY` or
  `PREAMBLE` to suit `history`: a summary that drops or misstates what it
  was given, with `compaction-carried` passing, is the model's
  (benchmarks.md 3.2).
Size: 30 lines.
Removes: "Keep the task, decisions, …" from the instruction; `history`'s
  `completions` check and `tool-call`'s read count.

## Done when

- The README's "Done when" item 4 prints at least 2 (`history`,
  `history/small-window`).
- `rg -n 'Keep the task' crates/smith-domain-run/src/prompt.rs` prints
  nothing.
