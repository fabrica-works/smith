# smith-bench

Attempt results are version-one JSON documents (`src/result.rs`), with strict
fields and explicit observed, lower-bound or unavailable measurements. The
token ledger retains one record per conversation scope and convention,
deduplicates response ids, and names missing scopes in partial totals. A
missing root keeps the total unavailable. Completed answers remain completed
when the harness forces their teardown, with a warning; budget and setup ends
are reported beside the eligible rate counts.

Real-provider benchmarks show whether a change has its intended effect.
Offline checks run in the workspace's focused suite, without agents,
network access or credentials. See `docs/design/benchmarks.md` for the
contract.

Commands available so far:

```sh
cargo run -p smith-bench -- build COMMIT [--repository REPOSITORY]
cargo run -p smith-bench -- check [TASKS_DIRECTORY]
cargo run -p smith-bench -- attempt PREPARED_REQUEST
cargo run -p smith-bench -- run SUITE --prepared REQUEST_DIRECTORY SEED
cargo run -p smith-bench -- guards --suite benchmarks/tests/recorded/suites/sample.toml --tasks benchmarks/tests/recorded/tasks/valid 'benchmarks.md, section 5.2'
cargo run -p smith-bench -- summarise RESULTS_DIRECTORY SUITE TIER single 123
cargo run -p smith-bench -- baseline benchmarks/summaries/RUN.json NAME
cargo nextest run -p smith-bench
```

`check` validates tasks, suites, model tiers and agent pins in this tree.
With a directory argument, it checks only tasks below that directory.
A refusal names its file and key path. Tasks carry
the complete prompt, under 4,088 UTF-8 bytes, as one paragraph ending in
a newline. `seed_sha256` freezes their `seed/` directory.

The seed digest is SHA-256 over files sorted by relative UTF-8 path.
Each file contributes the path length as a big-endian u64, path bytes
(with `/` separators), one byte indicating any executable permission,
the byte length as a big-endian u64, and the file bytes. Empty directories
contribute nothing. Symlinks, special files and `.git` directories are
refused. No seed may contain `benchmarks`, `grader`, `grading`,
`reference` or `reference-solutions` directories.

Runtime data will live under `$XDG_STATE_HOME/smith-bench/`, outside
repositories: raw `runs/`, agent `homes/`, build `clone/` and `arms/`,
and attempt `scratch/`. Summaries and baselines are committed here.
Agents use the user's existing login read-only. Codex runs in an attempt-owned
home, and smith receives a private access-only record until its borrow source
exists. No one signs in for a benchmark or copies a refresh token.

Suite task names are `<kind>/<id>[/<variant>]`; `[[select]]` can instead
name a `kind` and `behaviours`, matching all those behaviours. An agent
configuration names `agents/<agent>/<config>.pin.toml`: its agent,
provider, version and relative configuration file. Its digest includes
the exact metadata and configuration bytes, each framed with its byte
length as a big-endian u64. `[[arm]]` has `source = "binary"`,
`"override"` or `"agent"`, with the corresponding commit, smith settings
or agent pin. Comparison schedules arrive with their tasks.

`guards` uses a suite's wall and token budgets to list guarded tasks
cheapest first and explain omissions. Estimates reserve the configured
repetitions, agents, arms and failure rerun. Missing costs are reported
as unavailable. The latest committed medians in `summaries/` replace estimates
only when their task version, digests, tier, model and current agent pins match,
and every configured agent has complete wall and token observations. Variant
costs are separate; missing variant costs fall back to the base estimate.

`summarise` reads `result.json` files recursively or one explicit result file.
It writes `summaries/<run>.json`, recording the seed, suite, tier and design
(`single` or `interleaved`). An optional baseline path is its final argument.
Every end and every missing metric is counted; medians use all observed ends,
with observed and total counts beside them. Only `interleaved` runs compare
arms, with five complete observations each and 10,000 seeded resamples.
Comparisons preserve token conventions and spend bases; cross-agent resource
figures are kept apart. Audit samples include every check/grader disagreement.

`baseline` freezes one configuration, binary and tier in `baselines/<name>.json`.
Both commands refuse to overwrite an existing record. Drift notices ask for a
comparison in one session. They never declare a regression. No grader is
represented by an absent grade; an expected but missing grade has an explicit
unavailable reason.

The `small` tier is pinned for both providers. `working.codex` awaits
the user's model and effort; `working.anthropic` belongs to the next
comparison pass. A run requiring an unresolved tier refuses setup.

Codex's JSON-lines observer is pinned to codex-cli 0.160.0. It retains typed
thread, turn and item records, skips unknown kinds, and reports known shape
changes with their field path. The final turn and settled exit decide the end;
a forced teardown after its answer is a warning. Turn usage is root-only and
repeated terminals count once. Cache writes remain within input, and reasoning
within output; omitted fields are unavailable. JSON lines have no provider
response ids or complete child scopes, so response counts stay unavailable
and totals name the missing non-root evidence. Rollout collection supplies
those scopes later.

The recordings' manifest under `tests/recorded/codex/` pins archive sources,
retained lines and SHA-256 hashes. Completed and children are archived;
failed and repeated are explicitly synthetic derivatives awaiting the first
live recordings. The fuzzy parser sweep is `cargo nextest run -p smith-bench
--profile fuzzy`. These tests read no login and run no agent.

The shared `smith_bench::guard` checks every runtime write destination after
resolving existing ancestors and links. It protects the user's Codex and Claude
homes, `.claude.json`, and smith's XDG configuration and state directories,
including custom tool homes. Relative paths, parent traversal, dangling links
and aliases into those locations are refused before writes. The live tier uses
the same guard and reads its default models from `agents/models.toml`.
Credential conversion refuses refresh-token fields; a pre-borrow binary can
receive only the guard's explicit unusable sentinel, alongside the access token
and account id. The guard reads no login and never reports credential values.

`build` resolves a clean local source (the harness workspace by default),
fetches the commit into its private `clone/` and uses `cargo build --release
--locked --package smith --bins`. It captures Cargo's executable artifacts and
archives them under `arms/<full-commit>-<smith-sha256>/`. `arm.json` records each
binary's digest and the verbose rustc version. Rebuilding an identical arm
verifies its manifest and hashes instead of overwriting it. The clone has an
ownership marker; a dirty clone or changed arm refuses the build. Sources are
read-only, Git hooks and global configuration are disabled, and no developer
worktree is created. Build caches are private under the same state root.

Run each build as its own `heavy` command, separately from an attempt, with
the plan README's scoped local skein URL rewrite exported. Runtime data must
be outside repositories and every destination passes the shared guard.

The legacy smith reader maps the original `30259d8` Debug trace to neutral
run, conversation, completion and tool observations. It parses nested values
by field name, retains exact hex content, and identifies main's accepted or
delivered finish by the run-issued call. Child finishes never end the task.
Known shape changes are harness errors. Its calls-only fake-provider recordings
are under `tests/recorded/smith-legacy/`; usage and loss stay unavailable there.
Legacy exit zero alone proves no accepted answer. Forced teardown after an
observed acceptance remains completed with a warning. The legacy parser joins
the same offline and fuzzy suites as Codex's reader. The current ancestry-selected
Choice layout retains logical request intent without treating it as a provider
attempt start. `tests/recorded/smith-current/` additionally retains the actual
Everything trace from phase1 control 2396, its binary/source/harness identities
and original digest. Its three response usages are counted once; repeated
content usage and aggregate Used/Ended facts do not double charge them. Trace
loss and provider identities remain unavailable.

`attempt` consumes one strict JSON `Prepared` request (`src/run.rs`) in a fresh
harness process. A request freezes the task, binary and attempt identities,
arguments and deadlines. Setup uses private homes and warmed caches outside
the inference timer. The shared kit supervises the entire process tree;
the runner records actual pipe observations, bounded logs and scoped resource
counts. Outcome checks, protected paths, seed diffs and the external grader
run after tree settlement. Grading uses a separate home and pre-agent cache
copy. Raw archives include the request, task and prompt, and writable workspaces
are deleted after their evidence is retained. A setup refusal remains a result.

`run --prepared` validates adapter-produced requests against the suite's task
catalogue, configuration pins and model tier, then starts one fresh harness
process per attempt. It uses the seeded interleaving already shared with the
summariser, respects aggregate wall and token budgets, and retains the probe
rerun verdict. Missing whole-run token observations stop further dispatch and
mark the suite incomplete. Missing child results become harness errors with
the dispatcher's bounded logs and settlement evidence; parent resources are
never labelled as agent-only measurements. Invocation preparation belongs to
the upcoming Codex and Smith adapter increments; the current pipe face consumes
Codex JSON lines. Recorded replay programs provide offline runner controls.

The Codex adapter checkpoint adds a `configuration` path to a prepared request.
It verifies the installed executable digest and version before inference,
renders the pinned command itself, and archives its rendered overrides.
Its attempt-owned `CODEX_HOME` starts with only private access-only `auth.json`,
converted read-only from the selected source login. The one converter copies
id/access/account fields, leaves refresh empty, sets the write timestamp, and
refuses tokens expiring before the hard deadline and settlement ladder.
The guarded directory and file have modes 0700 and 0600. After tree settlement,
the private auth file is removed and source auth digests must match; only this
nonsecret proof is archived. No user configuration, instructions, skills,
plugins or memories are read or copied. Requests carrying a source Codex home
require this configuration path and cannot supply arbitrary arguments.
Unpinned pipe replays remain an offline control and cannot name a login directory.

Named-thread collection opens `state_5.sqlite` read-only, queries only the root
and its descendants, and reads their ordinary session files. It bounds the
attempt at 4,096 named threads and 64 MiB of raw rollouts. Immutable response ids
replace root turn aggregates; identical repeats count once. A child without
native parent evidence has an unavailable `unlinked-child` scope, making totals
lower bounds. Missing response fields remain unavailable rather than zero.

Installed codex-cli 0.160.0 accepts the arbitrary-root trust key as
`projects={"<workspace>"={trust_level="trusted"}}`. A quoted dotted-key
spelling was ignored because the [pinned override parser](https://github.com/openai/codex/blob/rust-v0.160.0/codex-rs/config/src/overrides.rs)
splits dots literally. The whole-table value preserves paths containing dots.
The [pinned configuration source](https://github.com/openai/codex/blob/rust-v0.160.0/codex-rs/core/src/config/mod.rs)
sets the default `agents.max_threads = 6`, now explicitly pinned.

Actual run `smith00-codex-isolation-20261009-1733/isolation-02` authenticated
from the private access-only home at gpt-6-luna/low. Its source auth digests
match, private refresh remains empty, auth is removed after settlement, and
its rollout contains only frozen seed instructions and four Codex built-in
skills rooted inside that attempt. No user instruction, skill, plugin, memory
or credential markers appear. These built-ins are installed by Codex itself.

Run `smith00-codex-child-20261009-1740/child-03` completed with one native linked
child, six immutable responses, and observed output totals 145 root + 41 child
= 186. A child's rollout includes its own parent-linked metadata followed by
inherited ancestor metadata; collection retains ownership from its own header.
Run `smith00-codex-failure-20261009-1757/failure-01` is the planned native
unknown-model failure. Both preserved auth digests, removed private auth and
scratch, and produced no stderr. Their actual stdout and projected usage
rollouts replace the synthetic failure and repeated-terminal fixtures, with
credential scan and exact source/destination digests in the fixture manifest.
The per-response/turn/thread usage fields provide actual repeated usage;
identical SDK terminals are repeated only by explicit parser mutations.

The child-01 and child-02 attempts exposed parser faults before the successful
child-03 control. Child-01 lost malformed rollout bytes; child-02 and subsequent
attempts retain bounded raw files before decoding and archive them on failure.
These controls establish adapter behavior, not calibrated reliability probes
or the pending runtime smoke backfill for other increments.

The native Smith driver verifies every executable in the frozen arm manifest
and identifies the agent by its full source commit and binary digest. Smith
has no `--version` command. The append-only `agents/smith/faces.toml` registry
selects settings and token formats by source ancestry; earlier arm invocations
remain intact. The driver renders `agents/smith/legacy.json`, task overrides
and exact environment values into guarded private settings. The baseline's
standard profile admits at most 4,096 output tokens. Unsupported named
overrides refuse setup before reading the source login.

Smith runs spawned at a terminal. The scripted person sends one whole prompt
line, waits for named tool or answer cues before later lines, and sends EOF
once main answers, including a failed or budget answer. The same Skein process-tree supervisor reads
the growing trace file and drains its final records after genuine settlement.
Two consecutive native IO close deadlines give the teardown allowance; the
driver preserves the binary's shipped close and exit grace policy.

The sole credential converter writes Smith's selected saved-token codec in a
private directory. Old binaries receive the unusable refresh sentinel that
their codec requires; access-only codecs omit refresh. The expiry check also
covers native renewal's sixty-second lead, so neither shape can renew during
the attempt. Source auth digests and private removal are recorded after
settlement. Legacy per-response content usage is counted once by conversation
and completion ordinal; aggregate `Used` facts never add another charge.
Unobservable loss and parent links keep measurements lower bounds or
unavailable. The pin's GPT-5 list rates are notional comparison prices, not
provider billing or a price claim about the selected model.

The first native controls are archived as
`smith00-legacy-20261009-1932/baseline-01` and
`smith00-legacy-20261009-1934/baseline-02`. The first exposed the pin's output
ceiling error before any provider attempt. The corrected second admitted one
attempt but received no response or usage before its deadline. Both preserved
the source auth digest and removed private credentials and scratch. The
required live `everything` recording with usage remains pending; these
controls establish no authentication success or reliability probe result.

The bounded diagnostic `smith00-legacy-diagnostic-20261009/diagnostic-01`
used gpt-6-luna/low, a 300-second native budget and a separate 330-second hard
deadline. Metadata-only `ss` samples show the native agent and its socket gone
by 2.230 seconds, after 8,325 outbound bytes were acknowledged and 2,821 bytes
were received. Encrypted counters do not establish a complete HTTP upload or
its response. The local host waited for input until the harness deadline;
source auth remained unchanged and private credentials and scratch were
removed after settlement. The designer classified this as a baseline agent
defect; its exact exit cause is unobservable on the frozen binary. The
60-second initial budget remains part of baseline-02's outcome. No further
baseline diagnosis runs: the requested wrapper was withdrawn because that
host hardcodes its own executable and has no agent-program override.

The first later candidate, `1a0dade1`, is the complete hosted-stderr/shared-drive
adoption after the frozen baseline; its predecessor only changes the dependency
pin. Run `smith00-legacy-later-20261009-2010/control-01` also timed out at its
330-second deadline, with no response or usage. Its private selection annotation
names both failed baseline attempts and retains the encrypted-TCP knowledge
boundary. That historical selection did not complete the required usage control;
the later phase1 control 2396 did and is retained as an offline recording.
The first `c883e2e` candidate, `smith00-legacy-later-20261009-2027/control-02`,
failed before any provider request because the harness incorrectly duplicated
the endpoint model declaration. The native local host generates this from
`settings.models` and rejects a competing declaration in `agent.endpoints`.
The corrected test follows the shipped local-to-agent generation path and then
the native child parser, including a changed output limit.
`smith00-legacy-later-20261009-2036/control-03` retained a second harness
settings failure: the agent refused the run before inference, and the local
host remained until the 330-second hard deadline. The declared-settings face
had applied the later product budget and micro-dollar prices to a binary whose
standard ceiling was 64 turns and one original currency unit. The registry now
selects the priced shape only from `a40cedc2`, where that policy changed.
The corrected `smith00-legacy-later-20261009-2044/control-04` reached a
completion attempt and ended after 675 ms with `Limit` and `Unknown` evidence.
Its fourteen retained records include the request, with no response or usage
content. That binary erases the shared limit's cap and bound, so the trace
does not establish the precise limit or an HTTP status. The successful usage
control waits for an eligible later native mechanism; no failure is converted
to passing evidence, and no further frozen-baseline diagnosis is run.
From `576a251e`, the registry selects the native Kind-based Debug facts,
retaining asked/granted share metadata and visible parent links. Frozen arms
keep their flag-based layout. Linked child response content contributes once
in its own scope; aggregate or child-return totals never charge it again.

## Smoke probes and deciding observations

`smoke.toml` currently selects five probes. `failures/one-turn` is authored
and checked offline but enters the suite only after proven product 10.5.
It expects the product's entrance refusal, exit2 and a typed invalid budget
reserve record, rather than an admitted run exhausting its budget.

Each probe's `deciding` names select its verdict; advisory observations remain
in the result. Both `attempt` and `run` return0 when every deciding observation
is observed true,1 for a deciding false or unexpected failed run, and2 when a
deciding observation is missing, lower-bound or unavailable. An explicitly
expected refused/budget diagnostic is judged by its named end/exit checks.
No unavailable smoke check counts as a pass.

`fix` separates frozen arm building from prepared inference so each uses its
own machine queue. Both steps retain the exact binary identities:

```sh
smith-bench fix PROBE PARENT FIX --build-only
smith-bench fix PROBE PARENT FIX --prepared REQUEST_DIRECTORY SEED
smith-bench calibrate PROBE --prepared FIX_REQUEST_DIRECTORY --control CONTROL_REQUEST_DIRECTORY SEED
```

Run the build step under the heavy wrapper and the prepared steps under the
live wrapper. The runner refuses an unstated invocation; it never invents
credentials or policies to make a comparison runnable. A fix's report prints
both immutable arms, run identifiers and its finding, including unavailable
or setup-refused controls. Calibration requires ten changed-arm passes and a
failing parent/control; every failure and rerun stays archived. No probe is
marked calibrated by this offline authoring checkpoint.

`first-request` currently exercises the merged `smith local` typed-input
invocation and its fixed writable workspace. Its headless and
without-workspace variants guard `10-product` 10.5 and are reported as
not yet supported by the current invocation; they are not judged or claimed
proven by this interactive task. The one-turn startup refusal is authored
with its product expectation but stays outside both active suites until
10.5 is merged and proven.
