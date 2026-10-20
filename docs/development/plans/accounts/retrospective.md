# Retrospective: accounts

2026-10-20. This is the run of smith's `accounts` plan on x1, a machine with 8 cores and 15 GB of memory. The plan is the first part of LLM accounts, its lower half.
- skein gained:
  - the MIT licence;
  - a credential with its scheme, and the failure classes an account meets;
  - OpenAI's public Responses route;
  - the fake LLM's failures and its list of credentials;
  - the OAuth machine's terms, ID token, revocation and record version 3;
  - the accounts driver's named table, sign-out and shared record.
- smith gained:
  - the licence, and its grants by scheme;
  - the channel's version 4 and its notices;
  - the charter's version 3 and the transcript's version 6;
  - the events' version 4.
- jig, hearth and temper were repinned.
- The withdrawn forms stay, marked for the second part, `accounts-sources`.

| Event | When (UTC) |
|---|---|
| The design, session `b357b336` on hel8 (smith `e79b9a6b` 06:44, `137a1cb4` 07:39; skein `49725eca` 08:26) | 05:06:51 to 08:26:53 |
| The planner, session `a341cbde` | 06:49:23 to 11:27:46 |
| Plan landed (smith `ff623361`) | 11:19:15 |
| Claim, with `kit/` at skein `b0510fa` | 12:06:35 |
| First landing (skein-01.1, skein `1222e7b`) | 12:30:49 |
| The machine down | 21:21 to 22:23 |
| Last landing (hearth-repin, hearth `3a2130d`, landed from hetzner) | 00:52:29 on 10-20 |
| `done.md` recorded | 00:52:52 |
| "Done when" run by the supervisor's sub-agents | 00:55 to 01:16 |
| Rulings 151 and 152 | 01:19:25, 01:23:16 |
| Release | 01:24:55 |
| `plan retro` | 01:25:32 |

All 56 steps landed. 54 were planned and 2 added (`jig-repin.s1` and `skein-10.s1`), and none was dropped. They were 37 in skein, 14 in smith and 5 repins, in 62 commits: skein 42, smith 16, jig 2, hearth 1, temper 1. This retrospective was written by a fresh session that took no part in the run (development.md, section 15).

**Who read what.** Nine read-only Sonnet sub-agents each read one source:
- the git history of the five repositories, with their trailers, and each step's Size by the plan's command;
- `logs/heavy.tsv` and the logs it names;
- the coordinator's Codex rollout;
- its 55 implementer threads;
- the supervisor's two sessions with their 41 sub-agents, and the designer's session in the run;
- the planner's session with its 48 sub-agents, and the design's session, `b357b336`, copied from hel8;
- the earlier retrospectives, the metrics history, and skein's process from the claim's commit to `main`;
- `log.md`, the blocks and `decisions.md`;
- the debts: the 108 reports, `later.md`, `done.md` and the code on each `origin/main`.

This session read:
- development.md's section 15, and sections 3.6, 7.2, 11.4 and 14.4;
- owners.md's opening;
- the plan's README (its opening, sessions, chain, Pulse and "Done when"), and `decisions.md`'s "For the user";
- `done.md` and `supervision.md`;
- live-text's retrospective, `metrics.tsv` and the history.

**Conventions.** Times are UTC, on 2026-10-19 unless a date is given. Costs are at the list prices of each message's model, as `plan metrics` prices them:

| Model | Input | 1-hour cache write | Cache read | Output |
|---|---|---|---|---|
| Opus 5.5 | $4 | $8 | $0.20 | $20 |
| Sonnet 5.5 | $2 | $4 | $0.10 | $10 |

This document proposes; the user, or the director, decides, and the designer applies what is adopted.

**Which process.**
- **The claim.** `kit/` was pinned at skein `b0510fa` (12:00:41), a forge-kit commit, and `supervision.md`'s Process line names it.
  - Its process is the one before client-api's retrospective was applied.
  - client-api's nine proposals, all adopted at 01:43, had none applied at the claim.
- **During the run.**
  - Twelve kit and process commits landed in skein between 13:14 and 14:44. They apply client-api's, live-text's and tools' retrospectives.
  - `1d105bc` (13:43) says: "Runs in flight (forge-kit, accounts) keep their claims' process and take this only by their supervisor's ruling (3.6)". No ruling took any of them (section 7).
  - The owners experiment landed from 16:29 (`7360708`) to 23:41 (`bc6c16c`). owners.md says that runs started before 10-19 finish under development.md.
- **So this run ran the process of before the last three retrospectives.** Fixes for four of the defects it met were on `main` from 14:44 (section 7).
- **Since then,** new drafts go to owners. development.md stands for the runs already started, and again if the experiment fails. `land`, `heavy` and the disk floor hold for both. Each proposal in section 8 says which it serves.

## 1. In one page

- **56 of 56 steps landed in 12.8 hours.**
  - "Done when" items 1, 3 and 5 passed as written.
  - Item 2 passed once ruling 151 corrected three stale commands.
  - Item 4 met smith's aims on a re-measure, by 0.19 s and 0.83 s. It missed skein's under load ("For the user" E, still open).
  - Every member of skein and smith carries the MIT licence.
- **The run took none of the fixes that landed for it.**
  - From 14:44, `main` had fixes for four defects the run met: the pulse's aims verdict, the short waits, the coordinator's wake-up after each timed-out wait, and a designer's wake.
  - Six hourly checks wrote "takeable once test-plan passes", and none ran `test-plan`.
  - So nine checks diagnosed the same false aims flag by hand, and the coordinator woke up 51 more times for nothing (proposal 1).
  - Under skein `4134983` (14:18), the claim itself would have been refused.
- **The plan's Shapes went stale twice.**
  - 25 of the 52 rulings came from ready checks and re-checks correcting session files.
  - Each of the 12 sessions checked early got a second such ruling, 0.6 to 3.8 hours later.
  - Those 25 sub-agents cost $44.6, 40% of the supervisor's spend. Their build tasks waited 5.2 times as long as they ran (proposal 2).
- **The end waited on things outside the steps.**
  - Disk holds stood 89 minutes on the chain that set the end (proposal 3).
  - The reboot cost about 71 minutes.
  - hearth's own `3df74d3` (23:03) made its default suite take 22.6 to 25.4 s on x1, against a 15 s budget. hearth-repin, a Cargo.lock commit, then took 85 minutes. 51 of them were idle after the director's fix, which nothing could wait for (proposal 8). It landed from hetzner.
- **Test shape was steered at 10 of the 11 hourly checks.**
  - 21 of the 56 steps landed with a breach of testing-strategy.md section 10, 15 of them after the README had a rule that named the fault.
  - 7 of the 10 test-shape steers aimed at code that had already landed (proposal 7).
- **Live-text's "Done when" defect recurred.** smith's timing logs went into the worktree they measured, and smith-bench refused the dirty tree.
  - Live-text's proposal 2 was decided at 13:22, during this run, and never reached this plan's README (proposal 1).
  - Two of the three stale commands could have been found at their sessions' landings (proposal 4).
- **The 15 s budget caused 7 of the 14 failed land gates.**
  - smith's default suite took 87% of its budget at the base, and 11.8 to 14.5 s in the run (proposal 5).
  - skein's aims were met only in quiet gates (proposal 6).
- **The numbers.**
  - 15.2M tokens a step, against tools' 12.1M. The implementers took 7.66M of it, and smith's sessions 12.9M each.
  - The queue waited 1.26 times its running time.
  - `cost_claude` counts $375. The design's $111.49 on hel8 is counted as $0.
  - The planner's $262 is the largest planner bill on record, $4.68 a step (proposal 9).
- **What held:**
  - blocks answered in a median 3.5 minutes;
  - the coordinator saw a landing in a median 3 s, and started the next increment 62 s after it;
  - no steer point refused, and no answer reversed;
  - no file crossed 800 lines; no `#[allow]` or TODO landed;
  - the reboot lost nothing;
  - a re-check found a real defect, which became skein-10.s1.
- **Ten proposals** (section 8):
  - the hourly check takes a met defect's fix, and applies a decided retrospective's fix to the plan;
  - the ready check runs one increment before its session can start;
  - a long disk hold is a block;
  - each session's "Done when" runs at its last landing;
  - a suite near its budget at the base is fixed first;
  - the aims are measured on a quiet machine;
  - `land` refuses a test literal that repeats a named constant;
  - `plan await` waits for an outside commit;
  - the drafters get the review's checks;
  - `queue_wait_ratio` counts a gate's drain, and `plan metrics` reads `origin/main`.

## 2. Outcome

`done.md`'s "Done when, run 2026-10-20T00:55:14Z" holds each command's output. After the last landing, the supervisor's Sonnet sub-agents ran the commands in worktrees of each repository's `origin/main`.

| Item | Result |
|---|---|
| 1. Every step landed or dropped | **Met.** "56 of 56 steps landed; 0 running, 0 blocked, 0 ready, 0 to check, 0 waiting." |
| 2. Each session's own "Done when" | **Met after ruling 151.** Three commands failed as stale while their properties held: skein 01's `access_token` grep found skein 11.1's `testing/skein-fake-peers/tests/oauth.rs`; skein 07's count read `machine.rs`, but its check moved to `machine/fetch.rs:197` under ruling 118; smith 03's `Refusal::(Endpoint\|Dialect)` pattern matched `connection::Refusal::Endpoint`, an unrelated enum, at `crates/smith-protocol-llm/src/component.rs:553`. The rest passed, skein 01's and 02's `scripts/check.sh` included. |
| 3. The licence | **Met** in skein and smith. jig got the same licence at `c2282f63` (01:27 on 10-20), outside the plan. |
| 4. The test-time aims | **skein missed; smith met on a re-measure** (ruling 152). skein's medians of package sums were 20.83 s focused against 16, and 21.06 s fuzzy against 19. The runs took 21.07, 20.83 and 15.53 s, and 21.06, 21.09 and 16.31 s, at a one-minute load of 7.4 to 8.8 on 8 cores, just after an owners gate; the third met both aims. In smith, the command's logs, written into the worktree, made `smith-bench`'s probe test refuse a dirty tree, so each focused run stopped at 18 to 20 of 2,453 tests; the loop still exited 0, and fuzzy took 93.64 s against 79. The re-measure, with logs outside the worktree at load 12.67, gave 59.81 s of 60 and 78.17 s of 79. The first run of each pair was about 50% slower than the other two. |
| 5. No budget raised | **Met.** 15 s and 60 s in both. |
| 6. Nothing else checked | Nothing to run. |

The release came 32 minutes after the last landing. "Done when" and its two rulings took 29 of them.

What the plan leaves for later is in `later.md`'s 15 rows, 5 of them added during the run: rulings 116, 145, 146 to 150 (one row), 150 and 152. "For the user" E (ruling 152) has no answer yet.

## 3. Numbers

`plan metrics` wrote `metrics.tsv`. The comparisons:
- tools, temper's run of the same day and kit generation, the nearest in steps and `step_h`;
- live-text, smith's last run;
- render, the nearest in wall time;
- parity, smith's nearest in steps;
- forge-setup.

"A step" divides by landed steps, 56 here.

| Metric | accounts | Notes | tools | live-text | render | parity | forge-setup |
|---|---|---|---|---|---|---|---|
| `wall_h`, `running_h` | 12.8, 11.0 | The machine was down from 21:21 to 22:23. | 8.3, 8.5 | 2.1, 2.1 | 13.8, 13.4 | 33.9, 13.3 | 43.8, 7.9 |
| `steps_planned`, `steps_added` | 54, 2 | skein 37, smith 14, repins 5; none dropped. | 42, 1 | 8, 0 | 151, 17 | 47, 11 | 40, 4 |
| `step_h` | 0.20 | A mean of 3.7 sessions in a turn before the reboot, at most 6 for half an hour; never 7 or 8 of the eight slots. | 0.20 | 0.27 | 0.08 | 0.23 | 0.18 |
| `chain_step_min`, `chain_step_max` | 33.6, 116.4 | New. The longest is smith-03.1. | | | | | |
| `escalations` | cross-session scope 6, design gap 2, plan defect 4, process 3 | 0.27 a step. Two filed as scope were plan defects by their own rulings (107, 130). | 13; 0.30 | 2; 0.25 | 48; 0.29 | 36; 0.62 | 29; 0.66 |
| `escalation_wait_min` | 3.5 | 14 of the 15 were answered within 7.2 minutes. | 4.2 | 2.5 | 2.7 | 2.5 | 1.7 |
| `blocked_h` | 1.2 | 05.3's design question was 31 of the 72 blocked minutes. | 1.2 | 0.1 | 2.6 | 7.3 | 3.2 |
| `answers_reversed` | 0 | Four chains extended an earlier ruling: 114 to 115, 131 to 133, 135 to 139, and 146 to 148 to 150. | 9 | 0 | 2 | 1 | 2 |
| `steers` | 26 sent, 0 points refused | 36 points, 18 on test shape. | 15, 0 | 2, 0 | 27, 1 | 11, 0 | 13, 1 |
| `restarts` | loaded 3, reboot 2 | Two "resumed" after the reboot, 4 minutes apart; one load by `plan resume`, two by rings. | | | | | loaded 6, reboot 1, the user's 1 |
| `tokens_coordinator` | 108.3M in (99% cached), 0.20M out | 1.93M a step. | 1.90M a step | 1.73M | 2.39M | 4.02M | 5.67M |
| `tokens_implementers` | 428.8M in (97%), 1.35M out | 7.66M a step: smith's sessions 12.9M, skein's 5.4M, the five repins 50.4M in all. | 5.64M | 7.67M | 6.68M | 13.43M | 7.29M |
| `tokens_claude` | 1,057.3M in (96%), 9.05M out; 730.1M before the claim, 312.3M in the run, 15.0M after | Before the claim is all the planner's; the design's session is not counted. | | | | | |
| `cost_claude` | $375: planner $262, supervisor $111, designer $2, retrospective $1; cache reads $137, writes $123, output $114 | Plus $111.49 for the design's `b357b336`, counted as $0. | $287: 174, 76, 21, 16 | $491: 53, 13, 424, 1 | $312: supervisor 276, designer 36, retrospective 1 | $231: 45, 114, 19, 54 | $197: supervisor 112, designer 32, retrospective 52 |
| `supervisor` | 2 sessions, 12.1 h, 1.2M weighted tokens an hour, 62% in the 40 requests after each start or compaction, 13 compactions | $1.98 a step. | 1, 8.5 h, 1.4M, 87%, 19 | 1, 2.5 h, 0.9M, 52%, 1 | 15, 13.4 h, 2.7M, 63%, 28 | | 6, 43.6 h, 0.4M, 17%, 0 |
| `tokens_per_step` | 15.2M | Coordinator 1.93M, implementers 7.66M, the run's Claude Code sessions 5.58M. With the planning, about 28M. | 12.1M | 13.8M | 14.4M | 23.0M | 23.4M |
| `coordinator_requests` | 943, 63 after a wait ran out | 16.8 a step. | 743; 17.3 | 150; 18.8 | 1,961; 11.7 | 1,169; 20.2 | 1,303; 29.6 |
| `compactions` | 3 | At 176K to 183K each time. | 2 | 0 | 4 | 2 | 2 |
| `gates_per_step` | 1.4 | 77 of the run's own land gates, 63 passed; 6 passing gates were wasted when `main` moved under them. | 1.1 | 1.2 | 1.3 | 1.8 | 1.5 |
| `gate_lane_other` | 11 | 5 for "Done when", 4 smith-bench probes, a gate after the reboot, a dogfood gate. | 25 | 8 | 21 | 103 | 148 |
| `queue_wait_ratio` | 1.26 | The run's own tasks alone 1.18: build lane 1.48, gate lane 0.22. 4.6 in the 12:00 hour. | 0.85 | 0.18 | 0.74 | 0.74 | 1.34 |
| `suites` | **skein:** focused 3,293 / 6.407 s → 3,597 / 5.424 s; fuzzy 104 / 40.635 s → 105 / 32.258 s; browser 38 / 22.548 s → 26.229 s; gate 164 s → 123 s. **smith:** focused 2,388 / 13.091 s → 2,453 / 13.427 s; fuzzy 43 / 15.438 s → 16.526 s; gate 34 s → 58 s. **jig:** 4,360 / 12.609 s → 10.684 s. **temper:** one gate. | Times follow the machine's load, not the test counts (section 5, item 9). | | | | | |
| `proofs` | unit 23, world 37, joint 7, process 5, 1 below plan; fuzzy 1, not a layer | World is 38 on `origin/main`. The one below plan is skein-05.4, judged by its last commit, a steer's. | unit 16, world 8, joint 19, process 3, 0 below | world 4, joint 4, 0 below | unit 52, world 61, joint 11, process 71, live 10, 0 below | unit 29, world 11, process 2, live 12, 4 below | unit 18, world 17, joint 3, process 25, 0 below |
| `report_lines` | 12 median, 13 p90 | 108 reports. | 12, 15 | 12, 14 | 12, 16 | 11, 16 | 16, 24 |
| `lines` | code +8,296 −2,547; tests +17,908 −4,115; generated +1,934 −390; data +1,166 −945 | Tests are 2.2 times the code. Data is +1,174 −953 on `origin/main`. | code +17,016 −1,623; tests +9,125 −1,118 | code +1,111 −175; tests +2,256 −307 | | | |
| `contexts` | 138K median, 188K p90; 33 handovers | 31 handovers at or above 100K, at a median of 132K. | 119K, 184K; 25 | 146K, 185K; 5 | 172K, 223K; 66 | 201K, 293K; 26 | 152K, 208K; 19 |
| `pin_lag` | smith: skein 7; jig: skein 7, smith 12; hearth: skein 7; temper: skein 3, smith 11, jig 5 | All four downstream repositories were repinned. The lag is other runs' landings since. | | jig: skein 27, smith 18 | | | |

**Not comparable, and why:**
- **`chain_step_min` and `chain_step_max`** are new (`0b006b8`, 14:34), and only this run has them.
  - The ledger keeps a step's last thread. With 33 handovers, a handed-over step counts from its successor, so the median is low.
  - A thread that landed two steps counts the first in the second's span. smith-01.2's 106.7 minutes include smith-01.1.
  - Counted step by step, the median span is 30.0 minutes and the 90th percentile 61.
- **`cost_claude`'s designer.** The window now takes only the Inputs' design commits (`baaebb7`).
  - Their session, `b357b336`, ran on hel8 and has no transcript on x1, so it counts $0.
  - It cost $111.49. Live-text's retrospective already counted it, under the old window.
  - About $50 of it was this plan's skein and smith design, about $42 jig's and temper's second part, and $20 shared.
  - The $2 is `fe8ad627`, the designer that answered 05.3's question in the run.
- **`running_h`** now ends at `done.md` (`ebf24d5`). tools' 8.5 h, above its `wall_h`, is the old reading.
- **`supervisor`'s 62%.** The supervisor's cycles between compactions were 41 to 79 requests long, so 67% of its requests fall within 40 of a start or a compaction, by construction. Those requests cost $0.054 each, the same as the others. render's proposal 11, a restore cost in its place, is still unbuilt.
- **`queue_wait_ratio`** counts every row of `heavy.tsv`, the director's dogfood and the owners experiment's 85 rows included. `supervision.md` (13:04) says the metrics leave them out; they don't.
  - A gate's drain of the build lane is booked as running: 5,556 of the gate lane's 14,276 s.
  - Counting the drain as waiting, the run's own ratio is 1.41 (proposal 10).
- **`proofs` and `lines`** read the local `main`, which lacks hearth `3a2130d`, landed from hetzner (proposal 10).
- **`escalations`** map kinds onto section 10.2's set since `baaebb7`. Compare totals with runs before tools.
- **`tokens_per_step`** counts the run only. The planning before the claim was 730M, 13.0M a step; tools' was 10.6M.
- **`cost_claude`'s retrospective** is this session's start. Earlier runs' figures include their finished retrospective.

**What explains the largest differences:**
- **12.8 hours, at tools' `step_h`.**
  - **The price.** The README priced its 17-step chain at "0.09 to 0.14 h a step, about 1.5 to 2.4 h", from earlier runs' `step_h`. At this run's 33.6 minutes a chain step, it is about 9.5 h. Live-text's proposal 6 and tools' proposal 5 made that change after this plan landed.
  - **The chain that set the end** ran through skein 08, 09 and 10, then skein-10.s1 and the repins: smith-repin, jig-repin, hearth-repin. On it:
    - disk holds, 89 minutes: 08.3 51, 08.4 12.5, 09.1 25;
    - the reboot: skein-10.2 landed at 21:21:33, as the machine went down, and its successor skein-10.s1 started at 22:32:36;
    - hearth-repin's 85 minutes.
- **The implementers' 7.66M a step,** 1.36 times tools'.
  - **smith's sessions** took 12.9M a step against skein's 5.4M. The dearest threads:

    | Thread | Step | Requests | Input | Why |
    |---|---|---|---|---|
    | w30 | smith-03.1 | 320 | 36.5M | 301 requests above 100K context; 8 failing clippy rounds; two compactions; a steer and a question mid-step; 35 minutes landing |
    | w22 | smith-04.1 | 201 | 24.6M | 183 requests above 100K; 37% only waited; 11 of 17 heavy tasks failed |
    | w20 | smith-02.1 | 210 | 22.4M | its first edit at request 46, at 148K; 42% only waited; 13 of 14 heavy tasks failed |
  - **Context.** 64% of all input was sent at contexts above 100K. Handovers came only after a landing, a median 32.5 requests past 100K.
  - **Waits.** 961 requests (21.3%) only waited, 24.4% of the input. 470 of them came back before 295 s with their command still running.
  - **Threads that ended on a question** and never landed took 30.0M (7%). jig-repin.s1 took 22.9M over five threads.
- **The queue's 1.26.**
  - Six sessions shared two build slots, and each gate paused the lane for 2 to 4 minutes, drain included.
  - The implementers caused 74% of their own wait.
  - The ready checks' and re-checks' 166 tasks waited 5.2 times their running, and caused 6.5% of the run's wait.
  - In the 12:00 hour the build lane waited 4.6 times its running: five sessions and five check agents started together.
  - 346 of the implementers' 762 tasks failed (45%), mostly the edit-compile loop: 127 compile errors, 118 lints, 69 test failures.
- **The supervisor's $111.**
  - Its main thread cost $46.41 and its 41 Sonnet sub-agents $64.11.
  - The 25 ready checks and re-checks cost $44.6, and turning their reports into rulings about $19.9 of the main thread: about 58% of the supervisor in all.
  - The 11 hourly checks cost $16.7. The 13 compactions' restores cost about $0.31 each.
- **The planner's $262,** the largest planner bill on record. It is $4.68 a step, below live-text's $6.6 and forgejo-17's $8.7.
  - Six Opus drafters cost $87.65.
  - Eight Sonnet fix agents and their helpers cost $78.29 to rewrite after the review's 113 findings, about three writes of each file.
  - Reviews and checks cost $36.91, reading $18.47, and the main thread $40.41.
  - The harness's own total is $336. The gap is compaction summaries, which leave no message record.

## 4. What worked

1. **The plan's aim was met.**
   - 56 of 56 steps landed, 2 of them added, none dropped.
   - Every member of skein and smith carries the licence.
   - The three repins downstream stayed under 200 lines each: smith-repin 169, jig-repin 79, temper-repin 151. No repair needed a question.
2. **Blocks were answered fast.**
   - The median was 3.5 minutes from a block to its ruling, and 14 of 15 took under 7.2.
   - The supervisor's watch woke in a median 13 s, and its share of the wait was about 91%.
   - The coordinator had the step running again in a median 3.8 minutes.
3. **The coordinator was never the bottleneck.**
   - From an implementer's landing report to `plan landed` took a median 3 s.
   - From a landing to the next increment of the same session took a median 62 s, of which the coordinator's share was about 20.
   - No `plan landed` check failed. It refused no steer point, read no code, and its spawn prompts did not grow.
   - Steers took 6 requests a ring, and unblocks 6.
4. **Steers reached their sessions.** 26 steers with 36 points; none refused. 11 of the 13 that handed a plan correction on reached their step before it landed.
5. **A re-check found a real defect.**
   - The 20:45 re-check of skein 10.1 found that `read_locked`'s `None` arm cleared the slot before `fail_exchange`, so a held grant ended as a refresh failure.
   - Ruling 144 made it skein-10.s1. It landed at 22:42, a one-line fix with a test that failed before the fix and passed after.
6. **The code kept the rules a gate checks.**
   - No file crossed 800 lines, and no new file was over 800. The largest new file is `openid.rs`, at 323 lines.
   - skein's `machine.rs` went from 820 lines to 606 by 05.3's move into `redirect.rs`, and ended at 767.
   - At least nine commit bodies say a split keeps a file under 800 lines.
   - No `#[allow]`, TODO or `todo!` landed. Nine `#[expect]` were added, each with a reason in a pattern the crate already uses, and six removed.
7. **The proofs were at their plans' layers.** The one counted below plan is an artifact of its last commit (section 3). Four smith steps added the process-layer check `smith_check_json_shows_each_value_with_its_layer` above their planned world.
8. **The reboot lost nothing.**
   - The coordinator resumed through `plan resume`, and found skein-10.2's landing, whose report the reboot lost, by git.
   - The supervisor's second session took over at 22:27.
9. **The one design question reached the design.**
   - Once a designer started, it answered in 5.9 minutes for $2.29: empty names are refused as `Limit`, and terms' entries are priced.
   - It landed skein `0ccaf59` (oauth.md 1, 5 and 6.7) and smith `d275fd98`.
10. **Memory never ran short.** Available memory stayed at 7.9 GB or more. No task was killed, and no disk event stopped the line.
11. **The supervisor kept its turns short:** 16K characters of text in 749 requests.

## 5. What did not

1. **The run took no fix that landed for it.**
   - **What happened.**
     - Between 13:14 and 14:44, twelve skein commits applied client-api's, live-text's and tools' retrospectives. Four of them fix defects this run met:
       - `e002687`, the pulse's aims verdict sums the packages;
       - `2cedfab`, the pulse times a short wait by when it returned;
       - `2349030`, a ring is one command, and a wait that runs out needs no touch;
       - `31ae5ab`, `plan wake` starts a fresh designer when none joined.
     - development.md 3.6 (in `1d105bc`, 13:43) says the next hourly check moves `kit/` to such a fix "once `<run>/kit/scripts/dev/test-plan` passes there".
     - The checks from 16:30 to 22:27, six of them, wrote "takeable by ruling once test-plan passes". None ran it. At 23:33: "two steps left, no kit move".
     - Of the twelve commits, only `1d105bc` says what this run does, and it says "keep". development.md 4.4 asks each to say.
   - **What it cost.**
     - The pulse's "fuzzy past 19" flag, which `e002687` removes, was diagnosed as false by hand at 9 of the 11 checks.
     - The coordinator woke up after every timed-out wait, as the kit's 7.4 asked: 63 times, 196 requests. 51 of them came after `2349030` landed.
     - 05.3's question waited 22 minutes for a designer that `31ae5ab` would have started (item 12).
   - **First visible:** 14:44, when the last of the four landed. The 16:30 check named them.
   - **Acted on:** no.
   - Proposal 1.
2. **Live-text's "Done when" defect recurred, and three commands were stale.**
   - **What happened.**
     - Item 4's command wrote `focused-$i.log` and `fuzzy-$i.log` into the worktree it measured. smith-bench's `a_replay_arm_runs_through_the_probe_command_and_prints_one_line` refuses a dirty tree, so the three focused runs stopped at test 18 to 20 and measured nothing. This is live-text's section 5, item 3.
     - Live-text's proposal 2 was decided at 13:22 and landed in development.md 5.2, 5.4 and 5.6 at 13:43. This plan's README had landed at 11:19 and kept the old command until ruling 152 at 01:23.
     - Item 2's three stale commands (section 2) were found at 01:16.
       - skein 07's check moved to `machine/fetch.rs` under ruling 118 at 15:41, and skein 07 landed at 18:08.
       - smith 03's pattern matched an enum that was already there when smith 03 landed, at 21:02.
       - skein 01's grep was broken by skein 11.1 at 19:20, after skein 01 had ended.
     - The planner's base run filed all three greps as EXPECTED, since a "finds nothing" check fails at the base by design. Four sessions, skein 10 and 11 and smith 04 and 05, were made by splits after the reviews, and no review read them.
   - **What it cost.** The two rulings and the re-measure held the release for 29 minutes.
   - **First visible:** item 4 at 13:22, in live-text's decision. The stale commands at 01:16.
   - **Acted on:** 01:19 and 01:23 (rulings 151 and 152).
   - Proposals 1 and 4. The probe test itself is section 9, smith item 3.
3. **The ready checks went stale, and every one was redone.**
   - **What happened.**
     - 12 ready checks and 13 re-checks gave 25 of the 52 rulings, each correcting session files against `main`.
     - Each of the 12 sessions with a ready check got a second ruling 0.6 to 3.8 hours later, once other sessions had landed in the same files: 101 to 117, 102 to 114 and 115, 103 to 108, 104 to 122, 105 to 123, 109 to 126, 118 to 128, 120 to 131, 121 to 136, 127 to 137, 132 to 143, and 138 to 144.
     - Five ready checks started at once at 12:13, into a build lane that the gates held paused for 30 of the first 67 minutes. Five of the 12 ran no suite: their tasks were removed unrun at the time box.
     - Three re-check rulings came after the code they corrected had landed: 108, 117 and 144.
     - Two corrections caused or missed a block: ruling 131 made block 9 (133), and the re-check behind 123 missed block 7 (124).
   - **What it cost.**
     - The 25 sub-agents cost $44.6, and the main thread's rulings from them about $19.9.
     - Their 166 build tasks waited 16,390 s against 3,173 s of running. In the first 67 minutes they were 36% of the queue's wait.
   - **Cause.**
     - development.md 7.2 sets a session's check "as soon as every session it needs at its start is running", hours before it can start. skein 09 was checked at 16:06 and could start only at 18:35, when 08.4 landed.
     - The run's own landings then moved the files the check had read.
     - tools' proposal 4 (`656ef69`, 13:14), ready checks on the named crates' rdeps only, landed a minute after the 13:13 check named this waste. Its body named no run, so the run did not take it. The supervisor's later prompts, its own text, kept "fuzzy included".
   - **First visible:** 13:13. The check counted 29 build tasks waiting 4,937 s.
   - **Acted on:** no.
   - Proposal 2.
4. **Test shape was steered at 10 of the 11 hourly checks.**
   - **What happened.**
     - Every hourly check found a breach of testing-strategy.md section 10. Five README rules followed (rulings 110, 119, 125, 134 and 142), and three `later.md` rulings (146, 148 and 150).
     - 21 of the 56 steps landed with a breach. 15 of them landed after rule 119, "a table test's rows are data", existed; 12 after rule 134.
     - 7 of the 10 test-shape steers aimed at code that had already landed, and relied on the same session's next increment. Where the session had ended, nothing carried them, and the landed tests went to `later.md`.
     - skein-08.2's report said "Tables follow ruling 119", and it landed label rows.
     - The repins started without the steers. smith-repin retyped `6 * string_bytes + 267` after rule 125 named sizes.
   - **Cause.**
     - The plan's own reference tests broke section 10. Ruling 125 says skein 08's asserted on the way, and held both sides of a bound in one test. Live-text's proposal 4, the planner's review of the reference tests, came after this plan.
     - development.md 14.4 makes a re-sent rule a gate check where a script can test it. The supervisor wrote "a script cannot test it" (119), and the parts a script can find were not separated out:
       - a literal that repeats a named constant: `const DIALECT: u32 = 1` in four jig files, the issuer's floor in six places, `b"client-1"` seven times;
       - an `_and_` name, which `land` refuses since `ebf24d5`, not in this kit.
     - render's proposal 6, the same gate check, was adopted on 10-18 and never built.
   - **First visible:** 13:13 (skein 01.2's `credentials.rs`).
   - **Acted on:** at each check, by steer; landed tests left as landed.
   - Proposal 7.
5. **Disk holds stood on the chain.**
   - **What happened.**
     - The coordinator held new starts below the 40 GB floor for about 200 minutes, 25% of the run, and below 60 GB, the six-session cap, for 78% of its samples.
     - Steps whose needs had landed waited 258 minutes in all for disk. 89 of them were on the chain that set the end: 08.3 51, 08.4 12.5 and 09.1 25.
     - Disk fell to 19 GB at 17:03. At 16:46, the alert said that `tmp/director` held 17 GB outside the run. It was down to 57 MB by 18:41.
     - The coordinator freed what development.md 11.4 asks: finished sessions' targets, then `kache gc`, 54 times. At 19:00, `cargo clean` on the ended jig-repin.s1 thread's worktree freed 19.3 GiB and ended the last hold.
   - **Cause.** 11.4 makes a hold no block, so nobody is asked to free what lies outside the coordinator's reach. The supervisor logged the hold at 18:41 and 18:56 ("each hour held is an hour on smith-repin"). Its only recorded action was a `du` at 18:55.
   - **First visible:** 12:29, and on the chain at 15:54 (08.3).
   - **Acted on:** by the alert at 16:46; the director's cleaning by 18:41.
   - Proposal 3.
6. **hearth-repin took 85 minutes for a Cargo.lock commit.**
   - **What happened.**
     - Two gates timed out at 15 s, at 491 and 511 of 2,149 tests (block 14, 23:31).
     - A sub-agent found hearth's own `main` failing alike. `tests/tree/tests/memory.rs:321` takes 12.6 to 14.1 s on x1, and the suite 22.6 to 25.4 s, against 6.5 to 8.8 s on hetzner. hearth's `3df74d3` had landed at 23:03, 24 minutes before hearth-repin started.
     - Ruling 147 (23:38) held the step "for a later unblock or steer".
     - The director landed `c4d0231` at 23:46 and rang at 23:47:38. The coordinator logged the ring as development.md 7.4 says, and idled from 23:58. The supervisor slept until its hourly wake at 00:38. `plan await` waits only for a Step trailer, and `c4d0231` has none.
     - The next gate failed again (498 of 2,150 at 15.017 s). Ruling 149 (00:43) gave the choice to the director, who chose to land on hetzner, at 00:51.
   - **Cause.** hearth's own landing broke its gate on the slowest machine that lands to it. Then the wait for the fix had no means to wake anyone.
   - **The planner** ran no hearth, jig or temper gate at the base. The base would have passed, since `3df74d3` came later.
   - **First visible:** 23:31. **Acted on:** 23:38 and 00:43. bc6c16c (23:41) put "main's gate holds on every machine, x1 the slowest" in owners.md.
   - Proposal 8. hearth's suite is section 9, hearth item 1.
7. **jig-repin.s1 spent 99 minutes on a flake.**
   - **What happened.**
     - jig's browser story `under_a_prefix_the_inbox_s_links_carry_the_prefix` failed at person step 9, waiting for "Cancelled", in three of four gates.
     - Rulings 135 and 139 gave budgets of 45 and 60 minutes, with 30 focused runs. All of them passed except one at the base pins, which also fail it "if less often seen".
     - It landed under the README's known-flake rule at 20:31.
     - Its `later.md` row, owed since 19:31, came with ruling 145 at 21:14.
     - Two threads, w42 and w45, both ran `land` on the same worktree after a handover that ruling 139 overtook. w45's landed it; about 4.4M tokens were spent twice.
   - **Cause:** a race in jig's client, older than the run.
   - **First visible:** 18:52. **Acted on:** 19:03 and 19:31.
   - No proposal. The race is section 9, jig item 1. The known-flake rule ends with this README.
8. **The 15 s budget failed 7 of the 14 failed land gates.**
   - **What happened.**
     - smith's gates timed out four times (rows 160, 433, 494 and 511), each with emacs at about 100% CPU when the gate started; hearth's three times.
     - smith's passing gates took 11.8 to 14.5 s, a median of 13.2, against 15. Five workspace runs in the build lane also met the timeout.
     - At the base, smith's suite took 13.1 s, 87% of its budget. The planner's note to the director warned of it at 10 s.
   - **Cause.** Contention, not growth: smith's time does not grow with its test count. Its slowest test, `native_validated_dense_prompts_fit_the_actual_joint_receiving_ceiling` (`config.rs:1399`), takes 3.6 s alone and 5.3 to 6.5 s loaded, and the margin is 0.5 to 3 s.
   - **First visible:** 13:53 (row 160). **Acted on:** each gate passed on its next try. The 17:35 check named the budget for the director, and no record of it reached `later.md`.
   - Proposal 5. smith's suite is section 9, smith item 1.
9. **skein's aims were met only on a quiet machine.**
   - **What happened.**
     - Over the 43 passing skein land gates, the aims' package sums were 15.8 to 28.7 s focused and 17.5 to 28.6 s fuzzy. Both aims were met only at row 896, at 22:42 after the reboot, and at an owners gate at load 2.48.
     - "Done when" ran at load 7.4 to 8.8, one second after an owners smith gate ended, while the graph was being re-indexed.
     - smith's re-measure met its aims by 0.19 s and 0.83 s, because the median of three dropped a cold first run 50% slower.
   - **Cause.** The aims are a sum of single tests' times, which load stretches. The six slowest skein tests date from 10-04 to 10-09, not from this run.
   - **First visible:** 12:23, at the first gate. 9 of the 11 checks said "load, not growth".
   - **Acted on:** ruling 152, with "For the user" E.
   - Proposal 6.
10. **The implementers waited, without the graph.**
    - **Waits.**
      - 961 of 4,503 requests (21.3%) only waited, 24.4% of the input: 32.3% of requests before the 13:25 steer and 19.9% after.
      - The steer reached w3, w5 and w7. Later threads read a README with no wait rule, and a card, `b0510fa`'s implementer.md 8.3, that says "`wait` ... `yield_time_ms: 300000`" without the `// @exec` form or the 31 s return.
      - Polls with `// @exec: {"yield_time_ms": 300000}` never came back early: 0 of 256.
      - Four threads (w30, w20, w22, w27) made 258 of the 361 early returns after the steer.
      - client-api's proposal 3 is in implementer.md since `1d105bc`.
    - **The graph.**
      - 53 of the 108 reports say the graph was unavailable, and 54 of the 55 threads read the graph's `SKILL.md` anyway.
      - The supervisor re-indexed the graph five times, for nobody.
      - w50 to w54, spawned after the 22:27 `plan resume`, had the graph tools and made 15 calls. So a resumed coordinator's spawns do not inherit `enabled = false`. That is the probe live-text's decision 3 asked for (an inference from the timing; section 9, kit).
    - **Handovers.** 33, at a median of 132K. The 100K check comes only at a landing, and a first thread often passes 100K within its first increment. Successors started at about 60K and peaked at a median of 139K. A handover buys one cheap start, not a long run below 100K.
    - No proposal. client-api's proposal 3 and live-text's proposal 3 are adopted. Their texts are in implementer.md, and live-text's config part is in section 9.
11. **The README priced its chain at `step_h` again.**
    - At 0.09 to 0.14 h a step, against this run's 33.6 minutes.
    - The 17:30 projection used 35 minutes a step, and still put smith-repin at about 21:15. It landed at 22:57: the holds, the reboot and jig's flake.
    - No proposal: live-text's 6 and tools' 5 landed after this plan.
12. **05.3's design question waited 22 minutes for a designer.**
    - **What happened.**
      - At 13:35, `plan wake designer` refused ("no designer recorded"), and the supervisor sent a push notification.
      - Nothing happened until the second block at 13:52, when it wrote to the director's inbox. A designer started at 13:57 and answered at 14:02.
      - 05.3 went on short of landing meanwhile (ruling 111).
    - **Cause:** `plan wake` started no one. `31ae5ab` (14:36) fixes it (item 1).
    - **Acted on:** 13:54.
    - Proposal 1.
13. **The planner wrote each session file about three times.**
    - **What happened.**
      - Six Opus drafters wrote the 16 session files for $87.65. Eight reviews then found 113 points, and eight Sonnet fix agents rewrote the files for $63.09, with helpers $78.29 in all.
      - No review prompt named testing-strategy.md section 10. Four sessions made by splits after the reviews were not reviewed.
      - The planner ran no gate in jig, hearth or temper.
      - Disk fell to 1 GB at 10:29, with its two base worktrees' targets at 25 GB.
    - **What the run then corrected,** besides the 25 Shapes rulings:
      - ruling 107: `secret N`, which skein's codec.md 3 already says;
      - ruling 113: what each skein step leaves unsupported;
      - ruling 130: decision 16's "handled as the rejected one was", without reading what jig does on a rejected account;
      - ruling 141: live-text's 490,000,000-byte floor, which the planner read at 09:29 and did not measure against smith 03's growth.
    - No proposal for the facts. The review's order is proposal 9.
14. **The supervisor compacted 13 times.**
    - Its usable window was about 110K a cycle.
      - 38% went to its own output. A ruling's text was typed three or four times: the edit script, the ruling file, the unblock or steer.
      - 62% went to new input: plan files, `git grep` premise checks, watch output (a `started:` event prints a `git diff --stat` of up to 150 lines), the cards re-read after each compaction, and the sub-agents' reports.
    - The restores cost $0.31 each. Their number is the cost, not their price.
    - The watch was found stopped after two compactions, and the 12:50 block waited 106 s for it.
    - No proposal: proposal 2 removes most of the reports.
15. **Small defects.**
    - `plan metrics` read the local `main` (section 3).
    - Three ledger rows (skein-04.1, 08.1 and 03.2b) carry the task name `/root/wNN`. `spawn_agent` returns no thread id, and those steps landed before the coordinator's next batch replaced the names.
    - The pulse:
      - its Record flag's `repin,` pattern lacks `.s1` (19:46, 20:52);
      - it counted the owners experiment's gates as the run's ("3 land gates for 1 landing", 23:33);
      - it counted a moved `#[expect]` as new (16:30);
      - it counted smith 02.1 twice (17:35).
    - `status.md` kept both answered alerts until the end.
    - The supervisor's two "for the director (later.md)" notes, skein's fuzzy suite at 49.5 s of 60 and smith's budget, are in no file.
    - The Sizes:
      - Seven steps went over 1.5 times their Size by the plan's command. Four of them (05.3, 05.4, 05.5 and 07.1) only because the command does not exclude `machine_tests/*.rs`; without those files they are 0.77 to 1.16 times.
      - The command counts added lines, and the one implementer who wrote a number counted changed lines.
      - In all, 0.88 times the planned lines.
    - Section 9, kit.

## 6. Decisions taken during the run

52 rulings, 101 to 152, each a smith commit "Ruling N of accounts", and 26 steers. By kind:

| Kind | Rulings | Reached the design? |
|---|---|---|
| Shapes against `main`, from ready checks and re-checks (25) | 101 to 105, 108, 109, 114, 115, 117, 118, 120 to 123, 126 to 128, 131, 132, 136 to 138, 143, 144 | Plan only. 128 (fake issuer literals use `Method::Post`; world token POST times at least 60) is "not in the design yet" (`done.md`). 144 added skein-10.s1, which follows oauth.md 6.4 |
| Blocks (15) | 106, 107, 111, 112, 113, 116, 124, 130, 133, 135, 139, 140, 141, 147, 149 | Only 112, as skein `0ccaf59`, oauth.md 1, 5 and 6.7 (below) |
| Test-shape rules (8) | 110, 119, 125, 134, 142 to the README or a session file; 146, 148, 150 to `later.md` | Plan rules. testing-strategy.md section 10 says most of them already. 148's "fixture constants once per crate" and "setup drains earlier events" are "not in the design yet" |
| Order and steps (2) | 129, the director's: `jig-repin.s1` now, and the start order toward the repins; 144, skein-10.s1 | Plan only |
| "Done when" (2) | 151, three stale commands; 152, the aims, with "For the user" E and a `later.md` row | Plan only |

The decisions whose substance belongs in a design:

| Ruling | Commit | What | Reached the design? |
|---|---|---|---|
| 112 | `8ab73679`, 14:03 | An empty added-parameter or sign-in-again name is refused as `Limit`, as an empty client id is; an empty value is admitted; `client_worst_case` prices the entries | **Yes:** skein `0ccaf59` (14:01), with smith `d275fd98` (05.3's Size 240 to 275), by designer `fe8ad627` |
| 130 | `b638629f`, 17:58 | jig's and temper's repins: `Refused` reaches no downstream event and adds no hub accessor; `Unauthorized` maps to `Rejected`, `Limited` to `Exhausted`, an absent reset to zero | **No:** "not in the design yet". An unknown reset reads as "available now" (section 9, jig item 4) |
| 141 | `617c6e72`, 20:02 | smith's spare-memory floor is 460,000,000 bytes, and a later margin below it is a question, never a lower floor | **No:** the floor is in no design document |
| 147, 149 | `1248e325`, 23:38; `8ca7eab9`, 00:43 | hearth's suite on x1 is outside the run; the director's choice: land the unchanged pin on hearth's gate machine | **In part:** owners.md's "Every machine's gate" (`bc6c16c`, 23:41), the principle only; the choice is in no document |
| 128, 148 | see above | Test-library conventions | **No** |

tools' proposal 3, adopted at 12:30 (`656ef69`), says a question decided without a designer reaches the design before the next plan. Rulings 130 and 141 are such questions, and `accounts-sources`, the next plan, builds on both.

**"For the user."**
- A to D were decided by the director before the plan landed. They are the licence's copyright line, smith's fault names, the repins' defaults downstream, and no live check.
- C was relied on in ruling 130.
- E was added by ruling 152 at 01:23, and has no answer yet.

**The design questions** 6 to 16 were answered by the designer before the plan landed, each as recommended: smith `137a1cb4`, skein `d4ef1680`, `378fd8cf` and `49725eca`.

## 7. The proposals adopted after the last retrospective

The last retrospective before the claim was client-api's. Its section 10 adopted all nine proposals at 01:43 on 10-19, each "to apply". None was in `b0510fa`. Their texts landed in `656ef69` (13:14) and `1d105bc` (13:43), and their kit parts from 14:18 to 14:44, all after the 12:06 claim. This run took none of them, so none was in force. The table says what this run's numbers show anyway.

| Proposal | Meant to move | client-api | accounts | Moved |
|---|---|---|---|---|
| 1. Long plans run as consecutive plans | plan-defect escalations a step; rulings a step; `step_h` | 0.47; 1.9; 0.38 | 0.07 (0.11 counting 107 and 130); 0.93; 0.20 | In part, by the director's split into `accounts` and `accounts-sources` (decision 1), not by the rule |
| 2. The rules every implementer reads stay under about 300 lines | context at request 30; implementers' tokens a step; points refused | 96K; 21.1M; 18 | not measured; 7.66M; 0. The README's rules grew from 82 to 145 lines through seven steers | Not applied |
| 3. The pulse counts a wait by when it returned; implementer.md names `// @exec` | requests that only poll | 28.5% | 21.3% of requests, 24.4% of input | A little, by the 13:25 steer; not by the rule. The card still had the old wait |
| 4. A kit fix for a defect the run meets moves `kit/` | process blocks with a fix on `main`; the longest check gap | 5; 4.2 h | four defects met with a fix on `main`, none taken; gaps of 65 to 66 minutes, 95 across the reboot | **No.** The rule landed during the run, which did not take it (section 5, item 1) |
| 5. A ring is one command, and a timed-out wait needs no touch | coordinator requests a step; its tokens a step | 29.3; 5.96M | 16.8; 1.93M | Yes for the ring: the first `plan steered` and `plan unblocked` were in `b0510fa`. No for the wait: 63 wake-ups, 196 requests |
| 6. Sub-agents and woken designers read named ranges | supervisor sub-agent dollars a step; designer dollars a question | $2.13; $6.5 | $1.14; $2.29 | Yes, by habit: the sub-agents' reads were ranges (119 `sed` or `grep` of development.md against 8 `cat`); the brief part was not in the kit |
| 7. No Proof lands "unavailable" | hours a regression goes unseen | 27 | No process tier | Not tested |
| 8. Unallocated btrfs space is a bound | infrastructure escalations | 6 | 0 | Not tested; the disk floor held starts instead (section 5, item 5) |
| 9. `plan metrics` maps kinds and words | unmapped kinds and words; misread landings | 4, 4; 1 | none unmapped; the local `main` is a new misreading (proposal 10) | Yes, at this retrospective's metrics |

**Other adopted proposals unapplied at the claim.**
- forge-setup's 6, the pulse's aims verdict, as kit: `e002687`, 14:34. The run met it at 9 checks.
- render's 9, the design's sessions by commit: `baaebb7`, 14:19, at the metrics only.
- render's 11, the supervisor's restore cost: still unbuilt, so the 62% above is the old measure.
- client-api's 8, as kit (`plan check`, `plan resume` and the pulse on btrfs): unbuilt.

**The kept commit, and the changes during the run.**
- **Kept.** The run kept `b0510fa`, as every check's "Process: none taken" says.
- **Landed during the run,** in skein's `docs/foundation` and `scripts/dev`:

  | Commit | UTC | What | Serves | Taken? |
  |---|---|---|---|---|
  | `656ef69` | 13:14:48 | tools' retrospective: splits before drafting, the upstream map, design answers to the design, ready checks on rdeps, the chain's price | tools 1 to 5, 7 | No: "its body names no run" (13:13) |
  | `1d105bc` | 13:43:17 | client-api's and live-text's texts | client-api 1 to 4, 6 to 8; live-text 1 to 6, 8 | No: it says runs in flight keep their process |
  | `a02b122` | 14:01:36 | test-plan's checks in three parts | the kit's gate | No, and not needed |
  | `ebf24d5` | 14:04:16 | `land` refuses "and" test names and org-named pins; `running_h` ends at `done.md` | tools 6 to 8 | No; the metric only |
  | `e1053a7` | 14:18:16 | a fresh designer reads earlier answers | client-api 6 | No |
  | `4134983` | 14:18:16 | `plan check` refuses an adopted proposal not yet applied | live-text 1 | No. It would have refused this claim |
  | `2cedfab` | 14:18:16 | the pulse times a short wait by its return | client-api 3 | No; defect met |
  | `baaebb7` | 14:19:59 | metrics read the Inputs' design commits, and map kinds, words and causes | live-text 5, client-api 9 | At this retrospective |
  | `0b006b8` | 14:34:47 | metrics measure a step's span | live-text 6, tools 5 | At this retrospective |
  | `e002687` | 14:34:47 | the pulse sums an aims line's packages | live-text 7, forge-setup 6 | No; defect met at 9 checks |
  | `31ae5ab` | 14:36:40 | `plan wake` starts a fresh designer when none joined | this run's finding | No; defect met at 13:35 |
  | `2349030` | 14:44:56 | a ring is one command; a wait that runs out needs no touch | client-api 5 | No; defect met 63 times |
  | `7360708` to `bc6c16c` | 16:29:20 to 23:41:15 | the owners experiment, nine commits; `387a65d` and `c139fd1` changed every agent's `AGENTS.md` | owners | Not for this run. Read from the primary checkout, `AGENTS.md` reached new sessions as the checkout moved; which ones read it is not known |

- **Retrospectives decided during the run.** tools' (12:30) and live-text's (13:22) adopted eight proposals each. None reached this run. Five of them would have shaped this plan, had it been written after them:
  - live-text's 2: "Done when" writes outside the tree it measures;
  - live-text's 4: the review checks the reference tests;
  - live-text's 8: the sketches compile against the base;
  - tools' 2: the upstream map;
  - tools' 5: the chain's price.

**What this says of the loop.**
- This is the third run in a row to inherit an adopted proposal's defect: tools, live-text, and now this one.
- Here the fixes were on `main` 2.6 hours into a 12.8-hour run. Section 3.6's rule landed in the same commit that told the run to keep its process.
- The supervisor did by hand much of what the fixes do, usually after the failure:
  - it summed the packages;
  - it steered the 300 s waits;
  - it made a README rule from each steer;
  - it moved "Done when"'s logs outside the worktree, at ruling 152.
- The gap is the step from "takeable" to taken (proposal 1).

## 8. Proposals

Proposals 1, 2, 4, 6 and 9 change development.md and its cards, for runs under development.md. Proposals 3, 5, 7, 8 and 10 change tools and rules that owners use too: the disk floor, the suites' budgets, `land`, `plan await` and the metrics.

1. **The hourly check that first names a fix for a defect the run has met runs `test-plan` in `kit/` and moves the kit in that check, or writes why not; and a decided retrospective's proposal that corrects this plan's own files is applied to them by ruling at the next check.**
   - **Evidence:** section 5, items 1, 2 and 12; section 7.
     - Four fixes were on `main` from 14:44. Six checks wrote "takeable once test-plan passes", and none ran it.
     - The same false aims flag was diagnosed by hand at 9 checks, and the coordinator made 51 needless wake-ups after `2349030`.
     - Live-text's proposal 2 was decided at 13:22, and this README's item 4 failed with it at 00:55.
     - Of the twelve commits, one said what this run does.
   - **Changes:** development.md 3.6 and 14.1 (step 2); supervisor.md 3 and 6.
   - **Moves:**
     - fixes for met defects left untaken at the release, 4 to 0;
     - checks that diagnose a flag a landed fix removes, 9 to at most 1;
     - "Done when" items failing for a cause a decided retrospective names, 1 to 0.
2. **A session's ready check is set when the increments it needs at its start are the last ones still to land in their sessions, not when those sessions start; the re-check at its start stays, and runs only on what `main` changed in its files.**
   - **Evidence:** section 5, item 3.
     - 25 of 52 rulings came from checks, and all 12 early-checked sessions got a second ruling 0.6 to 3.8 hours later.
     - Five of the 12 ready checks ran no suite.
     - The 25 sub-agents cost $44.6, and about $19.9 of the main thread went to their rulings.
     - Their tasks waited 5.2 times their running.
     - skein 09 was checked at 16:06 and could start at 18:35.
   - **Changes:** development.md 7.2 and 10.4; supervisor.md 5.
   - **Moves:**
     - sessions with two Shapes rulings, 12 to at most 4;
     - ready and re-check sub-agent dollars, $44.6 to under $25;
     - their tasks' queue wait, 16,390 s to under 5,000.
3. **A start held by the disk floor for more than ten minutes is filed as an infrastructure block that names what fills the disk, inside the run and outside it, so that the supervisor or the director frees it.**
   - **Evidence:** section 5, item 5.
     - 200 minutes of holds, and 258 step-minutes held, 89 of them on the chain that set the end.
     - `tmp/director` held 17 GB outside the run at 16:46.
     - The coordinator followed 11.4 to the letter, and no one else was asked.
   - **Changes:** development.md 11.4 and 10.2; supervisor.md (a disk block); for owners, the disk floor's message.
   - **Moves:**
     - step-minutes held by disk, 258 to under 60;
     - minutes held on the chain, 89 to under 15.
4. **Each session's "Done when" commands run at its last landing, by the sub-agent that checks the landing, and a command that fails there is corrected by ruling while its session's thread is still open.**
   - **Evidence:** section 5, item 2.
     - Three stale commands were found at 01:16. Two of them were stale when their own session landed: skein 07's at 18:08, and smith 03's at 21:02.
     - "Done when" and its rulings held the release for 29 minutes.
     - The planner's base run cannot test a "finds nothing" check.
   - **Changes:** development.md 5.2 ("Done when") and 7.3.
   - **Moves:**
     - commands corrected at the end, 3 to at most 1;
     - minutes from `done.md` to the release, 32 to under 15.
5. **A plan whose repository's default suite takes more than four fifths of its budget at the base starts with a step there that finds the slowest tests' cause and removes it, before any increment adds tests; the budget is never raised.**
   - **Evidence:** section 5, item 8.
     - smith took 13.1 s of 15 at the base, and its gates 11.8 to 14.5 s.
     - 7 of the 14 failed land gates were the 15 s timeout, and 5 more runs met it in the build lane.
     - The next plan adds about 1,900 to 3,300 lines in smith.
     - hearth's suite, at 22.6 to 25.4 s on x1, is the same budget met from the other side: a landing that only passes on a faster machine.
   - **Changes:** development.md 5.4 (the base runs) and 11.5 (budgets); for owners, owners.md's "Every machine's gate".
   - **Moves:**
     - land gates failed on a global timeout, 7 to 0;
     - smith's default suite at the next plan's base, 13.4 s to under 12.
6. **The test-time aims are measured on a quiet machine: "Done when"'s timing waits until the one-minute load is under 2, with no graph indexing, and takes the median of three runs after a discarded first.**
   - **Evidence:** section 5, item 9; section 2, item 4.
     - skein's aims were met only in quiet gates, and "Done when" ran at a load of 7.4 to 8.8, right after an owners gate.
     - smith's re-measure met its aims by 0.19 and 0.83 s because the median dropped a cold run 50% slower.
     - "For the user" E is the result.
   - **Changes:** development.md 5.2 (the aims, and "Done when"'s form); the README's measuring command.
   - **Moves:**
     - aims missed under load, 1 to 0;
     - the spread of one measurement's runs, 50% to under 10%.
7. **`land` refuses a new test whose body repeats, as a bare literal, a constant its crate names, as it refuses `_and_` names since `ebf24d5`.**
   - **Evidence:** section 5, item 4.
     - Test shape was steered at 10 of 11 checks, and 21 of 56 steps landed with a breach, 15 of them after a rule named it.
     - 7 of 10 test-shape steers came after the code had landed.
     - The parts a script can find recurred across repositories: `DIALECT` four times, the issuer's floor six times, `b"client-1"` seven times.
     - development.md 14.4 asks for such a gate check, and render's proposal 6 adopted it.
   - **Changes:** `land` (skein `scripts/dev`); development.md 14.4; implementer.md (the refusal, and how to name the constant).
   - **Moves:**
     - test-shape steers, 10 to at most 3;
     - steps landing a breach, 21 of 56 to under 5.
8. **A ruling that holds a step for a fix outside the run names the commit it waits for, or the repository and path, and `plan await` wakes the coordinator when that lands.**
   - **Evidence:** section 5, item 6.
     - hearth-repin idled 51 minutes after the director's `c4d0231`.
     - Ruling 147 waited "for a later unblock or steer", and the coordinator handled the bare ring as 7.4 says.
     - `plan await` takes only a Step trailer, and the supervisor's 23:33 note asks for this.
   - **Changes:** development.md 7.4 and 10.3; `plan await`.
   - **Moves:** minutes a step waits after the fix it needs has landed, 51 to under 5.
9. **The planner's drafters get the review's checks in their brief (the literal reading, the consistency and trace checks, and testing-strategy.md section 10 for each test shown), and every session file is reviewed once, after its last split.**
   - **Evidence:** section 5, item 13.
     - The planner cost $262: $87.65 to draft, then $78.29 to rewrite after 113 findings.
     - No review read section 10.
     - Four sessions split after the reviews were never reviewed, which is partly how ruling 151's commands came about.
     - Rulings 107, 113, 130 and 141 are facts a review could have read.
   - **Changes:** development.md 5.4 (the review); the planner's drafting brief.
   - **Moves:**
     - planner dollars a step, $4.68 to under $3;
     - the share of the planner spent rewriting after review, 30% to under 10%;
     - plan-defect blocks, counting those filed as scope, 6 to at most 2.
10. **`queue_wait_ratio` counts a gate's drain of the build lane as waiting and leaves out other workloads' rows, and `plan metrics` reads each repository's `origin/main`.**
    - **Evidence:** section 3, "Not comparable".
      - 5,556 of the gate lane's 14,276 s running were drain.
      - The ratio counts the dogfood and owners rows that `supervision.md` says it leaves out.
      - The local `main` lacked hearth `3a2130d`, so `proofs` and `lines` are off.
    - **Changes:** development.md 15.2 (`queue_wait_ratio`, and the commits every metric reads); `scripts/dev`'s metrics.
    - **Moves:**
      - figures that differ from `origin/main`, 2 to 0;
      - queue figures that count other workloads, 1 to 0.

## 9. Debts left in the code

For each repository's `docs/development/tech-debt.md`. Only smith and jig have one, and neither lists any of these. They are ordered by what they cost `accounts-sources`, the next plan.

**smith**

1. **The default suite has no room under its 15 s budget.**
   - **Where:** `.config/nextest.toml`, `global-timeout = "15s"`. The slowest tests:
     - `crates/smith-agent-shell/src/config.rs:1399` `native_validated_dense_prompts_fit_the_actual_joint_receiving_ceiling`, 3.6 s alone and 5.3 to 6.5 s loaded;
     - smith-bench's `probe_command` test, 2.1 to 3.2 s;
     - smith-bench's `offline recorded_catalogues_and_results`, 2.0 to 3.1 s;
     - smith-tools-world's `a_domain_with_every_kit_full_stays_within_its_worst_case`, 1.7 to 2.7 s.
   - **What it cost:** four land gates and five build-lane runs.
   - **Since:** these tests date from 10-05 to 10-11. The run added 65 tests and about 0.3 s.
   - **Needs:** the cause found before the next plan's first gate (proposal 5).
2. **The memory margin is 4.02 MB above the floor.**
   - **Where:** `crates/smith-agent-shell/src/config.rs:1606`, `>= 460_000_000`, lowered from 490,000,000 by ruling 141.
   - **How much:** 1,750,572,809 bytes complete, against 2,214,592,512 declared.
   - **Cause:** `Kind::First` and `Kind::Window` gained `account` and `backend` (`crates/smith-domain-session/src/record.rs:119-137`), and `Turn` grows with its `Kind`. That is about 27 MB, 22 MB of it from `61342a7a`.
   - **Risk:** the next plan's program backend and its accounts sit on the same sum. The floor is in no design.
   - **Needs:** a decision before the first increment that grows `Turn`: box `Kind`'s payload, which ruling 141 declined, or re-price the declared memory.
3. **smith-bench's probe test fails in any dirty checkout.** This is live-text's smith item 1, still open.
   - **Where:** `benchmarks/tests/probe_command.rs:105` builds an arm from the checkout, and `benchmarks/src/arm.rs:97` refuses it.
   - **What it cost:** "Done when" item 4, and nine of the implementers' runs, 1,704 s. The message names no file.
   - **Since:** `98163c75`.
   - **Needs:** the test builds from a clone of its own. The next plan changes the arms (benchmarks.md 6, 7 and 10).
4. **Long files grew.**
   - `crates/smith-agent-service/src/service.rs`: 2,920 to 3,087 lines; `9bd292ba` alone added 74.
   - `tests/agent-process/src/world.rs`: 1,433 to 1,648.
   - `tests/channel/tests/stories.rs`: 1,255 to 1,404.
   - `crates/smith-events/src/write.rs`: 2,190 to 2,283; `read.rs`, 1,865 to 1,941; `responses.rs`, 1,855 to 1,919.
   - Near 800, on the next plan's path:
     - `crates/smith-local-service/src/credentials.rs`, 759;
     - `crates/smith-local-protocol/src/terminal.rs`, 774;
     - `crates/smith-local-shell/src/settings/tests.rs`, 756;
     - `crates/smith-protocol-llm/src/component.rs`, 768;
     - `crates/smith-host-protocol/src/component.rs`, 775.
   - Needs code-organization.md 7's consolidation, before an increment adds more than 40 lines to `service.rs`, `config.rs` (1,712) or `credentials.rs`.
5. **The withdrawn forms, marked for `accounts-sources`** (`later.md` rows 1 to 8):
   - `crates/smith-agent-shell/src/config.rs:182`, `_account`, ignored;
   - `crates/smith-protocol-channel/src/credentials.rs:20-22`;
   - `crates/smith-agent-service/src/service.rs:926,938`;
   - `crates/smith-local-service/src/service/accounts.rs:56`;
   - `crates/smith-local-shell/src/credentials/product.rs:161`, a fixed 60 s `refresh_lead`;
   - the `unreachable!` driver events in `crates/smith-local-service/src/credentials.rs:513,738-750` and `credentials/private.rs:83,95,115`, which `smith logout` makes reachable.
6. **Landed tests that break section 10** (`later.md`):
   - `crates/smith-agent-service/src/tests/accounts.rs:15-22,39`: bare `Limits { 256, 1024, 8 }`, `account: 7` and `reset_ms` in long rows;
   - `tests/local-process/src/process.rs:128`, the issuer's floor retyped;
   - `tests/protocol-llm/tests/adapter_boundary.rs:204,239,262`, a function in a row.

**skein**

1. **The fake issuer's response floor is typed in six places, in three repositories.**
   - **Where:** `testing/skein-fake-peers/src/oauth.rs:633-635` computes it inline. It is retyped as `256 + 6 * 1024 + 267` at:
     - `tests/oauth-accounts/src/world.rs:66,1029,1169` and `sign_in/protocol.rs:64`;
     - smith's `tests/local-process/src/process.rs:128`;
     - jig's `tests/protocol-accounts/src/fixtures.rs:8-9` and `world.rs:130-131`.
   - **Risk:** the next plan moves the issuer to a sign-in account's addresses. A new field changes six files, or the peer refuses with `Error::Limits` and no pointer to the cause.
   - **Needs:** one exported function in skein-fake-peers, then the repins.
2. **The oauth-accounts world's sign-out stories bypass the issuer's revocation route.**
   - **Where:** `tests/oauth-accounts/src/world.rs:1124-1136,1300` point the token endpoint at `/revoke`, and `sign_out.rs:29,33` takes a story it ignores.
   - **Since:** skein 09's report said this lasts "until the hosted issuer's separate route lands in skein 11". `21ad555` landed it, and the world was never switched.
   - Two near-copies of the stream pump, `world.rs:1117` and `sign_in/protocol.rs:13`, each sit behind a `too_many_lines` expect in tests, against the Don't list.
3. **The keep-time flake** (`later.md`, ruling 116).
   - **Where:** `tests/world/tests/real.rs:129`, whose 5 ms deadline is taken before the world's setup. It failed once in 56 runs.
   - **Same pattern, unrun:** `:148` and `:160`.
4. **Long files grew.**
   - `crates/skein-oauth-accounts/src/component.rs`: 1,135 to 1,291 lines. `has_work` (`:1063`) and `next_deadline` (`:1117`) are 130 lines of scheduling.
   - `tests/oauth-accounts/src/world.rs`: 1,224 to 1,407.
   - `tests/llm/tests/fake.rs`: 1,210 to 1,419.
   - `crates/skein-oauth-accounts/src/private_tests.rs`: 382 to 777.
5. **Landed tests that break section 10.**
   - In `later.md`:
     - `crates/skein-oauth-accounts/src/openid_tests.rs:18,38,42`, bare ports;
     - `:78-85`, `signed_in_names_the_records_subject` asserts `subject.is_none()`, the opposite of its name;
     - `shared_tests.rs:372,376`.
   - In no record yet:
     - `tests/llm/tests/credentials.rs:94,114`, three outcomes under one name, and a branch on a row's `expected`;
     - `crates/skein-oauth/src/machine_tests/issued.rs`, `b"client-1"` seven times, and `revoke.rs`'s 6-tuple rows;
     - `machine_tests/openid.rs:290`, bare 2047, 2048, 8192 and 8193;
     - `tests/llm/tests/fake_credentials.rs:148-165`, a mixed table.
6. **Production repeats.** `crates/skein-oauth-accounts/src/component/accounts.rs:346` and `:387` repeat one origin test, and `:387` compares `Option == Option`, so two unparsable origins would agree (an inference). `exchange.rs:186-213` branches on `Method::Post` four times.
7. **The aims on x1** (`later.md`, ruling 152).
   - The six slowest tests predate the run: `tests/llm/tests/fuzzy_client.rs:199`, 8.26 s, and `tests/tls/tests/fuzzy_world.rs:118`, 5.75 s, among them.
   - The fuzzy gate took 32 to 52 s of its 60 in the run.
8. **The second part's markers:** `crates/skein-llm/src/types.rs:15,362,388` and `client.rs:1665`, and `FileLower::Remove` (decision 18).

**jig**

1. **A client race fails a gate now and then** (`later.md`, ruling 145).
   - **Where:** `tests/client-browser/tests/browser_tier.rs:37`, `under_a_prefix_the_inbox_s_links_carry_the_prefix`, person step 9.
   - **Where to look first:** the order of the durable `Cancelled` answer and the task watch's `Gone`. The unproved suspects are `crates/jig-client-view/src/ui.rs:798` and `objects.rs:720`.
   - **Cost:** 99 minutes of jig-repin.s1.
   - The known-flake rule that let it land ends with this plan's README.
2. **`const DIALECT: u32 = 1` is in four files** (`later.md`): `crates/jig-protocol-transcript/src/tests.rs:4`, `tests/client/src/joint/fixture.rs:5`, `tests/client/src/joint/stories/job_stories.rs:5` and `tests/ops/src/conformance/scripts.rs:34`. smith exports no such number to name it from.
3. **Accounts that nothing compares.** jig still carries `Endpoint.account` (`crates/jig-protocol-config/src/llm.rs:22`) and `Model.account` (`crates/jig-domain-core/src/run.rs:303`). The agent's `ModelBound` has no account. So a wrong account fails at run time, not at startup (an inference). See temper, item 1.
4. **Ruling 130's mapping has no design.**
   - **Where:** `testing/jig-test-worker/src/domain.rs:387-399`.
   - **The effect:** `crates/jig-domain-accounts/src/domain.rs:207-217` benches an account with no known reset until `now + 0`.
   - **Risk:** key, environment and program accounts make `Refused` and an unknown reset common.

**hearth**

1. **The default suite cannot pass on x1.**
   - **Where:** `tests/tree/tests/memory.rs:321`, `walking_a_layer_at_its_limits_holds_no_more_than_the_worst_case`, 12.6 to 14.1 s.
   - **How much:** 2,150 tests take 22.6 to 25.4 s on x1, against 6.5 to 8.8 s on hetzner.
   - **Since:** `3df74d3`, 23:03. `c4d0231` trimmed one test.
   - Every hearth repin from x1 meets it. Needs hearth's own cause found, never a bigger budget.

**temper**

1. **The account match was dropped without a question** (`later.md`, ruling 150).
   - **Where:** `crates/temper-service/src/startup/checks.rs:447-454`. The text and tests still say "under the same name and account": `startup/problems.rs:85` and `tests/checks.rs:80,86`.
   - **The root** is smith's: `EndpointOptions` has no account, and `config.rs:182` ignores it.
2. **Ruling 130's mapping** is also in `crates/temper-worker-domain/src/agents.rs:127-135`.
3. **Landed tests that break section 10** (`later.md`):
   - `tests/worker/domain/src/protocol.rs:21`, `SMALLEST: [u8; 120]`;
   - `b"acc"` six times;
   - `"dialect": 1` in `docs/examples/owned.json:26` and `owned/agent.json:22`.
4. **Each upstream limit's growth is hand arithmetic** in `docs/examples/owned.json`, at `:192` and `:825`.

**The kit** (skein `scripts/dev`)

1. `plan metrics` reads the local `main` (proposal 10).
2. **The ledger:** `spawn_agent` returns no thread id, so three steps' ledger rows name `/root/wNN`.
3. **The pulse:**
   - `numbers.sh`'s `repin,` pattern lacks `.s1`;
   - it counts other workloads' `heavy` rows;
   - it counts a moved `#[expect]` as new;
   - it counted a landing twice.
4. **Spawned implementer threads** inherit the coordinator's `enabled = false` for the graph. Threads spawned after the 22:27 `plan resume` had the graph: the probe live-text's decision 3 names (an inference from the timing).
5. **Smaller defects:**
   - `plan contexts` prints ended threads, up to 52 lines.
   - `plan handover` takes no peak, so the coordinator patched `log.md` by hand.
   - A watch `started:` event prints up to 150 lines of `git diff --stat`.
   - `status.md` keeps answered alerts.
6. **Not code:**
   - Local `accounts/*` branches remain: skein 39, smith 16, jig 2, hearth 1, temper 2.
   - jig's and temper's `STATUS.md` hold none of `later.md`'s rows for them (rulings 145, 148 and 150 left those to the designer's next landing).

## 10. Decisions

The director, 10-20 07:00Z, under owners.md: only what still matters to owners is taken.

| Proposal | Decision | Where |
|---|---|---|
| 1 | rejected | development.md is being retired (decision 177): no new run starts under it; forge-kit and real-loop finish as they are |
| 2 | rejected | as 1 |
| 3 | adopted, for owners | the disk floor's refusal names what fills the disk (the largest directories in tmp/ and the targets), so an owner frees it; skein scripts/dev |
| 4 | rejected | as 1 |
| 5 | adopted, already in force | owners.md 4.5 (every machine's gate; a slow test made cheaper, the budget never raised); smith's suite cause (section 9, smith 1) goes to o/smith-sources before it adds more tests |
| 6 | rejected | as 1: owners keep no Done-when aims |
| 7 | deferred | a later kit item; owners' reviewers check testing-strategy.md section 10 meanwhile |
| 8 | rejected | as 1 |
| 9 | rejected | as 1 |
| 10 | rejected | as 1 |

Section 9's debts go to the owners working there: smith's (1-6) and skein's 1 to o/smith-sources; jig's 3 and 4 and temper's 1, 2 and 4 to o/accounts-2; jig's 1 is fixed (jig ac8053a8); hearth's 1 waits for hearth's owner; the kit's to the next kit owner.
