# Findings: compaction

2026-10-14. Why half of the small-window attempts lose their run at or after compaction, and what
the parity plan should change. Written by a read-only sub-agent for the parity plan's designer
(skein's development.md, section 4.1).

## 1. In one page

- **Every compaction on record was asked with the conversation's own tool choice, `auto`,** in 43 of
  43 compaction requests on this machine. 9 of the 37 on `gpt-6-luna` answered with a call (6
  `finish`, 3 `read`); 8 of those had no text, and each ended the run `Failed(model/malformed)` at
  once. The 6 on `gpt-6-astra` (repository) had no calls. (K1)
- **`ToolChoice::None` with the tools kept is the clean fix.** On Codex, the one large request asked
  with `none` read 51,840 cached tokens of a previous prompt of 51,954
  (`repository-a64f0523-20261013T230741Z/02`, response 44), and 0 of 80 requests asked with `none`
  answered with a call. On Anthropic, a change of `tool_choice` keeps the tools and system caches
  but not the messages cache, so a compaction there would write its history once per window. (K1)
- **The summary rewrites the task's words, and the answer follows the summary.** In all 4 attempts
  that answered `ALPHA528`, the last summary itself said `ALPHA528`; in the 2 whose summary said
  `ALPHA 528`, the answer passed. The instruction asks the summary to "Keep the task ... and
  constraints", which the new window already carries verbatim. (K2)
- **The summary is every text block glued together, with no separator and no framing.** Two of 20
  compaction answers in 12.6 began with a block of working notes; in attempt 04 the notes ("Sum
  1..16=136. Their exact output instruction after 32nd read.") opened the new window and the model
  finished at once with `ALPHA136`. (K3)
- **The one wrong sum came from the summaries, not from the reads.** Attempt 08's first summary
  claimed 11 files read after 10; its second, after reading values 11-20, said "Files 12–20 had
  values 11–19" (sum 201, true 210); its third added 255 as 225: 489 instead of 528. The probe's values are 1 to 32 in order, so a summary can say "values
  1 through 16" without carrying the sum. (K4)
- **The nominal threshold is not where compaction happens.** At 4/5, compaction requests were 53.3%
  to 71.2% of the declared window (6 repository compactions) and 61.2% to 61.4% (probe); at 3/5,
  39.1% to 39.4%. 12.6 detected no difference between 80% and 60% (1/5 each), and malformed answers
  swamped it. Recommendation for decision 4.7: keep 4/5, measure again after K1-K4. (K5)
- **`history/small-window` cannot calibrate as authored**: 5 of 21 compacting attempts passed; its
  answer check is stricter than its prompt; its values are predictable; no event check looks at the
  compaction's choice. A version 2 with a `compaction-choice` check gives a certain before on the
  parent. (K6)
- **Codex frames its summary and survives a failed compaction.** Its binary carries a handoff prompt
  ("You are performing a CONTEXT CHECKPOINT COMPACTION ..."), a preamble for the new window
  ("Another language model started to solve this problem ..."), and "Post-turn compaction failed;
  preserving the completed turn". No Codex compaction is recorded on this machine. (K7)
- **smith's benchmark arms declare half the working model's window.**
  `benchmarks/src/agent/smith/settings.rs:148` sets 131,072 for the selected model; Codex used
  258,400 of 272,000. All 3 smith repository attempts compacted twice; no Codex attempt compacted,
  at up to 131,454 input tokens. (K8)

## 2. Entries

Sources: smith `main` at `689c15d0`, skein `bec13dc`, raw runs under
`~/.local/state/smith-bench/runs/` (named by run/attempt), and the private Codex 0.160.0 package at
`~/.local/state/smith-bench/tools/codex-0.160.0` (read with `strings`; never run). Every number
below comes from a `jq` or `strings` query over those records.

### K1 The compaction is asked with the conversation's tool choice, and a call ends the run

- **Symptom:** In `compaction-8a9747fa-20261014T004156Z`, every compaction request
  (`response.started` of the `compaction` conversation) had `"tool_choice":"auto"`, its last user
  message being the summary instruction (`prompt.messages[-1]`). Per attempt, from
  `response.completed` and `run.completed`:

  | Attempt | Arm | Compactions: answer per compaction | End |
  |---|---|---|---|
  | 01 | 80% | text, text | accepted `ALPHA528` |
  | 02 | 80% | text, call `finish {"form":"report","text":"ALPHA 528"}` (invalid 1) | `Failed(model/malformed)` |
  | 03 | 80% | text+text, call `finish` with prose and `"fields":{}` | `Failed(model/malformed)` |
  | 04 | 80% | text+text | accepted `ALPHA136` |
  | 05 | 80% | text, text | accepted `ALPHA 528` (pass) |
  | 06 | 60% | text, text, text | accepted `ALPHA 528` (pass) |
  | 07 | 60% | text, text, call `read node-1e619b65.txt` | `Failed(model/malformed)` |
  | 08 | 60% | text, text, text | accepted `ALPHA 489` |
  | 09 | 60% | call `read node-c20c1991.txt` | `Failed(model/malformed)` |
  | 10 | 60% | call `read node-c20c1991.txt` | `Failed(model/malformed)` |

  The 80% arm's second compaction always fell after read 32 (window 2 had 16 reads, 34 messages),
  just before the final answer: that is where both `finish` calls came. Across all runs: 43
  compaction requests, all `auto`; 34 text, 8 call only, 1 call and text
  (`probes-858c9642-20261012T224936Z/25-rerun`, accepted, calls not run). The 8 call-only answers:
  12.6's 02, 03, 07, 09, 10; `probes-62acc437-20261013T212404Z/27-rerun` (a `finish` whose text was
  the summary); `probes-history-small-window-f1a3fdab-20261012T183428Z/01` and `/03` (`finish` calls
  after read 32). Compaction requests on `gpt-6-astra` at high effort (repository, 6) had no calls.
- **Root cause:** The request is built by `complete()`
  (`crates/smith-domain-session/src/session.rs:2163`) with `tools: conversation.tools` (:2198) and
  `choice: conversation.choice.clone()` (:2201) whatever the purpose (:2186). The retrospective's
  `compaction.rs:107` and `:256` are the choice's byte counts in `prompt_bytes` and `new_bytes`, not
  where the choice is chosen. A text-free answer ends the session: `compaction.rs:326-335`,
  `Stop::EndTurn | Stop::ToolUse | Stop::Refusal => crate::NoSummary::Malformed` (:331). The design
  asks for it: session.md 8.2, "The compaction completion has the same model, affinity, system text,
  tools and tool choice as the completion it stands in for", and "Calls in the compaction's answer
  are not run." The system text tells the model "When the work is done, call `finish` with its
  result ... Stopping without `finish` does not finish the run" (12.6/04, response 18
  `prompt.system`); 6 of the 9 call answers were `finish`.
- **The clean fix:** The compaction keeps the tools and asks with `ToolChoice::None`, as a child's
  final turn already does (`session.rs:1845`; session.md 5: "The session sets `None` on its own
  final turn (section 7)"). In `complete()`, `choice` becomes `None` while
  `conversation.compaction.asking()`; the conversation's own choice is untouched, so the new window
  asks as before. `choice_bytes` charges `None` as 0 (`session.rs:743-751`), at most what `Only`
  charges, so admission still holds. Both dialects render it with the tools kept: Codex
  `"tool_choice":"none"` after `"tools"` (skein `crates/skein-llm/src/openai/request.rs:113-117`;
  `Only` renders as `"auto"`); Anthropic `"tool_choice":{"type":"none"}` after `tools`
  (`anthropic/request.rs:274-285`).
  - Cache: on Codex the record says `none` keeps the prefix (51,840 read of 51,954, above).
    Anthropic's caching documentation (the claude-api skill's `shared/prompt-caching.md`,
    "Invalidation hierarchy"): "you can change `tool_choice` per-request without losing the
    tools+system cache", while the messages cache does not survive it. skein's fake models that
    Anthropic rule (`testing/skein-fake-llm-domain/src/cache.rs:369`, "tools/system remain cached
    across a choice change") but also hashes the choice before messages for automatic caching
    (`cache.rs:113`, `:126`, `:149`), which the Codex record contradicts.
  - Dropping the tools would change the prefix's start on both providers and lose every cache;
    keeping `auto` with a stronger instruction leaves a rate the record puts at 9 in 37 on the small
    tier.
  - "Calls in the compaction's answer are not run" stays, for a provider that writes one anyway
    (skein's fake does so by chance, fake-llm.md 2.1). *Malformed* stays the fault for a text-free
    answer; with `None` it becomes a provider deviation (0 of 80 recorded), not a likely answer.
  - Design: session.md 8.2 becomes "the same model, affinity, system text and tools as the
    completion it stands in for, asked with `ToolChoice::None`"; its "Nothing before them differs
    from an ordinary prompt, so the request reads the cached prefix" says that the choice is outside
    the cached prefix on Codex and that Anthropic keeps only the tools and system caches; session.md
    5 "on its own final turn (section 7)" adds "and on a compaction (8.2)". decisions.md 1.1 names
    no choice and can stay.
- **Its tests:**
  - `tests/session/tests/compaction.rs`: `a_compaction_asks_with_none_and_keeps_its_tools` (the
    summary request's `choice` is `None`; its system, tools, namespace and delegated equal the
    previous prompt's; its messages extend it);
    `a_narrowed_conversation_compacts_with_none_then_keeps_its_choice` (a conversation at
    `Only(finish)` compacts at `None`, and the new window's first prompt is at `Only(finish)`
    again); extend `a_final_note_due_during_a_compaction_retry_waits_for_the_new_window` (:370) to
    assert the compaction's first request and its retry are at `None`;
    `a_call_only_compaction_answer_under_none_fails_malformed` (a call-only answer still ends
    `NoSummary::Malformed`, its call not run).
  - `tests/protocol-llm/tests/production_render.rs`:
    `a_compaction_renders_its_tools_with_choice_none` for the Codex and Anthropic dialects.
  - skein: `testing/skein-fake-llm-domain/src/cache.rs`,
    `an_automatic_query_reads_its_prefix_across_a_choice_change` once the fake follows the Codex
    record (section 4).
  - Live: `history/small-window` v2 (K6) with its `compaction-choice` check, calibrated: 10 passes
    within 12 on the fix; before: the parent fails that check on every attempt that compacts.
- **Confidence:** high that the choice causes the malformed ends (8 of 8 are call-only answers to
  `auto` requests). Medium that `None` keeps the Codex cache: one large sample; the 79 other `none`
  requests are at most 1,151 tokens and read 0 cached tokens each, as did the ordinary requests
  before them. A smith-on-Anthropic `cache-reuse` run around a compaction would settle the Anthropic
  cost.

### K2 The summary restates the task, and the answer follows the restatement

- **Symptom:** Small-window attempts whose last summary named the answer: `ALPHA528` in 4 (12.6/01,
  `probes-858c9642-20261012T224936Z/25` and `/25-rerun`,
  `probes-history-small-window-f1a3fdab-20261012T183428Z/05`), each answering `ALPHA528`; `ALPHA
  528` in 2 (12.6/05, f1a3fdab/04), each passing. 12.6/01's last summary ends "Final response must
  be exactly `ALPHA528`." Base `history` (window 272,000, no compaction) answered `ALPHA 528` in all
  5 attempts that read the whole chain.
- **Root cause:** The instruction is the run's (run.md 3.3: "what a summary must keep for the work
  to go on"), in `crates/smith-domain-run/src/prompt.rs:30`: "Summarise the work so far so it can
  continue in a new context window. Keep the task, decisions, changes made, current results, pending
  work and constraints. Be concise and concrete." The new window already opens with the task's own
  words (`compaction.rs:212`; session.md 8.2 "the conversation's task (its opening message, carried
  from window to window)"), so asking the summary to keep the task and constraints asks for a
  paraphrase beside the original, and the model trusted the paraphrase. Nothing in the instruction
  says the task goes on, that no tool may be called, or that exact values must be copied.
- **The clean fix:** New words in `prompt.rs:30`, owned by run.md 3.3, for example: "The work is not
  finished: a new context window will continue it, opening with the task's own words. Answer with
  text only; call no tool. Write what the next window needs that the task does not say: what was
  done and found, with exact values, names and paths as they were read; what remains; decisions
  taken. Do not restate the task or its required answer." The instruction's bytes count in the
  startup `compaction-headroom` check (`prompt.rs:35`; protocol/limits.md 3.9), so longer words
  lower derived bounds slightly. run.md 3.3 can stay as written; its words change.
- **Its tests:** `crates/smith-domain-run`:
  `the_summary_instruction_fits_the_compaction_headroom_derivation` (a charter at the smallest
  declared window still passes startup with the new words). Live: `history/small-window` v2, whose
  answer check states the format (K6); the measure is that no attempt's answer differs from the
  format the task states.
- **Confidence:** medium. 6 of 6 summaries that named the answer predicted its form; the counts are
  small, and a stated format in v2 would hide the defect from the probe while it stays in the
  product.

### K3 Working notes and summary are glued into one block with no framing

- **Symptom:** 12.6/03 and /04 first compactions had two text blocks; the first read as notes ("We
  need answer summary, but user earlier says no summary? ..."). In /04 the new window's opening
  second block was "...Summary should state constraints.Task: Follow the 32-file pointer chain ..."
  (response 18 `prompt.messages`), and the next completion was `finish {"text":"ALPHA136"}` after 16
  of 32 reads.
- **Root cause:** `accept` takes `super::text_of(&completion.content)` (`compaction.rs:341`), which
  concatenates every text block with no separator (`session.rs:2239-2259`). session.md 8.2: "The
  summary is the text of the answer's text blocks, in order." Codex message items carry a `phase`
  (`commentary`, `final_answer`) in the block's replay (skein
  `crates/skein-llm/src/translate.rs:706-707`), which the session cannot read; the events do not
  record which phase /04's first block had. The new window gives the summary no heading: session.md
  8.2 "one user message: the conversation's task ..., the summary (the answer's text), then the
  held-back messages."
- **The clean fix:** Join text blocks with a blank line, and open the summary with a run-owned
  preamble beside the summary instruction (run.md 3.3), for example "Summary of the earlier context
  window, written by you before it was compacted:". session.md 8.2's two sentences above gain
  "separated by a blank line" and "the run's summary preamble, then the summary". Whether to keep
  only the `final_answer` phase is a dialect question, left for later.
- **Its tests:** `tests/session/tests/compaction.rs`:
  `a_summary_of_two_text_blocks_opens_separated_by_a_blank_line`;
  `a_new_window_opens_with_task_preamble_summary_and_held_messages`. Live: v2's attempts, read for
  the preamble in each window's first `response.started`.
- **Confidence:** medium for the mechanism (the glued text is in the record); low that it alone
  caused /04's early finish.

### K4 The summaries carried wrong state, and the probe's values make that easy to miss

- **Symptom:** 12.6/08 read values 1-10, 11-20, 21-30, 31-32 in its four windows (`prompt.messages`
  results, `value: N`). Its summaries: "11 files read ... Values were 1 through 11, sum = 66" (after
  10 reads); "Files 12–20 had values 11–19, bringing the running sum to **201**" (true 210); "30
  files read. Running sum is **426**" (true 465; 201 + 255 = 456 by its own count). Answer `ALPHA
  489`. Every passing summary described values as a range ("values 1 through 16").
- **Root cause:** The model computed the sum and the summary carried only the result; the
  instruction does not ask for exact data (K2). The seed's values are 1..32 in chain order
  (`benchmarks/tasks/probes/history/seed/node-93e158a7.txt:1` "value: 1"), so a summary can be right
  without carrying the sum, and the probe does not test what compaction must preserve.
- **The clean fix:** K2's words ("exact values ... as they were read"). In the probe's v2, values
  that are not in order (K6).
- **Its tests:** Live only: v2 with shuffled values; the measure is the share of summaries whose
  running sum equals the sum of the values read before them (a new passive check, or an audit of
  each attempt).
- **Confidence:** high for 08's trail; low for how often it happens (1 of the 11 attempts that
  compacted and completed).

### K5 The threshold: what 80% and 60% mean in practice

- **Symptom:** 12.6: both arms 1/5, exact 95% interval [0.00505, 0.71642]; compactions median 2 at
  80%, 3 at 60%; 11 comparisons, none detected (task wall ratio 1.10632, MDE 0.97515; cache read
  1.70441, MDE 0.83241; common priced spend 1.18738, MDE 1.07995). Compaction requests by size
  against the declared window: 80% arm 61.2%-61.4% (12.6/01), 60% arm 39.1%-39.4% (12.6/06);
  repository, 80%: 59.0%, 71.2%, 71.2%, 65.5%, 53.3%, 68.4% of 131,072. Malformed answers by arm: 2
  of 9 compaction requests at 80%, 3 of 11 at 60%. Both wrong sums trace to summaries (04 at 80%, 08
  at 60%).
- **Root cause:** The default is `compaction_threshold: Fraction { numerator: 4, denominator: 5 }`
  (`crates/smith-agent-service/src/profile/quantities.rs:184`), decisions.md 4.7 "about 80% of the
  usable window". The check counts the last prompt, its output, the appended bound and the whole
  output reservation against window times threshold (session.md 8.1), and the appended bound counts
  a byte as a token; so in the probe (output 8,192, 8 KiB reads) compaction comes at about 61% of
  the window by tokens sent.
- **The clean fix:** No code change. Keep 4/5 (section 3, Q5). The experiment reruns after K1-K4 on
  v2, ten per arm, since 12.6's five per arm had an MDE near a ratio of 1.
- **Its tests:** Live: the `compaction` suite (`benchmarks/suites/compaction.toml`, arms
  `80-percent` and `60-percent`) on v2 after the fixes; about $0.16 per attempt (12.6: $1.615145 for
  10).
- **Confidence:** medium that fewer compactions are safer: of the 16 failures among K6's 21
  compacting attempts, 14 began at a compaction (8 malformed, 4 `ALPHA528`, 2 wrong sums) and 2
  stopped before any. Low on any cost difference.

### K6 `history/small-window` cannot calibrate as authored

- **Symptom:** 21 small-window attempts on binaries that compact (12.6's 10;
  `probes-62acc437-20261013T212404Z/27`, `/27-rerun`; `probes-858c9642-20261012T224936Z/25`,
  `/25-rerun`; `probes-f2c37fe8-20261013T214945Z/27`, `/27-rerun`;
  `probes-history-small-window-f1a3fdab-20261012T183428Z/01`-`05`): 5 pass, 8 malformed, 4
  `ALPHA528`, 2 wrong sums, 2 stopped before any compaction (`ALPHA 36` after 9 reads; "I can’t
  complete the chain in this response."). The 00.16 calibration: arm f1a3fdab 2 passed of 5
  (`tmp/smith/reliability/logs/smith-00/00.16-small-calibrate.log`), against "ten passes within at
  most twelve attempts" (benchmarks.md 3.1). Its control,
  `probes-history-small-window-8d9d5408-20261012T183429Z/01`, did fail as a before should: no
  compaction, `prompt-within-window` false, "used=78587, bound=65536". The report: "None calibrated:
  base rerun passed; small-window 2 passes/3 model deviations, control failed; next task version
  deferred".
- **Root cause:** Three task faults and one smith fault. The prompt says "finish with exactly ALPHA
  followed by the sum" and the check is `pattern = "^ALPHA 528$"`
  (`benchmarks/tasks/probes/history/task.toml`, 12.6/01's copy). Values in order (K4). No event
  check looks at the compaction request; the nearest is `final-turn`, which reads
  `request.tool_choice` (`benchmarks/src/agent/smith/events/final_turn.rs:13`, `:49`). And K1 makes
  the malformed ends smith's, not "model deviations", so they cannot count among calibration's
  allowed failures.
- **The clean fix:** Task `history` version 2: "finish with exactly the word ALPHA, one space, then
  the sum"; values shuffled; the chain kept at 32 files, so the 80% case of a compaction just before
  the final answer stays exercised. A new event check `compaction-choice` (every request of a
  `compaction` conversation at `tool_choice` `none`), modelled on `final-turn`, in `small-window`'s
  `[[variant.event]]`. v2 resets both variants' calibration, base `history` included.
- **Its tests:** `benchmarks`: `compaction_choice_fails_an_auto_compaction_request` and
  `compaction_choice_passes_when_every_compaction_asks_none`, on recorded traces such as 12.6/02.
  Live (benchmarks.md 4.2): after, v2 `small-window` on the fix, 10 passes within 12; before, the
  same on the parent, which fails `compaction-choice` on every compacting attempt (43 of 43 recorded
  `auto`), so one attempt suffices; base `history` v2 recalibrated beside it.
- **Confidence:** high on the faults; medium that v2 reaches 10 of 12 on `gpt-6-luna` at low effort,
  given 2 early stops in 21. If not, the probe names medium effort, as benchmarks.md 3.1 allows.

### K7 What Codex does, as far as this machine shows

- **Symptom:** No Codex rollout on this machine compacted: 60 `rollout-*.jsonl` under the runs, none
  with a compaction item; the largest input was 131,454 tokens
  (`repository-a64f0523-20261013T230741Z/05`) against `model_context_window` 258,400.
- **What the binary holds** (`strings bin/codex`):
  - The request: "You are performing a CONTEXT CHECKPOINT COMPACTION. Create a handoff summary for
    another LLM that will resume the task. Include: - Current progress and key decisions made -
    Important context, constraints, or user preferences - What remains to be done (clear next steps)
    - Any critical data, examples, or references needed to continue. Be concise, structured, and
    focused on helping the next LLM seamlessly continue the work."
  - The new window's preamble: "Another language model started to solve this problem and produced a
    summary of its thinking process. You also have access to the state of the tools that were used
    by that language model. Use this to build on the work that has already been done and avoid
    duplicating work. Here is the summary produced by the other language model, use the information
    in this summary to assist with your own analysis:"
  - The failure policy: "Post-turn compaction failed; preserving the completed turn", "Post-turn
    compaction completed without an assistant summary", "(no summary available)", "Failed to run
    pre-sampling compact", and a warning, "Long threads and multiple compactions can cause the model
    to be less accurate."
  - Paths `core/src/compact.rs` and `core/src/compact_remote_v2.rs` (local and remote compaction),
    settings `model_auto_compact_token_limit` and `model_post_turn_compact_threshold_percent` ("must
    be between 0 and 100"); the bundled entry for `gpt-6-luna` has `"context_window": 272000`,
    `"max_context_window": 872000`, `"auto_compact_token_limit": null`.
- **What it does not show:** what tool choice and tools Codex's compaction sends, and its default
  limit when `null`.
- **For smith:** the framing (K3) and the request's "critical data" line (K2) match Codex's. Codex
  compacts after a turn as well as before sampling, and a failed post-turn compaction keeps the
  turn; smith fails the run. See Q2.
- **Confidence:** high for the strings; low for behaviour.

### K8 smith's benchmark arms declare half the working model's window

- **Symptom:** `repository-a64f0523-20261013T230741Z` 01-03:
  `declared.window[endpoint=codex,model=gpt-6-astra]=131072`, output 4,096; each compacted twice
  (largest prompts 93,392, 93,426, 89,761). Codex 04-06 compacted none, at up to 119,639, 131,454
  and 122,098 input tokens. The fixture comparison `compare-a57ca3a8-20261013T221135Z/01` declares
  the same 131,072.
- **Root cause:** `benchmarks/src/agent/smith/settings.rs:148`, `model.insert("window".into(),
  json!(131_072))`, for the selected model unless a task's `[smith] window` overrides it. The
  product's presets give 272,000 (`crates/smith-local-shell/src/presets.rs:51`, `:58`).
- **The clean fix:** The harness takes the window from the product's preset (benchmarks theme), so
  the comparison compacts where the product would.
- **Its tests:** `benchmarks`: `smith_settings_declare_the_presets_window_for_the_working_tier`.
  Live: the next repository run records each arm's declared window.
- **Confidence:** high.

## 3. Questions for the designer

- **Q1. How does a compaction refuse calls?** Options: (a) keep the tools, ask with
  `ToolChoice::None`; (b) drop the tools; (c) keep `auto` and rely on the instruction; (d) let the
  dialect pick from `Purpose::Compaction` (`crates/smith-domain-session/src/llm.rs:242`).
  Recommendation: (a). Why: it is the final turn's mechanism already, the Codex record keeps the
  cache under it, Anthropic keeps tools and system, (b) loses every cache, (c) failed 9 of 37 times,
  and (d) moves a domain rule into dialects.
- **Q2. Does "a window is compacted once, neither retried nor nudged" stay?** Options: (a) keep it;
  (b) allow one identical re-ask on a text-free answer, nothing appended (fits the headroom, which
  holds one request, not an exchange); (c) on a failed compaction after an answer is ready, keep the
  answer, as Codex keeps its turn. Recommendation: (a) with K1, and (b) only if v2 shows a malformed
  end under `None`. Why: 0 of 80 `none` requests answered with a call; a nudge appends a message the
  headroom does not hold (session.md 8.1).
- **Q3. Is the summary framed?** Options: a run-owned preamble before the summary (run.md 3.3), a
  blank line between blocks, or only the last text block. Recommendation: preamble and blank line.
  Why: K3; Codex frames it; keeping only the last block drops a summary split across blocks.
- **Q4. The summary instruction's words.** Options: K2's proposal; Codex's words; today's.
  Recommendation: K2's, which keeps Codex's "critical data" line, drops "Keep the task ... and
  constraints", and says no tools and that the work goes on. Why: K2 and K4.
- **Q5. The user's decision 4.7, the threshold.** Options: 4/5 (today); 3/5. Recommendation: keep
  4/5, re-measured on v2 after K1-K4 with ten per arm. Why: no difference detected in 12.6; 3/5
  compacts more often (median 3 against 2) and each compaction is a lossy step; the effective
  trigger at 4/5 is already 53%-71% of the window. Alternative: 3/5, if the user values smaller
  prompts per completion over fewer summaries; no measurement favours it.
- **Q6. `history` version 2.** Options: format stated, values shuffled, `compaction-choice` check
  (all three); format only. Recommendation: all three, and 32 files kept. Why: K6; shuffled values
  make the probe test what a summary must carry.

## 4. What other themes must say

- **Benchmark tasks:** `history` v2 (K6), resetting base and `small-window` calibration; the
  `compaction-choice` event check; the harness's 131,072 window (K8) before any parity comparison of
  long tasks; the `compaction` experiment rerun on v2 after the fixes, ten per arm.
- **Cache reuse:** a compaction asked with `None` must still read the prefix on Codex (one sample,
  K1); skein's fake hashes the choice before messages for automatic caching (`cache.rs:113`, `:126`,
  `:149`), so a world test of cache reuse around a compaction fails against the fake until it
  follows the Codex record; on Anthropic the compaction writes its history once per window, to be
  measured when smith runs there.
- **Prompts:** the summary instruction (`smith-domain-run/src/prompt.rs:30`) and a new summary
  preamble (K2, K3) are the run's words; the system text's "Stopping without `finish` does not
  finish the run" pulls a summary request toward `finish` (K1).
- **Failures:** `NoSummary::Malformed` stays, rarer; a compaction that fails after the work is done
  loses an answer the model had (12.6/02's `finish` held `ALPHA 528`), which Q2 (c) would keep; 12.6
  counted these ends as the model's; K1 shows they are smith's.
