use super::*;
use crate::responses::tests::{at, intent, opened, prompt};

#[test]
fn prompt_timestamps_start_program_duration_without_moving_direct() {
    for (program, capture) in [
        (false, Capture::Everything),
        (true, Capture::Everything),
        (false, Capture::Calls),
        (true, Capture::Calls),
        (true, Capture::None),
    ] {
        let bounds = limits(1);
        let mut sinks = Sinks::new(&bounds, capture, bounds.output, DEADLINE);
        let destination = Token::new(400);
        sinks.add(Destination::Trace, destination, Delivery::Complete).unwrap();
        grant(&mut sinks, destination);
        let mut opening = opened(0, 0);
        if program {
            opening.model.backend = smith_events::Backend::ClaudeCode;
        }
        let conversation = Token::new(11);
        let owner = Token::new(99);
        sinks.responses.opened(at(1), conversation, &opening);
        intent(&mut sinks.responses, conversation, smith_domain::session::Choice::Auto, 0);
        let observation = sinks.request(at(2), Time::ZERO, owner, &prompt(0));
        if !program && capture == Capture::Calls {
            assert!(observation.is_none());
            let records = drain_ready(&mut sinks, destination);
            assert_eq!(records[0].t_ms, 2);
            continue;
        }
        assert!(observation.is_some());
        assert!(drain_ready(&mut sinks, destination).is_empty());
        let sent = if program { at(7) } else { at(2) };
        sinks.measure(
            Time::ZERO,
            smith_protocol_llm::Measure::Prompt {
                at: sent,
                owner,
                prompt: smith_events::Prompt { system: None, tools: None, messages: List::with_capacity(0) },
            },
        );
        let records = drain_ready(&mut sinks, destination);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].t_ms, if program { 7 } else { 2 });
        let Event::ResponseStarted(started) = &records[0].event else { panic!("response start") };
        assert_eq!(started.prompt.is_some(), capture == Capture::Everything);
        assert_eq!(sinks.responses.observed(owner), capture == Capture::Everything);
        sinks.measure(
            Time::ZERO,
            smith_protocol_llm::Measure::Completion {
                at: at(9),
                owner,
                completion: smith_events::Completion { blocks: List::with_capacity(0) },
            },
        );
        let records = sinks.responses.retried(at(10), Time::ZERO, owner, Some(503), None);
        let Event::ResponseCompleted(response) = &records.get(0).unwrap().event else { panic!("response") };
        assert_eq!(response.duration_ms, if program { 3 } else { 8 });
        assert_eq!(response.completion.is_some(), capture == Capture::Everything);
    }
}
