# Session 02: pieces partway through a reply

2026-10-19. Once skein's fake LLM writes a reply in pieces and holds it
(skein 01), smith moves its pin to it and sends live text on a timer:
a piece goes when its bytes reach the bound or `live_interval` has passed
since the oldest unsent byte (limits.md 3.6). The repin and the interval
are 02.1; the stories that need a reply held partway, a retry's restart
at offset 0, a channel with no room, and the host's joint story, are
02.2.

## Read first

- smith `docs/design/protocol/limits.md` 2.3 (`live_interval`) and 3.6
  (live content); `protocol/channel.md` 4, 5 and 7; `protocol/llm.md` 6
  and 10 (retries); `domain/host.md` 3 (the watchdog).
- skein `docs/design/fake-llm.md` 2.2 (pieces, holds, the release).
- `decisions.md` 3.4 to 3.7 and section 4; "For the user" 2 and 3;
  `findings/planning-audit.md` A18, A24, A30, A31.
- skein `testing-strategy.md`, sections 2.5, 6 and 10.

## What the design asks

- **limits.md 2.3:** `live_interval`, a duration of the policy, 100 ms:
  the longest the agent holds live bytes before sending them.
- **limits.md 3.6:** a piece goes when its bytes reach 4096 or the
  interval ends since the oldest unsent byte, whichever comes first.
- **channel.md 4, 5:** a retry's pieces start again at offset 0, under
  the next retry number; the host keeps the last retry's.
- **channel.md 7:** a channel with no room holds the bytes, drops the
  newest past 16 KiB and counts them; the next piece leaves a gap; the
  turn's record still goes.
- **fake-llm.md 2.2:** a reply in pieces, each its own delta, waits at a
  named hold until whoever hosts the peer releases it.

## Choices this plan makes

- The repin moves every skein crate together (one git source); its
  commit adapts only what the move breaks (A30), as the base check shows
  (`findings/planning-commands.md`, section 7).
- `live_interval` is a `Policy` field with its validation and its line
  in the effective list; no shell setting names it until shell.md 6.5
  does (decisions.md 3.6; "For the designer" 4).
- The live deadline is a policy deadline: it joins `next_deadline` and
  `next_policy_deadline` (decisions.md 3.6).
- The worlds release a hold by `Peer::release`, after a number of
  queries and a delay, never by a script's timing (decisions.md 1.8).

## Rules that bind it

- **The repin** (README, rule 1): its own commit, at 02.1's start.
- **Users outside Files** (README, rule 6): every `Policy` destructure
  and every match on the fake LLM's `Event` and `Request` (A18, A30).
- **The memory margin** (README, rule 4): each increment reports
  `PRODUCT_COMPLETE`.

## Increments

### 02.1 The repin, and the live interval
Scope: limits.md 2.3 and 3.6's interval; channel.md 7's holding by time
  (decisions.md 3.4, 3.6; A18, A24, A30).
Needs: skein 01's release [01.3] on skein's `origin/main`; smith 01's
  handover [01.3].
Files: `Cargo.lock` (the repin); the A30 sites, `tests/agent/src/
  world.rs`, `tests/local/src/world.rs`, `tests/protocol-llm/src/
  {world,wire}.rs`, `tests/session/src/world.rs`, `tests/host/src/
  inline.rs`; `smith-agent-service` (`src/profile/quantities.rs`,
  `src/profile/validate.rs`, `src/profile/effective/list.rs`,
  `src/profile/tests.rs`, `src/live.rs`, `src/service.rs`,
  `src/teardown.rs`);
  `tests/agent-process` (`src/world.rs`, `src/fake.rs`, `tests/live.rs`).
Shape: first, its own commit: `heavy cargo update -p
  skein-lib` moves the pin to skein's `origin/main`; each
  irrefutable `let provider::Request::Reply { .. }` and `for provider::
  Event::Call { .. } in` becomes a `match` whose other arms panic with
  what came, and each `while let Some(provider::Request::Reply { .. })`
  keeps its loop and panics on another request, so none stops silently
  (A30); each `provider::Event::Call { .. }` the worlds build, and
  `tests/protocol-llm/src/wire.rs`' `for` pattern, takes skein 01.1's
  `call: Token`, a token that names the call among the world's calls in
  flight (skein's `provider.rs` passes `call: id.token()`). Then `Policy` gains `live_interval: Duration` (sketch),
  `standard()` 100 ms; `policy_deadlines` checks it like its neighbours;
  the effective list's destructure names it and writes its line. The
  live module keeps the time its oldest unsent byte arrived; `pump(now,
  ..)` sends a piece when the held bytes reach 4096 or `now` is at or
  past that time plus the interval, and the turn's end sends what is
  held whatever the time. `live::deadline()` is that time plus the
  interval while bytes are held; the service's `next_deadline`,
  `next_policy_deadline` and `work_pending` include it, and `teardown`'s
  settling waits for no held bytes. 02.1 adds the interval to
  `Live::new` and `now` to `delta` and `pump`, and gives the tests'
  `Room` stub `Default` if 01.3's has none; 01.3's unit tests take the
  new parameters, and those that send a short piece at once pass a time
  at the interval's end. `skein_lib::Time` has no `Add` or `Sub`: write
  `AT_ZERO.saturating_add(interval)`, and `Time::from_nanos(end.
  as_nanos() - 1)` for the instant before `end`.
Test shape: `smith-agent-service/src/live.rs`' tests, as 01.3's, with
  `quantities::standard().policy.live_interval`, reference:
  ```rust
  #[test]
  fn a_short_piece_goes_at_the_intervals_end() {
      let interval = quantities::standard().policy.live_interval;
      let mut live = Live::new(Some(ask_text()), interval);
      live.open(OWNER, 1, 0);
      let mut room = Room { takes: true, ..Room::default() };
      live.delta(OWNER, b"Hel", AT_ZERO, &mut room);
      let end = AT_ZERO.saturating_add(interval);
      live.pump(Time::from_nanos(end.as_nanos() - 1), &mut room);
      assert!(room.sent.is_empty(), "held before the interval ends");
      assert_eq!(live.deadline(), Some(end));
      live.pump(end, &mut room);
      assert_eq!(room.sent.iter().map(|piece| &*piece.bytes).collect::<Vec<_>>(), [b"Hel"]);
      assert_eq!(live.deadline(), None);
  }
  ```
  Beside it: `a_piece_reaching_the_bound_goes_before_the_interval`; the
  profile's tests (`src/profile/tests.rs`):
  `a_zero_live_interval_is_refused`, which asserts the refusal's
  `Check::Deadlines` and its whole reason, as the code gives it: in the
  ready check's probe, where the interval joined the deadlines that
  derive `deadline_minimum`, `policy.live_interval=0 ns;
  policy.deadline_minimum=0 ns`. In `tests/agent-process`, the world
  gains `script(turns)`, which gives the peer its turns in the case's
  place, and `release_after(name, queries, after)`: its referee calls
  `Peer::release` once the peer has taken `queries` queries and `after`
  has passed since, and keeps the host's frames observed then
  (`frames_at_release`). The referee's `next_deadline` in
  `tests/agent-process/src/world.rs` returns the release's instant while
  it waits, so that the world's clock reaches it, and its `act` reads
  its `now` (`_now` today). `World::maximum_queries` is at least
  `release_after`'s `queries`, so that the referee's `fake::queries(peer)
  .count() <= self.maximum_queries` holds for 02.2's retry (2 queries). In `tests/live.rs`:
  `a_held_reply_shows_its_first_piece_at_the_intervals_end` (a turn of
  `Line::Pieces` "Hel", hold `h`, "lo"; `release_after(b"h", 1, 1 s)`,
  under the world's 3 s idle deadline). Units under 20 ms, the story
  under 0.5 s.
Proof: world, `a_held_reply_shows_its_first_piece_at_the_intervals_end`:
  the agent process against the byte peer, which holds its reply after
  "Hel", sends a piece of "Hel" at offset 0 before the release, then
  "lo" at offset 3 and the turn's record after it; within the checked
  memory; under 0.5 s.
Don't: adapt in the repin's commit anything the move does not break;
  name `live_interval` in a shell's settings; send a piece on a timer of
  its own outside the service's deadlines.
Size: 100 lines, the repin's adaptations apart (test packages).
Removes: nothing.

### 02.2 A retry, a full channel, and the host's story
Scope: channel.md 4 and 5 (a retry restarts at 0), 7 (no room, a gap);
  host.md 3 (decisions.md 3.4, 3.5; A26, A31; "For the user" 2).
Needs: smith on skein's `main`, with the live interval [02.1].
Files: `tests/agent-process` (`src/world.rs`, `tests/live.rs`);
  `tests/hosts` (`src/world.rs`, `src/lib.rs`, its stories' file);
  `smith-local-service/src/process.rs` only if the joint story finds the
  piece's route short.
Shape: no production change is planned: the stories prove 01.3 and
  02.1 across a retry, a channel with no room and the real host. The
  agent-process world's `observe_facts` frames bound fills the channel;
  `tests/hosts`' `Seen` gains the owner's live pieces (turn, retry,
  offset, bytes), recorded by `route_channel` before it turns them into
  `Progress`, and the world gains the start's ask and `release_after`
  as the agent-process world's.
Test shape: in `tests/agent-process/tests/live.rs`, reference:
  ```rust
  #[test]
  fn a_retry_sends_its_pieces_again_from_offset_zero() {
      let mut world = World::new(SEED, &charter());
      world.ask(true, false, false);
      world.script(held_after(b"Hel", b"h", b"lo"));
      world.release_after(b"h", 2, Duration::from_secs(1));
      assert!(world.settle());
      let pieces: Vec<_> = world.pieces().iter().map(|piece| (piece.retry, piece.offset, &*piece.bytes)).collect();
      assert_eq!(pieces, [(0, 0, &b"Hel"[..]), (1, 0, b"Hel"), (1, 3, b"lo")]);
      let kinds: Vec<u16> = world.observed().iter().map(|frame| frame.kind).collect();
      let last_piece = kinds.iter().rposition(|&kind| kind == 0x0112);
      assert!(last_piece < kinds.iter().position(|&kind| kind == 0x0109), "no piece follows its turn");
  }
  ```
  (try 0 waits at the hold past the world's 3 s idle deadline, times
  out, and is retried; the world releases try 1. `held_after` builds the
  one turn of `Line::Pieces`; `ask` and `pieces` are 01.3's.) Beside it:
  `a_channel_with_no_room_drops_the_newest_and_leaves_a_gap`
  (`observe_facts` with one frame, pieces past 16 KiB; `channel_loss`
  and `lost_live_bytes` agree, the next piece's offset past the gap,
  the turn's record still sent, `check_memory`). In `tests/hosts`:
  `a_host_asking_for_text_hears_the_reply_in_pieces_before_its_turn`
  (OpenAI, the world's default dialect, now writes a reply in pieces as
  deltas). Each under 0.5 s.
Proof: joint, `a_host_asking_for_text_hears_the_reply_in_pieces_before_
  its_turn`: the real host `Process` asks for text in its start; the
  real agent service, its LLM component and the fake LLM in one
  simulated process give the owner the reply's pieces, joined equal to
  the script's text, before the turn reaches its domain; under 0.5 s.
Don't: wait in a test for real time; script a failure after a piece
  (decisions.md 5.2, the accounts' plan).
Size: none planned: the worlds and stories are test code.
Removes: nothing.

## Interface sketches

```rust
// crates/smith-agent-service/src/profile/quantities.rs
pub struct Policy {
    // .. as now.
    /// The longest the agent holds live bytes before sending them as a piece (limits.md 2.3).
    pub live_interval: Duration,
}

// crates/smith-agent-service/src/live.rs
impl Live {
    /// When the held bytes must go, whether or not they reach the bound.
    pub(crate) fn deadline(&self) -> Option<Time>;
}
```

The policy owns the interval; the live module owns the time of its
oldest unsent byte; the service owns the wake-up.

## Questions to raise when reached

- If try 0's call stays held in the fake after the client gives it up,
  and the release of try 1 also frees it, 02.2's retry story reports
  what the host saw; a piece of retry 0 after one of retry 1 is refused
  by the host (`Rule::Retry`), which would be skein's to fix.

## Done when

```
git -C ~/src/rust/smith grep -c 'live_interval' origin/main -- crates/smith-agent-service/src/profile/quantities.rs crates/smith-agent-service/src/live.rs; git -C ~/src/rust/smith grep -l 'Line::Pieces' origin/main -- tests/agent-process tests/hosts
```

after `git -C ~/src/rust/smith fetch`, prints a count for each of the
two production files, and a file in each of the two worlds that
scripts a reply in pieces.
