# Retrospective: live-text

2026-10-19. This is the run of smith's `live-text` plan on debian-8gb-hel1-1, a machine with 8 GB of memory and a 78 GB ext4 disk. In the plan, a host asks for live content in the start, and the run sends its main conversation's text up the channel in pieces, best effort, before the turn's record. skein's fake LLM learned to write a reply in pieces and to hold it at a named point until its host releases it. smith moved its channel to version 3, with the ask and the live record. It sends pieces at the bound or on the live interval, and counts what it drops. jig's and temper's parts are later plans.

| Event | When (UTC) |
|---|---|
| Plan landed (smith `0e9c3308`) | 08:56:58 |
| The director's answers to "For the user" (smith `2efd6c92`) | 09:02:52 |
| Claim, with `kit/` at skein `6df8817` | 09:07:21 |
| First landing (skein-01.1, skein `f1a6219`) | 09:35:07 |
| Last landing (smith-02.2, smith `cd76fb64`), `done.md` recorded | 11:15:06 |
| "Done when" run by the supervisor's sub-agents | 11:16 to 11:36 |
| Release | 11:36:47 |
| `plan retro` | 11:37:49 |

All 8 steps landed: 3 in skein and 5 in smith. None was added or dropped. This retrospective was written by a fresh session that took no part in the run (development.md, section 15).

**Who read what.** Six read-only Sonnet sub-agents each read one source, and a seventh answered two questions about skein's history:
- the git history of skein and smith from the plan's base, with its trailers, and each step's Size by the plan's command;
- `logs/heavy.tsv` and the logs it names;
- the coordinator's Codex rollout and its eight implementer threads;
- the Claude Code transcripts: the planner's, the supervisor's with its five sub-agents, and the design sessions `plan metrics` counted;
- the earlier retrospectives, the metrics history, and skein's process from the claim's commit to `main`;
- the debts: the 14 reports, `later.md`, the code on `main` and smith's `tech-debt.md`.

This session read:
- development.md's section 15, and section 5's README form, aims and reference tests;
- implementer.md's graph (lines 74 to 82) and waits (8.3);
- the plan's README, `decisions.md`, `later.md` and `findings/planning-commands.md`;
- the run's ledger, `log.md`, `supervision.md`, `done.md`, session logs and reports;
- skein's `scripts/dev/workspace/coordinator.config.toml`;
- client-api's retrospective, and the metrics of the five runs in section 3.

Not read: the transcripts of `5e38559b` and `ffc2b313`, two of the five design sessions `plan metrics` counted. They are not on this machine.

**Conventions.** Times are UTC, on 2026-10-19 unless a date is given. Costs are at the list prices of each message's model, as `plan metrics` prices them:

| Model | Input | 1-hour cache write | Cache read | Output |
|---|---|---|---|---|
| Opus 5.5 | $4 | $8 | $0.20 | $20 |
| Sonnet 5.5 | $2 | $4 | $0.10 | $10 |

This document proposes; the user, or the director, decides, and the designer applies what is adopted.

**Which process.**
- **The claim.** `kit/` was pinned at skein `6df8817` at the claim, and `supervision.md`'s Process line names it.
  - `6df8817` is a forge-kit step (skein-05.4), not a process commit.
  - Its `docs/foundation` and `scripts/dev` are those of `b4c6241` (23:04 on 10-18) and `49e7eb6` (10-18), both from before client-api's retrospective.
- **During the run.**
  - Nothing landed in skein's `docs/foundation` or `scripts/dev` between the claim and the release. The hourly check at 10:14 found the same.
  - `92e177c`, a docs repoint to fabrig-dev, landed at 11:38:07, after the release.
  - forge-kit's run landed 21 commits on skein's `main` during the run, all in its forge crates.
- **So the nine proposals adopted from client-api reached this run neither as text nor as kit** (section 7).

## 1. In one page

- **8 of 8 steps landed in 2.1 hours.** "Done when" passed; item 8 passed only on a second measurement.
  - skein's fake LLM writes a reply in pieces, in both dialects, and holds it until released. Its byte peer releases it.
  - smith's channel is at version 3, alone. The host asks for the pieces and hears them.
  - The agent sends pieces on the interval and counts what it drops.
  - smith pins one skein, which has the release.
  - All 10 land gates passed. The five live smokes passed first time, for $0.0034.
- **The chain was the run.**
  - Five of the eight steps formed one chain, smith 01.1 to 02.2. Each took 25.7 to 27.1 minutes from its thread's start to its landing, except the last, 18.0.
  - The README priced the chain at 0.5 to 0.7 h, from earlier runs' `step_h`. It took 2.1 h (proposal 6).
- **smith 02's session file was corrected four times** (rulings 101, 103, 104 and 105), and two of the corrections came from blocks.
  - It was written at 08:19, three minutes after one of the planner's 16 compactions.
  - Two of the corrections were facts of the base, which the plan's sketches had replaced with stand-ins (proposal 8).
- **"Done when" item 8 failed as written.**
  - Its logs, written inside the worktree it measured, made smith-bench's probe test refuse a dirty tree. Each focused run stopped after about 15 tests.
  - The planner had met the same refusal at the base.
  - A clean re-measure gave 11.62 s against an aim of 12 (ruling 106; proposal 2).
- **The proposals adopted from client-api were not applied before the claim.**
  - The kit and the cards were those of 10-18.
  - The implementers' waits were the defect that client-api's proposal 3 named: 22.9% of their requests only waited, and those requests were 27.4% of their input (proposal 1).
- **The implementers had no code graph.** The coordinator's Codex profile turns the server off. Every report says it was unavailable, while implementer.md tells implementers to use it (proposal 3).
- **The test shape needed a steer again.**
  - The plan's reference tests broke testing-strategy.md section 10.
  - The 10:14 check found landed tests that broke it too.
  - Ruling 102 made section 10 README rule 7, for the tests still to come (proposal 4).
- **The numbers.**
  - 13.8M tokens a step, between forgejo-17's 19.1M and client-composition's 8.8M. The implementers took 7.67M a step.
  - The supervisor cost $13.
  - `cost_claude` says $491, but $424 of it is three designer sessions that designed other things (proposal 5).
- **What held:**
  - two blocks, each answered in under 4 minutes;
  - rings of 5 requests;
  - no steer point refused;
  - a queue wait ratio of 0.18;
  - no lint exception landed.
- **Eight proposals** (section 8):
  - adopted proposals are applied before the next claim;
  - no command a plan gives writes in the tree it measures;
  - implementers have the graph, or are told they have none;
  - the planner's review checks the reference tests;
  - the design's sessions come from the design's commits;
  - the chain is priced at a step's span;
  - the pulse's aims verdict sums the packages;
  - sketches compile against the base's own crates.

## 2. Outcome

`done.md`'s "Done when, run 2026-10-19T11:33:25Z" holds each command's output. After the last landing, the supervisor's Sonnet sub-agents ran the commands in detached worktrees of skein `9ffcb60` and smith `cd76fb64`.

| Item | Result |
|---|---|
| 1. Every step landed or dropped | **Met.** `plan status`: 8 of 8 landed. |
| 2. skein's fake LLM writes pieces, held until released | **Met.** The three greps print nothing. |
| 3. smith's channel at version 3, alone | **Met.** `channel-v3.schema` alone, then `1`. |
| 4. The host asks, and hears the pieces | **Met.** |
| 5. Pieces on the interval, drops counted, no second switch | **Met.** No `live_text` field remains. |
| 6. One skein pin, which has the release | **Met.** skein `5c8ae967`. |
| 7. The memory floor in smith's shipped-values test | **Met.** It prints 1. The headroom above the 490,000,000-byte floor fell from about 2.82 MB at the base to 2.26 MB, by 20% (section 9). |
| 8. The test-time aims | **Met on a second measurement.** As written, smith focused failed: the command's own logs in the worktree made `smith-bench`'s probe test refuse a dirty tree. It was re-measured at 11:36 in a clean worktree of smith `ff623361`, with the logs outside the tree (ruling 106). Medians of three package sums: skein focused 4.45 s against 5.7, fuzzy 6.93 s against 8.4; smith focused 11.62 s (11.38, 11.62, 12.67) against 12, fuzzy 17.88 s against 18.5. |
| 9. Each session's own "Done when" | **Met.** |
| 10. Nothing else checked | Nothing to run. |

smith's two aims passed by 3%, and one of the three focused runs was past its aim.

What the plan names but leaves for later is in `later.md`'s nine rows. They include jig's and temper's parts, their repins, and the allocation in `Peer::release`. One row is stale (section 5, item 11).

## 3. Numbers

`plan metrics` wrote `metrics.tsv`. The comparisons:
- forgejo-17, the run nearest in size: 11 steps in 2.4 h;
- kit-batch, a short run of 29 steps;
- wall-clock, smith's last run;
- client-composition;
- client-api, the run of the last retrospective.

Figures given "a step" divide by planned and added steps.

| Metric | live-text | Notes | forgejo-17 | kit-batch | wall-clock | client-composition | client-api |
|---|---|---|---|---|---|---|---|
| `wall_h`, `running_h` | 2.1, 2.1 | The chain's five threads ran from 09:08 to 11:15. | 2.4, 2.0 | 3.8, 2.2 | 29.1, 5.6 | 4.3, 4.3 | 63.5, 60.0 |
| `steps_planned`, `steps_added` | 8, 0 | skein 3, smith 5; none dropped. | 9, 2 | 25, 4 | 20, 0 | 36, 0 | 137, 22 |
| `step_h` | 0.27 | Five of the eight steps were one chain, with at most two sessions at once. | 0.18 | 0.08 | 0.28 | 0.12 | 0.38 |
| `escalations` | cross-session scope 1, plan defect 1 | 0.25 a step. Both were smith 02's. | 3 | 3 | 12 | 8 | 194 |
| `escalation_wait_min` | 2.5 | 3 min 44 s and 1 min 54 s, from each block's heading to its ruling. | 10.5 | 1.2 | 2.3 | 1.9 | 2.7 |
| `blocked_h` | 0.1 | | 0.6 | 0.1 | 1.0 | 0.4 | 13.9 |
| `steers` | 2 sent, 0 points refused | 10:26:53 (waits; test shape), 10:56:42 (ruling 104). | 1, 0 | 4, 0 | 10, 1 | 2, 0 | 64, 18 |
| `tokens_coordinator` | 13.9M in (98% cached), 0.03M out | 1.73M a step. | 16.6M; 1.51M a step | 22.4M; 0.77M | 82.6M; 4.13M | 41.6M; 1.16M | 947.0M; 5.96M |
| `tokens_implementers` | 61.4M in (98%), 0.18M out | 7.67M a step: 1.6 times forgejo-17's and client-composition's. | 51.1M; 4.65M | 68.0M; 2.35M | 143.4M; 7.17M | 168.4M; 4.68M | 3,349.8M; 21.1M |
| `tokens_claude` | 1,355.3M in (98%), 5.88M out; 1,313.0M before the claim, 34.8M in the run, 7.5M after | Most of the input before the claim is other designs (below). | | | | | 2,284.3M |
| `cost_claude` | $491: designer $424, planner $53, supervisor $13, retrospective $1; cache reads $249, writes $136, output $106 | $67 without the designer. | $174: planner 78, supervisor 37, designer 18, retrospective 41 | $54: supervisor 23, retrospective 31 | $233: planner 44, supervisor 95, designer 18, retrospective 76 | $132: planner 55, supervisor 37, designer 9, retrospective 31 | $795: supervisor 604, designer 189, retrospective 1 |
| `supervisor` | 1 session, 2.5 h, 0.9M weighted tokens an hour, 52% in the 40 requests after each start or compaction, 1 compaction | $1.63 a step. | | 1, 3.8 h, 0.7M, 36%, 1 | | | 36, 65.0 h, 1.4M, 36%, 25 |
| `tokens_per_step` | 13.8M | Coordinator 1.73M, implementers 7.67M, the run's Claude Code sessions 4.35M. | 19.1M | 5.4M | 23.6M | 8.8M | 41.5M |
| `coordinator_requests` | 150, 17 after a wait ran out | 18.8 a step. 17 of its 34 waits ran out at 300 s. | 162, 9; 14.7 a step | 188, 3; 6.5 | 461, 26; 23.1 | 292, 12; 8.1 | 4,655, 325; 29.3 |
| `compactions` | 0 | | 0 | 0 | 1 | 0 | 10 |
| `gates_per_step` | 1.2 | 10 land gates, all passed. skein-01.1 took three, because `main` moved under each of them. | 1.4 | 2.1 | 1.2 | 1.3 | 1.8 |
| `gate_lane_other` | 8 | | | | | | 37 |
| `queue_wait_ratio` | 0.18 | 1,219 s waiting against 6,661 s running. 96% of the waiting fell between 09:00 and 10:00, 985 s of it behind gates. | 0.29 | 0.83 | 0.97 (0.17 true) | 0.06 | 0.94 |
| `suites` | **skein:** focused 3,116 / 6.885 s → 3,139 / 6.865 s; fuzzy 104 / 29.583 s → 104 / 29.495 s; third suite 38 / 23.286 s → 38 / 22.288 s; gate 70 s → 83 s. **smith:** focused 2,358 / 10.863 s → 2,388 / 11.659 s; fuzzy 43 / 10.899 s → 43 / 10.987 s; gate 27 s → 29 s. | smith's focused suite grew 7% in time for 1.3% more tests. | | | | | |
| `proofs` | world 4, joint 4; 0 below plan | The repin's commit, `03a9d5f5`, names no test. | | | world 3, process 5, live 7 | unit 13, world 18, joint 3; 1 below plan | |
| `report_lines` | 12 median, 14 p90 | 14 reports for 8 steps. | 12, 15 | 11, 11 | 11, 18 | 10, 10 | 12, 18 |
| `lines` | code +1,111 −175; tests +2,256 −307; generated +581 −19; data +72 −28 | | | | | | |
| `contexts` | 146K median, 185K p90; 5 handovers | Handovers came at 110K to 181K, against implementer.md's 100K. | 113K, 182K; 2 | 113K, 160K; 3 | 101K, 199K; 7 | 189K, 241K; 5 | 186K, 332K; 150 |
| `pin_lag` | smith: skein 8; jig: skein 27, smith 18; hearth: skein 26; temper: skein 24, smith 21, jig 5 | jig and temper were not repinned, as planned. | | | | | |

**Not comparable, and why:**
- **`cost_claude`'s designer, and `tokens_claude` before the claim.** These count three sessions that designed other things:
  - `b357b336`, smith's accounts design, $111.49;
  - `78210e52`, a jig audit and the grant and memory design, $145.81;
  - `81770ae7`, temper's clean-up, $166.75.

  The metric's window runs from smith's previous plan landing (wall-clock's `0c26d801`, 10-16) to this one. It takes the `Designer-Session` trailer of every commit in it that changes an Inputs design path. The accounts plan landed after this one with no design commit between them, so its design falls in live-text's window. None of the counted dollars designed live-text.

  The two sessions named as missing, `5e38559b` and `ffc2b313`, have no transcript here. The live-text design is likely one of them, which is an inference. Two relatives of the counted sessions are not counted either: `fb0f90af`'s sub-agents ($59.78), the ancestor of `78210e52`, and `74881b0e` ($8.25), which continues `81770ae7`. Both did the same other work. The run's own part, `tokens_per_step`, is unaffected (proposal 5).
- **`supervisor`'s 52%.** The two 40-request windows after the session's start and its one compaction held 80 requests and $3.87 of its $6.77. In a 2.5-hour session, those windows are half the session by construction.
- **`queue_wait_ratio`** counts time in the queue only. A gate also drains the build lane before it starts; the first skein gate waited out 205 s of smith-bench's build. Counting the drains, the ratio is about 0.29.

**What explains the largest differences:**
- **Hours a step, 0.27.**
  - **The chain.** smith's five steps ran one after another, each in a fresh thread:

    | Step | Slot | Thread started | Landed | Minutes |
    |---|---|---|---|---|
    | smith-01.1 | w1 | 09:08:14 | 09:35:22 | 27.1 |
    | smith-01.2 | w3 | 09:36:04 | 10:02:42 | 26.6 |
    | smith-01.3 | w6 | 10:04:09 | 10:29:52 | 25.7 |
    | smith-02.1 | w7 | 10:30:00 | 10:56:58 | 27.0 |
    | smith-02.2 | w8 | 10:57:04 | 11:15:06 | 18.0 |
    | skein-01.1 | w2 | 09:08:20 | 09:35:07 | 26.8 |
    | skein-01.2 | w4 | 09:36:10 | 09:48:17 | 12.1 |
    | skein-01.3 | w5 | 09:48:51 | 09:56:58 | 8.1 |

    "Thread started" is the time in the thread's id.
  - **Beside the chain.** skein's three steps landed by 09:57. After that, one session ran at a time.
  - **Within a step.** The first edit came at a median of 1 min 44 s after the thread started, and the first commit at 8 min. The rest went to the implementer's own runs, its waits and the land gate. 48 of the implementers' 92 heavy tasks failed (1,251 s).
- **Implementers' tokens, 7.67M a step.**
  - **Eight threads for eight steps.** Each read the card, the README, its session file and the design before working. Their median context was 49K at request 10, 85K at request 20 and 102K at request 30. 360 of their 580 requests were at or above 100K.
  - **Waits.** 133 requests only waited on a running command, 27.4% of their input (section 5, item 4).
- **The coordinator's 18.8 requests a step.**
  - Rings took 5 requests, against client-api's 6 to 7.
  - The heartbeat ran in 37 requests, and kache's gc in 23.
  - 17 waits ran out.
  - Python heredocs were 29.5% of its output, and rewrites of `context.md` 42%.
  - It was idle from the last landing, 11:15:09, until the release.
- **The supervisor's $13** was its main thread, $6.77, and five Sonnet sub-agents, $6.28: the ready check $2.07, the hourly check $1.63, the re-check $2.05, "Done when" $0.38 and the re-measure $0.15.

## 4. What worked

1. **The plan's aim was met.**
   - 8 of 8 steps landed, none added or dropped.
   - 9 of the 10 "Done when" items passed as written, and item 8 passed when measured with its logs outside the tree.
   - All 10 land gates passed. The two failures before them were a prebuild's clippy wildcard and the land refusing the Pulse word "actually"; both were fixed before the gate.
2. **Blocks were answered fast, and nobody waited on them.**
   - The two blocks waited 3 min 44 s and 1 min 54 s.
   - The implementers kept working through both. The traces show no idle time, and 75 to 85% of each block's latency was the supervisor's.
3. **The ready check and the re-check corrected smith 02 before its work.**
   - Ruling 101 came 25 minutes after the claim, an hour before smith 02 started.
   - Ruling 104's re-check ran beside smith-02.1, since a re-check never holds a start. Its corrections, all for 02.2, reached w8 by steer at 10:56:42, as its thread started.
4. **The coordinator was cheap.**
   - Rings took 5 requests, with `plan steered` and `plan unblocked`.
   - It never compacted, and refused no steer point.
   - It used 1.73M tokens a step, against client-api's 5.96M.
5. **The live smokes** ran once for each of smith's five landings. All passed first time, for $0.0034 against the README's $0.06.
6. **The queue held.** The ratio was 0.18. 96% of the waiting fell in the first hour, while the five skein gates held the lane for 853 s.
7. **The supervisor's five Sonnet sub-agents** cost $6.28 for the ready check, the hourly check, the re-check, "Done when" and the re-measure.
8. **The code kept the rules that a gate checks.**
   - No lint exception, Pulse word, banned type or TODO landed.
   - Every Proof was at its plan's layer: world 4, joint 4.
   - The downstream check at 10:15 to 10:20 found only the plan's documented breaks: smith's two A30 sites, which smith-02.1 repins, and jig and temper, which wait for later plans. hearth built.
9. **The end was clean.**
   - `done.md` came with the last landing.
   - The release came 21 minutes later, after "Done when".
   - Both `STATUS.md` files were marked done, smith `7b4dfc07` and skein `2912d92`.

## 5. What did not

1. **The chain took four times its price.**
   - **What happened.** The README's longest chain is "5 steps ... at 0.09 to 0.14 h a step, about 0.5 to 0.7 h". The chain took 2.11 h, from 09:08:14 to 11:15:06, and its steps took 25.7 to 27.1 minutes each, the last 18.0.
   - **Cause: the README form.** development.md (line 814) prices the chain at the last runs' `step_h`. `step_h` is running hours divided by every landed step. In kit-batch, render and the other runs it cites, many sessions ran at once, so a step's own span was several times its `step_h`. A chain step's span holds:
     - a fresh thread's reading, with the first commit at a median of 8 minutes;
     - the implementer's own runs;
     - the land gate.
   - **First visible:** 09:35, when the first chain step landed, 27 minutes after it started. The 10:14 check wrote "chain steps take 27-28 min against 5-8 min planned; end projected near 11:30. No action."
   - **Acted on:** no; nothing in the run could shorten the chain.
   - Proposal 6.
2. **smith 02's session file was corrected four times.**
   - **The rulings:**
     - **101** (09:33:49), from the ready check against skein `12305e5` and smith `c0c89ddc`:
       - `Time` has no `Add` or `Sub`;
       - `profile/tests.rs` belongs in Files;
       - `Event::Call` gains `call:` at the worlds;
       - `release_after` and `maximum_queries`.
     - **103** (10:36:44), from a block of 10:33:00 that was a cross-session scope and also a plan defect: `service::Limits` gains `live_interval`, which `profile::derive` sets in `profile.rs`, a file outside the step's Files.
     - **104** (10:56:28), from the re-check against skein `5c8ae96` and smith `9da8ce13`: many Shape corrections for 02.2.
     - **105** (11:01:07), from a block of 10:59:13, a plan defect: a one-frame channel can never get a piece, because `takes_live` needs `frames > fact_reserve_frames + 1`. The fix was a no-room table world and `Service::lost_live_bytes()`.
   - **Causes:**
     - **The sketches' stand-ins.** The plan's sketches compiled in one scratch crate, "with `todo!()` bodies and stand-ins for the types each crate has now" (`findings/planning-commands.md`, 8). A stand-in `Time` that adds hid ruling 101's fact. A stand-in `Limits` hid ruling 103's: the real `profile::derive` builds `service::Limits` with a struct literal, and would not have compiled once the struct had the new field.
     - **The planner's session.** The planner ran from 06:40 to 08:59 for $52.90, and compacted 16 times, every 8 to 9 minutes. It re-read development.md 51 times, and channel.md, llm.md, limits.md and fake-llm.md 24 to 39 times each. From 07:22 to 08:05 it said about seven times that it was writing the session files now. It wrote `smith/02-best-effort.md` at 08:19, three minutes after a compaction.
     - **Behaviour.** Ruling 105's fact is behaviour, which no compile finds. Ruling 104's corrections follow smith 01's landings, as a re-check is meant to.
   - **First visible:** 09:33, ruling 101, before smith 02 started.
   - **Acted on:** each block was answered within 4 minutes. Rulings 101 and 104 came before the increments they corrected.
   - Proposal 8.
3. **"Done when" item 8 failed as written, and the release waited 21 minutes.**
   - **What happened.**
     - The command writes `focused-$i.log` and `fuzzy-$i.log` into the worktree it measures. smith-bench's test `a_replay_arm_runs_through_the_probe_command_and_prints_one_line` builds an arm from that tree, and refuses a dirty one ("source repository is dirty; commit or isolate changes before building an arm", `benchmarks/src/arm.rs:97`). So each focused run stopped after about 15 of 2,388 tests.
     - The `sh -c` loop's last command was a fuzzy run that passed, so the task exited 0 (`heavy.tsv` row 160).
   - **The planner had met it.** `findings/planning-commands.md` 7 records the same refusal at the base, from an uncommitted `Cargo.lock`. It concluded that the repin's commit would come before the gate, and did not apply that to the measurement's own logs.
   - **The run met it more than once.**
     - Six of the implementers' heavy runs were refused for a dirty tree, 536 s in all.
     - The re-check's scratch run counted 2,381 of 2,382 passing.
   - **A wait that never ended.** The "Done when" sub-agent waited with a `pgrep -f` loop that never ended, from 11:19 to 11:29.
   - **First visible:** at the base, 08:08 (log `20261019T080805-28.log`). In the run, at the first refused implementer run.
   - **Acted on:** the re-measure at 11:36:17, in a clean worktree with the logs outside it, and ruling 106 at 11:36:14.
   - Proposal 2. The test itself is section 9, smith item 1.
4. **The proposals adopted from client-api did not reach the run, and the implementers waited 31 seconds at a time.**
   - **What happened.**
     - client-api's section 10 adopted nine proposals at 01:43 on 10-19, to be applied by a process designer.
     - At the claim, 7.4 hours later, none was applied: the kit was `6df8817`, with 10-18's process.
     - Section 10 gave proposal 3's wait to the tools plan's README "from its first step". It did not name live-text, which was being planned that morning from 06:40 to 08:59. Its README has no wait.
   - **The effect.**
     - 133 of the implementers' 580 requests (22.9%) only waited on a running command, 27.4% of their input.
     - w2 to w5 and w8, without `// @exec: {"yield_time_ms": 300000}`, spent 36.2% of their input on these requests. w6 and w7, with it, spent 8.7%.
     - w2 waited through land's three gates in 31 wait-only requests, 3.65M tokens.
     - implementer.md 8.3's literal instruction, "`wait` on the command's cell with `yield_time_ms: 300000`", cost w1 three requests a command.
   - **First visible:** at the claim. The 10:14 check counted 46 short waits.
   - **Acted on.**
     - Steer point 1 at 10:26:53 went to w6, which was already using `// @exec`.
     - `done.md` says the later threads read implementer.md 8.3, which still names the wait that returns at 31 s.
     - w7 used `// @exec`. w8 never did.
   - Proposal 1.
5. **The implementers had no code graph.**
   - **What happened.**
     - All 14 reports say the graph was unavailable.
     - Each thread read the graph's `SKILL.md` anyway, as implementer.md (lines 74 to 82) tells it to use the graph.
     - The 10:14 check re-indexed skein and smith, which no implementer could reach.
   - **Cause.** skein's `scripts/dev/workspace/coordinator.config.toml` (lines 15 to 17) has `[mcp_servers.codebase-memory-mcp]` `enabled = false`, under the comment "The coordinator reads reports and git, and never reads code (section 2.3)". It came with `37c676e` (01:36 on 10-18), "Pass the coordinator profile settings into its Codex thread".
   - **Why it reached the implementers (an inference).** The threads the coordinator spawns carry its profile. No document says so, but all eight reported no server, while `~/.codex/config.toml` enables it.
   - **First visible:** the first reports, at 09:35.
   - **Acted on:** no.
   - Proposal 3.
6. **The test shape needed a steer again.**
   - **What happened.**
     - The 10:14 check found smith-01.2's `tests/channel/tests/live.rs` breaking four of testing-strategy.md section 10's rules, and steered (point 2).
     - Ruling 102 made section 10 README rule 7, ahead of any reference test, for the tests still to come. Landed tests stay as they are (section 9).
     - skein's `fake-llm-domain/src/tests.rs` had one test asserting four behaviours. It was not steered, since skein 01 was done.
   - **Cause.** development.md (section 5) gives implementers a reference test to copy, written out in full, for each new test file. The plan's reference tests used:
     - bare `10_000` and `16_384`;
     - three behaviours in one test;
     - a name with "or".

     The planner's review prompt asked only to flag text that restated section 10.
   - **It recurs.** wall-clock, render, kit-batch and forge-setup each met the test shape.
   - **First visible:** at skein-01.1's landing, 09:35, and smith-01.2's, 10:02.
   - **Acted on:** 10:26:53. By then six of the eight steps had landed or were running.
   - Proposal 4.
7. **The design's sessions were counted from the wrong window.**
   - **What happened.** `cost_claude` charges $424 to the designer, all of it from three sessions that designed other things (section 3).
   - **Cause.** The window is set by file paths and the previous plan's landing, not by this plan's design commits.
   - **First visible:** at this retrospective's `plan metrics`.
   - Proposal 5.
8. **Four of the pulse's signals were false.** The supervisor recorded them at 10:14 "for the retrospective":
   - **The aims verdict.** "Past the aims" compares the whole suite's time with an aim that is a package sum.
     - It flagged all three skein landings, whose package sums were 4.51 to 4.92 s against 5.7.
     - development.md (lines 850 to 862) already says the hourly check and "Done when" sum the packages.
     - forge-setup's proposal 6 left its kit part deferred.
   - **`words_at`** (`numbers.sh`, line 463) greps binary data without `-a`, as wall-clock and kit-batch also met.
   - **`testing/` fakes** are counted as tests.
   - **The helper signal** was a false positive.
   - **Cost:** the supervisor checked each by hand; none led to a wrong steer.
   - Proposal 7 for the aims; the others are in section 9, skein item 4.
9. **skein-01.1 took three gates.**
   - **What happened.** forge-kit's run moved skein's `main` under each gate, and `land` gated again. That cost 654 s of gate time and 181 s of prebuilds, and w2's 31 wait-only requests.
   - **The plan knew.** The README named that run, and that nothing here builds on its crates.
   - No proposal: `land` gates again by design when `main` moves, and client-api met the same.
10. **Disk holds, and one stop.**
    - The disk held new starts below 40 GB eight times.
    - It stopped the queue at 14 GB at 10:52:42. kache's gc brought it to 31 GB by 10:52:58, and the queue resumed at 10:53:05.
    - The stop came while the supervisor's re-check (10:31 to 10:53) was building in its scratch worktree, an inference from the timing.
    - The coordinator's trace shows no start delayed by a hold.
    - No proposal.
11. **Ruling 104 and `later.md` say what the code does not.**
    - **The ruling and the row.** Ruling 104 says "the two release stories call no check_memory", and `later.md`'s `Peer::release` row says the same.
    - **The code.** smith-02.2 landed release stories that call `check_memory` (`tests/agent-process/tests/live.rs:20`). They use a fixed 64-byte `ReleaseName` (`tests/agent-process/src/world.rs:356`), so the release allocates nothing.
    - **So the row is stale.** The debt that remains is skein's: `Peer::release` allocates a `Box` (section 9, skein item 2).
    - **First visible:** here.
    - **Acted on:** no.
12. **The Sizes.**
    - **By the plan's command,** five of the eight steps went over their Size:
      - skein: 693 lines against 510 (1.36 times);
      - smith: 1,111 against 710 (1.56 times), or 1.12 times without inline tests.
    - **The largest** was smith-01.3: 520 against 230, of which 190 were an inline `#[cfg(test)]` module.
    - **No ruling was needed:** the check found the Sizes "within 2 times", and render's proposal 5 lets such an increment land.
    - No new proposal.
13. **Small kit defects met by the supervisor.**
    - `plan pulse --help` runs the pulse.
    - `land --docs` refuses a detached worktree.
    - A relative path given to `git worktree add` made a stray directory.
    - `kit/` is removed at the release, so `plan retro` ran from `~/.local/bin`.

    Section 9, skein item 4.

## 6. Decisions taken during the run

Six rulings, 101 to 106, each one smith commit, and two steers.

| Ruling | Commit | What | Reached the design? |
|---|---|---|---|
| 101 | `7eaba5ab`, 09:33:49 | smith 02's ready check: `Time`'s missing operators, `profile/tests.rs` in Files, `call:` at the worlds, `release_after` and `maximum_queries` | Plan only; no design text needed |
| 102 | `e225df8a`, 10:26:55 | README rule 7: testing-strategy.md section 10 before a reference test; landed tests stay | Plan only; section 10 already says it |
| 103 | `e2b9f45a`, 10:36:44 | `service::Limits` carries `live_interval` from `profile::derive` into `Live::new`, apart from the repin's commit | The interval was already limits.md 2.3's; its carrier is code |
| 104 | `544ebe44`, 10:56:28 | The re-check's Shape corrections for 02.2, with a `later.md` row | Held bytes waiting for room were already channel.md 7's. The rest is code. The `later.md` row is stale (section 5, item 11) |
| 105 | `b1df8148`, 11:01:07 | The one-frame world checks loss at the held bound and one past it; `Service::lost_live_bytes()` | The count was already channel.md 7's; the accessor is code |
| 106 | `fabc3116`, 11:36:14 | Item 8's miss as written, and the clean re-measure | Plan only |

Both steers are in the table: steer 1 (10:26:53) carried point 1, the waits, and point 2, which ruling 102 recorded; steer 2 (10:56:42) carried ruling 104. Point 1 belongs in implementer.md 8.3, which it did not reach (proposal 1).

**"For the designer"** (`decisions.md`): five items the plan asked the designer to write. **None reached the design.** smith has no design commit after the plan's `0e9c3308`, except STATUS rows (`c0c89ddc`, `7b4dfc07`). skein's `fake-llm.md` has none after `d4ef168` (07:39).
1. Reword `fake-llm.md` 2.2 and 6: the release comes from the world or the loop, and a reply scripted whole keeps its bytes. 2.2 still says "The peer as a process of its own ... refuses a script with one at its start" (lines 97 to 100) and "A reply scripted whole is one piece" (line 80). Both now disagree with the code.
2. `events.md`: a field for the live bytes not projected.
3. llm.md 6: "main conversation only", read as the channel's rule.
4. shell.md 6.5 and `live_interval`.
5. `fake-llm.md` 2.2: no fault while a reply is held.

**The design questions and "For the user".**
- The five design questions were answered (a) by the director before the plan landed.
- The ten "For the user" items were all accepted by the director at 09:02:52 (`2efd6c92`), before the claim.

## 7. The proposals adopted after the last retrospective

The last retrospective was client-api's. Its section 10 adopted all nine proposals at 01:43 on 10-19, each "to apply". None was applied when this run was claimed at 09:07, or when it was released at 11:36. So none of the nine was in force; the table says what this run's numbers show anyway.

| Proposal | Meant to move | client-api | live-text | Moved |
|---|---|---|---|---|
| 1. Long plans run as consecutive plans | plan-defect escalations a step; rulings a step; `step_h` | 0.47; 1.9; 0.38 | 0.13; 0.75; 0.27 | Not tested: a plan of 8 steps and a chain of 5 |
| 2. The rules every implementer reads stay under about 300 lines | implementers' context at request 30; their tokens a step; steer points refused | 96K; 21.1M; 18 | 102K; 7.67M; 0. The README's rules were 66 lines, but every thread read implementer.md's 441 | No, at request 30 |
| 3. The pulse counts a wait by when it returned, and implementer.md names `// @exec` | the share of implementers' requests that only poll | 28.5% | 22.9% of requests, 27.4% of input | A little, by the steer and the threads' own `// @exec`, not by the rule |
| 4. A kit fix for a defect the run meets moves `kit/` | the longest check gap | 4.2 h | 1.1 h and 1.0 h, around one check; no kit fix was met | Not tested |
| 5. A ring is one command | coordinator requests a step; its tokens a step | 29.3; 5.96M | 18.8; 1.73M. Rings took 5 requests | Yes, through `plan steered` and `plan unblocked` in the kit at the claim |
| 6. No sub-agent or woken designer reads whole files | supervisor sub-agent dollars a step | $2.13 | $0.79. No designer was woken | Yes, by the run's size. The sub-agents still read the card and testing-strategy.md whole |
| 7. No Proof lands "unavailable" | hours a regression goes unseen | 27 | No process tier; no Proof "unavailable" | Not tested |
| 8. btrfs unallocated space is a resource | infrastructure escalations | 6 | ext4; 0 | Not tested |
| 9. `plan metrics` maps kinds and words | unmapped kinds and words; misread landings | 4 kinds, 4 words; 1 landing | None unmapped; the designer window is a new misreading (proposal 5) | Not tested |

**The kept commit and the changes during the run.**
- **Kept.** The run kept `6df8817`, as `supervision.md`'s Process line says. Its process is that of `49e7eb6` and `b4c6241`, both on 10-18.
- **Landed during the run.** No change to development.md, its cards or the kit landed between the claim and the release. The 10:14 check confirmed it against skein `5c8ae96`.
- **After the release.** `92e177c` landed at 11:38:07, after the release, and is not this run's.

**What this says of the loop.** The decisions were taken 7.4 hours before the claim, and the designer's batch had not landed. One decision named the plans it reached, the tools plan and the forge kit plan, and not this one. A decision recorded as "adopted, to apply" holds back nothing, so the run inherited the defect that proposal 3 described (proposal 1).

## 8. Proposals

1. **No run is claimed while the last retrospective has an adopted proposal still to apply, unless the plan's README says how this run applies it.**
   - **Evidence:** section 5, item 4; section 7.
     - Nine proposals were adopted at 01:43, and none was applied at the 09:07 claim.
     - Only the tools plan was given proposal 3's wait.
     - This run's implementers spent 27.4% of their input waiting, and those without `// @exec` spent 36.2%.
     - `plan check` refuses only an *open* proposal (development.md 15.1), so "adopted, to apply" passed it.
   - **Changes:** development.md 15.1 (the designer applies what is adopted before the next claim) and 6 (`plan check`); `plan check`.
   - **Moves:**
     - adopted proposals unapplied at a claim, 9 to 0;
     - the implementers' share of input in wait-only requests, 27.4% to under 10%.
2. **No command a plan gives writes inside the worktree it builds or measures, and a loop of runs fails when any run in it fails.**
   - **Evidence:** section 5, item 3.
     - Item 8's logs dirtied the tree it measured, and smith-bench refused it.
     - The task exited 0 because the loop's last run passed.
     - The planner met the same refusal at the base (`findings/planning-commands.md` 7).
     - The re-measure and the `pgrep` wait held the release for 21 minutes.
   - **Changes:** development.md, section 5: "Done when", and the aims' measurement (lines 850 to 862 and 1368), whose form should write its logs under the run's directory.
   - **Moves:**
     - "Done when" items that fail as written, 1 to 0;
     - minutes from `done.md` to the release, 21 to under 10.
3. **Implementers' threads have the code graph that implementer.md tells them to use, with the coordinator's profile turning it off for the coordinator alone; where no graph runs, implementer.md says so.**
   - **Evidence:** section 5, item 5.
     - `coordinator.config.toml`'s `enabled = false`, from `37c676e`.
     - 14 of 14 reports say the graph was unavailable.
     - Every thread read `SKILL.md` for nothing.
     - The 10:14 check re-indexed the graph for nobody.
   - **Changes:** skein's `scripts/dev/workspace/coordinator.config.toml`, or how `plan` starts the implementers' threads; development.md 2.3; implementer.md, lines 74 to 82.
   - **Moves:** reports saying the graph was unavailable, 14 to 0.
4. **The planner's review checks every reference test against testing-strategy.md section 10, rule by rule, before the plan lands.**
   - **Evidence:** section 5, item 6.
     - The reference tests had bare literals, three behaviours in one test, and a name with "or".
     - The review prompt asked only about restating section 10.
     - Steer point 2 and ruling 102 came after six steps had started.
     - Four earlier runs met the test shape.
   - **Changes:** development.md, section 5 (the reference tests, and the review of a plan).
   - **Moves:**
     - test-shape steers and rulings, 1 to 0;
     - landed tests breaking section 10 (section 9, smith item 4), down.
5. **The README's Inputs name the design's commits, and `plan metrics` counts as the design's sessions only those commits' `Designer-Session` trailers.**
   - **Evidence:** section 3, "Not comparable", and section 5, item 7.
     - $424 from three sessions that designed the accounts plan, a jig audit and temper's clean-up.
     - The live-text design itself is likely uncounted.
   - **Changes:** development.md 5 (the README's Inputs) and 15.2 (`tokens_claude`); the designer sessions in `numbers.sh`.
   - **Moves:** designer dollars from sessions that designed something else, $424 to $0.
6. **The README prices its longest chain at the last runs' median span of a step, from its thread's start to its landing, not at `step_h`.**
   - **Evidence:** section 5, item 1.
     - The plan said 0.5 to 0.7 h; the chain took 2.1 h.
     - Chain steps spanned 25.7 to 27.1 minutes against the 5 to 8 that `step_h` implied.
     - The 10:14 check saw the gap and could do nothing.
   - **Changes:**
     - development.md 5, the README form (line 814);
     - development.md 15.2, a new metric: the median and the longest span of a step, from its thread's start to its landing;
     - `plan metrics`.
   - **Moves:** the chain's planned hours against its actual, from about a quarter to within 25%.
7. **The pulse's "past the aims" sums the `aims` line's packages from the gate's log, as the hourly check and "Done when" do.**
   - **Evidence:** section 5, item 8.
     - Three of three skein verdicts were false: package sums of 4.51 to 4.92 s against 5.7.
     - It is forge-setup's proposal 6's kit part, still deferred.
   - **Changes:** the pulse (`numbers.sh`); development.md 14.2.
   - **Moves:** false aims verdicts, 3 to 0.
8. **A plan's interface sketches compile against the base's own crates, with stand-ins only for what the run adds.**
   - **Evidence:** section 5, item 2.
     - Ruling 101's `Time` and ruling 103's `profile::derive` were facts of the base.
     - The sketches' stand-ins hid both (`findings/planning-commands.md` 8). The real `Limits` literal in `profile.rs` would not have compiled.
     - Both corrected the same session file, which was corrected four times in all.
   - **Changes:** development.md, section 5 (the plan's premises, where parity's proposal 10 put the compiled sketches).
   - **Moves:** rulings that correct a session file with a fact of the base, 2 to 0.

## 9. Debts left in the code

For smith's `docs/development/tech-debt.md`, which lists none of these, and for skein, which has no such file yet. They are ordered by what they cost the next plans.

**smith**

1. **smith-bench's probe test fails in any dirty checkout.**
   - **Where:** `benchmarks/tests/probe_command.rs:105` builds an arm from the real checkout, and `benchmarks/src/arm.rs:93-98` and `probe.rs:593` refuse a dirty tree.
   - **What it cost:**
     - "Done when" item 8;
     - six of the implementers' runs, 536 s;
     - the re-check's 2,381 of 2,382;
     - the planner's run at the base.
   - **Since:** the guard dates from `98163c75`.
   - **Needs:** the test builds from a synthetic repository, as `arm.rs`'s unit test at lines 408 to 430 already does.
2. **The memory headroom fell 20%.**
   - **How much:** above the 490,000,000-byte floor, from about 2.82 MB to 2.26 MB.
   - **Cause:** `HeldByte { byte, arrived: Time }` keeps one record for each held byte, and there is a `Gap` queue beside it (`crates/smith-agent-service/src/live.rs:60-75`, `94`).
   - **Risk:** the next plan that adds to the worst case meets the floor.
3. **Repeated test code and bare literals.**
   - smith's tests have six identical `other @ (Request::Piece { .. } | Request::Hold { .. }) => panic!` arms.
   - The 64-byte hold bound is a bare literal at five sites.
   - The frame kinds `0x0109` and `0x0112` have no named constant.
4. **Landed tests that break testing-strategy.md section 10.** Ruling 102 let them stay.
   - `tests/channel/tests/live.rs`: repeated setup, two behaviours in a test, bare indices at lines 61, 72 and 82, and `sent > 0 && sent < 8`.
   - `crates/smith-agent-service/src/live.rs:335` asserts three behaviours, and `AT_ZERO.saturating_add(...)` appears eight times.
   - `tests/agent-process/tests/live.rs`, at lines 25, 54, 66 and 158.
   - `tests/channel/tests/referee.rs:86`.
   - `content_observations.rs:90`.
5. **Records with no producer.** `LiveKind::Reasoning` and `LiveKind::Input` have no producer. Only tests read `lost_live_bytes`.
6. **Taste.**
   - `ends: [Option<(u32, u64)>; 3]` is indexed by bare 0, 1 and 2 (`crates/smith-host-protocol/src/live.rs:30-61`).
   - `[0; 4096]` repeats `PIECE`'s value (`crates/smith-agent-service/src/live.rs:221`).
7. **`later.md` and `STATUS.md`.**
   - The `Peer::release` row is stale (section 5, item 11).
   - The repin row leaves out jig's four paths from v2 to v3, and temper's `encode_start` (audit A32).
   - temper's `STATUS.md` (lines 124 to 128) still says it "waits for the live-text plan".
8. **Timeouts under load.** Three test runs (`heavy.tsv` rows 82, 83 and 125) hit nextest's global 15 s timeout while the downstream checks loaded the machine.
9. **Files past 800 lines grew.** The agent process's test world `tests/agent-process/src/world.rs` went from 1,433 to 1,586 lines, and `crates/smith-agent-service/src/service.rs` from 2,920 to 2,968. Needs the consolidation pass (code-organization.md, 7).
10. **Not code.** Local `live-text/*` branches remain in smith.

**skein**

1. **The shared fake LLM fixtures grew.**
   - `tests/llm/tests/fake.rs` went from 1,210 to 1,365 lines; `40d85476` alone added 121, against an aim of 100.
   - The `Line::Pieces` literal is copied at three sites.
   - smith's `tests/hosts/tests/simulation.rs` repeats the constants.
2. **`Peer::release` allocates a `Box`.** smith's stories call `check_memory` only because they keep the release's name in a fixed buffer.
3. **Landed tests that break section 10.**
   - `testing/skein-fake-llm-domain/src/tests.rs:121`: one test asserts four behaviours.
   - The `use` lines at 58 to 60 are out of place.
   - `tests/llm/tests/fake.rs` lines 1293, 1308 and 1348.
4. **The kit.**
   - The pulse's aims verdict (proposal 7).
   - `words_at` (`numbers.sh`, line 463) greps binary data without `-a`.
   - `testing/` fakes count as tests.
   - The helper signal's false positive.
   - `plan pulse --help` runs the pulse.
   - `land --docs` refuses a detached worktree.
   - `kit/` is gone after the release, so `plan retro` takes whatever `~/.local/bin` holds.
5. **Not code.** Local `live-text/*` branches remain in skein.

## 10. Decisions

| Proposal | Decision | Where |
|---|---|---|
| 1 | adopted: client-api's proposals 2 to 9 are the first it holds back, so real-loop and browser-1 are claimed once they are applied | skein 1d105bc: development.md 15.1 (no claim while an adopted proposal is unapplied); `plan check`'s refusal, with section 6: skein's kit, to land |
| 2 | adopted | skein 1d105bc: development.md 5.2 ("Done when"), 5.4 (the literal reading) and 5.6 |
| 3 | adopted: the coordinator alone runs without the graph, as 37c676e meant | skein 1d105bc: implementer.md 8.1 (a thread without graph tools); the implementers' thread config, with 2.3, 7.1 and 7.2: skein's kit, to land |
| 4 | adopted | skein 1d105bc: development.md 5.4 (the literal reading checks each shown test against testing-strategy.md's section 10) |
| 5 | adopted | skein 1d105bc: development.md 5.2 (the Inputs name the design's commits); `plan metrics`' count, with 4.4, 15.1 and 15.2: skein's kit, to land |
| 6 | adopted, with the tools retrospective's proposal 5 (one metric) | skein 1d105bc: development.md 5.2 and 15.2, `chain_step_min` from a step's first thread and `chain_step_max`, one metric with the tools retrospective's proposal 5; `plan metrics`: skein's kit, to land |
| 7 | adopted | skein's kit, to land: the pulse, with development.md 5.2 and 14.2 |
| 8 | adopted | skein 1d105bc: development.md 5.2 (interface sketches) and 5.4 (the plan's commands) |
