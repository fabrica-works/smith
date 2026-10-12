# Session 08: context and caching

Provisional, 2026-10-12. What is left keeps each conversation within its
model's window. Today a session counts no tokens against its window: a
conversation grows until its history's byte or message bound ends it as
transcript full, or the provider answers *context too long* and the
session fails. This session adds the window check and compaction into a
new window (domain/session.md, section 8), cuts the results owed to a
compaction to their room, removes the transcript-full end, and lets the
local host resume from the last window, which compaction needs first.
smith 00's `history` probe waits for it.

## Read first

- domain/session.md, sections 3, 7, 8, 10 and 11;
- domain/run.md, sections 3.3, 9, 11 and 13;
- protocol/transcript.md, sections 2, 3, 4 and 9;
- protocol/hosts.md, section 5.3; protocol/channel.md, section 4;
- protocol/limits.md, sections 3.2, 3.9 and 4 (`fresh-window`,
  `compaction-headroom`, `window-first`);
- protocol/events.md, sections 3.4 and 3.5;
- benchmarks.md, section 11.1 (`history`) and 11.3 (`compaction`);
- decisions.md, 1.1 and 4.7; findings C7 and C8 (`findings/context.md`).

## What the design asks

- **Hosts resume from the last window** (transcript.md, section 3;
  hosts.md, 5.3): "A chat's transcript is its turns' files, in order,
  from the last turn whose record had `opens_window`. Earlier windows'
  files are kept for the person to read, and never sent back."
- **The check** (session.md, 8.1), before each completion: "the last
  prompt, plus the last output, plus the appended bound, plus the output
  reservation, is at most the window times its threshold." A byte counts
  as at most one token; with no earlier prompt in the window, or no input
  count reported, the bound is the whole prompt's bytes plus the
  allowance. "A completion that fails the check is not made: the session
  compacts instead."
- **Compaction** (session.md, 8.2): triggered by a failed check or a
  provider's *context too long*; the same model, affinity, system text,
  tools and tool choice, the window's history with the results owed to
  it, then the summary instruction as one user message. Messages the
  opener passed since the last completion are held back; calls in its
  answer are not run. "The new window opens with one user message: the
  conversation's task, … the summary, … then the held-back messages."
  Told as a window turn; counted as a turn and as spend. "A window is
  compacted once": *context too long* on the compaction, or a failed
  check or *context too long* before a window's first ordinary
  completion, "fails the session with a limit naming the window".
- **No summary, no window** (session.md, 8.2, settled 2026-10-12): "A
  summary cut at the output limit opens the window as written, with no
  marker". "An answer that is a refusal (its completion stopped for
  refusal, or it holds a refusal block), or whose text blocks hold
  nothing but ASCII whitespace, opens no window. The session fails at
  once, with the fault of the model's answer": *refused*, "whatever text
  came with it"; *truncated*, "for no text and a stop at the output
  limit"; *malformed*, "for no text otherwise". It is "neither retried
  nor nudged", charged like any completion, and "told as no turn"; "The
  messages held back for the new window are not read." Main's fails the
  run with that fault (run.md, section 10).
- **The results owed to a compaction** (limits.md, 3.9) "are rendered
  within what the window leaves after the history before them, the
  summary instruction and one `output` …; each is cut with its marker".
- **History bounds** (session.md, 8.1): "A history that would cross one
  of them anyway is compacted as a full window would be."
- **A compaction due near the end** (session.md, section 7) counts in a
  child's final-turn trigger: two turns, or twice the allowance.
- **The summary instruction** is the run's, given with each session it
  opens (run.md, 3.3), and part of smith's own part (limits.md, 3.9).
- **Facts and records** (run.md, section 11; events.md, 3.4 and 3.5): a
  compaction is a conversation of kind `compaction`, with the
  conversation it summarises as `parent`; `response.started`'s `window`
  counts from 1, one more after each compaction.

## Choices this plan makes

- **Hosts first.** 08.2 lands before any window turn exists, so that the
  first compaction never leaves a chat that cannot load.
- **The local store keeps the flag in its turn file's header,** beside
  the read fence, so it finds the current window without decoding
  earlier turns.
- **Compaction lands in three increments:** the check and compaction on
  its two triggers (08.6), the owed results cut to their room (08.7), and
  the history bounds as a third trigger with the transcript-full end
  removed (08.9).
- **New state goes in new modules:** `window.rs` and `compaction.rs` in
  `smith-domain-session`; `session.rs` (3,481 lines) gains only the call
  sites: aim for at most 100 added lines per increment, by `git diff
  --numstat origin/main` on it; past that, code moves to those modules.
- **The task** is the conversation's first user message, kept by the
  session, or the first text block of the window turn it resumed from.
- **A fresh window at its history bound** (08.9) fails with the existing
  `Limit::History` (`llm.rs:374`), as a fresh window fails its check with
  `Limit::Window`.

## Rules that bind it

- **Compaction into a new window, never elision** (decisions.md, 1.1):
  within a window, history only grows; a window boundary replaces it
  whole and keeps the system text, the tools and the affinity
  (session.md, section 3). The referees check it
  (`tests/session/src/world.rs:18`).
- **Removing a mechanism removes all of it** (development.md, 8.2): 08.9
  removes the transcript-full end, its mappings, tests and goldens.
- **Formats change under this plan's versions** (the README's version
  rule): transcript 3, channel 2 and events 1. Nothing here bumps one.

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 08.1 | `f8d47ec7` | merged, unproven: smith-13 |
| 08.3 | `21fada73` | merged, unproven: smith-13 |
| 08.4 | `59a8c624` | merged, unproven: smith-13 |
| 08.5 | `c61cfa0d` | merged, unproven: smith-13 |
| 08.8 | `85322db0` | deterministic |

## Increments

### 08.2 `opens_window`, and hosts resuming from the last window
Scope: smith 08's window turns resumed by hosts. The local host keeps
each told turn's `opens_window` with the turn, and its store loads only
the turns from the last one that opens a window, keeping earlier files on
disk (hosts.md, 5.3; transcript.md, section 3). The rest landed with
smith 05's `opens_window` on told turns (`07aeec1d`): the run sets it from
`Kind::opens_window` (`crates/smith-domain/src/route.rs:421`), and the
channel's turn record, the kit and the inline agent carry it.
Needs: nothing.
Files: `smith-local-domain` (`boundary.rs:202`, `domain.rs:827` and
`:858`; shared with session 05's 05.11 to 05.14);
`smith-local-protocol/src/store.rs` (`save_turn`, `decode_turns`);
`smith-local-shell` (`local_store.rs:117`, `local_host.rs:265`);
`smith-local-service/src/service.rs:548`; `tests/local/src/world.rs`
(`Store` at `:24`, `history` at `:55`, `SaveTurn` at `:1076`);
`tests/channel/tests/stories.rs`.
Shape: `Request::SaveTurn { number, read, opens_window: bool, turn }`,
`opens_window` copied from `smith_host_domain::Turn.opens_window`
(`channel.rs:122`). `save_turn(place, read, opens_window, turn, limits,
endpoints)`; the header becomes "SMLT", the read flag and token, then a
window flag byte. `decode_turns` keeps its signature and decodes the
bodies from the last file whose flag is 1; `read` comes from the last
file; a set of files with no flag set is `Malformed`. The store's bound
of 4,096 records (`local_store.rs:31`) counts the current window's files
only (decisions.md, 12.19). Where smith 10's chat store over io [10.1]
has landed, the flag and the bound go in its `Store`.
Test shape: the proof's test in `local_store.rs`'s tests module, beside
`synced_turn_recovers_the_read_fence_across_store_reopen` (`:320`),
reusing `directory()` (`:293`), `endpoints()` (`:298`), `turn()`
(`:306`) and `state()` (`:383`):

```rust
#[test]
fn a_chat_resumes_from_its_last_window_and_keeps_earlier_files() {
    let path = directory();
    let mut store = Store::new(path.clone(), endpoints(), 4096).expect("store opens");
    store.save_state(state(1), false).expect("state durable");
    let affinity = smith_domain::llm::Affinity { key: [0; 16], thread: 0 };
    let ordinary = Turn { kind: session::record::Kind::Ordinary, sequence: 2, ..turn() };
    let window = Turn { kind: session::record::Kind::Window { affinity }, sequence: 3, ..turn() };
    store.save_turn(1, None, true, &turn()).expect("first turn durable");
    store.save_turn(2, None, false, &ordinary).expect("ordinary turn durable");
    store.save_turn(3, Some(Token::new(2)), true, &window).expect("window turn durable");
    let local::Event::Loaded { transcript: Some(transcript), .. } = store.load().expect("records load") else {
        panic!("a saved chat loads");
    };
    assert_eq!(transcript.turns.iter().map(|turn| turn.sequence).collect::<Vec<_>>(), [3]);
    assert!(path.join("0000000001.turn").exists() && path.join("0000000002.turn").exists());
    fs::remove_dir_all(path).expect("remove test store");
}
```

Also: `a_load_starts_at_the_last_turn_that_opens_a_window` (`store.rs`,
like `:195`); `a_told_turns_window_flag_reaches_its_save`
(`smith-local-domain/src/tests.rs`); in the channel world, like
`stories.rs:426`, `a_host_resumes_from_its_last_window_turn_alone` (turns
first, ordinary, window, ordinary, window; the next activation's history
starts at place 5). Ceilings: unit 20 ms, world 0.5 s.
Proof: unit with real files, the store being where the choice is made:
`a_chat_resumes_from_its_last_window_and_keeps_earlier_files`, run once
with `heavy cargo nextest run -p smith-local-shell -E 'test(a_chat_resumes_from_its_last_window)'`.
It passes when the load holds only the window turn and the earlier files
remain; on the parent a chat holding a window turn after its first fails
to load (`StoreFailure::Read`). Under 1 s. No probe shows it live.
Don't: decode earlier windows' turns on load; delete or move their files.
Size: at most 80 lines of production code.
Removes: nothing.

### 08.6 Compaction into a new window, admitted against the window
Split (decisions.md, 13.13): the service's compaction records go to
08.10. From this increment, these move there, and 08.6 does not do them:
the Shape's sentences from "The service sets `response.started`'s
`window`" to "`end` `closed` or `failed`."; "`responses.rs:831` maps it
to `ConversationEnd::Failed` with no completion failure, and the
compaction's own `conversation.closed` is `failed` with none"; the Events
test `a_compaction_is_a_conversation_of_kind_compaction_in_the_stream`;
the Files `smith-agent-service/src/responses.rs` and `events.rs:596`;
and the first item of Removes. 08.6 gives the agent service only the
match arms its new facts and ends need, each doing what that match
does today for an ordinary completion's, and writes no compaction
record. One exception, from the designer's ruling of 2026-10-12
(session.md, 8.2, "The window's limit keeps its cause"; events.md,
3.10): 08.6 makes the window's limit keep its cause in the service's
terminals, as the Shape's last sentences say.
Scope: smith 08's compaction. The window check before each completion,
compaction on a failed check or *context too long*, the new window told
as a window turn, once per window (session.md, 8.1 and 8.2); a due
compaction counted by a child's final turn (session.md, section 7); the
run's summary instruction in each opening and in smith's own part
(run.md, 3.3; limits.md, 3.9); the compaction's facts and records
(events.md, 3.4 and 3.5). The transcript already has window turns and the
session resumes from one (`session.rs:2653`).
Needs: smith 08's window turns resumed by hosts (08.2). smith 04's charter
`window` and `output`, a window's room and the LLM path derived have
landed.
Files: `smith-domain-session`: `window.rs` and `compaction.rs` (new),
`session.rs` (`call` `:1743`, `call_failed` `:1706`, `final_prompt`
`:1771`, `complete` `:2062`, `tell_turn` `:3037`, restore near `:2653`),
`boundary.rs` (`Spec` `:253`), `limits.rs` (`Limits` `:21`, `derive`
`:244`), `llm.rs` (`Limit` `:368`), `facts.rs` (`:43`),
`final_turn.rs:54`, `lib.rs`. `smith-domain-run`: `prompt.rs`
(`fixed_prompt_bytes` `:42`, `render_fixed` `:59`), `boundary.rs`
(`Opening` `:639`), `opening.rs:47`, `run.rs:1941`.
`smith-domain/src/translate.rs:60` and `:103`. `smith-agent-service`:
`profile.rs:320`, `profile/room.rs` (docs at `:29`, `:39`),
`responses.rs` (`:201`, `:459`), `events.rs:596`, `children.rs:71` and
`:107`, `calls.rs:186` and `:255`. `smith-protocol-channel/src/facts.rs:32`.
Users of `Spec`, `Limits` and `FactKind`: `tests/session/src/world.rs`
(`:203`, `:276`, `:1153`), `recorded.rs:34`, `tests/memory.rs`,
`tests/recorded.rs`; `tests/agent/src/limits.rs`, `tests/agent/src/world.rs`
(`Settings` `:91`, `configuration` `:2134`, `main_model` `:2152`),
`tests/agent/tests/budget.rs`; `tests/protocol-llm/src/limits.rs`; the
`tests.rs` of `smith-domain`, `smith-domain-session` and
`smith-domain-run` (`:389`). Shared: `session.rs`, `run.rs`, the agent
service's records.
Shape: the sketches below. `Spec` gains `window` and `summary`, copied by
the root from `Opening.llm.window` and `Opening.summary`; `Limits` gains
`threshold`, copied from `Derivation.threshold` (`limits.rs:207`);
`llm::Limit` gains `Window`, whose bound is the window in tokens, and
`limit_name` (`responses.rs:797`) names it `window`. `Conversation` holds
a `Window`, a `Compaction`, the task, the summary and the model's window.
`take_note` passes whether the check would compact (`final_turn.rs:54`
passes `false` today). The service sets `response.started`'s `window`
from `CompletionStarted.window`; on `CompactionStarted` it writes
`conversation.opened` of kind `compaction`, `parent` the summarised
conversation, its thread and model kept, its number the next among the
run's conversations (events.md, 3.1); the compaction's responses
carry its number; on `CompactionEnded` it writes `conversation.closed`,
`end` `closed` or `failed`. `profile.rs:320` passes
`run::summary_bytes()` as `summary_bytes`. An answer with no summary:
`session::End` gains `NoSummary { fault: NoSummary }`, with `enum
NoSummary { Refused, Truncated, Malformed }`; `translate::end`
(`smith-domain/src/translate.rs:195`) maps it to `run::End::Fault` of
`Fault::Refused`, `Truncated` or `Malformed`, which the run already
answers as `Failure::Model` (`run.rs:2097`); `responses.rs:831` maps it
to `ConversationEnd::Failed` with no completion failure, and the
compaction's own `conversation.closed` is `failed` with none. No format
changes: the channel and the stream already carry the three faults.
The window's limit keeps its cause: after *context too long*, the
session ends `Failed { failure: Limit { which: Window, bound: window },
evidence }` with that attempt's evidence (`compaction.rs` gives `Unsent`
throughout today), and a failed check's end stays `Unsent`. The
service's `Ended` arm (`responses.rs:548-551`) keeps a typed limit's
class and `limit` and takes `evidence`, `status` and `detail` from the
captured native failure where there is one, so `conversation.closed`
and `run.completed` agree with it and `answer_failure`'s checks
(`responses.rs:264`) hold; that attempt's `response.completed` keeps
`context_too_long`.
Test shape: a new file, `tests/session/tests/compaction.rs`, on the
step-level world (`tests/session/src/recorded.rs`: `World` `:65`, `step`
`:180`, `open` `:285`, `complete` `:292`, `opening` `:34`, which gains a
window of 65,536 and a summary). Reference test:

```rust
fn said(text: &[u8]) -> Box<[llm::Block]> {
    Box::new([llm::Block::Text { text: text.into(), replay: None }])
}

#[test]
fn a_continue_past_the_threshold_compacts_and_holds_its_message_back() {
    let mut world = World::new(4701, 256);
    world.env.limits.threshold = [4, 5];
    let mut start = opening(None, world.env.limits.spend);
    start.spec.window = 2_000;
    start.spec.output = 512;
    start.spec.summary = b"Summarise the work so far.".as_slice().into();
    let task = start.spec.prompt.clone();
    world.open(start);
    let first = llm::Usage { input_tokens: Some(900), output_tokens: Some(300), ..llm::Usage::ZERO };
    world.complete(said(b"first answer"), llm::Stop::EndTurn, first);
    let session = world.session.expect("opened");
    world.step(session::Event::Continue { session, content: b"go on".as_slice().into() });
    // 900 + 300 + 5 + 512 = 1,717 tokens, past 1,600: the compaction is asked instead.
    let asked = world.prompts.last().expect("the compaction's request");
    assert_eq!(asked.messages.last().expect("its last message").content, said(b"Summarise the work so far."));
    world.complete(said(b"the summary"), llm::Stop::EndTurn, llm::Usage { output_tokens: Some(40), ..first });
    assert!(matches!(world.turns.last().expect("the window turn").kind, record::Kind::Window { .. }));
    let opened = world.prompts.last().expect("the new window's first request");
    let [message] = &*opened.messages else { panic!("one opening message: {opened:?}") };
    let expected = [&*task, b"the summary".as_slice(), b"go on".as_slice()];
    let texts: Vec<&[u8]> = message.content.iter().map(|block| {
        let llm::Block::Text { text, .. } = block else { panic!("a window opens with text: {block:?}") };
        &**text
    }).collect();
    assert_eq!(texts, expected);
}
```

Also in that file: `a_context_too_long_compacts_a_used_window`,
`context_too_long_on_the_compaction_fails_naming_the_window`,
`a_fresh_window_that_fails_its_check_fails_naming_the_window`,
`calls_in_a_compactions_answer_are_not_run`,
`a_cut_summary_opens_its_window_as_written`,
`a_refused_compaction_fails_as_refused_and_tells_no_turn`,
`a_compaction_with_no_text_fails_as_truncated_or_malformed` (whitespace
text counts as none),
`a_resumed_window_that_fails_its_check_compacts_first`,
`a_budget_with_no_room_for_a_compaction_ends_the_session`,
`a_child_at_its_threshold_with_two_turns_left_compacts_then_answers`.
Unit (`smith-domain-session/src/tests.rs`): `the_check_counts_reported_usage`,
`the_check_counts_bytes_without_usage`, `an_overflow_answers_compact`. Referee:
the prompt check at `world.rs:1153` accepts one window boundary per
compaction, at the first request after a window turn, and "no prompt whose
bound exceeds its window"; negatives in `tests/session/tests/referee.rs`:
`a_window_boundary_without_a_compaction_is_rejected`, `a_prompt_past_its_window_is_rejected`.
Fuzzy (`tests/session/tests/fuzzy_simulation.rs`): `drawn_small_windows_compact_within_them`. Agent world (new file `tests/agent/tests/compaction.rs`;
`Settings` gains `window`, which `main_model` and `configuration` use):
the proof's test,
`a_message_held_back_by_a_compaction_is_read_by_the_window_turn`, and
`a_refused_compaction_fails_the_run_as_refused_its_message_unread`. Events
(`smith-agent-service/src/tests.rs`):
`a_compaction_is_a_conversation_of_kind_compaction_in_the_stream`
(08.10's); 08.6's own: `a_window_limit_after_context_too_long_keeps_its_status_and_detail_in_every_terminal`
and `a_window_limit_from_a_failed_check_is_unsent_with_the_limits_words`,
and `native_provider_failure_keeps_status_and_capture_selected_detail_in_every_terminal`
(`tests/agent-process/tests/content.rs:255`), whose 400 *context too
long* falls in a fresh window: its `response` expectations stay, and its
`conversation` and `run` ones become class `limit`, `limit.which`
`window`, with the response's evidence, status and detail.
Ceilings: unit 20 ms, world 0.5 s, fuzzy 5 s.

```rust
// tests/agent/tests/compaction.rs
#[test]
fn a_compaction_counts_as_a_turn_and_as_spend_and_keeps_the_system_text() {
    let mut settings = Settings::calm(4801);
    settings.window = 6_144; // the calm job's tool rounds pass this window's threshold
    let mut world = World::new(settings);
    world.run(ITERATIONS);
    let Answer::Accepted { turns, .. } = world.answer() else { panic!("accepted: {:?}", world.answer()) };
    let opens: Vec<bool> = world.turns().iter().map(|turn| turn.kind.opens_window()).collect();
    let window = opens.iter().skip(1).position(|opens| *opens).expect("a window turn after the first") + 1;
    assert_eq!(*turns, u32::try_from(opens.len()).expect("few turns"), "the window turn counts as a turn");
    let spent: Vec<u64> = world.turn_metadata().iter().map(|(_, _, spent)| spent.units).collect();
    assert!(spent.get(window) > spent.get(window - 1), "the compaction is charged");
    let systems: Vec<&[u8]> = world.prompts().iter().map(|query| &*query.system).collect();
    assert!(systems.windows(2).all(|pair| pair[0] == pair[1]), "the system text never changes");
}
```

Proof: world, the real run, sessions and tools against the fake provider;
the process layer adds the store and the binaries, which compaction does
not touch. `a_compaction_counts_as_a_turn_and_as_spend_and_keeps_the_system_text`,
run once with `heavy cargo nextest run -p smith-agent-world -E 'binary(compaction)'`:
it passes when the run is accepted with a window turn after the first,
counted and charged, and one system text throughout; on the parent no
turn after the first opens a window. Under 0.5 s. `history/small-window`
(smith 00.16) shows it live; it is not this increment's proof.
Don't: edit history within a window; give a fact a field that no record
carries; add code to `session.rs` beyond call sites;
end a session on its history's bound in a new way (08.9 owns that).
Size: at most 700 lines of production code (decisions.md, 13.26; was 600): the window limit's projection in the service, 17bba0da, adds to the candidate's 600; README rule 9's 600 does not apply to 08.6.
Removes: `window: 1` and its comment in `responses.rs:200-201`; `0` as
`summary_bytes` in `profile.rs:320`.

### 08.10 A compaction's own conversation in the stream
Scope: the service's half of 08.6, split from it (decisions.md, 13.13):
a compaction told in the event stream as a conversation of its own
(events.md, 3.1, 3.4 and 3.5).
Needs: 08.6.
Files: `crates/smith-agent-service/src/responses.rs` (`:201`, `:459`,
`:797`, `:831`), `src/events.rs:596`, `src/tests.rs`.
Shape: as 08.6's Shape had it: "The service sets `response.started`'s
`window` from `CompletionStarted.window`; on `CompactionStarted` it writes
`conversation.opened` of kind `compaction`, `parent` the summarised
conversation, its thread and model kept, its number the next among the
run's conversations (events.md, 3.1); the compaction's responses carry
its number; on `CompactionEnded` it writes `conversation.closed`, `end`
`closed` or `failed`." `NoSummary` maps to `ConversationEnd::Failed`
with no completion failure, and the compaction's own
`conversation.closed` is `failed` with none. No format changes.
Test shape: `smith-agent-service/src/tests.rs`:
`a_compaction_is_a_conversation_of_kind_compaction_in_the_stream`, and
`a_refused_compaction_closes_failed_with_no_completion_failure`; each
reads the records as literals. Ceilings: unit 20 ms.
Proof: unit, `a_compaction_is_a_conversation_of_kind_compaction_in_the_stream`,
once: the stream holds a `conversation.opened` of kind `compaction`
whose `parent` is the summarised conversation, and the compaction's
`response.started` carries its number; on the parent no such record.
`history/small-window` (smith 00.16) shows it live; not this
increment's proof.
Don't: change a record's fields or the events version; write records
in the domain.
Size: at most 200 lines of production code.
Removes: `window: 1` and its comment in `responses.rs:200-201`.

### 08.7 The results owed to a compaction, cut to its room
Scope: a compaction's owed results render within the room the window
leaves after the history before them, the summary instruction and one
`output`, never less than the window above its threshold less those two;
each is cut at a character's boundary with its marker where they would
pass it (limits.md, 3.9; llm.md, section 5). Only that request's tail
differs.
Needs: smith 08's compaction (08.6).
Files: `smith-domain-session/src/llm.rs` (`Prompt` `:239`),
`session.rs` (`complete` `:2062`); `smith-domain/src/llm.rs` (`Prompt`
`:28`), `route.rs:787`; `smith-protocol-llm/src/prompt.rs` (`translate`
`:57`, `result_text` `:407`), `render.rs` (`render_outcome` `:20`,
`cut_text` `:433`). Users of both `Prompt`s: 23 files,
`rg -l 'max_tokens:' crates tests | xargs rg -l 'served:|delegated:'`.
Shared: the protocol-llm adapter, `session.rs`.
Shape: `Purpose` (sketch below) on the session's `Prompt` and, re-exported,
on the root's; the root copies it. The protocol renders a `Compaction`'s
owed results, the results message just before the summary instruction,
each within `min(rendered_result, room / owed)`, which the
`compaction-headroom` check keeps at or above `CUT_MARKER_BYTES`.
Test shape: unit (`smith-protocol-llm/src/tests.rs`, like `:124`):
`owed_results_past_their_room_are_cut_with_markers`; world
(`tests/protocol-llm/tests/production_render.rs`, reusing `limits()`
`:6`): `a_compactions_owed_results_are_cut_and_its_history_is_unchanged`.
Ceilings: unit 20 ms, world 0.5 s.
Proof: world, `a_compactions_owed_results_are_cut_and_its_history_is_unchanged`,
run once with `heavy cargo nextest run -p smith-protocol-llm-world -E 'test(owed_results)'`:
every owed result ends within the room with its marker and the messages
before them are byte-identical to the ordinary prompt's; on the parent
they render whole. Under 0.5 s. Live: `history/small-window`, as 08.6.
Don't: cut any result before the owed ones; cut an ordinary prompt.
Size: at most 120 lines of production code.
Removes: nothing.

### 08.9 History at its bounds compacts; transcript full goes
Scope: a history about to cross its message, block or byte bound is
compacted as a failed check is; in a fresh window it fails with
`Limit { which: History, bound }`. "A session never ends because its
history grew" (session.md, 8.1). The transcript-full end goes, with every
mapping of it. A completion's later batches, as the designer ruled on
2026-10-12 (session.md, 8.1, "A completion's results never cross
them"; limits.md, 3.2): the first batch's results fit whole by the
derivation, which stays at one batch (`profile.rs:506`); before each
later batch, `reserve_batch` (`session.rs` `advance`, near `:1443`)
gives each of its calls its share of what is left of the history, less
room for an answer to each call after it, as its cap: a read's window and
a command's kept output as the call's budget, a listing's and a search's
entries kept within it with the rest counted in `more`, a text result
cut with its marker. Every call runs and is answered; no call is answered
not run for this, and no new outcome or not-run reason is added. The
next completion is then the compaction, by the history trigger above.
Needs: smith 08's compaction (08.6).
Files: `smith-domain-session` (`boundary.rs:404`, the 14 sites of
`rg -n TranscriptFull crates/smith-domain-session/src/session.rs`,
`compaction.rs`); `smith-domain-run/src/boundary.rs:820`; `smith-domain`
(`translate.rs:210`, `feedback.rs:459`, `translation_tests.rs:49`);
`smith-host-domain/src/channel.rs:271`; `smith-inline-agent/src/translate.rs:159`;
`smith-host-protocol/src/answer.rs:120`; `smith-protocol-channel/src/translate.rs:627`;
`smith-channel` (schema `:114`, `generated/v2.rs` by skein-codegen, goldens
`enum_model_fault_context_full_*`); `smith-agent-service` (`answers.rs:154`,
`responses.rs:848`); the 9 test files of `rg -l
'TranscriptFull|ContextFull|context_full' tests`. Shared: `session.rs`,
the channel crates.
Shape: the history trigger joins 08.6's two; `End::TranscriptFull`,
`run::Fault::ContextFull`, `host::ModelFault::ContextFull` and the channel's
`context_full` go, under channel version 2.
Test shape: `tests/session/tests/compaction.rs`:
`a_history_at_its_message_bound_compacts_instead_of_ending`,
`a_fresh_window_at_its_history_bound_fails_naming_history`,
`a_later_batch_runs_within_what_is_left_of_the_history_then_compacts`;
fuzzy
(`fuzzy_simulation.rs`): `no_drawn_session_ends_because_its_history_grew`,
whose draws include completions of `calls_per_response` calls at their
caps near the threshold.
Ceilings: world 0.5 s, fuzzy 5 s.
Proof: world, `a_history_at_its_message_bound_compacts_instead_of_ending`,
run once with `heavy cargo nextest run -p smith-session-world -E 'test(history_at_its)'`:
it passes when the session compacts and goes on; on the parent it ends
`TranscriptFull`. Under 0.5 s. No probe shows it live: by derivation the
check fires first (`window-first`).
Don't: keep a transcript-full end under another name; size the history
for every call of a completion at its caps (it would pass `memory`);
answer a call not run because the history filled.
Size: at most 150 lines of production code, besides what it removes.
Removes: `End::TranscriptFull`, `Fault::ContextFull`, `ModelFault::ContextFull`,
`context_full` in the channel and in `run.completed`'s failure, their goldens
and tests.

## Interface sketches

`smith-domain-session` owns the window and the compaction; the run owns
the summary's words; the root copies the opening into the spec; the agent
service turns facts into records. Compiled under clippy at `564d4408`.

```rust
// crates/smith-domain-session/src/window.rs
/// A count of tokens; a byte counts as at most one (domain/session.md, 8.1).
pub(crate) struct Tokens(pub(crate) u64);
/// What the session knows of its current window.
pub(crate) struct Window {
    /// From 1, one more after each compaction.
    number: u32,
    /// Reported input, cache reads and cache writes; none before the window's first completion or when no input count was reported.
    last_prompt: Option<Tokens>,
    /// As reported, else the model's `output`; a new window counts its summary so.
    last_output: Tokens,
    /// What was appended since the last completion, a byte as a token.
    appended: Tokens,
    stage: Stage,
}
/// Fresh: opened by the start, a compaction or a resume from a window turn. Used: it made an ordinary completion.
pub(crate) enum Stage { Fresh, Used }
pub(crate) enum Admission { Fits, Compact }
pub(crate) fn check(window: &Window, prompt_bytes: u64, allowance: u64, output: u32, model: u32, threshold: [u32; 2]) -> Admission;
// bound = last_prompt + last_output + appended, or prompt_bytes + allowance when last_prompt is None
// limit = model * threshold[0] / threshold[1]
// Fits when bound + output <= limit; any overflow is Compact

// crates/smith-domain-session/src/compaction.rs
pub(crate) enum Compaction {
    Idle,
    /// What the opener appended since the last completion waits in `held` for the new window.
    Asking { held: List<Box<[u8]>> },
}

// crates/smith-domain-session/src/llm.rs (08.7); also on smith_domain::llm::Prompt, re-exported
/// What a completion is for: a compaction's owed results render within `room` bytes together.
pub enum Purpose { Ordinary, Compaction { room: u64 } }

// crates/smith-domain-session/src/facts.rs; `CompletionStarted` gains `window: u32`
pub enum FactKind { /* ... */ CompactionStarted { opener: Token, window: u32 }, CompactionEnded { opener: Token, window: u32, end: Compacted } }
pub enum Compacted { Opened, Failed }

// crates/smith-domain-run/src/prompt.rs and boundary.rs
pub(crate) const SUMMARY: &[u8];
/// The summary instruction's bytes, for the service's `compaction-headroom`.
pub fn summary_bytes() -> u64;

// crates/smith-local-protocol/src/store.rs
pub fn save_turn(place: u32, read: Option<Token>, opens_window: bool, turn: &smith_domain::Turn, limits: &transcript::v3::Limits, endpoints: &Endpoints) -> Result<File, StoreError>;
```

```text
window  compaction  event                                     next                      emits
Fresh   Idle        ordinary completion made                  Used                      -
Used    Idle        check fails, or context too long          Used, Asking              Complete (the summary instruction last)
Fresh   Idle        check fails, or context too long          -                         Ended: Limit { Window, window }; Unsent after a check, the response's evidence after context too long
Used    Asking      answer with text, cut or not; no refusal  Fresh (number + 1), Idle  Turn { kind: Window }; old history released
Used    Asking      a refusal: its stop, or a refusal block   -                         Ended: NoSummary { Refused }; no turn told
Used    Asking      no text, a stop at the output limit       -                         Ended: NoSummary { Truncated }; no turn told
Used    Asking      no text, any other stop                   -                         Ended: NoSummary { Malformed }; no turn told
Used    Asking      context too long                          -                         Ended: Limit { Window, window }, the response's evidence
Used    Asking      another failure, past its retries         -                         Ended: Failed, as any completion
Used    Asking      no room in a budget for it                -                         Ended: Budget
```

## Questions to raise when reached

None. A compaction cut short or refused, raised at 08.6, was settled by
the designer on 2026-10-12 (session.md, 8.2, `0f1d72ba`): "What the
design asks" quotes it, and 08.6 carries it.

## Done when

- The tests this session names pass, run once: `heavy cargo nextest run
  -p smith-local-shell -p smith-session-world -p smith-agent-world -p
  smith-protocol-llm-world -E 'test(a_chat_resumes_from_its_last_window) |
  binary(compaction) | test(owed_results)'`.
- Hosts keep the flag: `rg -l 'opens_window' crates/smith-local-domain/src/boundary.rs
  crates/smith-local-protocol/src/store.rs` lists both files.
- These find nothing: `rg -n 'window: 1,' crates/smith-agent-service/src/responses.rs`
  (the stream counts windows); `rg -n 'own, 0\)' crates/smith-agent-service/src/profile.rs`
  (the summary is in a window's room); `rg -n 'due\(turns, time, false\)'
  crates/smith-domain-session/src/final_turn.rs` (a final turn counts a due
  compaction); `rg -n 'TranscriptFull|ContextFull|context_full' crates tests`
  (no session ends because its history grew).
