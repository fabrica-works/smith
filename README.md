# smith

*Protocol-driven agent loop for AI applications.*

A Rust kit for LLM agents that a host drives over a protocol, sending
each run its instructions, tools, budget and what counts as done.

## The idea

- **Policy is data.** A host shapes a run entirely by its charter and its
  answers: instructions, a brief, the tools the agent may call and those
  the host serves, what counts as done, a budget. smith knows no workflow
  vocabulary: no roles, task kinds, forges or pull requests.
- **Not only a coding agent.** A chat assistant, a researcher, a
  reviewer, an operator acting through its host's tools, a coordinator of
  other agents and a producer of code changes are runs of the same loop,
  told apart by their charters. A workspace is optional.
- **One vocabulary, in a process or inline.** An agent runs as a process
  its host spawns, over one channel, or as smith's domains inside the
  host's own process. The host speaks the same vocabulary either way.
- **Any model, any provider.** The conversation is provider-neutral, and
  a run may give its sub-agents other models than its own.
- **Failures are visible.** The LLM sees what went wrong, bounded in
  size, and so does the person running it, in words, never replaced by a
  fixed message.
- **Built on skein, tested whole.** Every part is a domain, a protocol
  layer and io, so a host imports the layer it needs; a world of domains
  and fakes runs everything an agent does, with no io at all.

## Start here

- [`docs/design/domain/README.md`](docs/design/domain/README.md): the
  run, the session, tools and the host's contract.
- [`docs/design/protocol/README.md`](docs/design/protocol/README.md):
  the channel, the charter, transcripts, LLM calls and hosts, in bytes.
- [`docs/design/STATUS.md`](docs/design/STATUS.md): how far the code is
  from the design.
- skein's foundation, which smith follows: `programming-model.md`,
  `testing-strategy.md` and `notes.md`.

Before changing code, read [`AGENTS.md`](AGENTS.md) and
[`docs/development/workflow.md`](docs/development/workflow.md). `main`
moves only through skein's `land`, which runs the gate,
`scripts/check.sh`.

## Live tests

The `live` profile runs smith's account stories and the Claude Code probes
one at a time, without retries. It stays outside the default and fuzzy
suites. Install `claude` on `PATH` and `/usr/bin/python3` for the
prompt-shape probe's MCP fixture, then prepare a private directory and
its user layer once per machine:

```sh
export SMITH_LIVE=~/.local/state/smith-live
umask 077
mkdir -p "$SMITH_LIVE/config/smith"
cat > "$SMITH_LIVE/config/smith/config.toml" <<EOF
credentials = "$SMITH_LIVE"

[accounts.chatgpt]
provider = "openai"
source = "sign_in"

[accounts.claude]
provider = "anthropic"
source = "program"

[accounts.anthropic-key]
provider = "anthropic"
source = "env"
variable = "ANTHROPIC_API_KEY"
EOF
XDG_CONFIG_HOME=$SMITH_LIVE/config smith login chatgpt  # approve in the browser
XDG_CONFIG_HOME=$SMITH_LIVE/config smith login claude   # Claude Code's sign-in
cargo nextest run --profile live
```

Set `ANTHROPIC_API_KEY` for the key story. A missing setup fails by name;
each story checks only the accounts it uses. To run an account's stories
alone, select them with nextest's test filter. An optional
`SMITH_TEST_LIVE_GIT_REMOTE` names a writable remote for the push story;
without it that story reports the omission on stderr and passes. The
push story removes its remote branch, including on a failed assertion.

The directory holds `host-id`, `tokens/chatgpt`, `programs/claude/` and
`config/smith/config.toml`. Smith refreshes its own records in place under
their locks. Each test keeps its HOME, workspace and chats in scratch;
the suite never rewrites the live user layer or borrows a personal login.
A directory unused past the issuer's refresh-token lifetime needs another
sign-in. See [testing.md, section 2.3](docs/design/testing.md#23-end-to-end).

## The family

These are the repositories of [Fabrig](https://github.com/fabrica-works), an
agent-driven software factory: write, prove, run.

| Repository | | Builds on |
|---|---|---|
| [skein](https://github.com/fabrica-works/skein) | One thread, one loop, no async | |
| [smith](https://github.com/fabrica-works/smith) | Protocol-driven agent loop for AI applications | skein |
| [jig](https://github.com/fabrica-works/jig) | Engine for agentic applications | skein, smith |
| [temper](https://github.com/fabrica-works/temper) | Agent-driven software development on a forge | skein, smith, jig |
| [hearth](https://github.com/fabrica-works/hearth) | Immutable machines, built from a revision | skein |
