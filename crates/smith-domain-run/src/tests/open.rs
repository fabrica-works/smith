//! Open main lifecycle at the existing run entrance (domain/run.md, sections 3.1, 6 and 9.1).
#![expect(
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

fn reply(main: Token, stop: Stop, text: &[u8]) -> Event {
    Event::Yielded { conversation: main, stop, text: bytes(text) }
}

fn message(run: Token, name: Token, text: &[u8]) -> Event {
    Event::Message { run, name, label: bytes(b"person"), text: bytes(text) }
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
