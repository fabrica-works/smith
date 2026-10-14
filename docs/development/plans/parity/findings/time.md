# Findings: wall time and tool behaviour

2026-10-14. Where smith's task wall time goes against Codex's, on the 12.4 fixtures and the 12.5 repository task, and how each agent uses its tools. Written by a read-only sub-agent for the parity plan's designer (skein's development.md, section 4.1).

Sources: the raw attempts of `repository-a64f0523-20261013T230741Z` (smith 01-03, Codex 04-06) and `compare-a57ca3a8-20261013T221135Z` (30 attempts), under `~/.local/state/smith-bench/runs/`, read with `jq` and `python3 -I`. smith's records are `trace.jsonl`; Codex's are its `rollout-*.jsonl` files, one per thread. Code and design are cited at smith `main` (`689c15d0`). No commit between the measured arm `a64f0523` and `main` touches `descriptions.rs`, `smith-domain-tools`, `smith-domain-session/src`, `settings.rs`, `presets.rs`, or `domain/{tools,session,run}.md` (`git log a64f0523..689c15d0 -- <those paths>` prints nothing). The sibling file `harness.md` (T3, T4, T10, T11, T13) covers the configuration side of several items here. This file gives their cost in seconds.

## 1. In one page

- **smith's extra 316 s on the repository task is not model time.** Main's time inside model requests is 374.3 s on average for smith and 374.6 s for Codex. The difference is made of three things that Codex does not have on its critical path: synchronous shell commands (167.4 s against Codex's 38.7 s of tool time), compaction (81.6 s against none), and waiting on children (103.1 s against none). smith's own overhead between events is 0.2 s per attempt (W1-W3).
- **Builds block smith and do not block Codex.** smith's cargo commands ran 286.6, 135.1 and 73.4 s, all inside blocking `shell` calls. Codex's main threads ran 156.2, 111.1 and 104.0 s of cargo, but only 24% of that time fell inside a tool call. It starts builds with `yield_time_ms: 1000` and keeps working, then polls them 14, 10 and 12 times with `write_stdin`. smith's design has no background command, and `tools.md` 8 lists one as open (W1).
- **The harness halves smith's window, and smith pays 36.6-59.8 s per compaction.** The smith arm declares `gpt-6-astra` with a 131,072-token window (`benchmarks/src/agent/smith/settings.rs:148`). The product preset declares 272,000, and Codex reports 258,400. Main compacted 2, 1 and 2 times, for 93.8, 54.5 and 96.4 s, and attempt 01 then re-read 22 files it had already read. Codex's root prompts reached 115,845-131,454 tokens with no compaction (W2; `harness.md` T4).
- **Children block the parent by design.** Each of the five `sub_agent` calls was alone in its response, and the parent waited 178.2 s (02) and 131.2 s (03). Codex's `spawn_agent` returned in 0.1 s. Its two children per attempt ran for 147-281 s alongside a parent that never called `wait_agent` (W3).
- **Requests and calls are at parity on the repository task.** smith made 48, 35 and 33 main requests, and Codex 55, 46 and 39. Per-request times are similar (smith mean 7.9-11.7 s, Codex 7.1-9.0 s). More parallel reads would save almost nothing: all of smith's reads, lists and searches took 1.2-1.6 s per attempt. On the fixtures, smith's whole gap is request time. It makes 7-9 requests against Codex's 5-7, with 2.3-2.5 times the reasoning tokens (W5).
- **smith 03's failed grade came from a child's review.** At 284.3 s, child 3 told the parent to "Check raw spelling length against the path cap before splitting". At 315.9 s the parent added `cwd_bytes` ("Original spelling length in bytes, including redundant slashes"), which rejects the grader's 129-byte spelling. smith's own `normalise` already bounds the spelling of a path's parts, and attempt 02's first child said so. The cause is a model reasoning error, brought in by delegation. It is not a smith defect (W4).
- **No arm had a warm build cache.** The repository task has no `setup`. Codex's `workspace-write` sandbox made the harness's private target read-only ("Read-only file system" in all three repository attempts and in the fixture attempts read), so Codex built in `$PWD/target` or `/tmp`. smith used the private target, cold (W6).
- **Shell output and timeouts cost nothing here.** smith made 88 shell calls. None was clamped and none timed out: every verdict is `exited`, and the longest call took 95.6 s. 35 calls lost bytes to the output budget, mostly `cat` of long documents (W7).
- **The harness cannot show any of this yet.** `result.json` gives smith's `first_byte_ms` and `largest_gap_ms` as "unavailable: not exposed by the selected stream", although the trace carries both for every response. Codex's children are "unavailable", although the rollouts time them (W8; `harness.md` T11).
- **Fixing W1-W3 together projects the repository median from 728.5 s to about 405 s, against Codex's 421.8 s.** This is an estimate, not a measurement (section 2.4).

## 2. The numbers

### 2.1 Method

For smith, each main-conversation interval is taken from `trace.jsonl`:

- a request runs from `response.started` to `response.completed`;
- a tool call from `tool.started` to `tool.completed`, keyed by `call`;
- a compaction from `conversation.opened` (kind `compaction`, parent 0) to `conversation.closed`;
- children are main's `sub_agent` calls.

Each category is the union of its intervals. The gap is `run.completed.t_ms` minus the union of all of them.

For Codex, each thread's `rollout-*.jsonl` is read:

- a request runs from the last anchor (`task_started`, or a `*_call_output` item) to that response's first `token_usage_record` with the same `thread_id`, deduplicated by `response_id`;
- a tool call runs from its `custom_tool_call` or `function_call` item to its output item;
- a command's process time is the `duration` of its `CommandExecution` item (`secs` plus `nanos`), ending at that item's timestamp.

A Codex request time therefore includes Codex's own overhead. The call item precedes the usage record by tens of milliseconds, so in 04 requests and tools overlap by 3.5 s. The scripts were `/tmp/claude-time/{smith_decomp,codex_decomp,cargo_both,codex_overlap,fixture,fixture2}.py`, each run as `python3 -I <script> <attempt dirs>`.

### 2.2 Repository task, per attempt (main conversation or root thread; seconds)

| ID | Agent | Task s | Requests: n, s | Compaction: n, s | Shell or tools: n, s | Other tools s | Waiting on children: n, s | Gap s |
|---|---|---:|---|---|---|---:|---|---:|
| 01 | smith | 764.6 | 48, 380.6 | 2, 93.8 | 26 shell, 288.4 | 1.6 | 0, 0 | 0.2 |
| 02 | smith | 728.5 | 35, 356.8 | 1, 54.5 | 26 shell, 137.3 | 1.5 | 2, 178.2 | 0.2 |
| 03 | smith | 690.7 | 33, 385.4 | 2, 96.4 | 31 shell, 76.4 | 1.2 | 3, 131.2 | 0.2 |
| 04 | Codex | 432.9 | 55, 388.4 | 0 | 46 exec, 46.6 | 0.7 (collab) | 2 spawned, never waited | — |
| 05 | Codex | 421.8 | 46, 385.2 | 0 | 41 exec, 37.4 | 0.4 | 2 spawned, never waited | — |
| 06 | Codex | 380.0 | 39, 350.3 | 0 | 29 exec, 30.2 | 0.8 | 2 spawned, never waited | — |
| mean | smith | 727.9 | 374.3 | 81.6 | 167.4 | 1.4 | 103.1 | 0.2 |
| mean | Codex | 411.6 | 374.6 | 0 | 38.7 (all tools) | | 0 | |

The difference in means is 316.4 s: −0.3 in requests, +81.6 in compaction, +128.7 in shell against tools, +1.4 in other tools, +103.1 in children, and +0.2 in gap.

- **Inside smith's requests.** Time to first byte sums to 32.1, 25.6 and 21.3 s (median 592, 576 and 594 ms). The longest silent gap per response sums to 138.4, 147.6 and 150.8 s, which is 36-41% of request time. Output is 37.3, 36.4 and 37.6 tokens per request-second. No response was retried (`attempt` is 0 in every `response.started`). No response neared the 4,096-token output cap: the largest was 2,933 tokens.
- **Codex per request (root).** The median is 5.6, 5.8 and 6.1 s, and the mean 7.1, 8.4 and 9.0 s. For smith the median is 5.1, 7.5 and 6.7 s, and the mean 7.9, 10.2 and 11.7 s.
- **Codex's children (thread spans).** In 04 they ran 267.9 and 281.2 s, in 05 147.4 and 228.7 s, and in 06 147.6 and 155.9 s. All of it falls inside the root's span. Their model time was 252.2 and 237.3, 143.5 and 118.0, and 142.5 and 150.5 s. In total across threads, Codex spent 877.9, 646.7 and 643.3 s in requests. smith spent 474.4, 584.2 and 647.7 s, counting all conversations.

### 2.3 Calls

- **Calls per response (smith main).** In 01, 14 of 48 responses carried more than one call, and in 02, 6 of 35. In 03, 24 responses carried one call, 4 carried two, and 5 carried four to six. All reads in a response started within 5 ms of each other in 13 of 14 (01), 23 of 29 (02) and 11 of 19 (03) multi-read responses, counting children. The other cases have a write between the reads, which session.md 5 runs alone. Codex sent exactly one call per response in every thread. Its `exec` code-mode calls held 57, 51 and 42 `exec_command`s and 14, 10 and 12 `write_stdin` polls (root). Its fixture attempts used `Promise.allSettled` for parallel commands.
- **By tool (smith main).**

  | Attempt | read | shell | edit | search | list | sub_agent |
  |---|---:|---:|---:|---:|---:|---:|
  | 01 | 55 | 26 | 12 | 4 | 3 | 0 |
  | 02 | 25 | 26 | 0 | 3 | 2 | 2 |
  | 03 | 18 | 31 | 0 | 0 | 3 | 3 |

  02 and 03 made every edit through `shell` (Python `replace` scripts and heredocs). Codex did the same: `apply_patch` appears 0, 8 and 2 times in its roots.
- **Failed calls (smith).**
  - Edits: 2 `edit` `no_match`, both in 01.
  - Paths: `read` `not_found` 2 times, and `list` `not_found` 3 times.
  - Lists: `list` `failed` 8 times, each "operation failed: io error". `{"path": ".", "depth": 2}` failed once in every attempt; `"."` at depth 3 and `crates` at depth 2 also failed.
  - Shell: no timeouts. 4, 2 and 2 commands exited non-zero.
  - Codex: 13 of 97, 12 of 75 and 12 of 80 commands exited non-zero, cargo failures 100/101 included.
- **Repeated work.** 01's main read 30 distinct paths in 55 reads. After its two compactions it re-read 13 and 9 earlier-read paths, and re-ran `cat AGENTS.md; cat docs/development/workflow.md` at 361.5 s. 02 re-read 5 paths after its compaction, and 03 none. Both agents re-ran the gate several times. smith ran clippy 5, 4 and 3 times and nextest 5, 6 and 4 times. Codex ran clippy 3, 3 and 5 times and nextest 7 times in each attempt (all threads).
- **Fixtures (medians per group of 5).** smith's time is 97-99% requests. Its shell totals 0.8-1.9 s per attempt.

  | Fixture | smith: requests, request s, reasoning tokens | Codex: requests, request s, tools s, reasoning tokens |
  |---|---|---|
  | 01 | 7, 49.0, 273 | 5, 44.8, 4.3, 224 |
  | 02 | 8, 98.6, 854 | 7, 83.2, 3.3, 377 |
  | 03 | 9, 90.9, 459 | 7, 79.5, 3.3, 183 |

### 2.4 What the fixes would save (projection)

Each saving is subtracted from the attempt's task s. Compaction and children are removed outright. For shell, 76.1% of smith's shell time is removed, the share of Codex's root cargo time that ran outside tool calls: (156.2−39.3 + 111.1−25.6 + 104.0−23.8) / 371.3.

| Fix | 01 | 02 | 03 | Median |
|---|---:|---:|---:|---:|
| as run | 764.6 | 728.5 | 690.7 | 728.5 |
| window only (W2) | 670.8 | 674.0 | 594.3 | 670.8 |
| background commands only (W1) | 545.1 | 624.0 | 632.6 | 624.0 |
| asynchronous children only (W3) | 764.6 | 550.3 | 559.5 | 559.5 |
| all three | 451.3 | 391.3 | 405.0 | 405.0 |

The projection makes four assumptions:

- smith's model has other useful work while a build runs, as Codex's had;
- a longer window adds no request time;
- re-reads after compaction disappear with it;
- concurrent children do not slow the parent's requests.

None of these is measured.

## 3. Entries

### W1 Every build is on smith's critical path

- **Symptom:**
  - smith's main spent 288.4, 137.3 and 76.4 s in `shell`, 82%, 43% and 27% of each attempt's gap to Codex's mean of 411.6 s. Cargo-bearing commands account for 286.6, 135.1 and 73.4 s of it (`cargo_both.py`). The longest single command was 95.6 s (01, `cargo fmt; cargo clippy ...; cargo nextest ...` at 658.2 s).
  - Codex's roots ran 156.2, 111.1 and 104.0 s of cargo, of which 39.3, 25.6 and 23.8 s fell inside its tool calls (`codex_overlap.py`). Its tool time was 46.6, 37.4 and 30.2 s.
  - Codex's pattern: `exec_command({cmd:"CARGO_TARGET_DIR=$PWD/target ... cargo clippy ... > /tmp/smith-cwd-clippy.log 2>&1", yield_time_ms: 1000})` (04, 23:12:36.872). It then read diffs, and polled with `write_stdin({session_id: 2387, ...})` at 23:12:49.819 and 23:13:05.259. Clippy ended at 23:13:07.886 after 30.8 s.
  - smith never backgrounded a command: no `nohup` or trailing `&` in its 88 shell commands. It asked for 120-200 s timeouts and waited.
- **Root cause:**
  - The shell call is synchronous. tools.md 3: "**`shell { command, timeout_seconds?, output_bytes? }`** runs `command` with a deadline and a budget for its kept output" (`docs/design/domain/tools.md:118`).
  - Its effect is a write. session.md 5: "Adjacent reads in one turn run together, a write runs alone" (`session.md:197-199`).
  - The description tells the model so: "Batch writes, edits and commands; run serially in call order." (`crates/smith-protocol-llm/src/descriptions.rs:96`).
  - tools.md 8 keeps the remedy open: "**Background processes:** a tool to start a command, read its output as it comes, write to it and stop it, after process groups." (`tools.md:270-271`).
- **The clean fix:**
  - Close that open question with a job: `shell` gains `yield_seconds?`. A command still running when its yield ends answers `running { job, output so far }`, and its process group goes on under the same deadline.
  - A read-effect tool `job { job, yield_seconds?, output_bytes? }` answers the output since the last answer, and the exit when it comes. A job has exactly one terminal. Its kept output is bounded as section 5 bounds a command's.
  - A session closing, a run winding down, a check and a delivery cancel or await jobs (session.md 6; run.md 8.1, 9.1).
  - Design text to change:
    - tools.md 3, the `shell` bullet and the table;
    - tools.md 5, "**Shell** runs a command with `sh`, in the working directory" (`:201`), which gains the yield and the job's terminal;
    - tools.md 8, which drops the open question;
    - session.md 5, which states that a running job does not hold its call's place in the batch;
    - protocol/llm.md 3 and `descriptions.rs:74-76,96`, which state the yield's default and maximum.
- **Its tests:**
  - `smith-domain-tools`, `src/tests.rs`:
    - `a_command_past_its_yield_answers_running_and_continues`: `running` with a job id and the head bytes, and no group signal.
    - `a_job_poll_returns_new_output_then_one_exit`: output in order, one terminal.
    - `a_job_past_its_deadline_is_stopped_and_timed_out`: the group is signalled, and `timed_out` names the bound.
    - `a_closing_kit_cancels_running_jobs_before_it_ends`.
  - `smith-domain-session`, `src/tests.rs`:
    - `a_completion_is_made_while_the_sessions_job_runs`: `response.started` precedes the job's exit.
    - `a_check_waits_for_running_jobs_before_it_starts`.
  - `tests/tools/tests/fuzzy_simulation.rs`: `jobs_polls_and_edits_interleave_with_one_terminal_each`.
  - Live:
    - a new probe, `tool-output/background`: a 60 s command beside independent reads. It passes when a completion starts between the job's start and its exit. Ten attempts calibrate it (benchmarks.md 4.2).
    - Then `suite repository`, five per arm, interleaved. The measure is main shell-blocked seconds against Codex's tool seconds.
- **Confidence:** high that shell blocking is 76-288 s per attempt and that Codex overlaps 76% of its builds. Medium on the saving, which depends on the model finding other work while it waits. Codex's did: it made 55, 46 and 39 requests in the time.

### W2 The harness's window makes smith compact, and each compaction costs a minute

- **Symptom:**
  - The rendered smith settings declare `gpt-6-astra` with `"window": 131072`, `"output": 4096` and `"idle": 120` (01 `settings.json`, `.models`). `session.started` repeats them.
  - Codex's `token_count` reports `"model_context_window": 258400` (04 root rollout).
  - smith's main compacted 2, 1 and 2 times. The compactions took 53.8 and 40.0 s (01), 54.5 s (02), and 59.8 and 36.6 s (03), writing 1,469-2,286 output tokens at high effort. 02's child 3 also compacted, for 11.8 s.
  - The prompt before each compaction was 56,565-85,232 tokens by the provider's count (`ctx.py`, `check.py`). Codex's roots reached 115,845, 131,454 and 122,098 tokens with none.
  - Afterwards, 01 re-read 13 and then 9 paths it had read before (`reread.py`). The product preset declares `window = 272000` and `output = 32000` (`crates/smith-local-shell/src/presets.rs:50-55`). 272,000 × 0.95 = 258,400, Codex's figure.
- **Root cause:**
  - `benchmarks/src/agent/smith/settings.rs:146-152` replaces the template's model with `window` 131 072, `idle` 120 000 and the template's `max_tokens` 4096 (`benchmarks/agents/smith/first-request.json:22`). This came in with `46f6f1bc` (2026-10-11), and no reason is recorded.
  - session.md 8.1's check is conservative by design: "an upper bound on what has been appended since: that completion's output, then the results and messages after it, counting a byte as at most one token" (`session.md:303-305`). Results of 28-36 KB therefore count as 28-36 thousand tokens. Three of the five compactions reconstruct as crossing 104,857 (0.8 × 131,072) this way. Two do not (03 at 102.2 and 629.9 s), so another history bound or the rendered sizes triggered them. The trace does not carry the check's inputs.
  - Compaction is a full completion: "the same model, affinity, system text, tools and tool choice as the completion it stands in for" (`session.md:361-362`).
- **The clean fix:** the comparison arm declares what the product declares (`harness.md` T4): the preset's `window`, `output` and `idle`. Probes that need a small window keep it in their task's `[smith]` overrides, as `history/small-window` does.
  - Design: benchmarks.md 6.1, which today lists "Instructions", "The environment" and "Budgets", gains a bullet: "**Models.** A comparison's smith declares each model as its presets do. A smaller window or output is a probe's, named in its task."
  - Code: `settings.rs:146-152` takes the preset's values.
  - session.md 8.1's byte bound and 8.2's same-model compaction stay. With a 272,000 window, none of these five checks would have failed. A cheaper compaction is a question (Q5), not a fix.
- **Its tests:**
  - `benchmarks/src/agent/smith/settings.rs` tests:
    - `a_compare_arm_declares_the_presets_window_and_output`: 272 000, 32 000 and 300 000 for `codex/gpt-6-astra`.
    - `a_task_override_still_lowers_the_window_it_names`: `history/small-window`.
  - `benchmarks/tests/smith_adapter.rs`: `rendered_models_match_the_product_preset_for_comparisons`.
  - Live: `suite repository`, five per arm. Main `compactions` should be 0 wherever every prompt stays under 217,600 tokens (0.8 × 272,000). `history` and `history/small-window` are re-run to show that compaction is still exercised.
- **Confidence:** high on the asymmetry and on 54.5-96.4 s of main time per attempt. Medium on the re-read cost, which was counted as reads, not seconds.

### W3 A child's call blocks its parent for the child's whole life

- **Symptom:**
  - 02 asked for two inspect-only children (`"tools":["inspect"]`, effect `read`). They took 46.9 and 131.3 s.
  - 03 asked for three: one `inspect` (67.4 s) and two `inspect` plus `shell` (effect `write`, 33.2 and 30.6 s).
  - All five `sub_agent` calls were alone in their response (`response.completed.completion.blocks`). Nothing ran beside them, so `parent_blocked_ms` equals their sum: 178,195 and 131,170 (`result.json`, `.delegation`).
  - The children re-read 8 of 20, 9 of 26, 10 of 27, 8 of 13 and 9 of 12 paths that the parent had read before they opened (`childuse.py`). The parent re-read 0-3 of the children's paths afterwards, so their answers were used as summaries.
  - 02's child 3 ended `stop: budget` after 22 turns. Its answer was a P2 finding on decode-scratch allocation. The parent's next calls, at 350.8-365.6 s, read `decode.rs` and `component.rs`, the files it named.
  - Codex: `spawn_agent` took 0.1 s, then `send_message` and `followup_task`. No root called `wait_agent`. The children wrote too: there are 6, 3 and 3 `apply_patch` calls in child threads.
- **Root cause:**
  - run.md 5.3: "When the child ends, its final message is the parent's tool result" (`run.md:304-305`).
  - The lease (`run.md:349-356`): "its call is a write, which runs alone (session.md, section 5), so its parent waits and writes nothing meanwhile. Children granted only inspect hold no lease, and run side by side, with each other and with the reads around them in their parent's turn."
  - Even an inspect-only child therefore returns only when it ends, and the parent's next completion waits for it. run.md 15 defers the remedy: "then asynchronous children, whose spend shares become reservations and to which messages may go" (`run.md:909-915`).
  - The benchmark turns children on (`settings.rs:164`, `sub_agents = true`). The product default is off (`crates/smith-local-shell/src/settings/layers.rs:11`; shell.md 6.4, `:570`). Codex's arm sets `features.multi_agent = true` and `agents.max_threads = 6` (`benchmarks/agents/codex/overrides.toml:7,9`).
- **The clean fix:** take the first step of run.md 15's "work beside sub-agents".
  - An inspect-only `sub_agent` call answers at once with the child's ordinal. Its final message arrives later as a message to the parent, through run.md 6's inbox and `wait`, under the same terminals.
  - `finish` and `deliver` wait for or close live children.
  - Writing children keep the lease for now. Concurrent writers are Codex's model and wait for contained trees.
  - Design text to change:
    - run.md 5.3's "The result" and "One writer at a time" bullets;
    - session.md 5's sub-agent sentence ("A sub-agent's effect follows from its tools");
    - session.md 7, whose first paragraph says the child's turns "are summarised in the call that asked for it, whose result is the child's final message";
    - run.md 15, which drops the item it implements.
- **Its tests:**
  - `smith-domain-run`, `src/tests.rs`:
    - `an_inspect_child_answers_as_a_message_while_parent_works`;
    - `finish_waits_for_or_closes_each_live_child_once`;
    - `a_writing_child_still_holds_the_lease_alone`.
  - `tests/run/tests/lease.rs`: `a_parent_edit_waits_while_a_writing_child_holds_it`.
  - `tests/run/tests/fuzzy_simulation.rs`: `async_children_answer_once_under_random_schedules`.
  - Live:
    - `parallel-children` (2/5 at 13.4) and `parallel-children/reviews`, judged on completions of the parent overlapping the child;
    - then `suite repository` with `sub_agents` on and off, five attempts each.
- **Confidence:** high on the blocking and its 131-178 s. Medium on the saving: n is 2 attempts with children, and 01, with none, was the slowest at 764.6 s.

### W4 smith 03's failed grade traces to a reviewing child

- **Symptom:**
  - The grader fails `feature_normalized_path_limit_is_inclusive`: "expected Spawn Answer { ... outcome: TooLong }" (03 `grade-0-stdout.log`). The check spells `"/work/" + 123 × "x"`, 129 bytes, whose parts `work` and the 123 x's fit 128 (`benchmarks/tasks/repository/shell-cwd/grader/src/lib.rs:191-201`).
  - 03's diff rejects `cwd_bytes > u64::from(limits.path_bytes)`, where `cwd_bytes` is the original string's length (`diff.patch:91-94`, `:343`).
  - The chain is in the trace:
    - at 251.1 s, child 3's brief asks it to "Focus overlong path rejection with repeated slashes";
    - at 284.3 s, it answers "**Overlong cwd bypass:** `decode.rs:248–268` discards repeated slashes before `path::normalise` checks length ... Check raw spelling length against the path cap before splitting";
    - at 315.9 s, the parent adds `cwd_bytes: u64` with the doc comment "Original spelling length in bytes, including redundant slashes, before parsing".
  - 02's first child had said the opposite, and rightly: "`path::normalise` checks component spelling INCLUDING dots before scratch allocation, then normalized absolute length. Leading/repeated/trailing slashes are absent from that count; wire JSON/string limits bound those."
- **Root cause:**
  - smith's code already bounds what the child worried about. `normalise` sums each part's spelling (a name's bytes, 1 for `.`, 2 for `..`) before allocating, then bounds the joined result (`crates/smith-domain-tools/src/path.rs:131-147,177`; the seed's copy at `56c0c59` is the same, `:82-85`).
  - The task says "Reject malformed arguments, NUL-bearing/overlong paths" and "Reuse the same path normalization and deepest-mounted-root selection as other workspace tools" (`task.toml:6`).
  - The schema every path tool shows says only "max {path_bytes} bytes" (`descriptions.rs:101-109`), without saying what is measured.
  - The task, read whole, points to the normalised bound. The 03 model took a child's wrong claim about existing code. That is chance at n=3: 1 of 3 smith attempts, and none of Codex's.
- **The clean fix:** no product change is required for the grade.
  - One improvement would make the bound unambiguous to every model. tools.md 4's "Limits are told" names "a read's window, what a write or an edit may carry, ...". It gains: "a path's bound counts its names, `.` and `..` with one separator each, before normalisation, and the normalised path after, never repeated separators." The path schema then says "max N bytes, counted without repeated slashes".
  - Child answers that assert code facts are W3's and the delegation theme's.
- **Its tests:**
  - `smith-protocol-llm`, `src/tests.rs`: `a_path_schema_says_what_its_byte_bound_counts`.
  - `smith-domain-tools`: the existing `raw_paths_are_bounded_before_normalisation_erases_components` (`src/tests.rs:1446`), plus `redundant_slashes_never_count_against_a_paths_bound`.
  - Live: the repository grade itself, five per arm.
- **Confidence:** high on the causal chain, which the timestamps and texts show. Medium on calling it chance rather than a capability gap. A second model or more attempts would raise it.

### W5 Requests and calls: parity on the repository, extra requests on fixtures

- **Symptom:**
  - Repository: see 2.2 and 2.3. smith's read, list and search tools took 1.2-1.6 s per attempt in total. Running them more in parallel saves at most that.
  - Fixtures: smith's requests are 97-99% of its task wall. On 03 smith makes 9 requests against 7, takes 90.9 s against 79.5 s, and spends 459 reasoning tokens against 183.
  - Fixture 21 (smith/03) shows the shape:
    - a lone `list .` (4.9 s);
    - six reads together (9.9 s);
    - a whole-file `write` of `src/planner.rs` after a 24 s request;
    - an `edit` after a 45 s request with 2,557 output tokens;
    - two test runs;
    - a late `read AGENTS.md` (95.9 s);
    - `finish` at 99.0 s.

    Codex 26 batched its first commands in two `exec` calls, one of them with `Promise.allSettled`, and used `apply_patch`.
  - In 5 of 15 smith fixture attempts (04, 05, 13, 21, 24), the first call was `list {"path": ""}`. It was answered `invalid field value "path"` without running (no `tool.started`), and the retry request cost 1.7-2.7 s.
  - smith's fixture prompts miss the cache more: 16,413 fresh tokens against 9,085 (median, 03). Some responses read 0 cached tokens mid-run, for example 21's responses 3 and 5.
- **Root cause:** model behaviour under smith's text, not scheduling. smith already tells the model "Batch independent reads, lists and searches side by side." (`descriptions.rs:93`), and its batched reads ran together (2.3). The extra requests are:
  - a lone first `list`;
  - a whole-file `write` where an `edit` would do;
  - a final `AGENTS.md` read;
  - a rejected empty path. The decoder refuses an empty relative path (`crates/smith-protocol-llm/src/decode.rs:447-449`), while tools.md 2 says "**The directory itself** is an empty path within it", and the `list` schema names no spelling for the working directory. The extra reasoning tokens are the tokens theme's.
- **The clean fix:** none in scheduling. The decoder takes `""` as the working directory, as `.` is (protocol/llm.md 3, the path field), or the schema says "`.` for the working directory". A shorter path to first reads would also help: the opening could list the working directory's top level in the system text, as Codex's first `rg --files` effectively does. That is the prompt theme's to weigh. Cache misses are the cache theme's.
- **Its tests:**
  - `smith-protocol-llm`, `src/tests.rs`: `an_empty_relative_path_decodes_as_the_working_directory`.
  - `smith-domain-run`, `src/tests.rs`: `the_opening_lists_the_working_directorys_top_level_once`, if adopted. Live: `suite compare`, five per agent per fixture, with requests per attempt and request seconds reported (W8).
- **Confidence:** high on repository parity. Medium on the fixture causes, which come from one attempt's sequence and the medians.

### W6 No warm build cache for any arm, and Codex cannot write its private one

- **Symptom:**
  - `shell-cwd/task.toml` has no `setup` (lines 1-45). The warm loop runs only `task.setup` (`benchmarks/src/run.rs:472-484`), so `copy_cache` copies an empty `warm-target` (`run.rs:338-365`).
  - smith never set `CARGO_TARGET_DIR` (0 of 88 commands). It built in the harness's private target (`isolate.rs:266-267`; 01 `settings.json`, `.environment.set`), cold.
  - Every Codex repository attempt hit `failed to create directory .../target/debug ... Read-only file system (os error 30)`: 3, 3+2 and 4 occurrences in its rollouts. It then set `CARGO_TARGET_DIR=$PWD/target` (04 and 05, in 15 and 16 commands) or `/tmp/smith-shell-cwd-target` (06, in 19 commands).
  - Fixture Codex attempts 06, 07, 16, 17, 26 and 27 hit the same error and built in `target`. Their own task's warm cache went unused. smith's fixture shell totals of 0.8-1.9 s are against Codex's 3.1-4.5 s of tools.
- **Root cause:**
  - The repository task omits a warm-up. benchmarks.md 10 says caches are "warmed outside the timer and copied by reflink" (`benchmarks.md:512-516`).
  - Codex's `sandbox_mode = "workspace-write"` (`overrides.toml:4`) makes only the workspace and `/tmp` writable. The private target is the workspace's sibling.
  - benchmarks.md 6.1 says "`HOME`, `TMPDIR` and the build caches are the attempt's" (`:343`), without saying that each agent can write them.
- **The clean fix:** a harness change, `harness.md` T13.
  - `shell-cwd` version 2 gains a `setup` that builds the workspace's tests and runs clippy with the task's `[environment]`.
  - The Codex arm makes the private `CARGO_HOME` and `CARGO_TARGET_DIR` writable roots of its sandbox (`sandbox_workspace_write.writable_roots`, as a `-c` override).
  - benchmarks.md 6.1 then says: "Each agent can write the attempt's caches. A sandbox's writable roots include them."
- **Its tests:**
  - `benchmarks/src/isolate.rs` tests: `a_codex_sandbox_can_write_the_attempts_private_caches`.
  - `benchmarks/tests/offline.rs`: `every_cargo_task_warms_its_target_outside_the_timer`.
  - Live: `suite repository`. No rollout contains "Read-only file system", and each agent's first cargo command is reported.
- **Confidence:** high on the facts. Low on the seconds involved: cold compile and incremental time are not separable without the build logs, which the attempts kept in `/tmp`.

### W7 Shell output and timeouts are not where time goes

- **Symptom:**
  - All 88 smith shell calls ended `exited`, 0 `clamped`. The longest took 95.6 s, under the 120-200 s the model asked. The profile's default is 120 s and the ceiling 1,200 s (`session.started.limits`: `policy.shell_timeout`, `policy.tool_deadline`).
  - 6 of 26, 13 of 26 and 16 of 36 calls lost bytes to the output budget. Examples: 01's `cat AGENTS.md ... notes.md` with `output_bytes` 52,000 and 25,908 omitted, and its `cat AGENTS.md; cat docs/development/workflow.md` with 26,000 and 14,782 omitted.
  - Both agents redirected builds to literal `/tmp` logs and tailed them: 11, 9 and 8 smith commands, and 9, 10 and 9 Codex ones. The names collide across attempts. Run order was 04, 01, 05, 02, 06, 03: 01 reused 3 of 04's log names, 05 reused 2, and 03 reused 6 (`tmplogs.py`). No log was read before its own attempt wrote it.
- **Root cause:** none for time. tools.md 5's head-and-tail budget (`:208-212`) did its job, and the model chose to log rather than page. The `/tmp` writes are `harness.md` T10: `TMPDIR` is private, but nothing tells the model so.
- **The clean fix:** none in the shell for time. For hygiene, the system text names the private temporary directory: "Temporary files go under `$TMPDIR`." That is the harness and prompt themes' to settle.
- **Its tests:** `smith-domain-run`, `src/tests.rs`, `the_system_text_names_the_private_temporary_directory`, if adopted. Live: the `tool-output` probes stand.
- **Confidence:** high.

### W8 The harness reports no time breakdown

- **Symptom:**
  - `result.json` `.timing` gives `first_byte_ms`, `largest_gap_ms` and `longest_completion_ms` as "unavailable: not exposed by the selected stream" (repository 01). smith's `response.completed` events carry `first_byte_ms`, `largest_gap_ms` and `duration_ms` on every response.
  - For Codex, `children` is "unavailable: collaboration items do not enumerate every child", and `conversations` is a lower bound of 1 (04). Yet the rollouts time every thread, every call and every command.
  - Every number in section 2 had to be rebuilt by hand.
- **Root cause:** `benchmarks/src/run.rs:91-93` sets the three timing fields unavailable for every attempt, whatever the trace holds. The Codex adapter counts from the root's JSON lines alone (`harness.md` T11, `benchmarks/src/agent/codex.rs:480-487`).
- **The clean fix:** each result gains a `time` record for main or root, built by 2.1's rules:
  - seconds in requests, with first-byte and largest-gap sums;
  - seconds in each tool;
  - seconds in compaction;
  - seconds waiting on children;
  - command process seconds, and how much of them fell outside tool calls;
  - the gap.

  The summary gives medians per arm. Design: benchmarks.md 8.1 (one result per attempt) and 9 (statistics) list the record, so the parity plan's proofs are "main shell-blocked seconds" and "compaction seconds", not task wall alone.
- **Its tests:**
  - `benchmarks/tests/smith_adapter.rs`: `a_time_breakdown_sums_to_task_wall_within_one_second`, on a recorded trace.
  - `benchmarks/tests/codex_adapter.rs`: `rollout_threads_give_request_tool_and_command_overlap`, on rollouts trimmed from repository 04.
  - Live: none needed. Re-summarise the two runs offline and compare with section 2.
- **Confidence:** high.

## 4. Questions for the designer

1. **Background commands (W1).**
   - Options:
     - (a) `shell` gains `yield_seconds`, plus a read-effect `job` poll tool, as Codex has;
     - (b) a background `shell` answers at once and delivers its exit as a run message;
     - (c) no change, relying on children.
   - Recommendation: (a).
   - Why: it matches what was measured (76% of Codex's build time overlapped). The model chooses when to look, and each job keeps one terminal. (b) loses the polls the model used 10-14 times per attempt. (c) does not overlap builds at all.
2. **Writes while a job runs.**
   - Options:
     - (a) the session's own `edit`, `write` and further commands may run while its job runs;
     - (b) they wait for it.
   - Recommendation: (a) for the session's own job. Checks, delivery and writing children wait for or cancel jobs.
   - Why: Codex edited during its builds, and serialising them gives back W1's saving. A running job is the session's own write, not another holder's.
3. **The comparison arm's window (W2).**
   - Options:
     - (a) the preset's 272,000, 32,000 and 300 s;
     - (b) keep 131,072 as a stress setting.
   - Recommendation: (a).
   - Why: "smith's pinned settings add nothing" (benchmarks.md 6.1), Codex runs at 258,400, and `history/small-window` already exercises small windows. It changes the arm's configuration digest, so old summaries stay history.
4. **Asynchronous children (W3).**
   - Options:
     - (a) keep synchronous children;
     - (b) inspect-only children answer by message while the parent works, with writers exclusive;
     - (c) also concurrent writers on disjoint paths.
   - Recommendation: (b) now, (c) after contained trees.
   - Why: (b) removes 131-178 s of blocking without touching the lease. (c) needs write scopes that run.md 15 places after contained trees.
5. **Compaction's cost.**
   - Options:
     - (a) same model and effort as now;
     - (b) lower effort or the small tier for the summary.
   - Recommendation: (a) until W2's fix is measured.
   - Why: with a 272,000 window none of these compactions happens. (b) gives up the cached prefix that session.md 8.2 keeps on purpose.
6. **Which smith arm the parity claim is about.**
   - Options:
     - (a) the product default (`sub_agents` off);
     - (b) the benchmark's on;
     - (c) both.
   - Recommendation: (c), with (a) as the headline.
   - Why: the product ships off (`layers.rs:11`; sub-agents-by-default.md). Codex's arm has multi-agent on, so only (c) shows what children are worth.
7. **Repository attempts per arm.**
   - Options: (a) 3; (b) 5 or more.
   - Recommendation: (b).
   - Why: at n=3 the report can claim only disjoint ranges (13.132). A projected 405 s against 421.8 s is inside Codex's own range, 380.0-432.9 s.

## 5. What other themes must say

- **Harness** (`harness.md`):
  - T4's window and output halving is W2's 54.5-96.4 s per attempt.
  - T13's missing warm-up is joined by Codex's read-only private target, seen in every Codex attempt read, fixtures included (W6).
  - T11 should cover thread spans and command overlap, not only counts (W8).
  - T10's `/tmp` collisions are in W7.
- **Tools:** `list` on `"."` at depth 2 or 3, and on `crates` at depth 2, failed with "operation failed: io error" 8 times across the three smith attempts. The cause was not found here. A listing past its cap should count, not fail (tools.md 3, `list`). An empty `path` is refused by the decoder in 5 of 15 smith fixture attempts' first calls (W5).
- **Delegation:**
  - Children start cold and re-read 8-10 parent-read paths each (W3; the reliability findings `delegation.md` D7 and D8).
  - A review child's wrong claim about existing code caused the only failed grade (W4).
  - Every `sub_agent` call was alone in its response, despite "Inspect-only children may work side by side" in the system text.
- **Cache and tokens:**
  - On the fixtures smith's fresh tokens are 1.4-1.8 times Codex's, with mid-run responses reading 0 cached tokens (W5).
  - Fixture reasoning tokens are 2.3-2.5 times Codex's.
  - Repository reasoning is lower: 2,905 against 3,657, median.
- **Compaction:** 03's two compactions at 102.2 and 629.9 s do not reconstruct as crossing the window check from usage and result bytes. Whatever triggered them should be named in the event (W2).
- **Statistics:** the time record (W8) gives per-category medians and ranges. The parity plan should state its proofs in those categories, at five or more attempts per arm (Q7).
