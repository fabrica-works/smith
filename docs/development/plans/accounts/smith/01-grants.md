# smith 01: grants

smith takes skein's credential with its scheme, its failure classes
and the public route, and the channel moves to version 4: a grant's
value is a credential, a program's name or the withdrawn id and
bearer, and an account's failures go up as unauthorized, limited and
refused. smith is MIT-licensed.

Needs at start: nothing beyond smith's `origin/main`. 01.2 waits for
live text's channel schema step, 01.3 for skein 01 and 02 and live
text's LLM component step (decisions.md, 17).

## Read first

- smith's design at `137a1cb4` and skein's at `378fd8c`, the sections
  below; skein `docs/foundation/programming-model.md` 2.1, 4.4, 6.2,
  7, 10.4 and `testing-strategy.md` 2.1, 2.2, 8, 10.
- The README's Rules; code-organization.md 6, 7, 8; decisions.md 2,
  5, 7, 10, 11, 13, 16, 17, 22 and design questions 6, 7, 8, 13.

## What the design asks

- channel.md 4, 5, limits.md 2.4, README.md 8: the account notices;
  the transcript refusals go.
- channel.md 6: a grant's value is a tagged union.
- llm.md 6, 7: the component sends a credential by its scheme.
- shell.md 3.1: smith is MIT-licensed.

## Choices

- The withdrawn forms stay, doc-commented "withdrawn": the id and
  bearer grant and the `codex` dialect, for the second part of LLM
  accounts (decision 2). A program's grant decodes and is refused as
  uninstallable (decision 7).
- skein's names are `value`, `Scheme`, `CredentialLimits::value` and
  `Cap::Value` (decision 5).
- smith takes skein twice (README, Rules; decision 13): 01.3 repins at
  its start and `smith-repin` takes the rest. jig's breaks are
  `jig-repin.s1`'s (decision 22); the repository downstream of jig's
  are its own repin's, on decision 16's defaults: a refused notice
  reaches no event downstream (ruling 130).
- No new live or process tier (decision 10); the licence is decision
  11's.

## Rules that bind it

- A format moves in one increment: schema, generated code, goldens
  (from the generator), `V4`, `FORMAT_VERSION`, the drift test and
  `tests/codecs/tests/fuzzy_decode.rs`; version 3's go with it.
- Transitional code names, in a comment, the increment that removes it.
- No value in a `Debug`, fact, turn, trace or test output; tests
  compare tags, schemes, accounts and lengths.
- Files over 800 lines grow by at most ~100 per increment:
  `smith-agent-service/src/service.rs` (2920), `config.rs` (1658),
  smith-local-service's `process.rs` (1420), smith-protocol-channel's
  `component.rs` (1002), `tests/channel/tests/stories.rs` (1255).

## Increments

### 01.1 The licence
Scope: decision 11; shell.md 3.1.
Needs: nothing.
Files: `LICENSE` (new); `Cargo.toml`; every member's `Cargo.toml`
  (`benchmarks`, `crates/*`, `tests/*`).
Shape: `LICENSE` is MIT's text, `Copyright (c) 2026 Free Ekanayaka`.
  `license = "MIT"` joins `[workspace.package]`; `license.workspace =
  true` joins `[package]` of the root package and each member.
  `benchmarks/tasks/` holds fixtures, not packages, and is untouched.
Test shape: none beyond the check below.
Proof: unit, no test fn (manifests only): `cargo metadata
  --format-version 1 --no-deps | jq -e '[.packages[].license] |
  all(. == "MIT")'`, then `heavy cargo check --workspace
  --all-targets`; under 60 s.
Don't: give `benchmarks/tasks/` a licence; add one per crate.
Size: about 40 lines.
Removes: nothing.

### 01.2 The channel's version 4
Scope: channel.md 4, 5, 6; limits.md 2.4; README.md 8; questions 7, 8.
Needs: live text's channel version 3 schema, with its live record
  (`plan await coordinator live-text/smith-01.2`).
Files: `crates/smith-channel/`: `schema/channel-v4.schema` (new),
  `src/generated/v4.rs` (new), `golden/v4/` (new),
  `src/{kinds,lib}.rs`; `crates/smith-protocol-channel/src/`:
  `{component,translate,limits,credentials,lib}.rs`,
  `component/notices.rs` (new); `crates/smith-host-protocol/src/`:
  `{component,process,translate,answer,limits,tests,lib}.rs`;
  `crates/smith-agent-service/src/`: `{service,profile}.rs`,
  `profile/effective/channel.rs`;
  `crates/smith-agent-shell/src/agent_shell.rs`;
  `crates/smith-local-service/src/`: `process.rs`,
  `profile/effective/channel.rs`;
  `crates/smith-local-shell/src/startup.rs`;
  `tests/codecs/tests/{channel,drift,fuzzy_decode}.rs`;
  `tests/channel/{src/{lib,referee}.rs,tests/stories.rs}`;
  `tests/hosts/src/lib.rs`; `tests/agent-process/src/{lib,tracing}.rs`;
  `tests/end-to-end/end_to_end.rs` (formats line).
Shape: from v3: `enum Scheme { bearer, key }`, `record
  CredentialValue { scheme: Scheme bytes: secret 8192 }`, `record
  ProgramValue { name: text 32 }`, `record WithdrawnValue { id: text
  1024 bearer: secret 8192 }` and `enum GrantValue { credential,
  program, withdrawn }`. Both values are `secret` (skein codec.md 3:
  "A credential is a `secret N`, never `bytes N`"; ruling 107): they
  cross as `bytes 8192` would, their generated fields are
  `skein_lib::Secret`, whose `Debug` is its length, and their owners
  read them with `expose`. The id's 1024 is `Source::account_id_bytes()`
  in `crates/smith-local-service/src/credentials.rs`; the shipped
  profile's `grant_value_bytes` (2048, agent-service `profile.rs`)
  bounds id and bearer each and stays above it. The codec checks
  length and UTF-8 only: a program name's charset is smith 04.1's.
  `Unauthorized { account: u32 generation: u64 }` (0x010d), `Limited
  { account: u32 reset: option duration }` (0x010e) and
  `AccountRefused { account: u32 code: text 128 reason: text 512 }`
  (0x0113, after live text's `Live` 0x0112) replace `Rejected` and
  `Exhausted`: 20 kinds, all required, none optional. `ModelFault`
  keeps tags 0 to 5, tag 3 named `declined` (the model's own refusal,
  decision 3), `limited` as `LimitedFault { account: u32 reset:
  option duration }`, and gains `refused: RefusedFault { account: u32
  code: text 128 }`, `unauthorized: UnauthorizedFault { account: u32
  }` and `program: ProgramFault { name: text 32 exit: option u32
  detail: bytes 512 }` (nothing raises the last two yet);
  `ModelFaultValue` loses `account` and `retry_after_ms`.
  `CompletionFailure` gains `refused` and `unsupported`,
  `CompletionFault` gains `code`, `request_id` and `field` (`text 128`
  each; smith 02.1 fills them). `TranscriptRefusal` is `{ version,
  malformed, unresolved, too_large }`. The generated `Limits` loses
  `grant_value_credential` and gains a bound per new text and bytes
  field; `limits.rs` takes the largest arm for the Start's worst case
  and both `profile/effective/channel.rs` destructure the new names.
  `FORMAT_VERSION` is 4, `V3` becomes `V4`, `schema()` pushes
  `Version { version: 4, kinds }` and `pub mod v4` replaces `v3`.
  Each site naming the version moves from live text's 3 to 4:
  `highest.min(4)` in protocol-channel `component.rs`; the
  `smith_channel::v4::` paths in protocol-channel `limits.rs` and
  host-protocol `limits.rs`, `component.rs` and `tests.rs`;
  `schema.version(4)` in agent-service `profile.rs`, `service.rs`'s
  tests, `tests/channel/src/lib.rs` and `tests/codecs/tests/
  channel.rs` (with its `golden/v4` paths); the `Versions { channel: 4
  }` literals in `agent_shell.rs`, `startup.rs`,
  `tests/agent-process/src/tracing.rs` and `service.rs`'s tests; the
  formats line of `tests/end-to-end/end_to_end.rs`, now
  `"channel":4,"charter":2,"transcript":5,"events":3`.
  `component/notices.rs`, a child module declared `mod notices;` in
  `component.rs` (so it reaches `Component`'s private fields), holds
  `send_unauthorized`, `send_limited(account, reset: Option<Duration>,
  ..)` and `send_refused(account, code, reason, ..)`, moved from
  `component.rs`: a code past 128 bytes goes empty, a reason past 512
  is cut at a character. Until smith 04.1
  the host's `encode_start` and `send_grant` write each value as
  `withdrawn`, splitting the lent envelope into id and bearer (an id
  that is not UTF-8 or is past 1024 bytes is `Error::Grants`), and the
  agent keeps only a withdrawn value, repacked as the table's
  envelope: a credential or program arm is `Error::Grants`. Until
  smith 02.1 the agent sends the domain's zero reset (not said) as
  absent; `translate.rs` maps the transcript `Endpoint` and `Dialect`
  refusals to `malformed` (smith 03.3 removes them); `answer.rs` maps
  the new faults to `Provider` and the new completion classes to
  `Invalid`. host-protocol's `OpenEvent::{Rejected, Exhausted}` become
  `Unauthorized`, `Limited { account, reset: Option<Duration> }` and
  `Refused { account, code, reason }` (new); local-service's
  `to_host_event` maps them onto `Up::Rejected` and `Up::Exhausted`
  (absent as `ZERO`, refused as rejected) until smith 02.2.
  Downstream: jig's `smith_channel::v2` paths at its pin become
  `smith_channel::v4`: `tests/core/src/{agents_tools,protocol}.rs`,
  `testing/jig-test-service/src/tests/fixtures.rs` and
  `tests/ops/src/conformance/scripts.rs` (about 12 lines), all
  `jig-repin.s1`'s. jig's `OpenEvent::{Rejected, Exhausted}` matches in
  `testing/jig-test-service/src/worker_route.rs` and the repository
  downstream of jig's take decision 16's defaults at the closing
  repins.
Test shape: codecs, `channel.rs` (each under 0.1 s; bounds from
  `smith_channel::CEILINGS`): goldens for every grant value form, each
  notice (limited with a reset of zero, one and none; refused at both
  bounds) and each fault and class;
  `each_grant_value_form_reads_back_by_its_tag`, rows bearer, key,
  program, withdrawn; `a_grant_value_past_its_bounds_is_refused`, rows
  a program name of 33 bytes (`Bound`) and one that is not UTF-8
  (`Utf8`), a withdrawn id of 1025 bytes and a bearer of 8193
  (`Bound`); the charset is smith 04.1's `program_name`, which refuses
  an uppercase name;
  `every_required_kind_has_its_direction_and_bound_in_the_schema`
  reads `V4`, version 4 and 20 kinds. `drift.rs` reads v4; the fuzz
  target `fuzzy_decode.rs` (within its profile's 60 s) takes the rows
  `unauthorized`, `limited` and `account_refused` for `rejected` and
  `exhausted`, every row at v4. Channel world, `stories.rs` (each under
  0.1 s): `no_version_in_common_ends_the_channel_before_a_start` gains
  a version-3 peer's row;
  `rejected_and_exhausted_notices_reach_the_host_without_values`
  becomes `account_notices_reach_the_host_without_values`, rows
  unauthorized, limited with zero, limited with none, refused; the
  world's `agent_rejects_grant` and `agent_exhausts_account` become
  `agent_reports_unauthorized`, `agent_reports_limited` and
  `agent_reports_refused`, its Observations `HostRejected` and
  `HostExhausted` become `HostUnauthorized`, `HostLimited` and
  `HostRefused`. Unit, protocol-channel (under 20 ms):
  `a_refused_notice_keeps_its_bounds`, rows a code of 129 bytes sent
  empty, a reason cut inside a character.
Proof: world, `account_notices_reach_the_host_without_values`: the
  host half observes each notice, none carrying a value; under 0.1 s;
  process, `smith_check_json_shows_each_value_with_its_layer` (an
  existing test, so no new tier, decision 10): `smith check --json`
  reports `"channel":4`; under 1 s.
Don't: keep a v3 reader; clip a code; send zero for a reset not said.
Size: about 300 lines, `src/generated/` and goldens excepted.
Removes: the channel's v3 schema, generated code and goldens;
  `Rejected`, `Exhausted` and their senders; the two transcript
  refusals on the wire.

### 01.3 skein's credential and failure classes
Scope: llm.md 6, 7; skein's llm.md 2.5, 2.6, 4.8; questions 6, 13.
Needs: skein 01's `Scheme`, `Credential`, `Report` and optional
  credential; skein 02.3's `Provider::OpenAi`, measured and prepared
  (ruling 113); live text's LLM
  component step (`plan await coordinator live-text/smith-01.3`).
Files: `Cargo.lock`; `crates/smith-protocol-llm/src/`: `grants.rs`,
  `framing.rs`, `failure.rs`, `component.rs`, `guard.rs`,
  `endpoints.rs`, `tests.rs`; `crates/smith-agent-service/src/`:
  `service.rs`, `tests.rs`, `profile/words.rs`, `profile/tests.rs`,
  `profile/effective/tests.rs`; `crates/smith-agent-shell/src/
  config.rs`; `crates/smith-local-shell/src/credentials/product.rs`;
  `tests/protocol-llm/src/{adapter,pool_peer,wire}.rs`;
  `tests/protocol-llm/tests/`: `endpoints_grants.rs`,
  `component_boundary.rs`, `component_peer.rs`, `client.rs`,
  `adapter_continuation.rs`, `adapter_boundary.rs`,
  `production_contract.rs`, `root_memory.rs`, `history_window.rs`,
  `guard.rs`; `tests/agent/src/wire.rs`;
  `tests/agent-process/src/{fake,lib,world}.rs`,
  `tests/agent-process/tests/memory.rs`;
  `tests/local-process/src/world.rs`; `tests/end-to-end/end_to_end.rs`
  and `tests/end-to-end/support/{mod,exec,live_run,sample_host}.rs`;
  `benchmarks/src/` where it matches `Provider`; the smith files that
  skein 01's and 02's Downstream lists name (all above), and those a
  landed skein 05 or 08 step's Downstream names (decision 13).
Shape: the first commit is the repin alone, as the README's "smith
  takes skein twice" and decision 13 say (`$c` is computed there). It
  repairs, in the same commit, what skein 01 and 02 broke (at most
  about 200 lines): `access_token` becomes `value`;
  `CredentialLimits::value` in `credential_limits` and the tests'
  limits; `Cap::Value` in `failure.rs`'s `cap()` (smith's own
  `llm::Cap` keeps its name) and a word for it in `profile/words.rs`'s
  `cap_name`, "credential-value"; `Some(credential)` in the calls and
  in `guard.rs`'s measures; each `Credential` literal in the tests and
  fakes takes `scheme`; each match on `Provider` gains `OpenAi`. If
  skein 05 or 08 has landed, its Downstream's smith files are repaired
  there too, for example `Event::Removed` in smith-local-service's
  `credentials.rs`. Then: `grants.rs`'s `read()` builds
  `Credential::bearer(value)` for an empty account id and the
  withdrawn `Credential::codex(value, id)` otherwise (the bearer
  first, then the id), until smith 04.1 gives the table its tags;
  agent-service's `credential()` parses as today and feeds it.
  `framing.rs`'s `prompt_allowance` takes `Provider::OpenAi` with
  `Endpoint::openai()`. `config.rs` accepts the provider `openai`,
  building `Endpoint::openai()`; `codex` stays, its doc saying
  withdrawn. `failure.rs`'s `failed()` maps skein's `Refused` and
  `Unsupported` to the session's `Invalid` and `Exhausted` with no
  reset to `Exhausted { retry_after: ZERO }`, and drops the `Report`:
  each until smith 02.1 maps them for good. Downstream: none beyond
  smith.
Test shape: unit, `smith-protocol-llm`'s `tests.rs` (each under
  20 ms): `skein_failures_map_to_the_sessions_classes`, a row per skein
  class, `Refused` and `Unsupported` included; the prompt allowance's
  rows gain `OpenAi`. Unit, `config.rs`:
  `an_openai_endpoint_takes_the_public_route`. Unit, `profile/tests.rs`:
  `a_credential_value_cap_is_worded_credential_value`, the words of
  `Error::Limit { which: Cap::Value, bound: 7 }` end `request-head:
  credential-value bound=7`. World, `component_boundary.rs` (under
  0.5 s, a real client against a fake peer):
  `every_shared_failure_class_keeps_each_evidence_and_detail` gains
  rows for the two classes. World, `endpoints_grants.rs` (under
  0.1 s): `a_withdrawn_pair_keeps_the_bearer_in_value`, a row granting
  `Credential::codex(b"bearer-bytes", b"account-id")` and reading it
  back: `value` is `bearer-bytes`, `account_id` is `account-id`, the
  scheme is bearer.
Proof: world,
  `every_shared_failure_class_keeps_each_evidence_and_detail`: the two
  new classes reach the session as `Invalid` and `Exhausted`; under
  0.5 s; unit, `a_withdrawn_pair_keeps_the_bearer_in_value`: the
  bearer is in `value`; under 20 ms.
Don't: let any other step of this plan but `smith-repin` move the pin;
  call with no credential (smith 03.1's); rename smith's
  `llm::Cap::AccessToken`.
Size: about 180 lines, the repin's repairs included.
Removes: nothing.

## Interface sketches

```rust
// smith_protocol_channel, component/notices.rs (01.2)
// Component::send_unauthorized(name, ..),
//   send_limited(account, reset: Option<Duration>, ..),
//   send_refused(account, code: &[u8], reason: &[u8], ..)

// smith_host_protocol (01.2)
// OpenEvent::Unauthorized { account, generation }
// OpenEvent::Limited { account, reset: Option<Duration> }
// OpenEvent::Refused { account, code, reason }
```

## Done when

- `cargo metadata --format-version 1 --no-deps | jq -e
  '[.packages[].license] | all(. == "MIT")'` succeeds.
- `ls crates/smith-channel/golden` lists `v4` alone, and `ls
  crates/smith-channel/schema` lists `channel-v4.schema` alone.
- `rg -n '0x0113' crates/smith-channel/src/kinds.rs` prints one line:
  the `KindRule` of `Body::AccountRefused`, `FromResponder`,
  `required: true`.
- `rg -n -w 'smith_channel::v3|V3' --glob '*.rs' crates tests` and
  `rg -n -w 'grant_value_credential' --glob '*.rs' crates tests`
  find nothing.
- `rg -n -w 'send_rejected|send_exhausted' --glob '*.rs'
  crates/smith-protocol-channel crates/smith-agent-service/src
  tests/channel/src` finds nothing.
- For the size of the files this plan grows, code-organization.md 8's
  commands print nothing new.
