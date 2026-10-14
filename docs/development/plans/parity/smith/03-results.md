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
  that `rg -n 'prices.toml|PriceTable' benchmarks` lists; for the
  resolved model and window, `benchmarks/src/result.rs` (`ModelIdentity`,
  `:75-80`), `benchmarks/src/probe.rs` (`:226-233`),
  `benchmarks/src/agent/smith/events.rs`, `benchmarks/src/agent/codex.rs`,
  `benchmarks/src/agent/codex/rollouts.rs`,
  `benchmarks/tests/recorded/results/attempt.json`, and every user that
  `rg -n 'ModelIdentity|resolved_model' benchmarks` lists; and for the
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
  both directories empty. `ModelIdentity` gains `resolved_window:
  Measure<u64>`, set `Measure::unavailable("not run")` where
  `resolved_model` is today. The smith adapter fills `resolved_model` and
  `resolved_window` from the main conversation's first `Model` record
  (`model`, `window_tokens`); the Codex adapter from the root rollout's
  first `turn_context` (`model`) and first `token_count`
  (`info.model_context_window`); each is unavailable, with its reason,
  where its stream lacks it. `priced` prices the attempt's tokens with
  `list_prices` at the resolved model and window; it is unavailable,
  naming what is missing, when either is unavailable or the presets do
  not price the model, and is never priced from the requested model or a
  preset's window. `resolved_effort` stays as it is.
Test shape: `benchmarks/tests/offline.rs` (or `summary.rs`'s tests):
  `the_summary_prices_tokens_from_the_product_presets` (a fixed token
  record priced against a literal for `codex/gpt-6-astra` at 272,000),
  `the_priced_basis_names_its_catalog_commit`,
  `an_agents_own_figure_is_kept_beside_the_priced_one`. Under 20 ms each.
  `the_resolved_window_comes_from_each_agents_stream` (a recorded smith
  trace, and a recorded Codex rollout's 258,400),
  `an_unavailable_window_leaves_the_priced_spend_unavailable`.
  In `smith-local-shell`: `list_prices_switches_to_long_context_past_272000`.
Proof: unit, those tests.
Don't: read prices from an arm's `smith check` output, or keep any table
  of rates in `benchmarks/`.
Size: 230 lines.
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

### 03.s1 A retried failure's tokens
Scope: benchmarks.md 5.3, 8.2 ("A retried failure") and 16.3, as the
  designer's answer to the question of 2026-10-14T22:34:35Z amended
  them.
Needs: nothing (03.1 and 04.4 landed).
Files: `benchmarks/src/agent/smith/events/usage.rs` (`run_tokens`,
  `:29`; its `ledger.record`, `:79`), `benchmarks/src/tokens.rs`
  (`TokenLedger::record`, `:249`), `benchmarks/src/batch.rs`
  (`known_tokens`, `:124`), `benchmarks/src/result.rs` (`priced_spend`,
  `:315`; its lower-bound arm, `:342`), `benchmarks/src/parity.rs`
  (`measured`, `:73`), `benchmarks/tests/smith_adapter.rs`,
  `benchmarks/tests/parity.rs`, `benchmarks/tests/recorded/` (one
  excerpt).
Shape:
  - A retried failure, in `run_tokens`: a `response.completed` with
    `outcome` `failed`, `blocks` 0, a failure whose `evidence` is
    `response` or `unsent`, and all five usage counts null, with a later
    `response.started` of its conversation and `turn` at a higher
    `attempt`. The test is 04.s2's for `prompt_within_window`, plus
    `evidence` and the null counts. Its ledger entry is every count
    unavailable with the reason `retried failure`, so that the total's
    lower bound names it as `<scope>: <response>: retried failure`. Every
    other response is recorded as today.
  - `tokens::reported(&Measure<u64>) -> Option<u64>`: an observed value;
    a lower bound whose every `missing` reason ends with `: retried
    failure`; `None` otherwise.
  - `known_tokens` reads the total's fresh, cache-read and output counts
    through `reported`. A total that misses only retried failures is
    charged its lower bound, and the stop at `:533-537` fires only for
    another gap.
  - `priced_spend`: when each of the four counts it prices is
    `reported`, and one is a lower bound, it prices those values and
    returns `LowerBound` with their `missing` reasons, deduplicated. Any
    other lower bound stays unavailable, as today.
  - `measured`, for cost: a smith row's `LowerBound` whose every reason
    ends with `: retried failure` is read as its value. Task wall, and
    Codex's rows, as today. The row keeps its lower bound: that is the
    flag (16.3).
Test shape: `smith_adapter.rs`:
  `a_retried_failure_leaves_a_lower_bound_that_names_it` (an excerpt of
  `parity-17d63afd-20261014T222044Z` attempt 02, responses 1 to 7: total
  fresh a lower bound of 8783 whose one reason ends `response-3: retried
  failure`; the priced spend a lower bound),
  `a_failure_the_provider_never_answered_is_not_a_retried_failure` (that
  excerpt, response 3's `evidence` `maybe_sent`: the reasons as today,
  the priced spend unavailable). `batch.rs`'s tests:
  `a_total_missing_only_retried_failures_charges_its_lower_bound` (a
  literal), `a_total_missing_another_count_charges_nothing_known` (`None`).
  `parity.rs`: `a_smith_spend_missing_only_retried_failures_counts_as_reported`.
  `missing_reference_passes_and_smith_lower_bounds_are_incomplete` keeps
  its expectation. Under 20 ms each.
Proof: unit, those tests; the smoke (rule 8). The live proof is 12.1's
  run.
Don't: write a zero for a retried failure, or leave it out of the
  ledger; change the 4xx rule (`rejected_before_output`), Codex's
  reading, or a format's version (rule 12).
Size: 80 lines.
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
