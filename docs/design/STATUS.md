# Status

Updated 2026-10-15. How far smith's code is from its design, and which
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
| parity | active on debian-16gb-hel1-1 since 2026-10-14 | smith on par with Codex, as `benchmarks.md` section 16 defines it: the benchmark tasks, pins, price basis, summaries and verdict (`benchmarks.md` 3, 5, 6, 8–11, 16); jobs (`domain/tools.md` 3 and 5, `domain/session.md` 5, `domain/run.md` 5.3, 8.1 and 9.1, `domain/host.md` 4, `protocol/llm.md` 3, `protocol/events.md` 3.6, `protocol/limits.md` 2–7, `protocol/agent.md` 2, `protocol/transcript.md` 2 and 4); a compaction asked with `ToolChoice::None`, its summary framed (`domain/session.md` 8.2, `domain/run.md` 3.3); the task role (`shell.md` 5.2 and 6.6, `domain/run.md` 3.1, `protocol/limits.md` 4); a failed start that ends the command (`shell.md` 4.3); the tools' descriptions and the empty path (`protocol/llm.md` 3 and 4, `domain/tools.md` 2); `gpt-6.1-sol` (`shell.md` 6.1); skein's `fake-llm.md` 2.1 | `docs/development/plans/parity/` |
| Claude Code, and smith on Anthropic | drafting | `benchmarks.md`, sections 6, 7 and 10: Claude Code as the third agent, smith's probes on Anthropic models. Reworked for the user's `~/.claude` login before it runs, after reliability | `~/src/rust/plans/reliability-plan/smith/11-claude.md (not versioned)` |

## 2. The design

Left names each section's unbuilt part and what builds it. The steps of
smith's reliability plan that it named have all landed (the plan's
retrospective, sections 1 and 2). "The parity plan" is
`docs/development/plans/parity/`; "its own plan, drafting" is section
1's Claude Code plan; "not planned" is in section 3. Where a landed step
left the design unmet, Left says what is missing, then what the code
does instead.

| Document | Built | Left |
|---|---|---|
| `domain/README.md` | partly | 4: `smith-mcp` and supervised processes with a view, not planned |
| `domain/host.md` | partly | 4: a `job` call's wait reported as a long operation (the parity plan); a contained tree, not planned. 8: MCP host tools, not planned. 12: an isolated mode, not planned |
| `domain/run.md` | partly | 3.1, 3.3: the role first, the shell's task role when settings give none, the summary instruction's requirements and its preamble (the parity plan). 5.3, 8.1, 9.1, 11: jobs cancelled before another writer, the checks and a delivery, and a job's end told as its session closes (the parity plan). 5.1: MCP servers as a tool source, not planned. 5.3, 15: work beside a writing child, a child from its parent's prefix, not planned. 6, 9.2: mid-work messages and threshold notes, not planned |
| `domain/session.md` | partly | 5: `parallel_tools` from the profile, and jobs beside the turns (the parity plan). 8.1, 8.2: a compaction asked with `ToolChoice::None`, its summary's blocks joined by blank lines and opened by the run's preamble: the request keeps the conversation's choice today, and joins the blocks with nothing (the parity plan). 11: provider-native compaction, not planned |
| `domain/tools.md` | partly | 2: the empty path as the working directory, and a path's bound counted as written and as resolved (the parity plan). 3, 5: jobs, `shell`'s `yield_seconds` and the `job` tool (the parity plan). 5: containment, not planned |
| `protocol/README.md` | partly | 5: `smith-mcp`, not planned. 9: contained trees, Codex routing extras, not planned. 4, 11: a connected agent, not planned |
| `protocol/agent.md` | partly | 2: the machine holding, reading and stopping jobs (the parity plan). 3: the view, not planned |
| `protocol/channel.md` | partly | 2, 7: a connected agent's credential and refusals, not planned |
| `protocol/charter.md` | yes | Nothing |
| `protocol/events.md` | partly | 3.6: the `running` and `stopped` verdicts, a call's `job`, and `job.ended` (the parity plan). 3.2: `containment: "trees"`, not planned |
| `protocol/hosts.md` | partly | 3: the tree, not planned. 4: connecting later, not planned |
| `protocol/limits.md` | partly | 2.2, 2.3: `jobs`, `job_wait` and `parallel_tools`; 4: `host-part-holds-the-role`; 3, 5, 7: jobs derived and counted; 3.9: the summary preamble (the parity plan) |
| `protocol/llm.md` | partly | 2.2, 3, 4, 5: a compaction asked with `None`, `shell`'s `yield_seconds`, the `job` tool, the facts each description states, the empty path decoded as `.` (the parity plan). 12: edit formats per dialect, not planned |
| `protocol/transcript.md` | partly | 2, 4: a job's outcomes, the summary preamble's block, and job numbers that go on after a resume (the parity plan) |
| `shell.md` | partly | 4.3: a start that fails ends the command: `smith exec` waits forever after a writable repository's head cannot be read (the parity plan). 5.2, 6.6: the task role, whose words ask for no response the work does not need (the parity plan). 6.1: `gpt-6.1-sol` (the parity plan). 6.3, 6.5: `jobs`, `job_wait`, `parallel_tools` (the parity plan). 5.1: directories made at startup synced into their parents, not planned. 8: per-tree limits, not planned. 9: `isolate`, not planned |
| `testing.md` | partly | 6, 6.1: jobs in the referees and the tools world's draws (the parity plan). 2.1: fault sweeps, not planned. 2.2: real-loop signals, not planned. 6: replay by digests, containment checks, not planned |
| `benchmarks.md` | partly | 3.1, 5.2: each probe's and each variant's calibration in its task file, with its digests: it is in commit bodies, and only `credentials`' task file has a `[calibration]` block (the parity plan). 3.1, 3.2, 9: the large effect computed, and what decides a graded task: neither is in the summary code (the parity plan). 5.2, 11.4: the fixtures' and `shell-cwd`'s next versions, which a healthy agent can pass: fixtures 01–03 protect `tests/`, and `shell-cwd`'s protected check reads the manifest, not the bundle (the parity plan). 5.4, 6, 6.1, 6.2, 10: each agent as it ships, by pins that change only what an attempt must own; one smith face; Codex pinned by digest; `/tmp` listed: the compare pin is a probe's template, with an empty role, half the window, `sub_agents` on and no budget above the deadline (the parity plan). 8.1, 9: the common priced spend from smith's presets, the time record, each attempt's values in the summary: three price catalogs disagree, by up to 20 times for one rate (the parity plan). 5.3, 8.2, 16.3: a retried failure, a response the provider failed before any block and smith retried, with no usage reported: the harness stops the suite on the attempt's lower bound, and leaves its priced spend unavailable, so one provider failure makes a `parity` run incomplete; the suite should charge what was reported, and the verdict take smith's lower bound as observed (the parity plan). 11.1: `history` version 3, with the running sum written beside each read, and `tool-output/background` (the parity plan); `mid-work`, `build-environment`, not planned. 11.3: the `delegation` experiment, not planned. 11.5, 16: the `parity` suite, its verdict and its command, and what follows a verdict short of parity (16.5) (the parity plan). 6: Claude Code (its own plan, drafting). 15: beyond serial runs, not planned |

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
