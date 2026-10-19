# skein 11: the fake issuer on loopback, and its tokens at the LLM peer

skein-fake-peers serves the fake issuer's documents and revocation on
loopback, as the real loop runs it, and observes no value; a joint story
hands the issuer's tokens to the fake LLM peer.

Needs at start: skein 06's revocation and `granted` (06.1) and its
documents and `Event::Get` (06.2), and skein 04.4's Exchange with
`accept` and `presented`, on `origin/main`.

## Read first

- skein's design at `49725eca`: `oauth.md` 5, 6.8; `fake-llm.md` 3, 6.
- skein `docs/foundation/testing-strategy.md` 2.5, 2.8, 4.4, 8.
- The README's Rules; code-organization.md (7's Don't, 8's sizes);
  decisions.md 10, 13 and design question 12; skein 06's Choices.

## What the design asks

- **oauth.md 5, 6.8; testing-strategy.md 2.8:** the fake issuer on
  loopback, as the real loop runs it.
- **fake-llm.md 3, 6:** the script's tokens are the LLM peer's list; the
  peer observes each call's scheme and index.

## Choices

- One increment: the routes and the joint story are one capability, the
  issuer the real loop runs beside the LLM peer.
- The joint story drives the client and the fake issuer by steps, beside
  the LLM world's `Exchange`; the routes are proved in skein-fake-peers'
  own tests (decision 10: no process tier).

## Rules that bind it

- An observation names what a request carried, never a value: no token,
  code, ID token or secret in an observation, a `Debug` or an
  assertion's expected side.
- skein-fake-peers's `Cargo.toml` is unchanged and its `lib.rs` gains
  doc lines only: both are forge-kit's hotspots.
- `oauth::Peer::new` keeps its signature: smith and jig call it.

## Increments

### 11.1 The loopback routes, and the issuer's tokens at the LLM peer
Scope: oauth.md 5 (on loopback, as the fake LLM peer runs), 6.8 (the
  real loop's use); fake-llm.md 6; design question 12.
Needs: what the session needs.
Files: `testing/skein-fake-peers` (`src/oauth.rs`, `src/oauth/route.rs`
  (new), `src/lib.rs` (doc), `tests/oauth.rs`); `tests/oauth`
  (`Cargo.toml`, `tests/issuer_tokens.rs` (new)); `Cargo.lock`.
Shape: the peer routes each call through a crate-private
  `route(&Paths, Method, &[u8]) -> Route` (new, `pub(super)` in
  `route.rs`, which `oauth.rs` calls), `Route { Authorize, Token,
  Revoke, Metadata, KeySet, Missing }` (`Clone, Copy, PartialEq, Eq,
  Debug`: the test compares it, `Connection` holds an `Option`); `Paths`
  (`pub(super)`, as `Peer::new` builds it) holds the five paths it takes
  from the config, its terms and its OpenID, `revoke`, `metadata` and
  `key_set` each an `Option` (terms and OpenID may be absent); `path()`
  refuses a URL with no path (`http://127.0.0.1`), so the metadata path
  is taken from the whole metadata URL, never from the issuer. `route`
  matches `Method` by its variants or an `if` chain, never `_ =>`
  (`wildcard_enum_match_arm` is denied in skein-fake-peers'
  `Cargo.toml`). A GET of the metadata or key set path goes to
  `issuer::Event::Get` under the next exchange id with the whole URL
  that `skein_fake_oauth::openid::document` matches (the issuer's
  followed by `/.well-known/openid-configuration`, or `key_set_url`),
  which `Peer` keeps, with the revocation endpoint, as it keeps
  `token_endpoint`; its answer is delivered as a token answer is; a POST
  to the revocation path goes to `issuer::Event::Post` with the
  revocation endpoint in its `HttpRequest`; `Connection.token_post`
  becomes `post: Option<Route>`, which `call` sets for each of the four
  routes it forwards and `deliver`'s `Request::Http` arm reads and
  clears (`fn post` no longer clears it): it observes `Answered` for a
  token or a revocation answer, never for a Metadata or KeySet answer,
  since jig's `exchange_finished` waits on `Answered { status: 200, ..
  }` (ruling 127).
  `Observation` gains `Document { connection, target }` (a GET's target)
  and `Revoke { connection }`; `Post` drops its `body` and becomes
  `Post { connection }`, as a token POST's body holds a code, a refresh
  token or a secret (every user, in skein, smith and jig, matches
  `Post { .. }`: skein's `tests/oauth-accounts/src/world.rs` and
  `sign_in.rs` too); `Redirect` drops its `location`, which holds the
  code, and becomes `Redirect { connection }` (no user reads it; smith's
  `tests/local-process/src/oauth.rs` reads the response head's
  `Location`); `Authorize`'s target keeps `id_token_hint`'s name and
  drops its value, and `Observation`'s doc drops "secret bodies have no
  Debug". `Peer::granted()` (new) passes the issuer's through.
  `worst_case` counts the new paths. `lib.rs`'s doc gains the real
  loop's recipe: the issuer in the application's process on loopback,
  its script listing every token, `granted()` given to the LLM peer at
  start, and the test following each visit to the listener as the
  scripted browser does. `tests/oauth/Cargo.toml` gains the
  dev-dependencies `skein-llm` (`skein-llm.workspace = true`) and
  `skein-llm-world` (`path = "../llm"`, as `tests/tls` names it);
  `Cargo.lock` follows. Downstream: the new variants break nothing;
  smith's `tests/local-process/src/oauth.rs` (`fn posts`) and jig's
  `tests/protocol-accounts/src/world.rs` (`fn exchange_seen`,
  `fn exchange_finished`) match with `matches!`.
Test shape: unit, `route.rs` (its own `paths()`), under 20 ms:
  `route_names_each_endpoint`, rows each route (the authorize target
  with its query, the others exact, as the token POST's target is
  compared today), a POST to a GET's path, a GET to the token path, an
  unknown path.
  Reference test:
  ```rust
  #[test]
  fn route_names_each_endpoint() {
      for (method, target, expected) in [
          (Method::Get, &b"/authorize?state=s"[..], Route::Authorize),
          (Method::Post, b"/token", Route::Token),
          (Method::Post, b"/revoke", Route::Revoke),
          (Method::Get, b"/.well-known/openid-configuration", Route::Metadata),
          (Method::Get, b"/jwks", Route::KeySet),
          (Method::Post, b"/jwks", Route::Missing),
          (Method::Get, b"/token", Route::Missing),
          (Method::Get, b"/other", Route::Missing),
      ] {
          assert_eq!(route(&paths(), method, target), expected);
      }
  }
  ```
  Fake peers, `testing/skein-fake-peers/tests/oauth.rs` over its `make`
  (gaining `terms: Option<issuer::Terms>`, `None` for today's tests)
  and `support::call`, and `fetch(sim, (pid, peer), client, request)`
  (new: one plaintext request by `Op::Send`, the reply by `Op::Recv`,
  the peer iterated between), each under 0.5 s (two processes). The new
  tests' issuer has `issuer::Limits.carried: 1` (at 0 it refuses an
  `id_token_hint`); its client is public (`client_secret: None`), so
  the sign-in sends `code_challenge` and `code_challenge_method=S256` on
  the authorize and `code_verifier` on the token POST; `observations:
  16` holds exactly the four requests of four records that
  `no_observation_holds_a_token_or_an_id_token` makes. Helpers outside a
  `#[test]` use `expect`, never `unwrap` (`unwrap_used` is denied, and
  `allow-unwrap-in-tests` covers only `#[test]` functions):
  `the_metadata_and_key_set_are_served_on_loopback`;
  `a_revocation_is_answered_as_scripted_on_loopback`, rows confirmed,
  refused; `no_observation_holds_a_token_or_an_id_token`, over a
  sign-in, a refresh and a revocation. Joint,
  `tests/oauth/tests/issuer_tokens.rs`, the client and the fake issuer
  beside the LLM world's `Exchange`, each under 0.5 s (a real client):
  `the_issuers_granted_tokens_are_the_llm_peers_bearers`: the Exchange
  starts with `granted()`'s first as index 0 and `accept`s the rest
  before serving; a call with the sign-in's access token presents index
  0, and after a refresh index 1;
  `a_token_the_issuer_revoked_stays_accepted_by_the_llm_peer`. Its
  helpers: `signed_in_and_refreshed()` queues a sign-in and a refresh,
  takes `granted()`, signs in and refreshes, and returns `granted` with
  the client's two access tokens; `exchange(granted, carried)` is an
  `Exchange::prepared` on `Endpoint::openai()` and
  `limits_for(Provider::OpenAi)`, its Client's call carrying
  `Credential::bearer(granted[carried])`, the peer holding `granted[0]`
  with the rest `accept`ed, and no script. Reference test (imports
  `skein_llm::{Scheme, client}` and `skein_llm_world::fake::Exchange`):
  ```rust
  #[test]
  fn the_issuers_granted_tokens_are_the_llm_peers_bearers() {
      let (granted, held) = signed_in_and_refreshed();
      assert_eq!(granted.as_ref(), held.as_slice(), "the script lists what the client holds");
      for (carried, index) in [(0, 0), (1, 1)] {
          let mut world = exchange(&granted, carried);
          world.start();
          world.run();
          let [seen] = world.presented.as_slice() else { panic!("one request") };
          assert_eq!((seen.scheme, seen.index), (Some(Scheme::Bearer), Some(index)));
          assert!(world.seen.iter().any(|event| matches!(event, client::Event::Completed { .. })));
      }
  }
  ```
Proof: joint, `the_issuers_granted_tokens_are_the_llm_peers_bearers`:
  the client's sign-in token and its refreshed token reach the LLM peer
  as bearers at indices 0 and 1, each call completed; under 0.5 s;
  world, `the_metadata_and_key_set_are_served_on_loopback`: the listener
  answers both documents 200 and records each GET's target; under 0.5 s.
Don't: build the LLM peer's list from a record (it is the script's); add
  a dependency to skein-fake-peers; observe a body that holds a token.
Size: about 260 lines.
Removes: the `token_post` flag; `Observation::Post`'s `body`;
  `Observation::Redirect`'s `location`.

## Interface sketches

```rust
// skein_fake_peers::oauth (11.1)
// Observation { .., Post { connection: Token }, Redirect { connection: Token }, Document { connection: Token, target: Box<[u8]> }, Revoke { connection: Token } }
impl Peer {
    /// The issuer's `granted()`, for the LLM peer's list at start.
    #[must_use]
    pub fn granted(&self) -> Box<[Box<[u8]>]> { todo!() }
}
// src/oauth/route.rs: the paths Peer::new reads, and the route of one call
// pub(super) struct Paths { the five paths }  (fields pub(super): Peer::new builds it)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Route { Authorize, Token, Revoke, Metadata, KeySet, Missing }
pub(super) fn route(paths: &Paths, method: Method, target: &[u8]) -> Route { todo!() }
```

## Done when

- `rg -l 'pub fn granted' testing/skein-fake-peers/src` prints one
  file.
- `rg -c '^skein-llm\.workspace' tests/oauth/Cargo.toml` and
  `rg -c '^skein-llm-world' tests/oauth/Cargo.toml` each print 1.
- `rg -U '(Revoke|Post)\s*\{\s*connection\b[^}]*\bbody\b'
  testing/skein-fake-peers/src` prints nothing.
- `rg -U 'Redirect\s*\{\s*connection\b[^}]*\blocation\b'
  testing/skein-fake-peers/src` prints nothing.
- `git diff --quiet origin/main -- testing/skein-fake-peers/Cargo.toml`
  exits 0.
- code-organization.md, 8's size and long-file commands, at the step's
  end: no file crosses 800 lines and none grows past about 100.
