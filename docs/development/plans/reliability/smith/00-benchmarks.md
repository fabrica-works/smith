# Session 00: the benchmark harness

Provisional, 2026-10-12. `smith-bench`, the harness under `benchmarks/`,
runs smith and Codex against real providers; the first run built most of
it, and twelve of benchmarks.md's fifteen probes. What is left: one
command per live run, which each landing's smoke and session 13's
calibrations need, since today each run starts from a request written by
hand; fixtures 01 and 02; the scripted person's later lines; the `probes`
suite; the probes that wait for behaviour this plan lands (`follow-up`,
`history`, `credentials`, and `budget/visibility`'s told check), and
`cache-reuse/crossing`; and, once session 13 has calibrated what landed,
the first baseline on events, which deletes the legacy face.

## Read first

- benchmarks.md, sections 3, 4.2, 5, 6, 9, 10, 11 and 14; testing.md,
  section 2.4.
- protocol/events.md, sections 3.4, 3.7 and 3.8; domain/run.md, sections
  6 and 9.2; domain/host.md, sections 7 and 8.
- decisions.md, 1.4, 9.1 to 9.8, 12.3 and 12.4.
- findings/benchmarks.md, sections 3, 6 and 7 (B).
- `benchmarks/README.md`; smith's `docs/development/workflow.md`, section 5.

## What the design asks

Only what is left on `main`:

- **Commands** (benchmarks.md, 5.1): "Commands run a suite or a task with
  its arms, calibrate a probe, record a fix's before and after, … Their
  spelling is the README's." On `main`, `run`, `fix` and `calibrate` take a
  directory of requests prepared outside the harness (00.27).
- **Fixtures** (3.1, 11.4): `01-backoff` and `02-line-decoder`, each with
  "a hidden grader, run after the agent has exited". "Every grader fails
  on the seed and passes on the task's reference solution." The `compare`
  suite runs them (11.5) (00.13).
- **The scripted person** (5.2): "`[[person]]` … the scripted person's
  lines, each sent at the start or when an event the task names is
  observed …, and an end of input" (00.28).
- **Probes** (11.1) not on `main`: `follow-up`, with `eof` and `chat`
  (00.14); `history`, with `small-window` (00.16); `cache-reuse/crossing`
  (00.28); `credentials` (00.30); and `budget/visibility`'s check that "the
  told turns and time equal the `budget` record of the step that wrote
  them" (00.17).
- **The `probes` suite** (11.5): "every probe and variant", Codex "on
  outcome halves" (00.29).
- **Baselines** (9) and **the legacy face** (6.2): "The module is deleted
  in the change that takes the first baseline on events. Nothing else
  depends on it." (00.21)

## Choices this plan makes

- **One command per live run** (00.27): `probe`, `calibrate`, `fix` and
  `suite`, as the README spells them. Each landing's smoke (workflow.md,
  section 5) and session 13's calibrations use them.
- **A calibration batch** follows session 13's rule: at most 12 attempts
  per variant (a `cache-reuse` triplet is one attempt), calibrated at 10
  passes, or the variant's own count. It is recorded by its summary under
  `summaries/` (`summarise`, as `benchmarks/README.md` spells it) and the
  commit's line `Calibrated: <variants>`, or `none` (decisions.md, 12.20).
  Short of it for a reason other than a smith defect (model deviations, a
  task the tier cannot do, attempts not exercised), the increment lands
  with its counts in the commit body, and its report names the next task
  version for `later.md`. A smith defect stops the increment on a
  question. No task gets a new version in this plan.
- **Probes of work this plan lands** (`follow-up`, `history`,
  `credentials`) are written once that work has landed, and calibrated by
  the same increment, with the landed increment's parent as the control
  arm where it can take the task's settings. `cache-reuse/crossing` guards
  work that landed in the first run: session 13's cache step calibrates it.
- **`[[person]]`** reuses the runner's script types (`agent::Cue`,
  `Trigger`, `Close`). The task's `prompt` is the first line. Codex takes
  no later input: it refuses such a task as `Refused(setup)`.
- **An outcome half** (11.1): for an agent other than smith, a task's
  deciding names keep its outcome checks, and `cache-reuse`, which Codex's
  rollouts answer.
- **Graders are Rust crates** beside the seed (`grader/`), whose tests
  take the candidate as a path dependency; `[[grade]]` runs `cargo test
  --offline` and `cargo fmt --check` in the graded copy.
- **The `compare` pin adds no instructions** (6.1: "smith's pinned
  settings add nothing"): smith reads the seed's `AGENTS.md` through its
  `guide` convention, as Codex reads the file.
- **`cache-reuse/idle-gap` waits for the Anthropic pass** (`later.md`): it
  measures a cache's lifetime across a gap, which only Anthropic lets a
  request choose (11.3's `cache-lifetime`), and a person line has no delay.
- **The result format moves to version 2** at the first landing that
  changes it (README, rule 5): 00.21, when `SmithFace::Legacy` goes, or
  session 12's delegation part [12.10], if it lands first.
- **Session 10 moves the harness with the binary:** the events face to
  `smith exec --json` and `smith check --json`, a `borrow` account, and
  `failures/one-turn` into the smoke suite go with session 10's increments.

## Rules that bind it

- **Benchmarks are never the merge gate** (benchmarks.md, 1): no test in
  the suite makes a live call or reads the user's `~/.codex`, `~/.claude`
  or smith's XDG directories (code-organization.md, 7). Offline tests use
  synthetic logins (`unissued_auth`, `tests/smith_adapter.rs:91`).
- **Credentials are borrowed, access-only:** "never a copy of a refresh
  token" (benchmarks.md, 1 and 10); attempts write only under
  `$XDG_STATE_HOME/smith-bench/`, through `guard::Guard`.
- **A live run is a Proof's, run once** through `heavy --gate`, within the
  Proof line's bound. Each failed attempt is classified in the commit body
  (a smith defect, a model deviation, a task the tier cannot do).

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 00.1 | b5bfb165 | deterministic |
| 00.2 | cc42e3e2 | deterministic |
| 00.3 | 811d342a | deterministic |
| 00.4 | 59b9f982 | deterministic |
| 00.5 | c446b560 | deterministic |
| 00.6 | 98163c75 | deterministic |
| 00.7 | 4dc102f8 | deterministic |
| 00.8 | 560965fd | deterministic |
| 00.9 | 2d82c77b | deterministic |
| 00.10 | 0b887422 | proven |
| 00.11 | 46f6f1bc | proven |
| 00.12 | f558d7b5 | proven |
| 00.14 | e3e79e9b, 9f0394c2 | partly landed: `follow-up` left; `failures` proven; `paths` merged, unproven: smith-13 |
| 00.15 | 4970d5e8, 22c7cf34 | merged, unproven: smith-13 |
| 00.16 | 2f630ca3, 81640441 | partly landed: `history`, `crossing` left; cache and parallel merged, unproven: smith-13 |
| 00.17 | 97468249, f7217370 | partly landed: told check, `credentials` left; budget and slow merged, unproven: smith-13 |
| 00.20 | 5097c835 | proven |
| 00.22 | a0147bb7 | proven |
| 00.23 | 7ebcd21b | proven |
| 00.24 | eb311285 | proven |
| 00.25 | 32ef87dc | proven |
| 00.26 | c769a618 | merged, unproven: smith-13 |

## Increments

00.27, 00.13, 00.28 and 00.29 need nothing outside the session. 00.17,
00.14, 00.16 and 00.30 each wait for one other session's capability, and
00.21 for session 13's end. Each increment changes `benchmarks/` only.

### 00.27 One command per probe run
Scope: four commands that build an arm, prepare the requests, run them and print what happened (benchmarks.md, 4.2, 5.1, 9 and 11.5): `probe`, `calibrate`, `fix`, `suite`; the README's command section, run notes removed.
Needs: nothing.
Files: `benchmarks/src/probe.rs` (new), `src/main.rs`, `src/fix.rs` (`run` :146, `calibrate` :181), `src/batch.rs` (`probe_suite` :318), `src/run/outcomes.rs` (`Neutral::task` :35), `src/lib.rs`, `benchmarks/README.md`; tests `tests/probe_command.rs` (new), `tests/common/mod.rs`, `tests/smith_adapter.rs` (fixtures moved), `tests/probes.rs` (:164).
Shape: the commands, each run from a worktree with no uncommitted change as `heavy --gate cargo run -p smith-bench -- <command>`, after the arm is built in the build lane with `heavy cargo run -p smith-bench -- build <commit>`, which prints the arm's directory as its last line; `--commit`, `--control` and `--parent` reuse the arm `build` archived for their commit, so that no release build holds the gate lane:
  `probe PROBE [--agent smith|codex] [--attempts N] [--commit COMMIT | --arm DIRECTORY]`;
  `calibrate PROBE [--attempts N] [--passes N] [--control COMMIT] [--commit COMMIT | --arm DIRECTORY]`;
  `fix PROBE --parent COMMIT [--commit COMMIT]`;
  `suite NAME [--seed N] [--commit COMMIT | --arm DIRECTORY]`.
  PROBE is a suite's task name, `probes/<id>[/<variant>]`. Defaults: `HEAD`, smith, 1 attempt (rerun once on failure), `calibrate` 12 attempts and 10 passes, a seed drawn from the clock and printed.
  `probe::plan(repository, &Order, &ArmRef) -> Result<Planned, String>` builds the arm with `arm::build` (`arm.rs:343`, which refuses a dirty checkout) or reads `--arm` with `arm::read_build`, takes the probe's suite from `fix::calibration_suite` and its pin from that suite (Codex: `agents/codex/standard.pin.toml`), the tier from `agents/models.toml`, and writes one `run::Prepared` per attempt under `$XDG_STATE_HOME/smith-bench/requests/<run>/`, with the first run's values: teardown 10,000 ms, grace 1,000 ms, settlement 3,000 ms, setup 15 s, 2 MiB of stdout and of stderr, 16 MiB of disk, `force_walk`, `codex_home` the user's `~/.codex`. Run ids are `<probe, '/' as '-'>-<commit, 8 hex>-<UTC yyyymmddThhmmssZ>`, attempts `01`, `02`, ….
  Dispatch goes through `batch::run_probe` (:307), whose `probe_suite` also puts the order's agent in place of the suite's; `suite` through `batch::run` (:301).
  Each attempt prints `probe::line`: `<run>/<attempt> end=<end> verdict=<pass|fail|incomplete> wall=<s> spend=$<usd>`, an unavailable value as `-`, never 0; then one summary line. `calibrate` runs its control first (one attempt; a median probe's repetitions), then attempts until `Counted::calibrated` or not `Counted::reachable`, prints the counts and, when calibrated, the task's `[calibration]` block (`failed_before`: the control's run, or `none: <reason>`). `fix` prints `fix::body` (:95).
  Exit status: `Judgment::code` (`judgment.rs:20`): 0 passed or calibrated, 1 failed or not calibrated, 2 incomplete, refused or a harness error.
  `Neutral::task` (`run/outcomes.rs:35`) keeps, for an agent other than smith, the deciding names of outcome checks and `cache-reuse`: its rule at :39, generalised.
Test shape: `tests/probe_command.rs`. Reuse `unexecuted_arm` and `unissued_auth` (`tests/smith_adapter.rs:100`, `:91`, moved to `tests/common/mod.rs`), `common::events::{balanced, bytes}` (`tests/common/events.rs:74`, `:56`), `tests/recorded/results/attempt.json`. Tests: `a_probe_order_writes_one_request_per_attempt_with_its_pin_and_tier`, `a_codex_order_takes_the_standard_codex_pin`, `a_line_prints_an_unavailable_spend_as_a_dash`, `calibration_stops_at_ten_passes_or_once_ten_are_out_of_reach`, `a_codex_outcome_half_keeps_only_outcome_checks_deciding`, and the process-layer reference `a_replay_arm_runs_through_the_probe_command_and_prints_one_line`: it runs `CARGO_BIN_EXE_smith-bench probe probes/smoke-tools --arm <dir>` with `HOME` and the XDG directories in a temporary root holding a synthetic `.codex/auth.json` (expiry an hour ahead); the arm's `smith` is a shell script that writes `bytes(&balanced())` to the settings' trace path and exits 0; it asserts one `end=completed` line, the exit status matching its verdict, and the login's digest unchanged. Ceilings: 20 ms each, the process test 0.5 s. Reference test:
  ```rust
  #[test]
  fn a_line_prints_an_unavailable_spend_as_a_dash() {
      let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
      let mut result = smith_bench::read_result(&root.join("tests/recorded/results/attempt.json")).expect("recorded");
      result.spend.usd = smith_bench::Measure::unavailable("no prices for this model");
      let text = smith_bench::probe::line(&result).to_string();
      assert!(text.starts_with(&format!("{}/{} ", result.identity.run, result.identity.attempt)), "{text}");
      assert!(text.ends_with(" spend=-"), "{text}");
  }
  ```
Proof: process layer, `a_replay_arm_runs_through_the_probe_command_and_prints_one_line`; then live, once: `heavy cargo run -p smith-bench -- build HEAD`, then `heavy --gate cargo run -p smith-bench -- probe probes/smoke-tools --arm <the printed directory>`, at most 2 attempts, $0.02 and 5 minutes. A pass is exit 0 with a line `verdict=pass`.
Also (decisions.md, 13.14): `arm::build` (`src/arm.rs:343`) holds an exclusive lock, `std::fs::File::lock` on `<state root>/build.lock`, from `prepare_clone` to `archive`, so that two builds in the build lane never share the clone or `build-target` at once; a second build waits for the first. Test: `tests/probe_command.rs`, `a_build_waits_while_another_holds_the_build_lock` (the test holds the lock, a build in a thread has not started its clone after 100 ms, and finishes once the lock is released), at most 40 lines, under 2 s, or, if the build cannot run offline in a test, a unit test in `arm.rs` that `build` refuses to start while `try_lock` on the file fails. Files: `src/arm.rs`.
Don't: write into the checkout; read a login outside the adapters' converter; build a commit other than the one named.
Size: at most 500 lines.
Removes: the `run SUITE --prepared`, `fix … --build-only`, `fix … --prepared` and `calibrate … --prepared` forms; the README's run notes (runs and their identifiers, `phase`, `checkpoint`, increment numbers, the scoped skein URL rewrite).

### 00.13 Fixtures 01-backoff and 02-line-decoder
Scope: the two fixtures, `smith-bench validate`, and the `compare` suite (benchmarks.md, 3.1, 5.2, 11.4 and 11.5).
Needs: nothing.
Files: `benchmarks/tasks/fixtures/{01-backoff,02-line-decoder}/` (new), `src/validate.rs` (new), `src/main.rs`, `suites/compare.toml` and `agents/smith/compare.pin.toml` (new), `README.md`; tests `tests/offline.rs`.
Shape: from the spike's `~/src/rust/smith-benchmark-2026-10-08/tasks/`: `seed/` is `fixtures/<task>/` byte for byte; `grader/` is a crate whose `tests/regression.rs` and `tests/hidden.rs` come from `grading/<task>/` and take the candidate as `{ path = "../workspace" }`; `reference/` is `reference-solutions/<task>/`; the prompt is `prompts/<task>.txt` joined into one paragraph. `task.toml`: kind `fixture`, tier `working`, `setup` warming `cargo build --offline --tests`, `protected` the seed's tests, manifests and `AGENTS.md`, `[[grade]]` `cargo test --offline` in the copy, the grader's tests, and `cargo fmt --check`. 02's `all_chunk_partitions_produce_same_events` (spike `hidden.rs:56`) sweeps `0..(1u32 << 18)`, the 18 cuts of its 19-byte stream, not 15.
  `validate::validate(tasks, names) -> Result<Vec<Validation>, String>` runs each named graded task's `[[grade]]`, or each graded task's when none is named, through `grade::command` (`grade.rs:209`) on a private copy of the seed, then of the seed with `reference/` copied over it, the grader beside each; `smith-bench validate [TASK]…` (such as `fixtures/01-backoff`) prints `<task> seed=<end> reference=<end>`, exit 0 only when each seed fails and each reference passes.
  `compare.toml`: tier `working`, `interleaved`, 5 repetitions, both fixtures, smith (`compare` pin, naming `first-request.json`) and Codex (`standard`).
Test shape: `tests/offline.rs`, beside `check_benchmark_tree`'s tests. Tests: `fixtures_freeze_their_seeds_and_keep_graders_outside_them`, `the_compare_suite_names_both_fixtures_and_both_agents`, and in `src/validate.rs`'s tests a synthetic fixture whose grader is `sh -c 'test -f solved'`: `validate_reports_a_seed_its_grader_passes`, `validate_reports_a_reference_its_grader_fails`. Ceilings 20 ms; the synthetic grader tests 0.5 s.
Also (decisions.md, 13.17): `benchmarks/agents/models.toml`'s `[working.codex]` gets `model = "gpt-6-astra"` and `effort = "high"` (decisions.md, 12.17), its comment replaced by "Pinned by decisions.md 12.17; the user may veto it before session 12 runs (decisions.md, section 10, item 6)." The offline check's admission (`validate_suite`, `validate_agent`, `models.lookup`) is unchanged. `[working.anthropic]` stays empty.
Proof: process layer, once: `heavy cargo run -p smith-bench -- validate`, at most 5 minutes; a pass is exit 0 with both fixtures' seeds failing and references passing. Not live: `compare` runs at the working tier, pinned here and run by session 12 (decisions.md, 12.17).
Don't: run a grader inside the focused suite; put a grader's or reference's file in a seed.
Size: at most 200 lines, the fixtures' own code excepted.
Removes: nothing.

### 00.28 The scripted person's lines, and `cache-reuse/crossing`
Scope: `[[person]]` and `input_end` in tasks and variants, sent through the runner's script (benchmarks.md, 5.2); a variant's `effort`; the `crossing` variant (11.1: "a message part way, at medium effort, so reasoning crosses a user turn").
Needs: nothing.
Files: `src/agent/person.rs` (:18 `Cue`, :25 `Trigger`, :36 `Close`), `src/task.rs` (`Variant` :270, `Task` :295, `with_variant`), `src/check.rs` (`check_task` :116), `src/run.rs` (:1034), `src/agent/codex/prepare.rs`, `tasks/probes/cache-reuse/task.toml`; tests `src/tests.rs`, `tests/smith_adapter.rs`, `tests/codex_adapter.rs`, `tests/cache_probes.rs`.
Shape: `Cue`, `Trigger` and `Close` derive `Deserialize` (kebab-case). `Task` gains `person: Vec<Cue>` and `input_end: Option<Close>`; `Variant` gains `person`, `input_end` and `effort`, each replacing the task's when named. `check_task` refuses a line that is not one line ending in a newline, a line over 4,088 bytes, a `start` trigger, more than 63 lines, and a tool smith does not offer. `run.rs:1034` calls `execute_script` (`process.rs:584`) with the prompt as the `Start` cue, then the task's lines, closing at `input_end` or `AfterAnswer`. Codex refuses a task with person lines as `Refused(setup)` before it reads a login. `crossing`: `effort = "medium"`, a prompt that calls `wait` part way, one line sent `when = { tool-call = { tool = "wait" } }`, the base task's cache checks.
Test shape: in `src/tests.rs`: `a_person_line_names_its_tool_cue_and_ends_in_a_newline`, `a_start_trigger_or_a_two_line_person_text_is_refused`, `a_variant_replaces_person_lines_and_effort`. In `tests/smith_adapter.rs`, written like `a_later_person_line_waits_for_its_named_tool_cue_and_eof_follows_that_line` (:362): `a_task_person_line_follows_its_tool_cue_at_the_terminal`. In `tests/codex_adapter.rs`: `codex_refuses_later_person_lines_before_reading_a_login`. In `tests/cache_probes.rs`: `the_crossing_variant_waits_once_at_medium_effort`. Ceilings: 20 ms; terminal tests 0.5 s.
Proof: world layer, `a_task_person_line_follows_its_tool_cue_at_the_terminal`. Live: `cache-reuse/crossing`, calibrated by session 13's cache step, not by this increment.
Don't: add a delay to person lines; change the base `cache-reuse` task's prompt, seed or checks.
Size: at most 150 lines.
Removes: nothing.

### 00.29 The probes suite
Scope: `suites/probes.toml`, each probe and variant once, calibrated or not, with Codex on outcome halves (benchmarks.md, 11.5; decisions.md, 12.20); one place for each probe's pin.
Needs: nothing.
Files: `src/suite.rs` (`SuiteAgent` :35, `validate_suite`), `src/batch.rs` (:138, :196), `src/summary.rs` (:641), `src/fix.rs` (`calibration_suite` :14), `suites/`; tests `src/tests.rs` (:203), `tests/probes.rs`, `tests/path_probes.rs` (:175), `tests/smith_adapter.rs` (:36, :78), `tests/slow_completion_probe.rs` (:143), `tests/process_tree_probe.rs` (:29).
Shape: `SuiteAgent` gains `#[serde(default)] tasks: Vec<String>`: empty, the suite's tasks; otherwise a subset of them, refused otherwise by `validate_suite`. `probes.toml`: tier `small`, `single`, 1 repetition, `rerun_failed = 1`, at most 7,200 s and 8,000,000 tokens; one smith entry per pin with its tasks (`smoke`: `smoke-tools`, `large-write` and their variants; `first-request`: `first-request`, `paths`, `tool-output`, `process-tree`, `slow-completion`; `failures`, `budget`, `cache-reuse` and `parallel`: their families), and Codex `standard` on `smoke-tools`, `smoke-tools/child` and `large-write`. `calibration_suite` returns `probes.toml`, narrowed by `probe_suite` to the probe and the entry that names it.
Test shape: `tests/probes.rs`. Reference test:
  ```rust
  #[test]
  fn the_probes_suite_names_each_probe_and_variant_once() {
      let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
      let tasks = smith_bench::catalogue(&root.join("tasks"), &root.join("../docs/design")).expect("tasks");
      let suite = smith_bench::read_suite(&root.join("suites/probes.toml")).expect("probes suite");
      let mut named: Vec<&str> = suite.agents.iter().filter(|agent| agent.agent == smith_bench::Agent::Smith)
          .flat_map(|agent| agent.tasks.iter().map(String::as_str)).collect();
      named.sort_unstable();
      let mut expected: Vec<String> = tasks.iter().filter(|task| task.name.starts_with("probes/"))
          .flat_map(|task| std::iter::once(task.name.clone())
              .chain(task.task.variant.iter().map(|variant| format!("{}/{}", task.name, variant.name))))
          .collect();
      expected.sort_unstable();
      assert_eq!(named, expected.iter().map(String::as_str).collect::<Vec<_>>());
  }
  ```
  Also `an_agent_task_outside_the_suite_is_refused`, `codex_runs_only_its_outcome_halves`. Ceilings 20 ms.
Proof: unit layer, `the_probes_suite_names_each_probe_and_variant_once`. Live: the suite's first run is 00.21's.
Don't: change a probe's pin or tier.
Size: at most 120 lines.
Removes: `calibration_suite`'s chain of names; the family suites `budget`, `cache-reuse`, `failures`, `first-request`, `parallel`, `paths`, `process-tree`, `slow-completion` and `tool-output`. `smoke`, `calibrate`, `compare` and `probes` stay.

### 00.17 `budget/visibility`'s told check
Scope: what 00.17 left (its budget and slow-completion probes landed at 97468249 and f7217370): `budget-told` judged against the `budget` record of the step that wrote the child's usage line (benchmarks.md, 11.1; domain/run.md, 9.2), in place of today's unavailable.
Needs: smith 07's writing-step budget [07.14]: the `budget` record a child's return writes names its `sub_agent` call in `call` (protocol/events.md, 3.8).
Files: `src/agent/smith/events/budget_report.rs` (`State::record` :62, `check` :144); tests `tests/budget_visibility.rs`.
Shape: `State::record` pairs each child return to main with the `budget` record whose `call` is that return's call; `check(observer, false)` is true when the told turns and seconds equal that record's ordinary pool (limit less used less reserve; seconds rounded down), false when they differ, unavailable when no record names the call or the capture is missing. `budget-report` is unchanged.
Test shape: `tests/budget_visibility.rs`; reuse `records` (:8), `sequence` (:103), `observed` (:109), `checked` (:116). Tests: `told_values_equal_their_writing_steps_budget`, `a_told_value_off_its_writing_step_fails`, `a_return_without_the_join_stays_unavailable`; `writing_step_absence_stale_and_coincident_snapshots_cannot_prove_smith_told_values` (:136) keeps its cases without the join. Ceilings 20 ms.
Proof: unit layer, `told_values_equal_their_writing_steps_budget`, failing on its parent (unavailable there). Live: `budget/visibility`, calibrated by session 13's `budget/visibility` step [13.7].
Don't: infer the writing step from timestamps or the nearest record.
Size: at most 60 lines.
Removes: the unavailable reason "no exact writing-step Budget join was established".

### 00.14 `follow-up`
Scope: what 00.14 left (`failures` landed at e3e79e9b, `paths` at 9f0394c2): `follow-up` with `eof` and `chat` (benchmarks.md, 11.1; domain/run.md, 6; domain/host.md, 8), calibrated.
Needs: smith 05's lines carried and held [05.13]: carried, held until read, unread by outcome.
Files: `tasks/probes/follow-up/` (new), `suites/probes.toml`, `summaries/`; tests `tests/probes.rs`.
Shape: base: a greeting, then a file asked for through the shell, sent `when = { tool-call = { tool = "finish" } }`; `eof`: two lines, then end of input (`input_end = "after-last-cue"`); `chat`: three exchanges, each sent on `wait`. Checks: `exit-code` 0; `handled-once`; `chat`, `completion-per-exchange`; the passive message terminals.
Test shape: `tests/probes.rs`; reuse `common::events::{balanced, record}` (`tests/common/events.rs:74`, `:7`). Tests: `follow_up_sends_its_request_on_finish`, `follow_up_eof_ends_input_after_its_second_line`, `follow_up_chat_judges_one_completion_per_exchange`. Ceilings 20 ms.
Proof: live, once per variant: `heavy --gate cargo run -p smith-bench -- calibrate PROBE --control <05.13's parent>`, for PROBE `probes/follow-up`, `probes/follow-up/eof` and `probes/follow-up/chat`, at most 12 attempts each, $3 and 30 minutes for the three; calibrated at 10 passes, the control failing.
Don't: judge the terminal's words; read standard output.
Size: at most 40 lines.
Removes: nothing.

### 00.16 `history`
Scope: what 00.16 left (cache at 2f630ca3, parallel at 81640441; `crossing` is 00.28's): `history` and `small-window` (benchmarks.md, 11.1; domain/session.md, 8), calibrated.
Needs: smith 08's compaction into a new window, admitted against the window (08.6).
Files: `tasks/probes/history/` (new), `suites/probes.toml`, `summaries/`; tests `tests/probes.rs`.
Shape: a seed of small files whose values sum, read one call at a time, at least four times `small-window`'s window in total; `small-window` sets `window` (and the derived limits follow); checks: `conversation-opened` of kind `compaction` at least once, `prompt-within-window`, the answer's sum; `request_bytes` and prompt tokens stay in the attempt's records.
Test shape: `tests/probes.rs`. Tests: `history_reads_one_file_per_completion_to_its_sum`, `small_window_requires_a_compaction_and_no_prompt_past_its_window`. Ceilings 20 ms.
Proof: live, once: `heavy --gate cargo run -p smith-bench -- calibrate probes/history/small-window --control <08.6's parent>`, at most 12 attempts, $10 and 45 minutes; calibrated at 10 passes, the control failing. `history` itself runs once, within the same bound: `probe probes/history`.
Don't: shrink the window below what the system text and tools need.
Size: at most 40 lines.
Removes: nothing.

### 00.30 `credentials`
Scope: the rest of 00.17: `credentials` (benchmarks.md, 11.1; domain/host.md, 7), calibrated.
Needs: smith 10's `borrow` and the harness on `smith exec` [10.7, 10.15], whose smith pins name a `borrow` account of `~/.codex`.
Files: `tasks/probes/credentials/` (new), `suites/probes.toml`, `summaries/`; tests `tests/probes.rs`.
Shape: a one-line task answered with a token from the seed; checks: `end` completed, `answer`, `login-unchanged` (its digest unchanged by smith's run).
Test shape: `tests/probes.rs`. Test: `credentials_decides_on_the_login_digest_and_the_answer`. Ceiling 20 ms.
Proof: live, once: `heavy --gate cargo run -p smith-bench -- calibrate probes/credentials`, at most 12 attempts, $0.50 and 10 minutes; calibrated at 10 passes. No control: 10.7's parent has no `borrow` source and refuses the setting as setup (benchmarks.md, 4.2).
Don't: write, refresh or copy the login.
Size: at most 30 lines.
Removes: nothing.

### 00.21 The first baseline on events; the legacy face removed
Scope: `smoke` and `probes` on smith at `main` and Codex's outcome halves, summarised and committed as the first baselines (benchmarks.md, 9); the legacy face deleted in the same change (6.2).
Needs: session 13's last step landed; this session's `suite` command (00.27) and `probes` suite (00.29).
Files: `src/agent/smith/{legacy.rs,faces.rs,prepare.rs,observe.rs,observe/,settings.rs,directories.rs,mod.rs}`, `src/agent/mod.rs` (:145), `src/run.rs` (:644, :777, :848), `src/run/cache.rs` (:52), `src/credential.rs`, `src/guard.rs`, `src/result.rs` (:14, :67), `agents/smith/` (`faces.toml`, `legacy.json`, `smoke.pin.toml`, `standard.pin.toml`), `baselines/`, `summaries/`, `README.md`; tests `tests/legacy.rs`, `tests/smith_adapter.rs` (:8, :131, :276, :368), `tests/codex_adapter.rs` (:370), `tests/fuzzy_parsers.rs` (:44), `tests/offline.rs` (:60), `tests/smith_producer.rs`, `tests/recorded/`.
Shape: `faces.toml` keeps its two `events` entries as `commit`, `shape`, `receiving`; `FaceSince { commit, shape: events::Shape, receiving }`; `Prepared.face` is an `events::Shape`; `Observing { prepared, observer: events::Observer }`; `settings.rs` renders the one event-era layout, today's `LocalReserved`, without a shape parameter; the converter writes the access-only record only; `agents/smith/legacy.json` becomes `smoke.json`. `SmithFace` keeps `Events`, and `RESULT_VERSION` is 2. The three terminal tests at `smith_adapter.rs:131`, `:276` and `:368` replay `common::events::bytes(&balanced())` in place of the Debug recording.
Test shape: `tests/offline.rs`: `committed_summaries_and_baselines_parse_at_result_version_two`; the prompt test at :60 renamed `a_prompt_at_the_terminal_line_bound_is_refused`, unchanged. Ceilings 20 ms.
Proof: live, once each, after `heavy cargo run -p smith-bench -- build HEAD`, through `heavy --gate cargo run -p smith-bench --`: `suite smoke --arm <the printed directory>` (at most $0.50, 15 minutes), then `suite probes --arm <the printed directory>` (at most $10, 120 minutes); then `summarise` and `baseline` as `benchmarks/README.md` spells them. A pass is both suites run with no harness error or setup refusal; a probe's failure is recorded in its summary, and a smith defect stops the increment on a question. Deterministic: `committed_summaries_and_baselines_parse_at_result_version_two`.
Don't: keep a type or a file for pre-events binaries; drop the terminal's one-line prompt bound while `smith local` is the invocation.
Size: at most 150 lines added; about 3,000 removed.
Removes: `legacy.rs`, `observe/checks.rs`, `observe/health.rs` and the Debug usage code in `observe.rs`; `Face::Legacy`, `Observer::Legacy`, `FactShape`, `TokenShape`, `guard::UNUSABLE_REFRESH`, `SmithFace::Legacy`; the settings shapes `LocalJson`, `LocalDeclared`, `LocalPriced`, `LocalDelegating`; `faces.toml`'s eight `legacy` entries; `tests/legacy.rs`, `tests/recorded/smith-legacy/`, `tests/recorded/smith-current/`, `tests/recorded/tasks/smith-legacy/` and the legacy cases of the tests above; the README's legacy paragraphs.

### 00.s1 The harness's test programs in Rust
Added during the run (decisions.md, 13.30). Scope: the two programs
`smith-bench`'s tests spawn as a fake agent are written in Python
(`benchmarks/tests/probe_command.rs:149`, the replay arm, added by 00.27;
`benchmarks/tests/smith_adapter.rs:101`, the terminal peer, from before
this plan), and the user's ruling is Rust unless a language goes through
them (decisions.md, the preamble). Both become one Rust fixture binary.
Needs: nothing; it follows the increment smith 00 is on.
Files: `benchmarks/Cargo.toml` (one `[[bin]]`, `test-peer`, test-only by
its name and docs); new `benchmarks/src/bin/test_peer.rs`;
`benchmarks/tests/{probe_command.rs,smith_adapter.rs}`; `benchmarks/README.md`
(its "immutable `arms/`": say what the code does, "`arms/`, which
`build` writes once and never rewrites", and no synonym).
Shape: `test-peer replay <recording> <settings>` copies the recording to
the settings' `agent.trace.path`; `test-peer terminal <recording> <trace>`
does what the terminal script does (read a line, copy, read a line,
close stdin and stdout, write its stderr line, exit). In place of the
script's "default SIGHUP" (decisions.md, 13.31): it first reads `SigIgn`
from `/proc/self/status`, and if SIGHUP (bit 1, mask `0x1`) is ignored it
writes "test-peer: SIGHUP is ignored; a hangup would pass unseen" to
standard error and exits 3, so the test fails rather than passes blind.
No `unsafe`, no dependency, no skein change. Tests
name it by `env!("CARGO_BIN_EXE_test-peer")`.
Test shape: the two tests keep their names, steps and assertions; only
their program changes.
Proof: unit, `heavy cargo nextest run -p smith-bench -E 'test(a_replay_arm_runs_through_the_probe_command_and_prints_one_line) | test(a_terminal_peer_can_close_its_stdio_before_the_leader_exits_without_hangup)'`, once; and `git grep -n python origin/main -- benchmarks crates tests src` prints nothing once it lands.
Don't: add a dependency; change what either test checks.
Size: at most 80 lines of production code.

## Interface sketches

`benchmarks/src/probe.rs` (00.27) owns preparation and printing; dispatch
stays `batch`'s, judging `judgment`'s. An order ends when its last attempt
has settled and its summary line is printed.

```rust
/// The arm a command builds: the checkout's committed `HEAD`, or a named commit.
pub enum Source { Head, Commit(String) }

/// What one command runs, read from its arguments.
pub struct Order {
    /// `probes/<id>[/<variant>]`, as suites name tasks.
    pub probe: String,
    /// smith with its probe's pin, or Codex with `agents/codex/standard.pin.toml`.
    pub agent: crate::Agent,
    /// Attempts on the arm; a probe of one attempt reruns once on failure (11.5).
    pub attempts: u32,
    pub source: Source,
}

/// Requests written under `$XDG_STATE_HOME/smith-bench/requests/<run>/`.
pub struct Planned { pub run: String, pub suite: PathBuf, pub directory: PathBuf, pub requests: Vec<crate::run::Prepared> }

/// One attempt's printed line.
pub struct Line {
    pub run: String,
    pub attempt: String,
    pub end: crate::End,
    pub verdict: crate::Judgment,
    pub wall_ms: crate::Measure<u64>,
    pub spend_usd: crate::Measure<f64>,
}

/// A calibration batch's counts against the passes its probe requires.
pub struct Counted {
    pub passed: u32,
    pub failed: u32,
    /// Incomplete attempts, "not exercised" included: neither a pass nor a fail.
    pub unjudged: u32,
    /// 10, or a variant's own count (`large-write/oversized`: 3).
    pub required: u32,
    /// The batch's ceiling, 12 unless the command names another.
    pub ceiling: u32,
}

impl Counted {
    /// The required passes were reached within the ceiling.
    pub fn calibrated(&self) -> bool;
    /// More attempts could still reach the required passes; false stops the batch.
    pub fn reachable(&self) -> bool;
}

pub fn plan(repository: &Path, order: &Order, arm: &crate::arm::ArmRef) -> Result<Planned, String>;
pub fn line(result: &crate::AttemptResult) -> Line;
pub fn count(results: &[crate::AttemptResult], required: u32, ceiling: u32) -> Counted;
```

`benchmarks/src/validate.rs` (00.13):

```rust
/// One graded task's two runs: the seed must fail, the reference must pass.
pub struct Validation { pub task: String, pub seed: crate::grade::GradeEnd, pub reference: crate::grade::GradeEnd }

pub fn validate(tasks: &Path, names: &[String]) -> Result<Vec<Validation>, String>;
```

`benchmarks/src/agent/person.rs` and `task.rs` (00.28): the runner's
types become the task's vocabulary. A `[[person]]` line ends when written;
the input ends at `input_end`.

```rust
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cue { pub when: Trigger, pub text: String }

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    /// The task's prompt only; a `[[person]]` line never names it.
    Start,
    /// After this named tool call starts: `when = { tool-call = { tool = "finish" } }`.
    ToolCall { tool: String },
    /// After main's answer.
    Answer,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Close { AfterPrompt, AfterAnswer, AfterLastCue }

// In `Task`: `#[serde(default)] pub person: Vec<Cue>`, `pub input_end: Option<Close>`.
// In `Variant`: `pub person: Option<Vec<Cue>>`, `pub input_end: Option<Close>`, `pub effort: Option<String>`.
```

`benchmarks/src/agent/smith/faces.rs` and `observe.rs` (00.21), with one
face left:

```rust
pub struct FaceSince { pub commit: String, pub shape: super::events::Shape, pub receiving: Receiving }
pub struct Observing { pub prepared: Prepared, pub observer: super::events::Observer }
```

## Questions to raise when reached

None (benchmarks.md, 11.1; shell.md, 4.1; events.md, 3.8 settle them).

## Done when

- `heavy cargo run -p smith-bench -- check` exits 0 and counts at least
  one summary and one baseline, and `ls benchmarks/suites` prints
  `calibrate.toml`, `compare.toml`, `probes.toml`, `smoke.toml` and
  session 12's suites only (on 2026-10-12: 11 suites, no summary or
  baseline).
- `heavy cargo run -p smith-bench -- validate` exits 0.
- `rg -il legacy benchmarks` and `find benchmarks -iname '*legacy*'` print
  nothing (on 2026-10-12: 29 files and 5 paths).
- This prints nothing (on 2026-10-12: 20):
  ```sh
  rg -P -c 'smith00-|phase ?[0-9]|checkpoint|(?<!md |section |sections )\b(0[0-9]|1[0-3])\.[0-9]{1,2}\b' benchmarks/README.md
  ```
- `heavy cargo nextest run -p smith-bench` passes. Aim for under 10 s
  serially (8.25 s on 2026-10-12): 00.21 reads it once, adding `--profile
  measure -j 1`, and past it makes its slowest test cheaper.
- 00.14, 00.16 and 00.30 record calibrations: `git log origin/main --format=%B -E
  --grep='^Step: smith-00\.(14|16|30)$' | rg -c '^Calibrated: '` prints at least 3.
