//! Actual narrowed requests and concrete not-run replay through the session domain.
//! Contract: domain/session.md, section 5; protocol/llm.md, section 5.

use skein_lib::Token;
use smith_domain_session::{self as session, llm};
use smith_session_world::recorded::{World, opening};

#[test]
fn a_wind_down_offering_only_some_tools_answers_a_call_outside_them_as_not_run() {
    let mut world = World::new(71, 256);
    let mut start = opening(None, 1000);
    start.spec.authority.grants.inspect = true;
    world.open(start);
    let definitions = world.prompts[0].delegated.clone();
    let note = b"Finish now with what you have.";
    world.step(session::Event::Narrowed {
        owner: world.completing.expect("unsent completion"),
        note: note.as_slice().into(),
        choice: llm::ToolChoice::Only(Box::new([Token::new(19)])),
        names: Some(Box::new([b"finish".as_slice().into()])),
    });
    world.complete(
        Box::new([llm::Block::ToolCall {
            id: b"outside".as_slice().into(),
            name: b"list".as_slice().into(),
            input: b"{}".as_slice().into(),
            replay: None,
            call: llm::Decoded::Owned {
                call: smith_domain_tools::Call::List {
                    path: smith_domain_tools::Path { absolute: true, parts: Box::default() },
                    depth: smith_domain_tools::Depth::Whole,
                    glob: None,
                },
            },
        }]),
        llm::Stop::ToolUse,
        llm::Usage::ZERO,
    );
    assert!(
        world.operations.is_empty() && world.delegated.is_empty(),
        "outside choice starts neither IO nor delegated work"
    );
    world.resume();
    let result = &world.turns[0].messages.last().expect("concrete result").content[0];
    assert_eq!(
        result,
        &llm::Block::ToolResult {
            id: b"outside".as_slice().into(),
            result: llm::Returned::NotRun {
                why: llm::NotRun::OutsideChoice { choice: Some(Box::new([b"finish".as_slice().into()])) }
            }
        }
    );
    assert_eq!(world.prompts.last().expect("next request").delegated, definitions);
    let latest = world.prompts.last().expect("next request");
    assert_eq!(latest.choice, llm::ToolChoice::Only(Box::new([Token::new(19)])));
    let notes = latest
        .messages
        .iter()
        .flat_map(|message| &message.content)
        .filter(|block| matches!(block, llm::Block::Text { text, .. } if text.as_ref() == note))
        .count();
    assert_eq!(notes, 1, "the narrowing note remains exactly once");
    world.complete(
        Box::new([llm::Block::ToolCall {
            id: b"chosen".as_slice().into(),
            name: b"finish".as_slice().into(),
            input: b"{}".as_slice().into(),
            replay: None,
            call: llm::Decoded::Delegated {
                descriptor: Token::new(19),
                ticket: Token::new(991),
                source: session::ToolSource::Run,
                effect: smith_domain_tools::Effect::Write,
            },
        }]),
        llm::Stop::ToolUse,
        llm::Usage::ZERO,
    );
    assert_eq!(world.delegated.len(), 1, "descriptor membership does not compare the dynamic call ticket");
    let owner = world.delegated[0];
    world.step(session::Event::Answered { owner, text: b"accepted".as_slice().into(), error: false, spent: 0 });
    world.close();
}
