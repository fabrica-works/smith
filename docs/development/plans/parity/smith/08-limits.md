# Session 08: the event vocabulary, `parallel_tools`, `jobs` and `job_wait`

2026-10-14. Jobs need a vocabulary (a call's `job`, the verdicts
`running` and `stopped`, `job.ended`) and three named values the code
does not have: `parallel_tools`, today a literal 4 in three places;
`jobs`; and `job_wait`. The machine's processes are `min(calls, 8)`,
which counts neither the conversations' batches nor their jobs (findings
`planning-audit.md`, tools and machine, point 3). This session moves the
event vocabulary to version 2 once, for every change of this plan, then
names the values and derives what depends on them. Nothing here changes
what a model is sent.

## Read first

- smith `docs/design/protocol/events.md`, sections 3.6 (`tool.completed`,
  `job.ended`), 5 and 6; `docs/design/protocol/limits.md`, sections 2.2,
  2.3, 3.5, 4 (`machine-holds-tools`, `deadlines`), 5, 7 and 9.
- smith `docs/design/shell.md`, sections 6.3 and 6.5;
  `docs/design/domain/session.md`, section 5 (batches);
  `docs/design/domain/tools.md`, section 5 (a job's bound).
- `decisions.md` 5.2 and 6.5.
- skein `programming-model.md`, sections 6.3 and 7;
  `testing-strategy.md`, sections 2.1 and 6.

## What the design asks

- **`tool.completed`** gains `job`, an integer, for a `shell` call
  answered `running` and a `job` call not answered `invalid`; `verdict`
  gains `running` and `stopped`. **`job.ended`** is a record of its own:
  `run`, `conversation`, `job`, `call`, `verdict` (`exited`, `timed_out`,
  `cancelled`), `exit`, `unread_bytes`, `duration_ms` (events.md 3.6).
  Each is a new version of the vocabulary (events.md 6), and so is each
  new name among the effective limits (limits.md 9).
- **`parallel_tools`**, a policy value from 1 to 8, shipped at 4: the
  most adjacent reads run as one batch; a session uses the smaller of it
  and `calls_per_response` (limits.md 2.3; session.md 5).
- **`jobs`**, declared, shipped at 4: the most jobs one session holds
  (limits.md 2.2). **`job_wait`**, policy, 30 s: the ceiling of a yield
  and the default and ceiling of a wait; `job_wait ≤ tool_deadline`
  (limits.md 2.3, 4).
- **The machine's processes** hold at least a batch of at most
  `parallel_tools` in each of `conversations`, the checks, and beside
  them `jobs` for each of `conversations`; a job's kept output is
  `shell_head + shell_tail` (limits.md 3.5); `worst_case` counts them
  (limits.md 5); `machine-holds-tools` checks the processes (limits.md 4).

## Choices this plan makes

- 08.1 is the vocabulary's one move to version 2 (README, rule 12):
  08.2's and 08.3's new names, and 10.3's and 10.4's records, land under
  it. A producer of `job.ended` comes in 10.3; until then the codec is
  tested alone, as every record type is first.
- The machine's process bound is derived as `conversations ×
  parallel_tools + checks + conversations × jobs`, the checks counted as
  the run's checks in flight at once (one per directory with checks).

## Rules that bind it

- **Formats move once** (README, rule 12): 08.1 moves the vocabulary; a
  version's sweep is implementer.md 8.2's.
- **Users outside Files** (rule 3), for every `Limits` literal that gains
  a field (the audit lists them: `findings/planning-audit.md`, tools and
  machine, point 5).
- **Dependents' tests** (rule 6): one `rdeps` for each crate an increment
  changes, `smith-events` and `smith-agent-service` among them.

## Increments

### 08.1 The event vocabulary's version 2
Scope: events.md 3.6 and 6 (decisions.md 5.2).
Needs: nothing.
Files: `crates/smith-events/src/` (`records.rs` `ToolCompleted`, `:888-902`,
  and `Verdict`, `:199-267`; `write.rs` `:83`, `:97`, `:119`, `:260-296`,
  `:881`, `:1364`, `:1889`; `read.rs` `:115`, `:132`, `:557`, `:1377`;
  `limits.rs` `:34`, `:65`, `:98`, `:311`, `:813`; `lib.rs` `VERSION`,
  `:34`; `tests.rs` `:93`), `crates/smith-events/golden/v1/` (moved to
  `golden/v2/`), `tests/codecs/tests/drift.rs` (`:53`),
  `tests/codecs/tests/events.rs` (`:23`, `:264`, `:305`, `:914`, `:958`,
  `:974`, `:1005`), `tests/codecs/tests/support/events.rs` (`samples`,
  `value_tool_completed` `:579`), `crates/smith-agent-service/src/sinks/view.rs`
  (`:268`), `crates/smith-agent-service/src/calls.rs` (`verdict_kind`,
  `:426`), `benchmarks/src/agent/smith/events.rs` and the files naming
  version 1 (`rg -l '"v":1' benchmarks/tests tests crates --glob
  '!crates/smith-events/golden/**'`).
Shape: as sketched below; `VERSION = 2`; every golden rewritten under
  `golden/v2/`, with `job.ended`'s and the new verdicts' samples added;
  the refusal test's other version is 3.
Test shape: `tests/codecs/tests/events.rs`: `a_job_ended_record_round_trips_its_fields`,
  `a_tool_completed_with_a_job_round_trips`, and the drift test over
  `golden/v2`, named `event_records_and_every_listed_value_match_frozen_version_two_goldens`
  (the version's sweep). The two round trips under 20 ms each; the drift
  test, a focused world test over the whole vocabulary, under 0.5 s.
Proof: unit and codecs, those tests.
Don't: produce a `job.ended` or a `running` verdict anywhere yet.
Size: 200 lines, goldens not counted.
Removes: `golden/v1/`.

### 08.2 `parallel_tools`, named
Scope: limits.md 2.3; shell.md 6.5; session.md 5.
Needs: 08.1 (its name joins the vocabulary).
Files: `crates/smith-agent-service/src/profile/quantities.rs` (`Policy`,
  `:58-103`; `standard()`, `:153-197`), `crates/smith-agent-service/src/profile.rs`
  (`:521`), `crates/smith-domain-session/src/limits.rs` (`:286`, `:117`
  `MAX_PARALLEL`), `crates/smith-domain-tools/src/limits.rs` (`calls`,
  `:247`), `crates/smith-agent-service/src/profile/list.rs` (`policy()`,
  `:120-170`), `crates/smith-local-shell/src/settings/schema.rs` (`:41-63`),
  `crates/smith-local-shell/src/settings/resolve/document.rs` (`:9-38`),
  `crates/smith-agent-shell/src/config.rs` (`:63-96`, `:570-592`,
  `:744-748`), `tests/session/tests/` (a batch story).
Shape: `Policy.parallel_tools: u32` (4, from 1 to 8, refused outside);
  each of the three literals becomes `min(parallel_tools,
  calls_per_response)`, passed in through each `Derivation`; the setting
  `parallel_tools` and its printed name `policy.parallel_tools`.
Test shape: `more_adjacent_reads_than_parallel_tools_run_in_the_next_batch`
  (session world, `parallel_tools` 2 and five reads; under 0.5 s),
  `parallel_tools_outside_one_to_eight_is_refused` (profile; under 20 ms).
Proof: world, those tests; behaviour at the shipped 4 is unchanged.
Don't: change a batch's order or the write-alone rule.
Size: 80 lines.
Removes: the three literal 4s.

### 08.3 `jobs` and `job_wait`, and what they derive
Scope: limits.md 2.2, 2.3, 3.5, 4, 5 and 7; shell.md 6.3 and 6.5.
Needs: 08.2.
Files: `crates/smith-agent-service/src/profile/quantities.rs`
  (`Declared`, `:25-54`; `Policy`; `standard()`),
  `crates/smith-agent-service/src/profile.rs` (`:107`, `:455`, `:630`),
  `crates/smith-agent-service/src/profile/validate.rs` (`deadlines`,
  `:80-125`; `machine`, `:229`), `crates/smith-agent-service/src/profile/list.rs`
  (`declared()`, `:86-118`; `policy()`), `crates/smith-agent-service/src/profile/effective/`
  (its tests print the names), `crates/smith-domain-tools/src/limits.rs`
  (`Limits`, `:19`; `Derivation`, `:191`), `crates/smith-protocol-machine/src/limits.rs`
  (`processes`, `:167`; `worst_case`, `:202`; `Limits` and `Derivation`
  gain `tool_payload`, ruling 110), `crates/smith-agent-service/src/profile/checks.rs`
  and `validate.rs` (`MachineHoldsTools`), `profile/effective/boundaries.rs`
  and `list.rs` (the printed `machine.tool_payload`), the settings and agent
  configuration files of 08.2, and each `Limits` literal the audit lists
  (rule 3).
Shape: `Declared.jobs: u32` (4) and `Policy.job_wait: Duration` (30 s),
  as sketched below; the tools domain's `Limits` gain `jobs` and
  `job_wait`; the machine's `processes` derived as above, replacing
  `min(operations, 8)`; its `worst_case` adds `conversations × jobs`
  kept outputs at `shell_head + shell_tail` and their process records;
  `Check::Deadlines` refuses `job_wait` of zero or above `tool_deadline`;
  `Check::MachineHoldsTools` checks the processes; printed as
  `declared.jobs` and `policy.job_wait`. The machine's `Limits` and
  `Derivation` gain `tool_payload`, derived from the tools' `tool_payload`
  (limits.md 2.2: "the arguments of one call"); `MachineHoldsTools`
  checks it, the machine refuses translated arguments above it, and a
  listing's or search's filter and pattern are priced at it, not at
  `file_bytes`; printed as `machine.tool_payload`. Shipped memory stays
  (decisions.md 7.4).
Test shape: profile tests: `the_machines_processes_hold_every_batch_check_and_job`
  (a derivation property, `conversations` 3), `a_job_wait_above_the_tool_deadline_is_refused`,
  `the_worst_case_counts_each_sessions_jobs_at_their_kept_output`. Under
  20 ms each.
Proof: unit, those tests, and `crates/smith-agent-service/src/profile/effective/tests.rs`
  with the new names among the printed effective limits.
Don't: let a job's values reach a tool yet.
Size: 160 lines.
Removes: the bound of 8 processes.

### 08.s1 Every probe's overrides start within shipped memory
Scope: limits.md 3.5 and 5 (the worst case within `declared.memory`);
  ruling 114. On `main` at `25388ac8`, `first-request`'s overrides refuse
  at startup: "service.worst_case=2147682394 bytes; declared.memory=2147462772
  bytes", 219,622 bytes over (`probes-first-request-25388ac8-20261014T192031Z/01/stderr.log`).
  The smoke's own configuration refuses too, by more: "service.worst_case=2147712231
  bytes; declared.memory=2147462772 bytes", 249,459 bytes over
  (`probes-smoke-tools-25388ac8-20261014T192551Z/01/stderr.log`; ruling
  115). Each of 07.1 (`bde09510`) and 08.3 (`257a41bd`) passed its
  smoke on a tree without the other; start the search with their
  combination.
Needs: 08.3.
Budget: 3 runs, 60 minutes (README rule 5), to find the priced term that
  grew past the bound.
Files: `crates/smith-protocol-machine/src/limits.rs`, `crates/smith-agent-service/src/profile/`,
  `benchmarks/tests/smith_adapter.rs` (beside `:449` and `:464`).
Shape: the worst case of every probe task's `[smith]` overrides fits the
  shipped memory, by removing an overcount a heap test shows (as ruling
  110 did), with no preset, no `declared.memory` and no expected value
  changed. If no such overcount closes the gap, stop on a question with
  the term and its bytes.
Test shape: `every_probe_tasks_overrides_pass_the_actual_public_startup_derivation`
  (one outcome over each `benchmarks/tasks/probes/*/task.toml`, as `:449`
  does for one profile), and the heap test that shows the overcount.
  Under 0.5 s each.
Proof: unit, those tests; the smoke (rule 8), which must pass on its
  branch: until 08.s1 lands, it is the only smith code landing that runs
  the smoke (ruling 115).
Don't: raise memory or change a probe.
Size: 60 lines.
Removes: the overcount.

## Interface sketches

```rust
// smith_events::records (08.1). ToolCompleted gains, after `exit`:
//     pub job: Option<u64>,
// Verdict gains `Running` and `Stopped`.

/// A job whose end no call's answer reported, told as its session closes.
pub struct JobEnded {
    pub run: u64,
    pub conversation: u64,
    pub job: u64,
    /// The provider's id for the `shell` call that answered it `running`.
    pub call: Box<[u8]>,
    /// `Exited`, `TimedOut` or `Cancelled`.
    pub verdict: Verdict,
    pub exit: Option<CommandExit>,
    /// What it wrote after the last answer that named it.
    pub unread_bytes: u64,
    pub duration_ms: u64,
}
// Event gains `JobEnded(JobEnded)`, written as `job.ended`.

// smith_agent_service::profile::quantities (08.2, 08.3).
// Declared gains:  pub jobs: u32,               // 4
// Policy gains:    pub job_wait: Duration,      // 30 s
//                  pub parallel_tools: u32,     // 4, from 1 to 8
```

## Done when

- `rg -n 'pub const VERSION: u64 = 2' crates/smith-events/src/lib.rs`
  prints one line.
- `rg -n 'min\(8\)|\.min\(4\)' crates/smith-protocol-machine/src/limits.rs crates/smith-domain-session/src/limits.rs crates/smith-domain-tools/src/limits.rs crates/smith-agent-service/src/profile.rs`
  prints nothing.
