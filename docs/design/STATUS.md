# Status

Updated 2026-10-19. How far smith's code is from its design, and which
plans take it there. The designer keeps this file (skein's
development.md, section 3.3).

## 1. Plans

| Plan | State | Takes the design to | Where |
|---|---|---|---|
| jig's test-budgets | done 2026-10-14 | no section of smith's design: smith's focused suite back within its 15 s budget on carbon-forge (`docs/development/workflow.md`, 2; skein's `testing-strategy.md`, 8), and skein's read of what has arrived in smith's readers of unframed bytes (skein's `lib.md`, 7) | jig's `docs/development/plans/test-budgets/` |
| jig's jig-protocol | done 2026-10-13 | no section of smith's design: smith takes skein's in-process addresses, `io::Limits::names` and the kernel's opens (its smith 01, and a closing repin) | jig's `docs/development/plans/jig-protocol/` |
| Domain completion | done 2026-10-07 | `domain/`: the run, session and tools domains conformed; the local host's domain, with transcripts, resuming, delivery in place and recovery of answered calls | `~/src/rust/plans/domain-completion-plan/ (not versioned)` |
| Protocol layer | done 2026-10-08 | `protocol/` as first designed: the codecs, both halves of the channel, the LLM and machine components, the agent process, spawning hosts, the local host's protocol layer | `~/src/rust/plans/smith-protocol-layer/ (not versioned)` |
| Testing, first pass | done 2026-10-08 | `testing.md`'s tiers: simulated worlds on skein's harness, the real loop, the binary end to end against fakes, an opt-in live suite | `~/src/rust/plans/smith-testing/ (not versioned)` |
| reliability | done 2026-10-14 | the 2026-10-09 revision of every document, with `shell.md`, `benchmarks.md`, `protocol/events.md` and `protocol/limits.md`: what is left of sessions 00 and 03–12, and session 13, which proves live what its first run (2026-10-09 to 2026-10-12, `~/src/rust/plans/reliability-plan/`, not versioned) landed unproven | `docs/development/plans/reliability/` |
| parity | done 2026-10-15: ended short of parity at `87a8d1e8` on RT (benchmarks.md 16.5) | smith on par with Codex, as `benchmarks.md` section 16 defines it: the benchmark tasks, pins, price basis, summaries and verdict (`benchmarks.md` 3, 5, 6, 8–11, 16); jobs (`domain/tools.md` 3 and 5, `domain/session.md` 5, `domain/run.md` 5.3, 8.1 and 9.1, `domain/host.md` 4, `protocol/llm.md` 3, `protocol/events.md` 3.6, `protocol/limits.md` 2–7, `protocol/agent.md` 2, `protocol/transcript.md` 2 and 4); a compaction asked with `ToolChoice::None`, its summary framed (`domain/session.md` 8.2, `domain/run.md` 3.3); the task role (`shell.md` 5.2 and 6.6, `domain/run.md` 3.1, `protocol/limits.md` 4); a failed start that ends the command (`shell.md` 4.3); the tools' descriptions and the empty path (`protocol/llm.md` 3 and 4, `domain/tools.md` 2); `gpt-6.1-sol` (`shell.md` 6.1); skein's `fake-llm.md` 2.1 | `docs/development/plans/parity/` |
| wall-clock | done 2026-10-17 | smith on par with Codex, RT included (`benchmarks.md` 16): the window's check bounding the next prompt, its appended bytes counted at a measured ratio, and a guard holding each prompt sent within the line, the history's bytes taken from the line, the line's material among the effective limits in event vocabulary version 3 (`domain/session.md` 4, 8.1 and 8.2, `protocol/limits.md` 2.1, 3.2, 3.9, 4 and 9, `protocol/llm.md` 6, `protocol/charter.md` 6, `protocol/events.md` 6), and the `history` probe's condition at the line (`benchmarks.md` 11.1); the task role's batched edits and checks, polls beside other work and ranged reads (`shell.md` 6.6); a failed write stopping the writes after it, in transcript version 5 (`domain/session.md` 5, `protocol/llm.md` 3 and 5, `protocol/transcript.md` 2 and 6); a job's wait giving way to the rest of its response (`domain/session.md` 5, `domain/tools.md` 3 and 5); the task role counted as written in the host part's check (`protocol/limits.md` 4, `shell.md` 6.6); spend priced per response, a response failed unsent at nothing, ten `shell-cwd` attempts, the `parity-change` suite and a change's report and stop, peak memory in summaries (`benchmarks.md` 1, 4.2, 5.4, 8.1, 8.2, 11.5, 14, 16.1, 16.2 and 16.6; `shell.md` 5.2); memory returning when its use ends, and `memory` raised should the worst case pass it (`protocol/limits.md` 5 and 11, `testing.md` 6, `shell.md` 6.3) | `docs/development/plans/wall-clock/` |
| Claude Code, and smith on Anthropic | drafting | `benchmarks.md`, sections 6, 7 and 10: Claude Code as the third agent, smith's probes on Anthropic models. Reworked for the user's `~/.claude` login before it runs, after reliability | `~/src/rust/plans/reliability-plan/smith/11-claude.md (not versioned)` |

## 2. The design

Left names each section's unbuilt part, then the plan of section 1 that
builds it (": <plan>" after its items), or "not planned": section 3
says what each waits for. Where the code does other than the design,
Left says what is missing, then, in parentheses, what the code does
instead.

| Document | Built | Left |
|---|---|---|
| `domain/README.md` | partly | 4: `smith-mcp` and supervised processes with a view: not planned |
| `domain/host.md` | partly | 2, 3: live content, the ask and the host's half handing pieces to its owner: not planned (live text). 4: a contained tree: not planned. 8: MCP host tools: not planned. 12: an isolated mode: not planned (the reliability plan's `later.md`) |
| `domain/run.md` | partly | 5.1: MCP servers as a tool source: not planned. 5.3, 15: work beside a writing child, a child from its parent's prefix: not planned (the reliability and parity plans' `later.md`). 6, 9.2: mid-work messages and threshold notes: not planned (the reliability plan's `later.md`) |
| `domain/session.md` | partly | 11: provider-native compaction: not planned (the reliability plan's `later.md`) |
| `domain/tools.md` | partly | 5: containment: not planned |
| `protocol/README.md` | partly | 5: `smith-mcp`: not planned. 9: contained trees, Codex routing extras: not planned (the reliability and parity plans' `later.md`). 4, 11: a connected agent: not planned |
| `protocol/agent.md` | partly | 3: the view: not planned |
| `protocol/channel.md` | partly | 2, 7: a connected agent's credential and refusals: not planned. 3, 4, 5, 7, 8: the ask in the start and live content in pieces, with their order and best effort: not planned (live text) |
| `protocol/charter.md` | yes | Nothing |
| `protocol/events.md` | partly | 3.2: `containment: "trees"`: not planned |
| `protocol/hosts.md` | partly | 3: the tree: not planned. 4: connecting later: not planned |
| `protocol/limits.md` | partly | 2.3, 2.4, 3.6: the piece, the interval and the held bytes of live content: not planned (live text) |
| `protocol/llm.md` | partly | 6: deltas handed over for the channel's ask: not planned (live text). 12: edit formats per dialect: not planned |
| `protocol/transcript.md` | yes | Nothing |
| `shell.md` | partly | 5.1: directories made at startup synced into their parents: not planned. 8: per-tree limits: not planned. 9: `isolate`: not planned (the reliability plan's `later.md`) |
| `testing.md` | partly | 2.1: fault sweeps: not planned. 2.2: real-loop signals: not planned. 6: the event stream's check pairing each `running` verdict with one end (`tests/agent-process/src/events.rs` passes over `job.ended` unpaired), replay by digests, containment checks: not planned |
| `benchmarks.md` | partly | 3.1, 5.2: each probe's and each variant's calibration in its task file (8 blocks in 4 of the 15 probe directories), with six task files and the two `history` variants to recalibrate before `suite smoke` relies on them: not planned (the parity and wall-clock plans' `later.md`). 11.1: `mid-work`, `build-environment`: not planned. 11.3: the `delegation` experiment: not planned. 15: beyond serial runs: not planned. 16.6: a change's projection on its own ratio (`summarise` multiplies in the ratios of the landed changes named to it, `benchmarks/src/parity/change.rs`): not planned. 6: Claude Code: Claude Code, and smith on Anthropic |

## 3. Not planned

- **Directories made at startup, durably** (`shell.md`, 5.1, "State";
  skein's `shell.md`, 6, `make_root`, and `io.md`, 5): the shell makes
  the state root and each chat's directory
  (`crates/smith-local-shell/src/compose.rs:413`), and the token
  directory's parent (`credentials/sign_in.rs:228`), with std, and never
  syncs them into their parents, so a power loss can drop a chat's
  directory with the files synced beneath it. skein's io makes the token
  directory itself, and syncs it into its parent. smith's pinned skein
  has `make_root`: waits for a smith step that moves those sites onto
  it. The temporary directories of `compose/files.rs` stay: nothing
  relies on them past a crash.
- **Contained trees** (`protocol/agent.md`, sections 3 and 7; also
  `shell.md`, 8, `domain/host.md`, 4 and 9.3, `protocol/hosts.md`, 3 and
  5.5): searches, commands, checks and agents in trees with views, git
  directories read-only, write scopes, network policy, per-tree limits
  and build profiles. Waits for a skein design and plan of their own
  (skein's `draft/process.md`); until then, plain children with process
  groups.
- **An isolated mode** (`shell.md`, section 9; `domain/host.md`,
  section 12): the product's `isolate`, the spawned agent as a
  containment boundary. Waits for contained trees.
- **Work beside a writing child** (`domain/run.md`, section 5.3): a
  read-only parent beside one writing child, then scoped write sets,
  then asynchronous children. Waits for contained trees and a document
  of its own.
- **A child from its parent's prefix** (`domain/run.md`, section 15).
  Waits for the `parallel-children` and `budget` probes to show children
  re-orienting at a material cost.
- **Messages mid-work and threshold notes** (`domain/run.md`, sections 6
  and 9.2). Waits for nothing. The `mid-work` probe comes with it.
- **Provider-native compaction** (`domain/session.md`, section 11).
  Waits for a `compaction` experiment with a provider-native arm, which
  no plan runs yet, to favour it.
- **Edit formats per dialect** (`protocol/llm.md`, section 12). Waits
  for the `edit-format` experiment, which no plan runs yet (the
  wall-clock plan's `later.md`).
- **Codex routing extras and WebSocket** (`protocol/README.md`,
  section 9). Waits for the `affinity` experiment, which no plan runs
  yet, to show a material gain. WebSocket is on skein's roadmap.
- **MCP as a tool source** (`protocol/README.md`, sections 5 and 11;
  `domain/run.md`, section 5.1): `smith-mcp`, and through it the local
  host's first host tools. Waits for its design.
- **A connected agent** (`protocol/hosts.md`, section 4;
  `protocol/channel.md`, sections 2 and 7): a daemon a host connects to,
  with its credential and refusals. Waits for the domain's contract:
  admission, cancellation, a lost connection and the daemon's own life.
- **The testing cleanup pass** (`testing.md`, sections 2.1 and 6;
  smith-testing's `later.md`): fault sweeps asserting each fault fell;
  `tests/protocol-llm` shrunk to a protocol world; referees for
  `tests/run`, `tests/tools` and `tests/host` beyond the rules they hold;
  a focused memory test for `tests/agent`; tiny
  limits; a failed write to the agent kept apart from a malformed agent;
  the event stream's check pairing each `running` verdict with one end
  (`tests/agent-process/src/events.rs`). Waits for nothing.
- **The probes' calibrations** (`benchmarks.md`, sections 3.1 and 5.2):
  the probes and variants without a `[calibration]` block, the six task
  files the parity plan's `later.md` names for recalibration, and
  `history` and `history/small-window`, whose blocks the window's check
  (`domain/session.md`, 8.1) has outdated (the wall-clock plan's
  `later.md`), each before `suite smoke` relies on them. Waits for
  nothing but live budget.
- **Real-loop signals** (`testing.md`, section 2.2): the referee's real
  interrupt cancels a run, and a second stops its tree. Waits for
  nothing.
- **Replay by digests** (`testing.md`, section 6): skein's harness
  carries state digests, and smith's pinned skein has them (skein's
  `testing.md`, 5: `state_digest` on `Host` and `Machine`,
  `Digests::Traced`), but no smith host or fake machine gives one.
  Waits for nothing: a smith step that hashes each process's domains
  and each fake machine's state, and replays its simulated worlds with
  them.
- **Containment checks** (`testing.md`, section 6), with the
  `build-environment` probe (`benchmarks.md`, section 11.1). Wait for
  contained trees, cgroup v2 delegated to the user and unprivileged user
  namespaces.
- **The delegation experiment** (`benchmarks.md`, section 11.3): each
  agent with its sub-agents off, then both on, on `shell-cwd`. Waits for
  a plan of its own (the parity plan's `later.md`); it informs the user's open decision
  on smith's `sub_agents` default (the reliability plan's decisions.md,
  10.7).
- **Benchmarks beyond serial runs** (`benchmarks.md`, section 15):
  concurrent attempts, task kinds beyond code, a commit hook for smoke
  runs. Waits for the suites to outgrow serial runs.
- **A change's projection on its own ratio** (`benchmarks.md`, section
  16.6): `summarise` still multiplies in the ratios of the landed
  changes named to it (`benchmarks/src/parity/change.rs`). Waits for the
  next plan that measures a change against its parent; until then, a
  plan names no landed change to it.

- **A run's live text** (`protocol/channel.md`, sections 3, 4, 5, 7 and
  8; `protocol/llm.md`, section 6; `protocol/limits.md`, 2.3, 2.4 and
  3.6; `domain/host.md`, sections 2 and 3): the start's ask, and the
  main conversation's text up the channel in pieces, best effort, which
  the agent's service coalesces from the LLM component's deltas; the
  host's half decodes them and hands them to its owner. The code sends
  only the count of the text (`TextArrived`). Designed; it waits for the
  downstream repository's
  live-text plan, after that repository's tools plan and the LLM
  accounts work. Reasoning and a tool call's input as they stream have
  schemas in the channel and no producer: later work.

## 4. Drafts

| Draft | About | Next |
|---|---|---|
| `draft/one-host.md` | smith's side of jig's one host: the inline agent | Nothing in smith: adopted into `domain/host.md`, section 9, built by session 03. jig's side is on jig's schedule |
| `draft/accounts.md` | smith's side of jig's `draft/llm-accounts.md`: the product as its own client (accounts by provider and source, several per provider, smith's own Sign in with ChatGPT registration in the `openai` preset, Claude Code as a program, a credentials directory, refreshes locked across processes, `smith login` and `smith logout`); the agent's two backends, grants and notices; a conversation carried over between providers; live tests and benchmarks on smith's own accounts | not adopted: review; then a design pass in the documents its section 8 names, after skein's design pass for jig's `draft/llm-accounts.md` |
