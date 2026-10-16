# Session 06: the baseline

2026-10-16. The plan's aim is a measured verdict (development.md 5.1):
smith on par with Codex at the plan's final `main`, every criterion of
benchmarks.md 16.2, RT included. Its baseline is the `parity` run that
measures where it starts, once the harness measures as the design asks
(benchmarks.md 16.6). Each change after it projects its ratios onto this
run's statistics. This session runs it, then runs the `parity-change`
pipeline whole at one attempt per task against it, before any full
`parity-change` run (development.md 5.5). It lands a committed summary
and changes no code.

## Read first

- smith `docs/design/benchmarks.md`, sections 9, 16.1 to 16.6.
- `decisions.md`, sections 1.7, 1.8, 3, 10 and 12.1; `findings/rt-power.md`.
- The README's rules 9 to 13.

## What the design asks

- **A run measures its commit as it is;** a summary is committed
  whatever its verdict, 0 or 1; a verdict of 2 measured nothing whole,
  and its summary is not committed (16.5).
- **The baseline is the plan's first `parity` run,** its committed
  summary, never `baselines/`'s files (16.6).
- **A live pipeline runs whole, small, before it runs in full**
  (development.md 5.5): the `parity-change` suite, its summary, its
  report and its stop, at one attempt per task.

## Choices this plan makes

- The baseline runs `main` once smith 01's measures, smith 02's report
  and stop, and smith 03's and 04's line and guard have landed: what
  changes what it measures, or what the smith arms of 07.1 to 10.1
  build on (decisions.md, 12.1).
- A miss at the baseline is expected: RT failed the parity plan's last
  run, and the plan's four changes are its answer. The supervisor still
  sends the baseline's failed criteria to the designer at once
  (development.md 5.1, 10.4); 07 starts on 06.1's landing, unless the
  designer's answer says otherwise.
- The small `parity-change` run builds both arms from the same commit,
  `HEAD`: it checks the pipeline, and its ratios are not read.

## Rules that bind it

- **Live runs** (README, rule 9): each suite in one `heavy --gate` call,
  unsplit; no attempt rerun; a run stopped by the provider runs once
  more, from the start.
- **The live record** (README, rule 10): the step's `Live:` trailer.
- **Summaries named in trailers** (README, rule 13).

## Increments

### 06.1 The baseline `parity` run, and the change pipeline at one attempt per task
Scope: benchmarks.md 16.1 to 16.4 on `main` as it then stands; 16.6's
  baseline; development.md 5.5's small run of the `parity-change`
  pipeline.
Needs: smith 01's pricing, rows and ten attempts [01.1, 01.2, 01.3];
  smith 02's stop [02.3]; smith 03's line and history bytes [03.1,
  03.2]; smith 04's guard [04.1].
Files: `benchmarks/summaries/parity-*.json`.
Shape: no code. On a worktree at `origin/main`:
  1. `heavy cargo run -p smith-bench -- build HEAD` gives `<dir>`;
  2. `heavy --gate cargo run -p smith-bench -- suite parity --arm <dir>`;
  3. `heavy cargo run -q -p smith-bench -- parity <its summary> --commit
     <SHA>`, where `<SHA>` is the full 40-hex `git rev-parse HEAD`, taken
     before the summary is committed. On an exit of 2, the summary is not
     committed, and the step stops on a question;
  4. on 0 or 1, commit the summary, then `heavy --gate cargo run -p
     smith-bench -- suite parity-change --arm <dir> --parent-arm <dir>
     --attempts 1 --baseline-run benchmarks/summaries/<its file>`: 8
     attempts. It passes when it exits 2 and `heavy cargo run -q -p
     smith-bench -- change <its small summary>` prints a line for each
     of FT, FC, RT, RC and O and exits 0; anything else stops the step
     on a question. Its summary is not committed;
  5. land the baseline's summary with `Baseline-Summary:
     benchmarks/summaries/<file>` among its trailers (README, rule 13).
Test shape: none: no code; the runs are the proof.
Proof: live, once each: the baseline, 68 attempts, about $17 and 6
  hours, and the small run, 8 attempts, about 40 minutes. In the commit
  body and the report: the five criteria lines and the verdict's exit;
  `jq -c '.groups[] | {task, arm, passes, attempts}' <its summary>`;
  README rule 13's counts beside, for `shell-cwd`; the small run's
  `change` lines; and the count of smith attempts in which the guard
  refused a request, `rg -l 'past the line'
  ~/.local/state/smith-bench/runs/<its run>/ | wc -l` (expected 0; any
  other count is reported, and changes nothing).
Don't: rerun an attempt, or change a task, a pin or a margin.
Size: none.
Removes: nothing.

## Questions to raise when reached

- A baseline whose verdict is 0: the plan's aim already holds before
  its changes; the supervisor asks the designer whether 07 to 10 still
  run (development.md 10.4).

## Done when

- `cd ~/src/rust/smith && git log origin/main --grep='^Plan: wall-clock$' --format='%(trailers:key=Baseline-Summary,valueonly)' | grep -c .` prints 1.
