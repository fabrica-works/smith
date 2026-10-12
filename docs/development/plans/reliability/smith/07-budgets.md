# Session 07: budgets and delegation

Provisional, 2026-10-12. Twelve of the session's thirteen increments are
on `main`: real spend, the reserve and the wind-down, children's shares
and final turns, the budget the model is told, and facts that show
shares. Two things are left. The first is new: a sub-agent's usage line
tells the model what the run has left, but no `budget` record is known
to hold the values the line told, so the `budget/visibility` probe
cannot judge smith's side of it. 07.14 makes the step that writes a
usage line emit the `budget` record it tells, naming the call. The
second is 07.13: with the reserve on `main`, the user decides whether
the local host grants sub-agents by default, and 07.13 gives them the
recommendation from session 13's budget and parallel-children runs.

## Read first

- domain/run.md, sections 5.3 (the result and its usage line), 9, 9.1,
  9.2 and 11;
- protocol/events.md, sections 2 ("Order", "Time"), 3.4, 3.6
  (`tool.completed`'s `child`), 3.8 and 6;
- benchmarks.md, section 11.1, the `budget` and `parallel-children`
  rows;
- shell.md, section 6.4 (`sub_agents`, "off, decided, until the reserve
  lands");
- decisions.md, 5.4, 5.8 and 5.10;
- findings D4, D9 and D11 (`findings/delegation.md`).

## What the design asks

- **The usage line** (run.md, 5.3 and 9.2): "a usage line the run
  writes: whether the child stopped for its budget, what it used, in
  turns and time, and the turns and time the run has left above its
  reserve."
- **Its values are the facts'** (run.md, 9.2): "Durations are told in
  whole seconds, rounded down, so a share or what is left is never
  overstated. ... The exact values are the facts', in milliseconds
  (protocol/events.md)."
- **The probe that judges it** (benchmarks.md, 11.1, `visibility`):
  "The told turns and time equal the `budget` record of the step that
  wrote them, time in whole seconds rounded down; a mismatch there is
  smith's."
- **The record** (events.md, 3.8): "`budget`, a snapshot of the run's
  budget each time a completion is charged, when its stage changes, and
  as a sub-agent's return writes its usage line (domain/run.md, 9.2):
  that record holds the values the line tells, and names the call", its
  `call` "absent on every other `budget` record".
- **Local sub-agents** (shell.md, 6.4): `sub_agents` "off", "decided,
  until the reserve lands"; decisions.md, 5.8: "It is then
  reconsidered."

## Choices this plan makes

- **The step that writes a usage line emits the snapshot it tells.**
  When a child returns with a result that carries a usage line
  (`Returned::Answered` or `Returned::Unanswered`), the run pushes one
  `Budget` fact in that step, naming the call, and computes the line's
  "left" from that snapshot. No other snapshot changes when it is
  pushed.
- **The snapshot is in whole milliseconds.** Its used time is rounded
  up to a whole millisecond, and the line's time left is computed from
  the snapshot's whole-millisecond values, as the record carries them.
  So the record holds the told value, and what is left is never
  overstated.
- **The join is a field, not a time.** The `budget` record names the
  `sub_agent` call by the provider's id, as `tool.completed` does. A
  join by `t_ms` or by the nearest record is not used: two steps may
  share a millisecond, and records of one step need not be adjacent in
  the stream.
- **07.13 runs nothing of its own.** Session 13 runs the `budget` and
  `parallel-children` probes on the product with sub-agents granted:
  smith-bench renders `sub_agents = true` for the events face's
  `LocalReserved` settings (`benchmarks/src/agent/smith/prepare.rs:83`,
  `settings.rs:191`). 07.13 reads what those steps recorded.

## Rules that bind it

- **No field that its counterpart across a boundary lacks**
  (code-organization.md, "Don't"): the run's `FactKind::Budget` gains
  `call`, and the `budget` record gains `call`, in the same increment.
- **Formats keep this plan's versions** (README, rule 5): events stays
  version 1, and the `budget` record gains `call` under it, with no
  bump. The rule overrides events.md, 6's "a field added" being a new
  version, as it does for every format in this plan.
- **Facts change nothing** (run.md, 11: "Nothing the agent decides
  depends on a fact"). The snapshot pushed at a return changes no
  decision: the values the line tells are those `budget::ordinary`
  gives today, read from the snapshot instead of computed beside it.

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 07.1 | a40cedc2 | merged, unproven: smith-13 |
| 07.2 | 75498577 | merged, unproven: smith-13 |
| 07.3 | 96d849be | deterministic |
| 07.4 | 1ac50a95 | deterministic |
| 07.5 | d64e9736 | merged, unproven: smith-13 |
| 07.6 | d19cb388 | merged, unproven: smith-13 |
| 07.7 | 0d826590 | merged, unproven: smith-13 |
| 07.8 | 576a251e | merged, unproven: smith-13 |
| 07.9 | 8c497030 | merged, unproven: smith-13 |
| 07.10 | 4aef8240 | merged, unproven: smith-13 |
| 07.11 | 5c711cdf | merged, unproven: smith-13 |
| 07.12 | 78096b27 | merged, unproven: smith-13 |

## Increments

07.14 comes first: it needs nothing outside the session. 07.13 needs
session 13's `budget` and `parallel-children` steps.

### 07.14 The budget a usage line tells, named by its call
Scope: smith 07's writing-step budget. The step that writes a
  sub-agent's usage line emits a `budget` record whose `call` is that
  `sub_agent` call, and the line's "ordinary left" turns and time are
  that record's limits less what was used and the reserve's part, time
  in whole seconds rounded down from its milliseconds (run.md, 5.3 and
  9.2; events.md, 3.8; benchmarks.md, 11.1, `visibility`). A snapshot's
  used time is rounded up to a whole millisecond. smith 00's told check
  [00.17] reads it.
Needs: nothing (protocol/events.md, 3.8 and domain/run.md, 11 name the
  record and its `call`).
Files: `crates/smith-domain-run`: `facts.rs` (`FactKind::Budget`,
  `Standing`), `run.rs` (`ended` at 1265, `standing` at 1403, the push
  sites at 706, 739, 868, 1301 and 1385), `budget.rs` (`left`, beside
  `ordinary` at 407); `crates/smith-agent-service`: `observations.rs`
  (`budget_record` at 126), `events.rs` (349), `tests.rs` (365);
  `crates/smith-events`: `records.rs` (`BudgetStanding` at 540),
  `write.rs` (`encode_budget_standing` at 2054), `read.rs` (1719),
  `limits.rs`, `golden/v1/` (a `budget_told.jsonl`).
  Users of the changed types: `crates/smith-domain/src/domain.rs:445`,
  `crates/smith-protocol-channel/src/facts.rs:25`,
  `tests/run/src/world.rs:1461`, `tests/run/tests/reserve.rs:102`,
  `tests/run/tests/simulation.rs:399`, `tests/run/tests/wind_down.rs:65` and `:146`,
  `tests/run/src/child_final_referee.rs:44`,
  `tests/agent/tests/budget.rs:290`,
  `tests/codecs/tests/support/events.rs:1664` and `:1679`,
  `tests/codecs/tests/support/events_max.rs:419`,
  `benchmarks/tests/events.rs:935`,
  `benchmarks/tests/budget_visibility.rs:13`,
  `benchmarks/tests/legacy.rs:278`. Shared hotspot: `run.rs`.
Shape: `FactKind::Budget { run: Token, standing: Standing, call:
  Option<Token> }`, `call` the run-issued call token that
  `facts::child_returned` names (`call.owner` in `ended`); `None` at a
  charge or a stage change of its own. In `ended`, for a child whose
  result is `Returned::Answered` or `Returned::Unanswered`: one
  snapshot, pushed before `facts::child_returned`, replacing the
  conditional push at 1298-1302 for that step; `left =
  budget::left(&snapshot)` replaces the `budget::ordinary` call at
  1320-1327. `standing` rounds the used time up to a whole millisecond.
  `smith_events::BudgetStanding` gains `call: Option<Box<[u8]>>`,
  mirroring `FactKind::Budget.call` as the provider's id, written after
  `reserve` and bounded like `tool.completed`'s `call`.
  `budget_record(fact, started_at, activation, call: Option<Box<[u8]>>)`;
  the event writer resolves the token to the id from its open calls,
  where the `sub_agent` call is still open when the fact drains.
Test shape: unit, `crates/smith-domain-run/src/tests.rs`, reusing
  `Harness::new` (106), `after` (242), `spend` (343), `families` (1544),
  `agents` (1549), `running_on` (1606) and `child` (1623), modelled on
  `a_sub_agent_answers_with_its_last_message_once_it_has_ended` (1633).
  Reference test:
  ```rust
  #[test]
  fn a_child_closed_after_its_last_charge_tells_the_budget_of_its_return() {
      let mut h = Harness::new(LIMITS);
      let (_, main) = h.running_on(1, 100, agents());
      let (child, _) = h.child(main, 7, families(true, false, false), 101);
      drop(h.step(Event::Used { conversation: child, spend: spend(50) }));
      drop(h.step(Event::Yielded { conversation: child, stop: Stop::EndTurn, text: bytes(b"Found.") }));
      h.after(Duration::from_nanos(2_000_000_500));
      let emitted = h.step(Event::Ended { conversation: child, end: End::Closed, spend: spend(50) });
      let [Request::Return { result: Returned::Answered { left, .. }, .. }] = &*emitted else {
          panic!("expected the child's answer, got {emitted:?}");
      };
      assert_eq!(*left, Budget { turns: 9, spend: 9_950, time: Duration::from_millis(597_999) });
      let mut told = None;
      while let Some(fact) = h.pop_fact() {
          if let Fact::Budget { standing, call: Some(call), .. } = fact.kind {
              told = Some((call, standing.time));
          }
      }
      assert_eq!(told, Some((Token::new(7), Duration::from_millis(2_001))), "one snapshot names the call");
  }
  ```
  Also: `a_snapshot_rounds_used_time_up_to_a_whole_millisecond`;
  `a_childs_return_pushes_one_budget_even_with_its_stage_change`;
  `mains_end_names_no_call_in_its_budget`; in
  `crates/smith-agent-service/src/tests.rs`, beside
  `actual_root_budget_facts_encode_without_content_under_every_capture`
  (356): `a_budget_record_names_the_provider_id_of_its_call`; in
  `tests/codecs/tests/events.rs`, a `budget_told` golden read and
  written under each capture, and `budget_ordinary` without `call`. One
  process test, `tests/agent-process/tests/fact_drain.rs`, beside
  `a_real_child_joins_its_actual_opening_response_usage_and_enclosing_tool_terminal`
  (132), reusing `World::new`, `capture` (`world.rs:864`),
  `observe_facts(TraceCase::ChildAnswered, 4)` (`world.rs:921`),
  `settle` and `records`:
  `a_child_usage_line_tells_the_budget_record_naming_its_call`, which
  finds the `budget` whose `call` is the child's, and the usage line in
  main's next `response.started` result block, and asserts the line's
  "ordinary left N turns and M seconds" equals that record's turns and
  `time_ms` (limit less used less reserve, seconds rounded down).
  Ceilings: unit tests under 20 ms; the process test under 0.5 s (its
  neighbour runs in 0.073 s).
Proof: process layer, `tests/agent-process`:
  `a_child_usage_line_tells_the_budget_record_naming_its_call`, run once
  through `heavy cargo nextest run -p smith-agent-process-world -E
  'test(a_child_usage_line_tells_the_budget_record_naming_its_call)'`;
  it shows that the values the model was told are a record's, named by
  its call; a pass is the test passing, under 1 second. The parent has
  no such record, so the test cannot pass there; the unit test
  `a_child_closed_after_its_last_charge_tells_the_budget_of_its_return`
  shows the defect: on the parent no snapshot of the return step
  exists. The probe that shows it live is `budget/visibility`, which
  session 13 calibrates after smith 00's told check [00.17]; it is not this
  increment's proof.
Don't: join by `t_ms` or by the nearest record; change what any other
  snapshot holds or when it is pushed; change the usage line's words
  (`crates/smith-domain/src/feedback.rs:280`); touch `benchmarks/src/`
  (smith 00's told check).
Size: at most 120 lines of production code.
Removes: the line's "left" computed beside the snapshot in `ended`
  (the `budget::ordinary` call at 1320-1327; `ordinary` stays for
  `agent.rs:111` and `run.rs:772`).

### 07.13 A recommendation on local sub-agents by default
Scope: the recommendation decisions.md, 5.8 asks for, now that the
  reserve is on `main` (shell.md, 6.4). From session 13's recorded
  `budget` step (starvation, wrap-up, child-quota, spend)
  and `parallel-children` step: the counts per variant (passes, model
  deviations, smith defects, calibrated or not), and the runs, where
  those records name them, in which a child opened into the reserve or a budget
  failure came before main's last final turn. It names the risks left (children run synchronously;
  a writing child runs alone; work beside sub-agents is not planned) and
  the change a yes needs: shell.md, 6.4's row, then the default at
  `crates/smith-local-shell/src/local_settings.rs:120`.
Needs: session 13's `budget` and `parallel-children` steps, recorded.
Files: `docs/development/benchmarks/sub-agents-by-default.md` (new,
  Markdown only), landed with `land --docs` in a commit carrying `Step:
  smith-07.13`. The report raises the recommendation as its question for
  the user (development.md, 10.4); the supervisor records the user's
  answer in decisions.md, section 10, item 7.
Shape: no code. The file, at most 30 lines: the recommendation (on, or
  off), the counts above with their run identifiers, the risks, the
  change a yes needs.
Test shape: none: no code lands.
Proof: none. It reads only `git log origin/main --format=%B -E
  --grep='^Step: smith-13\.(3|4)$'` and the summaries those commits add.
Don't: run any probe; change the default; edit `docs/design/`, the plan
  or `decisions.md`.
Size: no production code. Removes: nothing.

## Interface sketches

The run owns the snapshot; the agent service turns it into the record.
A fact's `call` is the run's token; the record's is the provider's id,
as `tool.completed` has it. A snapshot whose `call` is set is pushed
once, in the step whose `Returned` carries the usage line, and before
that `Returned` fact.

```rust
// crates/smith-domain-run/src/facts.rs
pub enum FactKind {
    // ...on main...
    /// The run's budget at a charge, when its stage changes, and as a sub-agent
    /// returns: the snapshot its usage line tells (domain/run.md, 9.2).
    Budget {
        run: Token,
        standing: Standing,
        /// The returning `sub_agent` call whose usage line tells this snapshot;
        /// none for a charge or a stage change of its own.
        call: Option<Token>,
    },
}
// `Standing` keeps its fields; its `time` is now rounded up to a whole millisecond.

// crates/smith-domain-run/src/budget.rs
/// What the run has left above its reserve as `standing` tells it, in the
/// whole milliseconds the `budget` record carries: each limit less what was
/// used and the reserve's part, saturating at zero. A usage line tells these.
pub(crate) fn left(standing: &Standing) -> Budget;

// crates/smith-events/src/records.rs
/// A run budget snapshot, sent at each charge, at a stage change, and as a
/// sub-agent returns (protocol/events.md, section 3.8).
pub struct BudgetStanding {
    pub run: u64,
    pub stage: BudgetStage,
    pub turns: Quantity,
    pub spend: Quantity,
    pub time_ms: Quantity,
    pub reserve: Reserve,
    /// The `sub_agent` call, by the provider's id, whose usage line tells this snapshot.
    pub call: Option<Box<[u8]>>,
}

// crates/smith-agent-service/src/observations.rs
pub fn budget_record(fact: &run::facts::Fact, started_at: Time, activation: u64, call: Option<Box<[u8]>>)
    -> Option<Record>;
```

In `ended`, after the end is charged:

```text
snapshot = standing(run, now)              # used time rounded up to a whole ms
if the conversation has an asker (a child):
    result = agent::ended(child, end, used, budget::left(&snapshot))
    if result is Answered or Unanswered:   # the results that carry a usage line
        push Budget { run, standing: snapshot, call: Some(call.owner) }
    else if unaccounted.turns > 0 or the stage changed:
        push Budget { run, standing: snapshot, call: None }
    facts::child_returned(...); Return, as on main
else (main):
    if unaccounted.turns > 0 or the stage changed:
        push Budget { run, standing: snapshot, call: None }
```

## Questions to raise when reached

- **07.13, a variant not calibrated.** If session 13 recorded a
  `budget` or `parallel-children` variant as not calibrated, the
  recommendation says so and rests on the counts it has; it does not ask
  for more runs.

## Done when

- A sub-agent's usage line tells the `budget` record that names its
  call: `heavy cargo nextest run -p smith-agent-process-world -E
  'test(a_child_usage_line_tells_the_budget_record_naming_its_call)'`
  passes.
- The return step's snapshot exists and rounds up: `heavy cargo nextest
  run -p smith-domain-run -E
  'test(a_child_closed_after_its_last_charge_tells_the_budget_of_its_return)
  | test(a_snapshot_rounds_used_time_up_to_a_whole_millisecond)'` passes.
- The record carries `call`: `rg -l '"type":"budget".*"call"'
  crates/smith-events/golden/v1/` lists `budget_told.jsonl` (on `main`
  today it lists nothing).
- The recommendation on local sub-agents has landed:
  `test -f docs/development/benchmarks/sub-agents-by-default.md && echo
  landed` prints `landed` (today it prints nothing).
