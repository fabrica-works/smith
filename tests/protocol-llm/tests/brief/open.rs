//! Open uses the existing root/native world and byte peers (domain/run.md, sections 6 and 13).
use super::*;

const CUE: &str = "@open-main";
const HELLO: &[u8] = b"Hello from an open turn.";
const AGAIN: &[u8] = b"A second reply.";
const NEXT: Token = Token::new(0);

fn world(dialect: usize, turns: Box<[Turn]>, reserve: run::charter::Reserve, budget_turns: u32) -> World {
    let bounds = Limits {
        client: skein_llm_world::limits(),
        tool_bytes: 32768,
        history_bytes: u64::from(skein_llm_world::limits().request),
        rendered_result: skein_llm_world::limits().strings,
        stated: adapter::stated(),
    };
    let mut settings = Settings { job: Job::Waiting, network: Span::millis(1, 1), ..Settings::calm(306) };
    settings.limits.session.completion_bytes =
        adapter::completion_worst_case(&bounds.client, settings.limits.decoded_call_bytes).expect("native receiving");
    settings.limits.session.completion_blocks = bounds.client.output_items;
    settings.limits.session.failure_bytes = bounds.client.detail_bytes;
    let mut selected = charter(&settings, CUE);
    selected.mode = run::Mode::Open;
    selected.grants.agents = false;
    selected.reserve = reserve;
    selected.budget.turns = budget_turns;
    World::with_workspace_wire_charter(
        settings,
        None,
        None,
        Checkout::new(),
        (wire::configurations().into_iter().nth(dialect).expect("native dialect"), bounds),
        Box::new([Script { cue: CUE.as_bytes().into(), turns }]),
        selected,
    )
}

fn no_reserve() -> run::charter::Reserve {
    run::charter::Reserve { turns: 0, time: Duration::ZERO }
}

fn tool_blocks(world: &World) -> usize {
    world
        .turns()
        .iter()
        .flat_map(|turn| &turn.messages)
        .flat_map(|message| &message.content)
        .filter(|block| matches!(block, llm::Block::ToolCall { .. } | llm::Block::ToolResult { .. }))
        .count()
}

#[test]
fn native_open_reply_wakes_once_without_artificial_transcript() {
    for dialect in 0..2 {
        let mut world = world(dialect, Box::new([says(HELLO, 2), says(AGAIN, 3)]), no_reserve(), 4);
        world.message_at(
            skein_lib::Time::ZERO.saturating_add(Duration::from_secs(2)),
            NEXT,
            b"Next words".as_slice().into(),
        );
        world.run(100_000);
        assert!(matches!(world.answer(), run::Answer::Parked { turns: 2, .. }));
        assert_eq!(world.prompts().len(), 2, "each genuine reply has one completion");
        assert_eq!(world.turns().len(), 2, "Open main must keep its actual turns");
        assert_eq!(tool_blocks(&world), 0, "text replies create no tool call or result");
        let prompt = &world.prompts()[1];
        assert_eq!(prompt.messages.last().expect("waking message").role, Role::User);
        assert_eq!(prompt.messages.last().expect("waking message").parts.as_ref(), [text(b"host: Next words")]);
        assert_eq!(
            prompt.messages.iter().flat_map(|message| &message.parts).filter(|part| **part == text(HELLO)).count(),
            1
        );
        assert_eq!(world.waiting().len(), 2);
        assert_eq!(world.waiting()[1].1, Some(NEXT));
        assert!(world.answered_at() >= world.waiting()[1].0.saturating_add(Duration::from_secs(30)));
    }
}

#[test]
fn native_open_finish_is_unoffered_and_decodes_as_unknown() {
    for dialect in 0..2 {
        let mut world =
            world(dialect, Box::new([calls(vec![call(b"finish", FINISH_INPUT)], 2), says(HELLO, 3)]), no_reserve(), 4);
        world.run(100_000);
        assert!(matches!(world.answer(), run::Answer::Parked { turns: 2, .. }));
        assert_eq!(world.prompts().len(), 2);
        for prompt in world.prompts() {
            assert!(
                !prompt.tools.iter().any(|tool| tool.name.as_ref() == b"finish" || tool.name.as_ref() == b"deliver")
            );
        }
        assert!(
            world.turns().iter().flat_map(|turn| &turn.messages).flat_map(|message| &message.content).any(
                |block| matches!(
                    block,
                    llm::Block::ToolResult {
                        result: llm::Returned::Invalid { problem: llm::Problem::UnknownTool },
                        ..
                    }
                )
            ),
            "actual public decoder feedback"
        );
        assert_eq!(world.waiting().len(), 1);
    }
}

#[test]
fn native_open_final_text_parks_after_the_last_reserved_turn() {
    for dialect in 0..2 {
        let truncated = Turn {
            lines: Box::new([Line::Text { text: b"partial".as_slice().into() }]),
            finish: Finish::Length,
            tokens: 2,
        };
        let reserve = run::charter::Reserve { turns: 1, time: Duration::from_secs(60) };
        // One actual truncated ordinary completion leaves the one final turn.
        let mut world = world(dialect, Box::new([truncated, says(HELLO, 3)]), reserve, 2);
        world.run(100_000);
        assert!(matches!(world.answer(), run::Answer::Parked { turns: 2, .. }));
        let prompt = world.prompts().last().expect("final completion");
        assert_eq!(prompt.choice, skein_fake_llm_domain::api::ToolChoice::None);
        assert!(
            prompt
                .messages
                .iter()
                .flat_map(|message| &message.parts)
                .any(|part| matches!(part, Part::Text { text } if contains(text, b"Reply now with what you have.")))
        );
        assert_eq!(tool_blocks(&world), 0);
        assert_eq!(world.waiting().len(), 1);
        assert!(
            world.answered_at().saturating_since(world.waiting()[0].0) < Duration::from_secs(30),
            "wind-down parks without another idle interval"
        );
    }
}
