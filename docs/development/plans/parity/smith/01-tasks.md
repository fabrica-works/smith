# Session 01: the graded tasks

2026-10-14. No fixture or repository attempt can pass today: the
fixtures protect the `tests/` their prompts ask the agent to change, the
repository task compares its protected files with a manifest instead of
the bundle, and with no `deciding` list every check decides, the always
unavailable passive checks included (findings T1, T2, M1). This session
gives the three fixtures and `shell-cwd` their version 2, says what
decides a graded task, refuses a comparison task that names a budget, and
lists the user's `/tmp` around each attempt. It changes `benchmarks/`
only.

## Read first

- smith `docs/design/benchmarks.md`, sections 3.1, 3.2, 5.2, 6.1 (its
  Budgets and Caches bullets), 8.3, 9 (its first and "Every attempt counts"
  bullets), 10 and 11.4.
- `findings/harness.md` T1, T2, T5, T10, T13, T14; `findings/measure.md`
  M1.
- skein `testing-strategy.md`, sections 2.1 and 8.

## What the design asks

- **A task never protects a path its prompt asks the agent to change;**
  that the seed's tests still pass is its grader's, which holds a copy of
  them. Each reference adds the tests its prompt asks for, and passes the
  task's outcome and protected checks (5.2, 11.4, 3.1).
- **Every check that reads the seed reads what the workspace was copied
  from:** the bundle, for a repository task (5.2). A task that builds
  warms its build in `setup` (5.2, 6.1).
- **A graded task decides on its end, its grader, its protected files and
  its tree's end.** A passive check that cannot be observed for an agent
  is reported beside, and decides nothing (3.2).
- **A comparison task names no `[budget]`;** `check` refuses one that
  does, or whose `deadline_seconds` is not below the run defaults'
  `budget.seconds` (6.1; shell.md 6.4: one hour).
- **The user's `/tmp` entries are listed before and after each attempt;**
  a new or changed one is a write outside the workspace (10).

## Choices this plan makes

- `deciding` for every fixture and for `shell-cwd` is `["end", "grade",
  "protected-unchanged", "tree-ended"]`, written in each task file; the
  judgment of a graded kind with no `deciding` is refused by `check`.
- The repository prompt's version 2 drops "Subagents may be used at your
  discretion." and adds "Temporary files go under `$TMPDIR`." (decisions.md,
  2.4; findings T10).
- `/tmp` is listed one level deep: each entry the user owns, with its kind,
  size and modification time; a change in any of them is a write.

## Rules that bind it

- **Formats move once** (README, rule 12): this session changes no format
  version; a task's own `version` follows benchmarks.md 5.2.
- **The smoke** (README, rule 8) on each landing that changes code; a
  landing that changes only Markdown, or only files under
  `benchmarks/tasks/`, `benchmarks/suites/`, `benchmarks/summaries/` or
  `benchmarks/baselines/`, skips it.
- **Splits** (README, rule 1).

## Increments

### 01.1 The fixtures' version 2
Scope: benchmarks.md 3.1, 3.2, 5.2, 6.1 and 11.4 for `01-backoff`,
  `02-line-decoder` and `03-planner` (findings T1, T5, T14).
Needs: nothing.
Files: `benchmarks/tasks/fixtures/{01-backoff,02-line-decoder,03-planner}/`
  (`task.toml`, `reference/`), `benchmarks/src/validate.rs`,
  `benchmarks/tests/offline.rs`.
Shape: each `task.toml` at `version = 2`; `protected = ["Cargo.toml",
  "Cargo.lock", "AGENTS.md"]`; `deciding = ["end", "grade",
  "protected-unchanged", "tree-ended"]`; no `[budget]` table. Each
  `reference/` adds one regression test to `tests/regression.rs`, as its
  prompt asks. `validate` also applies the task's `protected` check and
  every `[[outcome]]` check to the reference's settled copy, after its
  graders: `fn reference_checks(task: &Task, settled: &Path) ->
  Result<(), Refusal>` in `validate.rs`.
Test shape: `benchmarks/tests/offline.rs`, beside its task-manifest tests
  (reuse `benchmarks/tests/common/`): `no_fixture_protects_a_path_its_prompt_asks_to_change`,
  `every_fixture_grader_holds_the_seeds_test_functions` (each `#[test] fn`
  name in `seed/tests/*.rs` appears in `grader/tests/`),
  `a_reference_that_changes_a_protected_path_fails_validation` (a
  temporary task whose reference edits `Cargo.toml`). Each under 20 ms
  but the last, under 0.5 s. Reference test:

  ```rust
  #[test]
  fn no_fixture_protects_a_path_its_prompt_asks_to_change() {
      for id in ["01-backoff", "02-line-decoder", "03-planner"] {
          let task = common::fixture_task(id);
          assert!(!task.protected.iter().any(|path| path == "tests"), "{id} protects tests");
      }
  }
  ```
Proof: process: `heavy cargo run -p smith-bench -- validate
  fixtures/01-backoff fixtures/02-line-decoder fixtures/03-planner` exits
  0 (each grader fails on the seed and passes on the reference, and each
  reference passes its protected and outcome checks), once, within 10
  minutes.
Don't: change a prompt, a seed or a grader.
Size: 120 lines of production code.
Removes: the fixtures' `[budget]` tables and `tests` from their
  `protected`.

### 01.2 `shell-cwd` version 2
Scope: benchmarks.md 5.2 (the bundle), 6.1 (caches warmed, no budget),
  10 and 11.4 for `repository/shell-cwd` (findings T2, T5, T10, T13).
Needs: nothing.
Files: `benchmarks/tasks/repository/shell-cwd/task.toml`,
  `benchmarks/src/run.rs` (the snapshot at `:1050` and the checks'
  seed at `:908`), `benchmarks/src/grade.rs` (`checks`, `snapshot`),
  `benchmarks/src/bundle.rs`, `benchmarks/src/isolate.rs`,
  `benchmarks/tests/runner.rs` (or `offline.rs`), `benchmarks/tests/recorded/bundles/tiny/`.
Shape: `version = 2`; `deciding` as 01.1's; no `[budget]`; `setup =
  [["cargo", "build", "--offline", "--locked", "--workspace",
  "--all-targets"]]`, run with the attempt's build environment; the prompt
  without "Subagents may be used at your discretion." and with "Temporary
  files go under `$TMPDIR`." `prepare` returns the `Snapshot` of the
  source `seed_source` resolved, taken right after the copy and before
  `seed_commit` and `setup`; `attempt` never reads
  `task_directory.join("seed")` again, and `grade::checks` takes that
  snapshot.
Test shape: `a_repository_attempt_compares_protected_files_with_its_bundle`
  over `tests/recorded/bundles/tiny` (unchanged: `[]`; a changed
  `Cargo.toml`: `["Cargo.toml"]`), `the_seed_snapshot_comes_from_the_copied_source`
  (unit, in `run.rs`'s tests). Under 0.5 s each.
Proof: process: `heavy cargo run -p smith-bench -- validate
  repository/shell-cwd` exits 0, once, within 20 minutes.
Don't: change the bundle, its manifest's digest or the grader.
Size: 150 lines.
Removes: the snapshot of `<task>/seed` for a repository task.

### 01.3 What decides a graded task
Scope: benchmarks.md 3.2's "What decides" and 9 (findings M1).
Needs: 01.1 and 01.2 (the graded tasks name their `deciding`).
Files: `benchmarks/src/judgment.rs`, `benchmarks/src/check.rs`,
  `benchmarks/src/run/outcomes.rs`, `benchmarks/tests/outcomes.rs`.
Shape: `judge` decides an attempt on its task's `deciding` checks only; a
  passive check outside `deciding` is reported in the result and decides
  nothing. `check` refuses a task of any kind whose `deciding` is empty
  (a probe names its own, 9; a graded task names 01.1's four). At the
  plan's base only the four graded tasks lack one (`rg --files-without-match
  '^deciding' benchmarks/tasks -g task.toml`), and 01.1 and 01.2 add
  theirs.
Test shape: `benchmarks/tests/outcomes.rs`: `an_unavailable_passive_check_outside_deciding_decides_nothing`,
  `a_graded_attempt_passes_on_end_grade_protected_and_tree`,
  `a_task_without_deciding_checks_is_refused`. Each under 20 ms.
Proof: unit, the three tests; then `heavy cargo run -p smith-bench --
  check` exits 0 on `main`'s tasks, once.
Don't: waive a passive check to make an attempt pass.
Size: 100 lines.
Removes: "every check decides" when `deciding` is empty.

### 01.4 Comparison tasks name no budget
Scope: benchmarks.md 6.1's Budgets bullet (findings T5).
Needs: 01.1 and 01.2 (their tasks carry no `[budget]`).
Files: `benchmarks/src/check.rs` (the kind match at `:191-198`),
  `benchmarks/tests/offline.rs`.
Shape: `check` refuses a `fixture` or `repository` task that names a
  `[budget]`, and one whose `deadline_seconds` is not below 3,600 (the run
  defaults' `budget.seconds`, shell.md 6.4, as a named constant).
Test shape: `a_comparison_task_with_a_budget_is_refused`,
  `a_comparison_deadline_at_the_run_defaults_is_refused`. Under 20 ms
  each.
Proof: unit, the two tests.
Don't: refuse a probe's budget: the budget probes keep theirs.
Size: 40 lines.
Removes: nothing.

### 01.5 The user's `/tmp`, listed
Scope: benchmarks.md 10's "Out of sight" bullet (findings T10).
Needs: nothing.
Files: `benchmarks/src/isolate.rs`, `benchmarks/src/run.rs` (the scope
  gate at `:514`), `benchmarks/src/run/passive.rs`,
  `benchmarks/tests/runner.rs`.
Shape: before and after each attempt, the harness lists the entries of
  `/tmp` the user owns (name, kind, size, modification time); each new or
  changed entry is reported as a write outside the workspace, observed,
  in the result's `nothing-outside` evidence. Entries under the attempt's
  own private directories are not the user's.
Test shape: `a_file_left_in_tmp_by_an_attempt_is_a_write_outside`, with
  the replaying agent of `tests/runner.rs` writing
  `/tmp/smith-bench-test-<pid>` (and removing it after the assertion).
  Under 0.5 s.
Proof: unit/process, that test.
Don't: read or write anything under `/tmp` but the listing and the test's
  own file.
Size: 120 lines.
Removes: nothing.

## Questions to raise when reached

- None known.

## Done when

- `heavy cargo run -p smith-bench -- check` exits 0 on `main`.
- `heavy cargo run -p smith-bench -- validate fixtures/01-backoff
  fixtures/02-line-decoder fixtures/03-planner repository/shell-cwd` exits
  0, once.
