//! Independent arithmetic properties for declared model windows.

use alloc::boxed::Box;
use skein_lib::{Duration, Rng};
use crate::profile::{Check, Configuration, Endpoint, Fraction, Layer, LimitName, Model, OversizedReasoning, Refusal, standard};
use super::room;

fn declarations(window: u32, output: u32) -> Configuration {
    Configuration {
        environment_bytes: 0,
        endpoints: Box::new([Endpoint {
            number: 7, connect: Duration::from_secs(1), handshake: Duration::from_secs(1),
            models: Box::new([Model {
                name: Box::from(b"model".as_slice()), window, output, reasoning_item: 64,
                oversized_reasoning: OversizedReasoning::Fail,
                head: Duration::from_secs(1), idle: Duration::from_secs(1),
            }]),
        }]),
    }
}

#[test]
fn the_smallest_declared_model_bounds_every_host_and_offer() {
    let mut configuration = declarations(100_000, 1000);
    let second = declarations(20_000, 1000).endpoints.into_vec().pop().expect("one endpoint");
    configuration.endpoints = Box::new([configuration.endpoints.into_vec().pop().expect("one endpoint"), second]);
    let parts = room(&standard(), &configuration, 2000, 0).expect("both models hold a fresh window");
    assert_eq!(parts.room, 14_000);
    assert_eq!(parts.own, 2000);
    assert_eq!(parts.host, 6000);
    assert_eq!(parts.messages, 6000);
    assert_eq!(parts.offer, 3000);
    assert_eq!(parts.message, 3000);
}

#[test]
fn smith_filling_the_room_names_the_model_and_actual_threshold() {
    let refusal = room(&standard(), &declarations(10_000, 1000), 6000, 0).expect_err("own fills room");
    assert_eq!(refusal.check(), Some(Check::FreshWindow));
    assert_eq!(crate::profile::words(&refusal).as_ref(),
        b"fresh-window: session.fresh_window[endpoint=7,model=0]=8000 tokens; declared.window_at_threshold[endpoint=7,model=0]=8000 tokens");
}

#[test]
fn compaction_holds_the_summary_instruction_and_one_marker_per_result() {
    let mut profile = standard();
    profile.policy.compaction_threshold = Fraction { numerator: 9, denominator: 10 };
    profile.declared.calls_per_response = 4;
    let refusal = room(&profile, &declarations(10_000, 1000), 100, 100).expect_err("no compaction marker room");
    assert_eq!(refusal.check(), Some(Check::CompactionHeadroom));
    assert_eq!(crate::profile::words(&refusal).as_ref(),
        b"compaction-headroom: session.compaction_headroom[endpoint=7,model=0]=1248 tokens; declared.window_above_threshold[endpoint=7,model=0]=1000 tokens");
}

#[test]
fn overflow_and_missing_declarations_are_typed_without_a_neutral_window() {
    assert_eq!(room(&standard(), &declarations(10_000, 1), u64::MAX, 0).expect_err("own sum overflows"),
        Refusal::Derived { limit: LimitName::deployment(Layer::Service, "fresh_window") });
    let empty = Configuration { environment_bytes: 0, endpoints: Box::new([]) };
    assert_eq!(room(&standard(), &empty, 0, 0).expect_err("model required"),
        Refusal::Field { limit: LimitName::deployment(Layer::Declared, "endpoints") });
}

#[test]
fn random_declared_rooms_refuse_by_name_or_hold_a_largest_fresh_window() {
    let mut random = Rng::new(46_006);
    for _ in 0..512 {
        let window = u32::try_from(random.next_u64() % 100_000 + 1).expect("bounded window");
        let output = u32::try_from(random.next_u64() % 10_000 + 1).expect("bounded output");
        let own = random.next_u64() % 10_000;
        let summary = random.next_u64() % own.checked_add(1).expect("bounded own");
        let mut profile = standard();
        let denominator = u32::try_from(random.next_u64() % 10 + 2).expect("small fraction");
        let numerator = u32::try_from(random.next_u64() % u64::from(denominator - 1) + 1).expect("small fraction");
        profile.policy.compaction_threshold = Fraction { numerator, denominator };
        profile.declared.tool_payload = u32::try_from(random.next_u64() % 65_536 + 1).expect("bounded payload");
        match room(&profile, &declarations(window, output), own, summary) {
            Ok(parts) => {
                let within = u64::from(window) * u64::from(numerator) / u64::from(denominator);
                let fresh = own + parts.host + parts.offer * 2 + u64::from(output) * 2;
                assert!(fresh <= within, "largest task, summary and held-back offer fit");
                assert!(parts.message <= profile.declared.tool_payload);
                assert!(u64::from(parts.message) <= parts.offer);
                assert!(parts.host + parts.messages + own <= parts.room);
                assert!(summary + u64::from(output) + u64::from(profile.declared.calls_per_response) * u64::from(smith_protocol_llm::CUT_MARKER_BYTES) <= u64::from(window) - within);
            }
            Err(refusal) => assert!(matches!(refusal.check(), Some(Check::FreshWindow | Check::CompactionHeadroom))),
        }
    }
}
