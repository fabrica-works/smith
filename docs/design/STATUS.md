# Status

Updated 2026-10-16. How far smith's code is from its design, and which
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
| wall-clock | designed 2026-10-16, to plan | smith on par with Codex, RT included (`benchmarks.md` 16): the window's check bounding the next prompt, its appended bytes counted at a measured ratio, and a guard holding each prompt sent within the line, the history's bytes taken from the line (`domain/session.md` 4, 8.1 and 8.2, `protocol/limits.md` 2.1, 3.2, 3.9 and 4, `protocol/llm.md` 6, `protocol/charter.md` 6); the task role's batched edits and checks, polls beside other work and ranged reads (`shell.md` 6.6); a failed write stopping the writes after it, in transcript version 5 (`domain/session.md` 5, `protocol/llm.md` 3 and 5, `protocol/transcript.md` 2 and 6); a job's wait giving way to its batch (`domain/session.md` 5, `domain/tools.md` 3 and 5); spend priced per response, ten `shell-cwd` attempts, the `parity-change` suite and a change's report and stop, peak memory in summaries (`benchmarks.md` 1, 4.2, 5.4, 8.1, 11.5, 14, 16.1, 16.2 and 16.6; `shell.md` 5.2); memory returning when its use ends, and `memory` raised for room (`protocol/limits.md` 5 and 11, `testing.md` 6, `shell.md` 6.3) | `docs/development/plans/wall-clock/` |
| Claude Code, and smith on Anthropic | drafting | `benchmarks.md`, sections 6, 7 and 10: Claude Code as the third agent, smith's probes on Anthropic models. Reworked for the user's `~/.claude` login before it runs, after reliability | `~/src/rust/plans/reliability-plan/smith/11-claude.md (not versioned)` |

## 2. The design

Left names each section's unbuilt part and what builds it. The steps of
smith's reliability and parity plans that it named have all landed (each
plan's retrospective, sections 1 and 2), and what they left unmet is
below. "The wall-clock plan" is `docs/development/plans/wall-clock/`;
"its own plan, drafting" is section 1's Claude Code plan; "not planned"
is in section 3. Where a landed step left the design unmet, Left says
what is missing, then what the code does instead.

| Document | Built | Left |
|---|---|---|
| `domain/README.md` | partly | 4: `smith-mcp` and supervised processes with a view, not planned |
| `domain/host.md` | partly | 4: a contained tree, not planned. 8: MCP host tools, not planned. 12: an isolated mode, not planned |
| `domain/run.md` | partly | 5.1: MCP servers as a tool source, not planned. 5.3, 15: work beside a writing child, a child from its parent's prefix, not planned. 6, 9.2: mid-work messages and threshold notes, not planned |
| `domain/session.md` | partly | 5: a job's wait giving way to its batch, and a failed write stopping the writes and commands after it: a call's failure stops nothing today, and a `job` call waits its own wait whatever its batch holds (the wall-clock plan). 4, 8.1, 8.2: the check bounding the next prompt, its appended bytes counted at the window's measured ratio, the guard holding each prompt sent within the line, and a compaction's summary beside its prompt: the check reserves one `output` below the threshold and counts each appended byte as a token (`crates/smith-domain-session/src/window.rs`), no guard measures a request as rendered, and a compaction's owed results leave one `output` of the window for its summary (the wall-clock plan). 11: provider-native compaction, not planned |
| `domain/tools.md` | partly | 3, 5: a `job` call's wait ended early by its session (the wall-clock plan). 5: containment, not planned |
| `protocol/README.md` | partly | 5: `smith-mcp`, not planned. 9: contained trees, Codex routing extras, not planned. 4, 11: a connected agent, not planned |
| `protocol/agent.md` | partly | 3: the view, not planned |
| `protocol/channel.md` | partly | 2, 7: a connected agent's credential and refusals, not planned |
| `protocol/charter.md` | partly | 6, 8: the entrance's refusal of a negative spare (the wall-clock plan) |
| `protocol/events.md` | partly | 3.2: `containment: "trees"`, not planned |
| `protocol/hosts.md` | partly | 3: the tree, not planned. 4: connecting later, not planned |
| `protocol/limits.md` | partly | 2.1, 3.2, 3.9, 4: the line and the spare, a request's output beside its prompt, the history's bytes from the line with one whole completion's reserve, and the entrance's check of a charter's spare; 5, 11: memory shown returning when its use ends, and `memory` raised for room (the wall-clock plan) |
| `protocol/llm.md` | partly | 6: the guard, measuring a request's new part as rendered and refusing it unsent past the line. 3, 5: the descriptions' batched change and its check, a failed write leaving the writes after it not run, a poll beside other calls, and the not-run text naming the failed write (the wall-clock plan). 12: edit formats per dialect, not planned |
| `protocol/transcript.md` | partly | 2, 6: version 5, a not-run call's why after a failed write (the wall-clock plan) |
| `shell.md` | partly | 5.2: the tiers read per response by the benchmarks (the wall-clock plan). 6.3: `memory`'s status as the work needs. 6.6: the task role's topics 2 to 5, batched edits and their check, polls beside other work, ranged reads (the wall-clock plan). 5.1: directories made at startup synced into their parents, not planned. 8: per-tree limits, not planned. 9: `isolate`, not planned |
| `testing.md` | partly | 6: the event stream's check pairing each `running` verdict with one end: `tests/agent-process/src/events.rs` passes over `job.ended` unpaired, not planned; memory shown returning when its use ends (the wall-clock plan). 2.1: fault sweeps, not planned. 2.2: real-loop signals, not planned. 6: replay by digests, containment checks, not planned |
| `benchmarks.md` | partly | 1, 4.2, 11.5, 14, 16.6: the `parity-change` suite, a change's report and its stop; 5.4, 8.1, 16.1, 16.2: spend priced per response at its prompt's tier, ten `shell-cwd` attempts, peak memory and responses in summaries: the harness prices an attempt's total at the tier its declared window reaches (`benchmarks/src/result.rs`), and `parity` runs five (the wall-clock plan). 3.1, 5.2: each probe's and each variant's calibration in its task file: 8 blocks in 4 of the 15 probe directories, and six task files to recalibrate before `suite smoke` relies on them, not planned (the parity plan's `later.md`). 11.1: `mid-work`, `build-environment`, not planned. 11.3: the `delegation` experiment, not planned. 6: Claude Code (its own plan, drafting). 15: beyond serial runs, not planned |

## 3. Not planned

- **Directories made at startup, durably** (`shell.md`, 5.1, "State";
  skein's `shell.md`, 6, `make_root`, and `io.md`, 5): the shell makes
  the state root and each chat's directory
  (`crates/smith-local-shell/src/compose.rs:413`), and the token
  directory's parent (`credentials/sign_in.rs:228`), with std, and never
  syncs them into their parents, so a power loss can drop a chat's
  directory with the files synced beneath it. skein's io makes the token
  directory itself, and syncs it into its parent. skein's `make_root` has
  landed (skein 31366e0, skein-03.s1 of jig's jig-protocol), and smith's
  pinned skein has it: waits for a smith step that moves those sites onto
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
  and 9.2). It waited for the reliability plan's sessions 05 and 07,
  which have landed; it now waits for nothing. The `mid-work` probe
  comes with it.
- **Provider-native compaction** (`domain/session.md`, section 11).
  The `compaction` experiment ran on 2026-10-14 (the reliability plan's
  12.6), with no provider-native arm, and did not favour it. Waits for a
  compaction experiment after the parity plan fixes the compaction
  request (section 8.2).
- **Edit formats per dialect** (`protocol/llm.md`, section 12). Waits
  for the `edit-format` experiment, which no plan runs yet: the
  reliability plan's 12.6 had no second format per dialect to compare.
- **Codex routing extras and WebSocket** (`protocol/README.md`,
  section 9). Waits for the `affinity` experiment, which no plan runs
  yet, to show a material gain: the reliability plan's 12.6 ran none of
  its arms, two unavailable and the third unbuilt. WebSocket is on
  skein's roadmap.
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
  the probes and variants without a `[calibration]` block, and the six
  task files the parity plan's `later.md` names for recalibration before
  `suite smoke` relies on them. Waits for nothing but live budget.
- **Real-loop signals** (`testing.md`, section 2.2): the referee's real
  interrupt cancels a run, and a second stops its tree. Waits for
  nothing.
- **Replay by digests** (`testing.md`, section 6). Waits for skein's
  harness to take digests of the state.
- **Containment checks** (`testing.md`, section 6), with the
  `build-environment` probe (`benchmarks.md`, section 11.1). Wait for
  contained trees, cgroup v2 delegated to the user and unprivileged user
  namespaces.
- **The delegation experiment** (`benchmarks.md`, section 11.3): each
  agent with its sub-agents off, then both on, on `shell-cwd`. Waits for
  the parity plan's tasks and pins; it informs the user's open decision
  on smith's `sub_agents` default (the reliability plan's decisions.md,
  10.7).
- **Benchmarks beyond serial runs** (`benchmarks.md`, section 15):
  concurrent attempts, task kinds beyond code, a commit hook for smoke
  runs. Waits for the suites to outgrow serial runs.

## 4. Drafts

| Draft | About | Next |
|---|---|---|
| `draft/one-host.md` | smith's side of jig's one host: the inline agent | Nothing in smith: adopted into `domain/host.md`, section 9, built by session 03. jig's side is on jig's schedule |
