# Session 01: the ask, the live record, and the handover

2026-10-19. A host asks for live content in the start, and the run sends
its main conversation's text up the channel as pieces before the turn's
record, best effort (channel.md 3, 4, 5, 7). This session moves the
channel to version 3 with the ask (01.1), adds the live record with the
host half's checks and the owner's event (01.2), and has the LLM
component hand deltas to the service, which sends them as pieces (01.3).
It runs on skein as pinned, whose Anthropic dialect writes a part's text
in one delta: enough for pieces in order, the piece bound and the asks.
Pieces partway through a reply wait for skein 01 (session 02).

## Read first

- smith `docs/design/protocol/channel.md`, sections 3, 4, 5, 7 and 10;
  `protocol/limits.md` 2.3, 2.4, 3.6 and 11; `protocol/llm.md` 6;
  `domain/host.md` 2 and 3.
- `decisions.md`, sections 2, 3 and 5, and "For the user" 4 to 8;
  `findings/planning-audit.md` A11 to A31.
- skein `programming-model.md`, sections 3, 4.2, 5.2, 6.3 and 7;
  `testing-strategy.md`, sections 2.1, 2.5, 2.8, 4.1, 6 and 10.

## What the design asks

- **channel.md 3:** the start carries the ask, one boolean per kind; no
  ask, or every kind false, asks for nothing; an ask for a kind not
  built gets nothing and is not refused.
- **channel.md 4:** a `live` record with `turn`, `retry`, `kind`,
  `offset`, `bytes` and, for `input` only, the call.
- **channel.md 5:** version 3, both halves together; a piece only while
  admitted and not answered, of a kind asked and built, for the last
  turn told plus one; no piece follows its turn; offsets never go back.
- **channel.md 7, limits.md 3.6:** pieces at most 4096 bytes, held bytes
  at most 16 KiB, past them the newest dropped and counted; the queue's
  reserve for pieces is one piece's frame.
- **llm.md 6:** the component hands deltas over when the event sink or
  the ask wants them; the service coalesces and names turn and retry.
- **host.md 2, 3:** the host half decodes a piece and hands it to the
  owner, naming the run; the domain never sees it; a piece is progress
  for the watchdog.

## Choices this plan makes

- The ask is `Start.live: Option<LiveAsk>` in the host's domain, and the
  generated `LiveAsk` record in `DecodedStart` (decisions.md 2.1).
- Version 3 replaces version 2 in place, as 2 replaced 1: the schema,
  the generated module and the goldens are renamed (2.2).
- The live record is 0x0112, a required kind (2.3).
- The host half's checks are a new module of `smith-host-protocol`
  (2.4); the watchdog hears a piece as `process::Event::Progress` (2.6).
- The lost live bytes are the agent half's count (2.7).
- Deltas go to the service by `Complete.deltas`; the sink keeps only
  what it observed (3.1, 3.2); the live module is new (3.3 to 3.5).

## Rules that bind it

- **Users outside Files** (README, rule 6): the start's literals (A28)
  and every match on `OpenEvent` (A29) change with the type.
- **Lint exceptions never land** (README, rule 5); `land` refuses the
  Pulse's words.
- **The memory margin** (README, rule 4): each increment reports
  `PRODUCT_COMPLETE`.

## Increments

### 01.1 Version 3 and the start's ask
Scope: channel.md 3 (the ask) and 5 (version 3); host.md 2 (decisions.md
  2.1, 2.2, 3.7; A11, A12, A28).
Needs: nothing.
Files: `crates/smith-channel` (`schema/channel-v2.schema` renamed
  `channel-v3.schema`, `src/generated/v3.rs`, `golden/v3`, `src/kinds.rs`,
  `src/lib.rs`); `smith-host-domain/src/channel.rs`;
  `smith-host-protocol/src/translate.rs` and the `smith_channel::v2`
  paths in `component.rs`, `limits.rs`, `tests.rs`;
  `smith-protocol-channel` (`src/translate.rs`, `src/component.rs`,
  `src/limits.rs`); `smith-agent-service/src/profile.rs`;
  `smith-agent-shell/src/config.rs`; the start's literals (A28);
  `tests/codecs/tests/drift.rs`, `tests/channel`, `tests/agent-process/
  src/lib.rs`.
Shape: the schema gains `record LiveAsk { text: bool reasoning: bool
  input: bool }` and `record Start`'s last field `live: option LiveAsk`;
  `family smith-channel 3`. `git mv` the schema, `src/generated/v2.rs`
  and `golden/v2` to their v3 names, then regenerate both with the
  command in `findings/planning-commands.md`, section 6, the schema and
  out-dir changed to v3. `kinds.rs`' version, `KindRule`'s format
  version and `V2` become 3 and `V3`; `lib.rs`' `pub mod v3`;
  `profile.rs`' `schema.version(3)`; the component's `Request::Accept {
  version: highest.min(3) }`. The host domain's `channel::Start` gains
  `live: Option<LiveAsk>` (sketch); `encode_start` writes it;
  `DecodedStart` gains `live: Option<smith_channel::LiveAsk>`, taken
  from the record. Every literal of the start takes `live: None`. The
  drift test reads `channel-v3.schema`, `generated/v3.rs` and
  `golden/v3`. agent-shell's
  `product_values_pass_every_startup_check_and_fit_complete_memory`
  asserts the margin at or above `490_000_000` bytes (decisions.md 3.7)
  and prints its line as now.
Test shape: `tests/channel/tests/live.rs`, a new file, with `World`,
  `CEILINGS` and `typed_charter` from `tests/channel/src/lib.rs`; the
  world gains `send_start_asking(charter, live)` beside `send_start`.
  Limits: `smith_channel::CEILINGS`, as the file's neighbours. Reference
  test:
  ```rust
  #[test]
  fn a_start_with_an_ask_reaches_the_agent_with_its_kinds() {
      let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
      world.settle();
      let ask = LiveAsk { text: true, reasoning: false, input: true };
      world.send_start_asking(charter(), Some(ask));
      world.settle();
      let start = world.take_agent_start().expect("the start reached the agent");
      let live = start.live.expect("the ask crossed");
      assert_eq!((live.text(), live.reasoning(), live.input()), (true, false, true));
  }
  ```
  (`charter()` is the smallest charter golden the file's neighbours
  include.) Beside it: `a_start_with_no_ask_reaches_the_agent_with_none`;
  `stories.rs`' `the_halves_open_on_the_highest_version_both_speak`
  expects 3. In `tests/codecs`: the regenerated goldens, and the drift
  test at v3. Each under 0.1 s.
Proof: world, `a_start_with_an_ask_reaches_the_agent_with_its_kinds`:
  both real halves open on version 3 and the host's ask reaches the
  agent's decoded start, kind for kind; under 0.1 s.
Don't: keep version 2's schema, module or goldens; add the live record
  (01.2); give the ask to the domain.
Size: 120 lines, the generated code apart.
Removes: version 2's schema, generated module and goldens.

### 01.2 The live record, the host's checks and the owner's event
Scope: channel.md 4 (the record), 5 (its order), 7 (best effort, the
  reserve); host.md 2 and 3 (decisions.md 2.3 to 2.8; A13 to A17, A19,
  A27, A29).
Needs: the channel's version 3, with the start's ask [01.1].
Files: `crates/smith-channel` (the schema, `generated/v3.rs`,
  `golden/v3`, `src/kinds.rs`, `src/lib.rs`); `smith-protocol-channel`
  (a new `src/live.rs`, `src/component.rs`, `src/limits.rs`);
  `smith-host-protocol` (a new `src/live.rs`, `src/component.rs`,
  `src/process.rs`); `smith-host-domain` (`src/process.rs`,
  `src/domain.rs`); `smith-local-service/src/process.rs`;
  `smith-agent-service/src/profile.rs` (`channel_limits`), its
  `profile/effective/channel.rs`; `tests/codecs`, `tests/channel`,
  `tests/hosts/src/lib.rs`. `smith-protocol-channel`'s `component.rs`
  is over 800 lines: its growth stays under 100, the piece's encoding
  and checks go in `live.rs`.
Shape: the schema adds `enum LiveKind { text, reasoning, input }`,
  `record LiveCall { id: text 256 tool: text 64 }` and `record Live {
  turn: u32 retry: u32 kind: LiveKind offset: u64 bytes: bytes 4096 call:
  option LiveCall }`, kind 0x0112 in `kinds.rs` (`V3` holds 19 kinds).
  The agent's half: `send_live` (sketch) frames a piece and sends it
  only while admitted and not answered, for a kind the start asked, for
  the last turn sent plus one, and with room past the reserve for facts
  and pieces; otherwise it drops it and adds its bytes to
  `lost_live_bytes`. `takes_live(bytes)` tells whether a piece of that
  size would go now; `lose_live(bytes)` counts bytes dropped above it.
  `channel_limits` adds one output frame and one piece's frame bytes to
  the reserve (decisions.md 2.8). The host's half: a `live` frame goes
  through `live.rs`' checks against the ask it encoded, the last turn
  read, and per kind the retry and the end of the last piece (decisions
  .md 2.4); a breach is refused by `refuse_rules`, its reason naming the
  rule; a piece that passes becomes `OpenEvent::Live`. `Process::
  drain_channel` passes it to the owner as `ProcessEvent::Channel`. The
  host's domain gains `process::Event::Progress { owner }`, whose arm in
  `receive` sets the agent's progress after the checks `Received`
  makes, and nothing else. `smith-local-service`'s `to_host_event`, and
  `tests/hosts`' `route_channel`, turn `OpenEvent::Live` into `Progress`
  and drop the piece.
Test shape: `smith-host-protocol/src/live.rs`' tests, a new kind (the
  checks as one table through `Streams::check`), reference:
  ```rust
  #[test]
  fn each_order_rule_refuses_its_breach() {
      let ask = LiveAsk { text: true, reasoning: false, input: true };
      let rows: [(&str, &[Piece], Option<Rule>); 6] = [
          ("in order", &[text(1, 0, 0, 3), text(1, 0, 3, 2)], None),
          ("a gap", &[text(1, 0, 0, 3), text(1, 0, 9, 2)], None),
          ("not asked", &[reasoning(1, 0, 0, 3)], Some(Rule::NotAsked)),
          ("another turn", &[text(2, 0, 0, 3)], Some(Rule::Turn)),
          ("back in the stream", &[text(1, 0, 0, 3), text(1, 0, 2, 2)], Some(Rule::Offset)),
          ("an earlier retry", &[text(1, 1, 0, 3), text(1, 0, 0, 3)], Some(Rule::Retry)),
      ];
      for (case, pieces, refused) in rows {
          let mut streams = Streams::new(Some(ask), 0);
          let last = pieces.iter().map(|piece| streams.check(piece)).find(Result::is_err);
          assert_eq!(last.map(Result::unwrap_err), refused, "{case}");
      }
  }
  ```
  (`text` and `reasoning` build a `Piece` of `turn, retry, offset,
  length`; the table grows by `input` without a call, a call on `text`,
  and a retry's first piece past 0, which is a gap, never refused.) In
  `tests/channel/tests/live.rs`, the world gains `agent_sends_live` and
  `Observation::HostLive`: `pieces_reach_the_host_in_order_before_their_
  turn`, `a_piece_after_its_turns_record_is_dropped_and_counted`,
  `pieces_before_admitted_or_after_the_answer_are_dropped_and_counted`,
  `a_piece_of_a_kind_not_asked_is_dropped_and_counted`,
  `a_full_queue_drops_a_piece_and_still_takes_the_turn`; the referee
  (`src/referee.rs`) adds no piece after its turn's record, before
  admitted or after the answer, nor of a kind not asked. In
  `smith-host-domain`'s tests: `a_piece_counts_as_progress_for_the_
  watchdog`. In `tests/codecs`: the kinds' list at 19, the record's
  goldens, `fuzzy_decode` by the list. Units under 20 ms, stories under
  0.1 s.
Proof: world, `pieces_reach_the_host_in_order_before_their_turn`: the
  real agent half's pieces cross to the real host half, pass its
  checks, and reach the owner as `OpenEvent::Live` with their turn,
  retry and offset, before the turn's record; under 0.1 s.
Don't: tell the host's domain of a piece; add an `Up` variant; produce
  `reasoning` or `input` pieces.
Size: 260 lines.
Removes: nothing.

### 01.3 The handover and the live module
Scope: llm.md 6; channel.md 7 (holding, dropping, no piece after its
  turn); limits.md 3.6, 3.8 and 11's live content (decisions.md 3.1 to
  3.5, 3.7; A22 to A24, A26, A31).
Needs: the live record, the host's checks and the owner's event
  [01.2].
Files: `smith-protocol-llm` (`src/boundary.rs`, `src/component.rs`,
  `src/capture.rs`) and the places that build `Observation` or
  `FromDomain::Complete`; `smith-agent-service` (a new `src/live.rs`,
  `src/service.rs`, `src/sinks.rs`, `src/responses.rs`, `src/lib.rs`);
  `tests/protocol-llm`, `tests/agent-process` (`src/world.rs`, a new
  `tests/live.rs`). `service.rs` is 2,920 lines: its growth stays under
  100; the holding and cutting go in `live.rs`.
Shape: `Observation.live_text` goes; `FromDomain::Complete` gains
  `deltas: bool`, and the component pushes `Measure::Delta` for an
  active call that has it. At `domain::Request::Complete`, the service
  asks the sink (`Sinks::request`) and `live::wanted(ask, &prompt)`,
  true when the ask names `text`, the asker is `Main` and the purpose
  `Ordinary`; `deltas` is either. When wanted, the live module opens a
  stream for the owner, turn the last told plus one, retry
  `name.retry`; a new retry of the open turn ends the last stream
  first. `llm_measures` hands a delta to the sink only for an owner it
  observed, and to the live module only for the open stream's owner;
  with no event owner, a delta goes to the live module alone. The
  module holds each delta's bytes, at most 16 KiB; past it the newest
  are dropped, given to `lose_live`, and the next piece starts past
  them. A piece is cut from the held bytes, at most 4096, and goes as
  soon as `takes_live` says so. At `Request::Turn` the service ends the
  stream before `send_turn`: the held bytes go if the channel has room,
  and are dropped and counted if not; a delta of an ended stream is
  dropped and counted. `service::worst_case` adds the held bytes once.
Test shape: `smith-agent-service/src/live.rs`' tests, a new kind, with
  a stub of the channel's room (`Room { takes: bool }`), reference:
  ```rust
  #[test]
  fn a_stream_past_the_held_bytes_drops_the_newest_and_counts_them() {
      let mut live = Live::new(Some(ask_text()));
      live.open(OWNER, 1, 0);
      let mut room = Room { takes: false };
      live.delta(OWNER, &[b'a'; 10_000], &mut room);
      live.delta(OWNER, &[b'b'; 10_000], &mut room);
      assert_eq!(room.lost, 20_000 - 16_384);
      room.takes = true;
      live.pump(&mut room);
      let offsets: Vec<_> = room.sent.iter().map(|piece| (piece.offset, piece.bytes.len())).collect();
      assert_eq!(offsets, [(0, 4096), (4096, 4096), (8192, 4096), (12_288, 4096)]);
      live.delta(OWNER, b"c", &mut room);
      live.pump(&mut room);
      assert_eq!(room.sent.last().map(|piece| piece.offset), Some(20_000), "the gap");
  }
  ```
  Beside it: `a_delta_past_the_piece_bound_goes_in_several_pieces`,
  `ending_a_turn_sends_or_drops_the_held_bytes_first`,
  `a_delta_after_its_turn_is_dropped_and_counted`,
  `a_new_retry_starts_again_at_offset_zero`, and
  `only_the_main_ordinary_completion_is_wanted` (one table: `Main` and
  `Child` by `Ordinary` and `Compaction`, by the ask's kinds). In
  `smith-protocol-llm`'s tests: `deltas_are_handed_over_only_when_asked`.
  In `tests/agent-process/tests/live.rs` (`World`, `anthropic`,
  `observe_facts`, `check_memory`; the world gains `ask(text, reasoning,
  input)`, which puts the ask in its start, and `pieces()`, the 0x0112
  frames it observed, decoded, in order):
  `a_start_asking_for_text_gets_the_reply_as_pieces_before_its_turn`,
  `no_ask_and_an_ask_without_text_give_no_piece` (one table),
  `a_sub_agents_completion_gives_no_piece`. Units under 20 ms, stories
  under 0.5 s.
Proof: world, `a_start_asking_for_text_gets_the_reply_as_pieces_before_
  its_turn`: the agent process, its real service and LLM component
  against the byte peer in Anthropic's dialect, sends the reply's text
  as pieces from offset 0, before the turn's record, their bytes joined
  equal to the literal text the script gives, within the checked
  memory; under 0.5 s.
Don't: send a piece on a timer (02.1); withhold `text.delta` from the event
  stream of a sub-agent or a compaction (answer 3); count a byte twice.
Size: 230 lines.
Removes: `Observation.live_text`.

## Interface sketches

```rust
// crates/smith-host-domain/src/channel.rs
/// The host's ask for live content, one boolean per kind (channel.md 3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LiveAsk { pub text: bool, pub reasoning: bool, pub input: bool }
pub struct Start { /* .. as now */ pub live: Option<LiveAsk> }

// crates/smith-host-domain/src/process.rs
pub enum Event {
    // .. as now.
    /// The agent's half read a piece: progress for the watchdog, no content.
    Progress { owner: Token },
}

// crates/smith-host-protocol/src/component.rs
pub enum OpenEvent {
    // .. as now.
    /// A piece of live content that passed the order checks, for the owner.
    Live { turn: u32, retry: u32, kind: smith_channel::LiveKind, offset: u64, bytes: Box<[u8]>, call: Option<smith_channel::LiveCall> },
}

// crates/smith-protocol-channel/src/component.rs
impl Component {
    /// Sends one piece, or drops it and counts its bytes; true when sent.
    pub fn send_live(&mut self, piece: Piece<'_>, token: Token, to_service: &mut Queue<OpenEvent>, below: &mut Queue<Lower>) -> Result<bool, Error>;
    /// Whether a piece of `bytes` would go now.
    pub fn takes_live(&self, bytes: u32) -> bool;
    /// Counts live bytes dropped before they reached the channel.
    pub fn lose_live(&mut self, bytes: u64);
    /// Live bytes not projected, beside `lost_facts`.
    pub fn lost_live_bytes(&self) -> u64;
}
/// One piece as the service cuts it.
pub struct Piece<'a> { pub turn: u32, pub retry: u32, pub offset: u64, pub bytes: &'a [u8] }

// crates/smith-protocol-llm/src/boundary.rs
pub enum FromDomain {
    Complete {
        // .. as now.
        /// Hand this completion's text deltas over as `Measure::Delta`.
        deltas: bool,
    },
    // ..
}
```

The host's domain owns the ask; the host's half owns the checks and the
pieces until the owner takes them; the agent's half owns the lost count;
the service's live module owns the held bytes and the stream's turn and
retry.

## Questions to raise when reached

None.

## Done when

```
git -C ~/src/rust/smith grep -c -E 'pub live: Option<LiveAsk>|Live \{|fn lost_live_bytes|deltas: bool' origin/main -- crates/smith-host-domain/src/channel.rs crates/smith-host-protocol/src/component.rs crates/smith-protocol-channel/src/component.rs crates/smith-protocol-llm/src/boundary.rs; git -C ~/src/rust/smith ls-tree --name-only origin/main crates/smith-channel/schema/ crates/smith-agent-service/src/live.rs
```

after `git -C ~/src/rust/smith fetch`, prints a count for each of the
four files, `channel-v3.schema` alone in the schema directory, and the
live module.
