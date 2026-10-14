# Findings: the benchmark tasks and the harness

2026-10-14. What in smith-bench's tasks, pins and code keeps a smith/Codex comparison from passing, from being fair, or from being checked, at smith `689c15d0`. Written by a read-only sub-agent for the parity plan's designer (skein's development.md, section 4.1).

Sources: the code at smith `main` (`689c15d0`) and skein `bec13dc`; raw attempts under `~/.local/state/smith-bench/runs/` for `compare-a57ca3a8-20261013T221135Z` (30 attempts) and `repository-a64f0523-20261013T230741Z` (6 attempts), read with `jq`. "Compare NN" and "repository NN" name those attempts.

## 1. In one page

- **No comparison can pass today.** All 30 compare attempts and all 6 repository attempts fail overall on the protected-file check alone (T1, T2). Both are task or harness defects. The fix is a version 2 of the three fixtures, and one change to `run.rs`.
- **The harness does not run smith as it ships.** Its compare pin is the `first-request` probe's template. It turns on `sub_agents`, empties smith's role text, halves the window (131,072 against the preset's 272,000), cuts `output` to 4,096 against 32,000, and sends `originator: pi` headers that the product never sends (T3, T4). Each smith repository attempt compacted twice. Codex's threads reached 131,454 prompt tokens without compacting.
- **Codex is not run as it ships either.** Its pin turns on `features.multi_agent` and sets `agents.max_threads = 6` (T3). It spawned 2 children in each repository attempt. The harness reports Codex's children as "unavailable", although its rollouts link them (T11).
- **Budgets are smith's alone, and sit below the deadline.** The design says every agent's own limits sit above the hard deadline. The fixtures give smith 840 s under a 900 s deadline. The renderer refuses any budget above the deadline, and smith's model is told "64 turns and 839 seconds" (T5).
- **Spend has three catalogs.** For `gpt-6-luna`, the harness's table is 12.5 times the product's input rate and 20 times its output rate. For `gpt-6-astra` the product is 8 and 5 times the table. A smith run in the harness mixes the two (T6).
- **Calibration has no home in the task files.** One task has a `[calibration]` block, and no code reads it (T7).
- **Two pre-product smith faces and the old JSON template can go.** Event baselines exist, which is 6.2's condition for removal (T8).
- **The instruction-parity question is settled.** Both agents read the seed's `AGENTS.md` whole, by their own defaults. Only the framing differs (T9).
- **Some things cannot be checked yet.** Every repository attempt wrote to `/tmp` under names other attempts reuse (T10). The Codex binary is pinned by version, not by digest (T12). The repository task builds cold inside the timer, and its large-effect rule lives only in the report (T13). `validate` never applies a task's own checks to its reference (T14).
- **Proposal 19's nine rulings** are in place in code except 13.132, and none is in the design yet. Section 3 gives the sentence for each. io.md does not say what happens to a second child signal (13.39).

## 2. Entries

### T1 The fixtures protect `tests/` while their prompts ask for tests

- **Symptom:** in compare-a57ca3a8, all 30 attempts are `completed`, exit 0, grade `true`, and `protected_files_changed = ["tests"]` (each `result.json`). The summary gives 0/5 passes in all six groups, with interval [0, 0.5218] (`benchmarks/summaries/compare-a57ca3a8-20261013T221135Z.json`, `.groups`). In every diff, the only change under `tests/` is `tests/regression.rs`: 0 lines removed, and 65 to 234 lines added (awk over each `diff.patch`).
- **Root cause:** all three fixtures share `protected = ["tests", "Cargo.toml", "Cargo.lock", "AGENTS.md"]` (`benchmarks/tasks/fixtures/{01-backoff,02-line-decoder,03-planner}/task.toml:9`). Yet the prompts say "add regression tests, and run the suite" (01, `:6`), "Add tests and run the suite" (02), and "Add regression tests" (03). The seeds' `AGENTS.md` says "Add relevant regression tests and run them. Do not remove or weaken existing tests." Neither the prompts nor `AGENTS.md` mentions any protection. The check is `grade::changed` (`benchmarks/src/grade.rs:101-105`). It compares every snapshot entry under the path (name, SHA-256, executable bit; `.git` skipped, `:37-41`) between the frozen seed and the settled workspace. Any added, removed or changed file under `tests/` is a change (`run.rs:909-911`). A new file `tests/new.rs` would fail too. The references change only `src/` (`diff -r seed reference`), so they add none of the tests their prompts ask for.
- **The clean fix:** each fixture gets a version 2 (benchmarks.md 5.2: "`version` changes with the prompt, seed, grader or checks").
  - `protected = ["Cargo.toml", "Cargo.lock", "AGENTS.md"]`, with `tests` dropped.
  - "Did not weaken tests" stays checked by the grader. `grader/tests/regression.rs` is already the seed's `tests/regression.rs`, reformatted, with the same test functions. `grade[1]` runs it against the agent's `src/`, so a weakened workspace copy cannot hide a behaviour that fails the seed's tests.
  - Each reference adds one regression test, as its prompt asks.
  - Of the options, "protect only the seed's existing test files" would still fail all 30 attempts, because each one appended to `regression.rs`. "Name in the prompt where tests go" changes the prompt for every agent and still leaves weakening to the grader. "The grader re-runs the seed's tests" is what keeps "did not weaken" checked, and it is already built.
  - Design: benchmarks.md 5.2's row "`setup`, `protected` | ... files that must not change" gains: "A task never protects a path its prompt asks the agent to change. That the seed's tests still pass is the grader's: it holds a copy of them." 11.4's grader column for `01-backoff` ("hidden edge-case and regression tests") is already right.
- **Its tests:** in `benchmarks/tests/offline.rs`:
  - `no_fixture_protects_a_path_its_prompt_asks_to_change`: for the three fixtures, `tests` is not under `protected`.
  - `every_fixture_grader_holds_the_seeds_test_functions`: each `#[test] fn` name in `seed/tests/*.rs` appears in `grader/tests/`.
  - `a_fixture_reference_passes_its_protected_and_outcome_checks`: see T14.
  - Live: `suite compare`, 30 attempts, with at least one overall pass per group expected.
- **Confidence:** high. In all three fixtures, the `fn` names in `grader/tests/regression.rs` equal the seed's (`grep -o 'fn [a-z_]*'` over both files, then `diff`).

### T2 The repository task compares protected files with the manifest, not the bundle

- **Symptom:** in repository-a64f0523, all six attempts report `protected_files_changed = ["Cargo.toml","Cargo.lock",".config/nextest.toml","AGENTS.md","vendor",".cargo"]`, which is every protected path. The milestone report says none of them appears in any diff (`smith-codex-2026-10-14.md:142`).
- **Root cause:** the workspace is copied from the source that `seed_source` resolves. For a repository task, that is the digest-checked bundle (`run.rs:444`, `bundle.rs:74-87`). The "before" snapshot, though, is taken of the task's own `seed/` directory (`run.rs:1050`: `crate::grade::snapshot(&task_directory.join("seed"))`), which holds only `bundle.toml`. Every bundle file present after the run is therefore "added". The same `seed` feeds `grade::checks` (`run.rs:908`), so an outcome `protected` check on a repository task is wrong in the same way.
- **The clean fix:** one source for the copy and for the snapshot.
  - `prepare` returns the `Snapshot` of the path that `seed_source` resolved and `isolate::prepare` copied.
  - Or `Private` carries the snapshot, taken right after the copy and before `seed_commit` and `setup` run.
  - `attempt` never reads `task_directory.join("seed")` again.
  - Design: benchmarks.md 5.2's `seed/` paragraph ("A repository seed is the bundle's manifest ...; the bundle lives in the harness's data directory") gains: "Every check that reads the seed reads the bundle the workspace was copied from."
- **Its tests:**
  - `a_repository_attempt_compares_protected_files_with_its_bundle`, over `tests/recorded/bundles/tiny`. An unchanged workspace reports `[]`. A changed `Cargo.toml` reports `["Cargo.toml"]`.
  - `the_seed_snapshot_comes_from_the_copied_source`, a unit test in `run.rs`.
  - Live: none needed. The next `suite repository` (6 attempts) shows it.
- **Confidence:** high. The mechanism is in the code, and the report found the same thing.

### T3 Both agents run with sub-agents switched on by the harness

- **Symptom:** smith's rendered settings have `"sub_agents": true` in all compare and repository attempts (each `settings.json`; `check.json` gives the origin as the user layer). smith's system text holds a "## Sub-agents" section that offers `gpt-6-luna` (compare 01, `trace.jsonl` line 6, `prompt.system`). Codex runs with `features.multi_agent = true` and `agents.max_threads = 6` (`overrides.json` of repository 04). Its developer text carries the `/root` team role and "<multi_agent_mode>... Do not spawn sub-agents unless the user or applicable AGENTS.md/skill instructions explicitly ask" (compare 06 rollout). Children in each attempt:

  | Agent | Fixtures | Repository |
  |---|---|---|
  | Codex | 0 in all 15 attempts (only `exec` calls) | 2 `spawn_agent` calls in each of 04, 05 and 06 |
  | smith | 0 in all 15 attempts | 0, 2 and 3 in 01, 02 and 03 |

  The repository prompt says "Subagents may be used at your discretion".
- **Root cause:**
  - smith: `benchmarks/src/agent/smith/settings.rs:164` sets `settings["sub_agents"] = json!(true)` for every arm, and `:270` carries it into the product layer. The product default is `sub_agents=false` (`crates/smith-local-shell/src/settings/layers.rs:11`). shell.md 6.4 says "`sub_agents` | off | decided, until the reserve lands".
  - Codex: `benchmarks/agents/codex/overrides.toml:7,9`. Commit `01c622c8` calls `max_threads=6` "the upstream default six child threads". It says nothing about `multi_agent`.
  - benchmarks.md 5.4 says: "'Vanilla' is the agent's defaults plus this file".
  - Codex 0.160.0's default for `features.multi_agent` is not established by anything in the tree, the raw runs or the binary's strings.
- **The clean fix:** a parity comparison runs each agent as it ships.
  - The compare pin renders no `sub_agents` key, so the product's default applies.
  - Codex's pin drops every key that only restates or departs from a default. Before the change lands, check `features.multi_agent` once on the private copy (`codex features list`), and name the result in the pin's comment.
  - Delegation becomes a declared experiment (benchmarks.md 11.3), with both agents switched on, not the comparison's default.
  - Each result's identity records each agent's effective delegation setting.
  - Design:
    - 5.4 gains: "A comparison's pins change nothing the product ships, except the attempt's paths, model, effort and account. An experiment's arm names each key it changes."
    - 11.3 gains a row: "`delegation` | `shell-cwd` | each agent's sub-agents off; both on".
- **Its tests:**
  - `the_compare_pin_renders_no_key_the_product_defaults`: rendering the compare pin leaves out `sub_agents`, `instructions`, `[budget]` and model limits.
  - `codex_overrides_name_only_isolation_and_approval_keys`: the allowed key set is `approval_policy`, `sandbox_mode`, `check_for_update_on_startup`, `mcp_servers`, `projects`, `model_reasoning_effort` and `shell_environment_policy`.
  - Live: `suite repository`, 6 attempts, each arm as shipped.
- **Confidence:** high for smith. Medium for Codex until its default is read off the pinned binary.

### T4 smith's model limits, role text and headers come from a probe's template

- **Symptom:** in compare 01, smith's request declares `window_tokens 131072` and `output_tokens 4096` (`trace.jsonl` line 6). The system text starts at "## AGENTS.md in the working directory", with no role sentence before it. `check.json` gives the following, all from the user layer:
  - `idle=120` and `head=60`;
  - `connect=30` and `handshake=30`;
  - `deployment.memory=1000000000000`;
  - headers `originator`, `user-agent` and `OpenAI-Beta`;
  - `instructions=""`.

  The product preset has `window 272000`, `output 32000`, `idle 300`, `connect 10`, `handshake 10` and no headers (`presets.rs:38-63`; shell.md 6.1 and 6.2). It has `memory` 2 GiB (shell.md 6.3) and the role "Carry out the person's request in this workspace. Finish with what changed and what you checked." (`settings/resolve/local.rs:143-149`). Codex reports `model_context_window 258400` (repository 04 root rollout, `token_count`). Effects observed:

  | Measure | smith 01, 02, 03 | Codex 04, 05, 06 |
  |---|---|---|
  | Compactions per attempt | 2, 2, 2 | none |
  | Largest root prompt (tokens) | — | 115,845, 131,454, 122,098 |
  | Largest output in one response (tokens) | 2,773, 2,933, 2,485, of the 4,096 cap | — |

  smith's 80% threshold of 131,072 is 104,858 tokens.
- **Root cause:**
  - `benchmarks/agents/smith/compare.pin.toml:5` names `first-request.json`, the empty-brief probe's template. Its `"instructions": ""` (`:20`) is that probe's subject (`benchmarks/tests/smith_adapter.rs:16`, `the_first_request_pin_renders_only_the_probes_empty_instructions`). The template also sets `"max_tokens": 4096` (`:22`) and the "pi" headers (`:11-13`).
  - `settings.rs:144-152` writes `connect`/`handshake` 30 000, `window` 131 072, `idle` 120 000 and `reasoning_item` for every arm. `product_document` (`:215-275`) carries all of them into the product layer.
  - benchmarks.md 6.1 says "smith's pinned settings add nothing", and 12 says "It runs smith as a person would".
- **The clean fix:**
  - The compare and repository pins become product TOML user layers that hold only the account file and the trace. The harness adds the workspace directory, `-m` and `--effort`, and nothing else.
  - Probes that need a smaller window or an empty brief say so in their own task `[smith]` overrides, where 5.2 puts them.
  - Design: 6.1's Instructions bullet gains: "A comparison's smith runs on its presets' declared values and its own role. A probe that needs others names them in its task."
- **Its tests:**
  - `a_comparison_pin_keeps_every_preset_value_and_the_default_role`: render the pin through `smith check --json`'s recorded output. No `models.*`, `endpoints.*`, `deployment.*` or `instructions` key comes from the user layer.
  - Live: `suite compare`, 30 attempts, and `suite repository`, 6 attempts, after T1 and T2.
- **Confidence:** high that the harness differs from the product. How much this explains smith's repository times (task s median 728 against 422) is the compaction and caching themes' to measure.

### T5 Budgets are given to smith alone, and below the hard deadline

- **Symptom:** fixtures render smith `budget.seconds=840`, `turns=64` and `reserve 2 turns/300 s` under a 900 s deadline (compare 01 `settings.json`). The repository task renders `seconds=1200`, `turns=4096` and `spend=$1000` under a 1200 s deadline. smith's model is told "At open, you have 64 turns and 839 seconds of wall time" (compare 01 `prompt.system`). Codex gets no budget. Neither limit bound any attempt: the slowest fixture attempt ran about 100 s, and the slowest repository attempt 765 s.
- **Root cause:** the fixtures set `[budget] seconds = 840, turns = 64` (`task.toml:15-17`), and the repository task sets `seconds = 1200` (`:14-15`). `settings.rs:42` refuses `requested_seconds > task.deadline_seconds`, and `:29` defaults turns to 4096, spend to $1000 and seconds to the deadline. Yet benchmarks.md 6.1 says: "In a comparison every agent's own limits sit above the task's hard deadline, the bound they share." The code enforces the opposite.
- **The clean fix:**
  - Compare and repository tasks carry no `[budget]`, and the renderer then writes no `budget`. smith keeps its run defaults (512 turns, 1 h, $25; shell.md 6.4), and these sit above every deadline here.
  - `check` refuses a compare or repository task whose budget is below its `deadline_seconds`.
  - The budget probes keep their own budgets.
  - Design: 6.1's Budgets bullet gains "A comparison task names no budget; smith runs on its defaults."
- **Its tests:**
  - `a_comparison_task_with_a_budget_below_its_deadline_is_refused`, in `offline.rs`.
  - `a_task_without_a_budget_renders_the_products_defaults`, in `settings.rs`.
- **Confidence:** high.

### T6 Three price catalogs disagree

- **Symptom:** rates in dollars per million tokens (`python3` over the three sources):

  | Model | Source | Fresh | Cache read | Cache write | Output |
  |---|---|---:|---:|---:|---:|
  | gpt-6-astra | `benchmarks/agents/prices.toml:7-10` | 1.25 | 0.125 | none | 10 |
  | gpt-6-astra | `presets.rs:112` (window ≤ 272k) | 10 | 1 | 12.5 | 50 |
  | gpt-6-astra | `presets.rs:111` (window > 272k) | 20 | 2 | 25 | 75 |
  | gpt-6-astra | pins (`first-request.json:23-24`, `smoke.json`: 10, 1, 80 per 8,000,000) | 1.25 | 0.125 | the preset's 12.5 | 10 |
  | gpt-6-luna | `prices.toml:2-5` | 1.25 | 0.125 | none | 10 |
  | gpt-6-luna | `presets.rs:117` (window ≤ 272k) | 0.10 | 0.01 | 0.125 | 0.50 |
  | gpt-6-luna | pins | 1.25 | 0.125 | the preset's | 10 |

  - The pins' "selected-model" entry prices whichever model a tier picks.
  - Compare 01's `check.json` shows the mix: astra's `cache_write_price=100.0 [built_in]` per `price_unit=8000000 [user]` sits beside the pin's user-layer input, cached and output rates.
  - Repricing every attempt: the repository medians are Codex $1.409 and smith $0.966 at `prices.toml`, and $10.635 and $7.075 at the preset's rates. The compare medians move by 6.1 to 6.6 times.
- **Root cause:**
  - `prices.toml` came in 12.9 (`09d029e7`) "from the existing Codex notional rates in local_shell.rs". 10.13 later moved the product to documented list rates (`presets.rs:105-107`).
  - The pins predate both. The summary prices fresh, cache read and output only (`summary.rs:222-244`).
  - The design names the product's presets as the source. shell.md 5.2: "A preset model carries notional prices, the provider's list prices". benchmarks.md 8.1: "spend | the agent's own figure ... with its basis". 9: "spend on one basis".
  - `prices.toml:1` cites "shell.md, 6.1", which holds no prices.
- **The clean fix:**
  - One catalog, the product's.
  - The harness prices summaries from `smith_local_shell`'s preset table (made public, at the harness's commit), and the summary records that commit.
  - smith's own `spent` comes from its presets once the pins carry no rates (T4).
  - `prices.toml` and the pins' rates are deleted.
  - Reading prices through each arm's `smith check --json` is the alternative. It fits smith's own spend, but cannot price Codex on one basis across arms.
  - Design: 8.1's spend row gains "the common basis is the product's preset list prices at the harness's commit"; 9's "spend on one basis" names it.
- **Its tests:**
  - `the_summary_prices_tokens_from_the_product_presets`: a fixed token record against `presets::prices`.
  - `no_pin_declares_model_prices`.
  - `the_priced_basis_names_its_catalog_commit`.
- **Confidence:** high on the disagreement. Which rates are right is shell.md's question, not the harness's.

### T7 Calibration lives in commit bodies

- **Symptom:** one task file has a `[calibration]` block: `benchmarks/tasks/probes/credentials/task.toml:48-51`, with `failed_before = "none: no runnable control was named"` (`rg -l '^\[calibration\]'`). The milestone report's calibration column has 15 rows, written by hand from commit bodies and decisions.
- **Root cause:**
  - `Calibration` holds `binary`, `passes` and `failed_before` (`benchmarks/src/task.rs:261-265`). `Variant` has no such field (`:269-292`), yet `probes` lists variants as tasks of their own.
  - `check.rs:199-212` requires exactly 10 passes, but 3.1 allows "ten passes within at most twelve", and the oversized variant needs three.
  - Nothing else reads `task.calibration` (`rg '\.calibration\b'`).
  - `fix.rs:14` `calibration_suite(_probe)` always returns `suites/probes.toml`.
  - `smoke.toml` gates on five probes whether calibrated or not.
  - `probe.rs:606` prints a block without attempts, control, summary or date.
- **The clean fix:** a block per task and per `[[variant]]`:

  ```toml
  [calibration]
  binary = "<commit>"            # the arm calibrated
  binary_sha256 = "<digest>"
  date = "2026-10-13"
  summary = "summaries/<run>.json"  # the committed calibration summary
  attempts = 12                  # logical attempts run
  passes = ["<run>/<attempt>", ...]  # at least 10 (3 for an exercised-pass rule)
  deviations = ["<run>/<attempt>"]   # failed by the model only (8.3)
  control = "<commit>|control:<arm>" # what failed before
  failed_before = ["<run>/<attempt>"]
  ```

  - A probe gates only when its task file holds a block whose `binary` is an ancestor of the arm and whose task version is current.
  - `smoke` refuses to list an uncalibrated probe.
  - `probes` reports uncalibrated failures apart and counts no regression from them (9).
  - The block is written by `calibrate` and committed with the summary it names.
  - Design: 3.1 "a committed summary of its calibration says which" becomes "its task file's `[calibration]` block, naming the committed summary, says which". 11.5's `calibrate` row gains "writes the block". 5.4's "Smoke runs, and a fix's before and after, go in commit bodies" gains "a calibration goes in its task file".
- **Its tests:**
  - `a_calibration_block_names_its_summary_and_failing_control`.
  - `a_variant_carries_its_own_calibration`.
  - `smoke_refuses_an_uncalibrated_probe`.
  - `an_uncalibrated_probe_failure_is_not_a_regression`.
  - `ten_passes_within_twelve_attempts_is_calibrated`.
- **Confidence:** high.

### T8 Four smith faces and a hand-kept settings translation

- **Symptom:** `Shape` has `LocalTrace`, `LocalAgentTrace`, `Exec` and `Chat` (`benchmarks/src/agent/smith/events.rs:51-63`). `faces.toml` holds 7 entries, 5 of them for pre-product commits (`4735d72`, `0486bcf`, `b86b022`, `8d2ec9b`, `4d06c11`). Product arms go through `settings()` (the native JSON template, `settings.rs:98-172`) and then `product_document` (`:215-275`), a hand-kept list of field names and units.
- **Root cause:** 6.2's rule covered the pre-events face, removed in 00.21. It says nothing of the event-era native faces. The baselines on events exist: 10 smith files dated 2026-10-12 with `smith_face: "events"` (`benchmarks/baselines/`). They were taken before the first product face: `0d5604f` (2026-10-13) is not an ancestor of `858c9642`, the 00.21 arm (`git merge-base --is-ancestor`). The first-request pin and the smoke pin keep the old JSON shape (`agents/smith/{first-request,smoke}.json`). `prepare.rs:126-169` keeps the native path, with its own credential conversion (`credential::smith`) and teardown (`settings.rs:175-189`, the debt "teardown bound derived again").
- **The clean fix:** delete the following, with their tests in `tests/smith_adapter.rs` that name them:
  - `Shape::{LocalTrace, LocalAgentTrace}` and the native branch of `prepare.rs`;
  - `settings::settings`, `budget`, `overrides`, `teardown` and `product_document`;
  - the five pre-product `faces.toml` entries;
  - `first-request.json` and `smoke.json`.

  The pins become product TOML layers. Task `[smith]` overrides become `-c KEY=VALUE` in product names (5.2 already says "passed as `-c` settings"). Teardown comes only from `smith check`. The 10 baselines on pre-product binaries are retaken on `exec` or dropped (9: "retaken ... after ... a new result version"). Design: 6.2 is replaced by "smith is driven only as it ships, `smith exec --json` or `smith --trace` at a terminal; a binary that predates them is not run".
- **Its tests:**
  - `every_smith_pin_is_a_product_settings_layer`.
  - `a_task_override_reaches_smith_as_a_product_setting`.
  - Delete the native-face tests.
- **Confidence:** medium. Whether any committed summary still needs a native face for `fix --parent` is the one open fact: `rg` the summaries' `smith_face` and arm commits against `faces.toml`.

### T9 Instruction parity: settled

- **Symptom:** smith's system text begins "## AGENTS.md in the working directory" and then the seed's `AGENTS.md`, whole (compare 01; repository 01's 2,912-byte file, with no "The file goes on"). Codex's first user message is "# AGENTS.md instructions for <workspace>" and the same bytes inside `<INSTRUCTIONS>` (compare 06 rollout).
- **Root cause:** the product puts the text there, not the harness. `conventions::guide` defaults to `AGENTS.md` (`crates/smith-domain-run/src/conventions.rs:53-58`), `render_guide` writes the heading (`prompt.rs:432-447`), and `guide` keeps 16 KiB whole (shell.md 6.3). The harness restates `"guide": "AGENTS.md"` but changes `checks` to `.smith-bench-checks` (`first-request.json:29`), against the product's `.smith/check` (`conventions.rs:60-65`).
- **The clean fix:** benchmarks.md 15's "Instruction parity" is closed. 6.1 gains: "smith reads `AGENTS.md` by its default convention into its system text; Codex reads it into its first user message; both whole." The pin drops `conventions` (T4).
- **Its tests:** `both_agents_receive_the_seeds_guide_whole`, over the recorded compare-01 and compare-06 openings.
- **Confidence:** high.

### T10 Writes outside the attempt cannot be seen

- **Symptom:** every repository attempt wrote under `/tmp`, using names that other attempts reuse. `/tmp/smith-cwd-clippy.log` appears in 01, 03, 04 and 05, and Codex 06 used `/tmp/smith-shell-cwd-target` (`grep -o '/tmp/...'` over each trace and rollout). `writes_outside_workspace` is `unavailable` for smith ("event stream does not expose every shell filesystem...") and `lower-bound []` for Codex.
- **Root cause:** `TMPDIR` is private (10: "each with its own temporary directory"), but nothing stops a literal `/tmp`. The scope gate reads only the calls an agent shows (`run.rs:514`, `scope`).
- **The clean fix:** until contained trees exist:
  - the harness lists the entries under `/tmp` that the user owns before and after each attempt, and reports new or changed ones as observed writes outside;
  - the repository prompt or seed `AGENTS.md` (version 2) says "temporary files go under `$TMPDIR`";
  - attempts with a write outside are flagged in the summary.

  Design: 10's "Until contained trees exist nothing keeps an agent from the user's files; the scope gate looks for it" gains "including a listing of `/tmp` before and after".
- **Its tests:** `a_file_left_in_tmp_by_an_attempt_is_a_write_outside`, using a replaying agent from `tests/runner.rs`.
- **Confidence:** medium. A listing misses a write that is then deleted.

### T11 Codex's children, turns and conversations are reported unavailable although its rollouts link them

- **Symptom:** in repository 04, `counts.children` is "unavailable: collaboration items do not enumerate every child", and `delegation.*` is "Codex does not report child shares or parent blocking". Yet `tokens` holds the root and two `child` scopes. Each child rollout's `session_meta` has `source.subagent.thread_spawn.parent_thread_id`, `depth` and `agent_path` (for example `/root/protocol`), and each thread's rollout has its own `token_count` events: 62, 30 and 28 in 04.
- **Root cause:** the counts are built from the root's JSON lines alone. `benchmarks/src/agent/codex.rs:480` sets `children` unavailable, and `:481-487` sets `conversations` to a lower bound of 1. The named-rollout collection already holds the linked threads (`rollouts.rs:57`, `children: Vec<ChildThread>`) and scopes their usage, but feeds no count. benchmarks.md 15 still asks "whether they link child threads to their parents".
- **The clean fix:**
  - `children` becomes observed: the threads whose parent chain reaches the root.
  - Responses per thread are observed, and child shares and parent-blocked time stay unavailable.
  - Close 15's Codex question: "codex-cli 0.160.0 links each child by `source.subagent.thread_spawn.parent_thread_id`". Its trust key is `projects.<root>.trust_level = "trusted"` (`overrides.json`).
  - Codex's own spend stays unavailable: it reports none, and the common priced basis covers it.
- **Its tests:**
  - `linked_codex_children_are_counted_from_their_rollouts`.
  - `codex_responses_per_thread_come_from_token_counts`.
  - Both use recorded rollouts trimmed from repository 04.
- **Confidence:** high.

### T12 The private Codex is pinned by version and `PATH`, not by digest

- **Symptom and cause:** `standard.pin.toml` holds `version = "0.160.0"` only. `Configuration` has no digest field (`benchmarks/src/agent/mod.rs:215-221`). `prepare.rs:47-51` compares the `--version` text, and the program is whatever comes first on `PATH` (README, "Live services"). The SHA-256 is recorded (`12eb3e81...`, report line 18) but not checked. The overrides' comment still says "keeping authentication in the user's CODEX_HOME" (`overrides.toml:2`), against 10's private `CODEX_HOME`.
- **The clean fix:**
  - The pin gains `sha256` for `bin/codex` and `package = "tools/codex-0.160.0"` under the harness's state.
  - Setup refuses a mismatch, as for smith arms.
  - Design: 10 gains 13.41/13.44's sentence (section 3).
- **Its tests:** `a_codex_binary_whose_digest_differs_from_its_pin_is_refused`.
- **Confidence:** high.

### T13 The repository task: a cold build inside the timer, and a large-effect rule only in the report

- **Symptom and cause:**
  - `shell-cwd/task.toml` has no `setup`. The README (`:159-160`) says "Both arms start from the same unbuilt copy, with no shared attempt cache", so each agent's first build is in task wall. 5.2 says `setup` "warm[s] caches outside the timer", and 10 says caches are "warmed outside the timer".
  - The repository summary has `design: "single"` and 0 comparisons (`jq '.comparisons|length'`). The disjoint-range rule (13.132) was applied by hand in the report.
- **The clean fix:**
  - Version 2 of `shell-cwd` gains `setup = [["cargo","build","--offline","--locked","--workspace","--all-targets"]]` with the attempt's profile variables.
  - The summary computes 13.132's ranges for the `repository` design, and states "large effect" or "none".
- **Its tests:**
  - `a_repository_summary_marks_disjoint_ranges_as_large_effects`.
  - `overlapping_or_lower_bound_ranges_claim_no_effect`.
- **Confidence:** high on the facts. How long the cold build takes was not measured here.

### T14 `validate` never runs a task's own checks on its reference

- **Symptom and cause:** `benchmarks/src/validate.rs` runs the grader on the seed and on the reference, and only that (`rg protected validate.rs`: no hit). So a task whose reference fails its own `protected` or outcome checks still validates. Under T1's fix, a reference that adds tests, as asked, would expose a wrong `protected`.
- **The clean fix:** `validate` also applies `protected` and every outcome check to the reference's settled copy. 3.1's "Every grader fails on the seed and passes on the task's reference solution" gains "and the reference passes the task's outcome and protected checks".
- **Its tests:** `a_reference_that_changes_a_protected_path_fails_validation`.
- **Confidence:** high.

## 3. Proposal 19's rulings

Code is at `689c15d0`. None of the nine is in the design.

| Ruling | Its words (decisions.md 13) | Code now | Document, section | The sentence the design should gain |
|---|---|---|---|---|
| 13.14 | "00.27 ... takes an exclusive file lock over the whole build; a second build waits." | done: `arm.rs:346-361` `build_lock`, test `:378` | benchmarks.md 3.2, after "built in the harness's own clone" | "One build runs at a time: it holds an exclusive lock over the clone and its target, and a second waits." |
| 13.91 | "`smith-bench baseline` gains a selector of one group (agent, configuration digest, model) ... each suite is summarized once, whole, and each baseline cut from it." | done: README `:56` `--group`, `offline.rs:310` | benchmarks.md 9, Baselines | "A suite is summarised once, whole; each baseline is cut from that summary by one command naming one agent, configuration digest, model and effort." |
| 13.98 | "the dispatcher counts a passed explicit startup-refusal story ... as zero provider requests, judgment tells an expected startup refusal from a setup failure" | done: `batch.rs:117`, `:526` | benchmarks.md 8.3, after the ends table | "A task whose story is a startup refusal is judged by its checks: its refusal is the agent's end, not `Refused(setup)`, and it made no provider request." |
| 13.99 | "the dispatcher charges zero tokens to the suite's pool for an attempt whose every provider attempt is observed and was refused with a 4xx before any output, the result's usage staying unavailable" | done: `batch.rs:526` `rejected_before_output` | benchmarks.md 5.3, the suite's `max_tokens` | "An attempt whose every provider request was refused with a 4xx before output charges no tokens to the suite's budget; its usage stays unavailable." |
| 13.116 | "The charter's `effort` is an optional text of at most 32 bytes (schema `text32`) ... no catalog." | done in charter (`charter.rs:419-421`); literal also `compaction.rs:38`, `smith-charter/src/lib.rs:29` | protocol/limits.md 2.4, "Fixed by the vocabulary" | "**a model's effort:** a name of at most 32 bytes, the charter schema's; a provider's list of names is the model's declaration (shell.md, 6.1), never the vocabulary's." |
| 13.132 | "disjoint ranges are a large effect, overlapping ranges are none; a missing or lower-bound measurement keeps its reason and claims no separation ... no significance is inferred from three attempts." | not done: summary has no repository comparison (T13) | benchmarks.md 3.1, the repository row, and 9 | "A repository task's large effect: per permitted metric, the arms' observed ranges over three attempts do not overlap; overlapping, missing or lower-bound ranges claim none." |
| 13.41 | "the official release asset for 0.160.0 ... unpacked to `~/.local/state/smith-bench/tools/codex-0.160.0/bin/codex` ... first on `PATH` for each command that runs Codex. Nothing writes to `~/.codex`" | `PATH` order only; no digest check (T12) | benchmarks.md 10, the Codex bullet | "Codex runs from a private copy of its pinned release's whole package under the harness's state, never the user's install; its pin names the executable's SHA-256, and setup refuses another." |
| 13.44 | "The private Codex is the release's whole package" (`codex-package-...tar.gz`: `bin/codex-code-mode-host`, `codex-path/rg`, `codex-resources/`) | package present (`codex-package.json` "0.160.0") | the same sentence as 13.41 | (in 13.41's sentence: "whole package") |
| 13.39 | "io drops a second child signal in flight without telling its owner." | still so: skein `crates/skein-io/src/process.rs:243-245` returns without telling anyone | skein io.md 3.4 (and 6, Processes, the Signals bullet) | io.md 3.4 says only "a child waits and may have one signal in flight". It bounds the count, but it does not say what a second `Signal` request does. It needs one of the options in Q8. |

## 4. Questions for the designer

1. **Fixture version 2: how is "did not weaken tests" checked?** The options are:
   - (a) drop `tests` from `protected`, and rely on the grader's copy of the seed's tests;
   - (b) protect the seed's existing test files and name a new test file in the prompt;
   - (c) a new "lines retained" check.

   Recommendation: (a). It is already built (T1). It changes no prompt. It judges behaviour, not bytes. (b) fails all 30 recorded attempts, and (c) adds a check no other task needs.
2. **What is the parity comparison of: each agent as it ships, or both with equal capabilities?** Recommendation: as it ships, so pins change only paths, model, effort and account (T3, T4, T5). Delegation and window size become declared experiments. Why: the plan's goal is smith's performance, and today's numbers measure a smith that no user runs. If Codex ships with sub-agents on and smith off, the summary says so beside each comparison. The user's unanswered 07.13 decides smith's side.
3. **Should the repository prompt keep "Subagents may be used at your discretion"?** Recommendation: drop it in version 2. Codex's own text spawns only when asked, so the sentence is what made it spawn (2 in each of 3 attempts), and smith-as-shipped cannot.
4. **Where does the one price catalog live?** Recommendation: the product's presets, read by the harness as a library at its own commit (T6), and `prices.toml` deleted. Reading `smith check --json` cannot price Codex on the same basis.
5. **Which `[calibration]` fields does the task file hold, and who writes them?** Recommendation: T7's block, written by `smith-bench calibrate`, committed with its summary. A probe gates only when calibrated there.
6. **Are the native faces deleted now, with the 10 pre-product baselines retaken?** Recommendation: yes, in the parity plan's first harness step (T8). New baselines must be taken anyway once the pins change (T3, T4).
7. **Does the scope gate list `/tmp` until contained trees exist?** Recommendation: yes, plus a `$TMPDIR` line in the repository task's version 2 (T10).
8. **13.39, for skein's designer: what does io do with a second `Signal` while one is in flight?** The options are:
   - (a) drop it, and tell the owner with an event or a refusal;
   - (b) keep one pending, the strongest;
   - (c) say in io.md that it is dropped silently, and that owners must not send one.

   Recommendation: (c) now, as one sentence in 3.4. Smith's interrupts already go through skein-world controls (13.39). (b) changes io's operation counts, a design change for no current user.
9. **Does the repository task warm its build outside the timer?** Recommendation: yes, in version 2 (T13). Both agents pay the same cold build today, which adds variance to a three-attempt comparison and is not the agent's work.

## 5. What other themes must say

- **Compaction:** the repository attempts compacted under a harness window of 131,072, not the product's 272,000 (T4). Codex's root threads reached 131,454 prompt tokens without compacting at its 258,400 window. A compaction finding from those runs must be re-measured on the preset window.
- **Caching:** smith's cache-read volume was 0.20 to 0.24 of Codex's in compare, and about 0.36 in repository. smith sent `originator: pi` and `user-agent: pi (linux; x86_64)`; Codex's rollouts name `originator: codex_exec`; the product sends neither header (T4). Routing on those headers is a possible cause, to test as an affinity arm.
- **Delegation:** Codex's children are linked and countable (T11). A delegation comparison needs T3's declared experiment, not the default arms.
- **Prompt and role:** smith ran with an empty role (`instructions=""`), a "## Sub-agents" section and a "## Budget" section that Codex has no counterpart of (T3, T4, T5). A prompt-level finding from these runs describes the harness's smith.
- **Budgets:** smith's "64 turns and 839 seconds" text came from the fixtures, not from smith (T5).
- **Statistics and reporting:** 13.132 must be computed by the summary (T13). Prices must move to one basis (T6) before any spend ratio is a finding. Any rerun is a new task version (T1, T2, T13), never compared with version 1.
- **Reliability:** the five `Failed(model/malformed)` compaction attempts (report line 5) ran on the harness's 131,072 window too (T4).
