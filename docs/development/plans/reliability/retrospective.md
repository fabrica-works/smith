# Retrospective: reliability

2026-10-14. The second run of smith's `reliability` plan, on
debian-16gb-hel1-1: claimed at 2026-10-12T13:18:52Z, `done.md` recorded at
2026-10-14T01:14:15Z. It is the plan's first run under development.md;
its first run (2026-10-09 to 2026-10-12) used an older process.

Written by a fresh session that took no part in the run (development.md,
section 15). Nine read-only sub-agents read the record, one per source:
- the coordinator's Codex rollout;
- the implementers' rollouts, in two halves (w1–w35, w36–w69);
- the supervisor's Claude Code transcripts, with `supervision.md`;
- the designer's transcripts;
- `log.md` with the ledger;
- the git history of smith and skein;
- the landed code;
- `decisions.md`, with the first run's record.

`logs/heavy.tsv`, the gate logs and `plan metrics` were read directly, and
the sessions' `rg` and `git` "Done when" commands were run once. Times are
UTC. This document proposes; the user decides, and the designer applies
what is adopted.

## 1. In one page

- **Done in 36 hours, with no step dropped.**
  - 108 steps landed: 71 planned at the start, and 37 added during the
    run (11 split off planned increments, 26 supplements `.s<k>`).
  - Each "Done when" item has its ledger steps. Two hold only in form:
    - item 3: four of session 13's seven `Calibrated:` lines say "none";
    - item 4: the smoke suite did not exit 0, with 4 passed and 2 failed,
      and an unnumbered ruling closed it.
- **The run went at two speeds.**
  - The first 9 hours landed 74 steps; the remaining 27 landed 34.
  - The tail was session 10: 28 increments in one session, mostly one
    thread at a time, with session 12's live suites behind it.
  - For 20 of the 36 hours, one implementer thread was running.
- **The escalation channel was fast, and it carried a lot.**
  - The supervisor answered 185 blocks (tls-cleanup had 12): a median of
    1.1 minutes from block to unblock, p90 3.4. A block was open for 7.7
    hours in all (21% of the run).
  - It made 133 numbered rulings and 144 plan landings, and 12 steers.
  - The designer answered 26 questions in a median of about 3 minutes,
    with 32 design commits.
- **The blocks came from the plan's shape.**
  - 95 of the 185 came from the plan's Files, Shape, Test shape or Size
    not matching the code. 38 rulings fix plan defects the port missed.
  - README rule 8, on edits outside Files, was widened 11 times, and
    blocks of its kind went on to the last day.
  - The Size lines came out 1.6 times too small overall, and 2.6 to 4.2
    times for session 10's chains. That gave 20 size blocks and 11 splits.
- **It cost about 3.0B input tokens, 27.6M per step, against about 10M in
  tls-cleanup.** The four roles:
  - the coordinator 540M;
  - the implementers 1,601M;
  - the supervisor 584M;
  - the designer 250M.

  The largest avoidable costs:
  - contexts that only grew:
    - the designer kept one session for 26 questions, and 93% of its
      input was its own history;
    - the supervisor stayed past half its window for 15 hours;
    - two implementer threads compacted 16 times between them without a
      handover;
  - waiting: about 18% of the implementers' input went to calls that only
    waited on a running command, and 17% of the coordinator's to wake-ups
    that found nothing;
  - start-up reading carried through every request, and the coordinator's
    work with the code graph.
- **Seven of `plan metrics`' numbers are wrong** (section 3):
  - `tokens_claude` reads 4.04B for about 833M;
  - `blocked_h` reads −67.8 for 7.7;
  - the escalation, steer, restart and step counts misread the log;
  - the lines count benchmark data as code.
- **What held:**
  - every code landing went through `land` with its gate evidence;
  - no part of a gate was dropped, and both suites stayed within budget;
  - clippy in the build lane cut the clippy share of failed gates from
    5 of 9 to 13 of 70;
  - 21 landings kept their gate across Markdown commits;
  - times were read from the clock;
  - the designer did not watch;
  - standing rules came within minutes of a block's second occurrence;
  - the coordinator invented no step;
  - live spend was $17.60 against a $95 budget.
- **The main proposals** (section 8):
  - an implementer splits an increment that outgrows its Size line, without
    asking;
  - no session of more than eight increments;
  - the planner's reviews leave a record that `plan check` requires;
  - rule 8's mechanical edits go into the card;
  - every role's context has a ceiling it does not pass;
  - waits use the longest that Codex allows;
  - the hourly check runs in a fresh sub-agent;
  - `plan metrics` is fixed.

## 2. Outcome

The plan's "Done when", on `main` at `a8f2b7e3` (smith) and `bec13dcc`
(skein):

| # | Item | Result |
|---|---|---|
| 1 | Every step landed or dropped | **Holds.** `plan status` shows 108 of 108 landed. Its "Ledger and origin/main disagree" lines name smith-12.1 (a blank line cut its `Plan:` trailer off the trailer block) and smith-10.s1 (its commit says `Step: smith-10.10`): rulings 13.28 and 13.76. |
| 2 | Each session's own "Done when" | **Holds, as ruled.** done.md maps it to final reports; nobody ran the commands (section 5, item 11). Every `rg` and `git` item, run here once, holds except smith 08's `rg -n 'window: 1,' …/responses.rs`. It prints line 1113, a `cfg(test)` fixture, which ruling 13.82 accepts. The `heavy cargo nextest` items name tests in the suite that the last gate on `main`'s tree passed. |
| 3 | Session 13 ran its seven steps | **Holds as written, not as meant.** Both commands print 7. Four `Calibrated:` lines say "none" (13.2, 13.4, 13.6, 13.7). 13.1, 13.3 and 13.5 calibrated part of their probes. done.md names only 13.6 and 13.7 as not calibrated. |
| 4 | The smoke suite passes on `main` | **Does not hold.** 12.3's run of `smoke` had 4 attempts pass and 2 `first-request` attempts fail. The unnumbered ruling of 21:23:26Z classed them as the known uncalibrated configuration, "gating nothing". Not run again here: it is live. |
| 5 | Both gates within budget | **Holds, but its command no longer shows it.** In both repositories the command prints `none (Markdown only)`, since the newest commit is Markdown. The plan's last gates: smith `20ab17e4`, 2,000 tests in 4.211 s and 38 in 9.792 s; skein `7efc3072`, 2,072 in 4.275 s and 99 in 10.903 s. The run's worst: 9.933 s focused, 19.841 s fuzzy. |
| 6 | smith's report is published | **Holds.** `docs/development/benchmarks/smith-codex-2026-10-14.md`, from `5f4db659`, with its erratum. |

Against the README's purpose:
- **"Implement the spikes' fixes cleanly":** done as planned. The
  code-review agent finds the plan's "Don't" list kept in most places,
  and broken in four kinds (section 9): parallel types, second
  mechanisms, several behaviours under one test name, and new production
  lint exceptions.
- **"A harness that shows each fix works against a real provider":**
  - The harness exists and ran 660 attempts.
  - Of the 51 increment ids that session 13 was to prove, 22 are proven.
  - Of session 00's probes, `credentials` calibrated 10 of 10;
    `follow-up` and `history` did not.
  - The others stand on their deterministic tests, as decisions.md 12.4
    allows.
- **The comparison against Codex** is named "not an aim". The report's
  findings lead it:
  - 5 of 10 compaction attempts failed;
  - every fixture attempt failed the protected-tests scope check, for
    both agents;
  - smith reused the cache on the fixtures at about 0.2 times Codex's
    rate.

## 3. Numbers

`plan metrics --history` has one earlier run, tls-cleanup. Its first
column below is what `plan metrics` printed for this run, as it is in
`metrics.tsv`; the second corrects it where the script misreads the
record. The first run of this plan kept no metrics: it ran 73.1 hours
and merged 168 of 218 increments (95 of them unproven), with 22 block
reports and four stops.

| Metric | Printed | Corrected | tls-cleanup | Notes |
|---|---|---|---|---|
| `wall_h`, `running_h` | 36.0, 35.8 | | 1.3, 1.3 | One restart, 11 minutes. |
| `steps_planned`, `steps_added`, `steps_dropped` | 82, 26, 0 | 71, 37, 0 | 11, 2, 0 | The 11 split-off steps (08.10, 06.13, 10.17 to 10.25) count as planned. |
| `step_h` | 0.33 | | 0.10 | |
| `escalations` | 213, in 14 kinds | 185: plan defect 98, design gap 34, cross-session scope 32, process 14, infrastructure 7 | 12 | 28 open headings re-pasted into later sections were counted again. 14 blocks carry kinds the coordinator invented. |
| `escalation_wait_min` | 1.8 | 2.1 to the answer's log heading; 1.1 to the unblock's writing | 1.3 | |
| `blocked_h` | −67.8 | 7.7 | 0.2 | 21% of the run with a block open. |
| `answers_reversed` | 15 | 11 | 2 | 4 `Reverses:` lines name plan or design text, not an answer. |
| `steers` | 10 sent, 0 refused | 12 sent (13 points), 1 refused | 1, 0 | |
| `restarts` | "the 1" | 1, a crash, logged as "the user's request" | 0 | |
| `tokens_coordinator` | 540.2M in (99% cached), 1.11M out | | 15.9M | Up to its release at 01:17Z. |
| `tokens_implementers` | 1,600.9M in (98%), 6.0M out | | 43.2M | 69 threads. |
| `tokens_claude` | 4,253M in; 0 before the claim, 4,043M in the run, 209M after | 833.1M in the run (supervisor 583.5M, designer 249.6M), 1.31M out | 227.6M | **Not comparable** (below). "After" is this retrospective with its sub-agents, up to `metrics.tsv`. |
| `tokens_per_step` | 57.4M | 27.6M | 22.2M (about 10M for the run alone) | |
| `coordinator_requests` | 2,813, 168 after a wait ran out | 2,709 distinct requests | 147, 4 | |
| `compactions` | 7 | | 0 | Each at 340–390K of a 475K window. |
| `gates_per_step` | 3.5 | 1.75 gates of `scripts/check.sh` | 2.5 | **Not comparable:** the gate lane also ran 180 live tasks and 12 test runs. |
| `queue_wait_ratio` | 0.93 | | 0.66 | Gate lane: 2.8 h waiting, 8.5 h running. Build lane: 9.7 h waiting, 5.0 h running. |
| `suites` | last: "none (Markdown only)" | smith, first and last: 2,222 tests in 6.280 s → 2,000 in 4.211 s focused; 34 in 14.528 s → 38 in 9.792 s fuzzy | 1,940 / 5.0 s; 97 / 31.5 s | 786 generated tests went when 10.s2 regenerated the codecs. |
| `proofs` | unit 28, world 28, process 23, live 17, none 5; 7 below plan | 3 below plan: 00.16, 10.6, 04.13 | 0 below | Not really below: 3 are first commits of multi-commit steps, and 1 was matched against the wrong increment. Of the 5 "none", 2 meet a plan that says none, 2 have the Proof in the body, and 06.s2 has none. |
| `report_lines` | 10 median, 12 p90 | | 10, 10 | 72 reports under "## smith05," and "## smith 10b," were missed. 7 are over 40 lines. |
| `lines` | code +132,760 −30,530; tests +21,804 −4,400 | production Rust +27,351 −9,933; benchmark data +96,274 −733; generated +8,029 −18,822 | +1,445 −1,461 | smith's production Rust grew by 16,873 lines net. |
| `pin_lag` | smith: skein 1; temper: skein 262, smith 783 | | — | jig is not on this machine. |
| `contexts` | sessions' peaks 245K median, 348K p90; 41 handovers | 42 handovers (one line names two) | — | Only one handover came below 200K. |

**Not comparable, and why:**
- **`tokens_claude`.**
  - Each `plan wake designer` resumed the designer as a new session id,
    and each new id copies the earlier history. That gave 26 files that
    repeat each other, message for message.
  - `claude_tokens` counts each message once within a session id, and
    `metrics_of` sums the ids, so the designer counted 3.46B for 249.6M.
  - "Before the claim" is 0, because the session that ported the plan is
    not in `claude.tsv`.
- **`gates_per_step`** counts every gate-lane task: 189 gates, plus 180
  live runs (98 smoke, 65 probes or calibrations, 13 suites, 4 others)
  and 12 test runs.
- **`escalations`** carries the coordinator's invented kinds. The six
  kinds of section 10.2 are in the corrected column.
- **`lines`** counts benchmark summaries, baselines and generated code as
  code.

**What explains the largest differences:**
- **Escalations per step, 1.7 against 0.9.**
  - The plan's shape against the code (section 5, item 1): 95 blocks.
  - Design gaps: 34, against none in tls-cleanup.
  - Live work and diagnostics: 41.
  - Of the 185, 121 came in the first ten hours.
- **Tokens per step, 27.6M against about 10M.**
  - Larger contexts at every level (section 5, item 3):
    - the implementers' median peak was 245K;
    - the coordinator's median request was 198K (tls-cleanup's peak was
      164K);
    - the supervisor averaged 415K and the designer reached 918K.
  - Waiting: about 380M across the implementers and the coordinator.
- **`step_h`, 0.33 against 0.10:** the serial tail (section 5, item 2).
  The first nine hours ran at 0.12.
- **`queue_wait_ratio`, 0.93:**
  - The build lane waited 9.7 h for 5.0 h of running, because each gate
    task pauses it. The gate lane ran 8.5 h: 3.5 h of gates, 5.0 h of
    live runs.
  - 46% of build tasks failed: 384 of 793 test runs, 250 of 445 clippy
    runs. That is the ordinary work of an increment, not waste.
- **Gate times** varied with the machine's load:
  - focused from 4.2 s to 9.9 s;
  - as the gates started alone, the one-minute load had a median of 3.05
    on 8 cores, and 79 of 189 started above 4;
  - Codex, pueue and the code graph's indexer were among the three
    busiest processes. The indexer, which pueue does not hold, was there
    86 times.

## 4. What worked

- **The process ran 36 hours, 69 threads and 108 steps on one machine
  without a dropped step.**
  - Every code landing carries its `Gate-Result`.
  - No commit's tree differs from its `Gate-Tree` outside Markdown under
    `docs/`.
  - No ruling dropped part of a gate.
  - No suite passed its budget.
- **The escalation channel held at fifteen times tls-cleanup's volume.**
  - Blocks reached `blocked.md` a median of 35 s after the report.
  - The coordinator read `unblock.md` a median of 6 s after the ring (p90
    14 s).
  - The supervisor answered in a median of 1.1 minutes, and landed its
    ruling first in 106 of 156 unblocks.
  - The designer answered within 15 minutes each time, with the design and
    the session files landed before `answer.md`, as 10.4 asks.
- **Standing rules on the second occurrence, from the first minutes.**
  - The rules came in at:
    - rule 8 at 13:26, at the second block of its kind;
    - rule 9 at 13:46;
    - rule 10 at 14:58;
    - rule 11 at 15:17;
    - rule 12 at 16:40.
  - Each reached every running session through a steer, logged within two
    minutes.
  - No Pulse word landed after rule 12's grep: it ran before each of the
    52 landings of w36–w69, and before every landing after 16:40 in
    w1–w35.
- **The previous retrospective's rules that took** (section 7):
  - **`plan check`** caught a red `main` and a dirty primary checkout at
    the restart.
  - **`land`** kept 21 gates across Markdown commits, which made the
    supervisor's 122 Markdown landings free for waiting branches.
  - **Clippy in the build lane** before every Rust landing.
  - **Times from the clock:** the supervisor's skew was under a minute,
    against 2 to 12 minutes in tls-cleanup.
  - **Checks that print:** 139 of 139 ancestry checks printed `on-main`.
  - **The designer** joined, ended its turn, and was woken 26 times; it
    never watched.
- **The coordinator kept to 7.4 and 2.3.**
  - Its 548 waits were all 300 s, and it started no background loop, sleep
    or watcher.
  - It answered no question itself, and started no step without an
    instruction: all 26 supplements carry their `supplement:` notes.
  - It managed disk as 11.4 says: it held starts below 40 GB, and cleaned
    only stopped sessions' targets.
- **Supplements to skein were cheap.** skein-07's threads w52, w55, w56 and
  w57 each cost 1.6–4.1M, took 6 to 12 minutes, and landed on their first
  gate.
- **The live budget held.**
  - 660 attempts in 178 runs, exactly the README's estimate.
  - $17.60 of known smith notional spend against $95. Codex's 44 attempts
    are unpriced.
  - No provider failure stopped a landing.
- **Self-correction.**
  - The supervisor reversed 7 of its 11 wrong answers within 15 minutes,
    and said so in its checks.
  - The designer named its own errors ("My answer of 19:30:20Z was
    incomplete… I measured this instead of inferring it").

## 5. What did not

Each item gives its cause, when it was first visible, and when it was
acted on.

1. **The plan's shape did not match the code.**
   - **The blocks it caused: 95 of 185.**
     - 43 where Files, Users, Shape, Test shape or Done when did not fit
       the code.
     - 20 over Size.
     - 32 that crossed sessions or repositories.
   - **The 38 plan-defect rulings, by cause.** They are spread over every
     session; smith 03 and smith 10 have 7 each.
     - Users missed from Files: 7.
     - Shapes the code or design could not satisfy: 8.
     - Tests whose expected values the Scope changes: 8.
     - Wrong Needs: 4.
     - Done-when commands: 2.
     - The harness's rule 4 against a "Don't": 4.
     - A task authored with values it cannot meet: 1.
   - **The Size lines.** 65 lines added up to 15,650 production lines; the
     run landed about 24,400. Session 10's chains took more than their
     Size lines:

     | Increment | Size line | Landed | Ratio |
     |---|---:|---:|---:|
     | 10.13 (with 10.17, 10.18) | 500 | 1,519 | 3.0× |
     | 10.7 (with 10.19) | 550 | 1,422 | 2.6× |
     | 10.6 (with 10.23) | 450 | 1,353 | 3.0× |
     | 10.5 (with 10.20 to 10.22) | 600 | 2,325 | 3.9× |
     | 10.8 (with 10.24, 10.25, 10.s4) | 600 | 2,520 | 4.2× |

     Each overrun past 700 stopped and asked, as rule 9 says. The answers
     were always a split (11) or an exception (6), and each cost a block,
     a ruling and a plan landing.
   - **Cause: the port.**
     - It moved 71 first-run increments into development.md's shape.
     - It compiled the sketches through `heavy` in 10 scratch worktrees,
       which is more than tls-cleanup's planner did.
     - It did not search for the users of the types it changed: 13.1, six
       minutes into the run, found that 06.1 missed 3 expected values and
       4 callers, and 02.6 missed exhaustive matches in about 15 files.
     - It did not run each "Done when" command: session 04's printed 8,
       not 4 (13.69).
     - It kept the first run's Size lines.
     - The plan has no record of a code audit, a consistency review or a
       literal reading (5.4).
   - **Visible and acted on:**
     - First visible at 13:24:02Z, the first block. Acted on at 13:26 with
       rule 8, then with each widening, the last at 05:34Z on 10-13.
     - 25 blocks about edits outside Files came after the rule existed, the
       last at 21:11Z on 10-13. A rule that lists cases finds each new
       case through a block.
2. **One serial session held the critical path.**
   - **Session 10 grew from 14 increments to 28.**
     - It raised 66 of the 185 blocks.
     - smith 12.3 needed all of it.
     - The README put it ninth in the start order, and said sessions with
       more than five increments "are not split".
   - **One thread for most of the run.** One implementer thread ran for 20
     of the 36 hours.
   - **Stretches with no landing.** There were twelve longer than an hour,
     17.3 hours in all. In each, a block was open for 1 to 23 minutes and
     the queue was busy for 5 to 15. Ten of the twelve were session 10
     working its large increments one at a time.
   - **Visible and acted on:**
     - At the 19:45 check on 10-12: "session 10 on the critical path".
     - Steer 9 (19:46) started a second thread for 10.10.
     - Steer 10 (20:46) asked for another and was refused, because the
       Files overlapped. The supervisor's next check: "its check was
       right; mine was not: I steered without reading the Files".
     - Nothing else could widen it.
3. **Contexts grew without a ceiling.**
   - **The supervisor.**
     - The first session handed over at 15:45 with about 470K, starting
       its successor itself with `claude --bg`.
     - The second wrote its handover entry at 17:50 with 433K. It then
       asked the user, at 17:50, 20:07 and 22:03, to start the successor;
       nobody did.
     - It passed 500K at 18:45 and compacted at 966K at 09:47 on 10-13,
       15 hours later.
     - That stretch used 338M, 58% of the supervisor's tokens: about 220M
       more than at the other stretches' average context.
   - **The designer.**
     - One session answered all 26 questions. Its context went from 42K
       to 918K, and 93% of its 249.6M input was its own history, re-sent
       with every request.
     - On 11 of the 26 wakes the cache had expired, and 344K–883K of
       history was written again.
     - A fresh session per question would have cost an estimated 81–146M.
   - **The implementers.**
     - The 42 handovers came at a median of 247K: one below 200K, seven at
       300K or more.
     - w67 grew from 20K to 406K inside one increment, so it had no point
       between increments to hand over at.
     - w49 and w61 ran session 10 for 9.5 and 13.3 hours. They compacted 6
       and 10 times, and were never handed over: `plan contexts` reads
       the last request, which a compaction had just shrunk ("current 60K
       after compaction, peak 401K; … does not require a fresh thread").
       Together they used 371.8M, 45% of w36–w69's input, at an average
       context of 194K against 161K for the others.
   - **The coordinator.**
     - Its median request was 198K.
     - It compacted 7 times at about 75% of its window.
     - After the 11:11 compaction on 10-13 it re-read only parts of
       development.md, skipping 10.2. The 14 invented kinds and the
       "Unblock handled" headings start minutes later.
   - **The cause.**
     - `plan contexts` measures the last request.
     - 7.3's limit applies only between increments.
     - Section 10.5 gives the designer no handover.
     - The supervisor's handover depends on its starting a session itself,
       which 10.5 does not say how to do.
   - **First visible:** 17:50 on 10-12 (supervisor), the fifth wake at
     16:33 (designer, 369K), w49's first compaction at 22:1x. Not acted on.
4. **Waiting on one's own commands, still.**
   - **The implementers.** About 290M of their input (18%) went to calls
     that only waited on a running `land` or `heavy`:
     - w1–w35: 843 calls, 143.9M (18.4%);
     - w36–w69: 626 calls that returned nothing, 111.9M (13.7%), and 148.1M
       counting the calls that tailed a log.
     - The worst threads spent 41–42% this way (w27, w34, w67).
   - **The cause:**
     - In w1–w35, 1,037 launches of `land` or `heavy` asked for no wait
       longer than 60 s, and 806 gave none.
     - Codex's own base instructions say "Avoid performing blocking sleep
       or wait calls longer than 60 seconds". That contradicts 8.3's "the
       longest wait your tool allows".
     - `write_stdin` takes 300 s: w27 waited a full 300 s once, at
       17:27:46. The cell's own `wait` stops at about 61 s.
   - **The coordinator.**
     - 139 of its 548 waits timed out with nothing to do, and 50 `sleep
       60` cycles ran with no session running: 93M, 17% of its input.
     - Most came in the 20 hours with a single thread.
   - **Visible and acted on:** not counted by any metric, so not seen by the
     checks. Not acted on.
5. **The hourly check thinned out.**
   - **36 full checks, one missed** (20:45 on 10-13, the supervisor's
     weekly usage limit).
   - **Length.** The cron-driven checks took 0.3–2.7 minutes, median 1.1,
     against "about ten".
   - **Samples.**
     - A test was read against the checklist (14.3) in the first 8 checks
       only, to 20:45 on 10-12.
     - From 21:45 on 10-12 to 08:45 on 10-13, 13 checks sampled neither a
       test nor a diff, because the supervisor was "past its handover".
     - The tests it did find (several behaviours under one name, a 200-line
       scaffold) were noted "for the retrospective" and never steered,
       though 14.4 asks for a steer the first time.
   - **The pulse was filtered.** From 19:45 on 10-12, a `grep` on the pulse
     kept only some of its lines, dropping:
     - proofs, waste, code signals and rising words;
     - cost, pins and contexts.
   - **`plan downstream`.** It ran 14 times from 10:45 on 10-13, and its
     output was read once, 7 s after a start, when it was still empty.
     - smith did not build against skein's main from 00:10 on 10-14:
       carbon-forge's skein `2df5b58` (22:52:50Z) added `Read::Arrived`,
       which two smith crates do not match.
     - carbon-forge's own smith commit `c5eb35eb` repaired it at 01:10:59Z.
       The run never saw it.
   - **Cause:**
     - The check ran as a turn of a 400–900K context.
     - "About ten minutes" and the samples are guides that nothing checks.
   - **First visible:** 19:45 on 10-12 (the filter). Not acted on.
6. **Answers that rested on unchecked premises.**
   - **11 answers were reversed,** 10 of them on a number or a premise
     nobody checked:
     - one line allowed in a reader that needed two;
     - two signals in flight, which io.md 3.4 forbids;
     - an archive named without checking it had its helpers;
     - instructions given to 05.s1, which had already landed;
     - a misread HTTP 400;
     - "settings already check effort", which stood 4 h 41 min;
     - a `smith check` JSON field that does not exist, which stood 73 min;
     - the wrong owner;
     - a probe that had not run.
   - **Commands quoted.** 23 of 156 unblocks quote a counting command.
   - **The designer** was quick and wide twice:
     - wake 22 changed six design files in four minutes, and needed three
       more wakes to correct (23 to 25);
     - wake 9 inferred a cause that wake 11 then measured and corrected.
   - **Cause:** 10.4's "check an answer against the code before sending it"
     is a sentence that nothing checks, and the answers went out in about a
     minute.
   - **First visible:** 13:50 on 10-12, the first reversal. The supervisor
     wrote the lesson down at 18:29 and again at 21:45 on 10-13.
7. **Gates that failed after the implementer's own checks.**
   - **70 of 189 gates failed:**
     - 54 on tests;
     - 13 on clippy;
     - 3 on compile errors.
   - **The tests.** Most failures were expected values in end-to-end,
     fuzzy and smith-bench tests that no focused run of the increment
     included. `smith::end_to_end a_first_run_signs_in_and_ends_with_a_commit_in_place`
     failed four sessions' gates, and `legacy_recordings_terminals_and_emission_metadata`
     four.
   - **The clippy failures.**
     - In w1–w35, 10 of the 12 came after an edit that followed the last
       passing clippy: a conflict's resolution, a gate's fix, or a fixture.
     - Meanwhile clippy ran about 4 to 5 times per landing, up to 20 (w61,
       10.s4).
   - **Cost.** The 22 failed lands of w36–w69 cost 49.1M and 151 minutes.
     The 47 failed gates of w1–w35 took 44 minutes of the exclusive gate
     lane.
   - **Cause:**
     - The focused run before `land` covers the crates the implementer
       names, not the crates that depend on them.
     - 8.3 forbids whole suites outside the gate, though the focused suite
       runs in 4–10 s.
   - **First visible:** 13:32 on 10-12. Not acted on.
8. **The coordinator's record drifted from its shape.**
   - **`log.md` is 6.2 MB, and 77% of it is copies of `context.md`.**
     - From the compaction of 05:03 on 10-13, the coordinator pasted the
       old `context.md` before each rewrite: 179 copies.
     - It invented the rule itself ("log.md archives every … previous
       context verbatim"); 3.5 says `context.md` is "rewritten, never
       appended".
   - **`context.md`.** It kept its 60-line limit by squeezing out spaces:
     - lines up to 5.4 KB;
     - from 15:29 on 10-12, words glued together ("smith10SECOND
       current210K afterownS3landing"), which spread into `log.md`.
   - **After the 11:11 compaction on 10-13:**
     - 21 answers were logged under "Unblock handled" instead of "Blocked
       and unblocked";
     - two steers under "Steer handled", with no point lines;
     - 14 blocks carry kinds of its own ("plan Shape and Files gap",
       "process Proof allowance").
   - **Six of `plan metrics`' numbers** come out wrong because of these
     (item 12).
   - **Smaller lapses.**
     - It sent 34 `send_message` calls to threads that had ended their
       turn, where 7.3 says `followup_task`.
     - It named a pseudo-session "smith-10b", with its own worktree and
       checkpoint.
     - At the end it stashed `Cargo.lock` edits in two downstream
       worktrees, though 8.2 forbids `git stash`.
   - **Cause:** compaction pressure, and a 60-line limit enforced by
     retyping the file.
   - **First visible:** 15:29 on 10-12, the glued words. The pulse's log
     growth line was among those the supervisor filtered out. Not acted on.
9. **The coordinator worked with the code graph.**
   - **The calls.**
     - 336 graph calls: 123 `search_graph`, 105 `check_index_coverage` and
       30 full `index_repository` runs of the primary checkouts.
     - They make up 38% of the tool output it read, about 52–81M of its
       input.
   - **Spawn prompts.** They gained "Scout evidence" and grew from about
     1.5K to about 15K characters; 7.2's prompt is about 0.5K.
   - **Code read.** `context.md` holds code line ranges, though 2.3 says
     the coordinator reads no code.
   - **The primary checkout.** Smith's held an untracked `.codebase-memory/`
     directory, kept fresh by those re-indexes. It made `plan check` fail
     at the restart.
   - **Implementers.** They read the graph's skill file at every start.
     w61 made 285 graph calls.
   - **Cause:** a block in the user's global Codex `AGENTS.md`, outside
     the kit: "Whenever spawning a subagent, include … relevant
     graph/coverage evidence".
   - **First visible:** 13:19 on 10-12. Not acted on.
10. **Proving fell short, and the record says less than it seems.**
    - **Session 13.** Four of its seven steps calibrated nothing. Done
      when 3 counts `Calibrated:` lines, so "none" passes it.
    - **Calibration is recorded only in commit bodies.**
      - Only `credentials` has a `[calibration]` block in its task file,
        and its `failed_before` is "none: no runnable control was named".
      - `calibration_suite(_probe)` ignores its argument.
    - **A diagnostic step that blocked at every run.** 06.s2, a hunt for a
      silence:
      - it raised 14 blocks in 105 minutes (21:25–23:10Z), one for each
        diagnostic run or world-library allowance (160, 172, 173, 183,
        191, 192 lines);
      - README rule 11 counts runs per failure, and the step had no budget
        of its own.
    - **The supervisor never totalled the spend** for the README's "$60"
      report. It stayed far below, but unchecked.
    - **First visible:** 16:56 on 10-12 (13.1's partial calibration), and
      21:25 (06.s2). Ruled one by one.
11. **Nobody ran the plan's "Done when".**
    - **Who could have.**
      - 7.6 tells the coordinator to "run nothing".
      - The README says each item is "checked by its command, on main,
        after the last step lands", and each session file says "run once
        each".
      - done.md maps the items to reports.
    - **Commands that cannot show their item.**
      - Item 5's command reads the newest `Gate-Result`, which a Markdown
        commit lacks, so it now prints "none".
      - Item 3's passes with four "none" lines.
    - **done.md's accuracy.**
      - It says 14 "Calibrated/Proven records" where four say none.
      - It cites 11 of the 32 design commits.
    - **Visible and acted on:** visible at the end, not acted on.
12. **`plan metrics` misreads the record.** Its causes, from the kit's
    `numbers.sh`:

    | Metric | Cause |
    |---|---|
    | `tokens_claude` | It sums 26 forked designer sessions that repeat their parent's messages. |
    | `blocked_h`, `escalation_wait_min`, `escalations` | It reads only "Blocked and unblocked" headings. The 21 "Unblock handled" sections inherit the last such heading's time, which gives 21 negative intervals (−74.7 h). Re-pasted open headings count again. |
    | `restarts` | `awk` prints `$2`, the first word of the cause. |
    | `steers` | It misses "Steer handled", and "point1:cannot" without spaces. |
    | `steps_planned`, `steps_added` | Only `.s<k>` counts as added. |
    | `suites` | The last plan commit is Markdown. |
    | `proofs` | `planned_layer`'s `$((10#$session))` turns "00.27" into "0.27", which matches by accident. No `.s<k>` id ever matches, so the 26 supplements go unchecked; "10.10" matched "10.1". Intermediate commits of a multi-commit step count. |
    | `lines` | Benchmark data and generated code count as code, and inline `cfg(test)` modules as production. |
    | `gates_per_step` | Live runs in the gate lane count. |
    | `report_lines` | Reports under "## smith05," and "## smith 10b," are missed. |
    | `coordinator_requests` | It counts token-count events, some repeated: 2,811 for 2,709 requests. |
    | `run_ended` | It reads `done.md`'s modification time, 01:18:16Z, not its record time, 01:14:15Z. |

13. **One provider error, a missed push, and a supervisor fixing primary
    checkouts.**
    - **The crash.**
      - At 06:01:17Z on 10-13 the coordinator's turn, 16.7 hours long,
        ended with "Selected model is at capacity". The ring failed ("no
        turn in progress").
      - The supervisor's push to the user came back "Not sent — this
        terminal is active". It was the only push of the run.
      - The supervisor ran `plan resume` itself at 06:11:57, which 10.5
        does not say it may.
      - The coordinator logged the cause as "the user's request".
    - **The check that followed.** `plan check` then failed:
      - skein's primary checkout was one commit behind, from carbon-forge;
      - smith's held the untracked `.codebase-memory/`, and with it `main`'s
        gate failed.
      - The supervisor repaired both in the primary checkouts, by hand: a
        `git pull --ff-only`, and a line in `.git/info/exclude`.
    - **The supervisor's weekly limit** stopped it for 29.5 minutes at
      20:32 on 10-13. Its entry says "my watch ended".
    - **Visible and acted on:** all were visible at once and acted on within
      minutes. The causes recorded are wrong.
14. **Another machine on the same repositories.**
    - **carbon-forge's landings.** It landed 19 smith commits and 47 skein
      commits during the run, for jig's `jig-protocol` and `test-budgets`
      plans. Two consequences:
      - its skein change broke smith against skein's main for an hour
        (item 5);
      - the same rule landed twice: skein `codec.md` (`58cc905`) and smith
        `limits.md` rule 6 (`4c7b15e6`), "one piece bounded by its sum".
    - **Step numbers.** Its step numbers (`smith-01.1`, `smith-02.1`) clash
      with this plan's in blame, and only the `Plan:` trailer tells them
      apart.
    - **Gate times there.** Its smith gates ran the focused suite in 16.5 to
      24.1 s, over 15 s, on four commits. Budgets are the reference
      machine's (11.5), but nothing names the machine in the trailer.
    - **Visible and acted on:** visible in `plan downstream`, unread.

## 6. Decisions taken during the run

decisions.md section 13 holds 133 rulings. About 49 more unblocks rule
something that section 13 does not cite; about 20 of those carry a
designer's answer into the session files. The 26 design answers took 32
design commits: 26 in smith, 6 in skein.

| Rulings | What | In the design | Proposal |
|---|---|---|---|
| 13.1, .3, .4, .8, .19, .20, .24, .32, .36, .40, .43, .111 (README rule 8) | The edits a change in place forces outside Files | The plan's. development.md 8.2 has a narrower rule | 4 |
| 13.9, .46, .105 (rule 9); 11 splits; exceptions 13.15, .26, .29, .80, .96, .102 | Size as a guide up to 700; past it, a split | The plan's | 1 |
| 13.21 (rule 10) | A landed step checked on its own branch | The plan's; development.md 7.3 | 15 |
| 13.25, .38 (rule 11) | Diagnostics never committed; three runs per failure | The plan's | 13 |
| 13.45 (rule 12); 13.28 (Pass on) | The Pulse words grepped before `land`; trailers in the last paragraph | The plan's | 15 |
| 13.50, 13.67 (rule 3) | A live batch stopped by the provider runs once more; the `EINVAL` retry, until skein-07.s2 | The plan's | — |
| 13.41, .44 | Codex from a private copy of 0.160.0 | The README's "Live services"; not in benchmarks.md | 19 |
| The 26 answers | Compaction, limits, the product's commands, sign-in, the terminal | In the design (`0f1d72ba` … `dafea731`; skein `bdbe1fd` … `ed8d235`). done.md names 11 | — |
| 13.14, .91, .98, .99 | The harness: a build lock, the baseline selector, a refused start counts no request, a 4xx before output charges nothing | Not in benchmarks.md yet | 19 |
| 13.116 | Effort as text of at most 32 bytes | Not in limits.md; the literal is in five places (section 9) | 19 |
| 13.39 | io drops a second child signal in flight | A skein question, not in io.md | 19 |
| 13.132 | A large effect is disjoint ranges over three attempts | Not in benchmarks.md, whose 3.1 and 11.5 name no threshold | 19 |
| 13.131 | Audits of every grader disagreement; session 12's line at 149 attempts | The plan's; its README still says 126 | — |
| Steer of 15:46:42Z on 10-13 | A fix after the smoke takes one more smoke | A Pass-on only | — |
| 21:23:26Z, unnumbered | The smoke's two `first-request` failures gate nothing | Not in section 13 | — |
| 00:52:04Z on 10-14, unnumbered | The compaction threshold goes to the user, recommending 80% | Not in the design, nor in "For the user" | user |

**Still with the user** (decisions.md section 10, and the run's ends):
- **Borrowing another CLI's login (7.9):** never answered. The run worked
  read-only, under README rule 7.
- **The smoke before `land` (12.3):** open; the run worked that way.
- **The skein pin tags (12.2).** Unpublished: smith commits from 10-09 to
  10-12 pin skein revisions that exist only in this machine's skein clone.
  The push is `git -C ~/src/rust/skein push origin
  'refs/tags/smith-pin/*'`, from this machine.
- **The working tier, `gpt-6-astra` at high (12.17):** never answered.
  Session 12 spent $7.82 on it.
- **Local `sub_agents` on by default (07.13):** put to the user at
  18:36:41Z on 10-12, recommending off. Unanswered. The benchmark arms ran
  with it on (section 9).
- **The compaction threshold and contents (4.7):** routed at 00:52:04Z on
  10-14, recommending 80%. It is not in section 10.
- **temper:** 83 errors against smith's `main`, from before the run, wait
  for jig's and temper's own plans.

## 7. The proposals adopted after the last retrospective

All eighteen were adopted on 2026-10-12, four hours before the port. For
each: the metric it was to move, tls-cleanup's number, this run's, and
whether it moved. Counts are per landed step where the runs' sizes
differ.

| Proposal | Meant to move | tls-cleanup | This run | Moved |
|---|---|---|---|---|
| 1. `plan check` queues work and runs the gate | infrastructure blocks | 7 (0.54 a step) | 7 (0.06 a step); it caught a red `main` at the restart | Yes |
| 2. The planner runs the commands, compiles the sketches, lists the users | plan-defect blocks | 5 (0.38 a step) | 98 (0.91 a step); sketches compiled, users not searched | No |
| 3, 4. Edits outside Files; boundary types | plan-defect blocks | 6 blocks of the kind | 8.2's rule covered none of the first three questions; rule 8 replaced it within 2 minutes | No: too narrow |
| 5. One retry of a gate that times out | infrastructure blocks | 2 | 0 timeouts in 70 failed gates | Not exercised |
| 6. Clippy before `land` | `gates_per_step` | 2.5; clippy 5 of 9 failures | 1.75; clippy 13 of 70 | Yes. But clippy ran 4–5 times a landing |
| 7. A gate kept across Markdown | `gates_per_step` | 9 gates thrown away | 21 landings kept their gate | Yes |
| 8. Rising words in the pulse | `steps_added` for words | 1 | 0; filtered out of the supervisor's reading from 19:45 on 10-12 | Held |
| 9. Removed words name what replaces them | `steps_added` for words | 1 | 0; the rising words in Rust are domain words | Held |
| 10. Each aim a "Done when" item, or named unchecked | `lines` toward the aim | +270 lines against −500 | Six items with commands; two hold only in form | Partly |
| 11. Answers check their numbers | `answers_reversed` | 2 (0.15 a step) | 11 (0.10 a step); 10 unchecked premises | No |
| 12. The designer does not watch | `tokens_claude` | 49.3M, no question | 249.6M for 26 questions, no watching | Yes; the cost moved into one growing session |
| 13. Implementers wait in one call | `tokens_implementers` | up to 43% of a session | about 18% of all, up to 42% of a thread | No |
| 14. Times from `date -u` | — | 2–12 min ahead | under a minute | Yes |
| 15. A dropped gate reaches the user | gates without a suite | 13 landings | none; the one push was refused | Untested |
| 16. Gate logs record the load | comparable `suites` | — | all 189 gate logs; median load 3.05 on 8 cores | Done; gates do not run on a quiet machine |
| 17. `tokens_claude` in three parts | comparability | — | three parts, but forks counted 26 times and planning missing | Partly |
| 18. The kit keeps its record true | — | — | `plan retro` found the released run; every check printed | Mostly |

## 8. Proposals

Each proposal gives a rule in one sentence, its evidence, the document
it changes, and the metric it should move. They are ordered by expected
effect.

1. **An implementer whose increment outgrows its Size line splits it at a
   point that lands alone, lands the first part, and reports the split; it
   asks only when no such point exists.**
   - Evidence:
     - 20 size blocks, every one answered by a split (11) or an exception
       (6);
     - session 10's chains took 2.6–4.2 times their Size lines;
     - each split cost a block, a ruling and a plan landing (section 5,
       item 1).
   - Changes: development.md 5.3 (a session may split an increment, as it
     may reorder) and 8.7. The coordinator records the new step from the
     report, as `<step>` with a "split by the implementer" note.
   - Moves: `escalations` (plan defect) and `blocked_h` down.
2. **No session holds more than eight increments, and the README names
   the longest chain of increments that must land one after another,
   with its hours at the plan's expected `step_h`.**
   - Evidence: session 10 went from 14 to 28 increments and raised 66
     blocks. One thread ran for 20 of 36 hours. A second thread was
     refused because the Files overlapped (section 5, item 2).
   - Changes: development.md 5.3: "about four or five" becomes at most
     eight. Where "an area that needs more is two sessions, the second
     needing the first", its sessions get disjoint Files where they can.
     Also 5.2 (the README).
   - Moves: `wall_h` and `step_h` down.
3. **The planner records its code audit, its consistency review, its
   literal reading and each command it ran in the plan's `findings/`,
   and `plan check PLAN_DIR` refuses a plan without them; a plan
   rewritten from an earlier run counts as new.**
   - Evidence: 38 plan-defect rulings, the first six minutes into the
     run. Users of changed types were not searched (13.1). A "Done when"
     command printed 8, not 4 (13.69). The plan has no record of the
     three reviews (section 5, item 1).
   - Changes: development.md 5.4; `plan check`.
   - Moves: `escalations` (plan defect) down.
4. **An edit that only follows a change the increment's Shape names is
   part of the increment, wherever it falls in the repository, and its
   commit body lists it under "Users outside Files:".**
   - Such edits:
     - a new variant handled as its match handles the others;
     - a removed name replaced by its successor;
     - a call adapted to a new signature;
     - an expected value the same story now produces;
     - a cue or fixture input kept on its story.
   - Evidence:
     - README rule 8 was written at 13:26 and widened 11 times by blocks;
     - 77 commit bodies list such users;
     - the tls-cleanup rule (own files, 20 lines, no expected value)
       covered none of the first three questions.
   - Changes: development.md 8.2. It replaces the "edit outside the
     increment's Files" bullet's limits for these edits. Any other edit
     outside Files stays a question.
   - Moves: `escalations` (plan defect, cross-session scope) down.
5. **`plan contexts` marks a thread past the limit by the largest context
   it has reached, so a compaction cannot hide it; and a thread that has
   compacted hands over at its next landing.**
   - Evidence: w49 and w61 compacted 16 times and ran 9.5 and 13.3 hours
     without a handover. They used 371.8M, 45% of w36–w69's input, at a
     higher average context than the others (section 5, item 3).
   - Changes: `plan contexts`; development.md 7.3, step 4.
   - Moves: `tokens_implementers` and `contexts` down.
6. **The designer answers each question in a fresh session.**
   - How: `plan wake designer` starts that session with the question and
     sections 2.1 and 10.4. The design and `supervision.md` hold what
     earlier answers decided.
   - Evidence:
     - one session for 26 questions;
     - 93% of its 249.6M input was its own history;
     - the cache expired on 11 wakes;
     - an estimated 81–146M for fresh sessions.
   - Changes: development.md 6 (step 5), 10.4 and 10.5; `plan wake`.
   - Moves: `tokens_claude` (designer) down.
7. **The supervisor hands over with `plan handover supervisor` once its
   context passes 400K tokens.**
   - How: it starts the successor in the background, records it in
     `claude.tsv`, and stops the old session's watch and cron, which the
     new one starts.
   - Evidence:
     - the second session's handover entry, at 433K, waited for a user who
       was away;
     - it ran 15 hours past half its window, to 966K;
     - that stretch used 338M, and the checks thinned during it (section
       5, items 3 and 5).
   - Changes: development.md 2.5 and 10.5; `plan`.
   - Moves: `tokens_claude` (supervisor) down.
8. **The hourly check runs in a fresh sub-agent, which reads section 14,
   the whole `plan pulse` and `plan downstream`'s last output, samples
   one test and one diff, and returns the entry; the supervisor appends
   it and steers.**
   - Evidence:
     - the checks took a median of 1.1 minutes;
     - tests were sampled in 8 of 36;
     - the pulse was filtered from 19:45 on 10-12;
     - `plan downstream` was read once in 14 runs, so smith's break against
       skein's main went unseen (section 5, item 5);
     - each check paid the supervisor's whole context, 82M for 36.
   - Changes: development.md 14.1 and 14.5.
   - Moves: `tokens_claude` (supervisor) down; the check's findings up.
9. **Waits are the longest Codex allows, and the documents name them.**
   - An implementer waits on a running `land` or `heavy` with one
     `write_stdin` of `yield_time_ms` 300000, repeated until it ends.
   - `test-plan --live` measures that yield and `wait_agent`'s longest
     timeout, and section 7.4 uses the second.
   - Evidence:
     - about 290M of implementer input (18%) went to waiting calls;
     - no launch in w1–w35 asked for more than 60 s, as Codex's base
       prompt advises;
     - `write_stdin` took 300 s;
     - the coordinator's empty wake-ups cost 93M (section 5, item 4).
   - Changes: development.md 8.3, 7.4 and 13 (it closes "`wait_agent`'s
     longest timeout"); `test-plan`.
   - Moves: `tokens_implementers` down; `coordinator_requests` after a wait
     ran out, down.
10. **An implementer reads, before its first edit, this card, its
    repository's `AGENTS.md`, its session file and the sections its
    increment's "Read first" names; it reads the README,
    code-organization.md and skein's foundation documents by the sections
    the session file cites.**
    - Evidence:
      - a median of 72K tokens of documents before the first edit
        (w1–w35), re-sent with every later request: up to 40% of w1–w35's
        input;
      - `programming-model.md` (52 KB) was often read whole;
      - each of the 40 fresh threads after a handover rebuilt 75–150K.
    - Changes: development.md 7.2 (the prompt) and 8.1; 5.2 (the session
      file's "Rules that bind it" carries the README's rules that apply).
    - Moves: `tokens_implementers` and `contexts` down.
11. **The coordinator works without the code graph.**
    - How: its Codex profile turns off the codebase-memory server.
      `plan check --fix` adds `.codebase-memory/` to each repository's
      `.git/info/exclude`.
    - Evidence:
      - 336 graph calls and 30 full re-indexes, about 52–81M;
      - spawn prompts ten times 7.2's;
      - a dirty primary checkout that failed `plan check`;
      - the indexer among the three busiest processes as 86 of 189 gates
        started (section 5, item 9).
    - Changes: `workspace/coordinator.config.toml`; `plan check --fix`;
      development.md 7.2 ("the prompt above, with the Pass-on texts, and
      nothing else").
    - Moves: `tokens_coordinator` down; `suites` steadier.
    - The user's global Codex `AGENTS.md` is theirs to change.
12. **Every unblock has a "Checked" section: each premise about the code,
    the run or the machine, with the command that checked it and its
    output.**
    - In the same change, `Reverses:` names only an earlier answer, and a
      changed plan or design text is named as "Amends:".
    - Evidence: 10 of the 11 reversed answers rested on unchecked premises.
      23 of 156 unblocks quote a command (section 5, item 6).
    - Changes: development.md 10.3 (the unblock's shape) and 10.4.
    - Moves: `answers_reversed` down.
13. **A step whose Scope is to find a cause gets a budget in its session
    file, in runs and minutes, and its implementer asks only past it.**
    - Evidence: 06.s2's 14 allowance blocks in 105 minutes, and 17 such
      blocks in all (section 5, item 10).
    - Changes: development.md 10.4 (the short session file of a supplement
      names its budget); 8.2 takes README rule 11 (diagnostics never
      committed).
    - Moves: `escalations` (process) down.
14. **Before `land`, the implementer runs once, in the build lane, the
    focused tests of the crates that depend on what it changed (`heavy
    cargo nextest run --workspace -E 'rdeps(<crate>)'`), and runs clippy
    again after any edit that follows its last clippy, a conflict's
    resolution included.**
    - Evidence: 54 gates failed on tests that no focused run included, and
      10 of w1–w35's 12 clippy failures followed an edit after the last
      clippy (section 5, item 7). The focused suite runs in 4–10 s.
    - Changes: development.md 8.1 (step 5) and 8.3. "Whole suites run only
      in `land`'s gate" names this exception.
    - Moves: `gates_per_step` and `queue_wait_ratio` down.
15. **The run's checks of shape move into `land` and the card.**
    - `land` refuses a tip whose last paragraph lacks `Plan:` and `Step:`.
    - `land` runs the plan's Pulse-word check on the diff.
    - The coordinator checks a landed step on its branch only while the
      worktree is still on it, as README rule 10 says.
    - Evidence:
      - 12.1 and 10.s1 carry trailer exceptions to the end;
      - rule 12's grep kept every Pulse word out after 16:40, but only
        because each implementer remembered to run it;
      - rule 10 served every session.
    - Changes: `land`; development.md 7.3 and 8.4.
    - Moves: `escalations` (process) down.
16. **`plan pulse` checks the coordinator's record.**
    - It lists headings and Kind values that sections 7.3, 10.2 and 10.3
      do not name, and lines of `context.md` over 200 characters.
    - `context.md` is rewritten whole, and nothing keeps its earlier
      versions.
    - Evidence: 21 "Unblock handled" and 2 "Steer handled" headings, 14
      invented kinds, 179 pasted contexts, and six metrics misread (section
      5, items 8 and 12).
    - Changes: `plan pulse`; development.md 7.5 and 14.2.
    - Moves: `log.md`'s growth down; the metrics comparable.
17. **`plan metrics` is fixed as section 5, item 12 lists.**
    - It counts each Claude Code message once across sessions.
    - It reads the coordinator's headings, as proposal 16 fixes them.
    - It counts a split step as added.
    - It takes the last code commit's `Gate-Result`.
    - It matches `.s<k>` steps and tips, not intermediate commits.
    - It splits lines into production, tests, generated and data.
    - It counts the gate lane's gates apart from its live runs.
    - It reads the run's end from `done.md`'s "Recorded" line.
    - Evidence: section 3.
    - Changes: the kit's `numbers.sh`; development.md 15.2 (`lines` and
      `gates_per_step` defined so).
    - Moves: no metric. It makes them comparable.
18. **The supervisor runs each "Done when" command once after `done.md`,
    through `heavy`, and appends their output to it; a "Done when"
    command reads `main`'s code or its plan's commits, never the newest
    commit.**
    - Evidence: nobody ran them. Item 5's command now prints "none". Item
      3's passes with four "none" lines (section 5, item 11).
    - Changes: development.md 7.6 and 5.2.
    - Moves: no metric. "Done when" verified.
19. **The designer takes into the design the rulings that changed what
    the code must do.**
    - 13.14, .91, .98 and .99 go into benchmarks.md.
    - 13.116's effort bound goes into limits.md.
    - 13.132's "large effect" goes into benchmarks.md 3.1.
    - 13.41's private Codex goes into benchmarks.md 10.
    - 13.39's second child signal is a question for skein's io.md.
    - Evidence: section 6. done.md calls five decisions "not in the design
      yet".
    - Changes: smith's and skein's design, by the designer.
    - Moves: no metric. Decisions outside the design, down.
20. **When the doorbell finds no turn in progress, the supervisor runs
    `plan resume` itself, and records the cause from the coordinator's
    last event.**
    - Its alerts to the user also go to `alerts.md` in the run's
      directory, which `plan status` prints first.
    - `plan check --fix` fast-forwards a clean primary checkout that is
      behind `origin/main`.
    - Evidence:
      - the crash at 06:01 was logged as "the user's request";
      - the push was refused because the terminal was active;
      - the supervisor repaired two primary checkouts by hand (section 5,
        item 13).
    - Changes: development.md 7.6 and 10.5; `plan status`, `plan check`.
    - Moves: `restarts`' lost minutes down.
21. **A proving plan records each calibration in the probe's task file,
    and its "Done when" counts the probes calibrated there.**
    - Evidence: four of session 13's seven steps calibrated nothing, and
      Done when 3 passed. Only one task file has a `[calibration]` block
      (section 5, item 10).
    - Changes: the plan's shape, and smith's benchmarks.md (the
      designer's).
    - Moves: `proofs` (live) toward calibrated.

## 9. Debts left in the code

For smith's `docs/development/tech-debt.md`, beyond what it already lists.
Paths and lines are at smith `a8f2b7e3` and skein `bec13dcc`; each debt
names the step that left it. Ordered by what they cost the next plan.

- **Compaction answered with a call or with prose** (08.6, 08.s1).
  - The summary request keeps the conversation's tools and tool choice
    (`crates/smith-domain-session/src/compaction.rs:107`, `:256`).
  - A summary that calls a tool ends as `NoSummary::Malformed` (`:323-331`).
  - 5 of 10 attempts in 12.6 failed so.
  - The default threshold, 4/5 (`smith-agent-service/src/profile/quantities.rs:184`),
    waits for the user.
- **The fixtures' protected `tests/`.**
  - `benchmarks/tasks/fixtures/{01-backoff,02-line-decoder,03-planner}/task.toml:9`
    protect `tests/`, while their prompts ask for regression tests.
  - All 30 attempts of 12.4 failed the scope check, for both agents.
  - The comparison cannot pass until the tasks get a new version.
- **Calibration lives in commit bodies.**
  - Only `benchmarks/tasks/probes/credentials/task.toml:48` has a
    `[calibration]` block, with no failing control.
  - `benchmarks/src/fix.rs:14` `calibration_suite(_probe)` ignores its
    argument (00.29).
  - The probes suite runs calibrated and uncalibrated probes alike.
- **smith-bench keeps four smith faces, and renders the product's settings
  through the old template.**
  - `benchmarks/src/agent/smith/events.rs:54-63`.
  - `prepare.rs:126-169`.
  - `settings.rs:98-172`, converted by `product_document`, `:207-279`
    (10.15, 10.16), through a hand-kept list of field names.
  - The pins `benchmarks/agents/smith/{smoke,first-request}.json` keep the
    old shape.
- **Benchmark arms run with sub-agents on** (`benchmarks/src/agent/smith/settings.rs:164`,
  `:270`). The product's default is off (`smith-local-shell/src/settings/layers.rs:11`).
- **Three price catalogs that disagree, up to twelve times** for one model:
  - `benchmarks/agents/prices.toml:2-10` (12.9);
  - `crates/smith-local-shell/src/presets.rs:109-120` (10.13);
  - the pins.
- **Parallel types and second mechanisms.**
  - Three account-source enums: `smith-local-shell/src/credentials.rs:79`
    (10.19); `smith-local-service/src/credentials.rs:22`, `:59` (10.7).
  - The effective-limit projection written twice:
    `smith-agent-service/src/profile/effective/channel.rs:7-174` (04.21) and
    `smith-local-service/src/profile/effective/channel.rs:9-163` (04.22).
  - Startup paths resolved three times: `compose.rs:408-413`,
    `compose/files.rs:43-45`, `credentials/sign_in.rs:172-174`.
  - The teardown bound derived again in smith-bench, `settings.rs:175-189`.
  - Operator words outside `smith_events::words`:
    `smith-local-shell/src/front/interactive.rs:172-230` (10.24).
- **The binary parses its command line three times**
  (`src/main.rs:8-16`: `credentials`, `compose`, `check`), in an order
  that decides what runs.
- **A String-typed face beside the typed refusal.**
  `crates/smith-agent-shell/src/config.rs:231`, `:246`, `:304`, `:876-891`
  return `Result<_, String>`. `config/refusal.rs:1-23` keeps it "until that
  face is removed", and no step removes it.
- **A known notice sent as an unknown one.** `NoticeKind::Unknown(b"not_contained")`
  (`smith-agent-service/src/events.rs:222`, 03.10), matched on its bytes in
  `front/interactive.rs:177-180`.
- **Literal bounds repeated.**
  - Effort, 32 bytes, in five places: `smith-domain-run/src/charter.rs:420`,
    `smith-domain-session/src/compaction.rs:38`, and three more.
  - Model name, 128 bytes, at `compaction.rs:37` and `session.rs:2005`.
  - The codec ceilings own both.
- **Hotspots grown in small steps.**
  - `smith-local-service/src/process.rs`: 918 → 1,420 lines.
  - `service.rs`: 846 → 1,339.
  - `smith-agent-service/src/service.rs`: 2,589 → 2,905.
  - `responses.rs`: 1,534 → 1,852, and its `fact` at `:500` grew from 135
    lines to 235.
  - Crossed 800 during the run: `benchmarks/src/batch.rs` (954),
    `smith-local-shell/src/local_host.rs` (870),
    `smith-protocol-channel/src/translate.rs` (871).
  - New over 800: `smith-local-protocol/src/store/machine.rs` (818).
  - 50 new production functions over 60 lines, most just under clippy's
    100.
- **New lint exceptions in production.**
  - `wildcard_enum_match_arm`: `smith-local-service/src/credentials.rs:441`
    and `credentials/{private.rs:106,transport.rs:88}` (10.6, 10.7).
  - `manual_map`, module-wide: `smith-local-service/src/stream.rs:5` (03.7),
    and `translate.rs:573`.
  - `disallowed_methods`: `smith-events/src/words.rs:12`.
- **Panics on a wrong variant.**
  - 46 `unreachable!` in production, e.g. 8 in `store/machine.rs:318-633`
    (10.1).
  - In skein, 02.6's `Done::Window` added such arms in
    `crates/skein-io/src/file_layer/store.rs:610-658` and `layer.rs:393-578`.
- **Tests.**
  - Several behaviours under one name: `benchmarks/tests/events.rs:1458`
    holds 33, and is over 1 s; `offline.rs:188` holds 10.
  - The harness's offline tests grew from 99 to 155, so later.md's "95
    tests, about 8 s" is out of date.
  - Real-clock limits of 1 s in `tests/end-to-end/support/mod.rs:553`,
    `:641` and `support/login.rs:96`.
  - `tests/local-process/src/oauth.rs:138` still accepts the removed
    driver's page.
  - World constructors of 150–300 lines: `tests/local/src/world.rs:458`
    `with_mode`, 301; `tests/local-process/src/referee.rs:88` `observe`,
    231.
- **Words.**
  - "native" and "legacy" survive the removed face: `settings/resolve/relationships.rs:179`
    `fn native`, and `tests/end-to-end/support/live.rs:76` `fn legacy`.
  - `benchmarks/src/probe.rs:34` keeps "Immutable", landed before rule 12.
- **Raw terminal input without a user.** skein's `crates/skein-shell/src/terminal.rs:28-60`
  and `ring/terminal.rs:33-61` (02.6) have no smith caller, since 10.10
  kept the terminal's own line mode.
- **Formats changed under one version.**
  - Thirteen steps changed channel 2, charter 2, transcript 3 and events 1
    in place, as README rule 5 allowed.
  - smith-bench keys its readers to commit hashes (`faces.toml`).
  - The generated codecs move with every skein repin: 10.s2 regenerated
    about 751 unrelated lines (13.105).

## 10. Decisions

2026-10-14.
The user delegated these decisions (2026-10-14: 'take reasonable decisions'); the designer took them, and the user may revise any row.

| # | Decision | Where applied |
|---|---|---|
| 1 | adopted in part: as the parity plan's own rule, until development.md takes it | the parity plan's README |
| 2 | adopted in part: as the parity plan's own rule, until development.md takes it | the parity plan's README |
| 3 | adopted in part: the planner's records in `findings/`, as the parity plan's own rule, until development.md takes it; `plan check`'s refusal deferred until after the parity run, since a change to `plan` does not land right before an unattended run | the parity plan's README and `findings/` |
| 4 | adopted in part: implementer.md 8.2 now lets through, named in the commit body, a new variant's arms, the case the Shape changes, a value printed differently, a value the Shape makes invalid, a version's sweep and a fixture's inputs; a removed name replaced by its successor and calls adapted to a new signature, past 8.2's 20 lines, as the parity plan's own rule, until development.md takes it | skein 479f836: implementer.md 8.2; the parity plan's README |
| 5 | adopted in part: a session whose peak context passes the limit hands over at its next landing; `plan contexts`' mark by the peak deferred until after the parity run (a change to `plan`) | skein 7f4b0e8: development.md 7.3, step 4 |
| 6 | adopted in part: a fresh designer past 150,000 tokens or an hour idle, not one per question: a question within the hour reads a warm cache, and the numbers are section 13's open question on the roles' limits | skein 479f836: development.md 10.4 and 13; `plan wake` |
| 7 | adopted | skein 479f836: development.md 2.5 and 10.5; supervisor.md 7; `plan handover`, `plan watch`; at 300,000 tokens rather than 400,000 |
| 8 | adopted | skein 479f836: development.md 14.1 and supervisor.md 6 (a fresh sub-agent returns the entry); skein 7f4b0e8: development.md 14.1, step 1 (the pulse unfiltered, `plan downstream`'s lines read before the entry) |
| 9 | adopted in part: each document names its wait (implementer.md 8.3, the tool's longest, 60 seconds on Codex; development.md 7.4, `wait_agent` at 300,000 ms), and an idle coordinator ends its turn; the 300-second `write_stdin` and `test-plan --live`'s measurement deferred until after the parity run: of 71 `write_stdin` calls asking 300,000 ms in this machine's rollouts of 10-12 to 10-14, 70 returned within 60 seconds, many as a cell still running at 31, so the longer yield is not established | skein 479f836: implementer.md 8.3; development.md 7.4; `plan idle` |
| 10 | adopted in part: the implementer reads its card, its repository's `AGENTS.md`, its session file and the design sections its Read first names, and development.md only by section; skein's programming-model.md and testing-strategy.md by the sections the session file cites, as the parity plan's own rule, until development.md takes it | skein 479f836: implementer.md 8.1; development.md 7.2; the parity plan's README |
| 11 | adopted in part: the spawn prompt takes nothing beyond the Pass-on texts and a restart's findings, no graph or coverage evidence, whatever other instructions say; the profile's switch and `plan check --fix`'s exclude deferred until after the parity run: kit changes before an unattended run, and Codex's sub-agents may share the profile's servers, which implementer.md 8.1 tells the implementers to use. The user's global Codex `AGENTS.md` stays theirs | skein 7f4b0e8: development.md 7.2 |
| 12 | adopted | skein 7f4b0e8: development.md 10.3 (`## Checked`, `Amends:`); supervisor.md 3, step 3 |
| 13 | adopted in part: as the parity plan's own rule, until development.md takes it | the parity plan's README |
| 14 | adopted in part: clippy after the last edit is `land`'s build-lane pass, before the gate lane; the focused tests of the changed crates' dependents before `land`, as the parity plan's own rule, until development.md takes it | skein 479f836: development.md 9.1, `land`; the parity plan's README |
| 15 | adopted in part: a landed step's `git status` only while its worktree is on its branch; the trailers alone in the last paragraph; the Pulse-word check before `land`, as the parity plan's own rule; `land`'s refusal of a tip without `Plan:` and `Step:`, and its own Pulse-word check, deferred until after the parity run (a change to `land`) | skein 7f4b0e8: development.md 7.3, step 2; skein 479f836: implementer.md 8.4; the parity plan's README |
| 16 | adopted in part: `context.md` rewritten in place, its lines at most 200 characters, no earlier version kept, and `log.md` and `blocked.md` holding only the headings and kinds development.md names; `plan pulse`'s check of them deferred until after the parity run (a change to `plan`) | skein 7f4b0e8: development.md 7.5 |
| 17 | adopted in part: each Claude Code message counted once across sessions, the planner's included; escalations counted once; suites from each repository's gated commits; `land`'s gates apart from the gate lane's other tasks; Proof headings as written. The rest (split steps as added, other coordinator headings, `.s<k>` proofs, lines by kind, `done.md`'s recorded time, a restart's whole cause) deferred until after the parity run (a change to `numbers.sh`) | skein 479f836: numbers.sh; development.md 15.2 |
| 18 | adopted | skein 7f4b0e8: development.md 5.2 and 7.6; supervisor.md 8 |
| 19 | adopted | smith 97875b08: benchmarks.md 3.1, 3.2, 5.3, 8.3, 9 and 10 (rulings 13.14, .41, .44, .91, .98, .99, .132), protocol/limits.md 2.4 (13.116); skein bc26df2: io.md 6 (13.39) |
| 20 | adopted in part: the supervisor runs `plan resume` itself, once, when the ring finds no turn in progress; the cause from the coordinator's last event, `alerts.md` printed by `plan status`, and `plan check --fix`'s fast-forward deferred until after the parity run (changes to `plan`) | skein 7f4b0e8: development.md 10.5; supervisor.md 2 |
| 21 | adopted in part: each calibration in its probe's task file, a `[calibration]` block per task and per variant, and a probe without one gates nothing; a "Done when" that counts the probes calibrated there, as the parity plan's own rule, until development.md takes it | smith 97875b08: benchmarks.md 3.1, 5.2 and 11.5; the parity plan's README |

The rows deferred until after the parity run come back to the designer once that run's retrospective has landed.
