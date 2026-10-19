# skein 02: the public route

skein's client classes a provider's failure by its code first and by its
status where the code says nothing, on the Codex and Anthropic routes.
It gains OpenAI's public Responses route, `Provider::OpenAi`, called
with a key or a plan's token alike as a bearer, with its own limits,
replay tag and request id, and llm.md 4.8's code table, row by row.

Needs at start: skein 01's credential with its `Scheme` (01.2) on
skein's `origin/main`; 02.2 waits for 01's failure classes and report
values (01.3, 01.4).

## Read first

- skein's design at `49725eca`: `llm.md`, the sections named below.
- skein `docs/foundation/programming-model.md` 3, 4.4, 6.3, 8, 9;
  `testing-strategy.md` 2.1, 2.5, 4.1, 6, 8, 10.
- The README's Rules; code-organization.md (owners, 1, 3, 4, 7, 8);
  decisions.md 2, 4, 9, 10, 12, 13, 16, 20 and design question 9.

## What the design asks

- llm.md 1, 4: the public route is a provider of the client.
- llm.md 2.5: each route maps its error's code first, then the status.
- llm.md 2.6: a key and a plan's token both go as a bearer.
- llm.md 4.2, 7: the head bound is the floor; the error body 16 KiB.
- llm.md 4.6, 4.7: Codex's and Anthropic's failures, code or type first.
- llm.md 4.8: the request, its terminal events, request id and codes.
- llm.md 5: affinity in the body, the classes, the table's cases.

## Choices

- Replay tags: 1 Codex, 2 Anthropic, 3 the public route; the tags
  written before still read the same (decision 4).
- The public route's codes are the design's; its cases are synthetic and
  labelled so; no live suite (decisions 9, 10).
- `Codex` keeps its route and its account id, withdrawn (decision 2); it
  takes the public route's table for the codes it does not name itself.
- smith 01.3 maps `Refused` and `Unsupported` onto `Invalid` until smith
  02.1 (decision 20); each Shape names the symbols it breaks (decision
  13).

## Rules that bind it

- A test's expected class, code and field are literals of the design's
  tables; none comes from `classify`.
- `client.rs` (1744 lines at skein 9fbda6b), `openai/response.rs` (1307), `translate.rs`
  (1291) and `openai/tests.rs` (1569) grow by at most ~100 an increment;
  the worlds go in new files.
- No credential value in a test's output (code-organization.md, 7): a
  key's and a token's requests are compared by body bytes and by the
  names of the fields that hold the value (`fields_holding`, 01.2).
- A trailing `*` in a test's name is a prefix of that one test's name,
  cut to fit 72 columns.

## Increments

### 02.1 The route
Scope: llm.md 1, 4, 4.8 (the route), 7 (its bounds); decision 4.
Needs: skein 01's credential with its `Scheme` (01.2).
Files: `crates/skein-llm/src/{types,derive,dialect,translate,replay,
  client}.rs`, `derive/tests.rs`, `README.md`; `tests/llm/src/fake.rs`;
  `tests/llm/tests/{replay,derive,credentials,fake,fuzzy_client,
  live}.rs`; `tests/llm-connection/tests/fake_protocol.rs`.
Shape: `Provider::OpenAi` (new): the public Responses route.
  `Endpoint::openai()` (new). `derive::Limits` gains `openai`, from
  `const OPENAI` (new): Codex's decoder constants, the head at the
  floor, 16 KiB and 128 fields, the error body 16 KiB;
  `Limits::dialect` gives it. No test builds `derive::Limits` by its
  fields: each takes `openai` from `Limits::derive`.
  `dialect::Decoder::Codex` becomes `Decoder::Responses(Provider,
  openai::StreamDecoder)`, and `translate.rs` `part` and `replay` take
  the route, so a block's replay names it (both write `OpenAiCodex`
  for every block today). `Replay::to_bytes` writes tag 3 for `OpenAi`
  and `from_bytes` reads 3; neither OpenAI route takes the other's
  replay. `types.rs` `output_ceiling` gives `None`; `dialect.rs`
  `event_filter` gives Responses'; the media type is strict, Codex's
  leniency for a missing `Content-Type` staying Codex's. Every
  `Provider` match takes the arm: `derive.rs` `dialect`; in `client.rs`
  `request_head`, `measure`, `prepare_with_reasoning_drop`,
  `measure_headers` and `provider_headers`; in `derive/tests.rs`
  `owner_declared_counts_*`; `tests/llm/src/fake.rs`
  `prepared_with_options` and `delayed`; in `tests/llm/tests/`
  `derive.rs` `capture` and
  `archived_heads_fit_with_headroom_and_complete_with_derived_limits`,
  `fake.rs` `input`, `check_feedback`,
  `absent_affinity_sends_none_and_anthropic_ignores_it_on_the_actual_wire`,
  `peer_native_request` and the two of
  `growing_conversations_preserve_native_content_and_encode_twice_identically`,
  `fuzzy_client.rs` (its documents match), `live.rs` `input`; and
  `tests/llm-connection/tests/fake_protocol.rs` `run`. Each loop over
  the routes keeps its two until 02.3, and so do `translate.rs`
  `measure_request` and `encode_request`, through `validate_codex`'s
  `OpenAiCodex`, and `charge_replay_item`'s `== Provider::OpenAiCodex`:
  no `OpenAi` call is admitted before 02.3, and `live.rs` has no suite for
  the route (decision 10). The README lists the route.
  Transitional, ended by 02.3: admission refuses a call on `OpenAi` as
  `Error::Unsupported`. `heavy cargo check --workspace --all-targets`
  ends clean.
  Downstream (smith), each exhaustive `Provider` match, by path and
  symbol: `crates/smith-protocol-llm/src/framing.rs` `prompt_allowance`;
  `crates/smith-local-shell/src/credentials/product.rs` `codex`;
  `tests/protocol-llm/src/pool_peer.rs` `for_provider`; in
  `tests/protocol-llm/tests/` `client.rs` (the test of the run key and
  thread for Codex alone), `guard.rs` `measure`, `endpoints_grants.rs`
  `destination`, `history_window.rs` `history_edge` and
  `component_peer.rs` `run`. jig and temper: none.
Test shape: world, `tests/llm/tests/replay.rs`, with `call` and
  `limits` of `tests/llm/src/lib.rs`, each under 0.1 s:
  `each_route_replays_only_its_own_metadata`, rows tag 1, 2, 3 written
  and read back (the rows refusing each OpenAI route's replay on the
  other are 02.3's: until it, admission refuses every `OpenAi` call as
  `Unsupported`, which no such row can tell from the refusal);
  `a_replay_written_before_reads_the_same`, literal bytes of tags 1
  and 2. In `derive.rs`, a row for `OpenAi` in
  `declared_quantities_price_the_retained_event_*` (head 16384, 128
  fields, 30 tokens, 196 text, 86 metadata). In `credentials.rs`,
  `a_public_call_is_refused_as_unsupported` (removed by 02.3), through
  `Client::prepare` as `anthropic_admission.rs`'s `rejection` does.
  Unit in `derive/tests.rs`, under 20 ms:
  `the_public_route_takes_the_head_floor`, the head, fields and error
  body of `Limits::derive(&DECLARED).dialect(Provider::OpenAi)`.
Proof: world, `each_route_replays_only_its_own_metadata`: the tag byte
  reads 1, 2 and 3 for the three routes; under 0.1 s; unit,
  `the_public_route_takes_the_head_floor`: the head is 16 KiB with 128
  fields; under 20 ms. Nothing sends on the route until 02.3, so no
  higher layer reaches it.
Don't: no second Responses decoder; no tag renumbered; no head bound
  above the floor; no Codex header on the new route.
Size: about 230 lines.
Removes: `Decoder::Codex`.

### 02.2 The code first on the Codex and Anthropic routes
Scope: llm.md 2.5 (by code first), 4.6 (failures), 4.7 (failures by
  type); design question 9.
Needs: skein 01's failure classes (`Refused`, `Unsupported`, the
  optional reset) and its report values (01.4).
Files: `crates/skein-llm/src/{dialect,client}.rs`,
  `openai/{common,response,tests}.rs`, `anthropic/response.rs`;
  `tests/llm/tests/` `classes.rs` (new), `client.rs`, `anthropic.rs`.
Shape: `openai::classify` takes the provider: `classify(provider,
  status, error, rate, wall)`, called by `client.rs` `error_end` and
  `error_cut`, `dialect.rs` `provider_error`, and `accept` of
  `openai/response.rs` and `anthropic/response.rs`. The native
  `openai::StreamDecoder` keeps its route: `new` and
  `with_reasoning_drop` take the `Provider` first, held in a field its
  `accept` passes to `classify`; their callers are `dialect.rs`
  `Decoder::new` and `openai/tests.rs`'s five. On Anthropic the
  error's type decides first, as llm.md 4.7's list; `rejected` makes a
  `rate_limit_error` `Exhausted`, its reset from the unified reset,
  else `None`; `billing_error` is `Exhausted { retry_after: None }`; a
  type it does not name falls to the status. On Codex
  `usage_limit_reached` is `Exhausted`, its reset from
  `resets_in_seconds`, else `resets_at`, else `None`, never
  `retry-after`; another 429 is `RateLimited` with `delay`. Mid-stream
  (status 0 or 2xx) a named code keeps its class and another is
  `Unavailable`, its code in the report. A mid-stream failure is
  classed once, by the path that reaches it: `provider_error` on the
  client's path, with the head's status and rate, and the
  `Event::Failed` arm of each `accept` on a decoder driven alone; both
  call the one `classify`. The status fallback is llm.md 4.8's, shared
  by both OpenAI routes.
  Rows that read `Retry-After` for a limit change: in
  `server_events_roundtrip_*` of `openai/tests.rs`, `Retry-After: 2`
  no longer beats `resets_in_seconds` (60); in `tests/llm/tests/`
  `client.rs`, `error_within_sse_stream_is_terminal_and_uses_headers`
  expects `None`; in `anthropic.rs`,
  `nested_provider_errors_classify_http_and_sse_with_retry_headers`
  expects `None` for its `rejected` row, which has no unified reset.
  Downstream: none at compile time; smith meets `Refused` (an
  Anthropic 403), `Exhausted` with no reset (a 402) and a Codex reset
  that ignores `retry-after`, mapped by smith 01.3 (decision 20).
Test shape: world, `tests/llm/tests/classes.rs`, each an HTTP error
  through `World`, with `call`, `limits`, `limits_for` and `response`
  of `tests/llm/src/lib.rs` (bounds from `Limits::derive(&TINY_DECL)`)
  and an `anthropic_call` of its own, as `anthropic.rs` has; each under
  0.1 s. `anthropic_failures_are_classed_by_type_first`, rows
  `permission_error` 403 (`Refused`), `billing_error` 402
  (`Exhausted`, `None`), `rate_limit_error` 429 `rejected` with a reset
  (`Some`) and without (`None`), 429 not rejected (`RateLimited`),
  `overloaded_error` 529, `api_error` 500, "prompt is too long" 400,
  `request_too_large` 413, `invalid_request_error` 400 naming a balance
  (`Invalid`), `authentication_error` 401; each with its report's code
  and an empty field. `codex_failures_are_classed_by_code_first`, rows
  `usage_limit_reached` 429 with `resets_in_seconds`, with `resets_at`
  at a set wall, with `retry-after` alone (`None`),
  `rate_limit_exceeded` 429 with `retry-after` (`RateLimited`).
  `a_mid_stream_code_keeps_its_class_or_is_unavailable`, rows Codex
  `usage_limit_reached` in `response.failed`, Anthropic
  `overloaded_error` event, an unnamed code on each (`Unavailable`,
  the code kept). Units in `openai/tests.rs`, under 20 ms: the table of
  `an_http_error_body_refines_class_*` takes the provider in each row,
  an Anthropic row following its type before its status. Reference
  test, two of the Codex rows:
  ```rust
  #[test]
  fn codex_failures_are_classed_by_code_first() {
      for (code, expected) in [
          ("usage_limit_reached", Failure::Exhausted { retry_after: None }),
          ("rate_limit_exceeded", Failure::RateLimited { retry_after: Duration::from_secs(9) }),
      ] {
          let body = format!(r#"{{"error":{{"code":"{code}"}}}}"#);
          let mut world = World::new(call(1), limits(), response(429, "Retry-After: 9\r\n", body.as_bytes(), false), 1);
          world.request(client::Request::Start);
          world.run();
          assert!(world.seen.iter().any(|e| matches!(e, client::Event::Failed { failure, .. } if *failure == expected)));
      }
  }
  ```
Proof: world, `codex_failures_are_classed_by_code_first`: a 429
  `usage_limit_reached` with `Retry-After` alone fails `Exhausted` with
  `None`, and `rate_limit_exceeded` fails `RateLimited` with that
  delay; under 0.1 s. Nothing is higher until skein 04's stories.
Don't: no `retry-after` for `usage_limit_reached`'s `Exhausted` (02.4's
  `subscription_sharing_usage_limit_exceeded` reads it last); no zero
  reset; no field on an Anthropic report; no status read before a code
  the route names.
Size: about 260 lines.
Removes: today's single `classify` without the provider, and
  `retry-after` as `usage_limit_reached`'s reset.

### 02.3 The public request
Scope: llm.md 2.6 (either scheme as a bearer), 4.8 (the request,
  affinity, terminal events, request id), 5 (affinity, credentials).
Needs: 02.1's route; 02.2's code-first `classify` (the code table is
  02.4's); skein 01's call with no credential (01.5) and its report
  values (01.4).
Files: `crates/skein-llm/src/{client,translate,dialect}.rs`,
  `openai/{response,identity}.rs`; `tests/llm/src/lib.rs`;
  `tests/llm/tests/` `public_route.rs` (new), `credentials.rs`,
  `replay.rs`.
Shape: `provider_headers` on `OpenAi`, and in `client.rs` `headers`
  and `head_length`, which choose `x-api-key` or `Authorization:
  Bearer` from `Credential.scheme` since 01.2 (`measure_headers`
  refuses `Scheme::Key` on Codex alone): `Authorization: Bearer` for
  either scheme, none without a credential, no `chatgpt-account-id`, no
  `session-id` or `thread-id`; affinity renders in the body's
  `prompt_cache_key` alone. Admission refuses an account id as
  `Invalid`, and an explicit output cap as `Error::Unsupported`
  (`validate_codex`'s rule, on both OpenAI routes). The body is the
  Responses body Codex sends: `store:false`, `stream:true`, the whole
  context in `input`, `tool_choice` `"auto"` for `Auto` and `Only`,
  `"none"` for `None`, `parallel_tool_calls` true. The decoder ends a
  call on `response.completed` (`EndTurn`), `response.incomplete`
  (`MaxTokens` or `Refusal`), `response.failed` or `error` (by code,
  02.2); `response.done` stays Codex's; a stream ending without one is
  `Protocol`. The request id is the head's `x-request-id` (llm.md 4.8),
  read by `response_head` on `OpenAi` beside 01.4's `x-oai-request-id`,
  through a constant beside 01.4's; `openai::identity::REQUEST_HEADER`,
  which holds that value, is read nowhere, and sits in Codex's captured
  identity, goes. `translate.rs` `measure_request` and `encode_request`
  take the route: `validate_codex` validates against it, and
  `charge_replay_item` counts a reasoning item on either OpenAI route
  (llm.md: "Responses writes a reasoning item for each segment of its
  reasoning"). Neither OpenAI route takes the other's replay. If the draw below
  finds a field of the refused list in Codex's body, the field goes
  from both routes' bodies. `tests/llm/src/lib.rs` gains
  `public_call(owner)`: `call` on `Endpoint::openai()` with a bearer
  (`Some`, 01.5's optional credential) and no account id. Downstream: none.
Test shape: world, `tests/llm/tests/public_route.rs`, its cases labelled
  synthetic, with `World`, `public_call`, `limits_for(Provider::OpenAi)`
  (bounds from `Limits::derive(&TINY_DECL)`), `response` and
  `fields_holding`; each under 0.1 s.
  `a_key_and_a_token_send_byte_identical_bodies` (bodies equal, the
  value under `Authorization` in both);
  `the_public_route_sends_affinity_in_the_body_alone`, rows with and
  without affinity;
  `a_call_without_a_credential_sends_no_authorization`;
  `no_drawn_prompt_carries_a_refused_field`, prompts drawn by
  `skein_lib::Rng` as `fuzzy_client.rs` draws, the body's keys read as
  `tool_identities.rs` reads them, each with `store:false` and
  `stream:true`, none of llm.md 4.8's refused list (a literal) or
  `previous_response_id`; `an_explicit_output_cap_is_unsupported`;
  `each_terminal_event_ends_the_call_as_the_route_says`, rows
  completed, incomplete for its output, incomplete for content,
  `response.done` (`Protocol`), no terminal (`Protocol`);
  `the_public_request_id_reaches_the_report`. In `credentials.rs`,
  `input` empties the account id on every route but `OpenAiCodex`
  (`provider != Provider::OpenAiCodex`, where it reads `==
  Provider::Anthropic`), so a public row carries none, and rows
  for the public route join `a_credential_its_route_refuses_is_invalid`
  (an account id) and
  `an_endpoint_header_naming_a_credential_field_is_refused`
  (`session-id`, `thread-id`, `Authorization`, `x-api-key` and
  `chatgpt-account-id`). In `replay.rs`,
  `each_route_replays_only_its_own_metadata` gains the rows refusing
  each OpenAI route's replay on the other. Reference test:
  ```rust
  #[test]
  fn a_key_and_a_token_send_byte_identical_bodies() {
      let mut bodies = Vec::new();
      for make in [Credential::key, Credential::bearer] {
          let mut input = public_call(1);
          input.credential = Some(make(b"secret-test-token".to_vec().into()));
          let mut world = World::new(input, limits_for(Provider::OpenAi), response(401, "", b"{}", false), 1);
          world.request(client::Request::Start);
          world.run();
          assert_eq!(fields_holding(&world.sent, b"secret-test-token"), ["Authorization"]);
          bodies.push(world.sent.rsplit(|b| *b == b'\n').next().unwrap().to_vec());
      }
      assert_eq!(bodies[0], bodies[1]);
  }
  ```
Proof: world, `a_key_and_a_token_send_byte_identical_bodies`: a key and
  a token send the same body bytes, the value in `Authorization`
  alone; under 0.1 s. Nothing is higher until skein 04's stories.
Don't: no field of the refused list; no `previous_response_id`; no
  header for a thread; no request moved from a plan's token to a key.
Size: about 200 lines.
Removes: 02.1's refusal and `a_public_call_is_refused_as_unsupported`.

### 02.4 The code table
Scope: llm.md 4.8 (the table, the status fallback), 5 (failure
  classes); decision 12; design question 9.
Needs: 02.3's public request; 02.2's code-first rule.
Files: `crates/skein-llm/src/openai/{common,response,tests}.rs`;
  `tests/llm/tests/public_failures.rs` (new).
Shape: one table of llm.md 4.8's codes, read on both OpenAI routes
  after Codex's own `usage_limit_reached`:
  `subscription_sharing_usage_limit_exceeded` `Exhausted`, its reset the
  error's `resets_in_seconds`, else `resets_at`, else `retry-after`,
  else `None`; `insufficient_quota` `Exhausted` with no reset, whatever
  `retry-after` says; `rate_limit_exceeded` `RateLimited`;
  `subscription_sharing_user_not_eligible`,
  `chatpass_v2_scope_not_authorized` and
  `chatpass_v2_invalid_authorization_context` `Refused`;
  `subscription_sharing_usage_unavailable` and
  `subscription_sharing_user_unavailable` `Unavailable`;
  `subscription_sharing_unsupported_capability` `Unsupported`, the
  report's field from `error.param`;
  `subscription_sharing_route_not_supported` `Invalid`;
  `subscription_sharing_invalid_user` and `invalid_api_key`
  `Unauthorized`; `context_length_exceeded` `ContextTooLong`. A code the
  table names wins over its status, before a stream and in
  `response.failed`; a code it does not name falls to the status, or,
  mid-stream, is `Unavailable` with the code kept. Downstream: none at
  compile time; smith meets `Refused`, `Unsupported` and a `None` reset
  from these codes, mapped by smith 01.3 (decision 20).
Test shape: world, `tests/llm/tests/public_failures.rs`, labelled
  synthetic, each under 0.1 s, with `World`, `public_call`,
  `limits_for(Provider::OpenAi)` and `response`.
  `each_code_of_the_table_gives_its_class`, a row per code at its
  status, on the public and Codex routes, with the report's code;
  `a_code_the_table_names_beats_its_status`, rows `insufficient_quota`
  429 with `retry-after` (`Exhausted`, `None`), a `Refused` code at 400,
  `subscription_sharing_usage_unavailable` 503 (not `Overloaded`),
  `subscription_sharing_route_not_supported` 403;
  `the_plans_limit_inside_response_failed_is_exhausted`;
  `the_plans_reset_comes_from_the_error_then_retry_after`, rows
  `resets_in_seconds`, `resets_at` at a set wall, `retry-after` alone,
  none (`None`); `an_unsupported_capability_names_its_field`;
  `overloaded_is_a_529_or_a_5xx_overloaded_error`, rows 529, 503
  `overloaded_error`, 503 another code (`Unavailable`);
  `a_mid_stream_code_outside_the_table_is_unavailable`. Units in
  `openai/tests.rs`, under 20 ms: the classify table of
  `an_http_error_body_refines_class_*` gains a row per code. Reference
  test, two of the rows:
  ```rust
  #[test]
  fn each_code_of_the_table_gives_its_class() {
      for (code, status, expected) in [
          ("insufficient_quota", 429, Failure::Exhausted { retry_after: None }),
          ("subscription_sharing_user_not_eligible", 403, Failure::Refused),
      ] {
          let body = format!(r#"{{"error":{{"code":"{code}"}}}}"#);
          let source = response(status, "", body.as_bytes(), false);
          let mut world = World::new(public_call(1), limits_for(Provider::OpenAi), source, 1);
          world.request(client::Request::Start);
          world.run();
          assert!(world.seen.iter().any(|e| matches!(e, client::Event::Failed { failure, report, .. }
              if *failure == expected && *report.code == *code.as_bytes())));
      }
  }
  ```
Proof: world, `each_code_of_the_table_gives_its_class`: each code of
  llm.md 4.8's table ends the call in its class with its code in the
  report; under 0.1 s. Nothing is higher until skein 04's stories.
Don't: no `RateLimited` for `insufficient_quota`; no zero reset; no
  class from the status where the table names the code; no expected
  class computed by `classify`.
Size: about 210 lines.
Removes: nothing.

## Interface sketches

```rust
// crates/skein-llm/src/types.rs
/// A wire dialect and its route (one arm added).
pub enum Provider {
    OpenAiCodex, Anthropic,
    /// `OpenAI`'s public Responses route: a key or a plan's token, as a bearer.
    OpenAi,
}
impl Endpoint {
    /// `api.openai.com` and `/v1/responses`.
    pub fn openai() -> Endpoint;
}
// crates/skein-llm/src/derive.rs
/// One set of client limits per route.
pub struct Limits { pub codex: client::Limits, pub anthropic: client::Limits, pub openai: client::Limits }
// crates/skein-llm/src/openai/common.rs
/// A failure's class: the route's codes first, its status after.
pub fn classify(provider: Provider, status: u16, error: Option<&ProviderError>,
    rate: RateLimit, wall: Wall) -> Failure;
// crates/skein-llm/src/dialect.rs
/// A route's stream decoder; both OpenAI routes share Responses'.
pub(crate) enum Decoder { Responses(Provider, openai::StreamDecoder), Anthropic(anthropic::StreamDecoder) }
```

## Questions to raise when reached

None: the plan answers the three it had, in 02.1, 02.2 and 02.3.

## Done when

- skein's `scripts/check.sh` passes on `origin/main`.
- `rg -oIw -e 'fn anthropic_failures_are_classed_by_type_first'
  -e 'fn a_key_and_a_token_send_byte_identical_bodies'
  -e 'fn each_code_of_the_table_gives_its_class'
  -e 'fn a_mid_stream_code_outside_the_table_is_unavailable'
  tests/llm/tests | sort -u | wc -l` prints 4.
- `rg -ow 'subscription_sharing_\w+|chatpass_v2_\w+|insufficient_quota'
  crates/skein-llm/src/openai/common.rs | sort -u | wc -l` prints 10,
  the table's codes that no other route names.
- `rg -U --count-matches -e 'Provider::OpenAi\s*=>\s*3(_u8)?\b'
  -e 'Some\(3\)\s*=>\s*Provider::OpenAi\b'
  crates/skein-llm/src/replay.rs` prints 2, and
  `rg -lw 'Decoder::Codex' crates tests` prints nothing.
- The size count of code-organization.md 8 lists no file this session
  grew past its rule.
