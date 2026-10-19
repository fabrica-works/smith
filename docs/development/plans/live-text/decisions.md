# Live-text plan: decisions

2026-10-19. The choices this plan makes within the design's latitude,
each with its reason. The design is the contract: skein's
`docs/design/fake-llm.md` (2.2, at skein `c551a66`) and smith's
`docs/design/protocol/channel.md`, `protocol/limits.md`,
`protocol/llm.md`, `domain/host.md` and `domain/run.md` (at smith
`c241a7a4`; the LLM accounts design, landed at `e79b9a6b`, changed none
of what they say of live text). skein's `origin/main` is at `49725ec`
and smith's at `137a1cb4`; neither changes the fake LLM or what this
plan touches since the audit (skein's commits since `aba0c6f` are its
forge crates and `oauth.md`). smith pins skein at `93052c6b`, the pin
the tools plan's smith repin landed (smith `d56bbe3c`). The audit
entries (A1 to A32) are in `findings/planning-audit.md`. The last three
sections hold the design questions with their answers, what goes to the
designer, and what the user may veto.

## 1. skein's fake LLM: a reply in pieces, and its holds

1. **The script.** A new line, `Line::Pieces { pieces: Box<[Piece]> }`,
   with `Piece { text: Box<[u8]>, hold: Option<Box<[u8]>> }`: a hold
   names the place before its piece. `Line::Text` stays one piece, so
   the 109 builders of `Line::Text` (A8) do not change. A turn has at
   most one `Pieces` line, and it is the turn's first line; it holds at
   least one piece, no piece's text is empty, and a hold's name is 1 to
   64 bytes. The lines after it go whole when the reply ends.
   `Domain::configured` refuses a script that breaks this with
   `Error::InvalidRequest`. Why: 2.2 gives "the
   reply's text" as one ordered list, and a first line lets the stream
   open on it, so the server never writes a part out of order.
2. **What the domain sends.** A call's stream is named by a token the
   protocol gives with the call: `Event::Call` gains `call: Token`, and
   two requests carry it, `Request::Piece { call, usage, text }` and
   `Request::Hold { call, usage }`. `usage` is the reply's, decided at
   the call: the dialect's opening event reports it (Anthropic's
   `message_start`, which skein-llm's client reads), and the server opens
   the stream before the `Reply` reaches it. `Request::Reply` stays the
   call's one terminal and carries the whole answer; the server writes from it only
   what the pieces did not, the last piece among it. Why: a `ReplyTo` is
   consumed by the one reply and cannot be read before
   (`skein_lib::ReplyTo`), and programming-model.md 4.2 has a layer echo
   a token on every record about an entity. The 6 smith worlds that
   build `Event::Call` add `call` at smith's repin (A30).
3. **Timing.** Each fire sends one request, so `MAX_OUT` stays 1: the
   call's first fire, at its latency, sends its first piece, or a `Hold`
   when that piece is held; each later piece waits its own latency,
   drawn per piece; the last piece goes as the `Reply`. A held call has
   no timer, so a world settles while it waits. The server opens the
   stream at a call's first `Piece` or `Hold`, and writes the dialect's
   keep-alive once at each `Hold` (Anthropic's `ping`; OpenAI's Responses
   has none). A
   hold longer than the client's idle deadline times out, as a silent
   provider would. No `Config` field is added: `Config` is a struct
   literal downstream.
4. **Faults.** A fault is decided at the call (A2, A3). A faulted reply
   sends no piece and no hold: one `Reply`, as before. So "a reply cut
   or refused by a fault while held" is a cut reply whose text was
   scripted in pieces: it ends as the fault says, and its holds go with
   it. A reply cut by the query's `max_tokens`, or answered without its
   script (`refused`, `no_calls`), goes whole too: a call streams only
   when its answer's first part is its turn's pieces joined.
5. **Releases.** `Event::Release { name: Box<[u8]> }` frees every call
   held at that name; a release that finds no call held at it changes
   nothing, including a release before the stream reaches its hold.
   A released piece is sent at its own latency.
6. **Memory.** A call keeps its place in the script (the script and the
   turn, and the next piece), never a copy of the pieces, so
   `limits::worst_case` grows by fixed fields only.
7. **The server.** A new module of `skein-fake-llm-protocol` writes the
   pieces (`provider.rs` is 1031 lines, A4): Anthropic, a
   `content_block_delta` per piece within the text block; OpenAI's
   Responses, a `response.output_text.delta` per piece, then the
   `output_item.done` with the whole text, as the dialect does.
   `Service::target` routes `Piece` and `Hold` by their `call`.
8. **The hosts.** `skein_fake_peers::llm::Peer::release(&mut self, name:
   &[u8]) -> Result<(), Error>` queues the name, and the next iteration
   feeds it to its domain; the queue holds `Config::calls` names, a name
   past 64 bytes or a full queue is `Error::Limits`, the peer's
   `worst_case` counts the queue, and `work_pending` is true while it
   holds one. `tests/llm`'s `Exchange::release` steps its domain at once,
   for the one-process tests. The llm-connection world's simulated peer
   gets one held story: the release over loopback with the real client,
   the only skein story that runs `Peer::release` against a client, as
   temper's real loop will.

## 2. smith's channel

1. **The ask** is `smith_host_domain::channel::Start`'s new field
   `live: Option<LiveAsk>`, with `LiveAsk { text, reasoning, input }`,
   three `bool`s (`channel::Ask` is taken, A28). On the wire, `record
   Start` gains `live: option LiveAsk`. `None` and every kind false both
   ask for nothing (channel.md 3). The agent's half hands the ask to the
   agent's service, never to the domain: only the service sends pieces.
2. **Version 3.** The channel moves from 2 to 3 once, in smith 01.1, and
   01.2 adds the live record under 3 with no further bump (development.md
   5.3; channel.md 5). Version 3 replaces version 2 in place, as 2
   replaced 1: the schema, `smith_channel::v2` and the goldens are
   renamed, so nothing keeps a version no peer speaks. Downstream, jig
   names `smith_channel::v2` in four test files (`tests/core/src/
   {protocol,agents_tools}.rs`, `tests/ops/src/conformance/scripts.rs`,
   `testing/jig-test-service/src/tests/fixtures.rs`); temper names it
   nowhere. Those paths become `v3` in jig's repin, beside answer 4's
   share.
3. **The live record** is kind 0x0112 (A13), `record Live { turn: u32
   retry: u32 kind: LiveKind offset: u64 bytes: bytes 4096 call: option
   LiveCall }`, with `enum LiveKind { text, reasoning, input }` and
   `record LiveCall { id: text 256 tool: text 64 }`, the bounds of the
   transcript's `Call` (limits.md 2.4). It is required in version 3,
   like every kind: the design names no optional kind, and the host's
   half must decode it to check its order.
4. **The host's half** keeps the ask it encoded, the last turn read, and
   for each kind the retry and the end of the last piece, and refuses a
   piece that breaks channel.md 5's order (`refuse_rules`, A16): before
   admitted or after the answer, of a kind not asked, of another turn, of
   an earlier retry, starting before the end of the one before, with a
   call on a kind other than `input`, or without one on `input`. A start
   past the end is a gap, never refused.
5. **The owner's event** is a new `OpenEvent::Live { turn, retry, kind,
   offset, bytes, call }`, which reaches the owner as
   `ProcessEvent::Channel { agent, event }`, naming the run (host.md 2).
   `smith-local-service`'s owner never asks, so it drops a piece after
   telling the watchdog.
6. **The watchdog** takes a piece as progress through a new
   `process::Event::Progress { owner }` of `smith-host-domain`, which
   only sets the agent's progress after the checks `receive` makes
   (A27). Why: an `Up` variant would reach four matches and the
   domain's record checks, and a piece is never the domain's (host.md 2).
7. **The lost live bytes** are counted by the agent's half beside
   `lost_facts`, as `lost_live_bytes: u64`, which the service reads. The
   event stream is unchanged (answer 1).
8. **The queue's reserve for pieces** is one piece's frame
   (limits.md 3.6): `channel_limits` adds one output frame and its bytes.

## 3. smith's LLM component and service

1. **The handover** (llm.md 6). `Observation.live_text` goes, and
   `FromDomain::Complete` gains `deltas: bool`: hand the completion's
   text deltas over as `Measure::Delta`. The service sets it when the
   event sink observes the request (`Sinks::request` returns `Some`) or
   when the ask names `text` and the `Prompt`'s asker is `Main` and its
   purpose `Ordinary` (A23, A31). `Measure::Delta` keeps no kind, until
   a second kind is built.
2. **The sink records only what it observed.** The service hands every
   `Measure::Delta` to the sink and to the live module; the sink records
   a delta only for a request it observed, and the live module takes
   only the owners it recorded at `Complete`. Why: a completion handed
   over for the channel alone would otherwise put `text.delta` in a
   stream that did not ask (A24).
3. **The live module,** a new `smith-agent-service/src/live.rs`
   (`service.rs` is 2920 lines). It keeps the last turn told
   (`Request::Turn`'s number, 0 at the start), and one stream: at a main,
   ordinary `Complete` when the ask names `text`, the owner, the turn
   (the last told plus one) and the retry (`CompletionName.retry`, A31).
   A main conversation has one completion in flight, so one stream is
   held at a time; a new retry ends the last one's stream first.
4. **Held bytes.** Each delta goes into the stream's held bytes, at most
   16 KiB (limits.md 3.6); past it the newest bytes are dropped and
   counted, and the next piece starts past them (a gap). A piece is cut
   from the held bytes, at most 4096. In 01.3 a piece goes whenever the
   channel has room; from 02.1, when the held bytes reach 4096 or
   `live_interval` has passed since the oldest unsent byte.
5. **A stream ends** when its turn is told (before the turn's record
   goes), or a retry of its turn starts: its held bytes go if the
   channel has room, and are dropped and counted if not. A delta of an
   ended stream, which the event sink's backpressure can delay past its
   turn (`llm_measures` stops while the sink takes nothing), is dropped
   and counted: no piece follows its turn (channel.md 5).
6. **`live_interval`** is a new `Policy` field, 100 ms (limits.md 2.3):
   `quantities::standard()`, `validate::policy_deadlines` and the
   effective list (A18). No shell setting names it: shell.md 6.5's
   policy values do not, and `connection_keep`, which they do, shows a
   setting is the designer's word ("For the designer" 4). The service's
   live deadline is a policy deadline: it joins `next_deadline`,
   `next_policy_deadline` and `work_pending`, and teardown settles with
   no held bytes (A24).
7. **Memory.** The service's worst case adds the held bytes once
   (limits.md 3.6, 3.8), and the channel's queue its one more frame
   (2.8). At the base the agent's complete worst case is 1,721,769,467
   bytes against its 2,214,592,512, a margin of 492,823,045
   (`findings/planning-commands.md`, section 4). The plan names a floor
   of 490,000,000 bytes: smith 01.1, the first increment that adds to
   the sum, asserts it in agent-shell's
   `product_values_pass_every_startup_check_and_fit_complete_memory`,
   and each later smith increment reports that test's `PRODUCT_COMPLETE`
   line.

## 4. The order

1. **smith 01 first.** Its first increment, version 3 and the ask, is
   where the two halves meet, and needs nothing of skein. Its stories run
   on skein's fake LLM as pinned, whose text comes in one delta a part:
   enough for pieces in order, the piece bound and no ask.
2. **skein 01 beside it.** It is small, and only smith 02's stories,
   which need a reply partway, wait for it.
3. **smith 02 last,** after smith 01's joint story [01.3] and skein's
   release [skein 01.3]; it starts with smith's repin of skein, from
   `93052c6b` to skein's `origin/main`. Moved to `aba0c6f`, whose code
   smith builds on `49725ec` keeps, the pin breaks nothing
   (`findings/planning-commands.md`, section 7), so the repin's commit
   adapts only what skein 01 changes (A30).

## 5. For the LLM accounts

The accounts' skein and smith part runs on this machine after this
plan's run.

1. **What it awaits from this plan:** skein 01 (the fake LLM's pieces
   and holds, on which its failing lines sit), smith 01.1 (the channel's
   version 3 and the start's ask, on which its version 4 sits) and smith
   01.3 (the LLM's handover, beside which its failures go). The README
   names them.
2. **A failure after pieces.** fake-llm.md 2.3's scripted failures may
   end a reply that has sent pieces, which this plan never does (1.4).
   The server keeps what it has written of a stream, so a failure after
   pieces is an event of the dialect written after them; the accounts'
   plan adds it.
3. **The start's shape.** The ask is a record of its own, the start's
   last field, so the accounts' changes to grants and their values leave
   it alone.
4. **The events' version** stays 3: the live bytes' field waits for the
   designer's next pass, outside the accounts' version 4 (answer 1).

## Design questions

1. **Where the live bytes not projected are told** (channel.md 7;
   events.md 3.2 and 6). (a) The agent's half keeps the count beside the
   facts'; the service reads it and the stories assert it; the event
   stream does not change in this plan. (b) A field now, the events'
   version 4. (c) Folded into `loss.channel`.
   Answer: (a), by the director, 2026-10-19.
2. **fake-llm.md 2.2's "the peer as a process of its own ... refuses a
   script with one at its start".** No such process exists: the byte
   peer is hosted by the worlds and by temper's real loop. (a)
   `Peer::release(name)`, fed to its domain at its next iteration, and
   no refusal; later.md names it. (b) `Peer` refuses a script with a
   hold, and the real loop cannot drive holds.
   Answer: (a), by the director, 2026-10-19.
3. **Main conversation only: for which consumer?** (llm.md 6; events.md
   3.5 and 8.) (a) The rule is the channel's: the event sink keeps every
   conversation's deltas, and only the main conversation's ordinary
   completions become pieces. (b) Literal: sub-agents and compactions
   lose `text.delta` too.
   Answer: (a), by the director, 2026-10-19.
4. **smith's host API change reaches the tools plan's closing repins**
   (A32): the start's new field at 5 struct literals, and `OpenEvent`'s
   new variant in both `worker_route.rs`. (a) Accept: those repins take
   `live: None` and an arm that ignores pieces. (b) Fence smith's steps
   after them.
   Answer: (a), by the director, 2026-10-19.
5. **A reply scripted whole** (fake-llm.md 2.2, "a reply scripted whole
   is one piece", each piece a delta; A6). (a) It keeps its base bytes
   in both dialects: one delta in Anthropic's, and in OpenAI's the whole
   text in its `output_item.done`, with no `response.output_text.delta`;
   only a `Line::Pieces` streams OpenAI's deltas, and the designer
   rewords 2.2. (b) Literal: every OpenAI reply gains one delta, and each
   smith world on OpenAI (`tests/agent-process/src/fake.rs`) a
   `TextArrived` fact and a `text.delta` event per reply, through smith's
   repin and jig's and temper's next.
   Answer: (a), by the director, 2026-10-19: Anthropic's worlds still
   cover the delta path for a whole reply.

## For the designer

1. **fake-llm.md 2.2 and 6, reworded** to answer 2: the byte peer takes
   a release from the world or the real loop that hosts it; the refusal
   belongs to a peer run as a process of its own, when one exists
   (later.md). With them, 2.2's "a reply scripted whole is one piece",
   to answer 5: a whole reply keeps its bytes in both dialects, and only
   a reply scripted in pieces streams OpenAI's text deltas.
2. **events.md: a field for the live bytes not projected,** at the
   designer's next pass, not in the accounts' version 4 (answer 1).
3. **llm.md 6's "main conversation only"** reads as the channel's rule,
   to answer 3.
4. **shell.md 6.5 and `live_interval`:** limits.md 2.3 names the policy
   value, shell.md 6.5's table does not; the plan adds no shell setting
   for it (3.6) until the table names one.
5. **fake-llm.md 2.2's "a reply cut or refused by a fault while held":**
   a fault is decided at the call (1.4), so a faulted reply goes whole,
   with no piece and no hold, and a fault never strikes while a reply is
   held. A fault after pieces waits for 2.3's scripted failures (5.2).
   The sentence could say so.

## Rulings

101. **smith-02's Shapes against main** (ready check of smith-02.1).
     smith-02.1's ready check, against skein 12305e5 (4a16854 at its end, a forge-only commit) and smith c0c89ddc, corrects `smith/02-best-effort.md`:
     - 02.1's sketch: `skein_lib::Time` has no `Add` or `Sub` (E0369 under clippy); it writes `AT_ZERO.saturating_add(interval)` and `Time::from_nanos(end.as_nanos() - 1)`, a form the check compiled under clippy and ran passing. Checked: `git -C skein show origin/main:crates/skein-lib/src/time.rs` has `Time::saturating_add`, `from_nanos`, `as_nanos`, and no `impl Add`.
     - 02.1's Files gain `crates/smith-agent-service/src/profile/tests.rs`, where `a_zero_live_interval_is_refused` goes; the test asserts `Check::Deadlines` and the refusal's whole reason as the code gives it.
     - 02.1's Shape: skein 01.1's `call: Token` on `provider::Event::Call` reaches each Call the worlds build and `tests/protocol-llm/src/wire.rs`' `for` pattern, all in Files (`git grep 'Event::Call {' origin/main -- tests/`: agent, host/inline, local, protocol-llm world and wire, session). 02.1 adds the interval to `Live::new` and `now` to `delta` and `pump`, `Default` to the tests' `Room` stub if 01.3's has none, and moves 01.3's unit tests to the new parameters.
     - `release_after`: the agent-process referee's `next_deadline` returns the release's instant, its `act` reads `now` (`_now` today), and `World::maximum_queries` (1 for this case today) is at least the call's `queries`, so 02.2's retry (2 queries) passes the referee's `fake::queries(peer).count() <= self.maximum_queries`.
     A further copy of any of these (another `Time` sum in a sketch, another `Event::Call` built in a world): correct it the same way, and list it in the commit body.

102. **Test shape before a reference test, README rule 7** (steer of 2026-10-19T10:26:53Z).
     The README gains rule 7, "Test shape: testing-strategy.md section 10 before a reference test", from the check of 2026-10-19T10:14:18Z: from smith 01.3's next increment to the run's end, a test keeps section 10's rules (one behaviour per test, rows carry data, a setup written once, named fixtures and sizes, a bound's two sides) where a session file's reference test breaks them, with the splits, the table and the named constants it lists. Landed tests stay as they are. Checked: `git -C smith grep -n 'sent > 0 && sent < 8\|first()\|get(1)' origin/main -- tests/channel/tests/live.rs` prints lines 61, 72 and 127; `Observation::HostLive` appears there 5 times.

103. **service::Limits carries live_interval** (cross-session scope, smith-02's block of 2026-10-19T10:33:00Z).
     A plan defect: 02.1's Shape says "02.1 adds the interval to `Live::new`", but the service builds its `Live` from `service::Limits` alone, which `profile::derive` fills in `src/profile.rs`, a file 02.1's Files missed. Your proposal is the plan's path: `smith_agent_service::Limits` gains `pub live_interval: Duration`; `profile::derive` sets it from `policy.live_interval`; the service passes `limits.live_interval` to `Live::new`; `service.rs`' test `limits()` sets `quantities::standard().policy.live_interval`. No new constructor or configuration API. 02.1's Files gain `smith-agent-service/src/profile.rs`, and its Shape and sketches say this (ruling 103).

     The type's struct literals on smith's `main` are those two; the other `service::Limits { .. }` literals (`smith-local-shell/src/startup.rs`, `tests/local-process/src/process/configuration.rs`) build `smith_local_service::Limits` and stay as they are. jig, temper and hearth build none: temper's `agent_limits()` calls `profile::derive`. Refresh any further literal of `smith_agent_service::Limits` you meet the same way, and list it in the commit body.

104. **smith-02's Shapes against main, re-check** (ready check of smith-02.1).
     smith-02's re-check against skein 5c8ae96 and smith 9da8ce13 (01.3 landed), in a scratch worktree with 02.1 and 02.2 applied: clippy and fmt pass, `smith-agent-service` 152 of 152, the workspace 2381 of 2382 (the one, `smith-bench`'s probe, refuses the scratch's dirty tree), fuzzy 43 of 43, and both `tests/agent-process/tests/live.rs` stories pass. `smith/02-best-effort.md` is corrected to what compiled:
     - 02.1's A30 arms: `let .. else { panic!(..) }`, a `match` names `Piece { .. } | Hold { .. }` (a wildcard arm fails `clippy::wildcard_enum_match_arm`), and `wire.rs`' `for` lets-else each event inside the loop (`Event::Release` makes it refutable).
     - 02.1's live module: `now` also reaches `Action::{Delta, Pump}`, `Live::act` and `live_step`; `open` and `end` take none; `deadline()` returns a stored due time (`Option::map` is disallowed); the settling is `settled()` in `service.rs`, so `src/teardown.rs` leaves Files; the effective list's `service` destructure names `live_interval: _` (a user ruling 103 missed).
     - A past live deadline with no channel room keeps no work pending: the held bytes wait for room (channel.md 7), and the service does not spin.
     - 01.3's three tests that send a short piece at once pump at the interval's end after their delta.
     - The sketch uses `crate::profile::standard()`, `ask(true, false, false)`, `Room::new(true)`, `open(.., &mut room)`, `Time::ZERO` and `room.offsets()`.
     - The world keeps `frames_at_release` in an `Rc<RefCell<..>>` the `World` holds, and `maximum_queries` is the larger of `release_after`'s queries and the case's bound.
     - The two release stories call no `check_memory`: skein's `Peer::release` copies the hold's name outside every process meter; 02.1's Proof drops "within the checked memory", and later.md has the row.
     - 02.2's sketch reads `retry()`, `offset()` and `bytes()`, and `held_after` puts the hold on the piece that waits ("lo").

     Not checked by it: the `tests/hosts` story (grepped only), the profile test's reason string, the bound and no-room unit stories. A further site the same change breaks gets the same correction, listed in the commit body.

105. **the no-room story proves the bound and the turn; the service forwards lost_live_bytes** (plan defect, smith-02's block of 2026-10-19T10:59:13Z).
     A plan defect: 02.2's Test shape asks a one-frame channel for "the next piece's offset past the gap", but `Component::takes_live` never gives that channel a piece, and `channel_loss` (facts refused) is not the live count. Your option (a) is the ruling. The world story becomes `a_channel_with_no_room_holds_the_bound_and_drops_the_newest`: `observe_facts` with one frame, one table of two rows, a reply of the held bound (16 KiB) and one byte past it, each row with the `lost_live_bytes()` the code counts; no piece sent, the turn's record still sent, `check_memory`. The gap in the next piece's offset (channel.md 7) stays proven by `live.rs`' `a_gap_survives_partial_draining_and_later_held_bytes`; no world restores room, and production framing limits stay as they are.

     The one production change: `Service::lost_live_bytes()` in `smith-agent-service/src/service.rs`, beside `lost_channel_facts()` and as it does, forwarding the channel component's `lost_live_bytes()`. 02.2's Files gain `service.rs` for that accessor alone, and `tests/agent-process/src/tracing.rs` if the world keeps the observation there, as it keeps `channel_loss` (ruling 105). A further user the accessor or the rename needs gets the same treatment, listed in the commit body.

106. **Done when item 8: smith's focused aim, measured with the logs outside the tree** (done when run of 2026-10-19T11:33:25Z).
     Done when item 8, smith's focused suite, as written, measured nothing: the command writes `focused-N.log` and `fuzzy-N.log` into the worktree, and `smith-bench`'s `probe_command a_replay_arm_runs_through_the_probe_command_and_prints_one_line` refuses the dirty tree ("source repository is dirty; commit or isolate changes before building an arm", `benchmarks/src/arm.rs:97`), so each run aborted after about 15 of 2388 tests. Measured once more with the logs outside the tree, in a clean detached worktree of smith `ff623361` (code as at cd76fb64; the commits between are plan documents): 2388 of 2388 passed in each of three runs, sums 11.38, 11.62 and 12.67 s, median 11.62 s against the aim of 12 s: the aim holds. The run is done on it. The miss is the command's, not the code's: a Done when command that writes into the worktree it measures meets a test that wants a clean tree; for the retrospective, the aims command of later plans writes its logs outside the worktree.

## For the user

Each is a choice that outlasts the run; a `Decision:` line under it
answers it.

1. skein's script gives a reply in pieces as one `Line::Pieces`, the
   turn's first line, with a named hold before any piece; `Line::Text`
   stays one piece (1.1).
   Decision: accepted, by director, 2026-10-19.
2. A held reply writes the dialect's keep-alive once, not periodically,
   so a hold past the client's idle deadline times out, and a world
   settles while a reply is held (1.3).
   Decision: accepted, by director, 2026-10-19.
3. A release that comes before its hold is reached changes nothing, and
   one release frees every call held at its name (1.5).
   Decision: accepted, by director, 2026-10-19.
4. The start's ask is `live: Option<LiveAsk>` in smith's host API, and
   `live: None` is "no ask" downstream (2.1).
   Decision: accepted, by director, 2026-10-19.
5. The live record is a required kind of version 3 (2.3).
   Decision: accepted, by director, 2026-10-19.
6. The live bytes not projected are counted in the agent's channel half
   and reach no event (2.7; answer 1).
   Decision: accepted, by director, 2026-10-19.
7. The event stream keeps every conversation's `text.delta`; pieces come
   only from the main conversation's ordinary completions (3.1; answer
   3).
   Decision: accepted, by director, 2026-10-19.
8. A delta that comes after its turn is told, the event sink's
   backpressure included, is dropped and counted, never sent (3.5).
   Decision: accepted, by director, 2026-10-19.
9. No repin of jig or temper in this plan: the tools plan's closing
   repins take `live: None`, the ignored piece and, in jig, four
   `smith_channel::v3` paths (2.2; answer 4).
   Decision: accepted, by director, 2026-10-19.
10. `live_interval` is a policy value with its default, and no shell
    setting until shell.md 6.5 names one (3.6).
    Decision: accepted, by director, 2026-10-19.
