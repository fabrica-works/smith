# skein 07: OpenID

An OpenID sign-in fetches the issuer's keys before its visit, with
bounded GETs, and holds them for that sign-in only. The code exchange's
answer must then carry an ID token whose RS256 signature and claims the
client checks; a failed check is `Unverified`, naming the check, and
returns nothing. 07.2 removes skein 05.3's guard, so an OpenID sign-in
ends in `Tokens` for the first time.

Needs at start: skein 05.3's `OpenId` terms and skein 06.2's documents,
ID token and `Event::Get`, landed on skein's `origin/main`.

## Read first

- skein's design at `49725eca`: `oauth.md` 1, 2.1, 2.2, 3, 4, 5, 7;
  `tls.md` 4.
- skein `docs/foundation/programming-model.md` 3, 4, 5.4, 6.3, 9;
  `testing-strategy.md` 2.1, 2.5, 8, 10.
- The README's Rules; code-organization.md (owner 07.1 and 07.2, which
  use 05.1's check and 06.2's fake; its fixtures, 7's Don't, 8's
  sizes); decisions.md 10, 13, 15, 16 and 20.

## What the design asks

- oauth.md 1, 2.2: the keys first, held for the sign-in; then the token.
- oauth.md 3: `AwaitKeys`; the `Http` method; a GET retried.
- oauth.md 4: `Unverified` names the check; the code is spent.
- oauth.md 5: the caps and the skew; no ring or rustls type.
- oauth.md 7: a refresh's ID token is not checked.
- tls.md 4: the one RS256 check, `verify_rs256`.

## Choices

- `Check` holds the seven names oauth.md, 4 gives; `Missing` is both a
  missing ID token and a missing or overlong subject.
- Every way the keys fail before the visit is `Unverified { check:
  Keys }`: an unreadable or overlong document, another issuer, no key
  that fits, an error status, a spent attempt budget, the sign-in's
  deadline. Only `Cancel` is `Cancelled`. A kid the set does not hold,
  or two keys and no kid, is `Keys` as well: no key fits.
- The keys, the GET and `Unverified` land together in 07.1, which can
  fail no other way; 07.2 adds the check over them and the skew.
- The driver sends no GET until skein 10.2, which checks its origin:
  until then, the driver ends a GET demand as today's unbuildable
  demand, `Exchange(Malformed)`, and 10.2's Removes lists that.
- Each new limit's fixture and smallest valid value is in its Shape
  (decision 15); each downstream break is in its Shape (decision 13);
  an `Unverified` arm downstream is unreachable (decision 16). Unit and
  world tiers (decision 10).

## Rules that bind it

- `machine.rs` grows by at most ~100 lines in each increment: the
  documents' reading and the key filter go in `keys.rs`, the token's
  check in `jwt.rs`.
- No `ring::` or `rustls::` in `crates/skein-oauth/src`; the one check
  is 05.1's `skein_tls::verify_rs256`.
- The keys live in the exchange and go with it: no key outlives its
  sign-in, in the client or the driver.
- A type holding a nonce, an ID token or a token keeps its
  `#[expect(missing_debug_implementations, ..)]`; tests assert on
  variants, names and lengths, never on those bytes.
- `validate_registration`, `client_worst_case` and `Client::new` keep
  their signatures: smith and jig call them.

## Increments

### 07.1 The keys, by GET
Scope: oauth.md 2.2 (the keys come first; which keys fit), 3
  (`AwaitKeys`, the method, a GET's retry), 4 (`Unverified`, the keys),
  5 (the caps).
Needs: skein 05.3's `OpenId` terms; skein 06.2's documents and
  `Event::Get`.
Files: `crates/skein-oauth/src/keys.rs` (new), `machine.rs`,
  `machine/fetch.rs` (new), `common.rs`, `lib.rs`, `redirect.rs`,
  `machine_tests.rs`, `machine_tests/terms.rs`, `machine_tests/openid.rs`
  (new); `tests/oauth/src/lib.rs`, `tests/oauth/tests/openid.rs` (new),
  `world.rs`, `memory.rs`, `fuzzy_world.rs`;
  `crates/skein-oauth-accounts/src/exchange.rs` (not `component.rs`:
  ruling 118). The
  `HttpRequest` literals: `tests/oauth/tests/world.rs`
  (`authorization_code_cannot_be_redeemed_twice`), `memory.rs`
  (`client_and_rotating_issuer_steps_fit_declared_heap_bounds`),
  `tests/oauth-accounts/src/world.rs` (`protocol`),
  `src/sign_in/protocol.rs` (`protocol`), `exchange.rs` (`token_web`)
  and `testing/skein-fake-peers/src/oauth.rs` (`post`). The
  `ClientLimits` literals: `machine_tests.rs` (`limits`),
  `tests/oauth/src/lib.rs` (`client_limits`), `tests/oauth/tests/`
  `memory.rs` and `fuzzy_world.rs` (`scenario`) where 05.2 left them,
  `tests/oauth-accounts/src/world.rs` (`limits`), `tests/memory.rs`
  (`every_account_holds_a_maximal_record_within_the_heap_bound`) and
  skein-oauth-accounts's `src/tests.rs` (`limits`).
Shape: `HttpRequest.method: Method` (new `Method { Post, Get }`); a
  GET's `content_type` and `body` are empty, and every POST names
  `Post`. Under OpenID terms, a `SignIn` builds and checks its URL as
  today, then goes to `Phase::AwaitKeys { id, deadline }` (new) and
  emits a GET of the terms' key set URL, else of
  `<issuer>/.well-known/openid-configuration`; the `Exchange` holds the
  URL, the fetch in flight (the metadata, or the key set and its URL),
  which `Backoff` re-emits, and then the keys. `keys.rs` (new) reads
  both documents under `document`'s limits, each within its own cap:
  the metadata's `issuer` must equal the terms' issuer, and its
  `jwks_uri` must pass `issuer_endpoint` and `uri_bytes`; the key set's
  `keys` array, each entry kept where `kty` is `RSA`, `n` and `e`
  decode as base64url, `use` is `sig` or absent and the modulus,
  counted from its first set bit, holds 2048 to `modulus_bits` bits,
  with its `kid` if any, at most `keys` kept; any other entry is
  ignored. `Key.modulus` holds `n` without its leading zero bytes, so
  the bit count and `verify_rs256` read the same bytes. With at least
  one key held, the machine goes to `AwaitRedirect` and emits the
  `Visit`, and its attempt count starts again for the code exchange. A
  GET is retried after any transient failure, whatever its evidence,
  within `max_attempts` for that fetch and the sign-in's deadline.
  Every other end of `AwaitKeys` is `Failed { failure: Unverified {
  check: Keys } }` with no `Visit`; `Cancel` is `Cancelled`. New
  `Failure::Unverified { check: Check }` and `Check` (the sketches);
  `retry_or_fail`, the one exhaustive match on `Failure`, counts it not
  transient. Where it goes (ruling 118: `machine.rs` is 759 lines and
  `machine_tests.rs` 732 on main): `Phase::AwaitKeys` and its arm stay
  in `machine.rs`, which hands the GET's answer to `machine/fetch.rs`
  (new, beside `machine/revoke.rs`: the fetch in flight, its retry, the
  move to `AwaitRedirect`); 07.1's and 07.2's machine unit tests go in
  `machine_tests/openid.rs` (new, beside `terms.rs`), so that neither
  file passes 800 lines. `redirect.rs`'s `issuer_endpoint` becomes
  `pub(crate)` for `keys.rs`. `ClientLimits` gains `metadata_bytes`, `key_set_bytes`,
  `keys` and `modulus_bits`; `client_worst_case` counts the larger
  document and `keys` held keys, each `modulus_bits / 8` bytes and two
  strings. `Client::new` refuses `modulus_bits` outside 2048..=8192, as
  `Limit`, so its smallest valid value is 2048; `validate_registration`
  refuses OpenID terms where `keys` or `key_set_bytes` is 0, or
  `metadata_bytes` is 0 and no key set URL is named, as `Limit`, so
  each of those three has smallest valid value 0. A host's
  `string_bytes` must hold a key's `n` (342 bytes at 2048 bits), or no
  set is readable; no host downstream takes OpenID terms. 07.1's worlds
  rest on 06.2's `documents()` (`document_bytes` 4096, `string_bytes`
  1024): under main's 1024 and 256 no key is readable; under 06.2's,
  four keys (1666 bytes, 61 tokens) fit `tokens` 64, five (2080 bytes,
  75 tokens) do not. Fixture in
  `tests/oauth/src/lib.rs`: 1024, 2048, 4 and 2048, and `begin`,
  `openid_registration` and `openid_issuer` (new; the test shape). The
  world answers a GET with skein 06.2's `fake::Event::Get`. The
  driver: `Web::new` returns `None` for a GET (a `match` on
  `request.method`: clippy disallows `matches!`), which `component.rs`
  already ends as `Exchange(Malformed)`, unchanged, until skein 10.2;
  `exchange.rs`'s test demand names `Post`. In skein, the driver's
  `ClientLimits` literals (`tests/oauth-accounts/src/world.rs`
  `limits`, `tests/memory.rs`, skein-oauth-accounts's `src/tests.rs`
  `limits`) take 0, 0, 0 and 2048 too: no driver registration takes
  OpenID terms. The tests that meet the GET (ruling 118):
  `machine_tests/terms.rs`' `a_nonce_is_bounded_and_needs_openid` and
  `an_admitted_nonce_is_sent_in_the_visit_url` answer the key GET (a
  one-key set literal) before they read the `Visit`; `machine_tests.rs`'
  `limits()` sets `metadata_bytes` 1024, `key_set_bytes` 4096, `keys` 4,
  `document_bytes` 4096 and `tokens` 128 (the row of `keys` and one more
  is 2080 bytes and 75 tokens), so that
  `terms_are_refused_where_the_design_refuses` still reaches its rows,
  and `modulus_bits` with `string_bytes` such that a modulus one bit past
  `modulus_bits` reads as a string (a 4097-bit `n` is 684 base64url
  bytes); a test that rests on `string_bytes` 512 keeps its own limits.
  `world.rs`' `an_openid_sign_in_fails_before_its_check_exists`, whose
  `visit()` now meets the GET, answers the keys first through `begin`,
  the fake's `Event::Get` and the answer; its `Malformed` holds until
  07.2 removes the guard. Downstream, the `ClientLimits` literals take 0,
  0, 0 and 2048: smith-local-shell `src/credentials/product.rs`,
  smith-local-service `src/credentials/tests.rs`; jig's
  `crates/jig-protocol-config/src/profile/small.rs`,
  `crates/jig-protocol-accounts/src/tests/fixtures.rs`,
  `tests/protocol-accounts/src/fixtures.rs`. jig's exhaustive match in
  `crates/jig-protocol-accounts/src/translate.rs` gains an unreachable
  `Unverified` arm: no registration there has OpenID terms. No
  `HttpRequest` literal is downstream.
Test shape: unit, `machine_tests/openid.rs` over `machine_tests.rs`'
  `limits()`, `registration(public, wire)` given OpenID terms, `step`,
  `request`, `answered`, and document literals, each under 20 ms:
  `an_openid_sign_in_fetches_its_keys_before_the_visit`, rows a key set
  URL, the metadata then `jwks_uri`;
  `a_get_carries_no_body_or_content_type`;
  `a_non_openid_sign_in_visits_at_once`;
  `the_key_set_keeps_only_keys_that_fit`, rows `kty` `EC`, `use` `enc`,
  no `use`, 2047 bits, 2048, `modulus_bits`, one bit past, a leading
  zero byte (kept, its modulus 256 bytes), `keys` and one more;
  `each_keys_failure_is_unverified_before_any_visit`, rows unreadable
  metadata, unreadable set, over each cap, another issuer, a `jwks_uri`
  on `http` off loopback, no key that fits, 404, budget spent,
  deadline; `a_get_is_retried_after_an_ambiguous_send`, rows unknown
  evidence then 200, 503 then 200;
  `cancel_while_fetching_keys_is_cancelled`;
  `the_key_caps_are_admitted_where_the_design_allows`, rows
  `modulus_bits` 2047, 2048, 8192, 8193, and each zero cap with and
  without a key set URL. World, `tests/oauth/tests/openid.rs` over
  `tests/oauth/src/lib.rs`' `client_limits()`, `fake_limits()`,
  `client_step`, `fake_step`, `http` and `response`, which gains
  `begin(client, registration, seconds) -> Option<oauth::Request>` (the
  `SignIn` event `visit` builds, its `key` 7, at `time(seconds)`, with a
  nonce, returning the client's first demand unasserted), `openid_registration()` (a key set URL named)
  and `openid_issuer(fault)`; each under 0.1 s. Reference test, imports
  as `world.rs`' and the fixtures from `skein_oauth_world`:
  ```rust
  #[test]
  fn the_keys_are_fetched_again_at_each_sign_in() {
      let mut client = oauth::Client::new(client_limits()).expect("client");
      let mut issuer = openid_issuer(None);
      for round in 1..=2 {
          let get = http(begin(&mut client, openid_registration(), round));
          assert_eq!(get.method, oauth::Method::Get);
          assert!(get.body.is_empty() && get.content_type.is_empty());
          let event = fake::Event::Get { id: get.id, endpoint: get.endpoint, now: time(round), wall: wall(100) };
          let answer = response(fake_step(&mut issuer, event));
          let visit = client_step(&mut client, oauth::Event::Http(answer));
          assert!(matches!(visit, Some(oauth::Request::Visit { .. })));
          assert!(client_step(&mut client, oauth::Event::Cancel).is_some());
          client_step(&mut client, oauth::Event::Reset);
      }
  }
  ```
  Beside it: `each_spoiled_document_fails_before_the_visit`, rows
  `UnreadableMetadata`, `UnreadableKeySet`, `MetadataIssuer`,
  `LongModulus`, `UseEnc`, each `Unverified { check: Keys }` and no
  visit. Memory (`memory.rs`):
  `a_sign_in_holding_its_keys_fits_the_worst_case` (new; ruling 118:
  `client_and_rotating_issuer_steps_fit_declared_heap_bounds` meters
  only an `Event::Refresh` step, which holds no keys): the four caps at
  their bounds, the step that answers the key GET metered against
  `client_worst_case`.
Proof: world, `the_keys_are_fetched_again_at_each_sign_in`: two
  sign-ins on one client each emit a bodiless GET and, once the fake
  answers it, their `Visit`; under 0.1 s; unit,
  `each_keys_failure_is_unverified_before_any_visit`: every failure row
  ends `Unverified { check: Keys }` with no `Visit`; under 20 ms. World
  is the highest layer: the driver sends no GET until skein 10.2.
Don't: keep keys past the sign-in; visit before a key is held; put a
  secret or a body in a GET; check a GET's origin or resolve a name
  (skein 10.2's); fetch keys for a refresh or a revocation.
Size: about 330 lines.
Removes: nothing.

### 07.2 The ID token's check
Scope: oauth.md 2.2 (the signature, the claims, the subject, a
  refresh's token), 4 (`Unverified`), 5 (the skew), 7 (a refresh's ID
  token).
Needs: 07.1's keys and `Unverified`; skein 05.1's `verify_rs256`; skein
  06.2's ID token and its faults.
Files: `crates/skein-oauth/src/jwt.rs`, `machine.rs`, `common.rs`,
  `lib.rs`, `Cargo.toml`, `src/tests.rs`, `machine_tests/openid.rs`;
  `Cargo.lock` (`skein-oauth` gains `skein-tls`);
  `tests/oauth/src/lib.rs`, `tests/oauth/tests/openid.rs`, `world.rs`;
  the `ClientLimits` literals 07.1 lists, each gaining `skew`.
Shape: `verify_id_token` (new, crate-private, in `jwt.rs`) takes the
  token within `id_token_bytes`, the held keys, the issuer, the client
  id the exchange carried (the issued one under issued terms), the
  sign-in's nonce, the answer's wall clock, the skew and the document
  limits. It splits the compact JWS in three and decodes its header and
  payload under `document`'s limits. The header's `alg` must be
  `RS256`, else `Signature`; `pick_key` takes the held key whose `kid`
  the header names, else, where the header names none, the one key
  held, else `Keys`; `skein_tls::verify_rs256` over the key's `n` and
  `e`, the signing input and the decoded signature must hold, else
  `Signature`. Then `check_claims` reads the payload, each claim in
  this order: `iss` equals the issuer, else `Issuer`; `aud`, a string
  or an array of strings, holds the client id, and where it holds
  several, `azp` equals it, else `Audience`; `exp`, whole seconds, is
  checked as the edge below says, else `Expiry`; `nonce` equals the
  sign-in's through `skein_lib::sha256::equal`, else `Nonce`; `sub` is
  present and within `subject_bytes`, else `Missing`. The edge: the
  design's words are that `exp` "is after the injected wall clock, less
  the skew the limits allow". After is strict and the comparison exact:
  an `exp` equal to the wall clock less the skew is `Expiry`, and one
  second later is accepted. An answer with no `id_token` is `Missing`.
  An `id_token` past `id_token_bytes` never reaches the check: 05.2's
  decoder refuses the answer as `Limit`, as it does an access token
  past its cap. On success, the record keeps the subject and the token
  as its ID token, and `Tokens` is emitted as today. On any check's
  failure, `Failed { failure: Unverified { check } }`: no record, no
  retry, and the code spent. A refresh's answer is not checked and
  keeps the prior's (skein 05.2's `rotate`); without OpenID, an
  answer's `id_token` is not read and the record keeps none. 05.3's
  guard goes. `Cargo.toml` gains `skein-tls`, and so `Cargo.lock`'s
  `skein-oauth` entry. `ClientLimits.skew: Duration` (new), counted
  nowhere; `Client::new` admits any. Fixture 60 s; smallest valid 0,
  which the driver's and downstream's 07.1 sites take; jig's `translate.rs` arm
  stays unreachable.
Test shape: unit, `src/tests.rs` (`limits()`, `wall`, `boxed`, over
  header and payload literals and RFC 7515 A.2's vector), each under
  20 ms: `an_id_tokens_alg_must_be_rs256`, rows `none`, `HS256`,
  `RS384`, `RS256`; `the_key_is_chosen_by_kid_else_the_one_key`, rows a
  named kid among two, an unknown kid, no kid with one key, no kid with
  two; `rfc_7515_a2_passes_the_signature_check`: the vector copied from
  skein-tls's private `N`, `INPUT` and `SIGNATURE`
  (`crates/skein-tls/src/tests/rs256.rs`), whose payload (`iss` `joe`,
  no `aud`, `nonce` or `sub`) passes the signature and ends at
  `Issuer`, with a flipped signature bit as its `Signature` row; `each_claim_is_checked`, one row
  per claim spoiled, each its `Check`;
  `aud_may_be_a_string_or_an_array`, rows a string, one in an array,
  two with `azp` the client, two with `azp` another, two without `azp`;
  `an_expiry_on_the_skews_edge_is_expired_and_a_second_later_is_not`,
  at wall 1000 s and skew 60 s, rows `exp` 940 (`Expiry`) and 941
  (accepted), and at skew 0, rows 1000 and 1001. Reference test, with
  its payload helper (`format!` is a disallowed macro, so `Writer`
  builds it; imports `crate::jwt::{Expected, check_claims}` and
  `skein_lib::{Decimal, Writer}`):
  ```rust
  fn claims(exp: u64) -> Box<[u8]> {
      let head: &[u8] = br#"{"iss":"https://issuer.example","aud":"client","exp":"#;
      let tail: &[u8] = br#","nonce":"n-0123456789abcdef","sub":"user-42"}"#;
      let digits = Decimal::of(exp);
      let size = head.len().checked_add(digits.as_bytes().len()).expect("small");
      let mut out = Writer::new(size.checked_add(tail.len()).expect("small"));
      for part in [head, digits.as_bytes(), tail] {
          out.put(part).expect("measured first");
      }
      out.finish()
  }
  #[test]
  fn an_expiry_on_the_skews_edge_is_expired_and_a_second_later_is_not() {
      let expected = Expected { issuer: b"https://issuer.example", client_id: b"client", nonce: b"n-0123456789abcdef" };
      let skew = Duration::from_secs(60);
      for (exp, check) in [(940, Some(Check::Expiry)), (941, None)] {
          let got = check_claims(&claims(exp), &expected, wall(1000), skew, &limits());
          assert_eq!(got.err(), check, "exp {exp}");
      }
  }
  ```
  Unit, `machine_tests/openid.rs` (over `machine_tests.rs`' `limits()`,
  `registration`, `step`, `answered`), each under 20 ms:
  `an_openid_exchange_without_an_id_token_is_missing`;
  `an_id_token_past_its_cap_ends_as_limit_before_any_check`;
  `a_refresh_keeps_the_sign_ins_id_token_unchecked`. World,
  `tests/oauth/tests/openid.rs` over 07.1's fixtures, each under 0.1 s:
  `an_openid_sign_in_keeps_the_subject_and_the_id_token`;
  `each_spoiled_id_token_is_unverified_and_returns_nothing`, rows
  `UnknownKid` -> `Keys`, `AnotherKey` -> `Signature`, `AlgNone` ->
  `Signature`, `AlgHs256` -> `Signature`, `TwoKeysNoKid` -> `Keys`,
  `Issuer` -> `Issuer`, `Audience` -> `Audience`, `Azp` -> `Audience`,
  `Expired(60 s)` -> `Expiry`, `Nonce` -> `Nonce`, `NoSubject` ->
  `Missing`, `NoIdToken` -> `Missing`;
  `keys_and_audiences_the_check_ignores_still_sign_in`, rows
  `ExtraEcKey`, `TwoAudiences`, `Expired(59 s)` (the fixture skew's
  edge, one second inside it);
  `an_unverified_code_is_not_redeemed_again`;
  `an_issued_client_id_is_the_audience_checked`, under issued terms.
  The world's `an_openid_sign_in_fails_before_its_check_exists` goes.
Proof: world, `an_openid_sign_in_keeps_the_subject_and_the_id_token`:
  the sign-in ends in `Tokens` whose record holds the fake's subject
  and ID token bytes; under 0.1 s; unit,
  `an_expiry_on_the_skews_edge_is_expired_and_a_second_later_is_not`:
  `exp` 940 is `Expiry` and 941 is accepted; under 20 ms. World is the
  highest layer: the driver sends no GET, so none of its worlds reaches
  an OpenID `Tokens`, before skein 10.2.
Don't: read a claim beyond the five; return any of the sign-in on
  `Unverified`; retry after a failed check; compare the nonce with
  `==`; check a refresh's ID token; name a ring or rustls type in
  skein-oauth; take an email as the subject.
Size: about 260 lines.
Removes: skein 05.3's guard (an OpenID exchange's answer as
  `Malformed`) and its world story.

## Interface sketches

```rust
// skein_oauth (07.1)
/// What an `Http` demand asks of the caller's HTTP stack (oauth.md, 3).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Method {
    /// A token, exchange or revocation form or document.
    Post,
    /// The issuer's metadata or key set: no body and no secret.
    Get,
}
// HttpRequest { id, method: Method, endpoint, content_type, body, deadline }

/// The ID-token check that failed (oauth.md, 4).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Check {
    /// The keys: not fetched, unreadable, or none that fits.
    Keys,
    Signature,
    Issuer,
    Audience,
    Expiry,
    Nonce,
    /// No ID token, or no subject within its bound.
    Missing,
}
// Failure { .., Unverified { check: Check } }
// ClientLimits { .., metadata_bytes, key_set_bytes, keys, modulus_bits: u32 }

// crate-private (07.1): Phase::AwaitKeys { id: u64, deadline: Time }
pub(crate) struct Key {
    pub(crate) kid: Option<Box<[u8]>>,
    /// Without leading zero bytes.
    pub(crate) modulus: Box<[u8]>,
    pub(crate) exponent: Box<[u8]>,
}
// keys.rs (`todo!()` stands for each body: the workspace denies `todo`
// and `unimplemented`, so no stub lands; ruling 118)
pub(crate) fn issuer_key_set(metadata: &[u8], issuer: &[u8], limits: &ClientLimits) -> Result<Box<[u8]>, Check> { todo!() }
pub(crate) fn fitting_keys(key_set: &[u8], limits: &ClientLimits) -> Result<Box<[Key]>, Check> { todo!() }

// skein_oauth (07.2)
// ClientLimits { .., skew: Duration }
/// What the exchange expects of its ID token.
pub(crate) struct Expected<'a> {
    pub(crate) issuer: &'a [u8],
    pub(crate) client_id: &'a [u8],
    pub(crate) nonce: &'a [u8],
}
// jwt.rs: the checked token's subject; the token itself goes to the record.
pub(crate) fn verify_id_token(
    token: &[u8],
    keys: &[Key],
    expected: &Expected<'_>,
    wall: Wall,
    skew: Duration,
    limits: &Limits,
) -> Result<Box<[u8]>, Check> { todo!() }
/// The claims alone, in the design's order; `Ok` is the subject.
pub(crate) fn check_claims(payload: &[u8], expected: &Expected<'_>, wall: Wall, skew: Duration, limits: &Limits) -> Result<Box<[u8]>, Check> { todo!() }
pub(crate) fn pick_key<'k>(kid: Option<&[u8]>, keys: &'k [Key]) -> Result<&'k Key, Check> { todo!() }
```

## Questions to raise when reached

- None: ruling 118 names where 07.1's and 07.2's machine code and
  tests go.

## Done when

- `rg '\b(ring|rustls)::' crates/skein-oauth/src` prints nothing.
- `rg -c -w 'AwaitKeys' crates/skein-oauth/src/machine.rs` prints a
  count above 0.
- `rg -c -w 'verify_id_token' crates/skein-oauth/src/machine.rs` prints
  a count above 0: the check stands where 05.3's guard stood.
- `rg -c '^skein-tls' crates/skein-oauth/Cargo.toml` prints 1.
- `rg -U -c '"skein-oauth"\s+version[^\]]*"skein-tls"' Cargo.lock`
  prints 1.
- The long-files command of code-organization.md, 8 prints no file of
  `crates/skein-oauth/src`.
