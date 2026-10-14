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
as its last line. It builds only the named commit in a private clone and
records executable digests and the rustc version. Dirty sources, dirty
clones and altered archives are refused. `--commit`, `--control` and
`--parent` reuse these archives; live commands never compile a smith arm.

```text
probe PROBE [--agent smith|codex] [--attempts N] [--commit COMMIT | --arm DIRECTORY]
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
Summaries use version 3 and name the harness commit as `priced_at`.
Cross-agent comparisons retain both token conventions and use common priced
spend from the product presets at that commit. Tool calls stay within each arm; only task
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

Sign in to Codex once through its own installed CLI before a live run.
The adapters read the existing login through one converter, taking only
access, id and account fields into private files. No issued refresh token
is copied or used. Expiry must cover the hard deadline and settlement.
The source digest must be unchanged afterwards; private auth is removed.
No user's settings, instructions, skills, plugins or memories enter an
attempt. Never put credentials in task seeds or exported artifacts.

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
command deadlines sum to 180 seconds; focused and fuzzy nextest still
use the frozen seed's 15 and 60 second budgets. Both arms start from the
same unbuilt copy, with no shared attempt cache.

New product arms run `smith check --json` during setup, then
`smith exec --json -C <workspace> -m <endpoint>/<model> --effort <effort> -`.
The attempt's XDG user layer pins settings and borrows the caller's Codex
login file read-only. Teardown comes from check's effective `service.teardown`.
A nonzero check refuses setup; explicit startup-refusal probes retain that
check's exit, diagnostic and settled tree as their observation. Historical
event arms keep their original native configuration and credential codec.

The opt-in live stories use `SMITH_TEST_LIVE_CODEX_LOGIN_FILE` to name an
absolute Codex login file. The suite leaves it unchanged and runs through
`smith exec --json`; this plan does not run the live suite. Anthropic stories
and the optional issuer-refresh story retain their current invocation until
product sign-in lands (decisions.md, 13.108).
