# Session 10: jobs in the tools domain, the session and the run

2026-10-14. On the repository task smith spent 167 s an attempt blocked in
shell commands against Codex's 39, because Codex starts its builds in the
background and polls them (findings W1). The design now gives `shell` a
yield and a `job` tool (domain/tools.md 5). This session builds jobs from
the tools domain up to the run, over 09.2's held jobs; 11.2 offers them to
the model. No model sees a job before 11.2.

## Read first

- smith `docs/design/domain/tools.md`, sections 3 (the tool table), 4
  (`shell` and `job`), 5 (jobs) and 7 (the jobs list);
  `docs/design/domain/session.md`, sections 5, 6 and 10 (the jobs
  stories); `docs/design/domain/run.md`, sections 5.3 ("Jobs give way",
  "A child's jobs end with it"), 8.1 ("Jobs are cancelled first", the
  long operations), 9.1, 11 and 13 (its jobs list);
  `docs/design/domain/host.md`, section 4 (long operations);
  `docs/design/protocol/events.md`, sections 3.6 and 5 (one terminal per
  job); `docs/design/testing.md`, section 6.
- `decisions.md` 5.1 and 5.2; `findings/time.md` W1.
- skein `programming-model.md`, sections 4.5, 5.2, 5.3, 5.4 and 6.3;
  `testing-strategy.md`, sections 2.2, 6 and 7.

## What the design asks

- **A job** is a command a `shell` call left running past its yield
  (`yield_seconds`, clamped to `job_wait`): answered `running` with its
  number and output so far, it goes on under its deadline, counted from
  its start. A session numbers its jobs from 1 and never reuses one. A
  `job` call answers the output since its last answer, then `running` or
  the end (`exited`, `timed_out`, `stopped`, `cancelled`), waiting up to
  `wait_seconds` (default and ceiling `job_wait`); with `stop`, it stops
  the job first. At most `jobs` run per session, a job counting from its
  `running` answer until an answer reports its end or its session closes;
  a `shell` call with a yield past that, a `job` call naming a number
  never answered `running` or an end already answered, are invalid calls
  (tools.md 5).
- **Jobs run beside the turns:** a running job holds no place in a later
  batch; `job` is a read, and a write with `stop` (session.md 5).
- **A session's jobs are its writes in progress:** whatever takes the
  lease first cancels the running jobs of every other session, a check or
  a delivery cancels every running job, and each waits for them to
  settle; a session's close cancels its jobs, waits, then tells a
  `job.ended` for each job whose end no answer reported (tools.md 5;
  session.md 5; run.md 5.3, 8.1).
- **Every job's end is told once,** by an answer or by `job.ended`, before
  its conversation's `conversation.closed` (events.md 5).
- **A job charges no turn, token or spend;** a `job` call's wait is a long
  operation, a job running between calls is not (session.md 6; host.md 4;
  run.md 8.1).

## Choices this plan makes

- The session's job table lives in the tools domain's kit, beside its
  flights: `crates/smith-domain-tools/src/jobs.rs`, each job's number,
  the provider's call id, its start and whether its end has come, a cell
  each (limits.md 3's "a session's jobs").
- A cancel of jobs is one tools-domain request from the session, `Cancel
  jobs`, answered once every cancelled job has settled; the session's
  opener and its close use it.

## Rules that bind it

- **Users outside Files** (README, rule 3), for the arms each new variant
  needs across the session, the run and the service.
- **Dependents' tests** (rule 6): `rdeps(smith-domain-tools)`,
  `rdeps(smith-domain-session)`, `rdeps(smith-domain-run)`.
- **Splits** (rule 1): 10.1 and 10.3 are the largest.

## Increments

### 10.1 A command left running, and read
Scope: tools.md 4 and 5 (a job's start, number, reading, bound, invalid
  calls, clamps); limits.md 3's session jobs.
Needs: smith 08's `jobs` and `job_wait` [08.3]; smith 09's held jobs
  [09.2]; smith 11's transcript version 4 [11.1].
Files: `crates/smith-domain-tools/src/` (`call.rs` `Call::Shell`
  `:44-51`, `Tool`, `effect` `:212`, `Outcome` `:218`; `kit.rs` `admit`
  `:326`; `jobs.rs`, new; `flight.rs`; `command.rs`; `facts.rs`
  `Verdict` `:196`; `limits.rs`), `crates/smith-agent-service/src/calls.rs`
  (`verdict_kind`, `:426`: `running`, `stopped`, and a job's end),
  `crates/smith-protocol-llm/src/` (`render.rs` `:154`, `:214-246`;
  `limits.rs` `render_worst_case` `:147-176`; `tools.rs`, the name `job`;
  `tests.rs` `random_outcomes` `:111`), `crates/smith-domain-session/src/record.rs`
  and `crates/smith-protocol-channel/src/turn.rs` (the outcomes, to
  version 4's), `tests/tools/src/` (`world.rs` `Work` `:210`,
  `start` `:667`, `deliver` `:594`; `translate.rs`; `fixture.rs`),
  `tests/tools/tests/jobs.rs` (new).
Shape: as sketched below: `Call::Shell` gains `after: Yield`, `Call::Job`
  and `Tool::Job` are new, `effect(Job { stop: false })` is a read and
  with `stop` a write; `Outcome::Running` and `Outcome::Job`. The kit
  issues `Op::Spawn { after }` and `Request::Job`, numbers each job on its
  `Held`, and answers the invalid calls tools.md 5 lists, saying at most
  `jobs` may run and that `job` with `stop` stops one. Each exhaustive
  match outside the kit gains its arm here: the LLM component renders
  `Running` and `Job` as llm.md 5 says and names the tool `job` (it is
  offered, with its schema and description, only in 11.2); the session's
  record maps them to transcript version 4's outcomes; `verdict_kind`
  maps them to `running`, `stopped` or the end's verdict.
Test shape: `tests/tools/tests/jobs.rs`: `a_command_past_its_yield_is_answered_running_then_read_to_its_end`,
  `a_jobs_read_past_its_kept_ceiling_counts_the_dropped_bytes_once`,
  `a_yield_and_a_wait_past_job_wait_are_clamped_and_said`,
  `reads_edits_and_commands_run_while_a_job_runs`,
  `a_yield_while_jobs_jobs_run_is_an_invalid_call`,
  `a_job_call_naming_an_unknown_or_answered_job_is_invalid`. Under 0.5 s
  each. Reference test:

  ```rust
  #[test]
  fn a_command_past_its_yield_is_answered_running_then_read_to_its_end() {
      let script = vec![Step::Calls(vec![shell_after(b"cargo build", 1)]), Step::Calls(vec![job(1, 5)])];
      let (answers, _world) = run(Settings::calm(1), BUILD, script);
      let running = Outcome::Running { job: JobNumber(1), head: b"Compiling\n".into(), tail: Box::default(), dropped: 0, output: BytesClamp::Within, after: TimeoutClamp::Within };
      let ended = Outcome::Job { job: JobNumber(1), head: b"Finished\n".into(), tail: Box::default(), dropped: 0, output: BytesClamp::Within, wait: TimeoutClamp::Within, end: JobEnd::Ended(Exit::Code { code: 0 }) };
      assert_eq!(answers, vec![running, ended]);
  }
  ```
  `Step`, `run` and `Settings::calm` are `tests/tools`' (`tests/simulation.rs:48-58`,
  `src/world.rs:59`); `BUILD` is a checkout whose `cargo build` program
  writes "Compiling", then "Finished" two seconds later, a new
  `fixture::program` shape; `shell_after` and `job` are this increment's
  call builders.
Proof: world, those tests.
Don't: stop or cancel a job yet (10.2), or offer `job` to a model (11.2).
Size: 350 lines.
Removes: nothing.

### 10.2 A job stopped, cancelled, or ended with its session
Scope: tools.md 5 (stopping, cancelled for another writer, ended with
  its session, one terminal each, within its activation) and 7.
Needs: 10.1.
Files: `crates/smith-domain-tools/src/` (`jobs.rs`, `kit.rs`, `boundary.rs`
  `Event`/`Request` for the cancel, `facts.rs`), `tests/tools/`
  (`tests/jobs.rs`, `tests/fuzzy_*.rs`, the referee).
Shape: `job` with `stop` asks `JobAct::Stop` and answers `stopped`, or
  the end it already had. A cancel of jobs (`Event::CancelJobs` from the
  session to the tools domain, sketched below) stops every running job's
  group, and the kit answers `Request::JobsCancelled` once all have
  settled; a cancelled job's next `job` call
  answers `cancelled`. The kit's close cancels its running jobs, waits,
  and reports, as facts, each job whose end no answer reported, with its
  end as observed and its unread bytes. The tools world's referee checks
  one terminal per job and at most `jobs` running.
Test shape: `a_job_stopped_and_one_stopped_after_it_exited`,
  `a_job_past_its_deadline_answers_timed_out`,
  `jobs_cancelled_for_a_check_answer_cancelled_at_their_next_read`,
  `a_close_with_one_job_running_and_one_exited_unread_tells_each_once`;
  the fuzzy world draws jobs' polls, stops and ends among its edits (its
  run under 5 s). Under 0.5 s each.
Proof: world, those tests and the fuzzy world.
Don't: let a job outlive its kit's close.
Size: 250 lines.
Removes: nothing.

### 10.3 The session's jobs
Scope: session.md 5 and 6, and its jobs stories (10); host.md 4.
Needs: 10.2.
Files: `crates/smith-domain-session/src/` (`session.rs` `advance`
  `:1459`, `joins` `:1616`, `close` `:467`, `settle` `:1170`, `:1202`;
  `facts.rs` `FactKind` `~:148`; `boundary.rs`; `domain.rs`; `record.rs`;
  `compaction.rs`, its restore), `crates/smith-domain-tools/src/jobs.rs`
  (the first number after a resume), `tests/session/src/world.rs`, `tests/session/tests/jobs.rs` (new).
Shape: a `shell` answered `running` ends its call and holds no batch
  place; `job` without `stop` joins reads, with `stop` runs alone; the
  session asks the tools domain's cancel of jobs before another writer
  takes the lease, a check or a delivery runs, when the run asks it (the
  session's `Event::CancelJobs`, answered by its `Request::JobsCancelled`
  once they have settled, sketched below); each turn records its
  conversation's highest job number (`Turn.jobs`), and a resumed session
  numbers its next job after its last turn's; its close cancels its jobs and waits for them, then gives a `JobEnded`
  fact for each job its kit reported, before `Ended`; a job charges no
  turn; a `job` call's wait is reported as a long operation.
Test shape: `tests/session/tests/jobs.rs`:
  `a_command_answered_running_while_the_session_reads_edits_and_completes`,
  `a_job_with_stop_runs_alone_in_its_batch`,
  `jobs_cancelled_by_the_opener_before_another_writer_answer_cancelled`,
  `a_session_closing_with_a_job_running_tells_its_job_ended`,
  `a_resumed_session_numbers_its_next_job_after_its_last_turns`. Under
  0.5 s each.
Proof: world, those tests.
Don't: give a job a turn, a token or a spend.
Size: 300 lines.
Removes: nothing.

### 10.4 The run's lease and its jobs
Scope: run.md 5.3, 8.1, 9.1, 11 and 13 (its jobs list); transcript.md 4
  ("No job is resumed"); events.md 3.6 and 5; testing.md 6.
Needs: 10.3.
Files: `crates/smith-domain-run/src/` (`run.rs` `sub_agent` `:1898`,
  `:1976`, `finish` `:1767`, `:1817`, `deliver` `:1830`, `:1871`,
  `close`/`closing` `:2129`, `:2149`; `land.rs` `begin` `:53`; `lease.rs`;
  `facts.rs`), `crates/smith-agent-service/src/` (`calls.rs`, `sinks/`:
  `tool.completed`'s `job` and `job.ended` written), `tests/run/` (a
  `tests/jobs.rs`, and its referee), `tests/agent/` (its referee).
Shape: a writing `sub_agent` call asks its parent session's cancel of
  jobs before its child opens; a `finish` that delivers and a `deliver`
  ask every session's cancel before the checks start, or before the
  delivery when no directory has checks (a land stage before
  `land::begin`); a child's jobs end with its session; the service writes
  `tool.completed`'s `job`, and `job.ended` from the run's facts (10.1 owns
  the verdicts); the run's and the agent world's
  referees check one terminal per job before its `conversation.closed`.
Test shape: `tests/run/tests/jobs.rs`:
  `a_build_run_as_a_job_while_main_reads_and_edits_is_read_to_its_end`,
  `mains_jobs_are_cancelled_before_a_writing_child_opens`,
  `a_finishs_checks_and_a_deliver_run_after_mains_jobs_are_cancelled`,
  `a_writing_childs_job_ends_with_the_child`,
  `a_wind_down_with_a_job_running_is_cancelled_by_the_finish_that_delivers`,
  `a_run_parked_with_a_job_running_tells_it_ended_and_a_later_job_call_is_invalid`.
  Under 0.5 s each.
Proof: world, those tests.
Don't: change who holds the lease (run.md 15).
Size: 250 lines.
Removes: nothing.

## Interface sketches

```rust
// smith_domain_tools::call (10.1): the tools domain's jobs.
/// A job's number in its conversation: from 1, never reused.
pub struct JobNumber(pub u32);

/// How long a `shell` call waits before it answers `running`.
pub enum Yield {
    /// No yield: the call waits for the command's end.
    Never,
    /// The asked seconds, clamped to `job_wait`.
    Asked(Duration),
}

/// How long a `job` call waits for its job's end.
pub enum Wait { Default, Asked(Duration) }

// Call::Shell gains:  after: Yield,
// Call gains:         Job { job: JobNumber, wait: Wait, output: Budget, stop: bool },
// Tool gains:         Job,
// Outcome gains:
//     /// `shell`: the command still runs as `job`; its kept output so far.
//     Running { job: JobNumber, head: Box<[u8]>, tail: Box<[u8]>, dropped: u64, output: BytesClamp, after: TimeoutClamp },
//     /// `job`: the output since its last answer, then `end` (09.2's `JobEnd`).
//     Job { job: JobNumber, head: Box<[u8]>, tail: Box<[u8]>, dropped: u64, output: BytesClamp, wait: TimeoutClamp, end: JobEnd },

// smith_domain_tools::boundary (10.2): the session asks, the kit answers once its jobs have settled.
// Event (session to tools) gains:   CancelJobs,
// Request (tools to session) gains: JobsCancelled,

// smith_domain_session::boundary (10.3): the same pair, one level up; the run asks.
// Event (run to session) gains:     CancelJobs,
// Request (session to run) gains:   JobsCancelled,

// smith_domain_session (10.3): its job ends, as facts the run turns into `job.ended`.
// FactKind gains:  JobEnded { job: JobNumber, call: Box<[u8]>, end: JobEnd, unread_bytes: u64, started: Time },
```

## Done when

- `heavy cargo nextest run -p smith-tools-world -p smith-session-world -p smith-run-world -E 'binary(jobs)'`
  passes, once.
