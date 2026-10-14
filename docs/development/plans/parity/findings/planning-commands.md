# Planning: the commands run at the base

2026-10-14. Each command the plan prescribes that can run before its
sessions land, run once by the planner through `heavy` (development.md,
5.4), at smith `b7da2b25` (code; `72311b8e` adds Markdown only) and skein
`63fff2d`, in the scratch worktrees `~/src/rust/tmp/design/smith-parity-scratch`
and `skein-parity-scratch`. Each entry: the command, its exit status, its
last lines (at most 5), and what the plan changed because of it. A command
that names code an earlier session lands is the supervisor's ready check's
(development.md 7.2), and is listed as such.

## Run, and what they printed

1. `heavy cargo nextest run -p smith-bench` (03's Done when): exit 0.
   `Summary [1.030s] 152 tests run: 152 passed, 0 skipped`.
2. `heavy cargo run -q -p smith-bench -- check` (01, 02, 03.2, 04.3):
   exit 0. `checked 19 tasks, 6 suites, 9 configuration pins, 42
   summaries, 11 baselines`.
3. `heavy cargo run -q -p smith-bench -- validate fixtures/01-backoff
   fixtures/02-line-decoder fixtures/03-planner repository/shell-cwd`
   (01.1's and 01.2's Proofs, 01's Done when): exit 0, 116 s.
   `fixtures/01-backoff seed=failed reference=passed`, the same for the
   other three.
4. `heavy cargo nextest run --workspace -E 'rdeps(smith-bench) or
   rdeps(smith-charter)'` (README rule 6's form): exit 0. `Summary
   [3.668s] 1409 tests run: 1409 passed, 0 skipped`.
5. The interface sketches (03, 04, 08, 09, 10), as a scratch crate
   `sketches/` against `smith-domain-tools` and `smith-events`: `heavy
   cargo clippy -p parity-sketches -- -D warnings`: exit 0. `Finished
   dev profile`.
6. `rg -L '^deciding' benchmarks/tasks -g task.toml` (01.3, as first
   written): exit 0, but it printed every `deciding` line: `-L` is rg's
   `--follow`. Fixed to `rg --files-without-match '^deciding'
   benchmarks/tasks -g task.toml`: exit 0, it lists the four graded tasks
   only, so 01.3 now needs 01.1 and 01.2.
7. `heavy env XDG_CONFIG_HOME=… XDG_STATE_HOME=… cargo run -q --bin smith
   -- check --json exec -m codex/gpt-6-luna --effort low .` (05.1's Proof,
   as first written, with luna for sol): exit 2. `smith: built-in
   settings: accounts.codex.source: required value is absent`. Fixed:
   05.1's Proof is its unit tests, findings L1 its live evidence. (With
   `XDG_CONFIG_HOME` set on `heavy` itself, pueue found no configuration:
   such a variable goes on the command inside `heavy`.)
8. `heavy cargo run -q -p skein-codegen -- …` (11.1, as first written):
   exit 101, `package(s) skein-codegen not found in workspace`. Fixed to
   the Cargo.lock-pinned checkout: `heavy cargo run -q --manifest-path
   ~/.cargo/git/checkouts/skein-336d5ae5eecfa0f9/74b7fda/testing/skein-codegen/Cargo.toml
   -- crates/smith-transcript/schema/transcript-v3.schema <scratch dir>`:
   exit 0, and its `v3.rs` equals `crates/smith-transcript/src/generated/v3.rs`
   (`cmp`).
9. README "Done when" item 4 (the calibration count): exit 1, printed
   `0` (only `credentials` holds a block today). On a file with a
   top-level block and a `small-window` variant's block, its `awk`
   printed `history` and `history/small-window`.
10. README "Done when" item 2: `ls benchmarks/summaries/parity-*.json`
    finds none; the `parity` command does not exist yet (04.4). The
    ready check's.
11. README "Done when" item 5, as first written: `git log origin/main
    --format='%(trailers:key=Gate-Result,valueonly)' | grep -m1 .`
    printed `none (Markdown only)` in both repositories, a Markdown
    landing's. Fixed with `grep -v '^none'`: smith `scripts/check.sh
    passed: 1995 tests / 10.089 s; 41 tests / 14.361 s; 101 s in all`;
    skein `scripts/check.sh passed: 2105 tests / 3.912 s; 99 tests /
    31.569 s; 6 tests / 3.310 s; 42 s in all`.
12. README rule 10's total: exit 0, `0 attempts, $0.00`.
13. README rule 7's Pulse-word check, on the plan's branch: exit 1,
    nothing printed (the branch changes Markdown only).
14. skein 01's Done when, `heavy cargo nextest run -p
    skein-fake-llm-domain`: exit 0. `Summary [0.012s] 15 tests run: 15
    passed, 0 skipped`.
15. 02's Done when, `rg -n 'first-request.json|smoke.json|LocalTrace|multi_agent|max_threads'
    benchmarks | wc -l`: 21 (the files 02 removes or changes).
16. 08's Done when, `rg -n 'min\(8\)|\.min\(4\)' …`: 4 lines, the four
    literals 08.2 and 08.3 replace (`smith-protocol-machine/src/limits.rs:167`,
    `smith-agent-service/src/profile.rs:521`, `smith-domain-session/src/limits.rs:286`,
    `smith-domain-tools/src/limits.rs:247`).
17. 07's Done when, `rg -n 'Keep the task' crates/smith-domain-run/src/prompt.rs`:
    1 line (`:30`), which 07.3 removes.
18. 09's Done when, `rg -n '\bstruct Job\b' crates/smith-domain-tools/src`:
    `job.rs:81: pub(crate) struct Job {`, which 09.1 renames.
19. The users of each type a Shape changes (`git grep -cE <pattern>
    origin/main`), in smith, jig and temper: `Call::Shell` 64, 0, 0;
    `Outcome::Exited` 41, 0, 0; `Op::Spawn` 51, 0, 7; `Done::Exited` 33,
    0, 0; `tools::Limits {` 32, 0, 0; `ToolCompleted {` 15, 0, 0;
    `Declared {` 17, 0, 0; `Policy {` 22, 0, 54; `Failure::HeadUnavailable`
    3, 0, 0; `.jobs()` 21, 0, 0; `FORMAT_VERSION` 8, 0, 0. smith's go in
    the increments' Files or README rule 3 (extended, after this count, to
    constructions of a variant or struct that gains a field); temper's
    `Op::Spawn` and jig's and temper's `smith-domain-tools` users wait for
    their repins (`later.md`; decisions.md 8.3). temper's `Policy {` are
    its own type.
20. `git merge-base --is-ancestor 0d5604f9 <commit>`: `e1feddd6`
    (tool-output's old control) predates the product face; `f2c37fe8`
    descends from it. 04.6 and 12.3 calibrate with no control
    accordingly, under the design landing `72311b8e`.

21. `heavy cargo nextest run -p smith-machine-world -E 'binary(jobs) or
    binary(listing)'` (09's Done when) and `heavy cargo nextest run -p
    smith-bench -E 'binary(parity)'` (04's): exit 94, `operator didn't match
    any binary names`: nextest refuses a binary name that matches no test
    file, and `tests/jobs.rs` and `benchmarks/tests/parity.rs` are 09.2's
    and 04.4's. They run once those land.

22. After the reviews' corrections, run again: README "Done when" item 6
    (the versions): exit 0, it printed `FORMAT_VERSION: u64 = 3`, `pub const
    VERSION: u64 = 1` and `RESULT_VERSION: u32 = 2` (no `SUMMARY_VERSION`
    before 03.1); rule 7's check with its lint-exception pattern: exit 1,
    nothing printed; rule 5's `dbg!` check: exit 1, nothing printed; rule
    10's total with its `fetch`: exit 0, `0 attempts, $0.00`; rule 1's
    `git diff --numstat origin/main...HEAD`: exit 0, nothing (no commit
    yet on the plan's branch).
23. The interface sketches after the corrections (`Parity.smith_commit`
    added, and the cancel pair of 10.2 and 10.3): `heavy cargo clippy -p
    parity-sketches -- -D warnings`: exit 0, `Finished dev profile`.

## Not run at the base

- Every live Proof (02.1, 02.3, 04.6, 06.2, 07.3, 11.3, 12.1 to 12.4) and
  README "Done when" item 3 (the smoke suite): they spend the run's live
  budget, and name code earlier sessions land.
- Each test a Test shape names, and each Done-when filter over tests that
  do not exist yet (`binary(jobs)`, `test(task_role)`,
  `test(exec_in_a_repository_without_a_commit_exits_two_with_its_line)`):
  the ready check's.
