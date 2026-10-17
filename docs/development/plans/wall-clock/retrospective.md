# Retrospective: wall-clock

2026-10-17. The run of smith's `wall-clock` plan on debian-16gb-hel1-1.
It was claimed at 2026-10-16T12:38:47Z. The coordinator released it at
2026-10-17T17:38:57Z. The supervisor ran the "Done when" commands from
17:40 to 17:43 and closed `done.md` at 17:43:16. It was the first run
under the parity retrospective's decisions (skein `30bc10f`), and the
first with a director acting for the user. It was written by a fresh
session that took no part in the run (development.md, section 15).

Eleven read-only sub-agents, on Sonnet, one per source, read:
- the plan, `decisions.md` (rulings 101 to 128), `later.md`,
  `findings/` and the plan's git history;
- the ledger, `log.md`, the 48 reports, `context.md`, `sessions/` and the
  run's small files;
- `supervision.md`, `done.md` and the director's messages;
- smith's git history with its trailers, and the summaries they name;
- the code the run added, for its debts;
- `logs/heavy.tsv`, the queue's task logs and pueue's own record;
- the coordinator's Codex rollout;
- the 25 implementer rollouts;
- the supervisor's nine Claude Code transcripts and their 20 sub-agents;
- the designer's five sessions, the planner's session with its 11
  sub-agents, and the design's session with its four;
- skein's history during the run, and the earlier runs' retrospectives
  and metrics.

This session read:
- the plan's README, `later.md`, `findings/causes.md` and
  `findings/rt-power.md`;
- `done.md`, `status.md`, `context.md` and `claude.tsv`;
- development.md at skein `7e8fb70` and at `main`;
- the parity retrospective;
- the baseline's and the closing run's summaries, by arm.

Times are UTC; a time without a date is on the day last named. Costs are
at Opus 5.5's list prices, which `plan metrics` also applies to Sonnet's
tokens (section 3). This document proposes; the user, or the director,
decides, and the designer applies what is adopted.

**Which process.**
- This machine's skein checkout stayed at `7e8fb70` from 10-16 12:38:04
  to the end. It gave the run its process and its kit, and the
  coordinator's Codex profile is a link into it. `30bc10f`, parity's
  adoptions, is in it.
- Of the changes to the process that landed during the run, two parts
  reached it:
  - the director's relay at 17:51: Sonnet sub-agents (`ce02728`), and a
    handover at 150K;
  - `ce02728`'s proposal 4, passed on by a steer at 03:29 on 10-17.
- `c0268a8` (the coordinator compacting at about 200K) and `7779af1` (a
  resume that loads the thread) did not reach the run. `e6fd9fb`,
  `71f6c38`, `1b83a43` and `2f317f5` say they leave it as it was.
- Section 7 judges what was in force.

## 1. In one page

- **Done in 29.1 h, on par with Codex on all five criteria at the first
  closing run.**
  - 20 steps landed: 19 planned, and 02.4, which its implementer split
    off 02.3. 11.2, the second closing run, was dropped as not needed.
    No step was added.
  - Every "Done when" item holds.
  - The closing run, `parity-a87ba689`: O 34, FT 1.067, FC 0.825, RT
    1.033 [0.892, 1.180], RC 1.174. The baseline had missed FT (1.178),
    RT (1.397) and RC (1.407).
- **RT passed with 0.02 to spare, and part of each move came from
  Codex's arm.**
  - On `shell-cwd`, smith's responses fell from 70.1 to 54.2 an attempt,
    while Codex's went from 45.5 to 48.3. That is what the plan changed.
  - smith's median `shell-cwd` time barely moved, from 620.0 s to
    609.7 s. Each response took longer in the closing run, for both
    arms. Codex's median went from 443.6 s to 590.4 s.
  - By `findings/rt-power.md`, a smith exactly as fast as Codex passes
    RT in 60% of runs at ten attempts.
- **The four change runs decided nothing the verdict read.**
  - 07.1 to 10.1 each ran a `parity-change`: 9.3 h of the gate lane, 164
    attempts and $69.85.
  - Their RT intervals were 0.34 to 0.78 wide, and three of the four
    included no change at all. The causes they removed were worth 4% to
    22% of smith's time (`causes.md`).
  - Two of them stopped on RC, and the designer kept both.
  - At 10.1 the projection put RT at 1.513 and RC at 1.821. The closing
    run measured 1.033 and 1.174.
- **Live suites were the run.**
  - The gate lane ran live work for 23.1 of the 29.1 hours.
  - The first 14 steps landed in 2.4 h. Each of the other six waited on
    a live suite, and together they took 26.6 h.
- **06.1's baseline held the gate lane for 8.1 h, 3.1 h of which
  measured nothing.**
  - At 15:16:57 the coordinator removed session 06's `target/`, as
    development.md 11.4 tells it to when a session yields. The baseline
    was running from that directory and died 33 seconds later. The rule
    is still on `main`.
  - Its rerun stopped at 58 of 68 attempts, on one request the provider
    never answered, and the baseline ran a third time.
- **The parity retrospective's rules held.**
  - No question waited inside a tool, and `blocked_h` was 1.0, against
    7.3.
  - There was no disk or queue event, and no step was added.
  - The implementers' peak contexts had a median of 101K, against 201K.
    Their tokens a step halved, to 7.2M from 13.4M.
- **23.6M tokens a step, as in parity, but Claude Code is now the
  largest part.**
  - Claude Code in the run took 243M tokens, 12.2M a step, against the
    implementers' 7.2M and the coordinator's 4.1M.
  - The supervisor cost $95 over 29 h, 62% of it in sub-agents: $3.3 an
    hour, as in parity.
- **Eight proposals** (section 8). The first two change how a verdict's
  changes are measured and when a session's `target/` goes. Proposals 7
  and 8 change the kit.

## 2. Outcome

The supervisor ran the README's "Done when" once, from 17:40 to 17:43,
on smith `main` at `92f9494d` (the smoke's arm at `7ef5790a`, which adds
only `STATUS.md`'s `done`). Its commands and outputs are in `done.md`.

| # | Item | Result |
|---|---|---|
| 1 | Every step landed or dropped | Holds: 20 landed; 11.2 dropped, "dropped: 11.1 on par". |
| 2 | smith is on par with Codex at the plan's final `main` | Holds: exit 0, offline, on `parity-a87ba689-20261017T123849Z`. No code commit of the plan landed after its smith arm. |
| 3 | The smoke passes on `main` | Holds: `build HEAD` at `7ef5790a`; `smoke-tools` passed, 1 attempt, 4.3 s, $0.0004. |
| 4 | Each change to what the model is sent has its report | Holds: four reports, none missing or refused. 09.1's and 10.1's print RC's stop, kept by rulings 125 and 128. |
| 5 | Each format moved once | Holds: events 3, transcript 5, charter 2, channel 2, result 4, summary 4, baseline 1. |
| 6 | Both gates within their budgets | Holds: 2,356 tests in 4.992 s and 43 in 5.204 s, against 15 s and 60 s. |
| 7 | Each session's "Done when" | Holds for sessions 01 to 11. |
| 8 | Only the change steps changed the prompt's files | Holds: 03.1, 04.1, and 07.1 to 10.1. |
| 9 | The Don'ts a command can read | Holds: no wall-clock commit in skein, jig or temper; the profile's quantities untouched; no other probe's words in the prompt. |

**The verdict** (statistic, interval, and whether it holds). The margin
is 1.20 for FT, FC and RT, and 1.50 for RC.

| Criterion | Baseline, 06.1, `parity-c10edbe5` | Closing, 11.1, `parity-a87ba689` |
|---|---|---|
| O | 34 [0.763, 0.981], holds | 34 [0.726, 0.967], holds |
| FT | **1.178 [1.110, 1.251], fails** | 1.067 [1.000, 1.135], holds |
| FC | 0.969 [0.883, 1.060], holds | 0.825 [0.763, 0.890], holds |
| RT | **1.397 [1.149, 1.636], fails** | 1.033 [0.892, 1.180], holds |
| RC | **1.407 [1.087, 1.734], fails** | 1.174 [1.042, 1.444], holds |

**`shell-cwd` by arm,** ten attempts each, from the two summaries'
attempts (`responses`, `task_wall_ms`, `priced_usd`):

| | smith, baseline | smith, closing | Codex, baseline | Codex, closing |
|---|---|---|---|---|
| Responses an attempt, mean | 70.1 | 54.2 | 45.5 | 48.3 |
| Task wall, median | 620.0 s | 609.7 s | 443.6 s | 590.4 s |
| Seconds a response, mean | 8.9 | 11.5 | 10.1 | 12.3 |
| Spend, median | $1.552 | $1.412 | $1.103 | $1.203 |
| Passed | 10 | 10 | 7 | 6 |

- **What smith changed:** its responses fell by 23%, and the ratio to
  Codex's from 1.54 to 1.12. That is what causes 1, 3 and 4 aimed at.
- **What the closing run changed:** each response took 22% to 30%
  longer, for both arms. smith's time held at about 610 s, and Codex's
  rose by a third. RT reads the two arms of one run, so it followed the
  response ratio.
- **The fixtures moved the same way:** smith's median fixture wall went
  from 83.6 s to 80.4 s, and Codex's from 72.9 s to 76.3 s.
- **The pass was close.** RT's upper bound, 1.180, is 0.02 below its
  margin. By `rt-power.md`, at ten attempts a smith with Codex's true
  time passes in 60% of runs, and one 5% faster in 84%. The plan
  allowed two closing runs (11.2), and needed one.

**The four change runs,** each smith against its parent, ten `shell-cwd`
attempts an arm. A projection carries the step's ratio onto the
baseline's.

| Step | RT ratio | RC ratio | RT projection | RC projection | Stop |
|---|---|---|---|---|---|
| 07.1, the window's check (cause 2) | 0.815 [0.650, 0.989] | 0.926 [0.698, 1.115] | 1.138 | 1.303 | none |
| 08.1, batched change (causes 1, 3) | 1.014 [0.719, 1.496] | 1.021 [0.839, 1.265] | 1.154 | 1.331 | none |
| 09.1, polls (cause 4) | 1.149 [0.801, 1.358] | 1.393 [0.892, 1.664] | 1.326 | **1.853** | RC, kept (ruling 125) |
| 10.1, ranged reads | 1.142 [0.880, 1.324] | 0.983 [0.876, 1.350] | 1.513 | **1.821** | RC, kept (ruling 128) |

The steps' own RT ratios multiply to 1.083. The closing run measured RT
at 1.033 against the baseline's 1.397.

## 3. Numbers

`plan metrics` (the kit at `7e8fb70`) ran without error, and wrote
`metrics.tsv`. The comparisons:
- parity, the last retrospective's run: smith, this machine, 58 steps
  (its corrected column);
- skein-ui: jig and skein, 75 steps;
- forgejo-17, the most recent: temper, 11 steps.

Per-step numbers compare better than totals, but this run's last six
steps were each a live suite of 2 to 8 hours.

| Metric | Printed | Corrected | parity | skein-ui | forgejo-17 | Notes |
|---|---|---|---|---|---|---|
| `wall_h`, `running_h` | 29.1, 5.6 | 29.1 | 33.8 | 8.1, 7.6 | 2.4 | `running_h` is **not comparable** (below). The gate lane ran live work for 23.1 h. |
| `steps_planned`, `added`, `dropped` | 20, 0, 1 | 19, 1, 1 | 45, 13, 0 | 70, 5, 0 | 9, 2, 0 | 02.4 was split off 02.3 by its implementer (rule 1). 11.2 was dropped, as not needed. |
| `step_h` | 0.28 | 1.45 wall hours a step | 0.58 | 0.10 | 0.18 | The chain was serial by rule 11, each link a live run. |
| `escalations` | design gap 5, infrastructure 1, plan defect 3, process 3 | 11: design gap 4, plan defect 3, process 3, infrastructure 1 | 36 | 9 | 3 | 0.55 a step, against 0.62. 10.1's two blocks are one stop. |
| `escalation_wait_min` | 2.3 | | 2.5 | 4.3 | | From the watch's print to the ruling: a median 1.3 minutes without the designer, and 11.5 with. |
| `blocked_h` | 1.0 | | 7.3 | 1.3 | 0.6 | |
| `answers_reversed` | 0 | | 1 | 0 | | |
| `steers` | 10 sent, 1 point refused | 10 sent, 18 points, none refused | 12 points | 6 | | The "refused" point could not apply: 05.2 had landed at the steer's own second (ruling 103). |
| `restarts` | none | no restart; 3 loads by hand | 10 loads | none | | |
| `tokens_coordinator` | 82.6M in (97% cached), 142K out | | 233.1M | 114.1M | | 4.13M a step, against 4.02M and 1.52M. |
| `tokens_implementers` | 143.4M in (97%), 446K out | | 779.1M | 219.6M | | 7.17M a step, against 13.4M and 2.93M. |
| `tokens_claude` | 511.6M in, 3.49M out: 96.2M before the claim, 243.2M in the run, 172.2M after | | 121.2M, 315.5M, 128.9M | 83.0M, 247.6M, 2.4M | | 12.2M a step in the run, against 5.44M and 3.30M. An hour: 8.4M, against parity's 9.3M. |
| `cost_claude` | $233: planner $44, supervisor $95, designer $18, retrospective $76 | design and planning $68 | $231 | $149 | $174 | Section 3's notes. The run's part, $113, is $3.9 an hour, as parity's $133 was. |
| `tokens_per_step` | 23.6M | | 22.96M | 7.78M | 19.13M | |
| `coordinator_requests` | 461, 26 after a wait ran out | 457 | 1,169, 48 | 589, 32 | | 51 requests saw only a wait's end and the wake-up check: 9.3M, 11.3% of its input. |
| `compactions` | 1 | | 2 | 1 | | At 371K, 10-17 12:30, down to 16K. |
| `gates_per_step` | 1.2 | | 1.84 | 1.3 | 1.4 | 24 gates, none failed. Four ran again after `main` moved, all before 14:19. |
| `gate_lane_other` | 30 | 37 | 103 | 39 | | `heavy.tsv` lacks seven live suites (section 5, item 9). |
| `queue_wait_ratio` | 0.97 | 0.17; the build lane alone 1.35 | 0.74 | 0.65 | 0.29 | 89% of the build lane's waiting was behind a gate-lane task, 81% behind a live one. |
| `suites` | smith: 2,262 / 5.25 s → 2,356 / 4.99 s; 42 / 5.17 s → 43 / 5.20 s | | | | | Budgets 15 s and 60 s. |
| `proofs` | world 3, process 5, live 7; 0 below plan | and offline 5 | | | | "offline" (02.1 to 02.4, 03.3) is not a layer's word. forgejo-17's proposal 8, adopted after this plan was written, makes a Proof line start with one. |
| `report_lines` | 11 median, 18 p90 | | 11, 16 | 10, 11 | | 48 reports, the longest 25 lines. |
| `lines` | code +106,583 −67,728; tests +4,547 −1,681 | production Rust +1,955 −420, and +665 −23 generated; Rust tests +3,940 −1,160; data +97,847 −61,123; Markdown +3,719 −92 | | | | **Not comparable:** the metric counts the benchmarks' JSON as code. |
| `contexts` | 101K median, 199K p90; 7 handovers | | 201K, 293K; 26 | 129K, 237K; 2 | 113K, 182K; 2 | No thread began an increment past 150K. |
| `pin_lag` | smith: skein 128; jig: skein 56, smith 191; temper: skein 151, smith 260, jig 303 | | | | | The plan repinned nothing (its Repin: none). `b415f737`, the render plan's, repinned smith's skein three minutes after the run. jig's and temper's numbers come from stale checkouts here. |
| `supervisor` | not printed | 9 sessions; main threads $36.72; sub-agents $58.72 | | | | Section 15.2 gained it on `main` (`71f6c38`), after this kit. |

**Not comparable, and why:**
- **`running_h`.** It sums the coordinator's events, less every gap over
  ten minutes. The coordinator idled through each live suite, so 23.8 h
  of the run drop out: its turns took 5.2 h.
- **`gate_lane_other` and `queue_wait_ratio`.** `heavy.tsv` has no row
  for seven of the gate lane's live suites, 21.8 h of running (section
  5, item 9). The corrected values add them from pueue's record.
- **`cost_claude`:**
  - The planner's $44 is session `7f4cbcdf` with its 11 sub-agents.
  - The metric does not count the design's own session, `9e61c571`, and
    its four sub-agents ($23.76). They are named by the design commit's
    `Designer-Session:` trailer, not by a `Planner-Session:`.
  - Design and planning came to $67.91. The user's analysis session
    before them (`3ab477cd`) adds $9.12.
  - The supervisor's twelve sub-agents after 17:51 ran on Sonnet, and the
    metric prices their tokens at Opus's rates: $37.45 of its $95.
  - The retrospective's figure is this session's and its eleven readers'
    when `metrics.tsv` was written, all at Opus's rates.
- **`lines`** counts JSON, JSONL and TOML as code. 6 run summaries added
  and 15 removed make most of it.
- **`pin_lag`** reads each checkout's `Cargo.lock`, not origin's.

**Where the 29.1 hours went:**

| Span | Hours | Steps landed | What held it |
|---|---|---|---|
| 10-16 12:38 to 15:04 | 2.4 | 14 | Sessions 01 to 05 at once. 01.3's pipeline run held the gate lane for 37 minutes. |
| 15:04 to 23:47 | 8.7 | 1 | 06.1's baseline died at 15:17 when its `target/` was removed. Its rerun stopped at 18:32 on a request the provider never answered. The baseline that counted ran from 18:41 to 23:12, then its small change run and a design question. |
| 23:47 to 10-17 02:29 | 2.7 | 1 | 07.1's change run, 2.35 h. |
| 02:29 to 06:26 | 3.9 | 1 | The coordinator idled 59 minutes on a re-check that could not build. A plan defect (4 min) and a design question (11 min) followed, with the live run held until 04:13. 08.1's change run took 2.15 h. |
| 06:26 to 09:42 | 3.3 | 1 | 09.1's re-check (17 min), its change run (2.38 h), and its RC stop (15 min). |
| 09:42 to 12:35 | 2.9 | 1 | 10.1's change run (2.43 h), and its RC stop (13 min). |
| 12:35 to 17:37 | 5.0 | 1 | The closing run, 4.89 h. |
| 17:37 to 17:43 | 0.1 | — | The release and the "Done when". |

**What explains the largest differences:**
- **Wall hours a step, 1.45, against 0.58 and 0.10:**
  - **The chain was live:** 06.1, then 07.1 to 10.1 one at a time (rule
    11), then 11.1. Its live runs took 22.3 h as run, against the
    README's estimate of 24.5 h.
  - **06.1's lost runs:** 3.1 h (section 5, item 2).
  - **The four change runs:** 9.3 h (section 5, item 1).
- **The implementers' tokens a step, 7.17M, against 13.4M:**
  - **Contexts:** no thread began an increment past 150K, and seven
    fresh threads replaced idle ones. The median peak fell from 201K to
    101K, and requests past 200K took 9.2% of their input.
  - **Waiting** fell from 37% of their input to 26% (section 5, item 6).
  - **Nine threads landed nothing:** they launched, yielded on, or asked
    about a live step, and took 34.5% of the input.
- **Claude Code a step, 12.2M, against 5.44M:**
  - It is the same per hour (8.4M against 9.3M), over a third as many
    steps.
  - The supervisor's sub-agents read the most: 112M tokens of cache
    reads, against its own threads' 93M.
  - Ready checks and re-checks cost $32.56, and hourly checks $26.16 in
    sub-agents (section 5, item 8).
- **`blocked_h`, 1.0 against 7.3:** no stop like parity's five hours,
  and the four design questions came back in 10 to 15 minutes.

## 4. What worked

1. **The aim was met at the first closing run, and the plan had a path
   for missing it.**
   - The plan settled before the run: at most two closing runs, a
     diagnosis step (`smith-11.s1`) on a miss, and what each criterion
     would need (parity's proposal 3).
   - The baseline missed three criteria at 23:12. The question reached
     the designer at 23:13:54, 86 seconds later. The answer (`65a54642`)
     confirmed that steps 07 to 10 aimed at all three, and changed no
     step.
   - 11.1 passed, and 11.2 was dropped without a run: 108 attempts and
     about 6 h not spent.
2. **The pipeline ran live before the steps that needed it, and no step
   was added** (parity's proposal 2).
   - 01.3 ran `suite parity --attempts 1` at 13:35, 57 minutes after the
     claim. Its first call panicked (`probe.rs:630`, a TOML key), and two
     amends fixed it before the baseline.
   - 06.1 ran the `parity-change` pipeline at one attempt a task, against
     the baseline, before 07.1 started.
   - No step was added, against parity's 11, five of which came at a
     pipeline's first live run.
3. **Contexts stayed small, and the implementers' tokens a step halved**
   (parity's proposal 6).
   - The four threads that took a second increment did so at 86K to
     130K.
   - Seven threads idle for more than an hour were replaced by fresh
     ones. None was resumed.
   - Handovers came at 151K to 199K, and two mid-increment at 214K and
     265K, against 200K to 329K in parity.
4. **Rulings were fast, and none was reversed.**
   - Without the designer, a median of 1.3 minutes from the watch's print
     to the ruling, and at most 4.1.
   - All 28 rulings went through `plan ruling`.
   - The four design questions were each answered by a fresh designer in
     9 to 14 minutes, for $3.72 to $4.80. Two answers changed the design
     (`2c213b1b`, `7f3d4f4b`).
   - Interim ruling 114 kept 06.1 going while the designer answered.
5. **The supervisor's waker woke the coordinator within 3 seconds of
   every suite's end.**
   - README rule 19 (parity's proposal 5, by hand) armed a `pueue wait`,
     `thread/resume`, `plan ring` for each live suite. Seven fired, 1 to 3
     seconds after their task.
   - The coordinator started no waker of its own, and ran no background
     wait.
6. **The machine stayed quiet.**
   - Disk stayed at 101 to 147 GB free, and memory at 7.9 to 10.2 GB.
   - There was no kache, pueued or disk event, against parity's three
     disk stops and a dead queue.
   - 24 gates for 20 steps, none failed. The four that ran again after
     `main` moved all came in the first two hours, when 14 steps landed.
7. **The coordinator kept to its card.**
   - It read 26 reports, each once, against parity's 120 with 37 twice.
   - It read no code and no graph, and edited no ledger by hand.
   - It acknowledged each steer within 13 to 45 seconds.
   - It owned its fault at once: "I removed session 06's target while its
     suite was still running, which caused the baseline to fail."
8. **The director's word took effect at once.**
   - At 17:51:07 it relayed Sonnet sub-agents and a 150K handover. The
     supervisor handed over 45 seconds later, at 282K. All twelve later
     sub-agents ran on Sonnet, and all seven later handovers came at
     128K to 159K.
   - At 16:25 it accepted every decision reserved for the user
     (`decisions.md` 11.1 to 11.5 and 12.5.1 to 12.5.3). No question
     waited on the user during the run.
9. **The live spend came in under the estimate, and was counted.**
   - 390 attempts and $117.44, against about 495 and $154. 11.2's 108
     attempts were not needed.
   - 17 of 20 landings carry a `Live:` trailer. The three without one ran
     a smoke each, and their reports say so.
10. **Ready checks corrected the session files before the steps
    started.** Twelve rulings (101, 104, 106, 110, 111, 112, 116, 117,
    119, 121, 123, 127) fixed Shapes, Files, lines and commands against
    `main`. 06.1 was not made ready before its missing need, 02.4, had
    landed (ruling 110).
11. **The closure was complete.**
    - `done.md` maps each item to its steps, and marks each decision in
      force with its design commit or "not in the design yet".
    - The "Done when" commands ran once on `main`, with their outputs.
    - The downstream worktrees' `Cargo.lock`s were kept before removal.

## 5. What did not

1. **The four change runs took a third of the run, and decided nothing
   the verdict read.**
   - **What:**
     - Rule 11 ran 07.1, 08.1, 09.1 and 10.1 one at a time, each with a
       full `parity-change` against its parent: 9.3 h of the gate lane,
       164 attempts and $69.85. Each step's landing waited on its run.
     - Their RT intervals were 0.34 to 0.78 wide. `causes.md` put what
       each removed at 4% (cause 4) to 22% (causes 1 and 3) of smith's
       time. Three of the four intervals included no change: 08.1's, for
       the largest effect, ran from 0.72 to 1.50. Only 07.1's excluded
       it, at 0.815 [0.650, 0.989].
     - Two runs stopped on RC's projection (1.853 and 1.821, against the
       baseline's 1.734). The line stood 15 and 13 minutes, and two
       designer sessions cost $8.26. Both answers called the move noise
       and kept the step.
     - The projection multiplies each step's ratio onto the baseline's,
       and each parent arm measures again the code the step before it
       measured (`later.md`'s row, from the designer's answers). At 10.1
       it put RT at 1.513 and RC at 1.821. The closing run measured
       1.033 and 1.174.
     - What did move was the counts each step reported under rule 13:
       smith's responses on `shell-cwd` fell from 70.1 to 54.2 an
       attempt.
   - **Cause:**
     - Parity's proposal 4, deferred to this design, became `benchmarks.md`
       16.6: a change to what the model is sent runs `parity-change`
       against its parent, and stops when a projection passes the
       baseline's interval.
     - The design checked RT's power for the verdict (`rt-power.md`), but
       not for a change run, whose two arms are both smith and whose
       effects are a fraction of the verdict's margin.
   - **First visible:** 06:22, 08.1's RT interval [0.719, 1.496]; 09:17,
     09.1's stop. **Acted on:** the designer kept both stops (09:36,
     12:28), and left the projection's revision to `benchmarks.md` 16.6,
     before the next plan whose aim is a parity verdict (`later.md`).
2. **06.1's baseline held the gate lane for 8.1 hours, and 3.1 of them
   measured nothing.**
   - **What:**
     - **15:16:22:** w12 yielded on the running baseline.
     - **15:16:57:** the coordinator removed `wt/smith-06/target`: "Removed
       yielded session 06 target."
     - **15:17:30:** the suite, running `target/debug/smith-bench` from
       that directory, died at its third attempt with `binary start:
       Child(NotFound)`. Nothing rang. The supervisor saw it at 15:27 and
       steered at 15:28.
     - **15:29 to 18:32:** the rerun stopped at 58 of 68 attempts.
       Attempt 53's response timed out waiting 60 seconds for its head,
       with no usage (`maybe_sent`). README rule 9 named only capacity
       and server errors. Ruling 113 amended it, and the suite ran once
       more from the start.
     - **18:41 to 23:12:** the baseline that counted.
     - 60 attempts and $8.86 were spent on the two lost runs.
   - **Cause:**
     - **development.md 11.4,** from parity's proposal 7: "the coordinator
       removes a session's [`target/`] when it yields, hands over or
       ends." A session that yields on a live suite leaves the suite
       running from that directory. The rule is unchanged on `main`
       (development.md, line 2140). Disk never fell below 101 GB in this
       run.
     - **A task's end rang nobody.** The waker of rule 19 was armed for
       each suite from 16:33. Before that, only the supervisor's own poll
       of `heavy.tsv` watched it.
     - **A suite with one unanswered request** has an incomplete summary,
       and reruns whole (`benchmarks.md` 8.2, 16.5). That is a design
       matter (section 9).
   - **First visible:** 15:27. **Acted on:** the 15:28 steer (Pass on 4:
     never remove a `target/` while a heavy task from it runs), and ruling
     113 at 18:39.
3. **The hourly check ran late after every handover.**
   - **What:**
     - Six gaps between checks came at 71 to 91 minutes: 14:59 to 16:26,
       18:48 to 20:14, 03:17 to 04:36, 06:44 to 07:55, 09:47 to 11:18, and
       13:18 to 14:29.
     - Five followed a handover. Each successor created its cron at
       takeover, so its first check came about an hour after the
       takeover, not after the last check. One followed the session
       limit (15:59 to 16:25).
   - **Cause:** supervisor.md 1's interim rule (parity's proposal 11) runs
     a check at takeover only when the last is over 45 minutes old. Each
     handover came soon after a check, and each new cron counted its
     hour from the takeover. The kit's alarm at 65 minutes is still on
     the kit branch.
   - **First visible:** 20:14, the first check after an 86-minute gap.
     **Acted on:** not.
4. **08.1 waited 59 minutes on a re-check that could not build, and the
   rule against such a hold reached the run ten hours after it landed.**
   - **What:**
     - 07.1 landed at 02:29. 08.1, its dependant, stayed in `check` for
       the supervisor's re-check, which had started at 00:18.
     - The re-check's sub-agent had queued `cargo metadata` in the build
       lane at 00:20. The gate lane ran 07.1's suite alone until 02:25,
       and the build lane waited 7,512 seconds. The sub-agent stalled,
       and was stopped at 03:30 with no report.
     - The 03:17 check found the coordinator idle for 47 minutes, and the
       03:29 steer passed on `ce02728`'s proposal 4: "a re-check never
       holds a start". 08.1 started at 03:30.
     - `ce02728` had landed at 17:18 the day before, and named this run as
       taking its proposals 4, 5, 6 and 9. The director relayed 5 at
       17:51.
     - 11.1's ready check met the same lane, and skipped fmt and clippy
       "because the build lane was exclusive".
   - **Cause:**
     - **A ready check builds in a lane that a live suite pauses.**
     - **The quiet-hour check skips development.md 14.1's step 2,** the
       listing of process changes. Every check from 18:48 to 02:17 was a
       quiet one, so none listed `ce02728`.
   - **First visible:** 03:17. **Acted on:** the 03:29 steer.
5. **The coordinator carried 150K or more for 22 hours.**
   - **What:**
     - Its context passed 150K at 14:45 on 10-16, 200K at 18:35 and 300K
       at 06:23. It stayed above 150K until its one compaction, at 371K,
       on 10-17 at 12:30.
     - 76% of its input came above 150K, and 65% above 200K.
     - Eight requests read the whole context uncached, six of them the
       first after an idle of one to five hours: 74% of its uncached
       input.
     - Each wake that found only a wait's end cost two requests at about
       180K: 51 requests and 9.3M in all.
   - **Cause:** at `7e8fb70`, nothing hands over or compacts the
     coordinator. `c0268a8`, which compacts it at about 200K, landed at
     21:54 on 10-16. The run kept the kit of its claim.
   - **First visible:** at each check, in the pulse's contexts line.
     **Acted on:** not in the run. `main` has it.
6. **The implementers still waited a quarter of their input, and the
   card's commands failed on Codex.**
   - **What:**
     - **Waiting:** 326 requests saw only a running command: 37.1M, 25.9%
       of their input, against 37% in parity. By what they waited on:
       the build lane 16.5%, `land` 3.8%, a live suite 2.8%, a smoke 2.5%.
     - **Cut waits:** 94 cells with no wait set came back after 31
       seconds, and 141 launches returned at their own yield. The card
       asks for `yield_time_ms: 300000`, at the start too (implementer.md
       8.3). The threads asked 300 seconds in 172 waits, and 60 seconds
       in 119.
     - **Refused removals:** `rm -rf target`, which implementer.md 8.1
       prescribes after a landing, was refused by Codex's command review
       ("rm -f style commands are not permitted") 15 times. Each refusal
       dropped its whole cell, `plan log` included, and the report was
       sent again.
     - **`python`:** 16 threads ran `python -` first, and got "command not
       found".
   - **Cause:** the card's commands were written for a shell, not checked
     against Codex's review. The wait rule is the card's, and half the
     threads did not keep it.
   - **First visible:** 12:41, the first `python` failure; 12:56, the
     first refused `rm`. **Acted on:** each thread worked around them by
     itself (`cargo clean`, `python3`).
7. **The same test-shape rules were steered again and again, and copies
   still landed.**
   - **What:**
     - Ten steers carried 18 points, 11 of them on the shape of tests:
       named sizes, records not text, the setup a test needs, both sides
       of a bound, rows that carry data, fixtures written once.
     - "Write a shared setup once" was steered at 14:07, 15:08, 03:29 and
       06:46. It became README rule 18 in two rulings (117, 124), and
       ruling 124 merged four Pass-on texts into one. "Both sides of a
       bound" was steered at 03:29 and again at 06:46.
     - The coordinator's spawn prompts grew from about 850 characters to
       5K to 11K, carrying up to eleven Pass-on texts.
     - Copies landed anyway (section 9). The 14th `llm::Prompt` literal
       landed in `tests/protocol-llm/tests/guard.rs`. Pass on 11's
       `prompt_with` and `window_after` were never written. A table test
       in 08.1 branches on its rows' labels.
   - **Cause:** these rules are in no card or strategy document.
     testing-strategy.md says nothing of a bound's two sides, of named
     setups, or of editing a recorded stream as records. Each plan and
     each steer writes them again, and each fresh thread meets them as
     prose in its prompt.
   - **First visible:** 13:59, the second check. **Acted on:** by ten
     steers and two rulings.
8. **Claude Code cost the same per hour as in parity, and its sub-agents
   read the most.**
   - **What:**
     - The supervisor cost $95.44: its nine sessions $36.72, and 20
       sub-agents $58.72 (8 on Opus $21.27, 12 on Sonnet $37.45 at Opus's
       rates). That is $3.3 an hour, against parity's $3.4.
     - **By purpose:** 8 ready checks $25.54, 3 re-checks $7.02, 9 hourly
       checks $26.16.
     - **Two sub-agents ended with no report,** stopped at a handover
       after 17 and 20 requests; the first is item 4's re-check.
     - **The Sonnet sub-agents ran larger contexts than the Opus ones:**
       peaks of 87K to 246K (median 169K) against 125K to 160K (median
       137K), and $3.12 each at Opus's rates against $2.66.
     - **The checks paid off:** all nine sub-agent checks led to a steer or
       a ruling. The 19 quiet checks cost $3.00 in all (parity's proposal
       13).
   - **Cause:** the metric prices Sonnet as Opus, so it cannot show the
     saving the director's rule aimed at. A sub-agent's brief bounds its
     report, not its reading.
   - **First visible:** in this retrospective. **Acted on:** not.
9. **`heavy.tsv` lost the run's seven long suites.**
   - **What:** seven gate-lane suites have no row: 3787 at 15:29, 3788,
     and 07.1's to 11.1's. They took 21.8 h of running. pueue still holds
     them as done.
     - The pulse's queue figures left them out all run.
     - The metric prints `queue_wait_ratio` 0.97, where they make it 0.17.
   - **Cause:** `heavy` writes its row from the client, after `pueue wait`
     (`heavy`, line 121). An implementer that yields ends the shell that
     waits, and no row is written, nor is the task removed.
   - **First visible:** in this retrospective. **Acted on:** not.
10. **A premise the plan checked by a command was wrong again, and three
    Shapes were.**
    - **What:**
      - **Ruling 107 (plan defect):** "`memory` stays 2 GiB while the
        larger of the two worst cases the profile's memory tests print
        … fits it". `planning-commands.md` measured that quantity, as
        parity's proposal 10 asks. But those totals miss the `exec`
        faces, and the largest startup worst case was 2,193,357,213
        bytes, over 2 GiB. Ruling 108 met it again in 04.1's smoke.
      - **Rulings 109 and 118 (plan defects):** `AttemptRow` lacked the
        fields 02.2's Shape read, and 08.1's transcript Shape missed the
        channel's codec.
    - **Cause:** the command checked the premise's own words, not the
      bound the smoke enforces (`service.worst_case` against
      `declared.memory`).
    - **First visible:** 13:28. **Acted on:** rulings 107 and 108 at
      13:30 and 13:34.
11. **Small things in the record:**
    - **The status files:**
      - smith's `STATUS.md` still names "the wall-clock plan" in ten rows
        of section 2, as what builds what it has now built.
      - The README says "the designer's next landing" puts the jig and
        temper repin row in their `STATUS.md`. Four designer landings
        followed, and neither file has it.
    - **The supervisor:**
      - Its session hit its limit at 15:59 on 10-16 and resumed at 16:25.
        No watch event fell in those 26 minutes.
      - The coordinator's thread was loaded by hand three times, each
        after a ring found it unloaded (16:33, 03:29, 07:56).
      - The watch printed `quiet:` three times as a wake raced the
        heartbeat. Each cost a restart of the watch.
      - `plan downstream --help` ran the check instead of printing help,
        and the pulse's `grep` printed "binary file matches" on the v5
        goldens.
    - **The coordinator:**
      - It missed 02.4's split for 12 minutes (14:46 to 14:58).
      - Five of its rewrites of `context.md` failed their own 60-line
        assert.
    - **The designer:**
      - Each fresh designer read 116K to 138K before its first edit. Four
        hunted their own session id with `env | grep` for the trailer.
      - The design's session passed 300K and reached 371K, and the
        planner's reached 464K, neither handing over (development.md 2.1
        and 2.2).
    - **The plan:**
      - Ruling 127 cites ruling 119 for the FT row, which came from 115.
      - Four reports run words and digits together ("Its2159s").
    - **This retrospective's own cost** was $76 when `metrics.tsv` was
      written, with its eleven readers, who ran on Sonnet and are priced
      as Opus: about $7 a source.

## 6. Decisions taken during the run

Rulings 101 to 128, ten steers, and the director's two messages. The
supervisor marked each decision in force in `done.md` with its design
commit, or "not in the design yet".

| Rulings | What | Reached the design | Proposal |
|---|---|---|---|
| 101, 104, 106, 110, 111, 112, 116, 117, 119, 121, 123, 127 | Ready checks and re-checks: Shapes, Files, lines and commands against `main` | The session files only; not design matters | — |
| 102, 105, 107, 108 | The session's line as a named field; `memory` raised to the next 64 MiB multiple above the largest startup worst case | Not design matters; 107's value is in the code and the plan | — |
| 109, 118 | Plan defects: `AttemptRow`'s fields; the allowed names in the session's record | The plan only | — |
| 113 | A request never answered (`maybe_sent`) reruns the suite from the start; README rule 9 amended | The plan only; the design question is in section 9 | — |
| 114, 115 | The baseline's misses: 07 to 10 aim at FT, RT and RC; no step added | `65a54642`, plan files only, "No design text changes" | — |
| 120 | A poll waits as asked only beside `job` calls alone; the task role counted as written in the host part | Yes: `2c213b1b` (`session.md` 5, 10; `tools.md` 5; `llm.md` 3; `limits.md` 4; `shell.md` 6.6) | — |
| 125 | 09.1 kept: RC's move is noise | Yes: `7f3d4f4b` (`benchmarks.md` 16.6) | 1 |
| 128 | 10.1 kept: its RC projection carries 09.1's ratio | Not in the design yet: `db860283` changed the plan and `later.md` (the revision of 16.6) | 1 |
| 103, 122, 126 | Test debts deferred to `later.md` rows | Not design matters | 5 |
| 117, 124 | README rule 18 grows: shared setups, then any fixture the code holds | Not design matters | 5 |
| Steer 15:28 | Never remove a `target/` while a heavy task from it runs | Not in the design yet (process) | 2 |
| Steer 03:29, point 1 | A re-check never holds a start (`ce02728`'s 4) | In development.md on `main` | 4 |
| Steers 13:12, 14:07, 15:08, 16:33, 03:29, 06:46, 07:56 | Test shape: named sizes, records, setup, bounds, rows, fixtures; the summary copied into `benchmarks/summaries/` | Not in the design yet (process) | 5 |
| Steers 03:54, 12:27 | Hold 08.1's live run for the answer; a dropped need counts as met | The plan only | — |

**Decided by the director during the run:**
- At 16:25 on 10-16, every decision reserved for the user (`decisions.md`
  11.1 to 11.5, 12.5.1 to 12.5.3), and that questions for the user go
  under "For the user";
- At 17:51, sub-agents on Sonnet and the supervisor's handover at 150K.

**Still with the designer** (`later.md`): the projection's revision in
`benchmarks.md` 16.6; FT's output in writing responses; where
`CARGO_BUILD_JOBS=2` comes from; the `edit-format` experiment; the
`history` recalibrations. Nothing waits on the user.

## 7. The proposals adopted after the last retrospective

The last retrospective is parity's, decided on 10-16 and applied in
skein `30bc10f` at 07:25, five hours before the claim. Its text parts
were in force at `7e8fb70`. Its kit parts (proposals 5, 6's
`PLAN_CONTEXT_LIMIT`, 7's pulse line, 8, 11, 14's workspace file and
15) wait on skein's `kit-after-parity`. That branch holds one older
commit, `d7e6ee3`, and is not merged. Proposal 4 was adopted in this
plan's design (`benchmarks.md` 16.6).

| Proposal | Decision | Meant to move | parity | wall-clock | Moved |
|---|---|---|---|---|---|
| 1. No question waits inside a tool | adopted | `blocked_h` | 7.3 | 1.0; no AskUserQuestion; no question for the user arose | Yes |
| 2. A pipeline proven live at one attempt first | adopted | `steps_added`; gate hours that measure nothing | 11; 1.6 h | 0; 3.1 h, for other causes (section 5, item 2) | Steps yes |
| 3. A verdict plan settles its closing runs; a baseline's miss goes to the designer at once | adopted | criteria failed at the closing run | 1 | 0; the miss reached the designer in 86 s | Yes |
| 4. A change reports its effect on the criteria, and stops past the baseline's interval | adopted in the design | closing runs that fail on a step's own effect | 1 | 0; but two stops on noise, and 9.3 h of change runs (section 5, item 1) | At a cost: proposal 1 |
| 5. `plan idle` with task ids; the watch rings; ring and resume load the thread | adopted; kit not landed; by hand meanwhile | late wakes; hand loads | about 1 h; 10 | 7 of 7 wakes in 1 to 3 s, by the supervisor's waker; one death 11 min late; 3 hand loads | Yes, by hand |
| 6. A fresh thread past 150K between increments | adopted | `contexts`; `tokens_implementers` | 201K, 293K; 13.4M | 101K, 199K; 7.17M | Yes |
| 7. A session's `target/` goes; at most six sessions below 60 GB; `kache gc` below 80 GB | adopted in part | infrastructure escalations | 4 | 1, the provider's; no disk event. Its "when it yields" killed 06.1's baseline | Yes, and a new fault: proposal 2 |
| 8. pueued restarts itself; the watch alerts | adopted; kit not landed | `blocked_h` | 5.1 h on a dead queue | no queue event | Not tested |
| 9. A design term's block goes to the designer | adopted | wait to a design gap's answer | 87 min | 10 to 15 min; none read as a plan defect | Yes |
| 10. A bound's premise checked by a command; a test holds the margin | adopted | plan defects | 17 | 3. The command measured the wrong total; no test holds the margin (section 5, item 10) | In part |
| 11. The watch alarms at 65 minutes; meanwhile a check at takeover past 45 | adopted; kit not landed | longest gap between checks | 6.0 h | 91 min; six gaps past 65 (section 5, item 3) | In part: proposal 3 |
| 12. A check ends with the steer it proposes | adopted | sightings before a steer | 2 to 6 | 8 of 9 checks steered; the shared-setup rule steered four times | In part: proposal 5 |
| 13. A quiet hour is `plan pulse`, with no sub-agent | adopted | the supervisor's cost | $114 | $95; 19 quiet checks, $3.00 | Yes, but it skipped 14.1 step 2: proposal 4 |
| 14. Design and plan in sessions of their own | adopted in part | planning's cost | about $141 | $67.91; the eight questions by a designer sub-agent, as the exception allows | Yes. Both sessions passed 300K |
| 15. `plan metrics` fixes | adopted; kit not landed | comparability | 10 defects | the same, and three new (proposals 7, 8) | No |
| 16. A run keeps its process; a change names the runs that take it | adopted | steers that re-derive a rule | 3 | `ce02728` named this run; its 5 took 33 min, its 4 ten hours (section 5, item 4) | In part: proposal 4 |

**Changes to the process that landed during the run** (skein, 10-16
12:39 to 10-17 17:43):

| Commit | Time | What | Did the run take it? |
|---|---|---|---|
| `e6fd9fb` | 10-16 17:18 | A run keeps the kit of its claim in `kit/` (forgejo-17's 3) | No; its body says the run keeps `main`'s kit |
| `ce02728` | 17:18 | forgejo-17's proposals 1 to 9, and the director (2.7) | In part: Sonnet and the 150K handover at 17:51, by the director; "a re-check never holds a start" at 03:29, by a steer; 6, 9 and 2.7 are the supervisor's own |
| `c0268a8` | 21:54 | The coordinator compacts at about 200K | No: the profile is the checkout's, and nothing relayed it (section 5, item 5) |
| `7779af1` | 10-17 12:00 | `plan resume` loads an unloaded coordinator thread | No |
| `71f6c38`, `1b83a43`, `2f317f5` | 12:26 to 12:36 | The supervisor and the designer compact in place, as an experiment; the `supervisor` metric | No: "smith/wall-clock's … keep their process" |
| `d21d90d` | 10-16 13:25 | hearth's io design | Not process |

## 8. Proposals

Each gives a rule in one sentence, its evidence, the document it
changes, and the metric it should move. They are ordered by expected
effect. Proposals 7 and 8 change the kit; the rest change text.

1. **A change whose expected effect is smaller than its own
   `parity-change` run can resolve lands on the counts it targets, and
   the closing run measures such changes together; a plan states each
   run's resolution from its planning commands, and measures alone only
   a change that its run can tell from none.**
   - **Evidence:**
     - The four change runs took 9.3 h of the gate lane, a third of the
       run, plus 164 attempts and $69.85.
     - Their RT intervals were 0.34 to 0.78 wide, against effects of 4%
       to 22%. Their RT ratios multiply to 1.083, while the closing run
       moved RT from 1.397 to 1.033.
     - Two runs stopped on noise, and two designer answers kept both.
     - The counts the steps reported did move: responses on `shell-cwd`,
       70.1 to 54.2 (section 5, item 1).
     - It amends parity's proposal 4 as `benchmarks.md` 16.6 adopted it.
       The protection 4 sought, a closing run that fails on a step's own
       effect, stays with the plan's second closing run and its
       diagnosis step, as 11.2 and `smith-11.s1` gave it.
   - **Changes:** development.md 5.1 and 5.5; smith's `benchmarks.md` 16.6,
     the designer's, with `later.md`'s row on the projection.
   - **Moves:** `wall_h` of a verdict plan down by the change runs' hours
     (9.3 here); stops kept as noise, from 2 to 0.
2. **The coordinator removes a session's `target/` only when no heavy
   task the session queued is queued or running; a session that yields
   on a live run keeps it.**
   - **Evidence:**
     - At 15:16:57 the coordinator removed session 06's `target/` under
       11.4's "when it yields". The baseline running from it died 33
       seconds later, and the line stood until the 15:28 steer.
     - Disk stayed above 101 GB all run.
     - The rule is unchanged on `main` (section 5, item 2).
   - **Changes:** development.md 11.4; implementer.md 8.1, which already
     removes it only after a landing.
   - **Moves:** live runs that die on a removed `target/`, from 1 to 0.
3. **A successor supervisor's first hourly check is due one hour after
   its predecessor's last check, and runs at once if that time has
   passed.**
   - **Evidence:**
     - Five gaps of 71 to 91 minutes followed handovers. Each successor's
       cron counted its hour from the takeover, and first fired 61 to 79
       minutes later.
     - The interim rule, a check at takeover when the last is over 45
       minutes old, never applied (section 5, item 3).
   - **Changes:** supervisor.md 1 and 7; development.md 14.1. It stands
     until the kit's alarm (parity's proposal 11) lands.
   - **Moves:** checks more than 65 minutes apart, from 6 to 0.
4. **A quiet-hour check still runs development.md 14.1's step 2, and a
   process change whose commit names a running run reaches that run's
   coordinator, as a steer, within the hour.**
   - **Evidence:**
     - `ce02728` landed at 17:18 and named this run as taking its
       proposals 4, 5, 6 and 9.
     - Proposal 4 reached the coordinator at 03:29, ten hours later. By
       then a re-check had held 08.1's start for 59 minutes, which is
       what 4 forbids.
     - Every check from 18:48 to 02:17 was a quiet one, and none listed
       the change (section 5, item 4).
   - **Changes:** development.md 14.1; supervisor.md 6. It sharpens
     parity's proposal 16.
   - **Moves:** hours from a named change's landing to the run taking it,
     from 10 to under 1; starts held by a check (59 minutes) to none.
5. **The test-shape rules this run steered go into skein's
   testing-strategy.md, and a steer or a plan's rule cites the section:**
   - a bound's two sides;
   - a repeated setup or fixture written once as a named function;
   - a table's rows carrying data, never compared by label;
   - a recorded stream edited as records, with a count;
   - a test given only the setup its assertion reads;
   - named sizes, each with its derivation.
   - **Evidence:**
     - 11 of the 18 steer points were these rules.
     - "Write a shared setup once" was steered four times and became README
       rule 18 in two rulings (117, 124).
     - Spawn prompts grew from about 850 characters to 11K with the
       Pass-on texts.
     - Copies landed anyway, among them a 14th `llm::Prompt` literal and a
       table that branches on labels (section 5, item 7).
     - testing-strategy.md holds none of them.
   - **Changes:** skein's testing-strategy.md; implementer.md 8.2; the
     plan's shape (a README cites, rather than restates).
   - **Moves:** test-shape points in steers (11) down; the spawn prompt's
     length down; test debts at the end down.
6. **The card's commands run as written on Codex: implementer.md 8.1
   removes a `target/` with `cargo clean`, and the card names `python3`.**
   - **Evidence:**
     - Codex's command review refused `rm -rf target` 15 times. Each
       refusal dropped its cell, `plan log` included, and the report was
       sent again.
     - `python` was not found in 16 threads (section 5, item 6).
   - **Changes:** implementer.md 8.1 and 8.3.
   - **Moves:** failed cells from the card's own commands, from 31 to 0.
7. **`heavy` writes its row from inside the task it queued, so that a
   client that ends while the task runs loses nothing; `plan pulse`
   reads the tasks pueue still holds.**
   - **Evidence:**
     - Seven suites, 21.8 h of the gate lane, have no row. The pulse's
       queue figures left them out.
     - `queue_wait_ratio` printed 0.97 where it is 0.17, and pueue still
       holds the seven tasks (section 5, item 9).
   - **Changes:** `heavy`; `plan pulse`; development.md 11.2.
   - **Moves:** comparability of `queue_wait_ratio` and
     `gate_lane_other`.
8. **`plan metrics` prices each message at its own model's rates, and
   counts as design the sessions that the plan's design commits name in
   `Designer-Session:`.**
   - **Evidence:**
     - $37.45 of the supervisor's $95 are Sonnet's tokens at Opus's
       rates. The director's rule cannot show its effect.
     - The design's session and its sub-agents, $23.76, are not counted
       (section 3).
   - **Changes:** `numbers.sh`; development.md 15.2. It joins parity's
     proposal 15 on the kit branch.
   - **Moves:** comparability of `cost_claude`.

## 9. Debts left in the code

For smith's `tech-debt.md`, for skein's kit, and for the designer.

**smith:**
- **44 panics added outside tests:**
  - 42 `.expect(`: `smith-domain-session` 16 (`session.rs` 10,
    `batching.rs` 6), `smith-protocol-llm/src/guard.rs` 8, `benchmarks/src`
    8, `smith-domain-tools/src/jobs.rs` 3, and 2 each in
    `smith-domain-run`, `smith-domain` and `smith-protocol-channel`.
  - 2 `unreachable!` in `smith-protocol-machine/src/process/jobs.rs:43`
    and `:57`, each a state the types allow.
  - `smith-local-shell/src/settings/resolve.rs:88` expects "fits four
    kibibytes", but `TASK_ROLE_BOUND` is gone and nothing enforces it.
    `role.rs:28` asserts a bare `<= 4096`.
- **Files past 800 lines grew:**
  - `benchmarks/src/probe.rs` 823 → 967, `summary.rs` 822 → 871,
    `batch.rs` 1,004 → 1,022 and `run.rs` 1,223 → 1,231;
  - `crates/smith-domain-session/src/session.rs` 3,630 → 3,665;
  - `benchmarks/tests/codex_adapter.rs` crossed 800 (809).
- **Two tuples that dodge clippy's argument limit:** `probe.rs:638`
  (`selection`) and `probe.rs:400` (`arms`).
- **Untested branches in `guard.rs`:** `from > messages.len()`
  (`:23-25`), a measurement's error (`:63-66`), and the diagnostic's clip
  (`:104-106`).
- **Tests:**
  - `crates/smith-domain-session/src/tests/batched_change.rs:110-162`
    holds six unlike rows in one test. It branches on their labels
    (`:139`, `:156`, `:157`) and leaves tickets 9, 10 and 11 bare.
  - `a_compaction_carries_no_guard` (`src/tests.rs:3033`) first asserts
    that an ordinary prompt carries one.
  - Copies:
    - 14 `llm::Prompt` literals in 10 files;
    - the `Window::fresh` setup 4 times in `tests/window.rs`;
    - `parity.rs`'s wall loop 6 times, with `CHANGE_WALL_MS` defined 6
      times;
    - the observer's `Limits` setup 5 times;
    - the `JobAct::Hasten` arm in three world job tables;
    - `tests/session/src/world/llm.rs:26-31`, which types the guard's
      diagnostic again.
  - `str::replace` on recorded streams, with no count:
    `benchmarks/src/tests/comparison.rs:235` and
    `benchmarks/tests/summary_resources.rs:62-63`.
- **Memory's margin:**
  - 21,077,507 bytes under the new 2,214,592,512 at 03.2, less 04.1's
    205,248. No test holds it.
  - `settings/tests.rs:498` is still named `…_fit_two_gibibytes`.
- **Unchanged from parity's list:**
  - the six placeholder arms in `flight.rs`;
  - `tests/machine/tests/jobs.rs:132`, two outcomes in one test;
  - the flaky `a_second_run_resumes_the_chat_from_its_files` (140 passes
    here, no failure);
  - `tool-output/timeout`'s classification;
  - `calibrate`'s rewrite of the whole file.
- **From `later.md`:**
  - `history` and `history/small-window` recalibrated;
  - 08.1's one-sided bounds and its digits written by hand;
  - `memory_returns.rs`'s sizes;
  - 09.1's typed `edit()` and `read()`.

**For the designer:**
- **`benchmarks.md` 16.6's projection** counts each landed code twice
  (`later.md`), and is proposal 1's subject.
- **A suite stopped on one unanswered request** reruns whole:
  - 06.1's cost 3.04 h and $8.81 at attempt 53 of 68.
  - Whether a run may complete only the attempts it lacks is
    `benchmarks.md` 8.2's and 16.5's question.
- **The arms' spread:**
  - Codex's `shell-cwd` median went from 443.6 s at the baseline to
    590.4 s at the close, and each arm's seconds a response rose 22% to
    30%.
  - A verdict across runs reads within-run ratios, and those carry
    Codex's spread.
- **smith's `STATUS.md`, section 2,** still names "the wall-clock plan"
  as what builds ten rows that it has built.
- **jig's and temper's `STATUS.md`** lack `later.md`'s repin row:
  transcript 5, events 3, `Purpose::Ordinary { guard }`, `memory`.

**skein, the kit:**
- `heavy`'s row is lost when its client ends (proposal 7).
- development.md 11.4's "when it yields" (proposal 2).
- `numbers.sh`:
  - parity's proposal 15, unmerged;
  - Sonnet priced as Opus, and the design's sessions uncounted
    (proposal 8);
  - `steps_planned` counting a split as planned.
- `plan ring` and `plan resume`: three hand loads. `7779af1` mends
  `resume` on `main`; `ring` still exits on an unloaded thread.
- `PLAN_CONTEXT_LIMIT` is still 200,000 on `main`.
- `plan watch` prints `quiet:` when a wake races the heartbeat.
- `plan downstream --help` runs the check.
- The pulse's `grep` reads binary goldens.
- `kit-after-parity` holds one commit, 134 behind `main`.

**Workspace:**
- **jig and temper fail against smith's `main`,** with 4 and 5 errors in
  their own code, until their repins take this plan's formats.
- **smith's skein pins** were repinned by the render plan (`b415f737`)
  three minutes after the run.

## 10. Decisions

For the user, or the director.

2026-10-17. The director, acting for the user (skein's development.md,
2.7), adopted all eight proposals. The designer applied their text in
skein b30cc95 (development.md, its two cards and testing-strategy.md),
and proposal 1's design change in smith's `benchmarks.md`, `shell.md`
and `STATUS.md`, with this section. The kit's parts (`heavy`, `plan
pulse` and `numbers.sh`, with the sections of development.md that
describe them) go to the kit batch, a light plan run on hetzner next.
The runs in flight, hearth's render and temper's forge-setup and
client-api, take proposals 2, 3, 4 and 6 by their supervisors' steers,
and keep the rest (skein b30cc95's body).

| Proposal | Decision | Where |
|---|---|---|
| 1 | adopted, taking in `later.md`'s row on the projection: a change's projection is the baseline's statistic times its own ratio, and what the landed changes add up to is the closing run's to measure. `summarise`'s product over named landed changes is left in the code, a gap in smith's `STATUS.md` | skein b30cc95: development.md, 5.1 and 5.5; smith: `benchmarks.md`, 1, 4.2, 14 and 16.6; `shell.md`, 6.6; `STATUS.md`, 2 and 3 |
| 2 | adopted, the removal by `cargo clean`; it has no kit part: the coordinator reads `pueue status`, whose tasks carry the worktree they were queued from. implementer.md 8.1 already removes a `target/` only after a landing | skein b30cc95: development.md, 11.4 |
| 3 | adopted, as one-time cron jobs, each check scheduling the next: a recurring job counts its first hour from its creation, and runs late by an offset of its own. It stands until the kit's alarm (parity's 11) lands | skein b30cc95: development.md, 6, 10.5 and 14.1; supervisor.md, 1, 6, 7 and 8 |
| 4 | adopted, sharpening parity's 16 | skein b30cc95: development.md, 14.1, its quiet hour and step 2; supervisor.md, 6 |
| 5 | adopted: testing-strategy.md's new section 10, which a plan's rules, development.md 5.6 and 14.3's checklist cite in place of their own lists | skein b30cc95: testing-strategy.md, 1 and 10; implementer.md, 8.1 and 8.2; development.md, 5.2, 5.6 and 14.3 |
| 6 | adopted | skein b30cc95: implementer.md, 8.1 and 8.3 |
| 7 | adopted; kit part in the kit batch | the kit batch: `heavy` and `plan pulse`, with development.md, 11.2 |
| 8 | adopted; kit part in the kit batch, joining parity's 15 | the kit batch: `numbers.sh` (`plan metrics`), with development.md, 4.4 and 15.2 |
