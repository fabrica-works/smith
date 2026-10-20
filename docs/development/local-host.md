# Running the product

`smith` resumes a workspace chat and runs its agent in the same process.
The contracts are [shell.md](../design/shell.md),
[protocol/hosts.md](../design/protocol/hosts.md) and
[domain/host.md](../design/domain/host.md).

## Commands

```sh
smith [--plain] [SHARED FLAGS]
smith exec [--json] [--deliver] [--ephemeral] [-o FILE] [SHARED FLAGS] [PROMPT | -]
smith check [--json] [SHARED FLAGS] [exec [EXEC FLAGS]]
smith login ACCOUNT [SHARED FLAGS]
```

Shared flags are `-C DIR`, `-m ENDPOINT/MODEL`, `--effort EFFORT`,
`--chat NAME`, `--trace FILE` and repeated `-c KEY=VALUE` overrides.
`-C` selects the workspace. The product composes the native configuration
in memory; it writes no agent configuration file.

In line mode, a completed line is one message. A trailing backslash continues
it on the next line. A line beginning `//` sends one literal slash. Other
slash lines are local commands: `/model ENDPOINT/MODEL`, `/effort EFFORT`,
`/new [NAME]`, `/resume NAME` and `/quit`. They run only after the live
activation has answered or parked and its store work is idle. `/resume`
refuses an absent chat or a bad name. A model on another endpoint is refused
with the invocation to start it. An incompatible effort is dropped on a
model switch, with a line naming it and the new model's declared values.

The screen shows text, tool verdicts and durations, notices, usage, spend and
budget standing. The waiting prompt appears when the input read fence is
open. `--plain`, and input without a terminal, select plain line output.
The first interrupt cancels the run; a second drops it. End of input lets a
run answer or park, then exits; one unread message can start the next
activation before that exit.

`exec` reads its one prompt argument, or all standard input for `-` (also the
default when no prompt is supplied). It prints the last model text;
`--json` prints the event stream instead. `-o` durably replaces a result
file with that text. `--deliver` enables a change result and its checked
commit; an ordinary exec produces a report. `--ephemeral` uses a private
chat directory and removes its own files after settlement. Foreign files
are kept and named in a warning.

`check` resolves and validates the interactive invocation, or the exec
invocation after `exec`, without opening its destinations or starting a
run. Its own `--json` chooses the report format. The report lists sourced
settings, account availability and effective limits by name and unit.

## Settings and accounts

Settings are strict TOML. Built-ins are followed by the user layer
`$XDG_CONFIG_HOME/smith/config.toml` (or `$HOME/.config/smith/config.toml`),
the workspace's `.smith/config.toml`, ordered `-c` overrides and flags.
Tables merge and values replace. A repository layer can shape the run but
cannot configure accounts, endpoints, trust, inherited environment,
deployment limits, writable caches or trace destinations. Refusals name
the key and its source.

```toml
model = 'codex/gpt-6-luna'
effort = 'low'

[accounts.codex]
source = 'borrow'
file = '/path/to/login.json'
format = 'codex'
program = 'codex'
```

A `borrow` source reads another tool's access token without writing or
refreshing its login. An `env` source names its variable. A `sign_in`
source supplies its public-client registration under `[accounts.NAME]`:
`client_id`, `authorization_url`, `token_url`, loopback `redirect_uri`,
`scope` and `token_format` (`json` or `form`). Optional `trust` chooses
DER or PEM roots for the token endpoint; otherwise machine roots apply.
No preset supplies a registration. Missing keys are refused at check and
startup. See [shell.md, section 7](../design/shell.md) for those fields.

`login` shows the authorization address for a sign-in account and waits for
the browser redirect. Private records live under
`$XDG_STATE_HOME/smith/tokens`, or `$HOME/.local/state/smith/tokens`.
The native keeper owns their format and durable replacement. The shell
neither stores tokens in settings nor prints them. Exec never signs in;
without a usable record it names `smith login ACCOUNT` and exits 3.

Unknown models need declarations, including positive prices, under
`[models."ENDPOINT/MODEL"]`. Optional `efforts` declares accepted names,
each within 32 bytes. A model declaring none accepts any bounded effort;
a provider refusal then becomes the request's failure. Presets declare
no effort lists. Profiles derive limits and check memory before a run.
The defaults remain five conversations and 2,176 MiB.

## Durable chats, traces and exits

Chats live in private directories under
`$XDG_STATE_HOME/smith/chats/<workspace-key>/<chat>`, or the same suffix
under `$HOME/.local/state`. The workspace key derives from its resolved
path. State, numbered turn files and delivery records retain their layout
within each chat. Acknowledgements follow file sync, rename and directory
sync. Restart preserves unknown delivery intents for reconciliation.

Git deliveries use the configured result fields and a stable
`Smith-Delivery` trailer. An uncertain effect is inspected before another
commit can be attempted. Tools use the selected workspace roots and the
explicit configured environment.

`--trace FILE` appends product records to that private path. Its destination
capture and delivery policy remain independent of the screen's bounded
best-effort live view. A host using the library's spawned agent capability
runs `smith-agent CONFIG.json` with generated configuration and derived
`exit_grace`; its agent trace appends `.agent` to the product trace path.
That generated agent JSON is a host-to-agent format.

Exit codes are 0 for accepted, 1 for run/host/output failure, 2 for usage or
configuration, 3 for credentials, 4 for budget, 5 for needs input and 130
for interruption. Standard error ends with the typed outcome in words.
An accepted run whose result file cannot be written exits 1; other answers
keep their code and show that file failure as a warning before the last
line. `session.ended` carries the process's resulting exit code after its
output and lower owners have settled.
