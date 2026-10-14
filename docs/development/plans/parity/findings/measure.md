# Findings: measuring parity with Codex

2026-10-14. What "smith's performance on par with Codex's" can mean in `smith-bench`'s terms: the criteria, statistics, attempts and cost, and the one command a plan's "Done when" can name. Written by a read-only sub-agent for the parity plan's designer (skein's development.md, section 4.1).

Sources: smith `main` at `689c15d0`. The data are the committed summaries `benchmarks/summaries/compare-a57ca3a8-20261013T221135Z.json` (12.4) and `repository-a64f0523-20261013T230741Z.json` (12.5), and the raw `result.json` of their 36 attempts under `~/.local/state/smith-bench/runs/`. Scripts are in `/tmp/claude-measure/`. A jq extraction writes `attempts.tsv`. Then come `variation.py`, `sim.py` and its variants `sim8.py` and `simm.py`, `passrate.py`, `rules.py`, `repo3.py`, `current.py`, `joint.py` and `joint2.py`, and `cost.py`. Results are under `out/`, `joint/` and `joint2/`. All are seeded and use the standard library. Simulations use 1,000 experiments per cell (500 for the joint verdict) and 2,000 bootstrap resamples, so a probability carries about ±0.02. Sibling findings: `harness.md` (T1 to T14), cited where it already covers a defect.

## 1. In one page

- **No outcome verdict is possible today, even once harness.md's T1 and T2 are fixed.** Fixture and repository tasks declare no `deciding` checks, so every check decides (`benchmarks/src/judgment.rs:42-55`). Every one of the 36 attempts has checks that are always unavailable: `nothing-outside` and `benchmark-unread` for both agents, plus `teardown` and four `headroom/*` checks for smith. With `protected-unchanged` passing, each attempt judges `Incomplete`, and the pass interval becomes "some eligible attempt checks are unavailable" (`summary.rs:384-389`). See M1.
- **The basis is outcome, task wall time and common priced spend.** These are the only metrics both agents report on one basis (`summary/compare.rs:72-84`). Token fields compare too, but they are diagnostics. Codex's own spend, turns, children and parent-blocked time are unavailable, and cache writes are 0 for both (M2).
- **"None detected" cannot show parity.** Parity is non-inferiority: the upper bound of the 95% bootstrap interval of smith/Codex lies below a margin. At a true ratio equal to the margin, the simulated false-pass rate is 0.01 to 0.05 (M3).
- **Wall time varies little, spend more.** The pooled within-group log-scale spread over the fixtures is 0.089 for smith and 0.080 for Codex on task wall, and 0.116 and 0.200 on priced spend (section 3).
- **n=5 per task cannot show parity; pooling over the fixtures can.** Per task at n=5, P(upper bound < 1.20 | true ratio 1.0) is 0.57 to 0.99 for wall and 0.16 to 0.80 for spend. Pooled over the three fixtures, a stratified bootstrap reaches 0.82 (time per solved task) and 0.77 (cost per solved task) at n=8 per arm, with both agents solving 97% (M4, M6).
- **The repository task holds the gap.** smith's median task wall is 728.5 s against Codex's 421.8 s, a ratio of 1.727. Its fixtures already fit within 1.20: time per solved task is 1.073 [1.018, 1.128] and cost per solved task 0.855 [0.771, 0.945]. The repository suite runs `design = "single"` at n=3, so its summary holds no comparison at all (M8).
- **"Within Codex's exact interval" is a lenient outcome rule.** With Codex at 29/29, smith needs 26/29. A smith solving 80% still passes 0.36 of the time (M5). Measuring cost and time per solved task closes most of that gap (M6).
- **Caching is a diagnostic only.** smith reads 0.198 to 0.241 times Codex's cache volume and sends 0.25 to 0.38 times its total input, yet costs 0.69 to 0.99 times as much. Cache reads are already in the price (M7).
- **A proposed definition (section 5).** A `parity` suite of fixtures 01 to 03 at 8 attempts per arm and `shell-cwd` at 5, interleaved: 58 attempts, about 2.8 h, and $14.43 notional at `prices.toml`, of which smith's share is $6.06. It passes on five criteria. `smith-bench parity SUMMARY --commit SHA` exits 0, 1 or 2. Today it gives exit 0 with probability 0.00. Once the repository wall time is fixed, with today's fixture behaviour, it gives 0.90.
- **What the harness must add (M10).** Per-attempt values in the summary (version 3), a stratified per-solved bootstrap, an explicit Codex reference arm, and the `parity` command with its exit code.

## 2. What the harness computes today

| Statistic | Where | What it does |
|---|---|---|
| median | `benchmarks/src/stats.rs:75-86` | of every observation, whatever the attempt's end |
| CV | `stats.rs:89-107` | sample sd over mean; needs at least 2 values and a non-zero mean |
| MDE | `stats.rs:110-119` | 2.8 · CV · √(2/n); a comparison uses the larger CV of the two arms (`:181`) |
| bootstrap ratio | `stats.rs:152-190` | candidate's median over the reference's. It refuses fewer than 5 per arm, fewer than 1,000 resamples, or a zero in the reference (`:155-159`). Percentile bounds sit at sorted indices R·25/1000 and R·975/1000. `detected` needs the interval to exclude 1 and the effect to exceed the MDE (`:187`) |
| pass interval | `stats.rs:192-244` | Clopper–Pearson by bisection on the binomial tails; 0/5 gives [0, 0.5218], 0/3 gives [0, 0.7076] |
| comparisons | `summary/compare.rs:7-53`, called only for `Design::Interleaved` (`summary.rs:451-452`) | 10,000 resamples with the run's seed. Every attempt must have an observed value (`compare.rs:31-36`). Reference and candidate are the groups in `BTreeMap` order of (task, arm name) (`summary.rs:445`, `compare.rs:14-15`) |
| common priced spend | `summary.rs:222-244` | total fresh, cache-read and output tokens priced from `benchmarks/agents/prices.toml`; it replaces Codex's own unavailable `spend_usd/priced` (`summary.rs:169,189`) |
| attempt judgment | `judgment.rs:31-63` | `Pass`, `Fail` or `Incomplete`. Any unavailable deciding check makes it `Incomplete` |
| suite exit | `judgment.rs:67-90`, `main.rs:12-14` | 1 if any attempt fails, 2 if any is incomplete, else 0 |

Metrics in the 12.4 and 12.5 summaries (`.groups[].metrics`), observed out of n:

| Metric | smith | Codex | Compared across agents (`compare.rs:72-84`) |
|---|---|---|---|
| `task_wall_ms`, `process_wall_ms` | 5/5, 3/3 | 5/5, 3/3 | yes |
| `tokens/*/{fresh,cache_read,output,reasoning}` | 5/5, 3/3 | 5/5, 3/3 | yes, by field across conventions |
| `tokens/*/cache_write` | all 0 | all 0 | refused: "reference zero can make a bootstrap ratio undefined" |
| `spend_usd/priced` (common) | 5/5 | 5/5 | yes |
| `spend_usd/notional` (own) | 5/5 | absent; the raw value is "adapter has not supplied scoped usage and prices" (attempt 06 `.spend`) | no |
| `delegation/*`, `tool_calls/*` | observed | 0/5, 0/3 (lower bounds; harness.md T11) | no |
| `peak_rss_bytes/*` | 0/5 | 0/5 | no |

## 3. The variation measured

CV computed as the harness computes it (`.groups[].metrics[m].cv.value`), as smith / Codex. n=5 per arm for the fixtures, n=3 for `shell-cwd`.

| Metric | 01-backoff | 02-line-decoder | 03-planner | shell-cwd | Pooled log-sd, fixtures, df 12 |
|---|---|---|---|---|---|
| task wall | 0.101 / 0.100 | 0.096 / 0.081 | 0.056 / 0.052 | 0.051 / 0.068 | 0.089 / 0.080 |
| fresh | 0.229 / 0.719 | 0.385 / 0.279 | 0.203 / 0.219 | 0.236 / 0.146 | 0.306 / 0.444 |
| cache read | 0.115 / 0.091 | 0.221 / 0.242 | 0.256 / 0.121 | 0.140 / 0.191 | 0.241 / 0.155 |
| output | 0.111 / 0.083 | 0.085 / 0.124 | 0.062 / 0.019 | 0.095 / 0.121 | 0.088 / 0.086 |
| reasoning | 0.443 / 0.241 | 0.218 / 0.382 | 0.169 / 0.224 | 0.305 / 0.229 | 0.330 / 0.280 |
| common priced spend | 0.091 / 0.322 | 0.144 / 0.178 | 0.099 / 0.049 | 0.139 / 0.168 | 0.116 / 0.200 |

Medians, smith / Codex. Task wall: 50.107 / 49.878 s, 100.571 / 87.285 s, 92.206 / 83.179 s, 728.470 / 421.755 s. Spend: $0.0287 / $0.0384, $0.0641 / $0.0646, $0.0614 / $0.0629, $0.9658 / $1.4089. `variation.py` reproduces every summary median of priced spend.

- **A CV from five values is itself uncertain.** Its relative standard error is about 1/√(2(n−1)), which is 0.35 at n=5 and 0.20 for the pooled df of 12. Codex's 0.322 on 01's spend comes from one attempt: attempt 09 used 34,206 fresh tokens, where the median was 10,317. Planning therefore uses each task's own CVs, with lognormal draws. Resampling the five observed values instead is optimistic. For 01's and 02's task wall at n=5 it gives P(upper < 1.20) = 1.00, where lognormal draws give 0.57 and 0.70 (`sim.py obs`).
- **The 2026-10-08 benchmark is not pooled.** It used another harness, other models and n ≤ 3, and its summary holds only median, min and max. Its range over median, divided by 1.693 (the expected range of three normal draws in σ), agrees in scale: `01_backoff` Codex task wall 0.10 (stage `baseline`); `05_lease_queue` Codex 0.05, smith 0.11; `06_journal_replay` Codex 0.09, smith 0.36 (`smith-benchmark-2026-10-08/summary.json`, `task_wall_s`). Its pass counts were Codex 27/27 and smith 32/33 over its n=3 groups. On `07_shell_cwd` (stage `iteration7`), Codex passed 3/3 at a median of 412.8 s and smith 2/3 at 996.9 s.

## 4. Entries

### M1 The fixtures' and the repository's outcomes cannot be judged

- **Symptom / need:** every overall verdict in 12.4 and 12.5 fails on protected paths (harness.md T1, T2). A parity verdict needs pass counts that can be observed.
- **Analysis:** only probes declare `deciding` (`rg -l '^deciding' benchmarks/tasks`: 15 manifests, all under `probes/`). With an empty `deciding`, every check decides (`judgment.rs:46-55`). In all 18 Codex attempts, `nothing-outside` and `benchmark-unread` are unavailable. In all 18 smith attempts, `teardown`, `headroom/{request-bytes,first-byte,largest-gap,peak-rss}`, `nothing-outside` and `benchmark-unread` are too (raw `outcome.checks`). So once T1 and T2 are fixed, all 36 judge `Incomplete`. The pass interval then becomes unavailable (`summary.rs:388`), and the suite exits 2.
- **The clean fix:** fixture and repository version 2 (harness.md T1, T2) declare `deciding = ["protected-unchanged", "tree-ended"]`. The grade always decides (`judgment.rs:56-61`), and the end is judged first (`:35-41`). The rest stay recorded and audited. benchmarks.md 3.2 has "Passive checks run on every attempt (section 11.2); a task that expects a failure names those it waives." It gains: "A graded task decides on its end, its grader, its protected files and the tree's end. A passive check that cannot be observed for an agent is reported beside, and decides nothing." This touches 9's scope gate ("nothing written outside the workspace, no read of the benchmark tree"), which stays sampled by audits (question Q6).
- **Its tests** (`benchmarks/tests/outcomes.rs`): `graded_fixture_passes_when_scope_evidence_is_unavailable`, a recorded Codex attempt with `nothing-outside` unavailable judges `Pass`; `graded_fixture_fails_when_a_protected_file_changes`; `fixture_manifests_declare_grade_and_protected_as_deciding`.
- **Confidence:** high; code read and 36 of 36 raw results checked.

### M2 The cross-agent basis, and what a target can use

- **Symptom / need:** benchmarks.md 9 says "Only outcome, wall time, tokens by convention and spend on one basis are compared across agents".
- **Analysis:** the code compares task wall, process wall, common priced spend and token fields by name (`compare.rs:72-84`). Task wall ends at the observed answer, which makes it the agent-neutral time. Process wall adds teardown: 397 to 457 ms for Codex against 9 to 10 ms for smith (`teardown_ms` medians), so it favours smith by about 0.5 s. Cache writes are 0 for both on `codex`, so their comparison is refused. Token fields agree in meaning here: Codex's fresh is input less cached, and smith's is `input_tokens` plus cache writes, which are 0 (benchmarks.md 8.2). They will diverge on a provider that reports cache writes.
- **The clean fix:** parity's criteria use only the overall judgment, `task_wall_ms` and `spend_usd/priced`. Token fields, process wall, reasoning and cache volumes are reported diagnostics. No new metric is needed.
- **Its tests:** `parity_criteria_read_only_outcome_task_wall_and_common_priced_spend`.
- **Confidence:** high.

### M3 "Detected" is a difference test; parity is non-inferiority

- **Symptom / need:** benchmarks.md 9 says: "Otherwise the summary says none was detected, not that the arms are equal." Section 2 says: "Nor are they a ranking: a comparison states an effect with its interval, or that none was detected." Neither can serve as a "Done when".
- **Analysis:** the MDE term 2.8 = 1.96 + 0.84 gives 80% power for a two-sided test at 5%. At n=5, effects up to the MDE go unseen: 0.178 for 01's wall and 0.570 for its spend (`.comparisons[]`). Non-inferiority asks something else: is the upper 95% bound below a margin M? Its error rate at the margin was measured by simulating a true ratio equal to M=1.20. Per-task median wall (02's CVs) passes 0.01 to 0.02. The pooled geometric mean passes 0.01. Time per solved task passes 0.03 to 0.05, and cost per solved task 0.03 to 0.04. The percentile bootstrap at 5 to 8 attempts is slightly liberal against the nominal 0.025.
- **The clean fix:** benchmarks.md 9 gains a bullet: "**Parity** with another agent is non-inferiority: each of its criteria holds when the upper bound of the 95% bootstrap interval of smith's ratio to the other agent lies below that criterion's margin. Parity is a verdict of its own suite (11.5), never inferred from 'none detected'." The words quoted above stay true for comparisons.
- **Its tests** (`benchmarks/tests/parity.rs`): `non_inferiority_holds_only_below_the_margin`, with recorded arms whose upper bound is 1.19 and 1.21 at M=1.20; `parity_never_follows_from_none_detected`.
- **Confidence:** high for the rule. The rates come from the simulation.

### M4 Attempts per arm against the margin

- **Symptom / need:** how many attempts show a ratio below M when smith truly equals Codex.
- **Analysis:** P(upper bound < M | true ratio 1.0), from lognormal draws with the observed CVs (`sim.py`):

| Statistic (CVs, smith/Codex) | n per arm | M=1.10 | M=1.20 | M=1.25 |
|---|---|---|---|---|
| per task, median wall, 01 (0.101/0.100) | 5 / 8 / 10 / 15 / 30 | 0.17 / 0.36 / 0.39 / 0.52 / 0.84 | 0.57 / 0.85 / 0.91 / 0.98 / 1.00 | 0.74 / 0.95 / 0.98 / 1.00 / 1.00 |
| per task, median wall, 02 (0.096/0.081) | 5 / 8 / 10 / 15 / 30 | 0.23 / 0.38 / 0.48 / 0.61 / 0.91 | 0.70 / 0.92 / 0.97 / 0.99 / 1.00 | 0.87 / 0.98 / 0.99 / 1.00 / 1.00 |
| per task, median wall, 03 (0.056/0.052) | 5 / 8 / 10 | 0.59 / 0.81 / 0.89 | 0.99 / 1.00 / 1.00 | 1.00 |
| per task, median spend, 01 (0.091/0.322) | 5 / 10 / 20 / 30 | 0.07 / 0.14 / 0.18 / 0.25 | 0.16 / 0.32 / 0.52 / 0.70 | 0.22 / 0.44 / 0.69 / 0.85 |
| per task, median spend, 02 (0.144/0.178) | 5 / 10 / 20 / 30 | 0.09 / 0.16 / 0.30 / 0.45 | 0.25 / 0.55 / 0.82 / 0.94 | 0.37 / 0.73 / 0.95 / 0.99 |
| per task, median spend, 03 (0.099/0.049) | 5 / 8 / 10 | 0.34 / 0.54 / 0.62 | 0.80 / 0.96 / 0.99 | 0.91 / 0.99 / 1.00 |
| per task, median output, 01 / 02 / 03 | 8 | 0.36 / 0.34 / 0.91 | 0.86 / 0.83 / 1.00 | 0.95 / 0.95 / 1.00 |
| `shell-cwd`, median wall (0.051/0.068) | 5 / 8 / 10 | 0.49 / 0.73 / 0.83 | 0.97 / 1.00 / 1.00 | 0.99 / 1.00 / 1.00 |
| `shell-cwd`, median spend (0.139/0.168) | 5 / 10 / 30 | 0.09 / 0.18 / 0.47 | 0.28 / 0.59 / 0.96 | 0.43 / 0.76 / 0.99 |
| pooled geometric mean of median ratios, wall | 5 / 8 / 10 / 15 | 0.67 / 0.90 / 0.95 / 1.00 | 1.00 | 1.00 |
| pooled geometric mean of median ratios, spend | 5 / 8 / 10 / 15 | 0.16 / 0.33 / 0.43 / 0.50 | 0.59 / 0.88 / 0.94 / 0.99 | 0.77 / 0.97 / 0.99 / 1.00 |
| pooled time per solved task, both solve 97% | 5 / 8 / 10 / 15 | 0.61 / 0.51 / 0.56 / 0.67 | 0.69 / 0.82 / 0.91 / 0.97 | 0.79 / 0.91 / 0.96 / 0.99 |
| pooled cost per solved task, both solve 97% | 5 / 8 / 10 / 15 | 0.37 / 0.38 / 0.45 / 0.55 | 0.65 / 0.77 / 0.85 / 0.94 | 0.76 / 0.88 / 0.93 / 0.98 |
| pooled cost per solved task, both solve 100% | 5 / 10 / 15 | 0.48 / 0.72 / 0.87 | 0.95 / 1.00 / 1.00 | 0.98 / 1.00 / 1.00 |

  As a check, for medians, n ≈ 24.5 · CV² / ln(M)², with CV² averaged over both arms. That is 2 · 2.8² over the median's efficiency of 0.64. For 01's wall at M=1.20 it gives 7.4, against 0.85 simulated at n=8. For 01's spend it gives 41, against 0.70 simulated at n=30.
- **The clean fix:** criteria pool the fixtures with a stratified bootstrap (resampling within each task) and report each task's ratio beside. Per-task criteria at 5 to 10 attempts fail a truly equal smith on spend half the time.
- **Its tests:** `stratified_bootstrap_resamples_within_each_task`, where a two-task pool whose tasks differ tenfold in scale keeps each draw's task sizes; `parity_bootstrap_reproduces_from_the_summary_seed`.
- **Confidence:** medium-high. Lognormal draws with five-value CVs; ±0.02 simulation error.

### M5 Pass rates: "within Codex's interval"

- **Symptom / need:** an outcome criterion with Codex at or near 100%.
- **Analysis:** take "smith's observed rate is at least the lower bound of Codex's exact 95% interval" (`passrate.py`, mirroring `stats.rs:211`). If Codex passes everything, the lower bound is 0.025^(1/n). Codex 3/3 gives 0.2924, so smith needs 1/3. Then 5/5 gives 0.4782 (smith needs 3/5), 10/10 gives 0.6915 (7/10), 15/15 gives 0.7820 (12/15), 29/29 gives 0.8806 (26/29) and 30/30 gives 0.8843 (27/30). The rule is lenient. With Codex's true rate 0.97, P(smith passes the rule) by smith's true rate:

| Attempts per arm | smith 0.97 | 0.90 | 0.80 | 0.70 | 0.50 |
|---|---|---|---|---|---|
| 5 | 1.00 | 0.99 | 0.95 | 0.86 | 0.55 |
| 15 | 1.00 | 0.96 | 0.73 | 0.40 | 0.05 |
| 30 | 1.00 | 0.83 | 0.36 | 0.08 | 0.00 |

  "smith fails at most one more than Codex" at 30 attempts fails an equal smith (0.97) 0.12 of the time (`rules.py`). A Newcombe non-inferiority bound on the difference with δ=0.10 needs about 100 attempts per arm (0.93 power at 0.97/0.97). **Pooling is sound when each task has the same attempts per arm.** The pooled count is then Poisson-binomial with variance at most the binomial's, so the exact interval on the sum covers the equal-weight mean rate conservatively. With task rates (1.0, 0.95, 0.85) at 5 and 10 each, coverage is 0.9887 and 0.9900. With (1.0, 1.0, 0.7) it is 0.9976 and 0.9984 (`passrate.py`). When tasks have different attempt counts, the pool estimates the attempt-weighted rate.
- **The clean fix:** the outcome criterion pools every parity task, with equal attempts per arm on each task. Section 9's "Pass rates are counts with exact intervals" stands. The parity bullet adds: "smith's pooled pass count is at least the lower bound of the other agent's exact 95% interval times the attempts". Most of the strictness comes from M6.
- **Its tests:** `pooled_pass_threshold_is_26_of_29_when_codex_passes_29`; `parity_refuses_unequal_attempts_per_arm_on_a_task`.
- **Confidence:** high; exact arithmetic.

### M6 Cost and time per solved task

- **Symptom / need:** "every attempt counts" (benchmarks.md 9) without letting a fast failure look cheap.
- **Analysis:** define cost per solved task = Σ common priced spend over every attempt ÷ number of passes, and time per solved task the same with task wall. The ratio is smith's over Codex's, pooled over the fixtures. Its interval is a stratified bootstrap over attempts. A resample with zero smith passes gives ∞. A resample with zero Codex passes is counted as ∞ too, which is conservative. On today's attempts, with graded passes standing in for the overall verdict and 10,000 resamples at the 12.4 seed (`current.py`):
  - fixtures, time per solved: 1.073 [1.018, 1.128], smith 79.78 s against Codex 74.38 s;
  - fixtures, cost per solved: 0.855 [0.771, 0.945], $0.0509 against $0.0595;
  - fixtures, output per solved: 1.062 [1.001, 1.123];
  - `shell-cwd` (n=3, point values only): time per solved 2.653 (1091.9 s against 411.6 s, smith 2/3 graded) and cost per solved 0.965.

  The ratio couples outcome to cost. With both agents at 100% and n=10, P(time per solved < 1.20) = 1.00. With smith solving 90% against Codex's 100%, it is 0.29; at 85% against 97%, cost per solved < 1.20 holds 0.13 of the time.
- **The clean fix:** a `bootstrap_per_solved(strata, seed, resamples)` in `stats.rs`. It sums values and passes per arm over resampled strata, lets ∞ through instead of refusing it, and keeps the percentile indices of `bootstrap_ratio`. If smith has no passes, the criterion fails (exit 1). If Codex has no passes, the verdict is incomplete (exit 2): the task does not tell agents apart.
- **Its tests:** `per_solved_sums_every_attempt_including_failures`; `zero_smith_passes_fail_cost_per_solved`; `zero_codex_passes_make_the_parity_verdict_incomplete`; `a_resample_without_passes_counts_as_infinite`.
- **Confidence:** high for the definition; medium for the power figures.

### M7 Caching belongs in diagnostics

- **Symptom / need:** cache read volume is the one "detected" effect on every fixture: 0.198, 0.206 and 0.241 (`.comparisons[]`, `tokens/cache_read`).
- **Analysis:** smith sends far less input. Its median total input (fresh plus cache read) is 20,289, 34,143 and 43,677 tokens, against Codex's 80,116, 122,462 and 122,073. On `shell-cwd` it is 2.74 M against 7.19 M (`variation.py`, `total_input`). Its cached share is lower (0.652, 0.597 and 0.624 against 0.870, 0.902 and 0.926; `shell-cwd` 0.876 against 0.964), yet its priced spend ratio is 0.747, 0.992, 0.976 and 0.686. A cache read costs a tenth of a fresh token (`prices.toml:8-9`), so caching is already in the cost criterion. A share or volume criterion would reward a longer prompt. Cache reuse in smith's sense, a request reading its predecessor's prompt, is the `cache-reuse` probe's (benchmarks.md 11.1), a contract of smith's own.
- **The clean fix:** the parity verdict reports cache read, cached share and total input per arm as diagnostics, and none of them decides.
- **Its tests:** `cache_volume_and_share_are_reported_and_never_decide_parity`.
- **Confidence:** high.

### M8 The repository task: the gap, and no comparison

- **Symptom / need:** `shell-cwd` is where smith trails, and its suite cannot say by how much.
- **Analysis:** `repository.toml` has `design = "single"` and `repetitions = 3`. A single design runs blocks in a fixed order (`batch.rs:73-76`) and gets no comparisons (`summary.rs:451-452`). Three attempts are below the bootstrap's floor of five (`stats.rs:155`; benchmarks.md 9 says "three can never reach significance"). The 12.5 report judged it by disjoint ranges instead (`smith-codex-2026-10-14.md:146`). On task wall, the median ratio is 1.727 with disjoint ranges: smith 690.7 to 764.6 s, Codex 380.0 to 432.9 s. Simulated at n=5 per arm, P(upper < 1.20 | 1.0) = 0.97 for wall (CVs 0.051/0.068). For spend it is only 0.28 at M=1.20 and 0.43 at 1.25; at M=1.50 it is 0.90, and 0.02 when the true ratio is 1.5. At n=3 a point-ratio rule would pass wall with 1.00 at a true ratio of 1.0 and 0.08 at 1.3; for spend the figures are 0.89 and 0.29 (`repo3.py`). smith's spend ratio is 0.686 today. At a true 0.85, n=5 passes M=1.25 with 0.85.
- **The clean fix:** the parity suite runs `shell-cwd` interleaved at 5 per arm, judged on median task wall (M=1.20) and median priced spend (M=1.50, "large effects only"). Its passes join the pooled outcome. benchmarks.md 3.1's row "repository | the grader's pass, and large effects only | three attempts per agent" gains "five per agent, interleaved, in `parity`". 11.5 gains the `parity` row (section 5). Repository attempts run serially on the gate lane of `debian-16gb-hel1-1`, where the bundle lives (`benchmarks/README.md:130-138`).
- **Its tests:** `repository_parity_criteria_need_five_interleaved_attempts_per_arm`; `single_design_summaries_carry_no_parity_verdict`.
- **Confidence:** high on the gap; medium on spend power at n=5.

### M9 Which catalog prices the cost

- **Symptom / need:** a plan's live budget in dollars.
- **Analysis:** summaries price from `prices.toml`, which charges gpt-6-astra $1.25 per M fresh, $0.125 cache read and $10 output (`:7-10`). The compare pin gives smith the same rates (`agents/smith/first-request.json`: 10, 1 and 80 per 8,000,000). Main's preset for `codex/gpt-6-astra` at a window up to 272k charges $10, $1, $12.50 cache write and $50 (`crates/smith-local-shell/src/presets.rs:112`). The ratio is unaffected, since both agents carry one basis. The dollars are not: 58 parity attempts cost $14.43 at `prices.toml` and $104.47 at the preset (`cost.py`). harness.md T6 covers the catalogs' disagreement.
- **The clean fix:** parity's cost criterion uses `spend_usd/priced` only. The plan states its budget against the catalog the summary used, and the verdict prints that catalog's digest.
- **Its tests:** `common_priced_spend_ignores_smith_own_notional_prices`.
- **Confidence:** high.

### M10 The verdict and its command

- **Symptom / need:** a "Done when" names one command with an exit code.
- **Analysis:** today's suite exit fails the run on any failed attempt (`judgment.rs:74-75`), so one smith failure among 58 gives 1 even when parity holds. Committed summaries keep no per-attempt values. `MetricSummary` holds counts, median, range, CV and MDE (`summary.rs:21-31`), so no pooled or per-solved interval can be recomputed from a committed file. The reference arm is whichever arm name sorts first: "codex-standard-12eb3e81" before "smith-compare-…" (`summary.rs:445`, `compare.rs:14-15`). A `claude-…` arm would become the reference.
- **The clean fix:**
  - Summary version 3 keeps each attempt's id, task, arm, judgment, end, `task_wall_ms` and `spend_usd/priced`, plus the three diagnostic token fields.
  - `summarise` writes a `parity` section when the suite is `parity`. It holds each criterion's statistic, interval, margin and verdict, the reference (the arm whose agent is Codex, by agent), the seed and the price table's digest.
  - `smith-bench parity SUMMARY [--commit SHA]` recomputes that section offline from the per-attempt values. It refuses a section that disagrees, and refuses when the smith arm's `agent_version` is not `SHA`. It prints one line per criterion and exits 0 for parity, 1 for not on par, and 2 for incomplete. This is README's convention (`benchmarks/README.md:37-38`).
  - `check` validates committed parity sections.
- **Its tests** (`benchmarks/tests/parity.rs`): `parity_passes_when_every_criterion_is_below_its_margin` (exit 0); `parity_names_the_failing_criterion_and_exits_one`; `parity_reference_is_the_codex_arm_whatever_the_arm_names`; `parity_refuses_a_summary_of_another_smith_commit`; `an_unavailable_task_wall_makes_the_verdict_incomplete`; `a_setup_refusal_or_harness_error_makes_the_verdict_incomplete`; `timeouts_and_budget_ends_count_as_unsolved_with_their_wall`; `parity_section_recomputes_from_the_committed_attempts`.
- **Confidence:** high.

## 5. A proposed definition of parity

**smith is on par with Codex** at a commit when one run of the `parity` suite meets all five criteria. The run uses that commit's built arm against the pinned Codex (CLI 0.160.0, `standard` pin), both at the working tier (gpt-6-astra, high), interleaved in one session.

| # | Criterion | Statistic | Margin | P(pass) at n=8/5, both at ratio 1.0 and solving 97% |
|---|---|---|---|---|
| O | outcome, every task pooled (29 per arm) | smith's passes ≥ ⌈L · 29⌉, L the lower bound of Codex's exact 95% interval (26 when Codex passes 29) | — | 0.99 to 1.00 |
| FT | fixtures, time per solved task | Σ task wall ÷ passes, stratified bootstrap | upper < 1.20 | 0.85 |
| FC | fixtures, cost per solved task | Σ common priced spend ÷ passes, stratified bootstrap | upper < 1.20 | 0.81 |
| RT | `shell-cwd`, median task wall | bootstrap ratio of medians | upper < 1.20 | 0.97 |
| RC | `shell-cwd`, median common priced spend | bootstrap ratio of medians | upper < 1.50 | 0.91 |

- **Statistics:** smith over Codex. 10,000 resamples with the summary's seed, resampled within each task. Every attempt counts.
- **Suite `parity`:** fixtures 01, 02 and 03 at version 2 (M1; harness.md T1), 8 per arm. `shell-cwd` at version 2 (harness.md T2), 5 per arm. Design interleaved, `rerun_failed = 0`, serial on `debian-16gb-hel1-1`. It needs a per-task repetition count, which suites lack today (`batch.rs:64-69` lets only a task's own `repetitions` override the suite's). Alternatively it runs as two suites whose summaries `parity` reads together. smith's own limits sit above the hard deadline (benchmarks.md 6.1), which today's fixtures break: `[budget] seconds = 840` against `deadline_seconds = 900` (harness.md T5).
- **What the verdict does with each end:**
  - `Completed` with a failed grade or check, `Failed`, `Timeout` and `Budget` are unsolved. Their wall and spend stay in the sums.
  - `Refused(setup)` and `HarnessError` make the verdict incomplete (exit 2): neither is the agent's.
  - Any unavailable or lower-bound `task_wall_ms` or `spend_usd/priced` also makes it incomplete; unavailable never passes (Q7).
  - Codex with zero passes is incomplete; smith with zero passes is exit 1.
- **Expected outcome (`joint.py`, wall and spend correlated at r = 0.34 within an attempt):** P(exit 0) is
  - 0.69 when both agents equal each other and solve 97%; 0.86 when both solve 100%;
  - 0.90 for today's fixture ratios, with both agents solving every fixture, and `shell-cwd` at wall 1.0, spend 0.686 and smith solving 97%;
  - 0.00 for smith 1.2 times slower everywhere, and 0.10 for smith solving 85%;
  - 0.00 today (RT is 0.00).

  With the fixtures at M=1.25: 0.80, 0.90, 0.99, 0.02, 0.17 and 0.00. At n=10 per fixture (70 attempts) with M=1.20, the first figure is 0.75.
- **Live cost of one run:** 58 attempts. Agent wall is 2.63 h, from the medians; measured overheads are 2.4 s per fixture attempt (2,391 s against 2,319 s) and 55 s per repository attempt (3,752 s against 3,420 s), which gives about 2.8 h. Notional cost is smith $6.06 and Codex $8.37, together $14.43, at `prices.toml` (M9). Per attempt, smith's medians are $0.0287, $0.0641, $0.0614 and $0.9658, and Codex's $0.0384, $0.0646, $0.0629 and $1.4089.
- **The command:** `cargo run -p smith-bench -- parity benchmarks/summaries/parity-<run>.json --commit <sha>`, exit 0. A plan's "Done when" reads: "a committed `parity` summary of a run on the plan's final `main`, for which this command exits 0".
- **Whether n=5 and n=3 can show parity:** not jointly. On the fixtures at n=5, a truly equal smith passes FT and FC only 0.69 of the time each. At n=3, `shell-cwd` has no interval in the harness. Even if the bootstrap ran at n=3, which the harness refuses, all five criteria at those counts pass a truly equal smith only 0.20 to 0.29 of the time (`joint.py`, n=5/3, repository-cost margin 1.20 or 1.25). The minimal credible design is 58 attempts. Two runs, one at the plan's start on the version-2 tasks (a baseline and a validation) and one for "Done when", come to 116 attempts and about $29 notional, within the 60 to 120 budget. One full run plus `shell-cwd`-only checks of 10 attempts each comes to 78 to 98.

## 6. Questions for the designer

1. **Margin.** Options: 1.10, 1.20 or 1.25. **Recommend 1.20** on time and on fixture cost, and 1.50 on `shell-cwd`'s cost. 1.10 needs about 30 attempts per task for wall alone. 1.25 lets a smith 1.2 times slower through 0.02 of the time, against 0.00 at 1.20.
2. **Per solved task or median over all attempts, on the fixtures.** **Recommend per solved task.** It is what a person pays to get a task done. It fails a smith solving 90% against Codex's 100% (wall passes 0.29 at n=10). Pooled medians never see a failure: at n=8 they keep 1.00 on wall and 0.88 on spend whatever smith solves. They lean on criterion O alone, which passes an 80% solver 0.36 of the time.
3. **Pooled or per fixture.** **Recommend pooled, with each fixture's ratio reported.** Per fixture at n=8, spend passes 0.28 to 0.96. If masking is a concern, add a guard that no fixture's point ratio exceeds 1.25. At n=8 the point ratio's spread makes that guard nearly free.
4. **Outcome rule.** Options: within Codex's exact interval; at most N more failures than Codex; Newcombe δ=0.10. **Recommend within the interval, pooled.** It is exact, never fails an equal smith (1.00), and FT and FC tighten it. Newcombe needs about 100 attempts per arm.
5. **`shell-cwd` attempts.** Options: 3 (point ratio, no interval), 5 or 10. **Recommend 5 interleaved.** 10 gives cost 0.76 at 1.25 but doubles the repository's $11.90 and 1.6 h.
6. **Unavailable scope checks** (`nothing-outside`, `benchmark-unread`). Options: deciding (every attempt is incomplete, M1) or advisory and audited. **Recommend advisory** until contained trees exist (benchmarks.md 10). The audits already read every disagreement.
7. **Lower bounds on Codex's side.** A lower-bound Codex total makes smith's ratio an upper bound. Options: admit it (conservative) or make the verdict incomplete. **Recommend admitting it on the reference side only, and flagging it.** smith's lower bounds stay incomplete.
8. **Price catalog.** **Recommend `prices.toml`, as summaries price today, named in the plan.** Whether it should follow main's presets is harness.md T6's question.
9. **Codex moving.** Parity is against a pinned Codex. **Recommend holding the pin for the plan's length**, and re-pinning only by commit with a fresh baseline.
10. **The command's shape.** Options: offline `smith-bench parity SUMMARY` or live `smith-bench suite parity`. **Recommend offline.** The verdict reproduces from a committed file, and the live run stays `suite parity` followed by `summarise`.
11. **Output or reasoning tokens as criteria.** smith's reasoning is 2.27 to 2.51 times Codex's on 02 and 03 (detected). **Recommend diagnostics only.** They are inside the cost.
12. **The plan's live budget.** **Recommend two full runs**, at the start and for "Done when": 116 attempts and about $29 notional (prices.toml). Repository-only progress checks of 10 attempts each come out of the same 120.
