# Code and test organization

2026-10-19. Where the accounts plan's code goes: the crates each session
changes, where tests and fixtures live, the owner of each shared
interface, the plan's "Don't" list and its size limits. It binds every
session. The design is the contract; this document adds no behaviour.
Where a session finds the code shaped otherwise on `main`, the code on
`main` wins over a sketch, unless the design asks for the change: report
the difference, and do not reshape landed code for its own sake.

## 1. Crates

No crate is added. The crates keep their kinds (skein's
`programming-model.md`; smith `AGENTS.md`): domains in the strict
subset, protocol components as step machines, services with their
profile, and the test harnesses as ordinary Rust under their own
`clippy.toml`. The dependencies added are all in the workspace already:
`skein-oauth` on `skein-tls`, for the ID token's RS256 check (skein
07.2, over 05.1's `verify_rs256`); `skein-fake-oauth` on `rustls` with
its `ring` provider, the workspace's, and `skein-tls` as a
dev-dependency, to sign its test key's tokens (skein 06.2); and
`tests/oauth` on `skein-llm` and `skein-llm-world` as dev-dependencies,
for the issuer's tokens at the LLM peer (skein 11). No crate joins
`Cargo.lock`.

| Session | Crates and modules |
|---|---|
| skein 01 | `LICENSE`, the root `Cargo.toml`'s `[workspace.package]` and every member's manifest (01.1); `skein-llm` (`types.rs`, `client.rs`, `dialect.rs`, `derive.rs`, `openai/common.rs`, `openai/response.rs`, `anthropic/`), `skein-llm-connection` (`endpoint.rs`, `component.rs`, `limits.rs`, `boundary.rs`); `tests/llm`, `tests/llm-connection` |
| skein 02 | `skein-llm` (`types.rs`, `derive.rs`, `translate.rs`, `client.rs`, `openai/`: the public route beside the Codex route, the replay tag); `tests/llm` |
| skein 03 | `testing/skein-fake-llm-domain` (`api.rs`, `limits.rs`, `respond.rs`, `domain.rs`), `testing/skein-fake-llm-protocol` (`provider.rs`, `documents.rs`), `skein-llm`'s error writer (the `param`); `tests/llm` |
| skein 04 | `testing/skein-fake-llm-protocol` (`provider.rs`), `testing/skein-fake-peers/src/llm.rs`, `tests/llm/src/fake.rs`; `tests/llm/tests/fake.rs`, `fake_memory.rs`, `testing/skein-fake-peers/tests/llm.rs` |
| skein 05 | `skein-tls` (an RS256 check), `skein-oauth` (`machine.rs`, `record.rs`, `documents.rs`, `redirect.rs`, `common.rs`, the limits); `tests/oauth`, `tests/tls` |
| skein 06 | `testing/skein-fake-oauth`, `testing/skein-fake-peers/src/oauth.rs`; `tests/oauth`, `tests/oauth-accounts` |
| skein 07 | `skein-oauth` (the phase, `HttpRequest`, `jwt.rs`, the failures); `tests/oauth` |
| skein 11 | `testing/skein-fake-peers` (`src/oauth.rs`, `src/oauth/route.rs`, `src/lib.rs`'s doc); `tests/oauth` (with its `Cargo.toml`) |
| skein 08 to 10 | `skein-oauth-accounts` (`component.rs`, `component/private.rs`, `sign_in.rs`, `listener.rs`, `exchange.rs`, the limits); `tests/oauth-accounts` |
| smith 01 | `LICENSE`, the root manifest and every member's (01.1); `smith-protocol-llm` (`grants.rs`, `framing.rs`, `types.rs`, `endpoints.rs`, `component.rs`, `guard.rs`, `limits.rs`, `failure.rs`), `smith-agent-service`, `smith-agent-shell/src/config.rs`; `smith-channel` (schema, `generated/`, golden, `kinds.rs`, `lib.rs`), `smith-protocol-channel`, `smith-host-protocol` (`translate.rs`); `tests/codecs`, `tests/channel`, `tests/protocol-llm`, `tests/agent` |
| smith 02 | `smith-domain-session` (`llm.rs`, `session.rs`), `smith-domain-run`, `smith-domain` (`route.rs`, `translate.rs`), `smith-host-protocol`, `smith-host-domain` (`channel.rs`, `domain.rs`), `smith-inline-agent`, `smith-local-domain`, `smith-local-protocol`, `smith-local-shell`; `tests/session`, `tests/run`, `tests/agent`, `tests/channel` |
| smith 03 | `smith-charter`, `smith-domain-run/src/charter.rs`, `smith-agent-shell` (the endpoint's `account`), `smith-transcript`, `smith-domain-session` (`record.rs`, `compaction.rs`), `smith-protocol-channel` (`transcript.rs`, `translate.rs`), `smith-local-shell/src/check.rs`; `tests/codecs`, `tests/agent`, `tests/session` |
| smith 04 | `smith-protocol-channel` (`credentials.rs`), `smith-host-protocol`, `smith-agent-service`, `smith-protocol-llm` (`grants.rs`, `endpoints.rs`), `smith-agent-shell/src/config.rs`, `smith-local-service` (the lending), `smith-local-shell` (`settings/resolve/`); `tests/agent-process`, `tests/protocol-llm`, `tests/channel`, `tests/hosts` |
| smith 05 | `smith-events` (`records.rs`, `read.rs`, `write.rs`, `limits.rs`, `words.rs`, `golden/v4/`), `smith-agent-service` (`answers.rs`, `events.rs`, `responses.rs`, `sinks/`), `smith-local-service/src/stream.rs`, `smith-local-protocol/src/terminal/failure.rs`, `smith-local-shell`, `smith-agent-shell/src/config.rs`; `tests/codecs`, `benchmarks` |

## 2. Naming

- **The design's words** for public types, variants and functions:
  `Scheme`, `Bearer`, `Key`, `Credential`'s `value`, `Refused`,
  `Unsupported`, `Report`, `Provider::OpenAi`, `Terms`, `Revoke`,
  `Revoked`, `AwaitKeys`, `Unverified`, `NotGranted`, `AnotherSubject`,
  `Busy`, `want`, `lock`, `unlock`, `locked`, `Limited`. A public name
  the design does not use needs a reason in the commit body.
- **smith's model refusal is `ModelFault::Declined`** and an account's
  refusal is `ModelFault::Refused` (decisions.md, 3).
- **Withdrawn forms** keep their names and say "withdrawn" in their
  doc comment, with the plan that removes them (decisions.md, 2).
- **Units in types:** `Duration` for resets and the lock wait; wire
  fields keep their units in their names.
- **No generic names** (`Manager`, `Handler`, `Util`, `Helper`, `Data`,
  `Info`).

## 3. Public API

- Explicit re-exports in `lib.rs`; one line of prose per public type and
  variant: what it is, who sends it, its terminal.
- Design citations at module or type level, never stamped on fields.
- New public API across crates only where a Shape names it.

## 4. Tests

| Tier | Where | Named |
|---|---|---|
| unit | `crates/<crate>/src/tests.rs` (or the module's `tests`), through `step` and `fire` or the component's entry points | the behaviour, `snake_case` |
| world | skein `tests/llm`, `tests/llm-connection`, `tests/oauth`, `tests/oauth-accounts`, `tests/tls`; smith `tests/protocol-llm`, `tests/session`, `tests/run`, `tests/agent`, `tests/channel` | sentences of about eight words |
| codecs | smith `tests/codecs/tests/`: goldens and drift | |
| fake peers | skein `testing/skein-fake-peers/tests/` | sentences |

- **Ceilings per test** (development.md, section 5.6): a unit test under
  20 ms, a focused world test under 0.5 s, a fuzzy test about 5 s, read
  from the times nextest prints in the last focused run. A Proof line
  bounds its test tighter: a world in memory under 0.1 s, one with a
  real client or several processes under 0.5 s.
- **Budgets:** 15 s focused and 60 s fuzzy per repository, which each
  gate enforces. The aims are the README's Pulse.
- **One behaviour per test;** cases of one behaviour are rows of one
  table test (testing-strategy.md, section 8).
- **No live call.** This plan has no live suite (decisions.md, 10): the
  public route's codes are the design's, rendered by the fake LLM.

## 5. Fixtures

Each test package reuses its fixtures and extends them; none is defined
twice:
- skein's LLM world: `tests/llm/src/fake.rs` (`Exchange`, its
  `extra_worst_case`), which skein 04.4 moves to a list of credentials;
- skein's connection world: `tests/llm-connection/src/world.rs`;
- skein's OAuth worlds: `tests/oauth/src/lib.rs`,
  `tests/oauth-accounts/src/world.rs` and `sign_in.rs`;
- the fake issuer's RSA key: one fixture in `testing/skein-fake-oauth`
  (06.2), a fixed key pair generated once and checked in, never
  generated in a test;
- smith's codecs: `tests/codecs/` goldens, one per format version;
- smith's agent and session worlds: `tests/agent/src/wire.rs`,
  `tests/session/src/world.rs`.

## 6. Owners of shared interfaces

The owner defines the interface; the others use it by name.

| Interface | Owner | Used by |
|---|---|---|
| `skein_llm::Scheme` and `Credential { value, scheme, account_id }` | skein 01.2 | 01.5, 02.1, 04.1, 04.2, smith 01.3 |
| `skein_llm::Failure`'s `Refused`, `Unsupported`, `Exhausted`'s optional reset, and `Report` | skein 01.3 | 01.4, 02.2, 02.3, 03.2, smith 01.3, 02.1 |
| `REPORT_BYTES` and `ProviderError.param` | skein 01.4 | 03.2 |
| the optional `Call.credential` and start credential | skein 01.5 | 02.3, 04.4, smith 01.3, 03.1 |
| `Provider::OpenAi`, `Endpoint::openai`, the route's limits, replay tag 3 | skein 02.1 | 02.3, 04.1, 04.3, smith 01.3 |
| the fake LLM's `Line::Fail`, `Query.credential` and `api::Failure` | skein 03.1 | 03.2, 04.2, 04.4 |
| the fake protocol's `provider::route` | skein 04.1 | 04.2, 04.3 |
| the fake peer's credential list, `Presented`, `MAX_CREDENTIALS` and `Observation::Credential` | skein 04.2 | 04.4, the repins |
| the Exchange's credential list and `Exchange.presented` | skein 04.4 | 11.1, the repins |
| the RS256 check | skein 05.1 | 07.2 |
| the OAuth record's version 3, `SavedToken`'s and `TokenResponse`'s fields | skein 05.2 | 05.4, 06.1, 08 to 10 |
| `Registration`'s terms | skein 05.3 | 05.4, 06.1, 08.4, 10.2 |
| `Revoke` and `Revoked` | skein 05.5 | 06.1, 09.1 |
| the fake issuer's RSA key, ID token, key set and metadata | skein 06.2 | 07.1, 07.2, 10.2, 11.1 |
| `AwaitKeys` and GET requests | skein 07.1 | 07.2, 10.2 |
| the driver's named account table | skein 08.1 | 08.2 to 10.2 |
| the driver's lock and its `lock_wait` | skein 09.3 | 10.1 |
| the channel's version 4, its fault and class tags (`Unauthorized` and `Program` among them) | smith 01.2 | 02.1, 02.2, 04.1, 05.2 |
| the session's `Failure::Invalid` for skein's `Refused` and `Unsupported`, until 02.1 | smith 01.3 | 02.1 |
| the grant value union | smith 04.1 | 02.2, 03.1 |
| the dialects' numbers | smith 04.2 | 03.2 |
| the session's `Failure::{Refused, Unsupported}` and `llm::Report`; the run's `Fault::{Declined, Refused, Limited}` | smith 02.1 | 02.2, 02.3, 05.2 |
| the host's `ModelFault` and the local host's notices | smith 02.3, 02.4 | 05.2 |
| the charter's version 3 | smith 03.1 | 03.2 |
| the transcript's version 6 | smith 03.2 | 03.3, 05.1 |

## 7. Don't

No increment of this plan does these. A session file and an increment's
Don't line add their own.

- No type beside an existing one for the same thing: change the type in
  place, and its users in the same increment.
- No field or predicate that its counterpart across a boundary lacks.
- No test that checks several behaviours under one name.
- No lint exception in tests, and no new `allow` in production code.
- No expected value computed by the code under test.
- None of the words the README's Pulse forbids, in code, names or
  comments, and no synonym in their place.
- No second reader of an older format version: each format reads one
  version, except the OAuth record, whose version 2 is read as the
  design says (oauth.md, section 4).
- No withdrawn form removed: that is the second part's (decisions.md, 2).
- No mechanism kept half-removed: removing one removes its code, limits,
  memory accounting, facts, tests and docs outside `docs/design/`, in the
  same increment.
- No edit to the repositories downstream of smith outside the closing
  repins, and no edit to `docs/design/` or to this plan by an
  implementer.
- No credential value in a log, an observation, a `Debug` or a test's
  expected output; tests read routes, schemes and indices.
- No reading of, or writing to, the user's `~/.codex`, `~/.claude`,
  `$XDG_CONFIG_HOME/smith` or `$XDG_STATE_HOME/smith` from a test.
- No RSA key generated in a test, and no randomness from outside the
  seeded generator in a world's trace.
- No `Cargo.lock` merged by hand.
- No optimized dependencies in a test build, and no budget raised.

## 8. Size limits

- **A new file:** at most 800 lines, `src/generated/` excepted.
- **A file already over 800 lines** (skein's `client.rs`, `translate.rs`,
  `openai/response.rs`, `skein-oauth-accounts/src/component.rs`,
  `skein-llm-connection/src/component.rs`, the fake protocol's
  `provider.rs`, `tests/llm/tests/fake.rs`, `tests/oauth-accounts/src/world.rs`;
  smith's `session.rs`, `run.rs`, `service.rs`, `route.rs`,
  `smith-events/src/write.rs` and `read.rs`, `smith-protocol-channel`'s
  `component.rs`, and others): aim for growth of at most 100 lines per
  increment; an increment that needs more moves its concern into a module
  of its own, and says so in its commit body.
- **A test function:** aim for at most 40 lines; past it, the implementer
  moves the setup into a named helper of its file.
- **An increment's size,** in hand-written production lines (tests,
  generated code and goldens aside), at its end, by file, against its
  Size (development.md, 5.3):

  ```
  $ git diff --numstat origin/main...HEAD -- 'crates/*.rs' 'testing/*.rs' \
      'benchmarks/*.rs' ':!*/generated/*' ':!*tests.rs' ':!*/tests/*'
  ```

- **The long files,** at a step's end: each file it changed that is over
  800 lines, its length and its growth. The growth is at most about 100,
  and no file crossed 800 in the step:

  ```
  $ git diff --numstat origin/main...HEAD -- '*.rs' ':!*/generated/*' \
      | awk '{ n = 0; c = "wc -l < " $3 " 2>/dev/null"; c | getline n;
        close(c); if (n > 800) print $3, n, $1 - $2 }'
  ```
