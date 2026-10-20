//! Program routing at the root (protocol/programs.md 2, 5.7).
use super::*;

fn program() -> Harness {
    let mut h = Harness::new();
    h.domain.config.programs = Box::new([crate::Program { endpoint: Endpoint(1), name: bytes(b"claude_code") }]);
    h
}

#[test]
fn a_program_completion_skips_the_session_reservation() {
    let mut h = program();
    let (_, owner, _) = h.admit(7, charter());
    assert_eq!(session::preview_completion(&h.domain.session, owner, Usage::NONE), Err(session::End::PriceOverflow));
}

#[test]
fn a_program_completion_without_usage_overflows() {
    let mut h = program();
    let (_, owner, _) = h.admit(7, charter());
    let emitted = h.step(Event::Completed {
        owner,
        completion: Completion {
            reasoning_dropped: Box::default(),
            content: Box::default(),
            stop: Stop::EndTurn,
            usage: Usage::NONE,
        },
    });
    let [Request::Answer { answer: run::Answer::Failed { failure, .. }, .. }] = emitted.as_ref() else {
        panic!("overflow answer: {emitted:?}");
    };
    assert_eq!(
        *failure,
        run::Failure::Budget { exhausted: run::Exhausted::Overflow(run::Overflow::Spend), used: None, bound: None }
    );
}

#[test]
fn a_program_overrun_is_spent() {
    let mut h = program();
    let mut selected = charter();
    selected.llm.prices.unit = 1;
    let (_, owner, _) = h.admit(7, selected);
    let usage = Usage { input_tokens: Some(10_001), output_tokens: Some(1), ..Usage::ZERO };
    let emitted = h.step(Event::Completed {
        owner,
        completion: Completion {
            reasoning_dropped: Box::default(),
            content: Box::default(),
            stop: Stop::EndTurn,
            usage,
        },
    });
    assert!(emitted.is_empty());
    let emitted = h.next();
    let [Request::Answer { answer: run::Answer::Failed { spent, .. }, .. }] = emitted.as_ref() else {
        panic!("spent answer: {emitted:?}");
    };
    assert_eq!(spent.units, 10_002);
}

#[test]
fn a_program_main_offers_no_sub_agent() {
    let mut h = program();
    let (_, _, prompt) = h.admit(7, charter());
    assert!(!prompt.served.contains(&Served::SubAgent));
}

#[test]
fn a_direct_main_keeps_its_sub_agents() {
    let mut h = Harness::new();
    let (_, _, prompt) = h.admit(7, charter());
    assert!(prompt.served.contains(&Served::SubAgent));
}

#[test]
fn a_program_on_an_unknown_endpoint_is_invalid() {
    let mut h = program();
    h.domain.config.programs[0].endpoint = Endpoint(99);
    assert!(!h.domain.config.valid(&LIMITS));
}

#[test]
fn the_run_s_time_cancels_a_program_completion() {
    let mut h = program();
    let (_, owner, _) = h.admit(7, charter());
    h.env.now = Time::ZERO.saturating_add(BUDGET.time);
    assert!(h.fire().is_empty());
    assert_eq!(h.fire().as_ref(), &[Request::Cancel { owner }]);
    assert!(h.next().is_empty());
    let emitted = h.step(Event::Cancelled { owner });
    let [
        Request::Answer {
            answer: run::Answer::Failed { failure: run::Failure::Budget { exhausted: run::Exhausted::Time, .. }, .. },
            ..
        },
    ] = emitted.as_ref()
    else {
        panic!("time budget answer: {emitted:?}");
    };
}

#[test]
fn a_program_spec_names_its_backend() {
    let mut h = program();
    let (_, owner, _) = h.admit(7, charter());
    h.step(Event::Completed {
        owner,
        completion: Completion {
            reasoning_dropped: Box::default(),
            content: Box::new([Said::Text { text: bytes(b"answer"), replay: None }]),
            stop: Stop::EndTurn,
            usage: Usage::ZERO,
        },
    });
    let turn = h.turns.last().expect("program turn");
    let session::record::Kind::First(opener) = &turn.kind else { panic!("opening") };
    assert_eq!(opener.backend, session::record::Backend::Program { name: bytes(b"claude_code") });
}
