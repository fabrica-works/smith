# skein 01: credentials

skein takes the MIT licence, and its LLM client takes a credential with
its scheme: a bearer or a key, each put where its route takes it, with
Codex's account id kept as the withdrawn form. A failure gains the
classes `Refused` and `Unsupported`, `Exhausted`'s reset goes optional,
and every failure carries the provider's report: its code, request id
and field. A call may carry no credential.

Needs at start: nothing (decision 17).

## Read first

- skein's design at `49725eca`: `llm.md` 2.5, 2.6, 4.2, 4.6, 4.7, 5, 7;
  `llm-connection.md` 3, 4, 4.2; `docs/foundation/programming-model.md`
  3, 4.4, 6.3, 7, 8, 9; `testing-strategy.md` 2.1, 2.5, 4.1, 6, 8, 10.
- The README's Rules; code-organization.md (owners, 1, 3, 4, 7, 8);
  decisions.md 2, 5, 6, 11, 13, 15, 16, 21 and design question 6.

## What the design asks

- llm.md 2.5: the two classes, the optional reset, the report.
- llm.md 2.6, design question 6: a value and its scheme; no `Debug`; a
  call may carry none and sends no credential field.
- llm.md 4.2, 7: the report's three values; 128 bytes; the worst case.
- llm.md 4.6, 4.7: each route's credential fields and request id.
- llm-connection.md 3, 4, 4.2: reserved names, bounds by name, the
  failed event's report, admission's `Invalid` refusals.

## Choices

- The names are decision 5's, the withdrawn forms decision 2's and
  admission's refusals decision 6's.
- The connection's stories of both schemes over one pool are skein
  04.4's. Each Shape names the downstream symbols its interface
  breaks: smith 01.3's repin repairs them, jig's and the downstream
  repository's the closing repins (decisions 13, 15, 16).

## Rules that bind it

- No credential value in a test's expected output, a `Debug` or a
  failure's detail: tests compare header names, routes and schemes
  (code-organization.md, 7).
- `client.rs` (1716), `openai/response.rs` (1307), `openai/tests.rs`
  (1569), the connection's `component.rs` (1026), `src/tests.rs` (815)
  and `tests/llm/tests/memory.rs` (844) grow by at most ~100 an
  increment; new stories go in new files.
- `Cargo.lock` is never merged by hand (development.md, 5.7). A name
  ending in `*` is the prefix of one test's name, cut to fit.

## Increments

### 01.1 The MIT licence
Scope: decision 11.
Needs: nothing.
Files: `LICENSE` (new); `Cargo.toml`; every member's `Cargo.toml`
  (`crates/*`, `tests/*`, `testing/*`, `examples/*`: 71 packages).
Shape: `LICENSE`'s first line is `MIT License`, then the MIT text with
  `Copyright (c) 2026 Free Ekanayaka`. The root `Cargo.toml` gains
  `license = "MIT"` under `[workspace.package]`, and each member gains
  `license.workspace = true` beside `edition.workspace = true`.
  Downstream: none; `Cargo.lock` does not change.
Test shape: none; the metadata query of the Proof is the check.
Proof: unit, no test fn (the increment adds no code): `cargo metadata
  --no-deps --format-version 1 | jq -r '.packages[] | select(.license
  != "MIT") | .name'` prints nothing, under 10 s; `heavy cargo check
  --workspace --all-targets` passes.
Don't: no `license` spelled per member, no `license-file`, no member
  left out, no `Cargo.lock` change. The root `Cargo.toml` is a hotspot
  with forge-kit: rebase, never merge by hand.
Size: about 95 lines.
Removes: nothing.

### 01.2 A credential and its scheme
Scope: llm.md 2.6, 4.6-4.7; llm-connection.md 3, 4.2; decisions 2, 5, 6.
Needs: nothing.
Files: `crates/skein-llm/src/{types,client,lib}.rs`, `derive/tests.rs`,
  `README.md`; `crates/skein-llm-connection/src/tests.rs`;
  `testing/skein-fake-llm-protocol/src/provider.rs`;
  `testing/skein-fake-peers/src/llm.rs`; `tests/llm/src/{lib,fake}.rs`;
  `tests/llm/tests/credentials.rs` (new); in `tests/llm/tests/`
  `{anthropic_admission,bounded_peer_memory,client,derive,fake,live,
  measure}.rs`; `tests/llm-connection/src/{world,simulated}.rs`; in
  `tests/llm-connection/tests/` `{failures,fake_protocol,fuzzy_close,
  memory,protocol,waiting}.rs`; `tests/tls/tests/fake_peer.rs`.
Shape: `Scheme { Bearer, Key }` (new, `types.rs`, re-exported).
  `Credential { value, scheme, account_id }` keeps its `#[expect]`;
  `Credential::bearer(value)`, `Credential::key(value)` and
  `Credential::codex(value, account_id)` (withdrawn) are new;
  `Credential::anthropic(value)` stays, a withdrawn bearer.
  `client::CredentialLimits { value, account_id }`; `Cap::Value`
  replaces `Cap::AccessToken`, its name `credential value`
  (`check_credential`, `request_head`). `headers`, `measure_headers`,
  `head_length` and `provider_headers` take the scheme: an Anthropic
  key writes `x-api-key` and `anthropic-version` `2023-06-01` unless
  the endpoint sets it, no `Authorization`, and a beta only from the
  endpoint's headers; a bearer writes `Authorization: Bearer`, the
  version and `anthropic-beta: oauth-2025-04-20`; Codex stays
  `Authorization` and `chatgpt-account-id`. `head_length` counts the
  field each scheme writes (an Anthropic key's head has 6 fields, a
  bearer's 7, before the endpoint's); `request_head` measures the
  larger. Admission's `Error::Invalid` adds a key on Codex;
  `Authorization`, `x-api-key` and `chatgpt-account-id` stay reserved.
  Doc comments: `Provider` names the dialect, not who pays;
  `Provider::OpenAiCodex`, `Endpoint::codex`, `Credential::codex`,
  `Credential::anthropic`, `account_id` and the bearer branch say
  withdrawn, naming the second part; `Endpoint::codex`'s doc is
  rewritten without the Pulse word. The README's prose on the field
  says value; its `jq` recipe for `.tokens.access_token` stays. The
  fake protocol and peer read `value` (their schemes are skein 04's).
  Downstream (smith; path and symbol; each a bearer by
  `Credential::codex` or `Credential::anthropic`): in
  `crates/smith-agent-service/`
  `src/service.rs` `credential`, `src/tests.rs`
  `an_admitted_opening_fits_*`, `src/profile/effective/tests.rs`
  `endpoints`; in `crates/smith-protocol-llm/src/` `grants.rs` `grant`,
  `framing.rs` `prompt_allowance`, `failure.rs` `cap` (the
  `Cap::AccessToken` arm); `crates/smith-agent-shell/src/config.rs`
  `credential_limits`; in `tests/protocol-llm/` `src/wire.rs`
  `configurations`, `src/pool_peer.rs` `for_provider`, and in `tests/`
  `root_memory.rs` `credential_bytes`, `production_contract.rs`
  `contract_schemas_reach_the_scripted_*`, `endpoints_grants.rs`
  `credential`, `component_peer.rs` `run`, `component_boundary.rs`
  `credential`, `client.rs` `two_calls`, `adapter_continuation.rs`
  `adopt`, `adapter_boundary.rs` `input`; `tests/agent/src/wire.rs`
  `configurations`; in `tests/agent-process/src/` `world.rs` `peer`,
  `lib.rs` `scenario_credential`, `fake.rs` `peer`;
  `tests/local-process/src/world.rs` `provider`; in `tests/end-to-end/`
  `end_to_end.rs` `standard_profile_runs_a_large_coding_*`, and in
  `support/` `mod.rs` `run_with_provider`, `exec.rs` `credential`,
  `live_run.rs` `run_fake`, `sample_host.rs` `run`.
  jig: `tests/one-process/src/fixtures.rs` `provider`; in
  `tests/channel/src/` `processes.rs` `window_provider`, `world.rs`
  `launch`; `testing/jig-test/tests/process.rs`
  `a_killed_jig_test_restarts_and_the_work_is_found`. The repository
  downstream of jig: in `tests/one-process/src/` `fixtures.rs`
  `provider`, `stories.rs` `provider`; in its application crate's
  `tests/support/` `mod.rs` `fake_llm`, `sign_in.rs` `llm`.
Test shape: world, `tests/llm/tests/credentials.rs`, through
  `World::new(call, limits, source, seed)` and `World.sent` with
  `call`, `limits` and `response` of `tests/llm/src/lib.rs` (bounds
  from `Limits::derive(&TINY_DECL)`), which gains the helper
  `fields_holding`, the names of the head's fields that hold the
  fixture's value, so no value is asserted. Admission goes by
  `client::Client::prepare`, as `anthropic_admission.rs`'s `rejection`
  does. Each under 0.1 s. `each_scheme_goes_where_its_route_takes_it`,
  rows Anthropic key (`x-api-key`; version; no `Authorization`, no
  beta), Anthropic key with an endpoint beta (that beta alone),
  Anthropic bearer (`Authorization`, version, OAuth beta), Codex
  (`Authorization`, `chatgpt-account-id`);
  `a_credential_its_route_refuses_is_invalid`, rows key on Codex,
  Codex without an id, Anthropic with an id;
  `an_endpoint_header_naming_a_credential_field_is_refused`, rows each
  reserved name on each route; `a_key_head_at_its_bound_is_admitted`,
  rows the measured head and one byte less, 6 fields and 5. Units in
  `skein-llm-connection/src/tests.rs`, each under 20 ms, with its
  `endpoint()` and `credential()`:
  `a_key_on_the_codex_route_is_refused_at_entrance` (`Event::Refused`,
  `Refusal::Client(Invalid)`, no terminal);
  `a_value_past_its_bound_is_refused_by_name` (`Cap::Value`);
  `either_scheme_at_its_bound_fits_the_measured_head`. Reference test:
  ```rust
  #[test]
  fn each_scheme_goes_where_its_route_takes_it() {
      let mut input = call(1);
      input.endpoint = Endpoint::anthropic();
      input.credential = Credential::key(b"secret-test-token".to_vec().into());
      input.prompt.affinity = None;
      let mut world = World::new(input, limits(), response(401, "", b"{}", false), 1);
      world.request(client::Request::Start);
      world.run();
      assert_eq!(fields_holding(&world.sent, b"secret-test-token"), ["x-api-key"]);
  }
  ```
Proof: world, `each_scheme_goes_where_its_route_takes_it`: the key is
  in `x-api-key` alone and a bearer in `Authorization` alone; under
  0.1 s. Nothing is higher: skein 04.4 adds the connection's stories.
Don't: no `Debug` on `Credential`; no value in an assertion; no second
  credential type; no optional `account_id`; no withdrawn form removed.
Size: about 280 lines, about 60 of them the rename.
Removes: `access_token` and `Cap::AccessToken`.

### 01.3 The failure classes and the report
Scope: llm.md 2.5 (the classes, `Report`); llm-connection.md 3, 4.
Needs: nothing.
Files: `crates/skein-llm/src/{types,lib,translate,dialect,client}.rs`,
  `openai/{common,response,tests}.rs`, `anthropic/response.rs`;
  `crates/skein-llm-connection/src/{boundary,call,component}.rs`;
  `tests/llm/tests/` `failures.rs` (new), `client.rs`, `anthropic.rs`.
Shape: `skein_llm::Failure` and the native `openai::Failure` gain
  `Refused` and `Unsupported`, and `Exhausted { retry_after:
  Option<Duration> }`. Each exhaustive match takes the arms
  (`translate.rs` `failure`; `client.rs` `fail` and `abort`; `fail` of
  `openai/response.rs` and `anthropic/response.rs`), and the rows that
  build `Exhausted` take `Some(..)`: in `tests/llm/tests/` `client.rs`
  `error_within_sse_stream_is_terminal_and_uses_headers` and
  `anthropic.rs` `nested_provider_errors_classify_http_and_sse_*`, and
  in `openai/tests.rs` `server_events_roundtrip_*` and
  `incomplete_and_refused_terminals_*`.
  `RateLimit::reset(self, error, wall) -> Option<Duration>` (new) keeps
  `delay`'s order and gives `None` when nothing gives a reset;
  `limited` builds `Exhausted` from it, and `RateLimited` keeps
  `delay`. Nothing raises the two classes yet (skein 02). `Report`
  (new, `types.rs`, re-exported; `Report::none()` is the empty one,
  with no `Default`) rides `dialect::Output::Failed { failure, report,
  detail }` (native `openai::Output::Failed` and
  `anthropic::Output::Failed` stay `{ failure, detail }`),
  `client::Event::Failed { owner, failure, evidence, report, detail }`
  and the connection's `Event::Failed { call, failure, evidence,
  report, detail, measures }` (`call.rs` `route`; `component.rs`
  `fire_event`), empty until 01.4.
  Downstream (smith): `crates/smith-protocol-llm/src/` `failure.rs`
  `failed` (the class match), `component.rs` `route` (the failed
  event); `tests/protocol-llm/src/wire.rs` `take`;
  `tests/agent/src/wire.rs` `take`; in `tests/protocol-llm/tests/`
  `adapter_continuation.rs` `observe_failure`, `adapter_boundary.rs`
  `failed` and `failed_owner_class_evidence_*`, `component_boundary.rs`
  `every_shared_failure_class_keeps_each_evidence_and_detail`. smith
  01.3 maps the two classes onto `Invalid` until smith 02.1 (decision
  20). jig and the repository downstream of jig: none.
Test shape: world, `tests/llm/tests/failures.rs`, with `World`,
  `call`, `limits` and `response` of `tests/llm/src/lib.rs`, as
  `tests/llm/tests/client.rs` does in
  `provider_error_body_end_and_http_done_share_one_terminal`; each
  under 0.1 s. `exhausted_without_a_reset_carries_none_never_zero`,
  rows Codex `usage_limit_reached` with no reset field (`None`), with
  `resets_in_seconds` (`Some`), Anthropic `rejected` without
  `anthropic-ratelimit-unified-reset` (`None`), with it (`Some`). Unit
  in `openai/tests.rs`, under 20 ms:
  `a_reset_comes_from_the_first_field_given`, rows per field and none.
  The reference test, two rows:
  ```rust
  #[test]
  fn exhausted_without_a_reset_carries_none_never_zero() {
      let body = |extra: &str| format!(r#"{{"error":{{"code":"usage_limit_reached"{extra}}}}}"#);
      for (extra, expected) in [("", None), (r#","resets_in_seconds":90"#, Some(Duration::from_secs(90)))] {
          let source = response(429, "Content-Type: application/json\r\n", body(extra).as_bytes(), false);
          let mut world = World::new(call(1), limits(), source, 1);
          world.request(client::Request::Start);
          world.run();
          world.assert_once();
          assert!(world.seen.iter().any(|e| matches!(e, client::Event::Failed {
              failure: Failure::Exhausted { retry_after }, .. } if *retry_after == expected)));
      }
  }
  ```
Proof: world, `exhausted_without_a_reset_carries_none_never_zero`: a
  Codex limit with no reset field fails `Exhausted` with `None`, and
  with `resets_in_seconds` with `Some`; under 0.1 s. Nothing is
  higher: 01.4 adds the connection's story.
Don't: no zero for an unknown reset; no report field filled here; no
  `Option<Report>`; no class raised before skein 02.
Size: about 200 lines.
Removes: `Exhausted`'s required reset.

### 01.4 The report's values
Scope: llm.md 2.5, 4.2, 4.6, 4.7, 7 (the report's values and bound).
Needs: 01.3's `Report`.
Files: `crates/skein-llm/src/{client,dialect}.rs`,
  `openai/{common,response,filter,tests}.rs`;
  `crates/skein-llm-connection/src/tests.rs`;
  `testing/skein-fake-llm-protocol/src/provider.rs`;
  `tests/llm/tests/failures.rs`, `tests/llm/tests/memory.rs`;
  `tests/llm-connection/tests/failures.rs`.
Shape: `client::REPORT_BYTES: u32 = 128` (new), one for every route.
  `response_head`, and `fail`'s loop over queued heads, keep the
  request id on the client: `x-oai-request-id` on Codex, `request-id`
  on Anthropic; so every failure after a head, an abort included,
  reports it. The code is `ProviderError.kind` (its code, else its
  type, as `read_error` reads it), set by `dialect.rs`
  `provider_error`. `ProviderError.param` (new) is read from
  `error.param`, added to `filter.rs`'s error fields; its literals take
  `param: Box::new([])`: three in `openai/tests.rs`
  (`server_events_roundtrip_*`, `incomplete_and_refused_terminals_*`,
  `an_http_error_body_refines_*`) and both in the fake protocol's
  `provider.rs` `response_error` (`anthropic::ProviderError` is the
  same type); Anthropic's field stays empty. A value past
  `REPORT_BYTES` is left empty, never clipped. A failure before a head
  has an empty report; a local one after it (a limit, a timeout, a
  malformed stream, the owner's abort) the request id alone, whether
  or not the body decoded. `worst_case` adds `3 * REPORT_BYTES`.
  Downstream: a memory ceiling fixed in a test moves by 384 bytes a
  call; each repin's gate checks it again (decision 15).
Test shape: world, in `tests/llm/tests/failures.rs` with 01.3's
  helpers and reference test; each under 0.1 s:
  `each_routes_request_id_reaches_the_report`, rows Codex and
  Anthropic, with and without a decodable body;
  `a_report_value_past_its_bound_is_empty`, rows code, request id,
  field (Codex), each at 128 kept and at 129 empty;
  `a_failure_before_any_head_has_an_empty_report`;
  `a_local_failure_after_a_head_keeps_the_request_id`, rows `Limit`,
  `TimedOut`, `Protocol`, an abort. In `memory.rs`, with its counting
  allocator and `client::call_worst_case`:
  `a_full_report_stays_within_the_call_worst_case`, the peak against
  the worst case. World, `tests/llm-connection/tests/failures.rs`, with
  its `World` and `assert_failure`, under 0.5 s (a real component and a
  byte peer): `a_provider_failure_reaches_the_owner_with_its_report`.
  Unit in `skein-llm-connection/src/tests.rs`, under 20 ms:
  `a_whole_deadline_wait_fails_with_an_empty_report`.
Proof: world, `a_provider_failure_reaches_the_owner_with_its_report`:
  the owner's `Event::Failed` carries the code, the request id and the
  field; under 0.5 s. Nothing is higher in skein (smith 01.3 repins).
Don't: no clipped value; no request id from the body; no budget raised;
  no expected bytes computed by the client.
Size: about 280 lines.
Removes: nothing.

### 01.5 A call with no credential
Scope: llm-connection.md 4.2; llm.md 2.6; design question 6.
Needs: 01.2's `Credential`.
Files: `crates/skein-llm/src/{types,client}.rs`, `derive/tests.rs`;
  `crates/skein-llm-connection/src/{boundary,component,tests}.rs`;
  `tests/llm/src/{lib,fake}.rs`; `tests/llm/tests/credentials.rs`; in
  `tests/llm/tests/` `{memory,measure,bounded_peer_memory,fake_memory,
  fake,client,live,derive,archives,raw_call_history,replay,selective,
  prepared_raw,meaning,fuzzy_client,anthropic,anthropic_admission}.rs`;
  `tests/llm-connection/src/{world,simulated}.rs`; in
  `tests/llm-connection/tests/` `{waiting,protocol,fake_protocol,
  memory,failures,fuzzy_close}.rs`;
  `testing/skein-fake-peers/tests/llm.rs`.
Shape: `Call.credential` and `Request::Start.credential` are
  `Option<Credential>`, as are `admit`, `admit_with_reasoning_drop`,
  `admit_policy`, `measure_start`, `Waiting` and the second argument of
  `client::measure(prompt, credential, endpoint, limits)`.
  `check_credential` is skipped for none. With none, Codex is `Invalid`
  at the entrance; Anthropic sends the version alone, with no beta, no
  `x-api-key` and no `Authorization`; `head_length` leaves out the
  credential's field. `request_head` refuses a zero bound on the value
  as `Cap::Value`, whether or not the call carries one (decision 21).
  The fake protocol's `provider.rs` and the fake peer's `llm.rs` keep
  `&Credential`; `tests/llm/src/lib.rs` gains `credential_of`, giving
  the `&Credential` of a `&Call`, so a fake peer and a measure call
  unwrap the fixture's `Some`; `make` of the fake peers' `tests/llm.rs`
  does so with `expect`, as `Peer::new` takes a `Credential`.
  `limits.rs` is unchanged.
  Downstream (smith), each `Some(..)`: in
  `crates/smith-protocol-llm/src/` `component.rs` `start` (the
  `Request::Start` build) and `observed_prompt` (`client::measure`),
  `guard.rs` `measure`, `framing.rs` `prompt_allowance`;
  `crates/smith-agent-shell/src/config.rs`
  `native_validated_dense_prompts_*` (`client::measure`);
  `crates/smith-agent-service/src/tests.rs`
  `an_admitted_opening_fits_*`; `tests/protocol-llm/src/adapter.rs`
  `prepare` (the `Call` literal); `tests/agent-process/tests/memory.rs`
  `full_pool`. jig and the repository downstream of jig: none.
Test shape: world, `tests/llm/tests/credentials.rs`, with 01.2's
  helpers and `credential_of`; each under 0.1 s:
  `a_call_without_a_credential_sends_no_credential_field`, rows
  Anthropic (the version alone) and Codex (refused, `Invalid`);
  `a_zero_value_bound_is_refused_as_a_value_cap`, rows a call with a
  credential and a call with none. Unit in
  `skein-llm-connection/src/tests.rs`, under 20 ms:
  `a_start_without_a_credential_on_codex_is_refused`.
Proof: world, `a_call_without_a_credential_sends_no_credential_field`:
  Anthropic with none sends the version alone, Codex is refused as
  `Invalid`; under 0.1 s. Nothing is higher: the entrance has a unit.
Don't: no empty value for none; no default credential; no beta
  without a bearer.
Size: about 160 lines.
Removes: nothing.

## Interface sketches

```rust
// crates/skein-llm/src/types.rs
/// A value with its scheme; `account_id` is Codex's, withdrawn. No derives: it keeps its `#[expect(missing_debug_implementations)]`.
pub struct Credential { pub value: Box<[u8]>, pub scheme: Scheme, pub account_id: Box<[u8]> }
/// The provider's evidence of a failure; a value is empty if not given.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Report { pub code: Box<[u8]>, pub request: Box<[u8]>, pub field: Box<[u8]> }
impl Report { #[must_use] pub fn none() -> Report { todo!() } } // `Default` is disallowed

// crates/skein-llm/src/client.rs
pub fn measure(prompt: &Prompt, credential: Option<&Credential>, endpoint: &Endpoint,
    limits: &Limits) -> Result<Measured, Error> { todo!() }
// openai/common.rs: none when nothing gives a reset, never zero.
impl RateLimit { #[must_use] pub fn reset(self, error: Option<&ProviderError>, wall: Wall) -> Option<Duration> { todo!() } }
```

## Questions to raise when reached

None: decision 21's readings are folded into 01.4 and 01.5.

## Done when

- `cargo metadata --no-deps --format-version 1 | jq -r '.packages[] |
  select(.license != "MIT") | .name'` prints nothing, and `head -1
  LICENSE` prints `MIT License`.
- skein's `scripts/check.sh` passes on `origin/main`.
- `rg -ohw -e 'fn each_scheme_goes_where_its_route_takes_it'
  -e 'fn exhausted_without_a_reset_carries_none_never_zero'
  -e 'fn a_report_value_past_its_bound_is_empty'
  -e 'fn a_call_without_a_credential_sends_no_credential_field'
  tests/llm/tests | sort -u | wc -l` prints 4.
- `access_token` survives in `crates/skein-llm/README.md` alone (the
  `jq` recipe for `.tokens.access_token`): `rg -lw
  'access_token|AccessToken' crates/skein-llm
  crates/skein-llm-connection tests/llm tests/llm-connection tests/tls
  testing/skein-fake-llm-protocol testing/skein-fake-peers` prints it.
- code-organization.md 8's size count flags no file of this session.
