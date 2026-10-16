# Retrospective: parity

2026-10-15. The run of smith's `parity` plan on debian-16gb-hel1-1. It
was claimed at 2026-10-14T10:22:42Z. `done.md` was recorded at
2026-10-15T20:11:43Z, and the supervisor's "Done when" commands ran at
20:13 to 20:17. It was the first run under the reliability
retrospective's decisions, and under test-budgets' and jig-protocol's
adopted parts. Written by a fresh session that took no part in the run
(development.md, section 15).

Ten read-only sub-agents, one per source, read:
- the plan, `decisions.md` (rulings 101 to 146), `later.md` and the
  plan's git history;
- the ledger, `log.md`, the 156 reports, `context.md` and the run's
  small files;
- `supervision.md` and `done.md`;
- smith's, skein's and temper's git history, with the trailers;
- `logs/heavy.tsv`, the queue's task logs and pueued's journal;
- the coordinator's Codex rollout;
- the 44 implementer rollouts;
- the supervisor's five Claude Code transcripts and their 36
  sub-agents;
- the designer's five sessions, the two sub-agents the
  `Planner-Session:` trailers name, and the session that started them;
- the earlier runs' retrospectives and metrics, jig's included.

This session read:
- the plan's README and `later.md`;
- `done.md`, `status.md`, `context.md` and `claude.tsv`;
- `numbers.sh`;
- development.md at skein `3d7d304` and at `main`;
- the reliability and skein-ui retrospectives;
- the three `parity` summaries.

Times are UTC; a time without a date is on the day last named. Costs
are at Opus 5.5's list prices. This document proposes; the user
decides, and the designer applies what is adopted.

**Which process.** This machine's skein checkout, which holds
development.md, the cards and the kit, stayed at `3d7d304` from
10-14 10:29 to 10-15 20:12. Its process documents and kit are those of
the plan's base, `63fff2d`. Five changes to them landed on other
machines during the run: `e30fe58`, `f80521b`, `e601ee2`, `d296e3e` and
`e3b148e`. None reached the run, and section 7 judges only what was in
force at `3d7d304` (section 5, item 10).

## 1. In one page

- **Done in 33.8 h; every step landed, and the aim was missed on one
  criterion.**
  - 58 steps landed: 45 planned, 2 split off by an implementer, and 11
    added. None was dropped.
  - "Done when" items 1, 3, 4, 5 and 6 hold. Item 2, the verdict, exits
    1 on RT alone, which ruling 146 allows. In item 7, two checks
    written before later steps fail on what those steps rightly added.
  - smith is on par with Codex on O, FT, FC and RC. RT, repository time
    on `shell-cwd`, is 1.326 [1.031, 1.756] against a margin of 1.20.
- **RT failed at all three `parity` runs, and no step aimed at it after
  the baseline showed it.**
  - The baseline held FT at 0.995. The steps after it, the task role
    and the `job` tool offered, took FT to 1.193, and the first closing
    run failed.
  - 12.s1, the designer's answer, brought FT back to 1.090. RT went from
    1.156 to 1.326.
- **smith gained what the plan set out to build:**
  - jobs, a compaction asked with `None`, the task role, and the
    `gpt-6.1-sol` tier;
  - a harness with graded tasks, one price basis, a summary row per
    attempt, calibrations in task files, and the `parity` command;
  - in all, +5.2K lines of production Rust and +7.3K of tests, net.
  - It spent 522 live attempts, $41.78 notional, of the 680 and $100
    allowed. That was within the original $50, but past the original
    340 attempts.
- **The line stood still for 5 hours on the first day.**
  - The disk filled at 12:33, and pueued died with it. The supervisor
    had just asked the user a question through a tool that waits for
    the answer, and saw nothing until 17:31.
  - That was 68% of `blocked_h`, and 15% of the wall time.
- **Live suites held half the run.**
  - The gate lane ran live work for 17.6 h.
  - The verdict's pipeline first ran live 12 h into the run. The
    baseline then took four runs and three added steps.
- **22.96M tokens a step:** below reliability's 27.6M, and three times
  skein-ui's 7.78M. The causes:
  - 37% of the implementers' input waited on a running command, in
    polls that Codex's cell cut at 30 seconds;
  - 69% of the coordinator's input came past 200K, and it never handed
    over;
  - sessions took new increments past 150K until a steer at 06:33.
- **The kit could not wake its own coordinator.**
  - The thread unloaded at each idle, and `plan ring` and `plan resume`
    misread that. The supervisors loaded it by hand 10 times, and
    wrote their own wakers.
  - About an hour went to late wakes.
- **Sixteen proposals** (section 8). The first four target the 5-hour
  stop, the live pipeline and the verdict. Proposals 5, 6, 7, 8, 11
  and 15 change the kit, which waits for this retrospective
  (`kit-after-parity`).

## 2. Outcome

The supervisor ran the README's "Done when" once, at 20:13 to 20:17, on
smith `main` at `9d1119b3` and skein at `ecccbac`. Its commands and
outputs are in `done.md`.

| # | Item | Result |
|---|---|---|
| 1 | Every step landed or dropped | Holds: 58 of 58 landed. The command's `grep -c` exits 1 because it counts 0 open steps. |
| 2 | smith is on par with Codex at the plan's final `main` | **Exits 1, short on RT alone,** which the item, as amended by ruling 146, allows: the plan ends with it failed, naming RT. The summary is the last closing run, `parity-87a8d1e8-20261015T170125Z`. No code commit of the plan landed after its smith arm. |
| 3 | The smoke passes on `main` | Holds: `suite smoke` passed, 1 attempt, 2.2 s. |
| 4 | The calibrations are in their task files | Holds: prints 4. They are `history`, `history/small-window`, `first-request` and `tool-output/background`. `tool-output/timeout` stays uncalibrated (ruling 145, `later.md`). |
| 5 | Both gates within their budgets | Holds. smith: 2,261 tests in 4.926 s, and 42 in 6.094 s, against 15 s and 60 s. skein's last gated landing is skein-ui's, 2,443 in 6.531 s. |
| 6 | Each format moved once | Holds: events 2, transcript 4, result 3, summary 3. |
| 7 | Each session's "Done when" | 11 sessions hold, and 12's is items 2 and 4. Two checks fail on later work, not on a regression: 06's "one role summary" finds two, the second 12.s1's (ruling 146); 09's "no `struct Job`" finds the held-job table that session 10 added (`jobs.rs:26`). Session 09's calls in flight became `Flight`. |

**The verdict across the plan's three `parity` runs** (statistic,
interval, and whether it holds). The margin is 1.20 for FT, FC and RT,
and 1.50 for RC.

| Criterion | Baseline, 12.1, `744e0820` | First closing, `b7b78fb1` | Last closing, `87a8d1e8` |
|---|---|---|---|
| O | 29 [0.88, 1.00], holds | 29 [0.82, 1.00], holds | 29 [0.88, 1.00], holds |
| FT | 0.995 [0.916, 1.085], holds | **1.193 [1.128, 1.260], fails** | 1.090 [1.021, 1.166], holds |
| FC | 0.486 [0.446, 0.530], holds | 0.598 [0.545, 0.654], holds | 0.532 [0.485, 0.584], holds |
| RT | **1.293 [0.909, 1.539], fails** | **1.156 [1.003, 1.581], fails** | **1.326 [1.031, 1.756], fails** |
| RC | 0.642 [0.321, 0.766], holds | 0.606 [0.504, 0.819], holds | 0.778 [0.549, 0.971], holds |

- **RT's upper bound never came below 1.54.** 11.3's own measure of it
  was 1.176 [0.729, 1.599].
- **`later.md`'s last row** carries RT to the next smith plan: a
  diagnosis, then a change that names its cause, then a `parity` run.
  In the first closing run, smith made 75.4 responses an attempt on
  `shell-cwd`, against Codex's 55.0.

## 3. Numbers

`plan metrics` stopped with exit 5 on this run's record (section 5,
item 15). The coordinator's rollout, and six others, hold one torn line
each, written at 12:33:18 to 21 as the disk filled. `jq` stops at the
first, so `running_seconds` read only up to 12:33:04, and `set -e`
ended the script. This session ran the same script with `CODEX_HOME`
pointing at a copy of the run's 45 rollouts, without those seven lines
and with their times kept. That is `metrics.tsv`.

The nearest runs are:
- reliability, the last retrospective's run: smith, this machine, 108
  steps (its corrected column);
- skein-ui, the most recent run: jig and skein, 75 steps.

Per-step numbers compare better than totals.

| Metric | Printed | Corrected | reliability | skein-ui | Notes |
|---|---|---|---|---|---|
| `wall_h`, `running_h` | 33.9, 13.3 | 33.8 to `done.md`'s record | 36.0, 35.8 | 8.1, 7.6 | **Not comparable** (below). Any Codex thread was at work 13.5 h. The gate lane ran live work for 17.6 h. The line stood still 5.0 h. |
| `steps_planned`, `added`, `dropped` | 47, 11, 0 | 45, 13, 0 | 71, 37, 0 | 70, 5, 0 | 10.5 and 10.6 were split off 10.1 by its implementer (rule 1). Seven added steps were harness, four product. |
| `step_h` | 0.23 | 0.58 wall hours a step; 0.50 without the stop | 0.33 | 0.10 | |
| `escalations` | 36: cross-session scope 14, plan defect 13, design gap 4, infrastructure 4, process 1 | 36: plan defect 17, cross-session scope 11, design gap 3, infrastructure 4, process 1 | 185 | 9 | 0.62 a step, against 1.71 and 0.12. Four rulings name their block a plan defect: 104, 117, 130 and 141. |
| `escalation_wait_min` | 2.5 | | 2.1 | 4.3 | The four design questions took 12.0 to 19.0 minutes from block to ruling. Interim rulings 119 and 123 count as their answers. |
| `blocked_h` | 7.3 | | 7.7 | 1.3 | 5.10 h was one block, 07.1's dead queue (section 5, item 1). The other 35 blocks made 2.2 h. |
| `answers_reversed` | 1 | 1, a steer's | 11 | 0 | Ruling 131 reverses the 22:29 steer's first point. Ruling 115 amends 114's premise, and the 04:31 steer replaces the 23:33 steer's waker; neither has a `Reverses:` line. |
| `steers` | 11 sent, 0 refused | 12 points in 11 rings | 12 | 6 | The 01:32 and 02:30 points went in one ring, at 03:31. |
| `restarts` | none | no restart; 10 loads by hand | 1 | none | The thread unloaded at each idle (section 5, item 4). |
| `tokens_coordinator` | 233.1M in (98% cached), 408K out | | 540.2M | 114.1M | 4.02M a step, against 5.0M and 1.52M. |
| `tokens_implementers` | 779.1M in (98%), 1.69M out | | 1,600.9M | 219.6M | 13.4M a step, against 14.8M and 2.93M. |
| `tokens_claude` | 565.6M in, 2.82M out: 121.2M before the claim, 315.5M in the run, 128.9M after | | 833.1M in the run | 247.6M in the run | 5.44M a step in the run, against 7.7M and 3.30M. **Not comparable** before the claim (below). "After" is this retrospective with its readers. |
| `cost_claude` | $231: planner $45, supervisor $114, designer $19, retrospective $54 | design and planning about $141 | — | $149 | The run's part, $133, is $2.29 a step, against skein-ui's $1.46. The supervisor's hourly checks were $55 of its $114. The retrospective's figure is this session's when `metrics.tsv` was written, with its ten readers. |
| `tokens_per_step` | 22.96M | | 27.6M | 7.78M | |
| `coordinator_requests` | 1,169, 48 after a wait ran out | | 2,709 | 589 | 20.2 a step, against 25.1 and 7.9. The waits that saw only the wake-up check took 15.7M, 6.7% of its input. |
| `compactions` | 2 | | 7 | 1 | At 321K (18:55) and 392K (03:30). It grew again to 404K by the end. |
| `gates_per_step` | 1.8 | 1.84 | 1.75 | 1.3 | 107 `land` gates: 102 passed, 3 failed a test, and 2 died on the full disk. About 40 passing gates lost the push to a moved `main`. |
| `gate_lane_other` | 103 | | — | 39 | 100 live runs (14 suites, 67 probes of which 66 smokes, 19 calibrations), 2 `scripts/check.sh` and 1 measurement: 17.6 h. |
| `queue_wait_ratio` | 0.74 | | 0.93 | 0.65 | The build lane waited 18.8 h for 7.7 h of running, 10.3 h of it behind live runs. Its five longest waits made 6.9 h. |
| `suites` | smith: 1,995 / 4.80 s → 2,261 / 4.93 s; 41 / 5.24 s → 42 / 6.09 s. skein: 2,106 / 2.78 s, one landing | | | | The highest single gate was 7.2 s focused and 6.8 s fuzzy, against budgets of 15 s and 60 s. |
| `proofs` | unit 29, world 11, process 2, live 12; 4 below plan | 1 below plan: 10.1 | 3 below | 1 below | 08.1's "codecs", 01.3's "unit/process" and 06.2's first commit are misread. 8 Proofs are not counted: "codecs", "machine", "unit/process", "end", "offline". |
| `report_lines` | 11 median, 16 p90 | | 10, 12 | 10, 11 | 156 reports, the longest 28 lines. |
| `lines` | code +74,784 −93,312; tests +9,804 −1,518 | production Rust +7,032 −1,798; Rust tests +8,689 −1,355; JSON +67,017 −90,712 | | | **Not comparable:** the metric counts the benchmarks' JSON as code. 03.1 alone removed 90,763 lines of version-2 summaries and baselines. |
| `pin_lag` | smith: skein 76; temper: skein 348, smith 917 | temper on origin/main: skein 99, smith 199 | | | smith's 76 are other machines' work since `e30fe58`, 07.1's repin. temper's printed numbers come from its checkout here, 268 commits behind origin. |
| `contexts` | 201K median, 293K p90; 26 handovers | 24 handovers and 2 session ends | 245K, 348K | 129K, 237K | Before the 06:33 steer, handovers came at 200K to 329K. After it, at 156K to 186K. |

**Not comparable, and why:**
- **`running_h` and `step_h`.** `running_seconds` sums the coordinator's
  events, less every gap over 10 minutes. Since `479f836` an idle
  coordinator ends its turn, so its idle hours drop out. In reliability
  it never idled.
  - Here 13 gaps make 20.5 h: 13.6 h of single live suites with nothing
    else to run, 4.9 h of the stop, and about 1.0 h of late wakes.
  - Wall hours a step, 0.58, compare with reliability's 0.33.
- **`tokens_claude` and `cost_claude` before the claim.** The
  `Planner-Session:` trailers name two sub-agents of session
  `663cdd27`, the reliability retrospective's session.
  - `a666e5e7` wrote the design ($21.73), and `ac56b334` wrote the plan
    ($22.82). The metric counts both as planning.
  - It does not count their own 21 sub-agents: 12 of the design's
    ($78.99) and 9 of the planner's ($17.81).
  - Design and planning cost about $141. Add $10.73 for the parent
    session that started them.
- **`gates_per_step`** counts only `land`'s gates. That is reliability's
  corrected basis, but not its printed 3.5.
- **`escalations`** counts the block's kind as the coordinator wrote it.
  Four rulings name their block another kind.
- **`lines`** counts JSON, JSONL and TOML as code.
- **`pin_lag`** reads each checkout's `Cargo.lock`, not origin's.

**Where the 33.8 hours went:**

| Span | Hours | Steps landed | What held it |
|---|---|---|---|
| 10-14 10:22 to 12:33 | 2.2 | 16 | Eight sessions at once. Disk stops at 10:45 and 12:32. |
| 12:33 to 17:43 | 5.2 | 0 | The disk, the dead queue, and the blocking question (section 5, item 1). |
| 17:43 to 22:20 | 4.6 | 24 | Five sessions and more. 07.3's calibration, and two design questions. |
| 22:20 to 10-15 03:48 | 5.5 | 6 | 12.1's baseline, in four runs (section 5, item 2). |
| 03:48 to 08:53 | 5.1 | 8 | 12.2, 06, 09.3, 11.2, and two panics found live (11.s1, 11.s2). |
| 08:53 to 20:07 | 11.2 | 4 | 11.3's repository check (2.4 h), the first closing run (3.6 h), the design question, 12.s1 (0.8 h), and the last closing run (3.0 h). |

**What explains the largest differences:**
- **Wall hours a step, 0.58, against 0.33 and 0.10:**
  - **Live suites.** The gate lane ran live work for 17.6 h, 52% of the
    wall time. From 22:00 on the 14th, it ran them for 15.8 of the 22.1
    hours left, and 19 of the 58 landings came then.
  - **The README's estimate:** about 13 h of live runs on the chain, and
    3.7 h of increments.
  - **Live work that measured nothing whole:** about 1.6 h. These were
    three spent `parity` suites, three role suites refused at once, and
    a repository check that stopped on a panic.
  - **The stop:** 5.2 h with no landing.
- **Tokens a step, 22.96M, against 27.6M and 7.78M:**
  - **The implementers, 13.4M a step:**
    - Waiting took 292M, 37% of their input: 1,932 requests that saw
      only "still running" (section 5, item 5).
    - Contexts before the 06:33 steer did the rest.
    - Input a landed step fell from 15.3M before the 23:33 steer to
      8.5M after it.
  - **The coordinator, 4.02M a step:** 69% of its input came past 200K
    (section 5, item 6).
  - **Claude Code in the run, 5.44M a step:** the supervisor's five
    sessions, $114, and four design answers, $19.
- **Escalations a step, 0.62, against 1.71:**
  - Plan defects fell from 0.91 to 0.29 a step. Process blocks fell
    from 14 to 1. No increment blocked on its size, against 20 in
    reliability.
  - What remains clusters on four causes (section 6): memory at its
    bound (6 rulings), whose failure a calibration counts (8), session
    10's job sketch (8), and the verdict's pipeline run live (5).

## 4. What worked

1. **Rulings were fast, checked, and almost never reversed.**
   - A block took a median of 1.5 minutes to its ruling, over 32
     ordinary blocks; the longest was 6.9.
   - 37 unblocks carry 38 `## Checked` sections, as reliability's
     proposal 12 asked.
   - The one `Reverses:` line reverses a steer of the supervisor's own,
     not an answer: 0.017 a step, against 0.10.
   - All 46 rulings went through `plan ruling`.
2. **The fresh designer answered each of four questions in 11 to 15
   minutes, for $4.26 to $5.51 each.**
   - Each answer landed a design commit: smith `62376cb8`, `f242ecaa`,
     `452c9967` and `7c345bd0`.
   - Each drafted its supplement for the supervisor to name.
   - Interim rulings 119 and 123 kept 07.3 calibrating while the
     designer worked.
   - The last answer, benchmarks.md 16.5, bounded the closing runs.
     Its 12.s1 brought FT from 1.193 to 1.090.
3. **The supervisor found faults the plan had not seen.**
   - **Ruling 115 (19:27).** The supervisor's successor ran the smoke
     on `main` itself, 4 minutes after taking over. It was refused at
     startup there too: no smoke had run on `main` as landed. 08.s1
     landed 5 minutes later.
   - **The checks:**
     - the pricing error at 02:24 (ruling 135);
     - `translate.rs:513`'s panic at 07:25 (11.s1);
     - the falling disk at 18:42;
     - a stale `idle.md` that silenced the watch, at 20:43.
   - **Ready checks** corrected sessions 06, 07, 10 and 12 before they
     started (rulings 106, 113, 129, 137).
   - **Ruling 141** saw that 06.2 quoted only half of benchmarks.md 9's
     stop rule. That saved a question and a 30-attempt run.
4. **The implementers stopped cleanly and lost no work.**
   - At the disk stop, all seven running sessions reached a clean
     commit, a checkpoint and a report within 0.5 to 3.5 minutes.
   - All six that had work resumed within two minutes of ruling 109,
     and landed by 19:12.
   - Successors continued from their predecessors' commits, as 10.6
     did from w23's work in progress.
   - Reports kept the card's shape: median 10 lines, longest 28.
   - The 31 questions gave numbers and options, and the sessions kept
     working while they were open.
5. **The steers were obeyed, and moved what they aimed at.**
   - **23:33, yield on a gate-lane run:** requests waiting on live runs
     fell from 266 (38.4M) to 4 (0.6M).
   - **06:33, a fresh thread past 150K:** handovers after it came at
     156K to 186K, against 200K to 329K before it.
   - **20:44:** 10.1's world proof landed with 10.6.
   - **04:31:** a task's log is found by its queue time, which closed a
     misread before it happened.
   - All 11 were acknowledged within about 1.5 minutes.
6. **Rule 1's splits replaced size blocks.** 10.5 and 10.6 were split
   off by the implementer, and no increment blocked on its size,
   against 20 in reliability.
7. **The test suites stayed small and fast.** The focused suite gained
   266 tests and 0.13 s, and the fuzzy suite one test. No budget was
   raised, and every format moved once.
8. **The live budget held, and was counted.**
   - Every live landing carries a `Live:` trailer, and they add up to
     `done.md`'s total, within one attempt.
   - 522 attempts and $41.78, inside the doubled caps and the original
     dollar cap.
   - The cap question went to "For the user" with a push, and waited 8
     hours without holding anything.
9. **The closure was complete.**
   - `done.md` maps each item to its steps.
   - The "Done when" commands ran once on `main`, with their output.
   - Each decision in force is marked with its design commit or "not in
     the design yet".
   - `later.md` names RT's follow-up, and both STATUS files are marked.
   - The downstream worktrees' `Cargo.lock`s were kept in named stashes
     before removal.
10. **The supervisor's handovers were clean.**
    - Four, at 283K to 297K. The watch's 300K alarm never had to fire.
    - Each takeover cost $0.40 to $1.55, and each successor was at work
      within 4 minutes.
    - Each handover entry was at most 30 lines.

## 5. What did not

1. **The disk filled, the queue died, and the line stood still for 5
   hours.**
   - **What:**
     - The disk stopped the line three times:
       - **10:45:** 11.7 GB free. kache's store was at 135 GiB against a
         74.5 GiB cap; ruling 103 ran `kache gc` by hand.
       - **12:32:** 214 MB free. Eight sessions' `target/` were at 17 to
         21 GB each, and the disk had gone from 29 GB at 12:26 to full.
       - **18:40:** the store again at 133.6 GiB, the disk falling 3.5
         to 7 GB a minute.
     - At 12:33:21 pueued could not save its state, and died. systemd
       restarted it five times within a second, then gave up.
     - By 12:38:50, removing five `target/` directories had left 112 GB
       free. The daemon stayed down.
     - At 12:37:08, its context at 224K, the supervisor asked the user, with
       AskUserQuestion, whether it could delete the old 2026-10-08
       benchmark's `target/` directories. Ruling 108 had already
       resumed the line without them.
     - At 12:38:47 the coordinator blocked on the dead queue. The watch
       printed it at 12:39:00, but the supervisor's session was waiting
       inside the tool, and took no event.
     - The five hourly checks due from 12:35 to 16:35 never fired: their
       cron lives in the same session.
     - The user answered at 17:31:26, after 4 h 54 min. The repair then
       took six minutes: 76 `target/` directories deleted, pueued
       started, ruling 109, and the coordinator's thread loaded by hand.
       The first landing came at 17:43.
   - **Cause:**
     - **The question waited inside a tool.** development.md 10.4
       already said "The run blocks on it only if no work can continue
       without it", and nothing waited on it. The card does not name
       the tools that wait.
     - **pueued under systemd's default start limit** gives up within a
       second. No check sees it down unless someone runs `heavy`.
     - **The disk rule watched starts, not growth.** 11.4 held starts
       below 40 GB, checked at each start and wake-up. Running sessions
       filled 29 GB in about five minutes, between two checks.
     - **kache's own eviction fails** on this machine. Its store ran
       past its cap twice.
   - **First visible:** 10:45, the first disk stop; 12:39, the dead
     queue's block. **Acted on:** ruling 108 at 12:36 (each session's
     `target/` removed after its landing); 17:31 for the queue; the
     18:44 steer (`kache gc` below 80 GB) for the store. No disk event
     followed.
   - **Cost:** 5.0 h of the line, 68% of `blocked_h`.
2. **Live suites held half the run, and the verdict's pipeline first
   ran live 12 hours in.**
   - **What:**
     - 12.1's baseline was the first live run of the pipeline: suite,
       summary, verify, verdict. 04.4, the `parity` command, had been
       proven on a recorded summary.
     - The baseline took four runs, 117 attempts against 58, from 22:20
       to 03:46:
       - 4 attempts, stopped on a provider answer with no usage (03.s1,
         ruling 131);
       - 6, a rerun stopped by the supervisor's hold;
       - 49, refused by a 15-second setup deadline that `shell-cwd`'s
         cold build could not meet (04.s4, ruling 133);
       - 58, all passed, but `verify` refused the summary, a float's
         round trip through JSON (03.s2, ruling 136).
     - Five of the 11 added steps came at a pipeline's first live run:
       03.s1, 04.s4 and 03.s2 at 12.1; 11.s1 and 11.s2 at 11.3.
     - About 1.6 h of suite time measured nothing whole.
     - While a suite runs alone, the build lane is paused. 10.4's one
       heap test, 3 to 7 seconds, waited 2.96 h behind 12.1's suite.
   - **Cause:**
     - development.md 5.5 at `3d7d304` asked for a proof at the
       increment's layer, not for a live run of a pipeline before the
       step that depends on it.
     - `e30fe58`, landed 9 minutes after the claim, now asks for a
       smoke check above the tests, and for parts connected from the
       start. It names no live suite.
     - Every Proof of sessions 03 and 04 was unit, and a recorded
       summary stood in for the live one.
   - **First visible:** 22:20, the first suite's null usage. **Acted
     on:** each fault by a ruling and a supplement, in turn, over 5.5
     hours.
3. **The plan aimed at a verdict with no path for missing it, and no
   step aimed at RT after the baseline missed it.**
   - **What:**
     - decisions.md 1.1 gives a truly equal smith a 0.69 chance of
       passing the whole verdict. 12.4 held one closing run, and left
       "a verdict of 1 or 2" as a question for the designer.
     - The baseline (03:48) failed RT, 1.293 [0.909, 1.539]. Ruling 136
       called it "the baseline's finding, not a block". The plan's only
       lever on RT was jobs (decisions.md 5.1), which had landed.
     - The steps after the baseline moved FT the wrong way:
       - **06.2 at 05:56:** the task role took responses an attempt from
         7.13 to 8.33. Ruling 141 landed it on "no difference
         detected", benchmarks.md 9's rule.
       - **The first closing run (15:29):** FT failed, 1.193. The
         designer's answer named the role and jobs, which had raised
         fixture responses from 185 to 231.
     - The first closing run stopped the line for 19 minutes on the
       question (15:32 to 15:51). Ruling 146 then gave one change and
       one last run.
     - That run held FT and failed RT at 1.326.
   - **Cause:**
     - **The plan settled the aim's statistics, but not its failure.**
       development.md 5.1 lists what to settle first, and a target that
       can fail by chance is not on it.
     - **Nothing sends a baseline's miss to the designer.**
     - **A step's comparison has its own stop rule.** Nothing asks
       whether the step moves a measure the verdict reads.
   - **First visible:** 03:48 for RT; 05:56 for FT. **Acted on:** 15:32,
     after the first closing run.
4. **The kit could not wake its own coordinator.**
   - **What:**
     - The Codex daemon unloaded the coordinator's thread at each idle.
       `plan ring` then says to run `plan resume` (`plan`, lines
       852–854). `plan resume` reads the daemon's "thread not found" as a
       thread with no turn, and its `turn/start` fails (lines 1005–1006).
     - The supervisors loaded it by hand with `codex_call
       thread/resume` 10 times: 10-14 17:37, then 10-15 00:06, 04:05,
       04:51, 05:06, 05:58, 08:43, 11:37, 11:47 and 15:51. That is
       client-composition's proposal 4, deferred.
     - Nothing woke the coordinator when the queue tasks it idled on
       ended:
       - The 23:33 steer told it to start a waker, against 7.4's "No
         background waits". The `nohup` wakers it started at 00:17,
         00:29 and 03:51 died with its turn.
       - It idled 14 minutes on tasks that had ended by 04:17. The 04:10
         entry still said "the gate waker remains armed".
       - It idled 45 minutes from 06:46 with no waker at all.
     - Ruling 138 (README rule 17) then made the waker the supervisor's
       own, by hand: `pueue wait`, `thread/resume`, `plan ring`. From
       11:37 the supervisor ran an idle watcher of its own.
   - **Cause:** the kit has no waker, and does not load an unloaded
     thread. Both wait on `kit-after-parity`.
   - **First visible:** 17:36, the first failed ring. **Acted on:** by
     hand each time; by rule 17 at 04:31.
   - **Cost:** about 1.0 h of late wakes, and the supervisors' wakers.
5. **Waiting and long contexts took the implementers' tokens.**
   - **What:**
     - **Waiting:** 1,932 requests saw only "still running": 292M, 37%
       of the implementers' input.
       - By what they waited on: the build lane 141.5M, `land` 95.3M,
         live runs 39.1M.
       - The card said "the tool's longest, 60 seconds on Codex". Codex's
         `exec` cell yields at about 30 seconds whatever the inner wait,
         and 1,006 requests (150M) came straight after the cell's own
         yield.
       - Only w7 and w15 set the cell's header to the full wait.
     - **Contexts:**
       - Before the 06:33 steer, sessions handed over only past 200K,
         and 9 of the 26 handovers came above 250K.
       - Session 10's threads each passed 266K within one increment.
       - 06.2 started at 164K and cost 24.2M, against 8.6M for 06.1 in
         the same thread.
       - w5 took 11.2 at 170K after 19 hours idle, with 12K cached. It
         spent 18.4M, and compacted at 364K.
     - **Reading:** a thread's first edit came at a median of 90K.
   - **Cause:**
     - At `3d7d304` the limit was 200K between increments and on a
       resume, and nothing sent an idle session to a fresh thread.
       `main`'s 7.3 now sends a resumed session past 150K, or idle over
       an hour, to a fresh thread (skein-ui's 3). The limit between
       increments is still 200K.
     - `main`'s implementer.md 8.3 now waits on the cell with
       `yield_time_ms: 300000` on Codex 0.162, which this machine runs.
   - **First visible:** 10:52, the first resumes past 150K; 23:24, the
     check that measured w31's and w32's waits. **Acted on:** 23:33 and
     06:33, by steers.
6. **The coordinator carried a long context, and read what it did not
   need.**
   - **What:**
     - 223 requests past 300K took 76.5M. 344 requests at 200K to 300K
       took 86.0M.
     - Codex compacted it at 321K and 392K. It grew again to 404K.
     - It read 120 of the 156 report files in full, 37 of them more than
       once: about 29.6M carried.
     - Its spawn prompts grew from about 1.5K to 12 to 13.5K characters
       by 20:36. They are encrypted, so this is an inference: the
       Pass-on texts then came to about 1.5K, and the rest was probably
       ruling text, which 7.2 rules out.
   - **Cause:**
     - protocol's proposal 4, compaction at 200K, is deferred to the
       kit. test-budgets' 9, the coordinator's handover, was rejected.
     - 7.3, step 1, already reads a report file "only for what the four
       lines do not say".
   - **First visible:** at each check, in the pulse's contexts line.
     **Acted on:** not.
7. **One design area took eight rulings and is still open.**
   - **What:** whose failure a calibration counts, what decides, and
     what is "not exercised":
     - ruling 112 at 18:53, ruled a plan defect, and 04.s1;
     - 118 at 20:05;
     - 119 and 121 at 20:34, the first design question, and 04.s2;
     - 123 and 124 at 21:11, a gap in 121's own answer, and 04.s3;
     - 126 at 21:51;
     - 145 the next day: `timeout` left uncalibrated, its
       classification defect in `later.md`.

     07.3 spent 59 attempts against the 28 it planned.
   - **Cause:** benchmarks.md 3.1 and 3.2 did not say whose failure a
     check's miss is. Ruling 112 read it as the plan's.
   - **First visible:** 18:51, 04.6's block. **Acted on:** the designer
     was asked at 20:18, 87 minutes later.
8. **Memory sat at its bound, and the plan said it needed no change.**
   - **What:**
     - smith's worst case at startup (`service.worst_case`) passed the
       shipped limit (`declared.memory`) eight times: in 02.1, 08.3,
       07.2, 04.6, `main` itself, 10.2, 10.4 and a dependents' run.
     - Six rulings followed (107, 110, 114, 115, 125, 134). README rule 8
       was amended three times.
     - Each fix removed an over-counted term, proven by a heap test. The
       margin left is 28,783 bytes of 2 GiB (10.4).
   - **Cause:** decisions.md 7.4: "Memory needs no change". No command
     checked it at planning, and no test held the margin.
   - **First visible:** 12:00, 02.1's second smoke. **Acted on:** each
     time, by a ruling; 08.s1 at 19:32.
9. **Test shape was steered late, and duplication never.**
   - **What:**
     - "One behaviour per test" was seen at 11:44, steered at 17:44 and
       made README rule 16 at 18:42. That is one sighting late each
       time, by 14.4.
     - Bare numbers were seen at 11:44, 17:44, 20:43, 22:24 and 23:24,
       and steered at 00:31. The 06:26 entry calls that point "holds",
       while its own sample has a bare 4096. 08:26 found 55 and 91.
     - Helpers defined again were seen in seven checks and never steered:
       `run` in 19 files, `finish` in 12, `read` in 14, a 25-line setup
       in 5.
     - The test the 17:44 steer named, `a_cancelled_held_job_answers_
       cancelled` (`tests/machine/tests/jobs.rs:132`), still tests two
       outcomes.
   - **Cause:** 14.4 already says a steer at the first sighting and a
     rule at the second. The checks' sub-agents reported findings, and
     the supervisor typed the steers itself, later.
   - **First visible:** 11:44. **Acted on:** 17:44, 18:44 and 00:31.
10. **The run kept the process of its claim, and found three of the
    newer rules again by itself.**
    - **What:**
      - skein's checkout stayed at `3d7d304` for 33.7 hours. Every
        implementer read the card of `479f836`, with its 60-second
        wait. w44's read, at 10-15 15:58, is the same.
      - Five adoptions landed elsewhere during the run, and none reached
        it. The run reached three of their rules hours later, by its
        own steers and rulings:
        - the 06:33 steer, a fresh thread past 150K: skein-ui's 3 and
          temper protocol's 7;
        - ruling 108 and the disk cap: close to protocol's 10, the disk
          rule;
        - the 23:33 steer on waits: close to protocol's 9.
      - `done.md` cites `d296e3e` and `e3b148e`. Those came from a pull
        at 20:12, after the run.
    - **Cause:** development.md says nothing of a change that lands
      during another machine's run. The kit's freeze was deliberate
      (smith `abe6ec2e`); the documents' was not decided.
    - **First visible:** in this retrospective. **Acted on:** not.
11. **`land` gated about 40 passing trees again, after `main` moved.**
    - **What:**
      - 107 gates for 62 commits.
      - In the first window, 37 gates for 15 landings: 01.2 was gated
        four times, 05.2 lost four passing gates.
      - 43 extra gates held the machine alone for 59 minutes.
    - **Cause:** test-budgets' 4, "`land` holds the repository's turn
      from rebase to push", is on `kit-after-parity`.
    - **First visible:** the 11:44 check, 20 gates for 9 landings.
      **Acted on:** not.
12. **The hourly checks left gaps, and cost most where nothing
    happened.**
    - **What:**
      - 27 checks, $55 with their sub-agents: 48% of the supervisor.
      - 16 led to no steer or action ($21.4). The eight from 10-15 12:47
        to 19:47 reported a running suite's progress.
      - Gaps:
        - 11:35 to 17:38, the blocked session (item 1);
        - 18:35 to 20:36: the 19:23 handover deleted the cron, and the
          successor's first run was due at 20:36;
        - 10:25 to 11:48.

        skein-ui had gaps of 87 and 75 minutes for the same reason.
    - **Cause:** the check is a cron in the supervisor's session. It
      runs in full whatever the run is doing.
    - **First visible:** 20:36, the first check after the two-hour gap.
      **Acted on:** not.
13. **The plan was designed and written by sub-agents of another
    session.**
    - **What:**
      - The user asked session `663cdd27`, the reliability
        retrospective's, to "launch a designer … then … a planner".
      - The planner ran as its sub-agent, from 09:27 to 10:22:
        - It passed 300K 12 minutes in, and peaked at 572K. 136 of its
          193 requests ran past 300K: $12.54 of its $22.82.
        - It waited with a `sleep` loop and two polling loops.
        - It changed the design (`72311b8e`, benchmarks.md 3.1), which
          2.2 forbids. The parent's brief allowed it.
      - The design's sub-agent rewrote its cache three times while it
        polled: $6.09.
      - The `Planner-Session:` trailers name the designer's sub-agent
        and the planner's, and the metric misses their 21 sub-agents
        ($96.80).
    - **Cause:** a sub-agent cannot end its turn and be woken. It waits
      inside a tool, and cannot hand over. 2.1 and 2.2 do not say how
      the roles are started.
    - **First visible:** in this retrospective. **Acted on:** not.
14. **temper broke on this plan's changes, and that went nowhere.**
    - **What:**
      - From 17:44 every check found temper failing against smith's
        `main` with 10 errors in its own code: `smith_transcript::v3`,
        `Request::Job` and `Turn`'s `jobs`. Each called it "for the
        user, not a steer".
      - It is not in "For the user", and temper's `STATUS.md` has no row
        for it. `later.md` has one.
      - The plan's "Repin: none" rests on decisions.md 8: the user
        reserved jig and temper for their own plans.
    - **Cause:** skein-ui's proposal 8, adopted in `e3b148e`, sends such
      work to the downstream `STATUS.md`. It was not in force here.
    - **First visible:** 17:44. **Acted on:** not; section 9 has the row
      for temper.
15. **`plan metrics` failed on the record, and misreads it.**
    - **What:**
      - Exit 5 on the torn line (section 3).
      - Then `running_h` (the coordinator's events), `restarts` (10 hand
        loads unseen), `proofs` (3 false, 8 unread), `lines` (data as
        code), `pin_lag` (a stale checkout), `steps_planned` (splits as
        planned) and `cost_claude` (sub-agents' sub-agents).
    - **Cause:** reliability's 17 left part of `numbers.sh`'s fixes to
      `kit-after-parity`. The torn line, the hand loads, and the
      sub-agents' sub-agents are new.
    - **First visible:** 20:19, this retrospective. **Acted on:** by
      hand here, with the copy of the rollouts.
16. **Small things in the record:**
    - **The user:** neither push reached the user. 00:13: "Not sent —
      this terminal is active"; 05:31: "Mobile push not sent (Remote
      Control inactive)".
    - **The supervisor:**
      - Ruling 133 priced 12.1's 58 attempts at $3.30, and the push told
        the user "under $10". Ruling 135 corrected it 2.3 hours later:
        $0.73 to $1.61 an attempt on `shell-cwd`.
      - It read code in its own context: 389K characters over 235 reads.
    - **The coordinator:**
      - `plan set --help` added a ledger row, which it then removed by
        editing `ledger.tsv`.
      - The thread column held `/root/wN` for w1 to w20.
      - Five `index_status` calls.
      - Its closing cleanup removed the supervisor's `plan docs`
        worktree.
    - **The implementers:** 22 `index_repository` calls, all on smith's
      primary checkout.
    - **The plan:**
      - Rule 10's `sed` reads only the plural, so 11.2's "1 attempt"
        trailer is missed: 521 against 522.
      - Session 04 reached 10 steps against README rule 2's 8.
      - 12.s1's drafted Proof line starts with "the".
    - **The designer:** its commits carry no `Designer-Session:` trailer,
      and three have no `Gate-Tree:`.
    - **This retrospective's own cost** was $54 when `metrics.tsv` was
      written, with its ten readers: about $5 a source.

## 6. Decisions taken during the run

Rulings 101 to 146, eleven steers, and two decisions of the user's. The
supervisor marked each decision in force in `done.md` with its design
commit, or "not in the design yet".

| Rulings | What | Reached the design | Proposal |
|---|---|---|---|
| 103, 108, 109, 111 | Disk: `kache gc`, a session's `target/` removed after its landing, the queue restarted, `kache gc` below 80 GB | Not a design matter. 108 is partly in development.md 11.4 on `main` (`d296e3e`); the 80 GB `gc` and the cap on sessions by disk are not | 7, 8 |
| 107, 110, 114, 115, 125, 134 | Memory at its bound: setup refusals, a branch's against `main`'s, and terms over-counted | 110 rests on limits.md 2.2 and 3.5, with no design change. 125 is not in the design yet, and contradicts benchmarks.md 4.2, "A red smoke stops the line" | 10 |
| 112, 118, 119, 121, 123, 124, 126, 145 | Whose failure a calibration counts; what is not exercised; `history` version 3; effort and token caps | 121 and 124 yes: smith `62376cb8`, `f242ecaa`. 145's defect is in `later.md` | 9 |
| 113, 116, 117, 120, 122, 127, 128, 130 | Session 10's job sketch: Files, numbering, clamps, the lifecycle across 10.2 and 10.6 | The session files only. 120, derived literals that follow a changed layout, is not in the design yet | — |
| 129, 131, 133, 135, 136 | The verdict's pipeline run live: compaction caps, a retried failure, the setup deadline, the cost, the baseline standing | 131 yes: smith `452c9967` | 2 |
| 139, 140, 141, 146 | The role: its count, its binary arms, 06.2's stop, and the closing runs | 146 yes: smith `7c345bd0`, benchmarks.md 16.5 and shell.md 6.6 | 3, 4 |
| 101, 102, 104, 105, 106, 137, 142, 144 | Prices at the resolved window; a lint exception; Files; a drift test's ceiling; ready checks; two panics found live (11.s1, 11.s2) | Not design matters | — |
| 132 | The flaky resume test retries once | Not a design matter; `later.md` has the test | Section 9 |
| 138 | No coordinator waker; a log by its queue time | Not in the design yet | 5 |
| 143 | The user doubled every cap: 680 attempts and $100 | Not a design matter | — |
| Steers 17:44, 18:44, 00:31 | One behaviour per test; named constants and fields | In development.md 5.6 and 14.3 before the run; named struct fields are not | 12 |
| Steers 23:33, 00:31 | Yield on a gate-lane run, with its task id; a one-attempt smoke waits | Close to `main`'s implementer.md 8.3, not the same | — |
| Steer 06:33 | A new increment past 150K goes to a fresh thread | In part: `main`'s 7.3 holds it for a resumed session only | 6 |
| Steers 20:44, 22:29, 07:32, 08:15 | 10.1's world proof with 10.6; hold 12.1; 11.s1 first; the caps | The plan only | — |

**Decided by the user during the run:**
- leave to delete the old benchmark's `target/` directories (17:31,
  ruling 109);
- the caps doubled (08:13, ruling 143).

**Still with the user:** "For the user" holds the compaction
threshold's evidence (12.2): keep 4/5, or change it. Nothing waits on
it.

## 7. The proposals adopted after the last retrospective

The last retrospective is reliability's, decided at 10-14 09:24 (smith
`abe6ec2e`), 58 minutes before the claim. It was applied in skein
`479f836`, `7f4b0e8` and `bc26df2`, and smith `97875b08`, all in force
at `3d7d304`. Its rows "adopted in part" put some rules into this
plan's README, and deferred kit changes to after this run. test-budgets'
and jig-protocol's adopted parts were in force too.

| Proposal | Decision | Meant to move | reliability | parity | Moved |
|---|---|---|---|---|---|
| 1. Split past Size without asking | in part: README rule 1 | size blocks; plan defects | 20 size blocks | 0; 10.5 and 10.6 split | Yes |
| 2. At most 8 increments a session; the chain named | in part: README rule 2 | `wall_h`, `step_h` | 0.33 | 0.58 wall hours a step | Not comparable: live suites set the length. Session 04 reached 10. |
| 3. The planner's records in `findings/` | in part: README | plan defects a step | 0.91 | 0.29 | Yes |
| 4. Users outside Files | in part: implementer.md 8.2, README rule 3 | cross-session scope a step | 0.30 | 0.19 corrected | Yes |
| 5. Hand over by the peak context | in part: 7.3 step 4 | `contexts` | 245K, 348K | 201K, 293K | Yes. The 06:33 steer did the most (section 5, item 5) |
| 6. A fresh designer past 150K or an hour idle | in part: 10.4, `plan wake` | the designer's tokens | 249.6M for 26 questions, 9.6M each | 39.6M for 4, 9.9M each | Not per question: every wake after the first started fresh, and each session passed 150K within 6 to 8 minutes, reading |
| 7. The supervisor hands over at 300K | adopted | the supervisor's tokens | 583.5M, to 966K | about 277M, peaks 251K to 302K | Yes |
| 8. The hourly check by a sub-agent; the pulse unfiltered; `plan downstream` read | adopted | findings | — | 27 checks; 11 acted on | Yes, but no check from 11:35 to 17:38 (section 5, item 12) |
| 9. Each card names its wait; an idle coordinator ends its turn | in part | waits | 6.0% of requests after a wait ran out | 4.1% | In part. 37% of the implementers' input waited, in the card's 60-second polls cut at 30 s. The 300-second part was deferred |
| 10. Read by section | in part: implementer.md 8.1, README rule 4 | the implementers' tokens | 14.8M a step | 13.4M | A little. First edits came at a median of 90K |
| 11. No graph in the spawn prompt | in part: 7.2 | the coordinator's tokens | 336 graph calls | 5 `index_status` calls | Yes |
| 12. `## Checked` in every unblock | adopted | `answers_reversed` | 0.10 a step | 0.017 | Yes |
| 13. A diagnosis budget | in part: README rule 5 | process blocks | 14 | 1 | Yes |
| 14. Clippy in `land`'s build lane; dependents' tests | in part: `land`, README rule 6 | `gates_per_step`, `queue_wait_ratio` | 1.75, 0.93 | 1.84, 0.74 | Gates no: re-gates after `main` moved (section 5, item 11). Queue yes |
| 15. A landed step checked by its branch; trailers; Pulse words | in part | process blocks | — | no trailer defect | Held |
| 16. The shapes of `log.md` and `context.md` | in part: 7.5 | comparability | 14 invented kinds | none | Yes |
| 17. `plan metrics` counts what 15.2 says | in part: `numbers.sh` | comparability | — | planner counted; 10 defects left (section 5, item 15) | In part |
| 18. "Done when" run once after `done.md` | adopted | verified ends | — | run at 20:13 to 20:17, with output | Yes |
| 19. The design's rulings applied | adopted | — | — | — | Not a metric |
| 20. The supervisor runs `plan resume` itself | in part: 10.5 | lost minutes | 11 minutes | 10 hand loads; `plan resume` failed each time | No: "thread not found" (section 5, item 4) |
| 21. Calibrations in task files | in part: README, `97875b08` | live proofs calibrated | — | "Done when" 4 prints 4 | Yes |

**test-budgets' and jig-protocol's adopted parts:**
- **test-budgets' 1, decide before the run:** held.
- **test-budgets' 9, `plan log`, no reports typed again:** held. The
  coordinator typed none, but read 120 of them in full.
- **test-budgets' 12, a design gap always reaches the designer:** four
  design questions, four answers. Ruling 112 read a gap as a plan defect
  (section 5, item 7).
- **jig-protocol's 10, `plan idle` and the doorbell:** the coordinator
  idled 29 times. Nothing in the kit rang it when its tasks ended
  (section 5, item 4).
- **jig-protocol's 14, questions to the user pushed at once:** never
  adopted. One question waited inside a tool for 4 h 54 min (section 5,
  item 1).

**Deferred until after this run.** The kit changes on skein's
`kit-after-parity` (`d7e6ee3`), and reliability's deferred rows (3, 5,
9, 11, 15, 16, 17 and 20, in part), come back to the designer now
(`later.md`). This run's evidence for them:
- **test-budgets' 4, `land` holding the turn:** about 40 passing gates
  lost (section 5, item 11).
- **reliability's 17, the metrics:** section 5, item 15.
- **reliability's 20, a restart's cause and `alerts.md`:** the
  supervisor's push did not arrive twice (section 5, item 16).
  `alerts.md` would have shown the dead queue in `plan status`.
- **client-composition's 4, `plan ring` and `plan resume` loading the
  thread:** 10 hand loads (section 5, item 4).
- **protocol's 4, the coordinator compacting at 200K:** 69% of its input
  past 200K (section 5, item 6).
- **skein-ui's 10, `plan downstream` outside `wt/`:** the closing
  cleanup removed `wt/smith-downstream` and `wt/temper-downstream`,
  keeping their `Cargo.lock`s in stashes.

## 8. Proposals

Each gives a rule in one sentence, its evidence, the document it
changes, and the metric it should move. They are ordered by expected
effect. Proposals 5, 6, 7, 8, 11 and 15 change the kit, and would join
`kit-after-parity`. The rest change text only.

1. **No role asks the user through a tool that waits for the answer:
   a question for the user goes into "For the user" with a push, and the
   session goes on with its work; supervisor.md names AskUserQuestion
   as one such tool.**
   - **Evidence:**
     - One question, asked at 12:37:08 with nothing waiting on it, held
       the supervisor until 17:31:26.
     - The dead queue's block, the five hourly checks and every event
       waited behind it: 5.0 h of the line, 68% of `blocked_h`.
     - After it, the cap question went into "For the user" with a push,
       and waited 8 hours without holding anything (section 5, item 1).
   - **Changes:** supervisor.md, its tools; development.md 10.4.
   - **Moves:** `blocked_h` from 7.3 toward 2.3; `wall_h` down.
2. **When a plan's verdict needs a live suite, the increment that
   completes the suite's pipeline proves it by that suite at one attempt
   per task, through its summary, `verify` and verdict, before any step
   runs it in full.**
   - **Evidence:**
     - 12.1 was the pipeline's first live run, 12 hours in. It took
       four runs, 117 attempts against 58, and 5.5 hours.
     - It found three faults, each a supplement: a provider answer with
       no usage, a setup deadline a cold build could not meet, and a
       summary `verify` refused.
     - 11.3's first live run found two panics (11.s1, 11.s2).
     - About 1.6 h of suite time measured nothing whole (section 5, item
       2).
   - **Changes:** development.md 5.5, beside `e30fe58`'s smoke check;
     5.3.
   - **Moves:** `steps_added` (11) down; gate-lane hours that measure
     nothing (1.6) down; `blocked_h` down.
3. **A plan whose aim is a verdict that can fail by chance settles,
   before the run, how many closing runs it holds and what it does if
   the last misses; when its baseline misses a criterion, the supervisor
   asks the designer at once whether the steps left aim at it.**
   - **Evidence:**
     - decisions.md 1.1 gives an equal smith a 0.69 chance. 12.4 left a
       miss "a question for the designer".
     - That question stopped the line for 19 minutes at 15:32, and
       ruling 146 then set the bound the plan lacked.
     - The baseline missed RT at 03:48, and ruling 136 called it "the
       baseline's finding". No later step aimed at RT, and the plan
       ended short on it (section 5, item 3).
   - **Changes:** development.md 5.1 (what to settle first) and 10.4;
     smith's benchmarks.md 16.5 already bounds the runs.
   - **Moves:** design gaps at the end down; criteria failed at the
     closing run down.
4. **A step after the baseline that changes what the model is sent
   reports the measures the aim's criteria read, and stops on a question
   when one moves the wrong way past its interval, whatever its own
   comparison concludes.**
   - **Evidence:**
     - 06.2's comparison took responses an attempt from 7.13 to 8.33,
       and ruling 141 landed it on "no difference".
     - FT then went from 0.995 at the baseline to 1.193 at the first
       closing run, which failed. 12.s1 brought it back to 1.090
       (section 5, item 3).
   - **Changes:** development.md 5.5 (a live proof's report); the plan's
     shape for a verdict.
   - **Moves:** closing runs that fail on a step's own effect, from 1 to
     0.
5. **`plan idle` takes the ids of the queue tasks the coordinator
   waits on, and `plan watch` rings it when they have all ended; `plan
   ring`, `plan resume` and the watch load an unloaded thread first.**
   - **Evidence:**
     - 10 hand loads of the coordinator's thread, each after "thread not
       found".
     - About 1.0 h of late wakes, at 04:17 and 06:46.
     - The coordinator's own wakers died with its turn. The supervisors
       wrote theirs by hand, by README rule 17 (section 5, item 4).
   - **Changes:** `plan` (`idle`, `watch`, `ring`, `resume`);
     development.md 7.4 and 10.5; supervisor.md. It takes in
     client-composition's 4.
   - **Moves:** the coordinator's idle on ended tasks (about 1 h) to 0;
     hand loads (10) to 0; the supervisor's tokens down.
6. **Between increments, a session hands over past 150,000 tokens, as a
   resumed one does.**
   - **Evidence:**
     - Before the 06:33 steer, handovers came at 200K to 329K, and 9 of
       26 came above 250K.
     - 06.2 started at 164K and cost 24.2M, against 8.6M for 06.1.
     - After the steer, handovers came at 156K to 186K, and later peaks
       were 135K to 178K (section 5, item 5).
     - `main`'s 7.3 already holds 150K for a resumed session.
   - **Changes:** development.md 7.3, step 4; `PLAN_CONTEXT_LIMIT`;
     implementer.md 8.6.
   - **Moves:** `contexts` p90 from 293K to under 200K;
     `tokens_implementers` from 13.4M a step down.
7. **A session's `target/` goes when its step lands; the coordinator
   starts a session only while the free disk covers 25 GB for each
   running session and 40 GB more; and `kache gc` runs at every disk
   check below 80 GB.**
   - **Evidence:**
     - Three disk stops: 10:45, 12:32 and 18:40. Sessions' `target/`
       were 17 to 21 GB, against 11.4's 15.
     - 29 GB went in about five minutes, between two checks.
     - kache's store was past its cap twice, at 135 and 133.6 GiB
       against 74.5.
     - After ruling 108 and the 18:44 steer, no disk event followed, and
       every check read 133 to 158 GB free (section 5, item 1).
   - **Changes:** development.md 11.4; `plan`'s pulse.
   - **Moves:** infrastructure escalations (4) down; `blocked_h` down.
8. **pueued restarts itself after a delay, with no limit on restarts,
   and `plan check --fix` sets that up; `plan watch` alerts when the
   queue's daemon is down.**
   - **Evidence:**
     - pueued died on the full disk at 12:33:21, and systemd gave up
       within a second.
     - From 12:38:50 the disk had 112 GB free. The daemon stayed down
       until 17:31:54, and nothing reported it but the coordinator's
       block (section 5, item 1).
   - **Changes:** `plan` (`check --fix`, `watch`); development.md 11.1.
   - **Moves:** `blocked_h` down; hours with no landing down.
9. **A block on what a term of the design means — what decides, what
   counts, whose failure — goes to the designer, with an interim ruling
   if the step can go on; it is a plan defect only when the ruling
   quotes the design text that answers it.**
   - **Evidence:**
     - Ruling 112 (18:53) read whose failure a calibration counts as a
       plan defect. The designer was asked 87 minutes later.
     - The same area took eight rulings and three supplements. 07.3
       spent 59 attempts against 28.
     - Ruling 145 leaves its defect open at the end (section 5, item 7).
   - **Changes:** development.md 10.4; supervisor.md 3.
   - **Moves:** the wait to a design gap's first answer down;
     `steps_added` down.
10. **A plan's premise that a bound holds unchanged is checked by a
    command in `planning-commands.md`, and the first increment that comes
    near the bound adds a test that fails when its margin falls below a
    floor the plan names.**
    - **Evidence:**
      - decisions.md 7.4 said "Memory needs no change". Eight setup
        refusals and six rulings followed, and the smoke failed on `main`
        itself at 19:25.
      - The margin left is 28,783 bytes of 2 GiB (section 5, item 8).
    - **Changes:** development.md 5.4 (the planning commands) and 5.6.
    - **Moves:** plan-defect escalations (17) down; `steps_added` down.
11. **`plan watch` returns `check:` when `supervision.md`'s last check
    is over 65 minutes old, and the hourly check needs no cron.**
    - **Evidence:**
      - The 19:23 handover deleted the cron, and the next check came two
        hours after the last.
      - The blocked session's cron did not fire for six hours.
      - skein-ui had gaps of 87 and 75 minutes for the same reason
        (section 5, item 12).
    - **Changes:** `plan` (`watch`); development.md 14.1; supervisor.md 6.
    - **Moves:** the longest gap between checks, from 6.0 h to about
      1 h.
12. **A check's report ends with the steer it proposes for each
    checklist finding not yet steered, and the supervisor sends it or
    writes in its entry why not.**
    - **Evidence:**
      - One behaviour per test was steered at its second and third
        sightings, bare numbers at their sixth.
      - Helpers defined again were seen in seven checks and never
        steered: `run` in 19 files, `finish` in 12.
      - The test the 17:44 steer named still tests two outcomes (section
        5, item 9).
    - **Changes:** development.md 14.4; supervisor.md 6.
    - **Moves:** sightings before a steer from 2 to 6 down to 1; test
      debts at the end down.
13. **While the run's only work is one live suite that a waker
    watches, the hourly check is `plan pulse`, read by the supervisor
    itself, with no sub-agent.**
    - **Evidence:**
      - The checks cost $55, 48% of the supervisor.
      - 16 of 27 led to no action ($21.4), among them all eight from
        10-15 12:47 to 19:47, which reported a suite's progress (section
        5, item 12).
    - **Changes:** development.md 14.1; supervisor.md 6.
    - **Moves:** `cost_claude`'s supervisor part ($114) down.
14. **The designer and the planner each run as a session of their own,
    started with `claude --bg` or by the user, never as a sub-agent of
    another session; each landing names its session in a trailer of its
    role.**
    - **Evidence:**
      - The planner, a sub-agent, ran to 572K: 136 requests past 300K,
        $12.54. It could not hand over.
      - It waited in three polling loops, and the design's sub-agent
        rewrote its cache three times ($6.09).
      - The trailers name a design sub-agent as a planner, and miss
        $96.80 of their sub-agents (section 5, item 13).
    - **Changes:** development.md 2.1, 2.2 and 15.2; the workspace's
      `AGENTS.md`, "Starting things".
    - **Moves:** the planning's cost ($141) down; `cost_claude`
      comparable.
15. **`plan metrics` skips a trace line it cannot parse, counts hand
    loads as restarts, reads every Proof word and a step's last commit,
    counts data apart from code, reads pins from origin's `main`, counts
    split steps as added, and counts the sub-agents of a named
    sub-agent.**
    - **Evidence:** exit 5 on one torn line, and the seven misreads of
      section 3 (section 5, item 15).
    - **Changes:** `numbers.sh`; development.md 15.2. It joins
      reliability's deferred 17.
    - **Moves:** comparability: no corrected column.
16. **A run keeps the process of its claim; a change to development.md
    or a card that lands during another run says in its commit whether
    that run should take it, and that run's supervisor passes it on as a
    steer.**
    - **Evidence:**
      - Five adoptions landed during the run, and none reached it.
      - The run reached three of their rules again, hours later and at a
        cost: the 06:33 steer, ruling 108, and the 23:33 steer.
      - Its implementers read the 60-second wait all run (section 5,
        item 10).
    - **Changes:** development.md 4.4 and 15.1 (the retrospective names
      the skein commit the run kept).
    - **Moves:** steers that re-derive an adopted rule (3) to 0;
      section 7's comparability.

## 9. Debts left in the code

For smith's `tech-debt.md`, and for skein's (its rows are for the
designer).

**smith:**
- **Memory's margin is 28,783 bytes of 2 GiB** between
  `service.worst_case` and `declared.memory` (10.4, `log.md` line 3813).
  The next term added will refuse at setup. No test holds the margin
  (proposal 10).
- **Six placeholder arms** in `crates/smith-domain-tools/src/flight.rs`
  (lines 430, 451, 482, 537, 582 and 611):
  `unreachable!("the tools issue no yield or job act yet")`, though the
  tools now issue job acts through the job table. An arm of the same
  words, in `smith-inline-agent`'s `translate.rs:513` (from 09.2),
  panicked in 11.3's live run, and 11.s1 replaced it (ruling 142).
- **48 `unreachable!` arms added in the run,** 33 of them outside tests:
  - `smith-domain-tools`: `flight.rs` 8, `jobs.rs` 4, `kit.rs` 3,
    `authority.rs` 1;
  - `smith-domain-run`: `land.rs` 4, `run/jobs.rs` 4, `agent.rs` 2;
  - `smith-agent-service`'s `calls/jobs.rs` 2,
    `smith-protocol-machine`'s `process/jobs.rs` 2,
    `smith-protocol-llm`, `smith-domain` and `benchmarks/src` 1 each.

  Each is a state the types allow.
- **Files past 800 lines grew:** `benchmarks/src/probe.rs` 634 → 823,
  `benchmarks/src/batch.rs` 954 → 1,004,
  `crates/smith-domain-session/src/session.rs` 3,578 → 3,630, and
  `crates/smith-agent-service/src/service.rs` 2,905 → 2,920.
- **Tests:**
  - `tests/machine/tests/jobs.rs:132`,
    `a_cancelled_held_job_answers_cancelled`, tests two outcomes (the
    17:44 steer).
  - Helpers defined again: `run` in 19 files, `finish` in 12, `read` in
    14. A 25-line setup is copied into 5 files, and a fifth copy of the
    Start…Started sequence.
  - Bare numbers: a 4096 in place of `TASK_ROLE_BOUND`, and 55 and 91
    (the 06:26 and 08:26 checks).
- **Two tuples that dodge clippy's argument limit,** and 04.1's branch
  on a probe's name (the 00:24 and 04:24 checks).
- **The design:** benchmarks.md 4.2's "A red smoke stops the line"
  against README rule 8 as amended by ruling 125.
- **From `later.md`:**
  - RT on `shell-cwd`;
  - `tool-output/timeout`'s calibration and its classification defect;
  - a `finish` without `fields`;
  - `calibrate`'s rewrite of the whole `task.toml`;
  - the flaky `a_second_run_resumes_the_chat_from_its_files`;
  - six task files to recalibrate before `suite smoke` is relied on.

**skein, the kit** (for `kit-after-parity`):
- **`numbers.sh`:**
  - stops on a torn trace line;
  - `running_h` counts only the coordinator's events;
  - `restarts` misses hand loads;
  - `proofs` misreads, and judges a first commit;
  - `lines` counts data as code;
  - `pin_lag` reads a stale checkout;
  - split steps count as planned;
  - `cost_claude` misses a named sub-agent's sub-agents;
  - `wall_h` reads `done.md`'s last write (proposal 15).
- **`plan ring` and `plan resume`** read "thread not found" as no turn
  (client-composition's 4; proposal 5).
- **No waker** for a coordinator idle on queue tasks (proposal 5).
- **pueued's start limit** (proposal 8).
- **pueue's state** is 13.2 MB with 3,586 finished tasks from 10-09 to
  10-12. pueued is among the three busiest processes at 119 of 207 gate
  starts, at a mean 40.6% CPU.
  - Ids are reused: 303 of 955 tasks are 3787.
  - `plan start` could clear finished tasks.
- **`land` re-gates** after `main` moves (test-budgets' 4).
- **The coordinator's compaction at 200K** (protocol's 4).
- **`plan downstream`'s worktrees in `wt/`** (skein-ui's 10).

**Workspace:**
- **temper fails against smith's `main`** with 10 errors in its own
  code: `smith_transcript::v3`, `Request::Job` and `Turn`'s `jobs`.
  - temper's `STATUS.md` has no row for it. skein-ui's 8 sends it there
    as "Not planned": the transcript's version 4, and the tools domain's
    new `Call` and `Outcome` variants, for temper's next repin.
  - jig, not on this machine, takes `smith-domain-tools` the same way.
- **temper's checkout here** is 268 commits behind origin.
- **The stashes** smith `96cdd17c` and temper `697a9805` hold only
  `plan downstream`'s regenerated `Cargo.lock`s. Copies are in
  `logs/coordinator/`.
- **kache's eviction fails on this machine,** and its store ran past its
  cap twice. `kache gc` by hand until it is fixed.

## 10. Decisions

2026-10-16. The user delegated these decisions. Before the run ended:
"if there are open questions to settle, tell the agents to use their
judgement to settle them, because otherwise the new plan can't be
started. Same for the smith plan currently in progress, in case it
lands." Earlier, the designer's recommendation stands "unless there is
something with long-term consequences and hard to change decisions"
(2026-10-14). And on 2026-10-16: "my idea is to start a new design/plan
for smith as follow-up of the parity one, with the goal of improving the
wall clock performance of smith wrt codex. I'll wait for the designer to
finish, then review its decisions and move ahead." None of these is hard
to change, and none is a product decision; proposal 4, which would
settle how a smith change shows its effect on the criteria, waits for
that design. The designer decided them on the evidence of sections 5, 7
and 8, as the rows say, and applied what is adopted in skein `30bc10f`
(development.md and its two cards). The same landing round put this
plan's `later.md` row for the repositories downstream of smith into
their `STATUS.md`, under "Not planned" (jig `0290387`, temper
`53b52a2`), as skein-ui's proposal 8 asks. Changes to the kit (`plan`,
`land`, `numbers.sh`, the coordinator's Codex profile and the
workspace's files) go on skein's branch `kit-after-parity`, which waits
until temper's client-api run has ended: that run lands in skein, and
its landing would bring a kit landed meanwhile into its machine's
checkout mid-run. Where a kit part leaves the next run exposed, a text
rule stands in until it lands (proposals 5 and 11).

| Proposal | Decision | Where |
|---|---|---|
| 1 | adopted, taking in jig-protocol's proposal 14, its first half, never decided: a question for the user is pushed at once, in one line with the choice and the default | skein 30bc10f: development.md, 1 and 10.4; supervisor.md, 9 |
| 2 | adopted | skein 30bc10f: development.md, 5.3 and 5.5 |
| 3 | adopted, generalizing smith's benchmarks.md 16.5 (a plan that targets parity bounds its closing runs) to any plan whose aim is a measured verdict; a baseline's miss goes to the designer at once | skein 30bc10f: development.md, 5.1 and 10.4; supervisor.md, 4 |
| 4 | deferred: to smith's wall-clock design, the parity run's follow-up. That design is to say how a change that alters what the model is sent reports its effect on the measures the criteria read (benchmarks.md 4.2 and 9, against 16.2), and whether a move the wrong way past the baseline's interval stops it, whatever its own comparison concludes. Until then, proposal 3's rule sends a baseline's miss to the designer. Adopted there, 2026-10-16: after a plan's baseline, such a change runs the `parity-change` suite against its parent, reports each criterion's ratio and its projection on the baseline, and stops on a question to the designer when a projection passes the baseline's interval, whatever its own comparison concludes | smith's wall-clock design: benchmarks.md, 1, 4.2, 11.5 and 16.6; the wall-clock plan's `decisions.md`, 4 |
| 5 | adopted; the kit's: `plan idle` takes the ids of the queue tasks waited on, `plan watch` rings when they have ended, and `plan ring`, `plan resume` and the watch load an unloaded thread first, taking in client-composition's 4 and protocol's 8, its kit part, both deferred. Until it lands, the supervisor loads the thread by hand, as the parity run's supervisors did | skein's branch `kit-after-parity`: `plan` (`idle`, `watch`, `ring`, `resume`), with development.md, 7.4, 10.5 and 13, and supervisor.md, 2; meanwhile skein 30bc10f: development.md, 10.5 and 13; supervisor.md, 2 |
| 6 | adopted, extending skein-ui's 3 (a resumed session past 150,000 tokens) to the handover between increments. `PLAN_CONTEXT_LIMIT`'s default is the kit's: until it moves to 150,000, the coordinator reads `plan contexts`' numbers, flagged or not | skein 30bc10f: development.md, 2.4, 5.3 and 7.3, step 4; implementer.md, 8.6; skein's branch `kit-after-parity`: `PLAN_CONTEXT_LIMIT` |
| 7 | adopted in part: a session's `target/` goes after each of its landings, at most six sessions run below 60 GB of disk (ruling 108), and `kache gc` runs at every disk check below 80 GB (ruling 111), replacing protocol's 10 ("`kache gc` when the store is past its cap, once"); the run tested these, and no disk event followed. The start bound of 25 GB for each running session and 40 GB more is rejected: the run never tested it, and on the 133 to 158 GB its checks read it would have held the run to four or five sessions, where ruling 108's rules allowed eight with no disk event. The pulse's disk line is the kit's | skein 30bc10f: development.md, 11.4; implementer.md, 8.1; skein's branch `kit-after-parity`: `plan pulse` |
| 8 | adopted; the kit's: pueued's unit restarts after a delay with no start limit, `plan check --fix` sets it up, and `plan watch` alerts when the daemon is down, with reliability's 20's `alerts.md`, deferred. Until then, development.md 10.6 holds (`plan check --fix` starts the daemon), and proposal 1 keeps the supervisor free to see the block | skein's branch `kit-after-parity`: `plan` (`check --fix`, `watch`), with development.md, 11.1 |
| 9 | adopted, sharpening test-budgets' 12 (a design gap always reaches the designer) | skein 30bc10f: development.md, 10.4; supervisor.md, 3 |
| 10 | adopted, the command's output in the plan's `findings/` (`planning-commands.md` was this plan's own file) | skein 30bc10f: development.md, 5.4 and 5.6 |
| 11 | adopted; the kit's: `plan watch` prints an event when `supervision.md`'s last check is over 65 minutes old, and the hourly check needs no cron. Its word is the kit's to choose: `check:` already names the ready check. Until it lands, a successor or a restarted supervisor whose last check is over 45 minutes old runs one at once | skein's branch `kit-after-parity`: `plan watch`, with development.md, 14.1, and supervisor.md, 1, 2 and 6; meanwhile skein 30bc10f: supervisor.md, 1 |
| 12 | adopted | skein 30bc10f: development.md, 14.1, step 5, and 14.4; supervisor.md, 6 |
| 13 | adopted, amending reliability's 8 (the hourly check by a sub-agent) for an hour with no session running | skein 30bc10f: development.md, 14.1; supervisor.md, 6 |
| 14 | adopted in part: a design and a plan are written in sessions of their own, started by the user or with `claude --bg`, never as a sub-agent of another session, extending client-composition's 7 (wait, and hand over past 300K) and test-budgets' planning in a fresh session. A bounded task of the designer's with one report, such as these decisions, may run as a fresh sub-agent, its landing naming both sessions. The workspace `AGENTS.md`'s "Starting things", and the metrics counting a named sub-agent's own sub-agents (proposal 15), are the kit's | skein 30bc10f: development.md, 2, 2.1 and 2.2; skein's branch `kit-after-parity`: `workspace/AGENTS.md` |
| 15 | adopted; the kit's, taking in reliability's 17's deferred rest, protocol's 11 and client-composition's 9's deferred part: `plan metrics` and section 15.2's definitions change together | skein's branch `kit-after-parity`: `numbers.sh`, with development.md, 15.2 |
| 16 | adopted | skein 30bc10f: development.md, 4.4, 14.1 (step 2) and 15.1; supervisor.md, 1 and 6 |
