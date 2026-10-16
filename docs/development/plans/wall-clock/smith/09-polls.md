# Session 09: a poll beside other work

2026-10-16. Cause 4 of `findings/causes.md`: blocking on cargo, about 26 s
an attempt: a `job` poll waits its own `job_wait` (30 s by default)
whatever its batch holds, so a poll beside reads costs its wait, where
Codex polls at one second inside its next read. The design has a job's
wait give way to its batch: a `job` call beside other calls answers once
every call of its batch that is not a `job` call has answered, `running`
if its job runs on; alone, it waits as it asks. The role's topic 5 sends
polls beside other work (decisions.md, section 7). Nothing ends a wait
early today: the machine waits until the job ends or the wait's time
passes (findings A23). This step changes what a tool answers when a
call asks for nothing, so it is proven by its `parity-change` run.

## Read first

- smith `docs/design/domain/session.md`, section 5 ("A job's wait gives
  way to its batch") and section 10's job stories.
- smith `docs/design/domain/tools.md`, sections 3 (`job`), 5 ("A wait
  ended early") and 7 (its world's stories).
- smith `docs/design/protocol/llm.md`, section 3 (`job`'s description).
- smith `docs/design/shell.md`, section 6.6 (topics 3 and 5).
- `decisions.md`, section 7; `findings/planning-audit.md` A23, A25, A26.
- skein `programming-model.md`, sections 3 and 4; `testing-strategy.md`,
  sections 3 and 5.

## What the design asks

- **The session ends a `job` call's wait** once every call beside it in
  its batch that is not a `job` call has answered; the call then answers
  at once, as its wait's end would: the output since its last answer,
  then `running`, or the end if the job has ended. A batch of `job` calls
  alone waits as each call asks (session.md 5; tools.md 5).
- **The description:** that `job`, called with reads, adjacent in one
  response, waits no longer than they run, so such a poll costs nothing
  (llm.md 3).
- **The role:** topic 3 batches a running job's poll beside the reads,
  lists, searches and commands that change nothing; topic 5 runs long
  commands as jobs whose polls go beside other work (shell.md 6.6).

## Choices this plan makes

- The session tells the tools `Event::EndWait { call }` for each `job`
  call still waiting once its batch's last other call has answered. The
  tools end the call's read: an active one through a new machine act,
  `JobAct::Hasten`, which sets its deadline to now; one queued behind
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
  lists and searches;": " beside them it waits no longer than they run,
  so such a poll costs nothing;".

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
  `:1755-1777`; `advance`, `:1596`), `crates/smith-domain-tools/src/boundary.rs`
  (`Event`, `:39-70`; `JobAct`, `:127`), `crates/smith-domain-tools/src/jobs.rs`
  (`:118`, `:173-190`), `crates/smith-domain-tools/src/kit.rs`
  (`:135-144`), `crates/smith-protocol-machine/src/process/jobs.rs`
  (`:37-61`, `:112-125`), `crates/smith-domain/src/operation.rs`
  (`:54-72`), `crates/smith-protocol-llm/src/descriptions.rs` (`:78-82`),
  `crates/smith-protocol-llm/src/tests.rs` (`:343-367`),
  `tests/protocol-llm/tests/production_tools.rs` (`:410-419`),
  `crates/smith-local-shell/src/role.rs`, the job fakes
  `tests/tools/src/world/jobs.rs` (`:66-87`), `tests/agent/src/world/jobs.rs`,
  `tests/protocol-llm/src/world/jobs.rs`, `tests/session/src/recorded.rs`
  (`:341-345`), and every match on `JobAct` `rg -n 'JobAct::' crates tests`
  lists; `benchmarks/summaries/`.
Shape: the session, when a batch that held a call other than `job` has
  no such call left running, gives the tools `Event::EndWait { call }`
  for each `job` call of the batch without `stop` still waiting. The
  tools' `jobs` ends that call's read: active, it sends the machine
  `JobAct::Hasten { job }`; queued, it marks the read to answer at once
  when it starts. The machine answers a hastened read at once with the
  output since its last answer and `running`, or the end. A batch of
  `job` calls alone sends nothing, and each waits as it asks; a `job`
  with `stop` is a write and never in such a batch.
Test shape: `tests/tools/tests/jobs.rs`, beside
  `different_jobs_can_be_read_in_the_same_batch` (`:460`):
  `a_wait_ended_early_answers_running_with_the_output_so_far`,
  `a_wait_ended_after_the_job_ended_answers_its_end`,
  `a_queued_read_ended_early_answers_when_it_starts`.
  `tests/session/tests/jobs.rs`, beside
  `a_job_with_stop_runs_alone_in_its_batch` (`:176`):
  `a_job_call_beside_reads_answers_once_they_have_answered`,
  `job_calls_alone_wait_as_each_asks`. In the machine's tests:
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
  stop lines, README rule 13's counts beside, with the report's tools seconds per arm, and names cause 4.
Don't: shorten `job_wait`; tell a job's end between completions
  (decisions.md 7, deferred).
Size: 220 lines.
Removes: `tests/session/src/world.rs:1209`'s `unreachable!` when a test
  of this step reaches that arm, named in the commit body; otherwise it
  stays.

## Questions to raise when reached

- None known.

## Done when

- `cd ~/src/rust/smith && rg -n 'Hasten' crates/smith-domain-tools/src/boundary.rs` prints a line, or `awk -F'\t' '$1 == "smith-09.1" { print $4 }' ~/src/rust/tmp/smith/wall-clock/ledger.tsv` prints `dropped`.
