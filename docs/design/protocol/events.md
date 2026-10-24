# Events

Provisional, 2026-10-09, revised 2026-10-20. The event vocabulary: one
versioned stream of JSON lines that says what a process running smith
did, from its start to its end. `smith exec --json` writes it to
standard output, and a trace file is the same stream. Its records,
their writer and their reader are `smith-events` (README.md, section
5). This document covers:

- the stream, and who writes it;
- its records and their fields;
- which facts and measurements feed each record;
- delivery and capture;
- its version.

## 1. In one page

- **One vocabulary, one stream.** Every record is one line holding one
  JSON object with `"v"`, `"type"` and `"t_ms"`. A process writes one
  stream: `session.started` first, `session.ended` last, and between them
  its runs, their conversations, each LLM response, each tool call, the
  end of each job that no call reported, each message, the budget and
  notices for operators.
- **Two destinations, one stream.** `smith exec --json` writes the
  stream to standard output; a trace file is the same stream, appended.
  Same records, same order, same encoder; each destination counts its
  own loss.
- **Documented and versioned, and it changes with smith.** The
  benchmarks' harness, the live tests and the front ends read it through
  `smith-events`. Like every format it is pre-release: it has one
  version, any change a reader would notice is a new one, and a reader
  refuses a stream of another (section 6).
- **Built from typed sources, never from text.** Records come from the
  domain's facts, the protocol layer's measurements and the shell's
  (section 4). Nothing in the stream is a type's debug rendering.
- **Authoritative records are never dropped:** `session.started`,
  `run.completed` and `session.ended`. Every other record follows its
  sink's policy, and what is dropped is counted, so a reader can prove a
  stream complete, or know what it lacks.
- **Complete by default.** The stream is held back by its destination
  rather than dropping records; a sink may slow the run, never change
  what it does (agent.md, section 5).
- **Content only under the capture policy.** Prompts, completions, tools'
  input and output, messages' text and live text appear only as the
  policy allows, or, for live text, as a front end asks; a prompt
  appears as each request's new messages, never whole. Credentials never
  appear.
- **Totals add up.** Usage and spend are per response; a run's totals are
  their sums, and a reader can check them.

## 2. The stream

- **Who writes it.** The process that runs a run: the agent process
  writes its trace (agent.md, section 5), and the product writes the
  stream of the run its inline agent composes (hosts.md, section 5.6). A
  host that spawns an agent hears its facts on the channel (channel.md,
  section 8), which carries them in its own codec, not as this stream.
  The product's stream is the local service's. It holds the run's
  records whichever kind of agent the local host runs, and the host's
  own notices, such as a spawned agent's forced exit.
- **Where it goes.** To standard output under `smith exec --json`, and to
  a trace file when the configuration names one (agent.md, section 4;
  `docs/design/shell.md`). A process writing both writes the same records
  to each; each destination is a sink of its own, with its own count of
  loss (5.2). When the product spawns its agent, the product's stream
  goes to the configured trace file, and the agent process's own trace
  to the same path with `.agent` appended, so that each file holds one
  process's stream.
- **One process, one stream.** A trace file appended across invocations
  holds one stream after another, each from its `session.started` to its
  `session.ended`.
- **Order.** Records go out in the order the process made them: a
  domain's facts in the order its steps pushed them, each measurement
  with the fact it belongs to.
- **Time.** `t_ms` is milliseconds since the process started, on the
  monotonic clock, taken when the event happened: a fact's at the step
  that made it (domain/run.md, section 11), a measurement's when it was
  taken. Durations within a record are on the same clock.
  `session.started` holds the wall clock once, so a reader can place the
  stream in the world.

## 3. Records

### 3.1 Every record

```json
{"v":4,"type":"tool.completed","t_ms":48213,"run":3,"conversation":1,"call":"call_4f2","tool":"read","verdict":"read","bytes":2048,"omitted_bytes":0,"clamped":false,"duration_ms":12}
```

| Field | Type | What it is |
|---|---|---|
| `v` | integer | the vocabulary's version: 4 |
| `type` | string | the record's type, `noun.verb` or a single word |
| `t_ms` | integer | milliseconds since the process started, when the event happened |

**Names** that recur:

| Field | Type | What it names |
|---|---|---|
| `run` | integer | a run, by its activation number (domain/run.md, 3.2), unique within the stream |
| `conversation` | integer | a conversation within its run, numbered in the order the run opened them, compactions included; main's is 0 |
| `response` | integer | one provider attempt within its run, numbered in order: its id in the stream, on each of its records |
| `call` | string | a tool call, by the provider's id for it |
| `name` | integer | a message, by the name its host gave it |

A field this document marks optional is absent when it does not apply.
`null` means a value is not known, never zero.

### 3.2 The process

`session` names the process's stream, from its start to its end, not a
conversation, which is `conversation`.

**`session.started`**, authoritative, first:

| Field | Type | What it is |
|---|---|---|
| `wall_ms` | integer | the wall clock at the process's start, in milliseconds since the Unix epoch |
| `agent` | object | `name` and `version`: the agent name and release version |
| `mode` | string | `agent`, a spawned agent process; `exec`, headless; or `chat`, interactive |
| `chat` | string, optional | the product's chat, by its name, under `exec` and `chat` (`docs/design/shell.md`, section 4) |
| `pid` | integer | the process's id |
| `profile` | string | the profile's name (limits.md, section 1) |
| `capture` | string | the capture policy: `none`, `calls` or `everything` (5.3) |
| `delivery` | string | this sink's policy: `complete` or `best_effort` (5.2) |
| `versions` | object | the version of each format the process speaks: `events`, `channel`, `charter` and `transcript`, and, for the product, `chat_store` |
| `limits` | object | the effective limits, by name, in their units (limits.md, section 9) |
| `containment` | string | `groups`, commands in process groups and not contained; or `trees` (agent.md, section 3) |

**`session.ended`**, authoritative, last:

| Field | Type | What it is |
|---|---|---|
| `exit` | integer | the code the process exits with: for the product, one of `docs/design/shell.md`, 4.3, written once its own output, `-o`'s file among it, has been written or has failed; for an agent process, 0 once it answered and 1 when it could not (agent.md, section 6) |
| `answered` | bool | whether every run it held answered |
| `teardown_ms` | integer or null | from the last answer to the end: `Answered` to `Closed` (agent.md, section 6) |
| `aborted` | bool | a termination signal turned graceful closes into aborts |
| `emitted` | object | per record type, how many the process made |
| `loss` | object | per sink, records not delivered: `events`, this destination's; `channel`, facts not projected, or null without a channel |
| `cpu_ms` | object or null | `user` and `system`, the process's own; `children_user` and `children_system`, its reaped children's; null when the process could not read its usage |
| `peak_rss_bytes` | object or null | `self`, the process's largest resident size; `children`, the largest of its reaped children's; null when the process could not read its usage |

A usage the process could not read (skein io's `Measured::Unread`) is
told as null in both fields, never as zero. It changes nothing else:
`session.ended` is still written, once and authoritative, and the run's
and the session's terminals and the exit code are those the run
earned. Resource usage is an observation, not an outcome. The shell
also writes an `operator` notice saying that usage could not be read.

### 3.3 Runs

**`run.started`**, when a run is admitted:

| Field | Type | What it is |
|---|---|---|
| `run` | integer | |
| `resumed` | bool | main opens from a transcript |
| `messages` | integer | the start's messages (domain/run.md, 3.2) |
| `main` | model | main's LLM (3.10) |
| `budget` | object | `turns`, `spend`, `time_ms`, and `reserve`: `turns`, `time_ms` and its derived `spend` (domain/run.md, 9.1) |
| `tools` | object | `families` granted, as a list of `inspect`, `modify`, `shell` and `sub_agents`; `withheld`, the granted families the run does not offer, `sub_agents` on a program backend (programs.md, section 2); `wait` and `deliver`, bools; `host`, the host tools declared |
| `contract` | list | the result forms the contract allows: `report`, `verdict`, `change`, `failure` |

**`run.completed`**, authoritative, once per run, also for a start
refused at the entrance, which has no `run.started`:

| Field | Type | What it is |
|---|---|---|
| `run` | integer | |
| `status` | string | `accepted`, `parked`, `failed` or `refused` (domain/run.md, section 10) |
| `failure` | answer failure | for `failed` and `refused` (3.10) |
| `result` | object | for `accepted`: `form` and, for a verdict, `label`; `text` and `fields` under `everything` |
| `wind_down` | bool | the answer came in the wind-down (domain/run.md, 9.1) |
| `turns` | integer | the turns told in this activation |
| `spent` | integer | what the run spent, in the host's unit |
| `usage` | usage | the run's totals, every conversation's included (3.10) |
| `read` | integer or null | the final fence: the last message read |
| `unread` | integer | the messages it answered without reading |
| `duration_ms` | integer | from admission to the answer |

A run dropped before it answered (domain/host.md, 9.2) has no answer, so
its host writes `run.completed` from the turns it was told:
- `status` is `failed`;
- `turns`, `spent`, `usage` and `read` come from those turns;
- `unread` counts the messages delivered after the last one read;
- `duration_ms` runs to the run's end;
- there is no `result`, and `wind_down` is false.

### 3.4 Conversations

**`conversation.opened`:**

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation` | integer | |
| `kind` | string | `main`, `child` or `compaction` (domain/session.md, section 8) |
| `parent` | integer, optional | a child's asker, or the conversation a compaction summarises |
| `call` | string, optional | the `sub_agent` call that opened a child |
| `thread` | integer | the affinity ordinal its requests carry (llm.md, 2.1) |
| `model` | model | its LLM (3.10) |
| `requested` | object, optional | a child's ask as the call made it: `turns` and `seconds`, whole seconds, each optional |
| `share` | object, optional | a child's effective share, exact: `turns`, `time_ms` and its `spend` (domain/run.md, 5.3) |
| `clamped` | list, optional | each part of a child's share that was lowered: `part`, `turns` or `time`; `by`, what bound it, such as `reserve` |

**`conversation.closed`:**

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation` | integer | |
| `end` | string | `closed`, `budget`, `failed`, `refused`, `transcript` or `overflow` (domain/session.md) |
| `which` | string, optional | for `budget`, what ran out |
| `failure` | completion failure, optional | for `failed`, when a completion failed (3.10); absent when a compaction's answer gave no summary, whose `response.completed` gives its `stop` (domain/session.md, 8.2) |
| `turns` | integer | its completions |
| `usage` | usage | its totals |
| `spent` | integer | its spend |
| `duration_ms` | integer | from opened to closed |

### 3.5 Responses

One `response.started` and one `response.completed` per provider
attempt, retries included.

**`response.started`:**

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation`, `response` | integer | |
| `turn` | integer | the conversation's completion this attempt is for, from 1 |
| `attempt` | integer | 0, then one more for each retry |
| `window` | integer | the conversation's window, from 1, one more after each compaction |
| `model` | model | (3.10) |
| `messages` | integer | the messages in its prompt |
| `output_tokens` | integer | the most output it asked for |
| `tool_choice` | string | `auto`, `none` or `only` (llm.md, section 3) |
| `prompt` | object | under `everything`: the request's new part (3.11) |

**`response.completed`:**

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation`, `response` | integer | |
| `outcome` | string | `completed`, `failed` or `cancelled` |
| `stop` | string, optional | for `completed`: `end`, `tools`, `max_tokens` or `refusal` |
| `blocks`, `calls`, `invalid`, `oversized`, `cut` | integer | the answer's blocks; its calls; those that did not decode; those past one call's input; those cut off at the output limit (limits.md, section 6) |
| `usage` | usage | as the provider reported it (3.10) |
| `spent` | integer | this completion's charge, in the host's unit |
| `request_bytes` | integer or null | the request as encoded, measured before it was sent; null when the attempt ended before its request was measured |
| `first_byte_ms` | integer or null | from the request sent to the response's first byte |
| `largest_gap_ms` | integer or null | the largest gap between two of the response's events that carry the model's output, a provider's pings not among them, the measure its `idle` deadline bounds (skein's llm-connection.md, section 6) |
| `duration_ms` | integer | from `response.started` |
| `failure` | completion failure, optional | for `failed` (3.10) |
| `retry_ms` | integer, optional | for a failure the session retries: the wait before the next attempt |
| `completion` | object | under `everything`: the answer's blocks (3.11) |

**`text.delta`,** live text: a piece of a response's text or refusal as
it streams, before its block completes. It is content: a destination's
stream carries it only under `everything`, and a front end in the
process that shows words as they come asks for it for itself
(`docs/design/shell.md`, 4.1). Neither asking, it is not made. It is
counted like any record that is not authoritative, and its block, once
complete, is in `response.completed`.

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation`, `response` | integer | |
| `block` | integer | the block's index within the response |
| `text` | string | the piece, as it came |

**On a program backend** (programs.md, 5.5), each completion the
program reports is one response: `response.started` at its first
partial message, `response.completed` at its assistant message. A
`system/api_retry` is a `response.completed` whose `outcome` is
`failed`, with the class its status gives and `retry_ms` its delay,
and the next attempt's `response.started`. The agent sees no request:
`request_bytes`, `first_byte_ms` and `largest_gap_ms` are null, and
`prompt` holds only what the agent sent the program, the start's text
and its user messages.

### 3.6 Tools and checks

**`tool.started`,** for every call a session runs or delegates: the
workspace's tools, the run's (`finish`, `deliver`, `wait`, `sub_agent`)
and the host's; and on a program backend, each of the program's built-in
tools that stands in for the workspace's, with `source` `workspace`
(domain/tools.md, section 10):

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation` | integer | |
| `call` | string | the provider's id for the call |
| `tool` | string | its name |
| `source` | string | `workspace`, `run` or `host` (domain/run.md, 5.1) |
| `effect` | string | `read` or `write` |
| `deadline_ms` | integer or null | its deadline, as a duration |
| `input_bytes` | integer | the input's size as the model wrote it, under every policy |
| `input` | string | under `calls` and `everything`: the input as the model wrote it |

**`tool.completed`:**

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation` | integer | |
| `call`, `tool` | string | as started |
| `verdict` | string | how it ended, without what it said: a workspace tool's outcome kind (`read`, `listed`, `found`, `written`, `edited`, `exited`, `conflict`, `missing`, `too_large`, `timed_out`, `running`, `stopped`, `failed`, `cancelled`, ...), `result` or `error` for a served or host tool or a program's built-in (as the program told it), `invalid`, `not_run` or `withdrawn`. A `shell` call answered `running` left its command running as a job; a `job` call's verdict is `running` while its job goes on, or its job's end: `exited`, `timed_out`, `stopped` or `cancelled` (domain/tools.md, section 5) |
| `exit` | object, optional | a command's or a job's end: `code`, or `signal` |
| `job` | integer, optional | for a `shell` call answered `running`, and for a `job` call not answered `invalid`: the job's number in its conversation |
| `bytes` | integer | the result's payload |
| `omitted_bytes` | integer | what the tool's own caps left out of its result: a command's output, or a job's since its last answer, beyond its kept head and tail, or a read beyond its window. What the render bound later cuts is told with the request that renders it, not here |
| `clamped` | bool | the call's requested budget or deadline was lowered to its ceiling, as the tool's outcome records it |
| `duration_ms` | integer | from `tool.started` |
| `child` | object, optional | for `sub_agent`, under every policy: `conversation`; `stop`, how the child ended: `end`, an answer it chose to give, `budget`, an answer its share ended (domain/run.md, 5.3), or, unanswered, `failed` or `closed`; `turns`, `time_ms`, `spent` and `usage` |
| `delivery` | string, optional | for `deliver`, or a `finish` that delivered: `delivered`, `nothing`, `refused`, `failed` or `stale` (domain/run.md, 8.2) |

A tool's terminal tells what the tool observed when it ended. Rendering
happens later, when the result goes into the next request, and may never
happen if the session closes first. So the result as the model read it,
and the bytes its render bound cut, are told with the request that
rendered it: in that request's new part, each rendered result with its
cut (under `everything`; 3.11). `tool.completed` carries no result text.
A tool's typed outcome has no text of its own before rendering, and the
rendered one is the record of what the model saw.

**`job.ended`,** for a job whose end no call's answer reported, told when
its session closes (domain/tools.md, section 5). Every job has exactly
one terminal: the first `tool.completed` whose verdict reports its end,
or this record, never both.

| Field | Type | What it is |
|---|---|---|
| `run`, `conversation` | integer | the session that started it |
| `job` | integer | its number in its conversation |
| `call` | string | the provider's id for the `shell` call that answered it `running` |
| `verdict` | string | its end as observed: `exited`, `timed_out` when its deadline had passed, or `cancelled` |
| `exit` | object, optional | for `exited`: `code`, or `signal` |
| `unread_bytes` | integer | what it wrote after the last answer that named it, which the model never read; what earlier answers dropped is in their `omitted_bytes` |
| `duration_ms` | integer | from its `shell` call's `tool.started` to its end as observed |

**`check.started`** (`run`, `deadline_ms`) and **`check.completed`**
(`run`, `exit`, `passed`, `duration_ms`), for the workspace's checks
(domain/run.md, 8.1). Every `check.started` has one `check.completed`,
whose `exit` says how the checks ended, as one of five kinds:

- `{"kind":"code","value":n}`: the checks' process exited with code `n`;
- `{"kind":"signal","value":n}`: a signal ended it, with its number, which
  the machine keeps;
- `{"kind":"timed_out"}`: their deadline passed, and their tree was
  stopped, whether or not a process had started;
- `{"kind":"unstarted"}`: they never started, because the spawn failed.
  The failure's detail goes in an `operator` notice;
- `{"kind":"cancelled"}`: the run withdrew them before they ended, by a
  cancel, a drop or a wind-down, before or after their process started.
  Their tree was stopped.

`passed` is true only for code 0. A command's `exit` in `tool.completed`
keeps its own shape.

### 3.7 Messages

Every message the run hears has exactly one terminal (domain/run.md,
section 6): refused in its `message.received`, or a later
`message.read` or `message.unread`. A start's messages each have a
`message.received` at admission, after `run.started` and before any
message relayed later; a start refused at the entrance refuses them,
each with `start_refused`, before its `run.completed`.

| Type | Fields | When |
|---|---|---|
| `message.received` | `run`, `name`; `bytes`, its size as the run reads it; `refused`, optional: `too_large`, `inbox_full`, `name_in_use`, `ending` or `start_refused`; `label` and `text` under `everything` | the run admitted it or refused it |
| `message.read` | `run`, `name`; `turn`, the told turn whose fence covered it | a told turn's fence named it or a later message |
| `message.unread` | `run`, `name` | the run answered without reading it |

### 3.8 Budget

**`budget`,** a snapshot of the run's budget each time a completion is
charged, when its stage changes, and as a sub-agent's return writes its
usage line (domain/run.md, 9.2): that record holds the values the line
tells, and names the call. The record that enters the wind-down is the
one whose `stage` is `wind_down` first.

| Field | Type | What it is |
|---|---|---|
| `run` | integer | |
| `stage` | string | `ordinary`, or `wind_down` once the ordinary pool is spent (domain/run.md, 9.1) |
| `turns`, `spend`, `time_ms` | object | each `used` and `limit` |
| `reserve` | object | `turns`, `time_ms` and `spend` |
| `call` | string, optional | the `sub_agent` call whose return wrote the usage line this record tells; absent on every other `budget` record |

### 3.9 Notices

**`notice`,** for operators: `level`, `info` or `warning`; `kind`; `run`
when it is about one; and the kind's fields.

| Kind | Level | Fields | When |
|---|---|---|---|
| `not_contained` | warning | | at startup, while commands run in process groups rather than contained trees |
| `limit_near` | info | `which`, `bound`, `value` | the first time in a run a measured size passes the compaction threshold's share of its limit |
| `reasoning_dropped` | warning | `bytes` | a reasoning item past its bound left out of later requests, as the model's declaration allows (limits.md, section 2.1) |
| `credential_unauthorized` | warning | `account` | a provider refused a credential, or a program source's login was lost (programs.md, 5.6) |
| `account_limited` | warning | `account`, and `reset_ms` when its provider said | an account reached a usage limit; `reset_ms` is how long from this record until it may be used again |
| `account_refused` | warning | `account`, `code` | a provider refused the account what the run asked, with its code. The notice's reason is the provider's text, so it is never here: it is the failed response's `detail`, content under the capture policy (3.10) |
| `teardown_aborted` | warning | | a termination signal after the answer turned closes into aborts |
| `forced_exit` | warning | `exit`, `forced` | a host's agent did not exit on its own (domain/host.md, section 4) |

### 3.10 Shared values

- **model:** `endpoint`, `model`, `window_tokens` and `output_tokens`,
  the window and output the charter gave it (charter.md, section 2),
  `effort`, or null, `account`, the number of the account that pays for
  it, its charter's `u32`, or null for none, and `backend`, `direct` or
  the program's name, `claude_code` (programs.md).
- **usage:** `input_tokens`, read afresh; `cache_read_tokens`;
  `cache_write_tokens`; `output_tokens`; and `reasoning_tokens`, the part
  of the output spent on reasoning. Each is an integer, or null when the
  provider did not report it (skein's `llm.md`). A prompt's size is its
  input, cache reads and cache writes. A total sums what was reported,
  including a response whose accounting the session refused, and is null
  only when nothing was.
- **A total past `u64`.** Totals add with checked arithmetic. A total
  that would pass `u64` is told as null in that field, never saturated
  or wrapped. That takes reported counts summing past 18 quintillion,
  which only a broken provider sends. The session has already ended with
  its typed usage overflow, which `run.completed`'s failure and an
  `operator` notice name. The format's integers are not widened for it.
- **completion failure:** `class` (`overloaded`, `rate_limited`,
  `exhausted`, `refused`, `unsupported`, `unavailable`, `timed_out`,
  `context_too_long`, `invalid`, `unauthorized`, `limit`, `protocol`,
  `cancelled`);
  `evidence` (`unsent`, `maybe_sent` or `response`); and, as they apply,
  `phase` for a timeout (`connect`, `handshake`, `head`, `idle`,
  `whole`), `limit` (`which` and `bound`) for a limit, `status` for an
  HTTP failure, `retry_after_ms` when the provider said, `field` for
  `unsupported`, the field the provider does not take, `code` and
  `request_id`, the provider's, when it gave them, and `detail`,
  the provider's text or the limit's words, as the session kept it
  (limits.md, 3.2): content, so only under `calls` and `everything`
  (llm.md, section 6). A limit a provider's failure caused, the window's
  after *context too long* (domain/session.md, 8.2), is class `limit`
  with its `limit`, and that failure's `evidence`, `status` and `detail`,
  in `conversation.closed` and `run.completed`; that attempt's
  `response.completed` keeps the provider's class. A limit no request
  met is `unsent`, its detail the limit's words.
- **answer failure:** `class`, one of `model` (with `endpoint` and
  `model`, the last completion's, `attempts`, how many it made, and
  `completion`, a completion failure; or with `account` and `state`:
  `limited`, with `reset_ms`, how long from this record until it may
  be used again, when the provider said, as `account_limited` has it
  (3.9), `refused`, with `code`, or
  `unauthorized`, a program source's login lost; or with `program`, its
  name, a program that could not run or exited before its answer, with
  its `exit` when it exited and, as content, its `detail`
  (programs.md, 5.6); or with
  `which`, a fault of the model's own answer, `refused`, `truncated` or
  `malformed`, domain/run.md, section 10), `budget` (with `which`: `turns`,
  `spend` or `time`, and `used` and `bound` in its unit: a count, the
  host's unit, or milliseconds of the run's wall time), `policy`, `cancelled` (with `dropped`), `stale`,
  `transcript` (with `reason`), `agent` (with `ended`) or `host` (with
  `reason`) for a failed run; `busy` or `invalid` (with `which`, what was
  beyond the limits, and `bound`) for a refused one.
  - **Without an answer,** the class is the first cause of the end:
    - `cancelled`, `dropped: true`: the host asked for the end, at a
      second interrupt or past the cancel's grace, and the run went
      without answering. A cancelled answer has `dropped: false`;
    - `agent`, with `ended`: a spawned agent ended without answering,
      with its exit status or signal;
    - `host`, with `reason` `store` and the `turn`: the host could not
      keep that turn and dropped the run (domain/host.md, 6).

### 3.11 What content looks like

- **A prompt's new part:** `system` and `tools`, the system text and the
  names of the tools offered, on a conversation's first request and on
  the first of each new window; then `messages`, the messages the
  conversation appended since its previous request, without the
  completion that request's `response.completed` already holds. A
  compaction's first request continues its `parent`'s history. A retry's
  new part is empty: its request is the attempt's before it. So a reader
  rebuilds every request in order, and a trace grows with the
  conversation, not with its square.
- **A message:** `role`, `user` or `assistant`, and `blocks`.
- **A block:** `{"type":"text","text"}`, `{"type":"refusal","text"}`,
  `{"type":"call","call","tool","input"}`,
  `{"type":"oversized","call","tool","bytes"}`,
  `{"type":"cut","call","tool","input"}`,
  `{"type":"result","call","error","text"}`, or
  `{"type":"opaque","bytes"}`: a provider's opaque block is never
  carried, only its size.
- **A completion:** its `blocks`, in order.
- **Text is UTF-8,** as the model read or wrote it: rendered results are
  (llm.md, section 5), and so are calls' input and messages, checked when
  decoded.

## 4. Where records come from

| Record | Fed by |
|---|---|
| `session.started` | the shell: the wall clock, the process id, the agent name and version; the configuration: profile, capture and delivery; the service's effective limits (limits.md, section 9); the machine's containment |
| `run.started` | the run's fact that it was admitted (domain/run.md, section 11), with the charter's main model, budget, reserve, tools and contract |
| `run.completed` | the run's answer (domain/run.md, section 10), where the stream's writer meets it: the agent service as it sends it, or the local service as its local domain receives it from its agent: status, typed failure, result, turns, spend and the final fence; the run's usage totals; a refusal at the entrance |
| `conversation.opened`, `.closed` | the run's facts that a conversation opened or closed, with its parent, a child's effective share and clamp reasons; the session's end, with its turns and usage |
| `response.started` | the session's fact that a completion started (its attempt, messages and output); the domain's content under capture |
| `response.completed` | the session's facts that a completion answered, failed, was cancelled or will be retried, and its usage and charge; the LLM component's measures of the encoded request, its first byte and its largest gap (llm.md, section 6); the domain's content under capture |
| `text.delta` | the LLM component's deltas, which the session never holds (llm.md, section 6) |
| `tool.started`, `.completed` | the tools' facts that a call started and was answered, with its verdict and bytes; the session's facts for delegated calls; the run's facts for its own calls and their returns, a child's usage included; each with the provider's id for the call |
| `job.ended` | the session's fact, as it closes, that a job ended with no answer reporting it, with its end, its output's unread bytes and the provider's id for the call that started it |
| `check.started`, `.completed` | the run's facts that checks started, with their deadline, and finished |
| `message.received`, `.read`, `.unread` | the run's facts that a message was admitted or refused, each told turn's fence, and the answer's final fence |
| `budget` | the run's facts of its budget, at each charge and at the wind-down |
| `notice` | the domain's notices of accounts; the session's dropped reasoning item; the service's measured sizes against their limits; the shell's containment and aborted teardown; a host kit's evidence of its agent's exit |
| `session.ended` | the shell: the exit code, the teardown's span and whether it aborted, each sink's loss, the counts emitted, and the kernel's resource usage of the process and its reaped children |

What the domain owes these records is domain/run.md, section 11's: no
record is a guess from text, and none is derived from a channel's
projection.

## 5. Delivery and capture

### 5.1 Authoritative and counted

- **Authoritative:** `session.started`, `run.completed` and
  `session.ended`. The sink reserves their room before the run starts, so
  they are written under either policy. `run.completed` is built from the
  answer the service sends, not from a fact, and `session.ended` from the
  shell's measurements.
- **Counted:** every other record. The sink delivers it or counts it as
  lost; `session.ended` says how many, and `emitted` says how many of
  each type were made, so a reader compares.
- **Lossless before the sinks.** Inside the process nothing is dropped:
  the domain's facts are a step's output, with room reserved
  (agent.md, section 5).

### 5.2 A sink's policy

- **`complete`,** the default: every record is written, in order. When
  the destination is behind, the stream's output cap holds the service
  back, and the run's next step waits for room, as any backpressure does
  (programming-model.md, section 7).
- **`best_effort`:** a counted record that finds its destination behind
  is dropped and counted; the run never waits for it.
- **Abandoned.** A destination that stays behind past io's write deadline,
  or whose write fails (a full disk, a closed pipe), is given up: the
  sink writes no more counted records to it, counts every later one as
  lost, and says so on standard error, naming the destination and why
  (agent.md, section 6). The run goes on (revised 2026-10-20: a failed
  write ended the destination's records, its authoritative ones too).
- **An abandoned destination gets nothing more** (revised 2026-10-20:
  it was to get its end). io ends a stream at its first failed write,
  and nothing follows but `Closed` (skein's `io.md`, 3.3), so the end
  cannot be tried there. The line on standard error names the
  destination and why, in io's terms: it stayed behind past the write
  deadline (the sink tells this by its own clock), its reader went
  away, or a write failed. Every shell prints it, the one that runs the
  agent in its own process and the one that runs it in another, once
  per destination. The run's exit and its last line on standard error
  still say how it ended. A teardown that aborts the destinations gives
  none of them up: it is the end, not a failure (agent.md, section 6).
- **One destination, one sink.** With standard output and a trace file,
  each has its own policy state and its own count, and each gets its own
  `session.ended`.
- **A front end in the process is not a destination.** It asks the sink
  for the records it shows (`text.delta`, a tool's records, a response's
  usage), typed, through a bounded queue, whatever the destinations'
  capture. It is best effort: what it cannot hold is dropped and counted,
  and it never holds the run back. Nothing it takes is serialized or read
  back from a destination (`docs/design/shell.md`, 4.1).

### 5.3 The capture policy

| Policy | What it adds |
|---|---|
| `none` | nothing: names, counts, sizes, times and classifications only |
| `calls` | each call's input (`tool.started`); a failure's `detail`. Tools' inputs, but no prompt or completion |
| `everything` | also each request's new part, each completion, live text, each result as the model read it, messages' label and text, and the result's text and fields |

- **Capture removes fields, never changes shape.** A field the policy
  excludes is absent; every other field is the same.
- **Never captured:** credentials and grants, which never reach the domain
  (channel.md, section 6); a provider's opaque blocks, of which only the
  size is told.
- **Bounded.** Each content field is within the limit of what it came
  from, so the largest record follows from the limits and the policy
  (limits.md, section 3.7).

## 6. Version

The vocabulary is pre-release, like every format (README.md, section 8).

- **One version.** This is version 4. Any change a reader would notice is
  a new version: a record type or a field added, renamed or removed, a
  field's type, unit or meaning changed, a value added to a field whose
  values are listed, an effective limit's name or unit (limits.md,
  section 9). Nothing is translated, and no older version is read.
  Version 2 added jobs: the `running` and `stopped` verdicts, a call's
  `job`, and `job.ended` (3.6). Version 3 adds `run.compaction_material`
  among the effective limits `session.started` carries (3.2), from which
  a reader finds each conversation's line (limits.md, 3.9). Version 4
  adds accounts and programs: the model's `account` and `backend`
  (3.10), `run.started`'s `withheld` (3.3), the notices
  `credential_unauthorized`, `account_limited` and `account_refused` in
  place of `credential_rejected` and `account_exhausted` (3.9), the
  completion failures `refused` and `unsupported` with their evidence,
  and a model's answer failure for an account, its reset as `reset_ms`
  in place of `retry_after_ms`, and for a program (3.10).
- **`v` is on every record,** so a line read alone says its version. A
  stream holds one version, which `session.started` gives.
- **A reader refuses another version.** `smith-events`' reader reads its
  own version and refuses a stream, or a line, of any other, naming it.
  Within its version, as a matter of robustness and no promise to any
  writer, a reader skips a record type or a field it does not know, and
  reads a listed field's unknown value as unknown rather than failing.
- **Units are in the names:** `_ms` for milliseconds, `_bytes` or `bytes`
  for bytes, `_tokens` for tokens; `spend` and `spent` are in the host's
  unit, and the rest are counts. The keys of `limits` are limits.md's
  names, each in the unit limits.md gives it. Integers are written exactly; a reader
  that holds numbers as doubles must not rely on values above 2^53.
- **Deterministic encoding.** Fields go in the order this document lists
  them, after `v`, `type` and `t_ms`; no whitespace; one record per line,
  ending in a newline. The same values give the same bytes.

## 7. What the domain is owed, and what skein owes

- **The domain is owed** a stream that decides nothing: no record changes
  what the run does, and a stream held back only slows it.
- **The domain owes** facts that carry what the records need: the time
  each happened, the provider's id for each call, a child's effective
  share, its clamps and its usage, every message's terminal, the
  compactions, each job's end that no answer reported, and the budget
  (domain/run.md, section 11;
  domain/session.md).
- **skein owes:**
  - its JSON writer and reader (skein's `json.md`);
  - an append stream on a file, and standard output adopted as a stream,
    with write deadlines (skein's `io.md`, 5.1);
  - the kernel's resource usage of the process and its children, read by
    the shell through io's usage request after the last child has closed
    and before the stream closes (skein's `io.md`, section 6.1;
    `shell.md`, section 13).

## 8. The world

- **The codec,** in `tests/codecs` (`docs/design/testing.md`):
  - a golden line for every record type, every value of every listed
    field, and each capture policy;
  - every golden read back to the values that wrote it;
  - each record at its largest, within its bound, measured against the
    counting allocator.
- **Drift.** The goldens are committed. A change that alters one fails
  until the schema, code, golden and this document change together.
  Before the first release the version stays fixed (skein's `codec.md`,
  section 5); a reader test refuses a golden line of another version.
- **Conformance,** in the agent's simulated world (agent.md, section 8)
  and the local host's (hosts.md, section 7). The referee reads the
  stream through `smith-events`' reader, as a consumer does, and checks:
  - `session.started` first and `session.ended` last;
  - one `run.completed` per run, equal to the host's answer;
  - one `.completed` per `.started` for responses, tools and checks, a
    tool's by its call id, and one `.closed` per `conversation.opened`;
  - one terminal per job: for each job that a `shell` call's
    `tool.completed` names `running`, exactly one later record reports
    its end, a `tool.completed` or a `job.ended`, before its
    conversation's `conversation.closed`;
  - one terminal per message;
  - each run's `spent` and `usage` equal to the sums over its responses;
  - under `complete`, no loss, and `emitted` equal to the records read;
    under `best_effort`, the difference equal to `loss`;
  - under `none`, no content field and no `text.delta` unless a front
    end asked; under `everything`, each request rebuilt from the stream
    equal to what the fake provider received, and each response's
    `text.delta` pieces joined equal to its text blocks;
  - three conversations that start calls in one pass, each attempt with
    its own `response.started`;
  - a run on a program backend against the fake program (programs.md,
    section 8): a response per completion, a program's retry as a failed
    attempt, `withheld` naming `sub_agents`, and each notice and answer
    failure of programs.md, 5.6;
  - a destination slowed under `complete`: nothing lost and the answer
    unchanged.
- **End to end:** `smith exec --json` against the fake provider: the
  stream reads whole, and the exit code agrees with `run.completed`
  (`docs/design/shell.md`).
- **Its consumers** read through the same reader: the live tests, and
  the benchmarks' harness (`docs/design/benchmarks.md`).

## 9. From temper

- **Kept:** the agent's log for operators on standard error (agent.md,
  section 9).
- **New:** the event vocabulary.

## 10. Open questions

- **A published schema:** a JSON Schema of the records for consumers
  outside Rust, generated from `smith-events`' records.
- **A host's stream** beyond the product's: whether a host other than the
  local host (temper's worker, say) writes one.
- **Traces kept:** rotation, and how long trace files are kept.
- **A destination's end after a failed write** (added 2026-10-20): a
  trace given up for a full disk lacks `run.completed` and
  `session.ended` even when room comes back. Either the shell keeps a
  spare descriptor of each destination, adopted for the end records
  alone (each tried once under the write deadline, after a newline if a
  write was torn); or io's write stream survives a failed write and
  takes later sends (a change to skein's `io.md`, 3.3). io's fault could
  also carry the kernel's reason (a full disk, a read-only file system),
  which it does not today.
