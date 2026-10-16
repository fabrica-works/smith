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
