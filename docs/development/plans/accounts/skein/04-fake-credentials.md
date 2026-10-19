# skein 04: the fake LLM's credentials

The fake LLM peer holds a list of credentials, each with its scheme. It
authorizes a request as its route sends the credential, takes its route
from the credentials, and observes each request's route, scheme and
index, never a value. The public route keeps its own cache rules and
answers a refused field as unsupported. The Exchange takes the list; its
stories are the ones the accounts' worlds and the real loop rely on.

Needs at start: skein 01.2's `Scheme` and `Credential { value, scheme,
account_id }`, skein 02.1's public route and skein 03.2 on skein's
`origin/main`; live text's fake LLM steps landed before 03.1.

## Read first

- skein's design at `49725eca`: `fake-llm.md` 1, 2.3, 3, 3.1, 5, 6;
  `llm.md` 2.6, 4.6 to 4.8; `programming-model.md` 3, 4.4, 6.3, 8;
  `testing-strategy.md` 2.1, 2.8, 4.1, 6, 8.
- The README's Rules; code-organization.md (owners, 5, 7, 8);
  decisions.md 2, 5, 6, 10, 13, 15, 17; design questions 6, 11, 12.

## What the design asks

- fake-llm.md 3: the credential list, its route, what a request showed.
- fake-llm.md 3.1: the public route's cache key, headers, refusals.
- fake-llm.md 5 and 6: the Exchange's list; the issuer's tokens.
- llm.md 2.6, 4.6 to 4.8: each scheme's field on each route.

## Choices

- The withdrawn routes are served as they are (decision 2); the value is
  `value` (decision 5); refusals at configuration are decision 6's;
  design questions 6, 11 and 12 as recommended; no live suite (decision
  10). Each Shape names the downstream lines it breaks, for the closing
  smith-repin (decision 13).
- What a request presented rides on `provider::Event::Head`, so the
  matches naming `Head { .. }` still compile.
- Live text's server step leaves `entrance`'s head-field reading alone,
  where 04.1 and 04.3 add theirs.

## Rules that bind it

- No credential value in a log, an observation, a `Debug`, an error, a
  panic message or a test's expected output; tests read routes, schemes
  and indices (code-organization.md, 7). Values are compared in
  `provider/credentials.rs` alone. The fake compares bytes: no sign-in,
  refresh, JWT parser, credential generation or credential record.
- `provider.rs` (1095 at skein b481ad9) and `tests/llm/src/fake.rs`
  (967) grow by at most ~100 per increment: rules go to
  `provider/credentials.rs` and `provider/public.rs` (new).
  `tests/llm/tests/fake.rs` (1367) takes
  04.3's one test; the stories go in `fake_credentials.rs`.
- Limits: the peers' worlds take `limits()` of its `tests/llm.rs`
  (observations 16; a row makes at most 6); the Exchange worlds take
  `limits()` and `config()` of `tests/llm/src/fake.rs`.

## Increments

### 04.1 The protocol's credential list
Scope: fake-llm.md 3 (credentials; the route follows them); llm.md 2.6
  and 4.6 to 4.8's credential fields; decision 6; design question 6.
Needs: skein 01.2's `Scheme` and `Credential`; 02.1's `Provider::OpenAi`
  and `Endpoint::openai`; 03.2's route tables and `provider::scripts`.
Files: `testing/skein-fake-llm-protocol/src/provider.rs`,
  `src/provider/credentials.rs` (new), `src/provider/failure.rs`;
  `testing/skein-fake-peers/src/llm.rs`, `tests/llm.rs`;
  `tests/llm/src/fake.rs`, `tests/llm/tests/fake.rs`;
  `tests/llm-connection/src/world.rs`, `src/simulated.rs`,
  `tests/fake_protocol.rs`; `tests/tls/tests/fake_peer.rs`: the entry
  calls and the `provider::Config` literals.
Shape: `start`, `up`, `down` and `resume` take `credentials:
  &[skein_llm::Credential]` where they took one.
  `Config.without_credential: bool` (new) takes calls with no credential
  field. `provider::route(config, credentials) ->
  Result<skein_llm::Provider, Error>` (new, in `credentials.rs` and
  re-exported by `provider.rs`; replaces 03.2's `failure::route_of`)
  names the route: Anthropic on its dialect; on OpenAI's, Codex where
  the credentials carry an account id, else public, an empty list's
  included. It refuses, `Error::Limits`: a key with an account id; an
  account id on Anthropic; a list mixing the two OpenAI routes; on
  OpenAI's dialect, a path other than the route's endpoint target;
  `without_credential` on Codex. `provider::scripts` takes the list.
  `Service::new` refuses `x-api-key` among the configured headers, as it
  does `authorization`. `entrance` reads the three fields into
  `credentials::Fields` (a repeated one is unauthorized) and
  `credentials::authorize(route, list, &fields, without_credential) ->
  Auth` (`Matched(u32)`, `Open`, `Unauthorized`; `Clone, Copy,
  PartialEq, Eq, Debug`, for the reference test's `assert_eq!`) matches
  them: on
  Responses `Authorization: Bearer` against every value; on Anthropic
  `x-api-key` against the keys and `Authorization: Bearer` against the
  bearers, both fields present unauthorized; on Codex the matched
  bearer's account id must equal `chatgpt-account-id`. No field is
  `Open` only under `without_credential`. The query's `credential` is
  the matched index, else `None`: `provider::Server` gains a field
  holding it, which `entrance` sets and `requested` copies into
  `Query.credential` (`documents::request` sets `None`). `Peer::new` and the Exchange call
  `route` and keep a one-entry list (04.2 and 04.4 take more).
  `prepared_with_options` of `tests/llm/src/fake.rs` maps
  `Provider::OpenAi` to `documents::Provider::OpenAi`, as it does
  `OpenAiCodex` (landed with 02.1); `route` reads the route from the
  credential; `delayed` sets `Endpoint::openai()` and a bearer on it
  (02.1 left its arm empty). `Exchange::new_configured` and the test
  module's `delayed` copy `input.credential.scheme` where they write
  `Scheme::Bearer`, or a key call is unauthorized at the Proof. Downstream (`without_credential:
  false`, a one-entry slice; about 10 lines): smith
  `tests/protocol-llm/tests/component_peer.rs` `run` and
  `tests/agent-process/src/fake.rs` `configured_bounds`, jig
  `tests/channel/src/processes.rs` `window_provider`, and the repository
  downstream of jig, its sign-in support's `llm()`.
Test shape: units in `credentials.rs`'s tests, each under 20 ms, literal
  field bytes, `fields(authorization, api_key) -> Fields` new there:
  `each_route_authorizes_its_schemes_in_their_fields`, rows public (a
  bearer, a key, each as `Authorization: Bearer`: matched; a key in
  `x-api-key`: unauthorized), Codex (the matched bearer's own id:
  matched; another credential's id, or none: unauthorized), Anthropic (a
  key in `x-api-key`, a bearer in `Authorization`: matched; a key in
  `Authorization`, a bearer in `x-api-key`, both fields, a repeated
  field: unauthorized); `the_route_follows_the_credentials`, rows;
  `a_list_the_route_cannot_serve_is_refused`, a row per refusal above;
  `no_credential_field_is_authorized_only_where_configured`, rows public
  and Anthropic with and without `without_credential`. The existing
  tests of the four packages keep their assertions, changing only their
  entry calls and `Config` literals. The reference test:
  ```rust
  #[test]
  fn each_route_authorizes_its_schemes_in_their_fields() {
      let list = [Credential::key(b"k0".as_slice().into()), Credential::bearer(b"b1".as_slice().into())];
      let got = |authorization, api_key| authorize(Provider::Anthropic, &list, &fields(authorization, api_key), false);
      assert_eq!(got(None, Some(b"k0")), Auth::Matched(0));
      assert_eq!(got(Some(b"Bearer b1"), None), Auth::Matched(1));
      assert_eq!(got(Some(b"Bearer k0"), None), Auth::Unauthorized);
      assert_eq!(got(Some(b"Bearer b1"), Some(b"k0")), Auth::Unauthorized);
  }
  ```
Proof: joint,
  `scripted_fake_peer_answers_over_io_and_simulated_loopback` of
  `tests/llm-connection/tests/simulated.rs`: the real Client's calls
  over the simulated loopback are authorized as index 0 and answered;
  under 0.5 s. Joint is above the units; no process uses the peer yet.
Don't: no route read from the request.
Size: about 300 lines. If it passes, 04.1a is `without_credential` with
  every `Config` literal, `credentials.rs` with its units and `entrance`
  calling `authorize` over a one-entry slice; 04.1b is the list through
  the entry calls, the downstream lines and the Proof.
Removes: `failure::route_of`, the entries' single credential, and
  Anthropic's authorization by a bearer alone.

### 04.2 The peer's list and its observations
Scope: fake-llm.md 3 (`accept`; no observation holds a value) and 5's
  bounds; design questions 6 and 11.
Needs: 04.1; 03.1's `Line::Fail` and `Query.credential`; 03.2's
  `provider::scripts`.
Files: `testing/skein-fake-llm-protocol/src/provider.rs`,
  `src/provider/credentials.rs`; `testing/skein-fake-peers/src/llm.rs`,
  `tests/llm.rs`, `tests/llm/request.rs` (new); `tests/llm/src/fake.rs`,
  `tests/llm/tests/fake.rs`; `tests/llm-connection/src/simulated.rs`.
Shape: `provider::Event::Head { headers, presented: Presented }`.
  `Presented { route, scheme: Option<Scheme>, index: Option<u32> }`
  (new) is what the request presented; `headers` leaves out
  `authorization`, `x-api-key` and `chatgpt-account-id`.
  `credentials.rs` defines `Presented` and `MAX_CREDENTIALS` (new, 16);
  `provider.rs` re-exports both with `route`. `Peer` holds `credentials:
  Vec<Credential>`; `Peer::new` keeps its signature. `Peer::accept(&mut
  self, credential) -> Result<u32, Error>` (new) returns the next index,
  and refuses, `Error::Limits`: once a request head has reached the
  peer; a list `route` refuses; a key at an index an Anthropic reset
  line names (design question 11, via `provider::scripts`); one past
  `MAX_CREDENTIALS`; the list's value and account-id bytes past
  `http.head`. `Observation::Credential { connection, route, scheme,
  index }` (new) follows each `Head` and owns no bytes. `worst_case`
  prices the list as `http.head` bytes, as today, plus `MAX_CREDENTIALS`
  entries; each request adds one observation. In skein: `simulated.rs`
  `observe` matches the new arm and asserts `Credential { index:
  Some(0), .. }`, not the `authorization` field; `run_with_shutdown`
  counts `calls * 6` observations, its `large` runs use the largest list
  the head cap admits; `take` of `tests/llm/src/fake.rs` takes
  `presented`, and its existing `observe_head` gains `presented`; the heads test of `tests/llm/tests/fake.rs` expects the
  reader's head less the three fields; the peers' `make` becomes
  `make_on(endpoint, credential, address, transport)`, and `make` calls
  it with the Codex call. Downstream: smith
  `tests/agent-process/src/fake.rs` `queries` adds `Credential { .. }`
  to its `None` arm. Its `limits()` (256 observations) now holds 42
  requests where it held 51, and every world that takes it moves: smith
  `tests/local-process`, `tests/hosts`, `tests/end-to-end`,
  `crates/smith-local-service` and `-shell` tests; jig `tests/channel`
  (`src/processes.rs` `window_provider`) and `tests/one-process`; the
  repository downstream of jig, its sign-in support's `llm()`. Each
  repin counts its worlds' requests on one peer against 42 and, past it,
  raises `observations` in smith's `limits()`; the memory ceilings price
  each observation and move with it, and their gates check them again
  (decisions.md 15). Jig's `diagnostics` uses `matches!`: none.
Test shape: world, `testing/skein-fake-peers/tests/llm.rs`, each under
  0.1 s, a scripted socket client sending literal heads
  (`request::request(sim, server, client, peer, fields, body) ->
  Vec<u8>`, new in `tests/llm/request.rs`, declared `mod request;` in
  `tests/llm.rs` alone, since in `tests/support/mod.rs` it is dead code
  in the `oauth` and `forge` binaries: it connects from `client`, sends
  a head carrying `fields`, then `body`, pumping the peer on `server`
  between the client's operations, and returns the bytes read;
  `support::call` neither pumps the peer nor returns the read buffer;
  `expect`, not `unwrap`, outside a `#[test]`):
  `accept_returns_each_credentials_index_in_order`;
  `accept_refuses_a_list_the_route_cannot_serve`, rows a key with an
  account id, a Codex credential on a public list, a public one on a
  Codex list, one past `MAX_CREDENTIALS`, one byte past `http.head`, a
  key at a reset line's index;
  `accept_is_refused_once_a_request_has_reached_the_peer`;
  `each_request_is_observed_by_route_scheme_and_index`, rows on
  Anthropic and the public route: a key at 0, a bearer at 1, an unknown
  key and bearer (no index), no field (the scheme none), the matched
  rows with their `Query.credential`; the unauthorized rows reach no
  `Query` (`requested` answers `server.refused` first) and assert the
  `Credential` observation alone; `no_head_observation_holds_a_credential_field`.
  Memory: the full fake process test of
  `tests/llm-connection/tests/memory.rs` passes with the largest list.
  The reference test, with `make_on` and `request::request` new:
  ```rust
  #[test]
  fn each_request_is_observed_by_route_scheme_and_index() {
      let key = Credential::key(b"k0".as_slice().into());
      let address = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
      let mut peer = make_on(Endpoint::anthropic(), key, address, Transport::Plaintext).unwrap();
      assert_eq!(peer.accept(Credential::bearer(b"b1".as_slice().into())).unwrap(), 1);
      let mut sim = skein_sim::Sim::new(23, skein_sim::Config::calm());
      let (server, client) = (sim.spawn_process(), sim.spawn_process());
      request::request(&mut sim, server, client, &mut peer, b"authorization: Bearer b1\r\n", b"");
      let seen = peer.observations().iter().find_map(|o| match o {
          llm::Observation::Credential { route, scheme, index, .. } => Some((*route, *scheme, *index)),
          _ => None,
      });
      assert_eq!(seen, Some((Provider::Anthropic, Some(Scheme::Bearer), Some(1))));
  }
  ```
Proof: joint,
  `scripted_fake_peer_answers_over_io_and_simulated_loopback` of
  `tests/llm-connection/tests/simulated.rs`: each of the real Client's
  requests is observed as its route, a scheme and index 0, with no
  credential field in its head observation; under 0.5 s. Joint is above
  the world tests; no process uses the peer yet.
Don't: no `accept` after serving; no new `Error` variant.
Size: about 280 lines.
Removes: the credential fields in head observations, and `Peer::new`'s
  single-credential bound.

### 04.3 The public route's cache rules and refused fields
Scope: fake-llm.md 3.1 (the cache instructions; what a plan's route
  refuses; the echoes); llm.md 4.8.
Needs: 04.1's route; skein 02.3's public request and 02.1's endpoint;
  03.2's `Unsupported` row and `param`.
Files: `testing/skein-fake-llm-protocol/src/provider.rs`,
  `src/provider/public.rs` (new), `src/documents.rs`;
  `testing/skein-fake-peers/tests/llm.rs`; `tests/llm/tests/fake.rs`.
Shape: `public.rs` holds `reserved(name)`, true for `session-id` and
  `thread-id` in any case: on the public route `entrance` refuses such a
  header as invalid, and `Service::new` refuses it among a public peer's
  configured headers. `requested` takes the scope from
  `prompt_cache_key` alone on the public route (`documents::request`
  already requires its 36-byte lowercase form, through `parse_uuid`);
  the Codex route keeps its rules (the key and `session-id` the same,
  `thread-id` well formed). `documents::Error` gains `Refused(&'static
  [u8])`, which the `requested` and `next_event` matches of
  `provider.rs` take. `documents::request_on(route, data, limits)` (new)
  returns it for the first of llm.md 4.8's fifteen fields at the
  request's top level, on the public route alone, from the document it
  already parses; `documents::request` keeps its signature (smith's
  `tests/protocol-llm/tests/nesting.rs` calls it). The peer answers
  `Refused(field)` as an `Unsupported` failure: 400
  `subscription_sharing_unsupported_capability`, the name as `param`, by
  03.2's table. The echoes, gated on the dialect in `requested`, serve
  the public route unchanged. Downstream: none.
Test shape: units in `public.rs`'s tests, under 20 ms, reference test:
  ```rust
  #[test]
  fn the_public_route_refuses_session_and_thread_headers() {
      for name in [&b"session-id"[..], b"Thread-Id", b"THREAD-ID"] {
          assert!(reserved(name), "{name:?}");
      }
      assert!(!reserved(b"x-request-id"));
  }
  ```
  World, `testing/skein-fake-peers/tests/llm.rs`, each under 0.1 s, the
  bodies from `body(extra)` (new: the smallest public request with
  `extra` after `input`): `a_refused_field_is_named` calls
  `documents::request_on` with
  `skein_llm_world::fake::limits(&skein_llm_world::limits()).documents`,
  as `make` does for the peer (the protocol crate cannot build them:
  `skein_llm_world` depends on it), a row per field of the fifteen and
  one with none; `a_refused_field_is_answered_unsupported_naming_it`,
  via `request::request`, rows `top_p` and `prompt_cache_retention`, the
  literal status line and error body;
  `the_public_route_takes_its_scope_from_the_cache_key_alone`, rows the
  key with no headers (a `Query` observed) and, invalid, the key with a
  `session-id`, an uppercase key, a Codex request with the key and no
  headers. Joint, `tests/llm/tests/fake.rs`, under 0.5 s:
  `a_public_call_gets_the_configured_echoes`, an `Exchange` on
  `Provider::OpenAi` via `new_with_codex_echo`, reading the terminal
  with `echoed_completion_json` and `key_count`, as the echoes test
  does.
Proof: joint, `a_public_call_gets_the_configured_echoes`: the real
  Client's public call is authorized as a bearer and its terminal
  carries `instructions`, `tools` and `attribution`; under 0.5 s. Joint
  is above the world tests; no process reaches the public route yet.
Don't: no second parse of the body; no refused field on the Codex route.
Size: about 200 lines.
Removes: nothing.

### 04.4 The Exchange with a list, and the stories
Scope: fake-llm.md 5 (the Exchange, the accounts' stories, the bounds)
  and 6, as the real loop of the repository downstream of jig uses them
  (its testing.md, 2.4); design questions 6 and 12.
Needs: 04.2's list and `Presented`; 04.3's public rules; 03.2's failures
  by credential; skein 01.5's call with no credential.
Files: `tests/llm/src/fake.rs`; `tests/llm/tests/fake_credentials.rs`
  (new), `tests/llm/tests/fake_memory.rs`; the callers in
  `tests/llm/tests/bounded_peer_memory.rs` (5 of `extra_worst_case`),
  `fake_memory.rs` (1 of it) and `fake.rs` (2 of `prepared`).
Shape: `Exchange::prepared`, `prepared_with_codex_echo` and
  `prepared_with_options` take `credential: Option<Credential>`, as
  skein 01.5 makes the call's: `Some` is index 0, `None` an empty list
  under `without_credential`; the `new` constructors pass the call's.
  `Exchange::accept(&mut self, credential) -> Result<u32,
  provider::Error>` (new) applies `Peer::accept`'s rules to its list.
  `Exchange.presented: Vec<provider::Presented>` (new) holds each
  request's, counted under `ObservationLimits.heads`.
  `extra_worst_case(bounds, observations, endpoint, credentials:
  &[Credential])` prices every credential the caller will give, at its
  bytes, and the `presented` entries. Downstream (about 8 lines):
  smith's callers of `Exchange::prepared` take `Some(credential)`:
  `tests/protocol-llm/tests/client.rs`, `production_contract.rs`,
  `adapter_boundary.rs` (two), `tests/agent/src/wire.rs`, and
  `prepared_with_options` in `tests/protocol-llm/src/wire.rs`;
  `extra_worst_case` in `tests/protocol-llm/tests/root_memory.rs` (two
  places) takes a one-entry slice. jig and the repository downstream of
  jig: none.
Test shape: worlds, `fake_credentials.rs`, each under 0.5 s (the real
  Client), asserting the client's terminal and `presented` as literal
  routes, schemes and indices:
  `both_schemes_on_one_endpoint_are_answered_by_index`, rows Anthropic
  and public, the Client presenting the key (0) and the bearer (1);
  `a_failing_line_naming_one_credential_lets_another_through`, rows the
  named one (its class and report) and the other (the lines after);
  `a_call_with_no_credential_is_observed_as_the_scheme_none`, rows a
  configured peer (answered) and one holding a key (unauthorized), on
  public and Anthropic;
  `a_credential_the_peer_does_not_hold_is_unauthorized`, rows a key and
  a bearer (their scheme, no index);
  `no_observation_holds_a_credentials_value`, over the rows above;
  `an_issuers_tokens_are_accepted_as_bearers_before_serving`, three
  bearers given at start, a call with each answered at its index,
  `accept` after the first request refused;
  `a_revoked_token_stays_accepted_until_a_line_names_it`, rows no line
  (answered) and `Unauthorized` naming index 1 (token 1 unauthorized,
  token 2 answered). The public route's failures, which 03.3 leaves
  here: `the_public_routes_failures_reach_the_client_as_scripted`, rows
  `Exhausted` with and without a reset, `RateLimited`, `Refused` and
  `Unsupported` naming a field, each read as the Client's `Failed` class
  and report; and
  `a_failure_after_text_ends_the_stream_on_the_public_route`. Memory, in
  `fake_memory.rs`:
  `the_largest_list_and_failing_line_fit_the_peer_price`,
  `MAX_CREDENTIALS` credentials at the head cap and a failing line
  naming each index, its report at the script cap, the high water within
  the Client's price plus `extra_worst_case`. The reference test, with
  `world(provider, sent)` and `completion` new in `fake_credentials.rs`
  (`fake.rs`'s `completion`, `echoed_completion_json` and `key_count`
  are private to it): the Client is prepared with the
  credential at index `sent` of [key, bearer]; the Exchange is
  `prepared` with the key and `accept`s the bearer:
  ```rust
  #[test]
  fn both_schemes_on_one_endpoint_are_answered_by_index() {
      for provider in [Provider::Anthropic, Provider::OpenAi] {
          for (sent, scheme) in [(0, Scheme::Key), (1, Scheme::Bearer)] {
              let mut world = world(provider, sent);
              world.start();
              world.run();
              let want = provider::Presented { route: provider, scheme: Some(scheme), index: Some(sent) };
              assert_eq!(world.presented, [want], "{provider:?} {sent}");
              let _ = completion(&world);
          }
      }
  }
  ```
Proof: joint, `a_revoked_token_stays_accepted_until_a_line_names_it`:
  the real Client's call with token 1 is answered, then unauthorized
  once a line names index 1, while token 2 is answered; under 0.5 s.
  Joint is the highest layer here: only smith's processes use the
  Exchange, and they are repinned after skein.
Don't: no token read from an issuer (06.1's); no value on an assertion's
  expected side; no price from the list's live length.
Size: about 300 lines. If it passes, 04.4a is `Exchange::prepared` with
  the `Option`, `accept`, `presented`, `extra_worst_case`, the smith
  callers and the first five stories; 04.4b is the issuer's two token
  stories, the public route's failure worlds and the memory test, with
  the Proof judged there.
Removes: the Exchange's single `credential`.

## Interface sketches

```rust
// testing/skein-fake-llm-protocol/src/provider/credentials.rs; provider.rs: pub use credentials::{MAX_CREDENTIALS, Presented, route};
/// What one request presented: its route, the scheme its field shows, the index it matched.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Presented { pub route: skein_llm::Provider, pub scheme: Option<skein_llm::Scheme>, pub index: Option<u32> }
/// The most credentials one peer holds.
pub const MAX_CREDENTIALS: u32 = 16;
// testing/skein-fake-llm-protocol/src/documents.rs
/// Decodes a request as `route` takes it; `Error` gains `Refused(&'static [u8])` for llm.md 4.8's fields.
pub fn request_on(route: skein_llm::Provider, data: &[u8], limits: &Limits) -> Result<api::Query, Error> { todo!() }
// testing/skein-fake-peers/src/llm.rs: Observation gains `Credential { connection, route, scheme, index }`.
impl Peer {
    /// Adds a credential before the peer serves, returning its index.
    pub fn accept(&mut self, credential: Credential) -> Result<u32, Error> { todo!() }
}
// tests/llm/src/fake.rs: Exchange gains `pub presented: Vec<provider::Presented>`; `prepared_with_codex_echo` and
// `prepared_with_options` take `prepared`'s `Option<Credential>`.
impl Exchange {
    /// Adopts the caller's prepared Client; `None` takes calls with no credential.
    pub fn prepared(machine: client::Client, endpoint: Endpoint, credential: Option<Credential>,
        bounds: client::Limits, scripts: Box<[api::Script]>) -> Self { todo!() }
    /// Adds a credential to the peer's list before the first request.
    /// # Errors
    /// As `Peer::accept`.
    pub fn accept(&mut self, credential: Credential) -> Result<u32, provider::Error> { todo!() }
}
/// Prices the peer and its world beside the Client, every given credential included.
pub fn extra_worst_case(bounds: &client::Limits, observations: &ObservationLimits,
    endpoint: &Endpoint, credentials: &[Credential]) -> Option<u64> { todo!() }
```

## Done when

- `rg -c '\bpub fn accept\b'` on `testing/skein-fake-peers/src/llm.rs`
  and on `tests/llm/src/fake.rs` prints `1` for each.
- `rg -n -U '^\s*Credential \{\s*connection'
  testing/skein-fake-peers/src/llm.rs` prints the variant's one line.
- `rg -nw route_of testing/skein-fake-llm-protocol/src` prints nothing.
- code-organization.md 8's size count flags no file of this session.

