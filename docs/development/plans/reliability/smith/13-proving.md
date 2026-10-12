# Session 13: proving what landed

Provisional, 2026-10-12. The first run proved the probes of its proving
steps 1 to 3 (`smoke-tools`, `first-request`, `failures`) and stopped
inside step 4. The probes of steps 4 to 9 are on `main` since `c077df98`,
built offline and never calibrated, and the increments they guard landed
"merged, unproven". This session runs one live batch per probe variant,
commits each batch's summary, and names in each commit body the merged
increments its calibration proves, or that they rest on their tests. It
changes the harness only where a check is stricter than its design.

## Read first

- smith `docs/design/benchmarks.md`, sections 3.1, 4.2, 5.2, 5.4, 9,
  11.1, 11.2 and 11.5.
- smith `docs/development/workflow.md`, section 5.
- smith `benchmarks/README.md`, "Smoke probes and deciding observations".
- skein `docs/foundation/development.md`, sections 5.5, 8.7, 8.8 and 10.4.
- `decisions.md`, 12.4, 12.5, 12.14 and 12.15.
- The before of each step, in `findings/`: `limits.md` S4, S5, S6 and
  S9; `context.md` C1; `delegation.md` D2, D3, D4 and D6; `tools.md` T2,
  T3, T6, T7, T8 and T9.

## What the design asks

- **Calibration** (benchmarks.md 3.1): "A probe is calibrated before it
  gates anything [...]. `probes` lists every probe and variant, calibrated
  or not (11.5); a committed summary of its calibration says which.
  Calibrated means ten passes within at most twelve attempts on the
  binary it guards, the other attempts failed only by the model, never by
  smith [...], and a failure on the binary before the fix or, for a
  behaviour already in place, on a control arm."
- **The before** (4.2): "When a parent fails the probe for a reason
  unrelated to the change [...], the before is the closest evidence that
  exists: the change's deterministic tests, which fail on the parent;
  recorded measurements, such as the spike's; a same-code run with a
  setting that turns the feature off, where one exists. The commit says
  which. No switch is added only for this."
- **What stays** (4.2): "The probe stays in its suites with its
  `guards`". The `probes` suite (11.5) holds "every probe and variant":
  smith on `codex`, and Codex on outcome halves.
- **What is committed** (5.4): "`summaries/`, one per run of the probes,
  compare and repository suites".
- **The variants' own rules** (11.1):
  - `large-write/oversized`: "An attempt that makes no call past the
    bound, and otherwise makes the file in pieces, accepted, is 'not
    exercised (bound honoured)': neither a pass nor a fail [...]. The
    variant is proven by three exercised passes within forty-eight
    attempts".
  - `cache-reuse`: "median of three at least 0.85, and full misses
    (under half) at most one per ten requests over the three, until
    calibration sets them; the three repetitions are one attempt".
  - `budget/visibility`: "The told turns and time equal the `budget`
    record of the step that wrote them, time in whole seconds rounded
    down; a mismatch there is smith's."
  - `slow-completion`: "the call lasting over 60 s [...]. An attempt
    under 60 s is 'not exercised': neither a pass nor a fail, counted
    like a model deviation".

## Choices this plan makes

- **One batch per variant, at most 12 attempts** (the user, 2026-10-12).
  A variant calibrates at 10 passes of 12 (benchmarks.md 3.1). `large-write/oversized`
  calibrates at a third exercised pass among its 24 recorded attempts
  and at most 12 more (decisions.md, 12.5); short of it, its two
  recorded exercised passes stand.
- **The failing half is the closest evidence of 4.2, for each step.**
  The increments these probes guard landed before `e548d632`, smith's
  first commit that completes a live call, so no parent runs live. smith
  has no setting that turns affinity or any other guarded mechanism off,
  and none is added. Each commit names its before: the findings listed
  under "Read first" and the guarded increments' tests; it runs none.
- **Steps 1 to 3 stand** (decisions.md, 12.4): nothing is calibrated
  again because `main` moved.
- **`large-write`'s normal half keeps its 10 passes in 11.** Version 9
  keeps version 7's prompt, budget and checks for it; only the seed
  changed, which that prompt tells the model not to read.
- **The record is the summary and the `Calibrated:` line** (decisions.md,
  12.20): `suites/probes.toml` lists each variant already, and no step
  adds a variant to a suite or removes one. A variant short of its count
  is left out of `Calibrated:`, its counts per class are in the commit
  body, the increments it guards rest on their tests, and the report
  names a task version for `later.md`.
- **Codex runs where the probe compares agents:** `large-write`'s
  outcome half once (13.1), one `cache-reuse` triplet (13.2), and each
  `tool-output` and `paths` variant once, for tool round trips (13.5).
- **`budget/visibility` is its own increment, 13.7,** so that the other
  four `budget` variants do not wait for smith 00's told check [00.17].
- **Order:** the first run's order, 13.1 to 13.6, then 13.7.

## Rules that bind it

1. **Judging attempts.** Each attempt that is not a pass falls in one
   class, named in the commit body with its count:
   - **A smith defect:** evidence on smith's side: a tool error, a
     refused or failed call, loss, a codec or limit failure, a wrong
     terminal, a failed passive check (benchmarks.md 11.2). It stops the
     step on a question (development.md, section 8.7). The supervisor
     adds the fix step, `smith-NN.s<k>` (development.md, section 10.4).
     The fix's Proof is this variant's batch, run once on the fix; this
     step then lands with that batch's summary, running no batch itself.
   - **A model deviation:** the model skipped a required step, made an
     extra or repeated call, or took the steps in another order; the run
     ended normally, with no evidence on smith's side. It is not a pass.
   - **A task the tier cannot do as written:** the run ended as smith's
     contracts say (a budget, a deadline, a limit), with no smith defect,
     because the task asks for what the model cannot do reliably. It is
     not a pass.
   - **Not exercised** (`large-write/oversized`, `slow-completion`): as
     benchmarks.md 11.1 says; not a pass.
   - **A check stricter than its design:** a deciding check implemented
     more strictly than benchmarks.md 11.1 states it. It is corrected
     within the increment, with a test that fails on `main`, and the
     batch's attempts are judged again; both verdicts go in the commit
     body. The batch does not run again.
   - **A setup refusal** (`Refused(setup)`, before inference): not an
     attempt. A lapsed login or the provider's capacity is escalated as
     README, rule 1 says; another is fixed within the step's Files. The
     missing attempts then run in one more call, once.

   Model deviations, tasks the tier cannot do and attempts not exercised
   never stop a step: short of the count, it lands as not calibrated.
2. **Batches.** One batch per variant, at most 12 attempts per `heavy
   --gate` call, so that other sessions' gates run between calls. Tier
   `small`, `gpt-6-luna` at low effort unless the task names an effort.
   Never change the model, the effort, the budgets or the deciding checks
   to pass, and write no new task version in this plan. Each step builds
   its arm once, `heavy cargo run -p smith-bench -- build HEAD`, and gives
   each command below, Codex's included, `--arm <dir>`, the directory it
   printed, so that nothing builds in the gate lane.
3. **Evidence is the summary and the commit body.** One summary per
   `calibrate` batch, by 13.1's `summarise` command, committed under
   `benchmarks/summaries/`; a Codex attempt's line in the body; the Proof
   line, the counts per class, one line `Calibrated: <variants>` and one
   line `Proven: <increment ids>` (`none` on either when nothing calibrated).
   Nothing else is wanted: no manifests, no observers, no repeated
   batches, and never a script that prepares requests by hand
   (smith-bench's one-command probe runs do it).

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| smith-00.23, step 1: `smoke-tools`, `smoke-tools/child` | `7ebcd21b` | proven |
| smith-00.25, step 2: `first-request` | `32ef87dc` | proven |
| smith-00.14, step 3: `failures`, four variants | `e3e79e9b` | proven |
| smith-00.26: `large-write/oversized` version 9 and its offline controls | `c769a618` | merged, unproven: smith-13 |
| smith-00.15: `tool-output` and `process-tree` | `c077df98` | merged, unproven: smith-13 |
| smith-00.16: `cache-reuse`, `parallel-read`, `parallel-children` | `c077df98` | partly landed: `history` left (session 00) |
| smith-00.17: `budget`, `slow-completion` | `c077df98` | partly landed: the told check and `credentials` left (session 00) |

## Increments

### 13.1 `large-write` and `large-write/oversized`
Scope: the first run's step 4, finished (benchmarks.md 11.1, `large-write`;
decisions.md, 12.5):
  - `oversized-write` judges an attempt with no call past the call's
    bound "not exercised (bound honoured)", including a `write` refused
    by the tool's payload within that bound. On `main` it fails such an
    attempt: a check stricter than its design (rule 1);
  - `tasks/probes/large-write/task.toml` becomes the version 9 document,
    `v9/oversized-v9.toml`, with its seed. `oversized-v8.toml`,
    `v9/oversized-v9-medium.toml` and the version 7 seed move under
    `tests/fixtures/large-write/`, beside versions 4 to 7; `v9/` goes;
  - one batch of `large-write/oversized`, after its 24 recorded attempts;
    Codex's outcome half of `large-write`, once.
Needs: nothing beyond the session's start: smith 00's one command per
  probe run and probes suite [00.27, 00.29].
Files: smith `benchmarks/`: `src/agent/smith/events/file_pieces.rs`
  (`oversized_check`, line 30; its callers `events.rs:287` and
  `observe.rs:182`); `tasks/probes/large-write/`; `tests/fixtures/large-write/`;
  `tests/large_write_probe.rs` (the tests at lines 15, 67, 125, 651 and
  757 read the moved documents or the changed verdict); `tests/probes.rs:88`;
  `summaries/`.
Shape: in `file_pieces::oversized_check`, protocol mode, once the stream
  has ended without loss (`complete`, line 58): no refusal pushes a
  `NotExercised: ...` evidence line and
  returns `Measure::unavailable("not exercised: ...")`, as `slow.rs:120`
  does; one refusal past the bound passes; more fail, as now. `judge`
  (`judgment.rs:31`) then makes an attempt whose other deciding checks
  pass Incomplete, and a failed deciding check still fails it. Interface
  sketch 1.
Test shape: `benchmarks/tests/large_write_probe.rs`; reuse
  `common::events::balanced` (`tests/common/events.rs:74`), `piece`
  (`large_write_probe.rs:462`) and `observe` (`large_write_probe.rs:605`).
  New: `oversized_write_without_a_call_past_the_bound_is_not_exercised`
  (below; it fails on `main`) and
  `payload_refusal_within_the_call_bound_is_not_exercised`. Changed: the
  `Observed { value: false }` expectations for no refusal at lines 42 and
  193 become "not exercised"; `version_nine_...` reads `task.toml`;
  `corrected_refusal_checks_...` and `oversized_refusal_threshold_...`
  read the fixtures. Each under 20 ms.
  ```rust
  #[test]
  fn oversized_write_without_a_call_past_the_bound_is_not_exercised() {
      let mut records = common::events::balanced();
      let input = serde_json::json!({"path":"payload.txt","content":"Line 0001"});
      piece(&mut records, "write", "first", &input, Verdict::Written);
      let workspace = std::env::temp_dir();
      let verdict = observe(&records).oversized_write(std::path::Path::new("payload.txt"), 2048, Some(13848), &workspace);
      assert!(matches!(&verdict.passed, Measure::Unavailable { reason } if reason.starts_with("not exercised")));
      assert!(verdict.evidence.iter().any(|line| line.starts_with("NotExercised")));
  }
  ```
Proof: live, once, on the increment's branch, after its tests pass:
  - `heavy --gate cargo run -p smith-bench -- calibrate
    probes/large-write/oversized --passes 1 --arm <dir>`: at most 12
    attempts, $0.50 and 20 minutes; it stops at its first exercised pass,
    the third of the variant, which calibrates it;
  - `heavy --gate cargo run -p smith-bench -- probe probes/large-write
    --agent codex --attempts 1 --arm <dir>`: one attempt, $0.25 and 5
    minutes; a pass is its outcome checks passing. The normal half does not run on smith;
  - before: findings S4, and the tests of smith 04.10, 09.13 and skein
    04.3;
  - the summary: `heavy cargo run -p smith-bench -- summarise <the
    batch's runs directory> calibrate small single <seed>`.
  Together: 13 attempts, $0.75, 25 minutes. Proven on calibration: smith
  00.26, 04.3, 04.10, 09.13; skein 04.3. With no third exercised pass,
  the two recorded ones prove smith 04.10, 09.13 and skein 04.3, and the
  commit names them so (decisions.md, 12.15).
Don't: run `large-write`'s normal half on smith again; change
  `tool_payload`, the effort or the deciding checks; touch smith's
  crates.
Size: at most 40 lines of production code; the task documents move
  unchanged.
Removes: `oversized-v8.toml` and `v9/` from `tasks/probes/large-write/`,
  kept as fixtures.

### 13.2 `cache-reuse`
Scope: the first run's step 5 (benchmarks.md 11.1, `cache-reuse`; 11.5):
  one batch each of `cache-reuse`, `cache-reuse/children` and
  `cache-reuse/crossing`, an attempt being a triplet (the task's
  `repetitions = 3`); one Codex triplet of `cache-reuse`, for fresh
  input, cache reads and full misses beside smith's. `idle-gap` is not
  built here (`later.md`).
Needs: as 13.1; smith 00's scripted person and `cache-reuse/crossing`
  [00.28].
Files: smith `benchmarks/summaries/`. Only for a
  check corrected under rule 1: `src/cache.rs`,
  `src/agent/smith/events/cache.rs`, `src/agent/codex/rollouts/cache.rs`.
Shape: no new code: the task, the pin and the calibration suite are on
  `main` (`tasks/probes/cache-reuse/task.toml`,
  `agents/smith/cache-reuse.pin.toml`, `suites/probes.toml`).
Test shape: none new: `tests/cache_probes.rs` stands. A check corrected
  under rule 1 adds one test there that fails on `main`, under 20 ms.
Proof: live, once each: `heavy --gate cargo run -p smith-bench --
  calibrate PROBE --arm <dir>` for PROBE `probes/cache-reuse`,
  `probes/cache-reuse/children` and `probes/cache-reuse/crossing`, then
  `probe probes/cache-reuse --agent codex --attempts 1 --arm <dir>`. A triplet passes
  when its deciding checks pass, with the median reuse at least 0.85 and
  at most one full miss per ten requests over the three. Bounds: 12
  triplets, $1.50 and 40 minutes; 12 triplets of `children`, $3.00 and
  60 minutes; 12 triplets of `crossing`, at medium effort, $3.00 and 60
  minutes; one Codex triplet, $0.50 and 5 minutes. Together: 37 attempts
  (111 runs), $8, 165 minutes. No control arm (decisions.md, 12.14):
  before, findings C1, and smith 08.1's wire referee tests
  (`tests/protocol-llm/tests/referee.rs`). Proven on calibration of
  `cache-reuse` and `children`: smith 04.optional-usage, 05.7, 08.1,
  08.3, 08.4, 08.5; skein 04.7, 04.9, 04.11.
Don't: add an affinity switch or a control arm; change the thresholds;
  build `idle-gap`.
Size: none; at most 40 lines for a corrected check.
Removes: nothing.

### 13.3 `budget`: starvation, wrap-up, child-quota, spend
Scope: the first run's step 6 without `visibility` (13.7) (benchmarks.md
  11.1, `budget`): one batch per variant. `spend` keeps the task's
  provisional ceiling, $1.00; its summary records the measured spend.
Needs: as 13.1.
Files: smith `benchmarks/summaries/`. Only for a
  check corrected under rule 1: `src/agent/smith/events/budget.rs`,
  `final_turn.rs`, `spend.rs`.
Shape: no new code: `tasks/probes/budget/task.toml`,
  `agents/smith/budget.pin.toml`, `suites/probes.toml` are on `main`.
Test shape: none new: `tests/probes.rs` and `tests/events.rs` stand; a
  corrected check adds one test that fails on `main`, under 20 ms.
Proof: live, once per variant: `heavy --gate cargo run -p smith-bench --
  calibrate probes/budget/<variant> --arm <dir>`, for `starvation`,
  `wrap-up`, `child-quota` and `spend`. 48 attempts, $3, 60 minutes.
  Before: findings D2, D3, D4 and D6, and the guarded increments' tests. Proven, by variant:
  `starvation`: smith 07.2, 07.6, 07.10; `wrap-up`: smith 07.11, skein
  04.2; `child-quota`: smith 07.7, 07.8, 07.11; `spend`: smith 07.1,
  07.12.
Don't: move the spend ceiling; change a variant's budget.
Size: none; at most 40 lines for a corrected check.
Removes: nothing.

### 13.4 `parallel-read` and `parallel-children`
Scope: the first run's step 7 (benchmarks.md 11.1, `parallel-read` and
  `parallel-children`): one batch each of `parallel-read`,
  `parallel-children` and `parallel-children/reviews`. Overlap is the task's
  `children-overlap` check.
Needs: as 13.1.
Files: smith `benchmarks/summaries/`. Only for a
  check corrected under rule 1: `src/agent/smith/events/checks.rs`.
Shape: no new code: the tasks, `agents/smith/parallel.pin.toml` and
  `suites/probes.toml` are on `main`.
Test shape: none new: `tests/parallel_probes.rs` stands; a corrected
  check adds one test that fails on `main`, under 20 ms.
Proof: live, once per variant: `heavy --gate cargo run -p smith-bench --
  calibrate PROBE --arm <dir>` for PROBE `probes/parallel-read`,
  `probes/parallel-children` and `probes/parallel-children/reviews`. 36
  attempts, $2.50, 45 minutes. Before: findings S9, and the
  guarded increments' tests. Proven, by probe: `parallel-read`: skein
  05.6, 05.8; `parallel-children`, both variants: smith 04.12, 07.5;
  skein 03.3, 03.5.
Don't: change `conversations` or `calls_per_response` in the tasks.
Size: none; at most 40 lines for a corrected check.
Removes: nothing.

### 13.5 `tool-output`, then `paths`, then `process-tree`
Scope: the first run's step 8 (benchmarks.md 11.1, `tool-output`,
  `paths`, `process-tree`): one batch each of `tool-output/test-output`,
  `quote`, `utf-8` and `timeout`, `paths`, `paths/single`, and
  `process-tree`, in that order; Codex once on each `tool-output` and
  `paths` variant, its completions before finish beside smith's.
Needs: as 13.1.
Files: smith `benchmarks/summaries/`. Only for a
  check corrected under rule 1: `src/agent/smith/events/tool_path.rs`,
  `checks.rs`.
Shape: no new code: the tasks and `suites/probes.toml` are on `main`.
Test shape: none new: `tests/tool_output_probes.rs`, `path_probes.rs`
  and `process_tree_probe.rs` stand; a corrected check adds one test that
  fails on `main`, under 20 ms.
Proof: live, once per variant: `heavy --gate cargo run -p smith-bench --
  calibrate PROBE --arm <dir>` for PROBE `probes/tool-output/<variant>`
  (`test-output`, `quote`, `utf-8`, `timeout`), `probes/paths`,
  `probes/paths/single` and `probes/process-tree`; then `probe PROBE
  --agent codex --attempts 1 --arm <dir>` for the four `tool-output` variants and the
  two `paths` ones. 90 attempts, $3, 60 minutes.
  Before: findings T2, T3, T6, T7, T8 and T9, and the guarded increments'
  tests. Proven, by probe: `tool-output` (`test-output`, `timeout`): smith
  04.1, 09.12; `quote`: 09.10; `utf-8`: 09.3, 09.8; `paths`, both
  variants: 09.4, 09.5, 09.6, 09.7, 09.9; `process-tree`: smith 09.11,
  skein 01.1, 01.2, 01.tree-termination, 01.tree-subreaper-restoration;
  smith 00.15 with `tool-output` and `process-tree`.
Don't: change a task's timeouts or output budgets.
Size: none; at most 40 lines for a corrected check.
Removes: nothing.

### 13.6 `slow-completion`
Scope: the first run's step 9 (benchmarks.md 11.1, `slow-completion`):
  one batch at the task's high effort. The summary carries, per provider
  and model, `first_byte_ms`, `largest_gap_ms`, the call's length and
  the largest reasoning item, for shell.md's `head`, `idle` and
  `reasoning_item`.
Needs: as 13.1.
Files: smith `benchmarks/summaries/`. Only for a
  check corrected under rule 1: `src/agent/smith/events/slow.rs`.
Shape: no new code: the task and `suites/probes.toml` are on `main`.
Test shape: none new: `tests/slow_completion_probe.rs` stands; a
  corrected check adds one test that fails on `main`, under 20 ms.
Proof: live, once: `heavy --gate cargo run -p smith-bench -- calibrate
  probes/slow-completion --arm <dir>`. 12 attempts, $4, 120 minutes; the
  task's deadline is 900 s per attempt. A pass: the answer accepted, no phase
  timed out while events arrived, no reasoning-size failure, a call over
  60 s. Before: findings S5 and S6, and the guarded increments' tests.
  Proven on calibration: smith 04.4, 04.11; skein 03.1, 04.4, 04.12.
Don't: raise the tier, or simulate a slow provider (a fake slow source is
  the worlds', benchmarks.md 11.1).
Size: none; at most 40 lines for a corrected check.
Removes: nothing.

### 13.7 `budget/visibility`
Scope: one batch of `budget/visibility` (benchmarks.md 11.1, `budget`,
  `visibility`; domain/run.md 9.2), judged by smith 00's told check
  [00.17].
Needs: as 13.1; smith 00's told check [00.17].
Files: smith `benchmarks/summaries/`. Only for a
  check corrected under rule 1: `src/agent/smith/events/budget_report.rs`.
Shape: no new code: `budget-told` is smith 00.17's.
Test shape: none new: `tests/budget_visibility.rs` stands; a corrected
  check adds one test that fails on `main`, under 20 ms.
Proof: live, once: `heavy --gate cargo run -p smith-bench -- calibrate
  probes/budget/visibility --arm <dir>`. 12 attempts, $0.75, 15 minutes.
  Before: findings D4, and smith 07.9's tests. Proven on calibration:
  smith 07.9.
Don't: judge the model's report against the record (that is
  `budget-report`'s).
Size: none; at most 40 lines for a corrected check.
Removes: nothing.

## Interface sketches

`benchmarks/src/agent/smith/events/file_pieces.rs` (13.1). The verdict
of `oversized-write` in protocol mode, once the stream has ended without
loss, from the refusals `protocol_refusals` found. 13.1 owns it.

```rust
/// No oversize block is "not exercised (bound honoured)" (benchmarks.md 11.1).
fn protocol_verdict(refused: usize, bound: u64, evidence: &mut Vec<String>) -> Measure<bool> {
    if refused == 0 {
        evidence.push(format!("NotExercised: no call past the stated call_input={bound}; bound honoured"));
        return Measure::unavailable("not exercised: no call past the call's bound");
    }
    Measure::Observed { value: refused == 1 }
}
```

## Questions to raise when reached

- **`cache-reuse`'s thresholds** stand "until calibration sets them"
  (benchmarks.md 11.1). If the measured medians sit far from 0.85, does a
  later task version move them? Proposal: record the medians; the version
  goes to `later.md`.
- **`budget/spend`'s ceiling** is "about twice a calibrated spend"; the
  task's $1.00 is provisional. Proposal: record the median spend; a
  version at twice it goes to `later.md`.
- **`slow-completion` not exercised:** 11.1 says more than two in twelve
  "strengthen the prompt, or raise the effort", and the task is already
  at high effort. Proposal: record the counts; a version goes to
  `later.md`.
- **A payload refusal in `oversized`:** the first run failed a `write`
  refused by the tool's payload within the call's bound; 11.1's words make
  it "not exercised". 13.1 follows 11.1; the supervisor rules on it if
  an attempt shows one.

## Done when

- Seven commits carry `Step: smith-13.<k>`: `git -C ~/src/rust/smith log
  origin/main --grep '^Step: smith-13\.' --format=%h | wc -l` prints 7
  (today 0).
- Each has one `Calibrated:` and one `Proven:` line: `git -C ~/src/rust/smith
  log origin/main --grep '^Step: smith-13\.' --format=%B | rg -c
  '^(Calibrated|Proven): '` prints 14 (today nothing).
- The batches' summaries are committed: `heavy cargo run -p smith-bench
  -- check`, run in a smith checkout at `origin/main`, prints at least
  20 summaries (today "checked 12 tasks, 11 suites, 8 configuration
  pins, 0 summaries, 0 baselines").
- `large-write` has one task document: `git -C ~/src/rust/smith ls-tree
  --name-only origin/main benchmarks/tasks/probes/large-write/` prints
  its `seed` and `task.toml` only (today also `oversized-v8.toml` and
  `v9`).
- The corrected check is in place: `heavy cargo nextest run -p
  smith-bench -E 'test(oversized_write_without_a_call_past_the_bound_is_not_exercised)'`
  passes 1 test.
- Each step committed its summaries: `git -C ~/src/rust/smith log
  origin/main --grep '^Step: smith-13\.' --name-only --format= | rg -c
  '^benchmarks/summaries/'` prints at least 20.
