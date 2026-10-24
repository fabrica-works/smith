# smith's shell and product

Provisional, 2026-10-09, revised 2026-10-20. smith's shell layer: the crates that turn its
services into processes, the root package whose binaries are glue, and
the product `smith`, one process a person or a script runs. It covers
the commands, the settings and their presets, credentials, the
environment commands build in, and how the product composes one
process. The contracts it serves are the domain's (`domain/`) and the
protocol layer's (`protocol/`). This is the only document that holds
profile numbers (section 6); everything else derives from them
(protocol/limits.md). What is still open is listed in section 11.

## 1. In one page

- **Libraries in `crates/`, glue in `./src`.** smith is a kit taken as
  libraries (domain/README.md, section 4). Its binaries are for
  showcase and internal use: the root package's `smith` and
  `smith-agent`, each a few dozen lines that parse, compose and drive
  (section 3).
- **Two shell libraries.** `smith-agent-shell` is the agent process a
  host spawns. `smith-local-shell` is the product: commands, settings,
  presets, credentials, composition and front ends (section 2). Both are
  ordinary Rust under skein's shell bans.
- **skein first.** The loop driver, an append sink, durable replace and
  a secret store, a directory's lock, the OAuth io driver, the LLM
  client's routes and credentials, trust roots, terminal modes and the
  process's resource usage are generic, and land in skein before smith
  uses them (2.4).
- **One process.** The product composes the local domain, which owns
  the inline agent as its child, with the effects: one loop, one ring,
  no channel. The shell builds the agent configuration from presets and
  settings as a value, and writes no file. Spawning an agent stays a
  library capability, and `smith-agent` is the spawned agent for hosts
  that want a prebuilt one, and for the product's runs on a program,
  which the inline agent cannot run (section 9).
- **Five commands.** `smith`, the interactive chat; `smith exec`, one
  headless run; `smith check`, validation; `smith login` and `smith
  logout`, an account's sign-in and sign-out (section 4). A headless run says how it ended in its exit code (4.3).
- **Instrumented as it ships.** `--json` and `--trace` carry the
  versioned event stream (protocol/events.md). The terminal
  renders the same records, never `Debug` text.
- **Settings are TOML,** layered, strict and bounded, with presets for
  `openai` and `anthropic`, which name no account. A first run needs a
  model and an account of the person's. Every refusal names the
  relationship it breaks (section 5).
- **The numbers are here.** Each preset's per-model values and the
  `standard` profile's deployment and policy values are defaults that
  the probes of benchmarks.md validate. A value with thin evidence is marked
  provisional (section 6).
- **smith's own role.** Without `instructions`, a run's role is the task
  role: one text for chats and tasks, on the topics section 6.6 lists.
- **An account is a provider and one source,** named in the person's
  settings, several per provider: smith's own sign-in, a key, an
  environment variable, a program's own login, or none. smith keeps its
  records and keys in a credentials directory of its own, reads no other
  program's login, and never copies a refresh token (section 7).
- **The build environment is declared:** a named core environment,
  writable caches, and warnings for build wrappers whose caches cannot be
  written (section 8).

## 2. Crates

### 2.1 Libraries only

`crates/` holds libraries only: no crate there declares a binary. A
project built on smith takes the layer it needs, and a binary is the
root package's (section 3). The layers are those of skein's echo
(skein's `examples.md`, section 2):

| Crate | Layer | Owns, for the shell |
|---|---|---|
| `smith-agent-service` | service | the agent's `iterate`; the `standard` profile's deployment values and their derivation into every layer's limits, as step code with its tests |
| `smith-local-service` | service | the local host's `iterate`: the local domain, which owns its agent as a child (the inline agent, or `smith-host-domain` for a spawned one), composed with the effects: the LLM and machine components, the chat store, the terminal and the event sink; the host's and the local layers' share of the profile |
| `smith-inline-agent` | domain | smith's side of the one-host draft: a run as a composed `smith-domain` that speaks `smith-host-domain`'s vocabulary (domain/host.md) |
| `smith-events` | codec | the event vocabulary (protocol/events.md), encoded over skein-json, with its goldens |
| `smith-local-protocol` | protocol | the chat store and delivery records over skein io (protocol/hosts.md, section 5.3); the credentials directory's keys and installation identifier, as private files (section 7) |
| `smith-agent-shell` | shell | the agent process (2.2) |
| `smith-local-shell` | shell | the product (2.3) |

A service owns its limits and the sum of their worst cases, as skein's
echo does, so the profile is a function of the service crates and is
tested there. The shell crates only choose a profile and the values a
setting overrides.

### 2.2 The agent shell

`smith-agent-shell` is the agent process a host spawns, as a library:

- **The agent configuration:** the JSON document of protocol/agent.md,
  section 4, read whole before the loop, strict and bounded. It names a
  profile and its declared values, the endpoints, the programs a run
  may use (protocol/programs.md) and the commands' environment. Each endpoint declares its models' `window`, `output`,
  `reasoning_item`, `head` and `idle` (protocol/limits.md, 2.1); a
  charter's model entries carry the effective `window` and `output`, at
  most the declared ones, and a charter naming a model the configuration
  does not declare is refused (protocol/charter.md, section 2). It never
  holds a credential.
- **Startup,** in the order of skein's `shell.md`, section 6: the
  configuration and the derived limits checked, the worst case within
  the configured memory, termination signals blocked, the endpoints'
  names resolved, trust roots loaded, the seed and the kernel.
- **The one `Agent` adapter:** a `skein_shell::Host` over
  `smith-agent-service`. The service drains the domain's facts each pass
  into its event sink, an append stream through io, and onto the
  channel, each with its own policy and loss count (protocol/agent.md,
  section 5). The adapter opens or adopts the workspace's roots, and its
  hook writes only the operator's short lines to an error writer its
  caller supplies (skein's `shell.md`, section 12). `smith-agent` runs
  it with `skein_shell::drive`, and the worlds host the same adapter
  (testing.md, section 4).
- **The end** is protocol/agent.md, section 6's: its answer sent, the
  channel finished, everything it owns closed, then exit.

### 2.3 The local shell

`smith-local-shell` is the product, as a library:

- **Commands:** typed command values and their parser, so that `main`
  holds no parsing and the parser has step tests (section 4).
- **Settings:** the layers, the presets, the defaults and their
  validation (section 5), and `smith check`, a function of the resolved
  settings and what startup found (4.4).
- **Credentials:** each account's provider and source, the credentials
  directory, a program source's check, and the front ends of sign-in
  and sign-out (section 7).
- **Composition:** `compose` builds `smith-local-service`: the local
  domain with the inline agent as its child, and the effects, the agent's
  LLM and machine components among them (section 9); or, for a run
  whose account's source is a program, the local domain with
  `smith-host-domain` as its child and a spawned `smith-agent`, whose
  JSON configuration it then generates (5.4). Either
  way the agent configuration is built from the presets and the resolved
  settings.
- **The `Local` adapter:** a `skein_shell::Host` over
  `smith-local-service`. The service's event sink writes the records
  through io; the front ends read them in the process (below); the
  adapter's hook writes only short operator lines.
- **Front ends:** the interactive renderer and its plain mode (4.1); the
  headless one, which maps an answer to output and an exit code (4.2,
  4.3). Both read `smith-events` records.

It owns no store and no loop: chats are kept by `smith-local-protocol`
over skein io, sign-in records by skein's OAuth driver as private files
under their lock (section 7), and the loop is `skein_shell::drive`.

### 2.4 What skein provides first

Each lands in skein, with its design, before its smith consumer
(AGENTS.md):

- **`Host` and `drive`,** moved from `skein-world` to `skein-shell`:
  `drive` is the loop of programming-model.md, section 2, with a hook
  each iteration for a shell's drains. `skein-world` re-exports `Host`,
  and no shipped binary links a testing crate.
- **An append sink:** a bounded append stream on the ring, to a file or
  an inherited descriptor, with a write deadline, that counts what it
  drops and settles exactly at exit (skein's `io.md`, 5.1). No thread.
- **Resource usage:** the CPU time and peak resident size of the process
  and of its reaped children, read by the shell at the end for
  `session.ended` (protocol/events.md, 3.2).
- **Durable replace and a secret store:** a whole file replaced over
  io's create, write, sync and rename, its directory synced; and private
  record files on it, with their permissions and links checked.
- **A directory's lock:** exclusive, taken without waiting, released
  when its root closes or its process ends (skein's `io.md`, section 5),
  so that processes sharing a record take turns writing it (section 7).
- **The OAuth io driver:** the loopback listener, the token exchange over
  skein-http, and refresh before expiry, which `skein-oauth` leaves to
  its caller (skein's `oauth.md`); and `localhost` redirects, bound to
  127.0.0.1 and compared exactly. With it, a client id the issuer
  issues, a checked ID token and its subject, revocation, a redirect
  whose port varies, and a record shared across processes, read again
  under its owner's lock before each write (skein's `oauth.md`, 2.1 to
  2.3, 6.4 and 6.5).
- **The LLM client's routes:** OpenAI's public Responses route and
  Anthropic's Messages with a key, a credential as a value and its
  scheme, and the failure classes for an account's limit, its refusal
  and a field a provider does not take (skein's `llm.md`, 2.5, 2.6, 4.7
  and 4.8).
- **Trust roots:** the machine's roots, or one DER file, loaded at
  startup (skein's `tls.md`).
- **Terminal modes:** the window's size, as a shell effect, for the
  status line. Raw input, and its restoration at exit, wait for a view
  that reads a key at a time (protocol/hosts.md, open questions: a
  richer terminal). The line-mode chat (4.1) leaves the terminal in its
  own line mode.

## 3. The root package

### 3.1 The manifest

- **One manifest, two roles.** The root `Cargo.toml` is the workspace
  and the package `smith`. Its binaries:
  - `src/main.rs`, the binary `smith` (`default-run`);
  - `src/bin/smith-agent.rs`, the binary `smith-agent`.
- **Nothing is discovered.** `autobins`, `autotests`, `autobenches` and
  `autoexamples` are off, and every target is declared:

  ```toml
  [[bin]]
  name = "smith"
  path = "src/main.rs"

  [[bin]]
  name = "smith-agent"
  path = "src/bin/smith-agent.rs"

  [[test]]
  name = "end_to_end"
  path = "tests/end-to-end/end_to_end.rs"

  [[test]]
  name = "live"
  path = "tests/end-to-end/live.rs"
  ```

  The end-to-end and live tests are the root package's own targets,
  because cargo gives `CARGO_BIN_EXE_<name>` only to a package's own
  tests (testing.md, section 7). The world crates under `tests/` stay
  packages of their own.
- **Bare commands keep their meaning.** `default-members` lists every
  member, so a bare `cargo build` or `cargo test` covers the workspace.
  The gate names `--workspace` (workflow.md, section 1).
- **Glue.** `main.rs`:
  1. parses its arguments into a command (2.3);
  2. reads the settings and runs `smith check`'s checks, refusing with
     their exit code (5.3);
  3. blocks termination signals, draws the seed, and opens the kernel
     and the roots (skein's `shell.md`, section 6);
  4. composes the service, runs it with `skein_shell::drive`, and exits
     with the code its outcome maps to (4.3), once the service holds
     nothing (skein's `shell.md`, section 13).

  `smith-agent.rs` reads the configuration's path, starts
  `smith-agent-shell`, runs it with `drive`, and exits 0 after its
  answer or 1 when it could not answer (protocol/agent.md, section 6).
- **MIT-licensed.** A `LICENSE` file at the root holds the MIT licence,
  and every manifest, the root package's and each crate's, says
  `license = "MIT"`. smith's registration with a provider is an
  open-source program's own (section 7).
- **Files before the loop.** A shell may read whole files with std before
  its loop runs: the settings, the agent configuration. Once the loop
  runs, every file and process operation is a ring operation
  (programming-model.md, section 2.1).

### 3.2 Lints

- **The root package has its own `[lints]` table,** not the workspace's:
  the ordinary-Rust table of skein's echo shell (programming-model.md,
  section 10.2). `unsafe` is forbidden; silent casts, overflow and
  panicking indexing are out; disallowed types and methods are denied.
  The shell crates under `crates/` carry the same table. No crate allows
  the `disallowed_*` lints wholesale.
- **Two `clippy.toml` files, split by layer.** Clippy reads the nearest
  `clippy.toml` above each manifest:
  - `crates/clippy.toml` holds the step crates' rules
    (programming-model.md, section 10), for every crate below `crates/`
    without its own;
  - the root `clippy.toml` holds the shell bans of skein's echo: `File`,
    `OwnedFd`, std's sockets, `std::process`, `Instant`, `SystemTime`,
    `HashMap`, `HashSet` and `thread::spawn`. It covers the root package
    and its test targets;
  - `smith-agent-shell` and `smith-local-shell` each keep a copy of the
    root file, since the step file sits nearer to them;
  - `tests/clippy.toml` is unchanged, and `benchmarks/` keeps its own
    (benchmarks.md), since a harness starts processes.

## 4. Commands

Shared flags:

| Flag | Meaning |
|---|---|
| `-C DIR` | the workspace: one directory, writable; a git working tree is a repository. The current directory by default |
| `--dir NAME=PATH[:ro\|rw]` | another directory of the workspace, beside `-C`'s, called `NAME` (one safe path component), read-only with `:ro`, writable without a suffix or with `:rw`; repeatable (added 2026-10-20; see below) |
| `-m ENDPOINT/NAME` | the main model |
| `--effort LEVEL` | its reasoning effort |
| `--account NAME` | the account main's endpoint runs on, for this invocation (5.2) |
| `--chat NAME` | the chat to resume or start |
| `-c KEY=VALUE` | a setting, by its dotted path, as a TOML value; repeatable (5.1) |
| `--trace PATH` | append the event stream to `PATH` |

- **Several directories** (added 2026-10-20). With `--dir`, the
  workspace holds `-C`'s directory first, as the working directory, and
  each `--dir` after it in the order given (domain/tools.md, section 2).
  The working directory is then named by its own last path component,
  and the others as `/NAME/...`. A name repeated, the working
  directory's name, a name that is not one safe path component, a path
  that is not a directory, or more directories than the profile's
  workspace holds, is a usage error. A directory that is a git working
  tree is a repository, as with `-C`.

### 4.1 `smith`

The interactive chat, the local host of domain/host.md, section 8:

- **A chat in the workspace.** It opens the workspace and the chat
  (`default`, or `--chat NAME`) under the state directory, and resumes
  its transcript. A chat whose window ran on another endpoint, account or
  backend is carried over to this invocation's, which the status line
  says once (domain/session.md, 8.2); a chat kept by an earlier version
  is refused (5.3).
- **Lines are messages.** A line entered is one message, and a line
  ending in a backslash continues it (protocol/hosts.md, section 5.1).
  The line that starts an activation travels in its start.
- **The run is shown from its events:**
  - the model's text as it arrives, from `text.delta` records, which the
    front end asks for (protocol/events.md, 3.5);
  - each tool call, with its verdict and duration;
  - notices, and failures in words;
  - a status line after each response: the model; its account and how
    it pays, `Using ChatGPT plan` for a sign-in on the `openai` preset,
    `through Claude Code` for a program, `API key` for a key or a
    variable, and nothing for none; tokens in, cached and out, notional
    spend, elapsed time and what is left of the budget.

  This is the live view. The front end asks the one event sink for the
  records it shows, typed and in process (protocol/events.md, 5.2),
  whatever any destination's capture, and never reads a destination
  back. It is best effort: a front end that falls behind drops what it
  cannot hold, counts it, and never holds the run back, and what it
  missed the told turn shows. What decides the outcome, the answer and
  its failure in words, comes from what the local host hears (4.2).

  A waiting prompt appears only once every relayed line has been read.

  A run spawned on a program (9) keeps its records in its own process
  (protocol/events.md, 2), so its live view comes from what the local
  host hears, with the same words: the model's text from live content,
  which the local host asks for at the start (domain/host.md, 2); and,
  from each told turn, the text the pieces did not show, each tool call
  with its verdict, and the status line, its model, tokens and spend the
  turn's, what is left from the charter's budget less the turn's
  cumulative spend, and its elapsed time the local host's, from the
  message it sent or the turn before to this turn. A tool's duration
  and a failed attempt's measures are not shown: records never cross
  the channel (protocol/channel.md, 8).
- **Commands** begin with `/`. They are the local host's, never
  messages:
  - `/model ENDPOINT/NAME` chooses another model of main's endpoint,
    and `/effort LEVEL` another effort, one the main model declares
    when it declares any (6.1), from the next activation, since
    a conversation's model and effort are fixed for its life. A window
    opened with another model is compacted first, with that model
    (domain/session.md, 8.2); another effort takes no compaction. A model
    on another endpoint is refused, and the chat kept: an endpoint is
    resolved, derived and signed in to at startup (5.2), so another
    endpoint is another invocation, and the refusal names it: `smith
    --chat NAME -m ENDPOINT/NAME`. So is another account: `smith --chat
    NAME --account ACCOUNT`. An effort the model does not declare
    is refused, naming those it does. `/model` to a model that declares
    efforts without the chat's current one applies, and drops that
    effort for the provider's default, saying so: the effort dropped and
    the efforts the new model declares. An effort the new model declares
    stays;
  - `/new` starts a new chat, and `/resume NAME` reopens one, once the
    live activation has answered or parked; the chat store then holds
    the other chat (protocol/hosts.md, 5.3). `/resume` of a chat that
    does not exist, or of a name past the store's bounds, is refused,
    naming it;
  - `/quit` ends input.

  A line that begins `//` is a message that begins `/`.
- **The terminal's own line mode.** The front end reads whole lines, and
  changes no terminal mode: the terminal echoes and edits the line, its
  end-of-input key ends input, and its interrupt key is the signal the
  next bullet names. smith reads the window's size for the status line,
  and again when the window changes.
- **Interrupts.** The first is the run's cancel; a second drops it now
  (section 9). After an accepted or parked answer, unread lines are
  carried to the next activation. After a cancel, a failure or a
  refusal they are shown as not delivered and dropped, and nothing starts
  by itself after an interrupt.
- **End of input** lets the run answer or park. Lines it left unread,
  after an accepted or parked answer, open one more activation, which
  answers or parks in turn (domain/host.md, section 8, "Closing input
  drains"); then the product exits.
- **`--plain`** is a line pipe: no terminal modes and no status line; the
  run's text per turn and each tool by name. It is for scripts and the
  worlds' scripted person, and it is the mode when standard input is not
  a terminal.
- **Its event stream,** with `--trace PATH`, is appended to a file. A
  benchmark harness reads it as it grows, while a scripted person types
  at the terminal.
- **It exits** 0 once its input ends, 1 when the chat Store refuses an older build, and 2 or 3 when startup refuses.

### 4.2 `smith exec`

```sh
smith exec [FLAGS] [PROMPT | -]
```

One run, then exit:

- **The prompt travels in the start,** as its one message, labelled
  `person` (domain/run.md, section 3.2), so the run opens on it
  (domain/run.md, section 3.4). `-` reads it from standard input to its
  end, as a harness gives it. A prompt past a message's bound
  (protocol/limits.md, 3.9), an empty one, or both an argument and `-`
  is a usage error.
- **The shared flags apply:** `-m` and `--effort` choose the main model
  and its effort, `-c` sets any key a layer may, and `-C` the workspace.
  A harness pins a run with them alone, beside its user layer.
- **No wait authority by default.** Nobody is there to answer: `wait` is
  offered with a waiting time of zero, so a run that would wait parks at
  once (domain/run.md, section 6) and ends as "needs input". Its last
  text is shown, the chat is kept, and `smith exec --chat NAME` answers
  it.
- **Edits in place.** The tools write the workspace, and nothing is
  committed. The contract is a report.
- **`--deliver`** makes the contract a change, delivered in place:
  checked, committed in each writable repository, never pushed unless
  configured (protocol/hosts.md, section 5.5).
- **Output:**
  - the result's text on standard output;
  - **`--json`** puts the event stream on standard output instead, one
    record per line, the same records `--trace` writes. The result is in
    `run.completed`, and nothing else is written there;
  - **`-o FILE`** also writes the result's text to `FILE`, replaced
    whole and synced, after the run's answer and before `session.ended`;
    a write or a sync that fails is 4.3's;
  - standard error carries a human status: the chat's name, each tool
    call and its verdict, notices, and a failure in words, with provider
    text clipped;
  - **the product reads back none of its own records.** The result, the
    status and the last line are rendered from what the local host hears
    of the run, its told turns, its notices and its answer, in either
    placement of its agent (domain/host.md, section 8). No destination
    is read back. The one thing a front end takes from the event sink is
    a chat's live view, which it asks for in process (4.1);
  - **standard output under `--json`** is the event sink's destination
    of its own (protocol/events.md, 5.2), adopted by what it is: a
    regular file as an append stream, a pipe or a terminal as a write
    stream, each under the stream's write deadline (skein's `io.md`,
    5.1).
- **`--chat NAME`** resumes or starts that chat. Without it, the run
  starts a new chat, named in `session.started` and on standard error.
- **`--ephemeral`** keeps no chat: its files live in a private temporary
  directory removed at exit. That directory is made beneath
  `$XDG_RUNTIME_DIR` when it is set, else beneath the system's temporary
  directory (`$TMPDIR`, else `/tmp`), and never beneath the state
  directory, so a run's chat sits apart from the disk the run's own
  builds fill (revised 2026-10-20). The credentials directory is kept as
  usual. A run on a program leaves the program's own session in the
  source's configuration directory, where no later run looks for it
  (protocol/programs.md, 5.8).
  - **Removed by the names it wrote.** At exit, after the run has
    answered and before the process ends, the chat store removes through
    io each file it wrote there, by the names it knows (its state, its
    turns by number, its deliveries), then the directory, empty (skein's
    `io.md`, 5, `Remove` with `directory`). It never lists or traverses
    the directory, so it removes nothing it did not write.
  - **A directory left behind is said.** One that is not empty then, or
    a removal the kernel refuses, is kept, and a warning before the last
    line names it and why; the code stays the answer's.

### 4.3 Exit codes

A headless run's code follows its answer (domain/run.md, section 10):

| Code | Meaning | When |
|---|---|---|
| 0 | accepted | an accepted answer, however long its teardown took |
| 1 | run failed | a failed answer: the model (a provider's failure past its retries, a field the provider does not take, a limited account, a limit that fired), the program it ran on, policy, stale, a transcript it could not resume; a spawned agent that ended without answering; a host failure while the run went on, such as its chat store failing or refusing an older build, even before a run starts; the result's file not written after an accepted answer (`-o`); or a start refused as busy |
| 2 | usage or configuration | arguments, a settings refusal (5.3), a start the run refused as invalid, a writable repository whose head the start cannot read (one with no commit among them), an account whose provider refused smith's registration |
| 3 | credentials | an account the run needs without a usable credential: not signed in, its sign-in lapsed or without a scope it needs, no key kept, a variable unset, a program not signed in, signed in otherwise or whose check failed; also a run that failed for its account: unauthorized past a refreshed grant, a program's login lost, or refused by its provider |
| 4 | budget | a failed answer for its budget: spend, time or turns |
| 5 | needs input | a parked answer |
| 130 | interrupted | a cancelled answer, after an interrupt or a termination signal; or a run dropped before it answered, at the person's second interrupt or past the cancel's grace |

The code is also `session.ended`'s `exit`. A start that fails before its run opens ends the command with its code and its last line, as an answer does: `smith exec` never waits on a start that has failed.

**The result's file is the product's, not the run's.** `-o`'s file is
written after the answer, so `run.completed` stays as the run answered.
When the answer was accepted and the file cannot be written or synced,
the code is 1 and the last line is the result's file's form above. After
any other answer, the answer's code and last line stand, and the file's
failure is a warning line before the last. `session.ended` is written
once the file's write has ended, so its `exit` is the code the process
exits with.

**Standard error's last line** says how the run ended, in words rendered
from `run.completed`'s typed failure or the startup refusal, matched
exhaustively, so a new class has no words until it is given some. The
words are one function of the answer's failure and of the two facts only
the host has: each account's name, and its own clock when the answer
came, which turns a wait into a time. Every other value is the failure's
own: a limited account's number and reset, and a budget's used and
bound, the time used being the run's wall time as its budget counted it,
never the product's elapsed time. Each
is one line, beginning `smith: `, its detail clipped as an operator's
line is (protocol/limits.md, 2.4). The forms, with `<...>` filled from
the typed values:

| Code | Class | Last line |
|---|---|---|
| 0 | accepted | `smith: accepted in <turns> turns, <spend> notional, <time>` |
| 1 | the model | `smith: run failed: <endpoint>/<model> <provider's class> after <attempts> attempts: <detail>`; for a limit, `smith: run failed: <limit> reached its bound of <bound>` |
| 1 | a limited account | `smith: run failed: account <account> reached its usage limit, usable again at <time>; manage it at <address>`, the time only where its provider gave one and the address only where the account's preset gives one (6.2); never "exhausted" or "empty" |
| 1 | a field the provider does not take | `smith: run failed: <endpoint>/<model> does not take <field>: <detail>` |
| 1 | the program | `smith: run failed: <program> <could not run, or exited before its answer>: <detail>`, the program by its plain name |
| 1 | policy, stale | `smith: run failed: policy: <what was refused>`; `smith: run failed: stale: <what moved>` |
| 1 | a transcript | `smith: run failed: chat <name> cannot be resumed: <reason>` |
| 1 | a spawned agent's failure | `smith: run failed: the agent <how it ended>: <its error output's tail>` |
| 1 | a host failure | `smith: run failed: chat <name>: turn <n> could not be kept: <error>` |
| 1 | the result's file | `smith: run accepted, but its result could not be written to <file>: <the kernel's error>` |
| 2 | usage or configuration | `smith: usage: <problem>`; a setting as in 5.3; `smith: run refused: <what> is <value>, past its bound of <bound>`; `smith: run refused: repository <directory>: its head cannot be read`; `smith: account <account>: the provider refused smith's registration` |
| 3 | credentials | ``smith: account <account> has no credential: run `smith login <account>` ``; ``smith: account <account> must sign in again: run `smith login <account>` ``; ``smith: account <account> was signed in without <scope>: run `smith login <account>` again and allow it, or use a key ``; ``smith: account <account>: <program> is not signed in: run `smith login <account>` ``; ``smith: account <account>: <program> is signed in with <method>, not a subscription: run `smith login <account>` ``; `smith: account <account>: <program> could not be checked: <detail>`; `smith: account <account>: the provider refused it: <its code and reason>`; `smith: account <account>: <VARIABLE> is not set` |
| 4 | budget | `smith: run failed: out of <spend, time or turns>: used <used> of <limit>` |
| 5 | needs input | ``smith: needs input: answer with `smith exec --chat <name>` `` |
| 130 | interrupted | `smith: interrupted: the run was cancelled`; for a dropped run, `smith: interrupted: the run was dropped before it answered` |

A forced exit after the run answered, whatever its answer, adds a
warning line before the last; the code and the last line stay the
answer's. An agent that ended without answering is not this: it is the
spawned agent's failure above.

### 4.4 `smith check`

```sh
smith check [--json] [SHARED FLAGS] [exec [EXEC FLAGS]]
```

What a run would start with, resolved and checked without starting one.
The run is the invocation named: the interactive `smith` with the shared
flags, or, after `exec`, the headless run its flags make, with no prompt.
Its event destinations are among what it would start with: standard
output under `exec --json`, a trace under `--trace` or `[trace]`, and
`-o`'s file. They are derived as that invocation would derive them, and
none is opened. So the effective limits and the teardown bound are the
ones `smith exec` with the same flags would have. `check`'s own
`--json` chooses only its report's form:

- the resolved settings, credentials redacted, each value with the layer
  it came from;
- the effective limits: the declared values, the policy values and every
  value derived from them, each under protocol/limits.md's name for it
  and in its unit (protocol/limits.md, section 9), the same names the
  event stream's header uses;
- each format's version: the channel, the charter, the transcript, the
  event stream, the chat store and this document's own; and the agent's
  name and version, in `agent`;
- each account: its provider, its source, its number (section 7) and
  its state: usable, with where its credential is kept and when it
  expires; not signed in; must sign in again; signed in without a scope
  it needs; a program not signed in, signed in otherwise, or whose
  check failed (protocol/programs.md, section 3); a variable unset.
  Never a value;
- the tools' paths, `sh`, `rg` and `git`, and each program a run may
  use, resolved;
- the commands' environment by name, and the build environment's
  warnings (section 8).

`--json` prints one versioned JSON document with the same content, for
scripts and the benchmark harness. It exits 0 when everything holds,
warnings included; 2 on a configuration problem; 3 when the only
problems are the credentials of accounts the run needs (5.2). Startup runs the same checks (5.3).

### 4.5 `smith login`

```sh
smith login ACCOUNT
```

Each source acts in its own way:

- **`sign_in`:** binds a loopback listener on 127.0.0.1, on a port drawn
  for this sign-in, or first on the account's `redirect_port` when the
  settings give one (section 7); shows the authorization address, and,
  when `SSH_CONNECTION` is set in smith's own environment, the line
  `ssh -L <port>:127.0.0.1:<port> <this host>` for a browser on another
  machine; waits for the browser's redirect; exchanges the code, checks
  the ID token and the granted scopes, and keeps the record under its
  lock (section 7).
  - **Signing in again:** an account that holds a record signs in with
    its issued client id and its hints, and a sign-in whose subject is
    another's is refused (section 7).
  - **Plan usage:** at an account's first sign-in that grants it, the
    preset's notice is printed once (6.2). A sign-in without a scope the
    preset requires keeps the record and says: "Signed in without plan
    usage: run `smith login <account>` again and allow it, or use a
    key".
- **`key`:** reads the key from standard input, to its end, without its
  last newline: `smith login ACCOUNT < FILE`. Standard input that is a
  terminal is a usage error, since a terminal echoes what is typed, and
  so is an empty key. The key is kept, private, in the credentials
  directory, replacing any before it.
- **`env`:** names the variable, and says whether it is set.
- **`program`:** makes the account's configuration directory, private,
  when absent; hands the terminal to the program's own sign-in, `claude
  auth login`, run in the source's environment (protocol/programs.md,
  4.2); waits for it to exit; then checks the source
  (protocol/programs.md, section 3) and says what the check answered.
- **`none`:** says that the account needs nothing.

It exits 0 when the account has a usable credential afterwards, 3 when
it does not, and 2 for an account the settings do not declare, printing
the first run's lines for it (5.3). `smith exec` never starts a
sign-in: an account without a record ends it with exit 3, naming this
command.

### 4.6 `smith logout`

```sh
smith logout ACCOUNT
```

- **`sign_in`:** under the record's lock, revokes its refresh token at
  the provider's revocation endpoint and removes the record. When the
  provider does not confirm the revocation, the record is removed all
  the same, and smith says to disconnect it in the provider's own
  settings, at the address the preset gives (6.2).
- **`key`:** removes the kept key.
- **`program`:** has the host's half run the program's own sign-out,
  `claude auth logout`, in the source's environment (protocol/hosts.md,
  section 2; protocol/programs.md, 4.1), then checks the source. The
  configuration directory stays, for the next sign-in.
- **`env` and `none`:** say that smith holds nothing for the account.

It exits 0 when the account holds nothing afterwards, 3 when it still
does, and 2 for an account the settings do not declare.

## 5. Settings

### 5.1 Layers

Settings are TOML, strict and bounded. Each layer overrides the one
before, key by key:

1. the built-ins: the presets and defaults (section 6);
2. `$XDG_CONFIG_HOME/smith/config.toml`, the user's;
3. `<repo>/.smith/config.toml`, at the top of the workspace's
   repository, or in the workspace directory when it is not one;
4. `-c KEY=VALUE`, in order;
5. flags.

- **Tables merge, values replace.** The commands' environment is a
  table, so a layer sets one variable without restating the others.
- **The repository layer shapes runs, not authority.** A repository is
  content a person may not have written. Its layer may set instructions,
  conventions, the contract, the model and effort among configured
  endpoints, and environment values. It may not set endpoints, accounts,
  the account a run takes, trust, the credentials directory, the
  programs' paths, inherited environment names, writable caches, the
  trace, the deployment's values, or a budget above the user's. A key it
  may not set is refused, naming the file.
  - **The line.** What the layer may set is what the repository's own
    files already do to a run: its guide is read into the system text,
    its checks are its own executable (domain/run.md, 3.3 and 8.1), and
    the agent's commands build and test its code with the person's
    authority. Its environment values are constants it writes, as its
    own scripts may export: they shape the commands and checks, and widen
    nothing a command can reach (section 8). What it may not set reaches
    past the repository: where requests go and who pays for them
    (endpoints, accounts, the account a run takes, trust), where smith
    finds credentials and the programs it runs (the credentials
    directory, the programs' paths), what is taken from smith's own
    environment (inherited names), what is written outside the workspace
    (writable caches, the trace), and what the person pays for or
    reserves (the deployment's values, a budget above the user's).
  - **No approval.** A person never approves a repository's settings,
    and smith keeps no record of trusted repositories: an approval would
    guard nothing the repository's own files cannot already do, and
    `smith exec` and a host's runs, which no person watches, would need
    a way around it. The refusal is the boundary, checked at every
    start.
- **State** lives under `$XDG_STATE_HOME/smith`: chats by workspace and
  temporary chats, and, unless `credentials` names another directory,
  the credentials directory's files (section 7).

### 5.2 Contents and defaults

A first run needs a model and an account:

```toml
model = "openai/gpt-6.1-sol"
effort = "high"

[accounts.chatgpt]
provider = "openai"
source = "sign_in"            # then `smith login chatgpt`
```

Several accounts, several per provider:

```toml
[accounts.claude-alice]
provider = "anthropic"
source = "program"            # Claude Code, signed in by `smith login claude-alice`

[accounts.anthropic-key]
provider = "anthropic"
source = "key"                # kept by `smith login anthropic-key < FILE`

[accounts.ci]
provider = "openai"
source = "env"
variable = "OPENAI_API_KEY"

[endpoints.anthropic]
account = "claude-alice"      # the endpoint's account, when its provider has several
```

| Key | What it is | Default |
|---|---|---|
| `model`, `effort` | the main model, `endpoint/name`, and its effort, one the model declares when it declares any (6.1) | none; the provider's effort |
| `account` | the account main's endpoint runs on, for this invocation; `--account` | the endpoint's own |
| `credentials` | the credentials directory (section 7) | `$XDG_STATE_HOME/smith` |
| `profile` | the named bundle of deployment values | `standard` |
| `instructions` | the role (domain/run.md, section 3.1) | the task role (6.6): smith's own, one text for a chat and for a task |
| `chat` | the interactive chat | `default` |
| `waiting_seconds` | how long an interactive run waits before it parks | 6.4 |
| `sub_agents` | whether runs may delegate | off (domain/run.md, section 5.3) |
| `[budget]` | spend, seconds, turns and the reserve | 6.4 |
| `[models."<endpoint>/<name>"]` | a model's declared values, its efforts and its prices | the preset's, 6.1 |
| `[endpoints.<name>]` | a preset, or its dialect, program, address, TLS name, trust and headers; and its account | `openai` and `anthropic` (6.2) |
| `[accounts.<name>]` | its provider, its source and the source's keys (section 7) | none: no preset names an account |
| `[programs]` | each program's path, by the program's name: `claude_code` | `claude`, found on `PATH` at startup |
| `[deployment]` | declared deployment values | the profile's, 6.3 |
| `[workspace]` | directories, conventions, the contract, delivery fields, the title field, pushes | the workspace directory; a report |
| `[environment]` | `inherit`, the names taken from smith's own environment, and `set` | the core list (section 8) |
| `[build]` | a build preset and writable caches | none |
| `[trace]` | a path and a capture policy (protocol/events.md) | none |

- **Presets.** `openai` and `anthropic` each supply an endpoint of that
  name, the sources its provider takes, and their known models' declared
  values (section 6). Neither names an account: a first run needs one of
  the person's. A model a preset does not know declares its `window`,
  `output` and prices in settings.
- **Accounts.** An account is a provider and one source (section 7),
  declared under `[accounts.<name>]`, several per provider, each signed
  in on its own.
  - **A provider** is a preset, `openai` or `anthropic`. An endpoint the
    settings declare without a preset is a provider of its own, under
    the endpoint's name, and takes `key`, `env` or `none`.
  - **A name** is 1 to 32 characters of `a-z`, `0-9` and `-`, since it
    names files and a directory in the credentials directory.
  - **An endpoint's account** is its `account`, by default the only
    account of its provider. For an endpoint of the run's models,
    several accounts of its provider with none named is refused, naming
    the key; for another endpoint, it is a warning.
  - **`--account NAME`,** or `account`, sets main's endpoint's account
    for one invocation, and one of another provider is refused, naming
    both. An account is fixed for an invocation, as main's endpoint is
    (4.1), and a run never moves to another account, at a limit or
    otherwise (section 7).
  - **A source's program** is its endpoint's: an account whose source is
    `program` on an endpoint that names no program is refused, naming
    both.
- **The run's models and accounts.** A run's charter lists main's model
  and, with `sub_agents` on, every model a preset knows or settings
  declare whose endpoint has an account whose source is not a program;
  the asking LLM names one of them, or gets main's (domain/run.md, 5.3).
  A sub-agent never runs on a program (protocol/programs.md, section 2).
  Every model main's endpoint declares is also the run's, so that
  `/model` may choose any of them (4.1): they share main's account and
  connections, and the derivation is the largest of them. An endpoint
  with no account is one the person has not set up, so its models stay
  out: a first run with one account needs no other credential.
  - **Needed accounts** are the accounts of the run's models. Main's
    endpoint must have an account, or the command exits 2 with the first
    run's lines (5.3). Each needed account's credential must be usable
    (exit 3, 4.3). An account no model of the run needs is reported, and
    its problems are warnings.
  - **The agent's configuration holds the run's models only,** so the
    derivation and every relationship (protocol/limits.md) are those of
    the models the run may name. A model outside the run is still
    checked as values: priced above zero at every rate, and its output
    below its window. `smith check` names each model outside the run and
    why.
- **Prices.** A preset model carries notional prices, the provider's list
  prices, so that spend is real (domain/run.md, section 9). A
  subscription account's spend is shown as notional, never as a charge.
  Every rate is positive: a model that costs nothing per token, such as
  a local one, is given notional prices, and settings that price a model
  at zero at any rate are refused, naming the key.
  The local host's unit is the micro-dollar; settings and the terminal
  use dollars.
- **Tiered list prices.** Where a provider's list price for a model rises
  past a prompt size, the preset prices the model at the highest rates a
  prompt within its window can reach, its window as settings leave it:
  Haiku 5.5 at its long-prompt rates whenever its window passes 100,000
  tokens, and each OpenAI model at its long-context rates whenever its
  window passes 272,000 tokens, so at its base rates at the preset's
  window. A charter's prices stay flat (domain/run.md, section 9), so
  spend is never under-counted. A person who wants the lower rates
  declares a window at or below the tier's size; prices a person sets
  are used as set. The tiers are a prompt's: the benchmarks price each
  response at the rates its own prompt reaches, as the provider bills
  it (benchmarks.md, 8.1), and smith's own spend stays at the flat
  rates its window can reach.
- **Raw byte limits are not settings.** Tests may override them; a person
  declares quantities (protocol/limits.md).

### 5.3 Validation

- **Each layer is read strictly.** An unknown key, a wrong type, or a
  value past its bound is refused, naming the file, the line and the
  key's path. A file holds at most 1 MiB.
- **The resolved settings are checked whole,** by `smith check`'s
  checks:
  - every relationship between declared values (protocol/limits.md);
  - the budget against the profile's ceilings, and against one
    completion's maximum cost, so an impossible spend is refused before
    a run starts (domain/run.md, section 9);
  - each model known to a preset or declared, and priced above zero at
    every rate;
  - each endpoint of the run's models resolved, and each needed
    account's source usable (5.2): a sign-in's record present and its
    scopes granted, a key kept, a variable set, and a program source's
    check usable (protocol/programs.md, section 3);
  - the tools' paths, and each program a run may use.
- **A refusal names the relationship:** the keys on each side, their
  values and their layers, and the relationship's name, for example:

  ```text
  smith: budget.turns = 5000 (from -c) exceeds the standard profile's ceiling of 4096 turns
  ```

- **Startup is `smith check`.** Every command runs its checks first and
  refuses with exit 2, or 3 for credentials.
- **A first run's lines.** With no model, or no account for main's
  endpoint, `smith` and `smith exec` exit 2 and print what to add and
  the command to run after:

  ```text
  smith: no account: add to ~/.config/smith/config.toml
    model = "openai/gpt-6.1-sol"
    [accounts.chatgpt]
    provider = "openai"
    source = "sign_in"
  then run `smith login chatgpt`
  ```

- **Settings of an earlier version are refused,** each naming what takes
  its place, since strict settings never guess:
  - `source = "borrow"`: "declare `provider = "openai"` and `source =
    "sign_in"`, then run `smith login <account>`";
  - a `sign_in` account's `client_id`, `redirect_uri`, `scope` or
    `token_format`: "the provider's preset gives the registration";
  - an endpoint's `identity`, `dialect = "codex"`, and the `codex`
    endpoint: "use `openai/<model>`".
- **An older local chat is refused before Store mutation,** with exit 1,
  whether interactive or headless, including /resume. The named chat lives
  under the workspace's `chats` tree in smith's state directory (5.1).
  A missing layout record says:

> Chat `<chat>` was kept by an older build of smith, from before chats recorded their layouts, and this start has changed nothing in its directory. Before the first release a chat is not translated, so every start refuses this one until it is moved. To start afresh, stop each smith using it, move its directory under smith's state directory aside, whole, and start again: nothing is deleted, and the next start makes a new, empty chat, without this chat's history or its deliveries' answers.

A different store layout says:

> Chat `<chat>` was kept by an older build of smith, whose records' layout differs from this build's, and this start has changed nothing in its directory. Before the first release a chat is not translated, so every start refuses this one until it is moved. To start afresh, stop each smith using it, move its directory under smith's state directory aside, whole, and start again: nothing is deleted, and the next start makes a new, empty chat, without this chat's history or its deliveries' answers.

A different transcript layout says:

> Chat `<chat>` was kept by an older build of smith, whose turns' layout differs from this build's, and this start has changed nothing in its directory. Before the first release a chat is not translated, so every start refuses this one until it is moved. To start afresh, stop each smith using it, move its directory under smith's state directory aside, whole, and start again: nothing is deleted, and the next start makes a new, empty chat, without this chat's history or its deliveries' answers.

Earlier startup may already prepare credentials, make directories, generate
child configuration and open traces outside the Store. An operator-configured
credentials directory inside the chats tree is an explicit exception to the
unchanged-directory claim. Explicitly colocated `-o` output may be written
after the refused Load outcome, during teardown; those later output bytes
are outside that claim. Configured local and generated child trace destinations
resolving inside the workspace's chats tree, including aliases and absent
files under existing parents, are refused before any trace open.

### 5.4 Generated agent configuration

Hosts generate an agent's configuration, so it stays JSON
(protocol/agent.md, section 4): `smith-agent CONFIG.json`. In the
product the shell writes the agent configuration from the presets and
the resolved settings: in one process as a value it hands the service,
with no file; when it spawns an agent (a run whose account's source is a
program, the worlds, and an `isolate` option later), as that JSON, with
the spawned agent's `exit_grace` derived beside it (6.5). It names each
endpoint's dialect and program, each program's path, and the programs
directory, `<credentials>/programs` (section 7); never an account or a
credential, which the run's charter and grants carry
(protocol/charter.md, section 2; protocol/channel.md, section 6).

## 6. Presets' declared values

These are the product's defaults: the per-model values each preset
declares, the `standard` profile's deployment and policy values, and
the defaults of a run the local host starts. Everything else is derived from them (protocol/limits.md) and checked at
startup. Each is validated by the probes of benchmarks.md, and moves when
a probe shows it should. "Provisional" marks a value whose evidence is
still thin. Settings override any of them (section 5). In code, the
per-model values and the run defaults are `smith-local-shell`'s
presets, and the profile's values are the service crates' `standard`
profile (2.1).

### 6.1 Per model

| Preset | Model | `window` | `output` | `reasoning_item` | `head` | `idle` |
|---|---|---:|---:|---:|---:|---:|
| `openai` | `gpt-6-astra` | 272,000 | 32,000 | 64 KiB | 60 s | 300 s |
| `openai` | `gpt-6-luna` | 272,000 | 32,000 | 64 KiB | 60 s | 300 s |
| `openai` | `gpt-6.1-sol` | 272,000 | 32,000 | 64 KiB | 60 s | 300 s |
| `anthropic` | `claude-opus-5-5` | 200,000 | 32,000 | 64 KiB | 60 s | 120 s |
| `anthropic` | `claude-sonnet-5-5` | 200,000 | 32,000 | 64 KiB | 60 s | 120 s |
| `anthropic` | `claude-haiku-5-5` | 200,000 | 32,000 | 64 KiB | 60 s | 120 s |

- **`window`** is usable input tokens, the most one prompt may hold
  with a completion's output beside it; **`output`** is the most output
  tokens, the wire's cap where the dialect sends one, and the output the
  headroom above the threshold holds for the completion whose results a
  compaction carries (protocol/limits.md, 2.1 and 3.9; domain/session.md,
  section 8.1).
- **`efforts`** are the reasoning efforts a model accepts, each a name
  within 32 bytes, as its provider documents them for that model. They
  are the model's, not the dialect's: a provider's models differ, and
  their names change with the provider. `smith check`, `--effort`,
  `/effort` and the run's entrance refuse an effort outside the list,
  naming it and the list. A model that declares none takes any effort
  within the bound, and a provider that refuses it says so as the
  request's failure. A preset declares a model's efforts only from its
  provider's documentation, with that source, and until it does the
  model declares none.
- **The Anthropic models take 1,000,000 input and 128,000 output
  tokens.** The preset declares less, so that a window is compacted
  before its cost grows and the headroom holds a likely output. Settings
  raise them, a window at most what protocol/limits.md, 2.1, allows
  beside the output declared.
- **All per-model values are provisional.** The history probe validates
  `window`, the large-write probe `output`, and the slow-completion probe
  `reasoning_item`, `head` and `idle`.
- **A model no preset knows** declares `window` and `output`;
  `reasoning_item`, `head` and `idle` default to its provider's values in
  this table.
- **The working tier's model.** `gpt-6.1-sol` is the working tier's
  OpenAI model, and a tier's model is one the presets declare
  (benchmarks.md, section 7), so that the tier runs smith as it ships. Its
  notional prices are its provider's list prices, tiered as 5.2's
  "Tiered list prices" says, and the code holds the numbers. It declares
  no efforts until a preset takes them from its provider's documentation
  (the `efforts` bullet above).
- **The small tier.** `gpt-6-luna` and `claude-haiku-5-5`, each at `low`
  effort, are the models benchmarks' probes run on (benchmarks.md,
  section 7). Each preset model takes its provider's effort names, `low`
  among them; without `effort`, the provider's default applies.
- **Every preset holds a window.** With the `standard` profile's 80%
  threshold, a window's room after two outputs, counting a byte as a
  token (protocol/limits.md, 3.9), is 96,000 tokens for the Anthropic
  models and 153,600 for the OpenAI models, and the headroom above the
  threshold, less one output, is 8,000 and 22,400. Both pass
  `fresh-window` and `compaction-headroom`. Less the summary instruction
  and a marker for each of `calls_per_response` calls, that is each
  model's spare, the room between its threshold and its line
  (protocol/limits.md, 3.9). Halved after smith's own
  part, the room bounds one offer, and so a message, a person's line or
  a headless prompt, near 20 KiB when an Anthropic model is declared and
  near 34 KiB with the OpenAI models alone; `smith check` prints the
  exact values.

### 6.2 Per provider

| | `openai` | `anthropic` |
|---|---|---|
| endpoint | `api.openai.com:443`, TLS name `api.openai.com`, `/v1/responses`, OpenAI's public Responses dialect, `openai` (skein's `llm.md`, 4.8) | `api.anthropic.com:443`, TLS name `api.anthropic.com`, `/v1/messages`, Anthropic's Messages dialect, `anthropic`, with a key (skein's `llm.md`, 4.7) |
| program | none | `claude_code`, Claude Code (protocol/programs.md) |
| sources | `sign_in`, with smith's registration; `key`; `env` | `program`; `key`; `env` |
| how a credential is sent | `Authorization: Bearer`, a sign-in's token or a key alike (skein's `llm.md`, 2.6) | a key, in `x-api-key` |
| `connect`, `handshake` | 10 s each | 10 s each |
| `connection_keep` | 30 s, provisional | 30 s, provisional |

- **The dialects** are named `openai` and `anthropic`, in settings as an
  endpoint's `dialect` (5.2), and the local host writes each into the
  agent's configuration with its number (protocol/agent.md, section 4).
- **smith's OpenAI registration,** the `openai` preset's and never a
  setting's: Sign in with ChatGPT for an open-source program, with no
  client secret and no partner key.
  - **Authorization** at `https://auth.openai.com/api/accounts/authorize`
    and the token endpoint
    `https://auth.openai.com/api/accounts/oauth/token`, with OpenAI's
    published keys for the ID token and its revocation endpoint.
  - **The entry client** `dynamic_agent_client`, used only to register:
    the first sign-in sends `agent_name_hint` `smith` and the
    installation's `ext_agent_host_id` (section 7), and keeps the
    client id the callback issues for every exchange, refresh and
    revocation after.
  - **The scopes** `openid profile email` for identity and
    `offline_access resource.invoke chatgpt.tokens.use.direct` for plan
    usage, with `chatgpt.tokens.use.direct` required for a run;
    `resource` `https://api.openai.com/v1`.
  - **The redirect** `http://127.0.0.1:<port>/auth/callback`, whose port
    alone may change between sign-ins (section 7). The callback listener
    admits a request head of at most 16 KiB and 64 header fields: room for
    a browser's roughly 2 KiB, 20-field navigation, its authorization
    query and client hints, with bounded headroom for larger codes and
    browser metadata.
  - **The condition:** OpenAI sanctions plan usage for open-source
    programs, and smith is MIT-licensed (3.1). A preset carries a
    provider's registration only on such a sanction; one withdrawn is a
    change to this document.
- **What the `openai` preset shows,** as OpenAI asks of a program on a
  person's plan:
  - at an account's first sign-in with plan usage, once: "You're using
    your ChatGPT plan. Eligible requests from smith use your plan's
    included usage; manage it at chatgpt.com/settings/usage.";
  - the usage address, `chatgpt.com/settings/usage`, in a limited
    account's last line (4.3);
  - after a sign-out whose revocation is not confirmed, to disconnect
    smith in ChatGPT's settings (4.6);
  - "Using ChatGPT plan" on the status line (4.1), and a plan's spend
    as notional, never as a charge (5.2).
- **The `anthropic` preset carries no registration.** Its subscription
  is reached through Claude Code, signed in by the program's own flow
  (protocol/programs.md, section 4), and its own loop takes a key.
- **The plan's models:** which of the `openai` preset's models a ChatGPT
  plan serves is what `/v1/models` lists for its account, open until a
  capture (section 11).

The connection keep is a live service's policy, never how it ends:
pools close at the answer (section 9). It and the deadlines are policy
values of protocol/limits.md, 2.3.

### 6.3 Per deployment: the `standard` profile

The declared quantities of protocol/limits.md, section 2.2:

| Value | Default | Status |
|---|---|---|
| `conversations` | 5: main and four children, the parallel-children probe's, within `memory` | provisional; the parallel-children probe |
| `tool_payload` | 40 KiB, so that a call at the payload, escaped at its largest (6 bytes a byte), fits one answer of the smallest preset `output` (32,000 tokens at 8 bytes a token): skein's `input ≤ answer` (limits.md, 4) | provisional; the large-write probe |
| `file_bytes` | 1 MiB: the largest file the tools read whole and edit | provisional; the large-write probe |
| `calls_per_response` | 16 | provisional; the parallel-read probe |
| `jobs` | 4 jobs held by one session at once | provisional; the tool-output probe's `background` |
| `read_window` | 32 KiB; a call may ask up to `tool_payload` | decided |
| `guide` | 16 KiB, a guide kept whole up to it | decided |
| `shell_output` | 16 KiB | provisional; the tool-output probe |
| `shell_head`, `shell_tail` | 16 KiB and 48 KiB: a call may ask for up to 64 KiB, kept in that proportion | model-selectable decided; sizes provisional, the tool-output probe |
| `search_hits`, `search_bytes` | 100 and 16 KiB | provisional; the tool-output probe |
| `list_entries` | 512 | provisional |
| `llm_pool` | 128 MiB | provisional; peak memory in `session.ended` |
| `memory` | 2,176 MiB (34 × 64 MiB) | decided: as large as smith's work needs, within the real memory a person's machine can spare, and never smaller than the derived worst case. A change whose worst case passes it raises it, naming the part and its bytes (protocol/limits.md, section 5) |

### 6.4 Run defaults

| Value | Default | Status |
|---|---|---|
| `budget.spend` | 25 dollars, notional | provisional |
| `budget.seconds` | 1 hour per activation | provisional |
| `budget.turns` | 512, a loop guard | provisional |
| `budget.reserve` | 2 turns and 5 minutes | provisional; the budget probe |
| waiting time | 15 minutes interactive; 0 for `smith exec` | provisional |
| `sub_agents` | off | decided, until the reserve lands |

### 6.5 Policy values

The profile's values that bound no bytes, under protocol/limits.md's
names (section 2.3):

| Value | Default | Status |
|---|---|---|
| `max_turns`, `max_spend`, `max_time`, `max_waiting` | 4,096 turns, 1,000 dollars notional, 24 hours of wall time, 24 hours of waiting | provisional |
| `sections`, `host_tools`, `verdicts`, `items`, `fields` | the charter schema's ceilings | decided: the profile narrows none |
| `inbox` | 64 messages unread | provisional |
| `unacknowledged` | 64 turns | provisional |
| `compaction_threshold` | 80% of the window | decided as about 80%; the history probe |
| `shell_timeout` | 2 minutes | provisional; the tool-output probe |
| `check_timeout` | 5 minutes, which the default reserve's 5 minutes holds (domain/run.md, 9.1) | provisional |
| `tool_deadline` | 20 minutes, the most a command's call may ask for | provisional; the tool-output probe |
| `job_poll` | 30 s, a `job` call's wait when it asks for none (added 2026-10-20) | provisional; the tool-output probe's `background` |
| `job_wait` | 5 minutes, the longest yield or wait a call may ask for (revised 2026-10-20: 30 s, which was also the default) | provisional; the dogfood's polls |
| `parallel_tools` | 4 adjacent reads run at once | provisional; the parallel-read probe |
| `group_stop` | 1 s for each of its three steps | provisional; the process-tree probe |
| `close` | 5 s | provisional |
| `write_deadline` | 30 s for one write of the event stream | provisional |
| `wrap_up` | 60 s, a child's final turn's allowance at most | provisional; the budget probe's `wrap-up` |
| `cancel_grace` | 10 s | provisional |
| `exit_grace` | derived: the agent's teardown bound, two `close`s, the event stream's `write_deadline` and its cancels' settling, plus one more `close` as a margin (skein's `shell.md`, section 13; protocol/limits.md, section 7); a setting below the bound is refused | decided as derived; the margin provisional |
| `refresh_lead` | 60 s before the access token expires (added 2026-10-20) | provisional; the `credentials` probe |

### 6.6 Instructions

The task role is smith's own instructions: the role a run is given when
settings set no `instructions` (5.2; domain/run.md, section 3.1).

- **One text, at most 4 KiB,** the same for every model, and for chats
  and tasks alike.
- **Its topics, in this order:**
  1. carrying the request to its end, within its scope, by the plan or
     the steps it gives, in their order, and never by what it rules out
     (revised 2026-10-20);
  2. finding files, with `list` and `search` first, then reading the
     lines the work needs: the documents the request names (its task,
     its plan, the guides it points to) read together in the first
     response, each from its start in the default window; code, a
     window from where a search found it, one window wide enough for
     the part the work needs rather than several small ones, and a
     whole file only when the work needs all of it (revised
     2026-10-20);
  3. batching in one response every call that waits on no other's
     answer: reads, lists, searches and commands that change nothing,
     with a running job's poll beside them; and a change's edits,
     across its files, one `edit` per file in one response, then the
     command that checks them (revised 2026-10-20);
  4. changing files with `edit` and `write`, never through the shell,
     the smallest change first, with every replacement one file needs
     in one `edit`, and an existing file written whole only when most
     of it changes (revised 2026-10-20), as code the repository's
     reviewer would accept by its guides, with no public item, lint
     exception or unreachable path the change does not need (revised
     2026-10-20);
  5. running the tests the change needs, each check once on the same
     files, and long commands as jobs whose polls go beside other work
     (domain/tools.md, section 5; revised 2026-10-20);
  6. not reverting changes the model did not make;
  7. the report that `finish` carries, as soon as the work is done and
     checked, with no reading or running for the report alone (revised
     2026-10-20);
  8. that a compaction does not end the work.
- **It names only smith's own tools** (domain/run.md, section 5.1), and
  says nothing that the run's own sections of the system text already
  say (domain/run.md, section 3.3).
- **Its words are code,** in `smith-local-shell`; its topics are this
  list. A topic added, dropped or moved is a change to this
  section.
- **Its words cost requests.** Each step the words ask of the model in a
  response of its own is a round trip to the provider. The words ask for
  no such step that the work does not need: what can go together goes
  together, and a command is made a job only when it runs long. A change
  to them is proven by the `compare` suite, or, on the way to a verdict,
  by the `parity-change` suite when its run can tell the change's
  expected effect from none, and otherwise by the counts it targets
  (benchmarks.md, sections 4.2 and 16.6).
- **It is a run's instructions,** so it counts where a charter's
  instructions count: in the window's host part (protocol/limits.md,
  3.9). The startup check counts it as written (protocol/limits.md,
  section 4, `host-part-holds-the-role`): its 4 KiB is the text's own
  cap, never a reservation in the host part.

## 7. Credentials

In smith's product, an account has one source, which its settings name
(5.2), and the local host lends every account's grants the same way,
whatever its source (domain/host.md, section 7).

### 7.1 Sources

| `source` | Its keys | What smith holds | Renewed by |
|---|---|---|---|
| `sign_in` | the registration's addresses, moved for a test, and `redirect_port` | a record, `tokens/<account>` | smith, under the record's lock (7.3) |
| `key` | none | the key, `keys/<account>`, kept by `smith login` | the person, with `smith login` |
| `env` | `variable` | nothing: the variable's value, read at start | whoever sets the variable |
| `program` | none: the endpoint names the program (5.2) | nothing: the program's own login, in `programs/<account>/` | the program |
| `none` | none | nothing | nothing |

- **`sign_in`:** smith's own OAuth sign-in, with skein's OAuth client
  and its driver (skein's `oauth.md`, section 6), as a public client
  with the registration its provider's preset carries (6.2), never one a
  setting gives.
  - **Its addresses may move,** so that a test reaches the fake issuer
    (testing.md, section 4): `issuer`, `authorization_url`, `token_url`,
    `keys_url` and `revocation_url`, each `https`, or `http` on
    loopback, and `trust`, the roots their servers are trusted by, in
    the same form as an endpoint's `trust` (`der` or `pem`), the
    machine's by default. The registration itself, its client, name,
    scopes and redirect path, never moves.
  - **`redirect_port`,** optional: the port the loopback listener tries
    first, on a port drawn for the sign-in when that one is taken
    (skein's `oauth.md`, 6.5). Without it, each sign-in draws its port.
    Only the port changes between sign-ins.
  - **What a sign-in checks:** the ID token, by the provider's published
    keys, its issuer, audience, expiry and nonce (skein's `oauth.md`,
    2.2), and the granted scopes. The account is the token's subject,
    never an email. A sign-in without a scope the preset requires keeps
    its record, and the account is not usable for a run until it signs
    in again and allows it (4.5).
  - **The issued client id** is kept with the record, and every
    exchange, refresh and revocation after uses it (skein's `oauth.md`,
    2.1).
  - **One subject per account.** A sign-in whose subject another of the
    settings' accounts holds is refused, naming that account, and its
    new tokens revoked; one whose subject is not the record's is refused
    too. Two names never race one rotating token, and an account never
    changes person.
  - **Refreshed before it lapses** by the local host, under the record's
    lock (7.3), and lent as a bearer.
- **`key`:** a key the person gave `smith login`, kept privately, sent as
  its endpoint's dialect sends a key (6.2), never refreshed.
- **`env`:** the variable `variable` names, holding a key, read at
  start, for unattended use.
- **`program`:** the program's own login, signed in by the program's own
  flow into the account's configuration directory (protocol/programs.md,
  section 4). smith holds nothing of it: the local host checks the
  source at startup and before each run on it (protocol/programs.md,
  section 3), grants the account's name, and runs the run on a spawned
  agent (section 9).
- **`none`:** for an endpoint that takes no credential, such as a local
  model's. It lends no grant: a run's LLM whose account it is names no
  account in its charter, so its calls carry no credential
  (protocol/charter.md, section 2; protocol/channel.md, section 6). The
  account's name still has a number, checked with the rest (7.4).

### 7.2 The credentials directory

```
<credentials>/                    private, the user's own
├── host-id                       the installation's identifier, urn:uuid:…
├── tokens/<account>              a sign-in's record, replaced whole
├── tokens/<account>.lock/        the record's lock: a directory, never replaced
├── keys/<account>                a key, as `smith login` read it
└── programs/<account>/           a program source's configuration directory
```

- **smith's own,** never another program's: no other program's login,
  such as `~/.codex` or `~/.claude`, is read, and the person's own
  settings, hooks and plugins for a program never load into a run
  (protocol/programs.md, 4.2).
- **Each file private,** read and written as skein's private files are,
  each replaced whole by durable replace (skein's `io.md`, 5.2 and 5.3).
  The programs directory is private; `smith login` makes each
  configuration directory in it, private, and only the program writes
  there.
- **`host-id`** is the installation's identifier, which a sign-in sends
  to its provider as `ext_agent_host_id` (6.2): `urn:uuid:` and a random
  UUID, drawn once, by the first process that finds it absent, and
  stored expecting no file. A process that finds one stored meanwhile
  reads that one. It is not a credential, and names no person.
- **Chats are kept under the state directory** (5.1), apart from the
  credentials directory, so that a test or a benchmark gives each run a
  scratch state directory and a durable credentials directory.

### 7.3 Records across processes

- **Several processes share a record:** two terminals on one account, a
  benchmark's attempts, a live test beside a person's chat.
- **Each sign-in record is shared** (skein's `oauth.md`, 6.4): every
  write of it, a refresh, `smith login`'s keep and `smith logout`'s
  revocation, runs under the record's lock, and first reads the record
  again. A newer generation another process kept is used with no
  refresh; no record is an account signed out; another subject fails
  the grant, so a run never changes person mid-run.
- **The lock** is `tokens/<account>.lock/`, a directory opened and
  locked with skein's `Lock` (skein's `io.md`, section 5): exclusive,
  released when it is closed or its process ends, never left behind by
  a crash. A process that finds it busy asks again on a timer, within
  the driver's lock wait (skein's `oauth.md`, 6.7), and then the write
  fails as busy, for now, and the grant is asked for again.
- **A refresh refused for good,** such as `invalid_grant` or
  `refresh_token_reused`, means the account must sign in again (exit 3,
  4.3); a transient failure keeps the record, and is retried.
- **A crash between the provider's rotation and the kept record** leaves
  a spent refresh token: the next refresh is refused as reused, and the
  person signs in again.

### 7.4 Account numbers

A charter's LLM names its account by number, a `u32`, and a
transcript's opening turn keeps it (protocol/charter.md, section 2;
protocol/transcript.md, section 2). The local host derives each
account's number from its name: the first four bytes of the name's
SHA-256 (skein's `lib.md`, 6.2), read as a big-endian `u32`. It never
comes from the account's place in the settings or among the run's
models, so that an account has one number in every invocation, and a
chat resumed on the same account is not carried over. Renaming an
account changes its number.

Two of the settings' accounts with one number are refused, naming both
(exit 2). Every declared account is checked, not only the run's, and
`none` accounts among them though their numbers name no grant (7.1), so
that neither the models a run uses nor an account's source changes
whether the settings are admitted.

### 7.5 What the person sees

- **`smith login` and `smith logout`** act per source (4.5 and 4.6).
- **Interactive sign-in:** the interactive `smith` signs in a needed
  `sign_in` account that holds no record, at the terminal, as `smith
  login` does, and the run starts once the record is kept. `smith exec`
  never does, and exits 3 naming `smith login`.
- **The notices** (domain/host.md, section 7):
  - **limited:** the provider says the account has reached its usage
    limit, with when it resets only where it says. The run fails for
    its account (exit 1), naming the usage address the preset gives
    (6.2). A run never moves to another account; the person chooses one
    with `--account`;
  - **refused:** the provider refused the account what was asked, such
    as an account not eligible for plan usage, or a scope not granted.
    It is never retried, and the run fails for its account (exit 3),
    with the provider's code and reason;
  - **unauthorized:** the local host refreshes a sign-in's grant once and
    lends the new generation. A key, a variable, a refresh refused for
    good, or a program's login lost, fails the run for its account
    (exit 3), naming the command to run.
- **The provider refused smith's registration** (`invalid_client`) at a
  sign-in or a refresh: exit 2, since no sign-in of the person's mends
  it.
- **Never shown:** credential values appear in no setting, event, trace,
  terminal line or `smith check` output.
- **Refresh tokens are never copied.** An issuer rotates them, and a
  copied one is spent by whichever copy refreshes first. Live tests and
  benchmarks sign in smith's own accounts once, in a credentials
  directory of their own (testing.md, section 2.3; benchmarks.md,
  section 6), and never copy a login.

## 8. The build environment

- **The commands' environment is the configuration's, whole**
  (protocol/agent.md, section 3): what `[environment]` inherits and sets,
  and nothing else. Shells and checks see it; search sees none.
- **A named core list.** By default `inherit` names `PATH`, `HOME`,
  `USER`, `LOGNAME`, `LANG`, `LC_ALL`, `LC_CTYPE`, `TZ`, `TERM` and
  `TMPDIR`, each taken from smith's own environment when present. Names
  are listed, never matched by a pattern. A build preset adds its own:
  `rust` adds `CARGO_HOME` and `RUSTUP_HOME`.
- **Writable caches.** A build preset, or `[build].caches`, names the
  directories outside the workspace that builds write: for `rust`,
  `$CARGO_HOME/registry` and `$CARGO_HOME/git`. `smith check` confirms
  each is a directory smith's effective user may write and search, as
  the kernel answers it (skein's `shell.md`, 6.1, `directory_access`),
  creating and changing nothing. A cache it cannot confirm (absent, not a
  directory, or denied, with the kernel's error) is a warning naming the
  cache and why: a build that never writes there is not stopped by it.
  Once contained trees exist, they are the writable part of the view
  (protocol/agent.md, section 3).
- **Wrapper warnings.** A build preset names the wrappers its tools may
  be configured with, such as `rustc-wrapper` for `rust`. `smith check`
  warns when a configured wrapper's cache is not among the writable
  caches, or cannot be written, and names the variable or file that
  configured it.
- **Credential-like names.** `smith check` warns when a variable the
  commands would see has a name that suggests a credential.
- **Per-tree limits later.** CPU, memory and process limits per command
  tree come with contained trees. Until then, settings do not stand in
  for them.

## 9. One process

- **The product is one process.** `smith-local-shell` composes
  `smith-local-service`: the local domain, which owns the inline agent
  as its child domain, composed with the effects, the agent's LLM and
  machine components, the chat store, the terminal and the event sink,
  in one protocol layer: one loop, one ring, no channel
  (domain/host.md, section 9). The agent configuration is a value the
  shell builds from presets and settings, never a file. Credentials are
  lent as entities. The loop is `skein_shell::drive`.
- **A run on a program is spawned.** The inline agent has no program
  backend (protocol/programs.md, section 2), so a run whose account's
  source is a program runs on a spawned agent: the local domain is
  composed with `smith-host-domain` and its protocol layer, and spawns
  `smith-agent`, found beside `smith`'s own binary, with the generated
  configuration (5.4). The agent spawns the program as its child, and
  the run's events, failures and exit codes are the same as inline
  ones. Every other run stays inline.
- **Events come from the domain's owner.** The inline agent's owner
  drains each run's facts every pass into the service's event sink, so
  the event stream is the same in one process as from a spawned agent
  (protocol/events.md).
- **Stopping:** an interrupt is the run's cancel; a second, or a run past
  its grace, is dropped, and everything below it settles before the
  process ends (domain/host.md, section 9.2).
- **The end:** after its last word, the process closes everything it
  owns and exits once each has reported closed and io is empty, when
  `drive` returns (skein's `shell.md`, section 13). The LLM pool closes
  at the answer. A termination signal while closing turns the remaining
  closes into aborts. A host that spawns `smith-agent` derives its exit
  grace from the same bound (6.5).
- **The spawned capability.** Spawning an agent is a library capability:
  `smith-host-domain` and `smith-host-protocol` in a host, and
  `smith-agent-shell` in the process. `smith-agent` ships with `smith`,
  for the product's runs on a program and for any host that wants a
  prebuilt agent process, and is the real-process proof of the channel.
  An `isolate` option of the product, which spawns the agent of every
  run, comes with contained trees.

## 10. The world

Within the focused suite's 30 seconds (15 seconds before the user's doubling
of 10-22, provisional) and the fuzzy suite's 120 seconds: (suite's 60:
before the user's doubling of 10-22, provisional)

- **Step tests in `smith-local-shell`:**
  - each command's arguments to its typed value, and each usage error;
  - each layer's merge, the environment per variable;
  - one refusal per strict rule, naming the file, line and key;
  - each key the repository layer may not set, refused;
  - preset expansion, byte-stable, against goldens;
  - the task role within 4 KiB, each tool it names one of smith's own;
  - one refusal per relationship, naming it;
  - the mapping from every answer to its exit code and its last line on
    standard error, one per class: limited, refused and unauthorized
    among them, and a limited account's line with and without its reset;
  - each source's settings, and each refusal of 5.2 and 5.3: a name out
    of bounds, an ambiguous account, `--account` of another provider, a
    `program` source on an endpoint without a program, and each setting
    of an earlier version naming what takes its place;
  - account numbers: one name, one number, in any order of the settings,
    the first four bytes of its SHA-256 read big-endian; two account
    names of one number, a `none` account's among them, refused with
    exit 2, naming both.
- **Step tests in `smith-agent-service` and `smith-local-service`:** the
  `standard` profile derives, and its derived worst case fits `memory`.
- **The local host's simulated world** (testing.md, section 2.1):
  - one headless story per exit code but 2;
  - events against what the fake provider saw: one `response.completed`
    per response, tool records paired by call id, usage summing to the
    answer's spend, no loss;
  - each source lent: a sign-in's record, refreshed before it lapses;
    a key; an `env` account; a `none` endpoint;
  - two processes on one record: one refreshes under the lock, the
    other uses the newer generation with no refresh; a lock held past
    the lock wait fails the write as busy, and a later grant succeeds;
  - a program account's check at startup, each answer of it, and a run
    on the fake program on a spawned agent (protocol/programs.md,
    section 8);
  - a limited account, with a reset and without one; a refused account;
    an unauthorized grant refreshed once, and one refused for good: each
    with its exit code and last line;
  - a chat of an earlier version refused, and a chat carried over to
    another account.
- **End to end** (testing.md, section 2.3), against fakes:
  - `smith exec --json` to an accepted answer, and to needs input;
  - `smith check --json`, passing and failing;
  - a startup refusal of each layer;
  - `smith` on a terminal: a first sign-in and a change committed in
    place, then a second invocation that resumes; and one with
    `--trace`, its records read from the file as it grows;
  - `smith login` and `smith logout` against the fake issuer, reached
    through the settings' moved addresses: a first sign-in that
    registers, a sign-in again with the issued client id, another
    subject refused, a scope declined, a revocation confirmed and one
    not; a key read from standard input, and one from a terminal
    refused;
  - `smith login` on a `program` account, against the fake program;
  - `smith-agent`, spawned by a host in the test's loop.

## 11. Open questions

- **The Anthropic window:** whether the preset's window should be the
  models' 1,000,000 tokens once the `history` and `cache-reuse` probes measure
  cost and quality there.
- **Kept traces:** a default place for traces, their rotation and how
  long they are kept.
- **The plan's models:** which of the `openai` preset's models
  `/v1/models` lists for a ChatGPT plan, and whether the small and
  working tiers' `gpt-6-luna` and `gpt-6.1-sol` are among them. Default:
  the preset declares them, and a run on one the plan refuses fails as
  refused.
- **A chat's account between invocations:** a chat resumed with another
  account is carried over (4.1). Default: allowed, said once on the
  status line.
- **A chat's directories between invocations** (added 2026-10-20): a
  chat resumed with other `--dir` directories than it began with.
  Default: the workspace is the resumed invocation's, and the
  transcript's paths into a directory no longer named are read as they
  were written.
- **An `isolate` option,** once contained trees make spawning contain
  more than one process does.
