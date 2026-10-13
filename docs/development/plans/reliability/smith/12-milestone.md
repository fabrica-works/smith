# Session 12: the milestone

Provisional, 2026-10-12. Nothing of this session landed in the first run.
It measures what the plan built and publishes it: fixture `03-planner`
and the repository task `shell-cwd`; the summaries' basis for comparing
agents, and the delegation each result records; then, on the `main` this
plan ships, the probes, the comparison against Codex, the repository run
and the cheap experiments; last, a benchmark report with the erratum to
the 2026-10-08 one. It compares smith on `codex` with Codex only. Claude
Code is not part of this plan: the first run's `smith/11-claude.md` is the
next pass (`later.md`), and this session cites no result of it. It fixes
nothing in smith.

## Read first

- smith `docs/design/benchmarks.md`, sections 2, 3.1, 4.1, 5.2 to 5.4,
  6.1, 8.1, 8.2, 9, 10, 11.3 to 11.5 and 15; `protocol/events.md`, 3.4
  and 3.6.
- `decisions.md`, 4.1 to 4.4, 4.7, 5.8, 5.11, 7.4, 7.11, 9.6 to 9.8,
  12.4, 12.11, 12.12 and 12.17; `findings/benchmarks.md`, B1 to B3 and section
  4; `findings/delegation.md`, D12.
- The 2026-10-08 report, `docs/development/benchmarks/smith-codex-2026-10-08.md`
  at `49057ae` (GitHub's `bench/smith-codex-2026-10-08`).
- For 12.1 and 12.2, the spike's material, on debian-16gb-hel1-1 only:
  `~/src/rust/smith-benchmark-2026-10-08/` (`tasks/*/03_planner*`,
  `grader-review.json`, `repository/`, `runs/iteration7-07_shell_cwd-codex-2/`).

## What the design asks

- **Comparisons** (benchmarks.md, 9 and 11.5): smith and Codex at the
  working tier, "at least five per arm, interleaved"; "Every attempt
  counts"; "Only outcome, wall time, tokens by convention and spend on
  one basis are compared across agents"; not a ranking (2).
- **The repository task** (3.1, 5.2, 10, 11.4): "frozen with every
  dependency vendored"; graded "serially on an idle machine"; three
  attempts per agent, "outcomes and large effects only".
- **Delegation** in each result (8.1); **`when_asked`** (5.3);
  **experiments** (11.3); **the erratum** (decisions.md, 5.11; D12).

## Choices this plan makes

- **The agents.** smith as it ships, through `smith exec` and the events
  face, with smith 00's pin; `sub_agents` is the user's (decisions.md,
  5.8), and results record it. Codex runs as
  `agents/codex/standard.pin.toml` pins it (codex-cli 0.160.0); another
  installed version refuses setup.
- **The repository reference** is the spike's Codex attempt
  `iteration7-07_shell_cwd-codex-2`, which passed the oracle and every
  gate: its 13 changed files, without its 18 lines of dated logs in
  `docs/development/workflow.md`.
- **Spend on one basis** is the harness's: each result's total tokens at
  `agents/prices.toml`, the list prices smith's presets give each model
  (shell.md, 6.1, "Prices"); notional (decisions.md, 7.11).
- **No plan report:** the run ends with `done.md` and the retrospective
  (development.md, 7.6 and 15). 12.7 is smith's benchmark report.
- **12.3 to 12.6 land data only** (summaries, suite files): no code, so
  no smoke (README, rule 1).
- **Live budgets.** Attempts, wall and tokens are rules: the suites carry
  them as `max_wall_seconds` and `max_tokens`, and the runner stops
  dispatching past them. Dollars are a guide, notional, from the spike's
  token medians at $1.25, $0.125 and $10 per million fresh, cache-read and
  output tokens: aim for at most twice the figure; past it, stop after
  the call and report.

  | Step | Attempts | Calls | Wall | Tokens | Dollars |
  |---|---|---|---|---|---|
  | 12.3 probes | at most 80 | 2, one per suite | 2 h | 8 M | 10 |
  | 12.4 comparison | 30 | 1 | 2 h | 8 M | 10 |
  | 12.5 repository | 6 | 1 | 3 h | 50 M | 20 |
  | 12.6 compaction | 10 | 1 | 1 h | 12 M | 5 |

## Rules that bind it

- **Live runs go through `heavy --gate`** (README, rule 3): a `suite`
  runs whole, in one call, with `--arm <the directory>` that `heavy cargo
  run -p smith-bench -- build HEAD` printed (smith 00.27).
- **It fixes nothing in smith.** A failure is a finding: the step's
  report names it, with the attempt's archive path. Edits stay in
  `benchmarks/` and `docs/development/benchmarks/`.
- **Never touch the user's logins** (README, rule 7).

## Landed in the first run

Nothing: none of this session's increments landed.

## Increments

### 12.9 The basis across agents
Scope: summaries compare across agents only what benchmarks.md, 9 allows:
  task and process wall time; each normalised token field (8.2) across
  conventions, both named; spend priced from each result's total tokens
  at `agents/prices.toml`. Tool calls by name are summarised per arm,
  never compared across agents. Split from the first run's 12.4, so that
  the code lands before the run.
Needs: nothing outside this session.
Files: `benchmarks/src/summary.rs` (722 lines; its comparison loop,
  lines 398-433, moves to a new `summary/compare.rs`); `src/models.rs`;
  `agents/prices.toml` (new: `gpt-6-luna`, then the working model);
  `src/check.rs` (reads the table); `src/main.rs` (`summarise` reads it);
  `src/lib.rs`; `README.md`. Users of `summarise`: `src/tests.rs` (ten
  calls), `tests/offline.rs:115`, `tests/cache_probes.rs:47`.
Shape: `PriceTable`, `Prices`, `read_prices`;
  `SummaryComparison.conventions`; `summarise(.., prices)`;
  `compare::{comparisons, counterpart}` (sketches). `metric_values` adds
  `spend_usd/priced` (unavailable, saying why, when the model is not in
  the table or a total field is not observed) and `tool_calls/<name>`. A
  token field compared across agents is recorded as `tokens/<field>`.
Test shape: `benchmarks/src/tests.rs`, registered in
  `summary_baseline_and_comparison_contracts` (tests.rs:811); reuse
  `recorded_result` (tests.rs:550, a Codex result with observed tokens)
  as tests.rs:724 does. Tests: the one below,
  `cross_agent_spend_is_priced_from_one_table`,
  `a_model_missing_from_the_price_table_leaves_priced_spend_unavailable`,
  `teardown_resources_and_tool_calls_are_never_compared_across_agents`,
  `tool_calls_by_name_are_summarised_per_arm`. Each under 20 ms.
  ```rust
  fn cross_agent_token_fields_pair_across_conventions_and_name_both() {
      let mut results = Vec::new();
      for (arm, agent, convention, fresh) in [
          ("codex", crate::Agent::Codex, crate::Convention::Codex, 80),
          ("smith", crate::Agent::Smith, crate::Convention::SmithEvents, 160),
      ] {
          for index in 0..5 {
              let mut result = recorded_result();
              result.identity.attempt = format!("{arm}-{index}");
              result.identity.arm = arm.into();
              result.identity.agent = agent;
              for record in &mut result.tokens {
                  record.convention = convention;
                  record.usage.fresh = crate::Measure::Observed { value: fresh + index };
              }
              results.push(result);
          }
      }
      let table = crate::PriceTable::default();
      let summary = crate::summarise(&results, "compare", "working", crate::Design::Interleaved, 17, None, &table)
          .expect("interleaved summary");
      let fresh = summary.comparisons.iter().find(|item| item.metric == "tokens/fresh").expect("compared");
      assert_eq!(fresh.conventions, Some((crate::Convention::Codex, crate::Convention::SmithEvents)));
      assert!(matches!(&fresh.result, crate::Measure::Observed { value } if (value.ratio - 162.0 / 82.0).abs() < 1e-9));
  }
  ```
Proof: unit, the five tests, which fail on the parent; once, in the
  focused suite, under 20 ms each. 12.4's run is its live use; nothing
  else runs for it.
Don't: compare `teardown_ms`, `grading_wall_ms`, `cpu_ms`,
  `peak_rss_bytes`, `cache/reuse`, `tool_calls/*` or `delegation/*`
  across agents; change a result's own `spend`; price a model the
  presets do not price.
Size: at most 180 lines of production code.
Removes: nothing.

### 12.10 What a run delegated
Scope: the result's delegation part (benchmarks.md, 8.1; D12). smith's
  events face fills it from `conversation.opened` (`requested`, `share`,
  `clamped`), `tool.completed.child` (`stop`, `turns`, `time_ms`) and
  main's `sub_agent` call spans (events.md, 3.4 and 3.6). The Codex
  adapter, and smith's legacy face until smith 00.21 removes it, leave
  each part unavailable with its reason. Summaries add `delegation/turns`,
  `delegation/children`, `delegation/children_budget` and
  `delegation/parent_blocked_ms`. Split from the first run's 12.5.
Needs: nothing outside this session.
Files: `benchmarks/src/result.rs`; `src/agent/smith/events/delegation.rs`
  (new); `src/agent/smith/events.rs`; `src/agent/smith/observe.rs`;
  `src/run.rs` (1,129 lines: `blank` at 81, the collection's Smith branch
  at 564; at most 15 lines more); `src/summary.rs`; `src/lib.rs`;
  `tests/recorded/results/attempt.json`. `blank` builds the only
  `AttemptResult`; `src/tests.rs` and
  `tests/{offline,probes,cache_probes,pipeline}.rs` read recorded ones.
Shape: `AttemptResult.delegation` and its types (sketch);
  `events/delegation.rs`'s `delegation` and `blocked_ms`. Event loss
  leaves each part unavailable, as `Observer::counts` does; an unknown
  `stop`, `part` or `by` leaves `children` unavailable, naming it. The
  result's version is 2 from the first landing that changes it, this
  one or smith 00.21 (README, rule 5).
Test shape: `benchmarks/tests/events.rs`, registered in
  `typed_event_stream_classification_usage_and_author_predicates`
  (events.rs:1454); reuse `child_run` (events.rs:554: a share of 2
  turns, 1,000 ms and 10, ended `Budget` after 2 turns and 100 ms) and
  `observe` (events.rs:9). Tests: the one below,
  `a_clamped_share_names_its_part_and_bound`,
  `parent_blocked_time_is_the_union_of_sub_agent_spans`,
  `event_loss_leaves_delegation_unavailable`,
  `an_unknown_child_stop_leaves_children_unavailable`; in `src/tests.rs`,
  `delegation_medians_join_the_arm_summary`. Each under 20 ms.
  ```rust
  fn a_childs_share_stop_turns_and_time_are_recorded() {
      let delegation = observe(&child_run()).delegation();
      let Measure::Observed { value: children } = delegation.children else { panic!("children observed") };
      assert_eq!(children.len(), 1);
      assert_eq!(children[0].share, smith_bench::Share { turns: 2, time_ms: 1000, spend: 10 });
      assert_eq!(children[0].stop, smith_bench::ChildStop::Budget);
      assert_eq!((children[0].turns, children[0].time_ms), (2, 100));
  }
  ```
Proof: unit, the six tests, which fail on the parent; once, in the
  focused suite, under 20 ms each. 12.5 records it live;
  `budget/starvation` would show it alone, and is not run for this.
Don't: estimate a part an agent does not report; read Codex's child
  threads for shares; change smith's event records.
Size: at most 200 lines of production code.
Removes: nothing.

### 12.1 Fixture 03-planner
Scope: `fixtures/03-planner` (benchmarks.md, 5.2 and 11.4), ported as
  smith 00 ported 01 and 02: `seed/` from the spike's
  `tasks/fixtures/03_planner` (identical to branch
  `bench/fixture-seed/03_planner`); `grader/` from
  `tasks/grading/03_planner/{hidden,regression}.rs`, as a crate;
  `reference/` from `tasks/reference-solutions/03_planner`; the prompt
  `tasks/prompts/03_planner.txt`, joined. The grader is strengthened first
  (11.4; B3): `random_dags_match_independent_ready_scan` labels its graphs
  by a seeded permutation, independent of topological order, so that
  sorting the selected names cannot pass. Also `when_asked` and
  `--include` (5.3); `suites/compare.toml` lists 03 under `when_asked`,
  with 12.4's bound as its `max_wall_seconds` and `max_tokens`.
  `agents/models.toml` pins the working tier, `gpt-6-astra` at `high`
  (decisions.md, 12.17), which `validate_suite` requires (models.rs:27);
  smith 00.13 pins it (decisions.md, 13.17), and 12.1 leaves it as it is.
Needs: smith 00's fixtures 01 and 02 [00.13]: their layout, the grader
  crate's form, `validate` and `suites/compare.toml`; smith 00's one
  command per probe run [00.27], whose `suite` this gives `--include`.
  debian-16gb-hel1-1.
Files: `benchmarks/tasks/fixtures/03-planner/` (new); `src/suite.rs`;
  00.27's `suite` command and `src/batch.rs` (`check_requests` admits
  included tasks); `suites/compare.toml`; `agents/models.toml`;
  `README.md`. Users of `Suite`:
  `src/batch.rs`, `src/fix.rs`, `src/main.rs`, `src/tests.rs:152`.
Shape: `Suite.when_asked: Vec<String>`, `#[serde(default)]`; `include`
  (sketch); `validate_suite` refuses a `when_asked` name that names no
  task or variant, or that `tasks` also lists.
Test shape: `benchmarks/src/tests.rs`, registered in
  `suite_resolution_and_guard_budget_contracts` (tests.rs:780), written
  like `a_suite_resolves_task_variants_selectors_and_configuration_pins`
  (tests.rs:177) with `sample_suite` (tests.rs:152) and
  `sample_catalogue` (tests.rs:157): with `probes/recorded/short` under
  `when_asked`, `validate_suite` selects `["probes/recorded"]` and
  `include` adds it. Tests: `a_when_asked_task_runs_only_when_a_run_includes_it`,
  `an_include_outside_when_asked_is_refused`,
  `a_when_asked_name_must_name_an_existing_task`. The task's manifest,
  digest and hidden files: `every_catalogue_manifest_is_checked_offline`
  (tests/offline.rs:23). Each under 20 ms.
Proof: process, `heavy cargo run -p smith-bench -- validate
  fixtures/03-planner` (the spelling is 00.13's), once: the grader fails
  on the seed and passes on the reference; and once on the reference
  with its scheduling replaced by sorting the selected names (kept in
  `<run>/logs/smith-12/`), which fails
  `random_dags_match_independent_ready_scan`. At most 10 minutes; no
  provider call.
Don't: change 01's or 02's tasks; weaken a spike assertion.
Size: at most 80 lines of production code, beside the task's files.
Removes: nothing.

### 12.2 The repository task shell-cwd
Scope: `repository/shell-cwd` (benchmarks.md, 3.1, 5.2, 10 and 11.4).
  Its bundle, `$XDG_STATE_HOME/smith-bench/bundles/shell-cwd-56c0c59/`:
  smith `56c0c59` (branch `bench/repository-seed/07_shell_cwd`), skein
  `f5fca44` from the local skein repository, every crate vendored by
  `cargo vendor`, built offline once through `heavy`. `tasks/repository/shell-cwd/`:
  `task.toml` (the spike's prompt, joined; `deadline_seconds = 1200`, the
  spike's); `seed/bundle.toml`; `grader/`, the spike's
  `repository/grading/oracle.rs` as a crate with path dependencies on
  `../workspace`'s crates, and `[[grade]]` commands: the oracle, `cargo
  fmt --check`, all-target Clippy, both suites under the seed's own
  `nextest.toml` and its 15 s and 60 s budgets; `reference/` (Choices).
  The runner seeds a repository attempt from its bundle; the offline tier
  checks a present bundle's digest and says when it skipped.
  `suites/repository.toml`: smith and Codex, working tier, `design =
  "single"`, three repetitions, `rerun_failed = 0`, 12.5's bound.
Needs: smith 00's `validate` and grader crate form [00.13]; smith 00's
  one command per probe run [00.27]; 12.1's working tier; debian-16gb-hel1-1.
Files: `benchmarks/src/bundle.rs` (new); `src/run.rs` (`prepare` at 441
  copies from `seed_source`; at most 15 lines more); `src/check.rs`;
  00.13's `validate`; `src/lib.rs`; `tasks/repository/shell-cwd/`;
  `suites/repository.toml`; `README.md` (the bundle's recipe).
Shape: `BundleManifest`, `Pin`, `BundleCheck`, `read_bundle_manifest`,
  `check_bundle`, `seed_source` (sketch). A bundle's digest is
  `seed_digest` of its directory, so the seed rules hold for it.
Test shape: `benchmarks/tests/offline.rs`, registered in
  `recorded_catalogues_and_results` (offline.rs:135); a recorded bundle
  `tests/recorded/bundles/tiny/` (two small files) and its task
  `tests/recorded/tasks/repository-seed/`. Tests: the one below,
  `a_bundle_whose_digest_differs_is_refused`,
  `a_present_bundle_matches_its_manifest`,
  `a_repository_seed_comes_from_its_bundle`. Each under 20 ms.
  ```rust
  fn an_absent_bundle_is_skipped_and_said() {
      let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded");
      let manifest = smith_bench::read_bundle_manifest(&root.join("tasks/repository-seed/seed/bundle.toml"))
          .expect("recorded manifest");
      let checked = smith_bench::check_bundle(&manifest, &root.join("bundles/none"));
      assert_eq!(checked, Ok(smith_bench::BundleCheck::Absent));
  }
  ```
Proof: process, `heavy --gate cargo run -p smith-bench -- validate
  repository/shell-cwd` (the spelling is 00.13's; the gate lane is the
  idle machine its budgets need), once: on the seed the oracle passes its
  3 controls and fails 13 of 14 features, so the grade fails; on the
  reference every `[[grade]]` command passes within the frozen budgets.
  At most 30 minutes; no provider call.
Don't: carry the spike's `grade.py`; change the seed's `nextest.toml`,
  budgets or `Cargo.lock`; commit the bundle or a vendored crate.
Size: at most 150 lines of production code.
Removes: nothing.

### 12.3 The milestone's preflight
Scope: once the `main` this plan ships has landed, the `smoke` and
  `probes` suites once at the small tier (benchmarks.md, 4.1, 9, 11.5):
  smith on `codex`; Codex on the outcome halves of `smoke-tools` and
  `large-write`. Commit the probes summary, with drift notices against
  smith 00's probes baseline; the smoke run's wall goes in the commit
  body. A calibrated probe failing twice of two is a regression (9): 12.3
  reports it as a question (development.md, 8.7); 12.4 to 12.6 wait for
  its answer. A probe session 13 recorded as not calibrated (decisions.md,
  12.4), or a quarantined one, is reported and gates nothing.
Needs: smith 00, 03 to 08, 10 and 13's steps landed or dropped (`plan
  status`); 12.s1 (decisions.md, 13.128); 12.s2 for the entries not
  dispatched by its first probes call (13.129); smith 00's `probes` suite and baselines on events
  [00.21]; smith 00's one command per probe run [00.27].
Files: `benchmarks/summaries/<run>.json` (new). Shape: none, a run.
Test shape: none new; tests/offline.rs:28 reads the summary.
Proof: live, `heavy --gate cargo run -p smith-bench -- suite smoke --arm
  <dir>`, then `suite probes --arm <dir>`, and `heavy cargo run -p
  smith-bench -- summarise <runs> probes small single <seed> <baseline>`; once,
  within the budget table. A pass: the summary committed, and no
  calibrated probe printed as `Regression`.
Don't: rerun beyond section 9's one rerun; calibrate a probe.
Size: no production code. Removes: nothing.

### 12.s1 The five probe tasks at six conversations take five
Added (decisions.md, 13.128), before 12.3 runs again. Scope: `[smith]
conversations` goes from 6 to 5 in `benchmarks/tasks/probes/{budget,
cache-reuse,follow-up,large-write,paths}/task.toml`, as the unblock of
2026-10-13T06:01:28Z did for `failures`, `first-request` and
`smoke-tools` after the designer lowered the standard profile's
(09deb036); task versions, seeds, prompts, budgets, outcomes and every
other limit unchanged.
Needs: nothing.
Files: those five `task.toml` files.
Shape: none.
Test shape: none new; the offline tests that read tasks stay green.
Proof: offline: for each of the five tasks (and `budget`'s variants), the
harness's prepared settings pass `smith check` on the current arm (exit
0), each worst case recorded in the body; once. No live run.
Don't: change a task's version, outcome, budget or any other limit; touch
smith's code.
Size: no production code.

### 12.s2 `process-tree` follows the product's `group_stop`
Added (decisions.md, 13.129), before 12.3 finishes. Scope:
`benchmarks/tasks/probes/process-tree/task.toml` drops its `group_stop`
pin, so the product's 1 s applies (shell.md 6.4), and its `tool-timeout`
check's `grace_ms` becomes 1000; its `version` goes to 2. If
`tool-output`'s `timeout` variant's `group_stop = 250` reached the
product's settings, it is treated the same way (its task's version with
it); the body says which.
Needs: nothing.
Files: those task files; benchmark tests that read them (README rule 8).
Shape: none.
Test shape: none new.
Proof: offline: each changed task's prepared settings pass `smith check`
on the current arm; once. No live run.
Don't: change any other task, limit or the product.
Size: no production code.

### 12.4 The comparison against Codex
Scope: `suites/compare.toml` with 03 included (benchmarks.md, 9, 11.5):
  smith and Codex at the working tier, interleaved, five attempts per
  arm per fixture, the order drawn from the run's seed, no reruns. Commit
  the summary: 12.9's comparisons, each a median ratio with its interval
  against the minimum detectable effect, or none detected; pass counts
  with exact intervals; counts by end; drift notices where smith 00 left
  a compare baseline; the audit sample, audited as the answer to
  "Questions to raise", 12.4, says.
Needs: 12.9; 12.1; smith 00's fixtures 01 and 02 [00.13]; smith 00's
  one command per probe run [00.27]; 12.3 passed on the
  same `main`.
Files: `benchmarks/summaries/<run>.json` (new). Shape: none, a run.
Test shape: none new; tests/offline.rs:28 reads the summary.
Proof: live, `heavy --gate cargo run -p smith-bench -- suite compare
  --include fixtures/03-planner --arm <dir>`, one call of 30 attempts, then
  `summarise <runs> compare working interleaved <seed>`; once, within the
  budget table. A pass: ten attempts per fixture summarised, and for
  `task_wall_ms`, `process_wall_ms`, `tokens/fresh`, `tokens/cache_read`,
  `tokens/output` and `spend_usd/priced` a comparison or its reason.
Don't: rerun an attempt; leave a failed attempt out of a median; rank
  the agents.
Size: no production code. Removes: nothing.

### 12.5 The repository task, once
Scope: `suites/repository.toml` once (benchmarks.md, 3.1, 10, 11.5):
  three attempts each for smith and Codex at the working tier, serially,
  grading in the same call. Commit the summary: pass counts, counts by
  end, per-arm medians and ranges, and 12.10's `delegation/*` medians.
  Each attempt's end, turns, children and blocked seconds go to 12.7's
  report, from the raw results.
Needs: 12.2; 12.10; smith 00's one command per probe run [00.27];
  12.3 passed; debian-16gb-hel1-1 until the bundle question is
  answered.
Files: `benchmarks/summaries/<run>.json` (new). Shape: none, a run.
Test shape: none new; tests/offline.rs:28 reads the summary.
Proof: live, `heavy --gate cargo run -p smith-bench -- suite repository
  --arm <dir>`, one call of six attempts, then `summarise
  <runs> repository working single <seed>`; once, within the budget
  table; each attempt's hard deadline is 1,200 s, then at most 180 s of
  grading. A pass: six attempts summarised, each graded.
Don't: rerun an attempt; change the frozen task to suit an agent.
Size: no production code. Removes: nothing.

### 12.6 Experiments, where cheap
Scope: of benchmarks.md, 11.3, `compaction` runs over
  `history/small-window`: smith on `codex`, small tier, two `[[arm]]
  source = "override"` arms of `compaction_threshold` (shell.md, 6.5),
  the shipped 80% and 60%, interleaved, five attempts each, in
  `suites/compaction.toml`; an arm for what the new window holds runs
  only if smith 08 lands a setting for it. The other three do not run,
  and 12.7 says why: `affinity`'s first two arms are no setting on `main`
  (affinity is the domain's; `crates/smith-agent-shell/src/config.rs:392`
  refuses affinity headers) and its third is not built (`later.md`);
  `cache-lifetime` runs on `anthropic`, the next pass; `edit-format`:
  each dialect has one edit format (protocol/llm.md, 12; `later.md`).
Needs: smith 00's `history` probe and `small-window` [00.16], with smith
  08's compaction; smith 00's one command per probe run [00.27], with
  the suite's override arms; 12.3 passed.
Files: `benchmarks/suites/compaction.toml`, `benchmarks/summaries/<run>.json`
  (new). Shape: none, a run.
Test shape: none new; tests/offline.rs:28 reads the suite and summary.
Proof: live, `heavy --gate cargo run -p smith-bench -- suite compaction
  --arm <dir>`, one call of ten attempts, then `summarise <runs> compaction small interleaved <seed>`;
  once, within the budget table. A pass: both arms summarised and
  compared.
Don't: add a setting or switch to make an arm (benchmarks.md, 4.2: "No
  switch is added only for this"); change the shipped threshold.
Size: no production code. Removes: nothing.

### 12.7 The new benchmark report
Scope: `docs/development/benchmarks/smith-codex-<date>.md` in smith, the
  2026-10-08 report's successor; that one stays at `49057ae`, its links
  pointing into `crates/smith`, gone from `main`. Sections: the method
  (harness, tiers, isolation and the user's logins, faces, what is
  unavailable and why, citing benchmarks.md); what changed since
  `30259d8`, one row per probe of 11.1 with its calibration as session 13
  recorded it and the steps it guards; the probes (12.3); the comparison
  (12.4), effects with intervals or none detected, and the audits; the
  repository task (12.5), one row per attempt; the experiments (12.6),
  each run or why not, the decisions each puts to the user (decisions.md,
  4.4, 4.7, 7.4) and the `later.md` items they unlock or keep deferred;
  what waits for the next pass, one line pointing to `later.md`: Claude
  Code, not part of this plan, smith on `anthropic`, `cache-lifetime`;
  and, under the heading "Erratum to the 2026-10-08 report"
  (decisions.md, 5.11; D12):
  - the "Shared 64-turn budget" row credits the repository successes to
    the 256-turn ceiling; both used fewer than 128 turns, and the failure
    ran out of time with turns unused. The mechanism was starvation: a
    child with no share took the whole remaining budget, main had no
    reserve to finish with, and no model was told its budget or time;
  - two of the five capped children ended unanswered, their work lost;
  - its statements on serial writable delegation stand: that was policy;
  - its cache recipe is replaced by affinity in the domain (decisions.md,
    4.1 to 4.3).
Needs: the summaries of 12.3 to 12.6 on `main`.
Files: the report (new). Shape: Markdown, in the style of
  `docs/development/`. Test shape: none, Markdown only.
Proof: none beyond `land --docs` (development.md, 9.4); the report names
  each summary it draws on by path, and `git ls-files
  benchmarks/summaries` lists each.
Don't: copy the 2026-10-08 report onto `main`; rank the agents; cite a
  raw result's path.
Size: no code; aim for at most 300 lines, past which it is cut to its
  tables. Removes: nothing.

## Interface sketches

This session owns them. 12.10's part crosses into every result: its types
are written field by field, each mirroring its event record without that
record's `Unknown`. 12.1's field and 12.2's manifest are formats the
offline tier reads strictly. 12.9's table and comparisons stay in the
summary. Derives as their neighbours have them.

```rust
// benchmarks/src/models.rs (12.9)
/// `agents/prices.toml`: list prices by model, as smith's presets give them (shell.md, 6.1); notional.
pub struct PriceTable { pub models: BTreeMap<String, Prices> }
/// One model's prices in micro-dollars per million tokens, by normalised field (benchmarks.md, 8.2).
pub struct Prices { pub fresh: u64, pub cache_read: u64, pub output: u64 }
pub fn read_prices(file: &Path) -> Result<PriceTable, Refusal>;

// benchmarks/src/summary.rs (12.9)
// `SummaryComparison` gains, after `metric` (a token field across agents is `tokens/<field>`):
//     /// Each arm's token convention, reference first, for a token field across conventions.
//     pub conventions: Option<(Convention, Convention)>,
pub fn summarise(results: &[AttemptResult], suite: &str, tier: &str, design: Design, seed: u64,
    reference: Option<&Baseline>, prices: &PriceTable) -> Result<RunSummary, String>;

// benchmarks/src/summary/compare.rs (12.9)
pub(crate) fn comparisons(groups: &[ArmSummary], grouped: &BTreeMap<(String, String), Vec<&AttemptResult>>,
    prices: &PriceTable, seed: u64) -> Result<Vec<SummaryComparison>, String>;
/// The candidate's key compared with the reference's metric. One agent: the same key. Across
/// agents: `task_wall_ms`, `process_wall_ms` and `spend_usd/priced`, the same key;
/// `tokens/<rc>/<field>`, the candidate's `tokens/<cc>/<field>` with `Some((rc, cc))`; else `None`.
fn counterpart(metric: &str, reference: &ArmSummary, candidate: &ArmSummary)
    -> Option<(String, Option<(Convention, Convention)>)>;

// benchmarks/src/result.rs (12.10); `AttemptResult` gains `pub delegation: Delegation`.
/// What a run delegated (benchmarks.md, 8.1), filled by the agent's adapter.
pub struct Delegation {
    /// Main's turns: smith's `run.completed.turns`.
    pub turns: Measure<u64>,
    /// Each child, in the order it opened.
    pub children: Measure<Vec<Child>>,
    /// The union of main's `sub_agent` call spans.
    pub parent_blocked_ms: Measure<u64>,
}
/// One child: `conversation.opened`'s first three fields, `tool.completed.child`'s rest.
pub struct Child {
    pub conversation: u64,
    pub requested: Option<Requested>,
    pub share: Share,
    pub clamped: Vec<Clamp>,
    pub stop: ChildStop,
    pub turns: u64,
    pub time_ms: u64,
}
/// Mirrors `smith_events::Requested`.
pub struct Requested { pub turns: Option<u64>, pub seconds: Option<u64> }
/// Mirrors `smith_events::Share`; `spend` in smith's unit, a millionth of a notional dollar.
pub struct Share { pub turns: u64, pub time_ms: u64, pub spend: u64 }
/// Mirrors `smith_events::Clamp`.
pub struct Clamp { pub part: SharePart, pub by: ShareBound }
/// The part lowered: its turns, or its time.
pub enum SharePart { Turns, Time }
/// What lowered it: the parent's pool above its reserve, or the asker's call deadline.
pub enum ShareBound { Reserve, Deadline }
/// How it ended: an answer it chose, an answer its share ended, or unanswered: failed, closed.
pub enum ChildStop { End, Budget, Failed, Closed }

// benchmarks/src/agent/smith/events/delegation.rs (12.10)
pub(crate) fn delegation(observer: &Observer) -> Delegation;
/// The union of `[start, end)` spans, in `t_ms`.
fn blocked_ms(spans: &mut [(u64, u64)]) -> u64 {
    spans.sort_unstable();
    let (mut total, mut open) = (0_u64, None::<(u64, u64)>);
    for &(start, end) in spans.iter() {
        open = match open {
            Some((from, to)) if start <= to => Some((from, to.max(end))),
            Some((from, to)) => { total += to - from; Some((start, end)) }
            None => Some((start, end)),
        };
    }
    total + open.map_or(0, |(from, to)| to - from)
}

// benchmarks/src/suite.rs (12.1); `Suite` gains, with `#[serde(default)]`:
//     /// Tasks that run only when a run names them with `--include` (benchmarks.md, 5.3).
//     pub when_asked: Vec<String>,
/// The default selection plus the asked tasks; a name not under `when_asked` is refused.
pub fn include(file: &Path, suite: &Suite, selected: &[String], asked: &[String]) -> Result<Vec<String>, Refusal>;

// benchmarks/src/bundle.rs (12.2)
/// `seed/bundle.toml`: a repository task's seed, its bundle's contents and digest (benchmarks.md, 5.2).
pub struct BundleManifest {
    /// The bundle's directory under `$XDG_STATE_HOME/smith-bench/bundles/`.
    pub name: String,
    /// The frozen source commit.
    pub source: String,
    pub pins: Vec<Pin>,
    /// `seed_digest` of the bundle's directory.
    pub sha256: String,
}
/// One vendored git dependency's pin, such as skein at `f5fca44`.
pub struct Pin { pub name: String, pub commit: String }
/// Present and matching, or absent from this machine: skipped, and said so.
pub enum BundleCheck { Matches, Absent }
pub fn read_bundle_manifest(file: &Path) -> Result<BundleManifest, Refusal>;
/// A present bundle whose digest differs is a refusal.
pub fn check_bundle(manifest: &BundleManifest, bundles: &Path) -> Result<BundleCheck, Refusal>;
/// The directory a seed is copied from, and its digest: `seed/`, or a repository task's bundle.
pub fn seed_source(task: &Task, task_directory: &Path, bundles: &Path) -> Result<(PathBuf, String), Setup>;
```

## Questions to raise when reached

- **12.2, the bundle's home** (benchmarks.md, 15: "where a bundle is kept
  durably, so that another machine can run the task"). Proposed: only
  debian-16gb-hel1-1's harness data until the user answers; 12.5 runs
  there.
- **12.4, audits** (15: "how many attempts a run samples, and who audits
  them"). Proposed: the three sampled attempts and every disagreement,
  audited by the session's model against each attempt's archived diff
  and calls, the list shown to the user in 12.7.
- **12.5, "large effects"** (3.1) has no threshold. Proposed: an effect is
  large when the arms' ranges of a metric do not overlap; the report
  states nothing else as a difference.
- **12.6, the compaction threshold and the new window's contents**
  (decisions.md, 4.7): the user decides, with 12.6's summary as evidence.

## Done when

- The two tasks are on `main`: `git -C ~/src/rust/smith ls-tree
  --name-only origin/main benchmarks/tasks/fixtures/03-planner/task.toml
  benchmarks/tasks/repository/shell-cwd/task.toml` prints both (today
  neither).
- Nine steps landed: `git -C ~/src/rust/smith log origin/main
  --format='%(trailers:key=Step,valueonly)' | rg -c
  '^smith-12\.([1-7]|9|10)$'` prints 9 (today 0).
- 12.3 to 12.6 each committed a summary: for each `<k>` of 3 to 6, `git
  -C ~/src/rust/smith log origin/main --grep "^Step: smith-12.<k>$"
  --name-only --format= | rg -c '^benchmarks/summaries/'` prints at
  least 1.
- The offline tier reads them: `heavy cargo nextest run -p smith-bench
  --test offline`, in a smith checkout at `origin/main`, passes (today 2
  tests pass).
- The report and its erratum are on `main`: `rg -c 'Erratum'
  docs/development/benchmarks/smith-codex-*.md` prints at least 1 (today
  no such file).
