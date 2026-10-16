# Session 01: the harness's measures

2026-10-16. The next verdict must measure what the providers bill and
what the design now reads beside it. Today the harness prices an
attempt's whole token record at the tier its declared window reaches,
so every Codex token is priced at long-context rates (findings
`causes.md`, "The price basis"); a summary row carries neither the
attempt's responses nor smith's own peak memory; and `parity` runs five
`shell-cwd` attempts per arm, at which a smith as fast as Codex fails RT
about two runs in three (`rt-power.md`). This session moves the result
and the summary to version 4. It changes `benchmarks/` only.

## Read first

- smith `docs/design/benchmarks.md`, sections 3.1 (the parity bullet),
  5.4 ("Committed"), 8.1 (the resources and spend rows), 8.2, 11.5 (the
  `parity` row), 14 ("Pricing"), 16.1 to 16.4.
- smith `docs/design/shell.md`, section 5.2 ("The tiers are a
  prompt's").
- `decisions.md`, sections 3 and 5; `findings/causes.md`, "The price
  basis"; `findings/rt-power.md`; `findings/planning-audit.md` A17 to A21.
- skein `testing-strategy.md`, section 2.1; `programming-model.md`,
  section 10.2.

## What the design asks

- **Spend priced as billed:** each response's tokens at the list rates
  of the tier its own prompt reaches, from smith's presets' price list;
  the declared window plays no part. The adapters read usage per
  response: smith's `response.completed`, Codex's usage per model
  response. Without a response's usage, the common priced spend is
  unavailable, or a lower bound for a retried failure (8.1, 8.2).
- **Rows carry responses and the agent's own peak resident memory,**
  beside time and cost, so that a verdict recomputes from the file and
  reports them beside (5.4, 16.1). smith's own is its `session.ended`'s
  `peak_rss_bytes` (8.1's resources row).
- **`parity` runs ten `shell-cwd` attempts per arm:** 68 attempts; O
  pools 34 per arm, and holds at 31 when Codex passes 34 (3.1, 11.5,
  16.1, 16.2).

## Choices this plan makes

- A response's prompt is its fresh input plus its cache reads and cache
  writes, in the convention `tokens.rs:119-147` already applies; it is
  priced at the model its conversation resolved (smith: the
  conversation's `conversation.opened`; Codex: the rollout's
  `turn_context`), so a child on another model is priced at its own.
- The 15 committed summaries of version 3 and the two baselines cut from
  them are dropped when the summary moves to 4 (01.1): a version 4
  reader refuses them, and their spend is on the old basis. Git keeps
  them (decisions.md, 12.2). 11's last `parity` run cuts the new
  baselines.
- A response that failed unsent (its failure's evidence `unsent`)
  reached no provider: its usage is zero, observed, and it is priced at
  nothing, whether smith retried it or compacted after it; a retried
  failure is one whose evidence is `response` (benchmarks.md 8.2, as the
  designer answered decisions.md 12.4's question 7).
- Codex's peak memory in a row is the harness's count over its process
  tree, with the scope `tree`; smith's is its own, with the scope
  `process`. benchmarks.md 8.1 asks for each with its scope; the
  harness does not see Codex's process alone (decisions.md, 12.3).
- `suite` takes `--attempts <n>`, every task's attempts per arm, for the
  one-attempt run that proves a changed pipeline before a full run
  (development.md 5.5). A run below its suite's attempts is summarised
  into its own run directory, never under `benchmarks/summaries/`, its
  verdict incomplete (exit 2).

## Rules that bind it

- **Formats move once** (README, rule 14): the result and the summary
  move to 4 in 01.1; 01.2 and 01.3 change them under 4.
- **The smoke** (README, rule 8) on each landing.
- **Splits** (README, rule 1).

## Increments

### 01.1 Each response priced at its own prompt's tier
Scope: benchmarks.md 8.1's spend row, 8.2 and 14's pricing tests;
  shell.md 5.2 (findings A17, A18).
Needs: nothing.
Files: `benchmarks/src/tokens.rs` (the ledger's per-response records,
  `:221-226`, `:287-288`; its sums, `:294-335`), `benchmarks/src/result.rs`
  (`RESULT_VERSION`, `:14`; `priced_spend`, `:315-381`),
  `benchmarks/src/run.rs` (`:579`, `:676`),
  `benchmarks/src/agent/smith/events/usage.rs` (`run_tokens`, `:74-98`;
  retried failures, `:118-144`), `benchmarks/src/agent/codex/rollouts.rs`
  (`parse_rollout`, `:156-175`; `token_records`, `:300-327`),
  `benchmarks/src/summary.rs` (`SUMMARY_VERSION`, `:10`, no other
  line),
  `benchmarks/src/check.rs`, `benchmarks/src/tests/comparison.rs`
  (`:116`, `:144`), `benchmarks/src/tests/batch.rs` (`:583`),
  `benchmarks/tests/codex_adapter.rs` (`repository_threads`, `:593-618`),
  `benchmarks/tests/recorded/results/attempt.json` and
  `benchmarks/tests/recorded/parity/on-par.json` (their `"version": 3`);
  `benchmarks/summaries/`
  (15 files), `benchmarks/baselines/` (2 files); `benchmarks/README.md`
  (its pricing paragraph).
Shape: the ledger keeps each response's record and gives it out:
  `pub struct ResponseTokens { scope: Scope, model: String, fresh: u64,
  cache_read: u64, cache_write: u64, output: u64 }` with `fn
  prompt_tokens(&self) -> u64` (fresh plus cache reads plus cache
  writes), and `Ledger::responses(&self) -> &[ResponseTokens]`.
  `priced_spend` sums, over the attempt's responses, each priced with
  `list_prices(endpoint, &response.model, response.prompt_tokens())`; it
  takes no window. It is unavailable, naming the response, when a
  response has no usage or the presets do not price its model; a spend
  that misses only retried failures, those whose evidence is
  `response`, stays the lower bound 8.2 names; a response that failed
  unsent is priced at nothing, observed, retried or not, and leaves no
  gap (`usage.rs:118-144`'s rule changes with it).
  The smith adapter fills `model` from the response's conversation's
  `conversation.opened`; the Codex adapter from each rollout's
  `turn_context`, for each `token_usage_record`. `ModelIdentity`'s
  `resolved_window` stays, recorded and priced by nothing.
  `RESULT_VERSION` = 4, `SUMMARY_VERSION` = 4, `BASELINE_VERSION` stays
  1. The 15 summaries and 2 baselines go; `smith-bench check` reads both
  directories empty.
Test shape: `benchmarks/src/tests/comparison.rs`, beside
  `the_summary_prices_tokens_from_the_product_presets` (`:116`), which
  is the reference test to copy:
  `each_response_is_priced_at_its_own_prompts_tier` (two responses on
  `codex`/`gpt-6.1-sol`, prompts of 100,000 and 300,000 tokens: the sum
  at base and long-context rates as a literal);
  `a_declared_window_past_the_tier_prices_short_prompts_at_base_rates`
  (window 475,000, prompts under 272,000: base rates, a literal);
  `a_response_without_usage_leaves_the_spend_unavailable`, which
  replaces `an_unavailable_window_leaves_the_priced_spend_unavailable`
  (`:144`); `a_child_on_another_model_is_priced_at_its_own`;
  `a_request_refused_unsent_is_priced_at_nothing`,
  `a_retried_unsent_failure_leaves_no_gap`. In
  `benchmarks/tests/codex_adapter.rs`:
  `codex_responses_are_priced_from_their_usage_records`, on
  `repository_threads()`. Under 20 ms each.
Proof: process, the smoke (README, rule 8), summarised once, not
  committed: `heavy cargo run -q -p smith-bench -- summarise <the
  probe's run directory> smoke working single <its seed>`, then `jq -c
  '.attempts[] | {task, priced_usd}' <that summary>`; the line goes in
  the commit body, and nothing else is wanted.
Don't: price from `resolved_window`, or keep any table of rates in
  `benchmarks/`.
Size: 150 lines.
Removes: `an_unavailable_window_leaves_the_priced_spend_unavailable`;
  the 15 summaries and 2 baselines of version 3.

### 01.2 Responses and the agent's own peak memory in each row
Scope: benchmarks.md 5.4 ("Committed"), 8.1's resources row, 16.1's
  "reported beside" (findings A19, A20).
Needs: 01.1 (the summary at version 4).
Files: `benchmarks/src/result.rs` (`Counts.responses`, `:160`;
  `Resources`, `:287-292`), `benchmarks/src/run.rs` (`:251-283`),
  `benchmarks/src/agent/smith/events.rs` (`:226`),
  `benchmarks/src/agent/smith/events/passive.rs` (`:171-187`),
  `benchmarks/src/summary.rs` (`AttemptRow`, `:95-107`, built at
  `:109-129`; `metric_values`, `:193-285`; the parity section's beside
  values), `benchmarks/tests/recorded/` (the streams
  `retried-failure-17d63afd-02.jsonl`, `compaction-threshold-8a5bb9ec-03.jsonl`,
  `smith-time.jsonl`).
Shape: `Resources` gains `own_peak_rss_bytes: Measure<u64>`: smith's
  from `session.ended`'s `peak_rss_bytes.self_bytes`, unavailable with
  its reason where the record is null; Codex's unavailable("not
  reported"). `AttemptRow` gains `responses: Measure<u64>` (from
  `Counts.responses`), `peak_rss_bytes: Measure<u64>` and
  `peak_rss_scope: PeakScope` (`Process` from `own_peak_rss_bytes` where
  available, else `Tree` from `Resources.peak_rss_bytes`). Each arm's
  metric values gain the medians of both; the parity section reports
  them beside, deciding nothing.
Test shape: `benchmarks/tests/` (beside the summary's row tests):
  `a_row_keeps_the_attempts_responses`,
  `smiths_own_peak_comes_from_session_ended` (the recorded
  `retried-failure-17d63afd-02.jsonl`, its `self_bytes` as a literal),
  `codexs_peak_is_its_trees_and_says_so`,
  `a_null_peak_record_leaves_smiths_own_unavailable`. Under 20 ms each.
Proof: process, the smoke, summarised as 01.1's Proof says: `jq -c
  '.attempts[] | {task, responses, peak_rss_bytes, peak_rss_scope}'
  <that summary>` shows the scope `process`; the line goes in the commit
  body.
Don't: read Codex's memory from anything but the harness's own count.
Size: 120 lines.
Removes: nothing.

### 01.3 Ten `shell-cwd` attempts, O over each arm's attempts
Scope: benchmarks.md 3.1, 11.5, 16.1 and 16.2; development.md 5.5's
  small run (findings A21).
Needs: 01.1 and 01.2 (the pipeline this run proves).
Files: `benchmarks/suites/parity.toml` (`:12`), `benchmarks/src/parity.rs`
  (`:150-151`, `:179`, `:196`), `benchmarks/src/probe.rs` (the `suite`
  options, `:526`), `benchmarks/src/suite.rs` (`:94-101`),
  `benchmarks/src/batch.rs` (`:64-67`), `benchmarks/tests/parity.rs`
  (`:28`, `:136`), `benchmarks/tests/common/parity.rs`,
  `benchmarks/tests/recorded/parity/on-par.json`, `benchmarks/README.md`
  (`:180-181`).
Shape: `"repository/shell-cwd" = 10` in `parity.toml`. `parity.rs`
  requires 68 attempts, eight per arm on each fixture and ten on
  `shell-cwd`, and O's bound is the lower bound of the reference arm's
  `pass_interval` (`parity.rs:177`, the exact 95% interval) times that
  arm's attempts (34), not the literal 29. `suite` accepts `--attempts
  <n>`, every task's attempts per arm; a run below its suite's attempts
  writes its summary into its run directory as `summary.json`, prints
  that path and its verdict, and exits 2. `on-par.json` holds 68
  rows.
Test shape: `benchmarks/tests/parity.rs`, beside
  `a_ratio_just_below_its_margin_holds_and_just_above_fails` (`:142`):
  `o_holds_at_thirty_one_when_codex_passes_thirty_four`,
  `o_fails_at_thirty_when_codex_passes_thirty_four`,
  `a_parity_run_of_five_shell_cwd_attempts_is_incomplete`,
  `a_suite_run_below_its_attempts_is_summarised_as_small`. Under 20 ms
  each.
Proof: live, once: `heavy cargo run -p smith-bench -- build HEAD`, then
  `heavy --gate cargo run -p smith-bench -- suite parity --arm <dir>
  --attempts 1`: 8 attempts, about 40 minutes, to a verdict of 2
  (incomplete, as one attempt per task is). Its exit, `jq -c
  '.attempts[] | {task, arm, priced_usd, responses, peak_rss_scope}'
  <the summary path it printed>`, and its `Live:` trailer go in the
  commit body; nothing else is wanted.
Don't: change a criterion, a margin, or the fixtures' eight attempts.
Size: 70 lines.
Removes: the literal 29 in `parity.rs`.

## Questions to raise when reached

- None known.

## Done when

- `cd ~/src/rust/smith && rg -n '"repository/shell-cwd" = 10' benchmarks/suites/parity.toml` prints one line.
- `cd ~/src/rust/smith && rg -n '29\.0' benchmarks/src/parity.rs` prints nothing.
- `cd ~/src/rust/smith && heavy cargo run -q -p smith-bench -- check` exits 0.
