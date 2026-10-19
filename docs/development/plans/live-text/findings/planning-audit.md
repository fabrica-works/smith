# The planner's code audit

2026-10-19. skein `origin/main` at `283a7ec` (its fake LLM's files unchanged
since `50f7436`, where the base run was taken), smith `origin/main` at
`e79b9a6b` (its code unchanged since `c241a7a4`), jig and temper at their
`origin/main`. Read by three Sonnet sub-agents and the planner, each name
checked with `rg`. The session files cite these entries by number.

## skein's fake LLM

- **A1. The script.** `testing/skein-fake-llm-domain/src/api.rs`:
  `Script { cue, turns }`, `Turn { lines, finish, tokens }`, `Line::{Opaque
  { bytes }, Text { text }, Call { name, arguments }}`. `Answer { parts,
  finish, usage }`, `Part::{Opaque, Text, ToolCall, ToolOutput}`.
- **A2. The domain.** `src/domain.rs`: `Event::Call { reply_to, query }`
  only; `Request::Reply { to: ReplyTo, result }` only; `MAX_OUT = 1`. A call
  is `State::Thinking { reply_to, result }` with one timer
  (`Deadlines<Id<Call>>`) at `now + latency`, the latency drawn per call
  from `Config::latency_min..=latency_max`; `fire` answers it and retires
  the call. The whole answer, a fault included, is decided at the call.
- **A3. Faults on a scripted turn** (`src/respond.rs`): random failures
  (overloaded, rate-limited, unavailable, too long, unauthorised) before the
  script; `refused` gives `ContentFilter` with the cut text `do`; a turn whose
  `tokens` pass the query's `max_tokens` is cut (`Length`, the text `do`, or
  its first call's arguments cut to 3 bytes).
- **A4. Sizes.** `respond.rs` 781 lines, `domain.rs` 356, `limits.rs` 178,
  `api.rs` 378; `testing/skein-fake-llm-protocol/src/provider.rs` 1031 (over
  800), `documents.rs` 623. The matches on `Line`, `Part` and `Request` in
  `limits.rs` and `respond.rs` are exhaustive; `wildcard_enum_match_arm` is
  denied.
- **A5. The byte peer's server** (`provider.rs`): `Server { answer:
  Option<api::Answer>, sequence: u32, .. }` streams the answer from
  `sequence` 0 when the `Reply` comes (`next_event`, one SSE event per
  `sse::Event::Sent`). `Service::target` destructures `Request::Reply`
  irrefutably. Routing is by `ReplyTo`'s `Token` (`Id::<Call>::from_token`).
- **A6. The dialects' events** (`documents.rs`): `anthropic_event` gives
  `message_start` at 0, then three events a part (block start, one delta,
  block stop), then `message_delta` and `message_stop`; `openai_event` gives
  `response.created` at 0, then two a part (`output_item.added`,
  `output_item.done`, the text whole in the latter), then
  `response.completed`. The fake emits no `response.output_text.delta`,
  though `skein_llm::openai::Event::TextDelta` exists. Anthropic's keep-alive
  is `ping` (`anthropic::Event::Progress`, already written for an opaque
  part); OpenAI's Responses dialect has none.
- **A7. The hosts of the domain.** `tests/llm/src/fake.rs` `Exchange` (it steps
  and fires the domain, with the real `skein-llm` client in one process);
  `tests/llm-connection/src/world.rs` (steps and fires it too) and
  `simulated.rs` (builds a `Domain` for a `Peer`); `testing/skein-fake-peers/
  src/llm.rs` `Peer`, a `skein_world::Host` on loopback that owns its
  `Domain`, with `new`, `address`, `observations`, `shutdown`,
  `ignore_half_close`, and no input for the domain.
- **A8. Users downstream.** 96 files use the fake crates (skein 14, smith 64,
  jig 5, temper 13). `Line::Text` is built at about 109 sites in 43 files;
  the builders are smith `tests/agent/src/script.rs` and
  `tests/protocol-llm/src/script.rs`, jig `tests/channel/src/fixtures.rs`
  (`provider_turns`, also feeding temper's one-process world), temper
  `tests/system/src/goals.rs` and `crates/temper/tests/support/`. Nothing
  outside skein matches on `Line`, and jig and temper match on neither
  `Event` nor `Request`; smith's tests match on both (A30), so a new
  `Event` or `Request` variant reaches smith's repin.
- **A9. The tools plan** (temper `docs/development/plans/tools/`, on
  hetzner). Its skein steps change `skein-llm`'s request table
  (`requests.rs`), which the fake crates do not use: no file overlaps this
  plan's. Its next part's 08.2 hosts `Peer::new` on `127.0.0.1:0`,
  plaintext, latency zero, and says "Don't change skein's fake LLM": it
  builds `Peer` as it is, and a `Peer::release` added beside `new` changes
  nothing it calls. Its 09.2 and 09.3 hold a log write, not a reply.
- **A10. Test homes.** Domain: `respond.rs`'s and `domain.rs`'s test
  modules (helpers `script`, `says`). Protocol: `documents.rs`' tests and
  `tests/llm/tests/fake.rs`. Worlds: `tests/llm/tests/fake.rs`,
  `tests/llm-connection/tests/fake_protocol.rs`,
  `testing/skein-fake-peers/tests/llm.rs`.

## smith's channel

- **A11. The schema.** `crates/smith-channel/schema/channel-v2.schema`,
  `record Start { identity, activation, charter, workspace, transcript,
  answered, grants, window, messages }`; generated `src/generated/v2.rs`;
  about 440 goldens in `golden/v2`. The drift test
  (`tests/codecs/tests/drift.rs`) compares, never writes.
- **A12. Version 2, hard-coded:** `crates/smith-channel/src/kinds.rs`
  (`Version { version: 2 }`, `KindRule::FORMAT_VERSION`, `V2`, "V2 has
  exactly 18 kinds", `family smith-channel 2`); `pub mod v2` in its
  `lib.rs`; `smith-agent-service/src/profile.rs` `channel_output`
  (`schema.version(2)`) and its kinds; `smith-protocol-channel/src/
  component.rs` `Request::Accept { version: highest.min(2) }`.
- **A13. A new kind** takes 0x0112: `Body` and `V2`'s list, the
  `worst_case_bytes` match, `lib.rs`' re-exports, `List::with_capacity(18)`;
  tests `tests/codecs/tests/channel.rs` (asserts 18) and `fuzzy_decode.rs`.
- **A14. The agent's half.** `smith-protocol-channel` `Component`:
  `offer_fact` (only while `Admitted`, with room past `fact_reserve_frames`
  and `fact_reserve_bytes`), `send_text_arrived`, `lost_facts: u64` reaching
  `Loss.channel`; `send_turn` keeps `number == last_sent_turn + 1` and the
  window.
- **A15. The queue's bound.** `smith-agent-service/src/profile.rs`
  `channel_limits`: `output_frames` = turns + conversations + messages + 2 ×
  long + 2; `fact_reserve_frames: 1`. `channel_bodies` starts from
  `smith_channel::CEILINGS`.
- **A16. The host's half.** `smith-host-protocol` `component.rs` `drain`,
  per-kind phase guards, `refuse_rules` (reason 256 bytes); `OpenEvent`
  (`Fact`, `Opened`, ...) wrapped as `ProcessEvent::Channel { agent, event }`
  by `process.rs` `drain_channel`. `translate.rs` `encode_start(start,
  window, values, limits, ..)`, `Values { paths, credentials }`.
- **A17. Turn order and the watchdog** are `smith-host-domain/src/
  domain.rs`: `valid_record`, `receive`, `Alarm::Watch` (see A27).
- **A18. Policy.** `smith-agent-service/src/profile/quantities.rs`
  `standard()`. A new `Policy` field also reaches `profile/validate.rs`
  `policy_deadlines`, `profile/effective/list.rs` `policy`,
  `smith-agent-shell` `config.rs`, `smith-local-shell` `settings/schema.rs`
  and `settings/resolve/document.rs`, and shell.md 6.5 (the designer's).
- **A19. The channel component's limits.** `smith_protocol_channel::Limits`
  (`src/limits.rs`) is a struct literal in its tests, in `tests/channel`'s
  `World::new`, in `service.rs`' tests and in `profile/effective/
  channel.rs`' twins.
- **A20. The memory sum.** `service.rs` `worst_case` adds
  `smith_protocol_channel::worst_case`; the startup check is
  `validate::memory` against 2_214_592_512, tested in agent-shell
  `config.rs` and `tests/channel/tests/memory.rs`.
- **A21. The channel world** (`tests/channel`, `smith-channel-world`):
  `World`, `Peer`, `agent_sends_fact`, the referee (`src/referee.rs`:
  `Meeting`, `Seen`, `review`), about 40 stories, none with an LLM;
  `tests/composed.rs` runs the real `smith-domain`.

## smith's LLM component and service

- **A22. Deltas.** `smith-protocol-llm` `Component::delta`: text sends a
  count, `ToDomain::Text { owner, bytes }`, and `Measure::Delta { at, owner,
  block, text }` when `Observation.live_text` is set; reasoning sends a
  count only; tool arguments are dropped. `Measure::Delta` has no kind.
- **A23. Who asked.** `Observation { first, messages, live_text }` is built
  in `Sinks::request`; `Book::observation` sets `live_text: true`.
  `Active { context, outcome, deliver, observation }` keeps neither
  `Context.asker` (`Main` or `Child`) nor `llm::Purpose` (`Ordinary` or
  `Compaction`), both on the input.
- **A24. The service.** `llm_measures` → `Sinks::measure` →
  `Event::TextDelta`, with no gate per consumer. `domain_request` drops
  `Request::Complete`'s `name` (`CompletionName { sequence, retry }`).
  `Request::Turn { number, position }` goes to `send_turn`. Time enters
  through `iterate(service, now, wall)`; a deadline joins `next_deadline`,
  `work_pending`, `settled` and `teardown.rs`.
- **A25. In process.** `smith-inline-agent` has no LLM component;
  `smith-local-service` has `Effects::Inline`.
- **A26. The worlds.** `tests/agent-process` observes frames of kinds
  0x0106, 0x0109, 0x010f and 0x0110, at zero latency, through
  `fake::configured`, with the Start builder `start_workspace_message`;
  `tests/protocol-llm`'s `pool_peer.rs` and `component_peer.rs` match
  `Measure::Delta`. Test homes: `content_observations.rs`,
  `component_peer.rs`, `component_boundary.rs`, agent-process `content.rs`,
  `fact_drain.rs`.
- **A27. The watchdog.** `smith-host-domain` `domain.rs` `receive` sets
  `agent.progress = env.now` on `process::Event::Received { owner, message:
  Up }`, after the checks that the agent is live. A record reaches it by
  `Component::drain` → `Process::drain_channel` (`ProcessEvent::Channel`)
  → `smith-local-service` `process.rs` `to_host_event` → `receive`. A new
  `Up` variant would reach four matches (`receive`, `discarded_work`,
  `too_large`, `valid_record`); the smallest signal without content is a
  new `process::Event::Progress { owner }` whose arm only sets `progress`
  after the same checks.
- **A28. The start.** `smith_host_domain::channel::Start { identity,
  messages, logical_run, activation, workspace, charter, transcript,
  answered, directories, grants }`, no `Default`; `channel::Ask` is taken
  (what a call asks). It is a literal at the host-domain and host-protocol
  tests, `smith-local-domain` `agent.rs`, `smith-inline-agent`'s tests,
  `tests/hosts/src/lib.rs`, `tests/host/src/{lib.rs, inline.rs}`,
  `tests/host/tests/memory.rs`, and `tests/channel/src/{lib.rs,
  stories.rs, composed.rs}`, and downstream (A32). `translate.rs`
  `Values { paths, credentials }` has no `Default`; a field on the domain's
  `Start` leaves it alone. The wire's way: the schema's `record Start`, the
  generated code and goldens, `encode_start`, and the start's parts as a
  literal in `tests/agent-process/src/lib.rs`.
- **A29. The host's events.** `OpenEvent` is matched, with no wildcard, in
  `smith-host-protocol` `component.rs` `drain` and `process.rs`
  `drain_channel`, `smith-local-service` `process.rs` `to_host_event`,
  `tests/hosts/src/lib.rs` `route_channel`, `tests/channel/src/lib.rs`, and
  downstream in jig `jig-test-service` and temper `temper-service`
  `worker_route.rs`. A `ProcessEvent` variant would also reach both
  downstream `worker.rs`. `Queue::<OpenEvent>::worst_case(1)` grows with a
  variant that carries bytes. `kinds: 18` is written out in the component's
  tests and `tests/channel/src/lib.rs`. `smith-local-service` reads one
  record a read, in `drain_process` and `read_terminal`, and drops what
  neither handles.
- **A30. The fake LLM in smith's tests.** `let provider::Request::Reply {
  .. }` irrefutably in `tests/agent/src/world.rs`, `tests/local/src/
  world.rs` and `tests/protocol-llm/src/world.rs`; a match arm in
  `tests/session/src/world.rs`; `while let Some(provider::Request::Reply {
  .. })` in `tests/host/src/inline.rs` and `tests/protocol-llm/src/
  wire.rs`, which would stop at a new variant rather than fail; `for
  provider::Event::Call { .. } in take(..)` irrefutably in
  `tests/protocol-llm/src/wire.rs`. No match on `Line`; none in jig or
  temper.
- **A31. Turns, retries and the worlds.** `domain_request` drops
  `Request::Complete`'s `name: CompletionName { sequence, retry }`
  (`retry` from 0, raised when a call fails). `Request::Turn { number,
  position }`'s number is the activation's and the main conversation's
  (a child's turns stop in `route.rs`); the session's `Turn` comes before
  the next `Complete`. `Prompt`'s `asker` (`Main`, `Child`) and `purpose`
  (`Ordinary`, `Compaction { room }`) are on the `Complete`, so the service
  can tell a main, ordinary completion there. The component's
  `last_sent_turn` is private. `answered_late` (`smith-domain-session`
  `session.rs`) is a turn recorded after its completion was given up. The
  worlds: `tests/hosts` (`smith-hosts-world`, the real host `Process`,
  `PipeHost<Agent>` and the fake LLM in one simulated process);
  `tests/local-process`; `tests/agent-process` (a scripted host, raw
  frames, latency zero, a reply's text in one delta); `tests/channel`
  (`Observation::HostTurn`, `HostFact`).

## Downstream of smith

- **A32. The start's literal.** `smith_host_domain::channel::Start` is
  built as a struct literal at 5 sites: jig `testing/jig-test-service/src/
  worker_route.rs` and `testing/jig-test-worker/src/tests/fixtures.rs`,
  temper `crates/temper-service/src/worker_route.rs`, `crates/temper-worker-
  domain/src/tests/fixtures.rs` and `tests/worker/domain/src/protocol.rs`.
  `encode_start` is called once downstream, in temper-service's
  `src/tests/worker.rs`. Both `worker_route.rs`
  match on the owner's events. jig names `smith_channel::v2` in four
  test files (`tests/core/src/protocol.rs`, `tests/core/src/
  agents_tools.rs`, `tests/ops/src/conformance/scripts.rs`,
  `testing/jig-test-service/src/tests/fixtures.rs`: `CallName`,
  `CallNameParts`, `CEILINGS`); temper none. The director accepted the repins' share
  (decisions.md, Design questions, 4).
