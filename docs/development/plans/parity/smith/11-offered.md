# Session 11: jobs offered to the model

2026-10-14. Session 10 builds jobs below the LLM component; this session
records them in the transcript, offers `yield_seconds` and the `job`
tool to the model with the facts their descriptions state, and proves
them live: `tool-output/background` calibrated, and one repository check
of `shell-cwd`, five attempts per arm against Codex (decisions.md 8.1,
step 5).

## Read first

- smith `docs/design/protocol/transcript.md`, sections 2 and 4 ("No job
  is resumed"); `docs/design/protocol/llm.md`, sections 3 (the tool
  table, "The facts each description states", batching), 4 and 5
  (rendering); `docs/design/shell.md`, section 6.6 (topic 5);
  `docs/design/domain/run.md`, section 13 (the parked run);
  `docs/design/benchmarks.md`, sections 3.1, 4.2, 5.3, 11.1
  (`tool-output`) and 11.3.
- `findings/time.md` W1, section 2.4; `decisions.md` 5.1, 7.1 and 8.1.
- skein `programming-model.md`, section 8; `testing-strategy.md`,
  sections 2.5 and 6.

## What the design asks

- **The transcript** records each turn's highest job number (`jobs`,
  zero before the first), and each outcome of a command answered
  `running` and of a `job` call; a resumed session has no jobs, numbers
  its next one after the highest its last turn records, and answers a
  `job` call naming an earlier job as invalid (transcript.md 2, 4).
- **The tools' table and descriptions:** `shell` takes `yield_seconds`,
  at most `job_wait`; `job` takes `job`, and optionally `wait_seconds`,
  `output_bytes` and `stop`; their descriptions state the facts llm.md 3
  lists for them, and that `job` without `stop` is a read, runs beside
  the reads, with `stop` a write, and that a running job holds no place
  in a later batch (llm.md 3). A `running` answer and a job's read render
  as llm.md 5 says.
- **The task role's topic 5:** "running the tests the change needs, and
  long commands as jobs" (shell.md 6.6).
- **`tool-output/background`:** a command of about a minute started as a
  job, beside reads the answer also needs; checks: the command answered
  `running`, a completion started between the job's start and its end,
  one terminal for the job (benchmarks.md 11.1).

## Choices this plan makes

- The repository check is a suite of its own, `suites/repository-check.toml`
  (benchmarks.md 5.3: "A new schedule is a new file"): `repository/shell-cwd`,
  interleaved, 5 per arm, no rerun, smith (`standard`) and Codex
  (`standard`), at the working tier. Its summary gives RT's and RC's
  ratios; the plan's verdict stays 12.4's.
- 11.1 lands at the session's start, from 10.1's sketched outcomes, so
  that 10.1 maps to it and 07.2 moves the window turn under it; then the
  session waits for 11.2's Needs.
- `tool-output`'s version moves with `background`; each other variant
  whose prompt, seed and checks keep their digests keeps its block
  (benchmarks.md 3.1).

## Rules that bind it

- **Formats move once** (README, rule 12): 11.1 moves the transcript to
  version 4, with its whole sweep.
- **The baseline comes first** (rule 11): 11.2 lands after 12.1.
- **The live budget** (rule 10): 11.3's line is 24 attempts and $7.

## Increments

### 11.1 The transcript's version 4
Scope: transcript.md 2 (a turn's `jobs`, a command's `running` outcome and a
  job's read, the window turn's blocks).
Needs: nothing: its outcomes follow 10.1's sketch field for field
  (`smith/10-jobs.md`, "Interface sketches"), and 10.1 maps the domain's to
  them.
Files: `crates/smith-transcript/schema/transcript-v4.schema` (from
  `transcript-v3.schema`, `Turn` `:83-96`, `Outcome` `:50-57`),
  `crates/smith-transcript/src/generated/v4.rs`, `crates/smith-transcript/golden/v4/`,
  `crates/smith-transcript/src/lib.rs` (`FORMAT_VERSION`, `:29`, `:12`),
  `crates/smith-protocol-channel/src/turn.rs` (`encode_turn`, `:17`,
  `:69-78`), `crates/smith-domain-session/src/record.rs` (`Turn`,
  `:143-159`), `crates/smith-host-domain/src/process.rs` (`Event`,
  `:10-12`: README rule 7's one exception), `tests/codecs/tests/drift.rs` (`:22-28`),
  `tests/codecs/tests/transcript.rs`, and the
  version's sweep (`rg -n 'transcript-v3|generated/v3|golden/v3|FORMAT_VERSION'`
  over `crates` and `tests`).
Shape: `Turn` gains `jobs: u32` after `spent`; `Outcome` gains the
  running and job outcomes, field for field with 10.1's; `FORMAT_VERSION`
  = 4; generated with the Cargo.lock-pinned skein-codegen:

  ```
  rev=$(grep -A2 'name = "skein-codegen"' Cargo.lock | grep -o '#[0-9a-f]*' | tr -d '#' | head -1)
  heavy cargo run -q --manifest-path ~/.cargo/git/checkouts/skein-336d5ae5eecfa0f9/${rev:0:7}/testing/skein-codegen/Cargo.toml -- crates/smith-transcript/schema/transcript-v4.schema crates/smith-transcript/src/generated
  ```

  which writes `src/generated/v4.rs` and `golden/v4/`; `v3`'s files go.
  The session's `record::Turn` carries `jobs`, 0 until 10.3 fills it.
Test shape: `transcript_schema_matches_generated_code_and_all_goldens`
  (the drift test, over version 4: the sweep);
  `a_turn_round_trips_its_highest_job_number`, `a_running_result_round_trips`.
  The two round trips under 20 ms each; the drift test, a focused world
  test over the whole version, under 0.5 s.
Proof: codecs, those tests.
Don't: read version 3.
Size: 120 lines, generated code and goldens not counted.
Removes: `transcript-v3.schema`, `generated/v3.rs`, `golden/v3/`.

### 11.2 `yield_seconds` and the `job` tool, offered
Scope: llm.md 3, 4 and 5 for jobs; shell.md 6.6's topic 5; limits.md 10
  (the agent world's memory with jobs).
Needs: 11.1; smith 10's lease [10.4]; the baseline `parity` run's summary
  on `main` [12.1]; smith 06's task role [06.2].
Files: `crates/smith-protocol-llm/src/` (`tools.rs` `:17-41`, `:154`;
  `decode.rs` `:32-42`, `:161-163`; `descriptions.rs`; `stated.rs`
  `:10-36`; `render.rs` `:154`, `:214-246`, `:408`; `limits.rs`
  `render_worst_case` `:147-176`; `tests.rs` `random_outcomes` `:111` and
  the golden description tests), `crates/smith-local-shell/src/role.rs`,
  `tests/protocol-llm/tests/` (a jobs story in each dialect),
  `tests/agent/tests/memory.rs`.
Shape: the `shell` schema's `yield_seconds` (seconds, at most
  `job_wait`) decodes to `Yield::Asked`; the `job` tool's schema
  (`job`, `wait_seconds`, `output_bytes`, `stop`) decodes to `Call::Job`;
  `Stated` gains `job_wait` and `jobs`; the descriptions state llm.md 3's
  jobs facts with the effective values; `Outcome::Running` renders the
  job's number, its output so far and that `job` reads the rest, and
  `Outcome::Job` its output then `running` or its end; `TASK_ROLE`'s topic
  5 adds "and long commands as jobs", naming `job`.
Test shape: `tests/protocol-llm/tests/`: `a_shell_call_with_a_yield_is_answered_running_then_read_with_job_in_each_dialect`,
  `a_job_call_with_stop_is_rendered_stopped`; `each_description_states_its_facts`
  (06.1's table test) gains the jobs rows; the agent world's memory test
  attains `worst_case` with every conversation holding `jobs` jobs at
  their kept output. Under 0.5 s each; the memory test as it stands.
Proof: world, those tests; the live proof is 11.3's.
Don't: let `job` answer a job of another session.
Size: 250 lines.
Removes: nothing.

### 11.3 `tool-output/background` and the repository check
Scope: benchmarks.md 11.1's `background` and 4.2's before and after;
  decisions.md 5.1 and 8.1 (step 5).
Needs: 11.2; smith 04's calibration blocks [04.1].
Files: `benchmarks/tasks/probes/tool-output/` (`task.toml`, `seed/`),
  `benchmarks/src/agent/smith/events/` (the new checks), `benchmarks/src/check.rs`
  (the known checks), `benchmarks/tests/tool_output_probes.rs`,
  `benchmarks/suites/repository-check.toml` (new), `benchmarks/summaries/`.
Shape: `tool-output` at its next version, with a `background` variant: a
  seeded command of about 60 seconds that prints a token, started with a
  yield, beside reads of two files the answer also needs; its checks
  `job-running` (the `shell` call answered `running`), `completion-during-job`
  (a `response.started` between the job's `running` answer and its end)
  and `job-terminal` (one `tool.completed` or `job.ended` reporting its
  end), all in its `deciding`.
Test shape: `job_running_passes_on_a_shell_call_answered_running`,
  `completion_during_job_fails_when_the_session_only_waited`,
  `job_terminal_fails_on_a_job_told_twice`, on recorded or edited traces.
  Under 20 ms each.
Proof: live, once each, through `heavy --gate`, arms built first:
  `calibrate probes/tool-output/background --control <11.2's parent>
  --arm <dir>` (the control fails `job-running`; at most 14 attempts);
  then `suite repository-check --arm <dir>` (10 attempts), its summary
  committed and its RT and RC ratios, with their intervals, in the commit
  body. 24 attempts, $7, about 2.5 hours.
Don't: change the other variants' prompts, seeds or checks.
Size: 120 lines.
Removes: nothing.

### 11.s1 The inline agent routes job acts
Scope: llm.md 5's `job` and tools.md 5, through the shell's inline agent;
  task 3787's harness-error (logs/20261015T064657-3787.log).
Needs: 11.2. 11.3's live proof needs it.
Files: `crates/smith-inline-agent/src/boundary.rs` (`Lower`, `:42-69`),
  `crates/smith-inline-agent/src/translate.rs` (`:513`),
  `crates/smith-inline-agent/src/workspace.rs` (`io`, `:154-167`),
  `crates/smith-local-domain/src/domain.rs` (`:1095-1103`),
  `crates/smith-local-service/src/service.rs` (`:1330`),
  `crates/smith-inline-agent/src/tests.rs`.
Shape: `translate.rs:513` is `smith::Request::Job { .. } =>
  unreachable!("the tools issue no job act yet")` (09.2), reached since
  11.2 offers `job`: smith panics when a live model reads a job. `Lower`
  gains `Job { owner: Token, act: smith::tools::JobAct }` ("A held
  command's read or stop, routed to the machine; ends with Done"), sent
  as `Io` is (`translate.rs:510-512`), through a `workspace::job` that
  takes the same callback right as `workspace::io`, so that its `Done`
  settles as an `Io`'s does (`workspace.rs:105-120`). The local domain
  passes it down with the other `Lower` requests, and the local service
  maps it to `agent::Request::Job { owner, act }`, which the agent
  service routes to the machine (`smith-agent-service/src/service.rs:2041-2047`).
Test shape: `tests.rs`: `a_job_act_is_lowered_and_settles_with_its_done`
  (beside `workspace_routes_keep_equal_tokens_in_separate_lower_families`,
  `:335`). Under 20 ms.
Proof: unit, that test; the smoke (rule 8). Its live effect is 11.3's
  calibration, whose `background` variant is only on 11.3's branch
  (`875a9981`): 11.3 rebases on 11.s1, rebuilds its arm, and reruns from
  its first attempt; an attempt that ends harness-error again stops 11.3
  on a block.
Don't: change the machine's job handling, the agent service's routing,
  or 11.3's probe.
Size: 40 lines, the test included.
Removes: the `unreachable!` at `translate.rs:513`.

### 11.s2 A closing session tells its tools' facts before its end
Scope: protocol/events.md 9 ("one `.completed` per `.started` …, and
  one `.closed` per `conversation.opened`"; a job's terminal "before its
  conversation's `conversation.closed`"); tools.md 5 ("The session ends
  after those records"); 11.3's block of 2026-10-15T08:36:24Z.
Needs: 11.s1. 11.3's repository proof needs it.
Files: `crates/smith-domain-session/src/session.rs` (`settle`,
  `:1222-1225`), `crates/smith-domain-session/src/jobs.rs` (`Jobs::empty`,
  `:22-24`), `crates/smith-domain-session/src/tests.rs`
  (`a_session_tells_what_happens_as_facts`, `:1886`),
  `tests/session/tests/jobs.rs`.
Shape: a session closing while a `job` read waits (11.3's smith attempt
  at its deadline) cancels the job; the job's end answers the read, the
  kit closes in the same step, the jobs table empties, and `settle`
  skips `pass_on_facts` (`:1223`), so the read's `Answered` is told
  after `Ended`; the agent service has dropped the scope by then, and
  `calls.rs:218`'s `expect` panics. `settle` calls `pass_on_facts`
  whenever the session is settled; `Jobs::empty` goes. `:1886`'s expected
  order becomes `[Tools Closed, Ended]`. `calls.rs` stays as it is.
Test shape: `tests/session/tests/jobs.rs`:
  `a_session_closing_while_a_job_read_waits_answers_the_read_before_it_ends`,
  beside `a_session_closing_with_a_job_running_tells_its_job_ended`
  (`:221`): a job, a waiting `job` read, the session closed, the job
  ended cancelled; the facts' order is the read's answer, then the end
  (on `main` it is the reverse). Under 20 ms.
Proof: world, that test; the smoke (rule 8). Its live effect is 11.3's
  repository suite, rerun on an arm rebuilt on it.
Don't: make `calls.rs` tolerate a missing scope, or drop a fact.
Size: 30 lines, the test included.
Removes: `Jobs::empty`.

## Done when

- The README's "Done when" item 4 prints at least 1 for
  `tool-output/background`.
- `ls benchmarks/summaries/repository-check-*.json` lists one summary.
