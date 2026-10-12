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

