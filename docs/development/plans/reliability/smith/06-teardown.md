# Session 06: how a run and its process end

Provisional, 2026-10-12. smith's worlds already pass skein's teardown
invariant: smith pins skein past it, its hosts state their policy
deadlines, and the LLM pool closes by skein's owner close once the
channel ends. What is left is the end as the design draws it: a spawned
agent's end carries `exit` and `forced`; the agent closes everything at
its answer in one phase, and a termination signal after the answer
aborts; graces and the watchdog are derived and checked by name; a
failure reaches the person and the operator in words, with its limit,
phase, status and kept detail, in place of "Run failed" and debug text;
and the worlds run at shipped times. Without it, a forced exit, a
linger or a failure's cause stays invisible, as in the spikes (findings
O1, O5, O13 to O16).

## Read first

- protocol/agent.md, section 6;
- domain/host.md, sections 4, 8, 9.3 and 10;
- protocol/hosts.md, sections 3, 5.1, 5.2 and 5.6;
- domain/run.md, section 10; domain/session.md, section 4 (the last
  failure's detail);
- protocol/limits.md, sections 2.3, 2.4, 3.2 (a failure's detail), 4
  (`grace-over-teardown`, `watchdog-over-silence`), 7 and 8;
- protocol/events.md, sections 3.2, 3.9 and 3.10;
- shell.md, sections 4.3, 6.5 and 9; testing.md, section 6;
- skein's `programming-model.md`, section 5.2; `shell.md`, section 13;
  `testing-strategy.md`, section 6; `llm-connection.md`, section 5.1;
- decisions.md, 6.1 to 6.4, 6.7, 6.8 and 12.2;
- findings O1 to O6 and O13 to O16 (`findings/operation.md`).

## What the design asks

- **The end is one phase** (agent.md, section 6): at `Answered` "the
  service closes everything it owns at once: the LLM pool …; the
  machine's roots; and the channel, which it finishes"; at `Closing` it
  waits for each report, "then writes `session.ended` and closes the
  event stream"; at `Closed` the process exits. "Only io's close
  deadlines bound `Closing`." A termination signal after the answer
  "turns every close still in progress into an abort".
- **Exit evidence** (host.md, section 4): "`exit`, the code it exited
  with or the signal that ended it, and `forced`, whether the kit had to
  end it (no, terminated or killed)."
- **Derived graces** (limits.md, section 4): `grace-over-teardown` and
  `watchdog-over-silence`, each refused by name.
- **Typed failures, carried to the answer** (limits.md, section 8): "The session
  keeps it in its failure, the run in its answer"; run.md, section 10
  adds "the endpoint and model of the last completion and how many
  attempts it made".
- **The last failure's detail** (session.md, section 4) "goes with the
  session's end to its opener, and on to the agent's log … It never
  enters the model's history."
- **Words, never debug text:** the agent's one line on standard error
  for a failed answer (agent.md, section 6); the local host's failure
  notices, "none … shown as a fixed 'run failed'", a forced exit after
  the run answered, "whatever its answer, … shown as a warning beside
  the answer's words, which it leaves as they are" (host.md, section 8;
  settled 2026-10-12).
- **The product's end** (shell.md, section 9): it "exits once each has
  reported closed and io is empty"; a signal while closing aborts.
- **Shipped times** (testing.md, section 6): "No world shortens the LLM
  connections' idle keep, or any other, so that it ends sooner."

## Choices this plan makes

- **The phase is the agent service's:** one `Teardown` in
  `smith-agent-service/src/teardown.rs`, matched after each pass;
  `work_pending` and `done` follow from it. `Effects::close`, the inline
  placement's owner close, enters the same phase.
- **Each component reports its close in its own vocabulary:** the LLM
  component forwards skein's `Closed` as `closed()`; the roots by the
  file layer's close terminals, as on `main`; the channel finishes, then
  ends; the event sink by `Sinks::closed()`.
- **The teardown bound takes agent.md's terms:** two of io's close
  deadlines in turn plus the stream's write deadline. The cancels'
  settling adds no term (limits.md, section 7).
- **The kit holds both bounds and both graces** (`teardown`, `silence`,
  `cancel_grace`, `exit_grace`) and checks both relationships in one
  `check`. The local service, the host that writes its spawned agent's
  configuration, derives `exit_grace` as the bound plus one `close`
  (shell.md, 6.5) and `no_progress` as `silence` plus one `close`.
- **One words function,** `smith_events::words`, over the answer
  failure of events.md, 3.10, in shell.md 4.3's forms after "run
  failed: ". The agent's line, the local host's notices and session 10's
  last line translate to it with a total function.
- **The `forced_exit` notice is session 03's** (03.10), from 06.1's
  evidence.

## Rules that bind it

- **No type beside an existing one** (code-organization.md, section 7):
  `End::Stopped` becomes `End::Exited`, the kit's `grace` splits, the
  run's `CompletionFailure` gains its fields, each in place with its
  users in the same increment.
- **Formats keep their versions** (README, rule 5): channel 2 and events
  1 change in place, with no bump.
- **A world that fails at shipped times is fixed in its owner,** never by
  shortening a time, naming a later last word or opting out (testing.md,
  section 6).

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 06.11 | b794fd51 | deterministic |

## Increments

### 06.1 Exit evidence
Scope: io's exit status goes up with the process's end, never dropped;
the kit ends a spawned agent with `exit` and `forced`, the last step it
took (hosts.md, section 3; host.md, section 4). The inline agent ends
`Settled` (host.md, 9.3). The local service passes the evidence on
unchanged; 06.9 shows it.
Needs: nothing.
Files: `crates/smith-host-protocol/src/process.rs` (:297-303, :316-319
and :426 drop `kernel::Exit`); `crates/smith-host-domain/src/boundary.rs`
(`End`), `process.rs:38`, `domain.rs` (`Alarm::Grace` at :391, `Gone`
at :1391), `facts.rs`; `crates/smith-inline-agent/src/stop.rs:113`.
Users: `crates/smith-local-service/src/process.rs:829`,
`crates/smith-local-domain/src/tests.rs:269`, `parent_tests.rs:62, 210,
291`, `crates/smith-inline-agent/src/tests.rs:202`,
`tests/hosts/src/lib.rs:295`, `tests/hosts/tests/simulation.rs`,
`tests/host/src/lib.rs:278, 314`, `tests/host/tests/memory.rs:177`,
`tests/host/tests/fuzzy_simulation.rs:72`,
`tests/host/tests/simulation/inline.rs:269`,
`tests/agent/tests/host_delivery.rs:202, 279`; and, found at the run's
start (decisions.md, 13.1), `End::Stopped` in `tests/host/tests/referee.rs`
(:18, :46, :83), `tests/agent/tests/host_messages.rs:160` and
`tests/channel/tests/composed.rs:194`, and `cleanup`'s callers in
`tests/host/tests/budget.rs`, `tests/host/tests/messages.rs`,
`tests/channel/tests/composed.rs:188` and
`tests/channel/tests/composed/long.rs:201`.
Shape: `smith_host_domain::{Exit, Forced}`; `End::Exited { exit:
Option<Exit>, forced }` and `End::Settled` (sketch);
`process::Event::Exited { owner, exit: Option<Exit> }`;
`ProcessEvent::Exited { agent, exit: Option<skein_io::kernel::Exit> }`,
`None` only on io's `Failed`, shown as "status unknown" (decisions.md,
12.19). The kit keeps the last `Signal` it sent per
agent; a total `const fn` in `smith-local-service/src/process.rs` gives
`kernel::Exit` as `host::Exit`.
Test shape: `tests/host/tests/simulation.rs`, after
`explicit_stop_cancels_once_and_does_not_restart_grace` (:741), with
`World::live`, `event`, `at`, `settled` (`tests/host/src/lib.rs:251,
191, 214, 297`); `cleanup` (:272) takes the exit to report. Tests:
`an_agent_that_ignores_terminate_is_gone_killed_with_its_signal`,
`an_agent_that_exits_0_is_gone_not_forced`,
`an_agent_that_exits_1_is_gone_with_its_code`,
`an_agent_that_exits_on_terminate_after_its_answer_is_gone_terminated`;
the stories matching `End::Stopped` in `tests/hosts/tests/simulation.rs`
match `End::Exited` with its evidence; no other story changes.
Ceilings: 0.5 s each.
Proof: world, `an_agent_that_ignores_terminate_is_gone_killed_with_its_signal`,
once: the end says `Killed` with signal 9; the parent's `Stopped`
carries no evidence. Under 1 s. No probe reads the kit's evidence.
Don't: infer an exit from a signal sent.
Size: at most 150 lines of production code.
Removes: `End::Stopped`; the `..` that drops io's exit.

### 06.2 The agent's line for a failed answer
Scope: the agent shell writes one bounded line when the agent ends:
without an answer, why; with a failed or refused answer, its typed
failure in words, clipped at a character's boundary to 512 bytes and one
line (agent.md, section 6; limits.md, 2.4). 06.7 and 06.12 add what the
words name.
Needs: nothing.
Files: new `crates/smith-events/src/words.rs`, `lib.rs`;
`crates/smith-agent-service/src/answers.rs` (`failed`, `refused`, new
`Last`); `crates/smith-agent-shell/src/agent_shell.rs:127-133`.
Shape: `smith_events::{words, clip, DETAIL_BYTES}`;
`smith_agent_service::{Last, last}` (sketch). The shell writes `smith:
run failed: <words>`, then `: <clip(detail)>` when a detail exists, or
`smith: the run could not answer: <why>`, `why` matched exhaustively
over `Failure`.
Test shape: unit `crates/smith-events/src/tests.rs`, as
`provider_failure_metadata_and_capture_detail_survive_the_reader` (:10):
`every_answer_class_has_its_words` (a table of literals, `Unknown`
included), `clip_stops_at_a_character_boundary`,
`clip_keeps_the_first_line`; world `tests/agent-process/tests/stories.rs`
(reference :5, one-turn budget), `World::errors` (`src/world.rs:1230`):
`a_failed_answer_ends_standard_error_with_its_failure_in_words` (its
last line `smith: run failed: out of turns`).
`an_agent_whose_host_hangs_up_before_its_answer_says_so_and_exits_1`
(:49) now expects `smith: the run could not answer: the channel ended
before the answer`. Ceilings: 20 ms unit, 0.5 s world.
Proof: world, `a_failed_answer_ends_standard_error_with_its_failure_in_words`,
once: the line names the budget; the parent writes no line for a failed
answer. Under 1 s. `failures/unknown-model` reads no standard error of
the agent; not run in this plan.
Don't: render with `{:?}`; write the words to the channel or a prompt.
Size: at most 200 lines of production code.
Removes: the `{:?}` at `agent_shell.rs:130`.

### 06.3 The watchdog over silence
Scope: the kit's limits take the agent's `silence`, and the kit refuses a
`no_progress` that does not exceed it, naming `watchdog-over-silence`
(limits.md, section 4). The local service derives both for its spawned
placement.
Needs: nothing (smith 05's long operations for completions and
commands are on `main`).
Files: `crates/smith-host-domain/src/limits.rs` (new `check`),
`domain.rs:142` (`Domain::new` asserts it); new
`crates/smith-agent-service/src/profile/bounds.rs`;
`crates/smith-local-service/src/profile.rs:37`;
`crates/smith-local-shell/src/local_shell.rs:98, 309`. Kit limits users:
`tests/hosts/src/lib.rs:113`, `tests/host/src/lib.rs:139`,
`crates/smith-local-domain/src/tests.rs:404`,
`tests/agent/tests/host_messages.rs:93, 206`.
Shape: kit `Limits.silence`; `smith_host_domain::{Check, check}`;
`smith_agent_service::profile::silence_bound(&Limits)`: the largest of
`file_timeout`, `io.close_timeout` and the stream's `write_deadline`;
`host_limits` takes the agent's `&Limits` and returns
`Result<host::Limits, Refusal>`, a failed check becoming
`Refusal::Broken { check: Check::WatchdogOverSilence, .. }`.
Test shape: unit `crates/smith-host-domain/src/tests.rs` with `limits()`
(:18): `no_progress_not_above_silence_is_refused_as_watchdog_over_silence`;
unit `crates/smith-local-service/src/tests.rs`:
`the_standard_no_progress_is_silence_and_one_close` (literals: 30 s and
5 s give 35 s). `tests/host/tests/simulation.rs:659` stays the referee's
pause rule. Ceilings: 20 ms.
Proof: unit, `no_progress_not_above_silence_is_refused_as_watchdog_over_silence`,
once: refused by name; the parent has no check. Under 1 s.
`slow-completion` would show the paused watchdog live; not run in this
plan.
Don't: count completions, commands or checks in `silence`: they are long
operations.
Size: at most 120 lines of production code.
Removes: `no_progress: profile.policy.max_waiting`.

### 06.4 The teardown phase, and the pool closed at the answer
Scope: `Teardown` drives the agent from its answer to its exit
(agent.md, section 6): at the answer it closes at once the LLM component
(skein's owner close), the roots and the channel; when each has reported
closed it reads usage, writes `session.ended` with `answered` and
`teardown_ms`, and closes the stream. A host that hangs up mid-run
cancels the run, which then closes the same way and exits 1. A start the
closed pool refuses cannot reach a session (O2).
Needs: nothing (skein's owner close is in smith's pin).
Also (decisions.md, 13.18): the local process adapter's read admission,
which 06.4's earlier close of the agent's input exposes:
`crates/smith-local-service/src/process.rs` hands the host a record only
for a `host::process::Request::Read` it holds (today `Read` is ignored at
:814-816 and every record is pushed at :611-617), with one regression test
at the lowest tier that shows the host's "record ends one requested read"
assertion on the parent; at most 80 production lines, in 06.4's landing,
named in its commit body. The host domain's assertion stays.
Files: `crates/smith-agent-service/src/service.rs` (hotspot: fields
:151-164, `Ending` :169, `Effects::close` :332, `cleanup` :891,
`record_usage` :937, the answer :1714, `work_pending` :2119, `done`
:2162, :2176-2220, tests :2500); new `src/teardown.rs`;
`crates/smith-protocol-llm/src/component.rs` (:311, :456, :475).
Shape: `Teardown`, `Answer`, `Close`, `Step`, `advance` (sketch and
table); `smith_protocol_llm::Component::closed(&self) -> bool`.
Test shape: unit in `service.rs`'s tests (:2500), one per row of the
table, such as `the_answer_requests_every_close_in_one_pass`,
`done_is_some_only_when_closed`; :2551 and :2563 rework onto the phase.
World `tests/agent-process/tests/stories.rs` with a `hold_channel` knob
on `Parent` (`src/world.rs:249`): standard input stays open until the
agent's exit is known. Reference:
```rust
#[test]
fn an_answered_agent_exits_while_its_host_holds_the_channel_open() {
    let mut world = World::new(7, &charter());
    world.hold_channel();
    assert!(world.settle(), "the agent ends by itself");
    assert!(world.answer().is_some());
    assert_eq!(world.exit(), Some(skein_io::kernel::Exit::Code(0)));
    assert!(!world.signalled());
    world.assert_agent_clean();
}
```
Also `a_host_that_hangs_up_mid_call_gets_one_cancelled_terminal_per_call`;
`tests/protocol-llm/tests/component_peer.rs`:
`an_owner_close_reports_closed_once_every_socket_settled`. Ceilings:
20 ms unit, 0.5 s world.
Proof: world, `an_answered_agent_exits_while_its_host_holds_the_channel_open`,
once: exit 0, never signalled; the parent waits for the host's hang-up
and the world does not settle. Under 1 s. `smoke-tools/child` would show
it live; not run in this plan.
Don't: close the pool before the answer; cancel the run's calls on the
close; keep `Ending` beside `Teardown`.
Size: at most 350 lines of production code.
Removes: `answer_sent`, `channel_ended`, `cleanup_started`, `Ending`,
`cleanup`, `end_pending`; the `struct_excessive_bools` expectation (at
most three bools remain); `connection::Event::Closed => {}`;
`Refusal::Closed => Error::Limit`.

### 06.5 A termination signal after the answer
Scope: io's `Shutdown` in `Answered` or `Closing` aborts the LLM
component (skein's owner abort) and turns io's closes in progress into
`io::Request::Abort`; in `Live` it stays the run's cancel; a second
means the same. `session.ended` says `aborted`, with a
`teardown_aborted` notice (events.md, 3.2 and 3.9). A signal in
`Closing`'s stream step, after `session.ended` is written, aborts the
stream's closes and writes nothing more (decisions.md, 13.34).
Needs: 06.4.
Files: `crates/smith-agent-service/src/teardown.rs`, `service.rs:999`,
`sinks.rs` (:107, :501), `events.rs:184`;
`crates/smith-protocol-llm/src/component.rs`;
`crates/smith-events/src/records.rs` (:369, :722), `write.rs`,
`read.rs`, `golden/v1/`; `tests/codecs/tests/events.rs`.
Shape: `Component::abort(&mut self, env, to_domain, io)`;
`SessionEnded.aborted`, `NoticeKind::TeardownAborted` (sketch);
`sinks::Ended.aborted`.
Test shape: world `tests/agent-process/tests/stories.rs`, a
`signal_after_answer` knob beside `signal` (`src/world.rs:995`), the
fake peer keeping its connection after its reply:
`a_termination_signal_after_the_answer_aborts_a_peer_that_ignores_the_close`,
`a_second_signal_after_the_answer_changes_nothing`; codecs: goldens
`session_ended_aborted.jsonl`, `notice_teardown_aborted.jsonl` through
`every_record_and_listed_value_reads_back_to_its_written_values` (:41).
Ceilings: 0.5 s.
Proof: world, `a_termination_signal_after_the_answer_aborts_a_peer_that_ignores_the_close`,
once: the process ends before io's 5 s close deadline with `aborted`
true; the parent waits the deadline out. Under 1 s.
Don't: abort before the answer.
Size: at most 150 lines of production code.
Removes: the cancel of an answered run on `Shutdown`.

### 06.6 The exit grace derived
Scope: the agent's teardown bound (Choices); the kit uses `cancel_grace`
before the answer and `exit_grace` after it, and refuses an `exit_grace`
not above `teardown`, naming `grace-over-teardown` (limits.md, section
4). The local service derives `exit_grace` as the bound plus one
`close`, and refuses a policy setting below the bound by name.
Needs: 06.3 (the kit's `check`).
Files: `crates/smith-host-domain/src/limits.rs:65`, `domain.rs`
(`Alarm::Grace` at :391); `crates/smith-agent-service/src/profile/bounds.rs`,
`profile/quantities.rs:192`, `profile/validate.rs:88`;
`crates/smith-local-service/src/profile.rs:47`;
`crates/smith-local-shell/src/local_host.rs:117-118`. Kit limits users:
`tests/host/src/lib.rs:143`, `tests/hosts/src/lib.rs:113-115`,
`crates/smith-local-domain/src/tests.rs:408`.
Shape: kit `cancel_grace`, `exit_grace`, `teardown`;
`Check::GraceOverTeardown` (sketch); `profile::teardown_bound(&Limits)`;
`standard()`'s `exit_grace` 45 s (two 5 s closes and the 30 s write
deadline, plus one close).
Test shape: unit `crates/smith-host-domain/src/tests.rs`:
`exit_grace_not_above_teardown_is_refused_as_grace_over_teardown`; unit
`crates/smith-agent-service/src/profile/tests.rs`:
`the_standard_exit_grace_exceeds_its_teardown_bound` (literals 45 s,
40 s); world `tests/host/tests/simulation.rs`:
`an_answered_agent_is_signalled_only_past_its_exit_grace`;
`tests/hosts/tests/simulation.rs`:
`an_answered_agent_exits_within_its_grace_and_is_never_signalled`.
Ceilings: 20 ms unit, 0.5 s world.
Proof: world, `an_answered_agent_is_signalled_only_past_its_exit_grace`,
once: terminated at 45 s after the answer, not at the cancel's grace;
the parent has one grace. Under 1 s.
Don't: change `cancel_grace`'s value (smith 04's 04.14).
Size: at most 150 lines of production code.
Removes: the kit's `grace`; the hand-set `exit_grace`.

### 06.7 Typed failures in the run's answer
Scope: the run's model failure carries what the session met (run.md,
section 10; limits.md, section 8): a limit with which and bound, a
timeout's phase, the HTTP status in the evidence. The root, the channel
(version 2), the kit's parent face and the inline agent carry them, each
translating with a total function. `315d7f08` put them in the event
stream already. Split (decisions.md, 13.52): the last completion's model
and its attempts are 06.13's.
Needs: nothing (skein's typed limits and phases, smith 04's retry by
phase are on `main`).
Files: `crates/smith-domain-session/src/llm.rs:428`, `boundary.rs:390`,
`session.rs` (hotspot); `crates/smith-protocol-llm/src/boundary.rs:44`,
`component.rs`, `failure.rs`; `crates/smith-domain-run/src/boundary.rs`
(:745, :789, :801); `crates/smith-domain/src/translate.rs:264-285`,
`feedback.rs:468`; `crates/smith-channel/schema/channel-v2.schema:103-111`,
`src/generated/v2.rs` and goldens; `crates/smith-protocol-channel/src/translate.rs:599`;
`crates/smith-host-domain/src/channel.rs:196-265`;
`crates/smith-host-protocol/src/translate.rs`;
`crates/smith-inline-agent/src/translate.rs:239`;
`crates/smith-agent-service/src/answers.rs`, `responses.rs:737-790`;
`crates/smith-local-domain/src/agent.rs:342`.
Shape: run sketch; the session's `Evidence::Response { status }` and
`End::Failed`'s `attempts`; the kit's `ModelFault::Completion`,
`CompletionFailure`, `CompletionEvidence`, `Limit`, `Cap`, `Phase`
mirror the run's field for field; the schema gains `limit` with `which`
and `bound`, `application_limit`, `timed_out` with `phase`, `response`
with an optional `status`, and `model` and `attempts` in
`CompletionFault`.
Test shape: `tests/agent/tests/feedback_table.rs:227` as reference:
`every_completion_failure_reaches_the_answer_with_its_fields`;
`tests/protocol-llm/tests/component_boundary.rs:221`:
`a_response_failure_keeps_its_status_in_the_evidence`; tests/host inline
(`tests/host/tests/simulation/inline.rs`):
`an_inline_answer_keeps_its_typed_completion_failure`; channel goldens
through `tests/codecs/tests/drift.rs:36`. Ceilings: 0.5 s.
Proof: world, `every_completion_failure_reaches_the_answer_with_its_fields`,
once: limit, phase and status reach the run's answer; the parent's
`Limit` has no fields. Under 1 s.
Don't: carry a limit or a status as text; change retry policy.
Size: at most 550 lines of production code, generated code excepted.
Removes: the fieldless `Limit` and the phaseless `TimedOut` in the run,
the channel and the kit; `ToDomain::Failed`'s `status` beside the
evidence.

### 06.13 The last completion's model and attempts in the run's answer
Split from 06.7 (decisions.md, 13.52). Scope: what 06.7's Scope gave
before the split and its candidate (`ee9d158d`) holds for it: the last
completion's model, numbered main first through one `Charter` lookup
(13.49), and its attempts, carried by the root (`Peer`'s model index),
the channel's `CompletionFault` (`model`, `attempts`), the kit's parent
face and the inline agent, each translating with a total function.
Needs: 06.7.
Files: 06.7's, those its candidate touches for the model and the
attempts; `crates/smith-domain/src/peer.rs`.
Shape: 06.7's sketch for `model` and `attempts`; `End::Failed`'s
`attempts`.
Test shape: 06.7's `every_completion_failure_reaches_the_answer_with_its_fields`
gains the model and attempts assertions; the parent resolves the
endpoint and name through the same lookup. Ceilings: 0.5 s.
Proof: world, that test, once: model and attempts reach the run's
answer. Under 1 s.
Don't: carry a model as text; change retry policy.
Size: README rule 9's.

### 06.s1 An accepted answer reaches the host before the agent's teardown
Added during the run (decisions.md, 13.59). Scope: smith 00.16's live
`history` (`probes-history-b26d2c8c-20261012T180834Z/01`): the agent's
stream has `tool.started` `finish` and `run.completed` `accepted` at
5,311 ms, then at once a `teardown_aborted` notice and `session.ended`
with `exit: 1`, `answered: false`, `aborted: true`, `teardown_ms: 0`;
the product shows "Agent process failed" and ends "local chat ended
with a failure". agent.md, section 6: "The process exits with success
once it has answered, whatever the answer, and with failure only when
it could not answer: startup, or a broken channel." Something broke the
channel or signalled the agent at its answer. 05.s1's live stall also
ends "Agent process failed"; read its findings first.
It also takes the live silence 05.s1 could not reproduce and that
`follow-up/chat` no longer shows on `main` (24 of 24, 3f15308c):
`probes-parallel-children-reviews-3f15308c-20261012T181923Z/03`, a main
turn ending with text and no calls at 14,845 ms, owed a nudge (run.md,
7.2), then nothing until the host's cancel at 49,846 ms (decisions.md,
13.64). No world reproduced either on the parent, so the first move is
one live diagnostic arm (README rule 11, a worktree commit that never
lands): on standard error, in the product and the agent, each turn's
save and acknowledgement, each completion held and released, and each
signal and channel close the product sends its agent and why; built as
an arm, `history` once and `parallel-children/reviews` once (12) on it.
Needs: nothing.
Files: the reproduction's world test file; then the crate where the
channel breaks or the signal comes from (the local host's agent handling,
`smith-host-domain`, `smith-local-domain`, `smith-local-service`, or the
agent service's teardown).
Shape: no new type; README rule 11 finds where.
Test shape: one process-world story of a spawned agent that answers
accepted after several tool calls; it fails on `main` (exit 1 or a
failure shown). Under 0.5 s.
Proof: a story the diagnostics point to, once, failing on the parent;
then `history` once and `parallel-children/reviews` once (12), live,
through `heavy --gate`, their counts in the commit body.
Don't: widen a grace or a deadline to hide it; add a retry or delay. If
no world reproduces it, stop and ask with the diagnostics.
Size: README rule 9's.

### 06.12 The last failure's kept detail
Scope: the session keeps the detail of the last failure it met, within
`failure_bytes`, each later failure replacing it, and hands it up with
its end; it never enters history (session.md, section 4). The run keeps
its main conversation's last detail and answers with it; the root's
`Answer` and the parent face's `Answered` carry it, empty from a spawned
agent, whose detail is its error tail. The agent's line (06.2) clips it.
Needs: 06.2.
Files: `crates/smith-domain-session/src/session.rs:745-760`
(`drop(detail)`), `boundary.rs:182`; `crates/smith-domain/src/route.rs:482`,
`boundary.rs:297`; `crates/smith-domain-run/src/boundary.rs` (:178,
:320), `run.rs` (hotspot); `crates/smith-inline-agent/src/translate.rs:357`;
`crates/smith-host-domain/src/parent.rs`, `domain.rs:932`;
`crates/smith-agent-service/src/service.rs:1714`, `answers.rs`. Users of
`Answered`: `crates/smith-local-domain/src/domain.rs`,
`crates/smith-inline-agent/src/tests.rs:192, 480`,
`crates/smith-host-domain/src/domain.rs:1427`.
Shape: `detail: Box<[u8]>` on `session::Request::Ended`,
`run::Event::Ended`, `run::Request::Answer`, `smith_domain::Request::Answer`
and `parent::Request::Answered` (sketch).
Test shape: `tests/session/tests/simulation.rs:168` as reference:
`a_failed_sessions_end_carries_its_last_detail`,
`a_later_failure_replaces_the_kept_detail`,
`no_prompt_holds_a_kept_detail`; world `tests/agent-process/tests/stories.rs`:
`a_provider_refusal_ends_standard_error_with_its_clipped_text`.
Ceilings: 0.5 s.
Proof: world, `a_failed_sessions_end_carries_its_last_detail`, once: the
provider's text reaches the session's end and no prompt; the parent drops
it. Under 1 s.
Don't: put the detail in a fact, a transcript or the channel.
Size: at most 250 lines of production code.
Removes: `drop(detail)`.

### 06.10 Worlds at shipped times
Scope: no world shortens a time that decides how something ends
(testing.md, section 6): `lower_configuration`'s 50 ms io close, 5 ms
idle keep and 50 ms LLM close (`tests/local-process/src/process.rs:58-60`,
which `tests/real` reuses), and the LLM component worlds' idle keeps of
1 s and 10 s, which take the `standard` profile's `connection_keep`.
Needs: smith 04's `standard` profile at shell.md's values (04.14), whose
`cancel_grace` of 10 s passes `cancel-over-close` (limits.md, section 4:
at least twice the 5 s `close`). With `main`'s 5 s, the
in-process `an_interrupt_in_the_middle_of_a_turn_cancels_the_run`
(`tests/local-process/tests/simulation.rs:62`) drops its run at 5 s:
the cancelled call's connection closes within one `close` against a
peer that keeps it open.
Files: `tests/local-process/src/process.rs`,
`tests/protocol-llm/tests/component_boundary.rs:35`,
`tests/protocol-llm/src/pool_peer.rs:33`,
`tests/protocol-llm/tests/component_peer.rs:37`.
Shape: none.
Test shape: the worlds' stories, unchanged; ceilings unchanged.
Proof: world, `heavy cargo nextest run -p smith-local-process-world -p
smith-real-world -p smith-protocol-llm-world`, once: the three packages
pass with no test skipped, at shipped times under skein's teardown
invariant. Under 2 minutes.
Don't: name a later last word for a story that is not about that time.
Size: no production code.
Removes: the six overrides.

### 06.8 The local service's end
Scope: after the local host's last word, the local service closes what
it owns: the agent's components through 06.4's phase in `Effects`, the
spawned agent's process adapter, the store's files and the terminal;
then it writes `session.ended` and closes the product's stream; it exits
when each has reported closed and io is empty (shell.md, section 9). A
termination signal while closing aborts, as 06.5 does. A chat's LLM pool
closes at its last word, not at each answer (decisions.md, 12.19).
Needs: 06.4 and 06.5; smith 03's product stream (03.7).
Files: `crates/smith-local-service/src/service.rs` (`finish_exit` :483,
`Request::Exit` :541-547), `process.rs`;
`crates/smith-local-shell/src/local_host.rs` (:77, :438, :604).
Shape: `Ending` (sketch), reusing 06.4's `Close` and `Step`.
Test shape: `tests/local-process/tests/simulation.rs`, both placements,
reference `an_interrupt_in_the_middle_of_a_turn_cancels_the_run` (:61);
`World::new`, `interrupted`, `settle`, `exit` (`src/world.rs:304, 315,
481, 622`), a signal knob on `src/terminal.rs`:
`a_chat_writes_session_ended_after_its_last_word`,
`a_termination_signal_while_closing_aborts_and_says_so`. Ceilings: 0.5 s.
Proof: world, `a_termination_signal_while_closing_aborts_and_says_so`,
once, both placements: the product ends before io's close deadline and
`session.ended` says `aborted`; the parent ignores the signal. Under 1 s.
Don't: exit before the store's files have closed.
Size: at most 250 lines of production code.
Removes: `finish_exit`'s conjunction; `local_host.rs`'s `closing` flag.

### 06.9 Failures in words in the local host
Scope: the local domain matches exhaustively what its agent hands it and
shows each as a failure notice, in its words, then its detail: a run's
typed failure, a refused start saying what, the kit's refusal before an
agent existed, an agent's failure with how it ended, a credential that
needs the person; a forced exit after any answer, accepted, failed,
parked or cancelled, is a warning, its code and last line the answer's
(host.md, section 8; shell.md, 4.3; hosts.md, 5.1). The same words go to the operator's
error output; none enters a turn or a prompt.
Needs: 06.1, 06.2, 06.7, 06.13 and 06.12; smith 05's typed notices (05.11).
Files: `crates/smith-local-domain/src/agent.rs:332`, `domain.rs:909-960`,
`boundary.rs`, `parent_tests.rs:328`; `crates/smith-local-protocol/src/terminal.rs`,
`Cargo.toml` (takes `smith-events`); `crates/smith-local-shell/src/local_host.rs:510-530`.
Shape: `smith_local_domain::{Failure, AgentFailure, Warning}` (sketch),
in 05.11's failure and warning notices; the terminal translates
`host::RunFailure` to `smith_events::AnswerFailure` with one total
function, then `words`.
Test shape: unit `crates/smith-local-domain/src/parent_tests.rs`, one
per kind (reference
`gone_without_an_answer_fails_after_retained_turns_are_saved`, :315);
world `tests/local/tests/simulation.rs`, after
`an_exhausted_account_is_shown_to_the_person` (:579), with a new
`FirstFailure` kind (`tests/local/src/world.rs:725-732`), `line`,
`shown`, `drive` (:836, :842, :907):
`a_budget_failure_is_shown_naming_what_ran_out`,
`a_provider_refusal_is_shown_with_its_class_and_text`; referee
`no_failure_words_enter_a_turn_or_a_prompt` (`tests/local/tests/referee.rs`);
`tests/local-process/tests/simulation.rs`, both placements:
`a_forced_exit_after_an_accepted_answer_is_shown_as_a_warning`,
`a_forced_exit_after_a_failed_answer_keeps_its_code_and_last_line`.
Ceilings: 20 ms unit, 0.5 s world.
Proof: world, `a_budget_failure_is_shown_naming_what_ran_out`, once:
the person sees `run failed: out of turns`; the parent shows "Run
failed". Under 1 s. `failures/unknown-model` would show it live; not run
in this plan.
Don't: show a fixed message or a debug rendering.
Size: at most 350 lines of production code.
Removes: "Run failed", "Agent process failed" and "Agent process ended
without an answer".

## Interface sketches

Sketches leave out derives and `#[must_use]`, which the crates' lints
ask for (`Debug` on each type).

**06.1 and 06.12: the kit's faces.** `End` crosses into the local domain
and session 03's stream.

```rust
// crates/smith-host-domain/src/boundary.rs
/// How a spawned agent's process ended, as io told it (domain/host.md, section 4).
pub enum Exit { Code(u8), Signal(u32) }
/// The last step the kit took to end its agent's tree.
pub enum Forced { No, Terminated, Killed }
/// Terminal disposition of one parent `Spawn`.
pub enum End {
    Busy,
    Invalid(Invalid),
    Unspawned,
    /// Process exited, tree empty, EOF and all rights settled; replaces `Stopped`.
    Exited { exit: Option<Exit>, forced: Forced },
    /// An inline run and everything below it settled (host.md, 9.3).
    Settled,
}
// crates/smith-host-domain/src/parent.rs, in `Request`
/// One run answer, with the last failure's kept detail; process cleanup still owes Gone.
Answered { client: Token, answer: Answer, detail: Box<[u8]> },
```

**06.3 and 06.6: the kit's limits.** `check` refuses; `Domain::new`
asserts it.

```rust
// crates/smith-host-domain/src/limits.rs, in `Limits`, replacing `grace`
pub cancel_grace: Duration,
/// An answered agent's grace to exit on its own; above `teardown`.
pub exit_grace: Duration,
/// The agent's teardown bound, from the host that writes its configuration.
pub teardown: Duration,
/// The longest the run may be silent between reports.
pub silence: Duration,
/// A relationship the kit refuses its limits for, with both sides (protocol/limits.md, section 4).
pub enum Check {
    GraceOverTeardown { exit_grace: Duration, teardown: Duration },
    WatchdogOverSilence { no_progress: Duration, silence: Duration },
}
pub fn check(limits: &Limits) -> Result<(), Check>;
// crates/smith-agent-service/src/profile/bounds.rs
pub fn teardown_bound(limits: &Limits) -> Option<Duration>;
pub fn silence_bound(limits: &Limits) -> Option<Duration>;
```

**06.4, 06.5 and 06.8: the phase.** The local service's `Ending` reuses
`Close` and `Step`; its `Closed` sends the shell's `Exit`.

```rust
// crates/smith-agent-service/src/teardown.rs
/// Where the agent process is in its end, matched in one place after each pass.
pub enum Teardown {
    Live,
    Answered { at: Time, answer: Answer, close: Close },
    Closing { at: Time, answer: Answer, close: Close, step: Step },
    /// `done` is `Some` only here.
    Closed { answer: Answer, close: Close },
}
/// Sent on the channel, or held by `Effects`' owner; or no channel to answer on.
pub enum Answer { Sent, Unsent(Failure) }
pub enum Close { Graceful, Aborted }
/// What `Closing` waits for, in order.
pub enum Step { Components, Usage, Stream }
pub(crate) fn advance(service: &mut Service);
pub(crate) fn shutdown(service: &mut Service);
// crates/smith-local-service/src/service.rs
pub(crate) enum Ending {
    Live,
    Closing { at: Time, close: Close, step: Step },
    Closed { exit: local::ExitStatus, close: Close },
}
```

| Phase | Event | Next | Does |
|---|---|---|---|
| `Live` | `Shutdown` | `Live` | the run's cancel; a second is the same |
| `Live` | the answer, sent or with no channel | `Answered` | `run.completed` written |
| `Answered` | room for the closes | `Closing(Components)` | LLM owner close, roots' closes, channel finished |
| `Answered`, `Closing` | `Shutdown` | same, `Aborted` | LLM owner abort; io's closes become aborts |
| `Closing(Components)` | LLM `closed()`, roots closed, channel ended | `Closing(Usage)` | io's `Usage` |
| `Closing(Usage)` | `Usage` | `Closing(Stream)` | `session.ended`: `answered`, `teardown_ms`, `aborted` |
| `Closing(Stream)` | `Sinks::closed()`, io empty | `Closed` | |

**06.2 and 06.5: words, the line and the records.**

```rust
// crates/smith-events/src/words.rs
pub const DETAIL_BYTES: usize = 512;
/// A typed failure in words, matched exhaustively, `Unknown` values included.
pub fn words(failure: &AnswerFailure) -> Box<[u8]>;
/// A detail clipped to `DETAIL_BYTES` at a character's boundary, cut at its first line end.
pub fn clip(detail: &[u8]) -> &[u8];
// crates/smith-events/src/records.rs: `SessionEnded` gains `aborted: bool`;
// `NoticeKind` gains `TeardownAborted`.
// crates/smith-agent-service/src/answers.rs
/// What the agent's last line on standard error says (protocol/agent.md, section 6).
pub enum Last {
    Quiet,
    /// A failed or refused answer: its failure, and the kept detail (06.12).
    Failed { failure: Box<smith_events::AnswerFailure>, detail: Box<[u8]> },
    /// No answer crossed the channel; why.
    Unanswered(Failure),
}
pub fn last(service: &Service) -> Option<Last>;
```

**06.7: the run's failure.** The kit and the channel mirror it; `Cap`
mirrors `smith_domain_session::llm::Cap`'s 21 variants.

```rust
// crates/smith-domain-run/src/boundary.rs
pub enum Limit { Pool { reservation: u64 }, Path(Cap), History, Call, Message }
pub enum Phase { Connect, Handshake, Head, Idle, Whole }
pub enum CompletionFailure {
    Limit { which: Limit, bound: u64 },
    ApplicationLimit,
    TimedOut { phase: Phase },
    // Protocol, Cancelled, Overloaded, Unavailable, ContextTooLong, Invalid,
    // Unauthorized, RateLimited, Exhausted: as on main
}
pub enum CompletionEvidence { Unsent, Unknown, Response { status: Option<u16> } }
pub enum Fault {
    Completion {
        failure: CompletionFailure,
        evidence: CompletionEvidence,
        /// The last completion's model, by its index in the charter's `models`.
        model: u32,
        attempts: u32,
    },
    // Exhausted, Provider, ContextFull, Refused, Truncated, Malformed: as on main
}
```

**06.9: what the local domain hands the terminal.**

```rust
// crates/smith-local-domain/src/boundary.rs
pub enum Failure {
    Run { failure: host::RunFailure, detail: Box<[u8]> },
    Refused(host::Refusal),
    /// No slot, invalid naming what, or not spawned.
    Unstarted(host::End),
    Agent { failure: AgentFailure, end: host::End, detail: Box<[u8]> },
    Credential { account: u32, reason: CredentialFailure },
}
pub enum AgentFailure { NotStarted, Fault(host::Fault) }
pub enum Warning { ForcedExit { exit: Option<host::Exit>, forced: host::Forced } }
```

## Questions to raise when reached

- ~~**A forced exit after a failed answer** (06.9)~~: settled by the
  designer on 2026-10-12 (shell.md, 4.3; host.md, sections 4 and 8): the
  same warning after any answer, before the last line, which stays the
  answer's, as does the code.

## Done when

Each command runs once, from smith's checkout, and prints nothing:

```
rg -n 'answer_sent|channel_ended|cleanup_started|struct_excessive_bools' crates/smith-agent-service/src
rg -n 'End::Stopped' crates tests
rg -n 'b"Run failed"|b"Agent process failed"|b"Agent process ended without an answer"' crates
rg -n 'could not answer: \{:\?\}' crates
rg -n 'idle_keep: Duration::from|idle_keep = Duration|close_timeout = Duration::from_millis' tests
rg -n 'drop\(detail\)' crates
rg -n 'grace: profile.policy.exit_grace|no_progress: profile.policy.max_waiting' crates
```

They check, in order: no flag records where the agent's teardown is
(06.4); a spawned agent's end carries its evidence (06.1); no failure is a
fixed text or a debug rendering (06.2, 06.9); no world shortens a keep or
a close (06.10); the detail is kept (06.12); graces and the watchdog are
derived (06.3, 06.6). The named tests of each Proof pass in the gate,
which `land` runs.
