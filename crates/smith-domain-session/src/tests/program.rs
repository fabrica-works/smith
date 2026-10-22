//! Recorded transport and portable restoration (domain/session.md 8.2).
#![expect(clippy::disallowed_methods, reason = "ordinary deterministic harness, programming-model.md 10.2")]
use super::*;
use crate::record::{Backend, Kind, Transcript};

fn program() -> Backend {
    Backend::Program { name: bytes(b"claude_code") }
}

#[test]
fn a_program_turn_records_its_backend() {
    let mut h = Harness::new(&LIMITS);
    let (owner, _) = h.open_with(1, Spec { backend: program(), ..spec() });
    h.step(Event::Completed { owner, completion: completion(Box::new([text(b"answer")]), Stop::EndTurn) });
    let turn = h.turn_records.get(0).expect("recorded turn");
    let Kind::First(opener) = &turn.kind else { panic!("first turn") };
    assert_eq!(opener.backend, program());
    assert_eq!(opener.account, spec().account);
    assert_eq!(size_of::<crate::record::Turn>(), 176);
    assert_eq!(size_of::<Kind>(), 16);
}

fn provider_answer() -> Box<[Block]> {
    Box::new([
        Block::Opaque { bytes: bytes(b"private reasoning") },
        Block::Text {
            text: bytes(b"old answer"),
            replay: Some(crate::llm::Replay { bytes: bytes(b"provider envelope") }),
        },
    ])
}

fn carried(from: Backend, to: Backend, account: Option<u32>) {
    let mut old = Harness::new(&LIMITS);
    let (owner, _) = old.open_with(1, Spec { backend: from, account, ..spec() });
    old.step(Event::Completed { owner, completion: completion(provider_answer(), Stop::EndTurn) });
    let mut h = Harness::new(&LIMITS);
    let mut start = opening(Spec { backend: to.clone(), ..spec() });
    start.transcript = Some(Transcript {
        version: crate::record::VERSION,
        endpoint: Some(Endpoint(0)),
        dialect: 1,
        turns: old.turn_records.to_boxed(),
    });
    step(&mut h.domain, &h.env, Event::Open { opener: Token::new(2), opening: start }, &mut h.out);
    let Some(Request::Opened { .. }) = h.next() else { panic!("restored session opens") };
    let (owner, prompt) = calling(h.one());
    let crate::llm::Purpose::Compaction { .. } = prompt.purpose else { panic!("compaction") };
    let assistant = prompt.messages.iter().find(|message| message.role == Role::Assistant).expect("old answer");
    assert_eq!(assistant.content.as_ref(), &[text(b"old answer")], "foreign history is portable");
    let (_, next) = calling(
        h.step(Event::Completed { owner, completion: completion(Box::new([text(b"summary")]), Stop::EndTurn) }),
    );
    let crate::llm::Purpose::Ordinary { .. } = next.purpose else { panic!("ordinary window") };
    assert!(next.messages[0].content.contains(&text(b"summary")));
    let turn = h.turn_records.last().expect("new window recorded");
    let Kind::Window(opener) = &turn.kind else { panic!("window opening") };
    assert_eq!(opener.backend, to);
    assert_eq!(opener.account, spec().account);
}

#[test]
fn a_window_from_another_backend_is_carried_over() {
    carried(Backend::Direct, program(), spec().account);
}

#[test]
fn a_window_from_another_account_is_carried_over() {
    carried(Backend::Direct, Backend::Direct, Some(7));
}

#[test]
fn a_program_window_is_carried_over_to_the_direct_backend() {
    carried(program(), Backend::Direct, spec().account);
}

#[test]
fn a_window_on_the_same_program_is_kept() {
    let mut old = Harness::new(&LIMITS);
    let (owner, _) = old.open_with(1, Spec { backend: program(), ..spec() });
    old.step(Event::Completed { owner, completion: completion(Box::new([text(b"old answer")]), Stop::EndTurn) });
    let mut h = Harness::new(&LIMITS);
    let mut start = opening(Spec { backend: program(), ..spec() });
    start.transcript = Some(Transcript {
        version: crate::record::VERSION,
        endpoint: Some(Endpoint(0)),
        dialect: 1,
        turns: old.turn_records.to_boxed(),
    });
    step(&mut h.domain, &h.env, Event::Open { opener: Token::new(2), opening: start }, &mut h.out);
    let Some(Request::Opened { .. }) = h.next() else { panic!("opened") };
    let (_, prompt) = calling(h.one());
    let crate::llm::Purpose::Ordinary { .. } = prompt.purpose else { panic!("kept ordinary window") };
}

fn restored_program(fresh: bool) -> (Harness, Token) {
    let mut old = Harness::new(&LIMITS);
    let (owner, _) = old.open_with(1, Spec { backend: program(), ..spec() });
    old.step(Event::Completed { owner, completion: completion(Box::new([text(b"old answer")]), Stop::EndTurn) });
    if fresh {
        let turn = old.turn_records.get_mut(0).expect("first turn");
        let Kind::First(opener) = core::mem::replace(&mut turn.kind, Kind::Ordinary) else { panic!("first") };
        turn.kind = Kind::Window(opener);
        turn.messages = Box::new([Message {
            role: Role::User,
            content: Box::new([text(b"old task"), text(b"preamble"), text(b"old summary")]),
        }]);
    }
    if !fresh {
        old.turn_records.get_mut(0).expect("first turn").messages.last_mut().expect("assistant").content =
            provider_answer();
    }

    let mut h = Harness::new(&LIMITS);
    let mut start = opening(Spec { backend: program(), ..spec() });
    start.transcript = Some(Transcript {
        version: crate::record::VERSION,
        endpoint: Some(Endpoint(0)),
        dialect: 1,
        turns: old.turn_records.to_boxed(),
    });
    step(&mut h.domain, &h.env, Event::Open { opener: Token::new(2), opening: start }, &mut h.out);
    let Some(Request::Opened { .. }) = h.next() else { panic!("opened") };
    let (owner, _) = calling(h.one());
    (h, owner)
}

#[test]
fn a_gone_session_carries_the_window_over() {
    let (mut h, owner) = restored_program(false);
    let (owner, prompt) = calling(h.step(Event::Failed {
        retry_ok: true,
        owner,
        failure: Failure::SessionGone,
        evidence: crate::llm::Evidence::Unsent,
        detail: bytes(b"missing"),
    }));
    let crate::llm::Purpose::Compaction { .. } = prompt.purpose else { panic!("carried over") };
    let assistant = prompt.messages.iter().find(|message| message.role == Role::Assistant).expect("old answer");
    assert_eq!(assistant.content.as_ref(), &[text(b"old answer")], "opaque content and replay are removed");
    let (_, next) = calling(
        h.step(Event::Completed { owner, completion: completion(Box::new([text(b"summary")]), Stop::EndTurn) }),
    );
    let crate::llm::Purpose::Ordinary { .. } = next.purpose else { panic!("new window") };
    assert!(next.messages[0].content.contains(&text(b"summary")));
}

#[test]
fn a_second_gone_session_ends_the_session() {
    let (mut h, owner) = restored_program(false);
    calling(h.step(Event::Failed {
        retry_ok: true,
        owner,
        failure: Failure::SessionGone,
        evidence: crate::llm::Evidence::Unsent,
        detail: bytes(b"missing"),
    }));
    let ended = h.step(Event::Failed {
        retry_ok: true,
        owner,
        failure: Failure::SessionGone,
        evidence: crate::llm::Evidence::Unsent,
        detail: bytes(b"missing again"),
    });
    let Some(Request::Ended { end: End::Failed { failure: Failure::SessionGone, .. }, .. }) = ended else {
        panic!("second missing ends: {ended:?}")
    };
}

#[test]
fn a_gone_session_carries_a_fresh_window_over() {
    let (mut h, owner) = restored_program(true);
    let (owner, prompt) = calling(h.step(Event::Failed {
        retry_ok: true,
        owner,
        failure: Failure::SessionGone,
        evidence: crate::llm::Evidence::Unsent,
        detail: bytes(b"missing"),
    }));
    let crate::llm::Purpose::Compaction { .. } = prompt.purpose else { panic!("fresh window carries over") };
    assert!(prompt.messages[0].content.contains(&text(b"old summary")));
    let (_, next) = calling(
        h.step(Event::Completed { owner, completion: completion(Box::new([text(b"new summary")]), Stop::EndTurn) }),
    );
    assert!(next.messages[0].content.contains(&text(b"new summary")));
    assert!(next.messages[0].content.contains(&text(b"fix the bug")), "waking prompt is held aside");
}

#[test]
fn a_window_from_another_program_is_carried_over() {
    carried(program(), Backend::Program { name: bytes(b"other_program") }, spec().account);
}

#[test]
fn a_program_window_from_another_account_is_carried_over() {
    carried(program(), program(), Some(7));
}
