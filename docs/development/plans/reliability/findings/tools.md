# Findings T: workspace tools, the machine, and local setup

## Summary

Smith's workspace tools work, but on real work the model makes extra round trips and runs into hidden limits.

- **Environment.** On main, shell commands got an empty environment. The contracts said the agent configuration supplies it, but the domain builds `Authority.env` empty and the machine gave the configured variables only to checks.
- **Listing ".".** It reached io as an empty path and failed.
- **What spike 1 did.** It merged the configured variables into every shell command inside the machine, and mapped the empty scan path to "." in one place only.
- **Larger causes neither spike touched:**
  - reads capped at 4 KiB, shell output at 6 KiB, search results at 1 KiB, and the guide (AGENTS.md) at 1 KiB, so models page through files and fall back to the shell;
  - a path convention that models get wrong in a third of the main-matrix Smith runs;
  - one snippet per `edit` call;
  - tool descriptions that never mention limits or units and discourage batching.
- **Local setup.** Smith's own OAuth sign-in cannot express Codex's registration: the redirect must be a numeric loopback address, there is no provider preset, and the account id is typed by hand. So the user hand-wrote a token record in Skein's private binary format. Settings validation lets profile violations through to a bare "The agent refused this run".

The clean fix has four parts:

1. One owner for the commands' environment: the agent configuration, resolved by the host from settings.
2. One translation from a domain `Place` to an io path.
3. A tool surface whose output budgets the model chooses within profile ceilings, checked against the machine at startup, with multi-hunk edits, a tree listing and honest descriptions.
4. Provider presets, a read-only "borrow an existing CLI login" credential source, and a `smith check` command.

Evidence comes from the diffs `main..bench/smith-codex-2026-10-08`, the benchmark traces in `/home/free/src/rust/smith-benchmark-2026-10-08/runs/*` (analysed read-only), and Codex 0.160.0 session logs and binary strings (read-only).

---

### T1. Shell commands never received the configured environment; the spike merged it in the machine

- **Origin:**
  - Reliability report item 5.
  - Commit `8e83931`: `crates/smith-protocol-machine/src/{component.rs,process.rs,limits.rs,tests.rs}` and `protocol/agent.md` §3.
- **Symptom:**
  - The report says: "Configured environment variables reached checks but not shell tools".
  - On main, `sh -c` ran with no `PATH` and no `HOME`, so `cargo` in `~/.cargo/bin` could not be found.
  - The spike's new live story asserts `SMITH_TEST_ENVIRONMENT=configured` from a shell command (`crates/smith/tests/live.rs`).
- **Root cause (verified on main):** there are two environment carriers, and only one was fed.
  - **The domain carrier is documented as the whole environment.** `Authority.env` is "the environment commands run with, whole" (`crates/smith-domain-tools/src/authority.rs:25-30`). The `Op::Spawn` doc says "Its environment is `env` and nothing else" (`boundary.rs:176`).
  - **The domain carrier is always empty.** `translate::authority` builds it empty: "Commands run with an empty environment: a charter carries none yet" (`crates/smith-domain/src/translate.rs:113-118,137`).
  - **The configuration's carrier only reached checks.** The agent configuration's `environment` goes to `Component::configure_environment` (`smith-agent-service/src/service.rs:408`). The machine used it only in `process::check`. `process::shell` used only the domain's empty list (main:`process.rs:352`).
  - **The contracts already promised the configuration's environment.** `domain/tools.md` §5 says the environment is "which the agent's configuration gives". `protocol/agent.md` §3 says "the configuration's, whole". The code had drifted from both.
- **Spike change:**
  - **The merge.** `process::shell` drops any configured default whose name appears in the per-call `vars`, appends the `vars`, and encodes the result. It refuses when the count `defaults.len()+vars.len()` exceeds `env_bytes`, or when the encoded bytes do (`process.rs:338-377`, spike tip).
  - **Memory bound.** The machine's `worst_case` environment term is multiplied by 4 (`limits.rs:79-83`).
  - **Tests.** Two unit tests: inheritance with an override, and refusal over the bound.
  - **Contract.** `protocol/agent.md` §3 gains "A shell call's explicit variables override defaults with the same name".
- **Assessment:** keep the behaviour, redo the ownership.
  - The visible effect is what both contracts always required.
  - The mechanism creates a second owner. It overlays a domain list that production never fills, and the LLM cannot set variables (the `shell` schema has no `env`). So the "explicit variables override" path is dead generality that only unit tests exercise.
  - The domain doc comments are now false.
  - Smaller defects:
    - The count check compares a variable count with a byte bound; it should be `env_bytes/2`, like `worst_case`.
    - Duplicate names are not refused (`config.rs:236-250`).
    - The baseline is cloned and re-encoded on every call.
    - The ×4 memory term is a hand multiplier, and the counting memory test uses `env_bytes: 0`.
    - Exceeding the merged bound is a runtime `Failed{Other}` ("operation failed: io error") that a startup check could rule out.
- **Clean fix:** the configuration owns the environment, whole.
  1. Remove `Authority.env` and `Op::Spawn.env`, or document that they are always empty.
  2. The machine applies the configured environment to every shell and check. Search keeps an empty environment.
  3. Encode the baseline once in `configure_environment`.
  4. At startup, check that names are unique and valid and the total fits the byte bound.
  5. If a per-run overlay is ever needed, make it a typed, bounded charter field, merged in one place (the machine) under a documented precedence, and refused at run admission rather than at spawn. Do not build it until a host needs it.

  Alternatives:
  - **(a) Carry the environment in the domain from the charter.** The domain would own machine facts it cannot validate, and every run would restate them.
  - **(b) Keep the two-layer merge.** This needs a real, contracted overlay source.
  - **Recommendation:** configuration-only now, with the overlay recorded as an open question in `protocol/agent.md` §10.
- **Contracts:**
  - `protocol/agent.md` §3: replace the spike sentence with "every shell and check sees exactly the configuration's environment; search sees none". List the startup validations in §4.
  - `domain/tools.md` §5 holds as written.
  - Update the `translate.rs` comment and `boundary.rs:176`.
- **Layer:** `smith-protocol-machine`, `smith-domain-tools`/`smith-domain` (field removal), `smith` config. None in skein.
- **Tests:**
  - **Spike:** focused machine unit tests (right tier) and an opt-in live story.
  - **Clean, focused:**
    - configured variables reach both shell and check spawns;
    - search spawns with an empty environment;
    - duplicate names are refused.
  - **Clean, agent-process world:** a config → service → machine → `Below::Spawn` story carries the entries.
  - **Clean, memory:** a saturated environment in `tests/machine/tests/memory.rs`.
- **Benchmark:** every Smith run relied on the merge for `PATH`, `HOME`, `RUSTUP_HOME` and `CARGO_HOME` (`smith_smoke.py:28-29`). Probe:
  - configure `SMITH_PROBE=<nonce>`;
  - ask the agent to run `printenv SMITH_PROBE > probe.txt` and `command -v cargo >> probe.txt`, then finish;
  - assert the file contents, and one shell call with exit 0.
- **Questions:** Is a charter-level environment overlay wanted? **Recommendation:** not now.

### T2. The directory itself (an empty `Place.path`) is spelled four different ways at the io boundary

- **Origin:** reliability report item 6; `8e83931` (`files.rs`).
- **Symptom:** "Listing "." produced an empty native path and failed."
- **Root cause (verified):**
  - The domain spells the directory itself as an empty path (`path.rs:69-77`). The machine forwarded `""` to skein's file layer, where `openat2(root,"")` fails with ENOENT (`skein-io/src/file_layer.rs:427-439`). The domain renders that as "path not found".
  - Search (`process.rs:415`) and the shell's working directory (`process.rs:374`) already mapped empty to `.` ad hoc, and checks hard-code `.`.
  - Load and Store still send `""`. So `read .` answers `NotFound` instead of `NotFile`, and `write .` answers an io error.
- **Spike change:**
  - `files::request` maps an empty `Scan` path to `"."` (`files.rs:28-44`), with one test.
  - The prompt gains "Use `.` to list the first directory" (`prompt.rs:198-199`).
- **Assessment:** redo properly. The spike adds a third copy of the mapping, and Load and Store stay inconsistent. The prompt line works around a bug the code now fixes.
- **Clean fix:**
  - The machine gets one `io_path(&Place)` used by every operation: Load, Scan, Store, Spawn directory, Search, OpenRead and Probe.
  - The domain answers `read`, `write` and `edit` of the directory itself as `NotFile`, with no io.
  - Optionally, skein's file layer answers an empty path with `InvalidArgument` instead of ENOENT.
- **Contracts:** `protocol/agent.md` §2 (Paths: "the directory itself is `.` at io"). Optionally `domain/tools.md` §4.
- **Layer:** `smith-protocol-machine`; a skein-io note.
- **Tests:**
  - A table-driven focused machine test: every operation × {directory itself, nested path}.
  - Tools domain tests that the directory itself is `NotFile` for reads and writes.
- **Benchmark:** none needed (covered by deterministic tests and T3's probe).
- **Questions:** none.

### T3. The named-directory path convention fails in practice, and tool and shell paths disagree

- **Origin:** reliability report item 6 (the guidance); the deeper issue is not in the spike.
- **Symptom:** every benchmark binary carried the spike's guidance ("Paths are relative to the first directory, or use `/directory-name/path`"). Models still wrote the directory name as a relative prefix.
  - Examples: `list project`, `read project/AGENTS.md`, `list project/docs/foundation`.
  - Main matrix: 23 such calls in 7 of 21 Smith runs, including every planner and every repository run. All Smith runs: 44.
  - Each resolves under `project/project/...` and fails `NotFound`.
  - Every planner run began with a lone failing `list project`, which wasted one round trip.
  - No run used `/project/...`.
- **Root cause:**
  1. **The prompt presents the checkout by name.** It shows "- `project`, which you may change…" and "## AGENTS.md in `project`" (`prompt.rs:212-240,180-190`). It never says "your working directory is `project`".
  2. **Tool schemas have no parameter descriptions** (`tools.rs:21-63`).
  3. **Errors do not say what was resolved.** The result is just "path not found" (`render.rs:176-179`).
  4. **Tools and shell use different namespaces.** Tool absolute paths are virtual (`/project/...`), and `../sibling` reaches another directory by name. The shell runs in the real directory, and the per-call roots view is ignored (`component.rs:125`, `roots: _`). The shared view in `protocol/agent.md` §3 is not implemented.
- **Spike change:** one prompt sentence.
- **Assessment:** the guidance is cheap and fine, but the benchmark shows it is not enough.
- **Clean fix:**
  1. **Prompt and descriptions name the working directory concretely.** For one directory: "the checkout is your working directory; write `src/lib.rs`". For several, say how to reach siblings. Every `path` property gets a description.
  2. **Failures name the resolved path**, for example "path not found: project/project/AGENTS.md (relative to project/)". `NotFound`, `NotDirectory` and `Outside` carry the normalised path, bounded by `path_bytes`.
  3. **`list /`** answers the mounted directory names from the authority.
  4. **Tool and shell paths agree.** Long term, the contained-tree view uses fixed absolute mounts that the tools also accept. Interim, the authority carries each directory's real host path as an alias.
  5. **No "first component equals a directory name" heuristic.** It is ambiguous and needs io.
- **Contracts:**
  - `domain/tools.md` §2 (single-directory presentation; one namespace);
  - `domain/run.md` §3.3 (the prompt states the working directory);
  - `protocol/llm.md` §3 (parameter descriptions) and §5 (failure text names the path);
  - `protocol/agent.md` §3 (the view's mount point).
- **Layer:** `smith-domain-run`, `smith-domain-tools`, `smith-protocol-llm`, `smith-protocol-machine`, and skein contained trees.
- **Tests:**
  - Prompt snapshots for one and two directories.
  - Domain tests for resolved spellings, `list /`, and aliases.
  - A session world test that the failure text reaches the LLM.
- **Benchmark:** measured (23 failing calls, from trace verdicts). Probe, three runs per model family:
  - set up two directories, `app` (writable) and `spec` (read-only);
  - ask the agent to read the README in `app` and the first heading of `spec/overview.md`;
  - assert zero `NotFound`/`Outside` verdicts and at most two completions before `finish`.
- **Questions:**
  - Should a single directory hide its name from path guidance? **Recommendation:** yes.
  - Should tools accept host absolute paths before contained trees exist? **Recommendation:** yes, as aliases.

### T4. Why Smith makes about twice the model tool calls: accounting, plus extra round trips

- **Origin:** the benchmark report's resource table; neither spike changed this.
- **Symptom:** model tool calls (Smith / Codex medians) were 12/6, 15/8, 17/8, 15/5, 14/5, 18/5 and 192/94.
- **Root cause (measured):**
  1. **The metric is not comparable.**
     - Codex exposes one code-mode `exec` tool. Inside it, the model's JavaScript runs `tools.exec_command`, `tools.apply_patch` and `tools.write_stdin`, often several at once in `Promise.allSettled`.
     - Each `exec_command` often chains `cat a && cat b`.
     - Codex's base instructions say "Batch independent searches and reads in one functions.exec using await Promise.allSettled" (session log `base_instructions`).
     - Smith counts every file read as a call, and already runs 4–5 reads in parallel within one completion.
  2. **Round trips are the real gap.** On the six fixture tasks Smith averaged 10.3 completions per run against Codex's 6.1:

     | Phase | Smith | Codex | Cause |
     |---|---|---|---|
     | explore | 3.39 | 2.11 | `list .`, then `list src` + `list tests` + reads, against `rg --files` plus a batched `cat` |
     | change | 3.11 | 2.22 | one snippet per `edit`, mostly one per completion, against a multi-hunk `apply_patch` |
     | shell | 2.33 | 1.22 | fmt, test and diff as separate calls, against parallel `exec_command`s |
     | finish | 1.06 | 0 | `finish` is a tool call; Codex ends with a message |

     On the repository task, T5 and T6 dominate.
  3. **Descriptions work against batching.**
     - "runs alone" appears in the `write`, `edit` and `shell` descriptions (`tools.rs:47,53,61`). Yet writes in one completion are legal and run in order (`session.rs:1185-1260`).
     - Nothing invites batching.
     - No description gives limits, units or the result format.
  4. **No tree listing and no multi-hunk edit.**
     - `search` returns at most 1 KiB.
     - The read-before-write rule forces a re-read after a shell formatter touches a file. `Stale`/`NotRead` recoveries appear in planner-2, journal-3 and repository runs 1–2.
- **Spike change:** none, apart from the sizing in T5.
- **Assessment:** the tool set is the right shape. The problems are missing composite operations, small windows and the descriptions.
  - Do not adopt a JavaScript wrapper. It needs an interpreter with unbounded semantics, hides policy in model-written code, and breaks the strict subset and bounded ownership.
  - Its benefits come from parallel tool calls, which Smith already runs, plus composite tools.
- **Clean fix:**
  - **`list { path, depth?, glob? }`:** a bounded tree that honours ignore files, run as a contained `rg --files`.
  - **`read`:** larger windows (T6); keep one file per call.
  - **`edit { path, edits:[{old,new,all?}] }`:** atomic per file; the outcome names the failing edit.
  - **Edit format per dialect.** OpenAI-family models get an `apply_patch`-grammar tool (the Codex binary ships one as `"freeform"`); Anthropic models get old/new. Both decode to the same typed call. That models do best with their vendor's native format is unverified; measure it.
  - **`shell { command, timeout_seconds?, cwd?, output? }`.**
  - **Descriptions.** Drop "runs alone". Say "independent reads and searches can be issued together; writes and commands run in order". Give every parameter its unit and limit. Add a batching line to the prompt's tool section.
- **Contracts:**
  - `protocol/llm.md` §3: the tool table, and replace "a write runs alone" in the descriptions paragraph.
  - `domain/tools.md` §3, and §4 if read-before-write is relaxed.
  - The scheduling rule in `domain/session.md` §5 is unchanged.
- **Layer:** `smith-domain-tools`, `smith-protocol-llm`, `smith-protocol-machine`.
- **Tests:**
  - Multi-edit domain tests: all match; one fails so none apply; overlapping and ambiguous edits; stale between load and store.
  - Tree listing bounds and ignore rules over the fake machine.
  - Per-dialect decode to the same call.
  - A fuzzy tools world with random edit lists. These are pure domain steps, well inside the suite budgets.
- **Benchmark:**
  - Report responses, completions per phase, and tool-output bytes instead of "model tool calls".
  - Probe on the planner fixture: "add a doc comment to each public function in src/ and run the tests". Target ≤ 2 completions before the first edit and 1 edit completion.
- **Questions:**
  - Relax read-before-write for `edit`, so a unique match of `old` against the current file suffices, while `write` keeps the rule? **Recommendation:** yes.
  - One edit format for all models, or one per dialect? **Recommendation:** per dialect, presenting the same typed call, decided by measurement.

### T5. Tool and machine output sizes are lockstep constants with no cross-layer check

- **Origin:** `8e83931`, `crates/smith/src/config.rs` `standard_limits`:
  - `shell_head` 256 → 2048;
  - `shell_tail` 256 → 4096;
  - `machine.output_bytes` 1024 → 4096.
- **Symptom:**
  - On main, a shell result kept 512 bytes in total.
  - After the spike, repository runs still truncated 21 of 49, 17 of 31 and 40 of 74 shell results at exactly 6,144 bytes.
  - Models read files through the shell 38, 27 and 48 times, and edited through it 8, 9 and 6 times.
  - `search` hit its 1,024-byte cap in 2 of 7, 1 of 8 and 6 of 12 calls, and models ran shell `rg` instead (16, 11 and 30 times).
  - Codex models chose per-command output budgets of 500–28,000 tokens. 177 of 302 were at least 3,000 tokens (about 12 KB). The binary default is 10,000.
- **Root cause:** sizes are fixed per profile, and domain and machine limits are set separately by hand.
  - The pairs are `shell_head`/`shell_tail` vs `output_bytes`, list limits, search limits, `file_bytes`, and `env_bytes` (256 in the tools domain vs 8192 in the machine) (`limits.rs:83-104`, `config.rs:419-433`).
  - A mismatch is not refused at startup. It fails at run time as "operation failed: io error" (`process.rs:350`, `files.rs:23-31`). That is why the spike had to raise `output_bytes` together with the tail.
- **Spike change:** the three constants above; no test checks them or their coherence.
- **Assessment:** keep the direction, redo the method.
  - Moving from 512 bytes to 6 KiB was necessary, but it is still a constant raised until the observed command fit, and it only moves the cliff (78 truncations in repository runs).
  - The tail-equals-output coupling is implicit.
- **Clean fix:**
  1. **One source of truth.** Derive the machine limits from the tools domain's limits, or check coherence at startup with named errors. The rules:
     - `output_bytes ≥ max(shell_head, shell_tail, check_tail)`;
     - machine search ≥ domain search;
     - `entries ≥ list_entries`;
     - machine `file_bytes` and `path_bytes` ≥ the domain's;
     - `env_bytes` ≥ the configured environment.
  2. **Model-chosen output budgets within ceilings.** `shell` and `read` take an optional budget, clamped to the profile. The session reserves the requested amount.
  3. **Measured defaults:**

     | Result | Default |
     |---|---|
     | `shell` | head 4 KiB + tail 16 KiB |
     | `search` | 100 hits / 16 KiB |
     | `list` | 512 entries / 32 KiB |

     Set these from the measured distributions (median and p90 of Codex's chosen budgets and of Smith's capped outputs).
  4. **Pair with history management** (eliding or summarising old tool results). This is cross-theme.
- **Contracts:**
  - `domain/tools.md` §4 says "set here" but holds no numbers. Either move the profile's numbers to `protocol/agent.md` §4 or delete "set here".
  - Add the coherence rule to `protocol/agent.md` §4.
- **Layer:** `smith` profile, `smith-agent-service` (coherence check), `smith-domain-tools`/`smith-protocol-llm` (budget parameter).
- **Tests:**
  - The standard profile is coherent.
  - One refusal per coherence rule.
  - A requested budget is clamped and reserved.
- **Benchmark:** measured (cap hits in trace facts). Probe:
  - a fixture with a failing test that prints about 12 KB;
  - ask "which assertion fails, and on which line?";
  - assert the answer is correct with one shell call.
- **Questions:** Should output budgets be model-selectable per call? **Recommendation:** yes.

### T6. 4 KiB read windows and a 1 KiB guide force paging on real repositories

- **Origin:** not in either spike (`read_bytes: 4096`, `guide_bytes: 1024`, `crates/smith/src/limits.rs:50,90`). This is the largest single cause of the repository-task gap.
- **Symptom:**
  - Capped reads in the three repository runs: 60 of 97, 30 of 55, 84 of 148.
  - `programming-model.md` was read 8 times in one run.
  - The failed run used 175 responses and 316 tool calls before its timeout.
  - The guide is cut at about 1 KiB with "(The file goes on…)" (`prompt.rs:189`), mid-sentence for the repository's 2 KiB AGENTS.md. The root and each child re-read it.
- **Root cause:**
  - The window is counted in bytes, so `lines: 400` returns about 100 lines (`window.rs:22-58`).
  - No description says so.
  - The guide has its own small cap.
- **Spike change:** none.
- **Assessment:** latent, high impact.
- **Clean fix:**
  - **Read window.** 32 KiB by default, model-selectable within the profile, still bounded by `file_bytes`. The description states the window and how to page.
  - **Guide.** Cap at 16 KiB, normally whole. Children inherit the parent's guide text (cross-theme with delegation).
  - **Trade-off.** Weigh this against history cost (T5).
- **Contracts:** `domain/run.md` §3.3 (state the guide limit); `domain/tools.md` §3.
- **Layer:** `smith` limits, `smith-domain-tools`, `smith-protocol-llm`.
- **Tests:**
  - Window tests at the default and at a requested size.
  - A guide under the cap is rendered whole.
  - A memory reservation check.
- **Benchmark:** measured. Probe: "quote the first sentence of §6 of programming-model.md". Assert the quote is correct with ≤ 2 reads.
- **Questions:** Should the default be whole files up to 64 KiB? **Recommendation:** no. Use 32 KiB with paging, because whole 64 KiB files exhaust the 1 MiB envelope quickly.

### T7. The shell timeout has no stated unit and is silently clamped to 60 seconds

- **Origin:** not in either spike.
- **Symptom:**
  - The schema declares `timeout` as an integer with no unit (`tools.rs:62`). Models sent 10, 120, 1000 and 200000 in one run.
  - The decoder reads seconds (`decode.rs:124`).
  - The kit clamps to 120 s (`kit.rs:236`), and the session caps every tool call at `tool_timeout` = 60 s (`session.rs:1232`). So the advertised 120 s cannot be reached.
  - `protocol/llm.md` §3 says the deadline is "within the charter's", but the charter has no shell deadline.
- **Root cause:** three local caps, none of them visible to the model.
- **Spike change:** none.
- **Assessment:** latent hidden policy. A cold workspace clippy or test run can exceed 60 s.
- **Clean fix:**
  - Rename the field to `timeout_seconds`, and document the effective maximum in the description, generated from the profile.
  - Make `shell_timeout_max ≤ tool_timeout` a coherence rule.
  - Say so in the outcome when a timeout is clamped.
  - Long term: background processes (start, poll, stop), like Codex's `exec_command` plus `write_stdin` (13–20 uses per repository run). This is a separate design, after contained trees.
- **Contracts:** `protocol/llm.md` §3, `domain/tools.md` §5, `domain/session.md`.
- **Layer:** `smith-protocol-llm`, `smith-domain-tools`, `smith-domain-session`, profile.
- **Tests:** decode tests for the renamed field, a clamp-report test, a coherence refusal.
- **Benchmark:** probe `sleep 70; echo done`. Assert a timeout outcome that names the maximum, and no hang.
- **Questions:** Is a background-process tool in scope? **Recommendation:** yes, after T9.

### T8. Tool output rendering escapes every non-ASCII byte, so copied text cannot match

- **Origin:** not in either spike; latent.
- **Symptom:**
  - Not observed: the fixtures are ASCII.
  - 56 Smith source and doc files contain non-ASCII text, for example `component.rs:2` "sections 4–6".
- **Root cause:**
  - `Sink::text` passes only `\n`, `\t` and bytes 32..=126 (`render.rs:49-58`). So "4–6" reaches the model as `4\xe2\x80\x936`.
  - If the model copies that into `edit.old`, the JSON decodes to a literal backslash sequence and the edit answers `NoMatch`.
  - It also quadruples the bytes of non-ASCII text.
- **Spike change:** none.
- **Assessment:** latent correctness hazard for edits.
- **Clean fix:** pass valid UTF-8 through unchanged, and escape only invalid sequences and control characters. Rendering stays deterministic.
- **Contracts:** `protocol/llm.md` §5 states the text policy.
- **Layer:** `smith-protocol-llm`.
- **Tests:**
  - Render tests: multi-byte, invalid and control bytes.
  - A session test that an edit copied from a rendered UTF-8 read matches.
- **Benchmark:** probe "replace the en dash on line 2 of component.rs with a hyphen". Assert one `Edited` outcome.
- **Questions:** none.

### T9. Commands are not contained: per-call roots are ignored, git directories are writable, and a timeout kills only the shell

- **Origin:** not in either spike. This is a known interim: "Process containment uses Skein's plain children until its contained-tree mechanism is available" (`local-host.md:239-240`).
- **Symptom:**
  - The per-call `roots` are discarded (`component.rs:125`). Commands can write anywhere, `.git` included, contrary to `domain/tools.md` §5 and `protocol/agent.md` §3.
  - `Process::stop` signals only the direct child (`process.rs:181-195`). skein-io creates no new process group (there is no `setsid` or `setpgid` in it).
  - A call is answered only after both pipes close, so a grandchild that keeps the pipes open can outlive the deadline. This is inferred from the code and not exercised on a real kernel.
- **Root cause:** contained trees do not exist yet.
- **Spike change:** none.
- **Assessment:** a known gap, but T1, T3, T7 and T10 all assume the view.
- **Clean fix:**
  - **skein:** contained trees (namespaces, a cgroup per tree, read-only mounts).
  - **Interim in skein-io:** a process group per child, signalled as a group.
  - **Interim in smith:** surface "commands are not contained" to the operator at startup.
- **Contracts:** `protocol/agent.md` §3 should state the interim honestly. skein `docs/design/draft/process.md` owns the mechanism.
- **Layer:** skein-io and contained trees, then `smith-protocol-machine`.
- **Tests:**
  - A skein real-kernel test: a grandchild holding the pipe is killed at the deadline.
  - Machine world: a hung grandchild is answered `TimedOut` within the grace steps.
- **Benchmark:** probe `sh -c 'sleep 600 & wait'` with a 5 s timeout. Assert an answer within deadline plus grace, and that no `sleep` survives.
- **Questions:** Should process groups land in skein before contained trees? **Recommendation:** yes.

### T10. Ambient tool configuration leaks into agent builds

This covers stale and read-only build-cache artifacts, and a CPU limit set through an environment variable.

- **Origin:**
  - The reliability report's configuration paragraph: `RUSTC_WRAPPER` empty, `CARGO_BUILD_JOBS=2`, "a stale cached build artifact".
  - The benchmark report's "sandbox failures caused by read-only warm Cargo artifacts".
- **Symptom:**
  - Baseline Codex runs failed `cargo test` with "output file …libfixture-….rmeta is not writeable" (`runs/baseline-01_backoff-codex-*/agent/events.jsonl`).
  - The harness then detached hard links and chmodded the warm cache for both agents (`benchmark.py:65-73`).
- **Root cause (inferred, well supported):**
  - The user's `~/.cargo/config.toml` sets `rustc-wrapper = "kache"`. Read-only, multiply-linked artifacts are what such a cache produces, and warm-up ran with that configuration.
  - So neither failure is about Smith's or Codex's sandbox. Agent builds inherit the operator's tool configuration files, which environment variables can only partly override.
  - `CARGO_BUILD_JOBS=2` stands in for a resource limit the machine lacks.
- **Spike change:** only the user's settings file (not in the repository).
- **Assessment:** a design gap. Once "the rest of the machine read-only" (`protocol/agent.md` §3) holds, caches outside the workspace become unwritable. Whether cargo tolerates a read-only `CARGO_HOME` is unverified.
- **Clean fix:** an explicit "build profile" in the agent configuration:
  - the environment (T1);
  - writable cache paths mounted into the view;
  - read-only tool roots;
  - per-tree CPU, memory and pids limits through the cgroup;
  - the network policy;
  - a documented Rust preset.

  `smith check` warns when a configured wrapper's cache is not writable.
- **Contracts:** `protocol/agent.md` §3, §4 and §10.
- **Layer:** `smith` config, `smith-protocol-machine`, skein contained trees.
- **Tests:**
  - Configuration validation tests.
  - A skein real-kernel story: a write to a declared cache path succeeds and a write elsewhere is `ReadOnly`.
- **Benchmark:** make the cache normalisation a documented setup step for both agents. Probe: a fixture warm-built with a wrapper, then "run the tests". Assert exit 0.
- **Questions:** Should resource limits be set per command tree? **Recommendation:** yes, and retire `CARGO_BUILD_JOBS` from the examples.

### T11. Smith cannot sign itself in to Codex today; the user hand-wrote a Skein token record

- **Origin:** the reliability report's credentials paragraph, and `smith_smoke.py:7-21`. Spike 1 only added an access-only mode to the live tests.
- **How Smith signs in today (verified):**
  - The local host runs skein-oauth's public PKCE flow: a loopback listener, the URL shown at the terminal, a code exchange, and a `SKOT` record in a 0700 directory, refreshed 60 s before expiry.
  - It needs a hand-typed `oauth` registration (`local-host.md:119-169`, `local_settings.rs:119-146`) and a hand-typed `account_id`.
  - An account without `oauth` only uses an existing record, and never refreshes it.
- **Why the import script was needed:**
  1. **No provider preset.** The docs say only "use the issuer's registered public client" (`workflow.md:499-508`).
  2. **The redirect cannot be expressed.**
     - Smith parses the redirect authority as a `SocketAddr` (`local_auth.rs:128-133`).
     - skein-oauth accepts only `http://127.0.0.1:PORT/…` or `http://[::1]:PORT/…` (`skein-oauth/src/machine.rs:633-643`).
     - Codex's client uses `http://localhost:1455/auth/callback`. That is external knowledge, unverifiable offline.
     - Extra authorisation parameters are fine: skein appends with `&` (`machine.rs:686`).
  3. **The account id is manual.** It lives in `~/.codex/auth.json` (`tokens.account_id`; only the keys were inspected) and in the ID token's claims.
  4. **Refresh tokens must not be shared.** Rotation would log one tool out (`workflow.md:493-496`). So the importer copies only the access token, and when it expires the user must re-run Codex, re-import and restart.
  5. **The importer writes Skein's private codec by hand:** `b'SKOT' + struct.pack('>HIQQ',1,0,1,expiry_ns) + …` (`smith_smoke.py:14-16`).
- **Assessment:** the credential architecture is sound: grants are lent by the host (`domain/host.md` §7) and never enter configuration or traces. What is missing is acquisition that works for the two providers Smith targets.
- **Clean fix:** a credential source per account.
  - **`sign_in`, with a built-in preset for Codex and Anthropic.** skein accepts a `localhost` redirect host (bound to 127.0.0.1, compared exactly). The account id is derived from the token's claims, which skein's record can already hold.
  - **`borrow`.** Read the CLI's login file read-only, taking the access token and account id. Never touch the refresh token, and give a precise "run `codex` once to refresh" message when the token is expired.
  - **Optional `command`.** A credential helper program.
  - **No user-written `SKOT` bytes.** At most a `smith login --from codex` subcommand.
- **Contracts:** `domain/host.md` §7, `protocol/hosts.md` §5.4, skein `docs/design/oauth.md` §1.
- **Layer:** skein-oauth (redirect host), `smith` local auth (presets, borrow source), `smith-local-domain` (expiry notice text).
- **Tests:**
  - skein: `localhost` redirect accepted; `localhost.evil` and `localhost:0` refused.
  - Fake-issuer tests for the presets.
  - A borrowed-login world test: valid token lent; expired token gives a notice and no run; the login file is never written; the refresh token never reaches a trace.
  - Live stories for own sign-in and borrowed login.
- **Benchmark:** the harness should use `borrow`. Probe: assert one completion and an unchanged hash of `~/.codex/auth.json`.
- **Questions:**
  - Is reading another tool's login file acceptable? **Recommendation:** yes, read-only, access-token-only, opt-in per account.
  - Should skein accept `localhost` redirects? **Recommendation:** yes.

### T12. A first-time local run needs expert settings, and mistakes surface as bare refusals

- **Origin:**
  - The reliability report's configuration paragraph.
  - Spike 2 commits `fc939dc` (the docs example `max_tokens` changed from 8192 to 4096) and `3fae871` (`sub_agents`).
  - Benchmark setup errors. Mostly not in the spikes.
- **Symptom:** a first-time user must write all of the following.
  - **Agent object:**
    - `profile`;
    - `memory_bytes` above a checked bound of about 360 GB (README:32-37; startup prints "worst case 275236501056 bytes");
    - `grace_ms`;
    - endpoints with `number`, `dialect`, `account`, `provider`, `address`, `server_name` and effort.
  - **Explicit environment.**
  - **Run settings:**
    - `chat` and `instructions`;
    - priced `models`, with zero prices for subscriptions;
    - `budget` and `waiting_seconds`;
    - `accounts`.

  Observed failures:
  - The docs example's `max_tokens: 8192` was refused until `fc939dc`.
  - A 128-turn budget on a 64-turn binary passed host startup. The terminal said only "The agent refused this run".
  - Policy errors print Rust `Debug` (`local_settings.rs:282`).
  - `/usr/bin/rg` is hard-coded (`process.rs:417`), so search fails if rg lives elsewhere (inferred).
  - Workspace overrides replace arrays, so adding one variable means restating `PATH`.
  - `delivery_environment` and `environment` use different formats.
  - The docs say prices must be positive, but zero is accepted (`charter.rs:288`).
- **Root cause:**
  - The settings mirror the agent configuration and charter directly. There are no defaults, no presets, and no host validation against the agent's profile, although the host has `agent_limits`.
  - The memory bound is cross-theme.
- **Assessment:** the strictness is right. Defaults, presets and actionable errors are missing.
- **Clean fix:**
  1. **Provider presets** that expand to endpoint and account, written to `agent.json`.
  2. **Defaults:**
     - a derived memory ceiling;
     - grace, waiting and budget defaults;
     - unpriced subscription models;
     - a visible core environment inheritance;
     - `environment` as an object, so overrides merge per variable.
  3. **`smith check`, also run at startup:**
     - prints the resolved settings, redacted;
     - names profile violations ("budget.turns 128 exceeds this binary's ceiling of 64");
     - checks credential source and expiry;
     - resolves `sh`, `rg` and `git` (configurable paths);
     - enforces credential-name hygiene in the environment (Codex excludes `*KEY*` and `*TOKEN*` by default);
     - warns about build wrappers.
  4. **Human-readable refusal reasons** reach the terminal.
- **Contracts:** `protocol/hosts.md` §5.2, `domain/host.md` §8, `protocol/agent.md` §4, and a rewrite of `local-host.md`.
- **Layer:** `smith` (settings, presets, check command), `smith-local-domain` (refusal reasons).
- **Tests:**
  - Preset expansion is byte-stable.
  - One case per named error.
  - Per-variable environment merge.
  - An end-to-end `smith check` test, both failing and passing. Each takes milliseconds.
- **Benchmark:** the harness starts from a five-line preset settings file. `smith check` passes and a one-line task finishes.
- **Questions:**
  - Should subscription models be unpriced? **Recommendation:** yes.
  - Should there be a core-inherited default environment? **Recommendation:** yes, named explicitly, never matched by pattern.

---

## Cross-theme dependencies

- **Request envelope, history and dialect limits.** T5 and T6 depend on the 1 MiB envelope, the 256-message cap, `result_bytes`, and a policy for old tool results. Without that, larger results move the cliff from round trips to context exhaustion.
- **Memory accounting.** The 360 GB bound (T12) and the environment and output terms (T1, T5) need a tighter derivation before `memory_bytes` can be derived.
- **Prompt caching.** Fewer round trips (T4) cut uncached input only if the provider cache hits.
- **Delegation.** Children re-read guides and repeat path mistakes (T3, T6), so the guide inheritance and child tool surface follow that theme's decisions.
- **Run diagnostics.** The reasonless "The agent refused this run" (T12) and "operation failed: io error" for incoherent limits (T5) depend on typed reasons reaching the terminal.
- **skein contained trees and process groups.** T3, T7, T9 and T10 all wait on these. Process groups are a small first step.
- **skein-oauth.** Its `localhost` redirect rule gates T11's Codex preset.
- **Waiting and finish mechanics.** The `finish` round trip is one row of T4's breakdown.