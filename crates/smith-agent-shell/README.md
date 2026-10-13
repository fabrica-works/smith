# Agent configuration

Run one agent with `smith-agent CONFIG.json`. The JSON file is read once before
the channel on standard input and output opens. Unknown fields, missing fields,
oversized files, invalid endpoints and a worst case above `profile.declared.memory` stop
startup with a reason on standard error.

```json
{
  "profile": {
    "name": "standard",
    "declared": { "memory": 1000000000000 },
    "policy": { "group_stop": 250 }
  },
  "endpoints": [
    {
      "name": "codex",
      "number": 1,
      "dialect": 1,
      "account": 0,
      "provider": "codex",
      "connect": 10000,
      "handshake": 10000,
      "models": [{ "name": "gpt-5.5", "window": 8192, "output": 4096,
        "prices": { "input": 1, "cached": 1, "output": 1, "unit": 1000000 },
        "reasoning_item": 2048, "head": 60000, "idle": 30000 }],
      "address": "chatgpt.com:443",
      "transport": "tls",
      "server_name": "chatgpt.com"
    }
  ],
  "environment": [
    { "name": "PATH", "value": "/usr/bin:/bin" }
  ],
  "trace": { "path": "/tmp/smith-agent.jsonl", "capture": "calls" }
}
```

The `standard` profile derives the domain, channel, LLM, machine, IO and
queue limits from deployment and model declarations. Each model supplies
positive prices per `unit` tokens; omitted `cache_write` uses `input`. `profile.declared.memory`
is the checked process ceiling, not an allocation. `profile.policy.group_stop`
is the process stop grace in milliseconds. Children use Skein's supervised
process trees. An endpoint may also set
`authority`, `target`, `headers` (an array of `{ "name", "value" }`),
`identity` (`plain` or `claude-code`). Reasoning effort is supplied by each charter’s LLM.
`trust_der` is an optional path to one DER root certificate; without it, the
system trust store is loaded. `transport` is `"tls"` by default, requiring
`server_name` and using the configured trust. For a server on this machine,
set `"transport": "plaintext"` and an address resolving to loopback (IPv4 or
IPv6); omit `server_name` and `trust_der`. Any non-loopback plaintext
destination, unknown transport or TLS-only field on plaintext refuses startup
before the channel opens. The local host carries the same inline endpoint
settings into its child's configuration. Addresses are resolved before the
channel opens. The file contains no bearer tokens; the host sends grant values
over the channel.

The optional trace appends version-one JSON lines through the service's IO
stream. `none` records content-free facts; `calls` adds call inputs and failure
details; `everything` adds request deltas, completions, results and live text.
The default `delivery` is `complete`, which holds records until the stream
has room. `best_effort` drops and counts records it cannot retain. Each
abandoned destination is named in the final diagnostics. The trace never
receives grant values. See [the event contract](../../docs/design/protocol/events.md)
for content bounds and delivery terminals.

The shared agent shell is `smith_agent_shell::Agent`. `Agent::read` reads this
configuration; `Agent::new` takes an already parsed `config::Configuration`.
Both take `Resources` (channel input/output, signal descriptor and injected
seed) and an error writer. The writer receives startup and final diagnostics
once. `Resources::roots` supplies inherited mount descriptors in Start order
for hosted worlds; omitted roots are opened on the machine, as in the binary.
The caller owns any provided roots that the Start does not consume.
`Agent` implements Skein's `Host`: the caller reaps into `completions`, calls
`iterate(now, wall)`, and submits `submissions`. That pass drives the same
event streams and opens/adopts the same roots as the binary. `result` is available
only after the channel and every lower child have settled.
`smith_agent_shell::run` is the binary's kernel-and-clock driver; its failures are
already written to the supplied error output.
