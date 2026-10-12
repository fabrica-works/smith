# Code and test organization

2026-10-09, revised 2026-10-12. How the reliability plan's code is laid
out: crates and their names, what goes in each kind of crate, naming,
public API, tests and their fixtures, the plan's "Don't" list and its
size limits. It binds every session. Each session file's "Interface
sketches" show the shape of the interfaces that matter most; this
document is the frame around them.

The design is the contract (smith `docs/design/`, skein `docs/`). This
document adds no behaviour; it says where behaviour lives and how it is
named. Where a session finds the code already shaped otherwise, the code
on `main` wins over a sketch, unless the design asks for the change.
Report the difference; do not reshape landed code for its own sake.

## 1. Crates

### 1.1 smith

| Crate | Kind | Holds |
|---|---|---|
| `smith-domain` | domain (root) | the agent's root domain: routes between its children and its parent |
| `smith-domain-run` | domain (child) | one run: charter, opening, messages, budget and reserve, sub-agents, results, delivery, facts |
| `smith-domain-session` | domain (child) | one conversation: turns, affinity, window admission, compaction, retries, tool choice |
| `smith-domain-tools` | domain (child) | the workspace tools: path namespace, list, read, write, edit, search, shell |
| `smith-host-domain` | domain | a spawned agent's supervision: its process face and the parent face shared with the inline agent |
| `smith-inline-agent` | domain | an agent in its host's process: one composed `smith-domain` per run, behind the parent face |
| `smith-local-domain` | domain | the local host: chats, the person's lines and their unread policy, typed notices, owning its agent as a child |
| `smith-charter`, `smith-transcript`, `smith-channel` | codec | the charter, transcript and channel schemas, with goldens |
| `smith-events` | codec | the event vocabulary (protocol/events.md), read and written over skein-json |
| `smith-protocol-llm` | protocol | the LLM component: prompts, tool schemas, decoding, rendering, affinity rendering through skein-llm |
| `smith-protocol-machine` | protocol | the machine component: io paths, processes and groups, searches, file operations |
| `smith-protocol-channel`, `smith-host-protocol` | protocol | the channel's agent half and host half |
| `smith-local-protocol` | protocol | the chat store over skein io, the terminal protocol and its typed notices |
| `smith-agent-service`, `smith-local-service` | service | each service's `iterate`, effects routing, and its profile: declared quantities and their derivation (module `profile`) |
| `smith-agent-shell` | shell | the agent process as a library: the agent configuration document, the `Agent` adapter |
| `smith-local-shell` | shell | the product as a library: commands, settings and presets, `smith check`, credentials, composition, the `Local` adapter, front ends |
| `smith` (root package) | binaries | `src/main.rs` (`smith`) and `src/bin/smith-agent.rs`, glue only |
| `smith-bench` (`benchmarks/`) | harness | the benchmark harness |

No other crate is added without a session file saying so. A concern that
seems to need a new crate goes into the nearest existing one of the right
kind, or is raised.

### 1.2 skein

Where skein's pieces of the plan live; only terminal modes (skein 02.6) are left:

| Where | What |
|---|---|
| `skein-io`, `skein-shell`, `skein-sim` | process groups, append streams, durable replace, private files, usage, terminal modes |
| `testing/skein-world` | `Host`, `drive` and the hook re-exported; the end-to-end kit's process trees (module `tree`, written as the prototype of io's contained trees) |
| `skein-json` | the selective collector: named caps, `Tagged` |
| `skein-http` | the SSE data face |
| `skein-llm` | affinity, per-call headers, `ToolChoice`, `Usage`, blocks, typed failures, `Limits::derive` |
| `skein-llm-connection` | owner close and abort, phases, waiting, the memory pool |
| `skein-oauth-accounts` (oauth.md, section 6) | the OAuth driver component: sign-in, refresh, keeping, access-only records |
| `skein-tls` | configuration from DER |

## 2. What each kind of crate holds

- **Domain** (strict subset, `no_std` outside tests): the shape of
  `smith-host-domain`.
  - `lib.rs`: module docs and explicit re-exports.
  - `boundary.rs`: `Event` (parent → domain) and `Request` (domain →
    parent), with the values they carry.
  - `domain.rs`: `Domain`, `new`, `step`, `fire`, `max_out`.
  - `limits.rs`: `Limits` and `worst_case`.
  - `facts.rs`: content-free facts.
  - One module per concern, each owning its state: `opening.rs`,
    `inbox.rs`, `reserve.rs`, `window.rs`, `compaction.rs`, and the like.
  - `tests.rs`.
- **Codec:** `records.rs` (the typed values), `write.rs` and `read.rs`
  (bounded, checked), `limits.rs`, `tests.rs`. The goldens and drift tests
  live in `tests/codecs`. A record type changes in its codec crate only.
- **Protocol:** a component is a step machine like skein's. `component.rs`
  holds its `Request` down and `Event` up, `limits.rs` its limits; one
  module per translation (`render.rs`, `decode.rs`, `tools.rs`,
  `prompt.rs`). A protocol crate decides nothing the domain owns
  (programming-model.md, section 4).
- **Service:** `service.rs` holds `iterate` and the routing between the
  domain, the components and io; `profile.rs` (or `profile/`) holds the
  declared quantities, the presets' structure and the derivation of
  every layer's limits, with its startup checks named as in
  protocol/limits.md §4.
- **Shell** (ordinary Rust under the shell bans): no threads, no blocking
  io, no clocks. Typed command and settings values, with their parsers
  under step tests; composition; adapters over `skein_shell::Host`; front
  ends that read `smith-events` records.
- **Binaries:** parse into a shell crate's command type, compose, drive,
  map the result to an exit code. About 60 lines at most each, and no
  logic that a test would want to reach.
- **Harness** (`benchmarks/`, `tests/*`): ordinary Rust under its own
  `clippy.toml`.

## 3. Naming

- **Crates:** `smith-<area>` or `smith-<layer>-<area>`, as today. Worlds
  are `smith-<name>-world` in `tests/<name>/`.
- **Vocabulary from the design.** Types and variants take the design's
  words: `Start`, `Opening`, `Awaiting`, `Fence`, `Unread`, `Reserve`,
  `WindDown`, `Share`, `Affinity`, `Window`, `Compaction`, `Declared`,
  `Profile`, `Teardown`, `Exit`, `Forced`. A word the design does not use
  needs a reason.
- **Units in types, not names.** Use `Duration`, and newtypes such as
  `Tokens(u64)` and `Bytes(u64)` where a bare integer could be confused,
  not `timeout_ms: u64`. Wire and config fields may carry units in their
  names (`teardown_ms`), because their readers see no types.
- **Typed outcomes.** One enum per terminal, named for what it ends
  (`Answer`, `Ended`, `Refusal`, `Failure`), never `Result<_, String>` or
  a bool that a variant would explain. Errors that name a relationship
  carry it typed (`Check::GraceOverTeardown`), not as text.
- **No generic names:** no `Manager`, `Handler`, `Util`, `Helper`, `Data`
  or `Info`. A module is named for its concern, a function for its
  effect.

## 4. Public API

- **Explicit re-exports** in `lib.rs`; no `pub use x::*`.
- **One line of prose** for each public type and variant: what it is, who
  sends it, and its terminal where it has one. A field is documented when
  its name and type do not say it (AGENTS.md).
- **Design citations** at module or type level (`domain/run.md, section
  6`), never stamped on fields.
- **Smallest surface.** Only what a sibling or a consumer needs is public;
  a parent reaches a child through its boundary, never its internals.
- **APIs break freely** inside smith and skein (the user, 2026-10-09).
  Consumers outside them (temper, jig) are fixed separately
  (decisions.md, 12.8).

## 5. Tests

| Tier | Where | Named |
|---|---|---|
| unit | `crates/<crate>/src/tests.rs`, through `step` and `fire` (domains) or the component's entry points | `snake_case` behaviour |
| world stories | `tests/<world>/tests/simulation.rs` | sentences: `a_follow_up_sent_while_finishing_is_read_by_the_next_activation` |
| referee negatives | `tests/<world>/tests/referee.rs` | the rule broken |
| memory | `tests/<world>/tests/memory.rs`: one test attaining `worst_case` | |
| fuzzy | `tests/<world>/tests/fuzzy_*.rs`, run by the fuzzy profile | |
| codecs | `tests/codecs`: goldens, drift, bounds | |
| end to end | `tests/end-to-end/`, root package `[[test]]` targets | sentences |
| live | `tests/end-to-end/live*.rs`, the `live` profile | sentences |
| benchmarks offline | `benchmarks/tests/` | |
| benchmarks live | `benchmarks/tasks/`, run by `smith-bench` | probe names (benchmarks.md §11) |

- **The lowest tier that shows a failure** gets the test (testing.md
  §2.5). A boundary is tested where the limit lives, often in skein.
  Derivation properties replace pinned constants.
- **A world needs a story per new behaviour.** Behaviour across a
  boundary (host and run, parent and child) gets a world story, not
  only unit tests.
- **Shared harness code** lives in one world crate's library, which the
  others take (`smith-local-world`, `smith-agent-process-world`). Generic
  harness code goes to skein first.
- **Budgets:** 15 s focused and 60 s fuzzy per repository. A new world
  takes at most about 1 s focused and 5 s fuzzy; more is a question, not
  a choice (development.md, section 5.3).
- **Ceilings per test** (development.md, section 5.6): a unit test under
  20 ms, a focused world test under 0.5 s, a fuzzy test about 5 s, read
  from the times nextest prints in the last focused run.

## 6. Fixtures

Each test package reuses the fixtures that exist, and extends them; it
never defines them again (development.md, section 5.2):

- **smith's worlds** share harness code through one world crate's
  library, which the others take: `smith-local-world` (`tests/local`)
  and `smith-agent-process-world` (`tests/agent-process`, with its
  `fixtures/`).
- **Recorded provider traffic** for the LLM component is in
  `tests/protocol-llm/fixtures/`; fakes are shaped by real captures
  (`docs/development/tech-debt.md`).
- **The harness's offline tier** keeps its recorded outputs in
  `benchmarks/tests/fixtures/`.
- **A domain crate's unit tests** build their values in its
  `src/tests.rs`, or a `src/tests/` module where the session file names
  one. A helper wanted by a second crate moves to the world library that
  both take, or to skein's test kit if it is generic.

A session file's Test shape names the fixtures to reuse by `file:line`.

## 7. Don't

No increment of this plan does these. A session file adds its own; an
increment adds its own in its Don't line.

- No type beside an existing one for the same thing: change the type in
  place, and its users in the same increment.
- No field or predicate that its counterpart across a boundary lacks.
- No test that checks several behaviours under one name.
- No lint exception in tests, and no new `too_many_lines` or other
  `allow` in production code.
- No expected value computed by the code under test.
- None of the words the README's Pulse forbids, in code, names or
  comments; and no synonym put in their place.
- No second mechanism for something the design gives one owner (the
  environment, the path namespace, the profile's derivation, the event
  sink): extend the owner.
- No mechanism kept half-removed: removing one removes its code, limits,
  memory accounting, facts, tests and docs, in this repository and in the
  same increment.
- No `Debug` text in traces or events, no placement switch in the local
  host, no static endpoint cache key, no hand-rolled run loop beside
  skein's `drive`: the first run removed them, and none comes back.
- No edit to jig or temper, and no edit to `docs/design/` or to this
  plan by an implementer.
- No live call in a test of the suite: live runs are probes, through
  `smith-bench`, and the live suite's own profile (workflow.md, section 4).
- No reading of, or writing to, the user's `~/.codex`, `~/.claude`,
  `$XDG_CONFIG_HOME/smith` or `$XDG_STATE_HOME/smith` from a test.

## 8. Size limits

- **A new file:** at most 800 lines, generated code under
  `src/generated/` excepted.
- **A file already over 800 lines** (53 in smith on 2026-10-12, such as
  `smith-domain-session/src/session.rs`): aim for growth of at most 100
  lines per increment. An increment that needs more moves its concern
  into a module of its own, and says so in its commit body.
- **A test function:** at most 40 lines.
- **An increment:** at most the production lines its Size line gives;
  about 600 at most, above which the plan splits it.

## 9. Where the remaining work goes

Each session file's Files lines name the files on `main`; this is the
map.

| Session | Crates and modules |
|---|---|
| smith 00 | `benchmarks/` (`smith-bench`) only |
| smith 03 | `smith-local-service` and `smith-local-domain` (the product's stream, either kind), `smith-events` (`chat`), `smith-inline-agent`, `tests/local-process` |
| smith 04 | `smith-agent-service::profile`, `smith-local-service::profile`, each layer's `limits.rs`, `smith-events` (the effective limits' names), `smith-agent-shell` (the header) |
| smith 05 | `smith-local-domain`, `smith-local-protocol`, `tests/local` (the reacting person, its referee) |
| smith 06 | `smith-agent-service` (the teardown phase), `smith-host-protocol` and `smith-host-domain` (exit evidence), `smith-events` (`words`, notices), `smith-domain-session` and `smith-domain-run` (kept detail), `smith-local-domain` (failures in words), `smith-agent-shell` (the stderr line) |
| smith 07 | `smith-domain-run` (the writing-step budget), `smith-events` (`budget`'s `call`) |
| smith 08 | `smith-domain-session` (`window.rs`, `compaction.rs`), `smith-domain-run`, `smith-local-domain` and `smith-local-protocol` (windows resumed) |
| smith 10 | `smith-local-shell` (commands, settings, presets, `check`, credentials, front ends), `smith-local-protocol` (the store over io), root `src/` |
| smith 12, 13 | `benchmarks/` only, and `docs/development/benchmarks/` for 12's report |
| skein 02 | `skein-io`, `skein-shell`, `skein-sim` (terminal modes), `testing/skein-world` |

## 10. Owners of shared interfaces

The owner defines the interface; the others use or extend it by name.

| Interface | Owner | Used or extended by |
|---|---|---|
| `smith-bench`'s `build`, `probe`, `calibrate`, `fix` and `suite` | smith 00.27 | 05.13, 10.8, 10.15, sessions 12 and 13; 12.1 adds `suite --include` |
| `smith-bench validate` | smith 00.13 | 12.1, 12.2 |
| `[[person]]` and `input_end` in tasks | smith 00.28 | 00.14 |
| `suites/probes.toml` | smith 00.29 | 00.14, 00.16, 00.30, 10.15; session 13 reads it |
| `budget-told` (`budget_report.rs`) | smith 00.17 | 13.7 |
| the `budget` record's `call` | smith 07.14 | 00.17 |
| `Answered`, the product stream's `Effects` methods | smith 03.7 | 03.8, 03.9, 06.8, 10.5 |
| `profile::effective` and `header` | smith 04.7 | 04.21, 04.22, 10.14 |
| `smith_local_domain::Notice` | smith 05.11 | 06.9, 10.6 |
| `End::Exited`, `Exit`, `Forced` | smith 06.1 | 03.10, 06.9 |
| `smith_events::words` and `clip` | smith 06.2 | 06.9, 10.5 |
| `Teardown`, `Close`, `Step` | smith 06.4 | 06.8 |
| the chat store over io (`smith_local_protocol::Store`) | smith 10.1 | 08.2 (the window flag and bound) |
| `compose::AgentKind` | smith 10.5 | 10.16 |
| terminal modes (`raw_input`, `Kept`, `Resized`, `Window`) | skein 02.6 | smith 10.10 |
| the result and summary formats' version 2 | README, rule 5 | 00.21, 12.9, 12.10 |
