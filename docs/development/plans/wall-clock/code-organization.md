# Code and test organization

2026-10-16. Where the wall-clock plan's code goes: the crates each
session changes, where tests and fixtures live, the owner of each shared
interface, the plan's "Don't" list and its size limits. It binds every
session. The design is the contract; this document adds no behaviour.
Where a session finds the code shaped otherwise on `main`, the code on
`main` wins over a sketch, unless the design asks for the change: report
the difference, and do not reshape landed code for its own sake.

## 1. Crates

No crate is added. smith's crates keep their kinds (smith `AGENTS.md`;
skein's `programming-model.md`): domains in the strict subset, protocol
components as step machines, services with their profile, shells, and
the harness as ordinary Rust under its own `clippy.toml`. skein, jig and
temper are read only.

| Session | Crates and modules |
|---|---|
| smith 01 | `benchmarks/`: `src/result.rs`, `src/tokens.rs`, `src/run.rs`, `src/summary.rs`, `src/parity.rs`, `src/agent/smith/events/` (`usage.rs`, `passive.rs`), `src/agent/codex/rollouts.rs`, `suites/parity.toml`, `summaries/`, `baselines/`, `tests/`, `README.md`; `crates/smith-local-shell/src/presets.rs` (read, `list_prices`) |
| smith 02 | `benchmarks/`: `src/suite.rs`, `src/probe.rs`, `src/main.rs`, `src/summary.rs`, `src/parity.rs` and a new `src/parity/change.rs`, `src/check.rs`, `suites/parity-change.toml`, `tests/` |
| smith 03 | `smith-agent-service` (`profile.rs`, `profile/room.rs`, `profile/validate.rs`, `profile/quantities.rs`, `profile/checks.rs`), `smith-domain-run` (`limits.rs`, `charter.rs`, the session's open), `smith-domain-session` (`limits.rs`, its open), `smith-events` (`VERSION`), `smith-local-shell` (`local_host/memory.rs`, `settings/tests.rs`), `smith-agent-shell` (`config.rs` tests), `benchmarks/src/agent/smith/events/` (`threshold.rs`), `benchmarks/tests/recorded/` |
| smith 04 | `smith-domain-session` (`llm.rs`, `session.rs`, `window.rs`), `smith-domain` (`llm.rs`, `peer.rs`), `smith-protocol-llm` (`component.rs`, `failure.rs`, a new `guard.rs`), `tests/protocol-llm`, `tests/session` (`src/world.rs`, `src/window_referee.rs`, `tests/fuzzy_simulation.rs`) |
| smith 05 | `tests/agent/` (`tests/fuzzy_memory.rs`'s driver, moved into `src/memory.rs`, and a new `tests/memory_returns.rs`) |
| smith 06 | `benchmarks/summaries/` |
| smith 07 | `smith-domain-session` (`window.rs`, `compaction.rs`, `session.rs`), `tests/session` (`src/window_referee.rs`, `tests/compaction.rs`, `tests/fuzzy_simulation.rs`) |
| smith 08 | `smith-domain-session` (`session.rs`, `llm.rs`), `smith-domain` (`llm.rs`, `peer.rs`), `smith-transcript` (schema, generated, golden), `smith-protocol-channel` (`turn.rs`, `transcript.rs`), `smith-protocol-llm` (`prompt.rs`, `descriptions.rs`, `tests.rs`), `smith-local-shell/src/role.rs`, `smith-local-domain/src/person.rs`, `tests/codecs`, `tests/session`, `tests/protocol-llm` |
| smith 09 | `smith-domain-session/src/session.rs`, `smith-domain-tools` (`boundary.rs`, `jobs.rs`), `smith-protocol-machine/src/process/jobs.rs`, `smith-domain/src/operation.rs`, `smith-protocol-llm/src/descriptions.rs`, `smith-local-shell/src/role.rs`, the job fakes of `tests/tools`, `tests/session`, `tests/agent`, `tests/protocol-llm` |
| smith 10 | `smith-local-shell/src/role.rs` |
| smith 11 | `benchmarks/summaries/`, `benchmarks/baselines/` |

## 2. Naming

- **The design's words** for public types, variants and functions:
  `line`, `spare`, `Guard`, `compaction_material`, `NotRun`. A public
  name the design does not use needs a reason in the commit body.
- **Units in names** where the type does not carry them: `_tokens`,
  `_bytes`.
- **No generic names** (`Manager`, `Handler`, `Util`, `Helper`, `Data`,
  `Info`).

## 3. Public API

- Explicit re-exports in `lib.rs`; one line of prose per public type and
  variant: what it is, who sends it, its terminal.
- Design citations at module or type level, never stamped on fields.
- New public API across crates only where a Shape names it: the run's
  `Limits::compaction_material` (03.1), the session's `Guard` and
  `Purpose::Ordinary { guard }` (04.1), the not-run why after a failed
  write (08.1), the tools domain's ended wait (09.1).

## 4. Tests

| Tier | Where | Named |
|---|---|---|
| unit | `crates/<crate>/src/tests.rs` (or the module's `tests`), through `step` and `fire` or the component's entry points | the behaviour, `snake_case` |
| world | `tests/<world>/tests/*.rs`: tools, session, run, machine, protocol-llm, agent | sentences of about eight words |
| codecs | `tests/codecs/tests/`: goldens and drift | |
| harness offline | `benchmarks/tests/*.rs`, `benchmarks/src/tests/`, with recorded outputs in `benchmarks/tests/recorded/` | sentences |
| live | `benchmarks/tasks/`, run by `smith-bench` | suite and probe names |

- **Ceilings per test** (development.md, section 5.6): a unit test under
  20 ms, a focused world test under 0.5 s, a fuzzy test about 5 s, read
  from the times nextest prints in the step's dependents' run (README,
  rule 6), in place of implementer.md 8.2's "last focused run".
- **Budgets:** 15 s focused and 60 s fuzzy (smith `workflow.md`, section
  2), which the gate enforces. Aim for the whole run to add at most 1 s
  to the focused suite and 5 s to the fuzzy one; the hourly checks follow
  the gates' times, and past the aim the supervisor adds one step that
  makes the three slowest tests the plan added, by the latest gate's
  nextest times, cheaper (development.md 5.3).
- **One behaviour per test** (README, rule 18).

## 5. Fixtures

Each test package reuses its fixtures and extends them; none is defined
twice:
- the session world: `tests/session/src/world.rs` (the simulation),
  `tests/session/src/recorded.rs` (`opening`, `USAGE`, `written_calls`,
  `job_acts`), `tests/session/src/fixture.rs`;
- the tools world: `tests/tools/src/world.rs` (`Step`), `world/jobs.rs`,
  `calls.rs`, `fixture.rs`;
- the agent world: `tests/agent/src/fixture.rs`, `src/script.rs`,
  `src/world/jobs.rs`; 05.1 moves `tests/fuzzy_memory.rs`'s driver into
  `src/memory.rs`;
- the protocol-llm world: `tests/protocol-llm/src/`;
- the harness: `benchmarks/tests/common/` (`parity.rs`),
  `benchmarks/tests/recorded/` (smith streams, Codex rollouts,
  `parity/on-par.json`).

Limits come from the shipped profile (development.md 5.6): a test takes
`quantities::standard()` and its derivation, and names any other value
beside the profile's, with its reason.

## 6. Owners of shared interfaces

The owner defines the interface; the others use it by name.

| Interface | Owner | Used by |
|---|---|---|
| result and summary version 4; each response priced at its prompt's tier | smith 01.1 | 01.2, 01.3, 02, 06 |
| `AttemptRow`'s `responses`, `peak_rss_bytes` and `peak_rss_scope` | smith 01.2 | 02.2, 06 |
| the `parity` suite's ten `shell-cwd` attempts and O over each arm's attempts | smith 01.3 | 02.2 (the same criteria over two smith arms), 06, 11 |
| `suite --attempts <n>`, its summary in the run's directory | smith 01.3 | 06.1 |
| a suite's change arms (`change = true`, `--parent`) | smith 02.1 | 02.2, 07.1 to 10.1 |
| the `change` section of a `parity-change` summary, and its criteria over a reference chosen by commit | smith 02.2 | 02.3 |
| the projection, the stop, `--baseline-run` and `--landed`, `suite parity-change`'s exit, `smith-bench change` | smith 02.3 | 06.1, 07.1 to 10.1, the README's "Done when" |
| the event vocabulary's version 3; `run.compaction_material`; each conversation's `line` | smith 03.1 | 03.2, 03.3, 04.1, 07.1 |
| the history's bytes from the line, and `memory` raised | smith 03.2 | 07.1 |
| the `history` probe's condition at the line (`threshold::passed_line`) | smith 03.3 | 07.1 |
| `Guard`, `Purpose::Ordinary { guard }`, the guard's refusal | smith 04.1 | 04.2, 07.1 |
| the session world's tokens counted from bytes, its rendered sizes and ratios | smith 04.2 | 07.1 |
| the agent world's memory driver (`tests/agent/src/memory.rs`) | smith 05.1 | 05.2 |
| the transcript's version 5; `NotRun::AfterFailedWrite` | smith 08.1 | 09.1 |
| the tools domain's ended wait (`Event::EndWait`, the machine's `JobAct::Hasten`) | smith 09.1 | — |
| the task role (`smith_local_shell::role::TASK_ROLE`) | on `main` | 08.1 (topics 3, 4), 09.1 (topics 3 and 5), 10.1 (topic 2) |
| the tools' descriptions (`smith-protocol-llm/src/descriptions.rs`) | on `main` | 08.1 (the batched change, a failed write), 09.1 (`job` beside reads) |

## 7. Don't

No increment of this plan does these. A session file and an increment's
Don't line add their own.

- No type beside an existing one for the same thing: change the type in
  place, and its users in the same increment.
- No field or predicate that its counterpart across a boundary lacks.
- No test that checks several behaviours under one name.
- No lint exception in tests, and no new `allow` in production code (the
  README's rule 7 checks both).
- No expected value computed by the code under test.
- None of the words the README's Pulse forbids, in code, names or
  comments, and no synonym in their place.
- No second mechanism for something the design gives one owner: the
  guard is the bound, and the session's check is an estimate; no other
  layer refuses a request for its size.
- No tokenizer, no provider count endpoint, no `edit` over several files,
  no patch tool (decisions.md, sections 2 and 6, "Not taken").
- No change to `window` (272,000 for `gpt-6.1-sol`), to
  `compaction_threshold` (4/5), to `job_wait`'s default or to
  `read_window` (decisions.md 1.2, 1.5, 7 and 8).
- No change to a criterion, a margin, the price basis, an agent's "as it
  ships" or the working tier: each is a question for the designer.
- No shaving of a part's memory accounting to fit `memory`: room is made
  by raising it (decisions.md, section 9).
- No mechanism kept half-removed: removing one removes its code, limits,
  memory accounting, facts, tests and docs outside `docs/design/`, in the
  same increment.
- No edit to jig, temper or skein, and no edit to `docs/design/` or to
  this plan by an implementer.
- No live call in a test of the suite.
- No reading of, or writing to, the user's `~/.codex`, `~/.claude`,
  `$XDG_CONFIG_HOME/smith` or `$XDG_STATE_HOME/smith` from a test.
- No words that name a task's files or commands in the role or a
  description (benchmarks.md 16.5).

## 8. Size limits

- **A new file:** at most 800 lines, `src/generated/` excepted.
- **A file already over 800 lines** (`session.rs`, `benchmarks/src/run.rs`,
  `summary.rs` and others): aim for growth of at most 100 lines per
  increment, added less removed by `git diff --numstat
  origin/main...HEAD -- <file>`; an increment that needs more moves its
  concern into a module of its own, and says so in its commit body.
- **A test function:** aim for at most 40 lines; nothing stops on it.
- **An increment's size** is its hand-written production lines, counted
  on the rebased branch by:

  ```
  git diff --numstat origin/main...HEAD -- ':(glob)crates/*/src/**/*.rs' ':(glob)src/**/*.rs' ':(glob)benchmarks/src/**/*.rs' ':(exclude,glob)**/generated/**' ':(exclude,glob)**/tests.rs' ':(exclude,glob)**/tests/**' | awk '{ s += $1 } END { print s + 0 }'
  ```

  It counts added lines in production sources, without generated code,
  test modules (`tests.rs`, `src/tests/`), the test packages (`tests/`,
  `benchmarks/tests/`), schemas, goldens, recorded files and Markdown.
  Each landing's report gives that count, and the count by file
  (`--numstat` without the `awk`) when it passes the Size line.
  Splits are the README's rule 1.
