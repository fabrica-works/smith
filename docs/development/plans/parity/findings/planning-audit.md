# Planning: the code audit

2026-10-14, at smith `b7da2b25` and skein `63fff2d`. The parity plan's
code audit (skein's development.md, 5.4): seven read-only sub-agents,
each with one question and the files to read, each report as it was
returned. The plan cites them as `planning-audit.md`, <section>, point
<N>. Paths are under the plan's design worktree,
`~/src/rust/tmp/design/smith-plan-parity/`, whose code was smith's `main`.

What the planner checked again itself: run.md 3.3's order is the
domain's to decide (point 3 of "Session, run and compaction" names no
defect); the private Codex package's digest is
`12eb3e81114588aca3b7998f4f19e8997b056aca08e57a7ca7c8a3ec8c652aad`; the
old controls `e1feddd6` and the reliability run's predate the product
face `0d5604f9`, and `f2c37fe8` descends from it.

## Harness tasks and pins

I checked all seven points against the worktree at b7da2b25. Its `benchmarks/` matches the indexed checkout file for file, so the graph's line numbers hold. All seven points differ from the design.

1. Fixtures. Design: no `tests` in `protected`; the grader keeps a copy of the seed's tests; references add the tests their prompts ask for; graded tasks declare `deciding`; `validate` applies protected and outcome checks to the reference.
   - Code: all three fixtures protect `tests` (fixtures/*/task.toml:9), though their prompts (:6) say "add regression tests". `grade::changed` (grade.rs:103-105) flags any new file under `tests/`.
   - The grader does hold a copy: grader/tests/regression.rs differs from the seed's only in formatting. Each reference's tests/regression.rs is byte-identical to the seed's, and the references change only the src file, with no `#[test]`.
   - No fixture, and not shell-cwd, declares `deciding`. An empty list means every check decides (task.rs:299-301, judgment.rs:43-55), passive checks included, since they are pushed into the outcome's checks (run/outcomes.rs:72).
   - `validate` runs only the `[[grade]]` commands and judges seed fails / reference passes (validate.rs:28-30, 95-163). It runs no protected or outcome check and no `setup`.
   - Change: drop `tests`, add a test to each reference, declare deciding (end, protected-unchanged, tree-ended; the grader already decides via judgment.rs:56), and run protected and outcome checks in validate.rs.

2. shell-cwd.
   - Protected check: it reads the manifest. run.rs:1050 snapshots `<task>/seed`, which holds only bundle.toml, while the workspace is copied from the bundle (run.rs:443-444, bundle.rs:74-87). So every protected path in the bundle always counts as changed. Change: snapshot the path `seed_source` returns.
   - Setup: there is no `setup` line in task.toml. Change: warm the build there.
   - Prompt (:6): it says "Subagents may be used at your discretion." and has no `$TMPDIR` sentence. Change: drop the first, add "Temporary files go under `$TMPDIR`."

3. Budgets. Design (6.1): comparison tasks name no `[budget]`; `check` refuses one that does, or whose `deadline_seconds` is not below the defaults' `budget.seconds` (1 hour, shell.md:770).
   - Code: every fixture names `[budget]` seconds=840, turns=64 (task.toml:15-17). shell-cwd names seconds=1200, equal to its deadline (:5, :14-15).
   - `check` tests only that budget values are positive (check.rs:285-293); the kind match only requires a grader (check.rs:191-198).
   - settings.rs:27-51 and :126 always write a budget, defaulting seconds to the deadline and turns to 4096, so smith never runs on its run defaults.
   - Change: remove the four `[budget]` blocks, add the refusal at check.rs:193, and write no budget when the task has none.

4. smith pins and faces. Design: the account's file and the trace (capturing `calls`) only, and one face.
   - Every pin is just a JSON template: compare, budget, cache-reuse, failures, first-request and parallel use first-request.json; smoke and standard use smoke.json.
   - From these, settings.rs:98-172 and 215-275 write a layer that also sets:
     - role: smoke.json:20 adds "Read the workspace's AGENTS.md…"; first-request.json has `instructions` "".
     - window 131072 (settings.rs:148), where the preset is 272,000 (shell.md:680-681).
     - output 4096 (json:22), where the preset is 32,000; idle 120 s (settings.rs:152), where the preset is 300 s; plus reasoning_item and head.
     - `sub_agents` true (settings.rs:164); memory 1e12 (json:4 via settings.rs:137-141).
     - headers originator and user-agent "pi" plus OpenAI-Beta (json:10-14); connect and handshake 30 s (:144-145).
     - prices, conventions, contract (json:23-30), and always a budget.
   - Trace: exec gets none and chat always gets `everything` (prepare.rs:181-188, 255-258); neither is `calls`.
   - What goes if "one face" holds:
     - faces.toml entries at :4-36 (local-trace and local-agent-trace) and the face registry; first-request.json and smoke.json.
     - events.rs:56-58 `Shape::LocalTrace`/`LocalAgentTrace`; faces.rs `CredentialShape::Converted` and the ancestry registry; prepare.rs:126-169 and 310-320 (`smith local`).
     - settings.rs:14-275, the whole template renderer and its translation.
     - credential.rs:233-260 `smith()` and `Shape::Smith`; process.rs:590-600 (the `.agent` split trace); the `session_ended_aborted` path in observe.rs:79-96.
   - `SmithFace` has only `Events` (result.rs:67-70; run.rs:787, 869), so results do not name exec, chat or local.
   - Baselines: all ten smith baselines (`baselines/probes-smith-*-20261012.json`, `smoke-smith-events-small-20261012.json`) come from arm 858c9642. That commit descends from face 4d06c110 (local-agent-trace) but not from 0d5604f9 (exec), so they are on the older face. Change: retake or drop them.

5. Codex pin.
   - overrides.toml:9 sets `features.multi_agent` = true and :7 `agents.max_threads` = 6. It also sets :8 `shell_environment_policy.inherit` and :5 `check_for_update_on_startup`. All are beyond the allowed keys; delegation is 11.3's arm. The comment at :2 is stale.
   - No writable roots: `sandbox_mode` is workspace-write (:4), while the cargo home and target sit outside the workspace (isolate.rs:137-138, 266-267). Change: add per-attempt `sandbox_workspace_write.writable_roots` in codex/prepare.rs:88-102.
   - Trust goes to the working directory, not the attempt root (codex/prepare.rs:93-98).
   - No SHA-256: the pin has only version "0.160.0" (standard.pin.toml:4), and the `Configuration` type has no digest field (agent/mod.rs:216-222). Setup checks only `--version` (codex/prepare.rs:48-52, run.rs:766-782).
   - The package is not private: `codex` is found on the machine's PATH (probe.rs:420-423, isolate.rs:61-74), so it is the user's installation.

6. first-request.
   - Its `[smith]` (task.toml:16-37) restates 21 limits, mostly preset values, but has no empty-brief override, and settings.rs:69-80 would refuse an `instructions` key anyway.
   - The empty brief comes from the pin (first-request.json:20), used only in probes.toml:20-21.
   - smoke.toml:8,13 and calibrate.toml:8,13 run it on smoke.json, whose brief asks for AGENTS.md; the seed holds only README.md.
   - Change: name the empty brief in the task's own overrides, and give the overrides a path that accepts it.

7. Users.
   - `prices.toml` (design 8.1: "the harness keeps no catalog of its own"): check.rs:54; main.rs:103-105; lib.rs:36; models.rs:66-69; summary.rs:141, 189, 224-234, 246-268, 343, 390-393, 424, 449-452; summary/compare.rs:10, 28, 30; src/tests/comparison.rs:50; README.md:60, 88.
   - Face and template types:
     - src: prepare.rs:13, 22, 121-169, 176, 327-345; probe.rs:386-388; run.rs:757, 787, 869, 1115; observe.rs:79-96; process.rs:585-600, 621; agent/smith/mod.rs:16; credential.rs:66-69, 146-163, 233-310.
     - tests: tests/smith_adapter.rs:130, 170, 241, 399, 913; tests/smith_adapter/exec.rs:68-69; tests/smith_producer.rs:62; tests/codex_adapter.rs:408, 453; tests/offline.rs:53; tests/events.rs:214-262.

Separately, docs/development/plans/parity/decisions.md §3.3 says `check` refuses a budget "below its deadline", which is not what 6.1 says; I followed 6.1.

### Critical Files for Implementation
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/agent/smith/settings.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/agent/smith/prepare.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/run.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/check.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/agent/codex/prepare.rs

## Harness results and summaries

I found eight gaps. One blocks parity outright: no fixture attempt can pass today. If an attempt's task file names no deciding checks, every check counts, including passive ones that are always unavailable. I checked everything with `rg` and by reading the files; one graph lookup (`bootstrap_ratio` at `stats.rs:152-190`) agreed with the worktree. Paths are under `/home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/`.

1. **Format versions.**
   - Design says: one version per format, readers refuse others, older committed files are regenerated or dropped.
   - Code does: `RESULT_VERSION = 2` (`src/result.rs:14`), refused by `parse_result` (`:456-462`) and `render_result` (`:468`). The summary's 2 and the baseline's 1 are bare literals (`src/summary.rs:462,527,604,688` and `:478,541,613`), refused by `read_summary` (`:602-608`) and `read_baseline` (`:611-617`). `committed_costs` skips other versions instead of refusing them (`:688`).
   - Change needed: name the summary and baseline constants. The time record and per-attempt rows make result v3 and summary v3, so the committed summaries and baselines must be regenerated or dropped.

2. **What a summary keeps per attempt.**
   - Design says (5.4): each attempt's identity, end, judgment, task wall, common priced spend and token fields.
   - Code does: `ArmSummary` (`src/summary.rs:48-68`) keeps only attempt ids, `counts_by_end`, the pass counts and per-metric median, range, CV and minimum detectable effect. The committed file's top keys are version, result_version, run, suite, tier, design, seed, date, groups, comparisons, audits, drift_notices; `groups[].attempts` is just `["06","07",…]`.
   - Change needed: add a per-attempt record carrying those fields, the commit that priced the spend, and a `parity` section. Today no verdict can be recomputed from the file.

3. **Spend and the time record.**
   - Design says (8.1): common priced spend uses smith's presets' list prices; the harness keeps no catalog of its own.
   - Code does: it reads its own catalog, `agents/prices.toml` (`src/models.rs:66`, `src/main.rs:103`), and `priced_spend` applies it (`src/summary.rs:222-244`).
     - It looks up the requested model, not the resolved one.
     - It prices only fresh, cache-read and output tokens, with no cache-write rate and no window tier.
     - The rates disagree with `presets::prices` (`crates/smith-local-shell/src/presets.rs:109-121`, `pub(crate)`). For astra the presets say $10/$1/$12.5/$50 per million (fresh/cache-read/cache-write/output); `prices.toml` says 1.25/0.125/10 (fresh/cache-read/output).
     - smith's pins restate prices (`agents/smith/smoke.json:23-24`).
     - The key `spend_usd/{basis}` (`summary.rs:169`) is overwritten by `spend_usd/priced` (`:189`), so an agent's own priced figure is lost.
   - Change needed: expose a public list-price function from `smith-local-shell` and add it as a dependency. Price by resolved model and window, add a separate common-priced field to `Spend`, and remove `prices.toml`.
   - Time record, code does: `Timing` (`src/result.rs:111-120`) has the four walls, but its `first_byte_ms`, `largest_gap_ms`, `longest_completion_ms` and `agent_t_ms` are always unavailable (`src/run.rs:91-94`); nothing writes them. smith's per-response first byte and gap reach only the headroom checks (`agent/smith/events/slow.rs:91`). Only `delegation.parent_blocked_ms` exists (`result.rs:147`). The Codex rollout parser reads usage only, with no timestamps (`agent/codex/rollouts.rs:137`).
   - Change needed: add a `TimeRecord` of `Measure<u64>` values: model requests (with first-byte and largest-gap sums), each tool by name, compaction, waiting on children, the rest, and Codex's commands outside any call.

4. **Calibration and smoke.**
   - Design says (3.1, 5.2): one `[calibration]` block per task and per variant, written by `calibrate`, and a probe without one gates nothing.
   - Code does:
     - `Calibration` holds only `{binary, passes, failed_before}` and exists per task only (`src/task.rs:258-265,342`); variants have none.
     - `check.rs:199-213` checks its shape (ten distinct passes); nothing else reads it.
     - Only `tasks/probes/credentials/task.toml:48-51` has one, and its `failed_before` is "none: no runnable control was named".
     - `calibrate` only prints the block (`src/probe.rs:606`), and it calibrates with no control at all (`:589`, `is_none_or`).
     - `count` (`probe.rs:118-129`) does not separate model deviations from smith's failures.
     - Other calibration statuses live in commit bodies (e.g. e7aa23a7).
   - Missing from the block: SHA-256, date, summary, attempts run, model deviations, and the prompt, seed and checks digests.
   - Smoke, code does: a static list in `suites/smoke.toml` (no `failures/one-turn`) with no calibration filter; `prepare` (`probe.rs:333-418`) takes the names as given. A probe failing twice counts as a regression whether calibrated or not (`batch.rs:507-520`).

5. **Statistics in `src/stats.rs`.**
   - Code has:
     - `median(&[f64])->Result<f64,String>` at :75
     - `coefficient_of_variation(&[f64])->Result<f64,String>` at :89
     - `minimum_detectable_effect(cv:f64, attempts:u32)->Result<f64,String>` at :110 (2.8·CV·√(2/n))
     - `interleave(arms:usize, repetitions:u32, seed:u64)->Vec<Vec<usize>>` at :131
     - `bootstrap_ratio(reference:&[f64], candidate:&[f64], seed:u64, resamples:u32)->Result<Comparison,String>` at :152 (seeded percentile bootstrap; needs ≥5 per arm and ≥1000 resamples; refuses a zero reference and any non-finite resampled ratio at :171; its minimum detectable effect comes from the run's own CV, not the committed baseline's)
     - `pass_interval(passed:u32,total:u32)->Result<Interval,String>` at :211 (exact Clopper-Pearson)
     - `audit_sample` at :247 and `probe_verdict` at :26
   - Missing: the large-effect (disjoint ranges) test; today there is only a per-group range (`summary.rs:272-281`). Also missing: the FT/FC per-task bootstrap of time or spend per pass, which must count a draw with no pass as infinite; the `resample` helper (:142) is private.

6. **What decides a graded task's pass.**
   - Design says (3.2): end, grader, protected files, tree's end.
   - Code does: `judge` (`src/judgment.rs:31-63`) counts every check when `deciding` is empty, including the unwaived passive checks that `run/outcomes.rs:73` appends.
     - `benchmark-unread` is always unavailable (`run/passive.rs:56-59`).
     - `nothing-outside` is unavailable for smith (`run.rs:667-670`) and only a lower bound for Codex.
     - No fixture or repository task sets `deciding` or `waives`, so a graded attempt is Incomplete at best.
     - The committed compare summary shows 0/30 passes because the fixtures protect a tests path the agent changes (commit e7aa23a7), which 5.2 forbids.
   - Change needed: fix the graded kinds' deciding set to end, grade, `protected-unchanged` and `tree-ended`, and fix the fixtures' protected paths.

7. **Suites and exit codes.**
   - Per-task repetitions, design says (5.3): a suite may give one task its own repetitions.
   - Code does: `Suite` (`src/suite.rs:67-86`) has a single `repetitions`. A per-task count comes only from the task file (`task.rs:339`, used at `batch.rs:64-68`, `probe.rs:395-399`, `suite.rs:294`).
   - Change needed: a per-task repetitions map in the suite.
   - Code does: `rerun_failed = 0` is already accepted (`suite.rs:143` refuses only values above 1).
   - Exit codes, design says (16.4): 2 when the verdict is incomplete, else 1 when a criterion fails, 0 at parity.
   - Code does: `summarise` exits 0 when it writes and 2 on any error (`main.rs:73-110`), never a verdict. `suite` exits with `probe::judgment` (`probe.rs:618-628`): 2 for a harness error, unexpected refusal, quarantine or incompleteness; otherwise `judge_suite` gives 1 on any failed attempt (`judgment.rs:67-91`).
   - Change needed: a parity exit path that tolerates failed attempts. There is no `parity` suite file or command (`main.rs:10-53`).

8. **Rulings not yet in the code.**
   - Startup refusal, design says (8.3): a `failures` variant's refusal is the agent's end, not `Refused(setup)`.
   - Code does: smith's exit 2 still becomes `Refused{setup}` (`result.rs:507`), patched by `expected_refusal` and `expected_startup_refusal` (`batch.rs:107-115`) and `expected_end` (`judgment.rs:33-37`). The summary counts it under setup, outside the rate (`summary.rs:369`, `result.rs:339`).
   - Deciding checks, design says (9): a probe names its deciding checks.
   - Code does: `check.rs:419-464` only validates the names; an empty list means every check decides.
   - Change needed: refuse a probe with no deciding list.
   - Codex package, design says (10): Codex runs from a private copy of its pinned package, and its pin names the executable's SHA-256.
   - Code does: Codex comes from PATH (`probe.rs:420-423`), its version is compared as a string (`agent/codex/prepare.rs:48-52`), and the pin (`agents/codex/standard.pin.toml`) has no SHA-256.
   - `/tmp` listing, design says (10): list the user's `/tmp` entries before and after each attempt.
   - Code does: no listing anywhere. Outside writes come only from Codex's `file_change` items (`run.rs:514-539`).

**Signatures a `parity` command could reuse:**
- `read_summary(&Path)->Result<RunSummary,Refusal>` (`summary.rs:602`)
- `read_results(&Path)->Result<Vec<AttemptResult>,Refusal>` (`:570`)
- `summarise(&[AttemptResult], &str, &str, Design, u64, Option<&Baseline>, &PriceTable)->Result<RunSummary,String>` (`:417`)
- `write_summary` (`:620`)
- `judge(&AttemptResult)->Judgment` and `Judgment::code(self)->u8` (`judgment.rs:20,31`)
- `pass_interval`, `bootstrap_ratio` and `median` (above)
- `same_inputs` (`pub(crate)`, `summary.rs:326`)
- `arm::read_build` (used at `probe.rs:480`) for the `--commit` check

### Critical Files for Implementation
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/summary.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/result.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/judgment.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/stats.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/benchmarks/src/probe.rs

## Tools domain and machine, for jobs

I checked all five points against the plan-parity checkout. Today the code only runs a command to its end and answers once: nothing in the domain or the machine can answer `running`, hold a process group past the answer, or read it again. All paths are under `/home/free/src/rust/tmp/design/smith-plan-parity/`; T = `crates/smith-domain-tools/src`, M = `crates/smith-protocol-machine/src`.

**1. The `shell` call and the type named `Job`**
- **Call:** `Call::Shell { command, timeout: Timeout, output: Budget }` (T/call.rs:44-51). It has no `yield_seconds`, and there is no `Call::Job` or `Tool::Job` (`Tool::Shell` is at T/call.rs:159; `effect` makes `Shell` a write at :212).
- **Admission:** `admit` (T/kit.rs:326) builds `Work::Shell` (T/job.rs:162) using `command::timed` and `command::kept` (T/command.rs:44 and :24; head = budget × `shell_head` / (`shell_head` + `shell_tail`)).
- **Asking the machine:** `job::start` sends `Op::Spawn { cwd, command, head, tail }` (T/job.rs:227-242; T/boundary.rs:175) and parks in `State::Running` (T/job.rs:113).
- **Answer:** `Done::Exited { exit, head, tail, dropped }` (T/boundary.rs:266) becomes `Outcome::Exited { exit, head, tail, dropped, timeout, output }` in `exited` (T/job.rs:507). `Exit` has only `Code`, `Signal` and `TimedOut` (T/call.rs:366). Every op ends in exactly one `Done`.
- **What `Job` is:** `job::Job { kit, tool, observation, state }` (T/job.rs:81) is one admitted call in flight, any tool, held in `Domain.jobs: Slab<Job>` (T/domain.rs:35).
- **Its users:** T/domain.rs:12, 35, 43, 59 (the public `Domain::jobs()`), 66, 97; T/kit.rs:40, 53, 120-121, 147-154, 174-176, 362 (`kit.jobs.len() >= limits.calls`, answered `Busy`); T/limits.rs:11, 95-96, 172.
- **Outside the crate:** `crates/smith-domain-session/src/domain.rs:110-111` re-exports `jobs()`. It is asserted in `tests/tools/src/world.rs:512,859`, `tests/tools/tests/simulation.rs:1328`, `tests/tools/tests/fuzzy_memory.rs:86`, `tests/session/src/world.rs:1998`, `tests/agent/src/world.rs:1793` and `tests/protocol-llm/src/world.rs:1642`.
- **Unrelated `Job` types turned up by the `rg`:** `tests/agent/src/script.rs:16` and `tests/protocol-llm/src/script.rs:16` (script kinds: `HostTools`, `Coding` and so on, used across tests/agent, tests/host and two crates' tests), `tests/run/src/host.rs:230` and `tests/machine/src/lib.rs:271`. They don't collide, but the agent-world one would read confusingly next to job stories.
- **Successor name:** `Flight` is unused in the crate (`Task` is also free). That means `flight.rs`, `Flight`, `Domain.flights` / `Kit.flights`, `Domain::flights()` and `flight::{start, cancel, done, slots, held, io_session}`, which frees `job` for a `jobs: Slab<Job>` per kit.

**2. How the machine runs a command**
- **Spawn:** `process::shell` (M/process.rs:471) runs `/bin/sh -c` with the configured environment, both pipes, and refuses `head`/`tail` above `output_bytes`. The service turns it into `kernel::Spawn` (`crates/smith-agent-service/src/service.rs:2184-2195`). No group flag is set there: every signal targets `kernel::Target::Group` (M/process.rs:240, 316, 341), so the group comes from skein io's spawn.
- **Kept output:** `Capture { head: List<u8>, tail: List<u8> /* ring */, next: u32, total: u64 }` (M/process.rs:67). Sizes are the domain's `head`/`tail`, each at most `output_bytes` = max(`shell_head`, `shell_tail`, `search_bytes`) (M/limits.rs:168). Pipes are re-demanded in reads of `OUTPUT_READ` = 4096 (M/process.rs:17, 565).
- **Group stop:** `Stop::{Running, Terminating, Killing, Closing}` with `Why::{Deadline, Cancel}` (M/process.rs:45-63). `stop` (:300) sends terminate, then `fire` (:325) sends kill after `group_stop`, then `io::Request::Close` after another `group_stop`. `Component::fire` (M/component.rs:396) drives it.
- **Answer:** `answer` (M/process.rs:376) runs only when the process is `ready()`, then removes it (M/component.rs:389-392). It consumes `Capture::finish(self)` (:98).
- **Enums:** `FromDomain::{Op, Cancel, Read, Probe, Check, Abort}` and `ToDomain::{Done, Read, Probed, Checked, Aborted}` (M/boundary.rs:12, 29).
- **To hold a job, the machine needs:**
  - a yield on spawn (an `Op::Spawn` field or a new op) answered by a new `ToDomain` "running" event, with the process kept in `processes`;
  - a `FromDomain` read-job request with a wait and a head/tail budget, answered by a `ToDomain` event carrying output since the last read plus the end once it has one;
  - a stop-job request using the same ladder (`Why::Stop`, which would make `Stopped` distinct from `Cancelled`);
  - a draining `Capture::take(&mut self, head, tail)` instead of the consuming `finish`;
  - a close-only stop for jobs still held at the component's close. The component has no close method today.
- **Routing:** `tools::Request::Io`/`CancelIo` goes through `crates/smith-domain-session/src/session.rs:1099-1100`, `crates/smith-domain/src/route.rs:499-503` and `service.rs:2027-2042` to `FromDomain::Op`/`Cancel`. The answer comes back as `ToDomain::Done` to `domain::Event::Done` at `service.rs:1337`. New variants need an arm at each of these hops.

**3. The process bound (M/limits.rs:167)**
- **Value:** `processes: inputs.operations.min(8)`, where `operations` = `domain.run.calls` (`crates/smith-agent-service/src/profile.rs:630`) = `calls_per_response` (`crates/smith-domain-run/src/limits.rs:262`).
- **Batches per conversation:** not counted. It has no `conversations`, `parallel_tools`, `jobs` or checks term. Tools alone may run `kits` × `calls` = `conversations` × min(`calls_per_response`, 4) at once (T/limits.rs:247), which can already pass 8.
- **A full table:** `start_process` gets an insert `Err` (M/component.rs:286-298; List checks earlier at :182) and answers `other()` = `Done::Failed { fault: Fault::Other }` (:423). That reaches the model as "operation failed: io error" (`crates/smith-protocol-llm/src/render.rs:320-324`).
- **Startup checks:** `validate::machine` (`crates/smith-agent-service/src/profile/validate.rs:229`) compares no process count, and `deadlines` (:118) has no `job_wait` ≤ `tool_deadline`.
- **What must change:** derive `processes` as `conversations` × `parallel_tools` + `conversations` × `jobs` + checks. Add both checks. Add job outputs at `shell_head` + `shell_tail` each to `worst_case`; today it prices only `output_bytes` × 2 × `processes` (M/limits.rs:202).

**4. The path bound**
- **As written:** counted in `normalise` (T/path.rs:136-147) as each name, 1 for `.`, 2 for `..`, plus one separator between parts. The decoder drops empty pieces (`append_part`, `crates/smith-protocol-llm/src/decode.rs:453-456`), so repeated, leading and trailing separators already count nothing (see `Path.parts` doc, T/path.rs:102-103).
- **As resolved:** `join` (T/path.rs:181) counts from the root including the working directory's names. A host alias is counted again in the namespace (T/authority.rs:196-206).
- **Empty path:** refused by the decoder, not the domain: `if parts.is_empty() && !absolute` gives `bad_value("path")` (decode.rs:447-449). The design wants it to decode as the working directory. The domain already refuses the directory itself for `read`/`write`/`edit` as `NotFile` (T/kit.rs:402).

**5. Where `jobs` and `job_wait` go**
- **Domain limits:** `smith_domain_tools::Limits` (T/limits.rs:19) and its `Derivation` (:191), whose service call site is `crates/smith-agent-service/src/profile.rs:455`. The policy values belong in `crates/smith-agent-service/src/profile/quantities.rs:86-94`.
- **Every `Limits` literal to update:** `tests/tools/src/world.rs:62` (`Settings::calm`), `tests/tools/src/memory.rs:11` (`LIMITS`), `tests/tools/src/noisy.rs:67`, `tests/tools/tests/simulation.rs`, `tests/session/src/world.rs`, `tests/agent/src/limits.rs`, `tests/protocol-llm/src/limits.rs`, and the tests.rs files of the domain-tools, domain-session, domain and protocol-llm crates.
- **The tools world's fake machine:** there is no separate crate. The world plays io itself over `skein_fake_checkout`: `enum Work { File, Command { started, since }, Ending }` (`tests/tools/src/world.rs:210`), `start` (:667), `deliver` (:594), and `translate::{spawn, exited, Started}` (`tests/tools/src/translate.rs:94, 123, 75`). Jobs need yield, read and stop states there.
- **Fixtures:** `tests/tools/src/fixture.rs` (`Fixture`, `program`, `test_log`, `long`).

The codebase-memory graph indexes a different checkout, and it reports `M/limits.rs` and `M/process.rs` as changed since indexing. So every claim above comes from reading the source directly; the graph only confirmed the `from_domain` → `admit_tool` → `start_process` path.

### Critical Files for Implementation
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-domain-tools/src/job.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-domain-tools/src/kit.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-machine/src/process.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-machine/src/limits.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/tests/tools/src/world.rs

## Session, run and compaction

Line numbers below are from the plan-parity tree (crates/... is relative to /home/free/src/rust/tmp/design/smith-plan-parity), checked with rg and file reads. The smith graph indexes /home/free/src/rust/smith, so I used it only to confirm callers. Overall: smith has no background shell jobs yet. There is no `job` tool, no `running` outcome and no `jobs` field. The kit's "jobs" (smith-domain-tools/src/kit.rs:11-22) are only its filesystem operations.

1. Batching. `advance` (smith-domain-session/src/session.rs:1459) starts one batch. It asks `joins` (:1616) at :1488 for owned calls and :1533 for delegated ones. Reads join while `started < parallel`, a write runs alone, and the next batch waits for `running == 0`. Room is shared out the same way by `history::reserve_batch` (history.rs:137, :161). The cap is `Limits::parallel_tools` (limits.rs:71), set by `derive` at limits.rs:286 as `inputs.calls_per_response.min(4)`, a hard-coded 4. The tools crate does the same at smith-domain-tools/src/limits.rs:247 (`calls`). There is no `parallel_tools` policy anywhere (`Policy`, smith-agent-service/src/profile/quantities.rs:58), so it would replace the 4 in both places, as `min(parallel_tools, calls_per_response)`.

2. Compaction's request. It is asked with the conversation's own choice: the `Prompt` built at session.rs:2201 uses `conversation.choice.clone()`, and `Purpose::Compaction { room }` (:2186) only changes the room for owed results (smith-protocol-llm/src/prompt.rs:146). Fix: send `ToolChoice::None` when the purpose is `Compaction`, and keep `conversation.choice` for the new window.
   The summary is `text_of` (session.rs:2239). It joins the blocks with nothing between them and also takes Refusal text. Fix: a blank line between blocks.
   The new window is built in `open_window` (compaction.rs:209-216): one user message of [task, summary, held-back messages...]. The preamble goes between the task and the summary. `restore` reads the summary at `content.get(1)` (compaction.rs:424), which then becomes index 2. The test tests/session/tests/compaction.rs:59 expects [wake, the summary, go on].
   No preamble exists anywhere. It needs a field beside `Spec.summary` (smith-domain-session/src/boundary.rs:281) and `Open.summary` (smith-domain-run/src/boundary.rs:673), set in opening.rs:60 (`main`) and run.rs:2036 (`child_opening`), and counted in `render_fixed` (prompt.rs:84) and `summary_bytes` (prompt.rs:34, used at smith-agent-service/src/profile.rs:486).
   The instruction is `SUMMARY` (prompt.rs:30): "Summarise the work so far so it can continue in a new context window. Keep the task, decisions, changes made, current results, pending work and constraints. Be concise and concrete." Against run.md 3.3's four requirements:
   - It never says the work is not finished, or that the new window opens with the task's own words.
   - It never says to answer with text and call no tool.
   - It never asks for exact values, names and paths as they were read.
   - "Keep the task" contradicts "do not restate the task or its required answer".

3. System text order. `system` (prompt.rs:209) does put the instructions first: `render_main_prefix` (:309) writes instructions then brief. `render_system` (:344) then writes: guides, `## Tools`, `## Checkout` (workspace and checks in one section), `## Sub-agents`, WAIT, `## Finishing`, `## Delivery`. `## Budget` comes last (:212, :218). run.md 3.3 wants: workspace, tools, guides, checks, budget, sub-agents, finishing, waiting. So guides must move after tools, Checkout must split (workspace before tools, checks after guides), Budget must move before Sub-agents, and wait must come last. A child gets `## Your share` last (:239). The test at prompt.rs:926 asserts the text starts with "## Tools".

4. Session close and the lease. `close` (session.rs:467): a Calling session goes to `cancel_call` (Request::Cancel, :1866); a Tooling one goes to `cancel_tools` (:1873), which withdraws delegated runs; the kit's runs are cancelled by `settle` (:1170), which closes the kit at :1183. It waits until `Waiting {call, runs, kit}` (:257) equals `SETTLED` (:277), then pushes `Request::Ended` (:1202). Since no jobs exist, there are no `job.ended` records today. They would be told in `settle` once the kit reports Closed, before :1202, as a new `FactKind` beside `Ended` (facts.rs ~148) and a matching run fact.
   The lease is `Lease::{Free, Child, Workspace}` (lease.rs:10-18).
   - `sub_agent` (run.rs:1898) calls `run.lease.child(id)` at :1976, before the child's `Conversation` is built. "Cancel the parent's jobs" goes there.
   - `finish` (run.rs:1767) calls `lease.workspace` at :1817, and `deliver` (run.rs:1830) at :1871, each followed by `land::begin` (land.rs:53). "Cancel every running job first" goes between the two: a new land stage before the checks.
   - The run closes a session with `close` / `closing` (run.rs:2129/:2149), which push `Request::Close`.

5. Transcript. `Turn::FORMAT_VERSION = 3` (smith-transcript/src/lib.rs:29), from `generated/v3.rs` (:12). The schema's `versioned record Turn` (schema/transcript-v3.schema:83-96) has kind, affinity, endpoint, model, effort, dialect, place, usage, spent, messages. `jobs: u32` goes after `spent`. Two other schema gaps: `Returned::not_run` (:71) carries no reason, and `Outcome` has no running or job output. The domain side is `record::Turn` (smith-domain-session/src/record.rs:143-159), where `jobs` goes after `spent` at :156; it is encoded by `encode_turn` (smith-protocol-channel/src/turn.rs:17, :69-78). Goldens are in crates/smith-transcript/golden/v3/, read by tests/codecs/tests/transcript.rs:8-9; tests/codecs/tests/drift.rs:22-28 ties the schema to the generated code. A v4 needs a new schema, generated file and golden directory.

6. skein's fake. Today every query's prefix includes its tool choice: `Prefix::system` (cache.rs:205: model, tools, system text), then `prefix.choice` for every caching mode, in `tokens` (:113), `read` (:126) and `write` (:149). The choice changes only the digest, not the byte count (:248). So an automatic query whose choice changed reads nothing, because automatic writes store only the whole prompt (:164).
   fake-llm.md 2.1 adds the choice for a marked query alone. That means `prefix.choice` only under `Caching::Marks` in `read`/`write`, so an automatic query keeps its whole prefix, as the Codex wire does.
   - The cache.rs test that covers choice, `marked_reads_stop_at_the_frontier_and_choice_changes_keep_only_system_hits` (:369), is marked and should keep passing.
   - The other four (:320, :343, :355, :389) use a fixed Auto choice.
   - A new test is needed: an automatic query that keeps its whole prefix across a choice change.
   
   Smith tests that assert the fake's cache counts:
   - tests/protocol-llm/tests/brief.rs: `anthropic_breakpoints_account_for_thinking_and_a_large_tool_batch` (:550, asserting cache_read > 0 on every later turn at :650) and `native_accounting` (:387).
   - tests/protocol-llm/tests/budget.rs: `two_priced_models_and_a_refused_child_delegate_conserve_global_own_charges` (:397, :422) and `parking_story` (:683, :741-742).
   - tests/agent/tests/budget.rs: the test of the same name (:338, :363).

### Critical Files for Implementation
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-domain-session/src/compaction.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-domain-session/src/session.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-domain-run/src/prompt.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-transcript/schema/transcript-v3.schema
- /home/free/src/rust/skein/testing/skein-fake-llm-domain/src/cache.rs

## LLM protocol, events and transcript

I checked all seven points against the worktree. The codebase-memory graph indexes the main checkout, not this worktree (its name search returned nothing), so every citation below comes from rg and direct reads. llm.md has no section 13; its world list is section 10 (llm.md:434-503). Paths are under /home/free/src/rust/tmp/design/smith-plan-parity/, with L = crates/smith-protocol-llm/src and E = crates/smith-events/src.

1. Descriptions. The limits come from `Stated` (L/stated.rs:10-31, filled by `Stated::from_tools` at :36) and are written in by `descriptions::words` (L/descriptions.rs:48-99) and `descriptions::properties` (:124-181), called from `tools::owned` (L/tools.rs:154).
   - Shell's units contradict each other: `timeout_seconds` reads "Seconds; default deadline <shell_timeout> ms; effective maximum <tool_deadline> ms" (descriptions.rs:169-174). Decode takes seconds (L/decode.rs:161-163), and the clamp renders in ms (L/render.rs:411-417).
   - Missing for `shell`: `yield_seconds` (absent from the schema, the decoder and `tools::Call::Shell` at crates/smith-domain-tools/src/call.rs:38-44), `job_wait`, the `running` answer with its job number, and the timeout counted from start. The whole `job` tool is absent: no `Tool::Job`, no "job" arm in decode.rs:32-42 or tools.rs:17-41. `Stated` has no `job_wait` or `jobs`, and neither name appears anywhere in crates/.
   - `search` (descriptions.rs:62-68, and "Search pattern." at :143) says neither that it runs rg nor that the pattern is a regular expression.
   - Path fields (`descriptions::path`, :101-110) say only "Relative to … max N bytes". Missing: `.` and `""` name the working directory, how the bound counts, and that the resolved path keeps the bound. The batching words (:91-98) never mention `job`.

2. Empty path. In `decode::path` (decode.rs:435-451), `append_part` skips empty pieces (:454-456), so `""` leaves `parts` empty and :447-448 returns `bad_value(b"path")`. Fix: return `Path{absolute:false, parts:[Part::Current]}`, the same value `"."` gives. That reaches read, list, write and edit (`required_string` at :111, :132, :153, :158) and search (`optional_string`, already defaulting to "." at :139-142). No test expects the refusal: `malformed_workspace_calls_have_field_named_typed_problems_then_correct` (tests/protocol-llm/tests/production_tools.rs:90) uses `{"path":12}` and `{}`. The only `{"path":"","content":""}` measures framing (crates/smith-agent-service/src/profile/path.rs:92). A new test is needed for each path field.

3. ToolChoice. smith maps it to skein's at L/prompt.rs:98-110. In skein rev 74b7fda (~/.cargo/git/checkouts/skein-336d5ae5eecfa0f9/74b7fda/crates/skein-llm/src):
   - Codex (openai/request.rs:114-118) always writes `tool_choice`: "auto" for Auto and Only, "none" for None.
   - Anthropic (anthropic/request.rs:276-284) omits it for Auto and Only, and writes `{"type":"none"}` for None.
   - So no dialect renders Only. tests/protocol-llm/tests/client.rs:579 `only_uses_automatic_wire_choice_and_keeps_all_definitions_and_not_run_feedback` pins that.
   - A compaction cannot carry a different choice: crates/smith-domain-session/src/session.rs:2186 sets `Purpose::Compaction`, but :2201 sends `conversation.choice.clone()` either way, and compaction.rs:107 and :256 price `choice_bytes(&conversation.choice)`. Change: force `ToolChoice::None` there.
   - A further bug: `result_text` renders a past `NotRun::OutsideChoice` with the current request's choice (prompt.rs:457, then `outside_choice` :181-210, which errors for Auto at :192). A None compaction after an Only narrowing would rewrite earlier history bytes. The not-run result must carry the choice that was in force. No test covers this; extend `a_compactions_owed_results_are_cut_and_its_history_is_unchanged` (production_render.rs:371).

4. Rendering. `render::render` (L/render.rs:154) matches `tools::Outcome` (call.rs:218-341). `Outcome::Exited` (:214-246) writes "exit code N", "signal N" or "command timed out", then the head, "... N bytes dropped ...", the tail, and `command_following` clamps (:408). There is no running or job outcome. Each needs:
   - a new `Outcome` variant and an arm in `render()`;
   - a size in `render_worst_case` (L/limits.rs:147-176);
   - a case in `random_outcomes` (L/tests.rs:111);
   - a verdict in crates/smith-domain-tools/src/facts.rs:196 and `verdict_kind` (crates/smith-agent-service/src/calls.rs:426);
   - a place in the transcript: `Outcome` in crates/smith-transcript/schema/transcript-v3.schema:50-57 has neither.

5. smith-events.
   - `ToolCompleted` (E/records.rs:888-902) has run, conversation, call, tool, verdict, exit, bytes, omitted_bytes, clamped, duration_ms, child, delivery; no `job`.
   - `Verdict` (records.rs:199-267; strings in `write_verdict`, write.rs:260-296; read at read.rs:557ff) has no `running` or `stopped`.
   - `VERSION = 1` (E/lib.rs:34), written at write.rs:83, checked at read.rs:115.
   - `job` goes after `exit` in records.rs:894, `encode_tool_completed` (write.rs:1889), `read_tool_completed` (read.rs:1377), `validate_tool_completed` (write.rs:1364), and `size_tool_completed`/`heap_tool_completed` (limits.rs:813/311).
   - `job.ended` needs a new struct and `Event` variant (beside records.rs:1061), its type name (write.rs:97), dispatch (write.rs:119 and 881, read.rs:132), sizes (limits.rs:34, 65, 98) and the agent's view (crates/smith-agent-service/src/sinks/view.rs:268).
   - Under events.md 6, both changes make version 2. The goldens are crates/smith-events/golden/v1/*.jsonl, 211 files, all "v":1; tool.completed's are `tool_completed_{none,calls,everything,signal,unclamped}` and `verdict_*`.
   - Drift: tests/codecs/tests/drift.rs:53 `event_records_and_every_listed_value_match_frozen_version_one_goldens` hardcodes golden/v1 and requires one file per `samples()` entry in tests/codecs/tests/support/events.rs (`value_tool_completed` :579).
   - `a_frozen_line_of_another_version_is_refused_by_name` (tests/codecs/tests/events.rs:264) expects golden/unsupported-v2.jsonl to fail with `Error::Version(2)`, which a bump to 2 breaks. events.rs:23, 305, 914, 958, 974 and 1005 also hardcode golden/v1; E/tests.rs:93 has `another_version_is_named`.

6. Generated code. `rg -l generated --type toml` finds only tests/codecs/Cargo.toml (in a description string) and Cargo.lock files. Only smith-channel (v2.rs), smith-charter (v2.rs) and smith-transcript (v3.rs) have src/generated, each "Generated by skein-codegen" from crates/*/schema/*.schema and checked by drift.rs:8, :22 and :36 (`*_schema_matches_generated_code_and_all_goldens`). smith-events and smith-protocol-llm are hand-written. temper (/home/free/src/rust/temper/Cargo.toml:78-87) depends on all three generated crates plus smith-domain-tools; jig (checkout 2860e63, Cargo.toml:55-59) depends on smith-domain-tools. Neither uses smith-events or smith-protocol-llm. So new `Outcome`/`Call` variants and a transcript v4 reach temper and jig; event version 2 does not.

7. Tests that pin text or bytes:
   - L/tests.rs:294 `golden_descriptions_state_batching_and_every_effective_parameter`: exact description strings, per-schema "description" counts, and shell's "Seconds", "12000 ms", "43000 ms".
   - L/tests.rs:388 `path_descriptions_follow_the_actual_conversation_namespace`.
   - L/tests.rs:488 `fixed_own_schema_measurement_matches_the_complete_fixed_inventory`.
   - tests/protocol-llm/tests/workspace.rs:530 `the_descriptions_state_each_effective_workspace_limit_on_both_native_wires` ("43000 ms", counts 3/3/3/2/5/3) and :615 `namespace_descriptions_and_list_slash_reach_both_native_provider_dialects`.
   - production_tools.rs:40 `production_schemas_cover_every_offered_workspace_and_run_tool` and :330 `shell_budgets_decode_by_unit_and_a_unitless_timeout_is_ignored`.
   - Render text: production_render.rs:96, and :312 in `command_clamps_are_rendered_without_losing_the_exit_or_omitted_output` ("9000 ms to 5000 ms").
   - Byte sums, which measure rather than pin: workspace.rs:89, brief.rs:417, native_root_restore.rs:375, root_memory.rs:506. tests/codecs is covered in point 5.

### Critical Files for Implementation
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-llm/src/descriptions.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-llm/src/decode.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-llm/src/prompt.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-domain-session/src/session.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-events/src/records.rs

## Product shell, presets and start

I found gaps on all six points. File:line references are relative to /home/free/src/rust/tmp/design/smith-plan-parity/. I ran nothing and changed nothing; the graph (project home-free-src-rust-smith) confirmed the call chain for point 6, and `rg` confirmed every name below.

1. Presets (crates/smith-local-shell/src/presets.rs)
- `MODELS` (7-13) has five models and no `gpt-6.1-sol`. The Codex block (50-63) gives astra and luna window 272000, output 32000, reasoning_item 65536, head 60, idle 300, price_unit 1e6. The Anthropic block (79-99) gives 200000/32000/65536/60/120. connect and handshake are 10 (45-46, 75-76). No model declares efforts, as 6.1 says.
- `prices()` (109-121), in micro-dollars: astra and luna switch to long-context rates past 272_000 (111, 113), haiku past 100_000 (116).
- For sol: add it to `MODELS` (array becomes 6), copy luna's block, and add two `prices` arms: the base arm `(2_000_000, 100_000, ?, 10_000_000)` and an `if window > 272_000` arm. The brief gives neither the long-context rates nor a cache-write rate. `fill_prices` requires a cache-write rate (`expect("preset write rate")`, 140). The astra/luna pattern would give 4_000_000 / 200_000 / 15_000_000 long, and 2_500_000 cache-write, but the rates need a provider source, and the doc comment (106-107) needs updating.
- The dot in the name makes `settings::name` quote it (settings/mod.rs:26-27). The keys become `models."codex/gpt-6.1-sol".*` and sort first in settings/built-in.golden, which `the_built_in_layer_matches_its_golden` pins (settings/tests.rs:93-101). Add a tier test like 139-144.
- Every model of main's endpoint joins the run, so sol enters every Codex run. Re-check the memory tests (settings/tests.rs:448-586).

2. Default instructions
- Set at settings/resolve/local.rs:143-151: `"Carry out the person's request in this workspace. Finish with what changed and what you checked."` (96 bytes).
- No test pins it. The end-to-end settings all set `instructions` themselves (tests/end-to-end/support/exec.rs:31, support/mod.rs:264, end_to_end.rs:406).
- Current bounds: the setting allows `Text(262_144)` (settings/schema.rs:32); local `text_bytes` is `1 << 16` (local.rs:69), checked by `Config::validate` (smith-local-domain/src/config.rs:128).
- The role should be a const in smith-local-shell, e.g. `TASK_ROLE` plus a 4096 bound in presets.rs or a new role.rs, used at local.rs:147. Add the step test from shell.md 10: within 4 KiB, each tool it names one of smith's own.

3. Declared quantities and policy values
- smith-agent-service/src/profile/quantities.rs declares them: `Declared` (25-54), `Policy` (58-103), `standard()` (153-197). `jobs`, `job_wait` and `parallel_tools` exist nowhere.
- `parallel_tools` is hard-coded as `calls_per_response.min(4)` in three places: profile.rs:521, smith-domain-session/src/limits.rs:286, smith-domain-tools/src/limits.rs:247 (`calls`).
- Name clash: `smith_domain_tools::Domain::jobs()` (domain.rs:59) and `job::slots` (job.rs:537) count calls in flight, not background jobs.
- Changes:
  - Add `Declared.jobs` = 4, `Policy.job_wait` = 30 s, `Policy.parallel_tools` = 4.
  - Pass `parallel_tools` into the session and tools `Derivation`s and use `min(parallel_tools, calls_per_response)` in the three places. Bound it 1..=`MAX_PARALLEL` (session limits.rs:117).
  - In `policy_deadlines` (validate.rs:80-125): `job_wait` positive and `job_wait <= tool_deadline` under `Check::Deadlines`.
  - Settings: schema.rs:41-45 and 59-63, resolve/document.rs:9-38. Agent JSON overrides: smith-agent-shell/src/config.rs:63-96, 570-592, 744-748.
  - `jobs` per conversation also goes into machine-holds-tools and memory (profile.rs:107).
- Printing: list.rs `declared()` (86-118) and `policy()` (120-170) emit `declared.*` and `policy.*`. Add `declared.jobs`, `policy.job_wait`, `policy.parallel_tools`. Today `parallel_tools` shows only as derived `session.parallel_tools` (list.rs:321), and `connection_keep` is not printed at all (`connection_keep: _`, list.rs:133).
- `smith check` gets its names from check.rs:232-279 through `profile::header` (effective/header.rs:12) and renders text at check.rs:198-230; the event stream's header comes from startup.rs:84-110.
- No golden test holds the names list. The pins are effective/tests.rs:55-85 and tests/end-to-end/end_to_end.rs:500-507. smith-events/golden/v1/session_started_effective_limits.jsonl holds sample names only.
- limits.md 9 makes names part of the events vocabulary, so these additions may mean a new events version (events.md 6). Ask the designer.

4. Startup checks
- `Check` (profile/checks.rs:12-55, names 60-84) has every limits.md 4 name except `host-part-holds-the-role`. Add `HostPartHoldsTheRole` there.
- The host's part is measured in `room::room` (profile/room.rs:41-115): per model, window at threshold less two outputs less (own + option_bytes), smallest taken; `host = remaining / 2` (109). That becomes `run::Limits::host_part` (profile.rs:505, smith-domain-run/src/limits.rs:244).
- Guides get `min(host / run.directories, guide)` (profile.rs:517-519), so today a small host part shrinks the guides instead of refusing.
- At the entrance, `check_window_parts` (smith-domain-run/src/charter.rs:302-347) measures with `prompt::host_bytes` (prompt.rs:172, pub(crate)).
- Put the check after `run::derive` (profile.rs:~514): role bound + `run.directories` (2, limits.rs:248) × `guide` + rendering ≤ `parts.host`. The service needs the 4 KiB bound passed in, e.g. on `Configuration` (quantities.rs:145), or the check runs in smith-local-shell. An exact rendering needs a public measure from smith-domain-run.
- The margin is thin: 6.1's ~20 KiB offer puts the host part near 40 KiB, against 36 KiB for the role and two guides.

5. Effort bound
- 32 is hard-coded at: charter-v2.schema:49, `CEILINGS.llm_effort_some` (smith-charter/src/generated/v2.rs:63; `CEILINGS` is pub), transcript-v3.schema:88, smith-domain-run/src/charter.rs:420, settings/schema.rs:24 and 119, smith-agent-shell/src/config.rs:815.
- limits.md 2.4 wants one bound, the charter schema's. Replace the code literals with `smith_charter::CEILINGS.llm_effort_some` and assert the transcript's bound equals it. A 33-byte refusal is already tested (settings/tests.rs:641-646).

6. `smith exec` with no commit
- The chain: src/main.rs:11 → `compose::start` (compose.rs:474). `maybe_start` (smith-local-domain/src/domain.rs:770-776) calls `survey_heads` (855-879), which sends `GitOp::Head` for each writable git directory (git is detected by an ancestor `.git`, local.rs:207).
- That runs `rev-parse HEAD` (smith-local-protocol/src/git.rs:154-155), which fails with no commit (208-210).
- `head_result` (domain.rs:881-908) then sets Phase::Idle and sends `Notice::Failure(Failure::HeadUnavailable)`, which renders as "Repository head unavailable" (terminal.rs:167).
- Why it waits: `finish_failed_start` (833-837) only calls `closed()`, and `closed()` returns at once in Headless mode (987-991). No `Request::Exit` is sent, so local_host.rs:374 never records an exit. An interrupt then finds Idle and exits `Accepted`, code 0 (966-969). The credential branch (756-768) does exit for Headless.
- Needed:
  - Make it `Failure::HeadUnavailable { directory }` (boundary.rs:274), from `Directory.name` or `.host` (smith-host-domain/src/channel.rs:77-89).
  - For Headless, set Phase::Done and push `Request::Exit { status: ExitStatus::Refused }`; that maps to 2 (front/headless.rs:38, smith-local-service/src/stream.rs:381).
  - Render "run refused: repository <directory>: its head cannot be read" at terminal.rs:167 and its test at 414; local_host.rs:691 adds the "smith: " prefix.
- Story: tests/end-to-end/end_to_end.rs, beside `local_host_io_failure_keeps_exit_1` (440-449). `Scratch::new` always commits (tests/real/src/lib.rs:78-108), and `support::smith` runs in `scratch.path()`, which is not a repository (support/mod.rs:603-630). It needs an uncommitted-repository fixture; then assert exit 2, the last line, and that no provider request was made. Nothing tests `HeadUnavailable` today, so add a step test in smith-local-domain/src/tests.rs as well.

### Critical Files for Implementation
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-local-shell/src/presets.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-agent-service/src/profile/quantities.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-agent-service/src/profile.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-agent-service/src/profile/checks.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-local-domain/src/domain.rs

## The list defect

The 8 failures were not caused by the process table. They come from the listing parser's sort check, and depth has nothing to do with it. The parser expects rg's output sorted byte by byte. `rg --sort=path` sorts each directory by name instead, so `foo/x.rs` comes out before `foo.rs`, and the parser treats that as a malformed stream.

Path of a list call (code at main, identical at a64f0523):
1. `crates/smith-protocol-llm/src/decode.rs:131` decodes `Call::List`. `crates/smith-domain-tools/src/kit.rs:274-290` admits it. At most min(calls_per_response, 4) calls per kit (`limits.rs:247`); more get "tool is busy" (`kit.rs:361`).
2. `crates/smith-domain-tools/src/job.rs:211-213` turns it into `Op::List`. `crates/smith-protocol-machine/src/component.rs:181-202` receives it.
3. The machine opens and closes the directory through skein io's file layer (`process.rs:173-209`). Then it spawns a process, not a directory walk: `/usr/bin/rg --no-config --files --null --sort=path --glob=!**/.git/** --glob=!**/.git -- .` (`process.rs:613-636`).
4. rg's stdout goes into `Listing::push`/`end_path` (`listing.rs:52-102`) and `finish` (`listing.rs:156-175`). Results come back through `job.rs:361/365`.
5. `Fault::Other` is rendered as "operation failed: io error" at `crates/smith-protocol-llm/src/render.rs:324`. Skein has no such text.

What maps to Fault::Other for a list:
- (a) listing.rs:71-72: a path not strictly greater, byte by byte, than the previous one; or empty; or containing `.`/`..`.
- (b) listing.rs:59-61: a path longer than search_line_bytes.
- (c) listing.rs:160-165: unterminated output, rg exiting with a code other than 0 or 1, or a signal.
- (d) process.rs:206-207: the directory close fails. process.rs:663: other kernel errors on open.
- (e) process.rs:577-578 and component.rs:366-367, then listing_failure (process.rs:645): a pipe failure, or skein io refusing the spawn with Busy because its entity slots are full (`skein-io/src/layer.rs:379`, `limits.rs:14-16`).
- (f) component.rs:182-184 and 286-296: the machine's process table is full.

Candidates:
1. Strongest. Traced in code, seen in a recorded attempt, and reproduced locally with read-only `/usr/bin/rg` 14.1.1. That rg emits `./profile/effective/tests.rs` before `./profile/effective.rs`, because `.` (0x2E) sorts below `/` (0x2F). The same happens for `smith-domain/` before `smith-domain-run/`. Case (a) then marks the whole listing failed.
   - Inversions in this checkout: `.` has 46 of 1984 paths, `crates` 24 of 1251. `docs/design/protocol` and `docs/design/domain` have 0.
   - This matches the 8 failures exactly. Every list of `.` or `crates` failed; every docs list succeeded.
   - Depth is irrelevant: `crates` at depth 1 also failed (attempt 02, t=257236). The parser reads every file rg selects whatever the depth, so "depth 2 or more" is a coincidence of which paths were asked for.
   - The durations (188-1422 ms, `.` about 1.2 s, `crates` about 0.2 s) show rg ran its full walk before the answer.
   - The tests assume the wrong order. `crates/smith-protocol-machine/src/tests.rs:897` feeds `./a.b\0./a/x`, which rg never produces. `tests/machine/tests/fuzzy_listing.rs` generates paths from a byte-sorted set with no prefix-sibling names. The native-rg fixture (`tests/machine/tests/listing.rs` around line 170) has no `x.rs` + `x/` pair.
2. The lead (8 processes, full table renders as that text): true in code, false for these failures.
   - limits.rs:167 sets processes = min(operations, 8), and operations = run.calls = calls_per_response (`smith-agent-service/src/profile.rs:630`). A full table answers `other()` immediately, which renders as "io error".
   - In the recorded attempts, at most a list, a search and some reads were in flight together; reads use file io, not processes. Two failing lists ran alone (02, t=303870 and 310868). A full-table refusal would take about 0 ms, not hundreds.
   - It can only be reached when calls_per_response is above 8 and three or more conversations each have 4 process tools in flight (shell, search or list). The run layer answers "busy" beyond calls_per_response, and each kit is capped at 4. One response alone can't reach it.
   - Not verified: that this run's calls_per_response was 16. I took that from `benchmarks/src/agent/smith/settings.rs:437`; it isn't in the attempt records.
3. Not verified, and unlikely given the timings: (c) rg exiting with code 2, (e) skein io refusing the spawn, (b) an over-long path.

Deterministic test for candidate 1:
- Machine world, native-rg story in `tests/machine/tests/listing.rs`, using its `listed` helper. Fixture: `src/a/x.rs`, `src/a.rs`, `src/a-b/y.rs`. Run the spawn's real rg arguments, assert stdout is `./src/a/x.rs\0./src/a-b/y.rs\0./src/a.rs\0` (or whatever order that rg actually prints), and expect `Done::Listed`.
- Today it should return `Failed{Other}`.
- Pure unit form: `listed(b"./a/x\0./a.b\0", Whole, 4, 512, None)` in `crates/smith-protocol-machine/src/tests.rs`.
- A fix would compare paths component by component (`/` lowest), which is rg's order, and change the fuzzy test to emit that order.

Graph note: the codebase-memory index is for `~/src/rust/smith` (main), and `process.rs` shows as changed there. I relied on direct reads of the parity checkout, and checked with `git show` that `listing.rs` and `process.rs` are the same at a64f0523.

### Critical Files for Implementation
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-machine/src/listing.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-machine/src/process.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/crates/smith-protocol-machine/src/component.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/tests/machine/tests/listing.rs
- /home/free/src/rust/tmp/design/smith-plan-parity/tests/machine/tests/fuzzy_listing.rs
