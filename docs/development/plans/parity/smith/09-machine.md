# Session 09: the machine's jobs, and the listing's order

2026-10-14. The machine runs a command to its end and answers once; a
job needs its group held past the call's answer, read again and stopped
(protocol/agent.md 2). The tools kit's type for any call in flight is
named `Job`, which the design now uses for a command left running. And
`list` failed 8 times on the repository task with "operation failed: io
error": the listing parser requires byte order, while `rg --sort=path`
sorts each directory by name, so `foo/x.rs` comes before `foo.rs`; traced
in code, seen in the recorded attempts and reproduced with this machine's
`rg` 14.1.1 (findings `planning-audit.md`, the `list` defect). This
session renames the kit's type, gives the machine held jobs and their
boundary, and fixes the listing's order after the baseline.

## Read first

- smith `docs/design/protocol/agent.md`, section 2 (the machine's table
  and its "Jobs" and close bullets); `docs/design/protocol/limits.md`,
  sections 3.5 and 7; `docs/design/domain/tools.md`, sections 4 (`list`)
  and 5.
- `findings/planning-audit.md`: tools and machine (points 1 and 2), and
  the `list` defect.
- skein `programming-model.md`, sections 4.2, 4.4, 5.2 and 5.4;
  `testing-strategy.md`, sections 2.4 and 4.3.

## What the design asks

- **Hold a job:** its group kept running under its command's deadline,
  what it writes kept as the head and the tail of the bytes not yet read,
  with the count dropped between them, within the job's kept output.
  **Read a job:** as much of its head and tail as the domain asks, the
  rest counted as dropped; the read empties it; then its end, once it has
  ended; it answers at the job's end, or when the asked wait ends. **Stop
  a job:** its group stopped in the three steps, then its end answered
  (agent.md 2).
- **A job's deadline is its command's, from its start;** past it, its
  group is stopped and its end is timed out. **The machine's close stops
  a job it still holds** with the third step alone, io's close of its
  child, within one `close` (agent.md 2; limits.md 7).
- **`list`** answers the tree's entries in path order (tools.md 4); a
  listing rg gives in its own order is not a malformed stream.

## Choices this plan makes

- The kit's calls in flight become `Flight`s: `flight.rs`, `Flight`,
  `Domain.flights`, `Kit.flights`, `Domain::flights()`, and
  `flight::{start, cancel, done, slots, held, io_session}` (the audit's
  successor name; `Flight` is unused in smith).
- A held job keeps the token of the `Spawn` that started it: the tools
  domain reads and stops it by that token (interface sketch).
- The listing compares paths name by name, `/` lowest, which is rg's
  order with `--sort=path`, and refuses only what is out of that order.

## Rules that bind it

- **Users outside Files** (README, rule 3): 09.1's rename reaches the
  worlds and other crates with no line limit.
- **The baseline comes first** (rule 11): 09.3 lands after 12.1.
- **Hotspots** (rule 15): 09.2 adds arms to `session.rs`, `route.rs` and
  `service.rs`, which 07 and 10 also change.

## Increments

### 09.1 The kit's flights
Scope: code-organization.md 2 (the name `job` freed for domain/tools.md
  5).
Needs: nothing.
Files: `crates/smith-domain-tools/src/job.rs` (to `flight.rs`),
  `domain.rs` (`:12`, `:35`, `:43`, `:59`, `:66`, `:97`), `kit.rs` (`:40`,
  `:53`, `:120-176`, `:362`), `limits.rs` (`:11`, `:95-96`, `:172`),
  `crates/smith-domain-session/src/domain.rs` (`:110-111`), and its
  users in `tests/tools/src/world.rs` (`:512`, `:859`),
  `tests/tools/tests/simulation.rs` (`:1328`), `tests/tools/tests/fuzzy_memory.rs`
  (`:86`), `tests/session/src/world.rs` (`:1998`), `tests/agent/src/world.rs`
  (`:1793`), `tests/protocol-llm/src/world.rs` (`:1642`).
Shape: a rename, and nothing else. The script kinds named `Job` in
  `tests/agent/src/script.rs:16` and `tests/protocol-llm/src/script.rs:16`
  stay.
Test shape: none new; every test keeps its name and its expected values.
Proof: unit, the crates' tests as they stand (`heavy cargo nextest run
  -p smith-domain-tools -p smith-domain-session`).
Don't: change behaviour.
Size: a rename, not counted (README, rule 1).
Removes: the name `Job` for a call in flight.

### 09.2 The machine holds jobs
Scope: agent.md 2's job rows and bullets; limits.md 3.5 and 7.
Needs: 09.1. Not 08.3: the machine holds what the domain asks, and its
  world sets its own limits; 08.3 derives the machine's processes for jobs.
Files: `crates/smith-domain-tools/src/boundary.rs` (`Op::Spawn`, `:175`;
  `Request`, `:68`; `Done`, `:238`), `crates/smith-domain-tools/src/call.rs`
  (`JobEnd`, beside `Exit`), `crates/smith-domain-tools/src/flight.rs`
  (the new `Done` arms), `crates/smith-protocol-machine/src/` (`boundary.rs`
  `FromDomain` and `ToDomain`, `:12`, `:29`; `process.rs` `Capture`
  `:67`, `Stop` and `Why` `:45-63`, `answer` `:376`, `shell` `:471`;
  `component.rs` `fire` `:396`, `:389-392`, a close), the hops
  `crates/smith-domain-session/src/session.rs` (`:1099-1100`),
  `crates/smith-domain/src/route.rs` (`:499-503`),
  `crates/smith-agent-service/src/service.rs` (`:1337`, `:2027-2042`,
  `:2184-2195`), `tests/machine/` (its lib and a new `tests/jobs.rs`).
Shape: as sketched below. `Op::Spawn` gains `after: Option<Duration>`:
  a command still running then is answered `Done::Held` with its output
  so far, and stays in the machine's processes under the same token.
  `Request::Job { owner, act }` reads it (`JobAct::Read`, waiting up to
  `wait` for its end) or stops it (`JobAct::Stop`, with `Why::Stop`
  beside `Deadline` and `Cancel`); each ends in one `Done::Job`.
  `CancelIo` on a held job stops it and answers `Done::Job` with
  `JobEnd::Cancelled`. `Capture::take(&mut self, head, tail)` drains
  what was read; the job's deadline is its command's, from its start;
  the component's close stops each held job by io's close of its child.
Test shape: `tests/machine/tests/jobs.rs`, on the machine world:
  `a_command_past_its_yield_is_held_and_read_to_its_end`,
  `a_jobs_output_past_its_kept_bound_is_head_tail_and_counted_once`,
  `a_stopped_job_answers_stopped`, `a_job_past_its_deadline_answers_timed_out`,
  `a_job_held_at_the_machines_close_is_closed_within_one_close`. Under
  0.5 s each. Reference test:

  ```rust
  #[test]
  fn a_command_past_its_yield_is_held_and_read_to_its_end() {
      let mut world = World::new(7, &[]);
      let (root, owner) = (world.root(), Token::new(10));
      let spawn = tools::Op::Spawn { cwd: place(root, b"."), command: b"printf one; sleep 2; printf two".into(), head: 64, tail: 64, after: Some(Duration::from_secs(1)) };
      let held = world.request(FromDomain::Op { owner, op: spawn, deadline: far() });
      assert_eq!(done(held), tools::Done::Held { head: b"one".into(), tail: Box::default(), dropped: 0 });
      let act = tools::JobAct::Read { head: 64, tail: 64, wait: Duration::from_secs(5) };
      let read = world.request(FromDomain::Job { owner, act });
      assert_eq!(done(read), tools::Done::Job { head: b"two".into(), tail: Box::default(), dropped: 0, end: tools::JobEnd::Ended(tools::Exit::Code { code: 0 }) });
  }
  ```
  `World`, `place` and `far` are `tests/machine`'s (`src/lib.rs:60`,
  `tests/stories.rs:9-15`); `done` and `FromDomain::Job`, the machine's
  mirror of `Request::Job`, are this increment's.
Proof: machine world, those tests (the real kernel; the job's group is
  skein's to prove dead at its deadline).
Don't: let the tools domain issue a yield yet (10.1 does), or write to a
  job's standard input (later.md).
Size: 300 lines.
Removes: `Capture::finish`'s consuming read for a held job.

### 09.3 The listing's order
Scope: tools.md 4's `list` (the `list` defect; decisions.md 7.3).
Needs: the baseline `parity` run's summary on `main` [12.1].
Files: `crates/smith-protocol-machine/src/listing.rs` (`push`, `end_path`,
  `:52-102`; `finish`, `:156-175`), `crates/smith-protocol-machine/src/tests.rs`
  (`:897`), `tests/machine/tests/listing.rs` (its native-rg story and
  `listed`), `tests/machine/tests/fuzzy_listing.rs`.
Shape: a path is in order when, compared name by name, it follows the
  last one; a name compares as bytes, and a path that ends before the
  other comes first. Out of that order is still `Fault::Other`.
Test shape: `rgs_sorted_listing_with_a_name_and_its_prefixed_sibling_is_listed`
  (unit: `listed(b"./a/x\0./a.b\0", …)` is `Done::Listed`); in
  `tests/machine/tests/listing.rs`, `a_native_rg_listing_of_a_file_beside_its_namesake_directory_is_listed`
  (fixture `src/a/x.rs`, `src/a.rs`, `src/a-b/y.rs`, the spawn's real rg
  arguments); the fuzzy listing generates rg's order, with names that are
  prefixes of their siblings. The unit test at `tests.rs:897` expects the
  new answer (the case the Shape changes). Under 20 ms, 0.5 s and 5 s.
Proof: machine world: the native-rg test, run once before the fix, where
  it fails (its failure line in the commit body), and once after.
Don't: sort rg's output in smith.
Size: 40 lines.
Removes: the byte-order check.

## Interface sketches

```rust
// smith_domain_tools::boundary (09.2); the tools domain owns the types, the machine answers them.
// Op::Spawn gains:  after: Option<Duration>,   // the yield, already clamped

/// One act on the job held under `owner`, the token of the `Spawn` the machine answered `Held`.
pub enum JobAct {
    /// Its output since the last read, answering at its end or once `wait` has passed.
    Read { head: u32, tail: u32, wait: Duration },
    /// Stop its group in the three steps, then answer its end.
    Stop { head: u32, tail: u32 },
}
// Request gains:  Job { owner: Token, act: JobAct },   // ends in one Event::Done

/// How a job stands in an answer.
pub enum JobEnd {
    Running,
    /// Exited, signalled, or past its deadline (`Exit::TimedOut`).
    Ended(Exit),
    Stopped,
    Cancelled,
}

// Done gains:
//     /// `Spawn` with a yield: still running at the yield, now held.
//     Held { head: Box<[u8]>, tail: Box<[u8]>, dropped: u64 },
//     /// A `JobAct`'s answer: the output since the last read, then `end`.
//     Job { head: Box<[u8]>, tail: Box<[u8]>, dropped: u64, end: JobEnd },
```

`JobEnd` lives in `call.rs` beside `Exit`; 10.1 uses it in its outcomes.

## Done when

- `rg -n '\bstruct Job\b' crates/smith-domain-tools/src` prints nothing.
- `heavy cargo nextest run -p smith-machine-world -E 'binary(jobs) or binary(listing)'`
  passes, once.
