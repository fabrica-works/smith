# Turns and transcripts

Provisional, 2026-10-07, revised 2026-10-24. The records of a
conversation kept beyond its process: a turn, as a run tells it, and a
transcript, as a run resumes from it (domain/session.md, section 3). They
are the family `smith-transcript`, versioned on their own, since a host
keeps them as long as it likes and a later agent reads them (README.md,
section 8).

## 1. In one page

- **A turn is one completion:**
  - the messages around it, in order;
  - the provider's blocks, in position;
  - its calls, as the LLM wrote them;
  - their results;
  - what it used and cost.

  A window turn is a compaction's: its messages are the new window's
  opening (domain/session.md, section 8.2).
- **A transcript is the current window's turns, in order:** from the
  conversation's first turn, or from its last window turn. A host keeps
  each turn's bytes as it was told and sends back, in order, those from
  the last turn whose `opens_window` is true to resume. It needs no codec
  of smith's to do so: the channel's turn record carries `opens_window`
  beside the body (channel.md, section 4).
- **The affinity, the account and the backend are kept once,** in the
  transcript's opening turn, so a resumed conversation keeps its
  affinity, and a resume knows whether its window ran as its charter
  now asks (domain/session.md, sections 2 and 8.2).
- **A window is carried over, not refused,** when it ran on another
  endpoint, dialect, account or backend: it is compacted on the
  charter's LLM, and a new window opens there (section 4).
- **Usage counts are optional.** A count the provider did not report is
  absent, not zero.
- **Opaque provider blocks travel verbatim,** tagged with the dialect that
  wrote them, and go back only to the same endpoint and dialect. A
  window carried over sends none of them, and a program backend's turns
  hold none (programs.md, 5.5).
- **Results are kept as the domain holds them.** Text that the domain wrote
  is kept as text. A workspace tool's outcome is kept typed, and rendered
  for the LLM again when the conversation resumes.
- **Names resolved.** A turn means the same outside the session that made
  it: no ticket and no slot number, only the run's call names and the
  provider's ids.
- **Readers beyond the agent:** a host that shows turns, such as temper's
  web, decodes them with `smith-transcript` and renders them as it likes.

## 2. A turn

A versioned record. Every turn starts with its version.

- **kind:** ordinary, or window, a compaction that opened a new window
  (domain/session.md, section 8.2);
- **endpoint:** the name of the endpoint it ran on, as the agent's
  configuration names it, and the dialect of its provider, by its number
  (protocol/agent.md, section 4); for a window turn, those of the window
  it opens, as with its model;
- **model and effort:** for an ordinary turn, the model its completion
  was made with and the effort it asked for, if any; for a window turn,
  the model and effort of the window it opens, the charter's, whatever
  model and effort its compaction ran with. So the last turn always
  names the window's model, which a resume compares with its charter's
  (domain/session.md, 8.2). The compaction's own model, usage and cost
  are its conversation's records in the event stream (events.md, 3.4
  and 3.5), and the window turn's usage and spend stay the compaction's
  (domain/session.md, 8.2);
- **its place in the conversation:** from one, going on across the
  activations that resume it and across windows. The channel numbers turns
  per activation (channel.md, section 4);
- **affinity,** in an opening turn only (section 3): the run tree's
  identity, sixteen opaque bytes, and the conversation's thread, an
  ordinal, main's being zero (domain/session.md, section 2);
- **account and backend,** in an opening turn only (section 3): the
  account the window's completions are paid by, as its charter's LLM
  names it, or none (charter.md, section 2), and the backend they run on, direct
  or a program, named as the agent's configuration names it,
  `claude_code` (programs.md). For a window turn they are the window's
  it opens;
- **usage:** the tokens the provider counted, each optional and absent
  when not reported: input, output, cache reads, cache writes and
  reasoning tokens (domain/session.md, section 6);
- **spend:** the run's spend so far, in the host's unit, sub-agents included
  (domain/session.md, section 6);
- **jobs:** the highest job number the conversation has given so far,
  zero before its first (domain/tools.md, section 5);
- **messages,** oldest first, each from the user or the assistant, each a
  list of blocks:
  - **text,** with the provider's replay, if any;
  - **a refusal,** with the provider's replay, if any;
  - **a call:** the provider's id for it, the tool's name, and its input as
    the LLM wrote it, with the provider's replay, if any;
  - **a cut call:** the provider's id for it, the tool's name, and how many
    bytes of input it had when the completion reached its output limit.
    Its input is not kept, and its result is invalid with the problem *cut
    off* (domain/session.md, section 4);
  - **an oversized call:** the provider's id for it, the tool's name, and
    the size of the input skein counted and never held. Its result is
    invalid with the problem *too large* (domain/session.md, section 5);
  - **a result:** the call's id, and one of:
    - a workspace tool's outcome, typed (domain/tools.md's vocabulary: lines read,
      entries listed, an edit made, a command's exit and output, a
      command answered `running` with its job's number and its output so
      far, a job's output since its last answer with its end or
      `running`);
    - text the domain wrote, as a result or an error: a served tool's, or a
      host tool's answer;
    - invalid, with the problem found in the call (domain/tools.md,
      section 4);
    - not run, with why: outside the tool choice, naming the calls that
      were allowed (protocol/llm.md, section 5); a completion's calls
      after its session ended; a compaction's; or a write or a command
      after a write of its completion that failed, naming that call by
      its position among the completion's calls, from 1, whose tool is
      that call's in the same message (domain/session.md, section 5;
      protocol/llm.md, section 5). A resumed history
      renders it as it rendered when told, so the prefix stays stable;
    - withdrawn;
  - **an opaque block:** the provider's bytes, tagged with its dialect's
    number (protocol/agent.md, section 4).

A call's typed form is not kept. The agent decodes the input again when it
resumes, as it did when the call was made.

A window turn's messages are one user message holding text blocks, in
this order: the conversation's task, the summary preamble and the
summary, each a block of its own, and the messages held back for the
new window. It holds no call. Its usage and spend are
the compaction's.

## 3. A transcript

- **Its turns, in order,** each as its own bytes, from its opening turn:
  the conversation's first turn, or its last window turn. They must share a
  version, an endpoint and a dialect, and be numbered without a gap from
  the opening turn's place. The account and the backend are the opening
  turn's for every turn after it.
- **`opens_window`** is true of a turn that can open a transcript: one
  of kind window, and the conversation's first turn, at place one. A
  told turn carries it, on the channel's turn record and in the run's
  vocabulary to a host in one process (domain/host.md, section 2), so a
  host finds the current window without decoding a turn. A host resuming
  a run sends back only the turns from the last one with `opens_window`.
- **The opening turn carries the affinity, the account and the
  backend,** and no other turn does. Every window turn opens a
  transcript, and so carries them too.
- **Earlier windows are not part of it.** A host may keep their turns for
  people to read, but never sends them back. Once a window turn is told,
  the transcript the next resume needs starts there.
- **Answers the transcript lacks** are not part of it. A call whose turn
  was never told, or one the run withdrew and its host decided later, has
  no open place in the history. The start carries the host's answers to
  them beside the transcript, and the run tells the LLM of them as text in
  its waking prompt (channel.md, section 3).

What a host keeps is therefore the turns' bytes, and nothing it must
assemble.

## 4. Resuming

The codec checks what a schema can:

- bounds;
- tags;
- text;
- one version, endpoint and dialect across the turns;
- numbers without a gap;
- an affinity, an account and a backend on the first turn and on no
  other;
- a first turn that is a window turn or at place one, and no window turn
  after it.

The session checks the rest before a completion starts (domain/session.md,
section 3):

- calls and results paired by their ids;
- no unresolved ticket;
- room for the waking prompt and a completion within its byte limits. A
  history that has that room but fails the window check is compacted
  before its first ordinary completion (domain/session.md, section 8.1).

**A window carried over.** A transcript whose window ran on another
endpoint or dialect than its charter's main LLM, one the agent's
configuration does not name, another account or another backend, or
on a program whose session its host does not hold (programs.md, 5.8),
is not refused. The session compacts it on the charter's LLM before its
first ordinary completion, sending its history without opaque blocks or
provider's replay, which go back only to the dialect that wrote them,
and opens the new window there; its window turn records the new
endpoint, dialect, account and backend (domain/session.md, 8.2).

Each refusal is distinct, and the run fails as transient, saying which
(domain/run.md, section 6):

- another version;
- malformed;
- unresolved;
- too large.

**No job is resumed.** A job does not survive its activation: a
`running` result renders as it was told, and a later `job` call naming
its job is answered as an invalid call (domain/tools.md, section 5). Its
number is not reused: the resumed session's next job takes the number
after the highest its last turn records (section 2), which counts the
jobs of every earlier window and activation.

## 5. Rendering

- **A workspace tool's outcome is rendered by the LLM component**
  (llm.md), live and on resume alike. A later release may render an old
  outcome in other words; what the LLM learns from it is the same.
- **Text the domain wrote** goes back to the LLM as it was written, so a
  served tool's and a host tool's results read the same after a resume.
- **A cut or oversized call** goes back to the provider as a call with an
  empty input, answered by its error, so the history stays replayable
  (llm.md, section 4).
- **The affinity is not text.** The session puts it into every prompt, and
  skein's dialect renders it (domain/session.md, section 2).
- **For people,** a host renders turns itself. smith offers the decoded
  records, not a presentation.

## 6. Versions

- **Version 2 is the first,** continuing the conversation vocabulary whose
  first version stays with temper's legacy run (domain/README.md,
  section 7).
- **Version 3** adds:
  - a turn's kind, and the window turn;
  - the affinity, in a transcript's opening turn;
  - usage counts as optional values, with reasoning tokens;
  - the cut and oversized calls, and the problem *cut off*;
  - blocks per message derived from calls per response (section 7).
- **Version 4** adds jobs: a command answered `running`, a job's read,
  and the highest job number a turn records (sections 2 and 4).
- **Version 5** adds a not-run call's why after a failed write of its
  completion, naming that call by its position (section 2).
- **Version 6 is current.** It adds the opening turn's account and
  backend, and a window turn's endpoint as the window's it opens
  (sections 2 and 3).
- **A reader reads one version,** the current, like every format
  (README.md, section 8). Any other is the transcript refusal *another
  version*, and nothing is translated: a version-2 transcript is
  refused, and its host starts the run afresh from its brief
  (domain/run.md, section 6). A transcript of version 5 or earlier,
  such as a conversation on the withdrawn `codex` dialect keeps, is
  refused the same way, and is never carried over to another
  endpoint.
- **A host may keep turns of several versions** for people to read. A
  reader is handed a transcript in one version, and only one in the
  current version resumes.
- **From the baseline, a host that keeps turns patches them** to the
  current version as it reads them, by smith's patches, so its agent
  and its people read the current version alone (section 12). Before
  it, a turn of another version stays refused.

## 7. Limits

- **A turn** is bounded by the bytes a session may hold (domain/session.md,
  section 6), within the schema's ceilings.
- **A message's blocks** are bounded by a number derived from the declared
  calls per response (protocol/limits.md). It is never below the
  completion's block bound, and leaves room for the text a message holds
  beside calls or results: the messages appended after results, and a
  window's opening.
  The schema's ceiling is the vocabulary's, and the derived bound is
  within it. A configuration that breaks either relationship is refused at
  startup, naming it, so every completion a session accepts can be told.
- **A search's hits** are one piece within an outcome, since they share
  one bound: at most `search_hits` hits within `search_bytes`, any one of
  which may hold nearly all of them (protocol/limits.md, 2.2 and rule 6).
  The piece holds each hit as the codec encodes a hit record, in order.
  The agent decodes them one by one, counting them against its
  `search_hits`, and refuses as malformed a piece whose hits do not end
  at its end. The hits stay typed; only their count and total are the
  piece's.
- **A transcript** is bounded by the resume limit, which is the host's and
  the agent's configuration's: the turns and bytes the agent takes in a
  start. Since a transcript holds one window, the resume limit is at least
  the most a session holds in a window (protocol/limits.md), so every
  transcript a live session leaves can be resumed, however long its
  conversation.
- **A host that keeps turns** bounds what it keeps unacknowledged with the
  channel's window (channel.md, section 7).

## 8. What the domain is owed, and what skein owes

- **The domain is owed** turns translated from `smith-domain-session`'s
  entities as they are told, and back on resume, with their kind and an
  opening turn's affinity, account and backend. Endpoint names are resolved through the agent's
  configuration both ways.
- **skein owes** the codec generator (skein's `codec.md`), and providers'
  replay as bounded bytes, tagged by dialect, with a call cut off at the
  output limit marked as such (skein's `llm.md`).

## 9. The world

- **The codec:**
  - golden bytes for every block and result, each kind of turn, an
    opening turn's affinity, and usage counts absent and zero;
  - each decoder fuzzed;
  - a domain turn encoded and decoded back unchanged;
  - each refusal: an affinity, an account or a backend on a later turn,
    a window turn after the first, a message's blocks over the bound.
- **Resuming,** in the channel's protocol world (channel.md, section 10):
  - from turns kept by a scripted host;
  - from a window turn after several compactions, earlier windows not
    sent;
  - each refusal;
  - a transcript from another dialect, another account, another
    backend, and a program's whose session is gone, each carried over:
    compacted on the charter's LLM with no opaque block sent, and its
    window turn naming the new ones;
  - a version-2 and a version-5 transcript refused as another version;
  - a transcript holding a `running` result, resumed with no job, and a
    `job` call naming that job answered as invalid;
  - `opens_window` on a conversation's first turn and on each window
    turn, and on no other, and a host resuming from the last of them.
- **A large transcript:** at the resume limit, one window at its largest,
  measured against the worst case.

## 10. From temper

- **Kept:** temper's second conversation version: turns told with names
  resolved, versioned, with opaque blocks verbatim and spend in a unit.
- **Gone:** snapshots, and tickets meaningful only inside the session that
  made them.
- **New:** windows, the affinity kept once, and optional usage.

## 11. Open questions

- **A window too large to carry over:** one larger than the charter's
  model's window fails as a compaction's *context too long* does
  (domain/session.md, 8.2). Cutting it first, oldest turns first, waits
  on a host that needs it.
- **A search's hits under a patch:** pieces are copied by a patch, not
  opened (skein's `codec.md`, 11.3), so a patch that changes `Hit` is
  refused until skein opens them (its section 13). Until then, from the
  baseline, a change of `Hit` waits for them.
- **The channel's records a host keeps as bytes:** jig's store keeps a
  settled call's name and a delivery's evidence as the channel encodes
  them (jig's `protocol/store.md`, section 16). The channel is versioned
  whole, so its records carry no version of their own, and no host can
  learn from their bytes which code reads them. Either smith keeps them as
  a versioned family of their own, kept and patched as the turn is, or a
  host keeps them in its own vocabulary, translated as a run's result is.

## 12. Kept, and patched

A host keeps turns beyond a release: jig's store keeps each in its
transcript's frames and reads them back after its engine is upgraded
(jig's `protocol/store.md`, 8.6; temper's `draft/upgrades.md`, 7.2). From
the baseline, that host patches a turn of an older version to the
current one as it reads it, by smith's patches, so a run resumes across
a change of turns and people read old turns in the current version. Every
other reader reads one version (section 6), and its host hands it no
other.

- **The kept families** are the turn's and the local host's store's:
  - **the turn,** `smith-transcript`, its records `Turn`, each a
    transcript's turn as its own bytes (section 3), its version first:
    the set below, which a host's store takes;
  - **the local host's store,** `smith-local-store`
    (`smith-local-protocol`'s `local-store-v1.schema`: its state, its
    turns and its deliveries), read after an upgrade by smith's local
    host alone (protocol/hosts.md, 5.3). It is kept like the turn from
    the baseline, under an owner of its own, `smith-local`, since an
    owner's patches and kept texts are one crate's (skein's `codec.md`,
    12.1); its `Turn.body` already says `holds smith-transcript Turn`, so
    a turn it keeps is patched by the turn's set, its own patches
    pinned after smith's where they need its turns. Its set takes this
    section's shape, written with the local host's reads at the
    baseline; no other host takes it.

  Nothing else of smith's is kept for a later release's reader:
  - **results:** a tool's outcome is part of its turn, kept and patched
    with it. A run's result is its charter's (charter.md, section 3),
    which its host translates into its own records (temper into jig's
    tasks' results) and never keeps as smith's bytes;
  - **the event stream** is read as it is written (events.md, section
    5), and is no host's store;
  - **the channel's records** a host keeps as bytes are open (section
    11).
- **The words owed** (skein's `codec.md`, section 3): a turn's one field
  whose bytes are pieces says so, `hits: bytes 2097152 pieces Hit` in
  `Found`, in place in `transcript-v6.schema`, version 6 unmoved; the
  word moves the family's layout, no byte and no Rust. The local host
  records that layout in each chat's `format` (protocol/hosts.md, 5.3),
  so a local chat made before the word is refused as an older store,
  as every store before the baseline is. Every other
  `bytes` of a turn is opaque: a provider's replay and opaque blocks, a
  call's input, paths, a command's output and a hit's text. A host's own
  schema says it holds the turn: jig's `transcript: bytes 8388608 holds
  smith-transcript Turn`.
- **The set, as data:** `smith-transcript` exports, as `&'static`
  constants, what a host's store takes for an owner's patches, naming no
  host:
  - `OWNER`, `"smith"`, the owner as its patches' headers spell it;
    `FAMILY`, `"smith-transcript"`, as its texts spell it; and `ROOT`,
    `"Turn"`, the declaration a kept turn is, where a machine starts;
  - `patches::PATCHES`, the patches' texts in number order, and
    `patches::kept::smith_transcript::TEXTS`, the kept texts in version
    order, with the patches' `Context` and `written`, in the module
    skein's command generates (skein's `codec.md`, 12.1 and 12.3);
  - `FRESH_WINDOWS`, `"fresh-windows"`, the mark below.

  Before the baseline the module is written by hand (`src/patches.rs`)
  in the generated shape, and is empty: no kept text, no patch, a
  `written` that answers every site `NoSite`. The baseline's run of the
  command (`--baseline`) keeps the current text as the first and
  generates `patches/mod.rs` in its place, under the same name. jig's store takes the constants into its `Smith` slot (jig's
  `protocol/store.md`, 10.3), handed by the host's shell.
- **The mark:** a smith patch whose change an agent would misread on a
  resume, such as a provider's replay it cannot carry or a call whose
  meaning moved, carries `mark fresh-windows` in its header (skein's
  `codec.md`, 11.1). A host closes each live window across it, and the
  next run starts afresh from its brief with the transcript's tail (jig's
  `domain/engine.md`, 7.2), never resumed from patched turns. A patch
  without it is resumed across. The word is `FRESH_WINDOWS` and its
  meaning smith's; a host reads it from the constant, as data. A patch
  may carry the mark and change no declaration.
- **Written functions** over a turn read that turn alone, with a
  `Default` `Context`: a host patches a turn each time it reads it, in any
  order, and keeps the bytes as they were written. A holder's own
  functions never read the turn it holds, so that the order of its
  patches and smith's changes no byte (jig's `protocol/store.md`, 8.6).
- **Frozen texts.** Once a landed patch names a version of
  `smith-transcript`, that version's kept text is frozen: a variant
  appended, a bound widened or a declaration added takes a patch the
  command writes and a version move, from the first release on (skein's
  `codec.md`, 12.2, step 3; temper's `draft/upgrades.md`, 4.3). Before the
  baseline nothing moves: no kept text and no patch, every change in
  place, and a host keeping turns of a build before refuses them as
  another version (section 6).
- **The tests** sit low, in the codec's world (`tests/codecs`), over a
  fixture family of their own (`tests/transcript-patches`): a fixture turn
  at version 1 and 2, made by skein's command, its first patch marked
  `fresh-windows`; each version-1 golden patched by skein's `Patcher` to
  the version-2 golden; the header marked by `FRESH_WINDOWS`; the pieces
  word read from the schema at run time; and the real set's constants as
  spelled, empty before the baseline. Each milestone's QA is an
  independent tester's, from this design.
