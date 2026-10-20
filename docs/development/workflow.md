# Development workflow

How work reaches main. Work happens on branches; main moves only by
landing a branch that passed every check below, or that changes only
documentation, with `land` (skein's `docs/foundation/development.md`,
section 9).

For the product, see [local-host settings and commands](local-host.md).
The root package builds `src/main.rs`; its end-to-end and live targets live
under `tests/end-to-end/`. Bare cargo commands include every workspace member.
Run the workspace chat with `cargo run --`, or one headless activation with
`cargo run -- exec PROMPT`. Install the binaries with `cargo install --path .`.

## 1. Before merging to main

`scripts/check.sh` is the gate. It runs these four checks, in this order:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo nextest run --workspace --profile fuzzy
```

`land` runs it through `heavy --gate` on the branch's tip, rebased on
`origin/main`, so that what is checked is what main becomes, then pushes
the tip to `origin`'s `main` (development.md, section 9.1).

A branch that changes only Markdown files (the documents under `docs/`,
`AGENTS.md`) skips the four checks, since nothing it touches is built or
tested, and lands with `land --docs`. Doc comments in `.rs` files are not
documentation in this sense: they are code, and clippy checks them.

Dependencies on skein use `https://github.com/fabrig-dev/skein.git` without
`rev`, `branch` or `tag`; `Cargo.lock` pins the revision. smith, jig and
temper must resolve the same skein revision when composed. Changes to
shared mechanisms land in skein first; smith and its consumers then take
that revision through their own gates.

## 2. The two test suites

The suites are those of skein's `docs/foundation/testing-strategy.md`
(section 8). smith's domains and their world stories are specified in
`docs/design/domain/`.

- **The default suite** is every crate's unit tests and each world's
  focused tests (`tests/<world>/tests/*.rs`): focused tests of
  expected behaviour (scenarios, referee tests, replay, facts changing
  nothing, memory against the worst case), and a cheap random world as a
  smoke test where one helps. It is what `cargo nextest run --workspace`
  runs, and it takes at most **15 seconds**.
- **The fuzzy suite** is the worlds' `tests/fuzzy_*.rs`: randomized tests,
  such as sweeps of random worlds and domains driven at random, over many
  seeds. It is not run by default, only with `--profile fuzzy`, as the
  gate before merging to main, and it takes at most **1 minute**.

The budgets are enforced, not advisory: `.config/nextest.toml` gives each
profile a `global-timeout`, so a suite that runs past its budget fails,
naming the tests it stopped, and a slow period past which a test is
flagged SLOW as it runs. A change that breaks a budget is fixed by making
tests cheaper, or by moving randomized ones to a world's fuzzy tests.
Raising a budget is a decision to take explicitly, not a fix. To see what
each test costs on its own, the `measure` profile has no budget:

```sh
NEXTEST_DOUBLE_SPAWN=0 cargo nextest run --workspace --profile measure -j 1
NEXTEST_DOUBLE_SPAWN=0 cargo nextest run --workspace --profile measure -j 1 --ignore-default-filter -E 'binary(/^fuzzy_/)'
```

The budgets are wall time on the slowest development machine, carbon-forge
(an i5-8265U, 4 cores, 8 threads, which nextest uses all of), with nothing
else building. Run the checks on an idle machine.

## 3. When a fuzzy seed fails

A failure names its seed, and a seed replays to the same run. Fix the bug,
then keep its seed: as a scenario among its world's focused tests if it
shows behaviour worth naming, or among the sweep's pinned seeds. A finding
that cannot be fixed yet joins its world's `FINDINGS`, which an ignored
test replays until it is fixed.

## 4. The live suite

The live suite runs smith on its own accounts as
[testing.md, section 2.3](../design/testing.md#23-end-to-end) specifies.
Follow the [README's live steps](../../README.md#live-tests) once per
machine to prepare and sign in `$SMITH_LIVE`, then run:

```sh
cargo nextest run --profile live
```

The profile runs the account stories and real Claude Code probes one at
a time, without retries; the default and fuzzy suites exclude them.
`SMITH_TEST_LIVE_GIT_REMOTE` optionally names a writable remote for the
push story, which removes its branch afterwards. With no remote named,
that story reports the omission on stderr and passes.

The account stories are `chatgpt_delivers_a_commit_in_place`,
`claude_delivers_a_commit_in_place`,
`an_anthropic_key_delivers_a_commit_in_place`,
`a_command_reads_the_whole_configured_environment`,
`a_second_run_resumes_the_chat_from_its_files`,
`a_chat_crosses_from_chatgpt_to_claude_then_back` and
`a_configured_push_lands`. The program probes are
`program::a_live_interrupt_ends_the_turn`,
`program::a_live_resume_continues` and
`program::the_real_program_uses_the_prompt_tool_shape`. They check outcomes
and wire shapes; the offline channel and configuration controls remain in
the default suite. Use nextest's test filter to select a single story.

## 5. Benchmarks

Benchmarks run real agents against real providers, and are designed in
`docs/design/benchmarks.md`. Section 1's gate stays deterministic: none of
its checks runs an agent. **A change is proven before it merges and before
it is complete,** by the kind of proof its kind of change needs:

- **A plain defect** (a crash, a wrong answer, a race, a limit that refuses
  valid input) is proven by a deterministic test that fails on the parent
  and passes on the fix. The gate runs it.
- **An improvement** (prompt caching, budgets, deadlines, tool round trips,
  limits sized for real work) is proven by its benchmark probe, run before
  the merge:
  - it passes on the change;
  - where the parent can run the probe live, it fails on the parent;
  - where the probe compares agents, Codex runs it too.

  Tests show that a mechanism works. Only a real model shows that it helps.
  A fix without a probe adds one in the same change.
- **An improvement whose effect does not depend on a model** (fewer loop
  iterations, allocations or copies, a cheaper read, with what the model is
  sent and returns unchanged) is proven by a deterministic test that counts
  what it saves (iterations, deliveries, bytes) and fails on the parent.
  The gate runs it, and `smoke-tools` runs as for every landing. No probe is
  wanted: a probe sees only what a model does, and here the model's side
  does not change.
- **Every smith landing that changes code runs `smoke-tools` live,**
  with the cheapest model and effort, whatever the change touched: once,
  through `heavy --gate`, on the branch rebased on `origin/main` just
  before `land`, and once more only if it fails (`benchmarks.md`, section
  11.5). A rebase inside `land` does not run it again. A Markdown-only
  landing skips it. It takes about ten seconds and ten thousand tokens. A
  red smoke stops the line: nothing else lands until it is green again.

The commit body records the proof: the test's name, or the probe's run
identifiers and verdicts, and the smoke's, with the binary it ran. Evidence
is never "pending" at merge (`docs/design/benchmarks.md`, section 4.2). A
skein change is proven by its tests, and then by its smith consumer's smoke
when smith pins it.
