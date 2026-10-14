# Parity plan: smith on par with Codex

2026-10-14. Starts from smith `main` at `72311b8e` and skein `main` at
`63fff2d`.

The plan takes smith to parity with Codex as `benchmarks.md` section 16
defines it, and gives the harness what that verdict needs: graded tasks
an agent can pass, each agent as it ships, one price basis, summaries that
keep each attempt, calibration in task files, and the `parity` command. It
fixes what the evidence says stands in the way: compaction answered with a
call, builds that block every call (jobs), a model with no role, and two
defects. Its last step is a `parity` run on its final `main`.

## Inputs

- **The design, which is the contract.** smith `docs/design/`:
  `benchmarks.md` (1 to 16); `domain/tools.md`, `session.md`, `run.md`,
  `host.md`; `protocol/llm.md`, `events.md`, `limits.md`, `agent.md`,
  `transcript.md`; `shell.md`; `testing.md`. skein `docs/design/fake-llm.md`,
  section 2.1. Each session's "Read first" names its sections.
- **`decisions.md`:** sections 1 to 9 are the designer's; section 10 is
  this plan's own; the run's rulings follow, from 101.
- **`findings/`:** the designer's eight themes (`README.md` indexes them;
  entries cited as T1, K6, W8, M10, P3, F1, L2, ...), and the planner's
  four records: `planning-audit.md` (the code audit), `planning-consistency.md`,
  `planning-literal.md` and `planning-commands.md` (each command run at the
  base, with its exit status).

## The sessions

A dependency is named by capability, with its increment in brackets. A
session starts when what it needs at its start has landed; an increment
that needs more waits, and the session does its other increments
meanwhile (implementer.md, section 8.7).

### smith (`smith/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `01-tasks.md` | 01: the graded tasks: fixtures and `shell-cwd` at version 2, what decides a graded task, comparison budgets refused, `/tmp` listed | — | — |
| `02-pins.md` | 02: each agent as it ships: smith's pins as product layers, one face, Codex as it ships | — | — |
| `03-results.md` | 03: priced spend from smith's presets with the formats' version 3, a summary row per attempt, the time record, Codex's children, large effects | — | — |
| `04-probes.md` | 04: calibration in task files, `history` version 2, the `parity` suite and command, probes gated by calibration, `first-request` calibrated | — | 04.4: smith 03's attempt rows [03.2]; 04.6: smith 02's pins and one face [02.1, 02.2], smith 03's summary version 3 [03.1] |
| `05-product.md` | 05: `gpt-6.1-sol` and the working tier, one effort bound, a failed start ends `smith exec` | — | — |
| `06-text.md` | 06: the empty path and the descriptions' facts, then the task role | the baseline `parity` run's summary on `main` [12.1]; smith 08's vocabulary [08.1] | — |
| `07-compaction.md` | 07: a compaction asked with `None`, its summary framed, the instruction's words | skein 01's prefix [skein 01.1]; smith 08's vocabulary [08.1] | 07.2: smith 11's transcript version 4 [11.1]; 07.3: smith 04's calibration blocks and `history` version 2 [04.1, 04.2], smith 03's summary version 3 [03.1] |
| `08-limits.md` | 08: the event vocabulary's version 2, `parallel_tools`, `jobs` and `job_wait` | — | — |
| `09-machine.md` | 09: the kit's flights renamed, the machine holding jobs, the listing's order | — | 09.3: the baseline [12.1] |
| `10-jobs.md` | 10: jobs in the tools domain, the session and the run | smith 08's `jobs` and `job_wait` [08.3]; smith 09's held jobs [09.2]; smith 11's transcript version 4 [11.1] | — |
| `11-offered.md` | 11: the transcript's version 4, the `job` tool offered, the `background` probe and the repository check | — | 11.2: smith 10's lease [10.4], the baseline [12.1], smith 06's role [06.2]; 11.3: 11.2 |
| `12-milestones.md` | 12: the baseline `parity` run, the `compaction` experiment, the calibrations left, the closing `parity` run | 12.1's Needs | 12.2: smith 07's words [07.3]; 12.3 and 12.4: every other step |

### skein (`skein/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `01-fake-prefix.md` | 01: a changed tool choice keeps an automatic query's prefix | — | — |

**How many at once.** At the start, nine sessions can run: smith 01,
02, 03, 04, 05, 08, 09 and 11, and skein 01. The coordinator runs at most
eight threads (development.md, section 7.2). smith 07 opens once skein
01.1 and 08.1 land, 10 once 08.3, 09.2 and 11.1 land, 12 once 12.1's
Needs land, and 06 after 12.1 and 08.1. When slots are short, start
first, in this order:

1. smith 03 (03.1 and 03.2 open 04.4, 04.6 and 07.3);
2. smith 04 (04.1 and 04.2 open 07.3; 04.4 opens the baseline);
3. smith 08 (08.1 opens 07 and 06; 08.3 opens 10);
4. skein 01 (opens 07);
5. smith 11 (11.1 opens 07.2 and 10; then it waits for 11.2's Needs);
6. smith 09 (09.2 opens 10);
7. smith 01, 02 and 05 (12.1 needs 01.1 to 01.3, 02.1 to 02.3 and 05.1);
8. then 07, 10, 12 and 06 as they open.

**The longest chain** is 11 increments, by two routes: skein 01.1, smith
07.1, 07.2, 07.3, 12.1, 06.1, 06.2, 11.2, 11.3, 12.3, 12.4; and smith
08.1, 08.2, 08.3, 10.1, 10.2, 10.3, 10.4, 11.2, 11.3, 12.3, 12.4. At 0.33 h
a step that is 3.7 h, before the live runs on it, which take the gate
lane: 12.1 and 12.4 about 4 h each, 06.2 about 1.5 h, 11.3 about 2.5 h,
12.3 about 1 h (estimates; the gate lane runs one task at a time, so
landings wait behind them).

## Rules of this plan

What development.md, implementer.md, smith's and skein's `AGENTS.md` and
smith's `workflow.md` do not already say. Each applies to every session,
until the run ends.

1. **Splits** (the reliability retrospective's lesson 1). A Size line is
   a guide. Past 1.5 times its Size line or 700 production lines,
   whichever is smaller, the implementer splits the increment at a point
   that lands alone, and lands that part. The count is the lines added by
   `git diff --numstat origin/main...HEAD`, without `src/generated/`,
   golden files, and test code (`tests/`, `benchmarks/tests/`, any
   `tests.rs`, any `src/tests/`). The rest becomes the session's next free
   item number. Its report says `Split: <step>, <what>`, and the
   coordinator runs `plan set <step> note="split from <old> by the
   implementer"`. It asks only when no such point exists. This replaces
   "at most" in every Size line. A Size of "none" (a step with no code)
   and 09.1's rename are not counted.
2. **At most 8 increments a session.** A step the supervisor adds to a
   session that holds 8 becomes a session of its own.
3. **Users outside Files** (lesson 4). A name the Shape removes is
   replaced by the successor the Shape names, and each call or
   construction is adapted to a signature, a variant or a struct whose
   fields the Shape changes (a new field set to the value the story
   already implies: `after: None`, `Yield::Never`, a limit's shipped
   value), in any file, with no line limit. No test is renamed and no
   expected value changes; the commit body lists each file under "Users
   outside Files:". This extends implementer.md 8.2 for these two cases.
4. **Reading** (lesson 5). Each session file's "Read first" names, by
   number, the sections of skein's `programming-model.md` and
   `testing-strategy.md` its increments follow: read only those.
5. **Diagnosis** (lesson 6). A step whose Scope is to find a cause
   carries `Budget: <N> runs, <M> minutes`; past it, the implementer stops
   and reports. Outside such a step, at most 3 runs with diagnostics in
   place for one failing test, then it asks. A diagnostic is a line added
   only to find a cause; none lands: before `land`, `git diff
   origin/main...HEAD | grep -nE '^\+.*\bdbg!'` prints nothing, and the
   implementer removes any other such line it added.
6. **Dependents' tests** (lesson 7). Once before `land`, after the last
   edit: `heavy cargo nextest run --workspace -E 'rdeps(<crate>) or
   rdeps(<crate>)'`, one `rdeps` for each crate the increment changed. A
   failure only on a timeout is left to the gate; any other is fixed
   first. This is the one exception to "whole suites only in land's
   gate"; a Proof or a Done when that names `-p` packages runs those
   packages' tests, as implementer.md 8.3 allows.
7. **Words and lint exceptions never land** (lesson 8). Once before
   `land`, on the rebased branch:

   ```
   git diff origin/main -- . ':!docs' | grep -inE '^\+.*\b(actually|genuine|immutable|canonical|verified|qualified|robust|properly|python3)\b|^\+.*#\[(allow|expect)\('
   ```

   prints nothing. Each word it prints is reworded to say what the code
   does, with no synonym in its place; each lint exception is removed and
   its lint fixed. Words already on `main` stay. The one exception: the
   `#[expect(clippy::large_enum_variant, reason = "sealed payloads are
   priced by bounded queues and state")]` that 11.1 puts on
   `smith-host-domain`'s `process::Event` (decisions.md, ruling 102),
   the attribute and reason that `process::Request` and `boundary::Input`
   carry on `main`.
8. **The smoke on every smith landing that changes code** (smith
   `workflow.md`, section 5): `smoke-tools`, live, on the branch rebased
   on `origin/main` just before `land`:

   ```
   heavy cargo run -p smith-bench -- build HEAD
   heavy --gate cargo run -p smith-bench -- probe probes/smoke-tools --arm <the directory build printed>
   ```

   It runs once; a failure, for any reason, runs it once more. A rebase
   inside `land` does not run it again. Its summary line goes in the
   commit body. A landing that changes only Markdown, or only files under
   `benchmarks/tasks/`, `benchmarks/suites/`, `benchmarks/summaries/` or
   `benchmarks/baselines/`, skips it, and so does a skein landing. A
   second failure stopped by the provider (an end `Refused(setup)` on the
   credentials, or `Failed` with the provider's capacity or a server error
   as its reason): the step lands and reports "smoke not run: <the end and
   its reason>", and the coordinator escalates it as `credentials` or
   `infrastructure`. A second failure for any other reason stops the line
   for smith, except a second failure refused at setup, before any model
   request, on a branch that changes `benchmarks/src/agent/smith/` or
   `benchmarks/agents/smith/`, while `main`'s last landed smoke passed:
   that stops its step on a question, and the line goes on (decisions.md,
   ruling 107).
9. **Live runs** (lesson 10). Every live command goes through `heavy
   --gate`. A `suite` runs in one call, unsplit, whatever its attempts. A
   `probe` runs at most 12 attempts a call; a `calibrate`, at most 12 on
   its arm, plus at most 2 on its control (it stops the control at its
   first failure, and the arm at its tenth pass or once ten are out of
   reach). Each commit a command names (`HEAD`, `--control`, `--parent`)
   is built first: `heavy cargo run -p smith-bench -- build <commit>`. A
   command stopped by the provider, that is, one that ends before its
   last attempt because an attempt met the provider's capacity or a
   server error, runs once more, from the start, as a new run, after 60
   seconds; stopped again, the step lands with what it could not run named
   in its commit body, and goes on. An attempt the provider failed within
   a command that ran to its end stays in its summary, counted as
   benchmarks.md 8.3 and 16.3 say. A control that passes stops the step on
   a question, and no block is written. Proofs are live only where their
   Proof line says so.
10. **The live budget** (decisions.md, 10.2): 340 attempts and $50 of
    smith's notional spend for the run, Codex's attempts counted but not
    its spend. Each landing that ran anything live ends its trailers with
    `Live: <attempts> attempts, $<notional> notional`, counting every
    attempt its step ran, smokes and failed runs included. Before each
    live command other than the smoke, the implementer prints the run's
    landed total:

    ```
    git -C ~/src/rust/smith fetch -q origin && git -C ~/src/rust/smith log origin/main --grep='^Plan: parity$' --format='%(trailers:key=Live,valueonly)' | sed -nE 's/^([0-9]+) attempts, \$([0-9.]+) notional$/\1 \2/p' | awk '{ a += $1; d += $2 } END { printf "%d attempts, $%.2f\n", a, d }'
    ```

    and adds its own step's attempts so far. If that sum plus the
    command's attempts passes 340 attempts or $50, it stops on a question,
    and the supervisor reports to the user (a push notification, and
    decisions.md's "For the user"); offline work goes on. A step whose
    attempts or notional pass 1.5 times its row in "Live services" stops
    on a question the same way; a code landing's smoke has a row of 2
    attempts.
11. **The baseline comes first.** These increments change what the model
    is sent or what a call answers, and land only after 12.1's summary is
    on `main`: 06.1, 06.2, 09.3 and 11.2.
12. **Formats move once.** At the plan's start: smith's event vocabulary
    1, transcript 3, charter 2, channel 2; `smith-bench`'s result 2,
    summary 2, baseline 1. The event vocabulary moves to 2 in 08.1, the
    transcript to 4 in 11.1, the result and the summary to 3 in 03.1.
    Later increments change those formats under their new version, with
    no further bump; "Done when" item 6 checks the versions. A new model
    adds effective limits' names by the existing scheme (limits.md 9:
    per model, by its configured name), which is no new version.
13. **Never touch the user's logins.** smith and `smith-bench` borrow
    `~/.codex` read only, the access token only. Nothing writes to
    `~/.codex`, `~/.claude`, or smith's own `$XDG_CONFIG_HOME/smith` and
    `$XDG_STATE_HOME/smith` (not `smith-bench`'s); no agent signs in; no
    refresh token is copied (benchmarks.md, section 10). The harness's
    scope gate is its check (benchmarks.md 9's audits).
14. **The harness follows the binary.** An increment that changes how
    smith is invoked, configured or observed changes `smith-bench`'s smith
    adapter in the same increment, so the smoke keeps running.
15. **Hotspots.** Sessions share `crates/smith-domain-session/src/session.rs`
    (07.1, 07.2, 09.2, 10.3), `crates/smith-agent-service/src/profile.rs` and
    `profile/` (06.2, 07.2, 08.2, 08.3), `crates/smith-local-shell/src/settings/`
    (05.1, 08.2), `crates/smith-protocol-llm/src/` (06.1, 07.1, 10.1, 11.2),
    `crates/smith-domain-tools/src/` (08.3, 09.1, 09.2, 10.1, 10.2),
    `benchmarks/src/run.rs` (01, 02, 03), `benchmarks/src/summary.rs` (03,
    04), `benchmarks/src/check.rs` (01, 02, 03, 04) and `Cargo.lock`. Keep
    increments small; conflicts are development.md 8.5's.
16. **One behaviour per test** (decisions.md, ruling 111). A test whose
    cases expect different outcomes is several tests: each its own test,
    named for what it shows in about eight words, with its expected value
    as a literal. A loop over seeds or inputs that share one expected
    outcome stays one test. Landed tests in files an increment does not
    otherwise change stay as they are, and so do test names the plan
    gives.

## Live services

**OpenAI, through the user's Codex login,** the only live service. smith
and `smith-bench` read `~/.codex/auth.json` read only, and each Codex
attempt gets a private, access-only copy in its own `CODEX_HOME`
(benchmarks.md, section 10). Codex runs from the private package
`~/.local/state/smith-bench/tools/codex-0.160.0/`, pinned by digest after
02.3. No agent signs in. A lapsed login stops every live run: the
coordinator escalates it as `credentials`, and the user runs `codex` once.

| Step | What | Attempts | Notional |
|---:|---|---:|---:|
| every smith code landing | `smoke-tools`, once, once more on failure (rule 8); 39 landings, most passing at once | 50 | $0.25 |
| 02.1, 02.3 | `first-request`, and Codex on `smoke-tools`, each at most 2 | 4 | $0.02 |
| 04.6 | `first-request` calibrated, no control (rulings 112, 114: 11 spent, 12 fresh) | 23 | $0.05 |
| 06.2 | `role`: 06.1's parent against 06.2, three fixtures, 5 each | 30 | $3 |
| 07.3 | `history` and `history/small-window` calibrated, with their control | 28 | $1.50 |
| 11.3 | `tool-output/background` calibrated (14); `repository-check`, 5 per arm (10) | 24 | $7 |
| 12.1 | the baseline `parity` run | 58 | $12 |
| 12.2 | the `compaction` experiment, 10 per arm | 20 | $1 |
| 12.3 | `tool-output` version 3's four variants and `process-tree`, while room is left | at most 50 | $0.30 |
| 12.4 | the closing `parity` run | 58 | $12 |
| reserve | provider-stopped reruns (rule 9) | 6 | $0.30 |
| **the run** | | **at most 340** | **about $37** |

12.3 runs only the calibrations that leave 64 attempts (12.4's 58 and the
reserve) under 340; those it cannot run go to `later.md`.

## Repin

None. The plan changes skein and smith, and repins neither jig nor
temper: the user reserved them for their own plans (decisions.md, 8.3).
smith 07.1 repins skein in a commit of its own, for skein 01's fake.

## Pulse

```
words: actually|genuine|immutable|canonical|verified|qualified|robust|properly|python3
types: ^(Legacy|New|Old|Owner)[A-Z]|V[0-9]+$
file-lines: 800
budgets: 15 60
```

## Done when

Each command runs once, on `main` (`~/src/rust/smith`, after `git -C
~/src/rust/smith pull --ff-only`), after the last step lands.

1. **Every step landed or dropped:** `plan status` lists no step
   `waiting`, `ready`, `running`, `check` or `blocked`.
2. **smith is on par with Codex at the plan's final `main`:**

   ```
   cd ~/src/rust/smith && run=$(ls benchmarks/summaries/parity-*.json | sort -t- -k3 | tail -n1) && sha=$(jq -r .parity.smith_commit "$run") && test -z "$(git log "$sha"..origin/main --grep='^Plan: parity$' --format=%h -- crates src tests benchmarks/src)" && cargo run -q -p smith-bench -- parity "$run" --commit "$sha"
   ```

   exits 0, offline. It reads the newest committed `parity` summary,
   checks that no code commit of the plan landed after its smith arm, and
   recomputes the verdict.
3. **The smoke passes on `main`:** `heavy cargo run -p smith-bench --
   build HEAD`, then `heavy --gate cargo run -p smith-bench -- suite smoke
   --arm <the directory build printed>` exits 0, once.
4. **The plan's calibrations are in their task files:**

   ```
   cd ~/src/rust/smith && for t in history first-request tool-output; do git show origin/main:benchmarks/tasks/probes/$t/task.toml | awk -v t=$t '/^\[\[variant\]\]/ { v = ""; named = 1; next } named && /^name *=/ { gsub(/[" ]/, "", $3); v = "/" $3; named = 0 } /^\[calibration\]/ { print t } /^\[variant\.calibration\]/ { print t v }'; done | grep -cxE 'history|history/small-window|first-request|tool-output/background'
   ```

   prints 4: `history` and `history/small-window` (version 2),
   `first-request`, and `tool-output/background`.
5. **Both gates pass within their budgets:** in each repository, `git log
   origin/main --format='%(trailers:key=Gate-Result,valueonly)' | grep -v
   '^none' | grep -m1 .` (the last gated landing, Markdown ones skipped)
   shows the focused suite within 15 s and the fuzzy suite within 60 s.
6. **Each format moved once:** `cd ~/src/rust/smith && rg -no
   '(pub const VERSION|FORMAT_VERSION|RESULT_VERSION|SUMMARY_VERSION)\b[^=]*= *[0-9]+'
   crates/smith-events/src/lib.rs crates/smith-transcript/src/lib.rs
   benchmarks/src/result.rs benchmarks/src/summary.rs` prints the event
   vocabulary's 2, the transcript's 4, the result's 3 and the summary's 3.
7. **Each session's own "Done when" holds:** the commands in each session
   file's "Done when", run once each.

Not checked by a command, and so not aims of this plan: the `compaction`
experiment's result, which 12.2 reports to the user; the calibrations
12.3 may leave to `later.md`.
