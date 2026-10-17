# Session 09: a poll beside other work

2026-10-16. Cause 4 of `findings/causes.md`: blocking on cargo, about 26 s
an attempt: a `job` poll waits its own `job_wait` (30 s by default)
whatever its batch holds, so a poll beside reads costs its wait, where
Codex polls at one second inside its next read. The design has a job's
wait give way to the rest of its response: a `job` call beside other
calls answers once every call of its batch that is not a `job` call has
answered, or as soon as it starts in a batch of `job` calls only, as
after a write or a command, `running` if its job runs on; in a response
of `job` calls alone, it waits as it asks. The role's topic 5 sends
polls beside other work (decisions.md, section 7). Nothing ends a wait
early today: the machine waits until the job ends or the wait's time
passes (findings A23). This step changes what a tool answers when a
call asks for nothing, so it is proven by its `parity-change` run.

In the baseline, `parity-c10edbe5-20261016T184102Z`, smith's `shell-cwd`
attempts spent a median 73.1 seconds in tools to Codex's 45.7: this step
addresses RT. No smith fixture attempt made a `job` call, so it does not
address FT.

## Read first

- smith `docs/design/domain/session.md`, section 5 ("A job's wait gives
  way to the rest of its response") and section 10's job stories.
- smith `docs/design/domain/tools.md`, sections 3 (`job`), 5 ("A wait
  ended early") and 7 (its world's stories).
- smith `docs/design/protocol/llm.md`, section 3 (`job`'s description).
- smith `docs/design/shell.md`, section 6.6 (topics 3 and 5).
- `decisions.md`, section 7; `findings/planning-audit.md` A23, A25, A26.
- skein `programming-model.md`, sections 3 and 4; `testing-strategy.md`,
  sections 3 and 5.

## What the design asks

- **The session ends a `job` call's wait** once every call beside it in
  its batch that is not a `job` call has answered, and as soon as it
  starts in a batch of `job` calls only whose response holds another
  call; the call then answers at once, as its wait's end would: the
  output since its last answer, then `running`, or the end if the job
  has ended. A response of `job` calls alone waits as each call asks
  (session.md 5: "A `job` call waits as it asks only in a response whose
  calls are all `job` calls"; tools.md 5).
- **The description:** that `job`, beside any other call of its
  response, waits no longer than they run, so such a poll costs nothing,
  and that it waits as it asks only in a response of `job` calls alone
  (llm.md 3).
- **The role:** topic 3 batches a running job's poll beside the reads,
  lists, searches and commands that change nothing; topic 5 runs long
  commands as jobs whose polls go beside other work (shell.md 6.6).

## Choices this plan makes

- The session tells the tools `Event::EndWait { kit: Token, call: Token }`
  for each `job` call still waiting once its batch's last other call has
  answered, and, in a completion that holds a call other than `job`, for
  each `job` call of a batch of `job` calls only as soon as the batch
  starts; the tools find the read by `reply_to == ReplyTo::new(call)`,
  as `CancelJobs { kit }` names its kit. The tools end the call's read:
  an active one through a new machine act, `JobAct::Hasten`, a unit
  variant (`Request::Job { owner }` names the read), which sets its
  deadline to now; one queued behind
  another read of the same job answers at once when it starts. The
  root's `Long` span for the read ends with it.
- The role's new words: a sentence after paragraph 3's first sentence
  (08.1's, or `main`'s if 08.1 was dropped), and, in paragraph 5, in
  place of "Only for a command expected to run long, set yield_seconds,
  continue independent work, then use `job` to read the output and the
  end.":

  > Poll a running job with `job` beside those calls; it waits no longer
  > than they run.

  > Only for a command expected to run long, set yield_seconds, continue
  > independent work, and poll the job with `job` beside that work until
  > it ends.

- `job`'s description gains, after "Without stop, read beside reads,
  lists and searches;", these 132 bytes: " beside other calls it waits
  no longer than they run, so such a poll costs nothing; only with job
  calls alone does it wait as asked;".
- The host part holds them because the check counts the task role as
  written (protocol/limits.md 4, `host-part-holds-the-role`). 08.1 makes
  that change (its Choices), landed in `d3cd7d31`: with this step's
  words the role is 1,915 bytes, `role_and_guides` 35,345, and the slack
  about 1,887 bytes (the ready check's re-check, 2026-10-17).
- The role's words stay as above: with the session's rule, a poll beside
  a `shell` call that changes nothing, or beside a change, waits no
  longer than that call runs. The session never asks what a command
  changes (designer's answer of 2026-10-17).

## Rules that bind it

- **One change at a time** (README, rule 11), **the stop** (rule 12),
  **no split** (rule 1).
- **No change to `job_wait`'s default** (`code-organization.md`, section
  7).

## Increments

### 09.1 A job's wait gives way to its batch
Scope: session.md 5; tools.md 3, 5 and 7's story "a wait ended early by
  its session"; llm.md 3's `job`; shell.md 6.6 topics 3 and 5 (findings
  A23; cause 4).
Needs: smith 08's batched change [08.1] landed or dropped.
Files: `crates/smith-domain-session/src/session.rs` (`tool_ran`,
  `:1782-1805`; `advance`, `:1514`), `crates/smith-domain-tools/src/boundary.rs`
  (`Event`, `:39-70`; `JobAct`, `:127`), `crates/smith-domain-tools/src/domain.rs`
  (`match event`, `:92`), `crates/smith-domain-tools/src/jobs.rs`
  (`:118`, `:173-190`), `crates/smith-domain-tools/src/kit.rs`
  (`:135-144`), `crates/smith-protocol-machine/src/process/jobs.rs`
  (`:37-61`, `:112-125`), `crates/smith-protocol-machine/src/component.rs`
  (`:112`), `crates/smith-domain/src/operation.rs`
  (`:54-72`), `crates/smith-inline-agent/src/workspace.rs` (`:164-172`),
  `crates/smith-protocol-llm/src/descriptions.rs` (`:78-82`),
  `crates/smith-protocol-llm/src/tests.rs` (`:343-367`),
  `tests/protocol-llm/tests/production_tools.rs` (`:410-419`),
  `crates/smith-local-shell/src/role.rs`, the job fakes
  `tests/tools/src/world/jobs.rs` (`:66-87`, the `assert!` at `:68`),
  `tests/agent/src/world/jobs.rs` (`:63`),
  `tests/protocol-llm/src/world/jobs.rs` (`:61`), `tests/tools/src/world.rs`
  (`:508`, and a script `Step` that sends `EndWait`),
  `tests/agent/src/memory/random.rs` (`:221-226`),
  `tests/session/src/recorded.rs` (`:341-345`), `tests/machine/src/lib.rs`
  (the sequence driver, `:845-851`), `tests/machine/src/jobs.rs`
  (`:36`, the terminals expected), `tests/session/tests/jobs.rs`
  (`read_and_edit`, `:110`, completes `[read, edit, job]`, the new case,
  so its poll is hastened; `a_command_answered_running_while_the_session_reads_edits_and_completes`,
  `:148`, still passes because `recorded.rs` keeps hastened acts apart:
  name it in the commit body), and every further match on `JobAct` `rg -n 'JobAct::' crates
  tests` lists; `benchmarks/summaries/`.
Shape: the session, when a batch that held a call other than `job` has
  no such call left running, gives the tools `Event::EndWait { kit, call }`
  for each `job` call of the batch without `stop` still waiting. When it
  starts a batch of `job` calls only, it gives each its `EndWait` at once
  if the completion holds a call other than `job`, read from the
  completion's call blocks whatever their answers (invalid, not run, a
  delegated or a host tool's call included). The
  tools' `jobs` ends that call's read: active, it sends the machine
  `JobAct::Hasten`; queued, it marks the read to answer at once
  when it starts. The machine answers a hastened read at once with the
  output since its last answer and `running`, or the end. A completion
  of `job` calls alone sends nothing, and each waits as it asks; a `job`
  with `stop` is a write and never in such a batch.
  A Hasten owes no terminal of its own, and each place that holds one
  act per owner or per job lets it pass: `operation.rs` returns before
  `commands.insert` (`:67`, "one active job act per owner");
  `process/jobs.rs` takes it in `Read` and ignores it in `Held`
  (`:39-41`'s `unreachable!`); `workspace.rs` gives it an early branch
  before the callback right (`:164-172`, "root machine requests own one
  callback right"); the job fakes take it beside their `assert!(…Held…)`;
  `recorded.rs` keeps hastened acts apart from the one pending act per
  owner; the machine's sequence driver sends the next act without
  waiting for a Hasten's terminal, and counts none for it.
  `advance` stands at 100 of `too_many_lines`' 100 on `d3cd7d31`: each
  line this step adds to it needs an extracted helper (the check moved
  the `Invalid` arm's four lines into `batching::invalid_slot`, which
  cleared it); aim for no new `#[expect]`; one left names this step in
  its `reason` and in the commit body. `crates/clippy.toml` disallows
  `matches!` (`:84`) and `Iterator::any` (`:145`): a `match` or a `for` loop.
Test shape: `tests/tools/tests/jobs.rs`, beside
  `different_jobs_can_be_read_in_the_same_batch` (`:460`):
  `a_wait_ended_early_answers_running_with_the_output_so_far`,
  `a_wait_ended_after_the_job_ended_answers_its_end`,
  `a_queued_read_ended_early_answers_when_it_starts`.
  `tests/session/tests/jobs.rs`, beside
  `a_job_with_stop_runs_alone_in_its_batch` (`:176`):
  `a_job_call_beside_reads_answers_once_they_have_answered`,
  `a_job_call_in_a_batch_of_its_own_beside_other_calls_answers_at_once`
  (one table test, its rows a `job` call before an `edit`, and one after
  a `shell` call, in one completion),
  `job_calls_alone_wait_as_each_asks` (a completion of two `job` calls).
  In the machine's tests:
  `a_hastened_read_answers_at_once`. Unit tests under 20 ms; world tests
  under 0.5 s.
Proof: live, through the stop: `heavy cargo run -p smith-bench -- build
  HEAD` and `build origin/main`, then `heavy --gate cargo run -p
  smith-bench -- suite parity-change --arm <HEAD's dir> --parent-arm
  <origin/main's dir> --baseline-run <the file README rule 13's first
  command prints> --landed <each file its second prints>`: 40 attempts,
  about 3 hours, run once; its exit decides (README, rule 12). Its
  summary is amended into the step's commit and named in
  `Change-Summary:`; the commit body gives its criteria, projection and
  stop lines, README rule 13's counts beside, with the report's tools seconds per arm, and names cause 4
  and the criterion it addresses, RT, with the baseline's figures above.
Stopped and kept: `parity-change-2318318b-20261017T065453Z` stopped on
  RC (projection 1.853, above the baseline's 1.734); the designer's
  answer of 2026-10-17T09:35Z keeps 09.1 (README rule 12). The run
  stands: `c94fcaaa` is rebased onto commits that change no prompt file
  (rule 11), and the body names `2318318b` and `c36fa508`. The body
  gives this cause for the move, as written: "RC's move is noise, not
  this change. Before its first compaction the change's arm read as its
  parent did, 59.8 reads an attempt against 58.0; after it, 21.8
  against 11.8, and it compacted in 5 attempts of 5, its parent in 3.
  While a job ran, it made 1.88 responses a job against 1.97. Its 46
  polls beside another call waited 43 ms in all; its parent's 31
  waited 48.0 s (cause 4). Its parent's code, measured as 08.1's arm, made 63.2
  responses an attempt at a median $1.510; here, 52.4 at $1.291."
Don't: shorten `job_wait`; tell a job's end between completions
  (decisions.md 7, deferred).
Size: 240 lines.
Removes: `tests/session/src/world.rs:1197`'s `unreachable!` (and its copies at `:2128` and `tests/channel/tests/composed/long.rs:95`) when a test
  of this step reaches that arm, named in the commit body; otherwise it
  stays.

## Questions to raise when reached

- None known.

## Done when

- `cd ~/src/rust/smith && rg -n 'Hasten' crates/smith-domain-tools/src/boundary.rs` prints a line, or `awk -F'\t' '$1 == "smith-09.1" { print $4 }' ~/src/rust/tmp/smith/wall-clock/ledger.tsv` prints `dropped`.
