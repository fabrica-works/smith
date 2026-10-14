# smith and Codex: the reliability milestone

2026-10-14. The reliability plan's successor to the 2026-10-08 benchmark.

Five compaction attempts (02, 03, 07, 09, 10) ended `Failed(model/malformed)` after compaction: the model selected `finish`, `read` or prose where summary text or a valid call was expected. Attempt 02's finish lacked fields; 03's finish carried prose; 07, 09 and 10 selected read. This repeats the malformed small-window finding in the stopped preflight's 27-rerun. It needs attention in the next plan whatever threshold the user chooses. Both threshold arms passed only 1/5; no metric difference was detected.

All 30 fixture candidates passed their graders but failed the protected-tests check. All six repository attempts were flagged for protected bundle files absent from their full diffs; one smith candidate also failed the inclusive normalized-path limit. These are separate reliability, task and harness findings. The results below retain every failure and unavailable field.

## Method

The Rust `smith-bench` harness follows [benchmarks.md](../../design/benchmarks.md), sections 2, 3.1, 4.1, 5.2–5.4, 6.1, 8–10 and 11.5. Fresh seeded repositories, identical task prompts, hidden graders after tree exit, pinned configurations and interleaved comparisons replace the spike's external harness. Repository attempts and grading ran serially on the idle gate lane on debian-16gb-hel1-1. Live suites ran whole through `heavy --gate`; builds and summaries through `heavy`. Setup and grading are outside task/process timers; agent-issued builds remain inside them.

| Item | This milestone |
|---|---|
| Small tier | `gpt-6-luna`, low effort; cache crossing names medium |
| Working tier | `gpt-6-astra`, high effort, for fixtures and repository |
| smith face | `smith exec` / interactive product, typed events; each summary names its built source and binary digest |
| Other agent | Standalone Codex CLI 0.160.0, SHA-256 `12eb3e81114588aca3b7998f4f19e8997b056aca08e57a7ca7c8a3ec8c652aad`, pinned overrides; private complete package selected per command, user's installation unchanged |
| Formats | Result and summary v2; channel 2, charter 2, transcript 3, events 1; historical faces/versions remain historical |
| Isolation | Attempt-local home, temp and warmed build caches; seeds omit graders, references and benchmark tree; pinned settings rather than inherited skills/configuration |
| User's logins | smith borrows Codex read-only/access-token-only; harness gives Codex a private access-only auth file, no refresh token; no sign-in or write to the user's tool directories |
| Scope limit | Process groups and observed calls do not provide containment or complete shell filesystem/read evidence |
| Tokens | Total scoped usage, conventions `codex` and `smith-events`; fresh excludes cache reads, output includes reasoning; unknown fields retain their reasons |
| Cost | Shared `agents/prices.toml` list-price calculation is notional `spend_usd/priced`; smith's own notional spend is separate; native charges unavailable |
| Statistics | Every attempt, including failures, enters available medians; exact 95% pass intervals; seeded 10,000-resample median ratios with intervals and MDEs; none detected does not establish equality |
| Audits | Session model reviewed the three seeded samples union every grader/check disagreement against complete archived diffs, calls, task and grader; finite evidence, no complete read-scope claim |

The committed evidence is below. Source identities describe measured arms; later landings and current `main` were not remeasured. Summaries preserve complete task/prompt/configuration digests and unavailable reasons.

| Step | Committed summary | Tested smith source / binary SHA-256 | Seed | Run wall, alone |
|---|---|---|---:|---:|
| 12.3 | [benchmarks/summaries/probes-f2c37fe8-20261013T214945Z.json](../../../benchmarks/summaries/probes-f2c37fe8-20261013T214945Z.json) | `f2c37fe8` / `2e50c229b106e158577c4f018567c51d7ed04ba8ce32dee2d92503fa3f99fd1e` | 1791928185450644852 | 673 s |
| 12.4 | [benchmarks/summaries/compare-a57ca3a8-20261013T221135Z.json](../../../benchmarks/summaries/compare-a57ca3a8-20261013T221135Z.json) | `a57ca3a8` / `a5dac5d7236cbcecf28fe36c498d23d3be1da925db16ad6b6ce5aca220ed6836` | 1791929495095257923 | 2391 s |
| 12.5 | [benchmarks/summaries/repository-a64f0523-20261013T230741Z.json](../../../benchmarks/summaries/repository-a64f0523-20261013T230741Z.json) | `a64f0523` / `8a8eb0c0eaa2ba25109e529d75318a545fbdec7fa46896cd35b0e2fde9c94ec9` | 1791932861735903846 | 3752 s |
| 12.6 | [benchmarks/summaries/compaction-8a9747fa-20261014T004156Z.json](../../../benchmarks/summaries/compaction-8a9747fa-20261014T004156Z.json) | `8a9747fa` / `c2c3fe9e1113985a24aa871c70b948807b0a70c91dac31d275b21d7da4422e32` | 1791938516099688948 | 322 s |

No cache-reuse fraction is reconstructed from cache-read volumes. Codex's own spend, provider completion/retry counts and child-share/blocking metrics remain unavailable. Startup headroom identity joins, teardown aborted metadata and full filesystem/read scope also remain unavailable where recorded. No same-configuration compare baseline supplied a drift basis; empty drift lists make no drift claim. The preflight's retained budget baseline has no exact match across the new arm.

Session accounting includes every earlier setup, stopped, failed and repeated record: 150 physical attempts against the revised 149-attempt basis, $7.820818 known smith notional against the $45 guide. The extra attempt is 12.s3's required smoke, explicitly accepted by the supervisor on 2026-10-14 at 00:52:04 UTC within the half-again stop. Unknown old/native charges stay unknown. No further live work ran for this report.

## What changed since `30259d8`

One row per [initial probe](../../design/benchmarks.md), section 11.1. Guarded steps are the reliability plan's names, not claims that every variant calibrated. First-run steps 1–3 stand under decisions 12.4/12.16. Session 13's recorded calibrations and exceptions stand; later passing smoke or changed task versions do not recalibrate them. Uncalibrated guards rest on their deterministic tests.

| Probe | Change / guarded steps | Recorded calibration and limit |
|---|---|---|
| first-request | Task in opening, park before messages, both product faces; smith 00.25, 10.8, 10.15; standing first-run proof includes smith 00.10, 02.2/2.4/resumed-binding, 03.1–03.6, 04.5, 05.2; skein 01.4, 04.1, 05.7 | First-run calibration stands; final preflight interactive/headless pass. Standing smoke's different configuration is uncalibrated |
| smoke-tools | Environment, `.` namespace, typed stream and teardown; smith 00.23; skein 02.4, 03.2 via teardown | First-run base/child calibration stands; no claim the whole standing smoke configuration calibrated |
| follow-up | Carried/unread lines and finish crossing; smith 00.14, 05.4/5.5/5.6/5.10/5.13 | Base 10/10, EOF 10/10, chat 10/12, but all controls passed: none calibrated (13.47); chat stall handed to 05.s1 |
| large-write | Declared payload and oversize blocks; smith 00.26, 04.3/04.10/09.13; skein 04.3 | Normal 10/11 calibrated; oversized two historical exercised passes prove guards by 12.15, no third in 13.1: 0 exercised, 9 NotExercised, 3 model deviations in 12; uncalibrated |
| history | Window checks and compaction, bounded requests; smith 00.16, 04.6/04.8/05.8, 08.6/08.10; skein 04.6 | None calibrated: base rerun passed; small-window 2 passes/3 model deviations, control failed; next task version deferred |
| cache-reuse | Domain affinity, stable prefix, optional usage; smith 04.optional-usage, 05.7, 08.1/08.3/08.4/08.5; skein 04.7/04.9/04.11 | 13.2 base/children/crossing 0/3 triplets each, none calibrated; original provider-stopped run kept; Codex cache criteria passed but answers failed; idle-gap deferred |
| budget | Reserve, clamped shares, final answer and visible budget; smith 07.1/07.2/07.6/07.7/07.8/07.9/07.10/07.11/07.12/07.14; skein 04.2 | Starvation 10/10, wrap-up 10/11, child-quota 10/10 calibrated (13.3). Spend refused before inference: $1 below $1.32 reserve, uncalibrated. Visibility raw 0/0/3 unjudged, told 3/3; reserve observation gap, uncalibrated (13.72) |
| parallel-read | Calls per response and pool bounds; smith 04.12; skein 05.6/05.8 | 13.4 7/10, three wrong sums; uncalibrated |
| parallel-children | Concurrent conversations, capacity waiting, write lease; smith 04.12/07.5; skein 03.3/03.5 | 13.4 children 2/5; reviews 1/4, two deviations and one no-nudge stall handed to 05.s1; neither calibrated |
| failures | Typed failures, clipped detail, refusals and headless exits; smith 00.14 (first-run step 3), 06.2/06.3/06.6, 10.5 | First-run four variants calibrated; expected startup/one-turn refusals stay apart from pass rates |
| paths | One namespace, aliases and one-directory guidance; smith 09.4/09.5/09.6/09.7/09.9 | 13.5 base 1/4, single 0/3; uncalibrated; final correct file/path checks accompany extra completions |
| tool-output | Output limits, paging, UTF-8, emitted timeout verdict; smith 00.15, 04.1, 09.3/09.8/09.10/09.12 | 13.5 test-output/quote/utf-8 10/10 each calibrated; timeout 0/3 projection defect handed to 04.s2. Final v3 passes but uncalibrated under 13.130; old summaries unchanged |
| process-tree | Whole command group stopped, timeout evidence; smith 00.15, 09.11; skein 01.1/01.2/tree-termination/tree-subreaper-restoration | 13.5 10/11 calibrated on the timeout-projector arm; final v2 uses product's 1 s group_stop, passes but uncalibrated under 13.130 |
| credentials | Read-only borrowed login; smith 00.30/10.7; skein 06.2/06.3 | 00.30 calibrated 10/10 on `c7b79bf3`; login digest and answer decide; final preflight passes |
| slow-completion | Progress deadlines and reasoning item bounds; smith 04.4/04.11; skein 03.1/04.4/04.12 | 13.6 historical 3 Pass, 2 NotExercised, 1 silence failure handed to 06.s2 (13.85), plus two earlier skein defects; none calibrated. Later 06.s2's 18 passes do not replace that record |

## Probes: 12.3

The final whole probes run dispatched all 46 catalogue entries: 52 physical records, 40 summary groups, logical 27 pass / 13 fail / 3 unavailable / 3 setup; physical ends 48 Completed / 1 Failed / 3 Refused. It took 673 s alone (676 s task), exit 2 because evidence remained incomplete. No calibrated probe failed twice; no unexpected provider/setup dispatch stop occurred. Only authored failure reruns ran. Audits selected 23, 27-rerun and 35; no extra calibration or joint/remainder summary.

| Final evidence | Finding |
|---|---|
| First-request, ordinary budget variants, credentials, follow-up, smith smoke-tools/large-write | Passed |
| 27 small-window / rerun | Fail then pass; quarantined; earlier malformed/double-failure history retained |
| 29 Codex large-write / rerun | File digest/size fail then pass; own spend and provider completions unavailable |
| 31 parallel-children, 33 parallel-read | Each failed twice: no overlap / wrong answer; uncalibrated, gates nothing |
| 34 paths, 35 paths/single | Each failed twice at three completions; correct path/file checks passed; uncalibrated |
| Cache triples / 16 budget visibility | Cache evidence / told-boundary evidence unavailable as recorded; children 09/10 also wrong answers |
| 30 oversized | Oversized-write unavailable, blocks false; historical two exercised passes retained |
| 17/18 bad-setting/derived-limit, 19 one-turn, 20 unknown-model | Expected refusals / HTTP 400 typed failure, no invented usage |
| Process-tree v2; tool-output v3 | All passed, remain uncalibrated; no comparison across task versions |

The standing smoke `smoke-62acc437-20261013T211935Z` kept six records, four pass/two fail, 93 s alone and $0.177455 smith notional. First-request used seven/four completions, the second also answered with the wrong file. The 21:23:26 UTC ruling calls this uncalibrated smoke configuration: it gates nothing; the earlier `858c9642` smoke double failure stands too.

The stopped `probes-62acc437-20261013T212404Z` kept 39 records, 549 s and $1.123296, ending at setup 36's duration pin. Large-write 28 failed then passed (quarantined), small-window 27 failed twice with a malformed terminal, paths/single 35 failed twice. Initial `0305f2fd` setup failures also remain. Decision 13.130 commits only the final whole probes summary; older records and summaries were neither replaced nor rejudged.

## Comparison: 12.4

One 30-attempt suite, five per agent per fixture, interleaved, 2391 s alone (2394 s task), no reruns. All 30 Completed, exit 0, forced=no, grader=true; all overall fail solely because `tests/regression.rs` is protected. Each of six groups has 0/5 passes, exact 95% interval [0, 0.5218237501049814], no setup/budget/unavailable rate entries. All failed attempts enter the following medians (n=5).

| Fixture / agent | Task s | Process s | Fresh | Cache read | Output | Reasoning | Common priced USD |
|---|---:|---:|---:|---:|---:|---:|---:|
| 01 / Codex | 49.878 | 50.335 | 10317 | 69120 | 1769 | 224 | 0.038383 |
| 01 / smith | 50.107 | 50.119 | 6914 | 13696 | 1693 | 273 | 0.028686 |
| 02 / Codex | 87.285 | 87.683 | 11869 | 110592 | 3768 | 377 | 0.0645865 |
| 02 / smith | 100.571 | 100.582 | 16669 | 22784 | 4230 | 854 | 0.06407425 |
| 03 / Codex | 83.179 | 83.648 | 9085 | 112896 | 3611 | 183 | 0.06287925 |
| 03 / smith | 92.206 | 92.216 | 16413 | 27264 | 3838 | 459 | 0.061351 |

All 24 comparisons follow. Ratios are smith/Codex, token conventions [codex, smith-events]; numbers rounded to five decimals, exact values in the committed summary. MDE is a relative effect. Cache-read effects are volumes, never reuse fractions.

| Fixture | Metric | Ratio | 95% interval | MDE | Effect |
|---|---|---:|---|---:|---|
| 01 | task wall | 1.00459 | [0.82541, 1.12689] | 0.17823 | None detected |
| 01 | process wall | 0.99571 | [0.81818, 1.11729] | 0.17816 | None detected |
| 01 | fresh | 0.67016 | [0.20213, 1.17280] | 1.27390 | None detected |
| 01 | cache read | 0.19815 | [0.17742, 0.25806] | 0.20282 | Detected |
| 01 | output | 0.95704 | [0.83041, 1.17167] | 0.19714 | None detected |
| 01 | reasoning | 1.21875 | [0.56696, 1.83258] | 0.78461 | None detected |
| 01 | common priced spend | 0.74736 | [0.41626, 0.86417] | 0.56951 | None detected |
| 01 | cache write | — | Reference zero can make a bootstrap ratio undefined | — | Unavailable |
| 02 | task wall | 1.15221 | [0.91859, 1.26799] | 0.17000 | None detected |
| 02 | process wall | 1.14711 | [0.91508, 1.26171] | 0.16999 | None detected |
| 02 | fresh | 1.40441 | [0.65602, 1.67212] | 0.68100 | None detected |
| 02 | cache read | 0.20602 | [0.12384, 0.24518] | 0.42802 | Detected |
| 02 | output | 1.12261 | [0.91697, 1.26194] | 0.22045 | None detected |
| 02 | reasoning | 2.26525 | [1.09068, 3.16092] | 0.67652 | Detected |
| 02 | common priced spend | 0.99207 | [0.70246, 1.13590] | 0.31604 | None detected |
| 02 | cache write | — | Reference zero can make a bootstrap ratio undefined | — | Unavailable |
| 03 | task wall | 1.10852 | [1.02292, 1.21846] | 0.09895 | Detected |
| 03 | process wall | 1.10243 | [1.01782, 1.21197] | 0.09895 | Detected |
| 03 | fresh | 1.80660 | [1.17336, 2.22675] | 0.38837 | Detected |
| 03 | cache read | 0.24150 | [0.13039, 0.29085] | 0.45344 | Detected |
| 03 | output | 1.06286 | [0.99371, 1.17170] | 0.10933 | None detected |
| 03 | reasoning | 2.50820 | [1.75410, 3.50376] | 0.39653 | Detected |
| 03 | common priced spend | 0.97570 | [0.86302, 1.13331] | 0.17571 | None detected |
| 03 | cache write | — | Reference zero can make a bootstrap ratio undefined | — | Unavailable |

Under decision 13.131, all 30 IDs were audited: 01–10 backoff, 11–20 decoder, 21–30 planner. Every complete diff preserves original behavior/API assertions and adds requested regression tests. Visible instructions ask for regressions and prohibit weakening tests, without naming the tests directory's protection. Task audits support the implementations within finite grader/source evidence; scope/overall audits fail due to protected edits. No grader read is visible in recorded calls; full shell effects/read scope are unavailable. No checker, fixture or verdict was repaired. No agent ranking follows from these effects.

## Repository task: 12.5

`shell-cwd` is frozen at smith `56c0c59`, with skein `f5fca44` vendored in its bundle. Six serial attempts ran once in order 04/01/05/02/06/03, 3752 s alone (3755 s task), each graded after exit; 1200 s process deadline and at most 180 s grading. All Completed, exit 0, forced=no. Both overall pass counts are 0/3, exact 95% interval [0, 0.7075982261787133]; Codex grades 3/3, smith 2/3. No setup/budget ends. All six audits (01–06) are complete under 13.132.

| ID | Agent | End | Grade | Overall | Main turns | Children | Parent blocked s | Task s |
|---|---|---|---|---|---:|---:|---:|---:|
| 01 | smith | Completed | Pass | Fail | 50 | 0 | 0 | 764.605 |
| 02 | smith | Completed | Pass | Fail | 36 | 2 | 178.195 | 728.470 |
| 03 | smith | Completed | Fail | Fail | 35 | 3 | 131.170 | 690.709 |
| 04 | Codex | Completed | Pass | Fail | Unavailable | Unavailable | Unavailable | 432.852 |
| 05 | Codex | Completed | Pass | Fail | Unavailable | Unavailable | Unavailable | 421.755 |
| 06 | Codex | Completed | Pass | Fail | Unavailable | Unavailable | Unavailable | 380.045 |

Smith 03 rejects an original 129-byte absolute spelling whose normalized path fits the inclusive 128-byte bound; the oracle passes 16/17 checks. Formatting, clippy, 1482 focused and 17 fuzzy tests passed. The failed grade stays in every statistic.

Every protected check flags Cargo.toml, Cargo.lock, .config/nextest.toml, AGENTS.md, vendor and .cargo; none appears as changed in any full diff. The harness snapshots the manifest-only task seed, then compares it with the selected bundle workspace ([run.rs](../../../benchmarks/src/run.rs), preparation and afterwards): bundle files' presence becomes a change. The prompt forbids protected changes. All raw flags and verdicts remain; no task/checker/score correction. Diffs do not establish every ignored file's contents. Audits support five visible implementations; 03 violates the inclusive edge. Old Shell constructors/assertions remain, while existing story/fuzz coverage changes are recorded in the audits.

Every attempt issued absolute `/tmp` log targets; Codex 06 also selected `/tmp/smith-shell-cwd-target` outside its private cache. Prior target contents and actual cache reuse are unknown. No grader/private-credential read is visible; full resolved reads and shell filesystem effects remain unavailable. Opaque collaboration payloads were not decoded in these audits.

All permitted cross-agent metrics have n=3 per arm. A large effect under 13.132 means disjoint observed min/max ranges only; no significance claim or bootstrap comparison at n=3. Medians [ranges] include all failures; token conventions Codex / smith-events.

| Metric | Codex median [range] | smith median [range] | Disjoint ranges |
|---|---|---|---|
| Task s | 421.755 [380.045, 432.852] | 728.470 [690.709, 764.605] | Yes |
| Process s | 422.139 [380.416, 433.107] | 728.505 [690.730, 764.654] | Yes |
| Fresh tokens | 264452 [226875, 304560] | 380427 [254090, 408176] | No |
| Cache-read tokens | 6930048 [6111232, 8825600] | 2483328 [2185088, 2885376] | Yes |
| Cache-write tokens | 0 [0, 0] | 0 [0, 0] | No |
| Output tokens | 21757 [21209, 26300] | 19856 [17959, 21716] | No |
| Reasoning tokens | 3657 [3135, 4868] | 2905 [1655, 3079] | Yes |
| Common priced USD | 1.408911 [1.26506775, 1.7469] | 0.96582975 [0.8076185, 1.069452] | Yes |

Smith delegation medians [ranges]: main turns 36 [35, 50], children 2 [0, 3], budget-ended children 0 [0, 1], parent blocked 131.170 [0, 178.195] s. Attempt 02 had one End child and one answered Budget child; 03 had three End children. Codex reports no child shares or parent blocking, so all four delegation metrics are unavailable. No cross-agent delegation/tool/resource/teardown comparison.

These benchmarks grant smith `sub_agents=true`, 4096 turns, 1200 s, $1000 notional, reserve 2 turns/300 s. They do not change [07's recommendation](sub-agents-by-default.md): keep the local default off. Its evidence and historical reviews stall remain as recorded; user approval and a design/default change would be needed to enable it.

## Experiments: 12.6

| Experiment | Run or reason it did not run | User decision / deferred item |
|---|---|---|
| Compaction threshold | Small-window v1, 65536-token window; two override arms, 4/5 (80%) and 3/5 (60%); five each, interleaved once | User decision pending; shipped 80% and task/summary/pending-host-message contents stand. Supervisor recommended retaining them at 00:52:04 UTC; malformed-after-compaction reliability work remains |
| New-window contents | No landed setting for a contents override; no arm ran | No contents-policy comparison; provider-native compaction remains deferred in `later.md` |
| Affinity | First two configuration arms unavailable: domain owns affinity and configuration refuses affinity headers; third routing/reasoning-state arm not built | Decisions 4.4: routing extras require material measured gain; extras/WebSocket remain deferred |
| Cache lifetime | Anthropic-only experiment belongs to next pass; idle-gap delay not supplied | No TTL decision; longer lifetime remains unmeasured |
| Edit format | Each dialect currently has one format; no alternative-setting arm | Decision 7.4 needs measurement before choosing; dialect alternatives remain deferred |

The compaction suite ran exactly ten attempts once, order 06/01/07/02/03/08/09/04/05/10, 322 s alone (326 s task), exit 1. Prepared settings and recorded startup fractions match both arms. No provider-native compaction was measured. No policy, pin, task, product or statistics changed; later data landing `20ab17e4` was not the measured source.

| ID | Arm | End / outcome and finding | Reads | Compactions |
|---|---|---|---:|---:|
| 01 | 80% | Completed/fail: ALPHA528 instead of ALPHA 528 | 33 | 2 |
| 02 | 80% | Failed model/malformed: compaction finish lacks fields | 32 | 2 |
| 03 | 80% | Failed model/malformed: compaction finish carries prose | 32 | 2 |
| 04 | 80% | Completed/fail: ALPHA136, early finish after summary | 16 | 1 |
| 05 | 80% | Completed/pass: ALPHA 528 | 32 | 2 |
| 06 | 60% | Completed/pass: ALPHA 528 | 32 | 3 |
| 07 | 60% | Failed model/malformed: compaction read call | 30 | 3 |
| 08 | 60% | Completed/fail: ALPHA 489 | 32 | 3 |
| 09 | 60% | Failed model/malformed: compaction read call | 10 | 1 |
| 10 | 60% | Failed model/malformed: compaction read call | 10 | 1 |

All ten exit 0, forced=no; no file diff, tool failure, delegation, setup, budget or timeout. Every deciding prompt-within-window and conversation-opened check passes. Typed Failed terminals and short failure durations stay in the summary.

| Arm | Passes / exact 95% interval | Ends | Task s median [range] | Process s median [range] | Compactions median [range] | Common priced USD median [range] |
|---|---|---|---|---|---|---|
| 80% | 1/5 [0.005050763379468026, 0.7164179361180896] | 3 Completed / 2 Failed | 39.073 [19.859, 43.588] | 39.088 [19.869, 43.608] | 2 [1, 2] | 0.21814025 [0.0725265, 0.237811] |
| 60% | 1/5, same interval | 2 Completed / 3 Failed | 35.318 [10.263, 42.537] | 35.327 [10.273, 42.543] | 3 [1, 3] | 0.18371625 [0.02916575, 0.185733] |

All 19 comparisons are retained below: 11 observed, none detected; eight unavailable. Ratio direction 80%/60%, rounded to five decimals; these are same-agent comparisons.

| Metric | Ratio | 95% interval | MDE | Effect / reason |
|---|---:|---|---:|---|
| Task wall | 1.10632 | [0.56229, 3.80717] | 0.97515 | None detected |
| Process wall | 1.10646 | [0.56243, 3.80493] | 0.97479 | None detected |
| Fresh | 1.03118 | [0.19156, 14.94637] | 1.18311 | None detected |
| Cache read | 1.70441 | [0.96471, 5.06114] | 0.83241 | None detected |
| Output | 1.05207 | [0.59884, 4.22605] | 1.11972 | None detected |
| Common priced spend | 1.18738 | [0.39477, 7.47933] | 1.07995 | None detected |
| Own notional spend | 1.18736 | [0.39479, 7.47886] | 1.07994 | None detected |
| Teardown | 1.33333 | [0.80000, 2.33333] | 0.63329 | None detected |
| CPU, pidfd-walk | 1.18512 | [0.62069, 3.49735] | 0.89962 | None detected |
| Main turns | 1.03125 | [0.56250, 3.50000] | 0.97176 | None detected |
| Read calls | 1.06667 | [0.53333, 3.20000] | 0.90977 | None detected |
| Children; budget children; parent blocked; cache write; reasoning (five comparisons) | — | — | — | Reference zero can make a bootstrap ratio undefined |
| Grading wall; peak RSS/largest-process; finish calls (three comparisons) | — | — | — | Comparison needs observed values for every attempt, regardless of end |

Seeded compaction audit IDs 02/06/08 remain in the summary; this experiment required no extra formal comparison audit. New compaction attempts cost $1.615145 observed smith notional. Failed attempts were retained, with no retries or repair. This evidence supplies no basis for changing contents or adopting provider-native compaction; the pending user decision does not hold the data or report landing.

## What waits for the next pass

[later.md](../plans/reliability/later.md): Claude Code (outside this plan), smith on Anthropic, cache-lifetime/idle-gap, next versions of uncalibrated probes, and the deferred affinity, edit-format and compaction items above.

## Erratum to the 2026-10-08 report

The historical report stays at `49057ae`; its former `crates/smith` links refer to that tree. These corrections follow decisions 5.11 and findings D12.

| Historical statement | Correction |
|---|---|
| “Shared 64-turn budget” row credits repository successes to the 256-turn ceiling | Both successes used fewer than 128 turns; the failure ran out of time with turns unused. The mechanism was starvation: a child with no share took the remaining budget, main had no finishing reserve, and models were not told budget or time. Raising the ceiling does not explain those successes |
| Capped children continued the repository work | Two of the five capped children ended unanswered, with their work lost |
| Serial writable delegation | Stands: it was the authority/scheduling policy |
| Endpoint cache-key/session-header recipe | Replaced by affinity in the domain: one run-tree key, per-conversation thread identity, fixed prefix; decisions 4.1–4.3 remove the static endpoint recipe |
