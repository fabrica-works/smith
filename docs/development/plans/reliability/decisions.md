# Reliability plan: decisions

2026-10-09, revised 2026-10-12. The decisions the plan rests on, and the
rulings of its first run. Sections 1 to 10 are the design questions from
the findings (`findings/README.md`, section 9), each with what was
decided, unchanged from the first run's `QUESTIONS.md`; they keep their
numbers, so "decisions.md, 5.8" is the first run's "QUESTIONS.md 5.8".
Section 11 holds the first run's rulings that still bind the work left,
and section 12 the decisions of the plan's port to skein's
`docs/foundation/development.md` (2026-10-12), and section 13 the
rulings of this run's supervisor.

On 2026-10-09 the user decided language choices: Rust, unless a
compelling reason goes through the user. The user delegated everything
else to Claude, asking to hear about anything critical with long-term
consequences that would be hard to change; section 10 lists those for the
user's veto. The user also ruled that everything is pre-release: formats
can change at will, at any level and in any project, with no
compatibility obligations (no translation of old transcripts, no support
for older versions of any record or file). Entry IDs (L1, S14, …) cite
the files under `findings/`.

The design follows these decisions. Where the design and this file
disagree, the design wins, and the disagreement is a question for the
designer (development.md, section 10.4).

## 1. Cross-theme

1. **Context is managed by compaction into a new window, not by elision.**
   Elision is dropped from `session.md` §8. When window-aware admission
   (C7) triggers, the session asks the same model, with the same system
   text, tools and history plus one summary instruction, for a summary.
   The answer opens a new window. In-place edits of history would miss the
   cache from the edit onward, and would break Anthropic's preserved
   thinking (C8 over S3).
2. **Model facts: derivation in skein, catalogue in smith's shell, values
   in the charter.**
   - skein-llm provides the derivation from declared quantities to every
     byte limit (S14). It ships no model catalogue: model facts change
     too often for a kit.
   - smith's shell presets carry known models' context window, maximum
     output and timeouts. Settings override them.
   - The host puts the effective values in the charter's model entries,
     so the domain enforces what the host chose (C7, S14).
3. **Fact delivery has a policy per sink.**
   - Inside the process, facts are lossless up to the sinks. The domain
     fact queue reserves room like a step output; this resolves skein's
     open "trace queue" question as "hold the step".
   - The event sink (the trace file and `--json`) is complete by default,
     backpressured through its stream.
   - The channel projection drops and counts, and never holds the run.
   - Summary and terminal records are never dropped.

   Sources: O10, P13.
4. **The benchmark baseline is taken before events exist.** The first
   harness has a legacy adapter that reads today's `Debug` trace. It is
   isolated in one module and deleted once events land. The spike's
   archived runs are historical reference only, never a baseline.
5. **Order of work: foundations, then the product, then behaviour.**
   1. Foundations: skein's ownership rule and LLM mechanisms; the event
      vocabulary; the limits model; the benchmark harness with its
      baseline.
   2. The product: crate split, inline agent, `smith exec` and the
      interactive front end.
   3. Behaviour themes in parallel: messages, budgets, context, tools.

   Step 4 refines this into sessions.

## 2. Messages and startup

1. **A start carries its triggering messages,** in the channel start
   record and the domain's start. Headless `smith exec` uses the same
   mechanism for its prompt. `Awaiting` stays for hosts that start before
   they have a message (L1, P14).
2. **A run opens main only when it has work:** a non-empty brief or a
   message. The opening is the brief's instruction, then every queued
   message, FIFO, bounded by the run's message limit. An empty brief
   without wait authority opens on the instructions (L1, L4).
3. **Every message has exactly one terminal.**
   - **Read:** a turn's fence names it or a later message.
   - **Refused** at admission.
   - **Unread** when the run answers.

   The answer carries the final fence, and the host kit reports unread
   names once. What a host does with an unread message is its policy
   (L5).
4. **A `finish` that crosses a queued message is not accepted** for
   wait-granted charters: the run returns the messages to the model
   instead (L5).
5. **The local host carries unread lines after an accepted or parked
   answer.** After cancel, failure or refusal it shows them as not
   delivered and drops them. It never auto-starts after an interrupt (L6).
6. **Messages pass between completions** (mid-work) later. The design
   names how, now (L10).
7. **`wait` defers its result.** The result and the message arrive
   together, so an exchange costs one request (L13).
8. **Time spent awaiting counts against wall time,** documented in
   `run.md` §9. An awaiting run parks at the charter's waiting time (L1,
   L2).
9. **"Waiting" shows only when every relayed message has been read** (L3).

## 3. Limits

1. **Limits are declared quantities.**
   - Per model: context window, maximum output, maximum reasoning item,
     head and idle timeouts.
   - Per deployment: concurrent conversations, maximum tool payload,
     render caps, the LLM memory pool, real memory.

   Every byte limit is derived with checked arithmetic, and every
   relationship is checked at startup, by name. Raw byte limits are not
   configuration. A profile is a named bundle of declared values (S14).
2. **skein-json gains a general selective collector.** Provider events
   are decoded streamed and selectively. Ignored fields cost a scan, not
   memory or tokens (S2).
3. **One declared tool payload** bounds read windows, writes and call
   arguments (S4).
4. **Deadlines measure progress.**
   - Head and idle are per provider and model.
   - The whole-call bound is the session's remaining time.
   - Timeouts carry their phase and are retried by phase.
   - There is no fixed per-attempt ceiling by default.

   Source: S5.
5. **An oversized reasoning item is a typed failure.** Dropping it from
   later requests is an explicit opt-in (S6).
6. **A truncated Anthropic `tool_use` is marked by skein.** smith replaces
   it in history with a note and an error result, so history stays
   replayable (S7).
7. **One declared "calls per response"** bounds both the completion's
   blocks and the transcript's blocks per message, in the new transcript
   version (S8).
8. **A call that cannot be served now waits.** A pool at capacity queues
   the call, bounded by conversations, within the call's deadline. Pool
   size is also checked against concurrency at startup (S9).
9. **Oversized rendered tool output is truncated with an explicit marker,**
   never a refused request (S10).
10. **Memory is accounted against a real target of at most 2 GiB.**
    Admission to a shared LLM memory pool replaces products of
    independent maxima. Waiting is the backpressure (S11).
11. **An oversized tool call is a per-call outcome,** told to the model as
    a fixable error, never a failed completion (S4).

## 4. Context and caching

1. **Affinity: one key per run tree, plus a per-conversation thread id.**
   The Codex dialect renders the key as `prompt_cache_key` and
   `session-id`, and the thread id as `thread-id`. Anthropic accepts the
   affinity and ignores it (C1).
2. **The host supplies the run's identity in the start,** stable across
   activations and resumes. The run mints one from its seeded RNG when
   the host gives none (C1).
3. **The static endpoint `cache_key` and the README recipe go** (C1).
4. **Private routing extras are adopted only if a probe shows a material
   gain.** These are Codex's turn-state token and `reasoning.context`.
   WebSocket incremental transport goes on skein's roadmap as a later
   item (C2, C4).
5. **Anthropic caching uses explicit breakpoints:** the last system block
   and the tail, with a 5-minute TTL, re-evaluated with the idle-gap
   probe (C5).
6. **Usage "not reported" is `Option` in skein's `Usage`.** Cache writes
   are decoded and priced at a rate of their own, which defaults to the
   input rate (C6).
7. **Compaction triggers at about 80% of the usable window.** The new
   window holds the task, the summary and pending host messages. The
   threshold and the contents are measured with the history probe (C7,
   C8).
8. **The stable-prefix invariant is a contract.**
   - History is append-only within a window.
   - System text and tools are fixed for a conversation's life.
   - Dynamic guidance is appended, never placed in the system text.

   Source: C3.

## 5. Budgets and delegation

1. **Turns stay in the charter as a generous loop guard.** Time and spend
   are the planning bounds (D1).
2. **Spend is real.** The standard ceiling becomes large. Flat-rate hosts
   use notional prices, and the local host refuses an impossible spend
   before starting (D2).
3. **A finishing reserve is a charter field with a documented default.**
   Children and ordinary work draw from what is left above it. Wind-down
   gives the main agent a final turn that offers only `finish` (and
   `deliver` if granted) (D3).
4. **The model sees its budget.**
   - A `## Budget` section in the system text.
   - `## Your share` for children.
   - A usage line on every sub-agent result.
   - Threshold notes once mid-work messages exist.

   Source: D4.
5. **`sub_agent` takes `max_turns` and `max_seconds`,** clamped to what is
   left above the reserve. The effective share is shown to the child.
   Spend shares are derived, never shown (D5).
6. **A child's last turn offers no tools,** so a capped child answers
   instead of vanishing. Its answer cap is stated (D6).
7. **The write-lease vocabulary goes into `run.md` now.** The first
   asynchronous step (the parent reads while one writer child holds the
   lease), scoped writes and asynchronous children go into a later
   document (D7).
8. **Local `sub_agents` stays off by default until the reserve lands.** It
   is then reconsidered (D9).
9. **The `agents` field leaves the `sub_agent` schema.** Nesting is refused
   with a typed problem (D10).
10. **Facts carry effective shares, clamp reasons and child usage** (D11).
11. **The benchmark report gets an erratum** for the 256-turn attribution
    when its successor is published (D12).

## 6. Shutdown, traces and diagnostics

1. **An owner close drains.**
   - Idle connections close at once; busy ones close when their call
     ends.
   - The component reports `Closed`.
   - Deadlines are derived per phase.
   - An owner that wants abort cancels its own calls first.

   Source: O2.
2. **The teardown invariant is checked once, in skein's world harness.**
   Every world inherits it (O3, O6).
3. **The pool closes at the answer.**
   - Teardown is one exhaustive phase: `Live → Answered → Closing →
     Closed`.
   - SIGTERM after the answer aborts what is still closing.
   - Hosts derive their exit grace from the agent's teardown bound.

   Source: O4.
4. **A forced exit after an accepted answer is a warning,** not a failure.
   The exit evidence reaches every host (O5).
5. **The trace is written through io,** after skein gains append streams.
   There is no shell thread (O7, P2).
6. **The trace records each turn's new messages,** not the whole prompt
   (O11).
7. **Failures are shown in words on the terminal.** Headless exit status
   is per failure class. Typed failures reach the local domain in both
   placements (O13, O14, P7).
8. **Provider failure text is clipped on stderr.** Full text goes to the
   trace only under content capture (O16).

## 7. Tools and local setup

1. **The configuration owns the commands' environment, whole.** There is
   no charter overlay now; the domain's environment fields go (T1).
2. **One directory hides its name from path guidance.** Host absolute
   paths are accepted as aliases until contained trees exist (T3).
3. **`edit` needs only a unique match of `old` in the current file,** not
   a prior read. `write` keeps read-before-write (T4).
4. **Edit formats are per dialect over one typed call.** The formats are
   chosen by measurement (T4).
5. **Shell and read output budgets are model-selectable** within profile
   ceilings (T5).
6. **Read windows are 32 KiB with paging; guides are whole up to 16 KiB**
   (T6).
7. **A background-process tool comes after process groups** (T7).
8. **Process groups land in skein before contained trees.** Per-tree
   resource limits come with contained trees (T9, T10).
9. **A local account may borrow another CLI's login.** Borrowing is
   read-only and access-token-only, opted into per account, and never
   touches refresh tokens (T11, P17). See section 10.
10. **skein-oauth accepts `localhost` redirects,** bound to 127.0.0.1 and
    compared exactly (T11).
11. **Subscription models are unpriced** for display. Spend uses notional
    prices (decision 5.2). The core inherited environment is a named list,
    never a pattern (T12).

## 8. Packaging and the product

1. **The root package is `smith`, with binaries in `./src`.**
   - The step-crate `clippy.toml` moves to `crates/`.
   - A shell `clippy.toml` at the root carries skein echo's bans.
   - Shell crates are libraries: `smith-agent-shell` and
     `smith-local-shell`.

   Source: P1, P11.
2. **A `smith-agent` binary ships** while any host may want a prebuilt
   agent process. It is the real-process proof of the channel (P11).
3. **Spawning agents is a library capability.** An `isolate` option of
   the product comes later, with contained trees (P10).
4. **The event vocabulary is specified and versioned** in
   `protocol/events.md`. The trace file and `--json` are the same stream
   (P13). Like every format it is pre-release and changes at will.
5. **`smith exec` has documented exit codes and edits in place without
   committing.** `--deliver` keeps the change contract (P14).
6. **Settings are TOML,** strict and bounded, layered from built-ins
   through user, `.smith/` in the repository, `-c` overrides and flags.
   Generated agent configuration for hosts stays JSON (P16).
7. **The product is built on smith's inline agent** (the user, 2026-10-09).
   - smith builds its own, the proper way, with jig's twin as a reference
     only.
   - The parent face is typed for both kinds of agent: the charter and
     turns cross as values, so no domain parses. A spawned agent's bytes
     are `smith-host-protocol`'s.
   - jig and temper are not changed by this plan. APIs break freely, and
     consumers are fixed separately (P18).

## 9. Benchmarks

1. **The harness is Rust:** a workspace crate under `benchmarks/`,
   ordinary Rust under the test-harness rules (B §6; the user's language
   decision).
2. **Agents run "vanilla",** meaning defaults plus a pinned configuration
   committed in the repository. Credentials are the user's existing
   logins (the user, 2026-10-09):
   - smith borrows `~/.codex` read-only, access token only;
   - Codex runs in a `CODEX_HOME` of the attempt's own, holding only an
     access-only `auth.json` written from the user's login, and with
     behaviour-shaping keys pinned on the command line. This was amended
     2026-10-09 (smith `95a9807`), replacing "the user's own
     `CODEX_HOME`": codex-cli 0.160.0 reads `CODEX_HOME/AGENTS.md`
     whatever its command line says, and no pin reaches the home's
     skills, plugins or memories;
   - no refresh token is ever copied.

   Claude Code, with `~/.claude`, is the next pass, not this plan.
3. **Probes run on `gpt-6-luna` at low effort.** `claude-haiku-5-5` at
   low effort joins with the Claude pass. Comparisons run at the user's
   working model (B §5).
4. **Raw results stay outside the tree.** Summaries, baselines and
   per-task variation are committed (B §6).
5. **Benchmarks are never the merge gate.**
   - A change to the runtime, limits, providers or the host records a
     smoke run in its commit body.
   - Every fix names the probe that guards it.
   - A probe fails before its fix and passes after.

   Source: B §4.
6. **The repository task is kept,** frozen with vendored pins, and run
   rarely (B2).
7. **Hidden graders live in the repository.** Attempts run in isolated
   locations from seeds that exclude `benchmarks/` (B §7).
8. **Per-attempt AI review is replaced by sampled audits** (B).

## 10. For the user: long-term decisions to veto

None of these blocks step 3. Each is hard to undo once built on.

1. ~~The product is built on smith's inline agent (8.7).~~ Decided by
   the user on 2026-10-09: yes, smith's own and typed; jig untouched. The
   original note follows. **The product is built on smith's inline agent
   (8.7).** This adopts
   smith's side of jig's one-host draft. It creates a smith crate that
   jig will later consume instead of its own `jig-inline-agent`, so jig
   inherits its contract. The alternative is to finish today's dual-path
   in-process local host. It is quicker, but leaves three one-process
   compositions across smith and jig.
2. **Borrowing another CLI's login (7.9).** smith would read
   `~/.codex/auth.json`, and later Claude's equivalent: read-only,
   access token only, opted into per account. It formalizes what the
   import script did. The alternative is smith's own sign-in only, which
   requires each provider to sanction a public client for it.
3. ~~The event stream is a public, versioned contract (8.4).~~ Withdrawn
   by the user's pre-release ruling: the stream is versioned and documented,
   and changes with smith, like every other format.
4. **The per-merge smoke runs before `land`, not on the landed tip**
   (12.3, 2026-10-12). It changes the user's rule of 2026-10-10 ("on the
   tip that becomes main"). The alternative, `smoke-tools` inside
   `scripts/check.sh`, smokes the landed tip but puts credentials and
   provider capacity into every gate.
5. **The skein pin tags stay unpublished** (12.2, 2026-10-12). smith
   commits on GitHub from 2026-10-09 to 2026-10-12 pin skein revisions
   that exist only on debian-16gb-hel1-1 (the nine `smith-pin/*` tags
   and `pin/smith-pre-invariant`), so they do not build from a fresh
   clone. Publishing the tags is one push, `git -C ~/src/rust/skein push
   origin 'refs/tags/smith-pin/*'`, from that machine; once its clone is
   gone, it cannot be done.
6. **The working tier for comparisons** (12.17): `gpt-6-astra` at
   `high`, the first run's proposal. It sets what smith's published
   comparison against Codex measures; session 12 pins it and spends about
   $45 on it.
7. **Local `sub_agents` on by default** (5.8, smith 07.13): smith 07.13
   recommends, from session 13's `budget` and `parallel-children`
   records; the user decides.

## 11. The first run's rulings

The rulings of the first run's supervisor (its "unblocks" and "steers")
that still bind the work left, as it ruled them. Section 12 replaces
parts of three: one batch of at most 12 attempts per variant and no new
task versions (12.4) replace 11.2's restarts and 11.9's stronger prompt
or higher effort; 12.5 replaces 11.6's count of 24 attempts.

### 11.a Probes and calibration

**11.1 Proof before landing** (2026-10-10; 2026-10-11). "A change is
proven before it merges": a plain defect by a test failing on its parent;
an improvement by its probe, failing on a parent that can run live, with
Codex where the probe compares agents. A landing that changes smith's code
runs `smoke-tools` live once, rerun once on failure; it is red when both
fail, or either fails as a smith defect; red stops landings. This replaces
9.5's first sentence. Why: on 2026-10-10 `main` could not finish a live
call. In the design: `docs/development/workflow.md` 5; benchmarks.md 4.2,
11.5; benchmarks.md 1 still reads "never the merge gate".

**11.2 Calibration** (2026-10-11; 2026-10-12). Ten passes on `main` within
at most twelve attempts; a median probe's attempt is its three
repetitions. Each failure is classified, its raw verdict kept: a smith
defect (a tool error, a refused or failed call, loss, a limit failure, a
wrong terminal) stops the line until its owner's fix; a model deviation (a
skipped, extra or repeated call, or another order, with a normal end)
counts, and past two in twelve a new task version restarts it; a check
stricter than its design is corrected to benchmarks.md 11.1's wording, the
runs judged again with both verdicts kept; a task the tier cannot do as
written (ended by a budget, deadline or limit, as designed) gets a new
version and is not an attempt; a setup refusal before any provider call,
or a harness defect (2867), is fixed and not counted; a failure no class
fits stops it. Model, effort, budgets and checks never change to pass,
though a version may name a higher effort, saying why. Why: a
`smoke-tools` run skipped a step with nothing wrong on smith's side. Not
in the design; benchmarks.md 3.1 reads "ten passes of ten".

**11.3 What a check cannot observe** (2026-10-11; 2026-10-12). A check of
work not landed is "not yet: <increment>", judged from its landing:
`teardown_ms` (06), checks on 04.7, `failures`' headless exit code and
last stderr line (10.5). A value the stream lacks is unavailable, never
inferred from the prompt, `input_bytes` or later calls; an oversize
block's path is "not carried". No terminal is synthesized. Why: an
inferred value is invented evidence. In the design: benchmarks.md 1, 11.1
`failures`; protocol/llm.md 4.

**11.4 `large-write`** (2026-10-12). Deciding: one `write` of at least
32,768 bytes, `written`, the saved file byte-equal to the call's content,
no `max_tokens` stop, accepted; fidelity recorded. Version 7 (numbered
lines, low effort) stands at 10 passes in 11. Codex pairs this variant's
outcome half; `oversized` is not paired. Why: a seed's digest judged the
model's copying. In the design: benchmarks.md 11.1, but for Codex's scope.

**11.5 `oversized`, version 9** (2026-10-12). A 2 KiB `tool_payload` for
this variant; the prompt says the first call is meant to be refused: a
`write` 15% or more past the stated call bound, then a smaller file past
the payload, a `write` and one appending `edit`; ALPHA; 12 turns, 300 s.
Deciding: one protocol oversize block for `write`, past the stated bound
(a payload refusal alone fails); the first piece `written`, then `edit`s
`edited`; the file, past the payload, byte-equal to the pieces in order;
accepted. A failed `edit` repaired later passes. Why: the production call
bound is past one response's output. In the design: benchmarks.md 11.1,
but for the budget; domain/tools.md 4.

**11.6 `oversized`'s proof count** (2026-10-12). Three exercised passes
within 24 low-effort attempts, else a block; an attempt with no oversize
block that passes the rest is "not exercised", neither pass nor fail; a
fifth deviation also blocks. On a pass, smith 04.10, 09.13 and skein 04.3
are marked proven. At the pause, 24 attempts gave two exercised passes
(3543, 3577): blocked, with no ruling after. Why: honouring a bound smith
states is what smith asks. In the design: in conflict; benchmarks.md 11.1
says "within forty-eight attempts" (`e60c929e`).

**11.7 `cache-reuse`** (2026-10-11). An attempt is a triplet, judged on
its median R (at least 0.85) and at most one full miss per ten requests;
ten passing triplets in twelve, per variant. A control triplet without
affinity must fail; if it passes, that is a finding, not a defect or a
reason to change thresholds. Codex pairs one triplet per variant. Why:
11.5 runs "three where judged on a median". In the design: benchmarks.md
11.1, 11.5, but for the last two sentences.

**11.8 `paths`** (2026-10-12). One byte-identical prompt names
`../spec/rules.txt`, not `/spec`, from `app`, each agent's working
directory; smith refusing it is smith's defect. Why: domain/tools.md 2
reaches siblings by path. In the design: benchmarks.md 11.1 `paths`.

**11.9 `slow-completion`** (2026-10-11). A pass needs the call over 60 s,
the spike's cap (findings S5); under 60 s is "not exercised", counted as a
deviation; past two in twelve, a stronger prompt or xhigh effort, and a
larger model only if xhigh cannot reach 60 s. Source: a real provider,
Codex. A recorded value over half its preset bound is a finding for the
presets, not a fail. Why: the worlds (04.4, 04.11) cover any length. In
the design: benchmarks.md 11.1, but for the larger model and the finding.

### 11.b Limits

**11.10 Memory within 2 GiB** (2026-10-11; 2026-10-12). A worst case over
2 GiB is its owner's repair under limits.md 3; no default is raised. A
change moving a declared or derived quantity rechecks it and records the
margin. Why: 02.6's product bound reached 2,272,511,660 bytes. In the
design: shell.md 6.3; protocol/limits.md 3.

**11.11 One call's bound holds a write** (2026-10-12). One call's stated
bound holds a payload-sized `write` at the worst escape with the longest
path, plus the call's keys and punctuation (35 bytes with `payload.txt`);
smith derives it. Why: without the framing such a write passed it. In the
design: domain/tools.md 4, "One payload bound", but for the framing.

### 11.c Budgets

**11.12 Budget checks** (2026-10-11). `visibility`: the report repeats
what main was last told (with a child, its usage line), else a model
deviation; told turns and seconds equal the writing step's `budget`
record, else smith's defect; a later record and spend are not judged.
`wrap-up` keeps its text check. A result is read from the rendered
request's `Block::Result`, by call id; below `everything` it is
unavailable, never a pass. Why: the model never saw a later record. In the
design: benchmarks.md 11.1; domain/run.md 9.2; protocol/events.md 3.6.

### 11.d Harness

**11.13 Offline tests** (2026-10-11). 4.1's costs are estimates; only
suite budgets bind (15 s, 60 s). A test sits with what it tests. Why:
"under a second" was never a gate. In the design: benchmarks.md 4.1.

**11.14 Exit codes** (2026-10-11). `smith-bench attempt` exits 0 on pass,
1 on fail, 2 when a deciding check is unavailable, not a smoke pass. Why:
passes exited 1 on checks the old trace lacked. Not in the design.

**11.15 A terminal outlives its child** (2026-10-11). The harness and
`skein_world::end_to_end` close a pseudo-terminal's master only after the
child exits, or its teardown bound plus grace. Why: closing first hung up
a finished smith (2867). Not in the design; it bears on skein 02.6.

### 11.e Other

**11.16 Gaps settled from existing rules** (2026-10-11). An
event-vocabulary gap is settled by protocol/events.md's rules ("unknown is
null; ... exactly one terminal per start"), raising only what they leave
open, with the smallest truthful handling. Why: a total past `u64` takes a
broken provider. In the design: protocol/events.md 3.1, 3.10.

### 11.f Superseded (times UTC)

- 10-11 12:40, "none needs 02.6's content": by the 12:47 unblock.
- 10-11 14:44, "model omission": widened at 17:16 (11.2).
- 10-12 00:42, `large-write` v2: by the 01:35 and 02:37 steers.
- 10-12 01:35, prose seeds and the v5, v6 route: by the 02:37 steer (v7).
- 10-12 01:35, "pieces, each `written`": by the 01:42 unblock (11.5).
- 10-12 01:42, "one `write` refused `oversized`": by 05:23 and 05:29.
- 10-12 02:37, "Do not write a v8": lifted by the 03:29 unblock.
- 10-12 03:29, v8 at medium and "Do not write a v9": by the 05:23 unblock.
- 10-12 03:35 and 04:37, the v7 and v8 oversized order: by 05:23 (v9).
- 10-12 05:23, low then medium, block if medium fails: by 06:15 (11.6).
- 10-12 05:23, "for payload.txt's write": by the 05:29 unblock (11.3).
- 10-12 07:35 steer, 48 attempts: withdrawn unsent, so never a ruling.

## 12. The port (2026-10-12)

1. **The plan lives in smith,** in `docs/development/plans/reliability/`,
   in development.md's shape (section 5.2), and holds only the work left.
   `plan` runs only a plan inside a repository. The first run's
   directory, `~/src/rust/plans/reliability-plan/` (not versioned), stays
   as that run's record, for the retrospective. Its `findings/scripts/`
   (Python, the spikes' analyses) stay there; `findings/` here holds the
   findings' text.
2. **The pin line is over.** smith pins skein's `main` (at the port,
   `554a878`), past the teardown invariant, and smith's gate passes there
   (`564d4408`). Nothing pins `pin/smith-pre-invariant` or a `smith-pin/*`
   tag again, and the old 6.11's repin is gone. The tags and the branch
   stay local to the first run's machine and are not published (section
   10, item 5).
3. **The per-merge smoke** follows smith's `workflow.md`, section 5, as
   amended in `564d4408`: `smoke-tools`, live, once, on the branch rebased
   on `origin/main` just before `land`, and once more only if it fails.
   It is not in `scripts/check.sh`: the gate stays deterministic, needs no
   credentials, and does not fail when a provider is at capacity. The
   first run smoked "the tip that becomes main"; `land` now owns the last
   rebase, so that tip cannot be smoked before it lands. Why: one smoke
   per landing keeps the user's rule (2026-10-10) at its cost of about ten
   seconds and a cent.
4. **A probe's calibration is the Proof of the increment that calibrates
   it,** run once as one batch of at most 12 attempts per variant, with a
   stated budget, and no new task versions. A pass proves the increments
   the probe guards (README, "Proving what landed"). A shortfall that is
   not a smith defect (model deviations, a task the tier cannot do,
   attempts not exercised) still lands the step: the probe is recorded as
   not calibrated, with its counts; the increments it guards rest on
   their deterministic tests; its next task version goes to `later.md`.
   Steps 1 to 3 of the first run's proving stand; nothing is calibrated
   again because the pause's reconcile moved `main` (development.md,
   section 5.5: a rebase does not run a proof again). Why: section 12.11;
   the first run's step 4 took about eight hours around minutes of live
   attempts.
5. **`large-write/oversized` continues its count.** Its 24 recorded
   attempts count (two exercised passes). smith 13.1 runs one more batch
   of at most 12; a third exercised pass calibrates it, and otherwise its
   two exercised passes stand with its deterministic tests (12.4).
6. **Old numbers stay.** Sessions and increments keep their first-run
   numbers (`smith-06.4`), so commits, smith's `docs/design/STATUS.md`
   and the first run's records stay mapped. A session with more than
   five increments left is not split: its implementer hands over between
   increments (development.md, section 7.3). New increments take the next
   unused number of their session; session 13 is new.
7. **No global fence.** The first run's "phase 4 waits for phase 3" is
   gone: each session starts when what it needs has landed. Why: `main`
   makes live calls again, and each new improvement brings its own probe.
8. **Repin: none.** The plan changes skein and smith only; jig and temper
   are never edited by it (the user, 2026-10-09), so it repins neither.
   jig moves its pins, and its Cargo URLs, when it takes smith's `main`
   (the workspace's `AGENTS.md`).
9. **Every landing pushes,** through `land`, to GitHub (the user,
   2026-10-12, in the workspace's `AGENTS.md`).
10. **The rulings of section 11** bind the sessions they name, as the
    plan does. A ruling that the design has taken in names the section.
11. **Live proof is bent where it helps the run** (the user, 2026-10-12:
    the guideline that each step is proved for real "can be bent, given
    the circumstances, if it helps"). For this plan:
    - a new increment proves live only when a calibrated probe that
      guards it is on `main` by then; otherwise at the highest layer short
      of live, naming the probe that would show it live;
    - no increment builds a probe only to prove itself, beyond the probes
      session 00 builds;
    - session 13 calibrates in one batch per variant (12.4);
    - a smoke stopped by the provider is not smith's failure: an end
      `Refused(setup)` on the credentials, or `Failed` with the
      provider's capacity or a server error as its reason (benchmarks.md,
      section 8.3). It is run once more after 60 seconds; stopped by the
      provider again, the landing goes ahead, its report says "smoke not
      run: <the end and its reason>", and the coordinator escalates it as
      `credentials` or `infrastructure` (development.md, section 10.2).
      Any other failed smoke is red, and stops the line (workflow.md,
      section 5).

12. **One command per probe run comes first** (smith 00.27). The first
    run prepared every live attempt by hand, with scripts outside the
    repository that checked digests and wrote manifests; `smith-bench`
    has no command that builds the binary, prepares the request and runs
    an attempt. The per-landing smoke and session 13 need one, so 00.27
    is session 00's first increment, and smith landings before it skip
    the smoke (README, rule 1). Why: development.md, section 5.5, "a
    script needed twice is kept" in the repository.
13. **benchmarks.md, sections 3.1 and 11.5, now read "ten passes within
    at most twelve attempts"** for a calibration, the other attempts
    failed only by the model, as the first run ruled (11.2); they read
    "ten passes of ten". Why: one model deviation in twelve is the small
    tier's measured rate, not smith's failure, and the first run's
    calibrations of steps 1 to 3 were judged so.
14. **`cache-reuse` calibrates without a failing control.** smith's
    configuration refuses a static cache key, and benchmarks.md 4.2 adds
    no switch only to make a before, so no arm without affinity exists.
    Its before is 4.2's closest evidence: the spike's recorded
    measurements (findings C1). The control arm stays in the design for
    when a configuration can express it.
15. **`large-write/oversized` at two exercised passes proves what it
    guards** (smith 04.10, 09.13, skein 04.3), named so in 13.1's commit,
    if its last batch adds no third (12.5). Once the oversize block
    arrives, smith's handling of it is deterministic, and its tests show
    it.
16. **What session 13 does not prove** (README, "Proving what landed"):
    increments that ran in steps 1 to 3's calibrated binaries are proven
    by those steps; those whose probes session 00 still builds
    (`follow-up`, `history`, `credentials`) are proven when those probes
    calibrate; those whose session files named no probe stand on their
    deterministic tests; skein 02.4 and 03.2 are shown by `smoke-tools`'
    teardown check once smith 06 lands.
17. **The working tier for session 12** is `gpt-6-astra` at `high`
    effort, the first run's proposal, pinned in
    `benchmarks/agents/models.toml` by smith 12.1. Only session 12's
    comparison and repository runs use it (section 10, item 6).
18. **Design gaps closed for this revision** (the designer, 2026-10-12,
    in the landing that carries this plan):
    - events.md 3.8 and run.md 11: a `budget` record as a sub-agent's
      return writes its usage line, with `call` (smith 07.14);
    - events.md 2: the product's stream goes to the trace path, a
      spawned agent's own trace to that path with `.agent` (smith 03.9);
      3.1: compactions are numbered among a run's conversations;
    - limits.md 4: the check `cancel-over-close`; 7 and shell.md 6.5:
      the teardown bound counts the stream's `write_deadline`, as
      agent.md 6 does; 9: per-endpoint and per-model limits named by
      their configured names;
    - shell.md 4.1: end of input with unread lines opens one more
      activation, then exits;
    - benchmarks.md 3.1 and 11.5: ten passes within twelve (12.13); 11.1:
      `follow-up` does not judge the "waiting" prompt, which the local
      worlds check.
19. **Smaller rulings, so the sessions do not stop on them:**
    - under `mode: agent`, `session.started` carries no `chat`: the
      writer refuses one, and a reader treats it as absent;
    - a cancel that races an accepted answer drops the unread lines as
      after any cancel (shell.md 4.1);
    - the local store's bound on turn files counts the current window's
      files only, so a long chat keeps loading (domain/host.md, 6);
    - an agent's exit that io could not observe is `exit: None`, shown
      as "status unknown" (hosts.md, section 3);
    - a chat's LLM pool closes at the chat's last word, not at each
      answer;
    - lines a crash lost get no notice in this plan (`later.md`).
20. **`probes` is the catalogue; calibration is a committed summary.**
    `suites/probes.toml` (smith 00.29) lists every probe and variant, as
    benchmarks.md 11.5 says, and `calibrate` reads each one's pin from
    it. A variant's calibration is recorded by the summary its
    calibration run commits under `benchmarks/summaries/` and by the
    `Calibrated:` line of the step's commit; a probe gates nothing until
    then (benchmarks.md 3.1, amended in this landing). Session 13 adds no
    variant to a suite and removes none.
21. **`first-request` has two faces as two variants:** `probes/first-request`
    drives the interactive product (smith 10.8's proof), and
    `probes/first-request/headless` runs `smith exec` (smith 10.15's).
22. **smith 03.7 proves at the world layer.** 00.27's commands take each
    probe's pin from `probes.toml`, one per probe, so no live run can pick
    the inline kind's pin; the per-landing smoke after smith 10.15 runs
    the product with it.
23. **`cancel-over-close`'s margin is one `close`:** `cancel_grace` at
    least twice io's `close` (limits.md 4, amended in this landing);
    smith 04.14 adds the check.
24. **A whole suite runs in one gate-lane call** (00.21's `suite probes`,
    session 12's suites): a suite's attempts cannot be split across
    calls. 00.21 runs after session 13, when few sessions are left to
    land.

## 13. This run's rulings (from 2026-10-12)

1. **Users the Files line misses are migrated in the increment** (blocks
   of 2026-10-12T13:24:02Z, smith 06.1, and 13:24:43Z, skein 02.6). Both
   sessions found that a type changed in place, as the plan asks (06.1's
   `End::Stopped` to `End::Exited`, with `cleanup` taking the exit; 02.6's
   new `io::Event` and `Request` variants), breaks code that the Files
   line does not name: three expected values and four callers for 06.1,
   exhaustive matches in about fifteen files for 02.6. code-organization.md,
   section 7, already says "change the type in place, and its users in the
   same increment"; development.md 8.2's limits on an edit outside Files
   (20 lines, no expected value changed) made that a question. Ruling: a
   plan defect, mine. README rule 8 lets the mechanical edits through,
   named in the commit body, and keeps every other edit outside Files a
   question. 06.1's Users line and 02.6's Files line name what was found.
2. **skein 02.6 gives `Window` room in io's operation table** (block of
   2026-10-12T13:24:43Z). `Window` is a request io submits as an
   operation, as `Usage` is (kernel.md, 6.3; `skein-io/src/layer.rs`, the
   `Request::Usage` arm), so io's bounds must count it: `limits.rs`'s
   `operations`, and with it `flights` and `worst_case`. `limits.rs`
   joins 02.6's Files. How it is counted, an independent slot as
   `Usage` has or a place among the stream's `PER_ENTITY` operations, is
   the implementer's choice, named in the commit body; the memory test
   that attains `worst_case` stays green, with no new test of its own.
3. **A lookup the Shape needs, in another crate** (block of
   2026-10-12T13:26:20Z, smith 04.7). 04.7's Shape gives `header(&[Limit],
   &channel::Endpoints, &Configuration)`, but `channel::Endpoints` finds a
   name only by number and dialect (`name_of`, `translate.rs:260`), and
   the configuration's endpoint has no dialect. Endpoint numbers are
   unique: the agent's configuration refuses a duplicate
   (`smith-agent-shell/src/config.rs:368`, "duplicate endpoint name or
   number"). Ruling: 04.7 adds `Endpoints::name_for(number) ->
   Option<&[u8]>` beside `name_of`, and keeps the Shape's signature. The
   third block of the run's first ten minutes on an edit outside Files:
   README rule 8 gains this case, a lookup or accessor of at most 20 lines
   that the Shape's signatures need.
4. **smith 03.7 at 600 lines, with its fixture and manifests** (block of
   2026-10-12T13:36:44Z). 03.7 needs about 500 production lines against
   its Size of 400: the parent face's answer projected whole (about
   180), the `Effects` stream boundary (about 85), the trace's startup
   and adoption, and the local service's drain. code-organization.md,
   section 8, splits an increment only above about 600, and a split now
   would cost more than it saves, so 03.7's Size becomes 600. Its world
   needs about 43 lines in `tests/local-process/src/process.rs`, the
   factory that owns the product's startup in that world, and
   `smith-events` in three manifests. The fourth block of the run's first
   twenty minutes on an edit outside Files: README rule 8 gains two cases,
   a dependency a `Cargo.toml` needs, and at most 60 lines in the library
   of the world crate the Test shape names.
5. **The process referee allows successive activations** (block of
   2026-10-12T13:37:41Z, smith 05.12). 05.12's process sweep draws one
   to three lines then end of input, and the process world's referee
   (`tests/local-process/src/referee.rs:78`) checks "one activation per
   invocation", which shell.md, 4.1 contradicts: "Lines it left unread,
   after an accepted or parked answer, open one more activation, which
   answers or parks in turn (domain/host.md, section 8, "Closing input
   drains"); then the product exits." The referee is the plan's to fix:
   its rule becomes "an activation starts only after the one before it
   answered", and each `Started` resets the activation's turn, answer and
   shown state and expects its own terminal; its other checks stay.
   Confining the scripts to one line, the alternative, would leave the
   design's drain untested at the process layer.
6. **The harness's adapter follows 07.14's `call`** (block of
   2026-10-12T13:41:01Z, smith 07.14). `FactKind::Budget` gains `call`,
   and `smith-bench`'s legacy reader refuses a field it does not list
   (`benchmarks/src/agent/smith/legacy.rs:544`), so the gate fails on
   `legacy_recordings_terminals_and_emission_metadata`. 07.14's Don't
   forbids `benchmarks/src/`, for smith 00's told check; README rule 4
   asks for the opposite here: "An increment that changes how smith is
   invoked, configured or observed changes `smith-bench`'s smith adapter
   in the same increment, so probes keep running." Rule 4 wins for the
   adapter: 07.14 adds `"call"` to that field list, and touches nothing
   else under `benchmarks/src/`.
7. **smith 10.1's store root is a descriptor** (block of
   2026-10-12T13:44:06Z). 10.1 says "the shell opens the state directory
   as a root and owns no file once the loop runs", but
   `smith-local-shell`'s `Resources` carries `state_directory: PathBuf`
   (`local_host.rs:41`) into a synchronous `Store::new`, while the worlds
   hand the product its other roots as inherited descriptors. A root
   beside the path would keep two representations (code-organization.md,
   section 7). Ruling: the field becomes `state_root: kernel::Fd`, opened
   before `Local::new`, in place, and 10.1's Files name its two users,
   the process world's factory and `local_library.rs`. The worlds'
   `Launch::state_directory`, which tests read and fault, stays. smith
   03.7 also edits `tests/local-process/src/process.rs`; whichever lands
   second rebases (development.md, section 8.5).
8. **The hosts referee accepts the agent's failure line** (block of
   2026-10-12T13:44:35Z, smith 06.2). 06.2's Shape has the shell write
   "`smith: run failed: <words>`", and `tests/hosts/src/referee.rs:124`
   accepts only the lines the agent wrote before it; three hosts stories
   fail. The referee accepts lines that start "smith: run failed: ", and
   nothing else changes in it. The second block in which a referee
   rejected what the Shape newly requires (13.5 was the first): README
   rule 8 gains that case.
9. **Size lines are guides up to 600** (block of 2026-10-12T13:45:18Z,
   smith 04.7). 04.7 needs about 570 production lines against its 450:
   the inventory (about 407 lines over 14 declarations, 22 policies and
   every layer's fields), the header (about 80) and its wiring (about
   80). The second block on a Size line in the run's first half hour
   (13.4 was the first), each under code-organization.md 8's split
   threshold of about 600. README rule 9 makes every Size line a guide up
   to 600, and 600 the rule; 04.7's Size reads 600.
10. **An io assertion is its caller's bug until a backtrace says otherwise**
   (block of 2026-10-12T13:45:54Z, smith 05.12). 05.12's process sweep
   panics at `skein-io/src/stream.rs:512`, "an announced socket is bound
   or rejected, not closed". skein's io.md, 3.1: "One naming an entity of
   the wrong kind, or an announced socket before its answer, is the layer
   above's bug, asserted"; 3.3's table: "Opening, Announced | `Close`,
   `Abort`, `Stream` | impossible". Ruling: one run with a backtrace names
   the sender; a sender in smith is fixed in 05.12 (at most 40 production
   lines, with a test failing on its parent); a sender in skein comes back
   for a skein step and a repin. The sweep is not narrowed.
11. **07.14's reader change is what the reader needs, not one line**
   (block of 2026-10-12T13:49:37Z, smith 07.14). Reverses the unblock of
   2026-10-12T13:41:46Z in part: its "Nothing else under `benchmarks/src/`
   changes in 07.14" left `validate_fact`'s optional-token arm
   (`legacy.rs:581`) reading `Budget`'s `call` as a bare token, so the
   gate failed again. README rule 4 asks that the adapter keep reading
   what the binary writes; 07.14 changes `legacy.rs` as far as that
   needs, the field list and that arm, and nothing else under
   `benchmarks/src/`.
12. **skein-07.s1: the fake peers reject what they will not take**
   (block of 2026-10-12T13:53:24Z, smith 05.12). The backtrace that 13.10
   asked for names the sender: `skein-fake-peers/src/face.rs:97`, called
   by `llm.rs:365`; `llm.rs:156` answers an announced socket with
   `Abort` when stopping or full, and `oauth.rs:171` does the same. io.md
   asks for `Reject`. A step not in the plan, in a new session file,
   `skein/07-fake-peers.md`, so that a new implementer takes it and skein
   02.6 goes on. smith 05.12 then repins skein at exactly that commit:
   skein's `main` between smith's pin (30dc6f7) and now changed only
   Markdown, and a precise pin keeps skein 02.6's new io records out of
   smith until smith 10.10 takes them.
13. **smith 08.6 splits; 08.10 takes the service's records** (block of
   2026-10-12T13:58:11Z). 08.6's work in progress adds about 645
   production lines before restored windows, the memory allowance and
   the world's support, past README rule 9's 600: `compaction.rs` 283,
   `window.rs` 104, the service's response joining 118, the session's
   call sites 50. Ruling: the implementer's split. 08.6 keeps the domain
   and root mechanism and its agent-world proof; 08.10, the next unused
   number of session 08, takes the service's compaction conversation in
   the stream, with a unit proof. Between the two, the stream tells a
   compaction's responses as an ordinary completion's; nothing reads
   that live until smith 00.16, which now needs 08.10 too.
14. **`smith-bench build` serializes on its clone** (block of
   2026-10-12T13:59:01Z, smith 04.7). `arm::build` fetches and checks out
   each commit in one shared clone, `$XDG_STATE_HOME/smith-bench/clone`,
   and builds into one shared `build-target` (`arm.rs:151-165`, `:343`);
   the build lane runs two tasks at once, so a session's build of its
   commit compiled another's checkout. Ruling: 00.27, which owns `build`
   and is not landed, takes an exclusive file lock over the whole build;
   a second build waits. Not the gate lane: 00.27's Shape keeps release
   builds out of it. Until 00.27 lands, no smoke runs (README rule 1), so
   04.7 lands without one.
15. **smith 10.1 lands whole, at up to 1,000 lines** (block of
   2026-10-12T14:04:18Z). The store component alone is about 731
   production lines, before about 100 to 150 of routing and the shell's
   root, past README rule 9's 600. A split cannot bring both parts under
   600: the component alone is past it, and a component landed unused
   would stand beside `local_store.rs`, two owners of the chat's files
   (code-organization.md, section 7). Ruling: 10.1 stays one increment,
   at most 1,000 production lines, its commit body giving the count by
   file. The exception is 10.1's; rule 9 stands for the rest.
16. **skein 02.6's pinned operation counts move by `Window`'s slot**
   (block of 2026-10-12T14:23:43Z). 13.2 let `Window` take an independent
   slot, as `Usage` has; the tests that pin `operations()`
   (`examples/echo/service/src/tests.rs:316`, `Some(17)`;
   `crates/skein-io/src/tests/layer.rs:81`, `Some(9)`) move by exactly
   one, and nothing else in them changes.
17. **smith 00.13 pins the working tier** (block of 2026-10-12T14:34:22Z).
   00.13's `compare.toml` names tier `working`, and the offline check
   validates every suite's agent through `models.lookup`, which refuses
   the empty `[working.codex]`; 12.1 was to pin it, and 12.1 needs 00.13.
   The tier is decided (12.17: `gpt-6-astra` at `high`), so 00.13 pins it,
   and the admission code stays as it is. Pinning spends nothing: only
   session 12 runs that tier, and the user's veto (section 10, item 6)
   stays open until then.
18. **smith 06.4 fixes the local adapter's read admission** (block of
   2026-10-12T14:34:22Z). 06.4 finishes the channel at the answer
   (agent.md, section 6), and the earlier close shows that
   `smith-local-service/src/process.rs` ignores the host's `Read`
   (:814-816) and hands it every record it receives (:611-617); the host
   domain asserts "record ends one requested read"
   (`smith-host-domain/src/domain.rs:325`), and `a_first_run_signs_in_and_ends_with_a_commit_in_place`
   panics in the gate. A plain defect, in session 06's crates; fixed in
   06.4 with a regression test, at most 80 production lines. The
   assertion stays: it is the host's protocol, and the adapter broke it.
19. **A sequence test consumes what the Shape newly requires** (block of
   2026-10-12T14:42:57Z, smith 03.7). 03.7's Shape: "its
   `Request::Answered` moves the answer to the service once shown, owing
   no terminal". `smith-local-domain/src/parent_tests.rs`'s
   `a_message_refused_after_the_last_word_keeps_the_single_transcript_reload`
   asserts Show, Load and Notice in order, and now meets `Answered`
   between Show and Load. The test gains one assertion that consumes it,
   and nothing else changes. The case is 13.8's in a unit test: README
   rule 8's referee case now names sequence tests too.
20. **An expected value follows a behaviour the Scope names** (block of
   2026-10-12T14:45:30Z, smith 05.13). 05.13's Scope: "the line that
   starts a run travels in its start's messages" (shell.md, 4.1: "The
   line that starts an activation travels in its start."), so
   `tests/local/tests/simulation.rs:512`'s
   `a_crash_before_a_turn_is_saved_resumes_from_the_turn_before_with_a_new_activation`
   now sees its first saved turn's fence at `Some(Token::new(1))`, not
   `None`. The assertion changes, and nothing else in the test. The
   fourth block of this kind (13.1, 13.8, 13.19): README rule 8's
   expected-value case now covers a value the story now produces because
   of a behaviour the Scope or Shape names, quoted in the commit body.
21. **A landed step is checked on its own branch** (block of
   2026-10-12T14:58:25Z, smith 12.9). development.md 7.3 checks a landed
   step with `git status --porcelain` printing nothing, and 8.6 tells the
   implementer to "send the report and go on": by the coordinator's
   check, smith 12's worktree held 12.10's work, and 12.9 (09d029e7, on
   `origin/main` with its `Step` trailer) waited. Ruling: 12.9 is
   `landed`; README rule 10 runs the porcelain check only while the
   worktree is still on the landed step's branch. A proposal for the
   retrospective: development.md 7.3's third check says the same.
22. **smith 03.9's harness face needs no bootstrap** (block of
   2026-10-12T15:02:53Z). `smith-bench` picks a face as the newest
   `faces.toml` entry whose commit is an ancestor of the built arm
   (`benchmarks/src/agent/smith/faces.rs:162`), and 03.9's own commit is
   not known before `land` rebases it. Pinning the new face to 03.9's
   landed commit in a following step would leave a window in which other
   sessions' smokes build arms with 03.9 under the old face, and fail.
   Ruling: the new entry is pinned to a commit already on `origin/main`,
   and reads the run's records from `<trace>.agent` when the product's
   spawned agent wrote one, from `<trace>` otherwise: before 03.9 no
   binary writes `.agent`, so arms from then read as before; with the
   inline kind the run's records stay in the product's stream (03.7).
23. **A diagnostic never committed** (block of 2026-10-12T15:02:53Z,
   smith 10.1). The one-off A/B of 13.10's kind showed 10.1's regression:
   `main` passes and the branch leaves a child running. skein-world's
   tree check kills the child before it reports its pid, so its command
   is gone. 10.1 may add a diagnostic in its worktree that prints the
   leftover child's command once, run the story once, and remove it
   before its commit; nothing of it lands.
24. **03.9's harness reads `.agent` once it exists; the world allowance is
   100 lines** (block of 2026-10-12T15:11:25Z, smith 03.9). The harness
   opens the trace it reads before it spawns the binary
   (`benchmarks/src/process.rs`, `trace_file`, `execute_script`), so 13.22's
   choice of `<trace>.agent` waits until that file exists; that code joins
   03.9's Files, for that choice only. 03.9's world needs about 70 lines in
   `tests/local-process/src/{process.rs,world.rs}` for the second trace
   and its reader, past README rule 8's 60; the second time a world
   library needed more (13.4 was 43), so rule 8's allowance becomes 100.
25. **Diagnostics in a worktree, never committed** (block of
   2026-10-12T15:16:15Z, smith 10.1). After the one-off of 13.23 named
   the leftover child (`smith agent .../agent.json`), 10.1 asked again for
   a second print, the product's buffered stderr, before the kit's
   cleanup. Each such print is a question under development.md 8.2, and
   none of them lands. README rule 11 lets an implementer add them in its
   worktree, for at most three runs per failure, and remove them before
   its commit.
26. **smith 08.6 at up to 700 lines** (block of 2026-10-12T15:16:15Z,
   size half). The designer's answer (17bba0da, 85308b95) has 08.6 give
   the window's limit its cause in the service's terminals, beyond the
   split's match arms (13.13), while the candidate already holds about
   600 production lines. The projection belongs with the end that
   causes it, and a split now would land 08.6 with a gate that fails, so
   08.6 may take up to 700, its commit body giving the count by file.
27. **shell-cwd's bundle stays on this machine** (block of
   2026-10-12T15:29:16Z, smith 12.2). 12.2's session file already names
   the bundle's place, `$XDG_STATE_HOME/smith-bench/bundles/shell-cwd-56c0c59/`,
   and benchmarks.md, section 15 leaves open "where a bundle is kept
   durably, so that another machine can run the task". This run needs
   only this machine, where 12.5 runs, so 12.2 goes on as written and the
   durable place goes to `later.md`; nothing waits on the user.
28. **smith-12.1's Plan trailer is lost; the step stands** (block of
   2026-10-12T15:30:05Z, the coordinator). 42f2faec's message has a blank
   line between `Plan: reliability` and `Step: smith-12.1`, so git reads
   only the last paragraph as trailers and `plan status` does not count
   it. The commit stays (no force-push); the coordinator keeps 12.1
   `landed`, and every implementer checks both trailers before `land`.
   For the retrospective: `land` could refuse a branch whose tip lacks
   the `Plan` or `Step` trailer during a run.
29. **smith 10.1 takes 06.4's read-right fix, and 1,050 lines** (block of
   2026-10-12T15:30:57Z). 10.1's diagnostic (README rule 11) found the
   local process adapter ignoring `Request::Read`, the defect 13.18 gave
   06.4, which landed its fix in `crates/smith-local-service/src/process.rs`
   (a398ca43, after 10.1's parent dc1a4549). 10.1 rebases and keeps 06.4's
   fix, with no read classifier of its own. Its candidate stood at 1,006
   lines against 13.15's 1,000; folding a module into another to fit a
   count is code bent to a number (development.md 4.5), so the bound
   becomes 1,050.
30. **smith-00.s1: the harness's test programs in Rust** (the check of
   2026-10-12T15:32:00Z). 00.27 (5a3bc557) added a replay arm whose
   program is a Python script, beside an older Python terminal peer in
   `smith_adapter.rs`; the user's ruling of 2026-10-09 is Rust, unless a
   compelling reason goes through the user, and a test's fake agent is
   not one. A step not in the plan, 00.s1, makes both one Rust fixture
   binary, and drops 00.27's "immutable" from `benchmarks/README.md`. The
   Pulse's words gain `python3`, so a new one shows in the next check.
31. **The Rust terminal peer refuses an ignored SIGHUP** (block of
   2026-10-12T15:44:11Z, smith 00.s1). The Python peer set SIGHUP to its
   default so a hangup could not pass unseen; `smith-bench` forbids
   `unsafe`, 00.s1 adds no dependency, and skein's safe surface sets no
   disposition. The peer reads `SigIgn` from `/proc/self/status` and exits
   3 with its reason when SIGHUP is ignored: the test cannot pass while
   blind to a hangup, and no skein function or repin is needed.

32. **A person's cue that waited for the crossing's notice moves to the
   answer** (block of 2026-10-12T15:48:34Z, smith 05.14). 05.14's Scope:
   "a wait whose fence is behind is a crossing, and shows nothing"
   (host.md, section 8: "\"Waiting\" is shown only when every line
   relayed is read"). `tests/local/tests/person.rs`'s
   `a_person_typing_on_the_first_turn_has_both_lines_read` closes input
   on `Trigger::Notice(Kind::Waiting)`, and its only waiting notice was
   the one after turn 2, whose fence is line 1 with line 2 relayed: the
   defect 05.14 removes. With it gone, turn 3 reads line 2 and the run
   answers, and nothing closes input. Ruling: a plan defect, mine; that
   cue becomes `Trigger::Answer`, which the person meets once, after
   turn 3, and nothing else in the test changes. Another provider turn
   before the wait, the alternative, would change the world's script for
   one story. 05.14's own regression asserts no waiting notice while
   line 2 is unread and the store's read fence at line 2 at its end; the
   Test shape's "the chat waiting with the store's read fence at the
   second line" is read so, since this script reads line 2 only in the
   turn that answers. A step of an existing story changed this way is
   not in README rule 8; a second such block adds it.
33. **sha2 is optimized in dev and test builds** (block of
   2026-10-12T15:52:22Z, smith 00.s1). The Rust test peer that replaces
   the replay arm's Python script is a 6.2 MiB debug executable, and
   `smith-bench` checks an arm binary's SHA-256 each time it reads the
   arm (`benchmarks/src/arm.rs:269`), so
   `a_replay_arm_runs_through_the_probe_command_and_prints_one_line`
   went from about 0.35 s to 2.939 s, past twice the 0.5 s ceiling
   (development.md 8.2). `smith-bench` is sha2's only user in
   `Cargo.lock`. Ruling: the root `Cargo.toml` gains
   `[profile.dev.package.sha2]` with `opt-level = 3`, which the test
   profile inherits: no dependency, every check kept. Not a stripped
   fixture, which would need a tool and a preparation step. The table
   joins 00.s1's Files, for that table only.
34. **A signal after `session.ended` is written changes no record**
   (block of 2026-10-12T15:54:32Z, smith 06.5, sent as a design gap).
   The design settles it, so it is not one. agent.md, section 6:
   "When all have, it writes `session.ended` and closes the event
   stream", and "A termination signal after the answer turns every close
   still in progress into an abort"; events.md, 3.2: "`session.ended`,
   authoritative, last", "written, once". A signal in `Closing`'s
   `Components` or `Usage` step comes before the terminal: it says
   `aborted`, after a `teardown_aborted` notice. A signal in its `Stream`
   step finds only the stream's closes in progress: they become
   `io::Request::Abort`, `session.ended` stands as written, and no
   record follows it, as `sinks.rs:457` asserts. Signals stay read
   through every `Closing` step, as 06.5's Scope says ("`Answered` or
   `Closing`"), and no test is added beyond its Test shape.
35. **03.8's two interrupts are two signals in one pass** (block of
   2026-10-12T16:01:54Z, smith 03.8). 03.8's Test shape had
   `World::dropped` deliver "both interrupts in one read of the signal
   stream"; skein's kernel `Op::ReadSignal` completes with one blocked
   signal, and `signals::up` emits one `Shutdown` and reads again, so no
   read carries two. A plan defect, mine: what the story needs is the
   rest of that sentence, "so the second reaches the local domain before
   any terminal of the cancel". The terminal sends both signals in one
   pass while the provider holds its response open; no skein change. If
   the two arrive as one `Shutdown`, the implementer stops and asks,
   adding no delay or timing to separate them.
36. **A drawn person's later Waiting cue moves to the answer** (block of
   2026-10-12T16:03:59Z, smith 05.14). The fuzzy sweep
   `drawn_people_settle_against_the_message_rules` failed at seed 45,
   "person closes and chat exits": `Person::drawn`
   (`tests/local/src/person.rs:94`) can cue the third line on
   `Notice(Waiting)` after the second line crossed a wait, and with 05.14
   that notice no longer shows ("a wait whose fence is behind is a
   crossing, and shows nothing"), so the cues after it, `Close` among
   them, never come. Ruling: draw 1 stays `Trigger::Notice(Kind::Waiting)`
   for the second line, where the first line has been read, and becomes
   `Trigger::Answer` for the third; the draws, seeds, counts and other
   branches stay. The second block of 13.32's kind: README rule 8 gains
   that case, a scripted cue that waits for what the Scope or Shape now
   withholds.
37. **03.8's queued signals: find the order before changing them**
   (block of 2026-10-12T16:06:54Z, smith 03.8). With 13.35's two
   `Signal::Terminate` in one pass, both dropped stories cancelled and
   answered instead of dropping. The implementer's reading, standard
   signals coalescing, does not hold in this world: at smith's skein pin
   (26502c96) the simulation queues each signal in order
   (`skein-sim/src/sim.rs:317-321`, a `VecDeque`), and the kit's
   `kernel::Signal` has only `Terminate` and `Kill`
   (`skein-io/src/kernel.rs:537`), so "Interrupt then Terminate" is not
   on offer. Ruling: the implementer's other proposal, README rule 11's
   diagnostic (at most three runs, removed before the commit), naming
   when each `Shutdown` reaches the local domain against the cancel's
   terminals. A cause in 03.8's Files is fixed in 03.8; a cause in skein
   comes back as a question, with the trace.
38. **README rule 11 counts diagnostic runs, not every run** (block of
   2026-10-12T16:11:12Z, smith 03.8). 03.8's new story
   `a_store_failure_completes_as_host_store_with_its_turn` failed three
   times as its fixture was rewritten (the scripted finish, then `wait`,
   each answering before the failed turn was durable), and the
   implementer stopped under rule 11 to ask for a fourth run. Rule 11
   (13.25) bounds diagnostics: prints and assertion messages added to
   find a cause, never committed. A run after a change to the
   increment's own code, tests or fixtures, with no such line in place,
   is ordinary work, and rule 11 now says so. 03.8's correction, a
   fixture on the world's `changed` workspace story whose scripted calls
   read `original.txt`, is its implementation choice, within rule 8's
   100 world-library lines.
39. **03.8's interrupts go through skein-world's controls** (block of
   2026-10-12T16:15:38Z, smith 03.8). Reverses 13.35's means, not its
   aim. 13.37's diagnostic found the terminal's second
   `io::Request::Signal` dropped by skein-io (`process.rs:241`, a child
   whose `signalling` is not `Idle`), which skein's io.md, 3.4 bounds:
   "a child waits and may have one signal in flight". The kit keeps its
   design; queuing a second would change io's operation counts, a skein
   design question for no gain here. The person's interrupts are signals
   the product receives, and skein-world delivers them in order:
   `Controls::signal(host, ServiceSignal::Interrupt)`
   (`testing/skein-world/src/referee.rs:173`, at smith's pin 26502c96),
   from a world built with `World::new_controlled`, as skein's
   `tests/world/tests/signals.rs:22-23` sends two in one step.
   `World::dropped` does so, within README rule 8's 100 world-library
   lines; no skein change, no repin, no timing. For the retrospective:
   io drops a second child signal in flight without telling its owner.
40. **08.6's maximum-context story gets a window its system text fits**
   (block of 2026-10-12T16:19:01Z, smith 08.6). 08.6's Scope: "The
   window check before each completion" (session.md, 8.1).
   `tests/protocol-llm/tests/root_memory.rs:1579`'s
   `a_restored_run_keeps_full_brief_and_custom_paths_with_two_clients`
   builds a system text past 16,384 bytes, by design, under a charter and
   configured model of 16,384 tokens, so its system alone fails the
   check and the first `Complete` is withheld. The story is about root
   memory with a maximum brief and paths, not about the window. Ruling:
   the implementer's option A, a plan defect of the kind 13.20 met in
   an expected value, here in a fixture's input: that story alone takes
   a 65,536-token charter and configured-model window, through a
   constructor parameter or helper; its history, wire, output and
   receiving caps, steps and assertions stay. Not option B, which would
   lose its memory coverage. A second block of this kind adds it to
   README rule 8.
41. **Codex's arm runs a private copy of codex-cli 0.160.0** (block of
   2026-10-12T16:27:43Z, smith 13.1). `agents/codex/standard.pin.toml`
   pins 0.160.0, and smith 12's choices say "another installed version
   refuses setup" (`benchmarks/src/agent/codex/prepare.rs:51`). The
   user's Codex updated itself to 0.162.1 at 2026-10-12T09:48
   (`~/.local/bin/codex`, a symlink into
   `~/.codex/packages/standalone/releases/0.162.1-x86_64-unknown-linux-musl/`),
   and the harness also refuses an arm whose executable is not a regular
   file. Ruling: the implementer's proposal, in a place session 12 shares:
   the official release asset for 0.160.0
   (`https://github.com/openai/codex/releases/download/rust-v0.160.0/codex-x86_64-unknown-linux-musl.tar.gz`,
   200 on 2026-10-12T16:29Z), unpacked to
   `~/.local/state/smith-bench/tools/codex-0.160.0/bin/codex`, beside
   smith-bench's other state, and first on `PATH` for each command that
   runs Codex. Nothing writes to `~/.codex` or the user's `PATH` (README
   rule 7). Not a re-pin to 0.162.1, which would need the observer
   checked against that version's recorded shapes, and would break again
   at the next update. The README's "Live services" says so.
42. **A run dropped before its answer hears no answer** (block of
   2026-10-12T16:32:06Z, smith 03.8). host.md, 9.2: a dropped run's
   "domain hears nothing more", and "its failure is that it went without
   answering"; "A drop after an answer ... the answer stands". 03.8's
   diagnostic shows both interrupts reach the local domain before the
   parent's answer (`Failed { Cancelled }` at 50 ms), and
   `smith-host-domain` hears it: `drop_run` (`domain.rs:1175`) moves the
   agent to `Terminating`, and `discarded_work` (`domain.rs:899`) admits
   `Up::Answer` there. A defect in the parent face's drop that 03.8's
   first part landed (1a1e8a39), so 03.8 fixes it: the host domain
   remembers a drop asked before an answer and discards a later answer
   of that run, an answer heard before the drop standing; at most 40
   production lines, with a host-domain regression that fails on the
   parent. `domain.rs` joins 03.8's Files for that.
43. **04.14's frozen control keeps its historic `host_tools`** (block of
   2026-10-12T16:32:06Z, smith 04.14).
   `benchmarks/tests/smith_producer.rs`'s
   `the_published_face_preserves_the_native_control_producers_joint_bounds`
   compares the frozen control's writer against its published face, and
   its input inherits the standard profile's policy, whose `host_tools`
   04.14 moves from 2 to 128 (shell.md's values), so its derived items
   rose from 52,754 to 52,880. The input gains `"host_tools":2`, the
   face and every assertion unchanged. The second block of 13.40's kind,
   a fixture's input that the Scope changed under it: README rule 8
   gains that case.
44. **The private Codex is the release's whole package; 13.1's Codex call
   runs again** (block of 2026-10-12T16:36:43Z, smith 13.1). Reverses
   13.41's archive: `codex-x86_64-unknown-linux-musl.tar.gz` holds the CLI
   alone, and both of 13.1's Codex attempts began "Code Mode is
   unavailable because failed to spawn code-mode host
   .../bin/codex-code-mode-host: host executable was not found", then
   could not write. My defect, not Codex's. The supervisor unpacked the
   release's `codex-package-x86_64-unknown-linux-musl.tar.gz` (its
   `codex-package.json` says 0.160.0; `bin/codex` byte-identical to the
   CLI already there, so the arm's digest is unchanged; `bin/codex-code-mode-host`,
   `codex-path/rg` and `codex-resources/`, the layout of the user's own
   0.162.1) in place of the bare CLI, at 2026-10-12T16:38:58Z. The two
   attempts are recorded in 13.1's commit body as a setup defect, not as
   Codex's outcome, and the missing attempts run in one more call, an
   exception to session 13's rule 1 ("once") because the defect was the
   supervisor's.
45. **The Pulse words are checked before `land`** (the check of
   2026-10-12T16:36:04Z). A Pulse word reached `main` a second time:
   `immutable` in `benchmarks/README.md` (00.27, the check of 15:32:00Z,
   reworded by 00.s1), then three times in 10.1 (8d9d5408:
   `crates/smith-local-protocol/src/store/boundary.rs:32` and `:43`,
   `src/tests/store.rs:59`). The words are the plan's (README, "Pulse"),
   and only `plan pulse` read them, after the landing. README rule 12
   has each implementer run one `git diff | grep` before `land`; the
   words already on `main` from the first run stay. smith 10 rewords
   10.1's three lines in its next landing, saying what the code does
   (a turn is stored only where none is, `Expect::Absent`).
46. **README rule 9's bound becomes 700** (block of 2026-10-12T16:52:00Z,
   smith 10.3). 10.3's draft stands at 626 production lines against its
   Size of 600, before its nested-array error locations: the layered
   TOML with origins and a strict schema, the check document, command
   handling. The third Size block past 600 in three hours, each ruled an
   exception rather than a split (08.6 at 700, 13.26; 10.1 at 1,000
   then 1,050, 13.15 and 13.29), since a split would land a part unused
   or with a failing gate. Rule 9 now lets an increment land up to 700
   without asking, its commit body giving the count by file; past 700,
   the plan splits it. Folding or compressing code to fit a count stays
   forbidden (13.29). 10.3 completes its Scope within 700.
47. **smith-05.s1: the turn acknowledged while a line crosses a wait;
   00.14 lands not calibrated** (block of 2026-10-12T16:50:44Z, smith
   00.14). 00.14's batches ran once each: `follow-up` and `eof` 10 of 10,
   `chat` 10 of 12, and every control passed, so none calibrates. The
   two chat failures are smith's: the model called `wait` as asked, the
   person's next line was received 8 ms later and ended it, and no next
   completion started before the cancel at 36 s. A read-only trace
   through `main` (supervisor's sub-agent, 16:57Z) finds no branch in
   the run or the session that skips the call without ending the
   session, and reads the stall as the next `Complete` held for turn 1's
   acknowledgement (the spawned host holds at one unacknowledged turn),
   which does not arrive; the acknowledgement's path runs through the
   local domain, the store and the host domain, which 05.13, 05.14 and
   10.1 changed after the control's commit (27ead2e7). Ruling: a step
   not in the plan, 05.s1, in session 05's file, reproduces it at the
   lowest world and fixes it where the acknowledgement stops; its Proof
   reruns `chat` once. 00.14 lands now with `Calibrated: none`, its
   counts, and the two failures named as handed to 05.s1; its task,
   prompt, cues and checks stay. `follow-up`'s next task version goes
   to `later.md`.
48. **03.10: an unobserved exit is `None`; the codec carries the forced
   exit notice** (block of 2026-10-12T17:07:59Z, smith 03.10, sent as a
   design gap). The design settles both; the plan's Shape and Files
   missed them. (1) 03.10's Shape gives `EndCause::Agent { ended:
   smith_events::AgentExit }`, while 06.1's `End::Exited { exit:
   Option<Exit>, forced }` has no exit when io failed, 12.19 shows an
   unobserved exit as "status unknown", and the codec's answer failure
   already holds `ended: Option<AgentExit>` (`smith-events/src/records.rs:613`).
   The field becomes `Option<smith_events::AgentExit>`: `None` until
   `Gone`'s evidence names an exit, the first cause kept. (2) events.md,
   3.9: "`forced_exit` | warning | `exit`, `forced` | a host's agent did
   not exit on its own"; host.md, section 4: "`forced`, whether the kit
   had to end it (no, terminated or killed)". `smith_events::Notice` has
   neither field and `NoticeKind` no `ForcedExit` (`records.rs:369`,
   `:971`), so 03.10 adds them, with the reader, the writer, a golden
   under `golden/v1/` and `tests/codecs/tests/events.rs`, under events
   version 1 (README rule 5), and the producer's notice function the
   product's stream uses; `smith-bench`'s event reader follows as far
   as it must to read them (README rule 4). Those files join 03.10's
   Files; its Size is README rule 9's.
49. **06.7's fault names its model by one charter index, main first**
   (block of 2026-10-12T17:11:50Z, smith 06.7, sent as a design gap).
   run.md, section 10 asks for "the endpoint and model of the last
   completion"; how the fault carries it is the plan's. 06.7's sketch
   gives `Fault.model: u32` "by its index in the charter's models", but
   `Charter` holds main's `llm` apart from `models`, the sub-agents'
   choices (`smith-domain-run/src/charter.rs:66-70`), and valid charters
   have `models` empty. Ruling: the implementer's numbering, 0 for
   `llm` and k + 1 for `models[k]`, read and written through one lookup
   on `Charter` (at most 20 lines, README rule 8) so that the offset
   lives in one place; the parent resolves endpoint and name from it.
   `smith-domain/src/peer.rs`'s `Peer` gains the conversation's model
   index, set at `Open`, with its constructor's parameter and the
   callers that pass it; that file joins 06.7's Files.
50. **A live batch stopped by the provider runs once more** (block of
   2026-10-12T17:15:40Z, smith 13.2). `cache-reuse`'s batch
   (`probes-cache-reuse-8d2ec9b8-20261012T170907Z`) stopped after two
   runs: attempt 02's response 13 met HTTP 503 ("upstream connect error
   or disconnect/reset before headers"), smith retried it 139 ms later
   and answered correctly, but the failed response has no usage, so the
   attempt's tokens are unknown and `batch.rs:481-489` stops dispatch to
   keep the suite's token ceiling. The provider's failure, not smith's:
   session 13's rule 1 lists "a refused or failed call" as smith's, and
   smith's call did not fail. README rule 3 now says what rule 1 says for
   the smoke, for every live batch: once more, from the start, as a new
   run, after 60 s; stopped again, not calibrated, and the step goes on.
   This matters most here: `cache-reuse` measures the prompt cache, the
   largest gap the spike measured against Codex (findings/benchmarks.md,
   task 01: smith 0 cached tokens, Codex 97.5k). For the retrospective:
   the harness could count a response refused before its headers as no
   tokens, rather than stopping the batch.
51. **05.s1 reproduces the live trace at the process world; 05.s2 takes
   the line typed while the run starts** (block of 2026-10-12T17:20:51Z,
   smith 05.s1). 05.s1's agent and inline-local candidates pass on the
   parent, so the acknowledgement hold is not shown there; its process
   candidate, two lines typed together, fails on the parent for another
   cause: the second line, received while the run is `Starting`, is
   never dispatched (`dispatch_line` refuses `Starting`; `state_saved`
   calls `maybe_start` without the next line), and the watchdog cancels.
   The live trace is not that: there, line 2 reached the run
   (`message.received`, run 1, name 2) as the wait started, after turn
   1. Ruling: 05.s1 keeps its scope and reproduces the live sequence in
   the process world, where the real store over io acknowledges turns
   (10.1): a scripted model that answers turn 1 with `wait`, and line 2
   injected on that wait; it stops and asks if that passes on the
   parent too. The start gap is a real defect against host.md, 8 ("every
   line received is relayed, carried or shown as not delivered"): a step
   not in the plan, 05.s2, after 05.s1, with 05.s1's process candidate as
   its story.
52. **smith 06.7 splits; 06.13 takes the model and the attempts** (block
   of 2026-10-12T17:34:28Z). 06.7's candidate (`ee9d158d`) holds 719
   production lines before its last mechanical fixes, past README rule
   9's 700: two 21-variant cap vocabularies, four total cap
   translations, typed limits, phases and status, and the event failure
   metadata kept. Ruling: the implementer's split. 06.7 lands the typed
   limit, phase and status with their mirrors, and its Removes; 06.13,
   the next unused number of session 06 (06.11 landed in the first run),
   adds the last completion's model (13.49's numbering, `Peer`'s index,
   the `Charter` lookup) and its attempts over the same carriers, with
   06.7's reference test gaining their assertions. 06.9 needs both.
53. **05.s1: run the live chat on `main` before narrowing further**
   (block of 2026-10-12T17:46:29Z, smith 05.s1). 05.s1's process story,
   the live order (wait, line 2 received, line 1 read, the next
   response), passes on the parent in both placements, with the turn
   acknowledged through the real store. The failed attempt's own files
   say more than its trace: its terminal ends "Agent process failed",
   and its stream's `run.completed` is `cancelled` at 36,401 ms with
   `session.ended` `answered: false`, `exit: 1`: the agent went silent
   after the wait, requesting nothing, until the host ended it, which is
   the watchdog over silence's doing (06.3). Ruling: `follow-up/chat` runs
   live on the current `main` in two calls of 12 attempts (README rule
   3). Stalled again: README rule 11's diagnostic in a worktree commit
   that never lands, naming in the product's and the agent's standard
   error each turn's save and acknowledgement and each completion held
   and released, built as an arm and run once more (12); then the fix
   where it shows. No stall in 24: 05.s1 lands its process story as
   coverage, with no production change, and the counts in its commit
   body.
54. **The harness reads arms from before 06.5's `aborted`** (block of
   2026-10-12T17:46:29Z, smith 00.16). 00.16's control
   (`probes-history-small-window-8d9d5408-20261012T174019Z/01`) ended
   `HarnessError("events record has an invalid known field")`: 06.5
   (8d2ec9b8) added `session.ended`'s `aborted`, and the reader now
   requires it of every arm, so no control built before 06.5 can be read,
   for any probe. README rule 4: "the harness follows the binary ... so
   probes keep running." 00.16 adds, as 13.22 did, a face entry pinned to
   8d2ec9b8 whose `session.ended` carries `aborted`, earlier faces reading
   it as absent, with a test that reads a recording without it; its
   control then runs. Not a lower-layer proof in its place.
55. **smith-04.s1: a small window lowers the material's bounds** (block
   of 2026-10-12T17:46:29Z, smith 00.16). The current arm's small-window
   attempt ended `Refused(invalid host_part)` before any request: the
   window derives `host_part` 8,874, and the charter's host bytes
   reserve the declared `guide`, 16,384 per directory, without an
   `AGENTS.md`. session.md, 8.1: "Where a declared window would not hold
   that material, the derivation lowers the material's bounds, never the
   window." A smith defect the probe found; pinning `guide` at 1,024 in
   the probe, the implementer's proposal, would hide it. A step not in
   the plan, 04.s1, in session 04's file; 00.16's `small-window`
   calibration waits for it and is its live proof, within 00.16's
   original bound ($10, 45 minutes); `history` runs meanwhile.
56. **`SessionEnded.aborted` is `Option<bool>` in the codec, `Some` on the
   wire** (block of 2026-10-12T17:52:19Z, smith 00.16). 13.54 has earlier
   faces read `aborted` as absent; the codec's field is `bool`, so absence
   needs `Option<bool>`, and writing `false` for an old arm would claim
   what it never said. Ruling: the implementer's shape. `smith-events`'
   `SessionEnded.aborted` becomes `Option<bool>`; the current reader
   requires the field and yields `Some`; a historical layout, selected by
   the harness's face entry pinned to 8d2ec9b8, yields `None`; the
   writer refuses `None`, so the wire stays events.md 3.2's `aborted`
   bool. The producers' and tests' constructors wrap `Some` (README
   rule 8). Not a harness rewrite of the bytes.
57. **04.s1 derives each run's guide bound at admission** (block of
   2026-10-12T17:53:11Z, smith 04.s1). The only opening material bound
   that can be lowered is `guide`, reserved in `prompt::host_bytes`
   (`smith-domain-run/src/prompt.rs:172`) through
   `charter::check_window_parts` (:297); a static cap in the profile alone
   still refuses charters that would fit with a smaller guide, against
   04.s1's "a run is refused only where no material bound can fit".
   Ruling: the implementer's proposal: the profile caps `guide` at its
   host share, and admission derives a per-run guide bound from the
   charter's measured bytes left in `host_part`, kept private in `Run`
   and used at `prepare::request` and in the guide's retention.
   `run.rs` (a hotspot, README rule 6) and `prepare.rs` join 04.s1's
   Files, with their unit stories; its Size is README rule 9's.
58. **13.3's `spend` lands not calibrated: its authored budget cannot
   hold the reserve at shell.md's values** (block of
   2026-10-12T17:57:33Z, smith 13.3). `budget/spend`'s $1.00 against the
   standard profile 04.14 shipped (window 272,000, output 32,000, the
   pinned rates) gives a completion of at most $0.66 and a two-turn
   reserve of $1.32, so smith refuses it at the entrance, as run.md, 9.1
   says ("A budget that cannot hold its reserve is refused at the
   entrance"). Not a smith defect, and 13.3's Don't forbids moving the
   ceiling or the budget: `spend` lands not calibrated with that reason,
   and its next task version is `later.md`'s existing row. For the
   retrospective: the harness reports an entrance refusal as
   `HarnessError(no event session was observed)` rather than a setup
   refusal.
59. **smith-06.s1: an accepted answer ended as the agent's failure**
   (block of 2026-10-12T18:12:42Z, smith 00.16). 00.16's live `history`
   attempt 01 answered (`run.completed` `accepted` at 5,311 ms) and its
   agent at once wrote `teardown_aborted` and `session.ended` with
   `exit: 1`, `answered: false`, `aborted: true`; the product showed
   "Agent process failed". agent.md, section 6 has an agent that answered
   exit with success. A smith defect, outside 00.16: a step not in the
   plan, 06.s1, reproduces and fixes it; its live Proof is `history`
   once. 00.16 counts attempt 01 as that defect, handed to 06.s1, and
   its rerun's pass as the base probe's evidence; it does not rerun
   `history` for it. `request_bytes` is `response.completed`'s field
   (events.md, 3.5; `smith-events/src/records.rs:845`), not
   `response.started`'s: 00.16 reads it there, and reports a null where
   it finds one.
60. **One land retry after skein-world's tree supervision failed**
   (block of 2026-10-12T18:21:15Z, smith 04.s1). 04.s1's gate failed in
   `benchmarks/tests/pipeline.rs:83`, a story it did not touch: the child
   panicked at "opening supervised tree member 426970: Other(22)", from
   skein-world's tree supervision at smith's pin
   (`testing/skein-world/src/tree.rs:249`), which skips `ESRCH` (3) from
   `pidfd_open` and fails on any other error; `EINVAL` (22) is what
   `pidfd_open` returns for a PID that names a thread rather than a
   process. The first occurrence in this run's gate logs. Ruling: one
   retry of `land`, as development.md 8.5 allows for a timeout. A second
   occurrence in any session's gate makes it a skein step (skein-world
   treating `EINVAL` as it treats `ESRCH`) and a repin.
