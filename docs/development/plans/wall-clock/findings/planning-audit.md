# The planner's code audit

2026-10-16. The code the wall-clock plan touches, read against the
design at smith `4ddb9b67` (development.md 5.4). Six read-only
sub-agents each answered one question; this file keeps what the plan
relies on. Each entry: what the code does, against what the design asks,
and the increment that changes it. Paths are smith's; skein's are at its
pinned `e30fe58`.

## The window, the line and the guard

- **A1.** The check reserves a completion's most output below the
  threshold, and counts appended bytes a byte a token:
  `crates/smith-domain-session/src/window.rs:88-99` (`+ output` at
  `:98`). `Window` (`:23-33`) keeps no count of the last prompt's bytes,
  which a measured ratio needs. Against session.md 8.1. 07.1.
- **A2.** The bound without usage, `prompt_bytes`
  (`compaction.rs:97-113`), counts cells (`size_of::<Prompt>()`, a
  `Message` per message, a `Block` per block), while the appended count
  counts payload bytes (`compaction.rs:86-94`). Against session.md 8.1's
  one measure. 07.1.
- **A3.** A compaction's owed-results room still subtracts one `output`
  (`session.rs:2199-2219`, at `:2215-2218`). Against limits.md 3.9 and
  llm.md 5. 07.1.
- **A4.** The session world's referee asserts `input + max_tokens <=
  window` (`tests/session/src/window_referee.rs:30-64`), and
  `drawn_small_windows_compact_within_them` adds 512 output
  (`tests/session/tests/fuzzy_simulation.rs:114`, `:135`): the old
  reservation. 04.2 adds the line; 07.1 drops the reservation.
- **A5.** The session world's provider is skein's
  `skein-fake-llm-domain`, which counts a request's input as its text
  bytes over 4 (`domain.rs:330-341`, `cache.rs:111-121`), with no size
  per block. Against session.md 10's provider. 04.2.
- **A6.** No guard. The domain's prompt (`smith-domain-session/src/llm.rs:242-276`)
  carries no line, last prompt or start of the new part; `peer.prompt`
  drops `protocol_allowance` (`smith-domain/src/peer.rs:250`). Nothing
  produces an unsent *context too long*: the local refusal path is
  `failure.rs:70-84`. The site is `component.rs` `start`, after
  preparation (`:384-403`) and before submission (`:408-442`). 04.1.
- **A7.** skein gives only a whole request's encoded size
  (`client::measure`, `client.rs:312-325`; `Measured { head, body }`,
  `:303`); its per-block writers are private. No per-block framing share
  exists: the prompt's allowance is one whole-prompt number
  (`smith-protocol-llm/src/framing.rs:15-57`). Question 3
  (`decisions.md` 12.4). 04.1.
- **A8.** Neither the line nor the spare is computed: `Room`
  (`smith-agent-service/src/profile/room.rs:13-26`) has neither. The
  summary instruction's bytes (`smith-domain-run/src/prompt.rs:28`,
  `:38-40`, 390) and the cut marker's (`smith-protocol-llm/src/render.rs:16`,
  37) reach only `room()` (`:79-94`). 03.1.
- **A9.** The entrance checks the room (`charter.rs:349-356`,
  `has_room` `:493-507`), not the spare; the run's `Limits` carry `room`
  and `compaction_threshold` only (`smith-domain-run/src/limits.rs:52-55`).
  A new refusal variant would reach the channel's schema
  (`smith-channel/schema/channel-v2.schema:160`) and eight translators.
  Question 5. 03.1.
- **A10.** The history's bytes come from the threshold: `history_bytes =
  threshold_bytes + (completion_reserve - answer_bytes) + batch`
  (`smith-domain-session/src/limits.rs:296-311`; `threshold_bytes`,
  `smith-agent-service/src/profile.rs:438-454`, `TOKEN_BYTES` = 8). From
  the line with the whole reserve, the standard Codex model's history
  grows by 171,344 bytes (the line's 21,418 tokens above the threshold)
  plus 256,000 (the answer), 427,344 a conversation, before the terms
  that multiply it (`limits.rs:150-172`; `smith-protocol-llm/src/limits.rs:119`;
  `smith-local-domain/src/limits.rs:117-120`). 03.2.
- **A11.** Tests that pin the derivation: `smith-domain-session/src/tests.rs:2769-2812`
  (`:2791`), `smith-agent-service/src/profile/tests.rs:519-552`,
  `tests/protocol-llm/tests/history_window.rs:54`,
  `tests/session/tests/startup_receiving.rs:38-50`. 03.2.
- **A12.** The worst case: `startup_memory`
  (`smith-local-shell/src/local_host/memory.rs:13-57`, by part
  `:68-93`), checked by `validate::memory` (`validate.rs:316`) at
  startup and by `smith check`; `memory` = 2 GiB at
  `profile/quantities.rs:179`. Two tests assert it fits and print the
  totals: `the_default_inline_product_parts_fit_two_gibibytes`
  (`smith-local-shell/src/settings/tests.rs:497-513`, `PRODUCT_TOTAL`)
  and `product_values_pass_every_startup_check_and_fit_complete_memory`
  (`smith-agent-shell/src/config.rs:1430-1552`, `PRODUCT_COMPLETE`).
  The margin at the base: `planning-commands.md`. 03.2.
- **A13.** The `history` probe's condition is the old reservation:
  `benchmarks/src/agent/smith/events/threshold.rs:13-74` (`reached`,
  `:59-69`), used at `checks.rs:321-326`. The effective limits carry
  neither the summary instruction's size nor the marker's
  (`benchmarks/tests/recorded/smith-product-check.json`). Questions 4
  and 6. 03.3.

## Batches, not-run calls and jobs

- **A14.** A call's failure stops nothing: `advance`
  (`session.rs:1489-1603`) and `joins` (`:1646-1655`) group calls by
  effect only; answers are recorded at `:1540-1554`, `:1584` and in
  `tool_ran` (`:1755-1777`). 08.1.
- **A15.** Not-run reasons are `Stopped` and `OutsideChoice`
  (`smith-domain-session/src/llm.rs:154-159`; `smith-domain/src/llm.rs:78-86`),
  rendered at `smith-protocol-llm/src/prompt.rs:452-456`. The
  transcript drops the reason (`smith-protocol-channel/src/turn.rs:160`)
  and restores `Stopped` (`transcript.rs:268`), so a resumed
  outside-the-choice call renders "not run", against transcript.md 2.
  08.1 carries every why in version 5.
- **A16.** The design's failed-write verdicts `conflict` and `missing`
  have no `Outcome`: `verdict_kind` (`smith-agent-service/src/calls.rs:437-470`)
  maps finer ones (`NotRead`, `Stale`, `NoMatch`, `Ambiguous`,
  `Unchanged`, `NotFound`, `NotFile`, `NotDirectory`, `Outside`,
  `ReadOnly`, `Linked`, `Protected`, `TooLong`, `NulByte`, `Busy`,
  `NotGranted`, `TooLarge`, `Failed`), and nothing emits
  `Verdict::Conflict` or `Missing`. A `failed` write may have written
  part of its change (`tests/tools/tests/simulation.rs:1071`), against
  "it changed nothing". Question 1. 08.1.
- **A23.** Nothing ends a job's wait early: the machine waits
  (`smith-protocol-machine/src/process/jobs.rs:44`, answers at
  `:112-125`), one act per held job (`:37-42`); `CancelIo` cancels the
  job itself. The fakes to extend: `tests/tools/src/world/jobs.rs:66-87`,
  `tests/agent/src/world/jobs.rs`, `tests/protocol-llm/src/world/jobs.rs`,
  `tests/session/src/recorded.rs:341-345`; the session simulation marks
  job acts unreachable (`tests/session/src/world.rs:1209`). 09.1.

## The harness

- **A17.** An attempt is priced once, at the main model's declared
  window: `priced_spend` (`benchmarks/src/result.rs:315-381`, `:327`),
  the tier switch at `smith-local-shell/src/presets.rs:117-130`. 01.1.
- **A18.** Both adapters read usage per response (smith's
  `usage.rs:74-98`; Codex's `rollouts.rs:156-175`, `:300-327`), but the
  ledger gives out per-scope sums only (`tokens.rs:221-226`,
  `:294-335`). 01.1.
- **A19.** A row (`summary.rs:95-107`) holds no responses, though the
  result counts them (`result.rs:160`). 01.2.
- **A20.** smith's `session.ended` `peak_rss_bytes` is read only for
  null (`passive.rs:171-187`); the summary's peak is the harness's count
  over the process tree (`run.rs:256-282`). 01.2.
- **A21.** `parity`'s literals: `parity.toml:12` (5), `parity.rs:150-151`
  (58), `:179` (29.0, not the arm's count), `:196`;
  `tests/recorded/parity/on-par.json` (58 rows), `tests/parity.rs:28`,
  `:136`; `benchmarks/README.md:180-181`. 01.3.
- **A22.** No change suite: `parity.rs` selects rows by the agent names
  "codex" and "smith" (`:87-89`, `:115-116`, `:131-133`, `:164-173`)
  and refuses two smith commits (`:240-248`); `suite` takes no
  `--parent` (`probe.rs:526`); `summarise` takes one drift baseline, no
  run summary (`main.rs:31-35`; `summary.rs:454-462`, `:667`). `role.toml`
  is the precedent of two binary smith arms with a named reference. 02.

## Memory, the role and the descriptions

- **A24.** skein's `Meter::held()` (`testing/skein-heap/src/lib.rs:357`)
  reads the thread's live heap at any point, re-exported by
  `skein_world::domain` (`domain.rs:32-34`). The agent's scripted
  `World` (`tests/agent/src/world.rs:375-424`) has no meter and grows
  its own vectors; `tests/agent/tests/fuzzy_memory.rs`'s driver makes
  its meter before the domain (`:736-802`), so `held()` there reads the
  domain's heap, but it never drives a job (`:328`, `:585`). 05.
- **A25.** The task role is `TASK_ROLE` (`smith-local-shell/src/role.rs:6-20`),
  1,728 bytes against `TASK_ROLE_BOUND` 4,096 (`:23`); its topics 2 to 5
  lack shell.md 6.6's ranged reads, batched change, one `edit` a file
  and polls beside work. Pinned by `role.rs:30`, `:35`,
  `settings/tests.rs:712`, `profile/tests/role.rs`. 08.1, 09.1, 10.1.
- **A26.** The descriptions are `descriptions.rs` `words` (`:48-110`):
  `job` at `:78-82`, the batching sentences at `:103` and `:107`; pinned
  by `smith-protocol-llm/src/tests.rs:323` (`:343-367`) and
  `tests/protocol-llm/tests/production_tools.rs:389` (`:410-419`); their
  lengths feed `fixed_schema_bytes` (`tools.rs:72`). 08.1, 09.1.
- **A27.** The transcript is version 4 (`smith-transcript/src/lib.rs:31`,
  `generated/v4.rs`, `schema/transcript-v4.schema`, its `not_run` at
  `:77` without a why; goldens `golden/v4/`), with its paths in
  `tests/codecs/tests/drift.rs:22-33` and `transcript.rs:5-9`. jig and
  temper pin older smith and read version 3. 08.1.
