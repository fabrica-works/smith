# Session 03: results and summaries

2026-10-14. A verdict must recompute from a committed summary, and today
none can: summaries keep counts and medians, not attempts; spend is
priced from a catalog of the harness's own that disagrees with the
providers' list prices by up to 20 times; the time a result reports is
mostly "unavailable"; Codex's linked children are not counted; and a
repository run computes no large effect (findings T6, T11, T13, W8, M10).
This session gives results their version 3 and summaries theirs. It
changes `benchmarks/`, and makes one price function of `smith-local-shell`
public.

## Read first

- smith `docs/design/benchmarks.md`, sections 3.1 (its large-effect
  bullet), 5.4 ("Committed"), 8.1, 8.2, 9 and 14.
- smith `docs/design/shell.md`, sections 5.2 ("Tiered list prices") and
  6.1.
- `findings/harness.md` T6, T11, T13; `findings/time.md` W8 and section 2;
  `findings/measure.md` M2 and M10; `findings/tokens.md` R7.
- skein `testing-strategy.md`, section 2.1; `programming-model.md`,
  section 10.2.

## What the design asks

- **The common priced spend** prices every agent's tokens on one basis:
  smith's presets' list prices at the harness's commit, which the summary
  names; the harness keeps no catalog of its own (8.1).
- **The time record:** the seconds of the main conversation (Codex's root
  thread) in model requests, with their first-byte and largest-gap sums;
  in each tool, by name; in compaction; waiting on children; the rest; and
  the seconds of commands that ran outside any call (8.1).
- **Summaries keep each attempt's** identity, end, judgment, task wall,
  common priced spend and token fields, so that a verdict recomputes
  from the file (5.4).
- **A repository task's large effect:** for a metric both arms observe,
  their observed ranges do not overlap; the summary computes it (3.1, 9).
- **Unknown is never zero** (8.1's `Measure`).

## Choices this plan makes

- The price function is `smith_local_shell::presets::list_prices(endpoint:
  &str, model: &str, window: u64) -> Option<Prices>`, the presets' table
  made public, priced by the model each attempt resolved and its
  declared window; the summary records the harness's commit as
  `priced_at`.
- The 42 committed summaries of version 2, and the 11 baselines cut from
  them, are dropped when the summary moves to version 3 (03.1): their raw results are of result version 2, which a
  version 3 reader refuses, so they cannot be regenerated (benchmarks.md
  8.1: "regenerated or dropped"). Git keeps them (decisions.md, 10.6).
- The reference arm of a comparison is the arm whose agent is Codex,
  never the arm that sorts first by name (findings M10); a suite with no
  Codex arm names its reference, `reference = "<arm>"` (06.2's `role`,
  12.2's `compaction`).

## Rules that bind it

- **Formats move once** (README, rule 12): the result and the summary
  move to 3 in 03.1; 03.2 to 03.5 change them under 3.
- **The smoke** (rule 8) on each landing.
- **Splits** (rule 1).

## Increments

### 03.1 The common priced spend from smith's presets
Scope: benchmarks.md 8.1's spend row and 9 (findings T6, R7).
Needs: nothing.
Files: `crates/smith-local-shell/src/presets.rs` (`prices`,
  `:109-121`), `crates/smith-local-shell/src/lib.rs`, `benchmarks/Cargo.toml`,
  `benchmarks/src/result.rs` (`RESULT_VERSION`, `Spend`),
  `benchmarks/src/summary.rs` (`priced_spend`, `:222-244`; the
  `spend_usd/{basis}` key, `:169`, `:189`), `benchmarks/src/models.rs`,
  `benchmarks/src/main.rs` (`:103-105`), `benchmarks/src/lib.rs`,
  `benchmarks/src/check.rs` (`:54`), `benchmarks/src/summary/compare.rs`,
  `benchmarks/src/tests/comparison.rs`, `benchmarks/agents/prices.toml`,
  `benchmarks/README.md` (`:60`, `:88`), every user of `prices.toml`
  that `rg -n 'prices.toml|PriceTable' benchmarks` lists; and for the
  summary's version, `benchmarks/src/summary.rs`'s literal versions
  (`:462`, `:478`, `:527`, `:541`, `:604`, `:613`, `:688`),
  `benchmarks/summaries/` (the 42 files of version 2) and
  `benchmarks/baselines/` (the 11 files).
Shape: `pub fn list_prices(endpoint: &str, model: &str, window: u64) ->
  Option<Prices>` in `presets.rs`, re-exported, with `Prices` holding
  fresh, cache-read, cache-write and output rates and their unit. `Spend`
  gains `priced: Measure<f64>`, the common priced spend, beside the
  agent's own figure and its basis, which it keeps. `RESULT_VERSION` = 3;
  `SUMMARY_VERSION` = 3 and `BASELINE_VERSION` = 1 as named constants,
  `committed_costs` refusing another version instead of skipping it; the
  summary records `priced_at`, the harness's commit. `summarise` takes no
  price table. The committed summaries and baselines go; `check` reads
  both directories empty.
Test shape: `benchmarks/tests/offline.rs` (or `summary.rs`'s tests):
  `the_summary_prices_tokens_from_the_product_presets` (a fixed token
  record priced against a literal for `codex/gpt-6-astra` at 272,000),
  `the_priced_basis_names_its_catalog_commit`,
  `an_agents_own_figure_is_kept_beside_the_priced_one`. Under 20 ms each.
  In `smith-local-shell`: `list_prices_switches_to_long_context_past_272000`.
Proof: unit, those tests.
Don't: read prices from an arm's `smith check` output, or keep any table
  of rates in `benchmarks/`.
Size: 150 lines.
Removes: `benchmarks/agents/prices.toml`, `PriceTable` and its reader and
  check; the 42 summaries and 11 baselines of version 2 (decisions.md,
  10.6).

### 03.2 A summary row per attempt
Scope: benchmarks.md 5.4 ("Committed") and 9 (findings M10).
Needs: 03.1 (the priced spend each row carries).
Files: `benchmarks/src/summary.rs` (`ArmSummary`, `:48-68`;
  `read_summary`, `:602-608`), `benchmarks/src/summary/compare.rs`
  (`:14-15`), `benchmarks/src/suite.rs` (`reference`), `benchmarks/tests/`.
Shape: `RunSummary` gains `attempts: Vec<AttemptRow>` (sketch below), one
  per attempt, in run order, under version 3; the reference arm of every
  comparison is the arm whose agent is Codex, and in a suite with no Codex
  arm the arm its file names, `reference = "<arm>"`, which `check` requires
  of such a suite.
Test shape: `a_summary_keeps_each_attempts_row`,
  `the_reference_arm_is_the_codex_arm_whatever_the_names`,
  `a_suite_without_codex_compares_against_its_named_reference`,
  `a_summary_of_another_version_is_refused_by_name`. Under 20 ms each.
Proof: unit, those tests; `heavy cargo run -p smith-bench -- check` exits
  0.
Don't: keep a reader of version 2.
Size: 120 lines.
Removes: the reference chosen by the arms' names.

### 03.3 The time record
Scope: benchmarks.md 8.1's timing row (findings W8, and time.md's section
  2 for what each figure means).
Needs: 03.2 (the summary's rows, beside which it gives its medians).
Files: `benchmarks/src/result.rs` (`Timing`, `:111-120`),
  `benchmarks/src/run.rs` (`:91-94`), `benchmarks/src/agent/smith/events/`
  (a new `time.rs`), `benchmarks/src/agent/codex/rollouts.rs` (`:137`),
  `benchmarks/src/agent/codex.rs`, `benchmarks/src/summary.rs` (medians
  per arm), `benchmarks/tests/smith_adapter.rs`,
  `benchmarks/tests/codex_adapter.rs`, `benchmarks/tests/recorded/`.
Shape: `TimeRecord` (sketch below) in each result, observed from smith's
  events (`response.completed`'s `first_byte_ms`, `largest_gap_ms` and
  `duration_ms`; `tool.*` spans by name; compaction conversations; child
  spans) and from Codex's root rollout with its timestamps (requests,
  calls, commands, and the command seconds outside any call); what a
  stream lacks is unavailable with its reason. `Timing`'s `first_byte_ms`,
  `largest_gap_ms` and `longest_completion_ms` are filled from the same
  records. The summary gives each figure's median per arm.
Test shape: `a_time_breakdown_sums_to_task_wall_within_one_second` (a
  recorded smith trace), `rollout_threads_give_request_tool_and_command_overlap`
  (rollouts trimmed from repository attempt 04 into
  `tests/recorded/codex/`), `a_stream_without_spans_leaves_the_record_unavailable`.
  Under 0.5 s each.
Proof: unit, those tests.
Don't: estimate a figure a stream does not carry.
Size: 300 lines.
Removes: the always-unavailable timing fields' constant.

### 03.4 Codex's linked children, counted
Scope: benchmarks.md 6's collect row for Codex, 8.1's counts and
  delegation (findings T11).
Needs: 03.1.
Files: `benchmarks/src/agent/codex.rs` (`:480-487`),
  `benchmarks/src/agent/codex/rollouts.rs` (`ChildThread`, `:57`),
  `benchmarks/tests/codex_adapter.rs`.
Shape: `counts.children` is observed: the threads whose parent chain
  reaches the root, by `source.subagent.thread_spawn.parent_thread_id`;
  `conversations` is observed; responses per thread come from each
  thread's `token_count` events. Child shares and parent-blocked time stay
  unavailable.
Test shape: `linked_codex_children_are_counted_from_their_rollouts`,
  `codex_responses_per_thread_come_from_token_counts`, on rollouts trimmed
  from repository attempt 04. Under 20 ms each.
Proof: unit, those tests.
Don't: count an unlinked thread.
Size: 80 lines.
Removes: the "unavailable" children count.

### 03.5 A repository run's large effects
Scope: benchmarks.md 3.1 and 9's large effect (findings T13).
Needs: 03.2.
Files: `benchmarks/src/stats.rs`, `benchmarks/src/summary.rs`,
  `benchmarks/tests/`.
Shape: `fn large_effect(reference: &[Measure<f64>], candidate:
  &[Measure<f64>]) -> LargeEffect` in `stats.rs`, `LargeEffect::{Separated
  { above: bool }, None, Unclaimed { reason: String }}`; the summary states
  one for each metric both arms of a repository task observe.
Test shape: `a_repository_summary_marks_disjoint_ranges_as_large_effects`,
  `overlapping_or_lower_bound_ranges_claim_no_effect`. Under 20 ms each.
Proof: unit, those tests.
Don't: infer significance from three attempts.
Size: 60 lines.
Removes: nothing.

## Interface sketches

```rust
// smith-bench, src/summary.rs (03.2): one attempt's row in a summary of version 3.
pub struct AttemptRow {
    pub attempt: String,
    pub task: String,
    pub arm: String,
    pub agent: String,
    pub end: String,
    pub passed: bool,
    pub task_wall_ms: Measure<u64>,
    pub priced_usd: Measure<f64>,
    pub fresh_tokens: Measure<u64>,
    pub cache_read_tokens: Measure<u64>,
    pub output_tokens: Measure<u64>,
}

// smith-bench, src/result.rs (03.3): the main conversation's seconds.
pub struct TimeRecord {
    pub requests_ms: Measure<u64>,
    pub first_byte_ms: Measure<u64>,
    pub largest_gap_ms: Measure<u64>,
    /// Per tool name, in the order of each tool's first call.
    pub tools_ms: Vec<(String, Measure<u64>)>,
    pub compaction_ms: Measure<u64>,
    pub children_ms: Measure<u64>,
    pub rest_ms: Measure<u64>,
    pub commands_outside_calls_ms: Measure<u64>,
}
```

`Measure` is the result's existing type. 04.4 reads `AttemptRow` for the
verdict, and owns nothing of it.

## Done when

- `rg -n 'prices.toml' benchmarks` prints nothing.
- `heavy cargo nextest run -p smith-bench` passes, once.
