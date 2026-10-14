# Session 04: calibration, probes and the verdict

2026-10-14. Calibration lives in commit bodies, has no home per variant,
and gates nothing: `smoke` runs its probes whether calibrated or not
(findings T7, F13). `history/small-window` cannot calibrate as written
(K6). And no command gives parity's verdict (M10). This session puts each
calibration in its task file, gives `history` its version 2 with a
`compaction-choice` check, adds the `parity` suite and the `parity`
command, gates probes by their calibration, and calibrates
`first-request`. It changes `benchmarks/` only.

## Read first

- smith `docs/design/benchmarks.md`, sections 3.1, 3.2, 4.2, 5.2 (its
  `[calibration]` row), 5.3, 9, 11.1 (`history`), 11.5, 14 and 16.
- `findings/harness.md` T7; `findings/probes.md` F1, F13;
  `findings/compaction.md` K4, K6; `findings/measure.md` M3 to M6, M10 and
  section 5; `decisions.md` 1.1, 1.2, 3.5 and 4.6.
- skein `testing-strategy.md`, sections 2.1 and 8; `programming-model.md`,
  section 10.2.

## What the design asks

- **A `[calibration]` block per task and per variant,** written by
  `calibrate` and committed with the summary it names: the arm's commit
  and SHA-256, the date, that summary, the attempts run, the passing
  attempts, the model deviations, the control and the attempts that
  failed before, and the digests of the variant's prompt, seed and checks.
  A probe without one gates nothing; a block whose digests differ gates
  nothing (3.1, 5.2, 14).
- **Calibrated means** ten passes within at most twelve attempts on the
  binary it guards, the other attempts failed only by the model, and a
  failure on the binary before the fix or on a control arm (3.1).
- **`smoke`** runs `first-request`, `smoke-tools`, `large-write`,
  `budget/child-quota`, `budget/wrap-up` and `failures/one-turn`, each
  only while its task file holds a calibration; a probe's failure in
  `probes` counts as a regression only once calibrated (9, 11.5).
- **`history` version 2:** the answer's form stated whole, the values out
  of order, 32 files, and `small-window`'s `compaction-choice` check:
  every compaction request at tool choice `none` (11.1; decisions.md 4.6).
- **The `parity` suite and verdict:** 16.1 to 16.4, quoted in 04.3 and
  04.4.

## Choices this plan makes

- A variant's block is a `[variant.calibration]` table inside its
  `[[variant]]`; the task's own is a top-level `[calibration]`. The
  README's "Done when" counts them so.
- `calibrate` writes the block into the task file in the worktree it runs
  from, with the summary it commits beside; it never commits.
- The prompt, seed and checks digests are SHA-256 over: the variant's
  effective prompt bytes; the seed's tree digest (`seed_sha256`); the
  bytes `toml::to_string` writes for the variant's effective outcome,
  event and grade checks with its `deciding` list.
- The `credentials` block on `main` lacks every new field, and names a
  summary 03.1 removes: 04.1 removes it, and `credentials` waits in
  `later.md` (decisions.md, 10.6).

## Rules that bind it

- **The live budget** (README, rule 10): 04.6's line is 12 attempts.
- **Live runs** (rule 9), and **the smoke** (rule 8) on each landing that
  changes code, with rule 8's exceptions.
- **Splits** (rule 1).

## Increments

### 04.1 Calibration in task files
Scope: benchmarks.md 3.1, 5.2 and 14's manifests (findings T7, F13).
Needs: nothing.
Files: `benchmarks/src/task.rs` (`Calibration`, `:258-265`; `Variant`,
  `:269-292`), `benchmarks/src/probe.rs` (`count`, `:118-129`; the block,
  `:589`, `:606`), `benchmarks/src/fix.rs` (`calibration_suite`, `:14`),
  `benchmarks/src/check.rs` (`:199-213`),
  `benchmarks/tasks/probes/credentials/task.toml`, `benchmarks/README.md`,
  `benchmarks/tests/probe_command.rs`, `benchmarks/tests/offline.rs`.
Shape: `Calibration` as sketched below, on `Task` and on `Variant`;
  `calibrate PROBE [--control COMMIT]` runs the control first when one is
  named, until its first failure and at most 2 attempts (a control that
  passes both ends the command, exit 1, with no block written), then the
  arm until its tenth pass or until ten are out of reach within 12; it
  counts model deviations apart from smith's failures (8.3's
  ends), and on success writes the block into the task file and the
  summary to `benchmarks/summaries/`; `check` refuses a block that does
  not parse, and reports (without refusing) a block whose digests differ,
  as gating nothing. A calibration with no runnable control writes
  `control = "none: <why>"` and `failed_before = []`.
Test shape: `benchmarks/tests/probe_command.rs`:
  `a_calibration_block_names_its_summary_and_failing_control`,
  `a_variant_carries_its_own_calibration`,
  `ten_passes_within_twelve_attempts_is_calibrated`,
  `a_model_deviation_is_counted_apart_from_smiths_failures`,
  `a_block_whose_prompt_digest_differs_gates_nothing`. Under 20 ms each,
  on recorded results.
Proof: unit, those tests.
Don't: commit from `calibrate`, or keep the old three-field block.
Size: 220 lines.
Removes: `credentials`' block, `calibration_suite`'s constant.

### 04.2 `history` version 2 and `compaction-choice`
Scope: benchmarks.md 11.1's `history` row (findings K4, K6; decisions.md
  4.6).
Needs: nothing.
Files: `benchmarks/tasks/probes/history/` (`task.toml`, `seed/`),
  `benchmarks/src/agent/smith/events/` (a new `compaction_choice.rs`
  beside `final_turn.rs`), `benchmarks/src/agent/smith/events/checks.rs`,
  `benchmarks/src/check.rs` (the known checks),
  `benchmarks/tests/events.rs`, `benchmarks/tests/recorded/`.
Shape: `version = 2`; the prompt ends "finish with exactly the word
  ALPHA, one space, then the sum."; the 32 files' values shuffled by a
  fixed permutation, the sum unchanged in form (`^ALPHA <sum>$`), the seed
  digest renewed; `small-window`'s `[[variant.event]]` gains `check =
  "compaction-choice"`, in its `deciding`. `compaction-choice` passes when
  every request of each `compaction` conversation was asked at tool choice
  `none`, and fails naming the first that was not; with no compaction it
  is unavailable.
Test shape: `compaction_choice_fails_an_auto_compaction_request` and
  `compaction_choice_passes_when_every_compaction_asks_none`, on recorded
  traces (12.6's attempt 02 for the first, a trace edited to `none` for
  the second). Under 20 ms each.
Proof: unit, those tests; the live proof is 07.3's calibration.
Don't: change the chain's length (32) or the window of `small-window`.
Size: 100 lines.
Removes: version 1's prompt and values.

### 04.3 The `parity` suite
Scope: benchmarks.md 5.3 (a task's own repetitions), 11.5's `parity` row
  and 16.1.
Needs: nothing.
Files: `benchmarks/src/suite.rs` (`Suite`, `:67-86`),
  `benchmarks/src/batch.rs` (`:64-69`), `benchmarks/suites/parity.toml`
  (new), `benchmarks/tests/overrides/suite.rs`.
Shape: `Suite` gains `[repetitions_by_task]`, a table from a task's name
  to its repetitions, which the interleaving honours. `parity.toml`:
  `tier = "working"`, `design = "interleaved"`, `repetitions = 8`,
  `rerun_failed = 0`, tasks `fixtures/01-backoff`,
  `fixtures/02-line-decoder`, `fixtures/03-planner` and
  `repository/shell-cwd` with `"repository/shell-cwd" = 5`; agents smith
  (`standard`) and Codex (`standard`), on `codex`; `max_wall_seconds` and
  `max_tokens` sized at least for 58 attempts (aim for 28,800 s and
  150,000,000 tokens; a run that stops on either says so).
Test shape: `a_suite_gives_one_task_its_own_repetitions`,
  `the_parity_suite_runs_58_attempts_interleaved`. Under 20 ms each.
Proof: unit, those tests; `heavy cargo run -p smith-bench -- check`
  exits 0.
Don't: rerun a failed attempt in `parity`.
Size: 60 lines.
Removes: nothing.

### 04.4 The `parity` section and command
Scope: benchmarks.md 9's parity bullet, 14's parity bullet and 16.2 to
  16.4 (findings M3 to M6, M10). 16.4: "`smith-bench parity SUMMARY
  --commit SHA` recomputes that section offline from the summary's
  per-attempt values (5.4). It refuses a section that disagrees with
  them, and a summary whose smith arm was not built from `SHA`. It prints
  one line per criterion, and exits 2 when the verdict is incomplete
  (16.3), whatever the criteria; otherwise 1 when a criterion fails or
  16.3 finds smith not on par, and 0 at parity. A run of `suite parity`
  exits with the same code."
Needs: smith 03's attempt rows [03.2]; 04.3.
Files: `benchmarks/src/parity.rs` (new), `benchmarks/src/stats.rs`
  (`resample`, `:142`, made usable by `parity.rs`; a stratified
  bootstrap), `benchmarks/src/summary.rs` (`summarise` writes the
  section), `benchmarks/src/main.rs`, `benchmarks/src/probe.rs`
  (`judgment`, `:618-628`, for `suite parity`'s exit), `benchmarks/src/check.rs`
  (a committed parity section is recomputed), `benchmarks/README.md`,
  `benchmarks/tests/parity.rs` (new), `benchmarks/tests/recorded/parity/` (new).
Shape: as sketched below. O: smith's pooled passes at least ⌈L · 29⌉,
  `L` the lower bound of Codex's exact 95% interval (`pass_interval`), 26
  when Codex passes 29. FT and FC: 10,000 resamples within each task and arm,
  seeded by the run's seed, of the ratio smith over Codex of (Σ over the
  fixtures' attempts ÷ passes there), the sum of `task_wall_ms` for FT and of
  `priced_usd` for FC, a draw with no pass in an arm counted as infinite;
  each holds when its 97.5th percentile is below 1.20. RT, RC: `bootstrap_ratio` of medians over
  `shell-cwd`'s five per arm, upper bound below 1.20 and 1.50. 16.3's ends
  map to `Incomplete` or to unsolved as written there. A refusal (a
  disagreeing section, another smith commit) prints its reason and exits
  2. `Parity.smith_commit` is the smith arm's full commit, which the
  README's "Done when" reads.
Test shape: `benchmarks/tests/parity.rs`, on summaries built in the test:
  `parity_passes_when_every_criterion_is_below_its_margin`,
  `parity_names_the_failing_criterion_and_exits_one`,
  `parity_reference_is_the_codex_arm_whatever_the_arm_names`,
  `parity_refuses_a_summary_of_another_smith_commit`,
  `an_unavailable_task_wall_makes_the_verdict_incomplete`,
  `a_setup_refusal_or_harness_error_makes_the_verdict_incomplete`,
  `timeouts_and_budget_ends_count_as_unsolved_with_their_wall`,
  `the_stratified_bootstrap_keeps_each_tasks_size_in_every_draw`,
  `a_ratio_just_below_its_margin_holds_and_just_above_fails`. Each under
  20 ms but the bootstrap's, under 0.5 s. Reference test:

  ```rust
  #[test]
  fn parity_names_the_failing_criterion_and_exits_one() {
      let summary = common::parity_summary(Rows { smith_wall_ms: 130_000, codex_wall_ms: 100_000, ..Rows::equal() });
      let verdict = parity::recompute(&summary.attempts, summary.seed, &summary.priced_at).expect("verdict");
      assert_eq!(verdict.verdict, Verdict::NotOnPar);
      assert_eq!(failing(&verdict), ["FT", "RT"]);
      assert_eq!(verdict.verdict.code(), 1);
  }
  ```
Proof: unit, those tests; then, once, `heavy cargo run -q -p smith-bench
  -- parity benchmarks/tests/recorded/parity/on-par.json --commit <the commit
  that file names>`: five criterion lines and exit 0 (process). The
  file, committed here, is the summary `parity_passes_when_every_criterion_is_below_its_margin`
  reads.
Don't: compute a criterion from medians alone, or read arm names to pick
  the reference.
Size: 350 lines.
Removes: nothing.

### 04.5 Probes gated by their calibration
Scope: benchmarks.md 3.1, 9 and 11.5's `smoke` and `probes` rows.
Needs: 04.1.
Files: `benchmarks/src/probe.rs` (`prepare`, `:333-418`),
  `benchmarks/src/batch.rs` (`:507-520`), `benchmarks/suites/smoke.toml`,
  `benchmarks/tests/probes.rs`.
Shape: `suite smoke` runs, of its listed probes, those whose task file
  holds a current block (its digests equal the task's), and prints the
  others as "not calibrated: skipped", which changes no exit; a run with
  none to run exits 2, incomplete. In `probes`, an uncalibrated probe's failures are reported
  apart and count no regression. `smoke.toml` lists 11.5's six probes,
  `failures/one-turn` added.
Test shape: `smoke_skips_an_uncalibrated_probe`,
  `an_uncalibrated_probe_failure_is_not_a_regression`. Under 20 ms each.
Proof: unit, those tests.
Don't: drop a probe from `probes`.
Size: 80 lines.
Removes: the static gate of `smoke`.

### 04.6 `first-request` calibrated
Scope: benchmarks.md 3.1 and 11.5's `calibrate`, for the smoke's
  `first-request` with its own overrides (findings F1; decisions.md 3.4,
  7.1).
Needs: smith 02's product pins and one face [02.1, 02.2]; smith 03's
  summary version 3 [03.1]; 04.1; 04.s1 (ruling 112); 08.s1 (ruling 114).
Files: `benchmarks/tasks/probes/first-request/task.toml` (the block),
  `benchmarks/summaries/` (its summary).
Shape: no new code. Build the arm, then `heavy --gate cargo run -p
  smith-bench -- calibrate probes/first-request --arm <dir>`, with no
  control: the behaviour it guards was in place before the first product
  face (`0d5604f9`), the oldest binary the harness runs after 02.2, so no
  runnable binary fails it. The block records `control = "none: the
  behaviour predates the oldest runnable face"`, and the commit body says
  so.
Test shape: none new.
Proof: live, once: that command, 10 passes within 12. At most 12
  attempts, 15 minutes, after 04.s1; the step's row is 22 attempts, the
  10 of its first calibration (`probes-first-request-ecd28c01-20261014T184751Z`,
  stopped by 04.s1's defect) and these 12 (ruling 112); with the 1 of
  `probes-first-request-25388ac8-20261014T192031Z`, refused at startup, 23
  (ruling 114).
Don't: change the probe's prompt, seed or checks.
Size: none.
Removes: nothing.

### 04.s1 Calibration reads only deciding checks
Scope: benchmarks.md 3.1 (the arm's other attempts fail "only by the
  model, never by smith") and 3.2 (an unobservable passive check "decides
  nothing"); ruling 112.
Needs: 04.1.
Files: `benchmarks/src/probe.rs` (`model_deviation`, `:142-162`), its
  tests.
Shape: `model_deviation` reads only the checks the result's
  `outcome.deciding` names; a check outside it, observed or unavailable,
  neither makes nor breaks a deviation. A deciding check that fails and is
  not the model's (`answer`, `budget-report`, `file-content:`,
  `file-digest:`, `pattern:`, or unavailable "not exercised:") still
  returns false.
Test shape: `an_unavailable_advisory_check_leaves_a_model_deviation`
  (attempt 10 of the run above, trimmed: `file-content:first.txt` false,
  `teardown` and `headroom/*` unavailable and not deciding),
  `a_failed_deciding_contract_check_is_no_model_deviation`. Under 20 ms
  each.
Proof: unit, those tests; the smoke (rule 8).
Don't: change a check, a task, or which checks decide.
Size: 20 lines.
Removes: nothing.

## Interface sketches

```rust
// smith-bench, src/task.rs (04.1): a `[calibration]` or `[variant.calibration]` block.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    pub binary: String,
    pub binary_sha256: String,
    pub date: String,
    /// `summaries/<run>.json`, committed with the block.
    pub summary: String,
    pub attempts: u32,
    pub passes: Vec<String>,
    pub deviations: Vec<String>,
    /// The control's commit, or `none: <why>`.
    pub control: String,
    pub failed_before: Vec<String>,
    pub prompt_sha256: String,
    pub seed_sha256: String,
    pub checks_sha256: String,
}

// smith-bench, src/parity.rs (04.4): a parity run's section and verdict.
pub struct Criterion {
    /// `O`, `FT`, `FC`, `RT` or `RC`.
    pub name: String,
    pub statistic: f64,
    pub interval: (f64, f64),
    pub margin: f64,
    pub holds: bool,
}

pub enum Verdict { OnPar, NotOnPar, Incomplete }

impl Verdict {
    /// 0 at parity, 1 not on par, 2 incomplete.
    pub fn code(self) -> u8;
}

pub struct Parity {
    /// The arm whose agent is Codex.
    pub reference_arm: String,
    pub smith_commit: String,
    pub seed: u64,
    /// The harness commit whose presets priced the spend.
    pub priced_at: String,
    pub criteria: Vec<Criterion>,
    pub verdict: Verdict,
}

/// Recompute a parity section from a summary's attempt rows.
pub fn recompute(rows: &[AttemptRow], seed: u64, priced_at: &str) -> Result<Parity, String>;
```

## Done when

- The README's "Done when" item 4 prints at least 1 (`first-request`).
- `heavy cargo nextest run -p smith-bench -E 'binary(parity)'` passes,
  once.
