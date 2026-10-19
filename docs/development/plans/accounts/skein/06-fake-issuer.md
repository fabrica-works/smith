# skein 06: the fake issuer

The fake issuer learns the issuer's terms: it issues a client id in the
redirect and checks that each later request carries it, observes by name
what each request carries, takes a redirect whose loopback port varies,
answers a revocation as scripted, and lists every access token its
script grants. It signs an ID token with one checked-in test key and
publishes its metadata and key set, each scriptable to fail. skein 11
serves them on loopback.

Needs at start: skein 05's terms, issued client id, `Revoke` and record
version 3 (05.2 to 05.5) on `origin/main`.

## Read first

- skein's design at `49725eca`: `oauth.md` 1, 2.1, 2.2, 2.3, 3, 5, 7;
  `fake-llm.md` 6; `tls.md` 4.
- skein `docs/foundation/programming-model.md` 6.3, 9, 10.2, 10.3;
  `testing-strategy.md` 2.5, 2.8, 4.1, 4.4, 8, 10.
- The README's Rules; code-organization.md (owner 06.2, its fixtures,
  7's Don't, 8's sizes); decisions.md 10, 13, 15 and design question 12.

## What the design asks

- **oauth.md 2.1, 5:** the issued client id, checked; `nonce`,
  `resource`.
- **oauth.md 2.2, 5:** an ID token signed with a test key; the
  documents.
- **oauth.md 2.3, 5:** revocation confirmed, refused or never.
- **oauth.md 5:** granted scopes, added error names, the script's
  tokens.
- **oauth.md 7:** the key the fake's own; it signs as it answers.
- **fake-llm.md 6:** the script lists the tokens the LLM peer accepts.

## Choices

- Design question 12, drafted to its recommendation: the script is the
  token list. `Issuer::granted` reads it from the queued plans and mints
  nothing; a revoked token stays accepted at the LLM peer unless a story
  scripts `Unauthorized` (fake-llm.md, 6).
- The scopes, `earliest_refresh_at` and the added error names need no
  new fake code: a `Plan` scripts them through skein 05.2's
  `TokenResponse` and `OAuthError`, and 06.1's stories drive them.
- A revocation takes the next `Plan`, from the one queue: a story
  scripts a sign-in, its refreshes and the revocation in the order its
  client sends them.
- The fake signs as it answers (oauth.md, 7's default). Unit and world;
  skein 11 has the joint story; no process tier (decision 10). Each new
  limit's fixture and smallest valid value is in its Shape (decision
  15), and each downstream break (decision 13).

## Rules that bind it

- `testing/skein-fake-oauth` stays `no_std` outside its tests, under the
  root `clippy.toml`: it names no `Vec`, `String` or `Arc`.
- The test key is one fixture, `src/key.rs`, generated once with
  `openssl` and checked in; no test and no run generates a key.
- An observation names what a request carried, never a value: no token,
  code, nonce, ID token or secret in an observation, a `Debug` or an
  assertion's expected side.
- skein-fake-peers's `Cargo.toml` and `lib.rs` are unchanged: both are
  forge-kit's hotspots.
- `Issuer::new` and `oauth::Peer::new` keep their signatures: smith and
  jig call them.
- 06.2 adds dependencies, so it lists `Cargo.lock`.

## Increments

### 06.1 The terms, revocation and the token list
Scope: oauth.md 5 (the fake's additions, the ID token aside), 2.1 (the
  issued client id), 2.3 (revocation); design question 12.
Needs: skein 05's version 3 record, terms, issued id and `Revoke`.
Files: `testing/skein-fake-oauth` (`src/lib.rs`, `src/terms.rs` and
  `src/tests.rs`, both new, `README.md`); `testing/skein-fake-peers`
  (`src/oauth.rs`: `Peer::deliver`, `fn redirect` and its test, `pub fn
  worst_case`); `tests/oauth` (`src/lib.rs`, `tests/terms.rs` (new),
  `tests/world.rs`, `tests/memory.rs`, `tests/fuzzy_world.rs`):
  `src/lib.rs` also takes from `tests/world.rs` the fixtures `response`,
  `time`, `wall`, `registration` and `error`, which 05.2 leaves there and
  `tests/terms.rs` uses; move any further one it uses the same way, and
  list it in the commit body. The `Config` and `Limits`
  literals, in skein: `tests/oauth-accounts/src/` (`world.rs`:
  `fn issuer`, `fn protocol`; `sign_in/protocol.rs`: `fn protocol`;
  `sign_in/issuer.rs`: `fn make`),
  `testing/skein-fake-peers/tests/oauth.rs` (`fn make`); in smith:
  `tests/local-process/src/process.rs` (`fn issuer_with_token`),
  `crates/smith-local-service/src/credentials/tests/private.rs`
  (`fn issuer`); in jig: `tests/protocol-accounts/src/world.rs`
  (`fn issuer`).
Shape: `fake::Config.terms: Option<fake::Terms>` (new; the sketches).
  Under issued terms, the k-th authorization's redirect carries
  `client_id`, `issued[k]` (the last repeated), unless `omit_issued`; an
  authorization admits the entry point's client id or any id issued so
  far; the code exchange, each refresh and a revocation must carry the
  latest issued id, else `refuse()` answers as today (`invalid_grant`,
  400). Under `varying_port`, an authorization's `redirect_uri` may
  differ from the config's only in its port, on `127.0.0.1` or `[::1]`:
  the code keeps it, the redirect goes there, and the exchange's
  `redirect_uri` must equal it. `fake::Request::Redirect` gains
  `client_id: Option<Box<[u8]>>`, which the peer's `redirect()` adds to
  the `Location`. The code also keeps the authorization's nonce, for
  06.2. `Issuer::carried()` (new) gives the names beyond OAuth's own
  that the last authorization or POST carried, in order (an added
  parameter, `resource`, `nonce`, `id_token_hint`, `login_hint`), and
  keeps no value. A POST to the terms' `revocation_endpoint` reads its
  form (`token`, `token_type_hint`, the client id, the secret), checks
  the client as a token POST does, and takes the next `Plan`: a 2xx
  answer to a revocation of the current refresh token spends it, so a
  later refresh is refused; an error status leaves it; a delay past the
  client's deadline, or `LoseResponse`, is "never". `Issuer::granted()`
  (new) gives the access tokens of the queued 2xx token plans, in the
  order they will be answered; it mints none. `fake::Limits.carried`
  (new) bounds the names kept, each within `document.string_bytes`, and
  widens the fields an authorization or form may hold from 10 to 10 +
  `carried`; `worst_case` counts both, and the terms' URLs within
  `uri_bytes`. `Issuer::new` refuses terms with more than `ISSUED` (a
  const, 4) issued ids, an issued id past `document.string_bytes` or a
  terms URL past `uri_bytes`; `fake::worst_case` counts them at those
  caps. skein-fake-peers' `oauth::worst_case` raises its `http.response`
  floor by `3 * document.string_bytes + 11`, the `&client_id=` that `fn
  redirect` adds, so `Peer::deliver`'s `expect` holds; a fixture whose
  `http.response` the new floor refuses rises to it, listed in the
  commit body. Fixture 8, in `tests/oauth/src/lib.rs`' `fake_limits()`;
  smallest valid 0. `terms.rs` holds the terms' checks and the carried
  names, so `lib.rs` (531) stays under 800. Breaks, each pattern taking
  `client_id`: `Request::Redirect` in `tests/oauth/src/lib.rs`
  (`fn sign_in`, which 05.2 moves there), `tests/oauth/tests/world.rs`
  (`a_localhost_registration_signs_in_through_its_exact_redirect`),
  `tests/oauth-accounts/src/sign_in/protocol.rs` (`fn protocol`) and
  `Peer::deliver`. Downstream, the `Config` literals take `terms: None`
  and the `Limits` literals `carried: 0`, and two fixtures rise to the
  new floor, `uri_bytes + 6 * string_bytes + 267` over the issuer's
  limits (2059 for 256 and 256, as skein's did at 06.1's landing):
  smith's `tests/local-process/src/process.rs` `fn issuer_with_token`
  (`response: 49_152`; about 57,611 by ruling 117's re-check) and jig's
  `tests/protocol-accounts/src/fixtures.rs` `limits()` (`response:
  2048`, which `world.rs`' `fn issuer` passes as `profile.server`);
  smith-local-service's `credentials/tests/private.rs` `fn issuer`
  (8192) holds. The closing repins make them; a further fixture the
  floor refuses rises the same way, named in the repin's commit body. `scenario` in
  `tests/oauth/tests/fuzzy_world.rs` is 93 lines on skein `887302f8`,
  under clippy's `too_many_lines` (100): only if clippy flags it does its
  `plan` match move into a `token_plan(mode, delay)` helper. `tests/oauth/src/lib.rs` gains
  `issued(ids)` (the one `fake::Terms` literal there; revocation
  endpoint `https://issuer.example/revoke`, a test sets a field by
  assignment), `issuer_with(public, terms)`, `issuing_registration()`
  (`registration(true)` under issued terms naming that endpoint) and
  `sign_in_with(client, issuer, registration)`, which `sign_in` calls
  and which passes the redirect's `client_id` on.
Test shape: unit, `src/tests.rs` (new; its own `limits()`, `config()`,
  `terms(issued)`, `issuer(terms)` and `authorize(issuer)`, which sends
  an `Authorize` for the entry point's client id), each under 20 ms:
  `an_issued_registration_redirects_with_each_issued_id`, rows first
  sign-in, signing in again, a newly issued id;
  `an_omitted_issue_leaves_the_redirect_without_a_client_id`;
  `a_later_request_without_the_latest_issued_id_is_refused`, rows
  exchange, refresh, revocation, each with the entry point's id;
  `carried_names_each_added_name_in_order`;
  `carried_names_stop_at_their_bound`, rows `carried` names and one
  past; `a_varying_port_redirect_is_held_for_its_exchange`, rows
  `127.0.0.1`, `[::1]`;
  `a_varying_port_is_refused_off_loopback_or_at_exchange`, rows
  `localhost`, another path, the exchange's other port;
  `a_confirmed_revocation_spends_the_refresh_token`;
  `an_unconfirmed_revocation_leaves_the_refresh_token`, rows 400, 503,
  delayed past the deadline, lost;
  `granted_lists_queued_tokens_in_answer_order`, rows a sign-in and two
  refreshes, an error plan between, a revocation plan between. Reference
  test:
  ```rust
  #[test]
  fn an_issued_registration_redirects_with_each_issued_id() {
      let mut issuer = issuer(terms(&[b"client-1", b"client-2"]));
      for expected in [b"client-1".as_slice(), b"client-2", b"client-2"] {
          let Some(Request::Redirect { client_id, .. }) = authorize(&mut issuer) else {
              panic!("redirect expected")
          };
          assert_eq!(client_id.as_deref(), Some(expected));
      }
  }
  ```
  World, `tests/oauth/tests/terms.rs`, the client against the fake, over
  `tests/oauth/src/lib.rs`' fixtures (`client_limits()`, `success`,
  `client_step`, `fake_step`, `response`, `record`, `time`, `wall`),
  each under 0.1 s: `the_redirects_issued_client_id_reaches_the_record`;
  `signing_in_again_keeps_a_newly_issued_client_id`;
  `an_incomplete_redirect_ends_the_sign_in_with_no_exchange`;
  `each_form_sends_what_it_must_and_nothing_else`, rows a new
  registration and signing in again, read through `carried`;
  `the_resource_reaches_the_issuer_in_every_request`, rows
  authorization, exchange, refresh;
  `a_varying_redirect_signs_in_on_its_bound_port`;
  `fewer_scopes_granted_than_required_are_named`;
  `an_added_error_name_on_an_exchange_signs_in_again`, rows 400, 401,
  500; `an_earliest_refresh_time_reaches_the_record`;
  `a_revocation_the_issuer_confirms_is_confirmed`;
  `a_revocation_the_issuer_refuses_or_loses_is_unconfirmed`, rows 400,
  lost. Reference test, imports as `world.rs`' and the fixtures from
  `skein_oauth_world`:
  ```rust
  #[test]
  fn the_redirects_issued_client_id_reaches_the_record() {
      let mut issuer = issuer_with(true, Some(issued(&[b"client-1"])));
      issuer.queue(success(b"access", Some(b"refresh"), 0)).expect("plan");
      let mut client = oauth::Client::new(client_limits()).expect("client");
      let request = sign_in_with(&mut client, &mut issuer, issuing_registration());
      let post = fake::Event::Post { request, now: time(2), wall: wall(100) };
      let answer = response(fake_step(&mut issuer, post));
      let saved = record(client_step(&mut client, oauth::Event::Http(answer)));
      assert_eq!(saved.issued_client_id.as_deref(), Some(b"client-1".as_slice()));
      assert!(client.is_done() && issuer.carried().is_empty());
  }
  ```
  Memory, `tests/oauth/tests/memory.rs`:
  `client_and_rotating_issuer_steps_fit_declared_heap_bounds` gains the
  terms and `carried` at their caps, against `fake::worst_case`.
Proof: world, `the_redirects_issued_client_id_reaches_the_record`: the
  redirect carries `client-1`, the exchange carries it back and the
  saved record holds it; under 0.1 s; unit,
  `a_later_request_without_the_latest_issued_id_is_refused`: each row is
  answered 400 `invalid_grant`; under 20 ms.
Don't: mint a token or derive one from a seed; keep a carried value;
  refuse a request for a name the fake does not know; answer a
  revocation from anything but the script.
Size: about 290 lines.
Removes: nothing.

### 06.2 The test key, the ID token and the documents
Scope: oauth.md 5 (an ID token signed with a test key, the metadata and
  key set, their faults; the key never a secret), 2.2's checks as
  faults, 7 (the fake signs as it answers).
Needs: 06.1's terms and the code's nonce.
Files: `testing/skein-fake-oauth` (`src/key.rs` and `src/openid.rs`,
  both new, `src/lib.rs`, `src/terms.rs`, `src/tests.rs`, `Cargo.toml`,
  `README.md`); `Cargo.lock`; `tests/oauth/src/lib.rs`.
Shape: `key.rs` holds `PKCS8`, a 2048-bit RSA key's PKCS#8 DER as
  standard base64 text, `MODULUS`, its `n` as base64url text, `EXPONENT`
  (`AQAB`) and `KID` (`skein-test`), with the lines that made them in
  its doc: `openssl genpkey` writes PEM, as `-outform DER` writes
  PKCS#1 on OpenSSL 3.6.4, which ring refuses as PKCS#8; `PKCS8` is
  the PEM's base64 body (or `openssl pkcs8 -topk8 -nocrypt -outform
  DER`'s), and `openssl pkey` gives `MODULUS` (ruling 117).
  `fake::Terms.openid: Option<fake::OpenId>` (new; the sketches); 06.1's
  `Terms` literals, in `src/tests.rs` and `tests/oauth/src/lib.rs`
  (`issued`), take `openid: None`. `Issuer::new` refuses an `OpenId`
  whose `issuer` or `key_set_url` passes `uri_bytes`, or whose `subject`
  passes `document.string_bytes`; `fake::worst_case` counts them at
  those caps. To a 2xx answer to an
  authorization-code exchange whose plan's `id_token` is `None`, the
  fake adds an ID token as it answers: the header `alg` `RS256`, `typ`
  `JWT` and, where `header_kid`, `kid`; the claims `iss` (the issuer),
  `aud` (the client id the exchange carried), `exp` (the POST's wall
  plus `lifetime`, in seconds: `Wall::as_secs` and
  `Duration::as_nanos` over 1,000,000,000, by checked arithmetic, as
  `Duration` has no `as_secs`), `nonce` (the code's) and `sub`
  (`subject`); signed through
  `rustls::crypto::ring::sign::any_supported_type` over the decoded key
  and its signer for `SignatureScheme::RSA_PKCS1_SHA256`, and written
  base64url. A plan that sets `id_token` is answered as scripted.
  `fake::Event::Get { id, endpoint, now, wall }` (new) answers, as
  `Request::Http` and taking no plan, the metadata at
  `<issuer>/.well-known/openid-configuration` (`issuer`,
  `authorization_endpoint`, `token_endpoint`, `jwks_uri`) and the key
  set at `key_set_url` (one key: `kty` `RSA`, `kid`, `use` `sig`, `alg`
  `RS256`, `n`, `e`); any other endpoint gets a 404. `OpenId.fault`
  scripts one `fake::Fault` (new), which changes one document or claim,
  each as its sketch comment says. The documents are written within
  `document.document_bytes` and the token within skein 05.2's
  `id_token_bytes`, else `Request::Refused`; `fake::Limits` gains
  nothing. `Cargo.toml` gains the workspace's `rustls` (the `ring`
  provider, no `std`) and, as a dev-dependency, `skein-tls`;
  `Cargo.lock` follows. The OpenID document fixture in
  `tests/oauth/src/lib.rs`' `documents()`: `document_bytes` 4096,
  `string_bytes` 1024, `id_token_bytes` 1024, `record_bytes` 2048.
  Downstream: nothing breaks, as no host there has terms. `Event::Get`
  breaks no match: only `step` matches `fake::Event`.
Test shape: unit, `src/tests.rs` (06.1's helpers, and
  `id_token(issuer)`, which authorizes, exchanges the code and returns
  the answer's `id_token`), each under 20 ms (the signature's bytes do
  not depend on ring's blinding, oauth.md, 7):
  `the_signature_verifies_against_the_published_key`, through skein
  05.1's `skein_tls::verify_rs256` over `MODULUS` and `AQAB`, not the
  signer; `signing_the_same_claims_twice_gives_equal_bytes`;
  `the_id_token_names_each_claim`, the payload's names read back, and
  `iss`, `aud` and `sub` against literals;
  `each_fault_changes_its_document_or_claim`, one row per `Fault`, the
  field it changes read back against a literal;
  `the_documents_name_the_issuer_and_its_key`;
  `a_get_for_another_endpoint_is_not_found`;
  `a_scripted_id_token_is_answered_unchanged`. Reference test, the
  signing kind (imports `skein_lib::base64::{Alphabet, decode}`,
  `skein_lib::bytes::{find, find_from}` and `skein_tls::verify_rs256`;
  the root `clippy.toml` disallows `Iterator::rposition`, and
  `arithmetic_side_effects` is denied):
  ```rust
  #[test]
  fn the_signature_verifies_against_the_published_key() {
      let token = id_token(&mut issuer(openid_terms(None)));
      let first = find(&token, b".").expect("a header");
      let cut = find_from(&token, b".", first.checked_add(1).expect("in range")).expect("three parts");
      let raw = |text: &[u8]| decode(Alphabet::Url, text, 1024).expect("base64url");
      let (input, dotted) = token.split_at_checked(cut).expect("in range");
      let signature = raw(dotted.strip_prefix(b".").expect("a dot"));
      let (n, e) = (raw(key::MODULUS.as_bytes()), raw(b"AQAB"));
      assert!(verify_rs256(&n, &e, input, &signature));
      assert!(!verify_rs256(&n, &e, b"other", &signature));
  }
  ```
Proof: unit, `the_signature_verifies_against_the_published_key`: the
  token the fake signs verifies over the published modulus and `AQAB`,
  and not over another message; under 20 ms. Unit only: the client first
  fetches the documents in skein 07.1 and checks an ID token in 07.2.
Don't: generate a key, in a test or at run time; seed or replace ring's
  blinding; name `Arc` or `Vec` (the signer's types stay inferred); keep
  a signature across answers; put the nonce or a token in a `Debug`.
Size: about 260 lines, besides the key's text.
Removes: nothing.

## Interface sketches

```rust
// skein_fake_oauth (06.1)
/// The issuer's terms the fake applies (oauth.md, 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Terms {
    /// The client id each authorization issues, the last repeated; empty issues none.
    pub issued: Box<[Box<[u8]>]>,
    /// The redirect leaves the issued client id out: an incomplete registration.
    pub omit_issued: bool,
    pub revocation_endpoint: Option<Box<[u8]>>,
    /// The redirect's loopback port may be any, held for its exchange.
    pub varying_port: bool,
    pub openid: Option<OpenId>, // 06.2
}
// Config { .., terms: Option<Terms> }
// Limits { .., carried: u32 }
// Request::Redirect { uri, state, code, client_id: Option<Box<[u8]>> }
impl Issuer {
    /// The names beyond OAuth's own that the last request carried, in order.
    #[must_use]
    pub fn carried(&self) -> &[Box<[u8]>] { &[] }
    /// The access tokens the queued plans grant, in answer order; none is minted.
    #[must_use]
    pub fn granted(&self) -> Box<[Box<[u8]>]> { Box::default() }
}

// skein_fake_oauth (06.2)
/// The fake's `OpenID` issuer, its documents and one scripted fault.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct OpenId {
    pub issuer: Box<[u8]>,
    pub key_set_url: Box<[u8]>,
    pub subject: Box<[u8]>,
    pub lifetime: Duration,
    /// The token's header names `kid`.
    pub header_kid: bool,
    pub fault: Option<Fault>,
}
/// One change to the plain documents or ID token (oauth.md, 2.2 and 5).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Fault {
    UnknownKid, // The header names a `kid` the set does not hold.
    AnotherKey, // The set publishes the key's modulus with one bit flipped.
    AlgNone, // The header's `alg` is `none`, the signature empty.
    AlgHs256, // The header's `alg` is `HS256`.
    TwoKeysNoKid, // The set holds two RSA signing keys and the header names none.
    Issuer, // `iss` names another issuer.
    Audience, // `aud` names another client.
    Azp, // `aud` holds the client and another, and `azp` names the other.
    TwoAudiences, // `aud` holds the client and another, and `azp` names the client.
    Expired(Duration), // `exp` is this long before the POST's wall clock.
    Nonce, // `nonce` differs from the authorization's.
    NoSubject, // No `sub`.
    NoIdToken, // The answer carries no `id_token`.
    LongModulus, // The set's one key has a modulus of 4096 bits.
    UseEnc, // The set's one key has `use` `enc`.
    ExtraEcKey, // The set holds an EC key beside the RSA one.
    UnreadableMetadata, // The metadata is not JSON.
    UnreadableKeySet, // The key set is not JSON.
    MetadataIssuer, // The metadata's `issuer` is another.
}
// Event::Get { id: u64, endpoint: Box<[u8]>, now: Time, wall: Wall }
```

## Questions to raise when reached

- 06.1: if `lib.rs` passes 800 lines with `terms.rs` in place, say what
  else moves before 06.2 grows it.
- 06.2: if signing must name `Arc` or `Vec` under the root
  `clippy.toml`, say where, before any `expect` is written.

## Done when

- `rg -l 'pub fn granted' testing/skein-fake-oauth/src` prints one
  file.
- `rg -w 'generate_keypair|generate_pkcs8' testing/skein-fake-oauth/src`
  prints nothing.
- `rg -c '^rustls' testing/skein-fake-oauth/Cargo.toml` and
  `rg -c '^skein-tls' testing/skein-fake-oauth/Cargo.toml` each print 1.
- code-organization.md, 8's size and long-file commands, at each step's
  end: no file crosses 800 lines and none grows past about 100.
