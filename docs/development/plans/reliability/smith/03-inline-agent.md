# Session 03: the inline agent

Provisional, 2026-10-12. The inline agent runs the product's chats in one
process, and it drops a run that ignores its cancel, but the product
still writes no event stream in one process (findings P6): its run's
facts are drained and counted, never written, so no probe and no reader
sees a run of the inline kind. What is left is that stream: the inline
run's records written by the local service (03.7), the record of a run
dropped before it answered and the second interrupt's story through the
real shell (03.8), the product's stream with a spawned agent (03.9), and a
spawned agent's exit in it (03.10). The product moves to the inline kind
by default in session 10, so its stream is what the benchmarks will read.

## Read first

- protocol/events.md, sections 2, 3.2, 3.3, 3.9, 3.10, 4, 5.2 and 8;
- protocol/hosts.md, sections 5.1 and 5.6;
- domain/host.md, sections 6, 9.1 and 9.2;
- shell.md, section 4.3 (exit codes);
- testing.md, section 2.6;
- decisions.md, 8.4 and 8.7;
- findings P6 and P8 (`findings/packaging.md`), O13 and O14
  (`findings/operation.md`).

## What the design asks

- **The product writes the stream** (events.md, section 2): "the product
  writes the stream of the run its inline agent composes (hosts.md,
  section 5.6) … The product's stream is the local service's. It holds
  the run's records whichever kind of agent the local host runs, and the
  host's own notices, such as a spawned agent's forced exit." hosts.md,
  5.6: the inline agent's "facts and content are drained every pass into
  the event stream".
- **`run.completed` where the writer meets the answer** (events.md,
  section 4): "the local service as its local domain receives it from
  its agent".
- **The header** (events.md, 3.2): `mode` is `chat`, and `chat` names the
  product's chat.
- **A run dropped before it answered** (events.md, 3.3): "its host writes
  `run.completed` from the turns it was told": `status` `failed`; turns,
  spent, usage and read from those turns; no `result`; `wind_down`
  false. Its class is the first cause of the end (3.10): `cancelled`
  with `dropped: true`, exit 130 (shell.md, 4.3); `host` with `reason`
  `store` and the `turn`, exit 1; `agent` with `ended`, the exit status
  or signal, exit 1. A cancelled answer has `dropped: false`.
- **The second interrupt** (hosts.md, 5.1): "a second, before the run has
  answered, drops the run (domain/host.md, 9.2), or, for a spawned agent,
  stops its tree."
- **A forced exit** is the host's `forced_exit` notice, with `exit` and
  `forced` (events.md, 3.9).

## Choices this plan makes

- **The inline kind's records go through the agent service's producer.**
  `smith_agent_service::Effects` already owns the LLM component's
  measures and a producer when the configuration names a trace. The local
  service feeds it the rest: the trace destination, the header, the
  run's start, its facts, `run.completed` and the exit. No second
  producer is written (code-organization.md, section 7).
- **One `run.completed` function for both kinds,** from the parent face's
  `smith_host_domain::Answer` (domain/host.md, 9.4: a host translates
  both kinds with the same functions). It lives in `smith-local-service`.
- **`limits` in the product's header stays empty** until session 04's
  local effective limits (04.22) fill it, as the agent's header is on
  `main`.
- **An answered run stays answered.** No increment changes runtime policy
  to force a drop after an answer (domain/host.md, 9.2).

## Rules that bind it

- **The harness follows the binary** (README, rule 4): 03.9 moves the
  smith adapter to the agent's trace path in the same increment.
- **Formats keep their versions** (README, rule 5): `session.started`'s
  `chat` is added under events version 1, with no bump.
- **No domain encodes or decodes** (decisions.md, 8.7): the local domain
  hands the service values; the records are built in the service.

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 03.1 | 1ee3a91b | proven |
| 03.2 | 254cf7c2 | proven |
| 03.3 | 5d0e1c3d | proven |
| 03.4 | 34e74248 | proven |
| 03.5 | c236b21d | proven |
| 03.6 | 7ac231f5 | proven |
| 03.8 | 1a1e8a39 | partly landed: the dropped run's record, the two-interrupt story |

## Increments

### 03.7 Events from the inline drains
Scope: with the inline kind, the product writes the event stream
(events.md, sections 2, 3.2, 3.3 and 4; hosts.md, 5.6) to the path of the
settings' `agent.trace`, with its capture: `session.started` with `mode`
`chat` and `chat` (a field added to the record); `run.started` and each
fact of the inline run, each pass, in drain order, with the LLM
component's measures; `run.completed` from the parent face's answer as
the local domain receives it; `session.ended` with the product's exit
code (shell.md, 4.3), its usage and its loss. The local domain stops
counting the facts it drops.
Needs: nothing beyond the session's start.
Files: `crates/smith-events` (`records.rs`, `read.rs`, `write.rs`,
`limits.rs`, goldens `golden/v1/session_started_*.jsonl`);
`crates/smith-agent-service/src/service.rs` (`Effects`; hotspot),
`src/events.rs` (`Producer::starting` taking values), `src/answers.rs`;
`crates/smith-local-domain/src/domain.rs` (`drain_agent_facts`,
`agent_facts_not_written`), `src/boundary.rs` (`Request::Answered`);
`crates/smith-local-service/src/service.rs`, a new `src/stream.rs`;
`crates/smith-local-shell/src/local_host.rs`, `src/local_shell.rs` (the
trace opened and adopted). Users of `SessionStarted`:
`crates/smith-agent-shell/src/agent_shell.rs:179`,
`tests/agent-process/src/tracing.rs:115`,
`tests/codecs/tests/support/events.rs:419`,
`tests/codecs/tests/support/events_max.rs:132`,
`crates/smith-events/src/read.rs:998`. Users of
`agent_facts_not_written`: `tests/local/src/world.rs:814`,
`tests/local/tests/simulation.rs:669`.
Shape: see "Interface sketches, 03.7". The local domain's
`pop_run_fact()` replaces its drop loop; its `Request::Answered`
moves the answer to the service once shown, owing no terminal.
`smith_local_service::stream::answered(&host::Answer, Capture) ->
Answered` builds the record's parts; `Effects::answered` adds the run,
wind-down, unread count, usage and duration the producer saw. Each pass,
the local service takes the run's facts until `Effects::takes()` is false;
a fact not taken holds its slot, as on `main`.
Test shape: `tests/local-process/tests/events.rs`, new, the inline kind
over the fake LLM. Reuse `World::new` (`tests/local-process/src/world.rs:304`),
`Placement::InProcess` (`world.rs:28`), and the agent-process world's
stream referee, `check` (`tests/agent-process/src/events.rs:16`). Make
its module public (`tests/agent-process/src/lib.rs:15`), and move its
`read_records` (`tests/agent-process/src/world.rs:454`) there as
`events::read(bytes, &EventLimits)`. `World::traced` names a trace in
the settings' agent document; `World::records` reads it with
`events::read`. Reference test:
```rust
#[test]
fn an_inline_chat_writes_a_complete_stream_through_its_answer() {
    let files = Files::new();
    let mut world = World::traced(11, &files, Placement::InProcess, b"First answer");
    world.settle();
    let records = world.records();
    events::check(&records);
    let Event::SessionStarted(header) = &records[0].event else { panic!("header first") };
    assert_eq!(header.mode, Mode::Chat);
    assert_eq!(header.chat.as_deref(), Some(b"main".as_slice()));
}
```
Tests: that one; `a_slowed_destination_loses_no_record_under_complete`;
`one_response_completed_per_provider_response_of_an_inline_chat`;
codecs: `session_started_chat_round_trips` (`tests/codecs/tests/events.rs`);
unit (`smith-local-domain`): `an_inline_runs_facts_wait_for_the_service`.
Ceilings: world 0.5 s each, unit 20 ms.
Proof: world, `an_inline_chat_writes_a_complete_stream_through_its_answer`,
once, under 0.5 s: the stream passes the referee, headed `mode` `chat`;
it fails on the parent, which writes no stream in one process. Not live
(decisions.md, 12.22): no live run picks the inline kind's pin; the
per-landing smoke runs the inline kind once smith 10.15 lands.
Don't: write records in the local domain; give the spawned kind a stream
here (03.9); fill `session.started`'s `limits` (smith 04.22).
Size: at most 400 lines of production code.
Removes: `drain_agent_facts`'s fact loop and `agent_facts_not_written`.

### 03.9 The product's stream with a spawned agent
Scope: with the spawned kind, the product writes its own stream, as the
local service's (events.md, section 2): `session.started`,
`run.completed` from the parent face's answer with 03.7's function, the
host's notices, and `session.ended`. The run's other records stay in the
agent process's own trace (channel.md, section 8). The product's stream
goes to the settings' `agent.trace` path; the generated agent
configuration names the agent's trace beside it, at that path with
`.agent` appended (events.md, section 2).
Needs: 03.7.
Files: `crates/smith-local-service/src/service.rs`, `src/stream.rs`,
`src/process.rs` (the stream's io beside the process adapter's);
`crates/smith-local-shell/src/local_settings.rs` (`agent_document`),
`src/local_shell.rs`; `benchmarks/src/agent/smith/` (the agent's trace
path, so `smoke-tools` keeps reading the run's records).
Shape: the local service owns a `smith_agent_service::events::Producer`
for the spawned kind, made public for it, writing through the process
adapter's io; it takes no facts. Nothing else crosses a boundary.
Test shape: `tests/local-process/tests/events.rs` (03.7's file), the
spawned kind. Tests: `a_spawned_chat_writes_its_answer_and_end_to_the_product_stream`;
`the_spawned_agents_records_stay_in_its_own_trace`. Ceilings: 0.5 s each.
Proof: world, the two tests above: the product's stream holds one
`run.completed` and one `session.ended` for a spawned run, and the
agent's trace its own stream; they fail on the parent, which writes no
product stream with a spawned agent.
Don't: decode channel facts into records: they reach this stream only
as the agent's own trace (channel.md, section 8).
Size: at most 200 lines of production code.
Removes: nothing.

### 03.8 Grace, drop and settlement (the rest)
Scope: what 1a1e8a39 left of 03.8; the parent face's `Drop`, the slots'
grace and settlement, the first cause and the host world's stories are
on `main`. Left: the local service writes `run.completed` for a run that
ends without answering, from the turns it was told (events.md, 3.3), its
class from the local domain's `EndCause`: `Cancelled` gives
`cancelled`, `dropped: true`; `Host { turn, reason }` gives `host`,
`reason` `store`, `turn`; `Agent` gives `agent`, without `ended` until
03.10. A cancelled answer carries `dropped: false`. The second interrupt's
story runs through the real shell for both kinds.
Needs: 03.7 for the inline kind's record; 03.9 for the spawned kind's.
Files: `crates/smith-local-service/src/stream.rs`, `src/service.rs`;
`crates/smith-local-domain/src/domain.rs`, `src/end.rs` (the cause and
the turns told handed to the service), `src/boundary.rs`;
`tests/local-process/src/terminal.rs` (two interrupts in one read of the
signal stream), `src/referee.rs` (`Ending::Dropped`, exit 130 and the
stderr line), `tests/local-process/tests/simulation.rs`.
Shape: the local domain's `Request::Unanswered { cause: EndCause, turns:
u32, spent: u64, read: Option<Token> }`, once, when its agent is `Gone`
without an answer; `stream::unanswered(&Request::Unanswered) -> Answered`.
Test shape: `tests/local-process/tests/simulation.rs`; reuse
`World::interrupted` (`tests/local-process/src/world.rs:315`),
`Terminal::interrupt` (`tests/local-process/src/terminal.rs:245`), the
referee's `Ending` (`tests/local-process/src/referee.rs:42`). Reference
test:
```rust
#[test]
fn an_interrupt_mid_turn_and_a_second_drop_the_run() {
    for placement in [Placement::Spawned, Placement::InProcess] {
        let files = Files::new();
        let mut world = World::dropped(12, &files, placement);
        world.settle();
        assert_eq!(world.exit(), Some(kernel::Exit::Code(130)));
        let errors = String::from_utf8_lossy(world.errors());
        assert!(errors.ends_with("smith: interrupted: the run was dropped before it answered\n"));
    }
}
```
`World::dropped` delivers both interrupts in one read of the signal
stream while the provider holds its response open, so the second reaches
the local domain before any terminal of the cancel. `World::errors` is
added beside `World::shown` (`world.rs:610`), returning the terminal's
(`terminal.rs:235`). Tests: that one;
`a_dropped_inline_run_completes_as_cancelled_and_dropped`;
`a_store_failure_completes_as_host_store_with_its_turn`;
`a_cancelled_answer_completes_with_dropped_false`. Ceilings: 0.5 s each.
Proof: world, `a_dropped_inline_run_completes_as_cancelled_and_dropped`,
reading `run.completed` from the product's trace: `failed`, `cancelled`,
`dropped: true`; it fails on the parent, which writes no such record.
`an_interrupt_mid_turn_and_a_second_drop_the_run` shows exit 130 and the
stderr line with either kind; it runs once, in the same focused run.
Don't: drop a run that has answered; invent an exit status the kit did
not report.
Size: at most 200 lines of production code.
Removes: nothing.

### 03.10 A spawned agent's end in the product's stream
Scope: a spawned agent that ends without answering completes as `agent`
with `ended`, its exit code or signal, from the host kit's evidence on
`Gone`; a forced end adds the host's `forced_exit` notice, with `exit`
and `forced` (events.md, 3.9 and 3.10).
Needs: 03.8 and 03.9; smith 06's exit evidence on `Gone` (06.1).
Files: `crates/smith-local-domain/src/end.rs`, `src/domain.rs`;
`crates/smith-local-service/src/stream.rs`.
Shape: `EndCause::Agent { ended: smith_events::AgentExit }`, filled from
06's evidence; nothing else crosses a boundary.
Test shape: `tests/local-process/tests/events.rs`; tests
`a_spawned_agent_that_exits_without_answering_completes_as_agent_ended`,
`a_forced_exit_is_a_host_notice_in_the_product_stream`. Ceilings: 0.5 s
each.
Proof: world, the two tests, reading the product's trace; they fail on
the parent, whose record has no `ended`.
Don't: carry the exit as text.
Size: at most 80 lines of production code.
Removes: `EndCause::Agent`'s doc line saying teardown adds the evidence.

## Interface sketches

**03.7.** The agent service's effects feed the producer they already own;
the local service drives them. `Answered` is what `run.completed` takes
from an answer, whoever meets it; `answer_record` keeps building it for
the agent process.

```rust
// crates/smith-agent-service/src/answers.rs
/// What `run.completed` takes from the answer its writer meets (events.md, 3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answered {
    pub status: smith_events::Status,
    pub failure: Option<smith_events::AnswerFailure>,
    pub result: Option<smith_events::RunResult>,
    pub turns: u64,
    pub spent: u64,
    pub read: Option<u64>,
}

// crates/smith-agent-service/src/service.rs
impl Effects {
    /// Adopt the product's trace; refused without an event configuration.
    pub fn adopt_destination(&mut self, fd: Fd, destination: Destination, delivery: Delivery) -> Result<(), Fd>;
    /// Record `session.started` before the first pass.
    pub fn start_session(&mut self, now: Time, header: smith_events::SessionStarted);
    /// The run's start, from the parent face's values (events.md, 3.3).
    pub fn starting(&mut self, activation: u64, messages: u64, charter: &run::Charter);
    /// Whether the sinks take another record this pass.
    pub fn takes(&self) -> bool;
    /// One fact of the inline run, in drain order.
    pub fn fact(&mut self, fact: &smith_domain::Fact);
    /// `run.completed`, once: the producer adds the run, wind-down, unread, usage and duration.
    pub fn answered(&mut self, now: Time, answered: Answered);
    /// The product's exit for `session.ended`; the stream closes after it.
    pub fn end(&mut self, exit: u8);
}

// crates/smith-local-domain/src/domain.rs
impl Domain {
    /// One fact of the inline run, oldest first; none for a spawned agent.
    pub fn pop_run_fact(&mut self) -> Option<smith_domain::Fact>;
}

// crates/smith-local-service/src/stream.rs
/// `run.completed`'s parts from the parent face's answer, for either kind.
pub fn answered(answer: &smith_host_domain::Answer, capture: Capture) -> Answered;
```

The header gains `chat: Option<Box<[u8]>>` after `mode`; under `mode`
`agent` the writer refuses it and the reader treats it as absent
(decisions.md, 12.19).

**03.8.** The record of a run without an answer:

```rust
// crates/smith-local-domain/src/boundary.rs (Request gains)
/// The agent went without answering; the service writes `run.completed`. Owes no terminal.
Unanswered { cause: EndCause, turns: u32, spent: u64, read: Option<Token> },

// crates/smith-local-service/src/stream.rs
/// `run.completed`'s parts for a run that went without answering (events.md, 3.3).
pub fn unanswered(cause: EndCause, turns: u32, spent: u64, read: Option<Token>) -> Answered;
```

## Questions to raise when reached

None: events.md, section 2 places each stream, and decisions.md, 12.19
settles `chat` under `mode` `agent`.

## Done when

- The product writes a stream with the inline kind: `heavy cargo nextest
  run -p smith-local-process-world -E 'test(/inline_chat|dropped|interrupt_mid_turn/)'`
  passes.
- The local host counts no fact unwritten: `rg -c agent_facts_not_written
  crates/smith-local-domain tests/local` prints nothing.
- The header carries the chat: `rg -l '"mode":"chat".*"chat":"' crates/smith-events/golden/v1`
  prints at least one golden.
- 03.7's Proof is recorded in its commit body: `git log origin/main
  --grep='Step: smith-03.7' --format=%B | rg -c '^Proof: world'` prints 1.
- A spawned agent's end is typed in the product's stream: `heavy cargo
  nextest run -p smith-local-process-world -E 'test(/agent_ended|forced_exit/)'`
  passes.
