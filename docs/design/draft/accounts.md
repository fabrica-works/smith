# Accounts and backends

Draft, 2026-10-18. smith's side of jig's `draft/llm-accounts.md`: how
smith's agent runs on a person's subscriptions and keys within jig's
`draft/providers.md`'s rules, and how `smith`, the product, an
application in that draft's sense, signs in, keeps and refreshes its
own credentials, and runs its live tests and benchmarks on them. What
changes in smith's design is section 8; the order of work, section 9;
what is open, section 10.

## 1. In one page

- **smith's product is its own client.** It signs in to ChatGPT under
  its own name, keeps its records in its own credentials directory,
  refreshes them itself, and reads no other program's login (jig's
  `draft/llm-accounts.md`, section 2).
- **An account is a provider and a source,** declared in the person's
  settings, several per provider. Sources are `sign_in`, `key`, `env`,
  `program` and `none`; none reads another program's login (section 2).
- **The presets are `openai` and `anthropic`,** and carry no account:
  `openai` is OpenAI's public Responses route, with smith's own Sign in
  with ChatGPT registration; `anthropic` is Anthropic's Messages with a
  key, and Claude Code as a program. Neither has a subscription route or
  an identity profile (2.3).
- **One credentials directory** holds smith's records, keys and Claude
  Code's configuration directories, by account; the person's own
  `~/.codex` and `~/.claude` are never read (2.5).
- **Refreshes are serialized across smith's processes** by a lock beside
  each record, so two terminals, or a benchmark's attempts, never spend
  one rotating refresh token twice (2.6).
- **`smith login` and `smith logout`** act per source, Claude Code's
  through its own flow; the terminal shows what jig's
  `draft/providers.md`, 2.6 asks of a plan (section 3).
- **The agent runs on two backends,** its own loop or Claude Code's,
  as jig's `draft/providers.md`, section 7 has it. In the product, a run
  on Claude Code runs on a spawned agent, and every other run inline;
  grants carry a value with how it is sent, or name a program's source
  (section 4).
- **A conversation may change provider, account or backend** at a
  window, keeping its text, calls and results and dropping the
  provider's opaque replay (4.4).
- **Live tests and benchmarks run on smith's own accounts,** in a
  credentials directory the developer signs in once, or on keys
  (sections 5 and 6).

## 2. The product's accounts

### 2.1 Settings

```toml
model = "openai/gpt-6.1-sol"
effort = "high"

[accounts.chatgpt]
provider = "openai"
source = "sign_in"

[accounts.claude-alice]
provider = "anthropic"
source = "program"            # Claude Code, in <credentials>/programs/claude-alice

[accounts.anthropic-key]
provider = "anthropic"
source = "key"                # kept by `smith login anthropic-key`

[accounts.ci]
provider = "openai"
source = "env"
variable = "OPENAI_API_KEY"

[endpoints.anthropic]
account = "claude-alice"      # the endpoint's account, when its provider has several
```

- **A provider** is a preset (2.3). An endpoint the settings declare
  without a preset is a provider of its own, under the endpoint's name,
  and takes `key`, `env` or `none`.
- **`[accounts.<name>]`:** `provider`; `source`; and the source's own
  keys (2.2). A name is 1 to 32 characters of `a-z`, `0-9` and `-`,
  since it names files and a directory (2.5).
- **Which account a run uses:** each endpoint's `account`, by default
  the only account of the endpoint's provider. For a needed endpoint
  (shell.md, 5.2), several accounts with none named is refused, naming
  the key; for another, a warning. `--account NAME`, or `account =
  NAME`, sets main's endpoint's account for one invocation, and one of
  another provider is refused, naming both. An account is fixed for an
  invocation, as its endpoint is (shell.md, 4.1).
- **`credentials`:** the credentials directory (2.5),
  `$XDG_STATE_HOME/smith` by default.
- **The repository layer sets none of** `accounts`, `account` or
  `credentials`, as it sets no endpoint (shell.md, 5.1): each says who
  pays.

### 2.2 Sources

| `source` | Its keys | What smith holds | Loop | Renewed by |
|---|---|---|---|---|
| `sign_in` | none: the provider's preset gives the registration | a record, in `<credentials>/tokens/<name>` | smith's | smith, under the record's lock (2.6) |
| `key` | none | the key, in `<credentials>/keys/<name>`, put there by `smith login` | smith's | the person, with `smith login` |
| `env` | `variable` | nothing: the variable's value, read at start | smith's | whoever sets the variable |
| `program` | `program`, the preset's by default (`claude_code`) | nothing: the program's login, in `<credentials>/programs/<name>/` | the program's (4.1) | the program |
| `none` | none | nothing | smith's | — |

- **A sign-in's registration** is the preset's: smith's identity at the
  provider, its name `smith`, its entry client, scopes and redirect
  path, which settings never change (jig's `draft/providers.md`, 2.2).
  Settings may move its issuer, the authorization, token, revocation
  and key-set addresses and their `trust`, so that a test reaches the
  fake issuer (testing.md, 2.3); the repository layer moves none of
  them.
- **A key and a variable are sent as the dialect sends a key**
  (`x-api-key` for Anthropic, a bearer for OpenAI), a sign-in's token as
  a bearer.
- **A program's status, sign-in and sign-out run in its run's
  environment:** its configuration directory set (`CLAUDE_CONFIG_DIR`),
  and none of the credential variables jig's `draft/providers.md`, 7.4
  removes, so that an `ANTHROPIC_API_KEY` in smith's own environment
  never stands in for the subscription. Its source is usable when
  `claude auth status` exits 0 with `authMethod` `claude.ai` (jig's
  `draft/providers.md`, 7.2).

### 2.3 The presets

| | `openai` | `anthropic` |
|---|---|---|
| endpoint | `api.openai.com:443`, `/v1/responses`, OpenAI's public Responses dialect | `api.anthropic.com:443`, `/v1/messages`, Anthropic's Messages dialect |
| sources | `sign_in`, with smith's registration; `key`; `env` | `program`, Claude Code; `key`; `env` |
| models | shell.md, 6.1's OpenAI models, named `openai/<model>` | shell.md, 6.1's Anthropic models |

- **smith's registration** (jig's `draft/providers.md`, 2.2): entry
  client `dynamic_agent_client`, `agent_name_hint` `smith`, its
  installation's `ext_agent_host_id` (2.5), the scopes for identity and
  plan usage, `chatgpt.tokens.use.direct` required, `resource`
  `https://api.openai.com/v1`, and the redirect
  `http://127.0.0.1:<port>/auth/callback` (3.2). OpenAI sanctions plan
  usage for open-source tools, which is shell.md, section 7's condition
  for a preset to carry a registration: the `openai` preset carries it
  once smith carries the MIT licence (jig's `draft/llm-accounts.md`,
  section 11, step 1).
- **No preset names an account.** A first run needs a model and an
  account of the person's (3.1).
- **Which models a plan serves** is what `/v1/models` lists for the
  account (jig's `draft/providers.md`, 2.4), open until a capture
  (section 10).

### 2.4 Several accounts with one provider

- **Each its own table,** signed in on its own: two ChatGPT accounts,
  two Claude subscriptions, each with its own email.
- **One subject per account:** a sign-in whose subject another of the
  settings' accounts holds is refused, naming that account, and its new
  record revoked, so two names never race one rotating token.
- **A run never moves to another account,** at a limit or otherwise: a
  limited account fails its run (3.4), and the person chooses another
  with `--account`.

### 2.5 The credentials directory

```
<credentials>/                    0700, the user's own
├── host-id                       the installation's ext_agent_host_id, urn:uuid:…
├── tokens/<account>              a sign-in's record, replaced whole
├── tokens/<account>.lock/        the record's lock: a directory, never replaced
├── keys/<account>                a key, as `smith login` read it
└── programs/<account>/           a program's configuration directory: Claude Code's own login
```

- **smith's own,** never another program's: `~/.codex` and `~/.claude`
  are never read, and the person's own Claude Code settings, hooks and
  plugins never load into a run (jig's `draft/providers.md`, 7.3).
- **Each file private,** read and written as skein's private files are
  (skein's `io.md`, 5.2 and 5.3); a program's directory is private, made
  by `smith login`, and only the program writes in it.
- **`host-id`** is drawn once, by the first process that finds it
  absent, written expecting no file (`Absent`); a process answered
  `Conflict` reads the one another wrote.
- **Chats are kept under the state directory** (shell.md, 5.1), a
  program's session file among them (4.4): a test or a benchmark gives
  each run a scratch state directory and a durable credentials
  directory.

### 2.6 Refreshing across processes

- **Several processes share a record:** two terminals on one account, a
  benchmark's attempts, a live test beside a person's chat.
- **The lock** is `tokens/<account>.lock/`, a directory opened and
  locked with skein's `Lock` (skein's `io.md`, 5): exclusive, released
  when its root is closed or its process ends, never left behind by a
  crash. A process that finds it `Busy` asks again on a timer, within
  the OAuth driver's HTTP deadline, then fails the grant as unavailable
  for now, retried by the session.
- **Every write of the record takes it:** a refresh, `smith login` and
  `smith logout`.
- **Under the lock,** the record is read again:
  - a newer generation than the process holds is lent, with no refresh;
  - an absent record means needs sign-in;
  - a record of another subject fails the grant as needs sign-in, so a
    run never changes account mid-run (2.4);
  - otherwise, if the refresh is still due, the process refreshes, keeps
    the rotated record durably, and only then releases the lock.
- **A refresh's terminal error** (`invalid_grant`,
  `refresh_token_reused` and their kin) clears the record's tokens under
  the lock; a transient failure keeps them (jig's `draft/providers.md`,
  2.5).
- **A crash between the provider's rotation and the kept record** leaves
  a spent refresh token: the next refresh is answered
  `refresh_token_reused`, and the person signs in again.
- **skein's driver** reads its owner-kept record again before a refresh
  when its owner asks, which skein's `oauth.md`, 6.4 does not do for an
  account that holds a record; the lock itself is io's.

### 2.7 Settings of earlier versions

Strict settings refuse an unknown key or value (shell.md, 5.3). Each of
these is refused naming what takes its place:

- `source = "borrow"`: "declare `provider = "openai"` and `source =
  "sign_in"`, then run `smith login <name>`";
- a `sign_in` account's registration keys: "the preset gives the
  registration";
- an endpoint's `identity`, and the `codex` endpoint and preset: "use
  `openai/<model>`".

A chat whose turns name the `codex` endpoint resumes on `openai` by
4.4's rule.

## 3. What the person sees

### 3.1 A first run

- **With no model or no account,** `smith` and `smith exec` exit 2 and
  print the four lines to add:

  ```text
  smith: no account: add to ~/.config/smith/config.toml
    model = "openai/gpt-6.1-sol"
    [accounts.chatgpt]
    provider = "openai"
    source = "sign_in"
  then run `smith login chatgpt`
  ```

- **The interactive `smith`** signs in a needed `sign_in` account that
  has no record, at the terminal, as `smith login` does (shell.md,
  section 7); `smith exec` never does, and exits 3 naming the command.

### 3.2 `smith login ACCOUNT`

- **`sign_in`:** binds `127.0.0.1:1455`, or a free port when 1455 is
  taken, since only the port may change (jig's `draft/providers.md`,
  2.2); prints the authorization address, and, without a display,
  `ssh -L <port>:127.0.0.1:<port> <this host>` for a browser on another
  machine; exchanges the code, checks the ID token and the scope, and
  keeps the record. An account that already has a record signs in with
  its issued client id and the hints, and refuses another subject. At
  an account's first sign-in with plan usage it prints, once: "You're
  using your ChatGPT plan. Eligible requests from smith use your plan's
  included usage; manage it at chatgpt.com/settings/usage." Plan usage
  declined keeps the sign-in and says: "Signed in without plan usage:
  run `smith login <name>` again and allow it, or use a key".
- **`key`:** reads the key from standard input, without echo, once, and
  keeps it.
- **`env`:** names the variable and says whether it is set.
- **`program`:** makes the account's directory, runs the program's own
  sign-in, `claude auth login`, on the terminal in the run's environment
  (2.2), waits for it, then reads `claude auth status`.
- **`none`:** says that the account needs nothing.

It exits 0 when the account has a usable credential afterwards, 3 when
it does not, and 2 for an account the settings do not declare, printing
the lines of 3.1 for it (shell.md, 4.5).

### 3.3 `smith logout ACCOUNT`

Revokes a sign-in's refresh token and removes its record, or says to
disconnect smith in ChatGPT's settings when the revocation is not
confirmed (jig's `draft/providers.md`, 2.3); removes a key; runs
`claude auth logout` in a program's environment. It exits 0 when the
account holds nothing afterwards, and 3 otherwise.

### 3.4 The status line and the last line

- **The status line** (shell.md, 4.1) names how the run is paid: "Using
  ChatGPT plan", "through Claude Code", "API key".
- **The last line's forms** (shell.md, 4.3):

  | Code | Class | Last line |
  |---|---|---|
  | 1 | a limit | `smith: run failed: account <account> reached its usage limit[, usable again at <time>][; manage it at <address>]`, the time only where the provider gave one, and never "exhausted" or "empty" (jig's `draft/providers.md`, 2.5) |
  | 1 | unsupported | `smith: run failed: <endpoint> refused <field>: <detail>` |
  | 2 | misconfigured | `smith: account <account>: the provider refused smith's registration` |
  | 3 | refused | `smith: account <account>: the provider refused it: <the provider's reason>` |
  | 3 | sign in again | ``smith: account <account> must sign in again: run `smith login <account>` `` |
  | 3 | a program's login | ``smith: account <account>: Claude Code is not signed in: run `smith login <account>` `` |

  These take the place of the exhausted account's row and of the
  borrowed login's expiry notice.
- **`smith check`** reports each account's provider, source and state,
  never a value.

## 4. The agent

### 4.1 Two backends

- **jig's `draft/providers.md`, section 7 is the design:** one agent
  process per run; the direct backend, smith's own loop over skein's
  client; the program's backend, `claude -p` spawned as its child and
  adapted both ways (its 7.4), isolated (`--setting-sources user`, no
  credential variables, never `--bare`). Claude Code's built-in tools
  are kept by default, each call that would prompt answered by smith's
  policy (jig's `draft/llm-accounts.md`, section 12).
- **`smith-inline-agent` has no program backend** (jig's
  `draft/providers.md`, 7.1). The product runs a run whose account's
  source is a program on a spawned agent: the local host composes
  `smith-host-domain` and spawns `smith-agent` with its generated
  configuration, `programs` included (shell.md, 2.3 and 5.4). Every
  other run stays inline (shell.md, section 9).
- **The agent's configuration** (protocol/agent.md, section 4): an
  endpoint names no account and no identity profile; `programs` gives
  each program's path and the directory of its configuration
  directories; a charter's LLM names its grant (protocol/charter.md,
  section 2).
- **Its budget:** where smith cannot reserve a completion's most before
  it is made, the run's reserve holds one turn's most, and an overrun
  within it is recorded as spend (jig's `draft/llm-accounts.md`, 5.1;
  domain/run.md, 9 and 9.1).

### 4.2 Grants and checks

- **A grant carries a value** with how it is sent, a bearer or a key,
  or **names a program's source,** whose configuration directory the
  agent finds by that name under its configuration's directory
  (protocol/channel.md, section 6).
- **A host-side check** reads a program source's login in its run's
  environment (2.2). A host calls it when it chooses: jig's worker at
  its hello, when told of a new source, on its holder's request and at
  an interval (jig's `draft/llm-accounts.md`, 3.4); the local host at
  startup. The agent checks again at each run's start.

### 4.3 Notices and failures

- **Limited:** the provider says the account is spent, with when it
  resets only where it says (skein's failure, or Claude Code's
  `system/api_retry` and its result). The session never retries it; the
  run ends, failed for its account, and the host hears it.
- **Refused:** the account may not do what was asked (not eligible, a
  scope not granted); never retried; the host hears it.
- **Unsupported:** a field the provider will not take, named; the
  request's error, never sent again unchanged (jig's
  `draft/providers.md`, 2.5).
- **Misconfigured:** the provider refused smith's registration
  (`invalid_client`); the host hears it.
- **Unauthorized:** the host refreshes the grant once (skein's
  `oauth.md`, 6.3); a program's login lost mid-run is unauthorized for
  that host alone.

### 4.4 Carrying a conversation over

- **At a window.** A session opened from a window whose endpoint,
  dialect or account is not its charter's main LLM's compacts that
  window first, with the charter's main model, rendering the old window
  without its opaque blocks; the new window turn records the new
  endpoint, dialect and account, so that a transcript still holds one
  of each (protocol/transcript.md, sections 2 and 3). This takes the
  place of the refusal *another endpoint or dialect*
  (protocol/transcript.md, section 4; domain/session.md, sections 3 and
  8.2).
- **Opaque blocks** are what is the provider's own: reasoning,
  signatures and item ids (skein's `llm.md`, section 2 refuses another
  provider's). Text, calls and results are kept.
- **Another account of one provider** drops them too, until a capture
  shows they cross accounts (jig's `draft/llm-accounts.md`, section 12).
- **Between backends.** A run on Claude Code tells its turns as smith's
  (jig's `draft/llm-accounts.md`, 5.3): a built-in tool's call as a call
  with its name and input, and its result as text the domain wrote. A
  conversation moves between backends by the same rule. Claude Code's
  own session file is kept in the chat's directory, given to
  `--resume` by its path (jig's `draft/providers.md`, 3.5) only on the
  same source, and removed with an `--ephemeral` chat.

## 5. Live tests

The live suite (testing.md, 2.3) runs on smith's own accounts in a
durable live directory the developer signs in once per machine; each
test keeps its chats in its scratch directory.

```
~/.local/state/smith-live/                 0700; the suite reads $SMITH_LIVE
├── config/smith/config.toml               the live user layer
├── host-id
├── tokens/chatgpt                         smith's ChatGPT record, refreshed in place
├── tokens/chatgpt.lock/
└── programs/claude-alice/                 Claude Code's own login
```

```toml
# $SMITH_LIVE/config/smith/config.toml
credentials = "/home/me/.local/state/smith-live"

[accounts.chatgpt]
provider = "openai"
source = "sign_in"

[accounts.claude-alice]
provider = "anthropic"
source = "program"

[accounts.anthropic-key]
provider = "anthropic"
source = "env"
variable = "ANTHROPIC_API_KEY"
```

Once, per machine:

```sh
export SMITH_LIVE=~/.local/state/smith-live
XDG_CONFIG_HOME=$SMITH_LIVE/config smith login chatgpt        # approve in the browser
XDG_CONFIG_HOME=$SMITH_LIVE/config smith login claude-alice   # Claude Code's own sign-in
```

Then:

```sh
SMITH_LIVE=~/.local/state/smith-live cargo nextest run --profile live
```

- **A story,** "a run ends with a commit in place", on ChatGPT: the
  test makes its scratch workspace and state directory, takes
  `$SMITH_LIVE`'s user layer, and runs `smith exec --json --deliver
  --account chatgpt -m openai/gpt-6-luna --effort low "Create hello.txt
  containing exactly: hi"`. smith loads the record, refreshes it under
  its lock if it is due, writing the rotated record back to
  `$SMITH_LIVE`, and calls OpenAI's public Responses route on the plan.
  The referee checks `hello.txt`, the commit, the typed events and exit
  0.
- **On Claude:** `--account claude-alice -m
  anthropic/claude-haiku-5-5`; the product spawns its agent, which
  spawns `claude -p` with Claude Code's configuration directory
  `$SMITH_LIVE/programs/claude-alice`; the same outcome is checked.
- **On a key:** `--account anthropic-key`, smith's own loop on
  Anthropic's model.
- **Carried over:** a chat started on ChatGPT, resumed with `--chat` on
  Claude, then on ChatGPT again, each run completing (4.4).
- **The refresh story** raises the refresh lead, a policy value a test
  may set, so its run refreshes at once, and checks that the record's
  generation rose and that the run completed.
- **A missing setup fails by name,** as a test lacking a credential
  does (testing.md, 2.3): "no record in `$SMITH_LIVE/tokens/chatgpt`:
  run `XDG_CONFIG_HOME=$SMITH_LIVE/config smith login chatgpt`", or
  "`ANTHROPIC_API_KEY` is not set".
- **Renewed by use:** a run more than an hour after the last refresh
  refreshes the record, which renews its refresh token's 30 days; one
  left unused past them is signed in again (jig's
  `draft/providers.md`, 2.3).
- **The live suite runs one test at a time,** without retries, never at
  the gate, on the small tier by default.

## 6. Benchmarks

- **smith's arms** run on a credentials directory of the harness's,
  signed in once like the live directory, shared by attempts under the
  record's lock (2.6); each attempt keeps its own state directory.
- **smith's loop on Anthropic's models** runs on a key. smith on a
  Claude subscription runs Claude Code's loop under smith's adapter: an
  arm of its own, `smith-claude-code`, not measured until the program's
  backend is built.
- **Codex's arm** keeps a `CODEX_HOME` of its attempt's own (benchmarks.md,
  6.1 and 10), whose access-only `auth.json` the harness writes from a
  Codex login of the harness's own: a `CODEX_HOME` the developer signs
  in once with Codex's own flow, and that Codex alone refreshes, never
  the person's `~/.codex`.
- **Claude Code's arm** runs with a configuration directory of the
  harness's, signed in once with `claude auth login`, never a copy of a
  login, which would intermediate a Claude.ai credential (jig's
  `draft/providers.md`, 3.1). The login is shared; each attempt's
  settings, instructions and MCP servers are its own, given by flags
  (`--settings`, `--strict-mcp-config`, `--setting-sources`), and its
  session found by its id. Attempts on one login run one at a time.
- **The credential guard** writes no person's own tool directory.

## 7. What stays

- **Grants by account and generation,** refreshed by the host before
  they lapse; values in no turn, fact, trace or charter.
- **The local host's one path for credentials,** whatever the source.
- **Notional spend** on a plan, priced at the preset's list prices
  (shell.md, 5.2).

## 8. Documents that change

- **`shell.md`:** 1; 2.1 (`smith-local-protocol` without borrowed
  logins' formats); 2.3 and 9 (a spawned agent for a program's run);
  2.4 (the driver's read under the lock); 3.1 (the licence); 4
  (`smith logout`, the status line, 4.3's rows); 4.5; 5.1
  (`credentials`, `account`, the repository layer); 5.2 (accounts,
  `--account`, providers, the presets); 5.3 (the refusals of 2.7); 6.1
  and 6.2 (the presets' endpoints, registration and model names); 7
  (sources); 10 (the world's credential stories).
- **`domain/host.md`,** 7, 8 and 9.1: the sources, notices, the local
  host's checks, and the inline agent's direct backend only.
- **`domain/session.md`,** 3, 4 and 8.2: limited, refused and
  unsupported, never retried; a window carried over.
- **`domain/run.md`,** 9 and 9.1: the reserve on the program's backend.
- **`domain/tools.md`:** Claude Code's built-in tools under smith's
  policy.
- **`protocol/agent.md`,** 1 and 4: the program's backend and
  `programs`; an endpoint names no account and no identity profile.
- **`protocol/charter.md`,** 2 and 5: a charter's LLM names its grant,
  and the charter's version.
- **`protocol/hosts.md`,** 3, 5.2, 5.3 and 5.4: the program's check, its
  session file in the chat, sign-in; `borrow` removed.
- **`protocol/channel.md`,** 4, 5 and 6: grants, sources reported,
  notices.
- **`protocol/llm.md`,** 6 and 7: limited, refused, unsupported; a
  grant's scheme.
- **`protocol/events.md`,** 3.9: `credential_rejected` and
  `account_exhausted` as limited, refused and unauthorized; a run on
  Claude Code's records.
- **`protocol/transcript.md`,** 2, 3, 4 and 6: the account in a window
  turn, a window carried over, and the version that allows it.
- **`testing.md`,** 2.3 and 4: the live suite of section 5; a fake
  `claude`; the fake issuer reached through the registration's issuer.
- **`benchmarks.md`,** 6, 7 and 10: the arms of section 6.
- **`protocol/README.md`,** 9: what smith needs from skein.

## 9. Order of work

0. **The MIT licence:** a licence file and each manifest's `license`
   (jig's `draft/llm-accounts.md`, section 11, step 1).
1. **skein first:** the dialects, credentials and failures, OAuth's
   additions, the driver's, and their fakes (jig's
   `draft/llm-accounts.md`, section 11).
2. **The product's accounts:** settings, sources, the credentials
   directory, the presets, `smith login` and `smith logout`, the lock,
   the refusals of 2.7.
3. **Grants and notices** across the channel and the hosts.
4. **The program's backend,** with its fake `claude` (jig's
   `draft/providers.md`, 7.6), and the product's spawned agent for it.
5. **Carrying a conversation over.**
6. **The live suite and the benchmarks' arms** on smith's own accounts.

## 10. Open questions

- **The plan's models:** which of the OpenAI models `/v1/models` lists
  for a ChatGPT plan, and whether the small and working tiers'
  (`gpt-6-luna`, `gpt-6.1-sol`) are among them. Default: the preset
  declares them, and a run on one the plan refuses fails as refused.
- **Claude Code's attempts at once on one login:** whether attempts that
  share a configuration directory interfere. Default: one at a time
  (section 6).
- **A chat's account between invocations:** a chat resumed with another
  `--account` of the same provider is carried over (4.4). Default:
  allowed, said once on the status line.
