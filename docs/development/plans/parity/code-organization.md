# Code and test organization

2026-10-14. Where the parity plan's code goes: the crates each session
changes, where tests and fixtures live, the owner of each shared
interface, the plan's "Don't" list and its size limits. It binds every
session. The design is the contract; this document adds no behaviour.
Where a session finds the code shaped otherwise on `main`, the code on
`main` wins over a sketch, unless the design asks for the change: report
the difference, and do not reshape landed code for its own sake.

## 1. Crates

No crate is added. smith's crates keep their kinds (smith
`AGENTS.md`; skein's `programming-model.md`): domains in the strict
subset, protocol components as step machines, services with their
profile, shells, and the harness as ordinary Rust under its own
`clippy.toml`.

| Session | Crates and modules |
|---|---|
| smith 01 | `benchmarks/` only: `tasks/fixtures/`, `tasks/repository/shell-cwd/`, `src/validate.rs`, `src/judgment.rs`, `src/check.rs`, `src/grade.rs`, `src/bundle.rs`, `src/run.rs`, `src/run/outcomes.rs`, `src/run/passive.rs`, `src/isolate.rs` |
| smith 02 | `benchmarks/` only: `agents/`, `suites/`, `tasks/probes/*/task.toml` (their `[smith]`), `src/agent/smith/`, `src/agent/codex/`, `src/agent/mod.rs`, `src/credential.rs`, `src/process.rs`, `src/probe.rs`, `src/run.rs`, `src/isolate.rs` |
| smith 03 | `benchmarks/`: `src/result.rs`, `src/summary.rs`, `src/summary/compare.rs`, `src/stats.rs`, `src/suite.rs`, `src/models.rs`, `src/main.rs`, `src/check.rs`, `src/run.rs`, `summaries/`, `baselines/`, `src/agent/codex.rs`, `src/agent/codex/rollouts.rs`, `src/agent/smith/events/`; `crates/smith-local-shell/src/presets.rs` (a public price function) |
| smith 04 | `benchmarks/`: `src/task.rs`, `src/probe.rs`, `src/fix.rs`, `src/batch.rs`, `src/check.rs`, `src/suite.rs`, `src/summary.rs`, `src/stats.rs`, a new `src/parity.rs`, `src/agent/smith/events/`, `suites/`, `tasks/probes/` |
| smith 05 | `smith-local-shell` (`presets.rs`, `settings/`), `smith-local-domain` (`domain.rs`, `boundary.rs`, `terminal.rs`), `smith-domain-run` (`charter.rs`), `smith-agent-shell` (`config.rs`), `benchmarks/agents/models.toml`, `tests/end-to-end` |
| smith 06 | `smith-protocol-llm` (`decode.rs`, `descriptions.rs`, `tests.rs`), `smith-local-shell` (a new `role.rs`, `compose.rs`, `settings/resolve/local.rs`), `smith-agent-service` (`profile.rs`, `profile/checks.rs`, `profile/validate.rs`, `profile/quantities.rs`), `tests/protocol-llm`, `tests/tools`; `benchmarks/suites/role.toml` |
| smith 07 | `smith-domain-session` (`session.rs`, `compaction.rs`, `boundary.rs`), `smith-domain-run` (`prompt.rs`, `opening.rs`, `run.rs`, `boundary.rs`), `smith-protocol-llm` (`prompt.rs`), `smith-agent-service::profile`; skein's pin |
| smith 08 | `smith-events`; `smith-agent-service::profile` (`quantities.rs`, `validate.rs`, `checks.rs`, `list.rs`, `effective/`), `smith-domain-session/src/limits.rs`, `smith-domain-tools/src/limits.rs`, `smith-protocol-machine/src/limits.rs`, `smith-local-shell/src/settings/`, `smith-agent-shell/src/config.rs` |
| smith 09 | `smith-domain-tools` (`job.rs` renamed, `boundary.rs`), `smith-protocol-machine` (`process.rs`, `component.rs`, `boundary.rs`, `listing.rs`), the hops in `smith-domain-session/src/session.rs`, `smith-domain/src/route.rs`, `smith-agent-service/src/service.rs`; `tests/machine` |
| smith 10 | `smith-domain-tools` (`call.rs`, `kit.rs`, a new `jobs.rs`, `facts.rs`, `boundary.rs`), `smith-domain-session` (`session.rs`, `record.rs`, `compaction.rs`, `facts.rs`, `boundary.rs`), `smith-protocol-channel/src/turn.rs`, `smith-protocol-llm` (`render.rs`, `limits.rs`, `tools.rs`), `smith-domain-run` (`run.rs`, `land.rs`, `lease.rs`, `facts.rs`), `smith-agent-service` (`calls.rs`, `sinks/`); `tests/tools`, `tests/session`, `tests/run`, `tests/agent` |
| smith 11 | `smith-transcript` (schema, generated, golden), `smith-protocol-channel/src/turn.rs`, `smith-domain-session/src/record.rs`, `tests/codecs`, `tests/protocol-llm`, `tests/agent`, `smith-protocol-llm` (`tools.rs`, `decode.rs`, `descriptions.rs`, `render.rs`, `stated.rs`, `limits.rs`), `smith-local-shell/src/role.rs`; `benchmarks/tasks/probes/tool-output/`, `benchmarks/src/agent/smith/events/`, `benchmarks/suites/` |
| smith 12 | `benchmarks/summaries/`, `benchmarks/baselines/`, `benchmarks/suites/compaction.toml`, `benchmarks/tasks/probes/` (calibration blocks) |
| skein 01 | `testing/skein-fake-llm-domain/src/cache.rs` |

## 2. Naming

- **The design's words,** for public types, variants and functions. `Job`, `JobNumber`, `Yield`, `Wait`,
  `JobEnd`, `JobEnded`, `Held`, `Calibration`, `TimeRecord`, `Parity`,
  `Criterion`, `Verdict`, `preamble`, `parallel_tools`, `jobs`,
  `job_wait`. A public name the design does not use needs a reason in the
  commit body.
- **The tools kit's calls in flight are `Flight`s** (09.1), so that `job`
  means only a command left running (domain/tools.md, section 5).
- **Units in types:** `Duration` for `yield_seconds`, `wait_seconds` and
  `job_wait`; wire fields keep their units in their names.
- **No generic names** (`Manager`, `Handler`, `Util`, `Helper`, `Data`,
  `Info`).

## 3. Public API

- Explicit re-exports in `lib.rs`; one line of prose per public type and
  variant: what it is, who sends it, its terminal.
- Design citations at module or type level, never stamped on fields.
- New public API across crates only where a Shape names it:
  `smith_local_shell::presets::list_prices` (03.1), the tools domain's job
  types (09.2, 10.1), `smith_events::JobEnded` (08.1).

## 4. Tests

| Tier | Where | Named |
|---|---|---|
| unit | `crates/<crate>/src/tests.rs` (or the module's `tests`), through `step` and `fire` or the component's entry points | the behaviour, `snake_case` |
| world | `tests/<world>/tests/*.rs`: tools, session, run, machine, protocol-llm, agent | sentences of about eight words |
| codecs | `tests/codecs/tests/`: goldens and drift | |
| end to end | `tests/end-to-end/` | sentences |
| harness offline | `benchmarks/tests/*.rs`, with recorded outputs in `benchmarks/tests/recorded/` and `fixtures/` | sentences |
| live | `benchmarks/tasks/`, run by `smith-bench` | probe names |

- **Ceilings per test** (development.md, section 5.6): a unit test under
  20 ms, a focused world test under 0.5 s, a fuzzy test about 5 s, read
  from the times nextest prints in the last focused run.
- **Budgets:** 15 s focused and 60 s fuzzy per repository (smith
  `workflow.md`, section 2), which the gate enforces. Aim for the jobs
  worlds' new stories to add about 1 s focused and 5 s fuzzy in all; past
  it, they are profiled and made cheaper (development.md 5.3).
- **One behaviour per test;** cases of one behaviour are rows of one
  table test (testing-strategy.md, section 8).

## 5. Fixtures

Each test package reuses its fixtures and extends them; none is defined
twice:
- the tools world: `tests/tools/src/fixture.rs` (`Fixture`, `program`,
  `long`) and `tests/tools/src/world.rs` (`Settings::calm`, `Work`), which
  10.1 extends with a job's states;
- the session world: `tests/session/src/world.rs`;
- the machine world: `tests/machine/src/lib.rs` and its `listed` helper
  in `tests/machine/tests/listing.rs`;
- the harness: `benchmarks/tests/common/`, `benchmarks/tests/recorded/`
  (recorded traces, rollouts and bundles), `benchmarks/tests/runner.rs`'s
  replaying agent.

## 6. Owners of shared interfaces

The owner defines the interface; the others use it by name.

| Interface | Owner | Used by |
|---|---|---|
| the formats' version 3: result and summary, the price basis, `SUMMARY_VERSION` | smith 03.1 | 03.2 to 03.5, 04, 07.3, 11.3, 12 |
| the `[calibration]` and `[variant.calibration]` block (`Calibration`) | smith 04.1 | 04.6, 07.3, 11.3, 12.3 |
| `history` version 2 and its `compaction-choice` check; version 3 and its `call-count` and `compaction-carried` (04.s2), in 07.3 (ruling 121) | smith 04.2 | 07.3, 12.2 |
| a summary's attempt rows (`AttemptRow`), a suite's `reference` | smith 03.2 | 04.4, 06.2, 12 |
| the `parity` section and `smith-bench parity` | smith 04.4 | 12.1, 12.4, the README's "Done when" |
| `presets::list_prices` | smith 03.1 | 05.1 adds a model to the presets' table, under whichever name it has when 05.1 lands |
| the event vocabulary version 2: `ToolCompleted.job`, `Verdict::{Running, Stopped}`, `JobEnded` | smith 08.1 | 06.1, 07.1 (their parents' arms), 08.2, 08.3, 10.1, 10.4, 11.3 |
| `Policy.parallel_tools` | smith 08.2 | 08.3 |
| `Declared.jobs`, `Policy.job_wait`, each layer's `jobs` and `job_wait` | smith 08.3 | 09.2, 10.1 |
| the tools boundary's jobs: `Op::Spawn`'s yield, `Request::Job`, `JobAct`, `Done::{Held, Job}`, and `JobEnd` in `call.rs` | smith 09.2 | 10.1 |
| the tools domain's jobs: `Call::Shell`'s yield, `Call::Job`, `Tool::Job`, `JobNumber`, `Outcome::{Running, Job}` | smith 10.1 (11.1 mirrors the sketch) | 10.2, 10.3, 11.2 |
| the tools domain's `Event::CancelJobs` and `Request::JobsCancelled` | smith 10.2 | 10.3 |
| the session's `Event::CancelJobs` and `Request::JobsCancelled`, and its `JobEnded` facts | smith 10.3 | 10.4 |
| the transcript's version 4 | smith 11.1 | 07.2, 10.1, 10.3, 10.4 |
| the task role (`smith_local_shell::role::TASK_ROLE`) | smith 06.2 | 11.2 adds topic 5's second half |
| the summary preamble (`Spec.preamble`, `Open.preamble`) | smith 07.2 | 07.3 |

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
- No second mechanism for something the design gives one owner (the
  profile's derivation, the event sink, the lease, the price basis).
- No mechanism kept half-removed: removing one removes its code, limits,
  memory accounting, facts, tests and docs outside `docs/design/`, in the
  same increment.
- No edit to jig or temper, and no edit to `docs/design/` or to this plan
  by an implementer.
- No live call in a test of the suite.
- No reading of, or writing to, the user's `~/.codex`, `~/.claude`,
  `$XDG_CONFIG_HOME/smith` or `$XDG_STATE_HOME/smith` from a test.
- No change to a criterion, a margin, the price basis, an agent's "as it
  ships", the working tier or compaction's `ToolChoice::None`
  (decisions.md, 8.4): each is a question for the designer.

## 8. Size limits

- **A new file:** at most 800 lines, `src/generated/` excepted.
- **A file already over 800 lines** (`session.rs`, `run.rs`, `service.rs`,
  `benchmarks/src/run.rs` and others): aim for growth of at most 100
  lines per increment; an increment that needs more moves its concern
  into a module of its own, and says so in its commit body.
- **A test function:** aim for at most 40 lines; past it, the implementer
  moves the setup into a named helper of its file.
- **An increment:** its Size line is a guide; the README's rule 1 says
  when to split.
