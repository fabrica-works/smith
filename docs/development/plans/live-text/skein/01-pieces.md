# Session 01: a reply in pieces, and its holds

2026-10-19. skein's fake LLM answers a call with one `Reply` after one
latency, and its server writes the reply's text in one delta. This
session lets a script give a reply's text in pieces, each a delta of its
own, and hold the reply before a piece until the world or the real loop
releases it by name (fake-llm.md 2.2). The domain learns the pieces and
the holds (01.1), the server writes them in both dialects (01.2), and
the byte peer takes a release from whoever hosts it (01.3). It changes
skein only; smith takes it at its repin (smith 02.1).

## Read first

- skein `docs/design/fake-llm.md`, sections 2.2, 3, 5 and 6.
- `decisions.md`, section 1, and "For the user" 1 to 3;
  `findings/planning-audit.md` A1 to A10.
- skein `programming-model.md`, sections 3, 4.2, 4.4, 5.4, 6.3, 7 and 9;
  `testing-strategy.md`, sections 2.1, 2.5, 2.7, 4.1 and 10.

## What the design asks

- **2.2, pieces:** a script may give a reply's text in pieces, each sent
  as a delta of its own: what a live client sees partway.
- **2.2, holds:** a hold, named in the script before a piece, keeps the
  reply waiting there, with the dialect's keep-alive, until released by
  name; a release sends the held piece and those after it up to the next
  hold; a release of a name nothing holds changes nothing; a fault ends
  a held reply as it ends any other.
- **5, limits:** the domain's and the peer's worst cases stay fixed sums
  of their configuration.
- **6, hosts:** the byte peer is hosted by a world, and by the real loop
  on loopback; both drive the release (answer 2).

## Choices this plan makes

- One `Line::Pieces`, the turn's first line; `Line::Text` stays one
  piece (decisions.md 1.1).
- The call's stream is named by `call: Token` on `Event::Call`, echoed
  by `Request::Piece` and `Request::Hold`; `Request::Reply` stays the
  terminal with the whole answer (1.2).
- One request a fire, each piece at its own latency, no timer while
  held, one keep-alive at each hold, no `Config` field (1.3).
- A faulted, cut, refused or unscripted reply goes whole (1.4).
- A release frees every call held at its name; one that comes early
  changes nothing (1.5).
- A call keeps its place in the script, not a copy (1.6).

## Rules that bind it

- **Users outside Files** (README, rule 6): `Event::Call`'s new field
  and the new variants reach every match in skein; adapt each.
- **Dependents' tests** (implementer.md 8.1, step 5): the fake crates'
  dependents are `tests/llm` and `tests/llm-connection`.
- **One behaviour per test** (testing-strategy.md, section 10).

## Increments

### 01.1 The domain plays a reply in pieces, held and released
Scope: fake-llm.md 2.2's pieces and holds, in the domain; 5's bound
  (decisions.md 1.1 to 1.6; A1 to A3).
Needs: nothing.
Files: `testing/skein-fake-llm-domain` (`src/api.rs`, `src/domain.rs`,
  a new `src/pieces.rs`, `src/respond.rs`, `src/limits.rs`, `src/lib.rs`,
  a new `src/tests.rs`); `testing/skein-fake-llm-protocol/src/provider.rs`
  (`Event::Call`'s `call`, `Service::target`, `down`);
  `testing/skein-fake-peers/src/llm.rs` (its iterate's refutable `let`);
  `tests/llm/src/fake.rs` (`Exchange::release`), `tests/llm/tests/fake.rs`
  and `fake_memory.rs` (each builds `Event::Call` or takes `Request::
  Reply` irrefutably). `tests/llm-connection/src/world.rs` passes the
  domain's requests to `provider::down` untouched and needs nothing.
Shape: `api::Line::Pieces { pieces }` and `api::Piece { text, hold }`
  (sketch). `Domain::configured` refuses with `Error::InvalidRequest` a
  script whose `Pieces` is not its turn's first line, or holds no piece,
  an empty text, or a hold's name outside 1 to 64 bytes. `respond`'s
  scripted play gives `Pieces` as one `Part::Text`, the pieces joined,
  and `respond` returns, beside the answer, the place (`pieces::Place {
  script, turn }`) when it played a turn whose first line is `Pieces`
  and did not cut it; a random failure, `refused`, `no_calls` and a cut
  return none. `domain::Event::Call` gains `call: Token`;
  `Event::Release { name }`; `Request::Piece { call, usage, text }`,
  `Request::Hold { call, usage }`. `src/pieces.rs` holds the walk:
  `State::Streaming { reply_to, call, answer, place, next, passed }` with
  a timer, `State::Held { .. }` the same with none, and the domain's
  `held: Set<Id<Call>>` of capacity `Config::calls`. `fire` on a
  streaming call: piece `next` held and not `passed` sends `Hold` and
  goes `Held`; the last piece sends the `Reply`, whole, and closes; any
  other sends `Piece` and arms the next piece at a fresh latency.
  `Release` moves each held call whose piece `next` names it to
  `Streaming` with `passed`, armed at `now` plus a latency; it sends
  nothing, so `MAX_OUT` stays 1. A call whose answer has no place goes
  `Thinking` as now. `limits::scripts` and `scripted_answer` count the
  pieces' bytes; `limits::worst_case` adds `Set::<Id<Call>>::worst_case(
  calls)` and fixed fields only. In `provider.rs`, `Event::Call` takes
  `call: id.token()`; `Service::target` routes `Piece` and `Hold` by
  `call` as it routes `Reply` by `to`; `down` takes `Piece` and `Hold`
  and changes nothing (transitional: skein 01.2 removes it). `Exchange::
  release(&mut self, name: &[u8])` steps its domain with `Release` at
  once.
Test shape: `testing/skein-fake-llm-domain/src/tests.rs`, a new file
  and kind (through `step` and `fire`); respond.rs' test helpers `CONFIG`,
  `user`, `query`, `script` and `says` move into it as `pub(super)`
  items, and respond's tests import them. Its limits: `CONFIG` with
  `latency_min` and `latency_max` one second, so each fire is at a known
  time (the profile's `tests/llm/src/fake.rs` `config()` has zero
  latency, which hides the order). Reference test:
  ```rust
  #[test]
  fn a_held_piece_waits_with_no_timer_until_released() {
      let config = Config { latency_min: Duration::from_secs(1), latency_max: Duration::from_secs(1), ..CONFIG };
      let pieces = Box::new([
          Piece { text: copy_of(b"Hel"), hold: None },
          Piece { text: copy_of(b"lo"), hold: Some(copy_of(b"h")) },
      ]);
      let turn = Turn { lines: Box::new([Line::Pieces { pieces }]), finish: Finish::Stop, tokens: 2 };
      let mut domain = Domain::scripted(&config, 1, Box::new([script(b"@main", Box::new([turn]))]));
      let mut first = query(Box::new([user(Box::new([Part::Text { text: copy_of(b"hi") }]))]));
      first.system = copy_of(b"@main");
      let mut env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits: config };
      let mut out = Queue::with_capacity(MAX_OUT);
      let call = Token::new(7);
      step(&mut domain, &env, Event::Call { reply_to: ReplyTo::new(call), call, query: first }, &mut out);
      env.now = Time::from_nanos(1_000_000_000);
      fire(&mut domain, &env, &mut out);
      assert!(matches!(out.pop(), Some(Request::Piece { text, .. }) if &*text == b"Hel"));
      env.now = Time::from_nanos(2_000_000_000);
      fire(&mut domain, &env, &mut out);
      assert!(matches!(out.pop(), Some(Request::Hold { call: held, .. }) if held == call));
      assert_eq!(domain.next_deadline(), None, "a held call has no timer");
      step(&mut domain, &env, Event::Release { name: copy_of(b"h") }, &mut out);
      assert!(out.is_empty(), "a release sends nothing");
      env.now = Time::from_nanos(3_000_000_000);
      fire(&mut domain, &env, &mut out);
      let Some(Request::Reply { result: Ok(answer), .. }) = out.pop() else { panic!("the reply ends the call") };
      assert_eq!(&*answer.parts, &[Part::Text { text: copy_of(b"Hello") }]);
  }
  ```
  Beside it: `each_piece_is_sent_at_its_own_latency`,
  `a_release_before_its_hold_changes_nothing`,
  `one_release_frees_every_call_held_at_its_name`,
  `a_faulted_reply_in_pieces_goes_whole_in_one_reply`
  (`overloaded: 1000`), `a_reply_cut_by_max_tokens_goes_whole`, and
  `a_script_breaking_the_pieces_rule_is_refused` (one table: a `Pieces`
  second, no piece, an empty text, a 65-byte hold; each refused). Under
  20 ms each. In `tests/llm/tests/fake.rs`:
  `a_held_reply_reaches_the_client_only_after_its_release` (both
  dialects, rows of one table: ticks until `tick` is false, no
  `Completed` seen; `release`; `run`; one `Completed` with the whole
  text), under 0.1 s.
Proof: joint, `a_held_reply_reaches_the_client_only_after_its_release`:
  the real `skein-llm` client against the server and the domain holds
  its completion until the release, then gets the whole text; under
  0.1 s. The pieces as deltas are 01.2's.
Don't: add a `Config` field; copy a turn's pieces into a call; send a
  piece of a faulted or cut reply.
Size: 220 lines.
Removes: nothing.

### 01.2 The server writes each piece as a delta, and a keep-alive at a hold
Scope: fake-llm.md 2.2's "each sent as a delta of its own" and "with
  the dialect's keep-alive" (decisions.md 1.2, 1.3, 1.7; A5, A6).
Needs: the domain's pieces, holds and releases [01.1].
Files: `testing/skein-fake-llm-protocol` (a new `src/pieces.rs`,
  `src/provider.rs`, `src/documents.rs`, `src/lib.rs`); `tests/llm/tests/
  fake.rs`. `provider.rs` is over 800 lines: its growth stays under 100
  (code-organization.md 8), the writing goes in `pieces.rs`.
Shape: the server opens the stream at a call's first `Piece` or `Hold`
  (`State::Waiting`): the 200 head, then the dialect's opening from the
  request's `usage` (Anthropic `message_start` and the text block's
  `content_block_start`; OpenAI `response.created` and the text item's
  `output_item.added`), then that piece's delta (Anthropic
  `content_block_delta`, OpenAI `response.output_text.delta`) or, at a
  `Hold`, Anthropic's `ping` (OpenAI's Responses writes nothing). The
  call stays active and the server in a new `State::Pieces { written }`,
  `written` the text bytes sent. A piece that comes while an event is in
  the writer waits in the server's pending pieces, at most the
  documents' answer bound in all, counted in `provider::worst_case`. At
  the `Reply`, the server writes the rest of the first part as one delta,
  then the answer's events from the first part's end on, as
  `documents::event_with_options` writes them (Anthropic from the text
  block's stop, OpenAI from its `output_item.done` with the whole text).
  A `Reply` with an error after pieces cannot happen in this plan
  (decisions.md 1.4); `down` writes the error's event if it comes.
  `down`'s transitional arm goes.
Test shape: `tests/llm/tests/fake.rs`, beside 01.1's test, with
  `Exchange` and `limits`/`config` from `tests/llm/src/fake.rs`; one
  table over both dialects each:
  `pieces_reach_the_client_as_deltas_before_the_completion` (three
  pieces: three `Delta::Text`, in order, before `Completed`, joined ==
  the completion's text);
  `a_held_reply_shows_only_the_pieces_before_its_hold` (two deltas seen
  before the release, the rest after);
  `the_lines_after_the_pieces_follow_them_whole` (pieces then a
  `Line::Call`: the tool call complete in the completion);
  `the_stream_opened_by_a_piece_reports_the_replys_usage`
  (Anthropic only: input usage at `message_start` equals the
  completion's). In `pieces.rs`' tests: `a_hold_writes_one_ping_in_
  anthropic_and_nothing_in_openai` (the bytes as a literal). Under
  0.1 s each.
Proof: joint, `pieces_reach_the_client_as_deltas_before_the_completion`:
  the real client parses each piece as its own `Delta::Text`, in order,
  before the completion, in both dialects; under 0.1 s.
Don't: change `documents::event`'s output for a reply without pieces
  (answer 5; the 2,859 tests at the base keep their bytes); write a
  keep-alive on a timer.
Size: 200 lines.
Removes: `down`'s transitional arm for `Piece` and `Hold`.

### 01.3 The byte peer takes a release from its host
Scope: fake-llm.md 2.2 and 6, as answer 2 reads them: the world or the
  real loop that hosts the peer releases a hold by name (decisions.md
  1.8; A7, A9).
Needs: the server's deltas and keep-alive [01.2].
Files: `testing/skein-fake-peers/src/llm.rs`, `testing/skein-fake-peers/
  tests/llm.rs`; `tests/llm-connection/src/simulated.rs`,
  `tests/llm-connection/tests/simulated.rs`.
Shape: `Peer::release(&mut self, name: &[u8]) -> Result<(), Error>`
  (sketch) copies the name into a queue of `Config::calls` names; a name
  of 0 or past 64 bytes, or a full queue, is `Error::Limits` and queues
  nothing. `iterate` steps the domain with `Event::Release` for each
  queued name before it fires the domain. `work_pending` is true while
  the queue holds a name; `worst_case` adds the queue and `calls` times
  64 bytes. The simulated world's `Process::Peer` takes an optional
  release, a name and a time: its `iterate` calls `Peer::release` once
  `now` reaches it, and its `next_deadline` includes it.
Test shape: `testing/skein-fake-peers/tests/llm.rs`, with its `limits`
  and `make`: `a_release_name_past_64_bytes_is_refused_as_limits`,
  `a_release_past_the_queue_is_refused_as_limits`,
  `a_queued_release_keeps_the_peer_working` (`work_pending` true, then
  false after one iteration). In `tests/llm-connection/tests/
  simulated.rs`, after the file's own stories:
  `a_held_reply_released_over_loopback_completes_the_call` (one piece,
  a hold, one piece; the release at 50 ms; `Memory::Checked`: the
  client's first delta before 50 ms, its completion after, both heaps
  within their bounds; replayed by seed with `assert_replay`), under
  0.3 s. The peer tests under 20 ms each.
Proof: joint, `a_held_reply_released_over_loopback_completes_the_call`:
  the real client over simulated loopback sees the first piece, waits at
  the hold, and completes after `Peer::release`, the peer's heap within
  its `worst_case`; under 0.3 s. It is the call temper's real loop will
  make (A9).
Don't: refuse a script with a hold in `Peer::new` (answer 2); change
  `Peer::new`'s signature.
Size: 90 lines.
Removes: nothing.

## Interface sketches

```rust
// testing/skein-fake-llm-domain/src/api.rs
pub enum Line {
    // .. Opaque, Text and Call as now.
    /// The reply's text in pieces, each sent as its own delta: the turn's first line.
    Pieces { pieces: Box<[Piece]> },
}
/// One piece of a reply's text; a hold keeps the reply waiting before it.
pub struct Piece { pub text: Box<[u8]>, pub hold: Option<Box<[u8]>> }

// testing/skein-fake-llm-domain/src/domain.rs
pub enum Event {
    /// A call: answer `query`; `call` names its stream on `Piece` and `Hold`.
    Call { reply_to: ReplyTo, call: Token, query: Query },
    /// The world's release: every call held at `name` goes on.
    Release { name: Box<[u8]> },
}
pub enum Request {
    /// The answer to a `Call`: exactly one per call, its terminal.
    Reply { to: ReplyTo, result: Result<Answer, Error> },
    /// One piece of the call's text, before its `Reply`.
    Piece { call: Token, usage: Usage, text: Box<[u8]> },
    /// The call waits at a hold until a `Release` names it.
    Hold { call: Token, usage: Usage },
}

// testing/skein-fake-peers/src/llm.rs
impl Peer {
    /// Queues a release for the peer's domain, fed at its next iteration.
    pub fn release(&mut self, name: &[u8]) -> Result<(), Error>;
}

// tests/llm/src/fake.rs
impl Exchange {
    /// Steps the domain with a release at once.
    pub fn release(&mut self, name: &[u8]);
}
```

The domain owns a call's place and its holds; the server owns what it
has written of the stream; the host owns the release queue. A call ends
at its `Reply`, whatever its pieces.

## Questions to raise when reached

None.

## Done when

```
s=~/src/rust/skein; d=testing/skein-fake-llm-domain/src; git -C $s show origin/main:$d/api.rs | grep -q 'Pieces {' || echo 'no Line::Pieces'; git -C $s show origin/main:$d/domain.rs | grep -q 'Release {' || echo 'no Event::Release'; git -C $s show origin/main:$d/domain.rs | grep -q 'Hold {' || echo 'no Request::Hold'; git -C $s cat-file -e origin/main:testing/skein-fake-llm-protocol/src/pieces.rs 2>/dev/null || echo 'no pieces.rs'; git -C $s show origin/main:testing/skein-fake-peers/src/llm.rs | grep -q 'pub fn release(' || echo 'no Peer::release'
```

after `git -C ~/src/rust/skein fetch`, prints nothing.
