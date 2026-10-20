#![allow(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "ordinary Rust deterministic harness, programming-model.md 10.2"
)]
use super::*;
use crate::responses::tests::{at, intent, opened, prompt};
use crate::sinks::tests::{drain_ready, grant, limits};
use crate::sinks::{Delivery, Destination};
use smith_events::{Backend, FailureClass, Outcome};

#[test]
fn a_program_opening_names_its_backend() {
    let charter = crate::tests::priced_charter();
    for backend in [Backend::ClaudeCode, Backend::Direct] {
        let model = crate::opening_model(&charter.llm, b"endpoint", None, backend.clone());
        assert_eq!(model.backend, backend);
        assert_eq!(model.account, charter.llm.account);
    }
}

fn retry_book() -> crate::responses::Book {
    let mut book = crate::responses::Book::new(1);
    let owner = Token::new(11);
    let mut opening = opened(0, 0);
    opening.model.backend = Backend::ClaudeCode;
    book.opened(at(1), owner, &opening);
    intent(&mut book, owner, domain::session::Choice::Auto, 0);
    book.request(at(2), Time::ZERO, Token::new(99), &prompt(0));
    book.observation(Token::new(99));
    book
}

#[test]
fn a_retry_fails_its_attempt() {
    for (status, class) in [
        (401, FailureClass::Unauthorized),
        (403, FailureClass::Invalid),
        (429, FailureClass::RateLimited),
        (503, FailureClass::Unavailable),
        (529, FailureClass::Overloaded),
    ] {
        let mut book = retry_book();
        let records = book.retried(at(7), Time::ZERO, Token::new(99), Some(status), Some(125));
        let Event::ResponseCompleted(response) = &records.get(0).unwrap().event else { panic!("failed response") };
        assert_eq!(response.outcome, Outcome::Failed);
        assert_eq!(response.failure.as_ref().unwrap().class, class);
        assert_eq!(response.failure.as_ref().unwrap().status, Some(u64::from(status)));
        assert_eq!(response.retry_ms, Some(125));
        assert_eq!(response.request_bytes, None);
    }
}

#[test]
fn a_retry_starts_the_next_attempt() {
    let mut book = retry_book();
    for attempt in 1..=2 {
        let records = book.retried(at(7), Time::ZERO, Token::new(99), Some(503), None);
        let Event::ResponseStarted(response) = &records.get(1).unwrap().event else { panic!("next attempt") };
        assert_eq!(response.attempt, attempt);
        assert_eq!(response.response, attempt.checked_add(1).unwrap());
        assert_eq!(response.turn, 1);
        assert_eq!(response.window, 1);
        assert!(response.prompt.as_ref().unwrap().messages.is_empty());
        let delta = book.delta(at(8), Time::ZERO, Token::new(99), 0, b"next".as_slice().into());
        let Event::TextDelta(delta) = delta.event else { panic!("delta") };
        assert_eq!(delta.response, response.response);
    }
}

#[test]
fn a_program_run_withholds_sub_agents() {
    for program in [false, true] {
        for agents in [false, true] {
            let bounds = limits(1);
            let mut producer = Producer {
                sinks: Sinks::new(&bounds, Capture::Calls, bounds.output, crate::sinks::tests::DEADLINE),
                names: channel::Endpoints::new({
                    let mut names = List::with_capacity(1);
                    names
                        .push(channel::Endpoint { name: b"endpoint".as_slice().into(), number: 1, dialect: 1 })
                        .unwrap();
                    names
                }),
                programs: if program { Box::new([1]) } else { Box::default() },
                calls: crate::calls::Book::new(&smith_agent_world::LIMITS.session, Capture::Calls),
                messages: crate::messages::Book::disabled(),
                capture: Capture::Calls,
                origin: Some(Time::ZERO),
                start: None,
                admitted: None,
                admitted_at: None,
                activation: None,
                answer_at: None,
                overflow_notice: None,
                wind_down: false,
                unread: 0,
                check_at: None,
            };
            let destination = Token::new(400);
            producer.sinks.add(Destination::Trace, destination, Delivery::Complete).unwrap();
            grant(&mut producer.sinks, destination);
            let mut charter = crate::tests::priced_charter();
            charter.grants.agents = agents;
            producer.starting(7, 0, &charter);
            producer.fact(&domain::Fact::Run {
                fact: run::facts::Fact {
                    at: Time::ZERO,
                    kind: run::facts::FactKind::Admitted { run: Token::new(1), resumed: false },
                },
                opening: None,
            });
            let records = drain_ready(&mut producer.sinks, destination);
            let Event::RunStarted(start) = &records[0].event else { panic!("run start") };
            assert_eq!(
                start.tools.withheld.as_slice(),
                if program && agents { &[smith_events::Family::SubAgents][..] } else { &[] }
            );
            assert_eq!(start.main.backend, if program { Backend::ClaudeCode } else { Backend::Direct });
        }
    }
}
