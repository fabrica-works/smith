# Session 05: memory returns when its use ends

2026-10-16. The user decided that `memory` is a cap, as large as the work
needs, and that smith allocates only what it needs and frees it when it
no longer does (decisions.md 1.6, section 9). The worlds check today
that the live heap stays within the worst case at every step; nothing
shows that it comes back once a use ends. smith's own peak on
`shell-cwd` was 36 to 43 MB against a 2 GiB worst case, so most of the
worst case is bytes allocated on demand, and whether they return is
unmeasured (findings `causes.md`, "Memory: what is measured"). This
session adds the two checks of limits.md 11, "Memory returns", in the
agent's simulated world. It changes test code only; a check that finds
bytes kept past their use stops on a question (below).

## Read first

- smith `docs/design/protocol/limits.md`, sections 5 ("A cap, not an
  allocation") and 11 ("Memory returns"); `testing.md`, section 6
  ("Memory returns when its use ends").
- `decisions.md`, section 9; `findings/planning-audit.md` A24.
- skein `programming-model.md`, sections 6.1 and 6.3 (the counting
  allocator); `testing-strategy.md`, section 6.

## What the design asks

- **A compaction:** in the agent's simulated world, with the counting
  allocator, the live heap falls, once the new window opens, by at least
  the payload bytes of the compacted history's blocks less those of the
  new window's opening, cells left out, since a window's message list
  keeps its capacity.
- **Repeated uses:** across a story that compacts, reads jobs to their
  end, and opens and closes conversations, several times each, the live
  heap at the same point of each repetition is no larger than at the
  first.

## Choices this plan makes

- The checks drive the agent's domain as `tests/agent/tests/fuzzy_memory.rs`
  does: its driver, with the `Meter` made before `Domain::new` and the
  driver's containers allocated before it, so that `Meter::held()`
  (skein's `skein-heap`, `lib.rs:357`, re-exported by
  `skein_world::domain`) reads the domain's own heap. 05.1 moves that
  driver into `tests/agent/src/memory.rs`; `fuzzy_memory.rs` uses it
  from there, unchanged in what it checks.
- The stories are scripted, not drawn: the fuzzy test keeps the random
  driving.
- Limits come from the standard profile (`quantities::standard()` and
  its derivation), with the results sized to reach its threshold.

## Rules that bind it

- **Diagnosis** (README, rule 5): a heap that does not fall is a finding,
  not a test to bend: at most 6 runs with diagnostics in place, then the
  step stops on a question with the measure, the part it points to, and
  its log.
- **One behaviour per test** (README, rule 18).

## Increments

### 05.1 A compaction releases the compacted history
Scope: limits.md 11, "Memory returns", its first bullet; testing.md 6
  (findings A24).
Needs: nothing.
Files: `tests/agent/tests/fuzzy_memory.rs` (the driver, `:255-256`,
  `:736-802`), a new `tests/agent/src/memory.rs`, `tests/agent/src/lib.rs`,
  a new `tests/agent/tests/memory_returns.rs`.
Shape: `tests/agent/src/memory.rs` holds the driver `fuzzy_memory.rs`
  defines today, public to the package. `memory_returns.rs` declares the
  counting allocator as `fuzzy_memory.rs` does (`:22-23`), opens one
  conversation, answers its reads with results of known payload bytes
  until the window's check compacts it, answers the compaction, and
  reads `meter.held()` once before the compaction's request and once
  after the new window's opening is in place and the driver has reclaimed
  (`:802`). The expected fall is the payload bytes of the compacted
  history's blocks, its task message, answers, calls and results, less
  those of the new window's opening, cells left out, written in the test
  as a literal from the scripted sizes.
Test shape: `tests/agent/tests/memory_returns.rs`:
  `a_compaction_releases_the_compacted_historys_bytes`. Its reference,
  written out in the step: the allocator declaration, the driver from
  `memory.rs`, the scripted results, the two reads of `held()`, and one
  `assert!(before - after >= EXPECTED_FALL)` with `EXPECTED_FALL` a
  literal. Under 0.5 s.
Proof: world, the test. Nothing above the agent's simulated world reads
  the heap; the benchmarks read smith's peak resident size (smith 01.2).
Don't: count cells in the expected fall; shrink a slab or a queue to
  pass.
Size: none (test code).
Removes: the driver's definition in `fuzzy_memory.rs`.

### 05.2 Repeated uses hold no more heap
Scope: limits.md 11, "Memory returns", its second bullet; testing.md 6.
Needs: 05.1's driver in `tests/agent/src/memory.rs` [05.1].
Files: `tests/agent/src/memory.rs` (the driver answering
  `Request::Job`, which `fuzzy_memory.rs:328` marks unreachable, and
  shells with a yield, `:585`), `tests/agent/src/world/jobs.rs` (read),
  `tests/agent/tests/memory_returns.rs`.
Shape: the driver answers a job's acts as `tests/agent/src/world/jobs.rs`
  does. A story repeats three times: a compaction, a command answered
  `running` and read with `job` to its end, and a child conversation
  opened and closed. After each repetition, once the driver has
  reclaimed, it reads `meter.held()`; the second and third are at most
  the first.
Test shape: `tests/agent/tests/memory_returns.rs`:
  `repeated_compactions_jobs_and_children_hold_no_more_heap`. Under 0.5 s.
Proof: world, the test, as 05.1.
Don't: run the repetitions on separate domains.
Size: none (test code).
Removes: `fuzzy_memory.rs`'s `unreachable!` for `Request::Job`.

## Questions to raise when reached

- A check that fails because a part keeps bytes past its use: the step
  stops with the part and its bytes; the supervisor adds the fix as a
  step of its own (development.md 10.4).

## Done when

- `cd ~/src/rust/smith && heavy cargo nextest run -p smith-agent-world --test memory_returns` passes.
