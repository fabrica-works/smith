# smith-bench

Real-provider benchmarks show whether a change has its intended effect.
Offline checks run in the workspace's focused suite, without network
access or credentials. The contract is `docs/design/benchmarks.md`.

Run commands from a clean committed worktree. Build each named smith
commit in the build lane before running it in the gate lane:

```sh
heavy cargo run -p smith-bench -- build HEAD
heavy cargo run -p smith-bench -- build CONTROL_COMMIT
heavy --gate cargo run -p smith-bench -- probe probes/smoke-tools --arm DIRECTORY
heavy --gate cargo run -p smith-bench -- calibrate probes/first-request --control CONTROL_COMMIT
heavy --gate cargo run -p smith-bench -- fix probes/first-request --parent CONTROL_COMMIT
heavy --gate cargo run -p smith-bench -- suite smoke --arm DIRECTORY
```

`build COMMIT [--repository REPOSITORY]` prints the archived arm directory
as its last line. It builds only the named commit in a private clone, using
Cargo's default dev profile without `--release`, and records executable
digests and the rustc version. Dirty sources, dirty clones and altered
archives are refused. `--commit`, `--control` and `--parent` reuse these
archives; live commands never compile a smith arm.

```text
probe PROBE [--agent smith|codex|smith-claude-code] [--attempts N] [--commit COMMIT | --arm DIRECTORY]
calibrate PROBE [--attempts N] [--passes N] [--control COMMIT] [--commit COMMIT | --arm DIRECTORY]
fix PROBE --parent COMMIT [--commit COMMIT]
suite NAME [--include TASK]... [--seed N] [--commit COMMIT | --arm DIRECTORY]
```

A probe name is `probes/<id>[/<variant>]`. The default source is `HEAD`,
the agent is smith, and a probe runs once, rerunning once on failure.
Codex uses its standard pin and decides on the outcome half of the task.
Every attempt prints its run and attempt ids, end, verdict, wall seconds
and spend. Unavailable measurements print `-`. A summary follows.
Exit codes are 0 for passed or calibrated, 1 for failed or not calibrated,
and 2 for incomplete evidence, setup refusal or a harness error.

Calibration defaults to ten passes within twelve logical attempts. A
cache probe's triplet is one logical attempt. A control runs first, up to
two attempts, stopping at its first failure; two passes write no block.
The changed arm stops when enough
passes arrive or the remaining attempts cannot attain them. Successful
calibration writes the task or variant block and its summary under
`benchmarks/summaries/`, and never commits. Model deviations count apart
from smith failures, and changed prompt, seed or check digests gate nothing.
`fix` prints both
arms' runs and the finding for a commit body. Suites use their pinned
agents, budgets and seeded ordering; the seed is printed.

Other commands:

```sh
cargo run -p smith-bench -- check [TASKS_DIRECTORY]
cargo run -p smith-bench -- validate [TASK]...
cargo run -p smith-bench -- attempt PREPARED_REQUEST
cargo run -p smith-bench -- guards --suite SUITE [--tasks TASKS_DIRECTORY] SECTION...
cargo run -p smith-bench -- summarise RESULTS_DIRECTORY SUITE TIER single SEED
cargo run -p smith-bench -- baseline benchmarks/summaries/RUN.json NAME [--group AGENT CONFIGURATION_SHA256 MODEL EFFORT]
cargo nextest run -p smith-bench
```

`check` validates tasks, suites, model tiers, prices, pins, summaries and
baselines. Tasks freeze their seed digest and complete prompt. Suite
names are `<kind>/<id>[/<variant>]`; behaviour selections resolve through
the task catalogue. Pins freeze each agent's provider, version and exact
configuration bytes. Unknown fields and unresolved model tiers refuse
setup. The small tier is pinned for both providers; the Codex working
tier uses `gpt-6-astra` at high effort.

`validate probes/credentials` checks the probe's offline pair controls: an
unchanged generation and a refused refresh fail, and two completions with
an advanced generation pass. It reads no login and calls no provider.

`validate` runs every graded task, or the named tasks such as
`fixtures/01-backoff`, on private copies of its seed and reference
solution. It prints both grader terminals and succeeds only when every
seed fails and every reference passes. Graders never run in the focused
suite. `compare` interleaves both fixtures across smith's comparison pin
and Codex's standard pin, five times each at the working tier.
`fixtures/03-planner` runs only with `suite compare --include
fixtures/03-planner`; each repeated `--include` must name a task or
variant listed under the suite's `when_asked`. Compare runs reserve two
hours and eight million tokens for all three fixtures.

`guards` lists affordable tasks guarding the named design sections,
cheapest first, and explains omissions. Estimates reserve repetitions,
agents, arms and a failure rerun. Matching committed medians replace
estimates; missing costs remain unavailable.

`summarise` reads result files recursively or one explicit result. It
writes `benchmarks/summaries/<run>.json`, recording the suite, tier,
design and seed. An optional baseline path is its final argument.
Summaries use version 4 and name the harness commit as `priced_at`.
Cross-agent comparisons retain both token conventions and use common priced
spend from the product presets at that commit, pricing each response at the tier its own prompt reaches. The declared window does not select a rate. Tool calls stay within each arm; only task
and process wall time, token fields and priced spend compare across agents.
`baseline` freezes one binary, configuration and tier under
`benchmarks/baselines/<name>.json`. For a suite with several configurations, `--group` selects one agent,
configuration digest and requested model/effort from the whole summary.
A selection with no observations or mixed binaries refuses. Both refuse
overwrites. Commit these
summaries and baselines; raw attempt data stays outside repositories.
Smoke and fix evidence belongs in commit bodies.

Results are strict version-three JSON documents with observed,
lower-bound or unavailable measurements. Missing observations never
become zero or a root-only fallback. Token records retain their scope
and convention, deduplicate responses and name missing child evidence.
Only interleaved runs compare arms, with five complete observations each
and seeded bootstrap resampling. Drift notices request a comparison;
they do not declare a regression.

Runtime writes pass the shared guard and live under
`$XDG_STATE_HOME/smith-bench/`: `requests/`, raw `runs/`, private `homes/`,
build `clone/` and `arms/`, which `build` writes once and never rewrites,
and attempt `scratch/`. Each attempt
uses fresh private paths, a frozen seed and a supervised process tree.
Setup stays outside the inference timer. Grading follows settlement in
its separate home. Workspaces are removed after their evidence is saved.

Once per machine, create the harness's own logins. These are separate from
personal tool homes. The harness never creates or writes `logins/`; smith
and Codex refresh their own records there through normal use.

```sh
export BENCH_LOGINS="${XDG_STATE_HOME:-$HOME/.local/state}/smith-bench/logins"
install -d -m 700 "$BENCH_LOGINS" "$BENCH_LOGINS/smith" "$BENCH_LOGINS/codex"
install -d -m 700 "$BENCH_LOGINS/smith/config/smith"
cat > "$BENCH_LOGINS/smith/config/smith/config.toml" <<EOF
credentials = "$BENCH_LOGINS/smith"
EOF
cat benchmarks/agents/smith/accounts.toml >> "$BENCH_LOGINS/smith/config/smith/config.toml"
chmod 600 "$BENCH_LOGINS/smith/config/smith/config.toml"
XDG_CONFIG_HOME="$BENCH_LOGINS/smith/config" smith login chatgpt
XDG_CONFIG_HOME="$BENCH_LOGINS/smith/config" smith login claude
CODEX_HOME="$BENCH_LOGINS/codex" codex login
```

The account declarations come from `agents/smith/accounts.toml`, included
by the harness as `credential::logins::ACCOUNTS`. Each attempt's own XDG user layer
uses those accounts, the pin's trace and `credentials = "<logins>/smith"`.
The smith arm selects `openai/<model>` with `endpoints.openai.account = "chatgpt"`; smith alone
refreshes that shared record under its lock. Each attempt keeps private state.
The Codex arm reads only its harness login's access, id and account fields
into a private access-only file. Its expiry must cover the deadline and
settlement; private auth is removed and the source digest checked afterwards.
Missing setup refuses by name with the absolute sign-in command.
No user's settings, instructions, skills, plugins or memories enter an
attempt. Never put credentials in task seeds or exported artifacts.

The `smith-claude-code` arm selects the Anthropic small tier with
`anthropic/<model>` with `endpoints.anthropic.account = "claude"` and
`endpoints.anthropic.program = "claude_code"`. The product spawns `smith-agent`
and Claude Code (`claude -p`) from the attempt's PATH, using the harness's
`logins/smith/programs/claude` configuration directory. A harness lock
outside `logins/` serializes these attempts across commands. This arm has
its own name and configuration digest, so its results stay separate. The product
stream supplies the answer, end and host notices; the separate agent trace supplies
usage, calls and event checks. Both streams must be whole. This arm records
`tool-call` checks as not measured with the reason "Claude Code's built-in tools"
and removes them from deciding checks; every other check still applies.

```sh
heavy --gate cargo run -p smith-bench -- probe probes/smoke-tools --agent smith-claude-code --arm DIRECTORY
heavy --gate cargo run -p smith-bench -- probe probes/credentials --arm DIRECTORY
```

The credentials probe runs two smith attempts concurrently on `chatgpt`,
with `deployment.refresh_lead=34560000`. It requires both accepted answers
and a larger saved generation, using smith's `credentials::record_generation`
reader. Only generation numbers enter its evidence.
A refused refresh cannot pass. An isolated `attempt` leaves this pair check
unavailable; run it through `probe` or `suite`. The minimum-source pin names
`446ebd12c1ab3e5c32e0ae7e7d4392f258626f64` (smith-14r), the first smith
with the refresh-lead policy.

Smith's append-only face registry selects the settings and event layout
for a frozen binary. Codex's JSON-lines
observer is pinned to CLI 0.160.0; named-thread rollout collection retains
root and child scopes. Known malformed records fail loudly; unknown
record kinds are skipped. Typed recording controls exercise the same
observers and supervision without running a real agent.

The repository suite uses `shell-cwd` frozen at smith
`56c0c59735f3a163753fdd43e20a7e87df369ce3`, with skein
`f5fca44f2c9ee92f2dc52d8dc845212ca692275f` and registry dependencies
vendored. Its manifest is `tasks/repository/shell-cwd/seed/bundle.toml`;
the directory lives at
`$XDG_STATE_HOME/smith-bench/bundles/shell-cwd-56c0c59/` on
`debian-16gb-hel1-1`. An absent bundle is reported as skipped offline,
and refuses an attempt. A present bundle must match its complete seed
digest. The task's own seed digest binds the bundle manifest.

To reproduce it with the two source commits available locally, export
smith's frozen tree to that directory with `git archive`. From the
exported directory run `heavy cargo vendor --locked vendor`, and save
Cargo's printed source replacements in `.cargo/config.toml`, using
`directory = "vendor"` so each copied workspace builds independently.
Export skein's three `tests/tls/fixtures/{leaf.der,intermediate.der,leaf.key}`
files at the frozen pin to those paths in the bundle; the vendored fake
peers include them by their existing relative paths. Keep the frozen
`Cargo.lock` and `.config/nextest.toml` unchanged. Build
once with `heavy cargo build --offline --locked --workspace --all-targets
--target-dir DIRECTORY_OUTSIDE_THE_BUNDLE`. The directory's digest,
including the vendored sources and config, must match `bundle.toml`;
never keep build output inside it.

The repository reference is the spike's successful Codex solution as
`reference/solution.patch`, excluding its dated workflow log. Validation
applies that patch to a private bundle copy. It prints the repository
oracle and every grade command's bounded output. The five grader
command deadlines sum to 360 seconds (180 seconds before the user's doubling of 10-22, provisional); focused and fuzzy nextest still
use the frozen seed's 15 and 60 second budgets. Both arms start from the
same unbuilt copy, with no shared attempt cache.

New product arms run `smith check --json` during setup, then
`smith exec --json -C <workspace> -m <endpoint>/<model> --effort <effort> -`.
The attempt's XDG user layer pins settings and selects the harness's
shared sign-in record. Teardown comes from check's effective `service.teardown`.
A nonzero check refuses setup; explicit startup-refusal probes retain that
check's exit, diagnostic and settled tree as their observation. Historical
event arms keep their original native configuration and credential codec.

`smith-bench suite parity --commit SHA` runs the three fixtures eight
times per arm and `shell-cwd` ten times, interleaved without reruns.
A pipeline check with `--attempts 1` runs each task once per arm, writes
`summary.json` in its run directory, and exits 2 (incomplete).
Its summary records the five non-inferiority criteria. Recompute them offline
with `smith-bench parity benchmarks/summaries/RUN.json --commit SHA`: exit 0
at parity, 1 when smith is not on par, and 2 for incomplete evidence or a
conflicting section or smith source commit. Infinite bootstrap ratios are
stored as `"infinite"` so that unsolved or incomplete sections round-trip.

`smith-bench suite parity-change --commit SHA --parent PARENT` runs the
four parity tasks five times per smith arm, interleaved without reruns.
Both commits must be built first; `--arm DIRECTORY` and
`--parent-arm DIRECTORY` select built directories instead. The arms are
`change` and `parent`, with `parent` the reference. Other suites refuse
parent options.

A change report takes its named run summaries with `--baseline-run SUMMARY`
and repeated `--landed SUMMARY` options on `suite parity-change` or
`summarise`. It records their paths and SHA-256 hashes, projects every
landed ratio and this change onto the baseline, and stops on incomplete
evidence or a criterion past its bound. The suite exits 0, 1 or 2 for no
stop, a moved criterion or incomplete evidence; a run below its attempts
always exits 2 and retains its run-local `summary.json`.

`smith-bench change SUMMARY` rereads those inputs and recomputes the report.
It exits 0 when the stored section agrees, including a stopped section,
and 3 on missing or changed inputs or a disagreement. `check` applies
that verification to every committed change summary.
