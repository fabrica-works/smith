# Parity plan: decisions

2026-10-14. The decisions the parity plan rests on, taken by its designer
from the findings in `findings/` (skein's development.md, sections 4.1
and 10.4). Sections 1 to 8 are the design's; the design documents follow
them, and where this file and the design disagree, the design wins and
the disagreement is a question for the designer. Section 9 lists the
decisions with long-term consequences that the user may veto. The run's
rulings, when it runs, go in a section 10 of their own.

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

Still with the user from the reliability plan, unchanged by this one:
borrowing another CLI's login (its 10.2), the smoke before `land` (its
10.4), the skein pin tags (its 10.5), and local `sub_agents` on by
default (its 10.7).
