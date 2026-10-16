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

Empty until the plan is written.
