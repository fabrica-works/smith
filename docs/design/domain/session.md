# Sessions

Provisional, 2026-10-05, revised 2026-10-20. What a smith session is, as
a domain layer: one conversation with an LLM, opened by a run. It is the
child domain `smith-domain-session`, whose own child is the tools domain
(tools.md). What is still open is listed in section 11.

## 1. In one page

- **A session is a conversation, only.** It knows the provider-neutral
  vocabulary of a conversation and nothing of runs, charters, results or
  hosts.
- **Turn by turn.** It calls the LLM, runs the tools the LLM asks for,
  sends their results back, and repeats until the LLM yields, a limit
  ends it, or its opener closes it.
- **Each turn is told** to its opener as it ends, with every name
  resolved, so that a turn means the same outside the session that made
  it: the main session's turns are its run's transcript.
- **One prefix per window.** Within a window the prompt only grows: the
  model, the affinity, the system text and the tools stay as they were,
  and history is appended to, never edited. Near the model's context
  window the session compacts, and a summary opens a new window.
- **Opened from a transcript,** a session goes on where an earlier one
  stopped, from its last window, with the same affinity. A window that
  ran on another endpoint, dialect, account or backend is carried over:
  compacted on the charter's LLM, and a new window opened (8.2).
- **Provider-neutral.** Nothing in a session depends on which provider
  answers; each provider's API is skein's shared LLM client's.
- **Bounded:** turns, tokens, spend, time and bytes held, each a budget
  its opener gives it; and each prompt by its model's window.

## 2. A session

- **Turn by turn.** A session that yields has not ended: its opener
  continues it with a new message, or closes it. Calls in the answer it
  yielded with are answered as not run when it continues.
- **A conversation's affinity.** Its opener gives each session an
  affinity: the run tree's identity as its key, and the conversation's
  thread, its ordinal in the run's activation, main's being zero (run.md,
  sections 3.2 and 5.3). The session puts it unchanged into every prompt. skein's
  dialect renders it, or ignores it where the provider has nothing to
  render it as (skein's llm.md). It is fixed for the conversation's life:
  across its windows (section 8), and across activations through its
  transcript, which records it (section 3). A session opened from a
  transcript keeps the affinity the transcript records.
- **Owned and delegated tools.** A session runs the tools it owns
  through its tools child domain, and delegates the rest to its opener
  (`finish`, `deliver`, `wait`, sub-agents and host tools, run.md,
  section 5) or, later, out of the domain (MCP calls, as opaque
  payloads). To the session they differ only in who answers: a delegated
  call is a ticket and an effect, and its answer is the result's bytes.
- **Everything in flight can be stopped.** Every tool call has a
  deadline, and an opener can abort its session, which cancels what is
  in flight and waits for it to settle (programming-model.md, 5.3).
- **No chains within a step.** An answer that comes back in the step that
  asked for it (a call the tools refuse at their entrance, one the opener
  answers at once) waits on the ready list (programming-model.md,
  section 2), so what one step emits and holds stays bounded.

## 3. Turns and transcripts

- **What a turn is:** one completion, its surrounding user messages, the
  provider's blocks in position, the calls with their provider ids,
  names and input as written, their results as concrete bytes, and what
  the completion used and cost. A withdrawn call has a result of its own
  kind, and a call cut off at the output limit is kept as a note of it,
  with its error result (section 4). A historical call holds no ticket
  that means something only inside the session that made it.
- **A turn has a kind:** an ordinary turn, as above, or a window turn,
  the compaction that closed one window and opened the next, holding the
  new window's opening as its messages (section 8.2).
- **Told as it ends.** Closing waits for actual terminal answers before
  telling the last turn, including answers that win a cancellation. The
  one exception is a delegated call whose answer its opener withholds
  and closes the session without giving, as a run parks with its `wait`
  withheld: the last turn is told with that call unanswered, and the
  opener answers it when it opens the conversation again (run.md,
  section 6). A turn's messages are those new since the turn before, so
  each message is told once and an ordinary turn's prompt is its
  transcript up to it. A trace records prompts the same way
  (protocol/events.md).
- **The stable prefix** is a contract, so that a provider can serve each
  prompt's prefix from its cache:
  - Within a window, the model, the endpoint's options, the affinity, the
    system text and the tools offered are fixed. The reasoning effort is
    the conversation's, from its charter, and a later activation may ask
    for another without a window boundary, since no block depends on it.
    History is append-only: each completion's prompt is the one before
    it, then that completion's blocks, then what was appended after them.
  - The system text and the tools offered are fixed for the
    conversation's life. A session never changes them; a run that resumes
    the conversation composes them again from its charter (run.md,
    section 3.3). A completion's tool choice narrows what the model may
    call (section 5) and changes no definition.
  - Dynamic guidance is appended as messages, never placed in the system
    text or a tool's description: budget and threshold notes, the
    wind-down and final-turn notes, and messages passed on between
    completions (run.md, sections 6 and 9).
  - A window boundary is the only exception (section 8.2). There, history
    is replaced whole; the system text, the tools and the affinity stay.
  - A prompt's encoding is byte-deterministic, so an unchanged prefix is
    unchanged bytes (protocol/llm.md, section 2.2).
- **Calls per response.** A completion's block bound is the declared
  calls per response; skein refuses a completion with more as a typed
  limit (section 4). A turn's messages are bounded by the transcript's
  blocks per message, derived from the same declared number and never
  below it (protocol/limits.md), so every completion the session accepts
  can be told.
- **Versioned.** Turns and transcripts are domain values with a version;
  their bytes are the protocol layer's (`smith-transcript`), so a host
  can keep them opaque and a later agent can resume them or know it
  cannot. A transcript records its conversation's affinity once, in its
  opening turn (protocol/transcript.md, section 3).
- **Opening from a transcript.** A session may open with earlier turns:
  its conversation's current window, from the conversation's first turn
  or from its last window turn, as they were told, including the
  provider's opaque blocks, which go back to the same provider and
  endpoint verbatim. History is checked before a completion starts:
  version, and one endpoint and dialect; contiguous turns; the affinity,
  account and backend in the
  opening turn; message and block bounds; roles and call and result ids;
  and no unresolved ticket. A window that ran on another endpoint,
  dialect, account or backend than the charter's main LLM is carried
  over, not refused (8.2). It must fit the session's byte limits with
  the waking prompt and room for a completion. A history that fits them
  but fails the window check (section 8.1) is compacted before its first
  ordinary completion. An unanswered call in the tail gets the same
  not-run result as an ordinary continuation, unless the opener answers
  it as the session opens, as a run answers the `wait` it withheld when
  it parked; answers the history does not hold go to the LLM as text in
  the waking prompt (run.md, section 6).
- **Refused transcripts.** Another version, a malformed, unresolved or
  oversized history are distinct refusals; the run treats every one as
  a transient failure (run.md, section 6).

## 4. Providers and retries

- **Any provider.** A session talks to the endpoint and model it was
  opened with; skein's shared client speaks each provider's API (HTTP,
  server-sent events, JSON) and its errors, and the protocol layer
  translates the session's vocabulary and tool schemas to it.
- **Retries are policy.** A session classifies a failed call
  (overloaded, rate limited, unavailable, timed out, context too long,
  invalid, unauthorised, an account exhausted or refused, a request
  unsupported, a local limit), retries the transient ones after a
  jittered exponential backoff, and gives up when its retries run out.
  An exhausted account, a refused one and an unsupported request are
  never retried: the same request on the same account would meet them
  again, and the agent never moves a run to another account
  (protocol/programs.md, section 3). The protocol layer runs the attempt and its
  deadlines. A provider's failure carries its HTTP status where it had
  one.
- **Timeouts carry their phase, and are retried by it:** connecting, the
  handshake, the response's head, idleness between events, or the whole
  call.
  - The head and idle deadlines are the model's, which its opener gives
    with it. The whole call's bound is the session's remaining time.
    There is no per-attempt ceiling: a stream that keeps making progress
    is cut only when the session's time runs out.
  - A timeout in any phase but the whole call is transient, and retried
    like any transient failure. A whole-call expiry is not retried: the
    session's time is up (section 6).
- **A full pool means waiting, not failure.** A call that finds the LLM
  pool at capacity, in connections or in memory, waits below the session
  for room, within its deadline. It is not a failed call and spends none
  of the session's retries.
- **A local limit is typed:** `Limit { which, bound }`, naming the limit
  that fired and its bound. It is not retried, since the same request
  would meet it again.
- **A request too large for skein's request bound** does not follow a
  passed admission. The session's own bounds on a window's messages,
  blocks and bytes are derived within skein's request and history
  bounds, with the system text and the tools at their largest, and are
  checked before each completion with the window (8.1), so a history
  that would pass skein's is compacted first (protocol/limits.md, section
  3.2). Should one fire anyway, a fault in that derivation, it is taken
  as *context too long*: one compaction, and a second in the same window
  fails the session.
- **A request past the line,** which the guard refuses before it is
  sent (8.1), is taken as *context too long*, unsent: one compaction, and
  a second in the same window fails the session.
- **Context too long** compacts the window once; a second in the same
  window fails the session (section 8.2).
- **A call cut off at the output limit,** which skein marks, never enters
  history as written: its input is incomplete, and a provider would
  refuse it on replay. The session replaces it in history with a note of
  it (its provider id, its tool and how many bytes of input it had) and
  answers it with an error result, the invalid call's problem *cut off*
  (protocol/llm.md, section 4), so the model can make it again with
  less. The session then goes on as after any answered call.
- **An oversized reasoning item** fails the completion with a limit
  naming the reasoning item. Dropping it instead is an explicit opt-in
  of the agent's configuration: the completion is kept without it, so
  the item never enters history, and the drop is reported as a fact.
- **The last failure's detail is the operator's.** The session keeps
  the detail of the last failure it met (the provider's text, clipped,
  or the limit and its bound) within a bound its limits give, each later
  failure replacing it. It goes with the session's end to its opener,
  and on to the agent's log and, as the capture policy allows, its trace
  (protocol/events.md). It never enters the model's history.
- **Credentials** are the protocol layer's. An unauthorised credential,
  an exhausted account and a refused one are each a notice the session
  raises, which the agent sends to its host as unauthorized, limited and
  refused (host.md, section 7). An unauthorised failure is transient when
  a refreshed credential may answer the retry: its call's grant was not
  a key, or a newer generation is already held. An actual unauthorised
  response on a key with none newer fails at once, after its notice,
  with the same class, evidence, report and account: no host renews a key
  (protocol/channel.md, section 6). The call retains its issued grant's
  renewal policy even if a later generation arrives while it is in
  flight. A missing or lapsed grant that fails unsent keeps its recovery
  retries. The others end the session as a failure of its model, naming
  the account. A model with no
  account raises none of them: no grant can come for it, so each ends
  the session as a failure of its model, with its provider's failure
  class and no account (protocol/charter.md, section 2). An unsupported
  request raises no notice: it names the field, and fails the session
  (protocol/llm.md, section 6).

## 5. Tools in a session

- **Tools run in parallel where that is safe.** Each tool has an effect,
  read or write. Adjacent reads in one turn run together, up to
  `parallel_tools` at once, or `calls_per_response` where that is
  smaller (protocol/limits.md, 2.3); the reads past it make the next
  batch, which starts once every call of the one before has answered. A
  write runs alone, and results go back in call order. A sub-agent's
  effect follows from its tools; a host tool's is as its charter
  declares; `finish` and `deliver` are writes; `job` is a read, and a
  write with `stop` (tools.md, section 5).
- **A job's wait gives way to the rest of its response.** A `job` call
  waits as it asks only in a response whose calls are all `job` calls.
  Beside any other call, it waits for its job's end no longer than the
  other calls of its batch run: once every call of its batch that is not
  a `job` call has answered, it answers at once, `running` if its job
  still runs; in a batch of `job` calls only, as after a write or a
  command, it answers as soon as it starts (tools.md, section 5). A poll
  beside other work costs at most that work's time, never its own wait,
  whatever the work's effect, so the session never needs to know what a
  command changes.
- **A failed write stops the writes after it.** When a write of a
  completion fails, the completion's later writes and commands are
  answered as not run, naming that call; its later reads still run. So
  a change made in several calls of one completion is never checked
  half made.
  - **A write fails** when its change is not known to be made whole:
    - **a call answered `invalid`,** whatever its tool and its effect:
      its arguments did not decode, or the tools refused them, so what it
      would have done is not known;
    - **a `write` or an `edit` answered anything but `written` or
      `edited`,** by protocol/events.md, 3.6's verdicts: a path refused,
      a version not read or stale, an `old` that matched nothing or more
      than once, edits that together leave the file as it was
      (`unchanged`: like every failed edit, it applies none of the
      call's edits, tools.md, section 3; revised 2026-10-20), a size
      refused, `failed`, which may have stored part of its
      change, and every other.
  - **Nothing else stops them:** a `shell` or a `job` call's answer,
    whatever it is but `invalid`, an exit that is not zero and
    `timed_out` included; a host tool's `error`; a sub-agent's answer; a
    call not run or withdrawn.
- **Jobs run beside the turns.** A `shell` call stays a write until it
  answers, `running` or ended. A job that is running holds no place in
  any later batch: the session's later reads, writes and commands run by
  these rules while it runs, and its completions go on.
  - **They are the session's writes in progress.** Its opener cancels
    them, and waits for them to settle, before another writer takes the
    lease or a check or a delivery runs (run.md, sections 5.3 and 8.1).
  - **They end with the session.** A session closing, for any reason,
    cancels its running jobs and waits for them to settle, then tells a
    `job.ended` for each job whose end no answer reported, before it
    ends (tools.md, section 5).
  - **None outlives its activation.** A session resumed in a later
    activation has no jobs.
- **Tool choice.** Each completion is asked with a tool choice: any tool
  offered (`Auto`), none (`None`), or only some (`Only(names)`). The
  tools offered never change (section 3); the choice narrows what may be
  called.
  - The opener gives the choice when it continues the session, as a run
    winding down offers only `finish` and `deliver` (run.md, section 9).
    The session sets `None` on its own final turn (section 7), and on a
    compaction (8.2), whatever the conversation's choice.
  - A call outside the choice, one a dialect that cannot express `Only`
    let through or one the model wrote anyway, is answered as not run.
    The session then goes on by its ordinary rules.
- **A call too large to take** arrives as an oversize block with its
  size. The session answers it as an invalid call the model can fix,
  never as a failed completion (protocol/llm.md, section 4).
- **The workspace is shared, knowledge is not** (tools.md, section 4):
  what a session's LLM has read, and at which version, is its own state.

## 6. Budgets and prices

- **Budgets:** turns, tokens (input, output, cache reads and writes, as
  the provider counts them), spend in the host's unit, and time, given by
  the opener at open; bytes held, against the agent's limits. Crossing
  a turn, token or byte budget stops the next completion, not the turn
  in flight: the calls of the completion that crossed it still run and
  settle. For spend, the session reserves each completion's maximum first
  (run.md, section 9), so spend never crosses it. Time is the exception:
  when it runs out, the session closes at once. A compaction is a
  completion like any other: it counts as a turn, and is reserved,
  charged and counted against these budgets (section 8.2). A job
  charges no turn, token or spend; its session's time runs on while it
  runs, as it does while any call runs, and a session that closes as its
  time runs out cancels its jobs with its other calls (section 5).
- **Context is a dimension of its own.** The window bounds one prompt
  (section 8.1); the token budgets bound what every completion used,
  summed. Neither stands for the other: a session well within its token
  budgets may have to compact, and one far from its window may cross a
  budget.
- **Usage** is what the provider counted: input, output, cache reads,
  cache writes and reasoning tokens. Each count is optional, and *not
  reported* is distinct from zero. Input, cache reads and cache writes
  are disjoint, as skein reports them: input is what was neither read
  from the cache nor written to it, so the three sum to the prompt's
  size. A token budget counts what was reported. Reasoning tokens are
  counted within output; they are reported for what they are, never
  charged again. A completion that
  reports no input or no output count is charged the reservation made
  for it, so spend is never under-counted (section 11).
- **Prices** are integer input, cached, cache-write and output amounts
  per positive `unit` tokens. New input uses the input rate, cache reads
  the cached rate, and cache writes the cache-write rate, which is the
  input rate when the charter gives none. The combined rational charge
  of **each completion** is rounded upwards once, using checked
  arithmetic; the result and the cumulative spend must fit `u64`.
  Overflow is a typed failure ending the session with a report of it,
  never a saturated charge: the charge that cannot be added is not, and
  the spend reported is what was charged before it.
- **Spend is cumulative** within an activation, sub-agents included. A
  delegated terminal, answered or withdrawn, carries the sub-agent's
  cumulative spend once under its call's identity; a duplicate or stale
  delivery cannot charge it again. Restoring history does not charge old
  activations again. A run counts each child once, through the parent's
  terminal answer, never adding both. Enforcing one aggregate budget
  across concurrently open sessions is the run's (run.md, section 9).

## 7. Sub-agents

A sub-agent is a session its run opens to answer a served call (run.md,
5.3). As a session it is like any other: its own tools, budget and model.
Its turns are not the run's transcript; they are summarised in the call
that asked for it, whose result is the child's final message.

- **The final turn.** A child's opener gives it, at open, a final-turn
  note and a wrap-up allowance of time (run.md, section 5.3). When the
  child's own turns are down to one, or its remaining time to that
  allowance, its next completion is its final turn: the session appends
  the note and asks with `ToolChoice::None`. Calls the model writes
  anyway are answered as not run. The session then yields that answer as
  its last, its budget spent, and the run gives it to the parent with
  `stop: budget` (run.md, section 5.3), so a capped child answers instead
  of ending unanswered.
- **A compaction due near the end.** The final-turn trigger counts a
  compaction that is due:
  - by turns, a child whose window needs compacting compacts with two
    turns left and answers with its last;
  - by time, the final sequence starts at twice the allowance, so the
    compaction and the final turn both fit what is left.
- **Its answer is bounded,** and its system text states the bound
  (run.md, section 3.3).
- **A main session has no final turn of its own.** Its run's wind-down
  gives it its last completions, with a narrowed tool choice (run.md,
  section 9).

## 8. Context management

A session keeps its conversation within its model's context window by
compaction into a new window. Nothing is edited in place: within a
window, history only grows (section 3).

### 8.1 Admission against the window

- **The window comes with the model.** The opener gives the model's
  usable input tokens and the most output tokens a completion may have,
  as the charter's model entry says (protocol/charter.md).
- **What the session tracks:** the last prompt's size, from its usage,
  the sum of its input, cache reads and cache writes, which do not
  overlap (section 6), each counted as reported; that completion's
  output, as reported, or the model's most output when none was; the
  bytes the session counts in that prompt; and the bytes it counts in
  what was appended since, the results and messages after that output.
  The session counts one measure throughout: the payload bytes of each
  block's text, call, result and replay as it holds them, never the
  cells that hold them, recounted from the history it loaded after a
  resume.
- **The appended count** turns those appended bytes into tokens at the
  window's measured ratio: the last prompt's counted bytes over its
  reported size, at least one byte a token and at most `TOKEN_BYTES`
  (protocol/limits.md, rule 4 of section 3). It is an estimate: content
  that the provider counts at more tokens a byte than the window so far
  passes it, and the line holds what it misses (below).
  - **Without a ratio:** with no earlier prompt in the window (a
    conversation's first completion, a window's first, the first after
    a resume from a window turn), or with usage that reports no input
    count, the bound is the whole prompt's bytes plus the allowance the
    session's limits give for what the protocol layer and the provider
    add (run.md, section 9), a byte a token. In a new window's first
    check, the summary that opens it counts as the output tokens its
    compaction reported, or the model's most output when none were
    reported, never as its bytes.
- **The check,** before each completion: the last prompt, plus its
  output, plus the appended count, is at most the window times its
  threshold, a fraction of the window that the session's limits give
  (protocol/limits.md). The arithmetic is checked. A completion that
  fails the check is not made: the session compacts instead (8.2).
  - **No output is reserved below the threshold.** The window bounds a
    prompt, and a completion's output is beside it (protocol/limits.md,
    2.1). What that output adds to the next prompt is held above the
    threshold.
  - **So the threshold is the prompt's.** A session compacts when its
    next prompt would pass the threshold's share of the window, as
    counted, not when that prompt and a completion's most output would.
- **The line holds a compaction.** The line is the window less one
  completion's most output, the summary instruction and a cut result's
  marker for each of `calls_per_response` calls; the spare is what lies
  between the threshold and the line (protocol/limits.md, 3.9). No
  ordinary completion is sent whose prompt may pass the line:
  - **the guard.** With each ordinary completion that has an earlier
    prompt in its window, the session gives the protocol layer the line,
    which its opener gave it at open from the charter's model entry
    (protocol/limits.md, 3.9), the last prompt's reported size, and
    where that completion's answer starts in the history. Before it
    sends the request, the protocol layer measures that answer and
    everything after it as the request encodes them, their framing
    included, a byte a token (protocol/llm.md, section 6). When the last
    prompt's size plus that measure passes the line, the request is not
    sent, and the completion fails as *context too long*, unsent
    (section 4): the window's one compaction (8.2);
  - **why it holds.** A token stands for at least one byte of the text
    it is read from, and encoding only adds bytes to that text, so the
    encoded measure bounds what the new part adds in tokens, whatever
    its ratio; its framing counts as encoded, as the prompt's allowance
    counts a whole prompt's (protocol/limits.md, 3.2). A prompt the guard lets through
    is within the line. Its output is at most the model's most output,
    so the compaction after it, carrying both, finds above them the
    summary instruction and a marker for each owed result.
- **The headroom holds a compaction.** Two relationships are checked at
  startup (protocol/limits.md):
  - the window above its threshold holds a completion's most output,
    the summary instruction and a cut result's marker for each of
    `calls_per_response` calls, so the line is at or above the
    threshold, and a compaction the check or the guard calls for always
    fits the window, its owed results cut to what is left;
  - a new window's opening at its largest (the system text, the task,
    the summary preamble, a summary of a completion's most output, and
    the messages held back, at most one offer of its opener's, which the
    opener bounds for this in messages and bytes, run.md, section 6) and
    one completion's most output fit within the window at its threshold,
    so a window just opened makes its first completion and holds that
    completion's output below its threshold. Where a declared window
    would not hold that material, the derivation lowers the material's
    bounds, never the window (protocol/limits.md).
- **The history's other bounds** (its messages, blocks and bytes) are
  derived so that the window check is reached first (protocol/limits.md).
  A history that would cross one of them anyway is compacted as a full
  window would be. A session never ends because its history grew.
- **A completion's results never cross them.** The derivation holds one
  batch's results whole: the calls that run together (section 5), each
  at its tool's caps (protocol/limits.md, 3.2). A later batch of the same
  completion runs within what is left of the history, less room for an
  answer to each call after it, shared among its calls: each result is
  what its tool answers with that share as its cap, saying what it left
  out (tools.md, section 4), and a text result, a host's answer or a
  sub-agent's, is cut with its marker. Every call is run and answered,
  and the compaction is owed each result as it was kept. A history so
  filled has no room for another completion, so the next is the
  compaction, whose request cuts the owed results to its own room (8.2).

- **Nor do they cross the line** (revised 2026-10-20). Every batch, the first
  included, also runs within an estimate of what is left below the line in
  the guard's units: the line, less the last prompt's reported size, less
  the retained byte count beyond that prompt's payload-byte baseline and
  everything already reserved, at `ENCODED_BYTES` (5/4: an allowance for
  escaping and framing as a request encodes them), less room for an answer
  to each call after the batch. The retained count includes the completion's
  answer, appended results and owned overhead. The smaller of this room and
  the history's is shared as above. So a completion of many large reads is
  answered with what fits, each result saying what it left out, and the next
  completion can be sent where the guard would otherwise refuse it unsent
  and the window compact without the results the reads were for. In a window
  with no earlier prompt, or where this room cannot give each call its
  minimum answer, the batch runs as before: the guard stays what makes the
  line hold, and this rule only makes its refusals rare.

### 8.2 Compaction

- **Its trigger:** a failed window check (8.1), a provider's *context
  too long* or the guard's (section 4; 8.1), or a session opened from a window whose last
  turn's model is not its charter's main model, as when a person chose
  another (`docs/design/shell.md`, 4.1). That compaction runs with the
  window's own model, when the agent's configuration declares it, and
  the new window opens with the charter's, which its window turn records
  (protocol/transcript.md, section 2), so a later resume does not
  compact again; one the configuration no
  longer declares makes the transcript one the run cannot resume
  (section 3).
- **A window carried over** (protocol/transcript.md, section 4): a
  session opened from a window that ran on another endpoint or dialect
  than its charter's main LLM, on another account or backend, or on a
  program whose session its host does not hold, compacts first. That
  compaction runs on the charter's main LLM, since the window's own
  account may not be granted, with the window's history less its
  opaque blocks and providers' replay, which go back only to the dialect
  that wrote them. It is otherwise this section's compaction, a model
  change's included: the new window opens on the charter's LLM, and its
  window turn records its endpoint, dialect, account and backend. On a
  program backend it runs as a program session of its own, with no
  tools (protocol/programs.md, 5.8).
- **The compaction completion** has the same model, affinity, system
  text and tools as the completion it stands in for, and is asked with
  `ToolChoice::None`, whatever the conversation's own choice; the new
  window asks with the conversation's choice again. It holds the
  window's history with every result owed to it, then one appended
  user message: the summary instruction, which the run gives at open
  (run.md, section 3.3). The results owed to it are rendered within the
  room the window leaves for them, each cut with its marker where they
  would pass it (protocol/limits.md, section 3.9). Its summary is its
  output, beside its prompt as every completion's is, and takes no room
  in the window (protocol/limits.md, 2.1). The history before them is
  the ordinary prompt's, unchanged:
  - **on the Codex dialect,** which keeps the tool choice outside the
    cached prefix, the request reads the cached prefix;
  - **on the Anthropic dialect,** a changed choice keeps the cache of the
    tools and the system text and loses that of the messages, so a
    compaction there writes its window's history to the cache once more
    (protocol/llm.md, section 2.2). The design accepts that cost, once
    per window, so that the model is asked for a summary and offered no
    call.

  Messages the opener passed since the last completion are held back for
  the new window. Calls in the compaction's answer, which a provider may
  write anyway, are not run.
- **The new window** opens with one user message of separate text
  blocks, in this order: the conversation's task (its opening message,
  carried from window to window); the summary preamble, which the run
  gives at open with the summary instruction (run.md, section 3.3); the
  summary (the answer's text); then the held-back messages, one block
  each. The compaction's own blocks are not kept, and the old window's
  history is released.
- **The summary** is the text of the answer's text blocks, in order,
  separated by a blank line. A summary cut at the output limit opens the
  window as written, with no marker, as an ordinary answer cut there
  stays in history as written;
  the compaction's `response.completed` has `stop` `max_tokens`
  (protocol/events.md, 3.5).
- **Told as a window turn,** like any other turn: its usage and spend
  are the compaction's, and its messages are the new window's opening
  (section 3). It records the conversation's affinity, so a transcript
  can start there: transcripts and resumes carry only the current window
  (protocol/transcript.md, section 3).
- **It counts as a turn and as spend.** It is reserved, charged and
  counted against the session's budgets like any completion (section 6).
  A budget that leaves no room for it ends the session for that budget.
- **Failure on repeat.** A window is compacted once. *Context too long*
  on the compaction itself, or a failed check or *context too long* in a
  window that has made no ordinary completion yet, fails the session
  with a limit naming the window. A compaction that fails otherwise
  fails as any completion does, after its retries (section 4).
  - **The window's limit keeps its cause.** When a provider's *context
    too long* caused it, the limit carries that failure's evidence and
    HTTP status, and the session's last failure detail stays the
    provider's text (section 4): the limit is the end it led to, not a
    later failure that replaces it. A check that fails before any request
    is unsent, and its detail is the limit and its bound.
- **No summary, no window.** An answer that is a refusal (its completion
  stopped for refusal, or it holds a refusal block), or whose text
  blocks hold nothing but ASCII whitespace, opens no window. The session
  fails at once, with the fault of the model's answer:
  - *refused*, for a refusal, whatever text came with it;
  - *truncated*, for no text and a stop at the output limit;
  - *malformed*, for no text otherwise.

  The compaction is neither retried nor nudged: a window is compacted
  once, and the headroom above the threshold holds one compaction, not
  an exchange after it (8.1). It is charged and counted against the
  budgets like any completion (section 6), and told as no turn, so the
  transcript still ends at the window's last told turn, and a session
  opened from it compacts first (section 3). The messages held back for
  the new window are not read. The run takes the failure as any
  session's: main's fails the run with that fault (run.md, section 10),
  and a child's returns unanswered (run.md, 5.3).

### 8.3 What else keeps context small

- **Results bounded at their source** stay the first defence: a read's
  window, a command's head and tail, search hits and list entries, each
  within the bounds the tools domain keeps (tools.md, section 4).
- **Children are the main isolation tool.** A child's reading never
  enters its parent; only its answer does (section 7).
- **No elision in place.** Old results are never shortened or removed
  within a window: that would edit history, lose the provider's cache
  from the edit onwards, and break providers' preserved reasoning. Only
  a window boundary drops history.

## 9. Below the domain

- **LLM providers:** skein's shared client (`skein-llm`) for each
  provider's API and its failures, rendering the affinity and the tool
  choice in each dialect; smith's protocol layer for tool schemas and
  decoding; credentials lent by the host (host.md, section 7).
- **Turns' encoding** (`smith-transcript`): a turn's bytes, with a
  version, every provider's opaque blocks kept verbatim with the dialect
  and endpoint they came from, and a transcript's affinity, account and
  backend, once.
- **Limits** derived from declared quantities (protocol/limits.md): the
  window's threshold and headroom, the history's bounds, the
  completion's block bound and the failure detail's.

## 10. The world

skein's fake LLM provider, whose completions a script draws (tool calls,
malformed input, calls cut off at the output limit, oversized reasoning
items, usage reported and not, every class of failure with each timeout
phase, streaming), the tools domain with the machine's faces, and a
scripted opener that continues, closes, aborts, narrows the tool choice
and answers delegated calls. Windows are small, so a conversation
compacts within a few turns and long stories stay within the suites'
budgets.

- **Its stories:**
  - a conversation that yields and is continued;
  - reads in parallel and a write alone; more adjacent reads than
    `parallel_tools`, the rest run in the next batch;
  - edits of several files and a command in one completion, run in call
    order; an edit among them that does not match, the writes and the
    command after it answered as not run, naming it by its position and
    its tool, and a read after it run; the same after an edit answered
    `unchanged` and after a read answered `invalid`; a command that exits
    non-zero, the writes after it run;
  - a command answered `running` while the session reads, edits and
    completes, then read to its end with `job`; a `job` call with the
    default wait beside reads, answered `running` once they have
    answered; a `job` call before an edit and one after a command, in one
    response, each answered at once; a `job` call alone, and `job` calls
    alone in their response, each waiting for its job's end; a `job`
    with `stop` alone in its batch; jobs cancelled by the opener before another
    writer, `cancelled` at their next read; a session closing with a job
    running, telling its `job.ended`;
  - every failure class retried or given up; head and idle timeouts
    retried, a whole-call expiry not; a full pool waited for;
  - a call cut off at the output limit, answered and made again; an
    oversized reasoning item failing, and dropped where opted in;
  - a budget crossed mid-turn, and one that leaves no room for a
    compaction;
  - a session opened from a transcript, and each refusal of one; one
    opened from a window turn; one whose tail holds a call its opener
    withheld at closing, answered by the opener as it opens; a window
    carried over from another dialect, account and backend, compacted on
    the charter's LLM with no opaque block sent;
  - an exhausted account, a refused one and an unsupported request,
    each failing at once with no retry, and an unauthorised credential
    answered by a refreshed grant; an unauthorised key failing at once,
    except when a newer generation is already held;
  - a conversation compacted at the threshold, and after a context too
    long; context too long on the compaction, failing; calls in a
    compaction's answer not run; a request skein finds too large, taken
    once as context too long;
  - the appended count at the window's measured ratio: a window whose
    prompts the provider counts at several bytes a token, then a turn
    whose results it counts at one byte a token, admitted by the check
    and refused unsent by the guard, then compacted within its window;
    a turn whose new part measures just within the line, sent; a
    completion of the most output just within the line, then its
    compaction fitting; a ratio below one byte a token, or above
    `TOKEN_BYTES`, taken at that bound;
  - a compaction asked with `None` and the window's tools; one in a
    conversation narrowed to `Only`, whose new window asks with `Only`
    again; a child's final turn falling due during a compaction;
  - a summary of several text blocks, opening its window after the task
    and the preamble, separated by blank lines; a summary cut at the
    output limit, opening its window as written; a refused compaction,
    and ones with no text, cut and not, each failing with its fault and
    telling no turn;
  - a wind-down offering only some tools, with a call outside them; a
    child's final turn answered with `stop: budget`;
  - a sub-agent's spend counted once; cache writes priced at their own
    rate; usage not reported, told apart from zero; a prompt's size
    taken as the sum of its disjoint input and cache counts.
- **Its referee:**
  - every call answered once, in call order;
  - turns told in order, each after its calls settled;
  - budgets exceeded by at most one completion;
  - every prompt of a conversation carrying its affinity, and distinct
    conversations distinct ones;
  - within a window, each prompt extending the one before; one window
    boundary per compaction; every compaction asked with `None`;
  - every job's end told once, and no job running once its session has
    closed;
  - no prompt whose bound exceeds its window, and no prompt, a
    compaction's included, whose size as the provider reports it does;
  - no ordinary completion made whose check, as 8.1 counts it, passes
    the threshold, and none sent whose reported prompt passes the line;
  - no failure detail in a prompt.
- **The provider counts tokens from bytes.** Where a story or the fuzzy
  suite reports usage, the world's LLM effect gives each block a
  rendered size, drawn at least its counted bytes, and counts its tokens
  at a ratio it draws between one byte of that size a token and
  `TOKEN_BYTES`, so that the appended count's error reaches its bounds
  both ways. It applies the guard as the protocol layer does, from that
  rendered size (8.1).
- **The fuzzy suite** draws usage, appended sizes, rendered sizes and
  those ratios, and checks that no request's bound or reported size
  exceeds its window, that none sent passes the line, and that no
  session ends for its history's length.

## 11. Open questions

- **Transcripts across providers:** a window is carried over by
  compaction (8.2), losing its opaque blocks; whether a later version
  keeps a provider-neutral form of them, so that less is lost.
- **A window's contents:** the task, the summary preamble, the summary
  and the held-back messages, as section 8.2 has them; whether the last
  results should also be kept is for the history probe to measure.
- **Which text makes the summary:** every text block of the compaction's
  answer, as section 8.2 has it, or only those a dialect marks as the
  answer rather than as working notes, where it marks them.
- **A job's end between completions:** the model learns a job's end only
  from a `job` call (section 5); whether the session appends a note of
  it, by the path messages mid-work take (run.md, section 6).
- **An unreported input or output count:** section 6 charges the
  reservation in its place. To confirm against each dialect's reporting.
- **Calls past the completion's bound:** failing the completion, as
  section 3 has it, or skein skipping them so that the session answers
  them as not run.
- **Routing state per turn:** if a probe shows that a provider's
  per-turn routing token is worth adopting, the session carries it
  within a turn and never keeps it in a transcript.
- **Provider-native compaction:** a dialect's own compaction items,
  replayed verbatim, as a later optimisation of section 8.2.

## 12. From temper

The session is temper's agent's, as its second version: turns told with
names resolved, transcripts versioned, prices and spend in a unit,
history checked before a completion. temper's first version, which
tickets made meaningful only inside its session, stays with temper's
legacy run until temper's cutover; smith starts at the second. Windows,
the affinity and tool choice are new.
