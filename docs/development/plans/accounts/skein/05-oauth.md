# skein 05: the OAuth machine

The OAuth client learns an issuer's terms: added parameters, a resource,
a client id the issuer issues, OpenID's nonce, a varying redirect port,
required scopes, its own sign-in-again error names, and revocation. The
record moves to version 3 and still reads version 2. skein-tls gains the
RS256 check; fetching and checking the ID token are skein 07's.

Needs at start: nothing beyond skein's `origin/main`. It waits on no
live text step (decisions.md, 17).

## Read first

- skein's design at `49725eca`: `oauth.md` 1 to 5 and 7; `tls.md` 4.
- skein `docs/foundation/programming-model.md` 3, 4.4, 5.4, 6.3, 9, 10;
  `testing-strategy.md` 2.1, 2.5, 8, 10.
- The README's Rules; code-organization.md (owners 05.1 to 05.5, its
  fixtures, 7's Don't, 8's sizes); decisions.md 8, 10, 13, 15, 16, 17.

## What the design asks

- **oauth.md 1:** record version 3 (2 and 3 read), the terms, the port.
- **oauth.md 2:** a sign-in's values, granted scopes, earliest refresh.
- **oauth.md 2.1:** the issued client id, the hints, `Incomplete`.
- **oauth.md 2.2:** required scopes reported; `openid`; no ring type.
- **oauth.md 2.3, 3:** `Revoke`, and `Revoked` as its terminal.
- **oauth.md 4:** the added error names; no secret in a `Debug` or log.
- **oauth.md 5, 7:** the caps; a bad `earliest_refresh_at` is ignored.
- **tls.md 4:** one RS256 check: modulus, exponent, message, signature.

## Choices

- The design's "sign in again" is today's `Failure::Refused`, which
  `classify` gives `invalid_grant`; the added names land there.

## Rules that bind it

- After 05.3's first commit splits `machine.rs`, each increment grows it
  by at most ~100 lines; the URL's new parts go in `redirect.rs`.
- A type holding a nonce, an ID token or a token keeps its
  `#[expect(missing_debug_implementations, ..)]`; a test asserts on
  variants, counts and names, and on bytes only in 05.4's hint rows
  (fixtures, not secrets).
- `redirect_address`, `validate_registration`, `decode_record` and
  `encode_record` keep their signatures for downstream callers.
- No increment adds a dependency (skein-oauth joins skein-tls at skein
  07.2): `Cargo.lock` stays out of the Files.

## Increments

### 05.1 The RS256 check
Scope: oauth.md 2.2's check; tls.md 4.
Needs: nothing.
Files: `crates/skein-tls/src/rs256.rs` (new), `src/lib.rs`,
  `src/tests.rs` (`mod rs256;`), `src/tests/rs256.rs` (new).
Shape: `skein_tls::verify_rs256(modulus, exponent, message, signature)
  -> bool` (new). It refuses a modulus past 1024 bytes or an empty
  modulus or exponent before any work, writes the RSAPublicKey DER from
  `n` and `e` into a buffer it measured first, and verifies with the
  ring provider's `signature_verification_algorithms.mapping` entry for
  `SignatureScheme::RSA_PKCS1_SHA256` (rustls 0.23.41 lists webpki's
  `RSA_PKCS1_2048_8192_SHA256`; `config.rs` uses that provider).
Test shape: unit, `src/tests/rs256.rs`, a new file in the style of its
  neighbours (`use crate::...`), under 20 ms:
  `rs256_checks_rfc_7515_appendix_a2`, over RFC 7515 A.2's public `n`,
  `e` (`AQAB`), signing input and signature, kept as text and decoded
  with `skein_lib::base64::decode(Alphabet::Url, ..)`: the vector
  (true); a flipped bit, another message, exponent 3, a 1025-byte
  modulus, an empty modulus, an empty exponent (each false). Reference
  test, the constants copied whole from the RFC:
  ```rust
  const N: &str = "…"; // A.2's n
  const INPUT: &[u8] = b"…"; // its signing input
  const SIGNATURE: &str = "…"; // its signature
  fn raw(t: &str) -> Box<[u8]> {
      decode(Alphabet::Url, t.as_bytes(), 1024).expect("base64url")
  }
  #[test]
  fn rs256_checks_rfc_7515_appendix_a2() {
      let (n, e, sig) = (raw(N), raw("AQAB"), raw(SIGNATURE));
      let mut bad = sig.clone();
      bad[0] ^= 1;
      assert!(verify_rs256(&n, &e, INPUT, &sig));
      assert!(!verify_rs256(&n, &e, INPUT, &bad));
      assert!(!verify_rs256(&n, &e, b"other", &sig));
      assert!(!verify_rs256(&n, &[3], INPUT, &sig));
      assert!(!verify_rs256(&[0x81; 1025], &e, INPUT, &sig));
      assert!(!verify_rs256(&[], &e, INPUT, &sig));
      assert!(!verify_rs256(&n, &[], INPUT, &sig));
  }
  ```
Proof: unit, `rs256_checks_rfc_7515_appendix_a2`: the RFC's signature
  verifies and each of six altered inputs is refused; under 20 ms. Unit
  only: nothing calls `verify_rs256` until skein 07.2's ID token check.
Don't: expose a ring or rustls type; generate or commit a private key.
Size: about 120 lines.
Removes: nothing.

### 05.2 The record's version 3
Scope: oauth.md 1, 2, 2.2, 5, 7; decision 8's codec half; decision 15.
Needs: nothing.
Files: `crates/skein-oauth/src` (`{record,documents,common}.rs`,
  `machine.rs`: `record()` and `refresh_state()`;
  `{tests,machine_tests}.rs`); `tests/oauth` (`src/lib.rs`,
  `tests/{world,memory,fuzzy_world}.rs`). The `Limits`, `SavedToken` and
  `TokenResponse` literals, in skein: `crates/skein-oauth-accounts/src`
  (`tests.rs`), `testing/skein-fake-peers` (`tests/oauth.rs`),
  `tests/oauth-accounts` (`src/{world,private,sign_in}.rs`,
  `src/sign_in/{issuer,protocol,cuts}.rs`, `tests/memory.rs`); in smith:
  smith-local-shell (`src/{local_host,credentials}.rs`,
  `tests/local_library.rs`), smith-local-protocol
  (`src/{tests,borrowed}.rs`), smith-local-service
  (`src/credentials/{tests,tests/private}.rs`), `benchmarks`
  (`src/credential.rs`), `tests/local-process/src/{world,process}.rs`;
  in jig: `crates/jig-protocol-config` (`src/profile/small.rs`),
  `crates/jig-protocol-accounts/src` (`tests/fixtures.rs`,
  `handed_in.rs`), `tests/protocol-accounts` (`src/fixtures.rs`).
Shape: `RECORD_VERSION` is 3. After version 2's fields, a version 3
  record writes the issued client id, the granted scopes, the subject
  and the ID token, each a presence byte then a length-prefixed field,
  then the earliest refresh time, a presence byte then `u64`
  nanoseconds; the presence byte keeps granted-nothing apart from
  as-requested. `decode_record` reads version 2 in place (the one second
  reader code-organization.md, 7 allows) with every addition `None`, and
  refuses any version but 2 and 3 as `Version`. The sketches give the
  new fields of `SavedToken`, `RefreshState` (`refresh_state()` fills
  it) and `TokenResponse`; `granted_scopes: None` means the requested
  scope. `decode_response` reads each, refuses one that occurs twice,
  and ignores an `earliest_refresh_at` that is not an unsigned number of
  seconds whose nanoseconds fit `u64`; `encode_response` writes each
  present one. `rotate`: the granted scopes are the response's `scope`,
  else the prior's; the issued client id, subject and ID token are the
  prior's (a refresh's `id_token` is not read); the earliest refresh
  time is the response's. The sign-in `record()` builds its prior with
  every addition `None`. `Limits` gains four caps (the sketches), each
  read and written within its cap and counted by `worst_case` and the
  record's measure; fixtures 64, 64, 64 and 1024, in that order;
  smallest valid 0 each. An issuer that answers with `scope` or
  `id_token` needs room for them whatever its terms: downstream, in the
  `Limits` literals above, `granted_bytes` takes the host's
  `scope_bytes`, `id_token_bytes` its `token_bytes`, the other two 0.
  `accounts::Limits` grows from 244 to 260 bytes, past the 256 clippy
  allows by value: `with_limits` (`tests/oauth-accounts`,
  `src/{world,sign_in}.rs`) takes `&accounts::Limits` and its callers
  (`sign_in.rs`, `sign_in/cuts.rs`) pass a reference.
  `every_account_holds_a_maximal_record_within_the_heap_bound`
  (`tests/memory.rs`) would pass 100 lines: its `SavedToken` literal
  moves to a `handed_in_record(account)` helper.
Test shape: unit, `src/tests.rs` (`limits()`, `record()`, byte literals
  written by hand), each under 20 ms:
  `a_record_round_trips_each_addition`, rows each addition alone, all
  five, granted empty, none;
  `a_version_two_record_reads_with_no_additions`;
  `a_version_three_record_reads_its_bytes`;
  `a_record_of_another_version_is_refused`, rows 1, 4;
  `a_token_response_reads_scope_id_token_and_earliest`;
  `an_unreadable_earliest_refresh_time_is_ignored`, rows a string, a
  negative, a fraction, past `u64` nanoseconds;
  `a_twice_named_addition_is_malformed`, rows each field;
  `each_new_cap_holds_at_its_boundary`, rows at the cap and one past,
  each cap. World, `tests/oauth/tests/world.rs` over
  `tests/oauth/src/lib.rs`, where this increment moves the OAuth worlds'
  fixtures (`issuer`, `sign_in`, `success`, `client_step`, `fake_step`,
  `record`) and limits (`documents()`, `client_limits()`,
  `fake_limits()`), each under 0.1 s:
  `a_refresh_naming_no_scope_keeps_the_granted_scopes`;
  `a_refresh_keeps_the_subject_and_id_token`;
  `a_version_two_record_refreshes_into_version_three`. Memory,
  `tests/oauth/tests/memory.rs`:
  `client_and_rotating_issuer_steps_fit_declared_heap_bounds`, every
  addition at its cap, against `worst_case`.
Proof: world, `a_version_two_record_refreshes_into_version_three`: a
  record written at version 2 by hand refreshes against the fake issuer
  and the saved record reads back at version 3 with the rotated refresh
  token; under 0.1 s. World is the highest tier here (decisions.md, 10).
Don't: add a reader beside `decode_record`; read a refresh's ID token;
  count an unreadable `earliest_refresh_at` as malformed.
Size: about 280 lines.
Removes: version 2 as the written version.

### 05.3 A registration's terms
Scope: oauth.md 1, 2, 2.2, 5.
Needs: 05.2's limits.
Files: `crates/skein-oauth/src` (`redirect.rs` new: the move below, then
  `Terms` and its types; `{machine,lib,machine_tests}.rs`);
  `tests/oauth` (`src/lib.rs`, `tests/world.rs`);
  `crates/skein-oauth-accounts/src` (`accounts.rs`,
  `component/sign_in.rs`). The `Registration` literals take `terms:
  None`, the `ClientLimits` literals 0, 0 and 0 (decision 15), in skein:
  `tests/oauth/tests/{memory,fuzzy_world}.rs`, `tests/oauth-accounts`
  (`src/{world,sign_in}.rs`, `tests/memory.rs`),
  `crates/skein-oauth-accounts/src` (`{tests,private_tests}.rs`); in
  smith: smith-local-shell (`src/credentials/{sign_in,product}.rs`),
  smith-local-service (`src/credentials/{tests,tests/private}.rs`),
  `tests/local-process/src` (`process/configuration.rs`); in jig:
  `crates/jig-protocol-config` (`src/profile/small.rs`),
  `crates/jig-protocol-accounts/src` (`tests/fixtures.rs`),
  `tests/protocol-accounts` (`src/fixtures.rs`),
  `testing/jig-test-service` (`src/accounts.rs`); downstream of jig
  (`Registration` only): `crates/temper` (`src/sign_in.rs`),
  `crates/temper-service` (`src/accounts.rs`).
Shape: first, in a commit of its own, `validate_registration` through
  `authorization_url` (`issuer_endpoint`, the port helpers,
  `redirect_address`: about 167 lines) and `form_fields`, `encoded_len`,
  `write_encoded` and `plain` (about 55) move unchanged into
  `redirect.rs`, with about 10 new lines (the module doc, its imports,
  `pub(crate)` on `form_fields`, which `token_body` and `json_fields`
  still call); `machine.rs` falls from 820 to about 600. Then
  `Registration.terms: Option<Terms>` and `Event::SignIn`'s `nonce` and
  `port` (the machine draws no randomness), as the sketches give them.
  `validate_registration` refuses, as `Malformed`, a terms URL (the
  issuer, the key set, the revocation endpoint) that `issuer_endpoint`
  refuses and OpenID whose scope lacks `openid`; as `Limit`, a terms URL
  past `uri_bytes` (as the token endpoint is), parameters past
  `parameter_bytes` and a resource past `resource_bytes`; and, on a
  sign-in, as `InvalidRedirect`, a varying port on a host other than
  `127.0.0.1` or `[::1]`. It admits port 0 or a preferred port in a
  varying redirect on `127.0.0.1` or `[::1]`, through the parse behind
  `redirect_address` (a flag admits 0); the driver binds the port (skein
  08.4). `redirect_address` still refuses 0, so `Component::new` refuses
  a port-0 varying redirect until 08.4. A sign-in refuses, as
  `InvalidRedirect`, a port missing or zero where the redirect varies,
  or present where it does not; and, as `Limit`, a nonce outside
  16..=`nonce_bytes` under OpenID, or one given without it. The bound
  port replaces the registration's in the URL, in `redirected()`'s exact
  comparison and in the exchange's `redirect_uri`. The URL carries,
  after OAuth's own fields, every added parameter (05.4 splits them by
  form), the resource and the nonce; the exchange and each refresh carry
  the resource. Until 07.2, the answer to an OpenID sign-in's code
  exchange ends `Failed { failure: Malformed }`, never `Tokens`.
  `ClientLimits` gains `parameter_bytes` (the added parameters' names
  and values, and the sign-in-again names, together), `resource_bytes`
  and `nonce_bytes`, counted by `client_worst_case`; fixtures 256, 128
  and 64; smallest valid 0 each. The driver: `copy_registration` copies
  `terms`; `component/sign_in.rs` sends `nonce: None, port: None`.
Test shape: unit, `machine_tests.rs` (`limits()`, `registration(public,
  wire)`, `state()`, `verifier()`, `step`, `failure`), each under 20 ms:
  `terms_are_refused_where_the_design_refuses`, rows OpenID without
  `openid`, a varying `localhost`, each terms URL on `http` off
  loopback, each cap one past;
  `a_registration_admits_port_zero_only_where_the_redirect_varies`, rows
  `127.0.0.1:0` and `[::1]:0` varying (admitted), `127.0.0.1:0` fixed
  (`InvalidRedirect`), `redirect_address` on each (`InvalidRedirect`);
  `a_varying_redirect_takes_the_bound_port`, rows `127.0.0.1` and
  `[::1]`, zero and preferred, the URL's `redirect_uri` reading back
  port 40000; `a_sign_in_refuses_a_port_the_redirect_does_not_take`,
  rows missing, zero, present on a fixed redirect;
  `a_nonce_is_bounded_and_needs_openid`, rows 15, 16, `nonce_bytes`, one
  past, without OpenID. World, `tests/oauth/tests/world.rs` over
  `tests/oauth/src/lib.rs`' fixtures, each under 0.1 s:
  `a_varying_redirect_refuses_another_ports_redirect`, the second port's
  redirect built by hand (the fake takes a varying port in skein 06.1);
  `the_resource_goes_in_every_request`;
  `an_openid_sign_in_fails_before_its_check_exists`.
Proof: world, `a_varying_redirect_refuses_another_ports_redirect`: a
  sign-in bound to port 40001, its redirect on 40002, ends `Failed {
  failure: InvalidRedirect }` with no exchange; under 0.1 s.
Don't: resolve or bind a name; admit `localhost` to vary; let a terms
  URL choose the token endpoint; return `Tokens` for an OpenID sign-in.
Size: about 240 lines, besides the move into `redirect.rs`.
Removes: nothing.

### 05.4 An issued client id, the hints, Incomplete
Scope: oauth.md 1, 2.1, 2.2, 4.
Needs: 05.3's terms.
Files: `crates/skein-oauth/src`
  (`{machine,redirect,record,common,machine_tests}.rs`); `tests/oauth`
  (`src/lib.rs`, `tests/world.rs`: its `record` helper moves to
  `lib.rs`; `tests/fuzzy_world.rs`: two `Tokens` arms);
  `crates/skein-oauth-accounts/src` (`component.rs`,
  `component/sign_in.rs`); jig's
  `crates/jig-protocol-accounts/src/translate.rs` (`fn exchange`).
Shape: `Event::SignIn` gains the held record's `issued_client_id`,
  `id_token` and `subject`. Under issued terms with no issued id, the
  URL carries the registration's client id and every added parameter;
  with one, the issued id, the parameters sent on every sign-in, then
  `id_token_hint=<the held ID token>` and `login_hint=<the held
  subject>`, each where one is held (oauth.md, 2.1; the design's default
  for `login_hint`). `Event::Redirected`'s `client_id` is read from a
  redirect whose state matched. Under issued terms, a missing or empty
  one ends the sign-in `Failed { failure: Incomplete }` (new), and one
  past `issued_client_bytes` `Failed { failure: Limit }`, each with no
  exchange, as a code past `code_bytes` does. The exchange, each refresh
  and (05.5) the revocation carry the issued id, and the record keeps
  the redirect's. A `Refresh` under issued terms whose prior has no
  issued id ends `Failed { failure: Refused }` with no request: a
  sign-in replaces it (oauth.md, 1). `Request::Tokens`'s `missing` names
  the required scopes absent from the granted ones (or the requested,
  where the response named none), and is empty when none is; the record
  is returned either way. An error whose name the terms list is
  `Refused` before `classify` runs, whatever its status. The driver:
  `component/sign_in.rs` sends the three as `None` and `client_id: None`
  until skein 08.4; `component.rs`'s `Tokens` arm reads
  `{ record, .. }`. Downstream: jig's match on `skein_oauth::Failure` in
  `translate.rs` gains `Incomplete` (`unreachable!`, like
  `InvalidRedirect`): no registration downstream has terms.
Test shape: unit, `machine_tests.rs` (`limits()`, `registration(public,
  wire)`, `answered`, `token_body`, `step`, `request`, `failure`), each
  under 20 ms: `each_form_sends_what_it_must`, rows a new registration
  (the entry point `client` and every added parameter, no hint) and
  signing in again (the issued `client-1`, the every-sign-in parameters,
  `id_token_hint=header.payload.signature`, `login_hint=user-42`), each
  URL's query compared whole, in the order of 05.3;
  `a_redirect_without_the_issued_id_is_incomplete`, rows missing, empty;
  `an_issued_client_id_is_bounded_by_its_cap`, rows
  `issued_client_bytes` (exchange made, carrying it), one past (`Limit`,
  no exchange); `every_later_request_carries_the_issued_id`, rows
  exchange and refresh;
  `a_refresh_without_an_issued_id_is_refused_unsent`;
  `an_added_error_name_signs_in_again`, rows each status 400, 401, 403,
  429, 500; `tokens_name_the_missing_required_scopes`, rows all granted,
  one declined, none named. World, `tests/oauth/tests/world.rs` over
  `tests/oauth/src/lib.rs`' fixtures (`error`, `success`), each under
  0.1 s: `an_added_error_name_on_a_refresh_signs_in_again`;
  `tokens_name_the_required_scopes_the_issuer_withheld`. The fake
  issues the id in skein 06.1.
Proof: world, `an_added_error_name_on_a_refresh_signs_in_again`: a
  refresh the fake answers 500 with an error name the terms list ends
  `Failed { failure: Refused }`, not `Unavailable`; under 0.1 s.
Don't: send the entry point's client id after an issued one; put the URL
  anywhere but `Visit`; enforce the required scopes.
Size: about 230 lines.
Removes: nothing.

### 05.5 Revocation
Scope: oauth.md 2.3, 3.
Needs: 05.2's record; 05.4's issued id.
Files: `crates/skein-oauth/src` (`machine.rs`, `machine_tests.rs`);
  `crates/skein-oauth-accounts/src/component.rs`.
Shape: `Event::Revoke` and `Request::Revoked` (new; the sketches give
  their fields). With a revocation endpoint, a form POST, whatever the
  registration's wire, with `token`, `token_type_hint=refresh_token`,
  the client id (the issued one under issued terms) and the secret where
  there is one, bounded by `request_bytes`; retried as any POST, after
  an unsent request or a 429; a 200, empty or not, is `Revoked {
  confirmed: true }`; any other answer, a spent budget or the deadline,
  `Revoked { confirmed: false }`; only `Cancel` gives `Failed`. With no
  endpoint, or under issued terms with no issued id, `Revoked {
  confirmed: false }` at once, with no `Http`. No new limit. The
  driver's `Request` match in `component.rs` gains a `Revoked` arm,
  unreachable until skein 09.1 sends a `Revoke`.
Test shape: unit, `machine_tests.rs` (`limits()`, `registration(public,
  wire)`, `answered`, `step`, `request`), each under 20 ms:
  `a_revocation_posts_its_form`, the fields read back, rows public and
  confidential, issued; `a_revocation_is_confirmed_only_by_a_200`, rows
  200 empty, 200 with a body, 400, 503, unsent then 200, budget spent,
  deadline; `a_revocation_without_an_endpoint_ends_unconfirmed_unsent`;
  `only_a_cancel_fails_a_revocation`.
Proof: unit, `a_revocation_is_confirmed_only_by_a_200`: a 200 gives
  `Revoked { confirmed: true }`; a 400, a 503, a spent budget and the
  deadline give `confirmed: false`; under 20 ms. Unit only: skein 06.1's
  fake issuer answers a revocation, 09.1's driver sends a `Revoke`.
Don't: revoke an access token; let `Revoked` carry a failure class;
  retry a revocation after an ambiguous send unless the caller asked.
Size: about 150 lines.
Removes: nothing.

## Interface sketches

```rust
// skein_tls (05.1)
/// An RS256 signature check by an RSA key of 2048 to 8192 bits.
#[must_use]
pub fn verify_rs256(modulus: &[u8], exponent: &[u8], message: &[u8], signature: &[u8]) -> bool { todo!() }
// skein_oauth (05.2)
pub const RECORD_VERSION: u16 = 3;
// Limits { .., issued_client_bytes, granted_bytes, subject_bytes, id_token_bytes: u32 }
// TokenResponse { .., scope, id_token: Option<Box<[u8]>>, earliest_refresh_at: Option<Wall> }
// SavedToken { .., issued_client_id, granted_scopes, subject, id_token: Option<Box<[u8]>>, earliest_refresh_at: Option<Wall> }
// RefreshState { .., issued_client_id, granted_scopes, subject, id_token: Option<Box<[u8]>> }
// skein_oauth (05.3): new public types, each deriving Clone, PartialEq, Eq, Hash and Debug (`Sent` also Copy)
// Terms { parameters: Box<[Parameter]>, openid: Option<OpenId>, issued_client_id: bool, varying_port: bool, resource, required_scopes, revocation_endpoint: Option<Box<[u8]>>, sign_in_again: Box<[Box<[u8]>]> }   (an issuer's terms beside a registration, oauth.md, 1)
// Parameter { name: Box<[u8]>, value: Box<[u8]>, sent: Sent }   (a parameter the authorization URL carries, and when)
// Sent { EverySignIn, NewRegistration }   (on every sign-in, or only on a new registration, oauth.md, 2.1)
// OpenId { issuer: Box<[u8]>, key_set: Option<Box<[u8]>> }   (the issuer an ID token comes from, and its key set's URL if configured)
// Registration { .., terms: Option<Terms> }
// ClientLimits { .., parameter_bytes, resource_bytes, nonce_bytes: u32 }
// Event::SignIn { .., nonce: Option<Box<[u8]>>, port: Option<u16>, issued_client_id, id_token, subject: Option<Box<[u8]>> }   (the last three 05.4)
// skein_oauth (05.4, 05.5)
// Event::Redirected { .., client_id: Option<Box<[u8]>> }
// Event::Revoke { registration: Registration, prior: RefreshState, now: Time }
// Request::Tokens { record: SavedToken, missing: Box<[u8]> }   (empty: none missing)
// Request::Revoked { confirmed: bool }
// Failure { .., Incomplete }
```

## Questions to raise when reached

- 05.2: if a downstream `record_bytes` cannot hold a version 3 record at
  its downstream caps (decision 15), name the fixture and the size.

## Done when

- `rg -c 'VERSION: u16 = 3' crates/skein-oauth/src/record.rs` prints 1.
- `rg '\b(ring|rustls)::' crates/skein-oauth/src` prints nothing.
- `rg -l -w 'fn authorization_url' crates/skein-oauth/src` prints only
  `crates/skein-oauth/src/redirect.rs`.
- `rg -c 'pub fn verify_rs256' crates/skein-tls/src/rs256.rs` prints 1.
- code-organization.md, 8's size and long-file commands, at each step's
  end: no file crosses 800 lines and none grows past about 100.
