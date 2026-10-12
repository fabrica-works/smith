# Reliability plan: implementing the spikes' lessons cleanly

2026-10-09, revised 2026-10-12. Starts from smith `main` at `564d4408` and
skein `main` at `30dc6f7`.

The plan implements, in smith and skein, the fixes and improvements two
spikes found, without the spikes' shortcuts, and gives smith a permanent
benchmark harness (`smith-bench`, in `benchmarks/`) that shows each fix
works against a real provider. Its first run (2026-10-09 to 2026-10-12,
from smith `30259d8` and skein `e8bc018`, under an older process) landed
168 of its 218 increments. This revision holds only the work left, in
skein's `docs/foundation/development.md` shape: the increments not
landed, and session 13, which proves live what landed unproven
(decisions.md, section 12).

## Inputs

- **The design, which is the contract:** smith `docs/design/` (`domain/`,
  `protocol/` with `events.md` and `limits.md`, `shell.md`, `testing.md`,
  `benchmarks.md`); skein `docs/foundation/` and `docs/design/`.
- **`decisions.md`:** the decisions the plan rests on (sections 1 to 10,
  the first run's `QUESTIONS.md`), the first run's rulings (section 11)
  and the port's decisions (section 12).
- **`findings/`:** what the spikes found, why, and what the clean version
  needs. Entries are cited as L1, S14, ….
- **The first run's record,** for the retrospective only:
  `~/src/rust/plans/reliability-plan/` on debian-16gb-hel1-1, not
  versioned. Sessions do not read it.

## The sessions

Each file is one session; its increments keep their first-run numbers
(decisions.md, 12.6). A dependency is named by capability, with the
increment in brackets. A session starts when what it needs at its start
has landed; an increment that needs more waits for it, and the session
does its other increments meanwhile (development.md, section 8.7).

### smith (`smith/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `00-benchmarks.md` | 00: one command per probe run first, then fixtures 01 and 02, the scripted person, the probes suite, and the probes still to build (`follow-up`, `history`, `credentials`, `budget/visibility`'s told check); the first baseline on events | — | 00.17: smith 07's writing-step budget [07.14]; 00.14: smith 05's lines carried and held [05.13]; 00.16: smith 08's compaction and its records [08.6, 08.10]; 00.30: smith 10's `borrow` and the harness on `smith exec` [10.7, 10.15]; 00.21: session 13's end |
| `03-inline-agent.md` | 03: the product's event stream, with either kind of agent; the rest of the cancel, grace and drop ladder | — | 03.10: smith 06's exit evidence [06.1] |
| `04-limits.md` | 04: effective limits by name; the derivation's rest; the memory default; the profile at shell.md's values | — | 04.22: smith 03's product stream [03.7] |
| `05-messages.md` | 05: typed notices; the reacting person; lines carried and held until read; "waiting" by the fence | — | 05.13's Proof: smith 00's one command per probe run [00.27] |
| `06-teardown.md` | 06: exit evidence, the agent's line, the watchdog, the teardown phase, the derived grace, typed failures with their kept detail, the local service's end, failures in words, worlds at shipped times | — | 06.8: smith 03's product stream [03.7]; 06.9: smith 05's typed notices [05.11]; 06.10: smith 04's profile at shell.md's values [04.14] |
| `07-budgets.md` | 07: the writing-step budget; the recommendation on local `sub_agents` | — | 07.13: session 13's `budget` and `parallel-children` steps [13.3, 13.4] |
| `08-context.md` | 08: windows resumed by the local host; the window check and compaction; owed results cut; a full history compacts | — | — |
| `10-product.md` | 10: the chat store over io, startup reads, headless runs, settings and `smith check`, presets, credentials, the build environment, `smith exec`, the harness on it, sign-in, interactive `smith`, the old commands removed, raw terminal input | — | 10.5: smith 03's product stream [03.7] and smith 06's words function [06.2]; 10.8: smith 05's typed notices and lines carried [05.11, 05.13]; 10.14: smith 04's effective limits by name [04.7]; 10.10: skein's terminal modes [skein 02.6]; 10.16: smith 03's product stream with a spawned agent [03.9]; 10.8's and 10.15's Proofs: smith 00's one command per probe run [00.27] |
| `12-milestone.md` | 12: cross-agent comparison code, the delegation part; fixture 03, `shell-cwd`; the milestone's runs and its report | — | 12.1, 12.2: smith 00's fixtures and one command per probe run [00.13, 00.27]; 12.3: smith 00 (00.21 included), 03 to 08, 10 and 13 landed; 12.4 to 12.7: 12.3 |
| `13-proving.md` | 13: the live proving of what the first run landed unproven (README, "Proving what landed") | smith 00's one command per probe run and probes suite [00.27, 00.29] | 13.2: smith 00's scripted person and `cache-reuse/crossing` [00.28]; 13.7: smith 00's told check [00.17] |

### skein (`skein/`)

| File | Session | Needs at start | Increments that need more |
|---|---|---|---|
| `02-shell-and-worlds.md` | 02: terminal modes | — | — |
| `07-fake-peers.md` | 07, added during the run (decisions.md, 13.12): the fake peers reject what they will not take | — | — |

**How many at once.** At the start, ten sessions can run: smith 00, 03,
04, 05, 06, 07, 08, 10 and 12, and skein 02; smith 13 opens once 00.27
and 00.29 land. The coordinator runs at most eight threads
(development.md, section 7.2). When slots are short, start first, in
this order, the sessions others wait on:

1. smith 00 (00.27 gates the smoke, session 13 and the live Proofs of
   05.13, 10.8 and 10.15);
2. smith 03 (its product stream opens 04.22, 06.8, 10.5 and 10.16);
3. smith 06 (06.1 and 06.2 open 03.10 and 10.5);
4. smith 05 (its notices and lines open 00.14, 06.9 and 10.8);
5. smith 07 (07.14 opens 00.17 and 13.7);
6. smith 04 (04.7 and 04.14 open 10.14 and 06.10);
7. smith 08 (08.6 opens 00.16);
8. skein 02 (opens 10.10);
9. smith 10, then smith 12, then smith 13 when it opens.

**Sessions with more than five increments** (00, 04, 06, 10, 12, 13) are not
split: their implementers hand over between increments (development.md,
section 7.3).

## Rules of this plan

What development.md, smith's and skein's `AGENTS.md` and their workflows
do not already say.

1. **The smoke on every smith landing that changes code** (smith
   `workflow.md`, section 5): `smoke-tools`, live, once, on the branch
   rebased on `origin/main` just before `land`, through `heavy --gate`,
   and once more only if it fails:

   ```
   heavy cargo run -p smith-bench -- build HEAD
   heavy --gate cargo run -p smith-bench -- probe probes/smoke-tools --arm <the directory build printed>
   ```

   Its summary line goes in the commit body and in the report's
   `Evidence:` line. Nothing else is wanted: no manifests, no digests, no
   scripts. A landing that changes only Markdown, or only files under
   `benchmarks/tasks/`, `benchmarks/suites/`, `benchmarks/summaries/` or
   `benchmarks/baselines/`, skips it, and so does a skein landing.
   - **Until smith 00.27 lands,** there is no one command for it: smith
     landings before 00.27 skip the smoke, and their reports say "smoke:
     not run, before smith-00.27".
   - **A smoke stopped by the provider** (an end `Refused(setup)` on the
     credentials, or `Failed` with the provider's capacity or a server
     error as its reason) is not smith's failure: run it once more after
     60 seconds; stopped again, land, and report "smoke not run: <the end
     and its reason>". The coordinator escalates it as `credentials` or
     `infrastructure`. Any other failed smoke is red: it stops the line
     for smith (decisions.md, 12.11).
2. **Proofs** (decisions.md, 12.11). An increment proves live only where
   its Proof line says so: where a calibrated probe guards it. Every other
   Proof is a named test at the layer its Proof line gives. Never build a
   probe, a script or a manifest that the Proof line does not name.
3. **Live runs go through `heavy --gate`,** at most 12 attempts per call,
   so that other sessions' gates interleave; a `suite` (00.21's, 12's)
   runs in one call, unsplit (decisions.md, 12.24). Each commit a command
   names (`HEAD`, `--control`, `--parent`) is first built: `heavy cargo
   run -p smith-bench -- build <commit>`. Probes run on the small tier,
   `gpt-6-luna` at low effort (`benchmarks/agents/models.toml`), unless
   the probe names an effort.
   - **A batch stopped by the provider** (decisions.md, 13.50): when a
     calibration batch or a suite stops because an attempt met the
     provider's capacity or a server error (a 5xx, or `unavailable`),
     smith's own handling of it otherwise correct, it is not smith's
     failure. The command runs once more, from the start, as a new run,
     after 60 seconds; nothing of the stopped run is resumed or reused.
     Stopped by the provider again, the step lands with that probe not
     calibrated, its commit body naming both runs and "stopped by the
     provider", and goes on. This extends rule 1's provider clause from
     the smoke to every live batch, for this run.
   - **skein-world's `EINVAL`** (decisions.md, 13.67): until smith-10.10
     repins skein, a gate or a live command that fails only with
     "opening supervised tree member N: Other(22)" runs once more, from
     the start; failed so again, the session stops on a question.
4. **The harness follows the binary.** An increment that changes how
   smith is invoked, configured or observed changes `smith-bench`'s smith
   adapter in the same increment, so probes keep running. smith's legacy
   face stays until smith 00.21 removes it.
5. **Formats have their versions for this plan:** channel 2, charter 2,
   transcript 3 and events 1, all on `main` since the first run. Later
   increments change these formats under those versions, with no further
   bump. `smith-bench`'s result and summary formats go to version 2 at the
   first landing of this revision that changes each, and stay there.
   Readers refuse other versions, and nothing is translated. This rule
   overrides any bump a session file implies.
6. **Hotspots.** Sessions share `crates/smith-domain-run/src/run.rs`,
   `crates/smith-domain-session/src/session.rs`,
   `crates/smith-agent-service/src/service.rs`,
   `crates/smith-protocol-llm/`, `benchmarks/src/` and `Cargo.lock`. Keep
   increments small (conflicts: development.md, section 8.5).
7. **Never touch the user's logins.** smith and `smith-bench` borrow
   `~/.codex` read only, the access token only. Nothing writes to
   `~/.codex`, `~/.claude`, `$XDG_CONFIG_HOME/smith` or
   `$XDG_STATE_HOME/smith`; no agent signs in; no refresh token is
   copied (benchmarks.md, section 10).
8. **Users the Files line misses** (decisions.md, 13.1). When an
   increment changes a type, a variant or a signature in place, as its
   Shape or its Removes line says, and the change breaks code outside its
   Files and Users lines, the implementer updates that code in the same
   increment, without asking, when each edit is one of these:
   - a match arm for a new variant, doing what that match already does
     for the variants it does not handle;
   - a removed variant or type replaced by the one the Shape names;
   - a test's expected value changed from the removed variant to what
     the same story now produces, its name, steps and other assertions
     unchanged; or changed to what the same story now produces because
     of a behaviour the increment's Scope or Shape names, the commit body
     quoting those words beside the test's name (decisions.md, 13.20);
   - a call changed to a new signature, passing the value the story
     already implies;
   - a lookup or accessor of at most 20 lines added to a type of another
     crate of the same repository, when the Shape's signatures need it,
     the type's fields and other methods unchanged (decisions.md, 13.3);
   - a dependency the increment's code needs, added to a `Cargo.toml`
     (decisions.md, 13.4);
   - at most 100 lines in the library of the world crate the Test shape
     names, that the Test shape's new tests need, with no existing
     story's expected value changed (decisions.md, 13.4 and 13.24);
   - a referee or checker of a world that accepts what the increment's
     Shape newly requires, a line or an event, and nothing broader, its
     other checks unchanged (decisions.md, 13.8); and a test that asserts
     a sequence of requests, events or records gaining one assertion for
     each that the Shape newly requires, where the story now produces it,
     its other steps and assertions unchanged (decisions.md, 13.19);
   - a cue of a test's scripted person or peer that waits for what the
     Scope or Shape now withholds, a notice or an event, moved to the
     next thing the same story shows, only where the withheld one can no
     longer come; in a generator, only those draws change, its seeds,
     counts and other branches unchanged; the commit body quotes the
     Scope's or Shape's words beside it (decisions.md, 13.32 and 13.36);
   - a test fixture's input set so the story keeps testing what it
     tested, where the Scope or Shape changes what that input meant: a
     default the fixture inherited, pinned to its value before; or a
     limit raised so the story fits under a check the Scope or Shape
     adds, at most to the next power of two above what it needs; the
     test's name, steps and assertions
     unchanged, the commit body quoting the Scope's or Shape's words
     beside it (decisions.md, 13.40 and 13.43).

   The commit body lists those files and lookups under "Users outside
   Files:". For
   these edits, this rule replaces development.md 8.2's limits on an edit
   outside Files (20 lines, the session's own files, no expected value
   changed). Any other edit outside Files is still a question: new
   behaviour, a test renamed, removed or with new steps, a limit or its
   accounting. Applies to every session, until the run ends.
9. **A Size line is a guide up to 700** (decisions.md, 13.9 and 13.46).
   An increment that needs more production lines than its Size line
   gives, but at most 700, lands without asking; its commit body gives
   the count by file and what took the lines. Past 700, it stops and
   asks: the plan splits it. Never fold, inline or compress code to fit
   a count. This replaces "at most" in every Size line, and
   code-organization.md 8's "about 600", for this run; the exceptions
   already ruled (10.1's 1,050) stand.
10. **A landed step's check** (decisions.md, 13.21). The coordinator
    checks a landed step by its first two commands of development.md
    7.3 (the commit on `origin/main`, its `Step` trailer), and runs `git
    status --porcelain` only while the worktree is still on the landed
    step's branch (`git -C <worktree> branch --show-current`). A worktree
    on the next increment's branch, clean or not, is that increment's
    work (development.md 8.6: "After a landing, send the report and go
    on"), and the step is `landed`. This replaces 7.3's third check for
    this run.
11. **Diagnostics never committed** (decisions.md, 13.25). To find the
    cause of a failure in its own increment, an implementer may add a
    print or an assertion message in any file of its worktree, run the
    failing test at most three times in all for that failure, and remove
    every such line before it commits; `git diff origin/main` shows none of
    them at `land`. Nothing else is wanted: no scripts, no repeated runs,
    no observers (development.md 8.2). Past three runs, it stops and asks.
    The three count runs made with such lines in place, for one failure;
    a run after a change to the increment's code, tests or fixtures, with
    none in place, is ordinary work, and is not counted (decisions.md,
    13.38).
12. **The Pulse words never land** (decisions.md, 13.45). Before `land`,
    on its rebased branch, an implementer runs, once:

    ```
    git diff origin/main -- . ':!docs' | grep -inE '^\+.*\b(actually|genuine|immutable|canonical|verified|qualified|robust|properly|python3)\b'
    ```

    It prints nothing; each line it prints is reworded to say what the
    code does, with no synonym in its place. Words already on `main`
    stay. Applies to every session, until the run ends.

## Proving what landed

The first run landed 95 increments as "merged, unproven": their
probes were not yet calibrated live. Steps 1 to 3 of its proving
(`smoke-tools`, `first-request`, `failures`) were calibrated, and stand
(decisions.md, 12.4). Session 13 calibrates the rest, one probe per
step; a step's pass proves what its probe guards, and its commit body
names them.

| Step | Probe | Proves |
|---|---|---|
| smith 13.1 | `large-write`, `large-write/oversized` | smith 00.26, 04.3, 04.10, 09.13; skein 04.3 |
| smith 13.2 | `cache-reuse`, `cache-reuse/children`, `cache-reuse/crossing` | smith 04.optional-usage, 05.7, 08.1, 08.3, 08.4, 08.5; skein 04.7, 04.9, 04.11 |
| smith 13.3 | `budget`: `starvation`, `wrap-up`, `child-quota`, `spend` | smith 07.1, 07.2, 07.6, 07.7, 07.8, 07.10, 07.11, 07.12; skein 04.2 |
| smith 13.4 | `parallel-read`, `parallel-children` | smith 04.12, 07.5; skein 03.3, 03.5, 05.6, 05.8 |
| smith 13.5 | `tool-output`, `paths`, `process-tree` | smith 00.15, 04.1, 09.3 to 09.12; skein 01.1, 01.2, 01.tree-termination, 01.tree-subreaper-restoration |
| smith 13.6 | `slow-completion` | smith 04.4, 04.11; skein 03.1, 04.4, 04.12 |
| smith 13.7 | `budget/visibility` | smith 07.9 |

The others (decisions.md, 12.16):
- **proven by steps 1 to 3,** which ran them in their calibrated
  binaries: smith 00.10, 02.2, 02.4, 02.resumed-binding, 03.1 to 03.6,
  04.5, 05.2; skein 01.4, 04.1, 05.7;
- **proven when session 00's probes calibrate:** `follow-up` (00.14):
  smith 05.4, 05.5, 05.6, 05.10; `history` (00.16): smith 04.6, 04.8,
  05.8, skein 04.6; `credentials` (00.30): skein 06.2, 06.3;
- **standing on their deterministic tests,** since their session files
  named no probe: smith 01.4 to 01.7, 03.8's landed part, 04.2, 05.1,
  05.3, 05.9, 05.opening-extract, 05.inbox, 07.3, 07.4; skein 01.8,
  01.supervised-stderr, 05.5, 06.6;
- **shown by `smoke-tools`' teardown check** once smith 06 lands: skein
  02.4, 03.2.

A probe that falls short of calibration for reasons that are not a smith
defect lands as not calibrated, with its counts, and the increments it
guards stand on their deterministic tests (decisions.md, 12.4).

## Live services

**OpenAI, through the user's Codex login.** The only live service.
- **How it is borrowed:** `smith-bench` reads `~/.codex/auth.json` read
  only, and gives each attempt a private, access-only copy (benchmarks.md,
  section 10; rule 7). Codex's own arm runs in an attempt-owned
  `CODEX_HOME`. No agent signs in. A lapsed login stops every live run:
  the coordinator escalates it as `credentials`, and the user runs
  `codex` once.
- **Codex runs from a private copy of its pinned version** (decisions.md,
  13.41 and 13.44): the release's whole package, unpacked in
  `~/.local/state/smith-bench/tools/codex-0.160.0/`, whose `bin/codex` is a
  regular file, first on `PATH` for each `smith-bench` command that runs
  Codex. The user's own install, which updates itself, is never used for
  an arm, and never changed.
- **It is shared with the run itself** (development.md, section 13,
  "Credentials under load"): a capacity error on a live run is a setup
  refusal, not smith's (rule 1).
- **Budgets,** in attempts and in `smith-bench`'s notional dollars
  (`benchmarks/agents/prices.toml` once smith 12.9 lands; the first run's
  prices until then):

  | Who | Attempts | Notional | Gate-lane time |
  |---|---:|---:|---:|
  | the smoke, on each smith code landing (about 80) | 160 at most | $2 | 15 min |
  | smith 00 (00.27, 00.14, 00.16, 00.30, 00.21) | about 115 | $25 | 4 h |
  | smith 05.13, 10.8, 10.15 | 8 | $1 | 35 min |
  | smith 13 | 248 | $22 | 8 h |
  | smith 12 | 126 | $45 | 8 h |
  | **the run** | **about 660** | **$95** | **about 21 h** |

  A session past its line's budget by half stops on a question. The run
  past $60 is reported by the supervisor to the user, once.

## Repin

None: the plan changes skein and smith, and repins neither jig nor
temper, which it never edits (decisions.md, 12.8). smith's own increments
repin skein when they need a skein change (development.md, section 5.3).

## Pulse

```
words: actually|genuine|immutable|canonical|verified|qualified|robust|properly|python3
types: ^(Legacy|New|Old|Owner)[A-Z]|V[0-9]+$
file-lines: 800
budgets: 15 60
```

## Done when

Each condition is checked by its command, on `main`, after the last
step lands.

1. **Every step landed or dropped:** `plan status` lists no step
   `waiting`, `ready`, `running` or `blocked`.
2. **Each session's own "Done when" holds:** the commands in each session
   file's "Done when", run once each.
3. **Session 13 ran its seven steps:** in `~/src/rust/smith`, `git log
   origin/main --format='%(trailers:key=Step,valueonly)' | grep -c
   '^smith-13\.'` prints 7 or more, and `git log origin/main --format=%B
   --grep='^Step: smith-13\.' | grep -c '^Calibrated: '` prints 7.
4. **The smoke suite passes on `main`:** `heavy cargo run -p smith-bench
   -- build HEAD`, then `heavy --gate cargo run -p smith-bench -- suite
   smoke --arm <directory>` exits 0, once.
5. **Both gates pass within their budgets:** in each repository, `git log
   origin/main --format='%(trailers:key=Gate-Result,valueonly)' | grep -m1 .`
   shows the focused suite within 15 s and the fuzzy suite within 60 s.
6. **smith's report is published:** `test -f` on the report session 12's
   12.7 names under `docs/development/benchmarks/`.
Not checked by a command, and so not an aim of this plan: smith's
comparison against Codex coming out in smith's favour. Session 12
reports what it measures.
