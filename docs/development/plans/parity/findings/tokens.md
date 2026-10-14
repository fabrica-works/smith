# Findings: cache reuse and tokens

2026-10-14. What the fixture and repository comparisons say about cache reuse, fresh tokens and reasoning tokens, and what the parity plan should measure and change. Written by a read-only sub-agent for the parity plan's designer (skein's development.md, section 4.1).

## 1. In one page

- **The 0.2 is prompt size, not reuse.** Codex sends a fixed preamble of at least 12,288 tokens with every request (its first request reads 12,288 cached tokens in 13 of 15 fixture attempts, before it has done anything); smith's is about 1,536. That preamble is 76% to 84% of Codex's fixture cache reads. Beyond each agent's own preamble, smith's cache-read volume is 0.53, 0.55 and 0.65 of Codex's, not 0.20 to 0.24; its cached share of all input is 0.60 to 0.65 against Codex's 0.87 to 0.93. The retrospective's "about 0.2 times Codex's rate" (`docs/development/plans/reliability/retrospective.md:130-131`) reads a volume as a rate, which the report itself forbids (`smith-codex-2026-10-14.md:96`). (R1)
- **smith misses the cache more often, and each miss costs it more.** On the fixtures, 21 of 102 eligible smith requests lost more than 1,024 tokens of their predecessor's prompt, against 9 of 83 for Codex. A smith miss falls to 0 (13 of 21) or to an earlier prompt of its own conversation (8); a Codex miss falls to its shared 12,288-token preamble (6 of 9). No miss in either agent stopped part-way through a prompt, which a changed prefix would do. (R2)
- **Fixture 03's 1.81x fresh tokens are those misses.** smith lost 35,951 tokens to 12 misses in 38 requests on fixture 03; Codex lost none in 30. Without them, smith's median fresh would be 8,673 against Codex's 9,085 (0.95x); on fixture 02, 8,415 against 9,081 (0.93x). The misses are about 12% of smith's notional spend on 02 and 03. (R2)
- **The stable prefix holds in every record.** System text and tools are recorded once per window; no request's size or bytes shrank within a window in 18 smith attempts; appended guidance comes after the latest results; tool choice `none` kept 0.998 of its predecessor. (R3)
- **The `cache-reuse` probe cannot pass as written.** On the small tier every one of 508 smith requests in its three calibration runs read either 0 or exactly 1,536 cached tokens. The base variant's best possible R is 0.83, under its 0.85 bar; the children's prompts (622 to 1,342 tokens) sit below the provider's minimum; the crossing variant's cached prefix never passes its first prompt (1,517 tokens), so it cannot see the user turn. (R4)
- **Reasoning is content, not protocol.** Both agents ask for encrypted reasoning, replay it, and send effort `high`, verbosity `low` (Codex's from its model catalogue) and no summary. smith's excess is 477 tokens per attempt on 02 and 276 on 03, mostly in one planning request whose identical prompts gave identical 516-token reasoning; on the repository task smith reasons less (2,905 against 3,657). (R5)
- **The repository's fresh-token gap is reading, children and compaction.** smith's 380K against Codex's 264K: misses are 0 to 20K; two compactions per attempt cost 18K to 22K plus 59K to 67K in the three requests after each boundary; smith's main conversation spends 4.8K to 5.4K fresh tokens per request against Codex's 1.9K to 2.4K. Codex read 6.93M cached tokens because it made 84 to 112 requests averaging 76K to 85K tokens; both reread 0.99 or more of each previous prompt. (R6)
- **Target notional spend and fresh tokens; guard smith with lost prefix tokens.** Cache-read volume and cached share reward a large fixed preamble; per-request R is distorted by the provider's granularity at small prompts and by Codex's preamble. The two price catalogs move the spend ratios by at most 0.04. (R7)

## 2. The numbers

### 2.1 Method

Scripts in `/tmp/claude-tokens/` (`python3 -I`, read-only over `~/.local/state/smith-bench/runs/`): `tally.py`, `decomp.py`, `detail.py`, `table.py`, `arms.py`, `pre.py`, `nomiss.py`, `machines.py`, `prefix.py`, `corr.py`, `rsn.py`, `after.py`. They are not kept; each definition below is enough to redo them.

- **smith:** each `response.completed` in `trace.jsonl`, with its `conversation.opened` kind and its `response.started` window. Prompt = `input_tokens + cache_read_tokens + cache_write_tokens`; fresh = `input_tokens + cache_write_tokens` (benchmarks.md 8.2). Cache writes were 0 in every attempt.
- **Codex:** each rollout's `token_usage_record`, one per `response_id`, per `thread_id`. Prompt = `input_tokens`, cached = `cached_input_tokens`, fresh = their difference. The records' sums equal the last `event_msg` `token_count` totals (attempt 06: 80,116 input, 71,424 cached, 1,859 output, 305 reasoning).
- **Reuse R** of a request = its cache read over its predecessor's prompt, within one conversation and window. **Bench R** = the sums' ratio over predecessors of at least 1,152 tokens, as smith-bench computes it (`benchmarks/src/agent/smith/events/cache.rs:56`; `benchmarks/src/agent/codex/rollouts/cache.rs:15-23`; `benchmarks/src/cache.rs:24-28`).
- **A miss** loses more than 1,024 tokens of the predecessor (`predecessor - cache read > 1024`, the step by which the small tier's cache advanced, 2.5; on the working tier hits came within 256 tokens); **lost prefix tokens** are that difference summed over misses.
- The medians reproduce the committed summary: smith fresh 6,914 / 16,669 / 16,413 and Codex 10,317 / 11,869 / 9,085; cache read 13,696 / 22,784 / 27,264 and 69,120 / 110,592 / 112,896; reasoning 273 / 854 / 459 and 224 / 377 / 183 (`smith-codex-2026-10-14.md:89-94`).

### 2.2 Per attempt

`table.py` over `compare-a57ca3a8-20261013T221135Z` and `repository-a64f0523-20261013T230741Z`. Columns: requests; conversations; first prompt and its cache read; total prompt; fresh; cached share; bench R; median per-request R; requests with R under 0.5 over requests with a predecessor; misses; lost prefix tokens; reasoning.

| Id | Agent | Req | Conv | First | First cached | Prompt | Fresh | Share | Bench R | Med R | R<0.5 | Misses | Lost | Reasoning |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 01 | smith | 6 | 1 | 1664 | 0 | 19842 | 6914 | 0.65 | 0.953 | 0.957 | 0/5 | 0 | 0 | 405 |
| 02 | smith | 9 | 1 | 1664 | 0 | 26578 | 10194 | 0.62 | 0.812 | 0.933 | 1/8 | 1 | 2682 | 190 |
| 03 | smith | 6 | 1 | 1664 | 0 | 18633 | 5961 | 0.68 | 0.941 | 0.938 | 0/5 | 0 | 0 | 397 |
| 04 | smith | 8 | 1 | 1664 | 0 | 24344 | 8856 | 0.64 | 0.849 | 0.956 | 1/7 | 1 | 1664 | 273 |
| 05 | smith | 7 | 1 | 1664 | 0 | 20289 | 6593 | 0.68 | 0.952 | 0.947 | 0/6 | 0 | 0 | 127 |
| 06 | Codex | 5 | 1 | 14756 | 12288 | 80116 | 8692 | 0.89 | 0.951 | 0.986 | 0/4 | 1 | 2468 | 305 |
| 07 | Codex | 5 | 1 | 14756 | 12288 | 79678 | 6206 | 0.92 | 0.987 | 0.988 | 0/4 | 0 | 0 | 224 |
| 08 | Codex | 5 | 1 | 14756 | 12288 | 79437 | 10317 | 0.87 | 0.919 | 0.986 | 0/4 | 1 | 4429 | 152 |
| 09 | Codex | 6 | 1 | 14756 | 0 | 97694 | 34206 | 0.65 | 0.798 | 0.989 | 1/5 | 1 | 15425 | 256 |
| 10 | Codex | 5 | 1 | 14756 | 12288 | 80397 | 21901 | 0.73 | 0.741 | 0.966 | 1/4 | 1 | 14942 | 221 |
| 11 | smith | 8 | 1 | 1772 | 0 | 39453 | 16669 | 0.58 | 0.726 | 0.938 | 1/7 | 2 | 7815 | 775 |
| 12 | smith | 8 | 1 | 1772 | 0 | 42233 | 17017 | 0.60 | 0.758 | 0.938 | 1/7 | 2 | 7307 | 1177 |
| 13 | smith | 8 | 1 | 1772 | 0 | 34143 | 8415 | 0.75 | 0.966 | 0.955 | 0/7 | 0 | 0 | 722 |
| 14 | smith | 7 | 1 | 1772 | 0 | 29803 | 7787 | 0.74 | 0.959 | 0.950 | 0/6 | 0 | 0 | 854 |
| 15 | smith | 7 | 1 | 1772 | 0 | 32902 | 19206 | 0.42 | 0.544 | 0.812 | 2/6 | 3 | 11178 | 1100 |
| 16 | Codex | 7 | 1 | 14864 | 12288 | 122462 | 11870 | 0.90 | 0.963 | 0.989 | 0/6 | 1 | 2789 | 534 |
| 17 | Codex | 10 | 1 | 14864 | 12288 | 187440 | 19248 | 0.90 | 0.945 | 0.986 | 0/9 | 2 | 7450 | 783 |
| 18 | Codex | 6 | 1 | 14864 | 12288 | 104414 | 11486 | 0.89 | 0.959 | 0.990 | 0/5 | 1 | 2777 | 377 |
| 19 | Codex | 7 | 1 | 14864 | 12288 | 120669 | 11869 | 0.90 | 0.956 | 0.988 | 0/6 | 1 | 3504 | 348 |
| 20 | Codex | 7 | 1 | 14864 | 12288 | 123938 | 10146 | 0.92 | 0.990 | 0.990 | 0/6 | 0 | 0 | 367 |
| 21 | smith | 10 | 1 | 1762 | 0 | 49542 | 20230 | 0.59 | 0.709 | 0.903 | 3/9 | 4 | 10903 | 459 |
| 22 | smith | 7 | 1 | 1762 | 0 | 32188 | 17468 | 0.46 | 0.608 | 0.709 | 2/6 | 3 | 8884 | 466 |
| 23 | smith | 9 | 1 | 1762 | 0 | 47292 | 15164 | 0.68 | 0.816 | 0.960 | 2/8 | 2 | 6491 | 444 |
| 24 | smith | 9 | 1 | 1762 | 0 | 43677 | 16413 | 0.62 | 0.783 | 0.940 | 2/8 | 2 | 6754 | 526 |
| 25 | smith | 8 | 1 | 1762 | 0 | 38181 | 11301 | 0.70 | 0.874 | 0.953 | 1/7 | 1 | 2919 | 321 |
| 26 | Codex | 8 | 1 | 14854 | 12288 | 141700 | 9220 | 0.93 | 0.989 | 0.990 | 0/7 | 0 | 0 | 228 |
| 27 | Codex | 6 | 1 | 14854 | 12288 | 103037 | 9085 | 0.91 | 0.991 | 0.991 | 0/5 | 0 | 0 | 139 |
| 28 | Codex | 7 | 1 | 14854 | 12288 | 121805 | 8909 | 0.93 | 0.991 | 0.990 | 0/6 | 0 | 0 | 133 |
| 29 | Codex | 7 | 1 | 14854 | 12288 | 122073 | 9049 | 0.93 | 0.990 | 0.990 | 0/6 | 0 | 0 | 183 |
| 30 | Codex | 7 | 1 | 14854 | 7168 | 124452 | 13988 | 0.89 | 0.990 | 0.991 | 0/6 | 0 | 0 | 187 |
| R01 | smith | 50 | 3 | 2426 | 0 | 2737418 | 254090 | 0.91 | 0.997 | 0.998 | 0/45 | 0 | 0 | 1655 |
| R02 | smith | 66 | 5 | 2426 | 0 | 3293552 | 408176 | 0.88 | 0.990 | 0.997 | 3/59 | 3 | 20084 | 2905 |
| R03 | smith | 55 | 6 | 2426 | 0 | 2565515 | 380427 | 0.85 | 0.993 | 0.996 | 3/47 | 3 | 7900 | 3079 |
| R04 | Codex | 112 | 3 | 15505 | 12288 | 9130160 | 304560 | 0.97 | 1.000 | 0.998 | 0/109 | 0 | 0 | 4868 |
| R05 | Codex | 85 | 3 | 15505 | 12288 | 7194500 | 264452 | 0.96 | 1.000 | 0.998 | 0/82 | 0 | 0 | 3135 |
| R06 | Codex | 84 | 3 | 15505 | 12288 | 6338107 | 226875 | 0.96 | 1.003 | 0.998 | 0/81 | 0 | 0 | 3657 |

R01 to R06 are the repository run's 01 to 06. smith's repository conversations include two compaction requests each (one-request conversations); window boundaries are not counted as misses.

### 2.3 Per arm: preamble, history, misses and spend

`pre.py` (preamble P = 1,536 for smith on the fixtures, 2,304 on the repository, 12,288 for Codex: the cache read each agent's second request, or first, shows for its fixed part), `arms.py` and `nomiss.py`; medians over each arm's attempts.

| Arm | Preamble share of input | Preamble share of cache reads | Cache reads beyond preamble | Misses / eligible (sum) | Lost tokens (sum) | Fresh | Fresh without misses | Spend, `prices.toml` | Without misses | Spend, `presets.rs` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 01 smith | 0.51 | 0.61 | 5,248 | 2/31 | 4,346 | 6,914 | 6,914 | 0.0287 | 0.0277 | 0.178 |
| 01 Codex | 0.77 | 0.84 | 9,984 | 4/21 | 37,264 | 10,317 | 6,224 | 0.0384 | 0.0356 | 0.251 |
| 02 smith | 0.33 | 0.42 | 13,568 | 7/33 | 26,300 | 16,669 | 8,415 | 0.0641 | 0.0560 | 0.391 |
| 02 Codex | 0.70 | 0.78 | 24,576 | 5/32 | 16,520 | 11,869 | 9,081 | 0.0646 | 0.0646 | 0.404 |
| 03 smith | 0.32 | 0.37 | 17,664 | 12/38 | 35,951 | 16,413 | 8,673 | 0.0614 | 0.0540 | 0.376 |
| 03 Codex | 0.71 | 0.76 | 27,008 | 0/30 | 0 | 9,085 | 9,085 | 0.0629 | 0.0629 | 0.391 |
| Repo smith | 0.05 | 0.05 | 2,370,432 | 6/151 | 27,984 | 380,427 | 372,527 | 0.966 | 0.957 | 7.08 |
| Repo Codex | 0.15 | 0.16 | 5,890,944 | 0/272 | 0 | 264,452 | 264,452 | 1.409 | 1.409 | 10.64 |

Spend in notional dollars: `prices.toml` is 1.25 / 0.125 / 10 per million fresh / cached / output (`benchmarks/agents/prices.toml:7-10`), which reproduces the report's common priced spend (0.028686 and 0.038383 on 01); `presets.rs` is 10 / 1 / 50 (`crates/smith-local-shell/src/presets.rs:112`). smith/Codex spend ratios: 0.75, 0.99, 0.98, 0.69 with the first; 0.71, 0.97, 0.96, 0.67 with the second.

### 2.4 The misses

`machines.py`: what each miss still read. "Earlier" means within 256 tokens of the prompt of an earlier request of the same conversation, not its predecessor.

| Agent, run | Read 0 | Read only the 12,288 preamble | Read an earlier request's prompt | Anything else |
|---|---:|---:|---:|---:|
| smith, fixtures | 13 | — | 8 | 0 |
| Codex, fixtures | 2 | 6 | 1 | 0 |
| smith, repository | 5 | — | 1 | 0 |

An example, `compare-a57ca3a8-20261013T221135Z/21` (`detail.py`): prompts 1,762, 1,797, 1,873, 2,934, 4,039, 6,613, 7,148, 7,232; cache reads 0, 1,664, 0, 1,792, 0, 2,816, 1,664, 7,040. Request 6 read request 4's prompt (2,934, to 128 tokens), request 7 request 1's (1,762), request 9 request 6's (6,613 to 6,528). Gaps between a response and the next request were 4 to 1,070 ms, and a miss's median first byte was 599 ms against 547 ms for a hit (21 and 81 requests), so misses cost tokens, not time. smith misses followed a response with reasoning in 10 of 31 cases and one without in 11 of 71 (Codex: 5 of 48 and 4 of 35) (`corr.py`).

### 2.5 The `cache-reuse` probe's records

| Run (summary) | Attempts | Cache reads seen | Largest prompt | Bench R median (summary) |
|---|---:|---|---:|---:|
| `probes-cache-reuse-8d2ec9b8-20261012T171928Z` (base) | 9 | 0 in 37 requests, 1,536 in 80 | 2,237 | 0.62 |
| `probes-cache-reuse-children-8d2ec9b8-20261012T172334Z` | 9 | 0 in 257, 1,536 in 4 | 1,829 | 0.0 |
| `probes-cache-reuse-crossing-8d2ec9b8-20261012T173024Z` | 9 | 0 in 34, 1,536 in 96 | 2,451 | 0.61 |
| `probes-cache-reuse-8d2ec9b8-20261012T170907Z` (provider-stopped) | 2 | 0 in 12, 1,536 in 14, one null | 2,237 | 0.48 |
| `probes-cache-reuse-12eb3e81-20261012T173550Z` (Codex, raw only) | 3 | 1,792 to 14,080 | 22,922 | 0.95 to 0.97 (`tally.py`) |

Codex on the same probe also read a flat prefix: 13,056 for three requests, then 14,080 for nine, while its prompt grew about 70 tokens a request (attempt 01). On the working tier smith's hits followed the predecessor to within 256 tokens in 78 of 81 (median 131; attempt 21's request 8: 7,040 of 7,148).

### 2.6 The claims, checked

| Claim | Source | Check |
|---|---|---|
| smith reused the cache at about 0.2 times Codex's rate on the fixtures | `retrospective.md:130-131` | The volume ratio is right (0.198, 0.206, 0.242); as a rate it is wrong: cached share 0.75, 0.66, 0.67 of Codex's; bench R 1.02, 0.79, 0.79 of Codex's (2.3, R1) |
| Cache-read effects are volumes, never reuse fractions | `smith-codex-2026-10-14.md:96` | Right, and the measure the report should add is lost prefix tokens (R1) |
| Fixture 03's fresh tokens are 1.81 times Codex's | `:118` | 16,413 against 9,085; without smith's misses 8,673, 0.95 times (2.3, R2) |
| Reasoning 2.27 times (02) and 2.51 times (03) | `:113,121` | 854 against 377 and 459 against 183: 477 and 276 tokens per attempt (R5) |
| Codex read 6.93M cached tokens to smith's 2.48M on the repository task | `:153` | Right; 85 requests at 84,600 mean against 55 at 46,600 (medians R05, R03), bench R 1.000 and 0.993 (R6) |
| smith's fresh 380K to Codex's 264K | `:152` | Right; misses are 7,900 of R03's 380,427; children, compaction and larger appends are the rest (R6) |
| smith 01's first prompt is 1,664 and later requests read near the previous prompt | the brief | Right: 1,536 to 4,096 read, R 0.92 to 0.97 (attempt 01) |

## 3. Entries

### R1 The cache-read ratio measures Codex's preamble

- **Symptom:** cache-read ratios 0.198, 0.206, 0.242 (`smith-codex-2026-10-14.md:103,111,119`), read as reuse by the retrospective (`retrospective.md:130-131`). Codex's first request is 14,756 to 15,505 tokens and reads 12,288 cached in 13 of 15 fixture attempts and all 3 repository mains; smith's is 1,664 to 2,426 tokens and reads 0 in all 18. Codex's preamble is 70% to 77% of its fixture input, smith's 32% to 51% (2.3). Beyond the preamble the ratios are 0.53, 0.55, 0.65 (fixtures) and 0.40 (repository). Codex's per-request bench R (0.92 to 0.99) and smith's (0.76 to 0.94) are much closer than their volumes.
- **Root cause:** Codex's base instructions are 21,710 characters (`session_meta.base_instructions`, every rollout), plus three developer messages and two user messages (serialized 4,953, 2,776, 584, 2,274 and 860 bytes in attempt 06's `response_item`s); smith's system text is 2,034 characters on the fixtures and 4,595 on the repository task, with 9 tool schemas (`response.started` 1, `prompt.system`, `prompt.tools`). The cross-attempt hit on Codex's first request shows the provider holds that prefix warm for any Codex session; smith's 1,536-token preamble is never warm on a first request. A volume therefore scales with requests times preamble. benchmarks.md 9 allows "tokens by convention" across agents (`docs/design/benchmarks.md:467-469`: "Only outcome, wall time, tokens by convention and spend on one basis are compared across agents") without saying which token measures mean something across agents.
- **The clean fix:** no smith code. benchmarks.md 9 should say: "Cache reads and the cached share are compared within one agent's arms, never across agents: they grow with an agent's fixed preamble." benchmarks.md 8.2 (`:415`, "One normalised record per scope: **fresh** ..., **cache read**, ...") gains a derived count per scope, **lost prefix**: over requests whose predecessor in the same conversation and window can be cached, the predecessor's prompt less the request's cache read where that exceeds the provider's step, summed. The summary reports it with fresh and spend. The parity plan's design restates the retrospective's sentence as a volume where it cites it.
- **Its tests:** `benchmarks/tests/events.rs`: `lost_prefix_counts_only_eligible_same_window_predecessors_beyond_step` (window boundaries and compaction requests excluded); `lost_prefix_is_unavailable_when_any_usage_is_unreported`. `benchmarks/src/agent/codex/rollouts/cache.rs` tests: `codex_lost_prefix_joins_children_by_parent_link_only`. `benchmarks/src/summary.rs` tests: `summary_never_compares_cache_reads_across_agents`. Live: none needed; a rerun of the summary over the committed runs should give 2.3's lost-token sums.
- **Confidence:** high for the arithmetic; medium for "the provider holds Codex's prefix warm for any session" (it rests on 16 of 18 first requests); a smith arm whose system text is padded to 12K tokens would show whether smith's first request would then hit.

### R2 smith's requests leave the machine that holds their prefix

- **Symptom:** 2.2 to 2.4. Fixtures: 21 misses in 102 eligible smith requests, 9 in 83 Codex; lost tokens 66,597 against 53,784; fixture 03 alone 12 in 38 against 0 in 30. Repository: 6 in 151 smith, all in children (four after a child's first prompt of 1,249 to 1,437 tokens; one losing 5,168 of 5,168 and one 17,513 of 18,665), 0 in 272 Codex. smith's misses are full (cache read 0 in 13 of 21), Codex's are cushioned (6 of 9 still read 12,288).
- **Root cause:** routing, not bytes. Each miss read exactly what an earlier request of its own conversation had left, or nothing (2.4, 0 "anything else"), which is what a request sent to a cache machine that last served that earlier request reads; a prefix change would leave an intermediate amount, at the first changed byte. smith renders the affinity as designed: the run key as `prompt_cache_key` (skein `crates/skein-llm/src/translate.rs:92-95`) and as the `session-id` header, the thread as `thread-id` (skein `crates/skein-llm/src/client.rs:1538-1551`), one key per run from OS entropy (`crates/smith-domain-run/src/identity.rs:25-29`, `run.rs:298-300`; skein `crates/skein-shell/src/seed.rs:12-16`). Codex's records show the same identities (children share the root's `session_id`, each with its own `thread_id`: rollouts of `repository-.../04`). What Codex also sends cannot be seen in its records; the reliability findings name its `x-codex-turn-state` sticky-routing token and its Responses-over-WebSocket transport (`docs/development/plans/reliability/findings/context.md:105-106`). The design defers both: protocol/llm.md 12 (`:473-475`): "**Routing extras:** Codex's turn-state token and `reasoning.context`, adopted only if a probe shows a material gain"; decisions 4.4 (`decisions.md:154-157`); skein llm.md 6 (`docs/design/llm.md:680-691`). The probe that was to measure them did not (R4), and the `affinity` experiment's arms were not built (`smith-codex-2026-10-14.md:169`).
- **The clean fix:** measure, then adopt the one that closes the gap. First a working-tier probe (R4's version 2) with two smith arms: as today, and with the turn-state token; and a Codex arm. If turn state brings smith's misses to Codex's rate, skein's Codex dialect returns the response's turn-state header as an opaque routing value with the completion, the session hands it back on every request until the next user message and never writes it to the transcript. Design: skein llm.md 4.6 gains it (and 6 drops it); smith protocol/llm.md 2.1 gains "the routing state the dialect returned, within a turn" after "Codex: the key as the body's `prompt_cache_key` and as the `session-id` header, and the thread as the `thread-id` header" (`:69-70`), and 2.3's "Codex caches a prefix on its own, routed by the affinity (2.1)" (`:105`) names it; domain/session.md 3 says it is per turn and not part of a turn as told; decisions 4.4 records the measured gain. If neither closes it, WebSocket stays skein's roadmap item, and the parity plan records the residual as the provider's. The gain is bounded: about 12% of smith's spend on fixtures 02 and 03 (0.0641 to 0.0560, 0.0614 to 0.0540), under 1% on the repository task.
- **Its tests:** skein `crates/skein-llm/src/openai/tests.rs`: `codex_routing_state_is_returned_from_the_response_header`; `codex_routing_state_is_replayed_until_the_next_user_message`; `codex_request_without_routing_state_sends_no_turn_header`; `routing_state_header_name_is_reserved_against_endpoint_headers`. smith `tests/protocol-llm/tests/client.rs`: `routing_state_never_enters_a_told_turn_or_transcript` (beside `actual_requests_render_the_run_key_and_thread_only_for_codex`, `:546-576`). Live: R4's version 2, three arms, five attempts each interleaved; the measure is misses per 100 eligible requests and lost prefix tokens.
- **Confidence:** medium that routing is the cause (all 36 misses fit it; nothing else does); low on which Codex mechanism prevents it, since no Codex wire was captured. Codex's own 9 misses on fixtures 01 and 02 say its mechanism is not perfect either.

### R3 The stable-prefix invariant holds in the records

- **Symptom:** none of a break. `prefix.py` over 18 smith attempts: system text and tools recorded once per conversation window (`(conversation, response, window, system chars, tools)` = `(0, 1, 1, 2034, 9)` in every fixture attempt); no request's prompt tokens or `request_bytes` fell within a window; after the first request the only text blocks sent were a child's "Begin the work your brief describes.", the compaction instruction, a new window's task and summary, and one final-turn note, each after the latest results. The one `tool_choice` `none` request (`repository-.../02`, response 44) read 51,840 of 51,954. Compaction requests read their history (65,280 of 65,454 in R01), and four of the five main windows opened after one read the 2,304-token preamble (R02's read 0).
- **Root cause:** decisions 4.8 (`decisions.md:168-173`) and session.md 3 (`docs/design/domain/session.md:81-101`) are kept by the code: tool choice `Only` renders as `auto` with every tool kept (`translate.rs:70-74`); reasoning goes back verbatim as an opaque block (`crates/smith-protocol-llm/src/completion.rs:136-143`, `prompt.rs:424-431`); the wire prefix is tested (`tests/protocol-llm/tests/brief.rs:467`, `five_native_completions_keep_the_wire_prefix_and_replay_each_previous_answer`). What a trace cannot show: `response.started` records only a request's new part (`docs/design/protocol/events.md:234`, "under `everything`: the request's new part (3.11)"), so a changed byte in an old part is visible only through usage, as in 2.4.
- **The clean fix:** a passive check, no code in smith: smith-bench flags any request whose cache read is neither within 1,024 tokens of its predecessor's prompt, nor 0, nor within 256 tokens of an earlier prompt of its window, as "prefix break suspected". benchmarks.md 11.2 gains the row.
- **Its tests:** `benchmarks/tests/events.rs`: `an_intermediate_cache_read_is_flagged_as_a_suspected_break`; `a_read_of_an_earlier_prompt_is_a_miss_not_a_break`. Live: the check runs passively on every suite.
- **Confidence:** high for these runs; the passive check makes it hold for later ones.

### R4 The `cache-reuse` probe measures the provider's granularity

- **Symptom:** 2.5. Base: 0 of 3 triplets, median 0.62; children 0.0; crossing 0.61 (`benchmarks/summaries/probes-cache-reuse-*.json`, `groups[].metrics["cache/reuse"]`). The report: "13.2 base/children/crossing 0/3 triplets each, none calibrated" (`smith-codex-2026-10-14.md:52`).
- **Root cause:** the probe runs on the small tier at low effort (`benchmarks/tasks/probes/cache-reuse/task.toml:4-5`: `tier = "small"`, `effort = "low"`), where the provider cached smith's prompt only as 0 or 1,536 tokens across all 508 requests, while each read added about 60 tokens. With a 1,536 ceiling the base variant's best bench R is 18,432 / 22,171 = 0.83, under the bar (benchmarks.md 11.1, `:552`: "median of three at least 0.85, and full misses (under half) at most one per ten requests"). The children's prompts are 622 to 1,342 tokens (`probes-cache-reuse-children-.../01`), under the provider's 1,024-token minimum (reliability `findings/context.md`, C3: "1,024 tokens on OpenAI, as the cached amounts show") or barely over it, and the probe's eligibility starts at 1,152; 257 of 261 requests read 0. The crossing variant reasons 0 to 171 tokens per attempt at medium effort, and its cached 1,536 is the first prompt (1,517 tokens) and a few tokens more, so a change at the user turn, which comes after six reads, cannot show; the request carrying its person message read the same 1,536 (attempt 01, response 8). The control arm "without affinity" (`:552`) was never built.
- **The clean fix:** version 2 of the probe. Seed files of about 1,500 tokens each (a fixed prose block plus the pointer), so the prompt passes 8,000 tokens by the fifth read and grows faster than the provider's step; the check becomes misses (lost more than 1,024 tokens) at most one per ten eligible requests over the three, and lost prefix tokens at most 10% of the eligible predecessors' sum, until calibration sets them. `children`: briefs that make each child's opening prompt at least 4,096 tokens (one long read first). `crossing`: on the working tier at high effort, where the planning request reasons 156 to 516 tokens (R5), and the person message comes after a reasoning response; its measure is the first request after the message. The control arm sends no affinity (a probe-only setting, refused outside smith-bench). The probe's row in benchmarks.md 11.1 changes its "Forces" and "Checks" cells accordingly.
- **Its tests:** `benchmarks/src/run/outcomes.rs` tests: `cache_reuse_v2_counts_misses_beyond_one_step_only`; `cache_reuse_v2_rejects_a_seed_whose_files_are_too_short` (validate). `crates/smith` settings tests: `probe_only_no_affinity_setting_is_refused_outside_benchmarks`. Live: version 2 calibrated: 10 passes within 12 on smith; the no-affinity arm fails the misses criterion on at least one attempt (the probe's before).
- **Confidence:** high that version 1 cannot pass on this tier (2.5's counts); medium that 1,500-token files are enough (Codex's own probe attempts, at 13,988 to 22,922 tokens, reached 0.95 to 0.97).

### R5 Reasoning tokens follow the prompt's content

- **Symptom:** reasoning ratios 2.27 (02) and 2.51 (03) (`smith-codex-2026-10-14.md:113,121`); medians 854 against 377 and 459 against 183, that is 477 and 276 tokens per attempt, 20% and 12% of smith's output (Codex 10% and 5%). On the repository task smith reasons less: 2,905 against 3,657 (15% and 17% of output). Per request (`rsn.py`): smith's planning request (the third or fourth) reasoned 501 to 516 tokens in all five 02 attempts, Codex's 194 to 448; on 03, 156 to 516 against 91 to 116. Identical smith prompts gave identical reasoning: attempts 12 and 15 sent a 3,002-token third request and both reasoned 516.
- **Root cause:** the request knobs match, as far as the records show. smith sends `store:false`, `include:["reasoning.encrypted_content"]`, `reasoning:{effort}` and `text:{verbosity:"low"}` (`translate.rs:47-52,85-91,96-100`; skein `crates/skein-llm/src/openai/identity.rs:11`), effort `high` (`response.started.model.effort`), no `reasoning.summary`, and replays each reasoning block (R3). Codex's turn context has `effort:"high"`, `summary:"none"` (`turn_context`, every fixture rollout); its model catalogue gives `gpt-6-astra` `default_verbosity: low` and `default_reasoning_summary: none` (`~/.codex/models_cache.json`); its reasoning items carry `encrypted_content` (attempt 06: 5,112 and 1,720 characters) with empty summaries. What differs is content: Codex's 21,710-character base instructions and one free-form `exec` tool, against smith's short system text and 9 JSON function tools. Which part drives the planning request's length is not in the records.
- **The clean fix:** none in the protocol. One latent difference to remove: smith's verbosity is a skein constant rather than the model's (`identity.rs:11`), so a model whose default differs would diverge from Codex; it belongs with the model's declared values (shell.md 6.1). Whether smith's system text should change to shorten planning is the instructions theme's, judged by output and spend, not by reasoning alone.
- **Its tests:** skein `crates/skein-llm/src/openai/tests.rs`: `codex_request_sends_the_models_declared_verbosity`; `codex_request_omits_reasoning_summary_and_keeps_encrypted_include`. Live: the fixture comparison's reasoning metric, read within output.
- **Confidence:** high that the protocol knobs match on the fields the records show; low on the cause of the planning request's length.

### R6 The repository task's fresh tokens

- **Symptom:** fresh 380,427 against 264,452 (`smith-codex-2026-10-14.md:152`). `after.py` and the main/child split:

  | Attempt | Main fresh (requests) | Children fresh (requests) | Compaction requests | First 3 requests of each later window | Lost to misses |
  |---|---:|---:|---:|---:|---:|
  | smith R01 | 231,988 (48) | 0 | 22,102 (2) | 66,500 | 0 |
  | smith R02 | 166,646 (35) | 223,444 (29) | 18,086 (2) | 58,985 | 20,084 |
  | smith R03 | 177,522 (33) | 184,801 (20) | 18,104 (2) | 64,222 | 7,900 |
  | Codex R04 | 103,614 (55) | 200,946 (57) | 0 | — | 0 |
  | Codex R05 | 112,341 (46) | 152,111 (39) | 0 | — | 0 |
  | Codex R06 | 93,694 (39) | 133,181 (45) | 0 | — | 0 |

  Codex's cache read is 2.8 times smith's because it made 84 to 112 requests at a mean prompt of 75,500 to 84,600 tokens, against 50 to 66 at 46,600 to 54,700; both reread nearly all of each predecessor (bench R 0.990 to 1.003).
- **Root cause:** outside this theme, except the misses (R2). smith's main conversation spends 4,833 to 5,379 fresh tokens per request against Codex's 1,884 to 2,442; a smith child's prompt grows from 1,341 to 19,480 tokens in one step (`repository-.../03`, conversation 1, responses 7 and 8). Each smith attempt compacted twice under the benchmark arm's declared window of 131,072 (`benchmarks/src/agent/smith/settings.rs:148`), against the preset's 272,000 (`presets.rs:50-51`); Codex's largest prompt was 131,454 tokens and it never compacted. A smith child starts with its brief as the first bytes of its system text and three tools (`crates/smith-domain-run/src/prompt.rs:334-341`), so it shares no prefix with its parent or a sibling; its first prompt is 1,295 to 1,437 tokens and read 0. Codex's children open at 16,737 to 16,773 tokens and read 0 to 14,464, the second of each pair 14,464.
- **The clean fix:** the window, the read sizes and delegation are the context, tools and delegation themes' (section 5). For this theme: put a child's shared guides before its brief (C3's low-priority option, `findings/context.md` C3) only if a child's fixed part grows past 1,024 tokens; today it would save at most about 1,000 tokens per child.
- **Its tests:** for the child order, if adopted: `crates/smith-domain-run` tests: `child_system_text_puts_shared_mechanics_before_its_brief`. Live: the repository comparison's fresh tokens, with lost prefix (R1) beside it.
- **Confidence:** high for the decomposition; the attribution of the per-request append to read sizes is medium until the tools theme counts result bytes per tool.

### R7 Which measure a parity target uses

- **Symptom:** three caching measures disagree about the same runs. Volume: smith 0.20 to 0.24 of Codex. Cached share: 0.60 to 0.65 against 0.87 to 0.93. Bench R: 0.76 to 0.94 against 0.92 to 0.99, and 0.62 on a probe where smith's bytes were stable. Spend, which prices cache reads at a tenth of fresh input in both catalogs: 0.75, 0.99, 0.98 and 0.69.
- **Root cause:** volume and share grow with a fixed preamble the provider keeps warm (R1); R is capped by the provider's step at small prompts (R4) and lifted for Codex by its preamble, which turns its full misses into partial ones (2.4); only fresh tokens and spend price what the provider did. The catalogs: `prices.toml` 1.25 / 0.125 / 10 per million (`:7-10`, the summaries' basis), the smith arms' pins the same rates per 8,000,000 (`benchmarks/agents/smith/first-request.json:23-24`), `presets.rs` 10 / 1 / 12.5 / 50 (`:112`); harness findings T6 lists them all.
- **The clean fix:** the parity target on tokens is notional spend on one catalog (`prices.toml`) and fresh tokens, per task, with benchmarks.md 9's statistics; lost prefix tokens (R1) are smith's guard against regressions, reported for both agents; cache-read volume and share are diagnostics within an agent. benchmarks.md 9 names this. Reasoning stays within output.
- **Its tests:** R1's summary tests; `benchmarks/src/summary.rs` tests: `parity_spend_uses_one_named_catalog_for_both_agents`.
- **Confidence:** high; the choice of catalog moves the ratios by 0.02 to 0.04 (2.3).

## 4. Questions for the designer

1. **Which caching measure does the parity target use?** Options: (a) cache-read volume ratio, as the report shows; (b) cached share of input; (c) per-request reuse R; (d) notional spend and fresh tokens, with lost prefix tokens as smith's guard. Recommendation: (d). Why: (a) and (b) reward a large fixed preamble (Codex's is 76% to 84% of its cache reads); (c) is capped by the provider's step at small prompts and flatters Codex's misses; (d) prices what the provider did and still names each miss.
2. **Which price catalog?** Options: `prices.toml`; `presets.rs`; both shown. Recommendation: `prices.toml` for the target, the summaries' existing basis, with `presets.rs` brought to the same list prices or its difference named in shell.md 6. Why: the verdicts do not move (ratios within 0.04), the dollars do (6 to 8 times).
3. **Build Codex's turn-state routing now, or measure first?** Options: build it in skein's Codex dialect now; measure with R4's version 2 first; leave the residual to the provider. Recommendation: measure first, in the parity plan's first wave. Why: the gain is bounded at about 12% of spend on two fixtures and under 1% on the repository task, the mechanism is private, and no Codex wire has been captured.
4. **Rewrite the `cache-reuse` probe?** Options: keep version 1 and lower its bar; version 2 with longer files and a misses criterion; move it to the working tier. Recommendation: version 2 on the small tier, with `crossing` on the working tier at high effort. Why: version 1's best R is 0.83 by the provider's step; longer files keep the small tier's cost while measuring routing; reasoning large enough to cross a user turn exists only at the working tier.
5. **Should the benchmark's smith arm declare the preset's window?** Options: keep 131,072; use the preset's 272,000; declare Codex's 258,400. Recommendation: the preset's, as compaction.md K8 and harness.md T4 also say. Why: two compactions per repository attempt cost 18,000 to 22,000 tokens, 59,000 to 67,000 more in rereading, and 37 to 60 seconds at each of the five main compactions (`detail.py`: R01 responses 7 and 20, R02 20, R03 17 and 52), which the product would not spend.
6. **Is reasoning a parity metric of its own?** Options: target reasoning tokens; report them within output and spend. Recommendation: within output and spend. Why: the excess is 276 to 477 tokens per attempt (4.5% and 7.4% of smith's median spend at $10 per million), reverses on the repository task, and has no protocol cause.

## 5. What other themes must say

- **Context and compaction:** the 131,072 window of `benchmarks/src/agent/smith/settings.rs:148` puts two compactions in every smith repository attempt: 18,086 to 22,102 fresh tokens in the compaction requests, 58,985 to 66,500 in the first three requests of the later windows, and a 37 to 60 s wait at each of the five main ones (R6). Codex peaked at 131,454 and never compacted.
- **Tools:** smith's main conversation spends 4,833 to 5,379 fresh tokens per request against Codex's 1,884 to 2,442; one child read took its prompt from 1,341 to 19,480 tokens (R6). The per-tool result sizes and read windows decide most of the repository task's fresh tokens.
- **Delegation:** smith's children are 55% and 49% of R02's and R03's fresh tokens (223,444 and 184,801); Codex's are 58% to 66% (133,181 to 200,946). A smith child's first prompt (1,295 to 1,437 tokens) reads nothing, and its second read 0 in 3 of 5 children; Codex's children open at about 16,750 tokens with up to 14,464 cached.
- **Benchmarks and harness:** add lost prefix tokens per scope and the suspected-break check (R1, R3); stop comparing cache reads across agents (R1, R7); rewrite `cache-reuse` (R4); one price catalog (R7, harness.md T6).
- **Instructions:** smith's planning request reasons 156 to 516 tokens against Codex's 91 to 448 with the same knobs (R5); a change to smith's system text is judged by output and spend; it changes the fixed preamble, which moves cache-read volume but not lost prefix tokens.
- **skein:** R2's routing state, if measured worth it, is skein's Codex dialect's; R5's verbosity belongs with the model's declared values; `SESSION_HEADER = b"session_id"` (`crates/skein-llm/src/openai/identity.rs:9`) is unused while the client sends `session-id` (`client.rs:1545`), which matches native Codex (`findings/context.md:44`), so the constant can go.
