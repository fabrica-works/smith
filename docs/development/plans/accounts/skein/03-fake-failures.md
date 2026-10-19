# skein 03: the fake LLM's failures

A script may fail a turn: a failing line gives a provider failure's
class, its code, request id and field, and the credentials whose calls
it fails. The byte peer says each failure as its route does, before a
stream or after the pieces sent, so that the client's own
classification reads back the class the script gave. Every response
head carries its route's request id. The fake never sends
`retry-after` for `Exhausted`, on any route (decision 21): the reset
goes in `resets_in_seconds`, or in Anthropic's two unified headers.

Needs at start: live text's three fake LLM steps on skein's
`origin/main` (decision 17).

## Read first

- skein's design at `49725eca`: `fake-llm.md` 1, 2, 2.2, 2.3, 3, 3.1, 5,
  7; `llm.md` 2.5, 4.2, 4.6, 4.7, 4.8. skein `docs/foundation/`:
  `programming-model.md` 3, 4.4, 6.3, 8, 9; `testing-strategy.md` 2.1,
  2.5, 4.1, 6, 8, 10.
- The README's Rules; code-organization.md (owners, 5's fixtures, 7's
  Don't, 8's sizes); decisions.md 2, 9, 10, 13, 17, 21 and design
  questions 10 and 11.

## What the design asks

- **fake-llm.md 2.2:** a failure after other lines ends the stream as
  its route fails one, a hold's released first.
- **2.3:** the failing line, whose calls it fails, where it falls, a
  class's row per route, the evidence as given, the seeded failure.
- **3.1:** a request id on each response head, under its route's name.
- **7:** the reset in `resets_in_seconds`, or Anthropic's two headers.
- **llm.md 2.5, 4.6 to 4.8:** the classes; each route's status, code
  and field as `error.param`.
- **decision 21:** no `retry-after` for `Exhausted`.

## Choices

- The domain keeps its own vocabulary: it depends on `skein-lib` alone
  (fake-llm.md, 1), so its `Report` is the script's evidence, read back
  through the client as skein 01's `Report`.
- The withdrawn routes are rendered as they are: the Codex route and
  Anthropic's bearer route (decision 2). The public route's codes are
  llm.md 4.8's, as stated (decision 9); no live suite (decision 10).
- The downstream lines each increment breaks are named in its Shape and
  repaired by the closing smith-repin (decision 13).
- Design questions 10 and 11, as recommended: only a line naming no
  credential must be its turn's last; resets as the bullets above say,
  the public route's field `resets_in_seconds` alone.
- The worlds run on Codex and Anthropic. Until skein 04 the peer
  authorizes an OpenAI-dialect call by account id, which a public
  request lacks, and refuses a scope other than its session's, which a
  public cache key makes: the public route's rows are units here, and
  its worlds are skein 04.4's.
- A turn whose `Pieces` line a reaching `Fail` follows plays whole in
  03.1, as a cut turn does (no place, the pieces joined as one text
  part); 03.3 plays it in pieces and ends the walk with the failure.

## Rules that bind it

- The client's classification is the oracle: a world asserts the
  client's terminal (class, report, evidence) against literal values;
  a unit asserts literal statuses and codes. No expected value comes
  from the renderer.
- The local classes (a limit, a malformed stream, a cancel, a timeout)
  have no `api::Error` variant: a script cannot name them.
- The worlds take their limits from `tests/llm/src/lib.rs` and
  `tests/llm/src/fake.rs`: the client's `skein_llm_world::limits()`, the
  domain's `skein_llm_world::fake::config()` (zero latency) and the
  peer's `skein_llm_world::fake::limits(&limits())`. No test sets a
  bound of its own. Each unit runs under 20 ms, each world under 0.1 s,
  each test fn within code-organization.md 8's aim of 40 lines.
- `provider.rs` (1031 lines) grows by at most ~100: the failure
  rendering moves to `provider/failure.rs` (new). `openai/response.rs`
  (1307) grows by at most ~100. `tests/llm/tests/fake.rs` (1210) takes
  no new story: they go in `tests/llm/tests/fake_failures.rs` (new).

## Increments

### 03.1 The failing line
Scope: fake-llm.md 2.3 (the line, where it falls, whose calls it fails,
  the evidence); llm.md 2.5's classes; design question 10.
Needs: live text's fake LLM steps: the domain's pieces, the server's
  deltas, the byte peer's release (`plan await coordinator
  live-text/skein-01.3`, the last of them).
Files: `testing/skein-fake-llm-domain` (`src/api.rs`, `src/respond.rs`,
  `src/limits.rs`, `src/domain.rs`, `src/cache.rs`, `src/tests.rs`);
  `testing/skein-fake-llm-protocol/src/{provider,documents}.rs`;
  `testing/skein-fake-peers/src/llm.rs`;
  `tests/llm/tests/{fake,fake_memory}.rs`.
Shape: `api::Error` gains `Refused` and `Unsupported`, and `Exhausted`
  becomes `{ retry_after: Option<Duration> }`; it stays `Copy`.
  `api::Report { code, request, field }` (new, bytes; `Report::none()`
  is the empty one, as `Default` is disallowed) is the evidence.
  `Line::Fail { error, report, credentials: Box<[u32]> }` (new arm).
  `api::Failure { error, report, parts }` (new) is a failed reply: its
  class, its evidence, and the parts before the failing line;
  `From<Error>` gives `Report::none()` and no parts, for the seeded
  chances and the domain's own refusals. `Request::Reply.result` is
  `Result<Answer, Failure>`. `Query.credential: Option<u32>` (new) is
  the index the peer matched; `documents::request` leaves it `None`
  (03.2 and skein 04 fill it). `respond` walks the turn's lines: a
  `Fail` naming credentials is passed over by a query whose index is not
  among them, or that has none; any other `Fail` returns the failure
  with the parts before it. A turn whose `Pieces` line a reaching `Fail`
  follows plays whole: the pieces joined as one `Part::Text`, and
  `respond` returns no place. Nothing advances the script on a failure:
  `said()` counts assistant messages, and a failure adds none.
  `Domain::configured` refuses, `Err(Error::InvalidRequest)`, a script
  with a line after a `Fail` naming no credential. `limits::scripts` and
  `limits::scripted_answer` price a `Fail`'s report and indices; a
  failure's bytes meet `answer_bytes` as an answer's do.
  `domain::Domain::scripts(&self) -> &[Script]` (new) lets the peer
  check its scripts (03.2). The protocol and the peer take
  `failure.error` where they took the error; `Observation::Answered`
  keeps `Result<(), api::Error>`. Downstream (smith): `outcome` and
  `failure` of `tests/session/src/translate.rs`, and `failure` of
  `tests/protocol-llm/src/translate.rs` and of
  `tests/agent/src/translate.rs`, gain the two classes and the optional
  reset; the `Err` of a reply becomes `failure.error` in
  `advance_provider` of `tests/agent/src/world.rs` and of
  `tests/protocol-llm/src/world.rs`, `drive_until` of
  `tests/local/src/world.rs`, `provider_replies` of
  `tests/host/src/inline.rs`, and `provider_request` and
  `Delivery::Answer`'s `result` of `tests/session/src/world.rs`; the
  five `Query` literals take `credential: None` (`query` of each of the
  three translate files, `prefix_query` of
  `tests/agent/tests/referee.rs`, the `queries` literal of
  `tests/local-process/tests/referee.rs`). About 25 lines. jig and the
  repository downstream of jig: none. In skein, the `Query` literals of
  `query` in `src/tests.rs`, of the tests of `cache.rs` and `respond.rs`
  and of `query` in `tests/llm/tests/fake_memory.rs`, and the
  `Request::Reply` replies matched in `tests/llm/tests/fake.rs` and
  `fake_memory.rs`, follow.
Test shape: units in `respond.rs`'s tests, through `respond`, with
  `CONFIG`, `user`, `query`, `script` and `says` imported from live
  text's `src/tests.rs`; each under 20 ms:
  `a_failing_line_answers_its_class_and_report`, rows `Exhausted` with a
  reset and without, `Refused`, `Unsupported` with a field,
  `Unauthorized`; `a_failure_leaves_the_turn_for_the_call_made_again`;
  `a_failing_line_naming_credentials_fails_only_their_calls`, rows
  (naming `[1]`, query 1: fails), (`[1]`, query 0: the lines after),
  (`[1]`, no credential: the lines after), (`[]`, query 0: fails),
  (`[]`, none: fails);
  `a_failing_line_after_text_keeps_the_text_before_it`, rows after a
  text line and after `Pieces` (played whole, no place). In
  `src/tests.rs`, through `Domain::try_scripted`:
  `a_line_after_a_failing_line_naming_none_is_refused`;
  `a_failing_lines_bytes_count_against_the_script_cap` (at the cap
  admitted, one over refused). The 301 `#[test]` fns of
  `testing/skein-fake-llm-protocol`, `testing/skein-fake-peers`,
  `tests/llm`, `tests/llm-connection` and `tests/tls` (`rg -c
  '#\[test\]'` at the base, before live text's) keep their assertions;
  only their `Query` literals and matched replies change.
Proof: unit, `a_failing_line_answers_its_class_and_report`: a call's
  reply is the scripted class with its code, request id and field;
  under 20 ms. Unit, because no byte peer renders a failure until
  03.2; the 301 dependents' tests keep their assertions (Test shape).
Don't: no route, status or wire name in the domain; no dependency of the
  domain on `skein-llm`; no failure that advances the script; no `Clone`
  dropped from `Line` or `Copy` from `Error`.
Size: about 280 lines.
Removes: `Exhausted`'s required reset.

### 03.2 Each route's failure before a stream
Scope: fake-llm.md 2.3 (as the route says it, a reset per route, a
  class needs its row, the evidence, the chances), 3.1's request id, 7;
  llm.md 4.6 to 4.8; design question 11.
Needs: 03.1's `Line::Fail`; skein 01's failure classes and `Report`
  [01.3], and its report values (the request id under each route's
  name, the error's `param` read as the field) [01.4]; skein 02's
  code-first rule, public request and code table [02.2 to 02.4].
Files: `testing/skein-fake-llm-protocol/src/provider.rs`,
  `src/provider/failure.rs` (new);
  `crates/skein-llm/src/openai/response.rs` (`write_error`);
  `testing/skein-fake-peers/src/llm.rs`; `tests/llm/src/fake.rs`;
  `tests/llm/tests/fake_failures.rs` (new).
Shape: `response_error` moves to `provider/failure.rs`, with one table
  per route: `pub(super) struct Row { status: u16, code: &'static [u8]
  }` and `pub(super) fn row(route: skein_llm::Provider, error:
  api::Error) -> Option<Row>`. The route is the peer's dialect and its
  one credential (`failure::route_of(config, credential)`): Anthropic,
  else Codex where the credential has an account id, else the public
  route; skein 04.1's `provider::route` replaces it. The defaults,
  status and code, a scripted code replacing the code. Public and Codex:
  `Exhausted` 429 `subscription_sharing_usage_limit_exceeded` (Codex
  `usage_limit_reached`); `RateLimited` 429 `rate_limit_exceeded`;
  `Refused` 403 `subscription_sharing_user_not_eligible`; `Unsupported`
  400 `subscription_sharing_unsupported_capability`; `Overloaded` 503
  `overloaded_error`; `Unavailable` 503
  `subscription_sharing_usage_unavailable`; `ContextTooLong` 400
  `context_length_exceeded`; `InvalidRequest` 403
  `subscription_sharing_route_not_supported`; `Unauthorized` 401
  `invalid_api_key`. Anthropic, by type: `Exhausted` with a reset 429
  `rate_limit_error`, without 402 `billing_error`; `RateLimited` 429
  `rate_limit_error`; `Refused` 403 `permission_error`; `Overloaded` 529
  `overloaded_error`; `Unavailable` 500 `api_error`; `ContextTooLong`
  400 `invalid_request_error`, "prompt is too long"; `InvalidRequest`
  400 `invalid_request_error`; `Unauthorized` 401
  `authentication_error`; `Unsupported` no row. The message is the code,
  as today, but for Anthropic's context too long. The report's field
  goes in `error.param`: skein-llm's `write_error` writes `param` where
  `ProviderError` holds one. `Exhausted`'s reset on the public and Codex
  routes is `resets_in_seconds`, rounded up. The fake never sends
  `retry-after` for `Exhausted`, on any route; `RateLimited` keeps its.
  On Anthropic the head gives `anthropic-ratelimit-unified-status:
  rejected` and `anthropic-ratelimit-unified-reset`, `env.wall` in whole
  seconds plus the reset rounded up. Each response head, success or
  failure, carries `req_<n>` (new), `n` the call's response id, under
  `x-request-id`, `x-oai-request-id` or `request-id`, or the failing
  line's id where not empty; the head's worst case prices it.
  `provider::scripts(config, credential, scripts) -> Result<(), Error>`
  (new, defined in `failure.rs`, re-exported by `provider.rs`) refuses,
  `Error::Limits`: a class with no row (`Unsupported` on Anthropic); an
  Anthropic reset after a line of its turn that is not a failing line,
  naming no credential, or naming a key's index. `Peer::new` calls it on
  `Domain::scripts()`, and the Exchange on its scripts (`expect`, as its
  constructors do). `entrance` returns `Result<(), api::Error>`, its
  `Err` the refusal it sets in `server.refused` today, which
  `http_event` keeps; after `entrance` returns `Ok` the server records
  `credential = Some(0)` and `requested` puts it in the `Query`.
  Downstream: none beyond 03.1's.
Test shape: units in `failure.rs`'s tests, each under 20 ms:
  `each_class_renders_its_routes_status_and_code`, a row per class and
  route, public included, literal bytes (reference test below);
  `a_class_without_its_row_is_refused`;
  `an_anthropic_reset_is_refused_off_a_first_bearer_line`, rows after a
  text line, naming none, naming a key, first on a bearer (admitted).
  Worlds, `fake_failures.rs`, on Codex and Anthropic, each under 0.1 s
  (its `input` is `tests/llm/tests/fake.rs`'s, with 02.1's public arm):
  `each_scripted_class_reaches_the_client_as_scripted`;
  `scripted_evidence_reaches_the_report_or_arrives_empty`, rows within
  bound, past the bound, an empty id (`req_1`), a field (reference
  test below); `a_reset_reaches_exhausted_in_whole_seconds_rounded_up`,
  rows Codex and Anthropic (`Credential::anthropic`) at a set `wall`;
  `exhausted_without_a_reset_reaches_the_client_without_one`;
  `every_response_head_carries_its_routes_request_id`, rows route by
  success and failure; `a_seeded_failure_carries_its_classs_code`;
  `a_line_naming_another_index_lets_the_call_through`, an `Exhausted`
  with a reset naming index 1, then a text line: the call, matched at
  index 0, reads the text.
  Reference tests (the other evidence rows vary `report` and the pair):
  ```rust
  // provider/failure.rs, tests.
  use skein_llm::Provider::{Anthropic, OpenAi, OpenAiCodex};
  #[test]
  fn each_class_renders_its_routes_status_and_code() {
      let none = api::Error::Exhausted { retry_after: None };
      let rows: [(_, _, u16, &[u8]); 4] = [
          (OpenAi, none, 429, b"subscription_sharing_usage_limit_exceeded"),
          (OpenAiCodex, none, 429, b"usage_limit_reached"),
          (Anthropic, none, 402, b"billing_error"),
          (Anthropic, api::Error::Overloaded, 529, b"overloaded_error"),
      ];
      for (route, error, status, code) in rows {
          let got = row(route, error).expect("the class has a row");
          assert_eq!((got.status, got.code), (status, code), "{route:?} {error:?}");
      }
      assert!(row(Anthropic, api::Error::Unsupported).is_none());
  }
  ```
  ```rust
  // fake_failures.rs: `input`, `limits`, `Exchange`, `api` as fake.rs.
  #[test]
  fn scripted_evidence_reaches_the_report_or_arrives_empty() {
      let report = api::Report { code: Box::default(), request: b"req_42".as_slice().into(), field: b"top_p".as_slice().into() };
      let fail = Line::Fail { error: api::Error::Unsupported, report, credentials: Box::new([]) };
      let turn = Turn { lines: Box::new([fail]), finish: Finish::Stop, tokens: 1 };
      let script = Script { cue: b"caller-script".as_slice().into(), turns: Box::new([turn]) };
      let mut world = Exchange::new(input(skein_llm::Provider::OpenAiCodex, 1), limits(), Box::new([script]));
      world.start();
      world.run();
      let Some((failure, got)) = world.seen.iter().find_map(|event| match event {
          client::Event::Failed { failure, report, .. } => Some((failure, report)),
          _ => None,
      }) else { panic!("{:?}", world.seen) };
      assert_eq!(*failure, skein_llm::Failure::Unsupported);
      assert_eq!((&*got.request, &*got.field), (b"req_42".as_slice(), b"top_p".as_slice()));
  }
  ```
Proof: joint, `each_scripted_class_reaches_the_client_as_scripted`: the
  real client reads each class, code, request id and field back on
  Codex and Anthropic; under 0.1 s.
Don't: no `retry-after` for `Exhausted`; no status read in place of a
  route's row; no wall time from outside `env`; no client-side clipping
  of the script's evidence.
Size: about 300 lines. If it passes that, it splits. 03.2a is the
  route tables, `provider::scripts` with its two callers, `entrance`'s
  `Result` and `Some(0)`, and the units, with
  `a_line_naming_another_index_lets_the_call_through`. 03.2b is what
  the client reads: `write_error`'s `param`, the reset's rendering, the
  request id on every head and the seeded code, with the other worlds
  and the Proof, judged there; 03.2a's own is its first unit.
Removes: today's single code per class (`server_error`,
  `authentication_error` and `invalid_request_error` on Responses), and
  the `retry-after` header on `Exhausted`.

### 03.3 A failure after other lines
Scope: fake-llm.md 2.2 and 2.3 (where it falls), 3.1.
Needs: 03.2's route tables; skein 02's public route's `response.failed`
  and `error` events [02.3, 02.4].
Files: `testing/skein-fake-llm-domain/src/{respond,pieces,tests}.rs`;
  `testing/skein-fake-llm-protocol/src/{provider,documents}.rs`,
  `src/provider/failure.rs`; `crates/skein-llm/src/openai.rs`,
  `src/openai/{response,tests}.rs`; `tests/llm/tests/fake_failures.rs`.
Shape: the domain: `respond` returns the place for a `Pieces` turn that
  a reaching `Fail` follows, as it does for one that ends; the walk of
  `src/pieces.rs` holds its reply as `Result<Answer, Failure>`, and its
  last piece sends `Reply { result: Err(failure) }`, the failure's parts
  being the pieces joined. The server: when the stream is open
  (`State::Pieces`), `down` takes that `Reply`, writes the rest of the
  first part, the remaining parts, a hold's when released, then, in
  place of the completion, the route's failure; when it is not, it opens
  the stream as an answer does (the 200 head, its request id). The
  route's failure: on Responses `response.failed`, its error under
  `response` with the code, message, reset and `param` of 03.2's table;
  on Anthropic the `error` event of `anthropic::encode_event`.
  `openai::encode_failed(error, limits)` (new, re-exported by
  `openai.rs` as `encode_error` is) writes `response.failed`, as
  `decode_event`'s `b"response.failed"` arm reads it; `encode_error`
  keeps the error body before a stream. An `Exhausted` with a reset
  after text goes out on Codex as `resets_in_seconds` in
  `response.failed`, rounded up; on Anthropic `provider::scripts`
  refuses it (03.2). Downstream: none.
Test shape: worlds, `fake_failures.rs`, each under 0.1 s:
  `a_failure_after_text_ends_the_stream_as_its_route_fails`, rows
  Codex and Anthropic (the client's deltas, then its terminal with the
  class and report); `a_failure_after_a_hold_waits_for_its_release`
  (a `Pieces` turn with a hold, then a `Fail`: nothing of the failure
  before the release);
  `an_exhausted_reset_after_text_reaches_the_client_on_the_codex_route`
  (script `[Text, Fail(Exhausted, reset 1.5 s)]`: the text's delta,
  then the terminal `Exhausted { Some(2 s) }`, no `retry-after`).
  Units, each under 20 ms, in `src/tests.rs`:
  `a_failing_line_after_pieces_ends_the_walk_with_its_failure`; in
  `src/openai/tests.rs`: `a_failed_response_reads_back_as_written`
  (`encode_failed`, then `decode_event`, through its `value` and
  `LIMITS`). Anthropic's refusal is the "after a text line" row of
  03.2's reset unit. The public route's rows are skein 04.4's.
Proof: joint, `a_failure_after_text_ends_the_stream_as_its_route_fails`:
  the real client reads the text deltas, then its terminal with the
  scripted class and report; under 0.1 s.
Don't: no failure event before the pieces before it; no `retry-after`
  for `Exhausted`; no second writer of the Responses error object.
Size: about 240 lines.
Removes: nothing.

## Interface sketches

```rust
// testing/skein-fake-llm-domain/src/api.rs
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Error { // two classes new, `Exhausted`'s reset optional
    Overloaded, RateLimited { retry_after: Duration }, Unavailable, ContextTooLong,
    Unauthorized, Exhausted { retry_after: Option<Duration> }, InvalidRequest,
    Refused, Unsupported,
}
/// The evidence a failing line gives: code, request id and field, as bytes.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Report { pub code: Box<[u8]>, pub request: Box<[u8]>, pub field: Box<[u8]> }
impl Report { #[must_use] pub fn none() -> Report { todo!() } } // `Default` is disallowed
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Line { // Opaque, Text, Call and Pieces as they are, and
    Fail { error: Error, report: Report, credentials: Box<[u32]> },
}
/// A failed reply: its class, its evidence, the parts before it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)] pub struct Failure { pub error: Error, pub report: Report, pub parts: Box<[Part]> }
impl From<Error> for Failure { fn from(error: Error) -> Failure { todo!() } }
// Query gains: pub credential: Option<u32>,

// testing/skein-fake-llm-domain/src/domain.rs
// Request::Reply { to: ReplyTo, result: Result<Answer, Failure> }
impl Domain { #[must_use] pub fn scripts(&self) -> &[Script] { todo!() } }

// testing/skein-fake-llm-protocol/src/provider/failure.rs
pub(super) fn row(route: skein_llm::Provider, error: api::Error) -> Option<Row> { todo!() }
/// Refuses at configuration a failing line its route cannot say
/// (`provider.rs` re-exports it as `provider::scripts`).
pub fn scripts(config: &Config, credential: &skein_llm::Credential, scripts: &[api::Script]) -> Result<(), Error> {
    todo!()
}

// crates/skein-llm/src/openai/response.rs, re-exported by openai.rs
/// Writes `response.failed`, its error under `response`.
pub fn encode_failed(error: &ProviderError, limits: &Limits) -> Result<Box<[u8]>, DecodeError> { todo!() }
```

## Done when

- `rg -c '^\s*Fail \{' testing/skein-fake-llm-domain/src/api.rs` prints
  `1`; `rg -n '\bserver_error\b' testing/skein-fake-llm-protocol/src`
  prints nothing.
- `rg -c 'fn encode_failed\b' crates/skein-llm/src/openai/response.rs`
  prints `1`.
- In `testing/skein-fake-llm-protocol/src`, `rg -c 'b"retry-after"'
  provider.rs provider/failure.rs` prints `provider/failure.rs:1` alone.
- `rg -l '\bprovider::scripts\(' testing/skein-fake-peers/src/llm.rs
  tests/llm/src/fake.rs | wc -l` prints `2`.
- The size count of code-organization.md 8 lists no file this session
  grew past its rule.
