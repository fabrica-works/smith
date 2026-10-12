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
suite NAME [--seed N] [--commit COMMIT | --arm DIRECTORY]
```

A probe name is `probes/<id>[/<variant>]`. The default source is `HEAD`,
the agent is smith, and a probe runs once, rerunning once on failure.
Codex uses its standard pin and decides on the outcome half of the task.
Every attempt prints its run and attempt ids, end, verdict, wall seconds
and spend. Unavailable measurements print `-`. A summary follows.
Exit codes are 0 for passed or calibrated, 1 for failed or not calibrated,
and 2 for incomplete evidence, setup refusal or a harness error.

Calibration defaults to ten passes within twelve logical attempts. A
cache probe's triplet is one logical attempt; oversized writes require
three passes. A control runs first. The changed arm stops when enough
passes arrive or the remaining attempts cannot attain them. Successful
calibration prints a copyable task calibration block. `fix` prints both
arms' runs and the finding for a commit body. Suites use their pinned
agents, budgets and seeded ordering; the seed is printed.

Other commands:

```sh
cargo run -p smith-bench -- check [TASKS_DIRECTORY]
cargo run -p smith-bench -- attempt PREPARED_REQUEST
cargo run -p smith-bench -- guards --suite SUITE [--tasks TASKS_DIRECTORY] SECTION...
cargo run -p smith-bench -- summarise RESULTS_DIRECTORY SUITE TIER single SEED
cargo run -p smith-bench -- baseline benchmarks/summaries/RUN.json NAME
cargo nextest run -p smith-bench
```

`check` validates tasks, suites, model tiers, pins, summaries and
baselines. Tasks freeze their seed digest and complete prompt. Suite
names are `<kind>/<id>[/<variant>]`; behaviour selections resolve through
the task catalogue. Pins freeze each agent's provider, version and exact
configuration bytes. Unknown fields and unresolved model tiers refuse
setup. The small tier is pinned for both providers; working tiers await
their owner's selection.

`guards` lists affordable tasks guarding the named design sections,
cheapest first, and explains omissions. Estimates reserve repetitions,
agents, arms and a failure rerun. Matching committed medians replace
estimates; missing costs remain unavailable.

`summarise` reads result files recursively or one explicit result. It
writes `benchmarks/summaries/<run>.json`, recording the suite, tier,
design and seed. An optional baseline path is its final argument.
`baseline` freezes one binary, configuration and tier under
`benchmarks/baselines/<name>.json`. Both refuse overwrites. Commit these
summaries and baselines; raw attempt data stays outside repositories.
Smoke and fix evidence belongs in commit bodies.

Results are strict version-one JSON documents with observed,
lower-bound or unavailable measurements. Missing observations never
become zero or a root-only fallback. Token records retain their scope
and convention, deduplicate responses and name missing child evidence.
Only interleaved runs compare arms, with five complete observations each
and seeded bootstrap resampling. Drift notices request a comparison;
they do not declare a regression.

Runtime writes pass the shared guard and live under
`$XDG_STATE_HOME/smith-bench/`: `requests/`, raw `runs/`, private `homes/`,
build `clone/` and immutable `arms/`, and attempt `scratch/`. Each attempt
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
for a frozen binary. The legacy face remains until the first baseline on
events; unavailable legacy metrics stay unavailable. Codex's JSON-lines
observer is pinned to CLI 0.160.0; named-thread rollout collection retains
root and child scopes. Known malformed records fail loudly; unknown
record kinds are skipped. Typed recording controls exercise the same
observers and supervision without running a real agent.
