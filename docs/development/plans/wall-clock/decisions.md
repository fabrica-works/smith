# Wall-clock plan: decisions

2026-10-16. The decisions the wall-clock plan rests on: smith's time
against Codex's, the parity run's follow-up (the parity plan's
`later.md`, its last row). Section 1 is the user's, on the evidence in
`findings/`. Sections 2 to 9 are the designer's, each naming the design
text it landed in; the design documents follow them, and where this file
and the design disagree, the design wins and the disagreement is a
question for the designer. Section 10 says what the plan carries.
Section 11 lists what the user asked to review. Section 12 is the
planner's: how the plan applies sections 1 to 10. The run's rulings
follow, under "Rulings", numbered from 101.

## 1. The user's decisions

The user decided these on 2026-10-16, on `findings/causes.md`:

1. **The price basis:** the benchmark prices each request by its own
   prompt size, as the provider bills, not by the declared window.
   Settled before the next `parity` run.
2. **The window:** smith keeps 272,000 for `gpt-6.1-sol`; the compaction
   trigger's arithmetic is fixed instead (a realistic output
   reservation; tool bytes at a measured bytes-per-token ratio, or
   compaction on the provider's overflow answer). Raising the window
   stays a fallback.
3. **RT's power:** the designer decides between more `shell-cwd`
   attempts and accepting that smith must be about 10% faster than
   Codex to pass at five.
4. **The parity retrospective's proposal 4:** the design says how a
   change to what the model is sent reports its effect on the criteria's
   measures, and whether a move the wrong way stops it.
5. **The compaction threshold** (the parity plan's 12.2): 4/5 stays.
6. **Memory:** the cap may be as large as the goal needs: raise
   `memory` where a change needs room, rather than shaving the worst
   case's accounting. smith allocates only (mostly) what it really
   needs, and frees it when it no longer needs it. A cap stays.
   Up-front capacity is skein's foundation rule, which jig and temper
   share: changing it is a question for skein's designer, not this
   plan's.
7. **Timing:** design and plan now; the run may start before skein's
   `kit-after-parity` lands, under the stand-in rules (the parity
   retrospective, section 10).
8. **Live runs and budget:** no limit. The plan may run as many
   attempts, calibrations and `parity` runs as the goal needs; the user
   watches the credit and says when it runs low. The plan still states
   its runs and the attempts it expects, so that the run is counted,
   with no cap for the run to stop on.

## 2. The window check (1.2)

**Decided:** the session's check estimates the next prompt against the
threshold, with no output reserved below it; a guard in the protocol
layer holds every prompt sent within the line below which a compaction
always fits (domain/session.md, 4, 8.1 and 8.2; protocol/limits.md, 2.1,
3.2, 3.9 and 4; protocol/llm.md, 5 and 6; protocol/charter.md, 6).

1. **No output is reserved below the threshold.** The check is the last
   prompt, plus its output, plus the appended count, against the
   threshold. The completion's most output, which the compaction after
   it carries, is held above the threshold, as `compaction-headroom`
   already counts it.
2. **A window is a prompt's.** A compaction's summary is its output,
   beside its prompt, as every completion's is: protocol/limits.md 2.1
   already defines `window` as the most one prompt may hold, and every
   preset sits within its provider's limits with `output` beside it. A
   provider that counts output within its context is declared at that
   context less `output`.
3. **The appended count is an estimate:** the bytes appended since the
   last prompt, at that prompt's counted bytes over its reported tokens,
   clamped to between one byte and `TOKEN_BYTES` a token. One byte
   measure throughout: the payload bytes of each block, as the session
   holds them, never their cells.
4. **The guard holds the line.** The line is the window less one
   `output`, the summary instruction and a marker for each of
   `calls_per_response` calls; the spare is the room between the
   threshold and the line, about 21,400 tokens for the Codex models and
   7,000 for the Anthropic ones (a 390-byte summary instruction, 16
   markers of 37 bytes). Before an ordinary request with an earlier
   prompt in its window is sent, the protocol layer measures its new
   part, the last answer and everything after it, as it renders it, a
   byte a token, each block's framing at the prompt allowance's share.
   The last prompt's reported size plus that measure past the line: the
   request is not sent, and fails as *context too long*, unsent, which
   compacts the window.
5. **Why it is a bound.** A token stands for at least one byte of the
   text it is read from, so the rendered measure bounds the new part's
   tokens, whatever its content: short numbered lines, escaped bytes,
   framing. The session's own count could not be the bound: it counts
   payload bytes, not what rendering adds, which the old reservation of
   one `output` below the threshold hid.
6. **What moves in the derivation:** a window's history bytes are now
   the line's tokens at `TOKEN_BYTES`, plus one completion's whole
   reserve, its answer included, plus one batch. That is the spare and
   one `output` more, at `TOKEN_BYTES` bytes a token: about 430 KB a
   conversation for the Codex models, with the history's items, the
   resume limit and the transcript's ceilings that derive from it. The
   standard profile's worst case was 28,783 bytes under `memory`, so
   the plan raises `memory` by what the derivation adds, naming the
   part (section 9). The room keeps its two outputs, the second now the
   first completion's output in a new window.
7. **A provider's context too long** stays a trigger, unchanged: the
   backstop for a provider whose window is smaller than declared.

**Effect on the evidence.** Attempt 49's firing check counted 219,281
(152,588 + 224 + 34,469 bytes + 32,000). With this check it counts
152,812 and 34,469 bytes at the window's ratio, about 10,000 at three
and a half bytes a token: about 163,000, and fits. The guard then
measures about 40,000 rendered bytes of new part: 192,600, within the
line of about 239,000. The check fires when the estimated next prompt
nears 217,600; uncompacted, smith's peaks were 177,000 to 220,000.

**Not taken:**
- **A realistic reservation**, the largest output seen so far: no
  bound, since a model that writes a large file in one completion
  passes it, and the compaction after it may not fit.
- **A floor in the session's count** (the appended bytes less the
  spare): first drafted, and dropped in review, since the session's
  bytes are not what the request renders.
- **Compaction on the provider's answer alone:** a prompt admitted near
  the window leaves no room to compact it, and a second *context too
  long* fails the session.
- **A tokenizer or the provider's count endpoint:** a dependency per
  provider, or a request per check, for what the guard measures
  locally.
- **Raising the window** (1.2's fallback): memory and long-context
  prices, `findings/causes.md`.

## 3. RT's attempts (1.3)

**Decided:** ten attempts per arm on `shell-cwd` (benchmarks.md, 11.5,
16.1 and 16.2). From `findings/rt-power.md`:

- at ten, a smith 10% faster than Codex passes RT in 0.96 of runs (0.60
  at five), and one as fast in 0.60 (0.31);
- a smith at the margin passes in 0.01 of runs at ten, as at five: the
  margin's promise holds;
- RC gains the same power from the same attempts.

**Not taken:** five, accepting that smith must be about 10% faster (the
user's other option): the plan's aim would turn on the test's power,
not on smith's time. Fifteen: an on-par smith passes 0.72, at about 1.7
more hours a run in the gate lane. A geometric-mean statistic would
gain power at five, but is a new criterion, not a number of attempts.

## 4. A change on the way to a verdict (1.4)

**Decided:** benchmarks.md 16.6, with the `parity-change` suite (11.5)
and 4.2's pointer to it.

- **What counts:** a change to a prompt smith sends, or to when it sends
  one: the role, smith's texts, a tool's description, schema or
  defaults, the window's check.
- **Its proof:** one `parity-change` run, the `parity` suite's tasks at
  five attempts per arm, the change against its parent.
- **Its report:** each criterion's ratio of the change to its parent,
  the parent arm chosen by its commit, and its projection: the
  baseline's statistic times the product of the ratios of every change
  landed since the baseline, this one included, beside the baseline's
  interval. The baseline is the plan's first `parity` run.
- **The stop:** a projection past the baseline's upper bound (FT, FC,
  RT, RC), passes below O's bound against the parent, or a report that
  is incomplete, stops the step on a question to the designer, whatever
  its comparison concludes. The product makes small moves that each
  pass their own test add up where the plan sees them.

In the parity run, 06.2's comparison took responses an attempt from
7.13 to 8.33 and landed on "no difference"; FT then failed the first
closing run. A step's comparison at five attempts cannot show that a
change does no harm; the baseline's interval is the tolerance the plan
already accepts as noise.

**The parity retrospective's row 4** now names this section and
benchmarks.md 16.6.

## 5. The price basis (1.1)

**Decided:** benchmarks.md 8.1 and 16.1; shell.md 5.2. Each response's
tokens are priced at the list rates of the tier its own prompt reaches,
from the presets' price list; the declared window plays no part. The
adapters read usage per response: smith's `response.completed`, Codex's
per model response. smith's own spend keeps its flat rates (domain/run.md,
section 9). Without a response's usage, the common priced spend is
unavailable, or a lower bound for a retried failure (8.2).

At base rates for both, the last closing run's FC is 0.94 and its RC
1.47 (`findings/causes.md`): both still hold, RC with little room. The
plan's baseline is the first `parity` run on this basis.

## 6. Batching (causes 1 and 3)

**Decided:** the role's words, and a failed write stopping the writes
after it (shell.md, 6.6; domain/session.md, section 5; protocol/llm.md,
sections 3 and 5; protocol/transcript.md, sections 2 and 6).

- **The role's topics 3 and 4:** a change's edits, across its files,
  then the command that checks them, in one response; every replacement
  one file needs in one `edit`. The role's batching topic named only
  calls that change nothing, and smith's edits came one file, one
  replacement, a response; the descriptions already invite batched
  writes.
- **A failed write stops the writes and commands after it** in its
  completion, answered as not run, naming it; reads still run. This
  settles domain/session.md's open question: once the role asks for a
  change and its check in one response, a check run on a half-made
  change costs a build. Its why is new in a not-run outcome:
  transcript version 5, which jig's and temper's next repins of smith
  take.

**Not taken:** `edit` over several files, a patch: a new call shape in
the domain, the codecs and the transcript, which the `edit-format`
experiment exists to choose. If the role's words do not batch edits,
that experiment is the next step (the parity plan's `later.md`).

## 7. Job polls (cause 4)

**Decided:** a job's wait gives way to its batch (domain/session.md,
section 5; domain/tools.md, sections 3 and 5; protocol/llm.md, section
3), and the role's topic 5 sends polls beside other work.

- A `job` call beside other calls answers once every call of its batch
  that is not a `job` call has answered, `running` if its job runs on.
  Alone, it waits as it asks, up to `job_wait`.
- So a poll beside reads costs the reads' time, as Codex's poll inside
  its next read does, and a lone poll still waits for the end rather
  than spinning.

**Not taken:** a shorter `job_wait` default: a lone poll would return
`running` sooner and cost a request each time. **Deferred:** a job's end
told between completions waits for messages mid-work (domain/run.md,
section 6), as domain/session.md section 11 keeps it.

`CARGO_BUILD_JOBS=2` in four of five smith attempts is the model's own
setting; no design text answers it, since words naming a task's
commands answer no cause (benchmarks.md, 16.5). The plan's diagnosis may
find where the model took it from.

## 8. Ranged reads (fresh tokens)

**Decided:** the role's topic 2 (shell.md, 6.6): read the lines the
work needs, a window from where a search found them, a whole file only
when the work needs all of it. smith's reads were whole files or 10-19
KB windows, against Codex's line ranges, and ranged reads would cut
60,000 to 90,000 fresh tokens an attempt and keep prompts further from
the threshold.

**Not taken:** a line count on `read`: a new call field, and transcript
change, for what `offset` and `window` already express. A smaller
default `read_window`: more paging, so more requests.

## 9. Memory (1.6)

**Decided:** protocol/limits.md sections 5, 11 and 13; shell.md 6.3;
testing.md section 6; benchmarks.md 8.1, 5.4 and 16.1.

- **A cap, not an allocation:** bytes allocated on arrival at their
  length and freed with their owner when its use ends; up-front
  capacity the exception, sized by the limits.
- **Room is made by raising the cap,** in the commit that needs it,
  naming the part and its bytes; never by shaving another part's
  accounting. `memory`'s status in shell.md 6.3 follows the user.
- **Shown:** the agent's simulated world's counting allocator holds
  the live heap falling once a compacted window's history is released,
  and no larger at the same point of each repetition of a story that
  compacts, reads jobs to their end, and opens and closes
  conversations; the benchmarks' summaries carry smith's own peak
  resident memory beside time and cost.
- **Up-front capacity** is an open question in limits.md 13, skein's to
  decide once smith has measured it.

This design's own change needs room: the history's bytes grow by the
spare and one `output` at `TOKEN_BYTES` (2.6). The step that lands the
guard raises `memory` by what the derivation adds, and its commit names
the part and the bytes.

## 10. What the plan carries

- **Its aim:** smith on par with Codex at the plan's final `main`,
  every criterion of benchmarks.md 16.2, RT included. A measured aim
  (skein's development.md, 5.1), with the baseline, the closing runs
  and their bound stated in its live budget (benchmarks.md 16.5).
- **The order the design implies:** the price basis, ten attempts on
  `shell-cwd`, `parity-change` with its report, and peak memory in the
  summary land before the baseline `parity` run, since they change what
  it measures; each change to what the model is sent lands after it,
  with its `parity-change` run (16.6).
- **The causes it answers,** each change's commit body naming its cause
  from `findings/causes.md` (benchmarks.md 16.5): 1 and 3 (section 6),
  2 (section 2), 4 (section 7), fresh tokens (section 8).
- **Timing and budget:** 1.7 and 1.8.

## 11. For the user

The user asked to review the designer's decisions before the plan
moves on. Each is reversible by a later design change; none is a
product decision. The ones with the widest reach:

1. **The window's meaning** (2.2): a request's output is never counted
   in `window`. A person declaring a provider's full context, where the
   provider counts output within it, must now declare it less `output`.
2. **Ten attempts on `shell-cwd`** (section 3): about 1.7 hours more per
   `parity` run.
3. **The stop on a wrong-way move** (section 4): each change to what
   the model is sent costs a `parity-change` run, 40 attempts, about 2.5
   hours, and may stop on a question.
4. **Transcript version 5** (section 6): jig's and temper's next repins
   of smith take it.
5. **`memory` raised** (2.6 and 9): the first change under the user's
   decision 1.6, by the history the guard's line allows, about 430 KB a
   conversation before its cells.

## 12. The planner's

How the plan applies sections 1 to 10, with the user's answers of
2026-10-16 and the questions for the designer.

### 12.1 The order

- **Before the baseline,** everything that changes what the baseline
  measures, or what the smith arms of 07.1 to 10.1 build on: the price
  basis, the rows and the ten attempts (smith 01); the line, the spare
  and the history's bytes (smith 03.1, 03.2); the guard (smith 04.1);
  and, as section 10 lists, the `parity-change` suite with its report
  and stop (smith 02), so that 06.1 runs its pipeline whole at one
  attempt per task against the new baseline, before any full run
  (development.md 5.5).
- **The guard lands before the baseline,** not after it with the check.
  Under today's check it is never reached on the suites' tasks: a
  prompt the old check admits has at least 53,418 tokens below the line
  (the line's 21,418 above the threshold and the 32,000 reserved), plus
  that completion's output and the appended bytes, so the guard refuses
  only an answer and appended part rendering past that; no main request
  in the parity run produced more than 2,400 output tokens. Landing it
  first lets the baseline measure it, keeps 07.1 to the check alone,
  and runs the guard on 68 live attempts before the check depends on
  it. 06.1 counts its refusals.
- **The four changes, in the order of their causes' cost:** the check
  (07.1, cause 2: 74 s and the re-reads after), the batched change
  (08.1, causes 1 and 3: 152 s), polls (09.1, cause 4: 26 s), ranged
  reads (10.1, fresh tokens). The check comes first although batching
  costs more, because the check's own report is the cleanest: it
  changes no words, and every later change runs in windows that no
  longer compact early.

### 12.2 Formats and committed files

- The result and the summary move to version 4 in 01.1; the 15
  committed summaries of version 3 and the two baselines cut from them
  go, as the parity plan's 10.6 did at version 3: a version 4 reader
  refuses them, and their spend is on the old basis. Git keeps them.
  11's last `parity` run cuts the new baselines.
- The event vocabulary moves to version 3 in 03.1, for the one new
  effective limit (question 4); the transcript to 5 in 08.1, carrying
  every not-run why, so that a resumed outside-the-choice call renders
  as it was told (`findings/planning-audit.md` A15).

### 12.3 The user's answers, 2026-10-16

- **Two closing runs** (development.md 5.1; benchmarks.md 16.5): 11.1,
  and 11.2 only after a change that answers 11.1's miss. A miss at 11.2
  ends the plan with its aim unmet.
- **Four change steps, one per cause,** each with its `parity-change`
  run, one after another: about 12 hours of the gate lane.
- **No repin** of jig or temper: temper's client-api run is changing
  both on carbon-forge; their next repins take this plan's formats
  (`later.md`).
- And the planner's, within benchmarks.md 8.1's "each with its scope":
  Codex's peak memory in a row is the harness's count over its process
  tree, scope `tree`; smith's its own, scope `process`.

### 12.4 Questions for the designer

The design trace's questions (development.md 5.4): each is an
observable rule the plan needed and the design did not settle, asked
with the planner's default. **Answered** on 2026-10-16 by the designer,
a sub-agent of the planner's session at the user's request, in smith
`f1b33a17` (domain/session.md 5 and 8.1; protocol/llm.md 3, 5 and 6;
protocol/limits.md 3.2, 3.9, 4 and 5; protocol/charter.md 6 and 8;
protocol/events.md 6; protocol/transcript.md 2 and 6; benchmarks.md
3.2, 8.1, 8.2, 11.1 and 14): questions 2 to 6 as their defaults; 1 with
`unchanged` failing too, and any call answered `invalid`, a read's
included; 7 widened to every response failed unsent, priced at nothing
whether retried or compacted after, a retried failure being one whose
evidence is `response`; 8 without the floor, since limits.md 5 keeps no
margin above the worst case. The session files follow the answers.

1. **Which of the code's verdicts are a failed write** (session.md 5)?
   The design names `invalid`, `conflict`, `missing`, `too_large` and
   `failed`; the code's verdicts are finer, and none is `conflict` or
   `missing` (A16). And a `failed` write may have written part of its
   change, against "it changed nothing". Default: a `write` or an `edit`
   fails on every verdict but `written`, `edited` and `unchanged`, and
   any call answered `invalid` fails.
2. **How the not-run text names the failed call** (llm.md 5;
   transcript.md 2). Default: by its position among the response's
   calls, from 1, and its tool: `not run: call <n> (<tool>) failed
   before it; this call's changes were not made`, the transcript keeping
   the position.
3. **A block's framing in the guard's measure** (llm.md 6; limits.md
   3.2, 3.9): "at the prompt allowance's share for a block", but the
   allowance is one number for a whole prompt, and no share for a block
   is defined (A7). Default: the measure is the request's encoded body
   less the encoded body of the same request cut where the new part
   starts, both through skein's `client::measure`, so a block's framing
   counts as encoded, a byte a token.
4. **The line among the effective limits** (limits.md 9; benchmarks.md
   11.1): the `history` probe reads the line "from the effective limits
   `session.started` carries", which hold neither the summary
   instruction's size nor the marker's (A13). Default: the run's
   `Limits` gain `compaction_material`, the summary instruction's bytes
   plus the markers', named `run.compaction_material`; the run gives
   each conversation its line at open; the event vocabulary moves to
   version 3 (events.md 6).
5. **The refusal of a negative spare** (charter.md 6 and 8): which
   refusal names it. Default: the existing `Invalid::Llm`, so that the
   channel keeps version 2.
6. **benchmarks.md 11.1's `history` condition** at the line: it changed
   in the design's landing, but `STATUS.md` names no Left for 11.1.
   Default: this plan's (03.3), since 07.1 would otherwise fail the
   probe's `conversation-opened` on prompts between the threshold and
   the line.
7. **The price of a request the guard refused** (benchmarks.md 8.1,
   8.2, 16.3): it reached no provider, has no usage, and is never
   retried as such (the compaction after it is a new completion), so
   by 8.1's words the attempt's priced spend would be unavailable and
   the verdict incomplete, though 07.1's stories expect refusals.
   Default: a response that failed unsent is priced at zero.
8. **`memory` for 03.2's derivation** (limits.md 5; shell.md 6.3):
   sections 2.6 and 9 have the step raise `memory` by what the
   derivation adds, written when 28,783 bytes were left; at the base
   the standard profile's larger worst case leaves 120,442,167 bytes
   (`findings/planning-commands.md`), and limits.md 5 raises `memory`
   when "a change whose worst case passes" it. Default: `memory` stays
   2 GiB while the new worst case fits it, with a floor of 16 MiB that a
   test holds (development.md 5.6), and 03.2's commit names each part
   that grew and its bytes.

### 12.5 For the user

Reversible, and none a product decision; the widest in reach:

1. The guard before the baseline (12.1).
2. The 15 summaries and two baselines of version 3 dropped (12.2).
3. The event vocabulary's version 3 (question 4), which jig's and
   temper's next repins take with transcript version 5.

## Rulings

101. **smith 04's Shapes against main** (ready check of smith-04.1).
     The ready check of smith 04 (04.1 and 04.2, against smith `main` at `ba1fef5b`, skein `7e8fb700`; the corrected sketches compiled under clippy, and the default and fuzzy profiles passed with the guard on and its line at `u64::MAX`) corrected `smith/04-guard.md`:

     - 04.1: `start` is `component.rs:339-443`; the guard is one call, `guard::check`, between the output ceiling (`:403-406`) and `observed_prompt` (`:408`), returning `Option<Event>`. It builds the unsent *context too long* itself, since `refusal` maps only the crate's `Error` to fixed text; a measure's error goes through `refusal`. `guard::measure` takes `&mut shared::Prompt`, the credential and the `EndpointOptions`, as `shared::client::measure` needs. `complete` builds both `Ordinary` arms directly. Files add `lib.rs`'s `mod guard;`, `production_render.rs` and `root_memory.rs:1081`; the users-outside-Files rule now covers match patterns and names the six users found.
     - 04.2: Files add `tests/session/Cargo.toml`, whose `skein-llm` moves to `[dependencies]` for `TOKEN_BYTES`.

102. **the session's line lands as a named field, with its reader** (steer of the check of 2026-10-16T12:59:54Z).
     The session's line lands as a named field, with its reader. In 03.1 the line ends at the session's `Spec.line`, filled by the run at open; `Conversation.model_window` stays `u32`, with its reads as on main. 04.1, whose `complete` first reads the line, adds `Conversation.line: u64` beside `model_window` (`session.rs:184`), set at open where `model_window` is (`:2082`); if 03.1 has landed `model_window` as a tuple, 04.1 makes it `u32` again. A value is never put inside a tuple, or any other type, that a lint would not see unread. `smith/03-line.md` (03.1's Shape) and `smith/04-guard.md` (04.1's Shape) say so.

103. **no supplement step for 05's landed test sizes; a later.md row** (process, smith 05's block of 2026-10-16T13:13:46Z).
     Ruling 103. No supplement step for smith 05: `tests/agent/tests/memory_returns.rs` stays as landed at `728feb25`, and its sizes are named when an increment next changes the file. The steer of 2026-10-16T13:12:23Z, point 2, says so itself: "Unchanged: … landed tests in files an increment does not otherwise change". Its "smith 05 from now" reached no increment of smith 05, both having landed (05.2 at 13:13:46Z). No session file of the plan but `smith/05-memory.md` names the file, so no later step copies its form. `later.md` gets the row; smith 05 is done. Point 2 stays in force for each session's next test.

104. **smith 04's Shapes after 03.1** (ready check of smith-04.1, re-checked).
     Re-check of smith 04 after 03.1 landed (`0616a6c5`; the check read `ba1fef5b`). 03.1 moved only positions in 04's files and landed `model_window: (u32, u64)`. `smith/04-guard.md` now gives the moved lines (`world.rs:1425`, `root_memory.rs:1082`, `session.rs:185`, `:2083`, `complete` at `:2197-2255`, its purpose at `:2222`), and names the tuple's six `.0` reads that 04.1 returns to `u32`, as ruling 102 says. The protocol world's window is now `MODEL_WINDOW`, 20,480 (`tests/protocol-llm/src/limits.rs:8`), and the session world opens with `line: 64_512` (`tests/session/src/world.rs:288`, `recorded.rs:41`); 04.1's tests take the line from their worlds, as its Test shape says.

105. **04.1 owns the model_window tuple's correction; 03.2 goes on** (process, smith 03's block of 2026-10-16T13:19:13Z).
     Ruling 105. 04.1 owns the tuple's correction, as the session's report proposes and as rulings 102 and 104 already put it in `smith/04-guard.md` (04.1's Shape): "03.1 landed `model_window: (u32, u64)` (`0616a6c5`): 04.1 makes it `u32` again, its five `.0` reads in `compaction.rs` (`:58`, `:78`, `:128`, `:230`, `:388`) and the one at `session.rs:2210` as on `ba1fef5b`." 03.2 touches neither `model_window` nor `compaction.rs`, and goes on with the history derivation as its Shape says; its Files name neither file. The steer of 2026-10-16T13:12:23Z, point 3, asked smith 03 only about 03.1's work before it landed; for 03.2 it asks nothing more than its last line: "A value is never put inside a tuple, or any other type, to keep a lint from seeing it unread."

106. **smith 06's Shapes against main** (ready check of smith-06.1).
     The ready check of smith 06 (06.1 against smith `main` at `ed5e9103`; no sketch; smith-bench's suites passed under both profiles on a stand-in summary, with no provider called) corrected `smith/06-baseline.md` and `smith/02-change.md`:

     - 06.1, step 4 and "Choices": the small `parity-change` run builds its change arm from the summary's commit (`build HEAD` after committing it, `<dir2>`) and its parent from `<dir>`, the baseline's commit: one code, two commits. benchmarks.md 16.6 says "The reference arm is the one built from the change's parent, chosen by its commit", and the landed 02.1 gives each built arm its own commit as `agent_version` (`probe.rs:245`), so one `<dir>` for both arms would leave 02.2 nothing to tell apart.
     - 06.1, Proof: the guard's count is `rg -l -g trace.jsonl 'past the line: last prompt' …/runs/<its run>/ | wc -l`, smith's events being in each attempt's `trace.jsonl`.
     - 02.3: a `parity-change` run below its suite's attempts exits 2 whatever its stop, as 01.3 makes every `suite` do, with the test `a_change_run_below_its_attempts_exits_two_whatever_its_stop`; 06.1's step 4 expects that 2.

107. **03.2 raises memory to hold each startup's worst case** (plan defect, smith 03's block of 2026-10-16T13:28:07Z).
     Ruling 107. 03.2 raises the shipped `memory`, as proposed. limits.md 5: "One whose worst case passes `memory` raises the profile's `memory` to hold it; one whose worst case still fits leaves `memory` as it is … A change never shaves another part's accounting to fit. The startup check of the worst case stays." The startup check's `service.worst_case` is the design's worst case; 03's "Choices" read only the profile's two printed totals, which miss the `exec` faces on `gpt-6-luna` and the pinned product check: a plan defect, now amended.

     - `memory` (`quantities.rs:179`) becomes the smallest multiple of 64 MiB at or above the largest `service.worst_case` that any startup the suites run prints on your `HEAD`: the eight refused startups, the pinned product check, and the two totals. With the largest you report, 2,193,357,213, that is 2,214,592,512 (33 × 64 MiB); a larger one found takes the next multiple.
     - Refresh each copy of 2,147,483,648 that stands for the shipped cap: `git grep -l -E '2_147_483_648|2147483648' origin/main` lists 22 files at `86277671` (crates, tests, goldens, `benchmarks/tests/recorded/`); a copy that is a fixture's own setting stays. Refresh any further copy, and list it in the commit body. jig's and temper's copies wait for their repin (`later.md`).
     - No new test: the startup check is the guard. In the commit body, beside the Proof's two totals: each refused startup's `service.worst_case` on `HEAD` before the raise, and what the largest leaves under the new `memory`.
     - The two derived-value failures (`sink_memory` 19,427,968 → 19,855,232; `faces.toml` items 49,483 → 52,821) are 03.2's own, refreshed under README rules 3 and 16 as you propose.

108. **04.1 raises memory for the guard; the second of 03.2 and 04.1 rebases onto the first's raise** (process, smith 04's block of 2026-10-16T13:33:41Z).
     Ruling 108. `main`'s smoke passed (`probes-smoke-tools-f779396b-20261016T133307Z`), so the refusal is 04.1's own: the guard's ownership puts the smoke's `service.worst_case`, 2,147,533,175, past `declared.memory`, 2,147,462,709. 04.1 raises `memory` in its own commit, by ruling 107's rule: limits.md 5, "One whose worst case passes `memory` raises the profile's `memory` to hold it"; decisions.md 9, "Room is made by raising the cap, in the commit that needs it, naming the part and its bytes; never by shaving another part's accounting."

     - `memory` becomes the smallest multiple of 64 MiB at or above the largest `service.worst_case` that a startup prints on 04.1's `HEAD`: 2,214,592,512 (33 × 64 MiB) for the smoke's. The commit body names each part the guard grew, with its bytes, and what the largest worst case leaves under the new `memory`.
     - 03.2 raises `memory` by the same rule (ruling 107). Whichever of 03.2 and 04.1 lands second rebases onto the other's raise: it keeps that `memory` when the largest worst case on its rebased `HEAD` fits it, and raises it to the next multiple of 64 MiB when it does not; it refreshes each copy of the cap that the first left, as ruling 107 says, and lists them in its commit body.
     - 04.1 then runs README rule 8's smoke once more, as written.

109. **AttemptRow carries its commit and time record, required, at version 4** (plan defect, smith 02's block of 2026-10-16T14:23:43Z).
     Ruling 109. 02.2's Shape asks for "`ArmKey` names an arm by agent and commit" and "per task and arm, the medians … of the time record's requests, compaction and tools seconds" from `criteria(rows: &[AttemptRow], …)`, and `AttemptRow` carries neither: a plan defect. Your first option, without its defaults:

     - `AttemptRow` (`summary.rs:114-129`) gains `agent_version: String` and `time_record: TimeRecord`, copied from each result in `attempt_row` (`:131`). Both are required: no `#[serde(default)]`, no `Option`, no unavailable default for old rows.
     - The summary version stays 4. No version-4 summary is committed but test fixtures: `benchmarks/summaries/` holds only `.gitkeep`, and 06.1's baseline is the first. The recorded `on-par.json` (run `parity-recorded`, every `agent_version` zeros, synthetic) gives each of its 68 rows both fields: `agent_version` as its groups give it, and a time record of literal values; `change.json` is built from it as the Test shape says. `the_parity_section_is_unchanged_by_the_arm_arguments` keeps its literals. Refresh any further recorded summary the same way, and list it in the commit body.

110. **06.1 needs 02.4** (ready check of smith-06.1, re-checked).
     06.1 needs smith-02.4. 02.3 landed (`78e84cb8`) split under README rule 1: "Named inputs, SHA-256 checking and command integration remain for smith-02.4". On `main` at `78e84cb8`, `suite` takes no `--baseline-run` (`probe.rs:550`: `"--attempts", "--seed", "--commit", "--arm", "--include", "--parent", "--parent-arm"`), and `main.rs` has no `change` command, both of which 06.1's step 4 runs. `smith/06-baseline.md` now names 02.4 among 06.1's Needs; 06.1's ready check is re-run against `main` once 02.4 lands.

111. **smith 06's Shapes after 02.4** (ready check of smith-06.1, re-checked).
     The re-check of smith 06 after 02.4 (smith `34e0c51b`; `build HEAD` run, smith-bench's suites 301/301 and fuzzy 3/3, every 06.1 command parsed offline with no provider called) found 06.1's commands as its Shape gives them, and corrected three lines:

     - 06.1, step 2: `suite parity` prints `summary <run> attempts=68 verdict=…`, not a path (`probe.rs:677-689`); its summary is `benchmarks/summaries/<run>.json`.
     - 06.1, Proof: a group's `passes` is a count object and its `attempts` a list of ids (`summary.rs:67-69`); the line is `jq -c '.groups[] | {task, arm, passed: .passes.passed, attempts: (.attempts | length)}' <its summary>`.
     - README, rule 13: attempt ids are already two digits (`probe.rs:259`), and `printf %02d "08"` fails in bash as octal; the counts read `…/runs/<run>/$a/result.json`.

     The small `parity-change` run exits 2 whatever its stop (`probe.rs:661-666`, `:733-735`), and `change` exits 0 on a stored section it agrees with, as step 4 expects.

112. **smith 07's Shapes against main** (ready check of smith-07.1).
     The ready check of smith 07 (07.1 against smith `main` at `22b64718`; 07.1's check sketched in a scratch worktree, the full gate then passing, default 2327/2327 and fuzzy 43/43, with no provider called; 06.1, its need, not landed) corrected `smith/07-window-check.md`:

     - The session crate does not depend on skein-llm, so the clamp's `TOKEN_BYTES` is a `token_bytes` field of the session `Limits` (`limits.rs:26`, `:221`, `:269`), set by the service from `shared::TOKEN_BYTES` as `line_bytes` is (`profile.rs:448`, `:550`); Files add its three other `Limits` literals.
     - `prompt_bytes` drops `Descriptor` (`spec_cost`, `session.rs:2547-2548`) and `Token` (`choice_bytes`, `:753-761`) cells too. The referee walks payload bytes itself for the ratio; `Prompt::input_bytes` stays for `route.rs:701`. The profile's startup room (`room.rs`, two outputs below the threshold) stays as it is.
     - Moved lines: the room at `session.rs:2202-2222`, `receive_completion`'s `completed` at `:2519-2523`, the resume in `compaction.rs` (`:426`, `:457`, `:467-471`), the unit tests at `:2912`.
     - Fixtures 07.1 changes: the three window unit tests; `a_compaction_carries_no_guard`'s room, 64,466 → 65,490; the session world's `usage()` (`compaction.rs:9-11`), input 1,300 so that it still compacts; the agent world's `data.txt` (`tests/agent/tests/compaction.rs:47`) and `fill_window` (`memory_returns.rs:69`, `:77`), grown until they compact as before.
     - The room test uses the unit harness's 26-byte instruction (`tests.rs:307`): 272,000 − 232,000 − 26 = 39,974, not 39,610.

113. **06.1's baseline runs once more after a request the provider never answered** (infrastructure, smith-06.1's block of 2026-10-16T18:36:03Z).
     Ruling 113. 06.1's baseline runs once more, from the start, as a new run: README rule 9's once-more, for a run the provider stopped. The block is infrastructure, not design: the design says in its own words what the stopped run is and that it may run again.

     - What the timeout is: benchmarks.md 8.2, "A failure whose `evidence` is `maybe_sent` is never one. The provider never answered it: a deadline of smith's cut it, or the connection dropped, and the provider may have carried the request to its end. What it cost is open." So attempt 53's total misses more than a retried failure, and 5.3 applies: "Any other unknown in an attempt's total stops the suite as incomplete, since what it would charge is not known."
     - What the run counts for: benchmarks.md 16.5, "A run whose verdict is 2 measured nothing whole: its summary is not committed, and its attempts are spent", and "A smith arm is run once to a verdict of 0 or 1." The stopped run reached neither, so a new run repeats no verdict and no stop (rule 9's last sentence holds).
     - Rule 9 now reads "a request the provider never answered (a failure whose `evidence` is `maybe_sent`, benchmarks.md 8.2, such as a timeout waiting for the response head)" beside "the provider's capacity or a server error". It applies to every live command left in the run, 07.1 to 11.2 included. Stopped again, the step stops on a question, which the supervisor sends to the designer.
     - The incomplete report's lines (RT 1.34, holds=false) are not the baseline's verdict, and nothing is read from them. The baseline's failed criteria go to the designer when the new run's verdict is 0 or 1 (06's "Choices").
     - Unchanged: 06.1's Shape, steps 1 to 5, on a worktree at `origin/main`; the small `parity-change` run; no attempt rerun; the `Live:` trailer counts every attempt the step ran, the 60 already run included (rule 10); the worktree's target stays until each heavy task started from it has ended.

114. **06.1 goes on while the designer answers the baseline's misses** (design gap (interim), smith 06.1's block of 2026-10-16T23:20:46Z).
     Interim ruling 114, on a design gap whose question is already with the designer. 06's "Choices this plan makes" says: "The supervisor still sends the baseline's failed criteria to the designer at once (development.md 5.1, 10.4); 07 starts on 06.1's landing, unless the designer's answer says otherwise." The supervisor sent them at 2026-10-16T23:13:54Z, in the run's `question.md` (FT, RT and RC: whether 07 to 10 aim at each), to a fresh designer.

     - 06.1 goes on now, as its Shape says, steps 4 and 5: the small `parity-change` run (task 3789) must exit 2 and `change` must print FT, FC, RT, RC and O and exit 0; then it lands the baseline's summary (dad9509b) with `Baseline-Summary:` and `Live:` (README rules 10 and 13). Neither step rests on the designer's answer.
     - The live record past 1.5 times the step's attempts (128 attempts, $26.07) is said in the report and the commit body, as README rule 10 says: "nothing stops on it".
     - 07 starts on 06.1's landing, as the plan says. The designer's answer, when it comes, arrives as its own unblock; until then, no session acts on FT or RC beyond its own session file.
     - Unchanged: rule 9 (no attempt rerun, no run of the same commits repeated to change a verdict), the stop (rule 12), 06.1's Don't.

115. **The designer's answer to the baseline's misses of FT, RT and RC** (design gap, smith 06.1's block of 2026-10-16T23:20:46Z).
     Ruling 115: the designer's answer to the baseline's misses, landed as smith 65a54642 (sessions 07 to 10 and `later.md`); ruling 114 stands. Its decision:


     07 to 10 aim at all three; no step is added, and the plan ends short on nothing in advance. Each change's commit body names the criteria it addresses and their cause read from the baseline's attempts, as benchmarks.md 16.5 asks of a change that answers a failed verdict: "the criteria it addresses; their cause, read from the failed run's attempts ... against Codex's in that run". Sessions 07 to 10 now carry those figures.

     - **FT: 08.1 (causes 1 and 3).** In smith's 24 fixture attempts, none of the 71 responses that held an edit held a second edit or a command; smith made 194 responses to Codex's 148, and Codex ran its check in the call that made a patch in 23 of its 24. Folding an edit after an edit, or a command after an edit, removes 70 responses, about 111 s of 1,919: by that estimate FT about 1.11, its upper bound about 1.18. 07.1 and 09.1 change nothing the fixtures exercise (no fixture attempt compacted; no smith fixture attempt made a `job` call).
     - **RC: 08.1 and 10.1.** smith's mean `shell-cwd` spend is $1.552 to Codex's $1.131: fresh $0.614 to $0.367 (59% of the excess), cached $0.680 to $0.562, output $0.258 to $0.202. 10.1 addresses the fresh tokens ($0.12 to $0.18, `findings/causes.md`), 08.1 the cached (3.8 million, $0.38).
     - **07.1 may move RC the wrong way, and goes on as written.** Cause 2's tokens in `causes.md` are +$0.09 to +$0.22 an attempt at the presets' base rates, not "about neutral". Nothing in 07.1 changes for it; the stop decides (README, rule 12).
     - **RT: as planned.** Causes 1 to 4 hold in the baseline: 70.1 responses an attempt to Codex's 45.5, a compaction in each of smith's 10 attempts (44 to 70 s), tools a median 73.1 s to 45.7 s.
     - **The stop stays as benchmarks.md 16.6 says:** "A plan's position within the baseline's interval is within what it already measures as noise, and goes on." No bound is added for a missed criterion.
     - **Deferred, with its `later.md` row:** FT's output in writing responses (74,522 tokens to Codex's 67,569), which batching does not shorten: `smith-11.s1`'s diagnosis starts from it if FT is short at 11.1.
     - **Unchanged:** 07 starts on 06.1's landing (06's "Choices"); 11's closing runs end the plan short on whatever the last misses (benchmarks.md 16.5: "When the last it holds is short of parity, the plan ends with its target unmet").

116. **smith 08's Shapes against main** (ready check of smith-08.1).
     Ruling 116. The ready check of smith 08 (08.1 against smith `main` at `5e336b56`; a sketch compiled under clippy in a scratch worktree; default 2324/2325, the one failure the dirty scratch tree's `probe_command`; fuzzy 41/43 before the two fixes below, the session suites and fuzzy not run after them; no provider called; 07.1, its need, not landed) corrected `smith/08-batched-change.md`, Files and Shape:

     - `history::reserve_batch` (`history.rs:139-205`, called at `session.rs:1504`) takes the `failed` state, skips what `advance` answers not run, and counts an invalid call as setting it; without it fuzzy panics at `session.rs:2610`. `returned_cost` (`:2636-2639`) answers `Stopped | AfterFailedWrite { .. } => Some(0)`. `advance` stays within `too_many_lines`, a new `#[expect]` named in the commit body. The session world's referee (`world.rs:1352-1357`) accepts `AfterFailedWrite { call }` with `call` at or before its position.
     - `<tool>` in the not-run text is the call at `<n>` in the assistant message before the result's: transcript.md 2, "naming that call by its position among the completion's calls, from 1, whose tool is that call's in the same message". A plan defect: the design says where the name comes from. `prompt.rs`'s message loop (`:112-125`) passes those names to `translate_block` and `result_text`; `NotRun` keeps only the position.
     - skein's codegen refuses a variant that holds anything but a record (`skein-codegen/src/check.rs:181`), so version 5 wraps the why: `record NotRun { why: NotRunWhy }`, `record OutsideChoice { only: option AllowedCalls }`, `record AfterFailedWrite { call: u16 }`. An absent `only` is `choice: None`, rendered "none" (`prompt.rs:184-205`), so a resumed history renders as it rendered when told (transcript.md 2). A representation, not a design gap: transcript.md 2 names what is kept, "outside the tool choice, naming the calls that were allowed".
     - Files add each user of `smith_transcript::v4` (20 lines in 5 crates, `git grep -n 'smith_transcript::v4\|transcript::v4' -- crates ':!crates/smith-transcript' | wc -l` at `5e336b56`; `tests/codecs`' `memory.rs`, `fuzzy_decode.rs:57`, `transcript.rs:25-28,48,95`, and `end_to_end.rs:553`'s `"transcript":4`), `lib.rs:15-26`; `production_tools.rs:410-419` (the `job` facts) leaves them; lines moved to `main`'s. Refresh any further copy of version 4 you find, and list it in the commit body.
     - Unchanged: jig (13 files on `smith_transcript::v3`, 3 naming `Returned::NotRun`, at `73734dd6`) and temper (1 file, at `9abb2f98`) break only when they repin, `later.md`'s row 8; the plan repins neither (README, "Repin").

117. **smith 08 against main after 07.1; rule 18's shared setup** (ready check of smith-08.1; check of 03:17).
     Ruling 117. 08.1 against `main` at `40974eae`, after 07.1's landing (supervisor.md 5, step 3: `git diff --stat 5e336b56 origin/main` lists `session.rs`, `world.rs`, `tests.rs` among 08.1's Files): only lines moved. `smith/08-batched-change.md` now cites `returned_cost`'s `NotRun` arms at `session.rs:2632-2635`, the `expect` "actual call owned result credit" at `:2535`, the world referee at `world.rs:1353-1358`, and the unit tests beside `:793`. Nothing else in ruling 116's Files and Shape moved.

     README rule 18 gains a sentence, the second sighting of the 15:08 steer's point 1 (07.1's `src/tests/window.rs` types one eight-line setup in four tests): "When two tests of a file open with the same three lines or more, those lines are written once, as a named function of that file that takes what differs."

118. **smith 08's outside-choice names in the session's record** (plan defect, smith 08.1's block of 2026-10-17T03:34:48Z).
     Ruling 118. A plan defect: 08.1's Shape gave the transcript's version 5 the allowed names (ruling 116) but missed that the channel's codec encodes the session's `record::Turn`, whose `NotRun::OutsideChoice` holds `ToolChoice::Only(Box<[Token]>)`. The design says what a told turn holds, in its own words: "A historical call holds no ticket that means something only inside the session that made it" (domain/session.md, section 3), and transcript.md 2, "not run, with why: outside the tool choice, naming the calls that were allowed". The implementer's proposal is approved:

     - The session's `NotRun::OutsideChoice` holds the allowed names, in `smith-domain`'s shape (`choice: Option<Box<[Box<[u8]>]>>`, `None` no call allowed), never tickets.
     - The root resolves the names when it narrows the choice (`route.rs:741`, from the peer's descriptors, as `retained_choice`, `peer.rs:359-378`, does today). The session keeps them beside its ticketed `choice`, which `chosen` (`session.rs:2144`) still reads; `choice_bytes` (`:753-760`) counts their bytes; `answer_unchosen` (`:2132-2142`) copies them. The session's own `None` (final turn, compaction) answers `None`.
     - The peer's `block` (`peer.rs:343-349`) moves them unchanged; `retained_choice` goes when nothing else calls it. Decode restores the names into the session's history unchanged, so a resumed history renders as it rendered when told (`a_resumed_outside_choice_call_renders_as_it_was_told`).
     - Default: `session::Event::Narrowed` gains the names beside `choice`. Another representation that keeps the told turn free of tickets is yours to choose; name it in the commit body.
     - Files add each user found on `main` at `9d1d7c86`: `session.rs` (`narrowed`, `choice_bytes`, `answer_unchosen`), `boundary.rs:132`, `domain.rs:202`, the session's `tests.rs` (`Narrowed` steps `:2522-2660`; `OutsideChoice` `:2557`, `:2694`), `tests/session/tests/` (`wind_down.rs:16`, `:49`; `child_final.rs:51`; `compaction.rs:76`; `memory.rs:1334`), `tests/session/src/world.rs:2100`, `window_referee.rs:150`, `route.rs:741-742`, `peer.rs:74-78`. Refresh any further constructor of `Narrowed` or `NotRun::OutsideChoice` you find, and list it in the commit body.
     - Size: 300 lines, a guide; past it, say why in the commit body.
     - Unchanged: `smith-domain`'s `llm::NotRun` and `llm::ToolChoice`; the prompt's texts; ruling 116's schema (`record OutsideChoice { only: option AllowedCalls }`); jig and temper, which break only when they repin (`later.md` row 8).

119. **smith 09's Shapes against main** (ready check of smith-09.1).
     Ruling 119. The ready check of smith 09 (09.1 against smith `main` at `9d1d7c86`; its sketch compiled under clippy in a scratch worktree; default 2339/2339 and fuzzy 43/43 with its corrections and a 7-byte description; 08.1, its need, not landed; the report is the run's `ready-check-09.1.md`) corrected `smith/09-polls.md`, Choices, Files, Shape and Removes:

     - `Event::EndWait { kit: Token, call: Token }`, the read found by `reply_to == ReplyTo::new(call)`, as `CancelJobs { kit }` (`boundary.rs:67`) names its kit; `JobAct::Hasten` a unit variant, `Request::Job { owner }` naming the read.
     - A Hasten owes no terminal, and each place that holds one act per owner or per job lets it pass: `operation.rs:67` ("one active job act per owner"), `process/jobs.rs:39-41`'s `unreachable!`, `workspace.rs:164-172` ("root machine requests own one callback right"), the job fakes' `assert!(…Held…)` (`tests/tools/src/world/jobs.rs:68`, `tests/agent/…:63`, `tests/protocol-llm/…:61`), `recorded.rs:343-345`, and the machine's sequence driver (`tests/machine/src/lib.rs:845-851`, `tests/machine/src/jobs.rs:36`).
     - Files add `smith-domain-tools/src/domain.rs:92` and `tests/tools/src/world.rs:508` (exhaustive matches on `Event`; a script `Step` that sends `EndWait`), `smith-protocol-machine/src/component.rs:112` and `tests/agent/src/memory/random.rs:221-226` (matches on `JobAct`). Lines moved: `tool_ran` `:1757-1779`; the `unreachable!` `world.rs:1197`, with its copy at `:2130`. Refresh any further match on `JobAct` or the tools' `Event` you find, and list it in the commit body.
     - `advance` stood at 99 of `too_many_lines`' 100: after 08.1's helper, aim for no new `#[expect]`.
     - Waits on the designer (`question.md` of 2026-10-17T03:53Z): the `job` description's words, which fail `smith-local-shell`'s `settings/tests.rs:310` on the host part (about 9 bytes of slack on `main`), and topic 3's "commands that change nothing", which no batch of the session's joins with a poll (`call.rs:236`, `session.rs:1648`). 09.1 is not made ready before both answers.
     - Unchanged: 09.1's Proof and Test shape; jig and temper, which use none of these names.

120. **08.1 and 09.1: the designer's answer, a poll beside any call and the role as written; 10.1's Shape without TASK_ROLE_BOUND** (design gap, smith 08.1's block of 2026-10-17T03:57:28Z).
     The designer's answer to `question.md` (2026-10-17T03:53Z, both headings), landed as smith `2c213b1b` (session.md 5 and 10, tools.md 5, llm.md 3, limits.md 4, shell.md 6.6, STATUS.md; `smith/08-batched-change.md`, `smith/09-polls.md`; Markdown only):


     **1. A poll beside a command: the design means the response.** shell.md 6.6's topic 3 stands as written: "commands that change nothing, with a running job's poll beside them". session.md 5 is widened to match it:

     > **A job's wait gives way to the rest of its response.** A `job` call waits as it asks only in a response whose calls are all `job` calls. Beside any other call, it waits for its job's end no longer than the other calls of its batch run: once every call of its batch that is not a `job` call has answered, it answers at once, `running` if its job still runs; in a batch of `job` calls only, as after a write or a command, it answers as soon as it starts.

     - **09.1's Shape** keeps its rule and adds one case. When the session starts a batch of `job` calls only, it gives each call its `EndWait` at once if the completion holds a call other than `job`. That is read from the completion's call blocks, whatever their answers. A completion of `job` calls alone sends nothing, and each call waits as it asks.
     - **Test shape** gains `a_job_call_in_a_batch_of_its_own_beside_other_calls_answers_at_once`, a table whose rows are a `job` call before an `edit` and one after a `shell` call. `job_calls_alone_wait_as_each_asks` is a completion of two `job` calls. Size: 240 lines.
     - **The role's words are unchanged** (09-polls.md's two quotes). Under this rule they hold for every call that topic 3 names.
     - **`job`'s description** gains these 132 bytes after "Without stop, read beside reads, lists and searches;": " beside other calls it waits no longer than they run, so such a poll costs nothing; only with job calls alone does it wait as asked;". llm.md 3 now asks: "that beside any other call of its response it waits no longer than they run, so such a poll costs nothing, and that it waits as it asks only in a response of `job` calls alone".

     **2. What gives: the role's reservation, nothing else.** limits.md 4, `host-part-holds-the-role`: the host's part "holds the task role as written, its bytes as the shell's code holds them, not its 4 KiB cap". shell.md 6.6: "its 4 KiB is the text's own cap, never a reservation in the host part". The descriptions keep what llm.md 3 asks. `guide`, the halves and the presets' windows do not move.

     - **08.1 makes the change** (its Choices, Files and Shape). `resolve.rs:86` gives `role_bound` the value `TASK_ROLE.len()` as a `u32`, and `TASK_ROLE_BOUND` goes. `settings/tests.rs:715` expects the role's length. `the_task_role_is_within_four_kibibytes` stays. If 08.1 is dropped, 09.1 makes it (its Choices).
     - **It sends no byte to the model, so 08.1's `parity-change` run stands,** and its commit body says so. `role_bound` feeds only `host_part_holds_the_role`, a startup check that derives no limit. It touches `role.rs`, which only a change step may touch (README rule 11, Done when item 8). That is why it goes in 08.1 and not in a step of its own.
     - **The room it leaves.** `role_and_guides` falls from 37,526 to 35,248 with 08.1's role (1,818 bytes), and to 35,345 with 09.1's (1,915 bytes). The descriptions cost about half a byte of host part per byte:
       - 08.1's sentence, 216 bytes in three descriptions where 61 stand now: about 233 bytes;
       - 09.1's 132 bytes: about 66.

       That takes the host part from 37,535 on `main` to about 37,236: about 1,890 bytes of slack.

     **Unchanged:** 09.1's Proof, its Don'ts and its heading; 08.1's sentence and topics; `job_wait`; ruling 119's corrections; `later.md` (nothing is deferred).


     Ruling 120 adds: 10.1's Shape no longer names `TASK_ROLE_BOUND`, which 08.1 removes; the role stays within the 4,096 bytes `the_task_role_is_within_four_kibibytes` checks, and each byte 10.1 adds counts in the host part, measured by `settings::tests::composition_adds_every_model_of_accounts_with_a_source` in its gate. Refresh any further copy of `TASK_ROLE_BOUND` or of 4096 as the role's reservation, and list it in the commit body.

121. **09.1's Shapes against main after 08.1** (ready check of smith-09.1, re-check after 08.1's landing).
     09.1's re-check after 08.1 (`d3cd7d31`) and the designer's `2c213b1b`, by a fresh read-only sub-agent against smith `d3cd7d31`, jig `b58a49c3`, temper `30f74c67e`. `smith/09-polls.md` takes:
     - Files: `session.rs` `tool_ran` `:1782-1805` (was `:1757-1779`), `advance` `:1514` (was `:1491`); `tests/session/tests/jobs.rs` added: `read_and_edit` (`:110`) completes `[read, edit, job]`, the designer's new case, so its poll is hastened; `a_command_answered_running_while_the_session_reads_edits_and_completes` (`:148`) still passes, named in the commit body.
     - Shape: `advance` stands at 100 of `too_many_lines`' 100 on `d3cd7d31`: each line added to it needs an extracted helper (the check's sketch moved the `Invalid` arm into `batching::invalid_slot`); `crates/clippy.toml` disallows `matches!` (`:84`) and `Iterator::any` (`:145`).
     - Removes: the `unreachable!` copies are `tests/session/src/world.rs:1197`, `:2128` (was `:2130`) and `tests/channel/tests/composed/long.rs:95`.
     - What the design asks: 08.1 made the role-bound change; with 09.1's words the role is 1,915 bytes, `role_and_guides` 35,345, the slack about 1,887 bytes (`composition_adds_every_model_of_accounts_with_a_source` passes).
     The check's sketch (`EndWait { kit, call }`, a unit `JobAct::Hasten`, the session's `batching::end_waits` at the end of `advance` and in `tool_ran`) passed fmt, workspace clippy, the default profile (2356 of 2356) and fuzzy (43 of 43), with 3 session, 3 tools-world and the machine's `a_hastened_read_answers_at_once` tests added. jig and temper use none of the names 09.1 changes. Refresh any further copy of a moved line, and list it in the commit body.

122. **08.1's one-sided bounds and hand-written digits, deferred** (steer of the check of 2026-10-17T06:36Z).
     Two `later.md` rows from the check of 2026-10-17T06:36Z, for work no remaining step's Files open: 08.1's one-sided bounds (`after_failed_write`'s refused positions; `allowed_calls_names` and `call_name_name`, the `4` unnamed), and `after_failed_write`'s hand-written digits beside `Decimal::of`. The steer of that check asks sessions 09 to 11 for bounds from both sides in their own steps; 08.1's tests stay as landed.

123. **smith 10's Shapes against main** (ready check of smith-10.1).
     10.1's ready check against smith `c36fa508` found three names that 08.1 (`d3cd7d31`) moved, now corrected in `smith/10-ranged-reads.md`'s Shape and Test shape:

     - `the_task_role_is_within_four_kibibytes` is at `role.rs:27` and `each_tool_the_task_role_names_is_one_of_smiths_own` at `:32` (were `:30` and `:35`); 09.1 and 10.1 each change a paragraph in place, so both lines hold after them.
     - `composition_adds_every_model_of_accounts_with_a_source` starts at `settings/tests.rs:308` (was `:310`, its `.unwrap_or_else` line).
     - The role is 1,818 bytes on main (was 1,728, before 08.1); with 09.1's words about 1,915; with this step's about 1,995, within the 4,096 cap. `TASK_ROLE_BOUND` is gone from main.

     The check applied 10.1's role words in a scratch worktree: `cargo fmt --check` and `clippy -D warnings` passed; the two role tests and `composition_adds_every_model_of_accounts_with_a_source`, 3 of 3; the workspace 2349 of 2349 (default) and 43 of 43 (fuzzy); with 09.1's words and `job` description as well, `smith-local-shell` 108 of 108. No user outside Files: `TASK_ROLE` is read only by `resolve.rs:85-88`, `resolve/local.rs:147` and `settings/tests.rs:714-717`; jig and temper name neither the role nor `smith_local_shell`.

     `docs/design/STATUS.md:44` still says the check "reserves its 4 KiB cap (`crates/smith-local-shell/src/role.rs`, `TASK_ROLE_BOUND`)": the designer's text, refreshed at its next landing; nothing for 10.1. Refresh any further copy of these line numbers you meet, and list it in the commit body.

124. **Rule 18: a fixture the code already holds is called** (steer of 2026-10-17T07:56:23Z).
     README rule 18 now also covers a fixture, setup, expectation or literal of three lines or more that the test's file or the crate's shared fixtures already hold: written once, as a named function that takes what differs, and called; the older copies stay as they are. It replaces the coordinator's four Pass on texts for the same fault (the 14:07 and 15:08 steers' point 1, 03:29 point 2, 06:46 point 3). Seen again in 09.1's candidate `2318318b`: `tests/tools/tests/jobs.rs` holds 23 `Outcome::Job {` expectations against main's 20, and `tests/machine/src/lib.rs` types the `JobAct::Hasten` filter twice. 09.1 writes its three new `Outcome::Job` expectations through one function of that file, and the Hasten-filtered count once, before it lands; production code is unchanged, so its live summary stands. Refresh any further copy of this rule you meet in a session file, and list it in the commit body.

125. **09.1 kept: RC's move is noise** (design answer, smith-09.1's block of 2026-10-17T09:23:40Z).
     The designer's answer, smith `7f3d4f4b`:


     **09.1 is kept.** benchmarks.md 16.6: "The step lands only on the designer's answer: kept, with the cause the answer names for the move; reworked; or dropped." The cause: RC's move is noise, not this change. The run's attempts place it where 09.1 changes nothing:
     - Before its first compaction, the change's arm read as its parent did: 59.8 reads an attempt against 58.0. After one, 21.8 against 11.8. It compacted in 5 attempts of 5, its parent in 3.
     - While a job ran, it made 1.88 responses a job against 1.97. Polls answered at once cost no extra responses.
     - Its 46 polls beside another call waited 43 ms in all; its parent's 31 waited 48.0 s. That is cause 4, removed as session.md 5 says.
     - The parent's code is 08.1's, byte for byte. As 08.1's arm it made 63.2 responses an attempt at a median $1.510; here, 52.4 at $1.291.

     **How it lands.** `smith/09-polls.md` ("Stopped and kept") gives the cause, as written, for the commit body. `c94fcaaa` is rebased onto `origin/main`. Since `c36fa508`, main has moved by Markdown only, so the run stands (README rule 11), and the body names `2318318b` and `c36fa508`. The rest of 09.1's Proof is unchanged: `Change-Summary:`, the criteria, projection and stop lines, and rule 13's counts. The gate runs once, at `land`.

     **The design** (16.6, new): "The cause may be noise rather than the change. The answer then shows, from the run's attempts, that the move lies in work the change does not alter … An interval that holds 1 is no such showing … The projection keeps the ratio as measured."

     **10.1** starts from 09.1's ratios as measured: RC 1.853, RT 1.326. It stops on an RC ratio above 0.936 or an RT ratio above 1.234 (`smith/10-ranged-reads.md`). If it stops on RC or RT, the question gives the outputs of commands C2 and C3 below, each run once on its run, beside rule 13's counts.

     **Not taken:**
     - Dropped: the move is not 09.1's, and session.md 5's rule would stay unbuilt.
     - Reworked: no change answers a move the change did not make, and running the same commit again is no rework (16.6, "One run per commit").

     **Deferred, with its `later.md` row:** the projection multiplies two runs of each landed code. Of 09.1's 1.853, a factor of 1.254 is 07.1's and 08.1's code measured twice. It goes to the designer's revision of 16.6, before the next parity plan runs its first change.

     **Unchanged:** rule 12 and the stop; 09.1's Shape, Size and Don'ts; the steers of 06:46 and 07:57; 11's closing runs decide.

126. **later.md: 09.1's typed calls** (the check of 2026-10-17T09:57Z).
     A `later.md` row for 09.1's `edit()` (`tests/session/tests/jobs.rs:313`) and `read()` (`:47`), which type calls that `smith_tools_world::calls` builds (`tests/tools/src/calls.rs:44`), where README rule 18 asks them called. No step left writes a test (10.1: "no new test"; 11: no code), so no steer; the next smith plan that opens the file takes it.
