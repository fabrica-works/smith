# Session 12: the parity runs

2026-10-14. Four live steps: a first `parity` run as the baseline, once
the harness, the working tier and compaction have landed; the
`compaction` experiment again, for the user; the calibrations
decisions.md 7.1 names that are left, within the budget; and the closing
`parity` run on the plan's final `main`, whose verdict is the plan's
target. The session writes no production code.

## Read first

- smith `docs/design/benchmarks.md`, sections 3.1, 9, 11.3, 11.5 and 16.
- smith `benchmarks/README.md` (its commands).
- `decisions.md` 1.1, 1.2, 4.5, 7.1, 8.1, 8.2 and 10.2;
  `findings/measure.md` section 5.
- skein `testing-strategy.md`, section 2.9.

## What the design asks

- **The `parity` run** (benchmarks.md 16.1): the three fixtures at eight
  attempts per arm and `shell-cwd` at five, 58 attempts, interleaved in
  one session, no rerun, smith's built arm of the commit against Codex at
  its pinned version, at the working tier. `summarise` writes its
  `parity` section; `smith-bench parity SUMMARY --commit SHA` recomputes
  it, exit 0 at parity (16.4).
- **The `compaction` experiment** over `history/small-window`: the
  threshold's arms (benchmarks.md 11.3), ten attempts per arm, its
  summary put to the user (decisions.md 4.5).
- **Baselines** are cut from a committed summary, one command per agent
  configuration (benchmarks.md 9).

## Choices this plan makes

- 12.2 needs only compaction's fix (07.3): jobs change nothing
  `history` exercises, so it need not wait for decisions.md 8.1's step 5
  (decisions.md, 10.3).
- A `parity` run's own exit is its verdict, not the step's: the baseline
  lands whatever its verdict (README, rule 9's suite rule).
- 12.4 cuts two baselines from its summary, smith's and Codex's, in place
  of the 11 that 03.1 dropped (decisions.md, 10.6).

## Rules that bind it

- **Live runs** (README, rule 9): each suite in one `heavy --gate` call,
  unsplit; a run stopped by the provider runs once more after 60 s.
- **The live budget** (rule 10): print the run's total before each
  command; 12.3 runs only what leaves 64 attempts under 340.
- **The baseline comes first** (rule 11): once 12.1's summary is on
  `main`, the coordinator starts 06, 09.3 and 11.2 as their other Needs
  allow.

## Increments

### 12.1 The baseline `parity` run
Scope: benchmarks.md 16.1 to 16.4 on `main` as it then stands
  (decisions.md 8.1, step 3).
Needs: smith 01's graded tasks [01.1, 01.2, 01.3]; smith 02's pins
  [02.1, 02.2, 02.3]; smith 03's priced spend, attempt rows and time
  record [03.1, 03.2, 03.3]; smith 04's `parity` suite and command [04.3,
  04.4]; smith 05's working tier [05.1]; smith 07's compaction [07.3];
  smith 03's retried failures [03.s1].
Files: `benchmarks/summaries/parity-*.json`.
Shape: no code. `heavy cargo run -p smith-bench -- build HEAD`, then
  `heavy --gate cargo run -p smith-bench -- suite parity --arm <dir>`,
  then `cargo run -q -p smith-bench -- parity <its summary> --commit
  <SHA>`, where `<SHA>` is the full 40-hex `git rev-parse HEAD`, taken
  before the summary is committed (a short SHA or `HEAD` exits 2,
  `parity.rs:243-256`); commit the summary.
  - The two runs on `17d63afd` are spent, and neither summary is
    committed. `parity-17d63afd-20261014T222044Z` was stopped by the
    harness's token rule after 4 attempts, on a retried failure.
    `parity-17d63afd-20261014T222621Z` was stopped by the supervisor's
    hold at its sixth. Together: 10 attempts and $0.18 notional, counted
    in the step's `Live:` trailer.
  - Once 03.s1 is on `main`, the step rebases and runs the three commands
    above once, as a new run, from a new build of `HEAD`. Rule 9 applies
    to that run as to a first one: the harness's rule, which 03.s1
    changes, ended the first command, not the provider.
Test shape: none.
Proof: live, once: the run, 58 attempts, about $12 and 4 hours; its five
  criteria lines, the verdict's exit, and each arm's time-record medians
  go in the commit body and the report. So does the count of smith
  attempts whose priced spend misses only retried failures: `jq
  '[.attempts[] | select(.agent == "smith" and .priced_usd.state ==
  "lower-bound")] | length' <its summary>`.
Don't: rerun an attempt, or change a task, a pin or a margin.
Size: none.
Removes: nothing.

### 12.2 The `compaction` experiment
Scope: benchmarks.md 11.3's `compaction` (decisions.md 4.5).
Needs: smith 07's compaction [07.3].
Files: `benchmarks/suites/compaction.toml` (`repetitions = 10`;
  `reference = "80-percent"`, the shipped threshold; its smith `config` as
  02.1 left it; `max_tokens = 24000000`, from 12000000, since an attempt
  uses about 689K tokens and 20 need about 13.8M; `max_wall_seconds =
  7200`, from 3600), `benchmarks/summaries/compaction-*.json`.
Shape: no code. `heavy cargo run -p smith-bench -- build HEAD` gives
  `<dir>`; `heavy --gate cargo run -p smith-bench -- suite compaction
  --arm <dir>` with the arms `80-percent` and `60-percent`. The suite
  writes no summary (only `parity` and `calibrate` do), so then
  `cargo run -q -p smith-bench -- summarise
  ~/.local/state/smith-bench/runs/<run> compaction small interleaved
  <seed>`, with the run and seed the suite printed; it writes
  `benchmarks/summaries/compaction-<commit8>-<stamp>.json`. Commit it.
Test shape: none.
Proof: live, once: 20 attempts, about $1 and 45 minutes. The report's
  `Evidence:` gives each arm's passes, ends and median task wall, and the
  comparison, in at most 8 lines; the supervisor sends it to the user (a
  push notification) and records it in decisions.md's "For the user".
Don't: change the threshold on `main` (decisions.md 4.5 is the user's).
Size: none.
Removes: nothing.

### 12.3 The calibrations left
Scope: decisions.md 7.1: `tool-output/test-output`, `quote`, `utf-8`
  and `timeout`, and `process-tree` (findings F10).
Needs: every other step but 12.4 landed or dropped, so that no smoke or
  run is left to spend after it.
Files: `benchmarks/tasks/probes/tool-output/task.toml`,
  `benchmarks/tasks/probes/process-tree/task.toml` (their blocks),
  `benchmarks/summaries/`.
Shape: no code. For each probe, in that order, while the run's total
  (README, rule 10) plus 12 stays at most 276 attempts: `heavy --gate
  cargo run -p smith-bench -- calibrate <probe> --arm <dir>`, with no
  control (their controls predate the product face, as 04.6's), each
  block written by `calibrate`.
Test shape: none.
Proof: live: at most 12 attempts per probe, at most 50 in all, $0.30.
Don't: run one past the total's bound. The report names each probe left,
  which the supervisor adds to `later.md`.
Size: none.
Removes: nothing.

### 12.4 The closing `parity` run
Scope: benchmarks.md 16 on the plan's final `main` (decisions.md 1.2,
  8.1 step 7).
Needs: every other step of the plan landed or dropped.
Files: `benchmarks/summaries/parity-*.json`, `benchmarks/baselines/`.
Shape: no code. Build `HEAD`, run `suite parity` as 12.1, commit the
  summary, then cut the two baselines, each arm's configuration digest
  read from the summary's arm identity:

  ```
  cargo run -q -p smith-bench -- baseline <summary> parity-smith --group smith <smith arm's configuration sha256> gpt-6.1-sol high
  cargo run -q -p smith-bench -- baseline <summary> parity-codex --group codex <Codex arm's configuration sha256> gpt-6.1-sol high
  ```

  and run the README's "Done when" item 2 once.
Test shape: none.
Proof: live, once: 58 attempts, about $12 and 4 hours; item 2's command
  exits 0.
Don't: loosen a margin, rerun the suite to pass, or change a pin. A
  verdict of 1 or 2 is a question for the designer: a truly equal smith
  passes the whole verdict about 0.69 of the time (decisions.md 1.1).
Size: none.
Removes: nothing.

## Done when

- The README's "Done when" items 2 and 4 hold.
