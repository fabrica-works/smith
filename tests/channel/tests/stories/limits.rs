//! Derived transcript and channel admission across both halves.

use super::{Observation, StreamMode, World, window_turn_fixture};

fn completion_turn(calls: u32) -> smith_domain_session::record::Turn {
    use smith_domain_session::llm;
    let mut turn = window_turn_fixture();
    let call_blocks = (0..calls)
        .map(|number| llm::Block::ToolCall {
            id: format!("call-{number}").into_bytes().into_boxed_slice(),
            name: b"host".as_slice().into(),
            input: b"{}".as_slice().into(),
            call: llm::Decoded::Historical,
            replay: None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let results = (0..calls)
        .map(|number| llm::Block::ToolResult {
            id: format!("call-{number}").into_bytes().into_boxed_slice(),
            result: llm::Returned::Text { text: b"done".as_slice().into(), error: false, replay: None },
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    turn.messages = Box::new([
        turn.messages[0].clone(),
        llm::Message { role: llm::Role::Assistant, content: call_blocks },
        llm::Message { role: llm::Role::User, content: results },
    ]);
    turn
}

#[test]
fn a_completion_of_every_call_it_may_make_is_told_and_acknowledged() {
    let limits = smith_agent_process_world::limits();
    let turn = completion_turn(limits.domain.run.calls);
    let mut world = World::with_agent_limits(limits.channel, StreamMode::Two);
    world.settle();
    world.send_start(
        include_bytes!("../../../../crates/smith-charter/golden/v4/record_charter_smallest.bin").as_slice().into(),
    );
    world.settle();
    world.agent_admits();
    world.settle();
    world.agent_tells_turn(1, None, &turn);
    world.settle();
    assert!(world.observations().iter().any(|seen| matches!(seen, Observation::HostTurn { number: 1, .. })));
    world.host_acknowledges(1);
    world.settle();
    assert!(world.observations().contains(&Observation::AgentAcknowledged { turn: 1 }));
}

#[test]
fn a_window_that_holds_one_turn_of_the_largest_size() {
    use smith_domain_session::llm;
    let limits = smith_agent_process_world::limits();
    let mut turn = window_turn_fixture();
    turn.messages[0].content[0] = llm::Block::Text {
        text: vec![b'a'; usize::try_from(limits.channel.transcript.text_text).expect("text bound fits usize")]
            .into_boxed_slice(),
        replay: None,
    };
    let mut world = World::with_agent_limits(limits.channel, StreamMode::Two);
    world.settle();
    world.send_start(
        include_bytes!("../../../../crates/smith-charter/golden/v4/record_charter_smallest.bin").as_slice().into(),
    );
    world.settle();
    world.agent_admits();
    world.settle();
    world.agent_tells_turn(1, None, &turn);
    world.settle();
    assert!(world.observations().iter().any(|seen| matches!(seen, Observation::HostTurn { number: 1, .. })));
    world.host_acknowledges(1);
    world.settle();
    assert!(world.observations().contains(&Observation::AgentAcknowledged { turn: 1 }));
}

#[test]
fn a_window_of_more_than_sixty_four_turns_resumes() {
    use smith_domain_session::record;
    let limits = smith_agent_process_world::limits();
    assert!(limits.channel.resume_turns > 64);
    let mut turn = window_turn_fixture();
    let mut bodies = Vec::new();
    for sequence in 1..=65 {
        turn.sequence = sequence;
        if sequence > 1 {
            turn.kind = record::Kind::Ordinary;
        }
        bodies.push(
            smith_protocol_channel::encode_turn(
                &turn,
                &limits.channel.transcript,
                &smith_channel_world::test_endpoints(),
            )
            .expect("bounded saved turn"),
        );
    }
    let mut world = World::with_agent_limits(limits.channel, StreamMode::Two);
    world.settle();
    world.send_start_with_turns(
        include_bytes!("../../../../crates/smith-charter/golden/v4/record_charter_smallest.bin").as_slice().into(),
        Some(bodies.into_boxed_slice()),
    );
    world.settle();
    assert!(
        world.observations().contains(&Observation::AgentHistory { turns: 65, place: 1 }),
        "{:?}",
        world.observations()
    );
}
