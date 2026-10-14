# Session 02: each agent as it ships

2026-10-14. The harness measured a smith no user runs: its compare pin is
the `first-request` probe's JSON template, with an empty role, half the
window, an output of 4,096 tokens, sub-agents on, a 1 TB memory bound and
headers the product never sends; and a Codex with `multi_agent` on, run
from `PATH` and trusted at its working directory (findings T3, T4, T8,
T12, W6). This session makes smith's pins the product's own settings
layers, removes the earlier faces, and runs Codex as it ships, from its
digest-pinned private package. It changes `benchmarks/` only.

## Read first

- smith `docs/design/benchmarks.md`, sections 5.4, 6 (its table's
  prepare and command rows), 6.1, 6.2 and 10.
- smith `docs/design/shell.md`, sections 4.2 and 5 (the settings layers
  and `-c`).
- `findings/harness.md` T3, T4, T5, T8, T12; `findings/time.md` W6;
  `findings/probes.md` F1; `decisions.md` 2.1 and 3.4.
- skein `testing-strategy.md`, section 2.9.

## What the design asks

- **A comparison's pin changes nothing the agent ships** but what the
  attempt must own: its paths, its model and effort, its account, and the
  keys that keep it unattended and isolated (5.4).
- **smith's pin is the user's settings layer:** the account's file and the
  trace, capturing `calls` (`everything` where a task needs prompts), and
  nothing else of a comparison's; a task's overrides are `-c` settings;
  the teardown bound comes from `smith check --json exec` (6, 6.1).
- **One face:** `smith exec --json`, or `smith --trace` at a terminal; no
  earlier face, settings template or reader is kept; baselines on an
  earlier face are retaken or dropped (6.2).
- **Codex:** a private `CODEX_HOME`; only no MCP servers, the attempt root
  trusted, edits allowed, no approvals, the model and effort, and the
  private caches among its sandbox's writable roots; run from a private
  copy of its pinned package, whose executable's SHA-256 the pin names
  and setup checks (6, 10).

## Choices this plan makes

- The pins left are `standard` (trace `calls`) and, for the probes that
  read prompts, `everything`; a suite or a probe that named another pin
  names one of these, and moves its other values to its task's `[smith]`.
- `first-request`'s task gains `instructions = ""` in its `[smith]`
  table: its subject is an empty role (F1; decisions.md, 3.4).
- The 11 baselines on `main` predate the product face; 03.1 drops them
  with the summaries they were cut from, and 12.4 cuts new ones from the
  closing `parity` run (decisions.md, 10.6).
- Codex's pin keeps `check_for_update_on_startup = false`, its background
  service off where it can be (benchmarks.md 6's prepare row), and drops
  `features.multi_agent`, `agents.max_threads` and
  `shell_environment_policy.inherit`: the harness gives Codex its exact
  environment, and leaving the rest out runs Codex as it ships.

## Rules that bind it

- **The harness follows the binary** (README, rule 14) and **the smoke**
  (rule 8): each landing here changes how smith is invoked, so its smoke
  is its first live evidence.
- **Users outside Files** (rule 3): the removed face types' users are
  removed or moved to their successor, with no line limit.
- **The live budget** (rule 10): 02.1 and 02.3 run one attempt each
  beyond their smokes.

## Increments

### 02.1 smith's pins as product settings layers
Scope: benchmarks.md 5.4, 6 and 6.1 for smith (findings T4, T5, F1).
Needs: nothing.
Files: `benchmarks/agents/smith/` (every `*.pin.toml`, `first-request.json`,
  `smoke.json`), `benchmarks/src/agent/smith/settings.rs`,
  `benchmarks/src/agent/smith/prepare.rs`, `benchmarks/src/agent/mod.rs`,
  `benchmarks/suites/*.toml` (their smith `config`),
  `benchmarks/tasks/probes/*/task.toml` (their `[smith]` tables),
  `benchmarks/tests/smith_adapter.rs`, `benchmarks/tests/smith_adapter/`.
Shape: each smith pin names a product TOML user layer in
  `agents/smith/<name>.toml` that holds only the borrowed account's file
  and the trace's capture. The harness passes the workspace (`-C`), `-m`,
  `--effort`, and each of the task's `[smith]` entries as `-c KEY=VALUE`
  under its product name (shell.md, section 5), and nothing else; a key
  that names the tier's model writes it as the model the tier resolved.
  No budget is written when the task names none. The teardown bound is
  read from `smith check --json exec` only. `settings::{settings, budget,
  overrides, teardown, product_document}` and the two JSON templates go.
Test shape: `benchmarks/tests/smith_adapter.rs`:
  `every_smith_pin_is_a_product_settings_layer`,
  `a_task_override_reaches_smith_as_a_product_setting`,
  `a_comparison_pin_keeps_every_preset_value_and_the_default_role`
  (no `models.*`, `endpoints.*`, `deployment.*` or `instructions` key from
  the user layer, read from a recorded `smith check --json` output),
  `a_task_without_a_budget_renders_the_products_defaults`. Under 0.5 s
  each. Tests that name the removed functions are removed with them
  (rule 3).
Proof: live, once: the smoke (rule 8), then `heavy --gate cargo run -p
  smith-bench -- probe probes/first-request --attempts 1 --arm <dir>`: it
  passes with its own `instructions = ""`. At most 2 attempts beyond the
  smoke, 5 minutes.
Don't: keep a translation table from template names to product names.
Size: 250 lines of production code; most of the change removes code.
Removes: `first-request.json`, `smoke.json`, the template renderer and its
  translation, and the pins' prices, headers, window, output, memory,
  `sub_agents` and role.

### 02.2 One smith face
Scope: benchmarks.md 6.2 (findings T8).
Needs: 02.1 (no pin names a template).
Files: `benchmarks/src/agent/smith/events.rs` (`Shape`),
  `benchmarks/src/agent/smith/prepare.rs` (the native branch,
  `:126-169`, `:310-320`), `benchmarks/src/agent/smith/observe.rs`
  (`:79-96`), `benchmarks/src/agent/smith/mod.rs`, the faces registry and
  `benchmarks/agents/smith/faces.toml`, `benchmarks/src/credential.rs`
  (`smith()`, `Shape::Smith`), `benchmarks/src/process.rs` (the split
  trace, `:585-621`), `benchmarks/src/probe.rs` (`:386-388`),
  `benchmarks/src/run.rs` (`:757`, `:787`, `:869`, `:1115`), and the tests the audit lists
  (`findings/planning-audit.md`, harness tasks and pins, point 7).
Shape: `Shape` keeps the product's faces only (`Exec`, `Chat`); a binary
  that is not a descendant of the first product face (`0d5604f9`) is
  refused at setup as `Refused(setup)`, naming it.
Test shape: `a_binary_older_than_the_product_face_is_refused` (under 20
  ms); the native-face tests are removed (rule 3).
Proof: unit, that test; then `heavy cargo run -p smith-bench -- check`
  exits 0, once.
Don't: keep a reader of an earlier face for summaries or baselines.
Size: 60 lines added; the rest removes code.
Removes: `Shape::{LocalTrace, LocalAgentTrace}`, the native prepare path,
  `credential::smith`, the five pre-product faces.

### 02.3 Codex as it ships
Scope: benchmarks.md 6 (Codex's prepare row), 10 (findings T3, T12, W6).
Needs: nothing.
Files: `benchmarks/agents/codex/overrides.toml`,
  `benchmarks/agents/codex/standard.pin.toml`, `benchmarks/src/agent/mod.rs`
  (`Configuration`, `:215-222`), `benchmarks/src/agent/codex/prepare.rs`
  (`:47-52`, `:88-102`), `benchmarks/src/probe.rs` (`:420-423`),
  `benchmarks/src/isolate.rs` (`:61-74`), `benchmarks/tests/codex_adapter.rs`.
Shape: the pin gains `sha256 =
  "12eb3e81114588aca3b7998f4f19e8997b056aca08e57a7ca7c8a3ec8c652aad"` and
  `package = "tools/codex-0.160.0"` (under `$XDG_STATE_HOME/smith-bench/`);
  setup runs `<package>/bin/codex` and refuses one whose digest differs,
  as `Refused(setup)`. The overrides keep only: no MCP servers, the attempt
  root trusted (`projects."<attempt root>".trust_level = "trusted"`),
  workspace edits, no approvals, `check_for_update_on_startup = false`,
  the model and effort, and `sandbox_workspace_write.writable_roots`
  holding the attempt's cargo home, target and temporary directory.
Test shape: `a_codex_binary_whose_digest_differs_from_its_pin_is_refused`,
  `the_codex_pin_names_only_the_keys_an_attempt_must_own`,
  `codex_writable_roots_hold_the_attempts_caches`,
  `codex_trusts_the_attempt_root`. Under 20 ms each.
Proof: live, once: `heavy --gate cargo run -p smith-bench -- probe
  probes/smoke-tools --agent codex --attempts 1 --arm <dir>` passes: its
  `[[outcome]]` checks pass (its event checks read smith's stream and are
  not judged for Codex), Codex run from the package. At most 2 attempts, 3 minutes.
Don't: read or change the user's own Codex installation or `~/.codex`
  beyond the login read benchmarks.md 10 allows.
Size: 120 lines.
Removes: `features.multi_agent`, `agents.max_threads`,
  `shell_environment_policy.inherit`, finding `codex` on `PATH`.

## Done when

- `rg -n 'first-request.json|smoke.json|LocalTrace|multi_agent|max_threads' benchmarks`
  prints nothing.
- `heavy cargo run -p smith-bench -- check` exits 0 on `main`.
