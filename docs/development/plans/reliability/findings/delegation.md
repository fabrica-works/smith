# Findings: sub-agents, delegation and run budgets (D)

## Summary

The repository task failed under budgets for the same reasons each time, and the turn count was not one of them. A child opened without a share gets the whole run remainder. When it spends that remainder, the run goes `Over` and denies the main its next completion, so the main never sees the child's result and never calls `finish`. Neither the main nor the child is ever told its turns, time or spend.

The spike did two things. It raised the standard turn ceiling 64 → 128 → 256, which only moved the cliff. It also exposed the existing typed `Share` as an optional `sub_agent.max_turns`, which let the parent survive a child that ran out. Both successes used fewer than 128 turns. The last attempt failed on *time* with 81 turns unused: the parent was blocked in six serial write-granted children for 905 of 1,200 s, and it started a 25-turn child with about 90 s left.

The clean fix is a budget model:
- Time and spend are the planning bounds; turns are a loop guard.
- The main keeps a finishing reserve that children cannot touch.
- Children are told their share and get a wrap-up turn.
- Remaining budget reaches the model as text that the transcript keeps.

The scheduling work then follows in order: guidance and parallel inspect-only children now, a run-level write lease next, asynchronous handles and scoped writes in a later document.

### Evidence base: repository attempts (task 07, Astra/high)

| attempt | turn budget | child caps | completions (main + children) | delegated wall | end |
|---|---|---|---|---|---|
| iteration5-smith-1 | 64 | none, none | 18+35+11 = 64 | ≈316 s | `Failed(Budget(Turns))` at 409 s; main ended in the same step as child 2 |
| iteration6-smith-1 | 128 | none ×3 | 20+28+23+57 = 128 | ≈845 s | `Failed(Budget(Turns))` at 967 s; child 3 took 57 of the 58 left |
| iteration7-smith-1 | 256 | 28, 32 | 65+28+32 = 125 | ≈453 s | accepted at 997 s; child 2 ended `unanswered end=budget:turns` |
| iteration7-smith-2 | 256 | 20 | 51+20 = 71 | ≈116 s | accepted at 563 s; the child's first write was at its turn 19 of 20, then it was cut |
| iteration7-smith-3 | 256 | 32,40,35,45,25,25 | 38+23+36+13+34+16+15 = 175 | 905 s | `Failed(Budget(Time))` at 1,200 s; sixth child opened at 1,109 s |

Sources:
- `runs/<attempt>/agent/trace.jsonl`: usage per owner, `Run Opened/Ended/Returned/Answered` facts, `sub_agent` inputs.
- `runner.stdout` `delegation_durations`.

All 15 child calls asked for `["inspect","modify","shell"]`. Native Codex finished in 397–467 s with two asynchronous children overlapping the root and each other.

---

### D1. The standard turn ceiling 64 → 128 → 256

- **Origin:**
  - `5848e59` (`crates/smith/src/limits.rs:9`, 64 → 128).
  - `44e5120` (128 → 256).
  - Report row 84 and lines 97–101.
- **Symptom:** iteration5 (64) and iteration6 (128) both ended `Failed(Budget(Turns))` while a child was working. `run-capacity-analysis.json` and `budget-accounting-review.json` confirm accounting was exact: no double billing.
- **Root cause:**
  - A child with no share is opened with `Budget = left` (`smith-domain-run/src/agent.rs:102-106`, spike tip).
  - Global exhaustion sets `State::Over` (`run.rs:696`). `completion_permit` then denies every conversation, the main included (`run.rs:785`).
  - In the trace the main ends in the same step as the child (409.3 s and 966.8 s).
  - 64 was too small for this task class: Codex used 106–116 responses, and Smith's successes used 71 and 125. But no ceiling fixes "first child takes everything, parent gets no last word"; at 128 the same mechanism failed at 128.
- **Spike change:**
  - One preset constant (`BUDGET.turns`; `CEILING.turns` is derived from it).
  - `crates/smith/tests/run_capacity.rs`: 64/128/256 admitted, 257 refused.
  - `smith-domain-run/src/tests.rs:1442`: loops over 64/128/256 using the 18/35 split taken from the iteration5 trace.
  - Docs in `local-host.md`, plus dated validation logs appended to `workflow.md`.
- **Assessment: redo properly.**
  - The ceiling is a sanity cap with no memory cost, so raising it is harmless, but 256 has no derivation. The report concedes at line 101 that both successes fit under 128.
  - 128 plus the cap would have passed both successes. 256 bound nothing in the attempt that failed, which hit time at 175 turns.
  - A single session cannot use 256 turns anyway: 256 messages (`config.rs:368`) is about 127 turns, and the 1 MiB request size limit usually binds earlier.
  - The tests pin the constant. The domain test re-proves accounting that `tests/run/tests/simulation.rs:213` and `a_sub_agent_that_spends_past_the_budget_winds_the_run_down` already cover.
- **Clean fix:**
  - A generous, derived turn ceiling (for example time ceiling ÷ minimum completion latency), documented as a loop guard.
  - The work bound moves to time and spend (D2), with a parent reserve (D3).
  - `Invalid::Budget` should name the dimension and the admitted maximum. iteration4 learned the 64 ceiling only by refusal.
- **Contracts:**
  - run.md §9 holds.
  - `protocol/agent.md` §4 should give the profile's ceilings with their rationale.
  - The spike's `local-host.md` text is accurate. The `workflow.md` logs should not carry over.
- **Layer:** `smith` (profile), `smith-domain-run` (refusal detail).
- **Tests:** replace the pinned 256/257 test with "configured ceiling admitted; ceiling + 1 refused before effects, naming turns". Drop the 64/128/256 loop, which D3's tests subsume.
- **Benchmark:** measured but misattributed (D12). The probe lives in D3.
- **Question:** should turns stay in the charter? Recommendation: keep them as a generously admitted guard, never the planning unit.

### D2. Spend is the contracts' primary bound, but the standard profile disables it (not in spike)

- **Symptom:**
  - Every benchmark run used `"spend": 1` with zero prices.
  - `/tmp/smith-benchmark-smith-research/findings.txt`: "spend1 avoids currency confounds… Max64 turns… cannot exceed via JSON settings."
- **Root cause:**
  - The standard ceiling is `spend: 1` (`limits.rs:9`), and session `spend: 1` (`:63`). Admission refuses anything larger (`charter.rs:186`).
  - Reservations round up, so any non-zero price exhausts 1 unit at the first charge. `local-host.md`'s example prices (10/2/30 per 1M tokens) cannot fund more than one completion.
  - The local host never checks spend against the agent's ceiling (`local_settings.rs:462`).
  - run.md §3.1/§9 define the budget first as spend. In practice turns became the only cost proxy, and the worst one: turns do not compare across tool granularity or providers. Codex batches commands into one call; Smith's fresh input was 12.4× Codex's at similar turn counts.
- **Assessment: redo properly.** A global turn count is not the right primary bound for large tasks.
- **Clean fix:**
  - run.md §9 assigns the roles: time is the latency and planning bound, spend is the cost bound, turns are a loop guard.
  - Flat-rate hosts set notional prices (a token-weighted budget that exposes uncached-input cost).
  - Raise the standard spend ceiling to a documented large `u64`.
  - The local host refuses impossible spend settings with a clear message before spawning anything.
  - Alternatives:
    - A separate run-level token budget, which duplicates prices at weight 1.
    - Time visibility only, which leaves cost unbounded for subscription use.
- **Contracts:** run.md §3.1, §9; `protocol/agent.md` §4; `local-host.md` budget table.
- **Layer:** `smith`.
- **Tests:**
  - The profile admits a realistic priced charter.
  - Local settings refuse an impossible spend before start.
  - A run-world story where spend (not turns) exhausts and the reserve still lets the main finish.
- **Benchmark:** not measured, because spend was off. Probe: notional prices with about 2× a calibration spend. Assert accepted or `Failed(Budget(Spend))`, never overspent, and report spend per accepted run.
- **Question:** make spend real in the standard profile and document notional prices? Recommendation: yes.

### D3. A child's default share is the whole remainder, and exhaustion gives the main no last word (not in spike)

This is the real cause of the 64- and 128-turn failures and of the timeout.

- **Symptom:**
  - The main ended in the same step as the exhausting child.
  - Partial source passed 14/14 feature checks (64) and 1,488 focused tests (128), yet both are scored failures.
  - In iteration7-3, run time expired during the sixth child and the run answered `Failed(Budget(Time))` without consulting the main.
- **Root cause:**
  - `agent::plan` with `share: None` gives the child all turns, spend and time left (`agent.rs:102-106`).
  - The child's time is `run.deadline.min(call deadline)`, and the call deadline is the asker's session expiry (`session.rs:1266`, `run.rs:1516`).
  - `State::Over` denies everyone (`run.rs:785`). Time expiry closes sessions at once (session.md §6).
  - The spike's own `protocol/llm.md` text admits that a smaller `max_turns` "does not reserve that room".
- **Assessment: redo properly.** Safety should not be opt-in through a number the model cannot ground.
- **Clean fix: a finishing reserve.**
  - The charter names `reserve {turns, time}`; spend is derived at the main's max completion cost. Local default: about 2 turns and max(10% of time, one completion window).
  - Children and the main's ordinary work draw from `pool − reserve`, where the pool is what is left of the budget.
  - When the ordinary pool is exhausted, the run enters wind-down:
    - live children are closed and settled, each with a wrap-up result if possible (D6);
    - the main gets ≤ `reserve.turns` completions carrying a note ("budget spent; finish now with what you have");
    - those completions offer only `finish` (and `deliver` if granted), using the final-turn primitive from D6.
  - A change contract's time reserve must be at least `check_timeout`.
  - The hard ceiling stays intact.
  - Alternatives:
    - A fractional default share: arbitrary, and serial children still starve the main.
    - Reservation shares: needed for asynchronous children, but they do not give the main a last turn.
- **Contracts:**
  - run.md §3.1, §4.8, §5.3, §9 (reserve, wind-down), §10 (a budget-forced accept).
  - `protocol/charter.md` §2.
  - session.md §6.
- **Layer:**
  - `smith-domain-run`, `smith-domain-session`, the charter wire, `smith-local-domain`.
  - skein: `tool_choice: none`, because `skein-llm/src/openai/request.rs:115` hard-codes `"auto"`. Per AGENTS.md this shared mechanism goes into skein first.
- **Tests:**
  - Step tests:
    - share clamped to `pool − reserve`;
    - child exhaustion → wind-down → `finish`-only final turn → `Accepted`;
    - time-pool expiry closes children, then the main finishes;
    - an ignored final turn gives one `Failed(Budget)`.
  - Run-world fuzzy property: with `reserve.turns` ≥ 1, no `Failed(Budget)` before the main's final completion.
  - Agent-world: the fake provider sees the note and `tool_choice: none`.
  - All of these run at domain speed.
- **Benchmark probe ("starvation"):** 24 turns, 300 s, "delegate a full test survey, then report". Assert `Run Answered Accepted`, the main's last completion after the child's `Returned`, and total usage ≤ 24.
- **Questions:**
  - Should the reserve be a charter field or a fixed mechanism? Recommendation: a charter field with a documented default.
  - Should the final turn be `finish`-only? Recommendation: yes.

### D4. The model never learns its budget, time, or a child's cost (not in spike)

- **Origin:** the report recommends "clearer remaining-time guidance" (line 109).
- **Symptom:** in iteration7-3 the main opened a 25-turn write child at 1,109.4 s of 1,200 s, with 159 of 256 turns used. Earlier children had taken 58–276 s. Caps of 20–45 were chosen blind.
- **Root cause:**
  - The main system text (`prompt.rs` `render_system`) holds no budget numbers. `render_agents` (`prompt.rs:143`) says only "a share of the budget".
  - The child text (`prompt.rs:138`) omits its share.
  - Results (`feedback.rs:218`) carry no usage.
  - The `max_turns` description cites "the current run remainder", which the model cannot see.
- **Assessment: redo properly, and design it now.** This is the cheapest lever, and D5 is guesswork without it.
- **Clean fix:** three tiers, all run-written text kept in the transcript (run.md §3.3).
  1. Static and cache-stable:
     - The main gets `## Budget`: time, turns, spend if priced, the reserve, and how sub-agents draw from it. It also says that writer children serialise and inspect-only children run side by side.
     - Each child gets `## Your share`: turns, seconds, answer cap, and "your last turn has no tools".
  2. A usage line on every `sub_agent` result (at the tail, so cache-safe), for example `[child used 23 turns, 144 s; run: 41/256 turns, 1,019 s left]`.
  3. Threshold notes at 50/75/90% of time and turns, appended to tool results. These need the mid-work message mechanism (run.md §15).

  The clock is the domain's monotonic one, so the text is deterministic in worlds. Alternatives:
  - A `status` tool: costs a turn and relies on the model calling it.
  - Re-rendering the system text each turn: breaks caching.

  Do tiers 1 and 2 now.
- **Contracts:** run.md §3.3, §5.3, §9, §15; `protocol/llm.md` §5.
- **Layer:** `smith-domain-run`, `smith-domain` (feedback), `smith-domain-session` (tier 3).
- **Tests:**
  - Golden prompt text.
  - Exact usage line under a deterministic clock.
  - Agent-world: requests contain the budget section and the usage line.
  - Rerun the fuzzy-memory worst case.
- **Benchmark probe:** 240 s and 40 turns. The `finish` report must state the remaining time and turns; assert they match the trace within tolerance, and that no child opens in the last 15% of time unless its `max_seconds` fits.

### D5. `sub_agent.max_turns`, the optional child own-turn quota

- **Origin:**
  - `44e5120`: `smith-protocol-llm/src/decode.rs:137`, `tools.rs:77-78`, `protocol/llm.md` §3, `local-host.md`.
  - Report row 85 and line 99.
- **Root cause of the gap:** the domain `Ask::SubAgent.share` existed (`boundary.rs:399-410`; clamped at `agent.rs:104-107`), but the decoder hard-coded `None` and the schema omitted it.
- **Spike change:**
  - An optional integer from 1 to 4294967295, decoded to `Share { turns, spend: u64::MAX }`; that `u64::MAX` is a sentinel for "clamp to the remainder".
  - Time is inherited.
  - Tests:
    - `production_tools.rs:140`;
    - `adapter_boundary.rs:303`, through both providers' fake peers;
    - `smith-domain-run/src/tests.rs:1507`, which feeds the terminal by hand.
- **Assessment: keep with rework.** This change enabled both iteration7 successes: the parents continued after their child ran out. It is checked and clean under the strict subset. The shape is wrong:
  - **Turns are the hardest unit to estimate:** children took 9–23 turns before their first write (iteration7: 14, 19, 19, 13, 15, 19, 9, 15). iteration7-2's 20-turn child wrote first at turn 19 and was then cut.
  - **The child is not told its quota.**
  - **There is no time share,** and time is what bound in iteration7-3.
  - **The default is still "everything".**
  - **The `spend: u64::MAX` sentinel** is policy hidden in the protocol layer.
  - **The schema `maximum`** is noise.
  - **Own-turn semantics** would surprise if nesting were ever allowed.
  - **Mostly redundant domain test:** the behaviour was already covered by `tests/run/tests/simulation.rs:857`.
- **Clean fix:**
  - `sub_agent {brief, tools, llm?, max_turns?, max_seconds?}`, each clamped to `pool − reserve`.
  - Domain type `Share { turns: Option<u32>, time: Option<Duration> }`, with spend derived and never shown to the model, since host units mean nothing to it. It becomes a reservation once children are asynchronous.
  - Effective values go into the child's prompt and the usage line.
  - No schema `maximum`.
  - Alternatives:
    - A fraction: still blind.
    - Effort classes: hidden policy.
    - No model control: removes a lever the model used well.
- **Contracts:** `protocol/llm.md` §3 (the spike paragraph holds in substance; "does not reserve" becomes "the reserve keeps"); run.md §5.3; `boundary.rs` docs. The `local-host.md` paragraph should move into the contract.
- **Layer:** `smith-protocol-llm`, `smith-domain-run`.
- **Tests:**
  - Update the decode tests and add `max_seconds`.
  - A step test: effective share = `min(asked, pool − reserve)`, rendered in the child's prompt.
  - A session or agent-world test where a real session reaches its quota and takes the final turn, replacing the hand-fed terminal.
- **Benchmark:** measured (all iteration7 repetitions used caps), but there was no uncapped control.
- **Question:** add `max_seconds`? Recommendation: yes.

### D6. A capped child loses its work summary, and is never told its quota or answer cap (not in spike)

- **Symptom:**
  - In iteration7-2 the parent saw only `unanswered end=budget:turns` after the child's 20 turns and 116 s. The parent's own first write then came at turn 43 (409 s).
  - iteration7-1's child 2 ended the same way after 32 turns and 233 s.
- **Root cause:**
  - A turn-budget crossing stops the next completion (session.md §6). The child's last tool results are never read.
  - `agent.rs:162` returns `Unanswered{end}`, rendered at `feedback.rs:165`.
  - The child prompt says its last message goes "as it is" (`prompt.rs:138`), but it is cut at `answer_bytes = 1024` (`limits.rs:48`).
  - Observed answers were 267–656 bytes, so the cap did not bite in these runs.
- **Assessment: redo properly.** programming-model.md §7 says "refuse at the entrance, never in the middle".
- **Clean fix:**
  - A session final-turn primitive, shared with D3. When own turns remaining = 1, or time drops below the wrap-up allowance, the session appends the opener's note, withholds tools via `tool_choice: none`, and the child returns `Answered{stop: budget}`.
  - `Unanswered` is kept for real failures.
  - Tell the child its answer cap, and consider about 4 KiB, bounded in `peer.rs` `payload()`.
  - Complement: attach the written-paths list to `Unanswered`.
- **Contracts:** session.md §6, §7; run.md §5.3, §15; `protocol/llm.md` §5.
- **Layer:** session, run, `smith-domain`, skein `skein-llm`.
- **Tests:**
  - Session: the final turn withholds tools; calls made anyway are answered "not run", then the session yields.
  - Run: a capped child returns `Answered{stop: budget}`.
  - Agent-world: `tool_choice: none` on the final turn.
  - Fuzzy: a share of ≥ 2 turns never ends `Unanswered{budget}`.
- **Benchmark probe ("wrap-up"):** delegate "list and summarise tests/" with `max_turns: 3`. Assert `Answered`, non-empty text, `stop=budget`.

### D7. Scheduling: write-granted children run alone and the parent blocks

- **Origin:** report lines 103 and 109; `subagent-capability-analysis.json`.
- **Symptom:**
  - All 15 child calls requested inspect+modify+shell, so all ran serially with the parent blocked: 905 of 1,200 s in iteration7-3, and about 845 of 967 s in iteration6.
  - Codex (iteration7-codex-1) ran two children (about 195 s and 172 s) overlapping each other and the root. The root wrote files while both were active (root `fileChange` at ms 1791516045297, inside the children's span ms 1791516004709–1791516199194).
- **Root cause (intentional):**
  - Write effect when a child has modify or shell (`translate.rs:142`).
  - A write runs alone, and all results are assembled before the next model call (`session.rs:1325`, `finish_tools`).
  - A child lives only as long as its call and cannot nest (`agent.rs:112`).

  Together these give *at most one active writer per run*, which run.md §8.1 relies on for checks. Contributing factors:
  - The prompt never says that inspect-only children run in parallel (`render_agents`).
  - The local host always grants shell when there is a workspace, so "all my tools" means a writer.
- **What must stay true:**
  1. Overlapping write authority has at most one live holder, and checks or delivery hold the whole workspace.
  2. Bounded tree ownership: a parent settles its children before it ends, and nothing outlives the run.
  3. Exactly one terminal per call and per child; spend counted once.
  4. The transcript holds what each model was told.
- **Options:**
  - **(a) Now, no contract change:** guidance plus parallel inspect-only children.
  - **(b) An explicit run-level write lease.** Today's synchronous child means "lease transferred for the call's life", and checks take the whole-workspace lease. Next step: the parent keeps reading under a handle while a writer child holds the lease. Prerequisites:
    - mid-work messages;
    - a `join` or results-at-yield rule;
    - finish and deliver wait for or cancel live children;
    - no parking with live children.
  - **(c) Scoped write sets** over path prefixes:
    - file tools can enforce them in `smith-domain-tools`;
    - shell cannot: `Op::Spawn` `roots` are ignored (`smith-protocol-machine/src/component.rs:125`), and run.md §8.1 says checks are "not a security boundary";
    - this needs skein process containment.
  - **(d) Fully asynchronous writer children with messaging** (Codex style). Needs (b), (c), reservation shares, bounded mailboxes, and a defined place in the transcript.
  - **(e) A worktree per child:** this is its own authority, so it is a host tool (temper's `delegate`, run.md §5.3).
  - Codex shows that (d) without exclusion works only by task split. Concurrent fmt and builds on a shared tree are exactly the races Smith's contracts exclude.
- **Recommendation:**
  - Design now: (a), the budget pieces (D3–D6), and (b) written into run.md as the authority vocabulary.
  - Later document: (b)'s parent read-only overlap, then (c) with skein containment, then (d).
  - Leave (e) to hosts.
  - This refines the report's "scopes before async" with "lease first".
- **Contracts:** run.md §5.3, §8.1, §15; session.md §5; tools.md §4; skein shell/io (later).
- **Tests:**
  - Now: golden guidance text. The existing `simulation.rs:843` and session batch-order tests cover the mechanics.
  - Later: a referee invariant "≤ 1 lease holder; checks only under the whole-workspace lease".
- **Benchmark probe ("parallel review"):** three independent inspect-only reviews. Assert at least two children overlap in time.
- **Questions:**
  - Adopt the lease vocabulary now? Recommendation: yes.
  - Parent read-only overlap as the first asynchronous step? Recommendation: yes, in the later document.

### D8. Children start cold and re-orient every time (not in spike)

- **Symptom:**
  - Turns before a child's first write: 14, 19 / 19 / 13, 15, none, 19, 9, 15 / 13, 11, 23.
  - Briefs keep repeating "Read AGENTS and foundation docs first".
  - Codex offers `fork_turns` (unused in the observed calls).
- **Root cause:** the child prompt is only the brief, guides cut at 1,024 bytes, tools and checkout (`prompt.rs` `child`). There is no inherited context and no shared cache prefix, which contributes to the 12.4× fresh-input gap.
- **Assessment:** defer the mechanism; design the guidance now.
- **Clean fix:**
  - Now: guidance for focused briefs (paths, decisions, scope); maybe a larger guide allowance for children.
  - Later: an optional `inherit` that opens the child from the parent's transcript prefix (session.md §3), bounded by the child's limits and counted in its share.
- **Contracts:** run.md §5.3, §15; session.md §3, §7.
- **Benchmark probe:** the same subtask with and without inheritance; compare turns to first write and fresh input.

### D9. Local chats could not grant sub-agent tools (`3fae871`)

- **Root cause:**
  - `charter()` hard-coded `agents: false` from the first local-host commit (`3efed66`), with no recorded rationale and no settings field.
  - The charter has always carried `families.agents` (`protocol/charter.md` §2), and temper sets it from policy (`temper-engine-smith/src/start.rs:119`). The local host was the only host without the knob.
- **Spike change:** `sub_agents: bool` (default `false`) → `Config` → `grants.agents`. Unit tests cover the default, wire decoding and child model choice; docs updated.
- **Assessment: keep.** It is principled, defaults off, and adds no policy.
  - The mechanism already generalises to other hosts. The risks it exposes (D3, D4, D7) apply to every host.
  - Polish:
    - `Config.sub_agents` needs a doc line ("whether main may open sub-agents; children never nest").
    - The "repeat the main model to name it" rule is awkward; the benchmark listed `gpt-6-astra` twice.
- **Later:** a charter-level child policy (families ceiling, child count, default share), if hosts need it.
- **Contracts:** the spike's host.md §8 sentence and the `local-host.md` row and paragraph hold.
- **Tests:** add one `tests/local` world story with `sub_agents: true`; every local world currently sets it to `false`.
- **Question:** default to `true` once the reserve exists? Recommendation: keep `false` until D3 lands.

### D10. The schema offers `agents`, which the run silently drops (not in spike)

- **Evidence:** `tools.rs:78` declares it, `decode.rs:316` decodes it, it passes `within`, then `agent.rs:112` forces `agents: false`. run.md §5.3 already forbids nesting.
- **Fix: drop.** Remove it from the schema and decoder, and refuse nesting with a typed problem until it is designed with subtree shares and the lease model.
- **Tests:** schema inventory and decode.

### D11. Facts do not show what a child was given (not in spike)

- **Evidence:** `Run Opened {.., child: true}` has no share. The analyses had to parse `sub_agent` inputs.
- **Fix:** add the effective share and clamp reasons to `Opened`, and child usage to `Returned`. Facts are bounded and drop-tolerant (run.md §11). This lets the D3–D7 probes assert from facts alone.

### D12. Benchmark report claims

- **Row 84:** misattributes the fix to the 256 ceiling. Line 101 concedes both successes were under 128, and the remaining failure was time. The row should name the starvation mechanism.
- **Row 85 and lines 97–99, 103:** accurate. Every delegation asked for write grants, so serialisation was policy, not a bug.
- **Line 101:** should note that two of the five capped children ended `unanswered` with their work lost.
- **Line 109:** agree, refined to "visibility plus reserve; then lease; then scopes; then async".
- **Recommendation:** keep the report as a historical record with an erratum. Future benchmarks should record effective shares, child end kinds, and seconds the parent was blocked.

## Codex reference (what was verifiable)

- **Tools seen in native traces** (`runs/*codex*/agent/result.json`): `spawn_agent{task_name,message}` ×15, `followup_task{target,message}` ×7, `send_message{target,message}` ×99, `wait_agent{timeout_ms}` ×1. No budget, turn or time argument appeared.
- **Role prompts** (`smoke/codex/codex-home/catalog.json`):
  - children can spawn sub-agents;
  - `fork_turns` sets how much parent context is inherited;
  - messages arrive as `MESSAGE` or `FINAL_ANSWER`;
  - `send_message` does not trigger a turn, `followup_task` does.
- **Budgets:** `token_budget` is a context-window reminder feature (6,144-token threshold, `get_context_remaining`), disabled in the benchmark config. `thread_manager.rs:245` says the "shared agent budget remain[s] bound to the parent", but the `AgentControl` module is not in the research snapshot, so thread, depth and per-tree limits are unverified.
- **Write exclusion:** none observed. The root wrote files while both children were active.

## Cross-theme dependencies

- **Context and request size limits:** these (256 messages, 1 MiB, context management) cap a single session, not the turn budget. They affect D8's inheritance.
- **Cache affinity:** cache-stable placement of budget text (D4); keeping tools in the final-turn request (D3, D6); child prefix sharing (D8).
- **The 300 s completion window and `check_timeout`:** they set the minimum size of the time reserve and the wrap-up allowance.
- **Mid-work messages, waiting and startup:** threshold notes (D4) and asynchronous child results (D7) need the same mechanism.
- **Shutdown and settlement:** wind-down must keep exactly one terminal per in-flight child (iteration7-3's last child ended `Cancelled`).
- **Budget accounting:** `protocol_allowance: 0` makes spend reservations tight once spend becomes a real bound (D2).
- **Trace and diagnostics:** D11's fact fields.
- **Workflow and process:** `5848e59` and `44e5120` appended dated validation logs to `docs/development/workflow.md`; they don't belong in workflow rules.
- **skein:**
  - `tool_choice` support in `skein-llm`, for D3 and D6;
  - process write containment, for D7(c).