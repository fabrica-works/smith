# Code and test organization

2026-10-19. Where the live-text plan's code goes: the crates each session
changes, where tests and fixtures live, the owner of each shared
interface, the plan's "Don't" list and its size limits. It binds every
session. The design is the contract; this document adds no behaviour.
Where a session finds the code shaped otherwise on `main`, the code on
`main` wins over a sketch, unless the design asks for the change: report
the difference, and do not reshape landed code for its own sake.

## 1. Crates

No crate is added. Each crate keeps its kind (skein's
`programming-model.md`; smith's `AGENTS.md`): the fake LLM's domain in
the strict subset, its server a protocol layer, the byte peer a world
host; smith's channel halves and LLM component step machines, the agent's
service with its profile. jig and temper are read only.

| Session | Crates and modules |
|---|---|
| skein 01 | `testing/skein-fake-llm-domain` (`src/api.rs`, `src/domain.rs`, a new `src/pieces.rs`, `src/limits.rs`, `src/respond.rs`, a new `src/tests.rs`), `testing/skein-fake-llm-protocol` (`src/provider.rs`, a new `src/pieces.rs`, `src/documents.rs`), `testing/skein-fake-peers` (`src/llm.rs`, `tests/llm.rs`), `tests/llm` (`src/fake.rs`, `tests/fake.rs`, `tests/fake_memory.rs`), `tests/llm-connection` (`src/simulated.rs`, `tests/simulated.rs`: one held story; nothing there matches `Event::Call` or `Request::Reply`) |
| smith 01 | `smith-channel` (`schema/channel-v3.schema`, `src/generated/v3.rs`, `golden/v3`, `src/kinds.rs`, `src/lib.rs`), `smith-host-domain` (`src/channel.rs`, `src/process.rs`, `src/domain.rs`), `smith-host-protocol` (`src/translate.rs`, `src/component.rs`, `src/process.rs`, a new `src/live.rs`), `smith-protocol-channel` (`src/component.rs`, `src/limits.rs`, `src/translate.rs`, a new `src/live.rs`), `smith-protocol-llm` (`src/boundary.rs`, `src/component.rs`, `src/capture.rs`, `src/responses.rs`), `smith-agent-service` (`src/service.rs`, a new `src/live.rs`, `src/sinks.rs`, `src/profile.rs`), `smith-agent-shell` (`src/config.rs`: the memory floor), `smith-local-service` (`src/process.rs`), the start's literals (A28), `tests/codecs`, `tests/channel` (a new `tests/live.rs`), `tests/hosts`, `tests/agent-process` (a new `tests/live.rs`), `tests/protocol-llm` |
| smith 02 | `Cargo.toml` and `Cargo.lock` (the repin of skein), the worlds that build or match the fake LLM's events and requests (A30), `smith-agent-service` (`src/live.rs`, `src/service.rs`, `src/teardown.rs`, `src/profile/quantities.rs`, `src/profile/validate.rs`, `src/profile/effective/list.rs`; no shell's settings, decisions.md 3.6), `tests/agent-process`, `tests/hosts` |

## 2. Naming

- **The design's words** for public types, variants and functions:
  `live`, `LiveAsk`, `Live`, `LiveKind`, `piece`, `retry`, `offset`,
  `hold`, `release`, `Pieces`, `Piece`, `live_interval`,
  `lost_live_bytes`. A public name the design does not use needs a
  reason in the commit body.
- **Units in names** where the type does not carry them: `_bytes`.
- **No generic names** (`Manager`, `Handler`, `Util`, `Helper`, `Data`,
  `Info`).

## 3. Public API

- Explicit re-exports in `lib.rs`; one line of prose per public type and
  variant: what it is, who sends it, its terminal. skein's fake crates
  keep their "Contract:" line on each item, as their neighbours do.
- Design citations at module or type level, never stamped on fields.
- New public API across crates only where a Shape names it: skein's
  `Line::Pieces`, `Piece`, `Event::Release`, `Event::Call`'s `call`,
  `Request::Piece`, `Request::Hold`, `Peer::release`; smith's
  `LiveAsk` and `Start::live`, the generated v3 records, `OpenEvent::Live`,
  `process::Event::Progress`, `FromDomain::Complete`'s `deltas`, the
  channel component's `send_live`, `takes_live`, `lose_live` and
  `lost_live_bytes`, and `Policy::live_interval`.

## 4. Tests

| Tier | Where | Named |
|---|---|---|
| unit | `src/tests.rs` or the module's `tests`, through `step` and `fire` or the component's entry points | the behaviour, `snake_case` |
| joint | skein `tests/llm/tests/fake.rs` (the real `skein-llm` client against the server); smith `tests/hosts` (the real host `Process` and agent service, A31) | sentences of about eight words |
| world | skein `testing/skein-fake-peers/tests/llm.rs`; smith `tests/channel` (both real halves), `tests/agent-process` (the agent process, a scripted host, the byte peer) | sentences of about eight words |
| codecs | smith `tests/codecs/tests/`: goldens, drift, fuzzy decode | |

- **Ceilings per test** (development.md, section 5.6): a unit test under
  20 ms, a focused world test under 0.5 s, a fuzzy test about 5 s, read
  as implementer.md 8.2 says.
- **Budgets:** 15 s focused and 60 s fuzzy, in each repository (its
  `workflow.md`, section 2). The aims are the README's Pulse.
- **One behaviour per test** (testing-strategy.md, section 10).
- **No new fuzzy test:** the stories are focused; the codecs' fuzzy
  decode takes the new record by its kinds' list.

## 5. Fixtures

Each test package reuses its fixtures and extends them; none is defined
twice:
- skein's fake LLM: `respond.rs`' test helpers (`CONFIG`, `user`,
  `query`, `script`, `says`), which skein 01.1 moves to the crate's new
  `src/tests.rs` for both its test kinds; `tests/llm/src/fake.rs`
  (`Exchange`, `limits`, `config`); `testing/skein-fake-peers/tests/
  llm.rs` (`limits`, `make`);
- smith's channel world: `tests/channel/src/lib.rs` (`World`, `Peer`,
  `agent_sends_fact`), `src/referee.rs`, `src/stories.rs`;
- smith's hosts world: `tests/hosts/src/world.rs`, `src/lib.rs`
  (`route_channel`);
- smith's agent-process world: `tests/agent-process/src/world.rs`
  (`World`, `observe_facts`, `check_memory`; smith 01.3 adds `ask` and
  `pieces`, smith 02.1 `script` and `release_after`), `src/lib.rs`
  (`start`, `host`, `charter`), `src/fake.rs` (`configured`);
- smith's protocol-llm world: `tests/protocol-llm/src/`.

Limits come from the shipped profile (development.md 5.6): smith's tests
take `quantities::standard()` and its derivation; skein's take
`tests/llm/src/fake.rs`' `limits` and `config`. A test names any other
value beside the profile's, with its reason (a narrow output window, a
short idle deadline).

## 6. Owners of shared interfaces

The owner defines the interface; the others use it by name.

| Interface | Owner | Used by |
|---|---|---|
| `Line::Pieces`, `Piece`, the turn's rule for them | skein 01.1 | 01.2, 01.3; smith 02 |
| `Event::Call`'s `call`, `Request::Piece`, `Request::Hold`, `Event::Release` | skein 01.1 | 01.2, 01.3; smith 02's repin |
| the server's pieces and its keep-alive at a hold | skein 01.2 | 01.3; smith 02 |
| `Peer::release`, `Exchange::release` | skein 01.3 | smith 02; temper's real loop, later |
| the channel's version 3, `LiveAsk`, `Start::live`, the ask in `DecodedStart` | smith 01.1 | 01.2, 01.3; the accounts' version 4 |
| the `Live` record (0x0112), `send_live`, `takes_live`, `lose_live`, `lost_live_bytes`, the host's order checks, `OpenEvent::Live`, `process::Event::Progress`, the queue's reserve for pieces | smith 01.2 | 01.3, 02 |
| `FromDomain::Complete`'s `deltas`, the sink's gate, `live.rs`, the held bytes in the service's sum; the agent-process world's `ask` and `pieces` | smith 01.3 | 02 |
| `Policy::live_interval` and the service's live deadline; the agent-process world's `script` and `release_after` | smith 02.1 | 02.2 |

## 7. Don't

No increment of this plan does these. A session file and an increment's
Don't line add their own.

- No type beside an existing one for the same thing: change the type in
  place, and its users in the same increment.
- No field or predicate that its counterpart across a boundary lacks:
  the domain's `LiveAsk` and the wire's mirror each other field for
  field, and so do `OpenEvent::Live` and the `Live` record.
- No test that breaks a rule of a test's shape (testing-strategy.md,
  section 10).
- No lint exception in tests, and no new `allow` in production code (the
  README's rule 5 checks both).
- No expected value computed by the code under test.
- None of the words the README's Pulse forbids, in code, names or
  comments, and no synonym in their place.
- No live content in smith's event stream, transcript or domains: a piece
  is neither a fact nor a turn (channel.md 2; host.md 2).
- No producer of reasoning or input pieces (later.md).
- No version bump past the channel's 3, and none of the event
  vocabulary, the transcript or the charter.
- No mechanism kept half-removed: removing one (`Observation.live_text`)
  removes its code, tests and docs outside `docs/design/` in the same
  increment.
- No edit to jig or temper, and no edit to `docs/design/` or to this plan
  by an implementer. skein's increments change skein only; smith's change
  smith only.
- No live call in a test of the suite.

## 8. Size limits

- **A new file:** at most 800 lines, `src/generated/` excepted.
- **A file already over 800 lines** (skein's `provider.rs`; smith's
  `service.rs` and others): aim for growth of at most 100 lines per
  increment, added less removed by `git diff --numstat origin/main...HEAD
  -- <file>`; an increment that needs more moves its concern into the new
  module its Shape names.
- **A test function:** aim for at most 40 lines; nothing stops on it.
- **An increment's size** is its hand-written production lines, counted
  on the rebased branch. In smith:

  ```
  git diff --numstat origin/main...HEAD -- ':(glob)crates/*/src/**/*.rs' ':(glob)src/**/*.rs' ':(glob)benchmarks/src/**/*.rs' ':(exclude,glob)**/generated/**' ':(exclude,glob)**/tests.rs' ':(exclude,glob)**/tests/**' | awk '{ s += $1 } END { print s + 0 }'
  ```

  In skein, where the fakes are production code of `testing/`:

  ```
  git diff --numstat origin/main...HEAD -- ':(glob)crates/*/src/**/*.rs' ':(glob)testing/*/src/**/*.rs' ':(exclude,glob)**/generated/**' ':(exclude,glob)**/tests.rs' ':(exclude,glob)**/tests/**' | awk '{ s += $1 } END { print s + 0 }'
  ```

  Each counts added lines in production sources, without generated code,
  test modules (`tests.rs`, `src/tests/`), the test packages (`tests/`),
  schemas, goldens and Markdown. Each landing's report gives that count,
  and the count by file (`--numstat` without the `awk`) when it passes
  the Size line (development.md 5.3; implementer.md 8.6).
