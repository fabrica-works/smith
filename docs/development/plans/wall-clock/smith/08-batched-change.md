# Session 08: a change and its check in one response

2026-10-16. Causes 1 and 3 of `findings/causes.md`: a cargo start in a
turn of its own (+14.6 requests, about 90 s an attempt), and one file per
edit request (all 129 edit responses held one edit of one file, about
62 s), where Codex made 8 to 13 patches of 1.6 to 3.1 files. The role's
batching topic named only calls that change nothing. The design asks
the role for a change's edits, across its files, then the command that
checks them, in one response, with every replacement one file needs in
one `edit`; and so that a check never runs on a half-made change, a
failed write stops the writes and commands after it in its completion,
answered as not run, naming it, a why new in transcript version 5
(decisions.md, section 6). This step changes what the model is sent, so
it is proven by its `parity-change` run.

The baseline, `parity-c10edbe5-20261016T184102Z`, missed FT, RT and RC,
and causes 1 and 3 are FT's cause too. On the fixtures, none of smith's
71 responses that held an edit held a second edit or a command, and
smith made 194 responses in 24 attempts to Codex's 148; Codex ran its
check in the call that made a patch in 23 of its 24 attempts. The
responses this step folds into the one before them (an edit after an
edit, a command after an edit) are 70 there, about 111 of smith's 1,919
seconds: by that estimate, FT about 1.11, from 1.18. On `shell-cwd`,
smith made 70.1 responses an attempt to Codex's 45.5, and causes 1 and 3
are about 3.8 million cached tokens an attempt (`findings/causes.md`),
$0.38 at the base rate, of smith's median $1.552 against Codex's $1.103.
This step addresses FT, RT and RC.

## Read first

- smith `docs/design/domain/session.md`, section 5 ("A failed write stops
  the writes after it") and section 10's story of edits of several files
  and a command.
- smith `docs/design/protocol/llm.md`, sections 3 (the batching
  sentences) and 5 ("Not run and withdrawn").
- smith `docs/design/protocol/transcript.md`, sections 2 (not run, with
  why) and 6 (version 5).
- smith `docs/design/protocol/events.md`, section 3.6 (`verdict`).
- smith `docs/design/shell.md`, section 6.6 (topics 3 and 4).
- smith `docs/design/benchmarks.md`, sections 16.5 and 16.6.
- `decisions.md`, sections 6 and 12; `findings/planning-audit.md` A14 to
  A16, A25 to A27.
- skein `programming-model.md`, sections 3 and 4; `codec.md`, section 3.

## What the design asks

- **A failed write stops the writes after it:** when a write of a
  completion fails, its later writes and commands are answered as not
  run, naming that call; its later reads still run. A write fails when
  it is answered `invalid`, whatever its tool, or when a `write` or an
  `edit` is answered `conflict`, `missing`, `too_large` or `failed`.
  Nothing else stops them: a command's end, whatever its exit,
  `timed_out` included; a host tool's `error`; a sub-agent's answer; a
  call not run or withdrawn (session.md 5). `finish` and `deliver` are
  writes, and `job` with `stop` is one.
- **The not-run text** names the failed call, and says that the not-run
  call's changes were not made (llm.md 5).
- **The transcript** keeps the why, and a resumed history renders it as
  it rendered when told: version 5 (transcript.md 2, 6).
- **The descriptions** say that writes, edits and commands may be called
  together and run one at a time in call order: a change's edits across
  its files, then the command that checks them; and that a write that
  fails leaves the writes and commands after it in its response not run
  (llm.md 3).
- **The role's topics 3 and 4** (shell.md 6.6): batching a change's
  edits, across its files, then the command that checks them; every
  replacement one file needs in one `edit`.

## Choices this plan makes

- The failed write's set in the code's verdicts, and how the not-run
  text names the call, follow decisions.md 12.4's questions 1 and 2, as
  answered in the design (smith `f1b33a17`): a `write` or an `edit`
  fails on every verdict but `written` and `edited`, `unchanged`
  included; any call answered `invalid` fails, whatever its tool and
  effect, a read's included; a `shell` or `job` call's other answers
  stop nothing. The not-run text is `not run: call <n> (<tool>) failed
  before it; this call's changes were not made`, `<n>` the failed
  call's position counted from 1 over every call the response holds,
  invalid, oversized and cut ones included (llm.md 5).
- Version 5's not-run why carries every why the design names, so a
  resumed outside-the-choice call renders as it was told too, which
  version 4 drops (findings A15): one format change for both.
- The role's new words, replacing paragraphs 3 and 4 of `TASK_ROLE`
  (`role.rs:6-20`), are below; topic 3's poll clause is 09.1's.
- The descriptions' new sentence for `write`, `edit` and `shell`,
  replacing "Batch writes, edits and commands; run serially in call
  order." (`descriptions.rs:107`): "Batch a change's edits across its
  files, then the command that checks them; they run one at a time in
  call order, and a write that fails, or any call answered invalid,
  leaves the writes and commands after it not run."

Topic 3, replacing the role's third paragraph:

> Batch in one response every call that waits on no other call's answer:
> `read`, `list` and `search` calls, and `shell` calls that change
> nothing. Batch a change the same way: its edits, across all its files,
> then the command that checks them. Keep steps that depend on earlier
> results in order.

Topic 4, replacing the role's fourth paragraph:

> Change files with `edit` and `write`. Put every replacement one file
> needs in one `edit`. Use the smallest change that solves the problem,
> preserve surrounding conventions, and avoid unrelated changes. Do not
> change files through shell scripts.

## Rules that bind it

- **One change at a time** (README, rule 11), **the stop** (rule 12),
  **no split** (rule 1).
- **Formats move once** (rule 14): the transcript moves to 5 here.
- **No `edit` over several files and no patch tool**
  (`code-organization.md`, section 7).

## Increments

### 08.1 A failed write stops the writes after it, and the role asks for a change and its check in one response
Scope: session.md 5's failed write; llm.md 3's batching sentences and 5's
  not-run text; transcript.md 2 and 6, version 5; shell.md 6.6, topics 3
  and 4 (findings A14 to A16, A25 to A27; causes 1 and 3).
Needs: smith 07's check [07.1] landed or dropped.
Files: `crates/smith-domain-session/src/session.rs` (`Tools`, `:287-306`;
  `advance`, `:1489-1603`; `joins`, `:1646-1655`; the answers at
  `:1540-1554`, `:1584`, `tool_ran`, `:1755-1777`),
  `crates/smith-domain-session/src/llm.rs` (`NotRun`, `:154-159`),
  `crates/smith-domain/src/llm.rs` (`:78-86`), `crates/smith-domain/src/peer.rs`
  (`:343-349`), `crates/smith-domain-tools/src/call.rs` (`effect`,
  `:233-238`, read), `crates/smith-protocol-llm/src/prompt.rs`
  (`:181-208`, `:452-456`), `crates/smith-protocol-llm/src/descriptions.rs`
  (`:103-108`), `crates/smith-protocol-llm/src/tests.rs` (`:323`,
  `:343-367`), `tests/protocol-llm/tests/production_tools.rs` (`:389`,
  `:410-419`), `tests/protocol-llm/tests/production_render.rs` (`:401`),
  `crates/smith-transcript/` (`lib.rs:11-13`, `:31`; a new
  `schema/transcript-v5.schema`; `src/generated/v5.rs`; `golden/v5/`),
  `crates/smith-protocol-channel/src/turn.rs` (`:160`),
  `crates/smith-protocol-channel/src/transcript.rs` (`:268`),
  `crates/smith-local-domain/src/person.rs` (`:276`),
  `tests/session/src/translate.rs` (`:293-296`),
  `crates/smith-domain-session/src/history.rs` (`:120`),
  `tests/codecs/tests/drift.rs` (`:22-33`), `tests/codecs/tests/transcript.rs`
  (`:5-9`), `crates/smith-local-shell/src/role.rs`, `tests/session/tests/`,
  `benchmarks/summaries/`.
Shape: `NotRun` gains `AfterFailedWrite { call: u16 }`, the failed
  call's position in the completion, in the session's vocabulary and
  `smith-domain`'s. `Tools` gains `failed: Option<u16>`, set when a
  write's answer fails or any call is answered `invalid` (the set
  above), at each place an answer is
  recorded; while it is set, `advance` answers each later write (an
  owned call whose `tools::effect` is a write, a delegated call whose
  effect is) and each `shell` call as
  `NotRun { why: AfterFailedWrite { call } }` without starting it, and
  runs reads as today. The prompt renders it with the text above. The
  transcript's version 5 schema copies version 4's with `not_run`
  carrying a `NotRunWhy` enum, `stopped`, `outside_choice` with the
  allowed calls' names, and `after_failed_write` with the position; the
  generator writes `src/generated/v5.rs` (the command in
  `findings/planning-commands.md`); `Turn::FORMAT_VERSION` = 5; v4's
  schema, generated code and goldens go. `turn.rs` and `transcript.rs`
  keep the why both ways. The descriptions and `TASK_ROLE` take the
  words above; the goldens' expected texts change with them (README,
  rule 3).
Test shape: `crates/smith-domain-session/src/tests.rs`, beside
  `adjacent_reads_run_together_and_a_write_runs_alone_and_results_go_back_in_order`
  (`:791`): `a_failed_edit_leaves_the_writes_after_it_not_run`,
  `a_failed_edit_leaves_the_command_after_it_not_run`,
  `a_read_after_a_failed_edit_still_runs`,
  `an_invalid_call_stops_the_writes_after_it`,
  `an_unchanged_edit_stops_the_writes_after_it`,
  `an_invalid_read_stops_the_writes_after_it`,
  `a_call_that_is_no_failed_write_stops_nothing` (one table test, its
  rows a command's nonzero exit, a command `timed_out`, a `job` call's
  `running`, a host tool's `error`, a sub-agent's answer and a withdrawn
  call, each followed by an edit that runs),
  `a_finish_after_a_failed_write_is_not_run`. `tests/session/tests/simulation.rs`,
  beside `reads_in_one_answer_run_side_by_side_and_writes_alone` (`:95`):
  `edits_of_several_files_and_a_command_run_in_call_order`,
  `an_edit_that_does_not_match_leaves_the_rest_of_the_change_not_run`.
  `tests/codecs/tests/transcript.rs`: `a_not_run_why_survives_a_round_trip`
  for each why, as rows of one table; `a_version_four_transcript_is_refused`.
  `tests/protocol-llm/tests/production_render.rs`:
  `a_call_not_run_after_a_failed_write_names_it`,
  `a_resumed_outside_choice_call_renders_as_it_was_told`. Unit tests
  under 20 ms; world tests under 0.5 s.
Proof: live, through the stop: `heavy cargo run -p smith-bench -- build
  HEAD` and `build origin/main`, then `heavy --gate cargo run -p
  smith-bench -- suite parity-change --arm <HEAD's dir> --parent-arm
  <origin/main's dir> --baseline-run <the file README rule 13's first
  command prints> --landed <each file its second prints>`: 40 attempts,
  about 3 hours, run once; its exit decides (README, rule 12). Its
  summary is amended into the step's commit and named in
  `Change-Summary:`; the commit body gives its criteria, projection and
  stop lines, README rule 13's counts beside, with each arm's `edit` and `shell` calls an attempt from its
  `tool_calls`, and names causes 1 and 3 and the criteria they address,
  FT, RT and RC, with the baseline's figures above.
Don't: stop reads; stop calls after a command's end; name a task's files
  or commands in the role or the descriptions.
Size: 260 lines.
Removes: transcript version 4's schema, generated code and goldens.

## Questions to raise when reached

- None known; decisions.md 12.4's questions 1 and 2 are answered in the
  design (smith `f1b33a17`).

## Done when

- `cd ~/src/rust/smith && rg -n 'FORMAT_VERSION: u64 = 5' crates/smith-transcript/src/lib.rs` prints one line, or `awk -F'\t' '$1 == "smith-08.1" { print $4 }' ~/src/rust/tmp/smith/wall-clock/ledger.tsv` prints `dropped`.
- `cd ~/src/rust/smith && rg -c 'every replacement one file' crates/smith-local-shell/src/role.rs` prints 1, or `awk -F'\t' '$1 == "smith-08.1" { print $4 }' ~/src/rust/tmp/smith/wall-clock/ledger.tsv` prints `dropped`.
