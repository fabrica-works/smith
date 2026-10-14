# Findings: failures and the probes that did not calibrate

2026-10-14. Why each uncalibrated probe failed, whose fault it was, and what each needs to calibrate.
Written by a read-only sub-agent for the parity plan's designer (skein's development.md, section 4.1).

Sources: smith `main` at 689c15d0, the attempts under `~/.local/state/smith-bench/runs/`, and the run
record in `~/src/rust/tmp/smith/reliability/`. "Final" is `probes-f2c37fe8-20261013T214945Z`.

## 1. In one page

- **Few failures are smith's.** The probes found three smith defects, and the run fixed all three:
  timeout's terminal (04.s2), the slow-completion silence (06.s2, 5ccb4f2a) and small-window's guide
  bound (04.s1). Still open: compaction answered with a call (the compaction theme) and cache reuse
  below its criterion (the cache theme). Every other failure is the probe's or the model's.
- **The two standing smoke failures are the harness's configuration (F1).** The smoke runs
  `first-request` with `smoke.json`'s brief, "Read the workspace's AGENTS.md before working". The seed
  has none, and the prompt forbids reads. The model searched for it (7 completions), then in the rerun
  gave up (4 completions). That rerun wrote no file at all, so there was no wrong file. Smoke 858c9642
  failed the same way. With the empty-brief pin the task passes in 2 completions (final 21, 22).
- **paths fails on `finish`, not on paths (F2).** In 12 of 14 attempts the model wrote, then
  called `finish` in the next completion. The check counts the completion that holds `finish`, so
  these runs fail at 3. Every read resolved: `../spec/` 3 times and `/spec/` 4 times, with no
  `missing` or `outside` verdict. `paths/single`, with no namespace at all, fails 7 of 7 the same way.
  smith's `/name/...` text confuses nothing.
- **smith ran every batch as asked (F3, F4).** All 13 parallel-read attempts put 20 reads in one
  response, and smith ran them 4 at a time, within 5 ms. Children overlapped in all 18 attempts that
  asked for two or more in one response, and in none of the 29 that asked one at a time.
- **Sums at low effort cannot calibrate.** Wrong additions failed parallel-read (5 of 13, each `5192`
  where the results add up to 5190), cache-reuse/children (2 of 3, final), Codex's cache-reuse (3 of 3)
  and small-window (6 of K6's 21). Copy-only answers take the model's arithmetic out of the verdict (Q1).
- **Four probes cannot fail, or cannot be exercised (F7, F8, F9).** In follow-up, the second line
  arrived after run 1 ended in 26 of 26 base and eof traces, so the control passed. Oversized's call is
  never made by a model that honours smith's stated bound (0 of 13). spend's $1 ceiling is under its
  $1.32 reserve. The visibility child gets all the ordinary time, so its share touches the reserve by
  construction.
- **Two probes need only a calibration run (F10).** tool-output v3 and process-tree v2 changed one
  grace pin (f2c37fe8). The control exists: e1feddd6 failed timeout 3 of 3.
- **Memory bounds the plan's room (F12).** The default product prices 1,939,210,982 B of
  2,147,483,648 B (208 MB spare). A conversation costs 153 MB, so a sixth does not fit. A
  500,000-token model is estimated to fit by 61 MB, and a 1,000,000-token one to be 261 MB over.
  `check.json` records neither the total nor the margin. The history, fixture and repository tasks
  run smith with `memory` = 10^12.
- **Calibration has no per-variant home, and a version covers every variant (F13).** One timeout pin
  bumped tool-output to v3, which voided three calibrations whose prompts, seeds and checks are unchanged.
- **The parity plan needs (Q7):** small-window v2 (compaction), cache-reuse v2 with the no-affinity
  control (cache), parallel-read v2 (parallel tools), and the smoke's first-request (F1). The rest can
  wait in later.md.

## 2. The probes

Passes count every smith attempt of that task version on the fix arm or later, unless noted. Attempt
ids without a run are the final run's.

| Probe / variant | Last record | Passes / attempts | Cause class | Entry |
|---|---|---:|---|---|
| `first-request` in `smoke` | `smoke-62acc437-20261013T211935Z/03`, `/03-rerun` | 0/4 (858c9642, 62acc437) | probe defect (harness configuration) | F1 |
| `paths` v2 | `34`, `34-rerun` | 2/7 | probe defect | F2 |
| `paths/single` v2 | `35`, `35-rerun` | 0/7 | probe defect | F2 |
| `parallel-read` v1 | `33`, `33-rerun` | 8/13 | model deviation (addition) | F3 |
| `parallel-children` v1 | `31`, `31-rerun` | 3/8 | model deviation (serial calls) | F4 |
| `parallel-children/reviews` v1 | `32` | 14/39 | model deviation; 1 silence, smith's, fixed (06.s2) | F4 |
| `cache-reuse` v1 | `06`–`08` | 0/3 triplets (reuse median 0.619) | smith defect (cache theme) | F5 |
| `cache-reuse/children` v1 | `09`–`11` | 0/3 triplets (reuse 0.145–0.188; answers 1/3) | smith defect and probe defect | F5 |
| `cache-reuse/crossing` v1 | `12`–`14` | 0/3 triplets (reuse 0.550–0.675) | smith defect | F5 |
| `history/small-window` v1 | `27`, `27-rerun` | 5/19 since f1a3fdab | smith defect (compaction) and model deviation | F6 |
| `follow-up`, `eof`, `chat` v1 | `23`–`25` | 12/12, 12/12, 36/38; control 27ead2e7 3/3 passed | probe defect (cannot fail) | F7 |
| `large-write/oversized` v9 | `30` | 0 exercised of 13; 2 exercised on record | probe defect (cannot be forced) | F8 |
| `budget/spend` v1 | `probes-budget-spend-5ac25927-20261012T175402Z` | refused at the entrance | probe defect | F9 |
| `budget/visibility` v1 | `04` | told 4/4; `budget-visible` unavailable 4/4 | probe defect | F9 |
| `tool-output/timeout` v3 | `45` | v3 1/1; v2 13/13 on d02d18ee, 0/3 on e1feddd6 | not run (smith defect fixed, 04.s2) | F10 |
| `process-tree` v2 | `36` | v2 1/1; v1 10/11 | not run | F10 |
| `slow-completion` v1 | `37` | 10/27 (9 not exercised, 6 silences, 2 skein); since 06.s2, 5/7 | smith defect (fixed) and probe defect | F11 |
| default memory footprint | `supervision.md:3320` | 1,939,210,982 B of 2 GiB | design question | F12 |

## 3. Entries

### F1 The smoke's first-request is told to read an AGENTS.md its seed lacks

- **Symptom:** `smoke-62acc437-20261013T211935Z/03` took 7 completions. It made `read AGENTS.md`
  (not_found), `list /`, `list repo` (not_found), `read ./AGENTS.md` and `list ""`, then `write`, then
  `finish`. `/03-rerun` took 4: `read AGENTS.md`, `list ""`, `list .`, then `finish` with "Could not read
  AGENTS.md: it is not present in the workspace. No changes made." `file-content:first.txt` reads "No
  such file or directory". In `smoke-858c9642-20261012T215555Z`, `/03` took 5 completions and
  `/03-rerun` wrote no file.
- **Root cause:** `suites/smoke.toml` runs every task with config `smoke`, whose
  `agents/smith/smoke.json:20` sets the instructions "Read the workspace's AGENTS.md before working.
  ...". smith puts this brief first in its system text (03's `prompt.system`). `probes` runs the same
  task with the empty-brief pin (`suites/probes.toml:20-21`; `first-request.json:20`). benchmarks.md
  11.1 forces "an empty brief", and the prompt says "Do not list, read ...". No smoke seed holds an
  AGENTS.md (`first-request` and `smoke-tools` hold only a README). The brief contradicts the prompt:
  neither smith's fault nor the model's.
- **The clean fix:** `smoke` runs `first-request` with its own pin, as `probes` does, and
  `smoke.json`'s brief loses the AGENTS.md sentence, since the other smoke tasks spend completions on
  the same search (`01`: 3 lists, 3 reads). This is a suite change, not a task version. benchmarks.md
  11.5's `smoke` row gains "each with its calibration's configuration".
- **Its tests:** `benchmarks/tests/smith_adapter.rs`: `the_smoke_renders_first_request_with_its_empty_brief`,
  and `a_brief_naming_a_guide_requires_it_in_the_seed`. Live: one smoke, then 12 attempts of
  first-request under the smoke's configuration.
- **Confidence:** high. 4 of 4 attempts with the brief failed, and 2 of 2 without it passed.

### F2 paths fails on `finish`'s own completion, not on its paths

- **Symptom:** base runs were e1feddd6 01–04, 62acc437/34, and final 34 and 34-rerun. The 5 failures
  each ran reads, then `write`, then `finish`, while the 2 passes put `write`+`finish` in one
  response (e1feddd6/03, 62acc437/34). `single` failed 7 of 7 in that shape. The spec read used
  `/spec/rules.txt` 4 times and `../spec/rules.txt` 3 times, every one verdict `read`, with no `list`.
  Codex passed both variants (`probes-paths-12eb3e81-20261012T185710Z/01`: `cat ../spec/rules.txt`).
- **Root cause:** `completions at-most 2, before = "finish"` (`tasks/probes/paths/task.toml:80-82`,
  `:121-123`) counts until finish's `tool.started`, so it counts the response that holds `finish`
  (`benchmarks/src/agent/smith/events/checks.rs:91-99`). `task.rs:227` documents this as "Those
  preceding the first finish tool". smith says "When the work is done, call `finish`"
  (`crates/smith-domain-run/src/prompt.rs:517`) and "Batch writes, edits and commands; run serially in
  call order" (`smith-protocol-llm/src/descriptions.rs:96`), and the model waits for the write's result.
  smith takes both in one response: e1feddd6/03 was `written` at 2193 ms and `result` at 2194 ms.
  benchmarks.md 11.1's "at most two completions before `finish`" does not say whether that response
  counts.
- **The clean fix:** paths v3 (5.2). It counts "at most two completions before the one that calls
  `finish`" (a `before = "finishing"` that excludes it) and adds `tool-call list count exactly 0`,
  which is what a model searching for paths does. The prompt drops "the answer write and finish
  together". 11.1's row gains "and no `list`". The namespace text stays as it is.
- **Its tests:** `benchmarks/tests/events.rs`: `finishing_bound_excludes_the_response_holding_finish`,
  and `a_list_call_fails_the_paths_namespace_probe`, on final 34's trace (pass) and smoke 03's (fail).
  Live: 12 per variant. The control is the parent of 0597d241 ("Fix the workspace namespace in
  prompts"), through a pinned face entry (13.54). If no face can read it, the record says "none
  runnable".
- **Confidence:** high on the cause. Medium that v3 calibrates: 12 of 14 attempts already fit it.

### F3 parallel-read's wrong sums are the model's addition

- **Symptom:** of 13 attempts (3f15308c 01–10, 62acc437/33, final 33, 33-rerun), 8 passed. All 13 made
  20 reads in one response (`calls-in-one-response` passed), and all 13 got results (`value=N`) adding
  up to 5190. The 5 failures each answered `ALPHA 5192`, with 0 reasoning tokens. 7 of 13 made a
  `list` first, against "In your first completion issue exactly twenty read calls", and 4 of those
  7 failed, against 1 of the other 6. In final 33 the reads started at 4049–4053 ms and ended by 4054 ms.
- **Root cause:** the model's addition at low effort. smith did what session.md 5 says, "Adjacent
  reads in one turn run together, ... results go back in call order". It ran them 4 at a time:
  `parallel_tools` is `calls_per_response.min(4)` (`crates/smith-domain-session/src/limits.rs:286`;
  `smith-agent-service/src/profile.rs:521`), a cap the design never names. smith's requests set
  `parallel_tool_calls: true` (skein `crates/skein-llm/src/translate.rs:75-76`, `openai/request.rs:119-120`).
- **The clean fix:** parallel-read v2 answers "ALPHA and the twenty values in file order", copied, so
  the answer still proves every result reached the model in order. It stays at low effort. If more
  than 2 in 12 still deviate, it names `effort = "medium"` with that reason (3.1).
- **Its tests:** `smith-domain-session` `twenty_adjacent_reads_run_four_at_once_in_call_order`, and
  `benchmarks/tests/parallel_probes.rs` `parallel_read_v2_answer_is_twenty_values_in_order`, beside
  `twenty_reads_must_be_in_one_response_and_content_absence_is_unknown` (`:52-71`). Live: 12. The
  control is `[smith] calls_per_response = 16`, the product's default, under which 20 calls should
  fail `calls-in-one-response` or `no-failure` (limit). That control has never run, so its first
  run confirms it.
- **Confidence:** high that the tools were right. Medium that copying keeps deviations within 2 in 12.

### F4 Children overlap whenever the model asks for them together

- **Symptom:** across 47 `parallel-children` and `reviews` attempts (3f15308c, 24bae47a, c1dab295,
  ae84f57b, 62acc437, final), `children-overlap` passed in all 18 where a main response held 2 to 4
  `sub_agent` calls. It failed in all 29 where none held more than one. Children asked together opened
  in the same millisecond (3f15308c/02: four at 4811 ms; final 32: three at 1975 ms). Final 31 and
  31-rerun each made a `list`, then one `sub_agent` per response. Other deviations: no children
  (3f15308c reviews/04) and a `wait` after the reviews (ae84f57b/07, parked). One stall was smith's:
  3f15308c reviews/03 was owed a nudge and was cancelled at 49,846 ms (13.62). Its cause, a
  replay-bytes bound, was fixed in 06.s2 (5ccb4f2a's body). The nudge is now sent: final 27's ninth
  request reads "This is reminder 1 of 1".
- **Root cause:** the model's choice at low effort. smith opens every inspect-only call of one
  response together, as run.md 5.3 says ("Children granted only inspect ... run side by side"), and its
  text says "Inspect-only children may work side by side" (`prompt.rs:400`).
- **The clean fix:** no smith change. v2 prompts open with "Your first response is exactly four
  `sub_agent` calls; do not list". If more than 2 in 12 still deviate: `effort = "medium"`, with the reason.
- **Its tests:** `smith-domain-run` `four_inspect_children_in_one_response_open_in_one_step`. Live: 12
  per variant. The control is `[smith] conversations = 2`, under which the pool waits (limits.md 2.2)
  and `children-overlap` fails.
- **Confidence:** high (18 of 18 and 29 of 29).

### F5 cache-reuse fails on smith's reuse, and its answers on addition

- **Symptom:** final base reuse R was 0.619, 0.481 and 0.619 (median 0.619 against 0.85), with 3, 5
  and 3 full misses in 12 requests against at most 1 in 10. Children: 0.144, 0.188 and 0.145 (7, 5 and 7
  misses of 8, 6 and 8). Crossing: 0.550, 0.675 and 0.552. 8d2ec9b8 agrees (base 0.485–0.762, children
  0–0.188). Each attempt's check stays "judged on a complete three-repetition triplet", and the group
  reads `passed 0, failed 1`. Children 09 and 10 answered "ALPHA 197 196", and Codex answered 196, 198
  and 196 against 197. Codex had 172,800 of 187,569 input tokens cached (`12eb3e81/01/stdout.log`).
- **Root cause:** smith reuses less of its prompt cache than Codex does on the same chain. The cache
  theme owns why (affinity, a stable prefix, children starting cold). The wrong answers are additions (F3).
- **The clean fix:** the cache theme's smith change. v2 answers the END file's code word in all
  three variants. The reuse criteria stay until a calibration sets them (11.1).
- **Its tests:** the cache theme's, plus `cache_triplet_verdict_appears_in_its_summary_group`. Live:
  three triplets, then the no-affinity control 11.1 names, which must fall below.
- **Confidence:** high on the numbers. The cause is the cache theme's.

### F6 history/small-window: what the probe itself needs

- **Symptom:** 5 passed of 19 attempts on f1a3fdab and later (f1a3fdab 2/5, 62acc437 0/2, final 1/2,
  compaction suite 2/10). 8 ended `Failed(model)` after compaction (f1a3fdab 01 and 03, 62acc437
  27-rerun, compaction 02, 03, 07, 09 and 10). Final 27 stopped at read 7, before compacting, with "I
  can’t complete the chain in this response." The before failed as it should
  (`probes-history-small-window-8d9d5408-20261012T183429Z/01`: no compaction). K6 has the full count.
- **Root cause:** the malformed ends are smith's (compaction theme, K1). Three parts are the probe's.
  First, `^ALPHA 528$` (`history/task.toml:32`) judges a sum carried across summaries, and fails
  `ALPHA528`. Second, the task pins no `memory`, so the old template's `memory_bytes`
  (`agents/smith/smoke.json:4`) applies: `26/check.json` has `declared.memory` 1000000000000. Third,
  the variant lowers only luna's window. History bytes follow the largest configured window
  (`smith-agent-service/src/profile.rs:434-449`), and astra keeps 272,000, so `session.history_bytes`
  is 2,727,860 in both `26` and `27`.
- **The clean fix:** small-window v2, beside K6. It answers the END file's code word, with the sum
  recorded and not judged; it sets `[smith] memory = 2147483648`; and it gives astra luna's window.
- **Its tests:** K6's, plus `history_v2_pins_product_memory_and_both_windows` in the harness.
- **Confidence:** high on the probe's parts.

### F7 follow-up cannot fail: no line ever crosses a run

- **Symptom:** in all 26 base and eof traces (ed6af1e8 and 27ead2e7 of both, 62acc437, final 23 and
  25), the second `message.received` follows run 1's `run.completed` in record order. In chat's 39
  traces, the lines arrive during `wait`, as designed, and none is queued at a `finish`. The control
  27ead2e7 passed all three, 1/1 each (13.47). Chat's 2 failures of 12 were the 05.s1 stall, and
  55f8e5de then passed 24 of 24.
- **Root cause:** the cue `when = { tool-call = { tool = "finish" } }` (`follow-up/task.toml:40`)
  fires after a `finish` that is judged within 1 ms (ed6af1e8/01: tool.started and run.completed both at
  1008), so the line lands between runs. And 27ead2e7 is a docs commit of 2026-10-12, after the fence
  and crossing fixes of 2026-10-09 (10183b12 "Reject a finish crossing queued messages", ec232bb6,
  442276a9): it was no "before".
- **The clean fix:** follow-up v2, as later.md:27 says ("several person lines outstanding at once"):
  the cue fires on the first `tool.started` of a run that must read first. The control is the parent of
  10183b12, through a pinned face entry (13.54).
- **Its tests:** `benchmarks/tests/events.rs` `a_cue_on_the_first_tool_lands_before_the_run_completes`.
  Live: 12 per variant, then the control.
- **Confidence:** high that v1 cannot fail. Medium that the old arm runs under today's harness.

### F8 large-write/oversized cannot be forced on a model that honours the bound

- **Symptom:** in `probes-large-write-oversized-9174a8b4-20261012T161758Z`, none of the 12 attempts
  made a call past the bound, and 04 and 06 also failed the file checks (the report counts 9 not
  exercised and 3 deviations). Final 30 made 5 reads and lists, then the two pieces, and never made the
  15,995-byte call. The record holds 2 exercised passes, one each in
  `smith00-large-write-oversized-v9-low-calibration-20261012-0554` and `...-continuation-20261012-0638`,
  of about 24 attempts.
- **Root cause:** each write schema states the bound, "Entire JSON call input max N bytes"
  (`smith-protocol-llm/src/descriptions.rs:113`). 11.1 says it: "a model honouring a bound smith states
  is what smith asks of it". Its rule, "three exercised passes within forty-eight attempts", is half met.
- **The clean fix:** a deterministic world proves the oversize block. A scripted provider sends a call
  past the bound in `tests/protocol-llm` and `tests/session`. The live variant becomes record-only, and
  11.1's sentence becomes "proven by its world; the live variant records what a model does".
- **Its tests:** `tests/session`: `an_oversized_call_returns_as_an_invalid_call_with_its_size`, and
  `the_file_then_arrives_in_pieces_accepted`. No live run is required.
- **Confidence:** high.

### F9 budget/spend and budget/visibility: refused, or undecidable by construction

- **Symptom:** `spend` was refused before inference: a $1.00 ceiling against a $1.32 reserve (13.58),
  and `suites/probes.toml:8` drops it. In `visibility` (8d6cb05a 01–03, final 04), `budget-told` and
  `budget-report` passed 4 of 4, and `budget-visible` was unavailable each time: "child time share
  touches the reserve within submillisecond record precision". Final 04's share is 148,444 ms =
  300,000 − 1,556 used − 150,000 reserve.
- **Root cause:** at the standard profile's window and output, the reserve passes the ceiling. The
  visibility prompt names no time share, and smith says "leaving a part unasked gives the child all of
  that part left" (`prompt.rs:275`). So the share equals the time left exactly, and the checker cannot
  tell equal from past at a millisecond (`benchmarks/src/agent/smith/events/budget.rs:110-132`).
- **The clean fix:** spend v2 declares a window and an output whose two-completion reserve is under
  half the ceiling (still "a ceiling about twice a calibrated spend", 11.1). visibility v2 asks
  `max_turns 2, max_seconds 60` (`sub_agent`'s schema, `smith-protocol-llm/src/tools.rs:101`).
- **Its tests:** `spend_v2_reserve_fits_under_half_its_ceiling` (rendered settings through `smith
  check`), and `a_named_time_share_decides_budget_visible`. Live: 12 each. The controls are a ceiling
  under the reserve (refused) and v1 (unavailable).
- **Confidence:** high.

### F10 tool-output v3 and process-tree v2 need only their calibration runs

- **Symptom:** f2c37fe8 dropped a 250 ms `group_stop` pin and set `grace_ms` to 1000, bumping
  tool-output 2 → 3 and process-tree 1 → 2. Timeout v2 failed 3 of 3 on e1feddd6 (13.65), then passed
  3 of 3 and 10 of 10 on d02d18ee. Final 45 (v3) and 36 (v2) passed. process-tree v1 passed 10 of 11
  (37442dfb).
- **Root cause:** a new task version is new evidence (benchmarks.md 9). Nothing failed.
- **The clean fix:** one `calibrate` batch each: 12 on main, then e1feddd6, which fails the
  `tool-timeout` checks the two tasks share (13.65). Under F13's rule, the v2 calibrations of
  test-output, quote and utf-8 stand.
- **Its tests:** the existing `benchmarks/tests/process_tree_probe.rs`. Live: about 26 attempts of
  seconds each.
- **Confidence:** high.

### F11 slow-completion: the silence was smith's, and too few calls pass 60 s

- **Symptom:** of 27 attempts in seven runs, 10 passed and 9 ran under 60 s ("not exercised"). 6 were
  cancelled after a text stop with no call (38e252c3 02–04, c1dab295 03–04, ee17f028 06), and 2 hit
  skein's `output_items` defect (d104cecb, 71fc935a; 13.70). On 06.s2's arm and later (ae84f57b, final
  37), 5 passed and 2 were not exercised in 7, with no silence.
- **Root cause:** the silence was smith's, a replay-bytes bound that failed the parent, fixed in 06.s2
  (5ccb4f2a's body: "names TranscriptBody ReplayBytes Bound"). 9 not exercised in 27 is past 11.1's
  "more than two in twelve strengthen the prompt, or raise the effort", and the effort is already
  `high` (`slow-completion/task.toml:5`).
- **The clean fix:** v2, with a problem whose certificate needs a longer search, so that at least 10
  calls in 12 pass 60 s.
- **Its tests:** 06.s2's process story. Live: 12 attempts, minutes each.
- **Confidence:** medium. How long the problem must be is a guess until measured.

### F12 The default memory footprint, and what a larger window costs

- **Symptom:** after 10.s4 (080a18df), the defaults price Codex interactive at 1,939,210,982 B and exec
  at 1,925,551,076 B. Anthropic prices interactive at 1,892,802,863 B and exec at 1,879,119,827 B
  (`tmp/smith/reliability/supervision.md:3320`, `log.md:18139`, `:18442`). These are the "about 1.93 GB".
  09deb036's body has 2,148,944,938 B at 6 conversations and 1,995,870,279 B at 5, which is
  153,074,659 B a conversation. 12.3 refused `budget` at 6 conversations at 2,281,755,414 B (13.128).
  The only part breakdown is at 6 conversations (`logs/20261013T015040-3613.log:62-81`): of
  2,040,700,534 B, llm has 699,195,954 (34.3%), local_domain 684,705,795 (33.6%) and store 427,078,818
  (20.9%), then delivery 75,028,305, front 64,626,688 and machine 41,420,870. The rest are under 31 MB.
- **Where it is computed:** `smith check` prices `Local::startup_memory`
  (`crates/smith-local-shell/src/local_host/memory.rs:13-57`, called at `check.rs:370-378`):
  `service::worst_case`, the shell's parts, handed credentials, the front and routes. It refuses
  through `validate::memory` (`smith-agent-service/src/profile/validate.rs:291-293`, "memory:
  service.worst_case=... declared.memory=..."). `Local::memory_parts` (`memory.rs:68-93`) gives the
  parts, but only a test prints them (`settings/tests.rs:464-480`). `check.json` holds
  `declared.memory`, `service.memory` and `inline.memory`, the last being effects and io only
  (771,537,898 for paths). It holds no total and no margin.
- **What scales with the window:** a conversation's history holds the largest configured window at the
  threshold, at `TOKEN_BYTES` = 8 (skein `crates/skein-llm/src/derive.rs:31`; `profile.rs:434-449`).
  That is 1,740,800 B per conversation at 272,000 tokens, plus cells. A request is 48 B a token
  (`llm.request` 13,056,000). The presets differ by 46,408,119 B for 72,000 tokens of window, about
  645 B a token, though they differ in more than the window. At that slope a 500,000-token model fits
  by 61 MB, and a 1,000,000-token model, Anthropic's real input (shell.md 6.1), is 261 MB over.
- **The design today:** shell.md 6.3: "`memory` | 2 GiB | decided as at most 2 GiB; the derived worst
  case must fit it". limits.md 5: "The shell refuses to start when the worst case exceeds it."
- **The clean fix:** Q5's rule. `smith check --json` gains `footprint`: the total, the margin and
  `memory_parts`. Every task pins `memory = 2147483648` (F6). Any parity step that raises a window,
  `conversations` or a per-conversation bound reports the margin.
- **Its tests:** `smith-local-shell`: `check_json_reports_footprint_total_margin_and_parts`, and
  `a_500000_token_model_footprint_meets_the_memory_rule`. Harness: `every_task_renders_the_products_memory_bound`.
- **Confidence:** high on the recorded numbers and the code. Low on the slope, which is extrapolated
  from two presets. One `smith check` with a 500,000-token model settles it.

### F13 Calibration has no per-variant home, and one version covers every variant

- **Symptom:** `Calibration { binary, passes, failed_before }` is on `Task` only
  (`benchmarks/src/task.rs:261-265`, `:342`), and only `credentials` has one. f2c37fe8 changed the
  timeout variant alone, but bumped all of tool-output to v3, so test-output, quote and utf-8 (10/10
  each at v2, e1feddd6) no longer match.
- **Root cause and fix:** the harness theme's T7 gives a block per variant. This theme adds two
  fields. First, the variant's prompt, seed and check digests, which every result already holds
  (`prompt_sha256`, `seed_sha256`): the block stands while they are unchanged. Second,
  `status = "uncalibrated: <reason>"`, so that "none" is recorded and never counted.
- **What each block would say today:**

  | Task / variant | Status | Binary | Control | Passes | Summary |
  |---|---|---|---|---|---|
  | first-request, `headless`; smoke-tools, `child`; failures ×4 | calibrated, first run | per decisions 12.4, 12.16 | first run's | first run's | none committed |
  | large-write | calibrated, v9 | 13.1's arm | 13.1's | 10/11 | none committed |
  | large-write/`oversized` | uncalibrated: not exercised (F8) | — | — | 2 exercised in about 24 | 9174a8b4 |
  | budget `starvation`, `wrap-up`, `child-quota` | calibrated | 5ac25927 | 13.3's | 10/10, 10/11, 10/10 | probes-budget-*-5ac25927 |
  | budget/`spend`, `visibility` | uncalibrated (F9) | 5ac25927, 8d6cb05a | — | refused; 3 unavailable | as named |
  | follow-up, `eof`, `chat` | uncalibrated: control passed (F7) | ed6af1e8 | 27ead2e7, passed | 10/10, 10/10, 10/12 | ed6af1e8 ×3 |
  | history, `small-window` | uncalibrated (F6) | f1a3fdab | 8d9d5408, failed | no batch of ten; 2/5 | f1a3fdab, 8d9d5408 |
  | cache-reuse ×3 | uncalibrated: reuse under 0.85 (F5) | 8d2ec9b8 | none run | 0/3 triplets each | 8d2ec9b8 ×4 |
  | parallel-read | uncalibrated: deviations (F3) | 3f15308c | none | 7/10 | 3f15308c |
  | parallel-children, `reviews` | uncalibrated: deviations (F4) | 3f15308c | none | 2/5, 1/4 | 3f15308c ×2 |
  | paths, `single` | uncalibrated (F2) | e1feddd6 | none | 1/4, 0/3 | e1feddd6 ×2 |
  | tool-output `test-output`, `quote`, `utf-8` | calibrated at v2 | e1feddd6 | 13.65's | 10/10 each | e1feddd6 ×3 |
  | tool-output/`timeout` | uncalibrated at v3 (F10) | d02d18ee (v2) | e1feddd6, failed 3/3 | 13/13 at v2 | e1feddd6 |
  | process-tree | uncalibrated at v2; v1 calibrated | 37442dfb | 13.5's | 10/11 | 37442dfb |
  | credentials | calibrated, no control | c7b79bf3 | "none: no runnable control" | 10/10 | c7b79bf3 |
  | slow-completion | uncalibrated (F11) | ae84f57b | 06.s2's parent | 4/6 | ee17f028, 71fc935a, d104cecb |

- **Its tests:** T7's, plus `a_block_stands_while_its_variant_digests_are_unchanged`.
- **Confidence:** high on the code. The first-run rows rest on decisions 12.4 and 12.16, which this
  sub-agent did not re-read.

## 4. Questions for the designer

1. **Should a probe's answer need no arithmetic?** Options: keep the sums; raise each probe to medium;
   or copy (the values, or the END code word). *Recommendation:* copy, in parallel-read, cache-reuse
   and small-window v2, with sums recorded but not judged. *Why:* the arithmetic is the model's, and it
   failed 5 of 13, 2 of 3 and 6 of 21 attempts.
2. **Does "at most two completions before `finish`" count the finishing response?** Options: yes (now);
   no; or replace the bound with "no `list`". *Recommendation:* no, plus "no `list`" (F2). *Why:* 12 of
   14 attempts read, wrote and finished without searching.
3. **How is oversized proven?** Options: keep 3 exercised in 48; force a model; or a deterministic world
   with the live variant record-only. *Recommendation:* the world (F8). *Why:* a model that obeys the
   bound cannot exercise it, and 11.1 calls that obedience right.
4. **What control does a behaviour already in place get when its "before" predates the harness's faces?**
   Options: a pinned face entry (13.54); a configuration arm; or "none runnable", gating nothing.
   *Recommendation:* a face entry where one commit's reader suffices (follow-up, paths), and otherwise
   "none runnable", gating only as a smoke. *Why:* 3.1: "a probe that cannot fail guards nothing".
5. **The memory rule.** Options: (a) 2 GiB decided, refusing what passes it (today); (b) `memory`
   derived from the configuration, up to a share of the machine; (c) shrink the window- and
   conversation-proportional parts (store 427 MB, local_domain 685 MB) until a 1,000,000-token window
   fits. *Recommendation:* (a), with two sentences in shell.md 6.3. First, "the standard profile keeps
   at least a tenth of `memory` spare; a change that takes any of it names the part and the bytes,
   and `smith check --json` reports both". Second, "a window or `conversations` past what fits is
   refused at startup, naming the part". (c) becomes a parity step only if the plan raises a window or
   adds a sixth conversation. *Why:* the margin is 208 MB, already under a tenth (215 MB). A
   conversation costs 153 MB. Codex's 272,000-token window fits.
6. **The smoke's configuration.** Options: the first-request pin for that task; an empty brief for all
   five tasks; or an AGENTS.md in every seed. *Recommendation:* the pin, plus no AGENTS.md sentence in
   `smoke.json` (F1). *Why:* 4 of 4 failed on it, and the other tasks search too.
7. **Which probes does the parity plan calibrate?** *Recommendation:* small-window v2, cache-reuse v2
   with the no-affinity control, parallel-read v2, and the smoke's first-request, plus tool-output v3
   and process-tree v2, which take about 26 attempts of seconds. In later.md: paths v3 (unless the
   plan changes path text), parallel-children v2 (unless it changes delegation), follow-up v2,
   oversized's world, spend and visibility v2, and slow-completion v2. *Why:* a probe proves a change
   only if it fails before it, and the rest guard code the plan does not touch.
8. **How far does a task version reach?** Options: the file (today); a version per variant; digests
   per block. *Recommendation:* digests per block (F13). *Why:* one pin voided three calibrations.
9. **Is "four at once" a policy value?** *Recommendation:* name it in session.md 5 and limits.md 2.3,
   as `limits.rs:286` has it. The parallel-tools theme decides its value. *Why:* the design says
   "adjacent reads ... run together", while the code ran 20 reads in 5 waves.
10. **Effort for delegation and read probes.** *Recommendation:* v2 prompts at low effort first, and
    medium only with the reason recorded when more than 2 in 12 deviate. *Why:* 3.1 allows it, and
    medium costs reasoning tokens in every attempt.

## 5. What other themes must say

- **Compaction:** small-window v2 takes F6's three changes beside K1 and K6; its before already failed
  (8d9d5408). Whether the threshold experiment reruns on v2.
- **Cache:** smith's reuse is about 0.62, 0.15 and 0.60 (base, children, crossing) against 0.85, and
  Codex's 172,800 of 187,569: why, and whether 0.85 is attainable. v2 copies its answer either way.
- **Harness:** T7 gains digests and `status` (F13); the smoke's configuration (F1); the product's
  `memory` in every task (F6, F12); `before = "finishing"` (F2); the triplet verdict in its group (F5).
- **Parallel tools and delegation:** the cap of four (Q9), with `parallel_tool_calls` already true, and
  whether smith's text should push independent calls into one response: children came one at a time
  in 29 of 47 attempts.
- **Prompts:** `finish` comes in a response of its own in 15 of 15 compare attempts (P6) and in 12 of
  14 paths attempts (F2). Codex's final message has the same shape, so this is no gap.
- **Limits:** a step raising `window`, `conversations`, `tool_payload` or `llm_pool` reports the
  footprint and margin from `smith check` (F12, Q5), not an estimate.
