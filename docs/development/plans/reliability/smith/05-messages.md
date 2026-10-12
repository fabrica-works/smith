# Session 05: messages, from the start to their terminals

Provisional, 2026-10-12. What is left is the local host's side of
messages. The run, the channel and both kinds of agent give each message
one terminal; the local host still keeps one line at a time, lets a line
go silently at the answer, shows "waiting" while a line is unread, and
writes notices as untyped bytes. This session makes the product keep the
person's lines as host.md, section 8 says, so that no line is lost, and
gives the worlds a person and referees that check it.

## Read first

- domain/host.md, sections 8 and 10; protocol/hosts.md, section 5.1;
- testing.md, sections 2.6 and 5; shell.md, section 4.1;
- benchmarks.md, section 11.1 (`first-request`, `follow-up`);
- decisions.md, 2.3, 2.5 and 2.9;
- findings L3, L6, L7, L8, L9, L11 and L14 (`findings/lifecycle.md`).

## What the design asks

- **Notices are typed** (hosts.md, section 5.1): "The local domain says
  what to show as notices of a closed set of kinds (waiting, checking, a
  line not delivered, a failure, a warning, a sign-in step), and the
  terminal protocol renders each as whole lines, ending each with a
  newline. Worlds observe notices by kind, never by their bytes."
- **The person's lines** (host.md, section 8): "A line starts a run when
  none is live, carried in its start"; later lines go to the live run as
  messages labelled `person`. "Held until read": at most the inbox's
  worth, and "past that, it reads no more of the person's input until a
  line is read"; a refused line is shown as not delivered, with why.
  "After an accepted or parked answer, the unread lines open the next
  activation, in order, under their names. After a cancel, a failure or
  a refusal, they are shown as not delivered, and dropped. An interrupt
  never starts a run by itself." "'Waiting' is shown only when every
  line relayed is read." "Closing input drains." Late saves and the
  agent's end after the answer "are handled in the answered state".
- **The worlds** (testing.md, section 5; host.md, section 10): a person
  who types on what it observes and closes its input anywhere, drawn
  from the seed in the fuzzy sweeps; referees for message terminals; "no
  line lost" as liveness.

## Choices this plan makes

- **Today's fixed lines get kinds; no kind waits for its producer.**
  05.11 types waiting, checking, lines not delivered, and the local
  host's own failures as `main` has them. smith 06's failures in words
  (06.9) change `Failure` in place and add the warning kind; smith 10's
  sign-in adds the sign-in kind. The answer's words stay `Show`.
- **One bounded queue of lines,** `person::Lines`, at most the inbox
  (`run::Limits::messages`) plus one cell for a line completed from bytes
  already read. At the inbox the domain asks for `Reading::Held`, and the
  service demands no more terminal bytes until `Reading::Open`.
- **The unread policy is one total function** of the answer (or none)
  and of whether a cancel was asked. A cancel asked drops the unread
  lines whatever the answer. A start that fails before the agent runs (no
  grant, a head not read) drops them too.
- **Closing input with carried lines** opens one more activation on
  them, which answers or parks; then the host exits (shell.md, 4.1).
- **The in-process world observes notices by kind;** the process tier
  sees the terminal from outside, so its scripts and endings keep the
  rendered words. The person lives in `tests/local`'s library.
- **Each referee rule lands with the increment that makes the host keep
  it:** relaying and fences in 05.12, the unread report and "no line
  lost" in 05.13, waiting in 05.14. "No failure's words in a turn or a
  prompt" is 06.9's.

## Rules that bind it

- decisions.md, 2.5: "The local host carries unread lines after an
  accepted or parked answer. After cancel, failure or refusal it shows
  them as not delivered and drops them. It never auto-starts after an
  interrupt."
- decisions.md, 2.9: "'Waiting' shows only when every relayed message
  has been read."
- code-organization.md's "Don't" list: "no type beside an existing one
  for the same thing: change the type in place, and its users in the same
  increment". `domain.rs` (1,803 lines on `main`) must not grow: before
  each `land`, `git diff --numstat origin/main --
  crates/smith-local-domain/src/domain.rs` shows no more lines added than
  removed. New logic goes in `person.rs`.

## Landed in the first run

| Increment | Commit | Proof |
|---|---|---|
| 05.1 | 5a85c8b4 | deterministic |
| 05.2 | 442276a9 | proven |
| 05.3 | 1aa9c934 | deterministic |
| 05.4 | ec232bb6 | merged, unproven: smith-00.14 |
| 05.inbox | 478fe6ed | deterministic |
| 05.5 | 10183b12 | merged, unproven: smith-00.14 |
| 05.6 | 841ee168 | merged, unproven: smith-00.14 |
| 05.opening-extract | 7be7a66a | deterministic |
| 05.7 | e69470e6 | merged, unproven: smith-13 |
| 05.8 | 07aeec1d | merged, unproven: smith-00.16 |
| 05.9 | 2482a2de | deterministic |
| 05.10 | c49746ea | merged, unproven: smith-00.14 |
| 05.legacy-messages | ef21912e | deterministic |

## Increments

The four need nothing outside the session (smith 03's inline agent and
stop ladder are on `main`), and run in order.

### 05.11 Typed notices
Scope: smith 05's typed notices: `Request::Notice { notice }` beside
  `Request::Show`, which keeps the run's words; the kinds of the sketch,
  rendered by the terminal protocol as whole lines ending in a newline,
  observed by kind in `tests/local` (hosts.md, section 5.1).
Needs: nothing.
Files: `smith-local-domain` (`boundary.rs:193`; `domain.rs`, each
  `Request::Show` but the turn's `:845` and the answer's `:972`;
  `person.rs:10`'s `store_failure` moves to the terminal);
  `smith-local-protocol/src/terminal.rs:97`;
  `smith-local-service/src/service.rs:522`;
  `smith-local-shell/src/local_host.rs:278`; tests: `parent_tests.rs`
  `:78`, `:134`, `:228`, `:249`, `:307`, `:327`;
  `tests/local/src/world.rs:1031`; `tests/local/tests/simulation.rs:682`;
  `tests/local/tests/fuzzy_simulation.rs:58`. Shared: `smith-local-domain`,
  with smith 06's 06.9 and smith 08's 08.2.
Shape: `smith_local_domain::{Notice, Checks, Undelivered, Failure,
  AccountFailure, AgentEnd}` and `Terminal::notice`, as sketched, with
  the words of the sketch's table, cut at `show_bytes` on a character
  boundary.
Test shape: `smith-local-protocol/src/terminal.rs`'s tests (`:131`), one
  per kind, expected bytes as literals; reference:
  ```rust
  #[test]
  fn a_refused_line_is_shown_by_name_and_reason_as_one_line() {
      let terminal = Terminal::new(Limits { line_bytes: 32, show_bytes: 64 }).expect("valid terminal");
      let mut out = Queue::with_capacity(max_out());
      let notice = local::Notice::NotDelivered {
          line: Some(Token::new(3)),
          why: local::Undelivered::Refused(smith_host_domain::MessageRefusal::Full),
      };
      terminal.notice(notice, b"main", &mut out);
      let Some(Event::Write(text)) = out.pop() else { panic!("one notice written") };
      assert_eq!(text.as_ref(), b"Line 3 not delivered: the inbox is full\n");
  }
  ```
  Also `a_store_failure_names_the_chat_and_its_turn_on_two_lines`,
  `waiting_is_one_line_ending_in_a_newline`; `parent_tests.rs:78`'s test
  becomes `waiting_and_both_long_notices_reach_the_person_by_kind`, on
  `running` (`parent_tests.rs:10`). Unit tests under 20 ms.
Proof: process layer: `tests/local-process/tests/simulation.rs`'s
  `a_store_failure_stops_the_chat_and_tells_the_person` passes in both
  placements, its words now rendered from `Failure::Store`: `heavy cargo
  nextest run -p smith-local-protocol -p smith-local-world -p
  smith-local-process-world`, once, under 10 seconds. No probe shows
  notices live; none is run.
Don't: add a `Warning` or `SignIn` kind, or any variant nothing
  produces; change the answer's words; compare notice bytes in
  `tests/local`.
Size: at most 250 lines of production code.
Removes: notices as raw bytes from the local domain; `person::
  store_failure`; `tests/local`'s comparisons of notice text.

### 05.12 The reacting person and the message referees
Scope: smith 05's reacting person and message referees: a `Person` in
  `tests/local`'s library that types on a turn, the answer, a notice by
  kind or a number of steps, and closes its input anywhere; the rules "a
  line relayed at most once in an activation, and again only if it was
  never read" and "fences never go back"; a fuzzy sweep of drawn people
  (testing.md, section 5; host.md, section 10).
Needs: smith 05's typed notices.
Files: `tests/local` (`src/person.rs`, new; `src/lib.rs`; `src/world.rs`:
  `World::person`, `World::close`, `World::drive_to_exit`;
  `src/referee.rs`, whose `Meeting::Name` becomes `Pending`);
  `tests/local/tests/{referee.rs,fuzzy_simulation.rs}`;
  `tests/local/tests/person.rs`, new;
  `tests/local-process/tests/fuzzy_world.rs:9`, which draws one to three
  `Action::Send` lines and an `Action::Eof` into its script;
  `tests/local-process/src/referee.rs` (`Meeting`, :63 to :100), whose
  rule "one activation per invocation" becomes "an activation starts
  only after the one before it answered", each `Started` resetting the
  activation's turn, answer and shown state and expecting its own
  terminal (shell.md, 4.1; decisions.md, 13.5), with one negative in
  `tests/local-process/tests/referee.rs`:
  `an_activation_started_before_the_last_answered_is_rejected`.
Shape: `smith_local_world::person::{Person, Cue, Trigger, Kind, Act,
  Shown}` and `referee::Pending`, as sketched. The world calls
  `Person::saw` on each turn, answer and notice, and `Person::step` once
  per iteration of `drive_until` (`world.rs:915`).
Test shape: `tests/local/tests/person.rs`, new; reference:
  ```rust
  #[test]
  fn a_person_typing_on_the_first_turn_has_both_lines_read() {
      let mut world = World::new(23);
      world.person(Person::new(vec![
          Cue { on: Trigger::Steps(0), act: Act::Type(b"What can you do?".to_vec()) },
          Cue { on: Trigger::Turn, act: Act::Type(b"And then?".to_vec()) },
          Cue { on: Trigger::Notice(Kind::Waiting), act: Act::Close },
      ]));
      assert!(world.drive_to_exit(2_000), "the chat parks and exits");
      assert_eq!(world.exit(), Some(ExitStatus::Success));
      assert_eq!(world.into_store().read(), Some(Token::new(2)), "the second line was read");
  }
  ```
  Negatives in `tests/local/tests/referee.rs`, on `broken` (`:12`):
  `a_line_relayed_twice_in_one_activation_is_rejected`,
  `a_read_line_relayed_again_is_rejected`,
  `a_fence_that_goes_back_is_rejected`. Fuzzy:
  `drawn_people_settle_against_the_message_rules`, seeds 0 to 63,
  `Person::drawn(seed)`, `slow_store` on seeds divisible by 3, about
  5 s. World tests under 0.5 s.
Proof: world layer, the highest a referee has: the three negatives fail
  their histories and the sweep passes: `heavy cargo nextest run -p
  smith-local-world`, then `heavy cargo nextest run -p smith-local-world
  -p smith-local-process-world --profile fuzzy`, once each, under 20
  seconds.
Don't: add the waiting, unread-report or liveness rules (05.13, 05.14);
  read the domain's state in the referee.
Needs also (decisions.md, 13.12): skein-07.s1 landed; 05.12 then
  repins skein at that commit, `cargo update -p skein-io --precise
  <its commit>` in `Cargo.lock`, and nothing else moves with it.
Size: no production code; at most 450 lines of test code.
Removes: the rules "a person line is relayed only once", "person lines
  reach the agent in order" and "each saved line gets one increasing
  name" (`tests/local/src/referee.rs:133`, `:146`, `:148`).

### 05.13 Lines carried, held until read, unread by outcome
Scope: smith 05's lines carried and held (host.md, section 8): the line
  that starts a run travels in its start's `messages`; `Lines` holds each
  line until a turn's, a wait's or the answer's fence covers it; at the
  inbox, input is held; a refused line is shown not delivered with why;
  at the answer, `policy` carries or drops the unread lines; closing
  input drains; late `StateSaved`, `TurnSaved` and `Gone` are cells of
  `Phase::Answered`. The referee gains "the unread report comes once" and
  "no line lost": `Pending::Line`, armed when a line is named, rearmed
  on each relay, met when it is read or shown not delivered.
Needs: smith 05's typed notices and its reacting person; for its Proof,
  smith 00's one command per probe run [00.27].
Files: `smith-local-domain` (`person.rs`; `chat.rs:10`; `facts.rs:8`;
  `boundary.rs`; `agent.rs:89`; `limits.rs:31`; `domain.rs` `:363`,
  `:375`, `:480`, `:660`, `:849`, `:959`, each handing to `person.rs`);
  `smith-local-service` (`service.rs:522`; `process.rs:568`, the
  terminal's next demand); `smith-local-shell` (`local_host.rs:278`,
  `local_settings.rs:361`); tests: `tests/local/src/{world.rs,referee.rs}`,
  `tests/local/tests/messages.rs` (new),
  `tests/local-process/src/referee.rs:92`,
  `tests/local-process/tests/{referee.rs,simulation.rs}`,
  `smith-local-service/src/tests.rs:27`, `tests/agent/src/script.rs:112`
  (a `@local-finish` script). Shared: `smith-local-domain` and the local
  store, with smith 08's 08.2; `tests/agent/src/script.rs`.
Shape: `person::{Lines, Stage, UnreadPolicy, policy}` and
  `Request::Input { reading: Reading }`, as sketched. `chat::Phase` gains
  `Answered { gone: Gone }`, `enum Gone { Owed, Heard }`, whose cells
  take a late `StateSaved`, a late `TurnSaved` and the agent's `Gone`.
  `local::Fact`, changed in place: `Message { name }` also for a carried
  line; `Answered { activation, read: Option<Token> }`, the final fence;
  `Unread { name }`, a name in the kit's report. `Undelivered::Full` goes
  and `Undelivered::Dropped` comes; `max_out` (`domain.rs:25`) counts
  the inbox where it counted `limits.lines`.
Test shape: unit tests in `person.rs`'s tests (`:144`), one per cell of
  `Lines` and of `policy`, such as
  `a_cancel_asked_drops_unread_lines_after_a_parked_answer`.
  Domain cells (a late `StateSaved` after the answer; `Gone` before and
  after a late save) in `parent_tests.rs`, on `running` (`:10`).
  `tests/local/tests/messages.rs`, new; reference:
  ```rust
  #[test]
  fn lines_then_the_end_of_input_are_each_read_before_the_host_exits() {
      let mut world = World::new(19);
      world.line(b"What can you do?");
      world.line(b"And in this directory?");
      world.close();
      assert!(world.drive_to_exit(2_000), "the chat parks and exits");
      assert_eq!(world.exit(), Some(ExitStatus::Success));
      assert_eq!(world.into_store().read(), Some(Token::new(2)), "both lines were read");
  }
  ```
  Also `a_follow_up_sent_as_the_run_finishes_is_read_once`,
  `a_line_at_the_moment_of_parking_opens_the_next_activation`,
  `an_interrupt_with_a_line_unread_shows_it_not_delivered_and_starts_nothing`,
  `a_brief_with_an_early_finish_carries_the_line_to_the_next_activation`
  (`World::finishing`), `a_slow_store_racing_the_answer_loses_no_line`,
  `the_held_input_reopens_when_a_fence_reads_a_line`; in
  `tests/local-process/tests/simulation.rs`, both placements:
  `a_follow_up_sent_as_the_run_finishes_is_read_once_in_both_placements`,
  `an_interrupt_with_a_line_unread_starts_nothing_in_both_placements`.
  Unit under 20 ms, world under 0.5 s.
Proof: process layer, a plain defect:
  `a_follow_up_sent_as_the_run_finishes_is_read_once_in_both_placements`
  fails on the parent, which drops the follow-up at the answer, and
  passes on the increment: `heavy cargo nextest run -p
  smith-local-process-world`, once, under 10 seconds. Live:
  `first-request` (interactive, calibrated) shows the first line
  travelling in its start: `heavy --gate cargo run -p smith-bench -- fix
  probes/first-request --parent <the increment's parent>`, both commits
  built first (README, rule 3), one attempt per arm. It passes when the
  increment's arm passes its deciding checks; at most $0.20 and 15
  minutes. `follow-up` (smith 00.14) is not this increment's proof.
Don't: grow `domain.rs`; send a line twice in one activation; start a
  run after an interrupt; keep `Limits::lines` beside the inbox.
Size: at most 500 lines of production code.
Removes: `raw_lines` and the slot `line: Option<Line>` (`domain.rs:45`,
  `:46`); the silent clear at the answer (`:975`); `unreachable!` on a
  late `StateSaved` (`:493`); `messages: Box::default()` (`agent.rs:89`);
  `Limits::lines` and its users (`limits.rs:31`, `:98`, `:119`;
  `domain.rs:26`, `:170`; `tests.rs:29`;
  `smith-local-service/src/tests.rs:27`; `local_settings.rs:361`;
  `tests/local/src/world.rs:457`).

### 05.14 "Waiting" only when every line is read
Scope: smith 05's waiting rule: `Notice::Waiting` is asked for only when
  the wait's fence leaves no relayed line unread (`Lines::all_read`); a
  wait whose fence is behind is a crossing, and shows nothing. The
  referee gains "waiting is shown only when every relayed line is read"
  (host.md, section 8; hosts.md, section 5.1).
Needs: smith 05's typed notices and its lines carried and held.
Files: `smith-local-domain/src/domain.rs:868`, handing to `person.rs`;
  `tests/local/src/referee.rs`; `tests/local/tests/{messages.rs,
  referee.rs}`; `tests/local-process/tests/simulation.rs`;
  `tests/local/tests/person.rs`, one cue only (decisions.md, 13.32).
Shape: no new type: `parent::Request::Waiting { read }` fences `Lines`,
  then asks for the notice when `all_read()`.
Test shape: in `tests/local/tests/messages.rs`, written like 05.13's
  reference: `no_waiting_notice_shows_while_a_sent_line_is_unread`, a
  person typing a second line on the first turn, the chat waiting with
  the store's read fence at the second line.
  Also `no_waiting_notice_shows_before_the_first_turn`;
  `no_waiting_notice_shows_while_a_line_is_unread_in_both_placements` in
  `tests/local-process/tests/simulation.rs`; the negative
  `a_waiting_notice_while_a_relayed_line_is_unread_is_rejected`. World
  under 0.5 s.
Proof: world layer, a plain defect:
  `no_waiting_notice_shows_while_a_sent_line_is_unread` fails on the
  parent and passes on the increment: `heavy cargo nextest run -p
  smith-local-world -p smith-local-process-world`, once, under 10
  seconds. No probe shows it live (benchmarks.md, 11.1).
Don't: delay or reword the notice.
Size: at most 40 lines of production code.
Removes: nothing.

## Interface sketches

`smith-local-domain` owns `Notice` and its kinds, `smith-local-protocol`
words them, and 06.9 and smith 10 change `Failure` and add kinds in
place. `Lines` is the domain's one record of the person's lines; its
fences, a refusal or `drop_unread` end each one.

```rust
// crates/smith-local-domain/src/boundary.rs (05.11; Input and Reading, 05.13)
pub enum Request {
    /// A notice of a closed set of kinds (protocol/hosts.md, section 5.1).
    Notice { notice: Notice },
    /// Read the person's input, or hold it until a line is read.
    Input { reading: Reading },
    // ...Show, the run's words, and the rest as on main
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Notice {
    /// The run waits, having read every line relayed to it (05.14).
    Waiting,
    Checking(Checks),
    /// `line` is the line's name once saved.
    NotDelivered { line: Option<Token>, why: Undelivered },
    /// smith 06's failures in words change it in place.
    Failure(Failure),
}
pub enum Checks { Started, Ended }
pub enum Undelivered {
    /// Past the terminal's line bound; never named.
    TooLong,
    /// No room to keep it; never named. 05.13 removes it.
    Full,
    /// The kit or the run refused it at its entrance.
    Refused(smith_host_domain::MessageRefusal),
    // 05.13: Dropped, unread when the run was cancelled, failed or refused.
}
pub enum Failure {
    NamesExhausted,
    Account(AccountFailure),
    HeadUnavailable,
    NotStarted,
    Agent(AgentEnd),
    /// `turn` names the turn whose save failed.
    Store { turn: Option<u32>, reason: StoreFailure },
    DeliveryUnsaved,
}
pub enum AccountFailure { Unavailable, NotRefreshed, Exhausted }
pub enum AgentEnd { Failed, Dropped, NoAnswer }
pub enum Reading { Open, Held }

// crates/smith-local-protocol/src/terminal.rs (05.11)
impl Terminal {
    /// Write `notice` as whole lines, each ending in a newline, within `show_bytes`.
    pub fn notice(&self, notice: local::Notice, chat: &[u8], out: &mut Queue<Event>);
}
```

| Notice | Words, each line ending in a newline |
|---|---|
| `Waiting`; `Checking(Started)`, `(Ended)` | `Waiting for a message`; `Running checks`, `Checks finished` |
| `NotDelivered` with `TooLong`, `Full` | `Line is too long`, `Too many lines waiting` |
| `NotDelivered` with `Refused(..)`, `Dropped` | `Line N not delivered: ` then `it is too large`, `the inbox is full`, `the run is ending`, `its name is in use`, or `the run ended without reading it` |
| `Failure(..)` | the words `domain.rs` shows on `main`; the store's on two lines |

```rust
// crates/smith-local-domain/src/person.rs (05.13)
/// Lines received and not yet read, in order, at most the inbox plus one.
pub(crate) struct Lines { held: Queue<Line>, inbox: u32 }
pub(crate) struct Line { text: Box<[u8]>, stage: Stage }
pub(crate) enum Stage {
    Received,
    Naming { name: Token },
    /// Saved; for a start to carry or an admitted run to take.
    Named { name: Token },
    /// Carried in a start, or relayed to the live run.
    Relayed { name: Token },
}
pub(crate) enum UnreadPolicy { Carry, Drop }
/// `result` is `None` when the agent ended with no answer.
pub(crate) fn policy(result: Option<&host::RunResult>, cancel_asked: bool) -> UnreadPolicy {
    if cancel_asked {
        return UnreadPolicy::Drop;
    }
    match result {
        Some(host::RunResult::Accepted { .. } | host::RunResult::Parked) => UnreadPolicy::Carry,
        Some(host::RunResult::Refused { .. } | host::RunResult::Failed { .. }) | None => UnreadPolicy::Drop,
    }
}
impl Lines {
    pub(crate) fn receive(&mut self, text: Box<[u8]>) -> Result<(), Box<[u8]>>;
    /// Pops each relayed line up to `read`; returns how many.
    pub(crate) fn fence(&mut self, read: Option<Token>) -> u32;
    pub(crate) fn refused(&mut self, name: Token) -> bool;
    pub(crate) fn all_read(&self) -> bool;
    pub(crate) fn reading(&self) -> Reading;
    /// The named lines a start carries, marked relayed, in order.
    pub(crate) fn carry(&mut self) -> Box<[host::Message]>;
    /// After `Drop`: the names to show as not delivered, popped.
    pub(crate) fn drop_unread(&mut self) -> Box<[Token]>;
}

// tests/local/src/person.rs (05.12)
pub enum Trigger { Turn, Answer, Notice(Kind), Steps(u32) }
pub enum Kind { Waiting, Checking, NotDelivered, Failure }
pub enum Act { Type(Vec<u8>), Interrupt, Close }
pub struct Cue { pub on: Trigger, pub act: Act }
/// What the terminal showed, as the person sees it: never its bytes.
pub enum Shown { Turn, Answer, Notice(local::Notice) }
impl Person {
    pub fn new(cues: Vec<Cue>) -> Person;
    /// One to three lines at triggers drawn from `seed`, input closed anywhere.
    pub fn drawn(seed: u64) -> Person;
    pub fn saw(&mut self, shown: Shown) -> Option<Act>;
    pub fn step(&mut self) -> Option<Act>;
}

// tests/local/src/referee.rs (05.12): the liveness names
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pending { Line(Token) }
```

## Questions to raise when reached

None: shell.md, 4.1 settles the end of input; decisions.md, 12.19 lines lost.

## Done when

- `rg -c 'Request::Show \{ text: Box::from' crates/smith-local-domain/src/domain.rs`
  prints nothing (15 on `main`).
- `rg -n 'b"Waiting for a message"|b"A model account is exhausted"|b"The chat could not be saved"' tests/local`
  prints nothing (4 on `main`).
- `rg -n '"a person line is relayed only once"|"person lines reach the agent in order"|"each saved line gets one increasing name"' tests/local/src/referee.rs`
  prints nothing (3 on `main`).
- `rg -n 'raw_lines|line: Option<Line>|messages: Box::default\(\)' crates/smith-local-domain/src/domain.rs crates/smith-local-domain/src/agent.rs`
  prints nothing (6 on `main`).
- `rg -n 'SaveState is emitted only while starting or running' crates/smith-local-domain/src`
  prints nothing (1 on `main`).
- `heavy cargo nextest run -p smith-local-domain -p smith-local-protocol -p smith-local-world -p smith-local-process-world`
  passes (138 tests in 0.5 s on `main`).
- `heavy cargo nextest run -p smith-local-world -p smith-local-process-world --profile fuzzy`
  passes, each test under about 5 s (0.07 s and 2.5 s on `main`).
