//! A child answers its final turn through actual session boundaries.
//! Contract: domain/session.md, section 7.

use skein_lib::{Duration, Token};
use smith_domain_session::{self as session, llm};
use smith_session_world::recorded::{World, opening};

#[test]
fn a_childs_final_turn_is_answered_with_stop_budget() {
    for stop in [llm::Stop::ToolUse, llm::Stop::EndTurn, llm::Stop::MaxTokens, llm::Stop::Refusal] {
        let mut world = World::new(72, 256);
        let mut start = opening(None, 1000);
        start.spec.authority.grants.inspect = true;
        start.spec.budget.turns = 1;
        start.spec.final_turn = Some(session::FinalTurn {
            note: b"Answer with what you have.".as_slice().into(),
            allowance: Duration::from_secs(1),
        });
        world.open(start);
        assert_eq!(world.prompts[0].choice, llm::ToolChoice::None);
        let definitions = world.prompts[0].delegated.clone();
        assert!(!definitions.is_empty(), "definitions stay offered");
        world.complete(
            Box::new([
                llm::Block::Text { text: b"Found it.".as_slice().into(), replay: None },
                llm::Block::ToolCall {
                    id: b"outside".as_slice().into(),
                    name: b"opaque_host".as_slice().into(),
                    input: b"{}".as_slice().into(),
                    replay: None,
                    call: llm::Decoded::Delegated {
                        source: session::ToolSource::Host,
                        effect: smith_domain_tools::Effect::Read,
                        descriptor: Token::new(19),
                        ticket: Token::new(991),
                    },
                },
            ]),
            stop,
            llm::Usage::ZERO,
        );
        assert!(world.operations.is_empty() && world.delegated.is_empty());
        world.resume();
        assert_eq!(world.yielded.as_slice(), &[(session::Yield::Budget, Box::<[u8]>::from(b"Found it.".as_slice()))]);
        assert_eq!(world.prompts.len(), 1, "the last answer requests no further provider turn");
        let result = &world.turns[0].messages.last().expect("settled concrete result").content[0];
        assert_eq!(
            result,
            &llm::Block::ToolResult {
                id: b"outside".as_slice().into(),
                result: llm::Returned::NotRun { why: llm::NotRun::OutsideChoice { choice: None } },
            }
        );
        world.close();
    }
}
