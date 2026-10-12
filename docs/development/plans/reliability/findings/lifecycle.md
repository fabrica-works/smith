# Findings: message lifecycle, startup and activations (theme L)

Refs: **main** = `30259d8`; **tip** = `bench/smith-codex-2026-10-08` (`bf96e94`). Paths are relative to `/home/free/src/rust/smith` unless stated. "Reasoned" marks conclusions drawn from reading code that I did not reproduce, since the brief forbids running anything.

## Summary

- **What went wrong.** Two separate defects.
  - *Startup.* The run opened its main session as soon as preparation ended, with the generic "Begin the work your brief describes." Interactive chats have an empty brief, and the person's line reached the run only after a cross-process hop. So the first provider request often carried no task. What the model saw first depended on placement, configuration and IO timing.
  - *Messages.* The local host treated "sent" as "delivered":
    - a follow-up sent to a run that was finishing was lost;
    - a follow-up named during shutdown was dropped, or crashed the host on a late `StateSaved`;
    - an `Ending` bounce counted as an agent failure;
    - EOF threw away lines already typed.
- **What spike 1 did** (`8e83931`, `6136dc4`):
  - a message queued at open becomes the opening prompt;
  - an empty brief that may wait opens with "call wait" (`AWAIT`);
  - added a Waiting section to the system prompt;
  - added a four-state `LineState` that keeps a line until a turn's read fence names it, and carries unread follow-ups into the next activation;
  - an `Ending` bounce becomes `MessageBounced`;
  - the old answer waits for a pending save;
  - EOF drains typed lines;
  - final notices end with a newline.
- **What spike 2 did** (`140c351`):
  - added an `Awaiting` state, so an empty brief makes no provider request until the first message;
  - removed `AWAIT`;
  - made the `person: ` label the same in both placements.
- **Shape of the clean fix:**
  - one opening contract: a run opens when it has work, otherwise waits, and a start may carry the triggering messages;
  - one message contract: exactly one terminal per message (read, bounced or unread), with the answer naming the final fence;
  - explicit host policy for unread lines;
  - a bounded unread queue in the local host;
  - typed notices rendered by the terminal layer;
  - worlds with multi-line person schedules, and referees that check no line is lost.

---

### L1. Startup opened main before the first task: two fixes, one contract

- **Origin:**
  - reliability report item 3;
  - `8e83931` `crates/smith-domain-run/src/run.rs` (`open`, `opening`) and `prompt.rs` (`AWAIT`);
  - `140c351`, `ad8a793`;
  - the benchmark row "Startup requested a completion before the user task";
  - `startup-wait-plan.txt`.
- **Symptom:**
  - **`smoke2` research run.** The spike-1 binary in external placement, trace at `/tmp/smith-benchmark-smith-research/smoke2/trace.jsonl`.
    - Prompt 1's only user text is `AWAIT`, and the model calls `wait`.
    - Prompt 2 ends the turn with empty text.
    - The task first appears in prompt 3.
    - That is 2 wasted completions out of 4, about 2.03 s and 1,732 input tokens.
  - **Main.** It always opened with BEGIN on an empty brief.
  - **Scored benchmark.** 0 of 41 recorded Smith traces contain `AWAIT`. The baseline runs at `8e83931` opened with the message, because their workspace reads delayed the open. The defect is timing-dependent, so the scored matrix never showed it.
- **Root cause:**
  - The run opens main the moment preparation ends (tip `run.rs:316-318`, `prepared` at `run.rs:1279`).
  - Messages pass only at yields (`run.md` §6).
  - The local host sends the line only after `Admitted` (tip `domain.rs:777-780` in process, `863-866` external). The outcome depends on placement:
    - in process with no workspace, `open` runs inside `Start`, before any message exists;
    - in process with a workspace, the message always wins;
    - spawned, it is a race.
- **Spike change:**
  - **(a)** Spike 1: `open` pops a queued message into `opening.prompt` and sets `offered` (`run.rs:1325-1329`).
  - **(b)** Spike 1: an empty brief with wait authority opens with `AWAIT`.
  - **(c)** Spike 2: if `wait && brief empty && inbox empty`, the run emits `Waiting` and enters `Awaiting` (`run.rs:1304-1307`). The first `message` calls `open` (`run.rs:359`). Cancel or deadline answers through `stop` with zero spend. `follow` keeps the wall deadline and arms no park alarm. `AWAIT` is removed, and `run.md` §3.3/§6 and `local-host.md` are rewritten.
- **How the spikes relate:**
  - (c) supersedes (b).
  - (c) relies on (a), so they compose.
  - (a) still governs non-empty briefs, where the race remains (L4).
- **Assessment: keep with rework.** The direction is right (never send a provider request without a task), and the state is exhaustive and bounded. Four problems:
  1. The trigger is inferred from two charter fields. A role-only run (task in `instructions`, empty brief, wait authority) now idles until wall time ends it as `Failed(Budget(Time))`.
  2. `open` now serves two cells, against `programming-model.md` §5.4.
  3. Awaiting arms no idle parking, which contradicts `run.md` §3.1. A run that hears nothing ends as a failure, not parked.
  4. The startup `Waiting` leaks to the person (L3).
- **Clean fix:**
  1. Open main when prepared and there is work: a non-empty brief or at least one message.
  2. The opening is BEGIN (if there is a brief) followed by every queued message, FIFO. The read fence is the last message included.
  3. No work and wait authority: `Awaiting` (no session, no request, wall time runs). Parking is Q3.
  4. No work and no wait authority: BEGIN on instructions (Q2).
  5. Start-carried messages: add `messages` to the channel start record and to `smith_domain::Event::Start`, entering the inbox at admission. The local host and any interactive host use them. `Awaiting` stays for hosts that start before they have a message.

  Alternatives:
  - (i) spike 2 as is;
  - (ii) an explicit charter field `opens_on`;
  - (iii) start-carried messages plus `Awaiting` (recommended).
- **Contracts:**
  - `domain/run.md` §3.2 (start carries messages);
  - §3.3 (change "instead of the generic instruction" to the composition above);
  - §3.1 (meaning of waiting zero);
  - §6 "Before the first message" (parking clause per Q3);
  - `protocol/channel.md` §3 and §5;
  - `domain/host.md` §2;
  - `local-host.md` (`ad8a793` text holds for empty briefs).
- **Layer:** `smith-domain-run`, `smith-domain`, the channel crates, `smith-host-domain`, `smith-local-domain`. Nothing for skein.
- **Tests:**
  - **Spike additions:**
    - 7 run step tests and 3 root step tests;
    - the local-process first-query assertion;
    - `9340755` (memory fixture driven past `Awaiting`);
    - spike 1's run step test.

    These are the right tier.
  - **Missing:**
    - No world exercises `Awaiting`. Agent and run world briefs are always non-empty (`tests/agent/src/world.rs:1919`, `tests/run/src/world.rs:123`).
    - The agent messages referee forbids a pre-turn `Waiting` (`messages_referee.rs:229-237`).
    - Plan items 11–12 were never implemented.
  - **Clean version needs:**
    - an empty-brief job in `fuzzy_messages`, with the referee taught about `Awaiting`;
    - local scenarios with and without a workspace asserting the first prompt is the person's line;
    - a local-process scenario delaying host→agent delivery.
- **Benchmark:**
  - The scored runs never measured this; only `smoke2` did.
  - The benchmark row's "parks" is wrong: the run awaits, it does not park.
  - Probe: a one-line task with an empty brief, external placement, with and without a workspace, 5 repetitions. Assert from the trace:
    - the first prompt's last user text is `person: <task>`;
    - no `wait` comes before it;
    - completions ≤ 2;
    - record time to first request.
- **Questions:**
  - Q1: start-carried messages? Recommend yes.
  - Q2: an empty brief without wait authority? Recommend BEGIN.
  - Q3: parking while `Awaiting`? Recommend parking at the charter's waiting time after Q1 lands. Until then keep the spike's rule and fix the §3.1 wording.

### L2. `Awaiting` mechanics: wall budget and resume validation

- **Origin:** `140c351`; the plan's "Risks"; `9340755`.
- **Details:**
  - **Wall budget.** An awaiting run burns wall time, and main's remaining budget is computed at open (`opening`, tip `run.rs:1674+`). A host that starts a run early starves the session.
  - **Resume validation.** A malformed transcript is found only when the first message arrives. A cancel while awaiting discards the history without reporting it.
  - **Bounds.** Start emits `[Admitted, Waiting]` within `MAX_OUT`. The root `StartContext` stays held, which is bounded, and `9340755` correctly covers it.
- **Assessment: keep the state, and document the wall-time interaction in `run.md` §9.**
- **Clean fix:**
  - Split `open` into `prepared → await_task | open_main`.
  - Say in §9 that time spent awaiting counts against wall time (recommended), or start the clock at open.
- **Contracts:** `domain/run.md` §6, §9; the `run.rs` module table.
- **Tests:** a late first message; assert the opening's time remainder and exactly one transcript validation.
- **Benchmark:** none needed.
- **Question:** count awaiting time against wall time? Recommend yes, documented.

### L3. A "Waiting for a message" notice appears right after the person types; the world was relaxed to hide it

- **Origin:** `140c351` (`run.rs:1304-1307`; `tests/local/src/world.rs:790, 941, 1323`).
- **Symptom (reasoned, and confirmed by the world edit):**
  - In process, `Start` emits `[Admitted, Waiting]`. `route_agent` handles `Admitted`, and `send_line` nests a step that queues `Complete` behind `Waiting`. The loop then shows "Waiting for a message" (`domain.rs:787-789`) before the first completion.
  - External: the kit forwards `Up::Waiting` (`host-domain/domain.rs:900-904`) and the host shows the same text (`domain.rs:883`).
  - The world now clears the waiting goal on each line and each `Complete`, and requires `completions > 0`. It tolerates the notice instead of asserting it is absent.
- **Root cause:**
  - The local host ignores `Waiting.read`. A `Waiting` whose fence is behind the last message sent is a crossing, not idleness. The kit already knows this for its watchdog (`domain.rs:902`).
  - The nested `agent::step` inside the drain loop (`domain.rs:774-820`, pre-existing) is fragile.
- **Assessment: redo.**
- **Clean fix:**
  - Show "waiting" only when the fence covers every message sent; this is the same rule `host.md` §4 sets for pausing the watchdog.
  - Start-carried messages remove the startup case entirely.
  - Replace the nested step with a deferred event.
- **Contracts:** `domain/host.md` §4 and §8; `protocol/hosts.md` §5.1.
- **Layer:** `smith-local-domain`, optionally `smith-host-domain`.
- **Tests:**
  - Revert the three `world.rs` edits.
  - Add a referee safety rule: a waiting notice only when every relayed line has been read.
  - Add a no-early-notice scenario in both placements.
- **Benchmark:** none needed.

### L4. A non-empty brief makes the opening timing-dependent and can silently drop the first line

- **Origin:** not in the spike (latent). Reachable through the opening replacement (`run.rs:1325-1329`) and the local host dropping an unread opening line (`domain.rs:960-962`).
- **Symptom (reasoned):**
  - Local settings may carry a brief (`local_settings.rs:96, 207-231`).
  - If the person's line arrives early, it replaces BEGIN.
  - If it arrives late, BEGIN opens. If the model then finishes without yielding, the line is never read, and `finish_answer` drops it silently.
  - Temper-like hosts have the same timing dependence.
- **Root cause:** messages pass only at yields, and the drop rule is undocumented. The spike's text says lines are retained until read, which is not true here.
- **Assessment: latent, low frequency, real.**
- **Clean fix:**
  - compose the opening deterministically (BEGIN, then messages);
  - use start-carried messages;
  - never drop a line silently (L6);
  - optionally, refuse a `finish` that crosses a queued message (L5).
- **Contracts:** `domain/run.md` §3.3; `domain/host.md` §8.
- **Tests:** a non-empty brief with a scripted finish in the first completion; assert the line is read or reported.
- **Benchmark:** none needed.
- **Questions:**
  - Q4: replace BEGIN or append after it? Recommend append.
  - Q5: pass all queued messages per yield or at opening? Recommend all, FIFO, bounded by `run.messages`, instead of one per yield (`run.rs:377-389`).

### L5. Messages have no terminal: "offered is not proof of read" as the general contract

- **Origin:** report item 4; `8e83931` text in `host.md` §8 and `hosts.md` §5.1; the silent drop in `run.rs` (present on main too).
- **Symptom:**
  - Lost follow-ups (report item 4, lines 113-117).
  - The trial logs cited at report lines 104-111 no longer exist, so I could not check them.
- **Root cause:**
  - `run.md` §6 says each turn names the last message read "so the host knows what was taken", but nothing covers messages that were not taken.
  - `run.rs:337-338` drops messages silently in `Stopping`, `Winding` and `Closed`.
  - An accepted `finish` winds down with a non-empty inbox (`run.rs:1391-1395`).
  - The kit bounces `Ending` after the answer (`accepts` at `domain.rs:641-651`; `heard_answer` at `967-968`) and tracks `unread`/`read`. But `Request::Answered` carries no fence (`boundary.rs:151-155`).
  - Bounces and "unread" are undocumented in `docs/design` (no grep hits).
  - `programming-model.md` §5.2 requires exactly one terminal per request, and an accepted-but-unread message has none.
- **Assessment: the principle is sound (keep it); the implementation is local-only (redo as a contract).**
  - Messages reach the model FIFO, only at yields or at opening.
  - `read` advances only on a told turn after the offer (`run.rs:403-405`), and that turn's body contains the message.
  - So "read" means part of the transcript the host keeps.
  - Single-name fences require FIFO offering; keep that as an invariant.
- **Clean fix:**
  - Each message gets exactly one terminal:
    - **read:** the fence names it or a later message;
    - **refused:** too large, full, reused name or ending;
    - **unread:** the run answered or the agent went without reading it.
  - Add `read` to `run::Answer` and the channel answer record, so every host (in-process too) knows which messages went unread.
  - The kit returns unread names exactly once at `Answered`/`Gone`.
  - What happens to unread messages is host policy (`host.md` §5).
  - **Sub-agents.** No host messages reach children today. If async parent→child follow-ups arrive (benchmark report line 51), the same contract applies at the `sub_agent` call.
  - **Extension.** A `finish` that crosses a queued message is not accepted and returns the messages (Q6).
- **Contracts:**
  - `domain/host.md` §2, §4, §5;
  - `domain/run.md` §6 ("a message not read by the answer is never read"), §10;
  - `protocol/channel.md` §3–§5;
  - restate the spike's sentences in `host.md` §8 and `hosts.md` §5.1 as local policy. The `hosts.md` copy belongs in the domain doc.
- **Layer:** run, root, channel crates, host kit, local domain.
- **Tests:**
  - host world: messages around the answer each get exactly one terminal (safety and liveness);
  - agent world: the answer's fence equals the last turn's;
  - run step tests: a message in `Winding` is never read; `finish` with a queued inbox.
- **Questions:**
  - Q6: the `finish`-crossing extension? Recommend yes for wait-granted charters.
  - Q7: the kit reports unread names? Recommend yes.

### L6. Local host line lifecycle (`LineState`): right behaviour, wrong shape

- **Origin:** `8e83931` `smith-local-domain/src/{domain.rs,boundary.rs,tests.rs}`.
- **Symptom:** on main, `send_line` took the line (main `domain.rs:590`) and `finish_answer` cleared any named line (main `:905`).
- **Spike change:**
  - one `line` slot plus `line_state` (Ready/Followup/Offered/Retry), with transitions at `domain.rs:345, 625-629`;
  - the fence clears the line (`838-843`);
  - the answer drops Ready/Offered and keeps Followup/Retry (`960-963`);
  - a kept line opens the next activation under its original name;
  - no second line is named while one is outstanding (`330`).
- **Assessment: keep the carry; redo the mechanism.**
  - `line_state` is meaningful only when `line` is `Some`, which `programming-model.md` §5.4 forbids.
  - The single slot serializes the person, a hidden throughput policy.
  - The carry policy is hidden in state names: the unread opener is dropped silently, and follow-ups auto-start a new activation even after Ctrl-C, failure or budget end.
  - The `MessageBounced` arm (`867-874`) is nearly dead, because follow-ups are already `Retry` when sent. It matters only for an `Offered` opener bounced while Running, and that can loop with no progress bound: for example, a non-empty brief with a refused resumed transcript whose `Admitted` and `Answer` arrive in one read (reasoned, narrow window).
  - It adds own-enum `==` checks.
  - Lines live in memory only.
- **Clean fix:**
  - `unread: Queue<Line>` bounded by `min(lines, run.messages)`.
  - Send each line on naming; the fence pops the prefix.
  - At the unread terminal, apply policy by outcome:
    - accepted or parked: carry and start the next activation;
    - cancelled, failed or refused: show "Not delivered: …" (Q8).
  - The `Ending` bounce takes the same path.
  - On load, report `read < next_message`.
  - Document the policy in `host.md` §8.
- **Tests:**
  - The spike's two external step tests are narrow.
  - Add cell step tests and the L11 scenarios.
- **Question:** Q8, unread lines after cancel or failure? Recommend showing them and dropping them; never auto-start after an interrupt.

### L7. A follow-up's state save racing the answer crashed or lost the line

- **Origin:** report item 4; `8e83931` guard at `domain.rs:919-921` and the Running branch at `431-438`.
- **Symptom (reasoned from main):**
  - In process: line named, save pending, answer arrives. `finish_answer` (main `:864`) clears the line and sets `Loading`. The late `StateSaved` then hits `unreachable!` (main `:416`): fail-stop crash.
  - External: `state_saved` sends the line to an answered child, which bounces it (`Ending`), which main reported as "Agent process failed".
- **Root cause:** an "impossible" cell reachable by store latency, against §5.4's "anything a peer can cause is handled".
- **Assessment: keep with rework.**
- **Clean fix:** an explicit `Phase::Answered { answer }` with defined cells for StateSaved, TurnSaved, Line and Gone, instead of guards in three handlers.
- **Contracts:** reword the spike's `host.md` §8 mechanism sentence as an invariant.
- **Tests:**
  - The spike's `followup_after_shutdown(true)` runs Answer → StateSaved → Gone in external placement, where the guard is never decisive.
  - The decisive interleavings are in-process Answer → StateSaved and external Answer → Gone → StateSaved.
  - Add a slow-store world scenario in both placements.

### L8. An `Ending` bounce counted as an agent failure

- **Origin:** `8e83931` `smith-local-service/src/service.rs:731-745` (main `:693`).
- **Assessment:**
  - Keep the translation, folded into L5 as the unread terminal.
  - `Full`, `TooLarge` and `ReusedName` stay failures; local configuration rules them out.
  - With the L6 queue, configuration must ensure `lines ≤ host messages`.
- **Tests:** a local-process follow-up while the child answers. That is the only placement with a bounce path.

### L9. EOF drains received lines (partial)

- **Origin:** `8e83931`: `dispatch_line` no longer checks `closed` (main `:315`), and new exit conditions at `414` and `967`.
- **Symptom:** on main, `printf 'a\nb\n' | smith local` lost `b` (reasoned).
- **Assessment: keep, with gaps.**
  - After a failed grant (`492-496`) or head survey (`597-600`), the host goes `Idle` without dispatching raw lines. EOF then exits and drops them (`713-719`).
  - No deterministic test: the worlds never send `Closed` before an answer.
- **Clean fix:**
  - one drain rule: dispatch at `Idle`;
  - exit only with no raw or unread lines;
  - apply L6's policy after a failed start.
- **Tests:** lines then EOF; EOF mid-run; EOF after a failed grant.
- **Benchmark probe:** pipe two lines then EOF; assert two finishes and exit 0.

### L10. Races in the same area the spike did not cover

| Message arrives | Tip | Under L5/L6 |
|---|---|---|
| during preparation | becomes the opening | composed (L1) |
| in `Awaiting` | opens main | same |
| during a completion | waits for the yield | same; mid-work injection open (§15) |
| while main waits on a synchronous child | waits minutes (a 273 s child in the benchmark) | correct but slow |
| during the `finish(report)` decision | unread; local carries it | unread terminal; Q6 |
| during `finish(change)` landing | read if landing fails, unread if it succeeds | same |
| at the instant of park | unread, carried (lost before the spike) | carried |
| after Ctrl-C | carried and auto-starts | Q8 |
| during budget `Over` | carried, fresh budget | Q8 |
| during a crash | text lost, name saved | reported on load |

- **Assessment:** the fence contract makes every row loss-free. What is left is policy (Q8) and latency (the child row).
- **Long term:** pass messages between completions (§15). The fence contract already supports it.
- **Question:** Q9, design mid-work injection now and implement later.

### L11. Why the worlds and the live suite missed it (report section)

- **The report's account is partial.**
  - The "forced startup-wait" was added by spike 1 itself (`6136dc4`) and removed in `8e83931`; main never had it.
  - Main's live story passed with the generic opening because its oracle was only "finish appears".
- **Why the deterministic tiers missed it:**
  1. One line per scenario: every local, local fuzzy (`fuzzy_simulation.rs:31`) and local-process (`world.rs:316-356`) scenario.
  2. In-process placement has no bounce path and delivers synchronously, so the opening was fixed by configuration.
  3. The referees are wrong in both directions.
     - There is no "no line lost" liveness rule.
     - "relayed only once", "in order" (`tests/local/src/referee.rs:146-148`) and `next_message == last + 1` (`:133`) all fail on the spike's own carry, unnoticed.
     - The agent referee forbids spike 2's startup `Waiting`.
  4. No world person reacts to output.
  5. No fuzzy schedule perturbs message timing.
- **Clean fix (tests):**
  - **Local world person script:** sends lines on observed events (Complete, Turn, Answered, Shown, Waiting, or after N steps) and places EOF anywhere.
  - **Local referee safety:**
    - a line is relayed at most once per activation, and again only if it was never read;
    - fences are monotone;
    - a waiting notice appears only when every line is read.
  - **Local referee liveness**, using skein's `Judge::rearm`: by exit, every name is read or shown as not delivered. Relax the `next_message` rule.
  - **Agent referee:** accept a pre-turn `Waiting` for message-driven charters.
  - **Scenarios:**
    - a follow-up at finish;
    - a slow store;
    - two lines back to back;
    - lines then EOF;
    - a line at park;
    - Ctrl-C with an unread line;
    - a non-empty brief with an early finish.

    Run them in both placements where the bounce path matters.
  - **Fuzzy:** k ∈ {1,2,3} lines at random triggers, store delays and an EOF point; plus an empty-brief job in `fuzzy_messages`. The fuzzy suite is about 7.5 s today, so this fits.
- **Contracts:** `docs/design/testing.md` §5; `domain/host.md` §10 stories.

### L12. The new live interactive story

- **Origin:** `8e83931` `crates/smith/tests/live.rs` and `support/live_run.rs`.
- **Assessment: keep with rework.**
  - It covers one interleaving per run.
  - It passes whether the line was carried or read in place.
  - It does not check for duplicates or "Agent process failed".
  - The terminal is smith's `tests/local-process/src/terminal.rs`, not skein's as the report says.
- **Clean fix:** add trace assertions:
  - the second activation's first prompt contains the task exactly once;
  - there are 2 activations;
  - no `wait` comes before the first task;
  - no failure or early waiting text appears.
- **Benchmark probe:** greeting, then on `Tool: finish` "create note.txt via shell". Assert two finishes, the file, exit 0, and the carried prompt.

### L13. Waiting guidance and the cost of the two-step `wait`

- **Origin:** `6136dc4` `prompt.rs:113-118`; the brief golden tests.
- **Symptom:**
  - `run.md` §3.3 promised waiting text that main never rendered.
  - In `smoke2`, each `wait` cost a full extra completion: about 1.05 s and 878 input tokens.
- **Root cause:** `wait` returns `waiting` immediately (`run.rs:913-919`), and the model must then end a turn (`577-585`).
- **Assessment: keep with rework.** The text fills a contract gap and the golden tests are the right tier, but the mechanism is costly.
- **Clean fix:**
  - **(a) Recommended:** defer the `wait` result and send `[result: waiting, user: message]` together when a message arrives. Resume supplies the result the way it supplies absent host answers. One request per exchange.
  - **(b) Alternative:** `EndTurn` counts as a wait for wait-granted runs. Simpler, but autonomous runs lose the nudge.
  - After (a), the guidance shrinks to one sentence.
- **Contracts:** `run.md` §6; `session.md`.
- **Tests:** keep the golden tests; add a test counting completions per wait.
- **Benchmark probe:** a 3-exchange chat, counting completions and tokens per exchange.
- **Question:** Q10, recommend (a).

### L14. Final notice newline: wrong layer, partial

- **Origin:** `8e83931` `person.rs:104`; about 20 `trim_ascii_end` changes in `tests/local`.
- **Root cause:**
  - The domain emits raw bytes, and the terminal writes them verbatim (`smith-local-protocol/src/terminal.rs:97-101`).
  - Only answer notices got the newline. "Waiting for a message", "Running checks", "Agent process failed" and the rest (`domain.rs:788-905`) still have none.
- **Assessment: redo.**
- **Clean fix:** typed `Notice` variants, rendered as complete lines by the terminal protocol (`hosts.md` §5.1); worlds observe notices by kind.

### L15. Sender label: two renderings, an off-by-two bound, and asserts a host can trigger

- **Origin:** `140c351` `person.rs:15-31`, `config.rs:99`, `local_settings.rs:276`; the rest is latent.
- **Symptom:** placements diverged before `140c351` (`/tmp/smith-startup-local-failure.txt`).
- **Root cause:**
  - The label is rendered by the channel's `labelled_message` (`component.rs:877-884`) and by a hard-coded `person: ` in the local domain.
  - The kit's `TooLarge` check ignores the 2-byte `": "` separator (`host-domain/domain.rs:690-697`).
  - The agent channel uses `CEILINGS` (1 KiB label, 1 MiB text).
  - Nothing checks the rendered size or inbox count before `run::message`'s asserts (`run.rs:345-353`), via `component.rs:607-616` → `service.rs:1203-1208` → `route.rs:63`.
  - So a host can panic the agent (reasoned).
- **Assessment:** keep the parity; redo the mechanism and fix the crash.
- **Clean fix:**
  - one shared `render_message`, or `Event::Message { label, text }`;
  - one rendered-size bound, checked at agent ingress, refusing with rules;
  - the kit uses the same arithmetic;
  - the local reservation derives from the label.
- **Contracts:** `run.md` §6; `channel.md` §3, §5; `host.md` §9.
- **Tests:** a channel case at limit+1 after rendering must refuse with rules, not panic.

## Decisions for the user

| # | Decision | Recommendation |
|---|---|---|
| Q1 | Start carries its triggering messages | Yes |
| Q2 | Empty brief, no wait authority | BEGIN on instructions |
| Q3 | Parking while `Awaiting` | Charter waiting time after Q1; spike rule until then |
| Q4 | Brief plus queued messages at opening | Append after BEGIN |
| Q5 | Messages per yield and at opening | All queued, FIFO, bounded |
| Q6 | `finish` crossing a queued message | Refuse for wait-granted charters |
| Q7 | Kit reports unread names | Yes |
| Q8 | Unread lines after cancel or failure | Show and drop; never auto-start after an interrupt |
| Q9 | Messages mid-work (§15) | Design now, implement later |
| Q10 | `wait` mechanism | Deferred result |

## Cross-theme dependencies

- **Limits/config:** `message_bytes`, `messages` and `lines` bound L6 and L15.
- **Shutdown/settlement** (`a2c2e53`, `a54ba26`, skein `416cffd`/`f5fca44`): shifts `Gone` versus `StateSaved` timing (L7) and bounce windows (L6, L8).
- **Diagnostics** (`e69bfc3`, `ecb7562`, `8466afb`): shares the `service.rs` bounce/fault mapping.
- **Terminal facts and trace** (`136b9c4`, `6a1dfb2`): when the local host learns the answer; the probes rely on trace records.
- **Channel owner:** L1, L5 and L15 all change `smith-channel` records; coordinate one version bump.
- **Budget and sub-agents:** longer children widen the message-wait window (L10, Q9).
- **Tools:** `8e83931`'s path guidance (item 6) in `prompt.rs` and the shared brief golden files.
- **Test infrastructure:** typed notices and the person script and referee in `tests/local` and `tests/local-process`.