# Wall-clock plan: later

2026-10-16. What the design names, or the run will need, that this plan
does not do, and when it should be done.

| Item | Design | When |
|---|---|---|
| jig's and temper's repins of smith: transcript version 5 and its not-run why (`NotRun::AfterFailedWrite`), the event vocabulary's version 3 (`run.compaction_material`), `Purpose::Ordinary { guard }`, the shipped `memory` raised by 03.2 (jig's `testing/jig-test/tests/support/mod.rs` and `tests/store/src/root.rs`, temper's `crates/temper/tests/support/configuration.rs` copy 2 GiB); jig matches `Returned::NotRun` in `crates/jig-protocol-transcript/src/summary.rs:81,134` and `lib.rs:218`; both are pinned to smith's transcript version 3 | development.md 5.7; `decisions.md` 11.4 and 12.3 | jig's and temper's own plans, after temper's client-api run; the designer's next landing puts it in their `STATUS.md`, under "Not planned" |
| `history` and `history/small-window` recalibrated: their `[calibration]` blocks were made under the old check, and 07.1 changes when a window compacts | `benchmarks.md` 3.1, 5.2, 11.1 | the next run with live budget, before `suite smoke` relies on them |
| Capacity reserved up front: how much of smith's peak is slabs' and queues' capacity, allocated at startup | `protocol/limits.md` 13; `decisions.md` 1.6 and 9 | skein's question, once smith measures it: a probe of the agent's heap at startup against its peak |
| A job's end told between completions | `domain/session.md` 11; `decisions.md` 7 | after `mid-work`'s messages (`domain/run.md` 6) |
| The `edit-format` experiment, if 08.1's report shows smith's edit responses still holding one file | `benchmarks.md` 11.3; `protocol/llm.md` 12; `decisions.md` 6 | the next smith plan, on 08.1's report |
| `shell-cwd`'s time (RT), or any criterion, if the plan's last closing run is short of parity | `benchmarks.md` 16.5 | the next smith plan: a diagnosis of that run's attempts, then a change that names its cause |
| `CARGO_BUILD_JOBS=2` in four of five smith attempts: where the model takes it from | `decisions.md` 7 | the next smith plan's diagnosis, if 09.1's report still shows blocking cargo |
| `tests/agent/tests/memory_returns.rs`'s scripted sizes (1024, 32_768, 64, the six reads, the five turns) named once each as consts, as the steer of 2026-10-16T13:12:23Z, point 2, asks: 05.2 landed at `728feb25` before the steer reached it | none (a test's form) | the next increment that changes the file, in this plan or after it |
