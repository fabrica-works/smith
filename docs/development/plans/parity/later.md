# Parity plan: later

2026-10-14. What the design names, or the run will need, that this plan
does not do, and when it should be done.

| Item | Design | When |
|---|---|---|
| Children answering while their parent works (asynchronous children; work beside a writing child) | `domain/run.md`, sections 5.3 and 15; `decisions.md` 5.3 | after contained trees, in a document of their own |
| The `edit-format` experiment | `benchmarks.md` 11.3; `protocol/llm.md` 12; `decisions.md` 6.3 | after the closing `parity` run, if FT or FC fails there |
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
| Recalibrating `smoke-tools`, `large-write`, `budget/child-quota`, `budget/wrap-up`, `failures/one-turn` and `credentials` in their task files | `benchmarks.md` 3.1, 11.5; `decisions.md` 10.6 and section 9, item 9 | the next run with live budget, before it relies on `suite smoke` |
| The calibrations 12.3 could not run within the budget | `decisions.md` 7.1 and 10.2 | the next run with live budget |
| jig's and temper's repins: smith's transcript version 4, and the tools domain's new `Call` and `Outcome` variants, reach temper (all three generated crates and `smith-domain-tools`) and jig (`smith-domain-tools`) | development.md 5.7; `decisions.md` 8.3 | jig's and temper's own plans |
| What a failed call in a batch means for the calls after it | `domain/session.md` 11; `decisions.md` 6.4 | when a probe shows the cost |
| A job's end told between completions | `domain/session.md` 11 | after `mid-work`'s messages (`domain/run.md` 6) |
| The kit's changes held back for after this run (skein's `kit-after-parity` branch) and the reliability retrospective's deferred rows (its section 10: 3, 5, 9, 11, 15, 16, 17, 20) | skein `docs/foundation/development.md` | after this run's retrospective |
