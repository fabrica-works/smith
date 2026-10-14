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
