# Benchmarks

Provisional, 2026-10-09, revised 2026-10-19. How smith shows that a change has the effect it
claims and keeps it, and how smith compares with other agents. Tests are
testing.md's; the events smith is observed through are
protocol/events.md's; its commands, settings and exit codes are
shell.md's. What is still open is listed in section 15; parity with
Codex, the comparison's verdict, is section 16.

## 1. In one page

- **Tests specify; benchmarks show.** A behaviour is tested in the lowest
  tier that shows it. A benchmark runs the product against real
  providers to show that a change has its effect in the world, and keeps
  it. Benchmarks are never the merge gate.
- **One notion of measurement.** A task is a seed, a prompt and checks,
  run by an agent configuration some number of times. **Probes** force
  one behaviour and pass or fail; **fixtures** are small coding tasks,
  for trends and comparisons; **repository tasks** are real repositories
  at a frozen commit, run rarely.
- **Cheap by default.** Tasks name a model tier, not a model. Probes run
  on the small tier at low effort, cheapest first; comparisons and
  repository tasks run by choice.
- **A fix names its probe,** and its commit body records the probe
  failing before it and passing after it. A change to the runtime,
  limits, providers or host records a smoke run there.
- **Agents are adapters.** `Smith`, `Codex` and `ClaudeCode` are a closed
  set, each run as it ships, at a pinned version, on a login of the
  harness's own, signed in once with the agent's own flow, never a
  person's own tool directory and never a copy of a refresh token, under
  a hard deadline on its whole process tree. A pin changes only what an
  attempt must own: its paths, its model and effort, its account, and
  the keys that keep it unattended and isolated. smith is observed only
  through its event stream.
- **Unknown is never zero.** A measurement is observed, a lower bound, or
  unavailable with its reason. Tokens count once per response.
- **Statistics fit the question.** A probe reruns once on failure. A
  comparison interleaves its arms in one session and reports a median
  ratio with a bootstrap interval against the task's minimum detectable
  effect. Baselines are drift alarms. Raw results stay outside the tree.
- **Parity is a verdict, not an absence of effects.** smith is on par
  with Codex when one run of the `parity` suite shows, for outcome, time
  and cost, that smith is not worse than Codex by more than a stated
  margin. One offline command reads the committed summary and exits 0 at
  parity (section 16). A verdict short of it is committed too, and is
  answered by a change that names its cause, never by another run of the
  same smith (16.5).
- **On the way to a verdict, a change shows the criteria.** After a
  plan's baseline, a change to what the model is sent runs against its
  parent on the verdict's tasks when that run can tell its expected
  effect from none, reports each criterion's ratio, and stops on a
  question to the designer when one moves the wrong way past the
  baseline's interval, whatever its own comparison concludes. Any other
  lands on the counts it targets, and the closing run measures it with
  the others (16.6).
- **Spend is priced as billed.** Each response's tokens are priced at
  the list rates its own prompt reaches, never at the rates an agent's
  declared window could reach (8.1).

## 2. What benchmarks are for

- **To show a change is effective,** and that it stays so while
  providers, models and other agents change under smith: a probe sees a
  provider's wire, routing and caching as they are.
- **To compare agents** on the same tasks, prompts and graders, so that a
  difference in outcome, time, tokens or cost is the agent's, not the
  harness's.
- **To find what the tests missed:** passive checks on every attempt hold
  smith's evidence to its contracts (section 11.2).

They are not tests (section 12) and not the merge gate: no check of
`docs/development/workflow.md` runs an agent. Nor are they a ranking: a
comparison states an effect with its interval, or that none was
detected.

## 3. What is measured

### 3.1 Tasks and kinds

A task (section 5.2) holds a prompt, byte-identical for every agent, a
seed, a model tier, budgets, checks and the design sections it guards.
An attempt is one run of a task by one agent configuration in a fresh
workspace, ending in one result (section 8). The kinds are a closed set:

| Kind | Judged by | Default design | Cost per attempt (estimate) |
|---|---|---|---|
| probe | every check passes | one attempt, rerun once on failure | seconds to a minute |
| fixture | the grader's pass rate, and metrics against another arm | interleaved arms, at least five attempts each | minutes |
| repository | the grader's pass, and large effects only | three attempts per agent | tens of minutes |

- **A probe forces one behaviour.** Its prompt leaves the model no choice
  ("with one `write` call", "delegate with `max_turns` 3"); its seed is
  tiny and its outcome exact. One that cannot be calibrated on the small
  tier at low effort names a higher effort and says why. Its outcome
  checks hold for any agent; its event checks read smith's stream, and
  hold smith to its contracts or count what the prompt asks of the
  model (3.2 says whose a failure is).
- **A probe is calibrated** before it gates anything: before it joins
  `smoke`, and before its failure in `probes` counts as a regression
  (section 9). `probes` lists every probe and variant, calibrated or not
  (11.5); its task file's `[calibration]` block, one per task and one per
  variant, says which (5.2). Calibrated means ten passes within at
  most twelve attempts on the binary it guards, the other attempts failed
  only by the model, never by smith (a model deviation, 3.2; section
  8.3's ends), and a failure on the binary before the fix or,
  for a behaviour already in place, on a control arm. Where neither can
  run, as for a behaviour older than the oldest binary the harness runs
  (6.2) that no setting turns off, the block says why, and the
  calibration stands on its passes. A probe that cannot
  fail guards nothing. A calibration stands while the variant's prompt,
  seed and checks keep the digests its block records; a new task version
  that changes none of them for a variant keeps that variant's block.
- **A fixture** has a hidden grader, run after the agent has exited.
  Every healthy agent passes it, so it tells agents apart by time, tokens
  and cost. **A repository task** is frozen with every dependency
  vendored, so that it builds offline for good and its grader may rely on
  that commit's interfaces; it shows long conversations, delegation,
  compaction and budgets at scale. Every grader fails on the seed and
  passes on the task's reference solution, and the reference passes the
  task's outcome and protected checks.
- **A large effect** is the repository kind's only claim across agents:
  for a metric both observe, the two arms' observed ranges over their
  attempts do not overlap. Overlapping ranges claim none, and a missing or
  lower-bound value keeps its reason and claims no separation. Nothing is
  inferred about significance from three attempts. The summary computes
  it. Parity runs the repository task interleaved, ten attempts per arm,
  with intervals (section 16).

### 3.2 Checks and arms

- **Outcome checks** are agent-neutral, read once the agent's tree has
  ended: a file's content, digest, absence or pattern; protected files;
  writes outside the workspace and the attempt's private directories;
  the end (section 8.3) and, for smith, its exit code and the words on
  its standard error (shell.md, section 4); the grader's commands.
- **Event checks** read smith's event stream (protocol/events.md, section
  3): a closed set of named, typed checks with parameters, such as "one
  `write` call with `input_bytes` of at least N, `written`", each with an
  offline test.
- **Passive checks** run on every attempt (section 11.2); a task that
  expects a failure names those it waives. A failed check carries its
  evidence: the records or files it read.
- **What decides.** A probe names its deciding checks. A graded task (a
  fixture or a repository task) decides on its end, its grader, its
  protected files and its tree's end. A passive check that cannot be
  observed for an agent, such as reads of the benchmark tree or writes
  outside the workspace before contained trees, is reported beside and
  audited, and decides nothing.
- **Whose failure.** A failed deciding check is the model's when it
  judges what the prompt asks of the model: the answer, the files, a
  report of what the model was told (`budget-report`), and how many
  calls of a tool it makes where the prompt prescribes the number
  (`call-count`). It is smith's when it judges smith's contracts: its
  records, a call's verdict, a request's tool choice or content, a
  bound, the window. An attempt that completed, and whose failed
  deciding checks are all the model's, is a model deviation; any other
  failure is smith's. A deciding check of smith's contracts left
  unavailable, other than "not exercised" (below), leaves the attempt
  unjudged: neither smith's failure nor a deviation (section 1).
- **Not exercised.** Some of smith's contracts apply only once their
  condition arises: a call past a bound, a call that lasts over 60 s, a
  window at its compaction threshold. A probe's prompt asks the model to
  bring the condition about, and a model can stop short of it: it
  honours the bound, answers quickly, or finishes before its reading
  fills the window. A check of such a contract, in an attempt where its
  condition never arose, is "not exercised": neither a pass nor a fail.
  What kept the condition away was the model's, so in a calibration the
  attempt counts like a model deviation (3.1): never a pass, and within
  the attempts the model may fail. Section 11.1 says, for each probe,
  what the condition is and how the stream shows it. A check that
  cannot tell whether its condition arose, because a record it reads is
  missing, is unavailable instead, and leaves the attempt unjudged.
- **A count is the model's only where smith's side is checked.** A call
  is made again, or skipped, as readily when smith lost what the model
  had as when the model did: a result missing from a compaction's
  request, or a summary missing from the next window. So a task whose
  windows compact also decides on `compaction-carried`: each
  compaction's request holds a result for each call its window owes it,
  and the window after it opens with that compaction's summary
  (domain/session.md, sections 8.1 and 8.2). With that check passing, a
  summary that misstates or drops what it was given is the model's, and
  so is the count that follows from it.
- **A request smith retried** (domain/session.md, section 4) is judged
  through its retry. A request that failed with no blocks made no calls,
  and its prompt is the one its retry sends (protocol/events.md, section
  3.11), so the failed request's missing usage leaves no check
  unavailable. Its tokens are a retried failure's, or none when it failed
  unsent (8.2). A failure smith
  did not recover from is the attempt's end (8.3).
- **An arm** is a binary, a configuration override or an agent. Two arms
  make an experiment: a **fix** (before and after), a **control** (the
  mechanism switched off), a **decision** left to measurement (section
  11.3), or a **comparison** (another agent, or main against a
  candidate). An arm built from a commit is built in the harness's own
  clone, outside every developer checkout, and named by the commit and
  the binary's SHA-256. One build runs at a time: it holds an exclusive
  lock over the clone and its target, and a second waits.

## 4. Tiers

### 4.1 When each runs

| Tier | Contents | When | Agents | Cost (estimate) |
|---|---|---|---|---|
| offline | parsers on recorded outputs, manifests, statistics | in the focused suite | none | under a second |
| smoke | a few probes and the passive checks, one attempt each | each change to the runtime, limits, providers or host, and each fix | smith on `openai`, small tier; `anthropic` too for a change to its path | a few minutes, under 100k tokens |
| probes | every probe and variant, both providers; outcome halves on other agents | weekly | smith; Codex; later Claude Code | about an hour, a few million tokens |
| compare | fixtures, interleaved | on demand | smith and the agents compared, working tier | one to two hours |
| repository | repository tasks | at milestones | smith and Codex, working tier | several hours |

Only the offline tier is a test (section 14). The others are runs of
`smith-bench`, by choice. Each tier is a suite file (section 5.3).

The costs are estimates for planning, not gates. The offline tier's
tests are held to the workspace's suite budgets (smith's
`docs/development/workflow.md`, section 2), as every focused test is.
A test sits with what it tests: a parser's beside the parsers, an
adapter's among that adapter's tests.

### 4.2 A fix and its probe

- **Proof comes before the merge.** A change merges, and counts as done,
  only with its proof in its commit body (`docs/development/workflow.md`,
  section 5). Evidence is never left "pending".
- **Every improvement names the probe that guards it** in its commit
  body, except one whose effect does not depend on a model
  (`docs/development/workflow.md`, section 5), which names its counting
  test instead. A probe comes with the runs the harness prints ready for
  it:
  - **after,** on the change, passing;
  - **before,** on its parent, failing, wherever the parent can run the
    probe live;
  - **Codex,** on the same task, where the probe compares agents.
- **A fix with no probe adds one,** calibrated, in the same change. A
  defect no probe can force, such as a race or a rare provider stall, is
  guarded by a passive check or an event check on the closest probe, and
  the commit says which.
- **An improvement shown on fixtures,** such as a change to smith's role
  (shell.md, 6.6), names the `compare` suite in place of a probe: one run
  with two arms, its parent and the change, interleaved (section 9),
  whose comparison goes in its commit body. It adds no probe. In a plan
  whose aim is a parity verdict, after its baseline, a change to what
  the model is sent names the `parity-change` suite in its place when
  that run can tell the change's expected effect from none, with 16.6's
  report; any other names the counts it targets (16.6).
- **Every smith landing that changes code runs `smoke-tools` live,**
  once, on its branch rebased on `origin/main` just before `land`
  (`docs/development/workflow.md`, section 5). A red smoke stops the line
  until it is green again.
- **A parent that cannot run live.** When a parent fails the probe for a
  reason unrelated to the change (for example, its limits refuse what the
  provider sends today), the before is the closest evidence that exists:
  - the change's deterministic tests, which fail on the parent;
  - recorded measurements, such as the spike's;
  - a same-code run with a setting that turns the feature off, where one
    exists.

  The commit says which. No switch is added only for this.
- **The probe stays** in its suites with its `guards`, so a later failure
  names the design sections at stake.
- **Choosing what to run.** The harness lists the tasks guarding the
  sections a change cites, cheapest first by their committed medians or
  estimates, within the suite's wall and token budgets, and reports what
  did not fit. A fix's probes run at least once before it and once after;
  repetition beyond that is the statistics' (section 9).

## 5. Layout and formats

### 5.1 The crate

```
benchmarks/
  Cargo.toml               smith-bench, a member of the workspace
  clippy.toml              the harness's own lints
  README.md                how to run it, and sign each agent in once
  src/                     the harness
  tests/                   the offline tier and its recorded outputs
  tasks/probes/<id>/       task.toml, seed/
  tasks/fixtures/<id>/     task.toml, seed/, grader/, reference/
  tasks/repository/<id>/   task.toml, seed/ (a manifest), grader/, reference/
  suites/<name>.toml       what a run runs
  agents/<agent>/          the pinned configurations
  agents/models.toml       the model tiers
  baselines/               per suite, agent configuration and tier
  summaries/               one per committed run
```

- **Ordinary Rust** under the test-harness rules (programming-model.md,
  section 10.2): its orders, samples and bootstrap draw from a seed it
  records, and it measures time on one monotonic clock.
- **Its own `clippy.toml`.** The root file's shell bans include
  `std::process` (shell.md, section 3.2), and the harness starts
  processes. Like `tests/clippy.toml`, its file keeps the bans that make
  a run reproduce from its seed and read one clock.
- **Modules by concern,** one per agent (section 6). smith's events are read through `smith-events`, so a
  changed record fails the build or the offline tier, never a run.
- **Commands** run a suite or a task with its arms, calibrate a probe,
  record a fix's before and after, list the tasks guarding a section,
  summarise, write a baseline, sign an agent's home in, and check the
  tree offline. Their spelling is the README's.

### 5.2 Tasks

```toml
id = "large-write"
version = 1
kind = "probe"
guards = ["domain/tools.md, section 3", "protocol/limits.md, section 6"]
tier = "small"
deadline_seconds = 120
prompt = """
Create numbers.txt containing the integers 1 to 6000, one per line, \
with a single write call. Then finish.
"""

[[outcome]]
check = "file-digest"
path = "numbers.txt"
sha256 = "..."

[[event]]
check = "tool-call"
tool = "write"
input_bytes_at_least = 16384
verdict = "written"
```

| Field | What it holds |
|---|---|
| `id`, `version`, `kind`, `title` | `version` changes with the prompt, seed, grader or checks; results of different versions are never compared |
| `guards`, `behaviours` | design sections, cited as the design cites them; the design's names for what is guarded |
| `tier`, `effort` | the model tier; an effort only where calibration needs one |
| `prompt` | delivered whole, through a file or standard input |
| `deadline_seconds`, `estimate` | the hard deadline on the attempt's process tree; seconds and tokens, until measured |
| `[budget]` | turns, time and spend, given to each agent as far as it takes them (section 6.1) |
| `[smith]`, `[environment]` | settings overrides, passed as `-c` settings, never through the seed; variables the agent's commands see |
| `setup`, `protected` | commands that warm caches outside the timer, with the agent's own build profile; files that must not change. A task that builds warms its build in `setup`. A task never protects a path its prompt asks the agent to change: that the seed's tests still pass is its grader's, which holds a copy of them |
| `[[outcome]]`, `[[event]]`, `[[grade]]`, `waives`, `deciding` | the checks; the passive checks the task expects to fail; the checks that decide its verdict (3.2) |
| `[calibration]` | a probe's calibration, per task and per `[[variant]]`, written by `calibrate` and committed with the summary it names: the arm's commit and SHA-256, the date, that summary, the attempts run, the passing attempts, the model deviations, the control and the attempts that failed before, and the digests of the variant's prompt, seed and checks. A probe without one gates nothing (3.1) |
| `repetitions` | where the kind's default does not fit, as for a probe judged on a median |
| `[[variant]]` | a named change to the prompt, overrides or checks, reported as a task of its own (`large-write/oversized`) |
| `[[person]]` | for an interactive task: the scripted person's lines, each sent at the start or when an event the task names is observed (for example `follow-up` sends its request when `finish` is called), and an end of input |

- **`seed/`** is the workspace as the agent first sees it, committed in a
  fresh repository the agent's configuration trusts, its digest in the
  manifest. It holds no `.smith/` settings unless the task is about
  them, never `benchmarks/`, and no file of any grader or reference. A
  repository seed is the bundle's manifest (source commit, vendored pins,
  digest); the bundle lives in the harness's data directory. Every check
  that reads the seed, the protected files' included, reads what the
  workspace was copied from: the bundle, for a repository task.
- **`grader/`** holds hidden tests and their data. Each `[[grade]]`
  command runs after the agent's tree has ended, in a copy of the final
  workspace with the grader beside it, under a deadline, in its own
  process group, with bounded logs; its exit is its verdict.
  **`reference/`** is a solution, applied to the seed to validate it.

### 5.3 Suites

```toml
name = "smoke"
tier = "small"
design = "single"              # or "interleaved"
repetitions = 1
rerun_failed = 1
max_wall_seconds = 600
max_tokens = 100000
tasks = ["probes/first-request", "probes/budget/wrap-up"]

[[agents]]
agent = "smith"
provider = "openai"
config = "standard"
```

A suite names its tasks (`<kind>/<id>[/<variant>]`, or by kind and
behaviour), its agent configurations, a tier overriding the tasks' own,
its design, repetitions and budgets, and its arms where it compares. A
suite may give one task repetitions of its own, as `parity` does (11.5).
A task listed under `when_asked` runs only when a run names it
(`--include`). A new schedule is a new file. An attempt whose every
provider request was refused with a 4xx status before any output charges
no tokens to the suite's `max_tokens`; its usage stays unavailable. An
attempt whose total tokens miss only retried failures (8.2) charges the
total's lower bound, what the provider reported, and the suite goes on.
Any other unknown in an attempt's total stops the suite as incomplete,
since what it would charge is not known.

### 5.4 Agent configurations and results

- **`agents/<agent>/`** holds pinned configurations, each for a stated
  version of the agent, changed only by commit. "Vanilla" is the agent's
  defaults plus this file. A suite's agent configuration is an agent, a
  provider, a pinned configuration and a tier, and its digest is in
  every result's identity.
- **A comparison's pin changes nothing the agent ships** but what the
  attempt must own: its paths (home, temporary directory, caches, trace),
  its model and effort, its account, one of the harness's, and the keys
  that keep it unattended and isolated (section 10). It restates no
  default. An experiment's arm names each key it changes, and only those
  (11.3). A probe that needs another value, a small window or an empty
  brief, names it in its own task's `[smith]` overrides (5.2).
- **Raw results stay outside the tree,** under
  `$XDG_STATE_HOME/smith-bench/runs/<run>/<attempt>/`: the result, the
  agent's output and records, standard error's tail, the grader's logs,
  and the diff against the seed's commit, the agent's commits included.
  Workspaces are deleted once archived; anything raw is scanned for
  credentials before it leaves the machine.
- **Committed:** `summaries/`, one per run of the probes, compare,
  repository and parity suites (per task and arm, counts by end, medians,
  ranges, comparisons, drift notices and audits, and each attempt's
  identity, end, judgment, task wall, common priced spend, token fields,
  responses and the agent's own peak resident memory, so that a verdict
  recomputes from the file), and `baselines/`
  (section 9). Smoke runs, and a fix's before and after, go in commit
  bodies; a calibration goes in its task file (5.2).

## 6. Agents as adapters

`enum Agent { Smith, Codex, ClaudeCode }` is matched exhaustively, one
module per variant, in five steps. **Prepare** the attempt's private
directories and the agent's pinned configuration, writing nothing in the
user's tool directories.
**Command:** an exact environment from an allow-list, nothing inherited,
and the prompt on standard input. **Observe** the output as it arrives,
as neutral observations: a response and its usage, a tool call, a child,
the answer, which ends task wall time and starts teardown. **Collect**,
once the tree has ended, what the agent persisted, as metrics with their
scope and convention. **Classify** one end (section 8.3). Claude Code is
built after Codex, and designed now so that it is one module and its
files.

| Step | Smith | Codex | Claude Code |
|---|---|---|---|
| prepare | `XDG_CONFIG_HOME` holds the pinned settings as the user's layer: the account's file and the trace, capturing `calls` so the scope gate sees each call's input (`everything` where a task needs prompts), and nothing else of a comparison's; `XDG_STATE_HOME` the attempt's state; `credentials` the harness's credentials directory, and the account one of its own (section 10). Before an arm's first attempt, `smith check --json exec` with the attempt's own `exec` flags records the effective limits, the teardown bound, schema versions and credentials of the run it will start; a nonzero exit refuses the arm as setup. The attempt's events are its standard output's, under `--json`: the harness gives `exec` no trace file, so the run has one event destination | a `CODEX_HOME` of the attempt's own, holding only an access-only `auth.json` the harness writes from the harness's own Codex login (section 10), so none of the user's instructions, skills, plugins, memories or configuration reach it and its rollouts are the attempt's; only the keys an attempt must own (5.4) pinned by `-c` overrides: no MCP servers, the attempt root trusted, workspace edits allowed, no approvals, the model and effort, and the attempt's private caches among its sandbox's writable roots, so that it can write them; no other key, so that the rest is Codex's default; its background service disabled where it can be, or ended with the tree | `CLAUDE_CONFIG_DIR` the harness's own configuration directory, signed in once with `claude auth login` (section 10); the attempt's own settings, instructions and empty MCP configuration given by flags (`--settings`, `--strict-mcp-config`, `--setting-sources`), so that none of the login's directory's reach it, and its session found by its id; edits and commands allowed without asking |
| command | `smith exec --json -C <workspace> -m <endpoint>/<model> --effort <effort> -`, overrides as `-c KEY=VALUE` (shell.md, section 4.2); interactive, `smith --trace <file>` at a pseudo-terminal | `codex exec --json -m <model> -c model_reasoning_effort=<effort> -C <workspace> -o <final> -` | `claude -p --output-format stream-json --verbose --model <model> --effort <effort> --max-budget-usd <ceiling> --settings <pinned> --mcp-config <empty>`; `--input-format stream-json` for later messages |
| observe | the event stream, through `smith-events`; standard error is never parsed | JSON lines: thread, turns, items, and each turn's usage, of the root thread only, cached input within input | the opening record, assistant messages with usage, sub-agent messages with their parent's call, the result with durations, turns, cost and per-model usage, in the pinned version's recorded shapes |
| collect | nothing beyond the stream | the rollouts of the thread named and of each thread linked to it as a child (codex-cli 0.160.0 names its parent in the child's opening record): per-response usage, children and their spans, and each command's span against its call's; where a child is not linked to its parent, child scopes are unavailable and totals lower bounds | nothing; transcripts are archived |
| classify | the exit code (shell.md, section 4.3), held against `run.completed` | the exit, the last turn's completion or failure, the deadline | the result's subtype and error flag, the exit, the deadline |

- **One runner.** Each attempt runs on skein's world harness, end to end
  (skein's examples.md, section 6): the agent starts as it ships; a
  scripted person types where a probe is interactive; the attempt's
  referee reads the output as it arrives and keeps the hard deadline.
  The live tier runs smith the same way.
- **An adapter fails loudly.** A known record in an unknown shape ends the
  attempt as a harness error naming it. Unknown record types are skipped
  (protocol/events.md, section 6).

### 6.1 What each agent sees

- **Instructions.** A seed's are in `AGENTS.md`, and every agent gets the
  same text through the file it reads, whole: smith reads it into its
  system text by its default guide convention (domain/run.md, 3.3), Codex
  into its first user message, and Claude Code's `CLAUDE.md` is made at
  preparation and left out of the diff. smith runs with its shipped role
  (shell.md, 6.6): its pinned settings add nothing and remove nothing. No
  home or directory above the attempt's root holds
  instructions: each agent's home is the attempt's (section 10), since
  an agent may read its home's instructions whatever its command line
  says (codex-cli 0.160.0 reads `CODEX_HOME/AGENTS.md` unconditionally).
- **Models.** A comparison's smith declares each model as its presets do
  (shell.md, 6.1): its window, output and deadlines. A smaller window or
  output is a probe's, named in its task (5.2).
- **The environment.** `HOME`, `TMPDIR` and the build caches are the
  attempt's; `PATH` and the toolchain's home are the machine's,
  read-only; then the locale and the agent's home. A task's variables
  reach smith's commands through its settings, which own their
  environment whole, and the others' through their own process.
  A login shell sees the same `PATH`: the attempt's home holds a
  `.profile` that sets it to the machine's. Agents that run commands in
  a login shell would otherwise lose it, since the system profile
  resets `PATH` (codex-cli 0.160.0 runs each command as `bash -lc`, and
  Debian's `/etc/profile` drops `~/.cargo/bin`).
- **Budgets.** smith takes turns, time and spend; Claude Code a spend
  ceiling; Codex none. In a comparison every agent's own limits sit above
  the task's hard deadline, the bound they share: a fixture or a
  repository task names no `[budget]`, so smith runs on its run defaults
  (shell.md, 6.4), and `check` refuses such a task that names one, or
  whose `deadline_seconds` is not below the defaults' `budget.seconds`.
  Each result
  records the effective limits, and a budget end is labelled as one.
- **Caches.** Each agent can write the attempt's build caches: a sandbox's
  writable roots include them. A task that builds warms them in `setup`,
  outside the timer.

### 6.2 smith as it ships

- **One face.** The Smith adapter drives smith only as it ships: `smith
  exec --json` headless, or `smith --trace <file>` at a pseudo-terminal
  for an interactive task, with its settings as the product's TOML
  layers (shell.md, section 5) and a task's overrides as `-c` settings.
  A binary that predates them is not run, and no adapter face, settings
  template or reader of an earlier binary is kept.
- **What a trace lacks is unavailable,** never estimated. A result names
  the face, and results of different faces are never compared.
- **Baselines** taken on an earlier face are retaken or dropped (section
  9). Results from before this harness are never a baseline.

## 7. Models as tiers

| Tier | `openai` | `anthropic` | Used by |
|---|---|---|---|
| `small` | `gpt-6-luna`, low effort | `claude-haiku-5-5`, low effort | probes, smoke, the live tier |
| `working` | the user's working model and effort | the same | fixtures, repository tasks |

- **`agents/models.toml`** holds this mapping, pinned: the working tier
  changes by commit when the user's working model does, so that a
  comparison never mixes models. A new tier is an entry; a new model for
  a tier starts new baselines.
- **A tier's model is one smith's presets declare** (shell.md, 6.1), so
  that smith runs it as it ships; a model the presets lack gets its row
  there first.
- **Providers are a closed set,** named as smith's presets (shell.md,
  section 6.2). smith runs on both, Codex on `openai`'s models, Claude
  Code on `anthropic`'s. Each result records the model and effort asked
  for and those the agent reports; a mismatch is flagged.
- **smith's accounts.** smith's own loop runs on `openai` with a sign-in
  of the harness's, and on `anthropic` with a key. smith on a Claude
  subscription runs Claude Code's loop under smith's adapter
  (protocol/programs.md): its results are an arm of their own,
  `smith-claude-code`, never pooled with smith's own loop's, and it is
  not measured until the program backend is built.

## 8. The result schema

### 8.1 One result per attempt

A JSON document. Like every format it is pre-release: one version, any
change a reader would notice is a new one, and a reader refuses others.
Committed summaries and baselines of an older version are regenerated or
dropped, never read across versions.

| Part | What it holds |
|---|---|
| identity | run, attempt and arm; the task's id, version, seed and prompt digests; the agent, its version, its binary's SHA-256 and, for smith, its face; the configuration's digest; the provider; model and effort, requested and resolved; the harness's commit; the host and CPUs used; the start, in wall time |
| timing | on the harness's one clock: process wall, to the tree's end; task wall, to the answer observed; teardown, from the answer to the tree's end; grading wall. The agent's own times beside them: smith's `first_byte_ms`, `largest_gap_ms` and longest completion, and its `t_ms`, from its own start. The time record: the seconds of the main conversation (Codex's root thread) in model requests, with their first-byte and largest-gap sums; in each tool, by name; in compaction; waiting on children; and the rest; and the seconds of commands that ran outside any call |
| tokens | per scope (section 8.2) |
| counts | responses, provider attempts and retries; completions; tool calls by name, not comparable across agents; tool failures by verdict; children; conversations and compactions; messages by terminal |
| delegation | for each child: its effective share, how it ended (an answer, its budget, failed, closed) and its turns and seconds; the time the parent was blocked on children |
| resources | CPU time and peak memory, each with its scope: the tree's accounting, the agent's process alone, or unavailable. smith's own peak resident size is its `session.ended`'s `peak_rss_bytes` (protocol/events.md, 3.2) |
| spend | the agent's own figure, such as smith's `spent` or Claude Code's cost, with its basis: notional, priced or reported; and the common priced spend, every agent's tokens priced on one basis, as its provider bills them: each response's tokens at the list rates of the tier its own prompt reaches, from smith's presets' price list (shell.md, 5.2 and 6.1) at the harness's commit, which the summary names. The agent's declared window plays no part. The adapter reads each response's usage from the agent's records: smith's `response.completed`, Codex's usage per model response. Without a response's usage, its prompt's tier is unknown, and the common priced spend is unavailable, or a lower bound under 8.2's rule for a retried failure. A response that failed unsent reached no provider, and costs nothing (8.2). The harness keeps no catalog of its own |
| outcome | the end; the exit, a code or a signal; whether the harness forced it (no, terminated, killed); each check's verdict and evidence; protected files changed; writes outside the workspace |
| health | smith's loss per sink; headroom against each limit, as the fraction of its bound used; the passive checks |
| artifacts | each raw file's path and digest |

Every measurement is a `Measure`: observed, a lower bound, or unavailable
with its reason, never written as zero and never filled from another
scope.

### 8.2 Tokens

One normalised record per scope: **fresh** (input not read from a cache,
cache writes included), **cache read**, **cache write**, **output**
(reasoning included) and **reasoning**, each where reported. Each
convention is a tag on the record, with a fixed mapping:

| Convention | Fresh is | Not reported apart |
|---|---|---|
| Codex | input less cached input; cache writes (`cache_write_input_tokens`) are within input, reasoning (`reasoning_output_tokens`) within output | nothing, as codex-cli 0.160.0 reports them; a version that omits a field has it unavailable, never zero |
| Claude Code | input plus cache writes | thinking, within output |
| smith's events (protocol/events.md, section 3.10) | `input_tokens` plus `cache_write_tokens` | whatever is null |

- **Once per response.** A response's usage counts once however many
  records repeat it: smith's by its `response`, the others' by message
  id. A retried attempt that was billed is a response of its own.
- **A retried failure** is a response that the provider failed before
  any block and smith retried, with none of its usage reported
  (protocol/events.md, section 3.5). Its `response.completed` has
  `outcome` `failed`, `blocks` 0, a failure whose `evidence` is
  `response`, and every usage count null. A later
  `response.started` of its conversation and `turn` has a higher
  `attempt`.
  - **Its usage is unknown, never zero.** A provider that answered with
    a failure may have read the prompt first. skein's failed call
    carries no usage, whatever the provider sent with it (skein's
    `llm.md`, section 2: a completion carries the token usage).
  - **Its scope's totals and the attempt's are lower bounds,** and each
    names it as a retried failure, apart from any other gap. They miss
    the failed request alone. Its prompt is its retry's (protocol/events.md,
    section 3.11), and the retry's usage counts that prompt.
  - **The common priced spend** (8.1) of an attempt whose totals miss
    only retried failures prices what was reported. It is a lower bound
    naming them. Any other gap in the tokens it prices leaves it
    unavailable.
  - **A failure whose `evidence` is `maybe_sent` is never one.** The
    provider never answered it: a deadline of smith's cut it, or the
    connection dropped, and the provider may have carried the request
    to its end. What it cost is open.
- **A response that failed unsent costs nothing.** A `response.completed`
  whose failure's `evidence` is `unsent` reached no provider: smith
  refused the request before any byte of it was sent, as the guard does
  past the line (protocol/llm.md, section 6), or the connection failed
  before it. Its usage is zero, observed, though its counts are null,
  and the common priced spend prices it at nothing, whether smith
  retried it or compacted after it. It is no retried failure, and
  leaves no gap.
- **Scopes:** the root conversation; each child, under its parent;
  compactions; helper models; the total. A total is observed only when
  every scope is; otherwise it is a lower bound naming what is missing,
  and unavailable when the root is.

### 8.3 Ends

| End | Meaning | smith's exit (shell.md, section 4.3) |
|---|---|---|
| `Completed` | answered and exited by itself | 0 |
| `Failed(reason)` | ended without an answer: the provider, the model, a tool, input needed, or another typed reason | 1; 5 for input needed |
| `Budget(which)` | stopped on its own limit: turns, time or spend | 4 |
| `Refused(setup)` | never reached inference: the seed, the warm-up, the configuration or the credentials | 2; 3 for credentials |
| `Timeout` | the harness's hard deadline ended the tree | 130 after the harness's signal, or killed |
| `HarnessError(what)` | the harness failed: an unknown persisted shape, a grader that crashed | |

A budget end is not a failure, and a setup refusal is not the agent's:
rates leave both out and report them beside. A task whose story is a
startup refusal (a `failures` variant) is judged by its checks: its
refusal is the agent's end, not `Refused(setup)`, and it made no provider
request. The grade is apart: a
completed attempt can fail its grader. A forced exit after an accepted
answer is a warning on a completed attempt.

## 9. Statistics

- **Probes.** A probe passes when every check passes, and reruns once on
  failure. Two failures of two are a regression; calibration makes one
  unlikely to be chance. One of two quarantines the probe: reported, and
  gating nothing until it passes calibration again.
- **Comparisons interleave in one session.** Each block holds one attempt
  of each arm, in an order drawn from the run's seed, so that drift,
  warming caches and shared rate limits fall on every arm alike. At
  least five attempts per arm: three can never reach significance.
- **A difference is reported** per task and metric when the 95%
  bootstrap interval of the ratio of the arms' medians, resampled with
  the run's seed, excludes 1, and the effect exceeds the task's minimum
  detectable effect: about 2.8 · CV · √(2/n) for n attempts per arm, from
  its committed variation. Otherwise the summary says none was detected,
  not that the arms are equal.
- **Every attempt counts.** Pass rates are counts with exact intervals; a
  median over successes alone is never reported without the rest. Only
  outcome, wall time, tokens by convention and spend on one basis are
  compared across agents. Cache-read volumes and cached shares are
  reported, never compared across agents as rates: an agent with a
  larger fixed prompt reads more from its cache.
- **Parity** with another agent is non-inferiority, not "none detected":
  its time and cost criteria hold when the upper bound of the 95%
  interval of smith's ratio to the other agent lies below each one's
  margin, and its outcome criterion when smith's count of passes reaches
  the lower bound of the exact 95% interval of the other agent's count,
  over as many attempts (section 16). It is the
  verdict of its own suite, never inferred from a comparison that
  detected nothing.
- **A repository task's large effect** is section 3.1's disjoint ranges,
  computed by the summary for every metric both arms observe.
- **Baselines** are committed summaries of a suite for one agent
  configuration and tier: each task's medians, each metric's coefficient
  of variation, n and minimum detectable effects, with the binary and the
  date. A run outside one by more than a task's minimum detectable effect
  raises a drift notice, which asks for a comparison in one session and
  is never a verdict. A baseline is retaken by commit after an intended
  change, a new model, a new task version or a new result version. A
  suite is summarised once, whole; each baseline is cut from that summary
  by one command naming one agent, configuration digest, model and
  effort.
- **Audits are sampled.** A summary draws a few attempts with its seed,
  plus every attempt whose checks disagree with its grader, for a person
  or a model to audit: did the agent do the task asked, without reading
  the grader or changing what it must not. The scope gate is automatic:
  protected files unchanged, nothing written outside the workspace, no
  read of the benchmark tree in what the agent shows of its calls.

## 10. Isolation

- **Credentials are logins of the harness's own,** each signed in once
  by the developer with its agent's own flow, never a person's own tool
  directory and never a copy of a refresh token: issuers rotate them,
  and a copy is spent by whichever holder refreshes first.
  - **smith** runs on a credentials directory of the harness's, signed
    in once with `smith login` as the live suite's is (testing.md, 2.3),
    and shared by attempts under each record's lock (shell.md, 7.3);
    each attempt keeps its own state directory. Its Anthropic models run
    on a key.
  - **Codex** gets a `CODEX_HOME` of the attempt's own. Its `auth.json`
    is written by the harness from a Codex login of the harness's own: a
    `CODEX_HOME` the developer signs in once with Codex's own flow, and
    that Codex alone refreshes, never the person's `~/.codex`. The
    attempt's file holds the access and identity tokens and the account,
    with the refresh token left empty. It is `0o600` in a `0o700`
    directory and removed when the tree has ended. Codex cannot refresh
    it, so it never rotates the harness's login. Codex runs from a
    private copy of its pinned release's whole package under the
    harness's state, never the user's installation; its pin names the
    executable's SHA-256, and setup refuses another.
  - **Claude Code** runs with a configuration directory of the
    harness's, signed in once with `claude auth login`, never a copy of
    a login, which would carry a person's subscription credential
    outside its program. The login is shared; each attempt's settings,
    instructions and MCP servers are its own, given by flags (section
    6). Attempts on one login run one at a time.

  An access token that expires before the attempt's hard deadline (its
  expiry read from its claims) is a setup refusal, like a lapsed login.
  The refusal names the command to run once, whose own use refreshes the
  login where it lives. Nothing in the harness writes to a person's own
  tool directories.
- **Configuration is pinned, not inherited.** An agent's home is the
  attempt's, so nothing of the user's configuration, instructions,
  skills, plugins or memories reaches it. The keys it is given come
  from the command line, and only those an attempt must own (5.4): MCP
  servers, trust, approvals, model, effort and the caches it may write.
  smith's settings come only from the harness's layer (shell.md, section
  5).
- **Out of sight, nothing shared.** Attempts run under the harness's
  scratch root, outside every repository and the benchmark tree, each
  with its own temporary directory, build caches and target directory,
  warmed outside the timer and copied by reflink where the filesystem
  allows. A grader reaches the workspace's copy only after the tree has
  ended. Until contained trees exist nothing keeps an agent from the
  user's files; the scope gate looks for it, and lists the user's entries
  under `/tmp` before and after each attempt: a new or changed one is a
  write outside the workspace.
- **Process trees.** An attempt ends when its whole tree has ended. At
  the hard deadline the tree is terminated, then killed after a grace;
  after an observed answer, the deadline is the agent's teardown bound
  (smith's from its effective limits, read by `smith check --json exec`
  with the attempt's flags;
  another agent's from its pinned configuration)
  plus that grace. Accounting of a whole tree (a cgroup scope, its CPU
  and peak memory) is skein's to add first; until then each attempt runs
  in its own process group, and its resources carry the narrower scope.
- **Setup is not the agent's.** A seed digest that differs, a failed
  warm-up, a refused configuration or missing credentials end the attempt
  as `Refused(setup)`, and mark the suite incomplete.
- **Load and disk.** A grader sensitive to load runs serially on an idle
  machine. Raw output is bounded per attempt, and a suite checks free
  space against its estimate before it starts.

## 11. The initial catalogue

Costs are estimates on the small tier until measured.

### 11.1 Probes

Every probe's outcome is its file, or the answer it asks for. The outcome
halves of `smoke-tools` and `large-write` also run on Codex and Claude
Code.

| Probe | Guards | Forces | Checks | Cost |
|---|---|---|---|---|
| `first-request` | a start's messages, the opening, awaiting (domain/run.md, sections 3 and 6) | an empty brief and a one-line task that writes a file; headless with the task in the start, and interactive typed after startup; with and without a workspace | under `everything`, the first request's new messages hold the task; no response or `wait` before the task's `message.received`; at most two completions; time to the first `response.started` | seconds |
| `smoke-tools` | the commands' environment; one path namespace, the directory as `.`; a complete stream; teardown; exit evidence (domain/tools.md, sections 2 and 5; protocol/events.md, section 5) | list `.`, read the README, `printenv` a nonce variable and `command -v cargo` into a file through the shell, answer with a README token. `child`: one child first, so two pooled connections are open at the answer | `.` listed; one shell call, exit 0; tool records paired by `call`; the responses' `spent` and `usage` sum to `run.completed`'s; nothing lost; `teardown_ms` within bound, not forced | ten seconds, ten thousand tokens |
| `follow-up` | message terminals, the fence, the unread report, a finish crossing a message, deferred `wait` (domain/run.md, section 6); the local host's unread lines (domain/host.md, section 8) | interactive: a greeting, then, once `finish` shows, a file asked for through the shell. `eof`: two lines, then end of input. `chat`: three exchanges with wait authority | exit 0; each `message.received` has one terminal; the request handled once, in the run it crossed or the next one's opening; in `chat`, one completion per exchange. When the local host shows "waiting" has no record in the stream, so the local worlds check it (domain/host.md, section 8), not the probe | thirty seconds |
| `large-write` | the declared tool payload, per-call outcomes, each model's `output`, the truncated Anthropic `tool_use` (domain/tools.md, section 3; protocol/limits.md, section 6) | one `write` of tens of kilobytes within the payload: a seed file of non-repetitive plain prose, a little larger than the size asked, copied, so that the content has an obvious end. A model asked to count out a repeated character neither counts nor stops reliably. What is judged is smith: the saved file equals what the call sent. How faithfully the model copied is recorded, not judged. Where the small tier will not copy reliably, the content may instead be numbered lines the prompt specifies (`Line 0001: <a fixed sentence>`, and so on, to a stated last number): no read, and an end the model can see. `oversized`: a call past the bound on one call's input, which arrives as an oversize block (protocol/llm.md, section 4). With the production payload that bound, six payloads and more, is past what one response can write, so the variant declares a small payload, such as 2 KiB, and the bound follows from it. Its prompt says the first call is meant to be refused: one `write` of numbered lines past the call's bound; then a smaller file, past the payload, in pieces: a `write`, then an `edit` that extends it | one `write` of at least the size asked, `written`; no `max_tokens` stop. `oversized`: one call past the call's bound, as an `oversized` block, then the file in pieces, accepted. A refusal by the tool's payload instead (a `write` past the payload, within the call's bound) is not this variant's: domain/tools.md section 4's size refusal is proven by its deterministic test. An attempt that makes no call past the bound, and otherwise makes the file in pieces, accepted, is "not exercised (bound honoured)": neither a pass nor a fail, since a model honouring a bound smith states is what smith asks of it. The variant is proven by three exercised passes within forty-eight attempts (the plan's proving rules) | under a minute, ten thousand output tokens |
| `history` | the window, compaction, derived limits, selective decoding (domain/session.md, section 8; protocol/limits.md, section 3) | sum values held in many small files, read one call at a time, the values out of order so that a summary must carry them, and the answer's form stated whole in the prompt. The running sum and the next file are written as text beside each read, so that what a summary must carry is in the window's history, not only in the model's reasoning. `small-window`: a window several times smaller than the reading, with a compaction due just before the last read | `request_bytes` and prompt tokens per request recorded. Each read's verdict is smith's; the number of reads is the model's (`call-count`, 3.2). Each compaction's request holds the results owed to it, and the next window opens with its summary (`compaction-carried`). `small-window`: a `conversation.opened` of kind `compaction`; every compaction request at tool choice `none` (`compaction-choice`); no prompt above the window; cache reuse around it recorded. With those passing, a summary that drops or misstates a read it was given, so that the read is made again or skipped, is a model deviation. The compaction's condition is a window past its line: the window less one output, the summary instruction and the markers, which the stream gives as the conversation's model's `window_tokens` less its `output_tokens`, from `conversation.opened`, less `run.compaction_material`, from the effective limits `session.started` carries (protocol/limits.md, sections 3.9 and 9). A request has passed it when its prompt, as its usage counts it, is above the line. A stream without `run.compaction_material` gives no line, and leaves `conversation-opened` and `compaction-choice` "not exercised", naming why. The guard (domain/session.md, section 8.1) sends no ordinary request whose prompt may pass the line, so smith makes no ordinary completion past that point: it compacts first. A prompt between the threshold and the line is smith's estimate missing, and is no failure. An attempt that opened no compaction, and none of whose requests reached that point, leaves `conversation-opened` and `compaction-choice` "not exercised" (3.2): no compaction was owed, since the model stopped before its reading filled the window. An attempt with a request past that point and no compaction fails `conversation-opened`, which is smith's | two minutes, up to a million input tokens; weekly |
| `cache-reuse` | affinity, the stable prefix, Anthropic breakpoints, usage (protocol/llm.md, section 2) | pointer chasing over a dozen files, one read each, a fixed number of requests. `children`: two children in turn. `crossing`: a message part way, at medium effort, so reasoning crosses a user turn. `idle-gap`: a gap past the cache's lifetime before a message | reuse R, the cache read over the previous request's prompt, summed where the predecessor can be cached: median of three at least 0.85, and full misses (under half) at most one per ten requests over the three, until calibration sets them; the three repetitions are one attempt (11.5's `probes` suite). On `anthropic`, cache writes about each turn's addition. A control arm without affinity falls clearly below: one attempt of it fails these criteria, which makes it the probe's before (section 4.2) | a minute each; `idle-gap` weekly |
| `budget` | the reserve, wind-down, the final turn, budget text, the clamped share, real spend (domain/run.md, sections 5.3 and 9) | `starvation`: a tight budget and a delegated survey. `wrap-up`: a child given `max_turns` 3. `child-quota`: a child given `max_turns` 2. `visibility`: a report of what is left. `spend`: notional prices, a ceiling about twice a calibrated spend | `starvation`: accepted, main's last completion after the child's return, turns within budget. `wrap-up`: under `everything`, `stop: budget` and text, the last request at `tool_choice` `none`. `child-quota`: the child answers within two turns, its `share` in `conversation.opened`. `visibility`: main reports what is left as it was last told before the report (with a child, the last child's usage line, domain/run.md 9.2). The told turns and time equal the `budget` record of the step that wrote them, time in whole seconds rounded down; a mismatch there is smith's. The report repeats the told values; a mismatch there is the model's. No child opens into the reserve. `spend`: never past the ceiling | under a minute each |
| `parallel-read` | calls per response and the reads run at once (protocol/limits.md, sections 2.2 and 2.3) | twenty files read in one step | the reads in one response's calls; no `limit` failure; no failed send | thirty seconds |
| `parallel-children` | concurrent conversations, a pool at capacity waits, the write lease (domain/run.md, section 5.3; protocol/limits.md, section 2.2) | four read-only children in parallel, one directory each. `reviews`: three independent reviews | two children overlapping in time; no call refused by the pool | a minute |
| `failures` | typed failures in words, headless exit codes, limit failures and HTTP status, clipped provider text (domain/host.md, section 8; shell.md, section 4.3) | `unknown-model`; `derived-limit`, by a test profile; `one-turn`, a budget of one turn, which cannot hold the finishing reserve; `bad-setting`, a malformed setting | a run's failure in `run.completed`'s typed failure: the class, and the HTTP status and clipped provider text, or the limit and bound. `unknown-model` declares the model's `window` and `output`, so that its request reaches the provider. `one-turn` is refused at the entrance (domain/run.md, 9.1): `run.completed` `refused` with `invalid` naming the budget's reserve, and no provider request. `bad-setting` and `derived-limit` are refused at startup, in either mode (shell.md, 4.1): exit 2, standard error naming the setting, or the limit and bound, no provider request, and no run, so no `run.completed`. Headless only (shell.md, 4.3): the exit code of the class and standard error's last line, judged from 10.5's `smith exec`, "not yet: 10.5" until then. A run that runs out of budget once admitted is the budget probes' | seconds |
| `paths` | one path namespace (domain/tools.md, section 2) | `app` writable and `spec` read-only: read both, answer in `app`. `single`: one directory, its name never needed. The prompt names the files as any agent finds them on disk, from the working directory `app`: `spec`'s through `../spec/` (domain/tools.md, section 2: siblings by path). smith's own text tells its model `/spec/...`, and either route must resolve. Every agent runs in `app`, with `spec` beside it, read-only and protected | no `missing` verdict or path refused as outside; at most two completions before `finish` | seconds |
| `tool-output` | output budgets, read windows and timeouts as the model chooses them; UTF-8 passed through; jobs (domain/tools.md, sections 3 and 5) | `test-output`: which assertion fails, in over ten kilobytes of output. `quote`: a sentence deep in a long document. `timeout`: a command past its timeout, and one asking past the ceiling. `utf-8`: replace an en dash. `background`: a command of about a minute started as a job, beside reads the answer also needs | one shell call; at most two reads; `timed_out` naming its bound, `clamped` set, no hang; one edit, matching first time. `background`: the command answered `running`, a completion started between the job's start and its end, one terminal for the job | seconds each; `background` a minute |
| `process-tree` | a command in its own process group, signalled whole (domain/tools.md, section 5) | `sh -c 'sleep 600 & wait'` with a short timeout | no `sleep` left, by tree accounting; answered within timeout and grace; accepted | seconds |
| `credentials` | a sign-in record shared across processes (shell.md, 7.3) | two smith attempts at once on one `sign_in` account of the harness's, the refresh lead raised so that both refresh | both accepted; the record's generation rose, and no refresh was refused as reused | seconds |
| `slow-completion` | deadlines that measure progress; an oversized reasoning item as a typed failure (protocol/limits.md, section 7) | long reasoning at high effort, from a real provider (a fake slow source is the deterministic worlds', not this probe's) | the answer accepted; no timeout of any phase while events arrive; no reasoning-size failure; the call lasting over 60 s, the whole-response cap the spike's attempts were cut at (findings S5). An attempt under 60 s is "not exercised": neither a pass nor a fail, counted like a model deviation (more than two in twelve strengthen the prompt, or raise the effort). Recorded per provider and model, for shell.md's `head`, `idle` and `reasoning_item`: `first_byte_ms`, `largest_gap_ms`, the call's length and the largest reasoning item | minutes; occasional |

Later, as their behaviour is built: **`mid-work`**, a message offered
between completions of long work, and threshold notes the same way;
**`build-environment`**, a seed with an ambient build wrapper that builds
and tests, once trees declare build profiles.

### 11.2 Passive checks

On every smith attempt:

- one `session.started`, one `run.completed` per run, one
  `session.ended`, and the exit agreeing with the answer;
- no authoritative record lost, each sink's `loss` counted, none under
  `complete`;
- `teardown_ms` within the agent's bound and not forced; a forced exit
  after an accepted answer is a warning;
- every `response.started` and `tool.started` completed, a tool's under
  its `call`; every `conversation.opened` closed; every
  `message.received` with exactly one terminal;
- the responses' `spent` summing to the run's; no failure of class
  `limit` the task does not expect;
- headroom, with a warning at half: `request_bytes`; prompt tokens
  against the window; `first_byte_ms` against `head`; `largest_gap_ms`
  against `idle`; turns; spend; `peak_rss_bytes` against `memory`.

On every attempt of any agent: the tree ended; protected files
unchanged; nothing written outside the workspace and the private
directories; no read of the benchmark tree seen.

### 11.3 Experiments

| Experiment | Over | Arms |
|---|---|---|
| `affinity` | `cache-reuse` | the cache key and session header; plus the thread header; plus private routing state and reasoning context, adopted only on a material gain |
| `cache-lifetime` | `cache-reuse/idle-gap` | Anthropic's default lifetime, and a longer one |
| `compaction` | `history/small-window` | the threshold, and what the new window holds |
| `delegation` | `shell-cwd` | each agent with its sub-agents off; both with them on |
| `edit-format` | `03-planner`: "add a doc comment to each public function and run the tests" | each dialect's formats, by completions before the first edit and edits that fail |

### 11.4 Fixtures and the repository task

| Task | Asks for | Grader | Runs |
|---|---|---|---|
| `01-backoff` | overflow-safe delay and deadline arithmetic in a small crate | hidden edge-case and regression tests; formatting | by default |
| `02-line-decoder` | an incremental line decoder with a bounded buffer | hidden tests; fixed-seed comparisons with an oracle | by default |
| `03-planner` | a closure planner with ordered topological output and error priority, across files | hidden tests; an oracle over generated graphs | occasionally |
| `shell-cwd` | a working-directory option for the shell tool, through protocol, domain, machine and documentation, on smith at a frozen commit with its skein pin vendored | an oracle crate over that commit's interfaces; formatting, lints, both suites within budget, serially on an idle machine | at milestones |

A fixture takes minutes per attempt at the working tier; `shell-cwd`
tens of minutes and millions of tokens (estimates). Where a correct
solution could fail a grader, or a wrong one pass it, the grader is
strengthened first. Each grader holds a copy of the seed's own tests and
runs them against the agent's work, so that a weakened test cannot hide a
regression; a task's references add the tests its prompt asks for. A
prompt asks for nothing a pinned agent as shipped cannot do, and invites
no delegation: delegation is its own experiment (11.3). Per change, `budget` and `history` stand in for the
repository task's delegation and length.

### 11.5 Suites

| Suite | Tasks | Agents | Repetitions |
|---|---|---|---|
| `smoke` | `first-request`, `smoke-tools`, `large-write`, `budget/child-quota`, `budget/wrap-up`, `failures/one-turn`, each only while its task file holds a calibration | smith on `openai`, small | one, rerun on failure |
| `probes` | every probe and variant | smith on both providers; Codex, later Claude Code, on outcome halves | one; three where judged on a median |
| `compare` | `01-backoff`, `02-line-decoder`; `03-planner` when asked | smith and Codex, later Claude Code, working | at least five per arm, interleaved |
| `repository` | `shell-cwd` | smith and Codex, working | three, outcomes and large effects only |
| `parity` | `01-backoff`, `02-line-decoder`, `03-planner`, `shell-cwd` | smith and Codex, working, each as it ships | eight per arm for each fixture and ten per arm for `shell-cwd`, interleaved, no rerun (section 16) |
| `parity-change` | the `parity` suite's tasks | two smith arms, working: a change and its parent | five per arm for each task, interleaved, no rerun (16.6) |
| `calibrate` | one probe | smith, the probe's tier | ten passes within twelve on the fix; then the binary before, or a control arm; writes the variant's `[calibration]` block (5.2) |

## 12. How it relates

- **To smith's tests.** Benchmarks find and show; tests specify and gate.
  A benchmark does not replay and its outcome varies, so what it finds is
  reproduced in the lowest tier that shows it (testing-strategy.md,
  section 9) and fixed there with that test; the probe stays as evidence
  of the effect. Benchmarks never stand in for a world's story.
- **To the live tier** (testing.md, section 2.3). The live tier checks
  once, by outcome, that each wire, issuer refresh and push is right.
  Benchmarks repeat, measure and compare. Both read smith through
  `smith-events`, take logins of their own, signed in once, under one
  credential guard, and default to
  the small tier.
- **To skein's test kit.** The harness is not a world: no schedule,
  replay, counting allocator or referee plumbing of its own. What it
  needs that is generic to processes, such as a tree's settlement and
  resource accounting, lands in skein first, smith-bench its first user.
- **To the product.** It runs smith as a person would (shell.md, section
  4); a gap it meets there is the product's to close, never the
  harness's to work around.

## 13. Adding to it

| To add | Where |
|---|---|
| an agent | a variant of `Agent` and its module; its pinned configurations; its recorded outputs and their offline tests |
| a provider | a variant; its token convention; an entry per tier in `agents/models.toml` |
| a task | a directory under its kind, calibrated if a probe, validated if graded |
| a task kind | a variant with its judgement and defaults, and a directory under `tasks/` |
| a check | a variant of the outcome or event checks, with its offline test |
| a metric | a `Measure` in the result, with its scope, and a new result version |
| a tier or a schedule | an entry in `agents/models.toml`; a suite file |

## 14. The world

The offline tier runs in the focused suite, with no agent and no network:

- **Parsers on recorded outputs,** per agent and version: smith's are the
  event codec's goldens (protocol/events.md, section 8); Codex's are JSON
  lines and rollouts with root-only usage, an unlinked child and repeated
  usage; Claude Code's are streams with repeated message ids, sub-agent
  messages and helper models. Unknown records are skipped, a known record
  in a new shape is a harness error, usage counts once per response, and
  unavailable is never zero.
- **Classification:** one recorded end of each class per agent, and each
  of smith's exit codes.
- **Manifests:** every task, suite and configuration parses; seed digests
  match; a `[calibration]` block whose digests differ from its variant's
  prompt, seed or checks gates nothing (3.1); prompts are whole; every
  check is known; every guarded section
  exists in the design; no seed holds `benchmarks/` or a grader's or
  reference's file; suites name existing tasks and agents.
- **Statistics:** the bootstrap, the interleaving and the audit sample
  reproduce from their seeds; the rerun rule; the minimum detectable
  effect; the large effect's disjoint ranges.
- **Parity:** the verdict recomputes from a committed summary; the
  stratified bootstrap keeps each task's size in every draw; an arm
  without a pass; a ratio's upper bound just below and just above its
  margin; the reference chosen by agent; each end of 16.3.
- **Pricing:** recorded attempts whose prompts stay below a tier's size
  and pass it, each response priced at its own prompt's tier, whatever
  the declared window; a response without usage leaving the spend
  unavailable, a retried failure a lower bound, and a response failed
  unsent, retried or followed by a compaction, priced at nothing, the
  spend observed (8.1, 8.2).
- **A change's report:** a recorded `parity-change` summary gives each
  criterion's ratio of the change to its parent, the parent chosen by
  its commit, and its projection on a recorded baseline; a projection
  just below and just above the baseline's upper bound, and passes just
  within and past O's bound, stop it or not (16.6).
- **An attempt against a replaying agent:** a small program that prints a
  recorded stream, leaves a grandchild behind, or never ends, observed,
  ended at the deadline and classified.
- **Fuzzy:** parsers fed truncated and corrupted lines never panic and
  never count a response twice.

Each runs in milliseconds, within the focused and fuzzy budgets.

## 15. Open questions

- **Codex's later versions:** whether a version after 0.160.0 still
  links each child thread to its parent in its opening record, and still
  trusts the attempt root by `projects.<root>.trust_level`; a new pin
  checks both before its first comparison.
- **Claude Code:** the settings that allow edits and commands headless;
  whether `CLAUDE_CONFIG_DIR` keeps every user file out;
  `claude-haiku-5-5` at low effort on a subscription.
- **Claude Code's attempts at once on one login:** whether attempts that
  share a configuration directory interfere. Default: one at a time
  (section 10).
- **Tree accounting:** cgroup v2 delegated to the user on the development
  machine, as containment also needs (testing.md, section 8).
- **The shared credential guard:** which crate the live tier and
  smith-bench both take it from.
- **Repository bundles:** where a bundle is kept durably, so that another
  machine can run the task.
- **Concurrency:** attempts run serially; parallel ones would need
  disjoint CPUs.
- **Task kinds beyond code,** such as chat, research or review, with
  graders of their own; and **audits:** how many attempts a run samples,
  and who audits them.
- **Recording smoke runs:** whether a commit hook checks that a runtime
  change's body carries one.
- **A failed response's usage:** skein's failed call carries none, so
  usage a provider sends with its failure is told as null (8.2, a
  retried failure). Whether a failure should carry it is skein's
  `llm.md`'s question.

## 16. Parity with Codex

smith is **on par with Codex** at a commit when one run of the `parity`
suite on that commit's built arm meets every criterion of 16.2. Parity is
non-inferiority (section 9): on these tasks, smith is not worse than
Codex by more than each criterion's margin. It ranks nothing, and it
holds for one pinned Codex at one working tier.

### 16.1 The run

- **What runs.** The `parity` suite (11.5): the fixtures `01-backoff`,
  `02-line-decoder` and `03-planner`, eight attempts per arm each, and
  the repository task `shell-cwd`, ten attempts per arm: 68 attempts,
  interleaved in one session (section 9), with no rerun. Every attempt
  counts.
- **Why ten on `shell-cwd`.** RT and RC rest on that task alone, and on
  medians of its attempts. A verdict should turn on smith's time, not on
  the test's power: a smith as fast as Codex, or a little faster,
  should pass most runs, and one worse than the margin almost none. More
  attempts narrow the interval without moving the margin, so the second
  holds at any number; the plan's decisions record the power at five,
  ten and fifteen.
- **The arms.** smith's built arm of the commit, and Codex at its pinned
  version, each as it ships (5.4, 6.1, 6.2), at the working tier
  (section 7). Delegation is each agent's default; what delegation is
  worth is the `delegation` experiment's (11.3), not parity's.
- **The basis.** Outcome, task wall time and common priced spend (8.1),
  which both agents report on one basis, each response priced at its own
  prompt's tier. Tokens by field, cache reads, cached share, reasoning,
  responses, process wall, the time record, the agent's own peak
  resident memory and each fixture's own ratios are reported beside, and
  decide nothing.

### 16.2 The criteria

| # | Criterion | Statistic | Holds when |
|---|---|---|---|
| O | outcome | smith's passes over all 34 attempts per arm, pooled over the four tasks | at least the lower bound of Codex's exact 95% interval for its own pooled passes, times 34 (31 when Codex passes 34) |
| FT | the fixtures' time per solved task | each arm's total task wall over the three fixtures divided by its passes there; smith's over Codex's | the 95% interval's upper bound is below 1.20 |
| FC | the fixtures' cost per solved task | the same, with common priced spend | the upper bound is below 1.20 |
| RT | `shell-cwd`'s time | the ratio of the arms' median task walls | the upper bound is below 1.20 |
| RC | `shell-cwd`'s cost | the ratio of the arms' median common priced spend | the upper bound is below 1.50 |

- **The intervals** are percentile bootstraps of 10,000 resamples,
  seeded by the run's seed. FT and FC resample attempts within each task
  and arm, so that each draw keeps the tasks' sizes. A draw in which an
  arm has no pass counts the ratio as infinite. RT and RC take the
  bootstrap of section 9's comparisons, at ten attempts per arm.
- **A pass** is an attempt whose deciding checks all pass (3.2): its end,
  its grader, its protected files, its tree's end.
- **Why per solved task.** Every attempt's time and spend count, a
  failure's included, so a fast failure never looks cheap: the time and
  cost of getting the fixtures done.

### 16.3 Ends and missing values

- **Unsolved:** a `Completed` attempt whose grader or deciding check
  fails, `Failed`, `Timeout` and `Budget` (8.3). Their task wall and
  spend stay in the sums.
- **Incomplete:** a `Refused(setup)` or a `HarnessError` attempt; an
  unavailable task wall or common priced spend; a lower bound of either
  on smith's side, other than a spend that misses only retried failures
  (8.2); Codex without a pass on the fixtures or on `shell-cwd`, since
  such a task tells the agents nothing. A lower bound on Codex's side is
  taken as observed, which can only raise smith's ratio, and is flagged.
- **smith's retried failures are taken as observed,** and flagged: a
  spend that misses only retried failures counts as what was reported,
  and the attempt's row keeps the lower bound that names them.
  - The provider's failures are not the agent's. Both arms meet them,
    interleaved on one ChatGPT account, each on its own login.
  - Codex's tokens are its own running total, read as observed. The
    harness sees none of Codex's provider attempts, so whatever a Codex
    request that the provider failed and Codex retried added to that
    total is read as observed too.
  - Read the same way, smith's figure is what its provider reported to
    it, as Codex's is. Holding smith's failed requests to more would
    hold smith to an account that the harness cannot apply to Codex.
- **Not on par:** smith without a pass on the fixtures or on `shell-cwd`.

### 16.4 The verdict

- **`summarise`** writes a `parity` section into a `parity` run's
  summary: each criterion's statistic, interval, margin and verdict;
  the reference arm, chosen as the arm whose agent is Codex, never by the
  arms' names; the seed; and the commit whose presets priced the spend.
- **`smith-bench parity SUMMARY --commit SHA`** recomputes that section
  offline from the summary's per-attempt values (5.4). It refuses a
  section that disagrees with them, and a summary whose smith arm was not
  built from `SHA`. It prints one line per criterion, and exits 2 when
  the verdict is incomplete (16.3), whatever the criteria; otherwise 1
  when a criterion fails or 16.3 finds smith not on par, and 0 at
  parity. A run of `suite parity` exits with the same code.
- **A plan that targets parity** names this command in its "Done when",
  over a committed summary of a run on the plan's final `main`.
- **What moves it.** A new Codex pin, a new working tier, a new task
  version or a new result version starts a new comparison: parity is
  shown again, never carried across.

### 16.5 A verdict short of parity

- **A summary is committed whatever its verdict, 0 or 1.** A run
  measures its commit as it is. A verdict of 1 says where smith stands,
  and the runs that follow are read beside it. A run whose verdict is 2
  measured nothing whole: its summary is not committed, and its attempts
  are spent.
- **A smith arm is run once to a verdict of 0 or 1.** No run repeats an
  arm to pass (16.1, "with no rerun"). Each run of one smith is one more
  chance for a smith worse than a margin to pass it; one run per change
  keeps that chance to one per change.
- **A failed verdict is answered by a change, never by a run.** A later
  run is of a later commit whose smith arm differs, by a change whose
  commit body names:
  - the criteria it addresses;
  - their cause, read from the failed run's attempts: the measures
    reported beside (16.1), the counts and the traces, against Codex's in
    that run, or smith's in the last run where the criterion held;
  - the change's own proof (4.2).

  A change that names no cause earns no run. A change that serves the
  suite's tasks and no other work, such as instructions that name a
  task's files or commands, is not a cause's answer.
- **The newest run is the verdict.** Runs are never pooled, and none is
  chosen among them: 16.4's command reads the newest committed summary,
  and the earlier ones stay committed beside it.
- **A plan bounds its runs.** A plan that targets parity says in its live
  budget how many closing runs it holds. When the last it holds is short
  of parity, the plan ends with its target unmet: its "Done when" names
  the criteria that failed, its last summary is the evidence, and their
  answer is a later plan's, a change as above.
- **Baselines are cut from a plan's last `parity` run,** whatever its
  verdict (section 9): they alarm on the drift of the smith that ships,
  on par or not.

### 16.6 A change on the way to a verdict

A plan whose aim is a parity verdict runs a baseline first, the `parity`
run that measures where it starts (skein's development.md, 5.1). After
it, each step that changes what the model is sent shows its effect
before it lands: on every criterion, whatever criterion it claims to
move, by a run of its own when that run can tell its expected effect
from none; otherwise on the counts it targets, and the plan's closing
run measures it with the others.

- **What counts.** A change to a prompt smith sends, or to when it sends
  one:
  - the task role, and any text smith writes into the system text or a
    message: the summary instruction and its preamble included;
  - a tool's description or schema, or what a tool answers when a call
    asks for nothing: a read's window, a command's kept output, a job's
    wait;
  - the window's check, which decides when a window is compacted
    (domain/session.md, 8.1).

  A change that alters no prompt and no request's timing, such as the
  harness, the event stream or a refactor, is not one.
- **Measured alone, or with the others.** A run of two arms of smith
  resolves only an effect wider than its interval. The plan states each
  change run's resolution from its planning commands (a simulation at
  the run's attempts on the spread of the last committed baseline,
  `benchmarks/baselines/`), and each change's expected effect from its
  cause (16.5):
  - **a change its run can tell from none** runs `parity-change`, as
    below;
  - **any other** proves itself on the counts it targets, such as its
    task's responses (16.1), and lands. The closing run measures such
    changes together, and a criterion it misses has 16.5's answer.

  In the wall-clock plan, four change runs took 9.3 hours of the gate
  lane, a third of the run: their RT intervals were 0.34 to 0.78 wide,
  against effects of 4% to 22% of smith's time, and two stopped on
  noise. Their ratios multiplied to 1.083, while the closing run moved
  RT from 1.397 to 1.033, and the counts the steps targeted moved:
  responses on `shell-cwd`, 70.1 to 54.2.
- **Its proof is one run of `parity-change`** (11.5): the `parity`
  suite's tasks, five attempts per arm each, with two smith arms, the
  change and its parent, interleaved, with no rerun. It takes the place
  of the `compare` run of 4.2.
- **Its report** is what `summarise` writes for that run, and goes in
  the commit body; nothing else is asked for:
  - **each criterion of 16.2,** computed with the change in smith's
    place and its parent in Codex's: the change's ratio to its parent,
    with its interval, and, for O, each arm's passes. The reference arm
    is the one built from the change's parent, chosen by its commit;
  - **its projection:** the baseline's statistic times the change's
    ratio, beside the baseline's interval. `summarise` takes the
    baseline's committed summary as a named input. A product over the
    changes landed before it would count each of their codes twice, as a
    change and as the next one's parent: in the wall-clock plan, of
    step 10.1's RC projection of 1.821, a factor 1.485 was three codes
    measured twice, each costing more in its own run than as the next
    step's parent;
  - **beside them,** deciding nothing: each arm's responses an attempt,
    the time record's parts (requests, compaction, tools) and fresh
    tokens, per task.
- **A move the wrong way stops it.** The step stops on a question to
  the designer (skein's development.md, 10.4) when a criterion moves the wrong
  way past the baseline's interval:
  - **FT, FC, RT or RC:** its projection lies above the baseline's
    upper bound;
  - **O:** the change's passes fall below the lower bound of the exact
    95% interval of its parent's passes, over as many attempts.

  This holds whatever the step's own comparison concludes, "none
  detected" included. A report that is incomplete, a criterion
  unavailable or an arm with no pass on a task, stops the step the same
  way. The step lands only on the designer's answer: kept, with the
  cause the answer names for the move; reworked; or dropped.

  The cause may be noise rather than the change. The answer then shows,
  from the run's attempts, that the move lies in work the change does
  not alter, such as the reads after a compaction that more of one
  arm's attempts reached. Where an earlier `parity-change` run measured
  the parent's code, the answer gives that run's figures beside. An
  interval that holds 1 is no such showing: at five attempts, a
  comparison cannot show that a change does no harm. The projection
  keeps the ratio as measured.
- **Why the baseline's interval.** A comparison that detects nothing
  at five attempts per arm can hide a move larger than the baseline's
  own uncertainty; the projection sets the change's move on the
  baseline, where the plan can see it. A plan's position within the
  baseline's interval is within what it already measures as noise, and
  goes on. What the landed changes add up to, the moves no change run
  resolves included, is the closing run's to measure.
- **The baseline** here is the plan's first `parity` run, its
  committed summary, never `baselines/`'s files, which are cut from a
  plan's last run (16.5).
- **One run per commit.** A step reworked after a stop runs
  `parity-change` again, on its new commit. A `parity-change` run is a
  step's evidence for landing, never a verdict: the plan's `parity` runs
  judge it (16.5), and its runs are never pooled with them.
