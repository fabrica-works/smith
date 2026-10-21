# Programs as backends

Provisional, 2026-10-19. How smith's agent runs a run on an LLM program
in place of its own loop: the program, Claude Code first, runs as the
agent's child, signed in by the person through the program's own flow,
and the agent adapts it both ways. The agent process is agent.md's, the
host's side hosts.md's, and what a run is domain/run.md's. What is still
open is section 9.

## 1. In one page

- **One agent process, two backends.** A run runs on the *direct
  backend*, smith's own loop over skein's LLM client, or on a *program
  backend*, a program the agent spawns as its child and adapts both ways.
  Nothing above the agent changes: a host sees a smith agent, its turns,
  calls, notices and one answer, whichever backend runs the run
  (section 2).
- **The grant chooses.** A grant is a credential or a program source's
  name (channel.md, section 6). A credential runs the direct backend; a
  program source runs the program its endpoint names. The host chooses
  the source, and the agent never changes it (section 3).
- **The person signs in through the program's own flow,** into a
  configuration directory of the host's, never the person's own. Nothing
  of smith's holds the login; the only thing smith reads is the
  program's status (section 4).
- **Claude Code is started isolated:** settings the agent writes, no
  credential variable, no setting or hook of the workspace, its built-in
  tools limited to the families the charter grants, and every call of
  one asked of smith's policy through its permission prompt, so that
  none runs unseen by the host (section 5).
- **The terms are rules:** the program as its publisher ships it, each
  person's own subscription through the publisher's flow, nobody paying
  for or reselling its use, its name in plain text only (section 6).
- **What it weakens is said:** the budget may pass its ordinary pool by
  one completion; compaction and a command's bounds are the program's; a
  command can read the login, so a host that keeps a person's logins
  takes only that person's runs; managed settings are outside the
  agent's reach (section 7).
- **The worlds run a fake program,** smith's own, and the live suite
  runs the real one (section 8).

## 2. One agent process, two backends

- **Nothing above the agent changes.** A host's half spawns one agent
  process per run (agent.md, section 1; hosts.md, section 3), and the
  channel, the host's domain and the host see the same records whichever
  backend runs the run.
- **The direct backend** is the agent's own loop: its sessions make
  completions over skein's LLM client with the credential the run was
  granted (llm.md).
- **A program backend** runs main's conversation as one program process,
  the agent's child, started for the run and ended with it. The agent's
  component `smith-protocol-program` (README.md, section 5) adapts it
  both ways: what the charter says becomes the program's
  command line, settings and input (section 5), and what the program
  says becomes smith's turns, calls, notices and answer.
- **The child is inside the run's containment.** The agent's tree is the
  run's (hosts.md, section 1), so a cancel, a drop or the host's stop
  ends the program with the run, and its own children with it.
- **The run is still smith's.** Its charter, budget, result contract,
  host tools, `finish`, `deliver` and `wait`, its messages, parking and
  answer are domain/run.md's on either backend. What the program does in
  place of smith's own loop is section 5's, and what that weakens
  section 7's.
- **The inline agent has no program backend.** `smith-inline-agent` runs
  in its host's process, with no child of its own (domain/host.md, 9.1),
  so it runs the direct backend only. A host runs a run whose grant
  names a program source on a spawned agent.
- **Sub-agents are not offered on a program backend.** Main's
  conversation is the program's: the program's own sub-agents are
  withheld (5.3), and smith's `sub_agent` is not served, so a charter's
  sub-agents family is withheld from the LLM and the run's tools say so
  (events.md, 3.3). A sub-agent's LLM never runs on one either: a start
  that grants a program source for a sub-agent's LLM's account, and
  not for main's, is refused at the entrance (domain/run.md, section 4).
- **Open runs are not offered on a program backend.** A start whose
  charter is Open and grants a program source is refused at the entrance,
  as invalid grants, before the program is checked or started
  (domain/run.md, section 4). Open runs on the direct backend (section 9).

## 3. Choosing the backend

- **From the run's grant.** The charter's main LLM names the account
  that pays for it (charter.md, section 2), and the host grants that
  account (channel.md, section 6):
  - **a credential,** a bearer or a key: the direct backend, on its
    endpoint's dialect;
  - **a program source's name:** the program backend, running the
    program its endpoint's configuration names (agent.md, section 4) by
    the program's name: `claude_code`, Claude Code, the one program this
    document adapts; its executable is `claude`. A grant of a program
    source for an endpoint that names no program is a credential the
    agent cannot install.
- **The host chooses the source,** since it decides who pays. The agent
  runs the run on the grant it was given until the run ends: it never
  moves a run to another account or another source, at a limit or
  otherwise (5.6).
- **A program source is usable** when the program's status, run in the
  source's environment (4.2), says it is signed in the way the source
  needs. For Claude Code: `claude auth status` exits 0, and its answer's
  `authMethod` is `claude.ai`, a person's subscription. A check answers
  one of:
  - **usable;**
  - **not signed in:** the status exited 1;
  - **signed in otherwise,** naming the answer's `authMethod`, since
    another method pays with something other than the person's plan;
  - **failed,** with its detail: the program missing, its status not
    ended within its deadline, or an answer that does not read.
- **The check is smith's,** in two places:
  - **the host's half** checks a program source when its host asks
    (hosts.md, section 2), so that a host places a run on a program only
    where the login is usable. When to ask is the host's: smith's local
    host checks its program accounts at startup (domain/host.md,
    section 8);
  - **the agent** checks again at a program run's start, before the
    program starts, since a login may lapse in between. A check other
    than usable ends the run before its first turn (5.6).

## 4. The person's sign-in

### 4.1 The program's own flow

- **Once per host and source:** `claude auth login`, run in the source's
  environment (4.2), on a terminal the person holds. A host's setup may
  hand that command the terminal, as `smith login` does
  (`docs/design/shell.md`, 4.5); over SSH the person pastes the code
  Claude Code shows.
- **Signing out** is `claude auth logout`, run the same way: by the
  person on the terminal, or by the host's half when its host asks
  (hosts.md, section 2).
- **Nothing of smith's holds the login:** no grant, record, setting,
  transcript, trace, event or environment of smith's holds it, nor a
  token `claude setup-token` makes. The only thing smith reads is the
  status's answer (section 3).
- **Per host.** A login is its host's: one that lapses or is refused
  makes its source unusable on that host alone, and the person is told
  the host and the command to run there. Claude Code warns three days
  before a login expires.
- **A program source has no subject smith can read:** the status names
  no account. A host names it to people by its holder's display name.

### 4.2 The configuration directory

- **The host's own, never the person's.** Each program source's login
  lives in a *configuration directory* of the host's, never the person's
  `~/.claude`, at the cost of one more sign-in, so that the person's own
  settings, hooks and plugins never load into a run.
- **One per source, named by the source.** A program source's name is 1
  to 32 characters of `a-z`, `0-9` and `-`, and its configuration
  directory is that name in the *programs directory*, so that a person
  may sign in several subscriptions on one host. The programs directory
  is private; a host's sign-in makes each configuration directory,
  private, and only the program writes a login in it.
- **The source's environment.** Every command of a program source, its
  status, sign-in, sign-out and runs, runs with:
  - `CLAUDE_CONFIG_DIR` set to the source's configuration directory;
  - none of `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`,
    `CLAUDE_CODE_OAUTH_TOKEN`, nor a cloud provider's variables, which
    Claude Code takes before a subscription's login (in `-p`,
    `ANTHROPIC_API_KEY` whenever it is present), so that a key in the
    host's own environment never stands in for the subscription.
- **The settings file** in the directory is the agent's (5.3): written
  whole, by durable replace (skein's `io.md`, 5.2), with the same content
  for every run, so that runs on one source never race over it.

## 5. Running Claude Code

### 5.1 The command

| smith's run | Claude Code |
|---|---|
| the charter's instructions and brief | `--append-system-prompt-file`, keeping Claude Code's own prompt |
| the main LLM's model and effort | `--model`, `--effort` |
| host tools, `finish`, `deliver` and `wait` | the agent's own MCP server (5.3), given with `--mcp-config` and `--strict-mcp-config` |
| the result contract | `--json-schema`, the outcome read from `structured_output` when no `finish` was accepted (5.5) |
| policy on each call | `--permission-prompt-tool`, naming the agent's prompt tool (5.4); `--tools`, for the built-in tools the charter grants (5.3) |
| the start's messages, and messages mid-run | user messages on `--input-format stream-json` |
| turns, activity and text as it comes | `--output-format stream-json`, with `--verbose` (which `-p` requires for it) and `--include-partial-messages` |
| failures | `rate_limit_event`, `system/api_retry`, the `result` message, a lost login (5.6) |
| the budget | `--max-turns`, and the agent's own measure and clock (5.7) |
| a cancel | SIGINT, then SIGTERM (5.7) |
| parking and resuming | `--session-id`, derived from the conversation; `--resume` (5.8) |

- **`-p`, never `--bare`,** which reads no subscription's login.
- **`--setting-sources user`,** with the source's configuration
  directory, so that only the settings the agent wrote apply, and no
  settings, hooks or `.mcp.json` servers of the run's workspace.
- **The default permission mode,** never `dontAsk`, which turns each
  prompt into a denial, nor `bypassPermissions`.
- **Where it runs:** in main's working directory, the start's first
  directory (domain/tools.md, section 2), with the run's environment
  (agent.md, section 4) as 4.2 changes it.
- **The run's own files for the program,** its instructions, its MCP
  configuration and its outcome's schema, are private files (skein's
  `io.md`, 5.3) in a directory of the run's own outside its workspace,
  removed when the run ends.
- **Its version moves on its own:** the agent learns what the program
  offers from `system/init`'s `capabilities`, never from a version
  string.

### 5.2 Isolated

- **No credential variable** reaches the program (4.2), so the
  subscription is what pays.
- **No setting of the workspace's:** `--setting-sources user` reads only
  the configuration directory's settings, the agent's. A repository's
  hooks, settings and MCP servers never run.
- **No other MCP server:** `--strict-mcp-config` takes only the agent's.
- **What stays outside the agent's reach** is said in section 7: managed
  settings, and what a command can read.

### 5.3 The tools

- **Built-in tools for the granted families,** under their own names,
  and no other:

  | Family | Claude Code's tools |
  |---|---|
  | inspect | `Read`, `Glob`, `Grep` |
  | modify | `Write`, `Edit` |
  | shell | `Bash` |

  `--tools` names exactly these for the run's granted families, so the
  program's other built-in tools, its sub-agents, web access and task
  lists among them, are absent. A `system/init` that lists a built-in
  tool the run did not grant, or lacks one it granted, fails the start
  (5.6).
- **Every built-in call asks.** The settings the agent writes give each
  of these tools an ask rule (`permissions.ask`, by bare name). Claude
  Code applies deny rules, then ask, then allow, and an ask rule wins
  over allow rules, the permission mode and a hook's allow, so every
  call of a built-in tool reaches the permission prompt, and smith's
  policy (5.4), before it runs.
- **The agent's own MCP server** serves the run's tools: each host tool
  the charter declares, `finish`, and `deliver` and `wait` when granted
  (domain/run.md, 5.1), and the prompt tool. It is a mode of the agent's
  own binary, which Claude Code starts over standard input and output
  from the MCP configuration, and it relays each call to its agent,
  which answers it as on the direct backend. The settings allow its
  tools by rule, since the run answers them itself. Claude Code shows
  them to its model under its MCP prefix; the agent tells their calls
  under smith's names.
- **The prompt tool's name is reserved:** a host tool may not take it,
  as it may take no name of smith's (domain/run.md, 5.1).

### 5.4 The permission prompt

- **Each built-in call** reaches the prompt tool with its tool's name
  and input, and the agent answers allow, with the input unchanged, or
  deny, with a message for the model.
- **Allowed** when the tool is one the run granted and each path it
  names is one a workspace tool may use (domain/tools.md, section 4):
  `Read`'s, `Write`'s and `Edit`'s file, and `Glob`'s and `Grep`'s path,
  resolved against the working directory, within the start's
  directories, outside every `.git` directory, and writable as the start
  says for `Write` and `Edit`. A path is judged by its names alone,
  reading nothing, so a symbolic link within the directories that leads
  out of them is allowed: containment, where the host has it, bounds
  where it leads (section 7).
- **Denied otherwise,** with the rule it broke in words, as a workspace
  tool's invalid call is answered (domain/tools.md, section 4), so the
  model may try again otherwise.
- **`Bash`** is allowed as the shell family is. Its command is not
  parsed, so not confined to the workspace's paths: containment, where
  the host has it, bounds it (agent.md, section 3; section 7).
- **The host sees each call** as a direct backend's: its turn tells it
  with its name and input, and its result, and the event stream records
  it (events.md, 3.6).
- **The prompt tool's input and answer** are Claude Code's, and not
  documented by it: the agent reads the call's tool name and input from
  it, and answers in the shape the live suite checks against the real
  program (section 8).

### 5.5 Its stream, told as smith's

- **`system/init`** is checked before the first message: its tools are
  those 5.3 gives, its MCP servers only the agent's, connected, and its
  `capabilities` what the run needs; otherwise the start fails (5.6).
- **Each assistant message is a completion,** told as a turn of main's
  conversation (transcript.md, section 2): its text and calls in
  position, its usage as the program reported it, and the run's spend.
  A built-in tool's call is a call with its name and input as the model
  wrote them, and its result is text the program wrote. The program's
  opaque blocks, its thinking and their signatures, are not kept: no
  other backend takes them.
- **Partial messages** feed the event stream's content as it comes
  (events.md, 3.5), and nothing else.
- **The outcome:** a `finish` the run accepts ends the run, as on the
  direct backend (domain/run.md, 7.2): the agent closes the program's
  input, and the program exits. A turn that ends, with its `result`,
  and no `finish` has its `structured_output`, the outcome
  `--json-schema` asked for, taken as a `finish` call's input and judged
  by the same contract. A turn with neither is a model that named no
  tool, nudged as on the direct backend, and failed *malformed* past its
  nudges (domain/run.md, section 10).

### 5.6 Failures and notices

- **A usage limit** shows as a `rate_limit_event` whose
  `rate_limit_info.status` is `rejected`, with its
  `rate_limit_info.resetsAt`, in Unix seconds, when given, or as the
  final `result`'s `api_error_status`. The account is *limited*: the agent
  sends the limited notice, with the reset only when given (channel.md,
  section 4), and the run ends failed for its account (domain/run.md,
  section 10). It is never retried, and never moved to another source.
- **A login lost,** mid-run or at the start's check (section 3), not
  signed in or signed in otherwise, is *unauthorized*: the agent sends
  the unauthorized notice, and the run ends failed for its account,
  since no new grant of a program source helps. Its host tells the
  person the host and the command to run there.
- **`system/api_retry`** is the program's own retry, told as the event
  stream's (events.md, 3.5): the program, not the session, retries.
- **Any other error the `result` reports** (`is_error`, with its
  `api_error_status`, `terminal_reason` and `errors`) fails the run as
  its model's, with the status as evidence.
- **A start that fails,** the program missing, a check that failed, no
  `system/init` within the start's deadline, or one that lacks what the
  run needs or lists a built-in tool not granted, fails the run as its
  program's, naming the program, with its detail, and no notice.
- **A program that exits** before its `result`, or whose stream does not
  decode, fails the run the same way, with its exit and its standard
  error's tail.

### 5.7 The budget, the clock and a cancel

- **Spend is the agent's measure:** each completion's usage, as its
  assistant message reports it, priced at the charter's prices for the
  main model (domain/session.md, section 6). Claude Code's own
  estimate, `total_cost_usd`, is in dollars, not the host's unit, and is
  neither used nor given as `--max-budget-usd`.
- **No completion is reserved before it is made,** since the program
  makes it. The run's reserve, at least one of main's completions at its
  most (domain/run.md, 9.1), takes a completion that passes the ordinary
  pool, and its overrun is recorded as spend.
- **The wind-down:** when the ordinary pool is spent in any part, the
  agent ends the program's turn with SIGINT and sends one user message,
  the run's note that the budget is spent and to finish now
  (domain/run.md, 9.1). From then on the agent refuses every call but
  `finish`, and `deliver` when granted: a built-in's at its permission
  prompt (5.4), and its own server's tools' as it answers them, each
  answered as not run, naming the calls allowed, as a call outside a
  completion's tool choice is (domain/session.md, section 5). Main then
  has the reserve's turns and time; a program that has not finished
  within them, or whose spend passes the budget, is stopped as a cancel
  is, and the run fails for its budget.
- **Turns:** each completion counts as a turn, as on the direct backend,
  and `--max-turns` is the charter's turns, a guard behind the agent's
  own count.
- **Wall time** is the agent's clock: at the run's time the program is
  stopped as a cancel is.
- **A cancel:** SIGINT, which ends the program's turn, then, past a
  grace from the agent's limits, SIGTERM, on which the program exits
  143. The agent's tree holds anything left (section 2).

### 5.8 Parking, resuming and carrying over

- **The program's session** is named by `--session-id`, a UUID derived
  from the run's identity, main's thread and the window's first message
  (domain/session.md, section 2), so that a later activation finds it
  and a new window gets a session of its own. Claude Code keeps it in
  the source's configuration directory. A window's first call retried
  after the program took its user message resumes that session rather
  than naming it again.
- **Parking:** when the run parks (domain/run.md, section 6), the agent
  ends the program after its turn by closing its input, and answers
  parked. A turn held on `wait` has no end of its own: the agent stops
  the program with SIGINT, as a cancel does (5.7). The program's
  session stays.
- **Resuming on the same source and host:** when the transcript's
  opening turn names this backend and account (transcript.md, section 2)
  and the configuration directory holds the program's session, the
  program starts with `--resume` and that session, and the start's
  messages, and the answers the transcript lacks, go in as user
  messages.
- **Otherwise the conversation is carried over:** a transcript whose
  window ran on another backend, account or program source, or whose
  program session is gone, on another host or removed, opens a new
  window from smith's transcript, compacted first (transcript.md,
  section 4; domain/session.md, 8.2). On a program backend the
  compaction runs as a program session of its own with no tools, and
  the window turn's messages open the program's new session as its
  first user message.
- **A conversation its host does not keep,** such as the product's
  `--ephemeral` chat, resumes its program session only while it lasts:
  no later conversation derives the same session id, so the session is
  never resumed after it, and it stays in the configuration directory as
  the program keeps its sessions (section 9).

## 6. The terms it keeps

Anthropic allows a person's own Claude Code to run in a product under
conditions. smith keeps each as a rule of its own:

- **The program runs as its publisher ships it:** an unmodified binary,
  found by its path (agent.md, section 4), never patched or replaced,
  and no way of signing in to it removed or hidden.
- **Each person's own subscription, through the publisher's flow:** each
  person signs in with their own account through the program's own
  sign-in (section 4). smith never collects, stores, lends or forwards
  the login, nor a token made from it.
- **Nobody pays for or resells its use** on the person's behalf, and
  nothing of smith's stands between the subscription and Anthropic: the
  program makes its own requests.
- **Named in plain text only:** a host may say that a run runs on Claude
  Code, and names no product, feature or company after it, nor shows
  its logo. smith's own words say "through Claude Code".
- **A host's slots bound its runs at once,** a program run's as any
  other's: a run on a program counts as one run.
- **What is still to settle** with Anthropic is section 9's first
  question.

## 7. What it weakens

- **The budget:** the program makes its completions without smith
  reserving each one's most first, so a run may pass its ordinary pool
  by one completion, which the reserve takes, and stops at its next
  turn (5.7).
- **The program's, not smith's:** the bounds on a tool's output, a
  command's timeout, read before write and match before edit. The
  transcript tells what the program did; it does not make it smith's.
- **Two compactions:** the program compacts within its session as it
  judges, and smith still compacts at its own line (domain/session.md,
  8.2). The program's own compactions cost what they cost, uncounted:
  a completion's usage is what its assistant message reports (5.7).
- **A retried first call:** when a window's first call fails after the
  program took its user message, the retry resumes the session (5.8)
  and sends the message again, so the program's session holds it twice.
- **Symbolic links:** the prompt judges a built-in call's paths by
  their names, reading nothing (5.4), and the program follows a link
  that a workspace tool's write would not (domain/tools.md, section 4),
  so `Write` and `Edit` may change a file outside the start's
  directories through a link inside them. Containment, where the host
  has it, bounds them.
- **A command can read the login.** `Bash` runs what the model writes,
  and a command, like any the run starts, reads what the run's tree can,
  the configuration directory's login among it. A program's login on a
  host is so readable by the runs on that host, and a subscription
  serves one person: a host that keeps a person's program logins takes
  only runs that person's accounts pay for, until a run's tree is
  contained from the logins. A host that serves one person is
  unaffected.
- **Managed settings,** which a machine's administrator sets for Claude
  Code, still apply above the settings the agent writes, and are outside
  its reach. A host runs program runs only on a machine whose managed
  settings it accepts.
- **The prompt tool's shape** is not documented by the program; the live
  suite checks it (section 8).
- **A scripted run on a subscription** rests on `-p` without `--bare`,
  which Claude Code says will stop being the default. The agent never
  passes `--bare`, and the live suite finds the release in which `-p`
  stops reading the login.
- **Sub-agents** are not offered (section 2).

## 8. Testing: the fake program

- **The worlds run a fake `claude`,** smith's own, as skein's fake LLM
  peer stands in for a provider (`docs/design/testing.md`, section 4).
  The agent spawns it by its path, and its story scripts it:
  - `auth status`, `auth login` and `auth logout`, answering signed in,
    not signed in or signed in otherwise, and refusing to run without
    `CLAUDE_CONFIG_DIR`;
  - `-p` with stream-json both ways: `system/init` with the tools
    `--tools` gave and the MCP servers the configuration named;
    assistant and partial messages, with usage; `rate_limit_event`;
    `system/api_retry`; a final `result`, each as the story says;
  - an MCP client of the agent's server: it lists its tools, asks the
    prompt tool before each built-in call, in the shape the agent
    expects, and acts on the answer, and calls `finish`, host tools and
    `wait` as the story's model does;
  - SIGINT ending a turn, and SIGTERM exiting 143;
  - observations: its arguments, its environment's names (never a
    value), the settings it found, and each prompt and its answer.
- **Its stories:**
  - a run that finishes through `finish`, and one through
    `structured_output`;
  - a built-in call allowed, and one denied for a path outside the
    workspace;
  - a built-in tool not granted, listed by `system/init`, failing the
    start;
  - a usage limit, with a reset and without one;
  - a login lost mid-run, and each answer of the check;
  - a credential variable in the host's environment, absent from the
    program's;
  - the wind-down, a cancel and the wall time;
  - a park, and a resume by the program's session;
  - a conversation carried over from the direct backend and back.
- **The live suite** runs the real program on a source signed in once
  (`docs/design/testing.md`, 2.3), and checks what the fake assumes: the
  prompt tool's shape, `system/init`'s tools and capabilities, a limit's
  events, and that `-p` reads the login.

## 9. Open questions

- **Open on a program backend:** a launch without a result schema, a
  text end taken only at its `result`, and an interrupt for its last
  reply. Default: refused at the entrance (section 2).

- **Anthropic's terms for a product:** whether a product that runs the
  person's own Claude Code falls under the allowance for a person's own
  use, and whether it needs Anthropic's Commercial Terms. Default:
  section 6's rules, until Anthropic answers.
- **The prompt tool's shape:** Claude Code does not document its input
  and answer. Default: the shape the live suite checks against the
  release it runs.
- **How the MCP server reaches its agent:** Claude Code starts it with
  pipes of its own, and skein's io has no local sockets. Default: a
  loopback listener of the agent's (skein's `io.md`, 3.2), its address
  and a secret of the run's own in the MCP configuration's private file,
  and every connection without the secret refused.
- **Sub-agents on a program backend:** whether smith's `sub_agent` is
  served to the program, each child a direct session or a program of
  its own. Default: not offered (section 2).
- **A refusal from the program:** whether Claude Code tells an account's
  refusal (not eligible, a scope not granted) apart from its other
  errors. Default: it is not told apart, and fails the run as its
  model's (5.6).
- **Runs at once on one login:** whether runs that share a
  configuration directory interfere. Default: one at a time where the
  host can choose (`docs/design/benchmarks.md`, section 10).
- **A session not kept:** whether the agent removes the program session
  of a conversation its host does not keep, when that conversation ends.
  Where the program keeps a session is its own, and undocumented.
  Default: it is left to the program's own clean-up, and never resumed
  (5.8).
- **Logins out of the run's reach:** a contained tree that keeps the
  configuration directories from the run's commands while the program
  reads its own, which would lift section 7's one-person rule.
