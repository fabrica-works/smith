//! Configured names, exact units and event receiving bounds.

use super::super::{
    derive,
    tests::{declarations, profile},
};
use super::{effective, header};
use alloc::boxed::Box;
use skein_lib::List;
use smith_events::LimitValue;

fn names() -> smith_protocol_channel::Endpoints {
    let mut entries = List::with_capacity(2);
    for (number, name) in [(7, b"codex".as_slice()), (8, b"other".as_slice())] {
        entries.push(smith_protocol_channel::Endpoint { number, name: name.into(), dialect: 91 }).expect("names");
    }
    smith_protocol_channel::Endpoints::new(entries)
}

#[test]
fn a_models_head_is_named_by_its_endpoint_and_model_in_milliseconds() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    assert_eq!(value(&header, b"declared.head[endpoint=codex,model=one]"), Some(LimitValue::Integer(5_000)));
}

#[test]
fn effective_names_are_unique_across_two_endpoints_and_models() {
    let mut configuration = declarations();
    let mut second = configuration.endpoints[0].clone();
    second.number = 8;
    for endpoint in [&mut configuration.endpoints[0], &mut second] {
        let mut model = endpoint.models[0].clone();
        model.name = b"two".as_slice().into();
        endpoint.models = Box::new([endpoint.models[0].clone(), model]);
    }
    configuration.endpoints = Box::new([configuration.endpoints[0].clone(), second]);
    let limits = derive(&profile(), &configuration).expect("fixture derives");
    let named =
        effective(&profile(), &configuration, &limits, &endpoints(&configuration, &limits)).expect("bounded list");
    let header = header(&named, &names(), &configuration).expect("names fit");
    for (index, entry) in header.iter().enumerate() {
        for other in &header.as_slice()[index + 1..] {
            assert_ne!(entry.name, other.name);
        }
    }
    assert_eq!(value(&header, b"declared.output[endpoint=other,model=two]"), Some(LimitValue::Integer(512)));
}

#[test]
fn the_compaction_threshold_is_written_as_its_fraction() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    assert_eq!(
        value(&header, b"policy.compaction_threshold"),
        Some(LimitValue::Fraction { numerator: 4, denominator: 5 })
    );
}

#[test]
fn the_sinks_records_hold_the_headers_entries_and_longest_name() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    let events = crate::EventLimits::derive(
        &limits,
        crate::sinks::Capture::None,
        1,
        0,
        header.as_slice(),
        profile().policy.write_deadline,
    )
    .expect("event bounds");
    assert!(events.sinks.records.items >= header.len());
    for entry in &header {
        assert!(entry.name.len() <= usize::try_from(events.sinks.records.string).expect("name bound"));
    }
}

fn value(header: &List<smith_events::EffectiveLimit>, name: &[u8]) -> Option<LimitValue> {
    for entry in header {
        if entry.name.as_ref() == name {
            return Some(entry.value);
        }
    }
    None
}

fn endpoints(configuration: &super::super::Configuration, limits: &crate::Limits) -> smith_protocol_llm::Endpoints {
    let mut configured = List::with_capacity(2);
    for endpoint in &configuration.endpoints {
        configured
            .push(smith_protocol_llm::ConfiguredEndpoint {
                name: smith_domain::llm::Endpoint(endpoint.number),
                destination: skein_llm_connection::Endpoint {
                    limits: super::super::endpoint_client(
                        &profile(),
                        endpoint,
                        limits.llm.adapter.client.history_items,
                    )
                    .expect("endpoint client"),
                    credential: skein_llm::client::CredentialLimits { value: 0, account_id: 0 },
                    address: skein_io::kernel::Addr::from((core::net::Ipv4Addr::LOCALHOST, 443)),
                    transport: skein_llm_connection::Transport::Plaintext,
                    llm: skein_llm::Endpoint::codex(),
                },
                connect: endpoint.connect,
                handshake: endpoint.handshake,
                identity: smith_protocol_llm::IdentityProfile::Plain,
                models: Box::new([]),
            })
            .expect("two endpoints");
    }
    smith_protocol_llm::Endpoints::new(configured.into_boxed(), 2).expect("endpoint names")
}

#[test]
fn each_endpoint_names_its_own_request_bound() {
    let mut configuration = declarations();
    let mut second = configuration.endpoints[0].clone();
    second.number = 8;
    second.models[0].window = 131_072;
    configuration.endpoints = Box::new([configuration.endpoints[0].clone(), second]);
    let limits = derive(&profile(), &configuration).expect("fixture derives");
    let named =
        effective(&profile(), &configuration, &limits, &endpoints(&configuration, &limits)).expect("bounded list");
    let header = header(&named, &names(), &configuration).expect("names fit");
    assert_eq!(value(&header, b"llm.request[endpoint=codex]"), Some(LimitValue::Integer(3_145_728)));
    assert_eq!(value(&header, b"llm.request[endpoint=other]"), Some(LimitValue::Integer(6_291_456)));
}

#[test]
fn the_channels_turn_frame_is_named_under_channel() {
    let mut limits = derive(&profile(), &declarations()).expect("fixture derives");
    limits.channel.bodies.turn_body = 123_457;
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    assert_eq!(value(&header, b"channel.turn_body"), Some(LimitValue::Integer(123_457)));
}

#[test]
fn the_sinks_own_limits_are_named_and_fit_the_header() {
    let mut limits = derive(&profile(), &declarations()).expect("fixture derives");
    let endpoints = endpoints(&declarations(), &limits);
    for _ in 0_u32..2 {
        let named = effective(&profile(), &declarations(), &limits, &endpoints).expect("bounded list");
        let header = header(&named, &names(), &declarations()).expect("names fit");
        limits.events = Some(
            crate::EventLimits::derive(
                &limits,
                crate::sinks::Capture::None,
                1,
                0,
                header.as_slice(),
                profile().policy.write_deadline,
            )
            .expect("event bounds"),
        );
    }
    let named = effective(&profile(), &declarations(), &limits, &endpoints).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    let sinks = limits.events.expect("configured sink").sinks;
    assert_eq!(value(&header, b"events.destinations"), Some(LimitValue::Integer(1)));
    assert!(sinks.records.items >= header.len());
    for entry in &header {
        assert!(entry.name.len() <= usize::try_from(sinks.records.string).expect("name bound"));
    }
}

#[test]
fn parallel_tools_is_printed_as_a_policy_count() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    assert_eq!(value(&header, b"policy.parallel_tools"), Some(LimitValue::Integer(4)));
}

#[test]
fn jobs_print_their_declared_and_derived_tools_counts() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    for name in [b"declared.jobs".as_slice(), b"tools.jobs".as_slice()] {
        assert_eq!(value(&header, name), Some(LimitValue::Integer(4)));
    }
}

#[test]
fn job_poll_and_job_wait_print_their_policy_and_derived_tools_durations() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    for (name, millis) in [
        (b"policy.job_poll".as_slice(), 30_000),
        (b"tools.job_poll".as_slice(), 30_000),
        (b"policy.job_wait".as_slice(), 300_000),
        (b"tools.job_wait".as_slice(), 300_000),
    ] {
        assert_eq!(value(&header, name), Some(LimitValue::Integer(millis)));
    }
}

#[test]
fn the_machine_argument_payload_prints_its_derived_byte_bound() {
    let limits = derive(&profile(), &declarations()).expect("fixture derives");
    let named =
        effective(&profile(), &declarations(), &limits, &endpoints(&declarations(), &limits)).expect("bounded list");
    let header = header(&named, &names(), &declarations()).expect("names fit");
    assert_eq!(value(&header, b"machine.tool_payload"), Some(LimitValue::Integer(256)));
}
