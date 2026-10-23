//! Open main lifecycle at the existing run entrance (domain/run.md, sections 3.1, 6 and 9.1).
#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "programming-model.md, section 10.2: deterministic test harnesses are ordinary Rust"
)]

use super::*;
use crate::Mode;

const HOST: u64 = 1;
const PEER: u64 = 9;
const FIRST: Token = Token::new(99);
const NEXT: Token = Token::new(0);
const IDLE: Duration = Duration::from_secs(30); // The shared charter's shipped test interval.

fn policy() -> Charter {
    Charter { mode: Mode::Open, ..charter() }
}

fn listening() -> (Harness, Token, Token) {
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(HOST, PEER, policy());
    drop(harness.step(reply(main, Stop::EndTurn, b"Hello")));
    (harness, run, main)
}

fn finish_policy() -> Charter {
    let mut selected = policy();
    selected.budget.turns = 8; // Approved Close reproducer, domain/run.md section 6.
    selected.reserve = crate::charter::Reserve { turns: 1, time: Duration::from_secs(60) };
    selected.waiting = Duration::from_secs(120);
    selected
}

#[test]
fn admitted_finish_request_wakes_listening_and_next_reply_parks() {
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(HOST, PEER, finish_policy());
    drop(harness.step(told(main, 1)));
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Hello")).as_ref(),
        &[Request::Waiting { host_run: Token::new(HOST), read: None }]
    );
    assert_eq!(
        harness.step(finish_request(run, FIRST, b"Please finish")).as_ref(),
        &[Request::Say { peer: Token::new(PEER), text: bytes(b"person: Please finish") }]
    );
    drop(harness.step(told(main, 2)));
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Goodbye")).as_ref(),
        &[
            Request::Waiting { host_run: Token::new(HOST), read: Some(FIRST) },
            Request::Close { peer: Token::new(PEER) }
        ]
    );
    let emitted = harness.step(settled(main, Spend::ZERO));
    let [Request::Answer { answer, read, .. }] = emitted.as_ref() else { panic!("one parked terminal") };
    assert_eq!(*answer, Answer::Parked { spent: Spend::ZERO, turns: 2 });
    assert_eq!(*read, Some(FIRST));
}

fn reply(main: Token, stop: Stop, text: &[u8]) -> Event {
    Event::Yielded { conversation: main, stop, text: bytes(text) }
}

fn message(run: Token, name: Token, text: &[u8]) -> Event {
    Event::Message { finish_requested: false, run, name, label: bytes(b"person"), text: bytes(text) }
}

fn finish_request(run: Token, name: Token, text: &[u8]) -> Event {
    Event::Message { run, name, label: bytes(b"person"), text: bytes(text), finish_requested: true }
}

fn carried_finish() -> crate::Message {
    crate::Message { finish_requested: true, name: FIRST, label: bytes(b"person"), text: bytes(b"Please finish") }
}

fn prepared_finish(carried: bool) -> (Harness, Token, Token) {
    let mut harness = Harness::new(LIMITS);
    let messages: Box<[crate::Message]> = if carried { Box::new([carried_finish()]) } else { Box::default() };
    let emitted = harness.step(carried_start(finish_policy(), messages, Some(workspace())));
    let [Request::Admitted { run, .. }, Request::Read { .. }] = emitted.as_ref() else { panic!("preparing") };
    let run = *run;
    if !carried {
        drop(harness.step(finish_request(run, FIRST, b"Please finish")));
    }
    let emitted = harness.step(Event::Read { owner: run, read: Read::Missing });
    let [Request::Open { conversation, .. }] = emitted.as_ref() else { panic!("main opens") };
    let main = *conversation;
    drop(harness.step(Event::Started { conversation: main, peer: Token::new(PEER) }));
    drop(harness.step(told(main, 1)));
    (harness, run, main)
}

#[test]
fn start_and_preparing_finish_requests_survive_until_main_replies() {
    for carried in [true, false] {
        let (mut harness, _, main) = prepared_finish(carried);
        assert_eq!(
            harness.step(reply(main, Stop::EndTurn, b"Goodbye")).as_ref(),
            &[
                Request::Waiting { host_run: Token::new(HOST), read: Some(FIRST) },
                Request::Close { peer: Token::new(PEER) }
            ]
        );
        assert_eq!(
            answered(harness.step(settled(main, Spend::ZERO))),
            (HOST, Answer::Parked { spent: Spend::ZERO, turns: 1 })
        );
    }
}

#[test]
fn finish_request_wakes_wait_and_preserves_unread_words() {
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(HOST, PEER, finish_policy());
    drop(harness.step(wait(main)));
    assert_eq!(
        harness.step(finish_request(run, FIRST, b"Please finish")).as_ref(),
        &[returned(20, Returned::Waited { text: crate::prompt::waited() })]
    );
    drop(harness.step(told(main, 1)));
    drop(harness.step(reply(main, Stop::EndTurn, b"Goodbye")));
    let emitted = harness.step(settled(main, Spend::ZERO));
    let [Request::Answer { answer, read, .. }] = emitted.as_ref() else { panic!("one parked terminal") };
    assert_eq!(*answer, Answer::Parked { spent: Spend::ZERO, turns: 1 });
    assert_eq!(*read, None);
    let unread: Vec<_> = core::iter::from_fn(|| harness.pop_fact())
        .filter_map(|fact| if let Fact::MessageUnread { name, .. } = fact.kind { Some(name) } else { None })
        .collect();
    assert_eq!(unread, [FIRST]);
}

#[test]
fn refused_finish_requests_leave_ordinary_policy_and_deadlines_intact() {
    let oversized = vec![b'x'; usize::try_from(LIMITS.message_bytes).unwrap()];
    for (capacity, name, text, reason) in [
        (LIMITS.messages, NEXT, oversized.as_slice(), crate::MessageRefusal::TooLarge),
        (LIMITS.messages, FIRST, b"finish".as_slice(), crate::MessageRefusal::NameInUse),
        (1, NEXT, b"finish".as_slice(), crate::MessageRefusal::Full),
    ] {
        let mut harness = Harness::new(Limits { messages: capacity, ..LIMITS });
        let (run, main) = harness.running_on(HOST, PEER, finish_policy());
        drop(harness.step(message(run, FIRST, b"Pending")));
        let deadline = harness.domain.next_deadline();
        assert_eq!(
            harness.step(finish_request(run, name, text)).as_ref(),
            &[Request::MessageRefused { host_run: Token::new(HOST), name, reason }]
        );
        assert_eq!(harness.domain.next_deadline(), deadline);
        assert_eq!(
            crate::completion_permit(&mut harness.domain, main, harness.env.now, crate::Most::Unreserved),
            crate::CompletionPermit::Allowed
        );
    }
}

#[test]
fn cancel_after_finish_request_still_cancels_the_run() {
    let (mut harness, run, main) = listening();
    drop(harness.step(finish_request(run, FIRST, b"Please finish")));
    assert_eq!(harness.step(Event::Cancel { run }).as_ref(), &[Request::Close { peer: Token::new(PEER) }]);
    assert_eq!(
        answered(harness.step(settled(main, Spend::ZERO))),
        (HOST, Answer::Failed { failure: Failure::Cancelled, spent: Spend::ZERO, turns: 0 })
    );
    harness.domain.reclaim();
    assert!(harness.step(finish_request(run, NEXT, b"stale")).is_empty());
}

#[test]
fn finite_ignores_finish_bit_and_keeps_its_terminal_tool() {
    let mut harness = Harness::new(LIMITS);
    let selected = Charter { mode: Mode::Finite, ..finish_policy() };
    let emitted = harness.step(carried_start(selected, Box::new([carried_finish()]), None));
    let [Request::Admitted { run, .. }, Request::Open { conversation, opening }] = emitted.as_ref() else {
        panic!("finite main opens")
    };
    assert!(opening.finish);
    let (run, main) = (*run, *conversation);
    drop(harness.step(Event::Started { conversation: main, peer: Token::new(PEER) }));
    drop(harness.step(told(main, 1)));
    drop(harness.step(finish_request(run, NEXT, b"Another amendment")));
    assert_eq!(
        crate::completion_permit(&mut harness.domain, main, harness.env.now, crate::Most::Unreserved),
        crate::CompletionPermit::Allowed
    );
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Hello")).as_ref(),
        &[Request::Say { peer: Token::new(PEER), text: bytes(b"person: Another amendment") }]
    );
}

#[test]
fn duplicate_finish_request_cannot_renew_the_final_allowance() {
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(HOST, PEER, finish_policy());
    drop(harness.step(reply(main, Stop::EndTurn, b"Hello")));
    drop(harness.step(finish_request(run, FIRST, b"Please finish")));
    let permit = crate::completion_permit(&mut harness.domain, main, harness.env.now, crate::Most::Unreserved);
    let crate::CompletionPermit::Final { note: Some(note) } = permit else { panic!("host final work") };
    assert!(core::str::from_utf8(&note).unwrap().starts_with("The host requested wind-down."));
    let owner = Token::new(500);
    assert_eq!(crate::reserve(&mut harness.domain, main, owner, crate::Most::Unreserved), Ok(()));
    assert!(crate::settle_reservation(&mut harness.domain, main, owner, 0));
    assert_eq!(
        harness.step(finish_request(run, NEXT, b"Again")).as_ref(),
        &[Request::MessageRefused { host_run: Token::new(HOST), name: NEXT, reason: crate::MessageRefusal::Ending }]
    );
    assert_eq!(
        crate::completion_permit(&mut harness.domain, main, harness.env.now, crate::Most::Unreserved),
        crate::CompletionPermit::Denied(Exhausted::Turns)
    );
}

#[test]
fn finish_request_waits_for_the_existing_turn_ack_credit() {
    let mut harness = Harness::new(LIMITS);
    let mut start = carried_start(finish_policy(), Box::default(), None);
    let Event::Start { window, .. } = &mut start else { panic!("Start fixture") };
    *window = crate::Window { turns: 1, bytes: 1, largest_turn: 1 };
    let emitted = harness.step(start);
    let [Request::Admitted { run, .. }, Request::Open { conversation, .. }] = emitted.as_ref() else { panic!("main") };
    let (run, main) = (*run, *conversation);
    drop(harness.step(Event::Started { conversation: main, peer: Token::new(PEER) }));
    drop(harness.step(told(main, 1)));
    drop(harness.step(reply(main, Stop::EndTurn, b"Hello")));
    drop(harness.step(finish_request(run, FIRST, b"Please finish")));
    assert_eq!(
        crate::completion_permit(&mut harness.domain, main, harness.env.now, crate::Most::Unreserved),
        crate::CompletionPermit::Held
    );
    drop(harness.step(Event::Acknowledge { run, turn: 1 }));
    let permit = crate::completion_permit(&mut harness.domain, main, harness.env.now, crate::Most::Unreserved);
    let crate::CompletionPermit::Final { note: Some(_) } = permit else { panic!("final after ACK") };
}

fn told(main: Token, sequence: u32) -> Event {
    Event::Turn { conversation: main, record: Token::new(u64::from(sequence)), sequence, opens_window: sequence == 1 }
}

fn settled(main: Token, spend: Spend) -> Event {
    Event::Ended { conversation: main, end: End::Closed, spend, detail: Box::default() }
}

#[test]
fn open_requires_wait_and_refuses_every_delivery_grant() {
    for (wait, deliver) in [(false, None), (true, Some(change_spec())), (false, Some(change_spec()))] {
        let mut selected = policy();
        selected.grants.wait = wait;
        selected.grants.deliver = deliver;
        assert_eq!(
            answered(Harness::new(LIMITS).start_workspace(HOST, selected, None)),
            (HOST, Answer::Refused(Refusal::Invalid(Invalid::MalformedCharter)))
        );
    }
}

fn change_spec() -> ChangeSpec {
    ChangeSpec { fields: Box::new([]), checks_must_pass: false }
}

#[test]
fn open_neither_offers_nor_prices_the_finish_schema() {
    let mut limits = LIMITS;
    limits.schema_costs.finish_text = u64::MAX;
    let mut harness = Harness::new(limits);
    let emitted = harness.start_workspace(HOST, policy(), None);
    let [Request::Admitted { .. }, Request::Open { opening, .. }] = emitted.as_ref() else {
        panic!("open admitted without finish schema room: {emitted:?}");
    };
    assert!(!opening.finish && !opening.deliver && opening.wait);
    assert_eq!((opening.generated_schema_bytes, opening.ordinal), (0, 0));
}

#[test]
fn text_reply_listens_until_its_exact_idle_deadline() {
    let mut harness = Harness::new(LIMITS);
    let (_, main) = harness.running_on(HOST, PEER, policy());
    drop(harness.step(told(main, 1)));
    harness.after(Duration::from_secs(5));
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Hello")).as_ref(),
        &[Request::Waiting { host_run: Token::new(HOST), read: None }]
    );
    let deadline = Time::ZERO.saturating_add(Duration::from_secs(35)); // Five seconds of work + charter waiting.
    assert_eq!(harness.domain.next_deadline(), Some(deadline));
    harness.after(Duration::from_nanos(IDLE.as_nanos() - 1));
    assert!(!harness.domain.is_due(harness.env.now));
    harness.after(Duration::from_nanos(1));
    assert_eq!(harness.fire().as_ref(), &[Request::Close { peer: Token::new(PEER) }]);
    assert_eq!(
        answered(harness.step(settled(main, Spend::ZERO))),
        (HOST, Answer::Parked { spent: Spend::ZERO, turns: 1 })
    );
    harness.domain.reclaim();
    assert_eq!((harness.domain.runs(), harness.domain.conversations(), harness.domain.calls()), (0, 0, 0));
}

#[test]
fn listening_words_continue_once_and_advance_the_told_fence() {
    let (mut harness, run, main) = listening();
    harness.after(IDLE); // Input on the deadline wins before timer firing.
    assert_eq!(
        harness.step(message(run, NEXT, b"Next")).as_ref(),
        &[Request::Say { peer: Token::new(PEER), text: bytes(b"person: Next") }]
    );
    assert!(!harness.domain.is_due(harness.env.now));
    drop(harness.step(told(main, 1)));
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Second reply")).as_ref(),
        &[Request::Waiting { host_run: Token::new(HOST), read: Some(NEXT) }]
    );
    let read: Vec<_> = core::iter::from_fn(|| harness.pop_fact())
        .filter_map(|fact| if let Fact::MessageRead { name, .. } = fact.kind { Some(name) } else { None })
        .collect();
    assert_eq!(read, [NEXT]);
}

#[test]
fn queued_words_win_before_implicit_listening() {
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(HOST, PEER, policy());
    drop(harness.step(message(run, FIRST, b"One")));
    drop(harness.step(message(run, NEXT, b"Two")));
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Hello")).as_ref(),
        &[Request::Say { peer: Token::new(PEER), text: bytes(b"person: One\n\nperson: Two") }]
    );
    drop(harness.step(told(main, 1)));
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Answered both")).as_ref(),
        &[Request::Waiting { host_run: Token::new(HOST), read: Some(NEXT) }]
    );
}

#[test]
fn wind_down_reply_parks_and_keeps_unread_words_for_later() {
    let mut selected = policy();
    selected.reserve = crate::charter::Reserve { turns: 1, time: Duration::from_secs(60) };
    let ordinary = Duration::from_nanos(selected.budget.time.as_nanos() - selected.reserve.time.as_nanos());
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(HOST, PEER, selected);
    drop(harness.step(message(run, FIRST, b"Read")));
    drop(harness.step(reply(main, Stop::EndTurn, b"Hello")));
    drop(harness.step(told(main, 1)));
    drop(harness.step(message(run, NEXT, b"Later")));
    harness.after(ordinary);
    drop(harness.fire());
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Last reply")).as_ref(),
        &[
            Request::Waiting { host_run: Token::new(HOST), read: Some(FIRST) },
            Request::Close { peer: Token::new(PEER) }
        ]
    );
    let emitted = harness.step(settled(main, Spend::ZERO));
    let [Request::Answer { answer, read, .. }] = emitted.as_ref() else { panic!("one parked terminal") };
    assert_eq!(*answer, Answer::Parked { spent: Spend::ZERO, turns: 1 });
    assert_eq!(*read, Some(FIRST));
    let unread: Vec<_> = core::iter::from_fn(|| harness.pop_fact())
        .filter_map(|fact| if let Fact::MessageUnread { name, .. } = fact.kind { Some(name) } else { None })
        .collect();
    assert_eq!(unread, [NEXT]);
}

#[test]
fn zero_waiting_text_reply_parks_without_a_call() {
    let mut harness = Harness::new(LIMITS);
    let (_, main) = harness.running_on(HOST, PEER, Charter { waiting: Duration::ZERO, ..policy() });
    assert_eq!(
        harness.step(reply(main, Stop::EndTurn, b"Hello")).as_ref(),
        &[Request::Waiting { host_run: Token::new(HOST), read: None }, Request::Close { peer: Token::new(PEER) }]
    );
    assert_eq!(
        answered(harness.step(settled(main, Spend::ZERO))),
        (HOST, Answer::Parked { spent: Spend::ZERO, turns: 0 })
    );
}

fn wait(main: Token) -> Event {
    Event::Delegated {
        conversation: main,
        call: Token::new(20),
        name: crate::CallName { activation: 1, completion: 1, position: 0 },
        ask: Ask::Wait,
        deadline: EXPIRY,
    }
}

#[test]
fn explicit_open_wait_can_park_without_any_text() {
    let mut harness = Harness::new(LIMITS);
    let (_, main) = harness.running_on(HOST, PEER, Charter { waiting: Duration::ZERO, ..policy() });
    assert_eq!(
        harness.step(wait(main)).as_ref(),
        &[
            Request::Waiting { host_run: Token::new(HOST), read: None },
            returned(20, Returned::Withheld),
            Request::Close { peer: Token::new(PEER) }
        ]
    );
    assert_eq!(
        answered(harness.step(settled(main, Spend::ZERO))),
        (HOST, Answer::Parked { spent: Spend::ZERO, turns: 0 })
    );
}

#[test]
fn unsuccessful_open_ends_keep_the_existing_failure_policy() {
    for (stop, text, failure) in [
        (Stop::EndTurn, b"".as_slice(), Failure::Policy(Policy::Unfinished { nudges: 2, rejected: 0 })),
        (Stop::MaxTokens, b"truncated".as_slice(), Failure::Model(Fault::Truncated)),
        (Stop::Refusal, b"refused".as_slice(), Failure::Model(Fault::Declined)),
        (Stop::NoCalls, b"no calls".as_slice(), Failure::Model(Fault::Malformed)),
    ] {
        let mut harness = Harness::new(LIMITS);
        let (_, main) = harness.running_on(HOST, PEER, policy());
        for _ in 0..LIMITS.nudges {
            let emitted = harness.step(reply(main, stop, text));
            let [Request::Say { text, .. }] = emitted.as_ref() else { panic!("one nudge") };
            assert!(!text.windows(b"`finish`".len()).any(|bytes| bytes == b"`finish`"));
        }
        assert_eq!(harness.step(reply(main, stop, text)).as_ref(), &[Request::Close { peer: Token::new(PEER) }]);
        assert_eq!(
            answered(harness.step(settled(main, Spend::ZERO))),
            (HOST, Answer::Failed { failure, spent: Spend::ZERO, turns: 0 })
        );
    }
}

#[test]
fn listening_cancel_has_no_tool_terminal() {
    let (mut harness, run, main) = listening();
    assert_eq!(harness.step(Event::Cancel { run }).as_ref(), &[Request::Close { peer: Token::new(PEER) }]);
    assert_eq!(
        answered(harness.step(settled(main, Spend::ZERO))),
        (HOST, Answer::Failed { failure: Failure::Cancelled, spent: Spend::ZERO, turns: 0 })
    );
}

#[test]
fn a_listening_withdrawal_matches_no_retained_call() {
    let (mut harness, _, main) = listening();
    assert!(harness.step(Event::Withdraw { conversation: main, call: Token::new(20) }).is_empty());
    assert_eq!(harness.domain.next_deadline(), Some(Time::ZERO.saturating_add(IDLE)));
}

#[test]
fn listening_ordinary_deadline_parks_without_final_work() {
    let mut selected = policy();
    selected.waiting = LIMITS.waiting;
    selected.budget.time = Duration::from_secs(120);
    selected.reserve = crate::charter::Reserve { turns: 1, time: Duration::from_secs(60) };
    let mut harness = Harness::new(LIMITS);
    let (_, main) = harness.running_on(HOST, PEER, selected);
    drop(harness.step(reply(main, Stop::EndTurn, b"Hello")));
    harness.after(Duration::from_secs(60));
    assert_eq!(harness.fire().as_ref(), &[Request::Close { peer: Token::new(PEER) }]);
    assert_eq!(
        answered(harness.step(settled(main, Spend::ZERO))),
        (HOST, Answer::Parked { spent: Spend::ZERO, turns: 0 })
    );
}

#[test]
fn listening_usage_overflow_stops_without_a_call_terminal() {
    let (mut harness, _, main) = listening();
    let prior = Spend { units: 0, ..spend(u64::MAX) };
    drop(harness.step(Event::Used { conversation: main, spend: prior }));
    assert_eq!(
        harness.step(Event::Used { conversation: main, spend: Spend { units: 0, ..spend(1) } }).as_ref(),
        &[Request::Close { peer: Token::new(PEER) }]
    );
    assert_eq!(
        answered(harness.step(settled(main, prior))),
        (
            HOST,
            Answer::Failed {
                failure: Failure::Budget {
                    exhausted: Exhausted::Overflow(crate::Overflow::Usage),
                    used: None,
                    bound: None
                },
                spent: prior,
                turns: 0,
            }
        )
    );
}

#[test]
fn a_reply_does_not_spend_or_reset_unsuccessful_nudges() {
    let mut harness = Harness::new(LIMITS);
    let (run, main) = harness.running_on(HOST, PEER, policy());
    drop(harness.step(reply(main, Stop::EndTurn, b"")));
    drop(harness.step(reply(main, Stop::EndTurn, b"Hello")));
    drop(harness.step(message(run, NEXT, b"Continue")));
    drop(harness.step(reply(main, Stop::EndTurn, b"")));
    assert_eq!(harness.step(reply(main, Stop::EndTurn, b"")).as_ref(), &[Request::Close { peer: Token::new(PEER) }]);
    assert_eq!(
        answered(harness.step(settled(main, Spend::ZERO))),
        (
            HOST,
            Answer::Failed {
                failure: Failure::Policy(Policy::Unfinished { nudges: 2, rejected: 0 }),
                spent: Spend::ZERO,
                turns: 0
            }
        )
    );
}
