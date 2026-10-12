# Local sub-agents by default

Recommendation: keep `sub_agents` off by default. The reserve passed three budget variants, but spend and both child-parallelism variants remain uncalibrated; reviews also exposed a smith stall.

Evidence: smith-13.3 (`59ba93d915eef9e8cfd3d325a9e9036162c51c76`) and smith-13.4 (`e1feddd6ace6fe014df9d20d20bbfbe8c322049a`), their commit bodies and added summaries only. All inferred attempts used small `gpt-6-luna`, low effort, with sub-agents granted.

| Variant | Run identifier | Passes | Model deviations | Smith defects | Calibrated |
|---|---|---:|---:|---:|---|
| budget/starvation | `probes-budget-starvation-5ac25927-20261012T174514Z` | 10 | 0 | 0 | yes |
| budget/wrap-up | `probes-budget-wrap-up-5ac25927-20261012T174717Z` | 10 | 1 | 0 | yes |
| budget/child-quota | `probes-budget-child-quota-5ac25927-20261012T175039Z` | 10 | 0 | 0 | yes |
| budget/spend | `probes-budget-spend-5ac25927-20261012T175402Z` | 0 | 0 | 0 | no; one setup refusal |
| parallel-children | `probes-parallel-children-3f15308c-20261012T181530Z` | 2 | 3 | 0 | no |
| parallel-children/reviews | `probes-parallel-children-reviews-3f15308c-20261012T181923Z` | 1 | 2 | 1 | no |
| parallel-read (context) | `probes-parallel-read-3f15308c-20261012T181328Z` | 7 | 3 | 0 | no |

Wrap-up attempt 01 batched reads and ended before its child's final budget turn. Parallel-children 01, 03 and 05 delegated sequentially; reviews 02 did likewise, and 04 opened no children. These are recorded model deviations.

Spend 01 was refused before inference: its $1.00 budget could not hold the $1.32 finishing reserve. Its raw summary has one harness error and no event session; 13.3 classifies it as a correct entrance refusal, not a smith defect or an inferred attempt.

Reviews 03 emitted plain text, received no subsequent response, and failed/cancelled at 49,846 ms despite budget remaining. This is the recorded smith defect, not a budget failure.

Neither recorded step nor its summaries identifies a run with a child opening into the reserve or a budget failure before main's last final turn. The summaries aggregate attempts rather than report that ordering; this is no claim that such an ordering cannot occur. Spend 01 never reached a main turn.

Risks left: children run synchronously; a writing child runs alone; work beside sub-agents is not planned. Sequential delegation and the stalled review limit the evidence for making delegation the default. The recommendation rests on these counts and requests no further runs.

A yes first needs the designer to change shell.md, section 6.4's `sub_agents` row, then an implementation change to the default at `crates/smith-local-shell/src/local_settings.rs:120`. The supervisor records the user's answer in decisions.md, section 10, item 7.
