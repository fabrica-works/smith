# smith 03: records

The charter's version 3 names each LLM's account; the transcript's
version 6 records an opening turn's account and backend and carries a
window over.

Needs at start: smith 04.1's grant value union and 02.2's notices on
`origin/main`.

## Read first
- smith's design at `137a1cb4`: `protocol/` `charter.md` 2, 5, 8;
  `agent.md` 4; `llm.md` 6; `transcript.md` 2-4, 6, 8, 9; `README.md`
  8; `domain/` `run.md` 4, 6, 10; `session.md` 3, 4, 8.2, 10. skein's
  `programming-model.md` 4.4, 6.3, 8; `testing-strategy.md` 2.2, 2.5,
  8, 10.
- The README's Rules; code-organization.md (owners, fixtures, 6, 7);
  decisions.md 2, 3, 7, 10, 13, 16; questions 6, 8, 13, 14.

## What the design asks
- charter.md 2, 5, 8; agent.md 4: an LLM's `account`; its grant.
- run.md 4, 6; session.md 4, 8.2: grants checked; a window carried over.
- llm.md 6: no account, no table read, no credential.
- transcript.md 2, 3, 4, 6, 8, 9: opening, backend, dialect number.

## Choices
- The endpoint's `account` is read and ignored, doc-commented
  "withdrawn" (decision 2). No account number is bounded (question 14).
  Dialect numbers are agent.md 4's: `openai` 3, `anthropic` 2,
  withdrawn `codex` 1 (question 13).
- No conversation runs on a program yet (decision 7): the session's
  backend is `direct`; the program form is decoded, carried over and
  written, never produced.
- `Limited` and `Refused` are 02.1's names (decision 3); an LLM with no
  account fails as its model does (question 6).
- Codec and world tiers only (decision 10); the closing repins repair
  downstream breaks (decision 13).

## Rules that bind it
- A format moves in one increment: schema, generated code, goldens (a
  directory per version), version constant, `Versions` literal, its rows
  in `tests/codecs/tests/{drift,fuzzy_decode}.rs` and `smith check`'s
  formats line in `tests/end-to-end/end_to_end.rs`; the old version
  goes with it.
- Files over 800 lines grow by at most ~100 per increment (code-
  organization.md 8): `session.rs` (3671), `service.rs` (3002),
  smith-agent-shell's `config.rs` (1694), `route.rs` (1087),
  protocol-channel's `component.rs` (1033), at smith b638629f (ruling
  131).
- The files a change breaks, found at the base, `G` being `--glob
  '*.rs' --glob '!**/generated/**' crates tests benchmarks`; protocol-
  channel's `{charter,translate,turn,transcript,tests}.rs` alias a
  codec as `wire`, so Files names them:
  ```sh
  rg -l 'smith_charter::v2|charter-v2|charter/golden/v2' G  # 20
  rg -l -U '(?s)\bLlm \{.{0,700}?\n\s+account: ' G  # 23
  rg -l -U 'Endpoint \{[^}]*\baccount:' G  # 16
  rg -l -U -P '\bEndpoints::new\((?:[^(),]|(\((?:[^()]|(?1))*\)))*,' G  # 10, the llm's
  rg -l -U '(?s)Complete \{[^}]{0,500}\bgrant\b' G  # 12
  rg -l 'smith_transcript::v5|transcript-v5|transcript/golden/v5' G  # 13
  rg -l -U 'Kind::(First|Window)\s*\{[^}.]*\}' G  # 21, none with `..`
  rg -l -U '(?s)\b(Turn|Transcript) \{[^}]{0,600}\bdialect:' G  # 20
  rg -l 'Refusal::(Endpoint|Dialect)\b' G  # 18
  rg -n '\b(charter: 2|transcript: 5)\b' G  # 7 literals
  ```

## Increments

### 03.1 The charter's version 3
Scope: charter.md 2, 5, 8; run.md 4; agent.md 4; llm.md 6; session.md
  4; questions 6 and 14.
Needs: smith 04.1's grant value union; 01.3's start with no credential
  (skein 01.5, by its repin); 02.1's run faults and 01.2's wire faults;
  02.2's notices.
Files: `crates/smith-charter/`: `schema/charter-v3.schema` (new),
  `src/{lib,generated/v3}.rs`, `golden/v3/` (new); the files of the
  Rules' lists for the charter's v2, `Llm` and `Endpoint` literals,
  the llm's `Endpoints::new`, `Complete` literals and `charter: 2`,
  with those the Shape changes:
  in `crates/`: `smith-agent-service/src/{profile,
  profile/effective/tests,service,tests}.rs`,
  `smith-agent-shell/src/{agent_shell,config}.rs`,
  `smith-domain-run/src/{charter,tests}.rs`,
  `smith-domain-session/src/{boundary,session,tests}.rs`,
  `smith-domain/src/{boundary,domain,peer,route,tests,
  tests/completion_retry,translate,translation_tests}.rs`, `smith-host-domain/src/tests.rs`,
  `smith-host-protocol/src/{answer,component,limits,tests,
  translate}.rs`, `smith-inline-agent/src/{boundary,translate}.rs`,
  `smith-local-domain/src/{config,end,person,tests}.rs`,
  `smith-local-protocol/src/{store,terminal,terminal/failure}.rs`,
  `smith-local-service/src/{service,service/choices,tests}.rs`,
  `smith-local-shell/src/{credentials/product,local_host/words,
  settings/resolve/models,startup}.rs`,
  `smith-protocol-channel/src/{charter,component,limits,result,tests,
  tests/jobs,translate}.rs`, `smith-protocol-llm/src/{boundary,
  component,endpoints,grants,guard,tests}.rs`;
  in `tests/`: `agent-process/src/{lib,tracing}.rs`,
  `agent-process/tests/grants.rs` (ruling 131),
  `agent/src/{memory/fixture,world}.rs`, `agent/tests/{brief,
  budget,models}.rs`, `channel/src/lib.rs`, `channel/tests/{composed,
  composed/long,fuzzy_stream_cuts,live,memory,referee,stories,
  stories/limits}.rs` (`live.rs` reads `golden/v3`), `codecs/tests/{charter,drift,fuzzy_decode,
  memory}.rs`, `end-to-end/end_to_end.rs` (formats line),
  `host/src/lib.rs`, `local-process/src/{process,
  process/configuration}.rs`, `local/src/world.rs`,
  `local/tests/memory.rs`, `protocol-llm/src/{pool_peer,world}.rs`,
  `protocol-llm/tests/{brief,budget,component_boundary,component_peer,
  endpoints_grants,production_render,root_memory}.rs`,
  `run/src/host.rs`, `run/tests/memory.rs`,
  `session/tests/{window_room,memory,simulation}.rs`,
  `session/src/{world,recorded}.rs` (their full `session::Spec`
  literals, found by reading: ruling 120), `agent/tests/accounts.rs`
  (new).
Shape: the schema's `record Llm` gains `account: option u32`;
  `FORMAT_VERSION` 3. `run::charter::Llm.account` is `Option<u32>`,
  taken by `translate.rs`'s `llm()` from the wire LLM, not the
  endpoint. `Endpoint.account`, `ConfiguredEndpoint.account`,
  `EndpointOptions.account`, `EndpointError::Account`, the component's
  `options.account == grant.account` and the account limit of
  smith-protocol-llm's `Endpoints::new` go; the configuration's
  endpoint `account` becomes `#[serde(default)]`, withdrawn, read and
  dropped. `takes_grants` takes the charter: every account of
  `charter.llm` and `charter.models` needs a grant, in the start or
  held, else `Invalid::Grants`; LLMs naming one account share its
  grant; the count rule stays. 04.1's install check, smith-protocol-llm's
  `Component::grant` (`component.rs`: its loop over `self.options`
  that matches `options.account == name.account` and judges the scheme
  by `options.provider`, else `GrantError::Scheme`), reads the dialect
  of the charter's LLM naming the grant's account, not the endpoint's
  (ruling 131). The component keeps the bindings: at each admitted
  start the service hands it, from the charter's `llm` and `models`,
  each account with the endpoints its LLMs name, bounded by the
  charter's LLMs and replacing the last start's; `grant` keeps its
  signature and judges a value by those endpoints' `options.provider`,
  for the start's grants and for later refreshes alike, by the channel
  or colocated; an account no LLM names is judged by none, as today.
  The service keeps no charter for it (ruling 133). `Peer.account` and `Spec.account` (new, from the
  charter's LLM in `route.rs`) are `Option<u32>`; with none,
  `Request::Complete` and `FromDomain::Complete` carry `grant: None`
  and the component calls `Grants::credential(None, now)` (new):
  `credential: None`, no table read; `guard.rs`'s `check` and `measure`
  take `Option<&Credential>`, handed as it is to skein's
  `client::measure`, which takes an option already. `domain.rs`'s
  `IssuedCompletion.grant` is `Option<GrantName>`, and `route.rs`'s
  `failed()` arms for `Unauthorized`, `Exhausted` and `Refused` handle
  none (no
  grant to name). Every full `session::Spec` literal takes `account`. With `grant` an option, `complete`
  in `route.rs` would reach 101 lines (the limit is 100), so its
  unsent-unauthorized path becomes a helper fn. `backoff` in
  `session.rs` ends an unauthorized call with no account at its first
  attempt.
  `translate::end` ends an account failure (02.1's limited, refused,
  unauthorized) with no account as `run::Fault::Completion` of its
  class; 02.2's notice is raised only for an account. `answer_status`
  in smith-local-domain takes `Option<u32>`: `Credentials` for some,
  `RunFailed` for none; `models.rs` wraps its preset's account in
  `Some`; other readers of `Llm.account` take the option.
Test shape: codecs, `tests/codecs/tests/charter.rs` (goldens from
  `golden/v3`): `llm_goldens_carry_an_account_or_none`;
  `a_version_two_charter_is_refused`, beside
  `charter_rejects_an_unsupported_version_before_fields`; drift's
  `charter_schema_matches_generated_code_and_all_goldens` and the fuzzy
  rows read `v3`. Units, each under 20 ms: smith-protocol-llm
  `src/tests.rs`, `a_call_with_no_account_carries_no_credential` over
  `Grants::new(1, 64)` (none: `credential: None`, no table read; a
  named grant: `Err(GrantError::Missing)`); smith-domain-session
  `src/tests.rs` (`Harness::new(&LIMITS)`, `h.open`, `h.step(Event::
  Failed { .. })`, `ended`): `an_unauthorized_call_is_retried_only_for_
  an_account`, rows account (retried), none (`Failed { Unauthorized,
  attempts: 1 }`); smith-domain `src/tests/completion_retry.rs`, beside 02.1's
  `a_limited_account_ends_the_run_with_its_reset` (its `account_failure`
  takes the LLM's account, `Option<u32>`, where its charter fixes 0): `an_unaccounted_llms_
  account_failure_names_no_account`, rows limited, refused,
  unauthorized: `Fault::Completion` of that class, no `Request::
  {Limited, Refused, Unauthorized}`; smith-agent-shell `config.rs`:
  `an_endpoints_account_is_read_and_ignored`, built from `document()`,
  whose endpoint's dialect matches its provider (`build_models` refuses
  one that does not, since smith cbcecbbd). Agent world, each under
  0.1 s, `tests/agent/tests/accounts.rs` (new; seeds from 1501;
  `models.rs`'s private `world` moved into `tests/agent/src/world.rs`
  with the granted accounts a parameter, `world(settings, charter,
  granted: &[u32])`, which `models.rs` calls too (README, "A test
  helper is written once"); `Settings::calm`, `scripted_charter`):
  `a_start_without_an_accounts_grant_is_invalid`, rows main's account,
  a sub-agent's, one of two; `llms_sharing_an_account_share_its_grant`;
  `a_held_grant_serves_a_later_start`;
  `an_llm_with_no_account_is_admitted_with_no_grant`. Process world:
  `tests/agent-process/tests/grants.rs`'s
  `an_uninstallable_grant_refuses_the_start_as_grants`, whose rows set
  `world.openai()`, gains the row of a `withdrawn` value with an id on
  an `openai` LLM, refused as `Grants` (the dialect is `charter.llm`'s
  or `models`', not the endpoint's; ruling 131).
  Reference test:
  ```rust
  #[test]
  fn a_start_without_an_accounts_grant_is_invalid() {
      let settings = Settings { job: Job::Reporting, ..Settings::calm(1501) };
      let mut charter = scripted_charter(&settings);
      charter.llm.account = Some(9);
      let mut world = world(&settings, charter, &[0]);
      world.run(2000);
      let invalid = run::Refusal::Invalid(run::Invalid::Grants);
      assert_eq!(world.answer(), &run::Answer::Refused(invalid));
      assert!(world.prompts().is_empty() && world.turns().is_empty());
  }
  ```
Proof: world, `a_start_without_an_accounts_grant_is_invalid`: a start
  whose LLM names account 9, with only account 0 granted, is answered
  `Invalid::Grants` and sends no prompt; under 0.1 s; process,
  `smith_check_json_shows_each_value_with_its_layer`: `charter` reads
  3, beside the drift and fuzzy rows on `v3`; under 0.5 s (a real
  process; about 0.15 s at the base).
Downstream: jig-repin: `tests/channel/src/fixtures.rs`,
  `testing/jig-test-service/src/tests/{fixtures,worker}.rs`,
  `crates/jig-protocol-charter/src/{encode,charter}.rs`
  (`name.account == value.account`, `account: model.account`), and the
  further files naming `smith_charter::v2` (`v2::` in `result.rs`):
  `crates/jig-protocol-charter/src/{lib,tests,result}.rs`,
  `examples/ops/domain/src/{tests,limits}.rs`,
  `tests/ops/src/conformance/{scripts,config}.rs`,
  `testing/jig-test-worker/src/tests/fixtures.rs` (`SMALLEST: [u8;
  119]`). temper repin: `crates/temper-service/src/workers.rs`,
  `crates/temper-service/src/tests/{fixtures,fixtures/agent,
  workers}.rs`,
  `tests/one-process/src/fixtures.rs`, `tests/worker/domain/src/
  protocol.rs` (`Endpoint { .., account: 0 }`), `crates/temper-engine-
  smith/src/start.rs` (`account: model.account`) and `crates/temper-
  worker-domain/src/tests/fixtures.rs` (`Llm.account`).
Don't: read an endpoint's account anywhere; bound an account's number;
  give an LLM with no account an empty grant; keep a v2 reader.
Size: about 300 lines.
Removes: the charter's v2 schema, code and goldens; the channel
  endpoint's and the LLM component's account; `EndpointError::Account`.

### 03.2 The transcript's version 6
Scope: transcript.md 2, 3, 4 (the codec's list), 6, 8; agent.md 4;
  question 13.
Needs: 03.1's charter account and `Spec.account`; smith 04.2's dialect
  numbers, `openai` 3.
Files: `crates/smith-agent-shell/src/config.rs` (the memory floor, the
  README's rule; ruling 141); `crates/smith-transcript/`: `schema/transcript-v6.schema` (new),
  `src/{lib,generated/v6}.rs`, `golden/v6/` (new); the files of the
  Rules' lists for the transcript's v5, `Kind::First` and `Window`
  patterns without `..`, `Turn` and `Transcript` literals with
  `dialect:` and `transcript: 5`, with those the Shape changes:
  in `crates/`: `smith-agent-service/src/{profile,
  profile/effective/channel,profile/tests,service}.rs`,
  `smith-agent-shell/src/agent_shell.rs`,
  `smith-domain-session/src/{boundary,record,session,tests}.rs`,
  `smith-domain/src/{route,tests,tests/completion_retry}.rs`,
  `smith-host-protocol/src/{component,limits,translate}.rs`,
  `smith-inline-agent/src/tests.rs`,
  `smith-local-domain/src/{parent_tests,person,tests}.rs`,
  `smith-local-protocol/src/{store,tests/store}.rs`,
  `smith-local-service/src/profile/effective/channel.rs`,
  `smith-local-shell/src/{front/headless,startup}.rs`,
  `smith-protocol-channel/src/{component,limits,tests,transcript,
  turn}.rs`;
  in `tests/`: `agent-process/src/tracing.rs`,
  `agent/src/{messages_referee,world}.rs`, `agent/tests/{brief,
  compaction,messages}.rs`, `channel/tests/stories.rs`,
  `codecs/tests/{drift,fuzzy_decode,memory,transcript}.rs`,
  `end-to-end/end_to_end.rs` (formats line), `host/src/lib.rs`,
  `local/src/world.rs`, `protocol-llm/src/{messages_referee,world}.rs`,
  `protocol-llm/tests/{brief,budget,native_root_restore,
  production_render,root_memory,workspace}.rs`, `run/tests/jobs.rs`,
  `session/src/recorded.rs`, `session/tests/{memory,recorded}.rs`.
Shape: the schema gains `record Program { name: text 32 }`, `enum
  Backend { direct, program: Program }` and `record Opening {
  affinity: Affinity account: option u32 backend: Backend }`; `Turn`'s
  `affinity` becomes `opening: option Opening`, so an opening turn's
  absent account is told apart from an ordinary turn's; `dialect` in
  `Turn`, `Replay` and `Opaque` is a `u32`, agent.md 4's number. The
  domain: `record::VERSION` 6; `record::Backend` (new); `Kind::First`
  and `Kind::Window` gain `account` and `backend`, and `Kind` loses
  `Copy` (`opens_window` takes `&self`). `Turn` grows with its `Kind`
  (it is not `Copy`), so the referee worlds' `Seen::Turn`, in
  `tests/{agent,protocol-llm}/src/messages_referee.rs`, boxes it
  (`large_enum_variant`). A window turn of every
  compaction (a threshold, a model change) records the charter's
  endpoint, dialect, model, account and `direct`. The codec: an
  `opening` on a turn that is neither first nor a window turn, or
  missing on the first turn or a window turn, is malformed; a turn of
  version 5 or earlier is `Version`. `turn.rs` writes the number;
  `dialect_name` goes.
Test shape: codecs, `tests/codecs/tests/transcript.rs`: goldens for
  every block, result and turn kind, an opening with an account and
  with none, each backend; `a_turn_carries_its_dialects_number`, rows
  `openai` 3, `anthropic` 2; `a_version_five_transcript_is_refused`
  beside version 4's. Units, each under 20 ms: smith-domain-session
  `src/tests.rs`, `opens_window_is_true_of_first_and_window_turns_
  only`; smith-protocol-channel `src/tests.rs`, beside
  `a_transcript_records_affinity_only_in_its_opening_turn` (its helper
  `with_recorded_affinity` takes an opening): `an_opening_on_a_later_
  turn_is_malformed`, rows an ordinary turn with an account, with a
  backend; `a_first_turn_without_its_opening_is_malformed`, rows a
  first turn, a window turn lacking its opening. Session world, each
  under 0.1 s, `recorded.rs` (`recorded::{World, opening, scenario,
  transcript}`): `a_domain_turn_round_trips_its_account_and_backend`,
  rows a first turn, a window turn after the first; `each_compactions_
  window_turn_names_the_charters_llm`, rows threshold, model change.
Proof: world, `a_domain_turn_round_trips_its_account_and_backend`: a
  first turn and a later window turn recorded with account 7 and a
  program backend decode equal; under 0.1 s; process, `smith_check_
  json_shows_each_value_with_its_layer`: `transcript` reads 6, beside
  the drift and fuzzy rows on `v6`; under 0.5 s (a real process;
  about 0.15 s at the base).
Downstream: jig's contract keeps `dialect` text and takes the dialect
  number's decimal digits (decision 16). jig-repin: `crates/jig-
  protocol-transcript` (`src/{lib,summary,tests}.rs`, `tests/data/
  record_turn_{smallest,full}.bin` and that directory's `README.md`,
  which names the smith revision they were copied from); the `TurnParts` literals of
  `tests/client-protocol/src/fixtures.rs`, `tests/client/src/joint/
  fixture.rs`, `tests/ops/src/conformance/scripts.rs` and `crates/
  jig-client-protocol-link/src/tests/fixtures/turns.rs`; the nine other
  `tests/client/` files naming `smith_transcript::v5`; `testing/jig-
  test-service/src/tests/fixtures.rs` (`Kind::First`). temper repin:
  `tests/worker/domain/src/protocol.rs` (`session_turn`'s
  `Kind::First { affinity }`).
Don't: carry a version-5 window over; compare account or backend on
  resume yet (03.3 does); keep a v5 reader.
Size: about 250 lines.
Removes: the transcript's v5 schema, code and goldens; `dialect_name`.

### 03.3 A window carried over
Scope: transcript.md 4, 9; session.md 3, 8.2, 10; run.md 6; question 8.
Needs: 03.2's opening account and backend; smith 01.2's channel
  version 4, whose transcript refusals name no endpoint or dialect.
Files: in `crates/`: `smith-domain-session/src/{record,session,
  compaction}.rs`, `smith-domain/src/{route,translate,feedback}.rs`,
  `smith-domain-run/src/boundary.rs`, `smith-protocol-channel/src/
  {transcript,translate,component,turn,tests}.rs`; `tests/session/
  tests/{recorded,compaction}.rs`; `tests/agent/tests/{compaction,
  messages}.rs`; `tests/protocol-llm/tests/native_root_restore.rs`;
  every file of the lists above for `Refusal::Endpoint` or `Dialect`
  (18, among them `smith-agent-service/src/answers.rs`,
  `smith-local-service/src/stream.rs` and
  `tests/agent/tests/feedback_table.rs`) and `Turn` and `Transcript`
  literals with `dialect:` (20); `smith-host-domain/src/channel.rs`
  (`host::TranscriptRefusal`).
Shape: `record::Turn.endpoint` and `record::Transcript.endpoint` are
  `Option<Endpoint>`. `decode_transcript` resolves an endpoint name the
  configuration does not name to none, takes the dialect from the
  turn's own number, and checks opaque blocks and replay against it;
  another endpoint or dialect on a later turn, or another dialect's
  opaque block, is `Malformed`. `validate_transcript` no longer
  compares endpoint or dialect. `restore` compares the window's
  endpoint, dialect, account and backend with the charter's main LLM:
  on any difference it drops opaque blocks and replay from the
  history, and `compaction::carry_over` (new) marks the compaction due
  on the conversation's own LLM. Its window turn names the charter's
  LLM (03.2), so the next resume does not compact again.
  `resumed_model` gives no window LLM for a carried window: a model
  change compacts on the window's model only on the same endpoint,
  dialect, account and backend. `record::Refusal::{Endpoint, Dialect}`
  go, with `run::TranscriptRefusal`'s and `host::TranscriptRefusal`'s
  (`smith-host-domain/src/channel.rs`), their words in `feedback.rs` and
  their mappings in `translate.rs`, protocol-channel's `translate.rs`
  and `component.rs`; `encode_turn` fails a told turn of an unnamed
  endpoint as malformed. Downstream: the `record::Turn` literals'
  `endpoint` takes `Some`: jig's `testing/jig-test-{worker,service}/
  src/tests/fixtures.rs`, the closing `jig-repin`'s; temper's
  `tests/worker/domain/src/protocol.rs`, `crates/temper-worker-domain/
  src/tests/fixtures.rs` and `tests/system/src/{world,composed/
  process}.rs`, its own repin's.
Test shape: session world, each under 0.1 s, `recorded.rs`
  (`recorded::{World, opening, scenario, transcript}`):
  `a_window_from_elsewhere_is_carried_over`, rows another dialect,
  another endpoint, another account, a `claude_code` backend: each
  compacts on the charter's LLM with no opaque block or replay sent,
  and its window turn names the charter's endpoint, dialect, account
  and backend; `a_resume_after_a_carry_over_does_not_compact_again`;
  `a_same_window_with_another_model_compacts_on_its_own_model`; the
  endpoint and dialect refusal rows become the carry-over's. Agent
  world, `tests/agent/tests/compaction.rs` (`history`, `world`):
  `a_window_from_an_unnamed_endpoint_is_carried_over`; `messages.rs`'s
  `every_transient_history_refusal_is_exact_and_starts_no_provider_or_
  tool_effect` keeps the four rows version, malformed, unresolved, too
  large. Unit, under 20 ms, protocol-channel `src/tests.rs`, beside
  `transcript_refuses_a_gap_and_a_different_dialect`:
  `another_dialects_opaque_block_in_a_window_is_malformed`.
Proof: world, `a_window_from_an_unnamed_endpoint_is_carried_over`: a
  resumed window naming an endpoint the configuration lacks compacts
  on the charter's LLM and sends no opaque block; under 0.1 s.
Don't: send an opaque block or replay to a dialect that did not write
  it; compact a carried window on its own model or account; keep an
  endpoint or dialect refusal under another name.
Size: about 280 lines.
Removes: `Refusal::{Endpoint, Dialect}` in the domain, the run, the
  host and the protocol, and their words; 01.2's interim mapping of them to
  `malformed`.

## Interface sketches
```rust
// 03.1: run::charter::Llm.account: Option<u32>; none: no credential, no table read
// 03.1: smith_domain::Request::Complete, FromDomain::Complete: grant: Option<GrantName>

// smith_domain_session::record (Clone, PartialEq, Eq, Hash, Debug; Kind not Copy)
// 03.2: Turn grows with Kind; the referee worlds' Seen::Turn boxes it
/// What a window's completions ran on: its endpoint's dialect, or a program.
// (smith_events::Backend, 05.1, is another type of this name.)
pub enum Backend { Direct, Program { name: Box<[u8]> } }
/// An opening turn carries its window's affinity, account and backend.
pub enum Kind {
    First { affinity: Affinity, account: Option<u32>, backend: Backend },
    Ordinary,
    Window { affinity: Affinity, account: Option<u32>, backend: Backend },
}
// 03.3
/// Why a transcript cannot be used; anything else is carried over (Copy).
pub enum Refusal { Version, Malformed, Unresolved, TooLarge }
// Transcript { version, endpoint: Option<Endpoint>, dialect: u32, turns }
// (the domain's `dialect` is a `u32` at the base; the codec's text
// `dialect` becomes the number in 03.2)
```

## Done when
Each prints nothing, but the `ls` lines (code-organization.md 8's
commands check sizes and long files):
```sh
ls crates/smith-charter/schema crates/smith-transcript/schema  # v3, v6
ls crates/smith-charter/golden crates/smith-transcript/golden  # v3; v6
rg -n -e 'smith_charter::v2|charter-v2|charter/golden/v2' \
  -e 'smith_transcript::v5|transcript-v5|transcript/golden/v5' \
  crates tests benchmarks
rg -n --glob '*.rs' -P -e '\bEndpointError::Account\b' \
  -e '(?<!connection::)\bRefusal::(Endpoint|Dialect)\b' crates tests benchmarks
```
