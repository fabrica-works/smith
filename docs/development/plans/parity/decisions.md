# Parity plan: decisions

2026-10-14. The decisions the parity plan rests on, taken by its designer
from the findings in `findings/` (skein's development.md, sections 4.1
and 10.4). Sections 1 to 8 are the design's; the design documents follow
them, and where this file and the design disagree, the design wins and
the disagreement is a question for the designer. Section 9 lists the
decisions with long-term consequences that the user may veto. Section 10
is the planner's: how the plan applies sections 1 to 8. The run's
rulings follow, under "Rulings", numbered from 101.

The user was away while this was designed, and asked every agent to use
good judgement and take reasonable decisions. So the designer decided
what development.md 10.4 gives it, and also what it would otherwise have
asked the user, taking the recommendation it would have made; each of
those is in section 9, with its alternative, for the user to veto. The
user's standing rulings hold: Rust for everything; every format
pre-release; no sign-in, no copied refresh token, nothing written in the
user's tool directories. Entry IDs (K1, T4, M6, ...) cite `findings/`.

## 1. The target

1. **Parity is non-inferiority on outcome, time and cost, over one
   `parity` run** (benchmarks.md, section 16). smith is on par at a
   commit when, against the pinned Codex at the working tier:
   - **O:** smith's passes, pooled over 29 attempts per arm, reach the
     lower bound of Codex's exact 95% interval times 29;
   - **FT, FC:** on the fixtures, the upper bound of the 95% interval of
     smith's time per solved task and cost per solved task, over
     Codex's, is below 1.20;
   - **RT, RC:** on `shell-cwd`, the upper bound for the ratio of median
     task walls is below 1.20, and for median common priced spend below
     1.50.

   Rationale: "none detected" cannot show equality (M3); per solved task
   counts every attempt and lets no fast failure look cheap (M6);
   pooling the fixtures with a stratified bootstrap reaches 0.82 and
   0.77 power at eight attempts per arm, where per-task criteria at five
   fail a truly equal smith on spend about half the time (M4); the
   repository task needs at least five attempts per arm for an interval
   at all (M8); the 1.50 margin on its cost guards against large effects
   only, as three to five repository attempts allow (M8). A truly equal
   smith passes the whole verdict about 0.69 of the time, a smith 1.2
   times slower about never (M, section 5).
2. **The verdict is one offline command over a committed summary:**
   `cargo run -p smith-bench -- parity benchmarks/summaries/<run>.json
   --commit <sha>`, exit 0 at parity, 1 not on par, 2 incomplete
   (benchmarks.md, 16.4). It needs summaries that keep each attempt's
   values (M10). A plan's "Done when" names it over a run on its final
   `main`.
3. **Tokens, cache reads, cached share and reasoning are diagnostics,
   never criteria** (benchmarks.md, 9 and 16.1). The fixtures' "0.2
   times Codex's cache" was a volume: Codex sends a fixed prompt of at
   least 12,288 tokens with every request, and smith about 1,536 (R1,
   M7). Cache reads are priced into the cost criterion.
4. **The common priced spend is priced from smith's presets** (the
   providers' list prices, shell.md 5.2 and 6.1) at the harness's commit,
   and the harness's own catalog, `benchmarks/agents/prices.toml`, goes
   (benchmarks.md, 8.1). The presets match the providers' published list
   prices for `gpt-6-astra` and `gpt-6-luna`; `prices.toml` does not, by
   8 times (astra's input) to 20 times (luna's output) (T6, live.md L3).
   measure.md M9 and tokens.md Q2 recommended keeping `prices.toml`
   because the summaries use it; the ratios move by at most 0.04 between
   catalogs (R7), so the published figures should be the true list
   prices, from the one catalog the design already names.

## 2. The comparison's conditions

1. **Each agent runs as it ships** (benchmarks.md, 1, 5.4, 6.1, 6.2).
   - smith: its presets' window, output and deadlines; its shipped role;
     its `memory`; `sub_agents` at its default (off); no budget beyond
     its run defaults. The pin holds the account's file and the trace,
     nothing else; a probe that needs other values names them in its
     task.
   - Codex: only the keys that keep an attempt isolated and unattended
     (no MCP servers, the root trusted, edits allowed, no approvals,
     the private caches writable), its model and effort. The pin drops
     `features.multi_agent` and `agents.max_threads`: whatever their
     defaults are, leaving them out runs Codex as it ships.

   Rationale: the milestone measured a smith no user runs: an empty role,
   half the window, an output of 4,096 tokens, sub-agents on, a 1 TB
   memory bound and headers the product never sends (T3, T4, P1). Each
   smith repository attempt compacted twice for that reason, at 37 to 60
   s a compaction (W2, K8).
2. **The working tier is `gpt-6.1-sol` at high effort** (section 9, item
   1). benchmarks.md 7 defines it as "the user's working model and
   effort", and the user's Codex configuration names `gpt-6.1-sol` at
   `high` (live.md L4), as does the workspace's coordinator. smith runs
   it on the borrowed login (L1). Its list prices are a fifth of
   `gpt-6-astra`'s (L3), so the plan's live budget buys five times the
   attempts. smith's presets gain the model (shell.md, 6.1), with the
   same window and output as the other Codex models. This answers the
   reliability plan's open item for the user (its decisions.md, 10.6),
   whose proposal was `gpt-6-astra`.
3. **Codex stays pinned at 0.160.0 for the plan's length** (M Q9).
   benchmarks.md 15 says what a new pin checks first.
4. **Delegation is measured by its own experiment, never by parity**
   (benchmarks.md, 11.3 `delegation`). The repository task's next version
   drops "Subagents may be used at your discretion" (T Q3): with it,
   Codex spawned two children in each attempt; smith as shipped cannot.
   The `delegation` experiment is not in this plan (later.md); the
   user's open decision on smith's default (the reliability plan's 07.13)
   is unchanged.

## 3. The benchmark tasks and the harness

These come first, and no live comparison runs before them: until they
land, no attempt of either agent can pass (T1, T2, M1).

1. **Fixtures, version 2** (T1, T14, M1): `protected` drops `tests`; the
   grader already holds the seed's tests, so a weakened test still
   fails; each reference adds the test its prompt asks for; the task
   declares its deciding checks (benchmarks.md, 3.2: end, grader,
   protected files, tree's end). `validate` applies a task's protected and
   outcome checks to its reference.
2. **`shell-cwd`, version 2** (T2, T10, T13, W6): the protected check
   reads the bundle the workspace was copied from; `setup` warms the
   build outside the timer; the prompt drops its delegation sentence and
   says that temporary files go under `$TMPDIR`; Codex's sandbox can
   write the private caches.
3. **Comparison tasks name no budget** (T5; benchmarks.md, 6.1); `check`
   refuses one whose budget is below its deadline.
4. **Pins become the product's settings layers** (T4, T8, F1;
   benchmarks.md, 6.2): the pre-product faces, the JSON templates and
   their translation go; the smoke's `first-request` runs with its own
   task's overrides, not a brief that asks for a file the seed lacks; the
   baselines on earlier faces are retaken or dropped.
5. **Calibration lives in task files** (T7, F13; the reliability
   retrospective's proposal 21; benchmarks.md, 3.1 and 5.2): one
   `[calibration]` block per task and per variant, written by
   `calibrate`, with the variant's digests so that a new task version
   keeps an unchanged variant's calibration. A probe gates only when
   calibrated there; `smoke` lists only calibrated probes.
6. **The summary keeps each attempt's values, and the harness reports
   what it can see** (M10, W8, T11, T12, T13, T10): per-attempt values;
   the time record (8.1); Codex's linked children counted; Codex's
   binary pinned by digest; the repository's large effects computed; the
   user's `/tmp` entries listed before and after; the `parity` section
   and command (16.4).
7. **The reliability run's rulings that changed what the code must do
   are in the design** (its retrospective's proposal 19): 13.14 in
   benchmarks.md 3.2; 13.91 in 9; 13.98 in 8.3; 13.99 in 5.3; 13.132 in
   3.1 and 9; 13.41 and 13.44 in 10; 13.116 in protocol/limits.md 2.4;
   13.39 in skein's io.md 6 (skein `bc26df2b`).

## 4. Compaction

1. **A compaction keeps the tools and asks with `ToolChoice::None`**
   (domain/session.md, 5 and 8.2; K1). Every compaction on record was
   asked with the conversation's choice, `auto`; 9 of 37 on the small
   tier answered with a call, and the 8 with no text failed the run as
   malformed. Rejected: dropping the tools, which changes the prefix and
   loses every cache; keeping `auto` with stronger words, which failed 9
   of 37 times; a dialect's choice, which moves a domain rule into the
   protocol layer. On Codex the choice is outside the cached prefix (one
   large record read 51,840 of 51,954 tokens); on Anthropic a changed
   choice loses the messages' cache once per window, a cost accepted and
   to be measured when smith runs there. skein's fake follows the Codex
   record (skein's fake-llm.md, 2.1).
2. **The summary is framed:** its text blocks joined by a blank line,
   and the new window's message is the task, the run's summary preamble,
   the summary, then the held-back messages (K3).
3. **The summary instruction says** that the work goes on in a new
   window opening with the task's own words; to answer with text and call
   no tool; to keep exact values, names and paths as read, what remains
   and the decisions taken; and not to restate the task or its required
   answer (domain/run.md, 3.3; K2, K4). The run owns the words.
4. **A window is still compacted once, neither retried nor nudged**
   (K Q2). With `None`, 0 of 80 recorded requests answered with a call.
   Revisit only if `history/small-window` version 2 shows a malformed end
   under `None`.
5. **The threshold stays 4/5** (section 9, item 5; the reliability
   plan's decision 4.7, still the user's). Nothing measured favours 3/5,
   which compacts more often, and each compaction is lossy (K5). The
   `compaction` experiment is run again on `history` version 2 after
   these fixes, ten attempts per arm, and its summary is put to the user.
6. **`history`, version 2** (K6, F6; benchmarks.md, 11.1): the answer's
   form stated whole; the values out of order; 32 files kept, so that the
   80% case still compacts just before the last read; a
   `compaction-choice` event check, which the parent binary fails on every
   attempt that compacts, so that the fix has a certain before.

## 5. Wall time

1. **Long commands become jobs** (domain/tools.md, 3, 5 and 8;
   domain/session.md, 5; protocol/llm.md, 3; protocol/events.md, 3.6;
   protocol/limits.md; W1). `shell` gains `yield_seconds`; a command still
   running then is answered `running` with its job's number, and goes on
   in its process group under its deadline; a `job` tool reads what it
   wrote since, waits up to `job_wait` for its end, or stops it. On the
   repository task smith spent 167 s an attempt blocked in shell commands
   against Codex's 39, because Codex starts its builds in the background
   and polls them: 76% of its cargo time ran outside any call. The design
   had named the tool as open "after process groups", which have landed.
   Without it, the window fix and smith's default of no children project
   the three attempts to 1.10, 1.18 and 1.59 times Codex's median (W,
   section 2.4, less each attempt's wait on children), too wide for RT's
   upper bound to fall below 1.20.
2. **A job is its session's write in progress:** whatever takes the
   lease (a writing child's call, a check, a delivery) first cancels it,
   and a session's close cancels its jobs and tells each end once
   (`job.ended`). At most `jobs` (4) run per session. A job does not
   survive its activation. Writing to a job's standard input stays open.
3. **Asynchronous children are not in this plan** (W3; later.md). smith
   ships with `sub_agents` off, and parity compares it as shipped
   (2.1), so a child's blocking costs parity nothing. domain/run.md 15
   keeps the item.
4. **The path bound says what it counts** (W4; domain/tools.md, 4;
   protocol/llm.md, 3): each name, `.` and `..` with one separator, never
   repeated separators. The one failed smith grade traced to a child's
   wrong claim about it.

## 6. The role and the tools' text

1. **smith ships a task role** (shell.md, 5.2 and the new 6.6; P1, P2):
   its own words, at most 4 KiB, on the topics 6.6 lists, naming only
   smith's tools. It counts in the window's host part, as a charter's
   instructions do, and a startup check holds it there beside a guide at
   its bound for each directory (protocol/limits.md, 4,
   `host-part-holds-the-role`): with an Anthropic model declared, the
   host's part is about 40 KB, so 8 KiB beside two 16 KiB guides could be
   refused at the entrance. Today's default is one 96-byte line, and the arms ran
   with none, against Codex's 21,429 bytes of instructions. Its proof is
   the compare suite before and after (benchmarks.md, 4.2), smith's old
   role against its new one.
2. **The tool descriptions state units, the pattern's kind and the
   working directory** (protocol/llm.md, 3; P3, P4): seconds for
   `timeout_seconds` and `yield_seconds`; `search` runs `rg` with a
   regular expression; `.` and the empty path both name the working
   directory, and the decoder takes the empty path as it. In 6 of 15
   fixture attempts smith's first call was refused for an empty path.
3. **The edit format is not in this plan** (P5; later.md). The fixtures'
   time and cost per solved task already sit within 1.20 of Codex's
   (1.073 and 0.855 on the 2026-10-14 runs, M6); the `edit-format`
   experiment runs after the plan's first parity run, if FT or FC fails
   there.
4. **What a failed call in a batch means for the calls after it** stays
   open (P Q5; domain/session.md, 11).
5. **The cap on tools run together is named** (F3): `parallel_tools`, 4,
   a policy value (protocol/limits.md, 2.3; shell.md, 6.5).

## 7. Probes and defects

1. **The plan calibrates, in their task files:** `history` version 2,
   both variants (4.6); `tool-output/background`, new (benchmarks.md,
   11.1); the smoke's `first-request` with its own overrides (3.4);
   `tool-output` version 3 and `process-tree` version 2, which need only a
   run (F10). Their versions follow benchmarks.md 5.2.
2. **Not in this plan** (later.md): `cache-reuse` version 2 (R4, F5),
   `parallel-read` version 2 with a copied answer (F3), `paths` version 3
   (F2), `follow-up` version 2 (F7), `large-write/oversized` proven by a
   world (F8), `budget` version 2 (F9), `slow-completion`'s harder prompt
   (F11), Codex's turn-state routing (R2: misses cost about 12% of spend
   on two fixtures, and FC already holds; decisions 4.4 of the
   reliability plan still asks for a measured gain first).
3. **Two product defects are fixed with tests** (smith's
   `docs/development/workflow.md`, section 5):
   - `list` at depth 2 or more failed 8 times with "operation failed: io
     error" on the repository task (W, P); the plan finds the cause, with
     a diagnostic budget of its own. A lead, unverified: the machine
     allows at most 8 processes at once and answers a full table with a
     fault that renders as that text; protocol/limits.md 3.5 now derives
     the machine's processes from the batches and jobs they must hold;
   - `smith exec` in a repository without a commit says "Repository head
     unavailable" and then hangs until killed (live.md L2): a start that
     cannot read a writable repository's head is refused, and the command
     exits 2 with its last line (shell.md, 4.3).
4. **Memory needs no change** (F12): the new model has the same window as
   the other Codex presets, and `jobs` adds at most `jobs` kept outputs
   per conversation. A step that raises a window or the count of
   conversations reports `smith check`'s footprint.

## 8. Order, live budget and scope

1. **Order of work:**
   1. the harness and the tasks (section 3), the price basis (1.4), the
      `parity` command (1.2), the new model's preset (2.2): offline, no
      live run but the smoke;
   2. compaction (section 4), proven by `history/small-window` version
      2's calibration and its before;
   3. a first `parity` run on `main` as it then stands: the baseline that
      shows which criteria fail, and against which every later step
      proves itself;
   4. the role and the tools' text (section 6), and the two defects
      (7.3), proven by the compare suite before and after;
   5. jobs (section 5), proven by `tool-output/background`'s calibration
      and one `shell-cwd` check, five attempts per arm;
   6. the `compaction` experiment again (4.5), for the user;
   7. the closing `parity` run, on the plan's final `main`.
2. **Live budget for the run: at most 340 attempts and $45 of smith's
   notional spend at its presets' list prices**, Codex's attempts
   included in the count and spending the user's Codex plan, never
   smith's notional figure. Estimate: two `parity` runs, 116 attempts and
   about $20; one `shell-cwd` check, 10 and about $6; the role's before
   and after, 30 and about $2; the small tier's calibrations, the
   `compaction` experiment, the per-landing smokes and the fixes' befores
   and afters, about 190 and under $3. The supervisor stops live work and
   asks the user before either bound would be passed.
3. **Repositories:** smith and skein (skein's fake LLM, 4.1). jig and
   temper are read-only and not repinned: the user reserved them for
   their own plans when this was designed, and their repins wait for
   jig's and temper's plans (development.md, 5.7's step is "none" here,
   with this reason).
4. **What the plan must not decide differently:** the criteria and their
   margins (1.1), the price basis (1.4), each agent as it ships (2.1),
   the working tier (2.2), and the compaction's `ToolChoice::None` (4.1).
   A change to any of them is a design question.

## 9. For the user

Each of these is the designer's decision, taken while the user was away.
Each is hard to undo once a published comparison or the product rests on
it. To veto one, say which; the alternative is named.

1. **The working tier is `gpt-6.1-sol` at high** (2.2), so smith is
   measured on the model the user's own Codex runs. Alternative:
   `gpt-6-astra` at high, the reliability plan's proposal (its 10.6),
   which the 2026-10-14 milestone measured, at five times the price.
   Changing it later is one commit to `benchmarks/agents/models.toml` and
   new baselines.
2. **Each agent is compared as it ships** (2.1). Alternative: both with
   equal capabilities (both delegating, same window), which measures
   mechanisms rather than products.
3. **"On par" means the criteria and margins of 1.1:** not worse than
   Codex by more than 20% on time and fixture cost per solved task, 50%
   on the repository task's cost, and an outcome within Codex's exact
   interval. Alternative: tighter margins (1.10), which need about 30
   attempts per task and arm, or looser (1.25), which lets a smith 1.2
   times slower pass 2 times in 100.
4. **smith gains jobs, a second shape for its shell tool** (5.1): models
   and users will come to rely on it, and it is hard to take away.
   Alternative: no jobs, and a repository task that stays about 1.3
   times slower than Codex.
5. **The compaction threshold stays 4/5** (4.5; the reliability plan's
   4.7, routed to the user on 2026-10-14). Alternative: 3/5. The plan
   runs the experiment again after the compaction fix and reports it.

6. **The plan's live lines, and what yields to the closing run** (10.2).
   12.3's calibrations of `tool-output` version 3 and `process-tree` run
   only while 64 attempts are left for 12.4 and its reruns. Alternative:
   run them before 12.1, at the cost of a closing run that may not fit.
7. **The committed summaries and baselines of version 2 leave the tree**
   (10.6): 42 summaries and 11 baselines, kept in git's history.
   Alternative: an archive directory that `check` skips, read by nothing.
8. **A calibration with no runnable control stands on its passes** (10.9;
   benchmarks.md 3.1, as landed for this plan). Alternative: such probes
   stay uncalibrated, and `suite smoke` holds only probes with a control.
9. **The smoke suite after the plan holds `first-request` only,** with
   whatever 12.3 calibrates outside it: `smoke-tools`, `large-write`,
   `budget/child-quota`, `budget/wrap-up` and `failures/one-turn` lose
   their calibrations with the version 2 summaries (10.6), and wait in
   `later.md`. The per-landing `smoke-tools` run is unchanged.
   Alternative: recalibrate those five in this run, about 60 attempts
   more than the 340.
10. **The `compaction` experiment's result,** which 12.2 reports, for the
    user's decision 4.5 (the threshold).

Still with the user from the reliability plan, unchanged by this one:
borrowing another CLI's login (its 10.2), the smoke before `land` (its
10.4), the skein pin tags (its 10.5), and local `sub_agents` on by
default (its 10.7).

## 10. The plan's decisions

2026-10-14, by the parity plan's planner, within sections 1 to 8. None
changes what section 8.4 holds.

1. **The plan's shape** (README, "The sessions"). Thirteen sessions:
   twelve in smith, one in skein, each of at most six increments. The
   harness is cut into four sessions with few shared files (01 tasks, 02
   pins, 03 results, 04 probes and verdict) so that the baseline opens at
   its fifth step; jobs into four (08 vocabulary and limits, 09 machine,
   10 domains, 11 offered), so that the transcript's version 4 (11.1, from
   10.1's sketch) lands at once and the machine runs beside the domains. Each format moves its version once, by a named
   increment (README, rule 12).
2. **The live budget is $50 and 340 attempts** for the run, as the user
   set it on 2026-10-14, in place of 8.2's $45; Codex's attempts count,
   its spend does not. Each step has its line (README, "Live services"),
   every live landing carries a `Live:` trailer, and the run's total is
   summed from them (README, rule 10). Past 340 attempts or $50, live work
   stops and the supervisor reports to the user; offline work goes on.
   The closing run keeps its 58 attempts and 6 for reruns: 12.3's
   calibrations run only within what is left (item 6 of section 9).
3. **Section 8.1's order, kept where a dependency is true.** Steps 1 and
   2 come before the baseline (12.1) by its Needs; steps 4 and 5 after it,
   by README rule 11's fence on the increments that change what the model
   is sent (06.1, 06.2, 09.3, 11.2); step 7 last. Step 6, the `compaction`
   experiment, needs only compaction's fix (07.3): jobs change nothing
   `history` exercises, and a parallel plan keeps true dependencies only
   (development.md 5.1).
4. **`gpt-6.1-sol`'s rates** (shell.md 6.1: "the code holds the
   numbers"): $2 fresh, $0.10 cache read, $10 output per million tokens,
   from the provider's list (findings L3); cache writes at $2.50, 1.25
   times fresh, as the other Codex presets price them; past 272,000 tokens
   of window, twice the fresh and cached rates and 1.5 times the output's.
5. **The role and the descriptions, proven together.** The task role's
   topic 5 names jobs only once `job` is offered (11.2), since the role
   names only tools smith offers (shell.md 6.6). The descriptions' facts
   (06.1) and the role (06.2) are measured by one `compare` run of two
   smith arms, 06.1's parent and 06.2, the designer's 30 attempts (8.2).
   `host-part-holds-the-role` runs where the shell gives the task role,
   which passes its bound to the profile; a profile composed without it
   skips the check.
6. **What leaves the tree.** The 42 committed summaries of version 2 go
   when summaries move to 3 (03.1): their raw results are of result
   version 2, which version 3 refuses, so none can be regenerated
   (benchmarks.md 8.1). The 11 baselines predate the product face and go
   with the summaries they were cut from (03.1; benchmarks.md 6.2); 12.4 cuts smith's and Codex's from the
   closing run. `credentials`' calibration block lacks the new fields and
   names a removed summary: it goes (04.1), and `credentials` waits in
   `later.md`. `benchmarks/agents/prices.toml` goes (03.1; 1.4).
7. **The `list` defect's cause is found.** The planner's audit traced it
   and reproduced it with this machine's `rg` 14.1.1: the listing parser
   requires byte order, and `rg --sort=path` sorts each directory by name
   (`findings/planning-audit.md`). The 8-process lead of 7.3 is true in the
   code, but no recorded failure reached it; 08.3 derives the machine's
   processes as limits.md 3.5 asks anyway. 09.3 fixes the order with a
   test that fails on its parent; no diagnosis step is needed.
8. **3.3's "below its deadline" reads as benchmarks.md 6.1 says:** `check`
   refuses a comparison task that names any `[budget]`, and one whose
   `deadline_seconds` is not below the run defaults' hour (01.4).
9. **A calibration with no runnable control.** benchmarks.md 3.1 asked a
   failure on the binary before the fix or on a control arm; for a
   behaviour older than the oldest face the harness runs (`0d5604f9`,
   6.2), neither can run. The planner closed the gap as the designer
   would, in its own design landing before this plan (smith
   `72311b8e`): such a block records why, and the calibration stands
   on its passes. 04.6 (`first-request`) and 12.3 calibrate so.
10. **run.md 3.3's order of sections** is the domain's to decide ("The
    domain decides what each says and in which order"), with the role
    first, which the code already does. The plan reorders nothing.
11. **The pins left** are smith's `standard` (trace `calls`) and one for
    the probes that read prompts (trace `everything`); every other value a
    probe needs is in its task's `[smith]`, `first-request`'s
    `instructions = ""` among them (02.1).
12. **Codex's pin** keeps `check_for_update_on_startup = false`, which
    keeps an attempt unattended, and drops `features.multi_agent`,
    `agents.max_threads` and `shell_environment_policy.inherit`; it trusts
    the attempt root, not the working directory, and runs the private
    package by its digest (02.3).
13. **A `not run` result keeps its tool choice.** A compaction asked with
    `None` after a narrowing to `Only` would render earlier history with
    the new choice (`prompt.rs:457`); 07.1 keeps the choice in force
    with the result in the session's history, so the cached prefix holds
    (session.md 8.2); the transcript's `not_run` record is unchanged.
14. **The repository check is a suite of its own,**
    `repository-check.toml`: `shell-cwd`, 5 per arm, smith against Codex,
    interleaved, for 8.1's "one `shell-cwd` check, five attempts per arm"
    (11.3; benchmarks.md 5.3).
15. **The reliability retrospective's lessons** (its section 10, rows 1,
    2, 3, 4, 10, 13, 14, 15 and 21) are the README's rules 1 to 9 and its
    "Done when" item 4, in this plan's words.
16. **Arms of one event vocabulary.** A `smith-bench` of one vocabulary
    version reads no other (events.md 6). So the increments whose parents
    are run as arms after 08.1, 06.1 (06.2's before) and 07.1 (07.3's
    control), need 08.1; 11.2's parent comes after it by its Needs.
17. **The reviews' corrections.** The consistency review and the literal
    reading (`findings/planning-consistency.md`, `planning-literal.md`)
    found 54 points in the plan as first written; each was corrected
    before the plan landed, among them: the summary's version moved with
    the result's in 03.1; the transcript's version 4 first, so that 07.2's
    window turn and 10.1's outcomes land under it; `JobEnd` owned by 09.2;
    rule 9's limits per command and its "stopped by the provider"; FC's sum
    of priced spend; a suite without Codex naming its reference.

## Rulings

101. **03.1 prices at the resolved model and window** (cross-session scope, smith 03's block of 2026-10-14T10:26:36Z).
     Ruling 101. Yes: in 03.1, `ModelIdentity` gains `resolved_window: Measure<u64>`, and the adapters fill both `resolved_model` and `resolved_window`, because 03-results.md's choice prices spend "by the model each attempt resolved and its declared window", and on `main` neither is observed: `probe.rs` sets `resolved_model: Measure::unavailable("not run")` and no adapter changes it. benchmarks.md 8.1 asks for "every agent's tokens priced on one basis: smith's presets' list prices (shell.md, 5.2 and 6.1)", and shell.md 5.2 tiers by "its window as settings leave it", so the price needs the window each attempt ran with.

     - `ModelIdentity` gains `resolved_window: Measure<u64>`, set `Measure::unavailable("not run")` where `resolved_model` is set today (`benchmarks/src/probe.rs:226-233`).
     - The smith adapter fills `resolved_model` and `resolved_window` from the main conversation's first `Model` record (`smith_events::Model`'s `model` and `window_tokens`); the Codex adapter from the root rollout's first `turn_context` (`model`) and first `token_count` (`info.model_context_window`). Each is unavailable, with its reason, where its stream lacks it.
     - `priced` prices the attempt's tokens with `list_prices` at the resolved model and window. It is unavailable, naming what is missing, when either is unavailable or the presets do not price the model. It is never priced from the requested model or a preset's window.
     - `resolved_effort` stays as it is. The result's version is the 3 that 03.1 already sets (README, rule 12): no further bump.
     - 03.1's Size becomes 230 lines; README rule 1 splits it if it passes that (the observation of model and window lands alone).
     - The session file now names `result.rs` (`ModelIdentity`), `probe.rs`, `agent/smith/events.rs`, `agent/codex.rs`, `agent/codex/rollouts.rs` and `tests/recorded/results/attempt.json` in 03.1's Files, and two more tests: `the_resolved_window_comes_from_each_agents_stream` (a recorded smith trace, and a recorded Codex rollout's 258,400) and `an_unavailable_window_leaves_the_priced_spend_unavailable`, under 20 ms each.
     - Adapt any further construction or reader of `ModelIdentity` that `rg -n 'ModelIdentity|resolved_model' benchmarks` lists, as README rule 3 says, and list it in the commit body under "Users outside Files:".

102. **process::Event takes its neighbours' large_enum_variant expectation** (cross-session scope, smith 11's block of 2026-10-14T10:35:14Z).
     Ruling 102. Neither boxing: `process::Event` takes the attribute its neighbours carry. In 11.1, put on `pub enum Event` in `crates/smith-host-domain/src/process.rs` the line

         #[expect(clippy::large_enum_variant, reason = "sealed payloads are priced by bounded queues and state")]

     the same attribute and reason as `process::Request` (`process.rs:49`) and as `boundary::Input` (`boundary.rs:10`), which holds `process::Event` inline as `Input::Process(process::Event)` and is already excepted for that reason. The inline `Up` is priced where its slot is priced, as today; boxing `TurnValue`'s value or `Event::Received.message` would move bytes out of the slot that the queues price by `size_of`, and change the turn accounting (`TurnValue::owned_bytes`) and its tests, which 11.1 does not own.

     - No field changes type, no value is boxed, and no byte count or limit changes.
     - README rule 7 now names this attribute as its one exception: its grep prints this line in 11.1's landing, and nothing else. Every other lint is fixed, as rule 7 says.
     - 11.1's Files now name `crates/smith-host-domain/src/process.rs` (`Event`).
     - If clippy reports `large_enum_variant` on a further enum that holds `process::Event`, `channel::Up` or `TurnValue` inline after `Turn` gains `jobs`, give it the same line, with the same reason, and list it in the commit body under "Lint exceptions by ruling 102:"; any other enum's lint is a question.

103. **the line resumes; kache gc by hand below 40 GB** (infrastructure, smith 03's block of 2026-10-14T10:45:23Z).
     Ruling 103. The line resumes: the disk has 64 GB available, above development.md 11.4's 40 GB below which new starts are held. kache's store had grown to 135.2 GiB against its 74.5 GiB cap because its timed eviction (`kache-gc.timer`, every 30 minutes) fails on a race, "Running eviction...Error: No such file or directory (os error 2)", and the daemon's own GC request times out; two hand runs of `kache gc` at 10:46 and 10:48 took it to 67.0 GiB.

     - Until the run ends, the coordinator runs `kache gc` itself whenever its disk check (development.md 11.4) reads below 40 GB, before it holds any start; if that run ends with "No such file or directory", it runs `kache gc` once more. It logs each run's line from `kache stats` ("Store: …") in `log.md`.
     - It still removes the `target/` of a session that is not running when that is not enough, as 11.4 allows.
     - Nothing else changes: the 40 GB and 15 GB thresholds stay, and the user's primary checkouts are not touched.

104. **05.1's Files take the adapter's model counts** (cross-session scope, smith 05's block of 2026-10-14T10:55:33Z).
     Ruling 104. Yes: 05.1 changes the six count assertions in `benchmarks/tests/smith_adapter.rs` from 2 to 3, because 05.1's Shape says "`MODELS` gains `codex/gpt-6.1-sol`" and these tests count the models and allowances that the startup derivation renders from the presets. The session file missed them; 05.1's Files now name them, so implementer.md 8.2's limit of 4 does not apply to them.

     - The six lines: `smith_adapter.rs:686-687`, `:765-766` and `:853-854`, each `assert_eq!(local.models.len(), 2);` or `assert_eq!(allowances.len(), 2);`, become `3`. The tests' names, inputs and other assertions stay as they are.
     - A further assertion, in any file, that counts the presets' rendered models or their allowances and expects the count before `gpt-6.1-sol`, takes the count after it; list each in the commit body under "Users outside Files:".

105. **a whole-version drift test's ceiling is 0.5 s** (plan defect, smith 08's block of 2026-10-14T11:03:11Z).
     Ruling 105. Yes: the drift test over a whole golden version has the focused world test's ceiling, 0.5 s; the new round trips keep 20 ms. testing-strategy.md section 5.6's ceilings read "a unit test under 20 ms, a focused world test under 0.5 s", and the drift test is in `smith-codecs-world` and reads every listed value of the vocabulary: its version-one form took 67 to 117 ms in this run's gates before 08.1 touched it, so "Under 20 ms each" was a plan defect.

     - 08.1: `event_records_and_every_listed_value_match_frozen_version_two_goldens` under 0.5 s; `a_job_ended_record_round_trips_its_fields` and `a_tool_completed_with_a_job_round_trips` under 20 ms each.
     - 11.1: `transcript_schema_matches_generated_code_and_all_goldens` under 0.5 s; its two round trips under 20 ms each. The session files now say so.
     - Take out the drift test's worker threads, as you propose; keep the exact receiving ceilings. Every frozen equality and row stays checked.
     - A further drift test over a whole golden version in this plan takes the same 0.5 s; name it in the commit body with ruling 105.

106. **smith 07's Shapes against main** (ready check of smith-07.1).
     The ready check of smith 07 (07.1 and 07.2 checked against main at bdb26312; 07.3's Needs not landed) corrected `smith/07-compaction.md`:

     - 07.1: Files add the code that makes and carries `NotRun::OutsideChoice` (`session.rs:2102`; the session's `llm.rs` `NotRun` and `ToolChoice`; smith-domain's own `NotRun`, `llm.rs:21`; `peer.rs:339`; `prompt.rs:436`) and the six tests that build or match it. Only `prompt_bytes` (`compaction.rs:107`) prices `None`, and only while `compaction.asking()`; `new_bytes` (`:256`) keeps the conversation's choice. The repin changes no cache count (333 tests of three worlds unchanged); the `None` compaction's commit gives any changed value. Size 120 lines.
     - 07.2: `Open` is `Opening` (`smith-domain-run/src/boundary.rs:643`); the summary gets its own join at `compaction.rs:341`, and `text_of` with its yield users stays as it is; Files add `translate.rs:67`, `:121`, `session.rs:186`, `:2025`, `:2055`, `compaction.rs:259` and `:209` (`+ 3`), the `Spec`/`Opening` literals and byte checks in five test files, and the two tests that read the old layout (`compaction.rs:138`, `window_referee.rs:73-82`); `profile.rs:490`, `restore :423`. Size 160 lines.
     - A version-4 window turn written before 07.2 comes only from the run's own builds; 07.2 handles nothing else.
     - 07.3: Files add `tests/local-process/src/world.rs:1089`, whose fake cue matches "Summarise the work so far".

107. **02.1 smokes afresh; a branch's setup refusal stops its step** (process, smith 02's block of 2026-10-14T12:00:49Z).
     Ruling 107. The smith line resumes now, and 02.1's corrected arm takes the smoke afresh. Both failures were 02.1's own: its harness change gave the check two event destinations, and the service's worst case, 2,159,054,753 bytes, passed the shipped `declared.memory`, 2,147,463,091, at setup, before any model request. `main`'s smoke passed on every landing of the last two hours, the last at 11:44 (`probes-smoke-tools-27579583-20261014T114406Z`), so smith on `main` is not what failed.

     - 02.1 runs README rule 8's smoke on its corrected arm as written: once, and once more on a failure. A second failure stops 02.1 on a question; it does not stop the line.
     - The shipped memory and every other preset stay as they are.
     - The one-line change to smith's product resolver (a file trace only when `trace.path` is set) is a small fix outside Files (implementer.md 8.2): name it in the commit body with the text of benchmarks.md section 6 it follows ("one event destination"), and the test that covers it.
     - README rule 8 now says, until the run ends: a second failure refused at setup, before any model request, on a branch that changes `benchmarks/src/agent/smith/` or `benchmarks/agents/smith/`, while `main`'s last landed smoke passed, stops its step on a question, and the line goes on. Every other second failure stops the line, as before.
     - The two refused attempts count in the step's `Live:` trailer (rule 10).

108. **a session's target goes after each landing; 6 sessions below 60 GB** (infrastructure, smith 03's block of 2026-10-14T12:34:13Z).
     Ruling 108. The line resumes once a disk check reads 40 GB or more (it read 48 GB at 12:44), under two rules that bound the sessions' build output, until the run ends. The cause is not kache: a session's `target/` grows to 17–21 GB over a few increments (`target/debug/deps` alone is 20 GB in smith 03's), not the 1–8 GB that development.md 11.4 assumes, and eight of them outgrow what kache's eviction frees between wake-ups. A fresh `target/` is 8–9 GB (smith 05's and 07's).

     - **After each landing,** the coordinator removes that session's `target/` before its next increment starts, and when a session yields, hands over or ends. kache refills it from its store: "it gains most when worktrees are new" (development.md 11.3). The implementer is told this once, so that it does not wait for a warm `target/`.
     - **While a disk check reads below 60 GB,** at most 6 sessions run at once; at 60 GB or more, 8 again. Below 40 GB new starts are held, and below 15 GB the line stops, as 11.4 says.
     - Ruling 103's `kache gc` below 40 GB stays.
     - Nothing outside the run's own directories is removed by the run: the user's checkouts and `~/src/rust/smith-benchmark-2026-10-08/` stay unless the user says otherwise.

109. **the queue restarted after the full disk** (infrastructure, smith 07's block of 2026-10-14T12:38:47Z).
     Ruling 109. The queue is back and the line resumes, under ruling 108's limits. pueued stopped at 12:33:21 because it could not save its state on the full disk ("Failed to write temp file while saving state. No space left on device"), and systemd gave up after six restarts. Its state file was whole (3,589 tasks); the supervisor removed the empty `state.json.partial`, restarted the service, and ran `plan check --fix`, which started the lanes.

     - Tasks 3785 (smith's land gate) and 3786 (smith 03's `cargo nextest run`) were killed at 12:32; tasks 3788 (smith 04's land gate), 3789 (smith 05's smoke) and 3790 (smith 07's land prebuild) were queued with no process waiting on them, and were removed. Each session runs its interrupted command again, from the start: a rerun the disk caused counts as no retry of its own.
     - The 2026-10-08 benchmark's `target/` directories are gone, by the user's leave (about 50 GB): the disk reads 167 GB. Ruling 108's rules stay.

110. **the machine derives tool_payload and prices patterns at it** (cross-session scope, smith 08's block of 2026-10-14T18:11:54Z).
     Ruling 110. Yes: the machine's `Limits` and `Derivation` gain `tool_payload`, derived from the tools domain's `tool_payload`, as you propose. limits.md 3.5 says "`smith-protocol-machine` derives from the tools' limits, never beside them", and limits.md 2.2 defines `tool_payload` as "the most a tool exchanges in one piece: a `read`'s largest window, a `write`'s content, the arguments of one call". A listing's or search's filter and pattern come from one call's arguments, so pricing them at `file_bytes` ("the largest file the tools hold whole") overcounts every process slot that 08.3's 42 processes multiply. This is the design's own bound, not a new quantity, and no design text changes.

     - `machine::Limits.tool_payload` and `Derivation.tool_payload`, from `tools::Limits.tool_payload`; `Check::MachineHoldsTools` refuses a machine `tool_payload` below the tools', as it does `file_bytes` (`profile/validate.rs:229-240`).
     - The machine refuses translated tool arguments above `tool_payload`; filter and pattern ownership is priced at it.
     - Printed as `machine.tool_payload`, by the existing scheme (README rule 12: no new version).
     - Shipped memory and every preset stay (decisions.md 7.4, "Memory needs no change").
     - A further per-step overcount (the pathname copies you name) may be tightened only where a heap test like your saturated eight-listing check shows the bytes; no preset or expected value changes. If the three stories that fail now (`smith_exec_ephemeral_removes_its_private_files_after_a_failed_answer`, `smith_exec_json_ends_accepted_with_its_events_and_exit_0`, `slash_effort_reaches_the_next_request`) still refuse at startup after that, stop on a question with the remaining bytes, and change nothing else.
     - Each `machine::Limits` literal and destructuring outside Files takes the new field set from the tools' `tool_payload` (README rule 3; `service.rs`, `tests/host/src/inline.rs` among them); list each in the commit body under "Users outside Files:".

111. **kache gc below 80 GB; one behaviour per test is rule 16** (steer of the 18:42 check).
     From the check of 2026-10-14T18:42Z, two standing rules, until the run ends:

     - **kache gc below 80 GB,** amending ruling 103's 40 GB. The 18:21 timed gc left kache's store at 66.9 GiB; by 18:40 it read 133.6 GiB against its 74.5 GiB cap, and the disk fell from 135 GB to 69 GB by 18:42, 3.5 to 7 GB a minute. The supervisor ran `kache gc` at 18:5x: "Evicted 1665 entries", store 67.0 GiB, disk 69 GB to 132 GB. The coordinator runs `kache gc` at every disk check that reads below 80 GB (each wake-up, each start, each target removal), once more if it ends with "No such file or directory", and logs its `Store:` line beside the disk check. Ruling 108's removals and caps, the 40 GB hold, the 15 GB stop and the 30-minute `kache-gc.timer` stay.
     - **One behaviour per test** is README rule 16: the steer of 17:44's point 1, after a third finding (`benchmarks/tests/parity.rs`, 04.4: `a_ratio_just_below_its_margin_holds_and_just_above_fails`, five rows and three outcomes; `missing_reference_passes_and_smith_lower_bounds_are_incomplete`, three verdicts; landed before the steer, and left as they are).

112. **calibration reads only deciding checks: 04.s1, then 04.6 afresh** (plan defect, smith 04's block of 2026-10-14T18:51:38Z).
     Ruling 112. Yes to both: a new step, `smith-04.s1`, makes `model_deviation` read only deciding checks, and then 04.6 calibrates afresh, with up to 12 more attempts. benchmarks.md 3.1 lets the arm's other attempts fail "only by the model, never by smith", and 3.2 says an unobservable passive check "decides nothing". Attempt 10 failed only `file-content:first.txt`, a deciding check that is the model's, but `model_deviation` (`benchmarks/src/probe.rs:142-162`) returns false at the first failed check it does not list, and it met `teardown` and `headroom/*`, unavailable and not in the result's `outcome.deciding`. So calibration stopped at 9 passes in 10 when 2 attempts could still reach 10 in 12.

     - `smith-04.s1`, "Calibration reads only deciding checks", in `smith/04-probes.md`: `model_deviation` reads only the checks `outcome.deciding` names; a failed deciding check that is not the model's still returns false. Two unit tests, the smoke, about 20 lines. Its other users (`batch.rs:506`, `probe.rs:132`, `:686`) take the corrected function as it is; `a_model_deviation_is_counted_apart_from_smiths_failures` (`tests/probe_command.rs:240`) keeps its expected values.
     - 04.6 needs 04.s1, then runs its command once, as its Shape says: at most 12 attempts, 15 minutes. Its row in "Live services" is now 22 attempts: the 10 of `probes-first-request-ecd28c01-20261014T184751Z`, which count in its `Live:` trailer, and these 12. Its notional row stays $0.05.
     - No probe's prompt, seed, checks or deciding list changes.

113. **smith 10's Shapes against main** (ready check of smith-10.1).
     The ready check of smith 10 (all four increments, against smith `main` at `7aa4fa03`; two sketches compiled under clippy) found the session file written before the code it names: a sketch that does not compile (`b"…".into()` into `Box<[u8]>`) and two `Copy` fact kinds that cannot carry a `Box<[u8]>` call id; `record.rs` where the outcomes' mapping is `smith-protocol-channel`'s `turn.rs:367` and `transcript.rs:381`; the private `run` of `simulation.rs`; and about 40 constructions and exhaustive matches outside Files. `smith/10-jobs.md` now has a section, "The ready check's corrections", which each increment's Files include and which holds where it and the text above differ.

114. **every probe's overrides start within shipped memory: 08.s1, then 04.6** (plan defect, smith 04's block of 2026-10-14T19:21:56Z).
     Ruling 114. A new step, `smith-08.s1`, brings every probe task's overrides within shipped memory; then 04.6 calibrates once more. On `main` at `25388ac8` the chat face refuses `first-request`'s overrides at startup, "service.worst_case=2147682394 bytes; declared.memory=2147462772 bytes", 219,622 bytes over, while the smoke's configuration passes: a smith defect on `main` that only some probes' overrides reach, and 12.1's baseline runs them all.

     - `smith-08.s1`, in `smith/08-limits.md`, session 08 (08.3 is its owner's last increment): find the priced term past the bound (Budget: 3 runs, 60 minutes, README rule 5), and remove an overcount a heap test shows, as ruling 110 did, with no preset, `declared.memory` or expected value changed; if none closes the gap, stop on a question with the term and its bytes. Its guard is one test over every `benchmarks/tasks/probes/*/task.toml`, as `smith_adapter.rs:449` and `:464` do for one profile each.
     - 04.6 needs 08.s1, then runs its calibration once, at most 12 attempts. Its row is now 23: the 10 and the 1 already spent, which count in its `Live:` trailer, and these 12.
     - No probe's prompt, seed, checks or deciding list changes.

115. **main fails the smoke: the smith line holds for 08.s1** (plan defect, smith 07's block of 2026-10-14T19:22:27Z).
     Ruling 115. `main` fails the smoke itself, so 07.2's two refusals are not 07.2's: the smith line holds for `smith-08.s1`, the repair, and resumes when 08.s1 lands with its smoke passing. On `main` at `25388ac8` the smoke's own configuration is refused at setup, before any model request: "service.worst_case=2147712231 bytes; declared.memory=2147462772 bytes", 249,459 bytes over. 07.2's branch is 227,196 bytes over, less than `main`. No smoke ran on `main` as landed: 08.3's (`probes-smoke-tools-4c0cbee6-20261014T185826Z`) ran on a tree without 07.1 (`bde09510`), and 04.s1's (`probes-smoke-tools-c81471d4-20261014T190644Z`) on a tree without 08.3 (`257a41bd`); README rule 8 says "A rebase inside `land` does not run it again". Ruling 114 rested on the premise that the smoke passes on `main`, and it does not.

     - **Until 08.s1 lands,** no smith landing that changes code runs the smoke or lands, except 08.s1. Smith sessions go on with their increments up to the smoke, and hold there, their worktrees committed. Landings that skip the smoke under rule 8 (Markdown only; only `benchmarks/tasks/`, `benchmarks/suites/`, `benchmarks/summaries/` or `benchmarks/baselines/`; skein) go on.
     - **08.s1** closes the larger gap: every probe task's overrides, `smoke-tools`' included, within the shipped `declared.memory`, as ruling 114 says, with no preset, `declared.memory` or expected value changed. Its smoke must pass on its branch. Its search starts with the combination of 07.1 and 08.3, which each passed the smoke alone. If no overcount closes the gap, 08.s1 stops on a question with the term and its bytes, and the line stays held.
     - **When 08.s1 lands,** each held step rebases on `origin/main` and runs rule 8's smoke as written: once, and once more on a failure. 07.2 now needs 08.s1. Its two refused attempts count in its `Live:` trailer (rule 10). Its two fixture fixes outside Files (`tests/protocol-llm/tests/root_memory.rs`, 78 preamble bytes; `tests/protocol-llm/tests/history_window.rs`, 23) stay, named in its commit body under "Users outside Files:".
     - **README rule 8 now says,** until the run ends: before it escalates a second failure refused at setup, the coordinator runs the smoke once on `origin/main` (`build origin/main`, then the probe on that arm) and puts both summary lines in `blocked.md`. This is the second such block (ruling 107 was the first).
     - 04.6 still waits for 08.s1 (ruling 114). Nothing else in rulings 107, 108 or 114 changes.

116. **invalid job calls are invalid results with a problem** (cross-session scope, smith 10's block of 2026-10-14T19:55:35Z).
     Ruling 116. Option 2: the invalid job calls of tools.md 5 are invalid results with a problem, not outcomes, and 10.5's Files gain transcript version 4's schema, its generated codec and its goldens, with no version bump (README rule 12). The design keeps the two kinds apart. transcript.md 2 lists a result as "a workspace tool's outcome, typed (domain/tools.md's vocabulary: lines read, entries listed, an edit made, a command's exit and output, a command answered `running` …)" or "invalid, with the problem found in the call". llm.md 5 says: "An invalid call is rendered with its problem". tools.md 5 says the yield past `jobs` "is answered as an invalid call, and not run: it says that at most `jobs` may run at once, and that `job` with `stop` stops one, or reads the end of one that ended". Option 1 would record these calls as workspace outcomes, a kind transcript.md 2 does not have for them.

     - `tools::Request` gains `Invalid { to: ReplyTo, problem: JobProblem }`, with `JobProblem` either `Full { jobs: u32 }` or `Unknown { job: JobNumber }`. It answers a call in the place of `Answer`: one of the two per call. No `Outcome` or `Verdict` variant is added for them.
     - The session answers it with `Returned::Invalid`. `llm::Problem` gains `JobsFull { jobs: u32 }` and `UnknownJob { job: u32 }`, costed in `problem_cost` as the other problems are.
     - `CallProblem` in `crates/smith-transcript/schema/transcript-v4.schema` gains `jobs_full` and `unknown_job`, each a record of one `u32`. Regenerate `src/generated/v4.rs` and `golden/v4/` with skein-codegen, so that `tests/codecs/tests/drift.rs` passes.
     - `smith-protocol-llm/src/failure.rs`'s `problem` renders them. `JobsFull` says that at most `jobs` may run at once, and that `job` with `stop` stops one, or reads the end of one that ended. `UnknownJob` names the number, and says that no `running` answer named it or that its end was already answered. Both texts render live and on resume (llm.md 5: "the same outcome renders to the same bytes, live and on resume").
     - The users this changes are named in `smith/10-jobs.md`'s 10.5 block. Adapt any further exhaustive match over `llm::Problem`, `wire::CallProblem` or `tools::Request` that `cargo check --workspace --all-targets` reports, as README rule 3 says, and list it in the commit body under "Users outside Files:".
     - `a_yield_while_jobs_jobs_run_is_an_invalid_call` and `a_job_call_naming_an_unknown_or_answered_job_is_invalid` (10.1's Test shape) each expect one `Request::Invalid` with its problem. One codec test round-trips each new `CallProblem` through version 4. Nothing else in 10.1's Shape changes.

117. **a running answer says its deadline's clamp** (cross-session scope, smith 10's block of 2026-10-14T19:58:01Z).
     Ruling 117. Option 1: `Outcome::Running` gains `timeout: TimeoutClamp`, after `dropped`, and so does transcript version 4's `CommandRunning`. `Exited` and `CommandEnd` carry the same field. There is no version bump (README rule 12). The plan's sketch left it out, which is a plan defect. tools.md 5 says of a shell call: "its deadline is the one the LLM asked for, or the profile's default, clamped to a tool call's longest deadline … The outcome says when a request was clamped." A `running` answer is that call's outcome. llm.md 5 renders "a request clamped to a ceiling, saying so and naming the ceiling".

     - `render.rs` says both clamps when both apply: the deadline's and the yield's.
     - `facts.rs`'s `observed` counts `timeout` as it does for `Exited`.
     - 10.5's Files already hold the schema, `src/generated/v4.rs` and `golden/v4/` (ruling 116). Regenerate them once, for both rulings.
     - One test asks a timeout past `tool_deadline` and a yield past `job_wait` in one call, and expects one `Running` with both clamps. A codec test round-trips a `CommandRunning` with a clamped `timeout`.
     - The users are named in the 10.5 block. Adapt any further one that `cargo check --workspace --all-targets` reports, as README rule 3 says, and list it under "Users outside Files:".
     - `Outcome::Job` stays as it is: its `wait` is its own clamp, and its job's deadline was said in its `running` answer.

118. **history names medium effort and calibrates again** (plan defect, smith 07's block of 2026-10-14T20:02:49Z).
     Ruling 118. The `history` task names `effort = "medium"`, and 07.3 calibrates both `history` and `history/small-window` once more, at medium. `tool-call` stays a check the harness reads as smith's: `model_deviation` does not change. benchmarks.md 3.1 says: "One that cannot be calibrated on the small tier at low effort names a higher effort and says why." The 9 low-effort attempts show the model failing, not smith:

     - In base `history`, 3 of 6 arm attempts gave a wrong sum.
     - In the small-window arm, the first summary covers 15 files and says "Continue from `node-d9217a06.txt`", although that file's read had completed before the compaction opened (`tool.completed` at 19191 ms, `conversation.opened` at 19191 ms). The second compaction, which opened the same way, carried its last read (file 31, `node-1e619b65.txt`) into its summary, so smith gives a compaction the results owed to it. That summary's sum, 441, should have been 501, and 501 + 27 = 528 is the expected answer. Both compactions were asked with tool choice `None`, and the window checks passed.

     - **The reason, as a comment above `effort` in `benchmarks/tasks/probes/history/task.toml`:** at low effort the small tier missed the sum in 3 of 6 attempts, and in a compaction summary it dropped a read it had been given (ruling 118). `effort` applies to the task and its variant. The prompt, the seed, the checks, the deciding lists and the task's `version` do not change: a result records its effort (benchmarks.md 8).
     - **The reruns:** each `calibrate` command once, as 07.3's Proof gives it, with the same control (07.1's parent, `8a630f41`) and the arm rebuilt on the branch with the effort change. 07.3's row becomes 37 attempts, $4 and 90 minutes in all, the 9 spent included. 12.3 takes 9 fewer, as README line 268 says.
     - **If a non-model check fails again at medium** (`tool-call` or any other check `model_deviation` does not list), 07.3 stops on a question. The question names the attempt, and quotes its summaries' last lines and the `tool.completed`/`conversation.opened` times around each compaction. That classification then goes to the designer. No check, criterion or `model_deviation` changes in this step.
     - 07.3's other work stands: `SUMMARY` at 390 bytes and `PREAMBLE` at 97, and the fixture's two byte inputs in `root_memory.rs`, named under "Users outside Files:".

119. **07.3 calibrates base history while the designer answers** (design gap (interim), smith 07's block of 2026-10-14T20:17:56Z).
     Ruling 119. Interim, until the designer's answer: 07.3 runs the base `history` calibration at medium now, once, as its Proof gives it. `history/small-window` waits for the answer. The question is in the run's `question.md`: is a small-window attempt that fails only `tool-call` and `answer`, with its compaction checks passing, a model deviation or smith's failure? The base calibration has no compaction and so does not rest on that answer. Ruling 118 still says: "If a non-model check fails again at medium … 07.3 stops on a question."

     - Run `calibrate probes/history --control 8a630f41 --arm <the medium arm already built>` once. If it reaches 10 passes, commit its `[calibration]` block on `parity/smith-07-words`, and land nothing.
     - If it stops on a failure `model_deviation` does not list, report it as a question with the attempt's name, and stop. If it stops on model deviations alone (ten passes out of reach), report the count, and stop.
     - Then 07.3 waits for the answer. No small-window attempt runs, and no check, criterion, `model_deviation` or instruction word changes, until the answer's ruling.
     - Row 07.3's allowance holds: 37 attempts and $4 in all, 15 and $0.30 spent.

120. **a derived writer ceiling follows ruling 117's layout** (cross-session scope, smith 10's block of 2026-10-14T20:20:00Z).
     Ruling 120. Option 1. In `crates/smith-agent-service/tests/sink_memory.rs:68`, 10.5 changes the expected `line_ceiling(Capture::Everything)` from `Some(19_849_728)` to the value the product limits now derive, `Some(19_427_968)`. The test's name, its product fixture, its allocation checks and its sent-byte assertions stay as they are. The ceiling is derived, not a bound the design sets: `EventLimits::derive` adds `client.history_items * 128` (`crates/smith-agent-service/src/events.rs:84`). Ruling 117's `timeout` on `Running` widens the block the receiving derivation prices, so 3,295 fewer history items fit: 3,295 × 128 = 421,760, which is the difference.

     - Name it in the commit body under "Users outside Files:", as a consequence of ruling 117's layout, with that arithmetic.
     - **Further copies:** where a later test fails on a literal derived from the same receiving layout (a history-item count, or a ceiling built from it), refresh it to the value the product limits derive, and list each in the commit body with its old and new values. A test that fails on a bound the design states (a preset, `declared.memory`, a window) stops on a question instead.
     - `benchmarks/tests/recorded/smith-time.jsonl`'s `"events.observation_line":19849728` is a recorded run's limits, so it stays as recorded.
     - The smoke (rule 8) and 08.s1's guard over every probe task's overrides run as usual. A smaller derived ceiling only lowers the worst case.

121. **the designer's answer on whose failure; 04.s2, then history version 3** (design gap, smith 07's blocks of 2026-10-14T20:17:56Z and 20:25:12Z).
     Ruling 121. The designer's answer, landed as smith `62376cb8` (benchmarks.md 3.1, 3.2, 11.1) and `070efc75` (07.3 amended, `04.s2` drafted), follows. `smith-04.s2` is a new step in session 04. 07.3 needs it. Row 07.3 becomes 69 attempts and $4: the 19 spent, at most 25 for version 3 at medium, and 25 more at `high` only under the answer's fallback. The answer's text:


     1. **Attempt 05 at medium, and attempt 01 at low, are the model's: model
        deviations, once the probe checks smith's side of the compaction.** The
        defect is not 07.3's words. smith asked each compaction with `none`. It
        gave the second compaction read 32's result whole, and opened the next
        window with the summary as written. That summary misadded (443 for 501)
        and listed the read it had been given as "One read remains". The model
        then read it again. The words say what run.md 3.3 lists ("what was done
        and found, with exact values, names and paths as they were read; what
        remains"). In the same attempt they gave a correct first summary, and 4
        of the 5 medium attempts passed. `SUMMARY` and `PREAMBLE` do not change
        to suit `history`.

     2. **The defect is benchmarks.md's.**
        - 3.1 said "its event checks hold smith to its contracts". The harness
          took that to make every failed event check smith's. But the read
          count counts what the prompt asks of the model.
        - Nothing checked the one smith contract that a lost state could hide
          behind that count: the compaction holds the results owed to it, and
          the next window opens with its summary.

        benchmarks.md 3.2 now says:
        - **"Whose failure."** "A failed deciding check is the model's when it
          judges what the prompt asks of the model: the answer, the files, a
          report of what the model was told (`budget-report`), and how many
          calls of a tool it makes where the prompt prescribes the number
          (`call-count`). It is smith's when it judges smith's contracts: its
          records, a call's verdict, a request's tool choice or content, a
          bound, the window."
        - **"A count is the model's only where smith's side is checked."** "A
          task whose windows compact also decides on `compaction-carried`: each
          compaction's request holds a result for each call its window owes it,
          and the window after it opens with that compaction's summary
          (domain/session.md, sections 8.1 and 8.2). With that check passing, a
          summary that misstates or drops what it was given is the model's, and
          so is the count that follows from it."

        Of the supervisor's options this is (a), made safe by
        `compaction-carried`, and not (c). `tool-call` stays deciding, and each
        read's verdict stays smith's. Option (b) is ruled out by point 1.
        Option (d), a higher effort, is point 3's fallback.

     3. **Base `history`'s wrong sums (2 of 3 at medium, 3 of 6 at low) are
        the model's arithmetic, and the prompt now makes it written arithmetic.**
        The model writes nothing beside its reads. In base attempt 03, none of
        the 34 responses holds a text block, only calls and opaque reasoning.
        So it adds 32 values
        that live only in its reasoning, or none at all. Raising the effort
        does not touch that: low and medium miss alike, at about 50 s an
        attempt.
        - benchmarks.md 11.1's `history` row now forces: "The running sum and
          the next file are written as text beside each read, so that what a
          summary must carry is in the window's history, not only in the
          model's reasoning."
        - `history` goes to version 3, still at `effort = "medium"`. The prompt
          adds "In each completion, before its read call, write one line of
          text: the sum of the values read so far, and the file you are
          reading."
        - If model deviations alone still put ten passes out of reach, the task
          names `high` with its reason, and both calibrations run once more
          (benchmarks.md 3.1: "names a higher effort and says why").

     4. **The deciding checks left unavailable after the recovered 503 are
        neither smith's nor the model's: the harness misread the stream.**
        benchmarks.md 3.2: "A request smith retried (domain/session.md, section
        4) is judged through its retry. A request that failed with no blocks
        made no calls, and its prompt is the one its retry sends
        (protocol/events.md, section 3.11), so the failed request's missing
        usage leaves no check unavailable."
        - Under that rule, base attempt 03 failed only `answer`, so it is a
          model deviation.
        - In general: "A deciding check of smith's contracts left unavailable,
          other than 'not exercised' (11.1), leaves the attempt unjudged:
          neither smith's failure nor a deviation (section 1)."

     5. **What 07.3 does next** (07-compaction.md, 07.3, as landed):
        1. **`04.s2` lands first.** Its text is a draft in `04-probes.md`, for
           the supervisor to name and schedule. It adds `call-count` (the
           model's) and `compaction-carried` (smith's). `calls-per-response`
           and `prompt-within-window` read a failed request through its retry.
           `count` puts a model failure beside an unavailable contract check
           under `unjudged`. About 150 lines and six tests; the smoke.
        2. **07.3 rebases on 04.s2 and writes `history` version 3.** It
           changes:
           - the prompt's added sentence;
           - `tool-call` without its count;
           - `call-count` exactly 32;
           - `compaction-carried`;
           - `completions` removed: 32 reads, at most one per response, imply
             its 33 completions;
           - `deciding = ["end", "exit-code", "answer", "tool-call",
             "call-count", "calls-per-response", "prompt-within-window",
             "compaction-carried"]`.

           `small-window`'s own list and window are unchanged. 07.3 then
           rebuilds the arm.
        3. **The two calibrations run once each.** First `calibrate
           probes/history/small-window --control <07.1's parent> --arm <dir>`.
           Then `calibrate probes/history --arm <dir>`, with no control: base
           `history` never compacts, so 07.1's parent can fail it only by the
           model's errors. If that control passed, `calibrate` would write no
           block. The commit body says why.
        4. **Stopping.** 07.3 stops on a question for any failure that is
           smith's under 3.2, and for any unjudged attempt. For model
           deviations alone, step 3's fallback applies: `high`, both commands
           once more. An attempt that ends on its budget at `high`, or a second
           stop, also stops on a question.

     6. **For the supervisor:**
        - **Row 07.3's allowance.** Version 3 needs at most 25 more attempts
          at medium (12 and a control for `small-window`, 12 for base), and as
          many again if `high` is needed. At about $0.02 an attempt, that is
          within the $4.
        - **README "Done when" item 4** says "(version 2)". It is version 3
          now.
        - **code-organization.md:101** names "`history` version 2".

122. **10.6 stops a job and settles its jobs at close** (plan defect, smith 10's block of 2026-10-14T20:39:29Z).
     Ruling 122. Option 1. 10.6 takes from 10.2 the two parts without which a held job can outlive its kit or meet an act nothing answers:

     - `job` with `stop` asks `JobAct::Stop` and answers `stopped`, or the end the job already had (tools.md 5: "`job` with `stop: true` stops the job's group in the steps a deadline uses …, then answers its end, `stopped` … A job that had already ended is answered with that end").
     - The kit's close stops its running jobs' groups and waits for them to settle before `Request::Closed` (10.2's Don't: "let a job outlive its kit's close").

     Its tests are `a_job_stopped_and_one_stopped_after_it_exited` (moved from 10.2) and `a_close_with_a_job_running_settles_it_before_closed`, under 0.5 s each. 10.1's Don't, "stop or cancel a job yet (10.2)", no longer binds 10.6, for these two only.

     - 10.2 keeps `Event::CancelJobs` and `Request::JobsCancelled`, `cancelled` at a job's next read, the close's facts for each job whose end no answer reported, the referee and the fuzzy world, and its other tests.
     - Option 2 (10.6 and 10.2 landing together) is not taken. If 10.6 with these two parts passes README rule 1's split point, split it there, as rule 1 says.
     - Nothing is offered to a model: `job` waits for 11.2, and production `shell` calls keep `Yield::Never` until 11.2 (the 10.1 corrections).

123. **07.3 calibrates base history v3 while the designer answers** (design gap (interim), smith 07's block of 2026-10-14T20:59:36Z).
     Ruling 123. Interim, until the designer's answer: 07.3 runs base `history` version 3 at medium now, once: `calibrate probes/history --arm <the version 3 arm already built>`, with no control (ruling 121, point 5.3). `history/small-window` waits for the answer. The question is in the run's `question.md`: does a small-window attempt whose model finished after 4 reads, before any compaction was due, count as a model deviation, smith's failure or unjudged? Base `history` decides neither `conversation-opened` nor `compaction-choice`, so its calibration does not rest on that answer.

     - If it reaches 10 passes, commit its `[calibration]` block on `parity/smith-07-words`, and land nothing.
     - It stops as ruling 121's point 5.4 says: on a question for any failure that is smith's under benchmarks.md 3.2, or any unjudged attempt. On model deviations alone that put ten passes out of reach, report the count and stop. `high` waits for the answer, so that both calibrations move to it together.
     - Then 07.3 waits for the answer. No small-window attempt runs, and no check, classifier or instruction word changes.
     - Row 07.3: 69 attempts and $4 in all, 23 and $0.45 spent.

124. **the designer's answer on checks not exercised; 04.s3, then small-window** (design gap, smith 07's block of 2026-10-14T20:59:36Z).
     Ruling 124. The designer's answer, landed as smith `f242ecaa` (benchmarks.md 3.2, 11.1), `728c9660` (`04.s3` drafted, 07.3 amended) and `3f618dbe` (07.3 goes to `high` at once if base `history` needs it), follows. `smith-04.s3` is a new step in session 04, after `04.s2`. 07.3's small-window rerun needs it. Row 07.3 becomes 73 attempts and $4. The answer's text:


     1. **Attempt 03 is a model deviation.** No compaction was owed, so its
        compaction checks were not exercised. Its other failed checks,
        `call-count` and `answer`, are the model's.
        - The model stopped by itself after 12 s. It had 299 s and 64 turns,
          and smith told it so, accurately.
        - Every read came back whole.
        - Its largest prompt was 11,168 tokens. With the 8,192 reserved for
          output, that is 19,360. The window compacts at 52,428 (65,536 ×
          4/5).
        - smith compacts only when a window check fails: "A completion that
          fails the check is not made: the session compacts instead"
          (domain/session.md, section 8.1). The check never came near failing.
        - `conversation-opened`'s "at least 1" holds only for a model that
          reads far enough to fill the window. The attempt's failure there
          judged the model's early stop, not a contract of smith's.

     2. **The defect is benchmarks.md's.** "Not exercised" was defined only
        inside two probes' rows. The supervisor's reading missed the second
        one, slow-completion's, which is the closer precedent: "An attempt
        under 60 s is 'not exercised': neither a pass nor a fail, counted like
        a model deviation". `history` had no such case. Its compaction checks
        judged that a compaction happened at all, while smith's contract is
        conditional.

        benchmarks.md 3.2 now says:
        - **"Not exercised."** "Some of smith's contracts apply only once
          their condition arises: a call past a bound, a call that lasts over
          60 s, a window at its compaction threshold. … A check of such a
          contract, in an attempt where its condition never arose, is 'not
          exercised': neither a pass nor a fail. What kept the condition away
          was the model's, so in a calibration the attempt counts like a model
          deviation (3.1): never a pass, and within the attempts the model may
          fail. … A check that cannot tell whether its condition arose,
          because a record it reads is missing, is unavailable instead, and
          leaves the attempt unjudged."
        - The unjudged rule's "other than 'not exercised' (11.1)" now reads
          "(below)".

        benchmarks.md 11.1's `history` row now says how the stream shows the
        condition:
        - "A request has reached it when its prompt, as its usage counts it,
          plus the completion's most output, is above the window times the
          compaction threshold that `session.started` carries (protocol/limits.md,
          section 9)."
        - "The window check (domain/session.md, section 8.1) counts what was
          appended since the last prompt as an upper bound, so smith makes no
          ordinary completion past that point: it compacts first."
        - "An attempt that opened no compaction, and none of whose requests
          reached that point, leaves `conversation-opened` and
          `compaction-choice` 'not exercised' (3.2) … An attempt with a
          request past that point and no compaction fails
          `conversation-opened`, which is smith's."

        This is the supervisor's first reading, "the attempt's largest prompt
        against the compaction point", made exact. It is not "unjudged": the
        stream shows that nothing was owed. The output reservation is counted
        because 8.1's check counts it. A smith that failed to compact cannot
        hide behind this rule:
        - an attempt that reads the whole chain passes the point, because the
          variant's window is "several times smaller than the reading";
        - a not-exercised attempt is never a pass, so it uses up the
          calibration's room for model failures (at most 2 of 12).

     3. **What 07.3 does next** (07-compaction.md, 07.3, as landed):
        1. **`04.s3` lands first.** Its text is a draft in `04-probes.md`, for
           the supervisor to name and schedule. `threshold::reached` reads
           whether any request's prompt plus its `output_tokens` passed
           `window_tokens` × `policy.compaction_threshold`. Below that point,
           with no compaction, `conversation-opened` (kind `compaction`) and
           `compaction-choice` return unavailable "not exercised: no request
           reached the compaction threshold". `model_deviation` and `count`
           do not change: they already read "not exercised:" as the model's.
           About 70 lines and four tests, from an excerpt of attempt 03; the
           smoke.
        2. **Ruling 123's base `history` run stands as ruled.** Base
           `history` decides neither compaction check, so a block it writes
           before 04.s3 stands.
        3. **07.3 rebases on 04.s3 and keeps the version 3 arm** (`8a5bb9ec`).
           04.s3 changes only `benchmarks/`, so the arm is not rebuilt. It
           runs `calibrate probes/history/small-window --control <07.1's
           parent> --arm <that arm's directory>` once, whole. The first run of
           that command is spent: `calibrate` has no resume.
        4. **If the base run stopped on model deviations alone,** the
           small-window command does not run at medium. `effort` applies to
           the task and its variant, so the task names `high`, and both
           commands run once there (the fallback of ruling 121).
        5. **Stopping is unchanged otherwise.** 07.3 stops on a question for
           any failure that is smith's under 3.2, and for any unjudged
           attempt. An attempt whose compaction checks are not exercised, and
           whose other failures are the model's, is a deviation, and does not
           stop.

     4. **For the supervisor:**
        - **Row 07.3's allowance** is 69 attempts, 23 spent. The worst case is
          now 73:
          - base at medium: 12;
          - small-window again, with its control: 13;
          - `high`, both commands with the control: 25.

          If base goes to `high` at once (point 3.4), it is 60. At about $0.02
          an attempt (23 for $0.446), $4 holds either way.
        - **`04.s3` needs a README row** and a place in the order: before
          07.3's small-window rerun, after `04.s2`, which landed as
          `ca595dbc`.

125. **10.2's refusal is its own: the line resumes, its corrected arm smokes afresh** (plan defect, smith 10's block of 2026-10-14T21:24:14Z).
     Ruling 125. The smith line resumes now. Once its accounting correction is in, 10.2's corrected arm takes the smoke afresh, as README rule 8 says: once, and once more on a failure. Both refusals were 10.2's own. Its branch's worst case, 2,147,930,390 bytes, passed the shipped `declared.memory`, 2,147,462,772, at setup, before any model request. `main`'s smoke, run by the coordinator as rule 8 asks, passed: `probes-smoke-tools-2ae7b719-20261014T212344Z attempts=1 verdict=pass`. This is the second block of its kind (ruling 107 was the first), so it becomes rule 8's standing text.

     - **The correction** removes an overcount that a heap test shows, as rulings 110 and 114 did. The two capture scratch buffers are priced per domain step, not per kit, and the cancellation buffers stay per job. No preset, `declared.memory` or expected value changes. Name the heap test in the commit body, with the bytes it removes (the report expects 524,288) and the margin left (the report expects 56,670).
     - **A second failure** of the corrected arm stops 10.2 on a question with the term and its bytes, and the line goes on (rule 8, as amended).
     - The two refused attempts count in 10.2's `Live:` trailer (rule 10). They are $0, with no model request.
     - **README rule 8 now says,** until the run ends: a second failure refused at setup, before any model request, is followed by the coordinator's smoke of `origin/main`. If `main`'s smoke passes, the failure is the step's own: it stops its step on a question, and the line goes on. If `main`'s is refused too, the line stops (ruling 115). Every other second failure stops the line, as before.

126. **probes' token cap rises to 20M; base history at high once more** (plan defect, smith 07's block of 2026-10-14T21:50:31Z).
     Ruling 126. Yes. `benchmarks/suites/probes.toml`'s `max_tokens` rises from 8,000,000 to 20,000,000, as a one-line fix outside Files. Then base `history` at `high` runs once more, whole: `calibrate probes/history --arm <the version 3 arm's directory>`, with no control, as ruling 124 says. `calibrate` has no resume, so the six attempts already run are spent. The small-window block (`probes-history-small-window-8a5bb9ec-20261014T212649Z`, 10 passes and 1 model deviation) and its summary stand, committed. The cap is a token count, not a bound the design sets for this probe. Six attempts at `high` used 8,022,325 tokens, most of them cache reads, so twelve need about 16.1 million.

     - Name it in the commit body under "Users outside Files:", with that arithmetic. `compare.toml`'s own 8,000,000 does not change.
     - Stopping is ruling 124's: a smith failure or an unjudged attempt stops 07.3 on a question. If model deviations alone put ten passes out of reach at `high`, report the count and stop.
     - Row 07.3 holds: 73 attempts and $4, of which 46 and $0.93 are spent; this rerun is at most 12 more.

127. **tools' Open carries the last turn's highest job number** (cross-session scope, smith 10's block of 2026-10-14T21:54:54Z).
     Ruling 127. Option 1. `tools::Event::Open` gains `jobs: u32`: the highest job number its conversation's last turn records, or 0 for a fresh session. The kit starts `Jobs.highest` from it. tools.md 5 says: "A session resumed in a later activation numbers its next job after the highest its transcript's last turn records". transcript.md 4 says: "the resumed session's next job takes the number after the highest its last turn records". The session already holds that number as `Turn.jobs` (`crates/smith-domain-session/src/record.rs:158`). One field on the one opening is the smallest contract. A separate restore entrance (option 2) would add a second way to change the same table.

     - Production: `crates/smith-domain-session/src/session.rs:414` passes the restored last turn's `jobs`, and 0 when there is none. `crates/smith-domain-tools/src/domain.rs:93` passes it to `kit::open`.
     - Users to adapt (each `Event::Open { session, authority }` gains `jobs: 0` unless its test is about resuming): `crates/smith-domain-tools/src/tests.rs:152`, `:272`, `:465`, `:1409`, `:1525`; `tests/tools/src/world.rs:669`; `tests/tools/tests/fuzzy_memory.rs:47`; `tests/tools/tests/memory.rs:56`, `:143`. Adapt any further one that `cargo check --workspace --all-targets` reports, as README rule 3 says, and list it under "Users outside Files:".
     - One test opens a kit with `jobs: 3`, starts a job, and expects it answered `running` as job 4.
