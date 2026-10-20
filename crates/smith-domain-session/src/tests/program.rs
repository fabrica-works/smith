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

fn carried(from: Backend, to: Backend, account: Option<u32>) {
    let mut old = Harness::new(&LIMITS);
    let (owner, _) = old.open_with(1, Spec { backend: from, account, ..spec() });
    old.step(Event::Completed { owner, completion: completion(Box::new([text(b"old answer")]), Stop::EndTurn) });
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
    assert!(prompt.messages.iter().any(|message| message.content.contains(&text(b"old answer"))));
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
fn a_program_window_is_carried_over_before_resumption_is_enabled() {
    carried(program(), program(), spec().account);
}
