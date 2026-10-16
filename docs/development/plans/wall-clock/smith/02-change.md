# Session 02: a change on the way to a verdict

2026-10-16. In the parity run, 06.2's `role` comparison took responses
an attempt from 7.13 to 8.33 and concluded "no difference"; FT then
failed the first closing run (decisions.md, section 4). benchmarks.md
16.6 now has each change to what the model is sent, after a plan's
baseline, run against its parent on the verdict's tasks, report every
criterion, project the product of the landed changes onto the baseline,
and stop on a question when one moves the wrong way past the baseline's
interval. The harness has none of it: `parity` selects its arms by the
names "codex" and "smith" and refuses two smith commits, no suite takes
a parent, and `summarise` takes no run summary as an input (findings
A22). This session builds the `parity-change` suite, its report and its
stop. It changes `benchmarks/` only.

## Read first

- smith `docs/design/benchmarks.md`, sections 4.2, 9, 11.5 (the
  `parity-change` row), 14 ("A change's report"), 16.1 to 16.4 and 16.6.
- `decisions.md`, section 4; `findings/planning-audit.md` A21, A22.
- skein `testing-strategy.md`, section 2.1; `programming-model.md`,
  section 10.2.

## What the design asks

- **The suite** (11.5, 16.6): the `parity` suite's tasks, five attempts
  per arm each, two smith arms, a change and its parent, interleaved, no
  rerun.
- **Its report** is what `summarise` writes, and nothing else is asked
  for: each criterion of 16.2 computed with the change in smith's place
  and its parent in Codex's, the change's ratio to its parent with its
  interval, and for O each arm's passes; the reference arm is the one
  built from the change's parent, chosen by its commit. Its projection:
  the baseline's statistic times the product of the ratios of every
  change landed since the baseline, this one included, beside the
  baseline's interval; `summarise` takes the baseline's committed
  summary and those changes' summaries as named inputs. Beside them,
  deciding nothing: each arm's responses an attempt, the time record's
  parts (requests, compaction, tools) and fresh tokens, per task.
- **The stop:** a projection of FT, FC, RT or RC above the baseline's
  upper bound; the change's passes below the lower bound of the exact
  95% interval of its parent's passes over as many attempts; or a
  report that is incomplete (a criterion unavailable, an arm with no
  pass on a task). Whatever the comparison concludes.
- **One run per commit;** a `parity-change` run is never a verdict and
  never pooled with `parity` runs.

## Choices this plan makes

- A suite file says `change = true`; such a suite runs two smith arms
  from the command line, `change` from `--commit` (or `--arm`) and
  `parent` from `--parent` (or `--parent-arm`), with `parent` its
  reference. No other suite takes `--parent`.
- The named inputs are `--baseline-run <summary>` and `--landed
  <summary>`, repeated, on `summarise`, on `suite parity-change` and on
  a new `smith-bench change <summary>`; the summary records each by path
  and SHA-256, so that `change` recomputes the report offline, and
  `check` runs it on every committed `parity-change` summary. The
  README's rule 13 lists them.
- `suite parity-change` exits 0 when nothing stops the step, 1 when a
  criterion moved the wrong way, 2 when the report is incomplete, as
  `suite parity` exits by its verdict (README, rule 12). `change` exits
  0 when its recomputed section agrees with the summary's, whatever the
  stop it prints, and 3 when it refuses one; `check` fails on 3.
- With the change in smith's place and its parent in Codex's (16.6),
  16.3's sides follow: a lower bound on the parent's side is taken as
  observed and flagged; one on the change's side, but a spend that
  misses only retried failures, makes the report incomplete.
- The live run of the pipeline at one attempt per task needs a committed
  baseline of version 4, which only 06.1 makes: 06.1 runs it, before
  any full run (development.md 5.5).

## Rules that bind it

- **Formats move once** (README, rule 14): the summary is at version 4
  from smith 01.1, and this session's sections change it under 4.
- **No change to a criterion or a margin** (`code-organization.md`,
  section 7): the criteria are 16.2's, computed over a reference chosen
  by commit.
- **The smoke** (README, rule 8) on each landing.

## Increments

### 02.1 A suite with change arms
Scope: benchmarks.md 11.5's `parity-change` row and 16.6 ("Its proof")
  (findings A22).
Needs: nothing.
Files: `benchmarks/src/suite.rs` (binary arms, `:47-54`; the named
  reference, `:241-250`), `benchmarks/src/probe.rs` (the `suite`
  options, `:526`; arms by commit, `:334-336`, `:443-449`, `:499-514`;
  each attempt's arm and commit, `:245-246`, `:266`, `:340-343`),
  `benchmarks/src/batch.rs`, `benchmarks/src/check.rs` (suite
  validation), a new `benchmarks/suites/parity-change.toml`,
  `benchmarks/tests/` (the suite tests, `tests/overrides/suite.rs`),
  `benchmarks/README.md`.
Shape: `Suite` gains `change: bool`. A suite with `change = true`
  declares no `[[arm]]`, no Codex agent and no `reference`; `suite`
  requires `--parent <commit>` or `--parent-arm <dir>` for it beside
  `--commit` or `--arm`, and refuses either for any other suite. Its two
  arms are `change` and `parent`, both smith, each recorded with its
  build commit as `agent_version`; `parent` is the reference.
  `parity-change.toml`: the four `parity` tasks, five attempts per arm
  each, `design = "interleaved"`, no rerun, `change = true`.
Test shape: `benchmarks/tests/overrides/suite.rs` (its suite tests are
  the reference to copy): `a_change_suite_runs_a_change_and_its_parent`,
  `a_change_suite_without_a_parent_is_refused`,
  `a_parent_given_to_another_suite_is_refused`,
  `a_change_suite_declaring_arms_is_refused`,
  `the_change_suites_reference_is_its_parent`. Under 20 ms each.
Proof: offline, `heavy cargo run -q -p smith-bench -- check` validates
  `parity-change.toml`; the suite runs live first at 06.1, at one
  attempt per task (02's "Choices").
Don't: run Codex in a change suite, or name the arms' commits in the
  suite file.
Size: 120 lines.
Removes: nothing.

### 02.2 The change's report
Scope: benchmarks.md 16.6 ("Its report", its first and third bullets)
  and 14 ("A change's report", the ratios) (findings A22).
Needs: 02.1's change arms [02.1]; smith 01's rows at version 4, with
  responses [01.2]; smith 01's O over each arm's attempts [01.3].
Files: `benchmarks/src/parity.rs` (the row selection by agent name,
  `:87-89`, `:115-116`, `:131-133`, `:164-173`; `from_summary`,
  `:240-248`), a new `benchmarks/src/parity/change.rs`,
  `benchmarks/src/summary.rs` (where `summarise` writes the parity
  section, `:518-520`), `benchmarks/src/time_record.rs` (read),
  `benchmarks/tests/parity.rs`, `benchmarks/tests/common/parity.rs`, a
  new `benchmarks/tests/recorded/parity/change.json`.
Shape: the criteria take their two arms as arguments: `fn
  criteria(rows: &[AttemptRow], subject: &ArmKey, reference: &ArmKey) ->
  Criteria`, where `ArmKey` names an arm by agent and commit; the
  `parity` section calls it with smith's arm and Codex's, unchanged in
  what it writes. On a run of a change suite, `summarise` writes a
  `change` section instead of a `parity` one (sketch below): the change
  and parent commits; FT, FC, RT and RC as ratios of the change to its
  parent with their intervals; O as each arm's passes and the lower
  bound of the parent's `pass_interval` (`parity.rs:177`, the exact 95%
  interval) over as many attempts; and, per task and arm, the medians of
  responses, of the time record's requests, compaction and tools
  seconds, and of fresh tokens. The reference is the arm whose
  `agent_version` is the parent's commit. Lower bounds follow 02's
  "Choices": the parent's taken as observed and flagged, the change's
  incomplete.
Test shape: `benchmarks/tests/parity.rs`, beside
  `parity_reference_is_the_codex_arm_whatever_the_arm_names` (`:85`):
  `a_change_report_takes_its_parent_by_commit_whatever_the_names`,
  `a_change_report_gives_each_criterions_ratio_to_its_parent`,
  `a_change_report_gives_o_as_each_arms_passes`,
  `a_change_report_keeps_responses_and_time_beside`,
  `a_lower_bound_on_the_changes_side_is_incomplete`,
  `a_lower_bound_on_the_parents_side_is_taken_as_observed`,
  `the_parity_section_is_unchanged_by_the_arm_arguments` (the recorded
  `on-par.json` gives its five criteria's statistic, interval and
  `holds` as before, as literals). The recorded `change.json` is a
  40-row summary of two smith arms: the first five rows of each task per
  agent of `on-par.json`, Codex's relabelled `smith` with the parent's
  commit as `agent_version`. Under 20 ms each.
Proof: offline, those tests; live first at 06.1, at one attempt per
  task (02's "Choices").
Don't: compute the parity section from anything but smith's arm and
  Codex's; change a criterion's statistic or its bootstrap.
Size: 200 lines.
Removes: the literal arm names in `parity.rs`'s row selection.

```rust
// benchmarks/src/parity/change.rs; each type derives what `parity.rs`'s
// section types derive (Debug, Clone, PartialEq, Serialize, Deserialize).
/// A `parity-change` run's report: the change against its parent (benchmarks.md 16.6).
pub struct ChangeSection {
    pub change_commit: String,
    pub parent_commit: String,
    /// FT, FC, RT, RC: the change over its parent, each as `parity.rs`'s `Criterion`.
    pub ratios: Vec<Ratio>,
    /// O: each arm's passes.
    pub outcome: Outcome,
    /// Per task and arm, deciding nothing.
    pub beside: Vec<Beside>,
}
/// One criterion's ratio of the change to its parent: its name ("FT", "FC", "RT", "RC") and value.
pub struct Ratio { pub name: String, pub value: Criterion }
/// Each arm's passes over its attempts, and the lower bound of the parent's `pass_interval`.
pub struct Outcome { pub change_passes: u32, pub parent_passes: u32, pub attempts: u32, pub parent_lower: f64 }
/// The medians reported beside, for one task and arm.
pub struct Beside { pub task: String, pub arm: String, pub responses: Measure<f64>, pub requests_s: Measure<f64>, pub compaction_s: Measure<f64>, pub tools_s: Measure<f64>, pub fresh_tokens: Measure<f64> }
```

### 02.3 The projection and the stop
Scope: benchmarks.md 16.6 ("Its report", the projection; "A move the
  wrong way stops it"; "The baseline"; "One run per commit") and 14 ("A
  change's report", the projections and the stop).
Needs: 02.2's change section [02.2].
Files: `benchmarks/src/parity/change.rs`, `benchmarks/src/probe.rs`
  (`parity_summary`, `:585-613`, and the `suite` options),
  `benchmarks/src/main.rs` (a `change` command beside `parity`,
  `:27-28`, `:67-107`), `benchmarks/src/summary.rs` (`read_summary`,
  `:649`), `benchmarks/src/check.rs` (`:93-108`),
  `benchmarks/tests/parity.rs`, `benchmarks/tests/recorded/parity/`.
Shape: `summarise` and `suite parity-change` take `--baseline-run
  <summary>` once and `--landed <summary>` any number of times; they
  refuse a baseline that is not a `parity` summary with a `parity`
  section, and a landed file that is not a `parity-change` summary. The
  section gains `projection: Vec<Projection>`, for FT, FC, RT and RC the
  baseline's statistic times the product of each landed summary's ratio
  and this one's, beside the baseline's interval; and `stop:
  Option<Stop>`: `Incomplete { why }` when any of its causes holds (a
  criterion unavailable, an arm with no pass on a task, an input
  refused); otherwise `Moved { criterion }` for the first that moved, in
  the order FT, FC, RT, RC, O (a projection above the baseline's upper
  bound, or O's change passes below `parent_lower`); otherwise `None`.
  The summary records each input's path and SHA-256. The suite exits 0
  with no stop, 1 on `Moved`, 2 on `Incomplete`; a run below its
  suite's attempts exits 2 whatever its stop, as 01.3 makes every
  `suite` do, and keeps its summary in its run directory. `smith-bench change
  <summary>` recomputes the section from the rows and the recorded
  inputs, prints one line per criterion, the projection and the stop,
  and exits 0 when it agrees with the stored section, 3 when it refuses
  (a disagreement, a missing input, an input whose SHA-256 differs);
  `check` runs it on every committed `parity-change` summary, failing on
  3.
Test shape: `benchmarks/tests/parity.rs`:
  `a_projection_just_below_the_baselines_upper_bound_goes_on`,
  `a_projection_just_above_the_baselines_upper_bound_stops`,
  `the_projection_multiplies_every_landed_changes_ratio`,
  `o_just_within_the_parents_bound_goes_on`,
  `o_just_past_the_parents_bound_stops`,
  `an_arm_without_a_pass_on_a_task_is_incomplete`,
  `a_landed_input_that_is_not_a_change_summary_is_refused`,
  `an_incomplete_report_stops_before_a_moved_criterion`,
  `the_change_command_refuses_a_section_that_disagrees`,
  `the_change_command_agrees_with_a_stopped_section`,
  `a_change_run_below_its_attempts_exits_two_whatever_its_stop`. Each builds its
  baseline and landed inputs from the recorded files, with literal
  statistics. Under 20 ms each.
Proof: offline, those tests and `heavy cargo run -q -p smith-bench --
  check`; live first at 06.1, at one attempt per task, against the
  plan's baseline (02's "Choices").
Don't: read the baseline from `baselines/`; pool a change run with a
  `parity` run; stop on a comparison's own "difference detected".
Size: 180 lines.
Removes: nothing.

## Questions to raise when reached

- None known.

## Done when

- `cd ~/src/rust/smith && test -f benchmarks/suites/parity-change.toml && rg -n '^change = true' benchmarks/suites/parity-change.toml` prints one line.
- `cd ~/src/rust/smith && heavy cargo nextest run -p smith-bench -E 'test(a_change_report_takes_its_parent_by_commit_whatever_the_names) | test(a_projection_just_above_the_baselines_upper_bound_stops) | test(a_change_suite_runs_a_change_and_its_parent)'` runs three tests, and they pass.
