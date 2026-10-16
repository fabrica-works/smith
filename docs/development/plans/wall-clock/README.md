# Wall-clock plan: smith on par with Codex, RT included

2026-10-16. Starts from smith `main` at `4ddb9b67` and skein `main` at
`15ddb75`.

The parity plan ended short of parity on RT, `shell-cwd`'s time. smith's
requests cost what Codex's cost; it makes more of them. This plan gives
the harness what the next verdict needs: spend priced per response, ten
`shell-cwd` attempts, responses and smith's own peak memory in each row,
and a `parity-change` run that reports a change's effect on every
criterion and stops a move the wrong way. It puts in place the line
below which a compaction always fits, and the guard that holds each
prompt sent within it. After a baseline `parity` run it lands four
changes to what the model is sent, one per cause in
`findings/causes.md`, each proven by its `parity-change` run. It ends
with at most two closing `parity` runs.

## Inputs

- **The design, which is the contract.** smith `docs/design/`:
  `domain/session.md` (4, 5, 8.1, 8.2, 10), `domain/tools.md` (3, 5, 7),
  `protocol/limits.md` (2.1, 3, 3.2, 3.9, 4, 5, 9, 11), `protocol/llm.md`
  (3, 5, 6, its tests), `protocol/charter.md` (6, 8),
  `protocol/transcript.md` (2, 6), `protocol/events.md` (3.2, 3.6, 6),
  `shell.md` (5.2, 6.3, 6.6), `testing.md` (6), `benchmarks.md` (1, 3.1,
  4.2, 5.4, 8.1, 8.2, 9, 11.1, 11.5, 14, 16). Each session's "Read first"
  names its sections.
- **`decisions.md`:** section 1 is the user's, sections 2 to 11 the
  designer's, section 12 this plan's own; the run's rulings follow, from
  101.
- **`findings/`:** the designer's `causes.md` (causes 1 to 4, cited as
  cause N) and `rt-power.md`; the planner's `planning-audit.md` (the
  code audit, entries A1 to A27) and `planning-commands.md` (each
  command run at the base, with its output).

## The sessions

A dependency is named by capability, with its increment in brackets. A
session starts when what it needs at its start has landed; an increment
that needs more waits, and the session does its other increments
meanwhile (implementer.md, section 8.7).

### smith (`smith/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `01-measures.md` | 01: spend priced per response, result and summary version 4; responses and the agent's own peak memory in each row; ten `shell-cwd` attempts and O over 34 | — | — |
| `02-change.md` | 02: the `parity-change` suite with two smith arms, its report, its projection on the baseline and its stop | — | 02.2: smith 01's rows at version 4 and O over each arm's attempts [01.2, 01.3]; 02.3: 02.2's change section [02.2] |
| `03-line.md` | 03: the line and the spare, the entrance's spare check, the event vocabulary's version 3; the history's bytes from the line, `memory` raised; the `history` probe's condition at the line | — | — |
| `04-guard.md` | 04: the guard, measuring an ordinary request's new part as rendered and refusing it unsent past the line; the session world counting tokens from bytes | smith 03's line in each conversation [03.1] | — |
| `05-memory.md` | 05: memory shown returning in the agent's simulated world: after a compaction, and across repeated uses | — | — |
| `06-baseline.md` | 06: the baseline `parity` run, then the `parity-change` pipeline at one attempt per task against it | 06.1's Needs | — |
| `07-window-check.md` | 07: the window's check estimating the next prompt, no output reserved below the threshold (cause 2) | the baseline's summary and the change pipeline's small run on `main` [06.1]; smith 04's world counting tokens from bytes [04.2]; smith 03's `history` condition at the line [03.3] | — |
| `08-batched-change.md` | 08: a failed write stopping the writes and commands after it, in transcript version 5; the descriptions' and the role's batched change and its check (causes 1 and 3) | smith 07's check [07.1] landed or dropped | — |
| `09-polls.md` | 09: a job's wait giving way to its batch; the descriptions' and the role's polls beside other work (cause 4) | smith 08's batched change [08.1] landed or dropped | — |
| `10-ranged-reads.md` | 10: the role's ranged reads (fresh tokens) | smith 09's polls [09.1] landed or dropped | — |
| `11-closing.md` | 11: the first closing `parity` run; the second, only after a step that answers the first's miss | smith 10's ranged reads [10.1] landed or dropped, and every step in the ledger but 11.1 and 11.2 landed or dropped | 11.2: 11.1's summary with a verdict of 1, and the step the supervisor adds to answer it, landed or dropped |

"Landed or dropped": a step 07 to 10 that its stop sent to the designer
either lands on the designer's answer, or is dropped by it (rule 12);
the next starts after either.

**06.1's Needs** (the baseline): smith 01's pricing, rows and ten
attempts [01.1, 01.2, 01.3]; smith 02's stop [02.3]; smith 03's line and
history bytes [03.1, 03.2]; smith 04's guard [04.1]. Nothing that
changes what the model is sent lands between 06.1's `build HEAD` and
07.1's landing (rule 11).

**How many at once.** At the start, four sessions can run: smith 01,
02, 03 and 05. smith 04 opens once 03.1 lands; 06.1 once its Needs land;
07 once 06.1, 04.2 and 03.3 land; then 08, 09, 10 and 11, one after
another. When slots are short, start first, in this order:

1. smith 01 (01.1 to 01.3 open the baseline, and 02.2);
2. smith 03 (03.1 opens 04, and 04.1 is on the baseline's chain);
3. smith 02 (02.3 opens the baseline);
4. smith 05;
5. then 04, 06, 07, 08, 09, 10 and 11 as they open.

**The longest chain:** smith 01.1, 01.2, 01.3, 02.2, 02.3, 06.1, 07.1,
08.1, 09.1, 10.1, 11.1, and, if 11.1 misses, the step that answers it
and 11.2. The live runs on it take the gate lane, which runs one task at
a time, so landings queue behind them. From the parity run's times
(`shell-cwd` 393 to 932 s an attempt, about 650 s on average; a fixture
attempt about 165 s): 06.1 about 6.5 h (68 attempts, then 8), each of
07.1 to 10.1 about 3 h (40), 11.1 about 6 h (68). About 25 h of live
runs, 34 h if 11.2 and its answering step run, besides the code.

## Rules of this plan

What development.md, implementer.md, smith's `AGENTS.md` and smith's
`workflow.md` do not already say. Each applies to every session, until
the run ends.

1. **Splits.** A Size line is a guide. Past 1.5 times its Size line or
   600 production lines, whichever is smaller, the implementer splits
   the increment at a point that lands alone, and lands that part; the
   count is `code-organization.md`'s, section 8. The rest becomes the
   session's next free item number. Its report says `Split: <step>,
   <what>`, and the coordinator runs `plan set <step> note="split from
   <old> by the implementer"`. Lines under rule 3 count like any other.
   Two exceptions: 07.1, 08.1, 09.1 and
   10.1 never split, since each part would change what the model is
   sent without its own `parity-change` run: past 1.5 times its Size,
   the step asks, with the count by file; and a Size of "none" is not
   counted.
2. **At most 6 increments a session.** A step the supervisor adds to a
   session that holds 6 becomes a session of its own.
3. **Users outside Files.** A name the Shape removes is replaced by the
   successor the Shape names, and each call or construction is adapted
   to a signature, a variant or a struct whose fields the Shape changes
   (a new field set to the value the story already implies: `guard:
   None`, a limit's shipped value: the value that keeps the test's
   present behaviour), in any file. This replaces implementer.md 8.2's
   line limits for these edits only. No test is renamed but where a
   Shape renames it; an expected value changes only where the Shape
   changes what it counts, and the commit body lists each such test.
   The commit body lists each file under "Users outside Files:".
4. **Reading.** Each session file's "Read first" names, by number, the
   sections of skein's `programming-model.md` and `testing-strategy.md`
   its increments follow: read only those.
5. **Diagnosis.** A step whose Scope is to find a cause carries `Budget:
   <N> runs, <M> minutes`; past it, the implementer stops and reports.
   Outside such a step, at most 6 runs with diagnostics in place for one
   failing test, then it asks; a run is one `heavy` call that runs the
   failing test, and the report gives the count. A diagnostic is a
   `dbg!` line, and none other is written; none lands: before `land`,
   `git diff origin/main...HEAD | grep -nE '^\+.*\bdbg!'` prints
   nothing.
6. **Dependents' tests.** Once before `land`, after the last edit: `heavy
   cargo nextest run --workspace -E 'rdeps(<crate>) or rdeps(<crate>)'`,
   one `rdeps` for each crate the increment changed. A failure only on a
   timeout is left to the gate; any other is fixed first. A landing that
   changes no crate skips it. Its times are the ones the test ceilings
   are read from, in place of implementer.md 8.2's "last focused run".
7. **Words and lint exceptions never land.** Once before `land`, on the
   rebased branch:

   ```
   git diff origin/main -- . ':!docs' ':!benchmarks/summaries' ':!benchmarks/tests/recorded' | grep -inE '^\+.*\b(actually|genuine|immutable|canonical|verified|qualified|robust|properly|python3)\b|^\+.*#\[(allow|expect)\('
   ```

   prints nothing, but for lines whose word was on that line on `main`,
   which stay and are named in the report. Each other word it prints is
   reworded to say what the code does, with no synonym in its place;
   each lint exception is removed and its lint fixed.
8. **The smoke** (smith `workflow.md`, section 5) on every smith landing
   that changes code:

   ```
   heavy cargo run -p smith-bench -- build HEAD
   heavy --gate cargo run -p smith-bench -- probe probes/smoke-tools --arm <the directory build printed>
   ```

   It runs once, and passes when the probe exits 0; a failure runs it
   once more. A landing that changes
   only Markdown, or only files under `benchmarks/summaries/` or
   `benchmarks/baselines/`, skips it. A second failure stopped by the
   provider (an end `Refused(setup)` on the credentials, or `Failed`
   with the provider's capacity or a server error as its reason): the
   step lands and reports "smoke not run: <the end and its reason>", and
   the coordinator escalates it as `credentials` or `infrastructure`. A
   second failure for any other reason stops the line for smith, except
   a second failure refused at setup, before any model request: the
   coordinator then runs the smoke once on `origin/main` and puts both
   summary lines in `blocked.md`. If `main`'s passes, the failure is the
   step's own: the step fixes its cause and runs the smoke once more, a
   third failure being a question, and the line goes on. If `main`'s
   fails too, the coordinator escalates it as `infrastructure`, and no
   smith step lands until `main`'s passes.
9. **Live runs.** Every live command goes through `heavy --gate`. A
   `suite` runs in one call, unsplit, whatever its attempts. Each commit
   a command names (`HEAD`, `--parent`) is built first: `heavy cargo run
   -p smith-bench -- build <commit>`. A command stopped by the provider,
   that is, one that ends before its last attempt because an attempt met
   the provider's capacity or a server error, or a request the provider
   never answered (a failure whose `evidence` is `maybe_sent`,
   benchmarks.md 8.2, such as a timeout waiting for the response head;
   ruling 113), runs once more, from the start, as a new run, after
   60 seconds; stopped again, the step stops on a question, which the
   supervisor sends to the designer. An attempt the provider failed
   within a command that ran to its end stays in its summary, counted as
   benchmarks.md 8.3 and 16.3 say. No attempt is rerun, and no run of the same commits is
   repeated to change a verdict or a stop (benchmarks.md 16.5, 16.6). A
   run with `--attempts` (01.3, 06.1) checks a pipeline, and is never a
   verdict or a stop.
10. **The live record** (decisions.md, 1.8: no budget cap). Each landing
    that ran anything live ends its trailers with `Live: <attempts>
    attempts, $<notional> notional`, counting every attempt its step
    ran, smokes and failed runs included, Codex's attempts counted and
    its spend not; `<notional>` is the sum of smith's spend as each run's
    printed summary line gives it. A step whose attempts pass 1.5 times
    its own in "Live services" says so in its report; nothing stops on
    it.
11. **The baseline first, then one change at a time.** 07.1, 08.1, 09.1
    and 10.1 change what the model is sent (benchmarks.md 16.6), and
    land in that order, each after 06.1's summary is on `main` and the
    one before it has landed or been dropped. Each runs its
    `parity-change` on its commit rebased on `origin/main`, against
    `origin/main`, once (rule 9). The prompt's files are
    `crates/smith-local-shell/src/role.rs`,
    `crates/smith-protocol-llm/src/descriptions.rs`,
    `crates/smith-protocol-llm/src/prompt.rs`,
    `crates/smith-domain-session/src/window.rs` and
    `crates/smith-domain-session/src/compaction.rs`. A rebase after the
    run onto commits that change none of them keeps the run: the commit
    body names the run's two commits. A rebase onto one that does is a
    new run. No other step changes a prompt smith sends, a tool's
    description or schema, what a tool answers when a call asks for
    nothing, or the window's check, but a step the supervisor adds to
    answer 11.1's miss, under rules 12 and 13: a step that finds it must
    is a question. "Done when" item 8 checks it.
12. **The stop** (benchmarks.md 16.6). A full `suite parity-change` run
    (no `--attempts`) exits 0 when nothing stops the step, 1 when a
    criterion moved the wrong way (a projection of FT, FC, RT or RC past
    the baseline's upper bound, or O's passes below the parent's bound),
    and 2 when the report is incomplete. On 1 or 2 the step stops on a
    question, its summary and the report's lines in it; the supervisor
    sends it to the designer (development.md 10.4). The step lands only
    on the answer: kept, with the cause the answer names in its commit
    body; reworked, with a new run on its new commit; or dropped, and the
    coordinator runs `plan set <step> state=dropped note="dropped by
    <the answer's heading>"`.
13. **Summaries named in trailers.** 06.1's landing carries
    `Baseline-Summary: benchmarks/summaries/<file>`, and each landing
    that ran a full `parity-change` `Change-Summary:
    benchmarks/summaries/<file>`, its summary amended into the step's own
    commit. A `parity-change` run takes `--baseline-run` the file the
    first command prints, and `--landed` each file the second prints:

    ```
    git -C ~/src/rust/smith fetch -q origin && git -C ~/src/rust/smith log origin/main --grep='^Plan: wall-clock$' --format='%(trailers:key=Baseline-Summary,valueonly)' | grep .
    git -C ~/src/rust/smith log origin/main --grep='^Plan: wall-clock$' --format='%(trailers:key=Change-Summary,valueonly)' | grep .
    ```

    The counts beside a report come from its run's attempts, once:

    ```
    jq -r '.attempts[] | select(.task == "repository/shell-cwd") | "\(.attempt) \(.arm)"' <summary> | while read a arm; do printf '%s %s ' "$arm" "$a"; jq -c '[.counts.compactions, .counts.responses, .counts.tool_calls]' ~/.local/state/smith-bench/runs/<run>/$a/result.json; done
    ```

    prints each `shell-cwd` attempt's arm, compactions, responses and
    calls by tool; nothing else is wanted.
14. **Formats move once.** At the plan's start: smith's event vocabulary
    2, transcript 4, charter 2, channel 2; `smith-bench`'s result 3,
    summary 3, baseline 1. The result and the summary move to 4 in 01.1,
    the event vocabulary to 3 in 03.1, the transcript to 5 in 08.1.
    Later increments change those formats under their new version, with
    no further bump; the charter, the channel and the baseline keep
    theirs. "Done when" item 5 checks the versions.
15. **Never touch the user's logins.** smith and `smith-bench` borrow
    `~/.codex` read only, the access token only. No code of smith or
    `smith-bench`, and no test, writes to `~/.codex`, `~/.claude`, or
    smith's own `$XDG_CONFIG_HOME/smith` and `$XDG_STATE_HOME/smith` (not
    `smith-bench`'s); no agent signs in. Before `land`, `git diff
    origin/main...HEAD | grep -nE '^\+.*(\.codex|\.claude|XDG_(CONFIG|STATE)_HOME)'`
    prints nothing, or the commit body names each line it prints and why
    it writes nothing there.
16. **The harness follows the binary.** An increment that changes how
    smith is invoked, configured or observed changes `smith-bench`'s
    smith adapter in the same increment, so the smoke keeps running.
17. **Hotspots.** Sessions share `crates/smith-domain-session/src/session.rs`
    (04.1, 07.1, 08.1, 09.1), `window.rs` and `compaction.rs` (04.1,
    07.1), `crates/smith-agent-service/src/profile.rs` and `profile/`
    (03.1, 03.2), `crates/smith-domain-run/src/` (03.1),
    `crates/smith-protocol-llm/src/` (04.1, 08.1, 09.1),
    `crates/smith-domain-tools/src/` (08.1, 09.1),
    `crates/smith-local-shell/src/role.rs` (08.1, 09.1, 10.1),
    `benchmarks/src/summary.rs`, `parity.rs` and `result.rs` (01, 02),
    `benchmarks/src/agent/smith/events/` (01.2, 03.1, 03.3) and
    `Cargo.lock`. Conflicts are implementer.md 8.5's.
18. **One behaviour per test.** A test whose cases expect different
    outcomes is several tests: each its own test, named for what it
    shows in about eight words, with its expected value as a literal. A
    loop over seeds or inputs that share one expected outcome stays one
    test. Rows that differ only in dialect, or in a value a round trip
    returns unchanged, are one table test; rows that expect different
    answers are separate tests. Landed tests in files an increment does
    not otherwise change stay as they are.
19. **No waker of the coordinator's.** Before it idles on queue tasks,
    the coordinator names each in its `plan idle` text by id, label and
    session ("3787 smith-07 (w31), suite parity-change"), and starts no
    `nohup`, background or `until` waker. The supervisor starts the
    waker from its own shell: `pueue wait <ids>`, then `thread/resume` of
    the coordinator's thread, then `plan ring` (the parity
    retrospective's proposal 5), for the whole run; only a steer ends
    it.

## Live services

**OpenAI, through the user's Codex login,** the only live service. smith
and `smith-bench` read `~/.codex/auth.json` read only, and each Codex
attempt gets a private, access-only copy in its own `CODEX_HOME`
(benchmarks.md, section 10). Codex runs from the private package
`smith-bench` pins. No agent signs in. A lapsed login stops every live
run: the coordinator escalates it as `credentials`, and the user runs
`codex` once.

No cap (decisions.md, 1.8): the user watches the credit, and says when it
runs low. The rows are what the plan expects, so that the run is
counted (rule 10).

| Step | What | Attempts | Notional |
|---:|---|---:|---:|
| every smith code landing | `smoke-tools`, once, once more on failure (rule 8); 17 landings, 2 each at most | 34 | $0.20 |
| 01.3 | `parity` at one attempt per task | 8 | $1 |
| 06.1 | the baseline `parity` run (48 fixture attempts, 20 `shell-cwd`), then `parity-change` at one attempt per task | 76 | $19 |
| 07.1, 08.1, 09.1, 10.1 | one `parity-change` run each: 30 fixture attempts, 10 `shell-cwd`, both arms smith; 40 each | 160 | $72 |
| 11.1 | the first closing `parity` run | 68 | $17 |
| 11.2 | the second closing run, only after a step that answers 11.1's miss, with that step's `parity-change` run if it changes what the model is sent | 108 | $35 |
| "Done when" item 3 | `smoke-tools` on `main`, once | 1 | $0.01 |
| reserve | provider-stopped reruns (rule 9) | 40 | $10 |
| **the run** | | **about 495** | **about $154** |

## Repin

None, with the reason: jig and temper are changed by temper's
client-api run, active on carbon-forge since 2026-10-16, and their next
repins of smith take what this plan lands, transcript version 5 and the
event vocabulary's version 3 among it (decisions.md, 11.4; the user,
12.3). `later.md` names it, and the designer's next landing puts it in
jig's and temper's `STATUS.md` (development.md 5.7).

## One test of a suite that needs a built artifact

None: no suite this plan's increments run needs an artifact the gate
builds first.

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

1. **Every step landed or dropped:** in `plan status`'s table, every
   step's state is `landed` or `dropped`.
2. **smith is on par with Codex at the plan's final `main`:**

   ```
   cd ~/src/rust/smith && run=$(ls benchmarks/summaries/parity-*.json | grep -v '/parity-change-' | sort -t- -k3 | tail -n1) && sha=$(jq -r .parity.smith_commit "$run") && test -z "$(git log "$sha"..origin/main --grep='^Plan: wall-clock$' --format=%h -- crates src tests benchmarks/src)" && heavy cargo run -q -p smith-bench -- parity "$run" --commit "$sha"
   ```

   exits 0, offline. It reads the newest committed `parity` summary,
   checks that no code commit of the plan landed after its smith arm,
   and recomputes the verdict. When the plan's last closing run is short
   of parity, it exits 1, and the plan ends with this item failed,
   naming the criteria (benchmarks.md 16.5; `later.md`'s last row).
3. **The smoke passes on `main`:** rule 8's two commands, on `main`,
   exit 0, once.
4. **Each change to what the model is sent has its report:**

   ```
   cd ~/src/rust/smith && for f in $(git log origin/main --grep='^Plan: wall-clock$' --format='%(trailers:key=Change-Summary,valueonly)' | grep .); do test -f "$f" && heavy cargo run -q -p smith-bench -- change "$f" || echo "missing or refused: $f"; done
   ```

   prints no `missing or refused` line (`change` exits 0 when its
   section agrees with its rows and inputs, whatever the stop it
   prints), and one report per landing that ran a full `parity-change`.
5. **Each format moved once:**

   ```
   cd ~/src/rust/smith && rg -no '(pub const VERSION|FORMAT_VERSION|RESULT_VERSION|SUMMARY_VERSION|BASELINE_VERSION)\b[^=]*= *[0-9]+' crates/smith-events/src/lib.rs crates/smith-transcript/src/lib.rs crates/smith-charter/src/lib.rs crates/smith-channel/src/kinds.rs benchmarks/src/result.rs benchmarks/src/summary.rs
   ```

   prints the event vocabulary's 3, the transcript's 5 (4 if the ledger
   shows 08.1 dropped), the charter's 2, the channel's 2, the result's 4,
   the summary's 4 and the baseline's 1.
6. **Both gates pass within their budgets:** `git log origin/main
   --format='%(trailers:key=Gate-Result,valueonly)' | grep -v '^none' |
   grep -m1 .` shows the focused suite within 15 s and the fuzzy suite
   within 60 s.
7. **Each session's own "Done when" holds:** the commands in each session
   file's "Done when", run once each.
8. **Only the change steps changed the prompt's files** (rule 11):

   ```
   cd ~/src/rust/smith && git log 4ddb9b67..origin/main --grep='^Plan: wall-clock$' --format='%(trailers:key=Step,valueonly)' -- crates/smith-local-shell/src/role.rs crates/smith-protocol-llm/src/descriptions.rs crates/smith-protocol-llm/src/prompt.rs crates/smith-domain-session/src/window.rs crates/smith-domain-session/src/compaction.rs | grep . | sort -u
   ```

   prints only `smith-03.1`, `smith-04.1`, `smith-07.1` to `smith-10.1`,
   and steps whose landing carries `Change-Summary:`.
9. **The Don'ts that a command can read** (`code-organization.md`,
   section 7): `for r in skein jig temper; do git -C ~/src/rust/$r log
   origin/main --grep='^Plan: wall-clock$' --format=%h; done` prints
   nothing; `git -C ~/src/rust/smith diff 4ddb9b67..origin/main --
   crates/smith-agent-service/src/profile/quantities.rs
   crates/smith-local-shell/src/presets.rs | grep -nE
   '^[-+].*(compaction_threshold|job_wait|read_window|^[-+]window = )'`
   prints nothing; `rg -n -i 'backoff|line-decoder|planner|shell-cwd'
   ~/src/rust/smith/crates/smith-local-shell/src/role.rs
   ~/src/rust/smith/crates/smith-protocol-llm/src/descriptions.rs`
   prints nothing.

Not checked by a command, and so not aims of this plan: each change's
own effect on `shell-cwd`'s responses, which its report shows beside the
criteria and decides nothing (benchmarks.md 16.6).
