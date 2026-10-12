# Status

Updated 2026-10-12. How far smith's code is from its design, and which
plans take it there. The designer keeps this file (skein's
development.md, section 3.3).

## 1. Plans

| Plan | State | Takes the design to | Where |
|---|---|---|---|
| jig's jig-protocol | active on carbon-forge since 2026-10-12 | no section of smith's design: smith takes skein's in-process addresses, `io::Limits::names` and the kernel's opens (its smith 01, and a closing repin) | jig's `docs/development/plans/jig-protocol/` |
| Domain completion | done 2026-10-07 | `domain/`: the run, session and tools domains conformed; the local host's domain, with transcripts, resuming, delivery in place and recovery of answered calls | `~/src/rust/plans/domain-completion-plan/ (not versioned)` |
| Protocol layer | done 2026-10-08 | `protocol/` as first designed: the codecs, both halves of the channel, the LLM and machine components, the agent process, spawning hosts, the local host's protocol layer | `~/src/rust/plans/smith-protocol-layer/ (not versioned)` |
| Testing, first pass | done 2026-10-08 | `testing.md`'s tiers: simulated worlds on skein's harness, the real loop, the binary end to end against fakes, an opt-in live suite | `~/src/rust/plans/smith-testing/ (not versioned)` |
| reliability | active on debian-16gb-hel1-1 since 2026-10-12 | the 2026-10-09 revision of every document, with `shell.md`, `benchmarks.md`, `protocol/events.md` and `protocol/limits.md`: what is left of sessions 00 and 03–12, and session 13, which proves live what its first run (2026-10-09 to 2026-10-12, `~/src/rust/plans/reliability-plan/`, not versioned) landed unproven | `docs/development/plans/reliability/` |
| Claude Code, and smith on Anthropic | drafting | `benchmarks.md`, sections 6, 7 and 10: Claude Code as the third agent, smith's probes on Anthropic models. Reworked for the user's `~/.claude` login before it runs, after reliability | `~/src/rust/plans/reliability-plan/smith/11-claude.md (not versioned)` |

## 2. The design

Left names each section's unbuilt part and what builds it: "smith 06.1"
is the reliability plan's session 06, increment 06.1, and "skein 02.6"
its skein session's; "not planned" is in section 3.

| Document | Built | Left |
|---|---|---|
| `domain/README.md` | partly | 4: the product's commands, its agent inline (smith 10.3, 10.5, 10.6, 10.8); skein's OAuth driver and durable replace adopted (smith 10.1, 10.6, 10.7); `smith-mcp` and supervised processes with a view, not planned |
| `domain/host.md` | partly | 4: exit evidence, the watchdog over silence, the derived grace (smith 06.1, 06.3, 06.6); a contained tree, not planned. 6: resuming from the last window (smith 08.2). 7: `sign_in` on skein's driver (smith 10.6), `borrow`, `env` (smith 10.7). 8: a forced exit after any answer a warning (smith 06.9); typed notices, lines carried and held, "waiting" (smith 05.11, 05.13, 05.14); failures in words (smith 06.9); headless (smith 10.4); the agent inline (smith 10.5, 10.8); MCP host tools, not planned. 9: the inline run's stream (smith 03.7); a dropped run's record, the second interrupt (smith 03.8). 12: an isolated mode, not planned |
| `domain/run.md` | partly | 3.3: the summary instruction (smith 08.6). 6: mid-work messages, not planned. 9.2, 11: the `budget` record a usage line tells (smith 07.14). 10: a model failure's limit, phase, status, model and attempts (smith 06.7); a compaction that gives no summary failing the run with its fault (smith 08.6). 11: compactions (smith 08.6). 5.3, 15: work beside a writing child, a child from its parent's prefix, not planned |
| `domain/session.md` | partly | 3, 4, 7, 8: the window check and compaction, for a resumed history, *context too long* and a child's final turn, an answer that gives no summary failing with its fault, and the window's limit keeping its cause (smith 08.6); owed results cut to their room (smith 08.7); history at its bounds compacted, a completion's later batches kept within it (smith 08.9). 4: the last failure's detail kept (smith 06.12). 11: provider-native compaction, not planned |
| `domain/tools.md` | partly | 5: containment, not planned. 8: a background-process tool, not planned |
| `protocol/README.md` | partly | 4: the product in one process (smith 03.7, 10.5, 10.8). 5: `smith-mcp`, not planned. 8: versions in `smith check` (smith 10.3). 9: skein's durable replace, trust roots and OAuth driver adopted (smith 10.1, 10.2, 10.6, 10.7); contained trees, Codex routing extras, not planned. 4, 11: a connected agent, not planned |
| `protocol/agent.md` | partly | 3: the view, not planned. 6: exit evidence (smith 06.1), the stderr line (smith 06.2, 06.12), the teardown phase (smith 06.4), a signal after the answer (smith 06.5). 8: shipped times (smith 06.10); memory at its worst case (smith 04.13) |
| `protocol/channel.md` | partly | 3: turns sent from the last window (smith 08.2); the start's transcript in one piece (smith 04.9). 4: a model failure's limit, phase, status and attempts (smith 06.7). 7: the queues derived (smith 04.9). 2, 7: a connected agent's credential and refusals, not planned |
| `protocol/charter.md` | partly | 5: versions in `smith check` (smith 10.3) |
| `protocol/events.md` | partly | The agent's stream is built. 2, 3.3, 3.9, 4, 8: the product's stream (smith 03.7–03.10). 3.2: effective limits (smith 04.7, 04.21, 04.22); the teardown's fields (smith 06.4, 06.5); `exec` (smith 10.5). 3.4–3.5: compactions (smith 08.6). 3.8: a usage line's `budget` record (smith 07.14). 5.2: `smith exec`'s sinks (smith 10.5). `containment: "trees"`, not planned |
| `protocol/hosts.md` | partly | 3: exit evidence, no signal before the grace (smith 06.1, 06.6); the tree, not planned. 4: connecting later, not planned. 5.1: notices, lines, "waiting" (smith 05.11, 05.13, 05.14); failures in words (smith 06.9); the second interrupt (smith 03.8); headless (smith 10.4, 10.5). 5.2: TOML settings (smith 10.3, 10.13), the derived grace (smith 06.6). 5.3: the store over io (smith 10.1), resuming from the last window (smith 08.2). 5.4: skein's OAuth driver, `borrow`, `env` (smith 10.6, 10.7). 5.6: the inline agent by default (smith 10.5, 10.8), its stream (smith 03.7) |
| `protocol/limits.md` | partly | 2.1–2.3: the profile at `shell.md`'s values (smith 04.14); the exit grace derived (smith 06.6). 3: the transcript, the channel, the host kit and io derived (smith 04.9); owed results (smith 08.7). 4: three checks (smith 04.9), the two across processes (smith 06.3, 06.6); `cancel-over-close` (smith 04.14). 5: `memory` made real (smith 04.13). 6: compaction (smith 08.6, 08.7). 7: the teardown bound (smith 06.6). 8: typed limits carried (smith 06.7). 9: effective limits (smith 04.7, 04.21, 04.22, 10.14) |
| `protocol/llm.md` | partly | 5: results owed to a compaction (smith 08.7). 6: the HTTP status and the last failure's detail carried (smith 06.7, 06.12); closing at the answer (smith 06.4). 8: the worst case against `memory` (smith 04.13). 12: edit formats per dialect, not planned |
| `protocol/transcript.md` | partly | 3: a host resuming from the last turn with `opens_window` (smith 08.2). 7: the turn and resume limits derived, checked by name, and a search's hits as one piece (smith 04.9) |
| `shell.md` | partly | 2.1–3.2: built, but for the inline composition (smith 10.5). 2.4: terminal modes (skein 02.6); skein's durable replace, private files, OAuth driver and trust roots adopted (smith 10.1, 10.2, 10.6, 10.7). 4: the commands and exit codes (smith 10.3–10.6, 10.8, 10.10, 10.14; the last line's words, smith 06.2). 5: settings in TOML, the repository layer's refusals the only trust boundary, tiered list prices at their highest reachable rates, the run's models and needed accounts (smith 10.3, 10.13, 10.16, 10.17). 6: presets, run defaults (smith 10.13), the profile's values (smith 04.14). 7: credentials (smith 10.6, 10.7). 8: the build environment (smith 10.9); per-tree limits, not planned. 9: one process (smith 03.7, 03.8, 06.8, 10.5, 10.16); `isolate`, not planned |
| `testing.md` | partly | 2.1: fault sweeps, not planned. 2.2: real-loop signals, not planned. 2.3: `exec`, `check`, `login` and interrupt stories (smith 10.3, 10.5, 10.6, 10.8), live stories through `smith exec` (smith 10.15). 2.4: benchmarks (smith 00, 12, 13). 2.5: `standard` deriving and fitting `memory` (smith 04.13, 04.14). 5: the reacting person, message referees (smith 05.12–05.14). 6: the product's events complete (smith 03.7, 03.9); shipped times (smith 06.10); replay by digests, containment checks, not planned |
| `benchmarks.md` | partly | 5.1–5.2, 11.1: one command per run, the scripted person, `follow-up`, `history`, `credentials`, `crossing`, the told check (smith 00.27, 00.28, 00.14, 00.16, 00.30, 00.17); calibrations (smith 13.1–13.7). 6.2, 9: the first baseline on events (smith 00.21). 11.4–11.5: fixtures 01–02, `probes` (smith 00.13, 00.29). 8.1, 9, 11: delegation, comparisons, fixture 03, `shell-cwd`, experiments (smith 12.1–12.7, 12.9, 12.10). 6: Claude Code (its own plan, drafting). 11.1: `mid-work`, `build-environment`, not planned. 15: beyond serial runs, not planned |

## 3. Not planned

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
  and 9.2). Waits for sessions 05 and 07; the `mid-work` probe comes
  with it.
- **A background-process tool** (`domain/tools.md`, section 8): start,
  poll, stop. Waits for contained trees.
- **Provider-native compaction** (`domain/session.md`, section 11).
  Waits for the `compaction` experiment (session 12) to favour it.
- **Edit formats per dialect** (`protocol/llm.md`, section 12). Waits
  for the `edit-format` experiment, which no plan runs yet.
- **Codex routing extras and WebSocket** (`protocol/README.md`,
  section 9). Waits for the `affinity` experiment, which no plan runs
  yet, to show a material gain; WebSocket is on skein's roadmap.
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
  limits; a failed write to the agent kept apart from a malformed agent.
  Waits for nothing.
- **Real-loop signals** (`testing.md`, section 2.2): the referee's real
  interrupt cancels a run, and a second stops its tree. Waits for
  nothing.
- **Replay by digests** (`testing.md`, section 6). Waits for skein's
  harness to take digests of the state.
- **Containment checks** (`testing.md`, section 6), with the
  `build-environment` probe (`benchmarks.md`, section 11.1). Wait for
  contained trees, cgroup v2 delegated to the user and unprivileged user
  namespaces.
- **Benchmarks beyond serial runs** (`benchmarks.md`, section 15):
  concurrent attempts, task kinds beyond code, a commit hook for smoke
  runs. Waits for the suites to outgrow serial runs.

## 4. Drafts

| Draft | About | Next |
|---|---|---|
| `draft/one-host.md` | smith's side of jig's one host: the inline agent | Nothing in smith: adopted into `domain/host.md`, section 9, built by session 03. jig's side is on jig's schedule |
