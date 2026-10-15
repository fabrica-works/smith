# Parity plan: later

2026-10-14. What the design names, or the run will need, that this plan
does not do, and when it should be done.

| Item | Design | When |
|---|---|---|
| Children answering while their parent works (asynchronous children; work beside a writing child) | `domain/run.md`, sections 5.3 and 15; `decisions.md` 5.3 | after contained trees, in a document of their own |
| The `edit-format` experiment | `benchmarks.md` 11.3; `protocol/llm.md` 12; `decisions.md` 6.3 | after this plan's last closing `parity` run (12.4), if FT or FC fails there |
| The `delegation` experiment, for the user's decision on smith's `sub_agents` default | `benchmarks.md` 11.3; `decisions.md` 2.4 | after this plan's tasks and pins, in a plan of its own |
| `cache-reuse` version 2, with a no-affinity control | `benchmarks.md` 11.1; findings R4, F5 | the next probes plan |
| `parallel-read` version 2, with a copied answer | findings F3 | the next probes plan |
| `paths` version 3 | findings F2 | the next probes plan |
| `follow-up` version 2 | findings F7 | the next probes plan |
| `budget` version 2 | findings F9 | the next probes plan |
| `slow-completion`'s harder prompt | findings F11 | the next probes plan |
| `large-write/oversized` proven by a deterministic world | findings F8 | the next probes plan |
| Codex's turn-state routing | `protocol/README.md` 9; findings R2; the reliability plan's decision 4.4 | after the `affinity` experiment shows a material gain |
| Writing to a job's standard input | `domain/tools.md` 8 | when a probe shows a model needs it |
| `a_second_run_resumes_the_chat_from_its_files` (`tests/end-to-end/end_to_end.rs:151`) aborted once in about 73 runs since 05.3 (`d6ebc102`): `smith-local-domain/src/domain.rs:606`, "Load has one terminal: phase Done, requested true" (ruling 132; `tmp/smith/parity/logs/smith-03/03.s1-dependents.log`) | `domain/host.md`'s load of a chat | a step of its own if it comes back in this run (ruling 132); otherwise the next smith plan |
| Recalibrating `smoke-tools`, `large-write`, `budget/child-quota`, `budget/wrap-up`, `failures/one-turn` and `credentials` in their task files | `benchmarks.md` 3.1, 11.5; `decisions.md` 10.6 and section 9, item 9 | the next run with live budget, before it relies on `suite smoke` |
| The calibrations 12.3 could not run within the budget | `decisions.md` 7.1 and 10.2 | the next run with live budget |
| jig's and temper's repins: smith's transcript version 4, and the tools domain's new `Call` and `Outcome` variants, reach temper (all three generated crates and `smith-domain-tools`) and jig (`smith-domain-tools`) | development.md 5.7; `decisions.md` 8.3 | jig's and temper's own plans |
| What a failed call in a batch means for the calls after it | `domain/session.md` 11; `decisions.md` 6.4 | when a probe shows the cost |
| A job's end told between completions | `domain/session.md` 11 | after `mid-work`'s messages (`domain/run.md` 6) |
| The kit's changes held back for after this run (skein's `kit-after-parity` branch) and the reliability retrospective's deferred rows (its section 10: 3, 5, 9, 11, 15, 16, 17, 20) | skein `docs/foundation/development.md` | after this run's retrospective |
| `calibrate` rewrites the whole `task.toml` when it writes a `[calibration]` block, losing its layout; 11.3 restored it with a script (`tmp/smith/parity/logs/smith-11/preserve-calibration.py`) | `benchmarks.md` 5.2 | the next harness plan |
| `tool-output/timeout`'s calibration (ruling 145): its one attempt, `probes-tool-output-timeout-c98a562d-20261015T114015Z/01`, made no shell call with `shell` offered, yet `calibrate` counted it smith's and stopped: `tool-call` is not among the model's checks (`benchmarks/src/probe.rs:164`), and `tool-timeout` with no shell call fails (`benchmarks/src/agent/smith/events/checks.rs:559`) where benchmarks.md 3.2 has it "not exercised". A lead for the prompt: "After both results finish with TIMEOUTS VERIFIED" has no verb, unlike the other variants' "Then finish with" | `benchmarks.md` 3.1, 3.2; `decisions.md` 7.1 | the next harness plan: count a short `tool-call` count as the model's and `tool-timeout` without a shell call as not exercised, then calibrate `timeout` |
| A `finish` without a `fields` key was rejected, `missing field "fields"` (the attempt above, `trace.jsonl` line 10), and cost a turn; whether the finish tool's text says `fields` may be left out is unchecked | `protocol/llm.md`, the finish tool | the next smith plan, after a count of such rejections in the closing `parity` run's traces |
