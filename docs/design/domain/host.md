# Hosts

Provisional, 2026-10-05, revised 2026-10-20. The other side of a run:
what a host owes an agent and is owed, the channel between them, and the
domains smith ships for hosts: `smith-host-domain`, which a host embeds
to start and supervise agent processes; `smith-inline-agent`, its twin,
which runs agents in the host's own process; and `smith-local-domain`,
the host the `smith` binary runs. What is still open is listed in
section 12.

## 1. In one page

- **A host starts runs and serves them.** It gives each a start (a
  charter, a workspace, a transcript, the messages that triggered it,
  credentials), relays messages, answers host tools and deliveries, keeps
  the turns it is told, and hears the answer. temper's worker is a host;
  so is smith's local host, and so is a test's world.
- **One contract, three forms.** The same vocabulary crosses as domain
  entities (section 2), as frames over a channel (section 3), or as
  calls between domains in one process (section 9).
- **Two kinds of agent, one vocabulary.** An agent process per run,
  supervised by `smith-host-domain`, whose process tree is the run's
  containment (section 4); or the inline agent, smith's domains in the
  host's own process (section 9). A host's root translates either with
  the same functions.
- **Every message ends once.** A message a host sends a run is read,
  refused or unread. The answer names the last message read, and what
  becomes of an unread one is the host's policy (sections 2 and 5).
- **A host keeps its policy.** Which runs exist, what a delivery means,
  where transcripts live and what follows an answer are the host's
  (section 5); smith supervises, relays and enforces the channel's rules.
- **Standard features stand alone.** The local host runs a run in a
  directory, with a person at a terminal or a prompt given once,
  transcripts in files and changes committed in place, its agent inline,
  or spawned for a run on a program (section 8).

## 2. The contract

What a host and a run say to each other, in order.

Down, from the host:

- **the start** first (run.md, 3.2): the charter, the workspace, the
  transcript and the calls answered after it, the messages that
  triggered the run, credential grants, an activation number, the
  run's identity, and, if the host wants its words as they come, the ask
  for live content: one boolean for each kind (`text`, `reasoning`,
  `input`; protocol/channel.md, section 3). The transcript is the current window's turns only:
  those from the last turn that opens a window, as each told turn says
  (section 6). The identity is opaque, of the size the vocabulary seals
  (protocol/limits.md, section 2.4); the host keeps it the same across
  the run's starts, resumes included, or gives none and the run mints
  one. Each conversation of the run carries it with a thread number of
  its own, main's being zero (run.md, section 3.2);
- **messages,** named, in order (run.md, section 6);
- **answers** to the run's calls: host tools' (text, as a result or an
  error), deliveries' (run.md, 8.2), busy, unavailable, withdrawn, too
  large;
- **credential grants,** refreshed (section 7);
- **acknowledgements** of turns, when the host keeps them (section 6);
- **a cancel,** at most one.

Up, from the run:

- **admitted,** or a refusal;
- **messages refused,** each typed: too large as the run reads it, the
  inbox full, a name in use, or the run ending;
- **calls,** each named by the run and answered once, also once the run
  has withdrawn it; **withdrawals;**
- **turns,** numbered from one, consecutive, each with its cumulative
  spend, the last message read, and whether it opens a window: a window
  turn, or a conversation's first (session.md, section 3);
- **facts:** lossless inside the agent's process, where the event stream
  keeps them all by default; on the channel, projected for liveness, and
  dropped and counted as not projected when there is no room, never
  holding the run (run.md, section 11);
- **live content,** at the ask: pieces of what the run's main
  conversation streams before a turn commits, each with its turn, retry,
  kind and offset (protocol/channel.md, section 4). It is not a fact and
  never enters the host's domain: the domain stays content-free, and the
  host's half of the protocol (`smith-host-protocol`) decodes each piece
  and hands it, naming the run, to the host's owner, beside the domain.
  It is best effort and may show a gap, and a piece never follows its
  turn's record. In process there is no channel: the LLM
  component the host's root owns reads the ask from the host and hands
  the deltas to the host's owner directly (section 9.1);
- **long operations:** the run's checks, each LLM completion, each
  command until it ends or is answered `running`, and each `job` call's
  wait (tools.md, section 5), each with its span, bounded by the limits,
  and its end (section 4);
- **waiting,** with the last message read, if any: a run awaiting its
  first message has read none;
- **notices** about accounts: a credential unauthorized; an account
  limited, with how long until it may be used again when its provider
  said; or an account refused, with the provider's code (section 7);
- **the answer,** its last word, with its turn count, its spend and the
  last message read, the final fence (run.md, section 10).

Messages:

- **Each message ends once,** at one of three terminals:
  - **read:** a turn's last message read names it or a later one.
    Messages are offered to the LLM in the order they were sent, so one
    name, the fence, covers every message before it;
  - **refused** at admission: too large as the run reads it, the run's
    inbox full, a name already in use, or the run ending;
  - **unread:** the run answered without reading it. The answer's fence
    says which; a message is never read after the answer.
- **A start's messages** enter the run's inbox at admission, before any
  message sent after the start, and the run opens on them (run.md, 3.4).
  A host that has the first message before it starts a run carries it
  so; one that starts a run before it has a message relays it later.

What a host owes:

- **Each call name is decided once.** A call asked again with the same
  name gets the answer the first had, from the host's record, never a
  second decision (run.md, 5.2). A host that cannot keep that promise
  (one without a durable record) must offer no host tool that writes.
  Names stay distinct across a run's starts: each start has a new
  activation number, never reused, which every name carries.
- **A delivery is never abandoned.** One in flight runs to its end, which
  its deadline bounds, and is answered with what it did, so one that
  landed is reported landed.
- **Nothing written after a stop.** The host touches the workspace only
  once the run and everything it started are gone, but for the delivery
  it asked for.

What breaks the channel's rules, an agent failure: a call name reused
while in flight, a turn out of order, a spend that falls, a fence that
goes back or names a message never sent, the end of a long operation
never begun, a count in the answer that is not the turns told, anything
after the last word, a payload beyond the limits, or a message that does
not decode.

## 3. The channel

- **Framed,** one channel per run, over the streams its host makes: a
  spawned agent's standard input and output, or a connection. Both halves
  are smith's (`smith-protocol-channel`, `smith-host-protocol`), and a host
  owns its half as a component of its protocol layer
  (`protocol/README.md`).
- **Versioned.** Each side speaks one version and says it in its
  opening; with a peer of another, the run ends before its start is
  read. The charter and transcripts carry versions of their own. Every
  format is pre-release: one version at a time, and none read but its
  own (protocol/README.md, section 8).
- **Payloads are smith's vocabulary, as values:** a charter, a
  workspace, a transcript and turns, calls and their answers, results.
  A host's domain holds them as the vocabulary's typed values and never
  encodes or decodes them, since a domain never parses (skein's
  `programming-model.md`, section 4). Bytes belong to protocol layers:
  `smith-host-protocol` encodes a spawned agent's start and decodes what
  it tells (protocol/hosts.md, section 2), and a host's store encodes the
  turns it keeps (`smith-transcript`). A host that prepared a charter
  elsewhere (temper's engine) decodes it in its own protocol layer as it
  reads it, like any input, and adds the workspace it prepared. Only a
  host tool's input stays the bytes the LLM wrote, for the host's own
  protocol layer to read.
- **Live content bypasses the domain.** The host's half decodes a piece
  as it arrives and gives it to its owner, the host's root, with the
  run's name. `smith-host-domain` is not told, holds no content, and
  needs none to supervise: a piece counts as progress for the watchdog
  like any record the half reads (protocol/channel.md, section 7).
- **Bounded.** Every payload has a limit, sealed by its value
  constructor; a host's limits may be smaller, never larger.

## 4. Supervising an agent process

`smith-host-domain` is the child a host embeds to run agents. It knows
agent processes and the channel's rules, and nothing of the host's
policy.

- **Spawned within a deadline,** in a contained process tree, with an
  environment holding no credentials: what the run's LLM calls need comes
  as grants on the channel. A run on a program source has that source's
  configuration directory in its view, and no other login
  (protocol/programs.md, 4.2). A run whose agent could not be started
  fails as such.
- **Gone only once gone:** its process exited, its tree empty, which io
  proves, and its channel read to the end, so what it said before it went
  is heard.
- **A watchdog on progress.** Every message from the run counts as
  progress; silence past the no-progress deadline stops the run. The
  clock pauses while the run waits for a message, having read every one
  sent to it; while a call waits for its answer, which the run's own
  deadline for it bounds; and while a turn waits for room to be sent
  (section 6). It stretches to the span of the long operations the run
  reports (its checks, its LLM completions, its commands, its `job`
  calls' waits), each bounded by the run's own deadlines, until the run
  says each is done. A job that runs on between calls is no long
  operation: the run reports the calls that read it. What
  silence remains is the run's own steps and io's short operations, and
  the no-progress deadline exceeds the longest of their deadlines
  (protocol/limits.md, `watchdog-over-silence`). A separate bound covers
  wall time.
- **Cancel, then kill.** A cancel goes down the channel first, and the
  run winds down on its own (run.md, section 10); past a grace the
  process tree is terminated, then killed. Wall time ends the same polite
  way; a broken rule, no progress, or a channel hung up while the run is
  live terminate the tree at once.
- **An answered agent exits on its own.** After its answer, the agent
  closes everything it owns and exits once each has settled; only io's
  close deadlines bound that, and they make its teardown bound
  (protocol/agent.md, section 6). The host's exit grace is derived from
  that bound and exceeds it, as a supervisor's does (skein's `shell.md`,
  section 13; protocol/limits.md, `grace-over-teardown`), so an agent
  that keeps its contract is never signalled. Past the grace, its tree
  is terminated, then killed.
- **How the agent ended** is evidence, not a guess: its end carries
  `exit`, the code it exited with or the signal that ended it, and
  `forced`, whether the kit had to end it (no, terminated or killed). A
  forced exit after the run answered, whatever its answer, is a warning
  for operators, never the run's failure or a change to its outcome.
- **Messages' terminals.** The kit refuses, typed, at its entrance a
  message the run could not take: too large as the run reads it (its
  label, a colon and a space, then its text), more unread messages than
  the run's inbox holds, a name in use, or a run that is ending. It
  checks with the run's own arithmetic, so the run's refusals, which it
  hands up the same way, are rare. It follows the fence in turns, waits
  and the answer. When the answer or the agent's end is known, it tells
  its parent once which messages it relayed were never read.
- **What it hands its parent:** the run's calls, turns, waiting, notices
  and answer, in order; the unread names, once; the agent's failures,
  typed (not started, exited without answering, broke the rules, said too
  much, no progress, wall time), each with bounded detail for operators,
  such as its error output's tail, never shown to an LLM; and its end,
  with the exit evidence.

## 5. What a host decides

- **Which runs, with which charters,** and whether a run resumes;
- **the workspace:** whether there is one, which directories, from
  where, which writable, and preparing them, merges in progress
  included;
- **its host tools:** their schemas, their decisions, their answers'
  words, their durable records;
- **deliveries:** what making a tree durable means, and when a delivery
  is stale;
- **messages:** which reach a run, rendered how, and which a start
  carries;
- **unread messages:** what becomes of those a run answered without
  reading: carried into a later start, shown to their sender, returned
  to it, or dropped. The kit names them once and decides nothing about
  them;
- **transcripts:** where they are kept, how long, and what a fresh run's
  brief carries instead;
- **what follows an answer:** retrying, holding, waking a parked run.

## 6. Turns held for the host

- **Kept until acknowledged,** when the host keeps transcripts durably,
  as temper does: a turn is held until the host says it is safe, and a run
  may have a bounded number of turns, and bytes, unacknowledged. Past it,
  the host stops reading the run's channel, and the run waits until there
  is room, its watchdog paused.
- **Before the answer.** The answer says how many turns the run took and
  is sent after them.
- **Held by the kit.** `smith-host-domain` holds each turn for its parent
  until the parent acknowledges it, as temper's worker holds them across
  its own channel's losses; a host that keeps no transcript acknowledges
  at once.
- **A turn the host cannot keep is never acknowledged.** An
  acknowledgement means durable (channel.md, section 7). A host whose
  store fails therefore drops the run at once (9.2): nothing the run says
  after that could be kept, and its window would never open again.
  - **The drop releases the held turns.** It releases every turn held for
    the host, unacknowledged. The agent is gone once what the run started
    has settled, and does not wait for acknowledgements that can no
    longer come.
  - **The host owns the failure.** It says which turn it could not keep
    and why. It ends its run as a host failure, even after an answer,
    which then stands but was not kept whole.
  - **A later resume starts from what was kept.** A host that keeps its
    transcripts durably resumes from the last turn it kept.
- **Only the current window goes back.** Each told turn says whether it
  opens a window (section 2), so a host, or its store with the turns
  encoded, knows where the current window starts from that flag alone. Resuming a run,
  it sends the turns from the last that opens a window. Earlier windows'
  turns it may keep for people to read, and never sends
  (protocol/transcript.md, section 3).

## 7. Credentials

- **Lent by the host:** an agent under a host gets each credential its
  endpoints need as a grant on the channel, or from the host's protocol
  layer in one process (section 9), named by account and generation,
  refreshed before it lapses; the agent's protocol layer
  holds it, and the domain names only the grant and the form its value
  takes. A grant's value is a credential, a bearer or a key, or a
  program source's name (protocol/channel.md, section 6), and it decides
  the backend the run runs on (protocol/programs.md, section 3). The
  host's domain knows each grant's form, the credential's scheme or the
  source's name, never the credential, so that a host in one process can
  refuse what it cannot run (section 9.1). A host lends as a key only a
  credential it never renews.
- **Notices go up for the host to act on:** a credential unauthorized,
  an account limited, an account refused. The run that met one ends
  failed for its account, except an unauthorized bearer, which a
  refreshed grant may answer. An actual unauthorized response on a key
  ends its run at once unless a newer generation is already held
  (session.md, section 4). A host never moves
  a run to another account; which account its next run takes is the
  host's.
- **The local host's sources.** Each account the local host lends has
  the source its settings give it (`docs/design/shell.md`, 5.2):
  - **its own sign-in** (`sign_in`): skein's OAuth driver with the
    registration of the provider's preset (skein's `oauth.md`, 6.2), a
    redirect on a port drawn for each sign-in unless the settings fix
    one (6.5), and the subject from the ID token. The record is kept
    privately in the credentials directory, under its lock, and read
    again before each refresh, so two processes never refresh one
    record twice (6.4). The access token is lent as a bearer, refreshed
    before it lapses; signing out revokes and removes the record;
  - **a key** (`key`): a key the person gave, kept privately in the
    credentials directory, lent as a key, never refreshed;
  - **the environment** (`env`): a named variable, for unattended use,
    lent as a key as it is and never refreshed;
  - **a program** (`program`): a program source, signed in by the
    program's own flow into its configuration directory
    (protocol/programs.md, section 4). The host keeps nothing of the
    login: it checks the source's status (protocol/programs.md,
    section 3), has the program's own sign-out run when the person asks
    (protocol/hosts.md, section 2), and lends the source's name;
  - **none** (`none`): for an endpoint that takes no credential, whose
    charter's LLM names no account (protocol/charter.md, section 2). It
    lends no grant, and that LLM's calls carry no credential
    (protocol/channel.md, section 6).

  Every other source lends its grants the same way, so the agent has
  one path for credentials. Another program's login is never read.

## 8. The local host

`smith-local-domain` is a host for one person on one machine, and with
the inline agent it is the `smith` product:

- **A workspace in place,** or none: the directories named on the
  command line or in its settings, writable as configured.
- **Interactive or headless:**
  - **a chat:** the person's lines are messages; a chat's run waits for
    them, and parks past its waiting time, to resume when they return;
  - **headless:** one run from a prompt carried in its start. The host
    relays nothing more, and ends when the run answers, with a status
    for each class of outcome: accepted; the run failed; its budget ran
    out; a credential the person must fix; usage or settings refused;
    needing input; interrupted. Its charter grants `wait` with a waiting
    time of zero, so a run that would wait for a person parks at once,
    and needs input (run.md, 3.1). It edits the workspace in
    place and commits nothing, unless asked to deliver, which keeps the
    change contract.
    The statuses' numbers are the shell's (`docs/design/shell.md`).
- **The person's lines:**
  - **A line starts a run** when none is live, carried in its start, so
    the run opens on it (run.md, 3.4). Lines entered while a run is live
    go to it as messages, each as it is entered, labelled `person`
    whichever kind of agent runs.
  - **Held until read.** The host keeps each line it relayed until a
    fence covers it, at most as many as the run's inbox holds; past that,
    it reads no more of the person's input until a line is read. A line
    refused, by the kit or the run, is shown as not delivered, with why.
  - **Unread lines, by outcome.** After an accepted or parked answer, the
    unread lines open the next activation, in order, under their names.
    After a cancel, a failure or a refusal, they are shown as not
    delivered, and dropped. An interrupt never starts a run by itself.
  - **"Waiting" is shown only when every line relayed is read.** A wait
    whose fence is behind the last line sent is a crossing, not
    idleness.
  - **Closing input drains:** every line received is relayed, carried or
    shown as not delivered before the host exits.
  - **Late events are expected.** A turn's or the state's save that
    finishes after the answer, and an agent's end that follows it, are
    handled in the answered state, never assumed away.
- **Failures in words.** The person sees what went wrong, whichever kind
  of agent runs: a run's typed failure (the model, with its provider's
  class; a limited account, and when it may be used again; a refused
  account, with the provider's code; a program source whose login was
  lost, with the command to run on this host; a program that could not
  run, naming it; the budget,
  naming what ran out and how much of its bound was used; policy;
  cancelled; stale; a
  transcript it could not resume), an agent's failure with its bounded
  detail, a refused start saying what, and a credential that needs the
  person. Each kind has its own words, matched exhaustively, so a new
  kind has none until it is given some, and none is shown as a fixed
  "run failed". These lines are the operator's, and never enter turns or
  prompts. A forced exit after the run answered, whatever its answer, is
  shown as a warning beside the answer's words, which it leaves as they
  are.
- **Charters from configuration:** instructions, models, prices, a
  budget, conventions, and a result contract (a report, by default, or a
  change), with the shell's presets as defaults (`docs/design/shell.md`).
  A subscription's models carry notional prices, so spend still bounds a
  run; a budget that could not pay for one completion is refused before
  any run starts.
- **Program sources checked:** each account whose source is a program
  is checked as the local host starts, and again before each run on it
  (protocol/programs.md, section 3). One not usable is shown with its
  state and the command to run, and starts no run.
- **The run's identity:** minted once for each chat, kept with its
  transcript and given in every start, so the chat keeps its provider's
  cache across activations and resumes.
- **Transcripts in files,** in smith's state directory, so a chat resumes
  across invocations. The Store first checks its recorded layouts
  (protocol/hosts.md, 5.3). It keeps one checked bit for the selected root,
  never learns an older layout and never translates or wipes old records.
  An unrecorded chat, a different store layout or a different transcript
  layout is `StoreFailure::OlderBuild`, with the typed `Older` reason.
  It follows StoreFailed, EndCause::Host and RunFailed, including before
  a run exists or on /resume: show the named chat's words and close with
  exit 1. Saves require a checked root; close settles outstanding work.
- **Delivery in place:** a commit in each writable repository, with the
  result's fields as its message, nothing pushed unless configured; a
  plain directory's files kept as they are, changed or not by a bounded
  snapshot, and a tree past its bounds counted as changed
  (protocol/hosts.md, section 5.5).
- **Host tools:** none of its own at first; tools from MCP servers it is
  configured with, once MCP is a tool source.

It runs its agent inline, in its own process (section 9), except a run
whose account's source is a program, which the inline agent cannot run:
its agent is spawned (section 4; protocol/programs.md, section 2).
Spawning every run stays a capability of smith's libraries, tested by
its worlds, for other hosts and for a later isolated mode (section
12).

## 9. One process

A host may run smith's domain in its own process, beside its own
domains: the contract crosses as entities, translated by small total
functions, with no pipe and no frames. It does so through the inline
agent, `smith-inline-agent`, the twin of `smith-host-domain`. The local
host runs its agent so (section 8), and jig's engine its agents; temper
may so run its engine, a worker and agents in one process.

### 9.1 The inline agent

A child a host embeds, beside or instead of `smith-host-domain`:

- **The same vocabulary, in the same order.** Its parent face is
  `smith-host-domain`'s, the same types: down, a start, messages,
  answers to calls, acknowledgements, grants and a stop; up, message
  refusals, calls and withdrawals, turns, waiting, long operations,
  notices, the unread names, one answer, and the agent's failures and
  end. A host's root
  translates both kinds with the same functions.
- **A run is a composed `smith-domain`,** one per run in flight, on the
  slots its host gives it. Everything crosses as the vocabulary's typed
  values: the charter and the transcript go down to the run as values,
  and turns come up as values. The inline agent encodes and decodes
  nothing (section 3).
- **Messages as the kit takes them** (section 4): rendered and checked
  with the run's own arithmetic, refused typed at its entrance, followed
  to each fence, and the unread named once.
- **Requests below the run go up.** LLM completions and, when the run
  has a workspace, the machine's operations (files, searches, commands,
  checks, guides) leave as requests that the host's root routes to its
  protocol layer (`smith-protocol-llm`, `smith-protocol-machine`), and
  their terminals come back to the run that asked. The inline agent
  knows whose each request is, and answers none itself.
- **The workspace.** The start's directories go to the run. Its checks
  are reported up as long operations, and its deliveries go up as calls,
  as `smith-host-domain` relays them, for its host to answer. A run is
  gone only once every operation it started on the workspace has
  settled, so the host may then deliver, save or prepare (section 2).
- **Credentials:** the root's protocol layer lends the LLM component its
  grants directly; the inline agent passes the run only their names, as
  the channel would.
- **The direct backend only.** A run in the host's process has no child
  of its own, so the inline agent refuses as invalid a start whose
  grant names a program source (protocol/programs.md, section 2).
- **Facts and content** are drained from each run every pass by the
  inline agent's owner, independently of any channel, into the host's
  event stream (protocol/events.md), under the capture policy the host
  chose.
- **No watchdog on progress.** A composed domain does not hang as a
  process can: its waits are bounded by its own deadlines and by those
  of the requests it hands up. A bound on wall time stays.
- **Bounded:** each run's worst case is `smith-domain`'s, with its
  routing; the inline agent's is the sum over its slots, which its host
  gives it.

### 9.2 Stopping: cancel, grace, drop

1. **Cancel.** A stop is the run's cancel: the run winds down and
   answers (run.md, section 10). Wall time ends a run the same way.
2. **Grace.** The run has its host's cancel grace to answer.
3. **Drop.** Past the grace, or at once when the host asks, the run is
   dropped: its domain hears nothing more, its relayed calls are
   withdrawn, and the root is asked to end what the run holds below it,
   its LLM completions cancelled and its file operations and the process
   trees of its commands and checks stopped. The turns held for the host
   are released unacknowledged (section 6). The agent is gone only once
   each operation has its terminal, the twin of "gone only once gone"
   (section 4), and its failure is that it went without answering. A
   drop after an answer, from a host whose store failed while the run
   settled, releases the held turns the same way, and the answer stands.

- **After an answer,** the same settlement runs without a grace: what
  the run still held below it is cancelled, its relayed calls are
  withdrawn, and the agent is gone once each has settled.
- **A drop is the host's to ask for.** The local host's second interrupt
  asks for one (protocol/hosts.md, 5.1).
- **A panic is fail-stop** for the host's whole process, as for any
  domain. A host that keeps its transcripts durably resumes from them.

### 9.3 What stays and what goes

- **What stays:** the domain's rules; the calls' names; turns and their
  acknowledgements; messages' terminals and the unread report; budgets;
  the tools' confinement; commands and checks in contained process
  trees; credentials as grants; facts, content and the event stream;
  typed failures; a bound on wall time.
- **What goes:**
  - **the agent process as the run's containment boundary.** Stopping a
    run is its cancel, then its drop, which ends its process trees; a
    run cannot be killed whole. That suits tests, development and one
    person's machine, not a host that runs agents it does not trust with
    its own process;
  - **the channel and its frames,** and with them the rules only a peer
    across a channel can break;
  - **what only a process has:** the progress watchdog, the exit evidence
    and standard error's tail.

### 9.4 Its shape

- **Two crates, one face.** The inline agent is a crate beside
  `smith-host-domain`, not a mode of it: supervising processes and
  composing domains share only their vocabulary, and each has its own
  worst case and world. `smith-host-domain` keeps its parent face apart
  from the face its process io speaks, and the inline agent speaks the
  parent face's own types, so a root's translations are the same by
  construction.
- **Values, never bytes.** The parent face carries smith's vocabulary as
  typed values for both kinds of agent. So the two stay interchangeable
  and no domain parses: the spawned kind's bytes are
  `smith-host-protocol`'s, below `smith-host-domain` (skein's
  `programming-model.md`, section 4).
- **smith's own.** `smith-inline-agent` is built in smith, with jig's
  `jig-inline-agent` as a reference for its shape, not as code to keep in
  step: the typed face, the workspace routing and the stop ladder (9.2)
  are smith's design. jig is not changed by this work; whether it later
  takes smith's is jig's (section 11).
- **A wall bound, and no progress watchdog** (9.1).

## 10. The world

`smith-host-domain`'s world runs it against a scripted agent (progress,
calls, turns, waits, long operations, parks and answers, but also hangs,
crashes, broken rules and ignored cancels) and the machine's process
trees. Its stories add:

- messages sent before, across and after the answer, each ending once;
- agents that exit with success, exit with failure, ignore the
  termination signal, and exit on it after their answer, each end
  observed once with its evidence;
- an answered agent that exits by itself within its grace, never
  signalled;
- a completion that outlasts the no-progress deadline, reported as a
  long operation, and not stopped;
- a message one byte over the run's bound as the run reads it, refused
  typed;
- a run awaiting its first message, its waiting naming no fence;
- a run resumed after a compaction, sent only the turns from the last
  that opens a window.

The same world runs the inline agent: smith's real domain over skein's
fake LLM and the fake machine, whose runs answer, wait, park and resume,
are cancelled mid-turn, and ignore a cancel past the grace, to be dropped
with every completion, file operation and command tree below them
settled before they are gone.

Its referee: one answer per run; no slot released before the tree is
empty, or, inline, before everything below the run has settled; every
call answered once; every message relayed ends once, and the unread
names are told once; the watchdog paused exactly while the run may be
silent; an answered agent signalled only past its exit grace, and its
end's evidence saying so.

The local host's world runs it with the inline agent, a fake LLM
provider, a person scripted at the terminal and directories on a fake
disk. The person reacts to what is shown: it sends lines on notices,
turns and answers, or after some steps, and closes its input anywhere.
Its stories: a chat with no workspace that waits, parks and resumes
across invocations; a change checked and committed in place; a run
cancelled at the terminal; and, for messages, a follow-up as the run
finishes, two lines back to back, lines then the end of input, a line at
the moment of parking, an interrupt with a line unread, a brief with an
early finish, and a slow store racing the answer. Each failure kind is
shown in its words, a headless run ends with each status, each notice
is shown in its words, a program source not usable at startup starts no
run on it, and a run on a program source is spawned and answers,
against the fake program.

Its referee adds, as safety: a line relayed at most once in an
activation, and again only if it was never read; fences that never go
back; "waiting" shown only when every relayed line is read; no failure's
words in a turn or a prompt. As liveness: by its exit, every line was
read or shown as not delivered, a refused one with why. Its fuzzy sweep
sends one to three
lines at random moments, with store delays and the end of input
anywhere. Simulated time is free, so these fit the focused and fuzzy
budgets.

## 11. From temper

- **The contract and the channel** are temper's channel's agent hop, as
  temper's design changed it (turns, transcripts, messages carried whole,
  relayed calls with names, spend, waiting), with temper's names gone:
  runs are a host's, push is delivery, and the engine's tools are host
  tools.
- **`smith-host-domain`** is temper's worker's agent child domain, as
  temper's `domain/worker.md`, section 6, describes it: one process per
  run, the watchdog, cancel then kill.
- **The inline agent** follows jig's `jig-inline-agent`, the first of its
  kind, in shape, and is smith's own: typed, with a workspace and the
  drop's settlement. smith's APIs change freely for it; jig and temper
  adapt separately, on their own schedule.
- **The local host** is new.

## 12. Open questions

- **A host without a durable record** and host tools that write: refused
  by rule, as section 2 says, or allowed with a warning to the LLM.
- **Containment in one process:** whether commands may still run under a
  per-run cgroup, so cancelling a run in one process stops all it
  started.
- **Several runs per agent process,** for a host with many short runs.
- **A connected agent:** a daemon a host connects to instead of spawning
  (`protocol/README.md`, section 4), supervised without its process.
- **An isolated local agent:** the local host spawning its agent by
  choice, once contained trees give a process more containment than the
  inline agent has.
- **A vocabulary crate:** whether the face `smith-host-domain` and the
  inline agent share moves to a crate of its own, should a host want one
  kind without linking the other.
