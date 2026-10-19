//! The start's live ask crosses both real channel halves (protocol/channel.md, section 3).

use skein_channel::StreamMode;
use smith_channel::CEILINGS;
use smith_channel_world::World;
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
