# Findings: the spike's benchmark harness and method, as input to `./benchmarks` (prefix B)

`ARCHIVE/` = `/home/free/src/rust/smith-benchmark-2026-10-08/`. smith is at `bench/smith-codex-2026-10-08` (`bf96e94`); skein is at main.

## Summary

- **What happened.** Spike 2 compared smith with vanilla Codex using about 2,000 lines of Python outside the repository. It ran 76 attempts (42 in the scored matrix) over 3.3 agent-hours. The artifacts take 57 GB, 56 GB of it in workspaces. smith spent 15.8 M fresh input tokens, and 93 % of them went to the seven runs of Task07.
- **What it found.** Every smith defect it found was a runtime capacity or lifecycle defect: waiting at startup, 7 s of teardown, 2 KiB tool arguments, a 60 s completion deadline, a 256 KiB request envelope, 64 shared turns, no child quota, and lost terminal facts. A probe of 30 seconds or less on the smallest model would show each one, or a passive check on any run would. Once they were fixed, fixtures 01–06 passed 100 % for both agents and stopped telling them apart. The real signals were binary or very large effects (teardown 7.002 s → 5 ms, pass/fail, 12× fresh input). n=3 cannot separate the latency medians.
- **Where the harness effort went.** Most of it worked around observability.
  - smith: regexes over the Rust `Debug` strings in the trace, scraping stdout and stderr, closing stdin once acceptance was seen, a registry keyed by binary digest, and converting a Codex access token to the SKOT format.
  - Codex: usage reported for the root thread only, and reading the private SQLite files in `~/.codex`.
  - The host leaked into results: the user's Codex config and project trust, a shared `/tmp`, read-only hard-linked Cargo artifacts, and 84 worktrees registered in the developer's smith repo.
- **The clean shape.**
  - A Rust `benchmarks/` crate in the workspace, written as an ordinary-Rust test harness. It reads a typed, versioned smith trace and summary through types shared with smith.
  - A closed enum of agent adapters. Each runs from a pinned config in an isolated home with dedicated credentials.
  - Tasks are plain directories with TOML manifests, in three tiers: probes (gates, run per fix), fixtures (trends and comparison, interleaved A/B, n ≥ 5) and a rarely run repository tier.
  - Raw results stay outside the tree; summaries and baselines are committed. Parser tests run in the focused suite. Live runs never run at the gate.

## 1. Task inventory

Medians come from the scored main matrix, three attempts per agent, effort high, recomputed from `ARCHIVE/results.csv`. Values are smith / Codex; "fresh" is uncached input.

| Task | Asks | Seed | Grader (determinism, time) | Behaviours | Model | Wall s median (range) | Fresh / cached / out | Responses / calls | Pass, all attempts |
|---|---|---|---|---|---|---|---|---|---|
| smoke | read README, report with a marker (Codex: shell probe) | 1 file | none | startup, one read, finish, teardown | luna/low | smith 13.2 (7 s teardown); Codex 4.0 | smith 4,670 / 0 / 107; Codex 4,731 / 25,088 / 107 | smith 4 completions (2 wasted by the startup bug) | — |
| 01 backoff | overflow-safe delay/deadline | 32 Rust lines | rustc-linked external regression + 5 hidden, `cargo test`, fmt; deterministic; 0.8 s | arithmetic edge cases | luna | 22.0 (21.6–27.0) / 23.6 (22.0–26.3) | 14.7k / 0 / 1.8k vs 13.4k / 97.5k / 1.9k | 8 / 12 vs 7 / 6 | 9/9, 9/9 |
| 02 line decoder | incremental decoder, bounded buffer | 59 | 7 hidden, 128 fixed-seed oracles; 1.0 s | **large writes (> 2 KiB arguments)** | luna | 36.5 (29.6–51.1) / 50.0 (41.3–50.5) | 31.2k / 8.2k / 3.0k vs 19.8k / 148k / 4.5k | 11 / 15 vs 9 / 8 | smith 3/5 + 0/2 diagnostic (all failures were the 2 KiB defect); 5/5 |
| 03 planner | closure planner, Kahn with lexicographic order | 60 | 8 hidden + DAG oracle (labels correlate with order); 1.2 s | multi-file edits, error priority | sol | 43.8 / 51.9 | 30.7k / 6.7k / 3.3k vs 11.7k / 155k / 4.6k | 10 / 17 vs 9 / 8 | 3/3 (+6/6 cache diagnostics); 3/3 |
| 04 transactional edits | atomic UTF-8 multi-document patches | 108 | 9 hidden (96 seeds give only 12 patterns); 1.1 s | careful edits | 6.1-sol | 98.0 / 96.6 | 30.7k / 0 / 4.2k vs 18.2k / 96.1k / 4.4k | 9 / 15 vs 6 / 5 | 3/3, 3/3 |
| 05 lease queue | lease/retry state machine | 158 | 8 hidden + 38,400-command model; 1.1 s | slow model, **optional delegation** | astra | 121.9 / 117.5 | 39.0k / 0 / 5.2k vs 20.6k / 95.5k / 5.5k | 8 / 14 vs 6 / 5 | 4/4 (one at 447.6 s); 4/4 |
| 06 journal (holdout) | checksummed journal, sticky errors | 127 | 10 hidden, every 2-chunk split; 1.2 s | framing, optional delegation (66.3 s child response) | astra | 128.8 / 142.4 | 74.8k / 10.4k / 5.6k vs 28.1k / 91.9k / 6.2k | 13 / 18 vs 6 / 5 | 3/3, 3/3 |
| 07 shell cwd | `cwd` on the shell tool, through protocol, domain, machine and docs | full smith seed `56c0c59` | oracle crate on public APIs (3 control + 14 feature), plus fmt, Clippy, focused and fuzzy under frozen 15 s / 60 s budgets (load-sensitive); about 50 s of 180 s | **history, turn budget, delegation, quotas** | astra | 996.9 (563–1200) / 413.2 (397–437) | 3.52 M / 0.87 M / 24.0k vs 0.28 M / 7.26 M / 23.9k | 125 / 192 vs 97 / 94 | smith 2/6 (+1 setup error); Codex 6/6 (+1 excluded) |

Cost rank, cheapest first: smoke ≪ 01 < 02 < 03 < 04 < 05 < 06 ≪ 07. Task07 took 14.75 M of smith's 15.84 M fresh tokens and 4,430 of its 7,144 agent seconds.

- **B1. After the fixes, the fixtures stopped telling the agents apart.**
  - All fixture failures and outliers came from smith runtime defects, none from model quality.
  - The AI review agreed with the grader on all 76 attempts: 67 pass/pass, 9 fail/fail.
  - Keep the fixtures for comparison and trends. Gate fixes with forced probes.
- **B2. Task07 is the only scale test, and it is 10–50× the cost of the rest.**
  - Its oracle is coupled to smith's crate APIs at the seed (`ARCHIVE/repository/grade.py`), so it is frozen forever.
  - It needs the unpublished skein pin `f5fca44`, so it needs a vendored mirror.
  - Keep it rare; P4 and P5 stand in for it per change.
- **B3. The graders are sound in mechanism but finite.**
  - Sound: the external `rustc --test` against the candidate rlib, and the validation that baselines fail and references pass.
  - Weak, per `grader-review.json`: the planner labels correlate with order, the edits seeds give 12 patterns, and the decoder partition sweep is partial.
  - Missing: a global grading deadline, a process-group kill and bounded logs.
- **B4. Seeds are stored the wrong way for a permanent suite.**
  - Fixture sources are string literals in `build_fixtures.py`.
  - Seeds are smith branches (`bench/fixture-seed/*`), and attempts are `git worktree add` from the developer repo (`benchmark.py:56`); 84 worktrees are registered.
  - This was done for Codex trust: "Independent Git roots in untrusted/unconfigured paths default read-only" (`codex-vanilla-notes.txt`).
  - Clean: plain seed directories, materialised with `git init` in an isolated scratch root that the agent's config declares trusted.

## 2. Behaviour-to-task map

| ID | Defect | Revealed by | Cost | Cheapest revealer |
|---|---|---|---|---|
| B5.1 | completed event above the 1,024-token parser cap (spike 1) | the user's session | — | P1 |
| B5.2 | history and request capacities too small (spike 1) | the user's session | — | P5, scaled down |
| B5.3 | model started before the first request (spike 1; `140c351`) | research smoke2, every baseline run | 13 s, 4.7k tokens | P1 |
| B5.4 | follow-up dropped at shutdown (spike 1) | interactive use | — | P3: live story, 31.8 s on luna/medium |
| B5.5 | configured environment missing in shell (spike 1) | the user's checks | — | P1 |
| B5.6 | `list "."` failed (spike 1) | interactive use | — | P1 |
| B5.7 | report had no trailing newline | interactive use | — | deterministic end-to-end test against fakes |
| B5.8 | 7.0 s teardown | baseline-01 ×3 (7.002 s each); proxy smoke | about 25 s | passive: exit − accept < 0.5 s on every run |
| B5.9 | failure not diagnosable | iteration1-02 (harness missed it, 305 s) | 305 s | P6 plus deterministic tests |
| B5.10 | 2 KiB argument cap | iteration1-02 smith-1/2 | 14.9 s, 3.4k tokens | P2 |
| B5.11 | 60 s completion deadline | iteration2-05 (447.6 s, 74.5k fresh; four timeouts at exactly 60.000 s; only because the model delegated) | 450 s, nondeterministic | passive headroom P8; virtual-time tests |
| B5.12 | 256 KiB envelope | iteration3-07 (293 s, 1.30 M fresh) | 300 s, 1.3 M | P5, or envelope headroom in P8; replay test |
| B5.13 | terminal facts lost; trace bursts | iteration1-02 (3 dropped), Task07 (40) | free | passive invariants |
| B5.14 | 64-turn budget exhausted | iteration5-07 (409 s, 1.30 M) | 400 s | P4, P6 (the ceiling itself is policy) |
| B5.15 | no child turn cap | iteration6-07 (967 s, 3.32 M) | 970 s | P4 |
| B5.16 | harness asked 128 turns of a 64-turn binary | iteration4-07 (0.06 s, 0 tokens) | free | settings pre-flight (B12) |
| B5.17 | low cache reads (improvement) | baseline-01 (Codex 97–117k cached, smith 0–1.5k); planner A/B | 6 × 60 s | P7 |
| B5.18 | writable delegation runs serially (policy) | Task07 | — | trend metric |
| B5.19 | stale edits after shell mutations | Task07 (2 per run) | free | trend metric |
| B5.20 | **not in spike:** no Anthropic `cache_control`; `cache_key` refused (`skein/crates/skein-llm/src/anthropic/request.rs:35-37`) | — | — | P7 on Anthropic |
| B5.21 | **not in spike:** Anthropic wire `max_tokens` = 4096, shared with thinking (`skein-llm/src/types.rs:93-102`; `crates/smith/src/limits.rs:45,72`) | — | — | P2 on Anthropic (expected `MaxTokens` stop) |
| B5.22 | **not in spike:** reasoning tokens not recorded | — | — | schema gap |

### Probes

Common rules:
- Each probe is a frozen seed plus a one-line prompt under smith's 4,088-byte line limit.
- Outcome checks work for any agent; trace checks apply to smith only.
- The prompt forces the behaviour, so the model does not choose whether to exercise it.
- Each probe is calibrated at n=10: 10/10 passes on a known-good binary, and failure on the pre-fix binary where one exists.
- Costs are **unmeasured estimates** extrapolated from smoke2 (4 completions, 4,670 tokens, 13.2 s including 7 s of teardown).

| Probe | Prompt (abridged) | Outcome checks | smith trace checks | Estimated cost (luna/low) |
|---|---|---|---|---|
| P1 smoke-tools | list `.`, read README, `printenv BENCH_MARKER > env.txt`, write answer.txt, finish | file contents, exit 0 | first prompt contains the task; no completion before the first message; `.` is `Listed`; no `Limit`; one main `Accepted`; every `Opened` session `Ended`; usage records == attempts; zero usage-bearing loss; teardown < 500 ms | 4–6 completions, about 10k tokens, 8–15 s |
| P2 large-write | write tool, one call: integers 1..2000, one per line, to numbers.txt | sha256 of the 8,893-byte file | one `write` call with input ≥ 8 KiB, verdict `Written` | 2–3 completions, about 5k output, 15–40 s |
| P3 interactive follow-up | existing live story | existing | existing | 31.8 s (measured) |
| P4 child quota | sub-agent with `max_turns` 2 reads 9 files; the parent finishes the rest and writes words.txt | words.txt exact | input has `"max_turns":2`; child ends `Budget{Turns}` at ≤ 2 turns; parent `Accepted`; spent == Σ own usage | 6–10 completions, 20–30k tokens, 20–40 s |
| P5 history envelope | read all of `corpus/` (72 files × 4 KiB), sum the `KEY` lines | sum.txt exact | ≥ 72 reads; peak request > 256 KiB (needs B13); no `Limit` | 0.6–1.0 M tokens, 60–150 s; weekly |
| P6 budget exhaustion | `turns: 3`; run `echo 1`..`echo 6`, one call each | exits within 2 s of the terminal | `Failed(Budget(Turns))`; exactly 3 usage records; typed reason in the summary | 3 completions, about 5k tokens, 5–10 s |
| P7 cache affinity | 12 sequential single-file reads, with affinity configured | out.txt exact | cache-read fraction over completions 2..n ≥ 0.5 (A/B measured 0.72 vs 0.25) | 30–60k tokens, 20–40 s |
| P8 headroom (passive) | any run | — | longest completion / deadline, peak request bytes / envelope, turns / budget; warn at 0.5 | free |

The smoke suite (P1, P2, P4, P6, P8) is about 2 minutes and under 100k tokens. The outcome halves of P1, P2 and P4 also run on Codex and Claude Code, which compares fixed overhead per task. Codex carried about 25k cached tokens of built-in prompt, against about 1.2k for smith.

## 3. Harness mechanisms

### What it does

| Mechanism | Where | Class | Clean version |
|---|---|---|---|
| Frozen seed and prompt hash manifests | `benchmark.py:56-60` | essential | keep |
| Fresh workspace per attempt; refuse to overwrite | `benchmark.py:49-53` | essential | keep, outside any repo |
| Warm caches outside the timer | `benchmark.py:62`, `repository_runner.py:65` | essential for Rust tasks | keep, matching the agent's profile (the Task07 debug-0 mismatch was fixed later) |
| Detach hard links and make the warmed target writable | `benchmark.py:66-73` | workaround: Codex's sandbox failed on read-only artifacts; baseline Codex fell back to a shared `/tmp` target (`review-fairness.json`) | keep, plus private `TMPDIR` and `CARGO_TARGET_DIR` |
| Disjoint 4-CPU pairs, swapped per repetition | `pair.py:13` | incidental for fixtures (0.7–5 CPU s); essential for concurrent Task07 (233–311 CPU s) | serial and interleaved by default |
| Owned process tree, `/proc` walk, TERM then KILL | `owned_processes.py` | essential, added after a review | skein-world (pidfd) or a cgroup v2 scope (unverified on this host) |
| CPU from `RUSAGE_CHILDREN` / `wait4` | `smith_smoke.py:127,194` | essential but misses descendants that are not reaped | cgroup `cpu.stat` |
| RSS summed from `/proc` every 50 ms | both runners | misleading | cgroup `memory.peak`, or drop it |
| Copy the access token from `~/.codex/auth.json` into SKOT with a dummy refresh | `smith_smoke.py:8-21,125` | workaround, contrary to the live tier's `safe_directory` policy | dedicated sign-ins |
| Terminal found by regex over `Debug` strings | `terminal.py:10-17`; also `support/live_run.rs` | workaround (`crates/smith/src/trace.rs:111,167` emits `format!("{fact:?}")`) | typed trace and summary (B13) |
| Fallback terminal from stdout and stderr lines | `smith_smoke.py:169,190` | workaround; assistant text could match | typed exit and summary |
| Close stdin after acceptance | `smith_smoke.py:172` | workaround; caused the 305 s hang | one-shot entry, or EOF after the task (whether EOF completes or cancels the run is unverified) |
| Join the prompt into one line; 4,088-byte cap | `smith_smoke.py:124` | workaround | task-file input |
| Loss counters parsed from stderr | `smith_smoke.py:53-63`; emitted by `agent_shell.rs:106` | workaround, contrary to `protocol/agent.md` §6 ("never parses it") | summary record |
| Binary-digest capability registry | `binary_profiles.py` | workaround; its only incident cost 0.06 s and 0 tokens | settings pre-flight (B12) |
| Codex run with the user's `CODEX_HOME` | `codex_vanilla_runner.py:200-201` | invocation essential; the home is a leak | isolated home with pinned config |
| Codex metrics from `state_5.sqlite`, rollouts and history | `codex_vanilla_runner.py:13-196` | essential intent, fragile mechanism | rollouts only, in an isolated home (unverified that they carry parent links) |
| Token normalisation | `collect.py:84-90` | essential | typed per-provider convention |
| External grading, source tar, seed-relative diff | `benchmark.py:104-135` | essential | add a deadline and group kill |
| AI review per attempt | `runs/*/review.json` | incidental for regression; essential for quality claims | sampled audits plus a scope gate |
| Harness snapshot per attempt | `benchmark.py:86-100` | essential provenance | harness commit and binary SHA |
| Hard-coded `DEADLINE` | `benchmark.py:20` | incidental | suite wall budget argument |
| Credential scanning of artifacts | `credential_scan.py` | needed before export only | export step |
| Packaging, bundles, history review | `package_artifacts.py` etc. | incidental (handoff of unpublished pins) | publish pins instead |
| Matched-tool Codex bridge and recording proxy | `codex_runner.py`, `recording_proxy.py` | excluded from scoring | a future "controller-only" measurement kind. Codex drops schema numeric bounds |

### What went wrong in the harness itself

- **B6. A missed failure gave an invalid 305 s wall and needed a manual kill.**
  - iteration1-02 smith-1 (`harness-terminal-recognition-error.json`).
  - smith printed `Run failed`, but its terminal facts were lost (B5.13) and stdin was never closed, so the local host idled until a manual SIGTERM.
  - Fix: smith writes an authoritative summary synchronously at exit and offers a one-shot mode; the harness keeps a hard deadline with tree kill.
- **B7. The baseline was confounded.** Codex's sandbox failed on read-only warmed artifacts and retried with a `/tmp` target; two repetitions shared it.
- **B8. Native-metric defects were found only by later reviews.**
  - Delegation calls reported 0 against 27 actual; patched with `native-derived-supplement.json`.
  - Failures and durations covered the root thread only.
  - The collector misread Task07's grade shape.
  - The diff missed the agent's own commits.
  - An accepted run with a non-zero exit would still count as accepted.
  - The 34 harness tests were never gated.
- **B9. Latent silent fallback to root-only Codex usage (not in spike).**
  - `collect.py:88` uses `all_threads_token_usage or usage`, and `session_metrics` returns early without that key if `state_5.sqlite` is missing (`codex_vanilla_runner.py:118-119`).
  - On Task07 that would report 4.78 M instead of 8.32 M input.
  - Fix: a missing measurement is `Unavailable`, never a fallback to another scope.
- **B10. A 128-turn harness setup error.** smith refused correctly before inference; the registry built afterwards was overkill.
- **B11. Measurement scope drifted inside the spike.** The RSS scope changed at iteration1 repetition 3, and the matrix mixes three binaries (`8466afb`, `d75190b`, `44e5120`). Every row should carry its scope and binary identity.

## 4. Metrics and statistics

**Meaningful:**
- binary outcomes;
- teardown;
- fresh, cached and output tokens per thread, deduplicated by response ID;
- usage records compared with completion attempts;
- CPU including tool processes;
- delegation count and seconds; turns per owner;
- headroom (proposed in P8).

**Misleading:**
- root-only Codex usage (4.78 M vs 8.32 M);
- tool-call counts across tool sets (84 Codex `exec` calls vs 192 smith calls);
- total input alone (Codex 7.6 M, but 96 % cached);
- summed RSS, and comparing it with `wait4` max RSS;
- summed overlapping durations;
- success-only medians (780 vs 996.9 s);
- turns treated as HTTP requests;
- zero-price spend;
- reasoning tokens as 0.

**Noise:**
- Run-to-run variation (coefficient of variation, CV) in wall time is 0.04–0.11 for Codex, 0.08–0.29 for smith on fixtures and 0.39 for smith on Task07.
- Fresh tokens vary with CV 0.03–0.25 for Codex and 0.10–0.54 for smith.
- Nine Codex 01 runs with an unchanged binary: wall 19.0–26.3 s, fresh 9.5–18.8k.

**What n buys:**
- With 3 runs per side, the smallest two-sided Mann–Whitney p is 0.10, so n=3 can never reach p < 0.05. You need n ≥ 4 (0.029); n=5 gives 0.008.
- The smallest detectable relative difference is about 2.8·CV·√(2/n): about 25 % for wall time at n=5, and about 60 % for fresh tokens.
- Three passes out of three bound the true failure rate at about 63 %; thirty passes bound it at about 10 %.

**Thresholds:**
- **Gates:** outcome checks and trace invariants. Rerun once on failure; two failures out of two is a regression, one out of two quarantines the probe for recalibration.
- **Trends:** a same-session interleaved A/B (main against the candidate), n ≥ 5, median ratio with a bootstrap 95 % interval. A regression needs the interval to exclude 1 **and** the effect to exceed the task's stored minimum detectable effect.
- Committed baselines serve only as coarse drift alarms.
- **Fix works:** the probe fails on the pre-fix binary and passes on the fix. **Stays working:** the probe stays in the smoke suite, naming the fix it guards.

## 5. Agents and models

**Codex.**
- Invocation: `codex --no-daemon exec --json -m M -c mcp_servers.codebase-memory-mcp.enabled=false -C WS -o final.txt -`, CLI 0.160.0. Effort came from the user's config (high) and was read back from SQLite. Approvals were never asked; trust came from the user's config.
- Observed from JSONL: command and file items, and root usage (input includes cached). From persisted state: per-response usage, model calls, child threads.
- Not observable: retries and wire request bytes.
- Clean: a dedicated `CODEX_HOME` signed in once (never a copy of `auth.json`), a pinned `config.toml` in `benchmarks/agents/`, per-attempt sessions, rollouts only. The trust key syntax and rollout parent links are unverified.

**Claude Code (unverified, from general knowledge).**
- Invocation: `claude -p --output-format stream-json --verbose --model … --permission-mode bypassPermissions --max-turns N`, with `CLAUDE_CONFIG_DIR` dedicated. The user's global CLAUDE.md mandates the codebase-memory MCP, so isolation matters. Use `--strict-mcp-config` with an empty MCP config. Whether an effort flag exists is unknown.
- Events: `init`; `assistant` with `message.usage` (`input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `output_tokens`); sub-agent messages carry `parent_tool_use_id`; a final `result` with `subtype`, `is_error`, `duration_ms`, `num_turns`, `total_cost_usd`, `usage` and `modelUsage`, which includes helper models.
- Deduplicate usage by `message.id`.
- Uncached input = `input_tokens + cache_creation_input_tokens`.
- It reads `CLAUDE.md`, not `AGENTS.md`.

**smith.**
- On the Codex route, `max_output_tokens` is omitted on the wire, and usage is fresh-normalised (`skein-llm/src/openai/response.rs:525-534`).
- On Anthropic, smith has no prompt caching (B5.20), a 4096 output cap on the wire (B5.21), and effort limited to `off|low|medium|high|max` (no `xhigh`).

**Smallest models:**

| Provider | Model | Effort for probes | Basis | Expected per-probe cost |
|---|---|---|---|---|
| OpenAI via Codex subscription | `gpt-6-luna` | low (medium if calibration fails) | smoke2 worked at luna/low; the live interactive story passed at luna/medium in 31.8 s | P1 about 10k tokens and 10 s; smoke suite about 2 min, < 100k tokens; subscription quota, not dollars |
| Anthropic | `claude-haiku-5-5`: $0.10 / $0.50 per MTok at ≤ 100K prompt; cache reads about 0.1×, writes 1.25×; effort default medium | low | claude-api skill; availability on the subscription route unverified. The live tier still defaults to `claude-haiku-4-5` | smoke suite about $0.01; P5 about $0.10 |
| Comparisons | the user's working model and effort (spike: astra/high) | as configured | — | Task07: 1–3.5 M tokens, 400–1,200 s |

## 6. Design input for `./benchmarks`

**Rust or Python.**
- **Parsing smith output:** Rust can deserialise the same typed records smith serialises (once B13 lands), so drift fails the merge gate. Python used regexes over `Debug` strings.
- **Settings:** Rust builds them with smith's own settings types; the Python dict was hand-maintained.
- **Process supervision:** reuse skein-world `end_to_end::Binary` and `smith-real-world`. Missing tree accounting goes to skein first (AGENTS.md).
- **Codex state:** reading rollouts only removes the SQLite need.
- **Gating:** offline tests run in the focused suite; the Python tests were never gated.
- Python's only edge was iteration speed under a six-hour deadline.
- **Recommendation:** a Rust workspace crate, `benchmarks/` (`smith-bench`), under ordinary-Rust harness rules (programming-model.md §10.2).

**Layout.**
- `benchmarks/{README.md, Cargo.toml}`.
- `src/{main.rs, task.rs, suite.rs, agent/{smith,codex,claude}.rs, observe/, run.rs, grade.rs, result.rs, stats.rs, compare.rs}`.
- `tests/` holds recorded fixtures.
- `tasks/{probes,fixtures,repository}/<id>/{task.toml, prompt.txt, seed/, grader/ or oracle/, reference/}`.
- `suites/{smoke,probes-full,compare,repository}.toml`; `agents/{smith,codex,claude-code}.toml`; `baselines/`; `history/`.
- Raw attempts go to `$XDG_STATE_HOME/smith-bench/<run>/<attempt>/`; workspaces are deleted after archiving.

**Task format.**
- `id`, `version`, `kind`, `guards = [fix commits]`, `prompt`.
- `seed = {dir}` or `{smith_commit, patch, exclude = ["benchmarks/"]}`.
- `timeout_seconds`, smith overrides, agent-neutral `[[outcome]]` checks, and named typed `[[smith_trace]]` checks.

**Suite format.** Tasks, agents (each with model and effort), repetitions, design (`interleaved-ab`, `arms`), `max_wall_seconds`, and `gate`.

**Adapter.** A closed `enum Agent { Smith, Codex, ClaudeCode }` with:
1. `prepare`: write the pinned config into a private home and resolve dedicated credentials.
2. `command`: argv, an explicit environment allow-list, how the prompt is delivered.
3. `observe`: a typed live event parser that finds the terminal.
4. `collect`: read persisted artifacts into normalised metrics.
5. `classify`: `Completed`, `Failed(reason)`, `Refused(setup)`, `Timeout` or `HarnessError`.

**Result schema (per attempt, versioned).**
- **Identity:** task, seed and prompt hashes; agent version and SHA; config hash; model and effort, requested and resolved; arm; harness commit; host and CPU set.
- **Timing:** process wall, task wall, teardown, completion maxima.
- **Tokens:** per thread, as `Measure { Observed, LowerBound, Unavailable }` with a convention tag.
- **Counts:** responses, attempts, retries, calls by name (labelled non-comparable), operations and failures, delegations, threads, stale verdicts.
- **Resources:** CPU, and memory with a scope tag.
- **Outcome and grade:** typed terminal, exit, forced termination, grader checks, protected files, dependency policy.
- **Health:** loss counters and headroom.
- **Artifacts:** paths and SHA-256.

**Tiers.**

| Tier | Contents | Cadence | Gate? | Cost |
|---|---|---|---|---|
| Offline | parser and normaliser tests, manifest and seed-hash checks | every nextest run | yes (focused suite) | < 1 s |
| Smoke | P1, P2, P4, P6, P8 on smith, n=1 | per runtime, limits, provider or host change; per fix | by choice, recorded in the commit | about 2 min |
| Full probes | smoke + P5 + P7, both providers, n=3 | weekly | no | about 1–3 M tokens |
| Compare | fixtures × 3 agents, interleaved, n ≥ 5 | on demand | no | 1–2 h |
| Repository | Task07 and future repository tasks, n=3 | rare | no | 3–6 h |

**Live tier.** Keep it as a separate entry point (wire and issuer correctness, outcome-only). Share a test-support crate for dedicated credentials and the typed trace reader. Its `"Run { fact: Returned {"` string match moves to that reader.

**smith prerequisites.**
- **B12.** A settings pre-flight that prints the effective limits as JSON. It replaces the registry.
- **B13.** A typed, versioned trace schema plus a synchronous end-of-run summary record: terminal, per-session usage and turns, retries, loss counters, peak request bytes, longest completion, start time. Also call IDs on tool records and request size on `CompletionStarted`.
- **B14.** A one-shot task-file entry with an exit status that reflects the terminal.

## 7. Pitfalls for day one

1. **User homes and credentials.** The user's config, trust, MCP setup and CLAUDE.md must not leak into runs. Never copy refresh tokens.
2. **Graders readable from the workspace.** The spike's workspaces sat two levels from `tasks/grading`. No agent looked (traces and events checked), but nothing prevented it. With `benchmarks/` inside smith, a repository seed would contain the oracles. Exclude `benchmarks/` from seeds and run attempts without read access to the benchmark tree.
3. **Sandbox and trust differences.** Read-only hard-linked artifacts broke Codex; a shared `/tmp` let attempts reuse each other's target. Give each attempt private dirs.
4. **Warm caches** must be identical to the agent's own profile. Use offline git dependencies.
5. **Disk.** 56 GB for 76 attempts. Delete workspaces after archiving; use reflink copies of the warmed cache.
6. **Detached descendants.** Use pidfd or cgroups and a hard deadline. Never infer agent wall from wrapper wall.
7. **One harness-owned monotonic clock.** smith's `at_ns` is uptime-based. Keep process wall, task wall and teardown separate.
8. **Unknown is not zero.** Never fall back to another scope (B9).
9. **Token conventions** differ per provider. Deduplicate by response or message ID; reasoning tokens are unavailable for smith.
10. **Root and child scopes.** Codex JSONL is root-only; Claude Code sub-agents must be counted; delegation counts mean different things per agent.
11. **Provider state.** Caches warm across repetitions, rate limits are shared and models drift. Interleave arms, record the resolved model, use same-session baselines.
12. **Setup refusal ≠ agent failure.**
13. **Budget asymmetry.** Record each agent's limits; label budget exhaustion as such.
14. **Instruction files.** AGENTS.md vs CLAUDE.md, plus smith's settings `instructions`.
15. **Grader robustness.** Deadlines, group kill, bounded logs. Task07's suite budgets make grading load-sensitive: grade serially on idle CPUs.
16. **Version identity.** One binary per arm. Adapters fail loudly when a persisted format changes.
17. **Prompt delivery.** smith's 4,088-byte line limit and line joining; prompts must be byte-identical across agents.
18. **Credential scan** before any export.
19. **No rankings from n=3.**

## Questions for the user (with recommendations)

1. **Rust or Python for the harness?** Rust workspace crate.
2. **Headless one-shot smith entry?** Land the typed trace and summary first, then a one-shot task-file mode.
3. **What does "vanilla" mean for Codex and Claude Code?** Defaults plus a pinned config committed in the repo, run in isolated homes.
4. **Models per tier?** Probes on luna/low and claude-haiku-5-5/low (confirm Haiku 5.5 on the subscription route, and update the live tier's 4-5 default). Comparisons at the user's working model.
5. **Credentials?** Dedicated sign-ins per agent.
6. **Where do results live?** Raw results outside the tree; commit summaries, baselines and per-task CVs.
7. **Gate policy?** Never at the merge gate. The workflow asks for a smoke run on runtime changes, and every fix names its probe.
8. **Keep Task07?** Yes, frozen and rare, with vendored pins; P4 and P5 stand in per change. New repository tasks need reference solutions.
9. **Hidden graders inside the repo?** Acceptable with an isolated run location and seeds that exclude `benchmarks/`.
10. **AI review?** Drop it per attempt; keep sampled audits plus a scope gate.

## Cross-theme dependencies

- **Trace and observability:** B13 (typed schema, synchronous summary, loss counters, request bytes, call IDs, clock epoch). The probe assertions and B6/B9 depend on it.
- **Limits and config:** B12 pre-flight; Anthropic `max_tokens` 4096 (B5.21); the ceilings P8 measures against.
- **Provider and caching (skein):** Anthropic `cache_control` and `cache_key` (B5.20); reasoning tokens (B5.22); affinity default (P7).
- **Delegation:** `max_turns`, global vs own budgets, serial writes (P4, B5.14, B5.15, B5.18).
- **Lifecycle and shutdown:** teardown invariant; EOF and one-shot semantics (B14); exit status after a failed terminal (P6).
- **Host and interactive:** P1 and P3 depend on `protocol/hosts.md` §5.1.
- **Credentials:** the dedicated store shared by the live tier and benchmarks.
- **Testing / skein:** process-tree accounting belongs in skein-world first; a shared test-support crate.