//! Random startup declarations preserve complete memory and named refusals.

#![cfg(test)]
#![allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::missing_assert_message,
    reason = "programming-model.md section 10.2: deterministic test fixtures use ordinary Rust"
)]

use smith_agent_service as service;
use smith_agent_service::sinks::Capture;
use smith_agent_shell::config;

#[path = "support/product.rs"]
mod product;

use product::product_document;

fn trust_path() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/protocol-llm/fixtures/root.der")
}

fn parse_value(document: &serde_json::Value) -> Result<config::Configuration, String> {
    config::parse(&serde_json::to_vec(document).expect("drawn document"))
}

#[test]
fn drawn_declarations_are_named_or_fit_complete_process_memory() {
    use service::profile::{Check, validate};
    use skein_io::kernel::{Complete, Fd, Submit};
    use skein_lib::{Queue, Rng};

    let mut base = product_document();
    base["endpoints"][0]["trust_der"] = trust_path().into();
    let baseline = config::from_value(base).expect("bounded product startup").declarations;
    assert_eq!(baseline.environment_bytes, 835);
    let mut rng = Rng::new(4_170_005);
    let (mut admitted, mut relationship_refusals, mut memory_refusals) = (0, 0, 0);
    for sample in 0..96 {
        let mut document = product_document();
        // Native endpoint construction uses the bounded DER loader and test root.
        document["endpoints"][0]["trust_der"] = trust_path().into();
        let payload = rng.next_u64() % 40_960 + 1;
        let declared = &mut document["profile"]["declared"];
        declared["conversations"] = (rng.next_u64() % 6 + 1).into();
        declared["tool_payload"] = payload.into();
        declared["file_bytes"] = (rng.next_u64() % 1_048_576 + payload).into();
        declared["calls_per_response"] = (rng.next_u64() % 16 + 1).into();
        declared["read_window"] =
            if sample % 6 == 0 { (payload + 1).into() } else { (rng.next_u64() % payload + 1).into() };
        // Draw both the declared memory ceiling and the quantities which
        // determine the owned bytes. Some otherwise valid draws must fail.
        declared["memory"] = if sample % 6 == 1 { 1_u64.into() } else { 2_147_483_648_u64.into() };
        let model = &mut document["endpoints"][0]["models"][0];
        let window = rng.next_u64() % 272_000 + 272_000;
        let output = rng.next_u64() % 32_000 + 32_000;
        model["window"] = window.into();
        model["output"] = output.into();
        let profile = config::profile_from_value(document["profile"].clone()).expect("positive drawn declarations");
        let mut declarations = baseline.clone();
        declarations.endpoints[0].models[0].window = u32::try_from(window).unwrap();
        declarations.endpoints[0].models[0].output = u32::try_from(output).unwrap();
        let mut limits = match service::profile::derive(&profile, &declarations) {
            Ok(limits) => limits,
            Err(refusal) => {
                let check = refusal.check().expect("draw names a startup relationship");
                let words = String::from_utf8(service::profile::words(&refusal).into_vec()).unwrap();
                assert!(words.starts_with(check.name()), "{words}");
                assert_eq!(parse_value(&document).err().as_deref(), Some(words.as_str()));
                relationship_refusals += 1;
                continue;
            }
        };
        validate::declarations(&profile, &declarations).expect("all drawn declaration comparisons");
        validate::window_first(&profile, &declarations, &limits.domain.session).expect("whole window and batch");
        validate::machine(&limits.machine, &limits.domain).expect("machine holds tools");
        assert!(limits.file_bytes >= limits.domain.session.tools.file_bytes);
        let effort = smith_charter::CEILINGS.llm_effort_some;
        let events =
            service::EventLimits::derive(&limits, Capture::Everything, 1, effort, &[], profile.policy.write_deadline)
                .expect("drawn event ownership");
        limits.events = Some(events);
        // Public startup performs the same native endpoint construction. Give
        // that construction room, then price the original drawn ceiling below.
        let mut unbounded = document.clone();
        unbounded["profile"]["declared"]["memory"] = u64::MAX.into();
        let configured = config::from_value(unbounded).expect("drawn native endpoints");
        assert_eq!(configured.declarations, declarations, "the independent drawn declarations match startup");
        let destinations = configured.service.llm_endpoints;
        let service_bytes = service::worst_case(&limits, &destinations).expect("drawn service bound");
        // The shell may hold every inherited root, its error stream and
        // the append file's temporary root before its first pass.
        let roots = limits.domain.run.directories;
        let queue = limits.queue.checked_add(roots).and_then(|value| value.checked_add(2)).unwrap();
        let header = u64::try_from(
            b"smith"
                .len()
                .checked_add(env!("CARGO_PKG_VERSION").len())
                .and_then(|bytes| bytes.checked_add(env!("SMITH_BUILD").len()))
                .and_then(|bytes| bytes.checked_add(b"standard".len()))
                .expect("header strings"),
        )
        .unwrap();
        let complete = service_bytes
            .checked_add(header)
            .and_then(|value| value.checked_add(Queue::<Complete>::worst_case(queue)?))
            .and_then(|value| value.checked_add(Queue::<Submit>::worst_case(queue)?))
            .and_then(|value| value.checked_add(u64::try_from(size_of::<smith_agent_shell::Agent>()).ok()?))
            .and_then(|value| value.checked_add(u64::from(roots) * u64::try_from(size_of::<Fd>()).ok()?))
            .expect("drawn complete bound");
        match validate::memory(complete, limits.memory) {
            Ok(()) => {
                assert!(complete <= profile.declared.memory);
                parse_value(&document).expect("all concrete startup comparisons");
                admitted += 1;
            }
            Err(refusal) => {
                assert_eq!(refusal.check(), Some(Check::Memory));
                let words = String::from_utf8(service::profile::words(&refusal).into_vec()).unwrap();
                assert!(words.contains(&format!("service.worst_case={complete} bytes")), "{words}");
                assert!(words.contains(&format!("declared.memory={} bytes", limits.memory)), "{words}");
                assert!(complete > limits.memory);
                memory_refusals += 1;
            }
        }
    }
    assert!(admitted > 0 && relationship_refusals > 0 && memory_refusals > 0);
    println!(
        "DRAWN_STARTUP samples=96 admitted={admitted} relationship_refusals={relationship_refusals} memory_refusals={memory_refusals}"
    );
}
