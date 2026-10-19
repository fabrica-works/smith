//! The start's live ask crosses both real channel halves (protocol/channel.md, section 3).

use skein_channel::StreamMode;
use smith_channel::CEILINGS;
use smith_channel_world::{Observation, World, window_turn_fixture};
use smith_host_domain::channel::LiveAsk;

fn start(live: Option<LiveAsk>) -> Box<smith_protocol_channel::DecodedStart> {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    let charter = include_bytes!("../../../crates/smith-charter/golden/v2/record_charter_smallest.bin");
    world.send_start_asking(Box::from(charter.as_slice()), live);
    world.settle();
    world.take_agent_start().expect("the start reached the agent")
}

#[test]
fn a_start_with_an_ask_reaches_the_agent_with_its_kinds() {
    let ask = LiveAsk { text: true, reasoning: false, input: true };
    let live = start(Some(ask)).live.expect("the ask crossed");
    assert_eq!((live.text(), live.reasoning(), live.input()), (true, false, true));
}

#[test]
fn a_start_with_no_ask_reaches_the_agent_with_none() {
    assert_eq!(start(None).live, None);
}

fn asking(ask: LiveAsk) -> World {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start_asking(
        Box::from(include_bytes!("../../../crates/smith-charter/golden/v2/record_charter_smallest.bin").as_slice()),
        Some(ask),
    );
    world.settle();
    world
}

fn text_ask() -> LiveAsk {
    LiveAsk { text: true, reasoning: false, input: false }
}

#[test]
fn pieces_reach_the_host_in_order_before_their_turn() {
    let mut world = asking(text_ask());
    world.agent_admits();
    world.settle();
    assert!(world.agent_sends_live(1, 0, 0, b"abc"));
    world.settle();
    assert!(world.agent_sends_live(1, 0, 3, b"de"));
    world.agent_tells_turn(1, None, &window_turn_fixture());
    world.settle();
    let pieces: Vec<_> = world
        .observations()
        .iter()
        .filter(|seen| matches!(seen, Observation::HostLive { .. } | Observation::HostTurn { .. }))
        .collect();
    assert_eq!(pieces.len(), 3);
    assert_eq!(
        pieces.first().copied().expect("first piece"),
        &Observation::HostLive {
            turn: 1,
            retry: 0,
            kind: smith_channel::LiveKind::Text,
            offset: 0,
            bytes: b"abc".as_slice().into(),
            call: None
        }
    );
    assert_eq!(
        pieces.get(1).copied().expect("second piece"),
        &Observation::HostLive {
            turn: 1,
            retry: 0,
            kind: smith_channel::LiveKind::Text,
            offset: 3,
            bytes: b"de".as_slice().into(),
            call: None
        }
    );
    assert!(matches!(pieces.get(2).copied().expect("turn"), Observation::HostTurn { number: 1, .. }));
    assert_eq!(world.lost_live_bytes(), 0);
}

#[test]
fn a_piece_after_its_turns_record_is_dropped_and_counted() {
    let mut world = asking(text_ask());
    world.agent_admits();
    world.settle();
    world.agent_tells_turn(1, None, &window_turn_fixture());
    world.settle();
    assert!(!world.agent_sends_live(1, 0, 0, b"late"));
    assert_eq!(world.lost_live_bytes(), 4);
    assert_eq!(world.observations().iter().filter(|seen| matches!(seen, Observation::HostLive { .. })).count(), 0);
}

#[test]
fn pieces_before_admitted_or_after_the_answer_are_dropped_and_counted() {
    let mut world = asking(text_ask());
    assert!(!world.agent_sends_live(1, 0, 0, b"early"));
    assert_eq!(world.lost_live_bytes(), 5);
    world.agent_admits();
    world.settle();
    world.agent_parks();
    assert!(!world.agent_sends_live(1, 0, 0, b"late"));
    assert_eq!(world.lost_live_bytes(), 9);
    world.settle();
    assert_eq!(world.observations().iter().filter(|seen| matches!(seen, Observation::HostLive { .. })).count(), 0);
}

#[test]
fn a_piece_of_a_kind_not_asked_is_dropped_and_counted() {
    let mut world = asking(LiveAsk { text: false, reasoning: true, input: true });
    world.agent_admits();
    world.settle();
    assert!(!world.agent_sends_live(1, 0, 0, b"text"));
    assert_eq!(world.lost_live_bytes(), 4);
}

#[test]
fn a_full_queue_drops_a_piece_and_still_takes_the_turn() {
    let mut world = asking(text_ask());
    world.agent_admits();
    world.settle();
    let sent = (0..8).take_while(|offset| world.agent_sends_live(1, 0, offset * 3, b"abc")).count();
    assert!(sent > 0 && sent < 8);
    assert_eq!(world.lost_live_bytes(), 3);
    world.agent_tells_turn(1, None, &window_turn_fixture());
    world.settle();
    assert_eq!(
        world.observations().iter().filter(|seen| matches!(seen, Observation::HostTurn { number: 1, .. })).count(),
        1
    );
}
