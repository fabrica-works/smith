//! Channel opening stories (protocol/channel.md, sections 2 and 10).
use skein_channel::{Closed, StreamMode};
use smith_channel::CEILINGS;
use smith_channel_world::{Observation, World, initial_grant, latest_grant, refreshed_grant, window_turn_fixture};

#[test]
fn the_halves_open_on_the_highest_version_both_speak() {
    for mode in [StreamMode::One, StreamMode::Two] {
        let mut world = World::new(CEILINGS, CEILINGS, mode);
        world.settle();
        assert!(world.observations().contains(&Observation::HostOpened(5)));
        assert!(world.observations().contains(&Observation::AgentOpened(5)));
    }
}

#[test]
fn no_version_in_common_ends_the_channel_before_a_start() {
    for version in [1, 2, 3, 4] {
        let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
        world.agent_hears_foreign_version(version);
        world.settle();
        assert!(world.observations().contains(&Observation::AgentEnded(Closed::RefusedHere(1))), "version {version}");
        assert!(!world.observations().contains(&Observation::AgentOpened(5)), "version {version}");
    }
}

#[test]
fn a_host_whose_terms_are_too_small_is_refused_at_the_opening() {
    let mut host = CEILINGS;
    host.turn_body = 1;
    let mut world = World::new(host, CEILINGS, StreamMode::Two);
    world.settle();
    assert!(world.observations().contains(&Observation::AgentEnded(Closed::RefusedHere(2))));
}

#[test]
fn a_start_that_decodes_reaches_the_agent() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    assert!(world.observations().contains(&Observation::AgentStart));
}

#[test]
fn a_charter_in_an_unread_version_is_refused_as_invalid_with_no_turns() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from([0, 1]));
    world.settle();
    assert!(world.observations().contains(&Observation::InvalidStart {
        why: smith_channel::InvalidStart::CharterVersion,
        turns: 0,
        spent: 0,
    }));
    assert!(!world.observations().contains(&Observation::AgentStart));
    assert_eq!(
        world.take_host_answer(),
        Some(smith_host_domain::channel::Answer {
            read: None,
            turns: 0,
            spent: 0,
            result: smith_host_domain::channel::RunResult::Refused {
                refusal: smith_host_domain::channel::Refusal::Invalid(
                    smith_host_domain::channel::RunInvalid::CharterVersion,
                ),
            },
        }),
    );
}

#[test]
fn a_malformed_charter_is_refused_with_its_own_reason() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from([0, 4]));
    world.settle();
    assert!(world.observations().contains(&Observation::InvalidStart {
        why: smith_channel::InvalidStart::MalformedCharter,
        turns: 0,
        spent: 0,
    }));
    assert!(!world.observations().contains(&Observation::AgentStart));
}

#[test]
fn a_start_record_that_does_not_decode_breaks_the_channel_rules() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.agent_hears_malformed_start();
    world.settle();
    assert!(world.observations().contains(&Observation::AgentEnded(Closed::RefusedHere(256))));
    assert!(!world.observations().contains(&Observation::AgentStart));
}

#[test]
fn the_agent_resolves_the_charters_endpoint_before_admission() {
    let mut entries = skein_lib::List::with_capacity(1);
    entries
        .push(smith_protocol_channel::Endpoint { name: Box::default(), number: 17, dialect: 23 })
        .expect("one endpoint");
    let endpoints = smith_protocol_channel::Endpoints::new(entries);
    let bytes = include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin");
    let charter =
        smith_protocol_channel::decode_charter(bytes, &smith_charter::CEILINGS, &endpoints).expect("v4 charter");
    assert_eq!(charter.llm.endpoint.0, 17);
    assert_eq!(charter.llm.dialect, 23);
    assert_eq!(charter.llm.account, None);
    assert!(charter.brief.sections.is_empty());
    assert!(charter.models.is_empty());
    let unknown = smith_protocol_channel::decode_charter(
        bytes,
        &smith_charter::CEILINGS,
        &smith_protocol_channel::Endpoints::new(skein_lib::List::with_capacity(0)),
    );
    assert!(matches!(unknown, Err(smith_domain::run::Invalid::Endpoint)));
}

#[test]
fn the_full_charter_keeps_contracts_tools_and_model_prices() {
    let bytes = include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_full.bin");
    let wire = smith_charter::Charter::decode(&smith_charter::CEILINGS, &mut skein_lib::Reader::new(bytes))
        .expect("full charter");
    let mut entries = skein_lib::List::with_capacity(1 + wire.models().len());
    entries
        .push(smith_protocol_channel::Endpoint { name: Box::from(wire.main().endpoint()), number: 17, dialect: 23 })
        .expect("main endpoint");
    for model in wire.models() {
        entries
            .push(smith_protocol_channel::Endpoint { name: Box::from(model.endpoint()), number: 17, dialect: 23 })
            .expect("model endpoint");
    }
    let charter = smith_protocol_channel::decode_charter(
        bytes,
        &smith_charter::CEILINGS,
        &smith_protocol_channel::Endpoints::new(entries),
    )
    .expect("translated charter");
    assert_eq!(charter.instructions.as_ref(), wire.instructions());
    assert_eq!(charter.brief.sections.len(), usize::try_from(wire.brief().len()).expect("bounded count"));
    assert_eq!(charter.grants.host_tools.len(), usize::try_from(wire.tools().host().len()).expect("bounded count"));
    assert_eq!(
        charter.outcome.verdicts.len(),
        usize::try_from(wire.contract().verdicts().len()).expect("bounded count")
    );
    assert_eq!(charter.models.len(), usize::try_from(wire.models().len()).expect("bounded count"));
    assert_eq!(charter.llm.prices.input, wire.main().prices().input());
    assert_eq!(charter.llm.prices.cached, wire.main().prices().cached());
    assert_eq!(charter.llm.prices.output, wire.main().prices().output());
    assert_eq!(charter.llm.prices.unit, wire.main().prices().unit());
    assert_eq!(charter.budget.turns, wire.budget().turns());
    assert_eq!(charter.budget.spend, wire.budget().spend());
    assert_eq!(charter.budget.time, wire.budget().time());
}

#[test]
fn an_accepted_report_keeps_its_fields_in_charter_v2() {
    let result = smith_domain::run::outcome::Declared::Report(smith_domain::run::outcome::Report {
        text: Box::from(*b"done"),
        fields: Box::from([smith_domain::run::outcome::Field {
            name: Box::from(*b"source"),
            value: Box::from(*b"agent"),
        }]),
    });
    let bytes = smith_protocol_channel::encode_result(&result, &smith_charter::CEILINGS).expect("bounded result");
    let record = smith_charter::RunResult::decode(&smith_charter::CEILINGS, &mut skein_lib::Reader::new(&bytes))
        .expect("result decodes");
    assert!(matches!(record.form(), smith_charter::Form::Report));
    assert_eq!(record.text(), b"done");
    assert_eq!(record.fields().get(0).expect("one field").name(), b"source");
    assert_eq!(record.fields().get(0).expect("one field").text(), b"agent");
}

#[test]
fn an_accepted_verdict_keeps_its_label_and_items() {
    let result = smith_domain::run::outcome::Declared::Verdict(smith_domain::run::outcome::Verdict {
        name: Box::from(*b"ready"),
        text: Box::from(*b"reviewed"),
        fields: Box::default(),
        items: Box::from([smith_domain::run::outcome::Item {
            kind: Box::from(*b"path"),
            fields: Box::from([smith_domain::run::outcome::Field {
                name: Box::from(*b"name"),
                value: Box::from(*b"file"),
            }]),
        }]),
    });
    let bytes = smith_protocol_channel::encode_result(&result, &smith_charter::CEILINGS).expect("bounded result");
    let record = smith_charter::RunResult::decode(&smith_charter::CEILINGS, &mut skein_lib::Reader::new(&bytes))
        .expect("result decodes");
    assert!(matches!(record.form(), smith_charter::Form::Verdict));
    assert_eq!(record.label().as_deref(), Some(&b"ready"[..]));
    assert_eq!(record.items().get(0).expect("one item").kind(), b"path");
    assert_eq!(record.items().get(0).expect("one item").fields().get(0).expect("one field").text(), b"file");
}

#[test]
fn a_run_goes_from_start_to_answer() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    assert!(world.observations().contains(&Observation::AgentStart));
    world.agent_admits_and_parks();
    world.settle();
    assert!(world.observations().contains(&Observation::HostAdmitted));
    assert!(world.observations().contains(&Observation::HostParked { turns: 0, spent: 0 }));
    assert!(world.observations().contains(&Observation::HostEnded(Closed::Stream)));
    assert_eq!(
        world.take_host_answer(),
        Some(smith_host_domain::channel::Answer {
            read: None,
            turns: 0,
            spent: 0,
            result: smith_host_domain::channel::RunResult::Parked
        }),
    );
}

#[test]
fn a_model_failure_keeps_transport_evidence_and_cooldown() {
    let answer = smith_domain::run::Answer::Failed {
        failure: smith_domain::run::Failure::Model(smith_domain::run::Fault::Completion {
            failure: smith_domain::run::CompletionFailure::RateLimited {
                retry_after: skein_lib::Duration::from_nanos(42),
            },
            evidence: smith_domain::run::CompletionEvidence::Response { status: None },
            model: 0,
            attempts: 1,
            report: smith_domain::run::Report::none(),
        }),
        spent: smith_domain::run::Spend::ZERO,
        turns: 2,
    };
    let record =
        smith_protocol_channel::answer_record(skein_lib::Time::ZERO, answer, None, &CEILINGS, &smith_charter::CEILINGS)
            .expect("bounded failure");
    assert_eq!(record.turns(), 2);
    let smith_channel::RunResult::Failed(failed) = record.result() else {
        panic!("expected failure");
    };
    let smith_channel::RunFailure::Model(model) = failed.reason() else {
        panic!("expected model failure");
    };
    let smith_channel::ModelFault::Completion(fault) = model.value() else {
        panic!("expected completion");
    };
    assert!(matches!(fault.failure(), smith_channel::CompletionFailure::RateLimited));
    assert!(matches!(fault.evidence(), smith_channel::CompletionEvidence::Response(_)));
    assert_eq!(fault.retry_after(), &Some(skein_lib::Duration::from_nanos(42)));
}

#[test]
fn a_model_failure_reaches_the_host_domain_in_its_typed_vocabulary() {
    use smith_domain::run;
    use smith_host_domain as host;
    for (fault, expected) in channel_faults() {
        let mut world = admitted_channel();
        world.agent_answers(run::Answer::Failed {
            failure: run::Failure::Model(fault),
            spent: run::Spend::ZERO,
            turns: 0,
        });
        world.settle();
        assert_eq!(
            world.take_host_answer(),
            Some(host::Answer {
                read: None,
                turns: 0,
                spent: 0,
                result: host::RunResult::Failed { failure: host::RunFailure::Model(expected) },
            })
        );
    }
}

fn channel_faults() -> [(smith_domain::run::Fault, smith_host_domain::ModelFault); 10] {
    use smith_domain::run;
    use smith_host_domain as host;
    let retry_after = skein_lib::Duration::from_nanos(42);
    [
        (
            run::Fault::Limited { account: 7, reset_at: Some(skein_lib::Time::from_nanos(42)) },
            host::ModelFault::Limited { account: 7, reset: Some(retry_after) },
        ),
        (run::Fault::Limited { account: 7, reset_at: None }, host::ModelFault::Limited { account: 7, reset: None }),
        (
            run::Fault::Refused { account: 7, code: b"quota".as_slice().into() },
            host::ModelFault::Refused { account: 7, code: b"quota".as_slice().into() },
        ),
        (run::Fault::Provider, host::ModelFault::Provider),
        (run::Fault::Declined, host::ModelFault::Declined),
        (run::Fault::Truncated, host::ModelFault::Truncated),
        (run::Fault::Malformed, host::ModelFault::Malformed),
        channel_completion(
            run::CompletionFailure::RateLimited { retry_after },
            host::CompletionFailure::RateLimited { retry_after },
        ),
        channel_completion(run::CompletionFailure::Refused, host::CompletionFailure::Refused),
        channel_completion(run::CompletionFailure::Unsupported, host::CompletionFailure::Unsupported),
    ]
}

fn admitted_channel() -> World {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    world
}

fn channel_completion(
    failure: smith_domain::run::CompletionFailure,
    expected: smith_host_domain::CompletionFailure,
) -> (smith_domain::run::Fault, smith_host_domain::ModelFault) {
    use smith_domain::run;
    use smith_host_domain as host;
    (
        run::Fault::Completion {
            failure,
            evidence: run::CompletionEvidence::Response { status: None },
            model: 0,
            attempts: 1,
            report: run::Report {
                code: b"code".as_slice().into(),
                request: b"request".as_slice().into(),
                field: b"feature".as_slice().into(),
            },
        },
        host::ModelFault::Completion {
            failure: expected,
            evidence: host::CompletionEvidence::Response { status: None },
            model: 0,
            attempts: 1,
            report: host::Report {
                code: b"code".as_slice().into(),
                request: b"request".as_slice().into(),
                field: b"feature".as_slice().into(),
            },
        },
    )
}

#[test]
fn the_start_keeps_workspace_paths_saved_answers_and_grant_values_below_the_domain() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start_with_context(
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]),
        smith_host_domain::SavedReply::Host { error: false, body: Box::from(*b"ok") },
    );
    world.settle();
    assert!(world.observations().contains(&Observation::AgentContext {
        path: Box::from(*b"/tmp/src"),
        answered: 1,
        window: 2,
    }));
    assert!(world.agent_grant_value(3, 4) == Some(initial_grant()));
    assert!(world.observations().contains(&Observation::AgentSavedHost {
        activation: 6,
        tool: Box::from(*b"check"),
        text: Box::from(*b"ok"),
        error: false,
    }));
}

#[test]
fn a_saved_oversized_answer_reaches_the_agent_as_a_settled_decision() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start_with_context(
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]),
        smith_host_domain::SavedReply::TooLarge,
    );
    world.settle();
    assert!(world.observations().contains(&Observation::AgentSavedTooLarge));
}

fn saved_turn(place: u32, dialect: u32) -> Box<[u8]> {
    let limits = smith_transcript::CEILINGS;
    let turn = smith_transcript::Turn::new(
        &limits,
        smith_transcript::TurnParts {
            model: b"fake-1".as_slice().into(),
            effort: None,
            kind: smith_transcript::TurnKind::Ordinary,
            opening: if place == 1 {
                Some(
                    smith_transcript::Opening::new(
                        &limits,
                        smith_transcript::OpeningParts {
                            affinity: smith_transcript::Affinity::new(
                                &limits,
                                smith_transcript::AffinityParts { key: [0; 16], thread: 0 },
                            )
                            .expect("fixture affinity"),
                            account: None,
                            backend: smith_transcript::Backend::Direct,
                        },
                    )
                    .expect("fixture opening"),
                )
            } else {
                None
            },
            endpoint: Box::default(),
            dialect,
            place,
            usage: smith_transcript::Usage::new(
                &limits,
                smith_transcript::UsageParts {
                    input: Some(1),
                    output: Some(2),
                    cache_read: Some(0),
                    cache_write: Some(0),
                    reasoning: None,
                },
            )
            .expect("usage"),
            spent: 3,
            jobs: 0,
            messages: skein_lib::List::with_capacity(0),
        },
    )
    .expect("turn");
    let mut writer = skein_lib::Writer::new(usize::try_from(turn.measure()).expect("turn length"));
    turn.encode(&mut writer).expect("measured turn");
    writer.finish()
}

#[test]
fn a_saved_turn_reaches_the_agent_as_concrete_history() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start_with_turns(
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]),
        Some(Box::from([saved_turn(1, 0)])),
    );
    world.settle();
    assert!(world.observations().contains(&Observation::AgentHistory { turns: 1, place: 1 }));
}

#[test]
fn a_saved_turn_from_another_dialect_fails_as_transcript() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start_with_turns(
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]),
        Some(Box::from([saved_turn(1, 99)])),
    );
    world.settle();
    let start = world.take_agent_start().expect("foreign history reaches the agent");
    assert_eq!(start.transcript.as_ref().expect("saved history").dialect, 99);
    assert!(world.observations().contains(&Observation::AgentHistory { turns: 1, place: 1 }));
    assert!(world.observations().contains(&Observation::AgentStart));
}

#[test]
fn a_run_resumed_from_a_transcript_restores_the_calls_answered_after_its_last_turn() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start_with_context_and_turns(
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]),
        smith_host_domain::SavedReply::Host { error: true, body: Box::from(*b"retry") },
        Some(Box::from([saved_turn(1, 0)])),
    );
    world.settle();
    assert!(world.observations().contains(&Observation::AgentHistory { turns: 1, place: 1 }));
    assert!(world.observations().contains(&Observation::AgentSavedHost {
        activation: 6,
        tool: Box::from(*b"check"),
        text: Box::from(*b"retry"),
        error: true,
    }));
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "the story selects each activation's one forwarded turn")]
#[expect(
    clippy::too_many_lines,
    reason = "two activations follow concrete windows, identity, fences and exact turn accounting"
)]
fn a_run_parked_resumed_and_parked_again_keeps_its_identity_and_numbers_turns_per_activation() {
    use smith_domain_session::{llm, record};

    let charter: Box<[u8]> =
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]);
    let first = record::Turn {
        model: b"model".as_slice().into(),
        effort: None,
        kind: smith_domain_session::record::Kind::First(Box::new(smith_domain_session::record::Opener {
            affinity: smith_domain_session::llm::Affinity { key: [0; 16], thread: 0 },
            account: None,
            backend: smith_domain_session::record::Backend::Direct,
        })),
        version: record::VERSION,
        endpoint: Some(llm::Endpoint(0)),
        dialect: 0,
        sequence: 1,
        usage: llm::Usage {
            input_tokens: Some(1),
            output_tokens: Some(2),
            cache_read_tokens: Some(0),
            cache_write_tokens: Some(0),
            reasoning_tokens: None,
        },
        spent: 3,
        jobs: 0,
        messages: Box::default(),
    };
    let mut one = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    one.settle();
    let identity = smith_domain::run::Identity::new(*b"stable-chat-name");
    one.send_start_with_identity(charter.clone(), None, Some(identity));
    one.settle();
    let first_start = one.take_agent_start().expect("first decoded activation");
    assert_eq!(first_start.identity, Some(identity));
    one.agent_admits();
    one.settle();
    one.agent_tells_turn(1, Some(skein_lib::Token::new(99)), &first);
    one.settle();
    one.host_acknowledges(1);
    one.settle();
    let mut ordinary = first.clone();
    ordinary.sequence = 2;
    ordinary.kind = record::Kind::Ordinary;
    ordinary.spent = 4;
    one.agent_tells_turn(2, Some(skein_lib::Token::new(99)), &ordinary);
    one.settle();
    let saved: Vec<_> = one
        .observations()
        .iter()
        .filter_map(|event| match event {
            Observation::HostTurn { opens_window, body, .. } => Some((*opens_window, body.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(saved.iter().map(|(flag, _)| *flag).collect::<Vec<_>>(), [true, false]);
    let last_window = saved.iter().rposition(|(flag, _)| *flag).expect("first declared history window");
    let saved: Vec<_> = saved[last_window..].iter().map(|(_, body)| body.clone()).collect();
    one.host_acknowledges(2);
    one.settle();
    let mut spent = smith_domain::run::Spend::ZERO;
    spent.units = 4;
    one.agent_answers_with_fence(
        smith_domain::run::Answer::Parked { spent, turns: 2 },
        Some(skein_lib::Token::new(99)),
    );
    one.settle();
    assert!(one.observations().contains(&Observation::HostParked { turns: 2, spent: 4 }));
    assert_eq!(one.take_host_answer().expect("decoded final fence").read, Some(skein_lib::Token::new(99)));

    let mut two = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    two.settle();
    two.send_domain_start(
        smith_host_domain::Start {
            live: None,
            messages: Box::default(),
            logical_run: skein_lib::Token::new(1),
            identity: Some(identity),
            activation: 2,
            workspace: None,
            charter: smith_channel_world::typed_charter(&charter),
            transcript: smith_channel_world::typed_transcript(&saved),
            answered: Box::default(),
            directories: Box::default(),
            grants: Box::default(),
        },
        smith_host_domain::Window { turns: 1, bytes: 1_000_000_000 },
        smith_host_protocol::Values { paths: Box::default(), credentials: Box::default() },
    );
    two.settle();
    let resumed = two.take_agent_start().expect("resumed decoded activation");
    assert_eq!(resumed.identity, Some(identity), "both channel halves preserve the same exact sixteen bytes");
    assert_eq!(resumed.activation, 2);
    assert!(two.observations().contains(&Observation::AgentHistory { turns: 2, place: 1 }));
    two.agent_admits();
    two.settle();
    let mut next = first;
    next.sequence = 3;
    next.kind = record::Kind::Ordinary;
    next.spent = 5;
    two.agent_tells_turn(1, Some(skein_lib::Token::new(99)), &next);
    two.settle();
    let body = two
        .observations()
        .iter()
        .find_map(|event| match event {
            Observation::HostTurn { number: 1, opens_window: false, body, .. } => Some(body.as_ref()),
            _ => None,
        })
        .expect("new activation's first output turn");
    let received = smith_transcript::Turn::decode(&smith_transcript::CEILINGS, &mut skein_lib::Reader::new(body))
        .expect("bounded output turn");
    assert_eq!(received.place(), 3, "conversation sequence continues after saved history");
    two.host_acknowledges(1);
    two.settle();
    let mut spent = smith_domain::run::Spend::ZERO;
    spent.units = 5;
    two.agent_answers(smith_domain::run::Answer::Parked { spent, turns: 1 });
    two.settle();
    assert!(two.observations().contains(&Observation::HostParked { turns: 1, spent: 5 }));
}

#[test]
fn a_message_reaches_the_llm_with_its_senders_label() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let name = skein_lib::Token::new(31);
    world.send_message(name, Box::from(*b"Ada"), Box::from(*b"Please retry"), false);
    world.settle();
    assert!(world.observations().contains(&Observation::AgentMessage {
        finish_requested: false,
        name,
        label: Box::from(*b"Ada"),
        text: Box::from(*b"Please retry")
    }));
    world.agent_waits(Some(name));
    world.settle();
    assert!(world.observations().contains(&Observation::HostWaiting { read: Some(name) }));
}

#[test]
fn finish_requested_crosses_the_live_wire_with_name_label_and_text() {
    let mut world = admitted_channel();
    for (name, finish_requested) in [(31, false), (32, true)] {
        let name = skein_lib::Token::new(name);
        world.send_message(name, Box::from(*b"Ada"), Box::from(*b"Please finish"), finish_requested);
        world.settle();
        assert!(world.observations().contains(&Observation::AgentMessage {
            finish_requested,
            name,
            label: Box::from(*b"Ada"),
            text: Box::from(*b"Please finish"),
        }));
    }
}

#[test]
fn a_long_operation_stretches_progress_until_its_end() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let span = skein_lib::Duration::from_nanos(42);
    world.agent_starts_long(smith_domain::Operation::Checks, span);
    world.settle();
    world.agent_ends_long();
    world.settle();
    assert!(world.observations().contains(&Observation::HostLong { operation: smith_domain::Operation::Checks, span }));
    assert!(world.observations().contains(&Observation::HostLongDone));
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "test selects one observed wire turn")]
fn a_concrete_turn_reaches_the_host_and_its_acknowledgement_returns() {
    use smith_domain_session::{llm, record};

    let turn = record::Turn {
        model: b"model".as_slice().into(),
        effort: None,
        kind: smith_domain_session::record::Kind::First(Box::new(smith_domain_session::record::Opener {
            affinity: smith_domain_session::llm::Affinity { key: [0; 16], thread: 0 },
            account: None,
            backend: smith_domain_session::record::Backend::Direct,
        })),
        version: record::VERSION,
        endpoint: Some(llm::Endpoint(0)),
        dialect: 0,
        sequence: 1,
        usage: llm::Usage {
            input_tokens: None,
            output_tokens: Some(5),
            cache_read_tokens: Some(0),
            cache_write_tokens: None,
            reasoning_tokens: Some(2),
        },
        spent: 11,
        jobs: 0,
        messages: Box::from([llm::Message {
            role: llm::Role::Assistant,
            content: Box::from([
                llm::Block::Text { text: Box::from(*b"hello"), replay: None },
                llm::Block::ToolCall {
                    id: Box::from(*b"id"),
                    name: Box::from(*b"check"),
                    input: Box::from(*b"{}"),
                    call: llm::Decoded::Historical,
                    replay: None,
                },
                llm::Block::ToolResult {
                    id: Box::from(*b"id"),
                    result: llm::Returned::Invalid { problem: llm::Problem::UnknownTool },
                },
            ]),
        }]),
    };
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let read = Some(skein_lib::Token::new(31));
    world.agent_tells_turn(1, read, &turn);
    world.settle();
    let body = world
        .observations()
        .iter()
        .find_map(|observation| match observation {
            Observation::HostTurn { number: 1, spent: 11, read: actual, opens_window: true, body }
                if *actual == read =>
            {
                Some(body.clone())
            }
            _ => None,
        })
        .expect("one forwarded turn");
    let mut endpoints = skein_lib::List::with_capacity(1);
    endpoints
        .push(smith_protocol_channel::Endpoint { name: Box::default(), number: 0, dialect: 0 })
        .expect("one endpoint");
    let transcript = smith_protocol_channel::decode_transcript(
        &smith_protocol_channel::transcript_piece(&[body]).expect("fixture piece"),
        1,
        u32::MAX,
        &smith_transcript::CEILINGS,
        &smith_protocol_channel::Endpoints::new(endpoints),
    )
    .expect("decoded turn")
    .expect("one turn");
    assert_eq!(transcript.turns.as_ref(), std::slice::from_ref(&turn));
    let mut second = turn.clone();
    second.sequence = 2;
    second.kind = record::Kind::Ordinary;
    assert_eq!(world.try_agent_tells_turn(2, read, &second), Err(smith_protocol_channel::Error::Window));
    world.host_acknowledges(1);
    world.settle();
    assert!(world.observations().contains(&Observation::AgentAcknowledged { turn: 1 }));
    world.agent_tells_turn(2, read, &second);
    world.settle();
    assert!(
        world.observations().iter().any(|observation| matches!(observation, Observation::HostTurn { number: 2, .. }))
    );
}

#[test]
fn facts_are_dropped_under_pressure_and_waiting_still_passes() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let elapsed = skein_lib::Duration::from_nanos(7);
    let mut kept = 0_u64;
    for _ in 0..12 {
        let fact = smith_domain::Fact::Run {
            opening: None,
            fact: smith_domain::run::facts::Fact {
                at: skein_lib::Time::from_nanos(7),
                kind: smith_domain::run::facts::FactKind::Admitted { run: skein_lib::Token::new(1), resumed: false },
            },
        };
        if world.agent_sends_fact(&fact, skein_lib::Time::ZERO) {
            kept += 1;
        }
    }
    assert!(kept > 0);
    assert_eq!(world.lost_facts(), 12 - kept);
    assert!(world.lost_facts() > 0);
    world.agent_waits(None);
    world.settle();
    assert!(world.observations().contains(&Observation::HostWaiting { read: None }));
    assert_eq!(
        world
            .observations()
            .iter()
            .filter(|observation| matches!(
                observation, Observation::HostFact { kind: smith_channel::FactKind::Admitted, elapsed: seen, count: 1 }
                    if *seen == elapsed
            ))
            .count(),
        usize::try_from(kept).expect("small fact count")
    );
}

#[test]
fn grants_refreshed_while_a_call_is_in_flight_reach_the_table() {
    let mut world = world_with_initial_grant();
    world.agent_calls_host(
        smith_domain::run::CallName { activation: 7, completion: 1, position: 0 },
        smith_domain::run::RelayName { owner: skein_lib::Token::new(10), attempt: 1 },
    );
    world.settle();
    world.host_refreshes_grant(3, 5, refreshed_grant());
    world.settle();
    assert!(world.agent_grant_value(3, 4) == Some(initial_grant()));
    assert!(world.agent_grant_value(3, 5) == Some(refreshed_grant()));
    assert!(world.observations().contains(&Observation::AgentGrant {
        account: 3,
        generation: 5,
        valid: skein_lib::Duration::from_nanos(50),
    }));
    world.host_refreshes_grant(3, 6, latest_grant());
    world.settle();
    assert!(world.agent_grant_value(3, 4).is_none());
    assert!(world.agent_grant_value(3, 5) == Some(refreshed_grant()));
    assert!(world.agent_grant_value(3, 6) == Some(latest_grant()));
}

#[test]
fn account_notices_reach_the_host_without_values() {
    type Report = fn(&mut World);
    let rows: [(Report, Observation); 4] = [
        (|world| world.agent_reports_unauthorized(3, 4), Observation::HostUnauthorized { account: 3, generation: 4 }),
        (
            |world| world.agent_reports_limited(3, Some(skein_lib::Duration::ZERO)),
            Observation::HostLimited { account: 3, reset: Some(skein_lib::Duration::ZERO) },
        ),
        (|world| world.agent_reports_limited(3, None), Observation::HostLimited { account: 3, reset: None }),
        (
            |world| world.agent_reports_refused(3, b"model_refused", b"plan does not serve it"),
            Observation::HostRefused {
                account: 3,
                code: Box::from(*b"model_refused"),
                reason: Box::from(*b"plan does not serve it"),
            },
        ),
    ];
    for (report, expected) in rows {
        let mut world = admitted_notice_world();
        report(&mut world);
        world.settle();
        assert!(world.observations().contains(&expected), "{expected:?}");
    }
}

fn admitted_notice_world() -> World {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    world
}

#[test]
fn cancellation_records_and_end_each_wait_for_their_read() {
    #[expect(clippy::wildcard_enum_match_arm, reason = "this story selects the four demanded terminals it queues")]
    fn terminals(world: &World) -> Vec<&'static str> {
        world
            .observations()
            .iter()
            .filter_map(|observation| match observation {
                Observation::HostAdmitted => Some("admitted"),
                Observation::HostTurn { .. } => Some("turn"),
                Observation::HostAnswer { .. } => Some("answer"),
                Observation::HostEnded(_) => Some("end"),
                _ => None,
            })
            .collect()
    }

    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.host_holds_reads();
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    assert_eq!(world.host_read_count(), 0, "opening is internal");
    assert!(terminals(&world).is_empty(), "admission waits for demand");
    world.host_reads();
    world.settle();
    assert_eq!(world.host_read_count(), 1);
    assert_eq!(terminals(&world), ["admitted"]);

    world.host_cancels();
    world.settle();
    world.agent_tells_turn(1, None, &window_turn_fixture());
    world.agent_answers(smith_domain::run::Answer::Failed {
        failure: smith_domain::run::Failure::Cancelled,
        spent: smith_domain::run::Spend::ZERO,
        turns: 1,
    });
    world.settle();
    assert_eq!(world.host_read_count(), 1);
    assert_eq!(terminals(&world), ["admitted"], "queued final records stay below the read boundary");
    assert!(world.take_host_answer().is_none());

    world.host_reads();
    world.settle();
    assert_eq!(world.host_read_count(), 2);
    assert_eq!(terminals(&world), ["admitted", "turn"]);
    assert!(world.take_host_answer().is_none(), "one read cannot also deliver Answer");
    assert!(world.observations().iter().any(|observation| matches!(
        observation,
        Observation::HostTurn { number: 1, spent: 0, read: None, opens_window: true, .. }
    )));

    world.host_reads();
    world.settle();
    assert_eq!(world.host_read_count(), 3);
    assert_eq!(terminals(&world), ["admitted", "turn", "answer"]);
    assert_eq!(
        world.take_host_answer(),
        Some(smith_host_domain::channel::Answer {
            read: None,
            turns: 1,
            spent: 0,
            result: smith_host_domain::channel::RunResult::Failed {
                failure: smith_host_domain::channel::RunFailure::Cancelled,
            },
        })
    );

    world.host_reads();
    world.settle();
    assert_eq!(world.host_read_count(), 4);
    assert_eq!(terminals(&world), ["admitted", "turn", "answer", "end"]);
    assert!(world.observations().contains(&Observation::HostEnded(Closed::Stream)));
    assert_eq!(world.observations().iter().filter(|event| **event == Observation::AgentCancel).count(), 1);
    assert!(!world.observations().iter().any(|event| matches!(event, Observation::AgentAcknowledged { .. })));
    world.settle();
    assert_eq!(world.host_read_count(), 4);
    assert_eq!(terminals(&world), ["admitted", "turn", "answer", "end"], "end is reported once");
}

#[test]
fn a_cancel_with_a_host_call_in_flight_keeps_its_terminal_and_one_answer() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let name = smith_domain::run::CallName { activation: 1, completion: 1, position: 0 };
    let relay = smith_domain::run::RelayName { owner: skein_lib::Token::new(41), attempt: 1 };
    world.agent_calls_host(name, relay);
    world.settle();
    world.host_cancels();
    world.settle();
    assert!(world.observations().contains(&Observation::AgentCancel));
    world.host_answers(skein_lib::Token::new(1), smith_host_domain::channel::Reply::Busy);
    world.settle();
    assert!(world.observations().iter().any(|observation| matches!(
        observation, Observation::AgentHostReturned { relay: seen, .. } if *seen == relay
    )));
    world.agent_parks();
    world.settle();
    assert_eq!(
        world.observations().iter().filter(|observation| matches!(observation, Observation::HostParked { .. })).count(),
        1
    );
}

#[test]
fn a_failed_agent_write_does_not_end_its_read_stream() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    world.agent_next_write_fails();
    world.agent_waits(None);
    world.settle();
    assert!(world.observations().contains(&Observation::AgentWriteFailed));
    world.send_message(skein_lib::Token::new(44), Box::from(*b"Ada"), Box::from(*b"continue"), false);
    world.settle();
    assert!(world.observations().contains(&Observation::AgentMessage {
        finish_requested: false,
        name: skein_lib::Token::new(44),
        label: Box::from(*b"Ada"),
        text: Box::from(*b"continue"),
    }));
}

#[test]
fn host_tools_answered_busy_are_asked_again_under_their_names() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let name = smith_domain::run::CallName { activation: 1, completion: 1, position: 0 };
    let first = smith_domain::run::RelayName { owner: skein_lib::Token::new(40), attempt: 1 };
    world.agent_calls_host(name, first);
    world.settle();
    assert!(world.observations().contains(&Observation::HostCall {
        call: skein_lib::Token::new(1),
        name: smith_host_domain::channel::CallName { activation: 1, completion: 1, position: 0 },
        deadline: skein_lib::Time::from_nanos(100),
        ask: Box::new(smith_host_domain::channel::Ask::Host {
            tool: Box::from(*b"check"),
            effect: smith_host_domain::channel::Effect::Read,
            body: Box::from(*b"{}"),
        }),
    }));
    world.host_answers(skein_lib::Token::new(1), smith_host_domain::channel::Reply::Busy);
    world.settle();
    assert!(
        world
            .observations()
            .contains(&Observation::AgentHostReturned { relay: first, reply: smith_domain::run::HostReply::Busy })
    );
    let second = smith_domain::run::RelayName { owner: skein_lib::Token::new(40), attempt: 2 };
    world.agent_calls_host(name, second);
    world.settle();
    assert!(world.observations().contains(&Observation::HostCall {
        call: skein_lib::Token::new(2),
        name: smith_host_domain::channel::CallName { activation: 1, completion: 1, position: 0 },
        deadline: skein_lib::Time::from_nanos(100),
        ask: Box::new(smith_host_domain::channel::Ask::Host {
            tool: Box::from(*b"check"),
            effect: smith_host_domain::channel::Effect::Read,
            body: Box::from(*b"{}"),
        }),
    }));
    world.host_answers(
        skein_lib::Token::new(2),
        smith_host_domain::channel::Reply::Host { error: false, body: Box::from(*b"ok") },
    );
    world.settle();
    assert!(world.observations().contains(&Observation::AgentHostReturned {
        relay: second,
        reply: smith_domain::run::HostReply::Answered(
            smith_domain::run::HostAnswer::new(Box::from(*b"ok"), false).expect("bounded answer"),
        ),
    }));
}

#[test]
fn a_host_tool_answered_as_the_channel_is_lost_is_asked_again_under_its_name() {
    let charter: Box<[u8]> =
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]);
    let name = smith_domain::run::CallName { activation: 1, completion: 1, position: 0 };
    let first_relay = smith_domain::run::RelayName { owner: skein_lib::Token::new(40), attempt: 1 };
    let mut first = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    first.settle();
    first.send_start(charter.clone());
    first.settle();
    first.agent_admits();
    first.settle();
    first.agent_calls_host(name, first_relay);
    first.settle();
    assert!(first.observations().iter().any(|event| matches!(event,
        Observation::HostCall { name: received, .. }
            if *received == smith_host_domain::CallName { activation: 1, completion: 1, position: 0 }
    )));
    first.cut_agent_input_after(0);
    first.settle();
    first.host_answers(
        skein_lib::Token::new(1),
        smith_host_domain::Reply::Host { error: false, body: Box::from(*b"settled during loss") },
    );
    first.settle();
    assert!(!first.observations().iter().any(|event| matches!(event, Observation::AgentHostReturned { .. })));

    let second_relay = smith_domain::run::RelayName { owner: skein_lib::Token::new(41), attempt: 1 };
    let mut second = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    second.settle();
    second.send_domain_start(
        smith_host_domain::Start {
            live: None,
            messages: Box::default(),
            logical_run: skein_lib::Token::new(1),
            identity: None,
            activation: 2,
            workspace: None,
            charter: smith_channel_world::typed_charter(&charter),
            transcript: None,
            answered: Box::default(),
            directories: Box::default(),
            grants: Box::default(),
        },
        smith_host_domain::Window { turns: 1, bytes: 1_000_000_000 },
        smith_host_protocol::Values { paths: Box::default(), credentials: Box::default() },
    );
    second.settle();
    second.agent_admits();
    second.settle();
    second.agent_calls_host(name, second_relay);
    second.settle();
    assert!(second.observations().iter().any(|event| matches!(event,
        Observation::HostCall { name: received, .. }
            if *received == smith_host_domain::CallName { activation: 1, completion: 1, position: 0 }
    )));
    second.host_answers(
        skein_lib::Token::new(1),
        smith_host_domain::Reply::Host { error: false, body: Box::from(*b"settled again") },
    );
    second.settle();
    assert!(second.observations().contains(&Observation::AgentHostReturned {
        relay: second_relay,
        reply: smith_domain::run::HostReply::Answered(
            smith_domain::run::HostAnswer::new(Box::from(*b"settled again"), false).expect("bounded answer"),
        ),
    }));
}

#[test]
fn a_withdrawn_host_call_still_returns_under_its_live_relay() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let name = smith_domain::run::CallName { activation: 1, completion: 1, position: 0 };
    let relay = smith_domain::run::RelayName { owner: skein_lib::Token::new(40), attempt: 1 };
    world.agent_calls_host(name, relay);
    world.settle();
    world.agent_withdraws(relay);
    world.settle();
    assert!(world.observations().contains(&Observation::HostWithdraw { call: skein_lib::Token::new(1) }));
    world.host_answers(skein_lib::Token::new(1), smith_host_domain::channel::Reply::Withdrawn);
    world.settle();
    assert!(
        world
            .observations()
            .contains(&Observation::AgentHostReturned { relay, reply: smith_domain::run::HostReply::Withdrawn })
    );
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "test extracts one observation from many event kinds")]
fn a_delivery_keeps_fields_and_a_settled_landing_or_stale_terminal() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let owner = skein_lib::Token::new(50);
    world.agent_delivers(smith_domain::run::CallName { activation: 1, completion: 1, position: 0 }, owner);
    world.settle();
    let fields = world.observations().iter().find_map(|observation| match observation {
        Observation::HostCall { call, ask: boxed, .. } if *call == skein_lib::Token::new(1) => match boxed.as_ref() {
            smith_host_domain::channel::Ask::Deliver { fields } => Some(fields.value()),
            smith_host_domain::channel::Ask::Host { .. } => None,
        },
        _ => None,
    });
    let fields = fields.expect("host receives generic delivery fields");
    assert_eq!(fields.fields[0].name.as_ref(), b"title");
    assert_eq!(fields.fields[0].value.as_ref(), b"Fix");
    let receipt = smith_host_domain::Receipt::new(0, Box::from(*b"commit-1")).expect("bounded receipt");
    let landed = smith_host_domain::Delivered::new(Box::from([receipt])).expect("one receipt");
    world.host_answers(
        skein_lib::Token::new(1),
        smith_host_domain::channel::Reply::Delivery(smith_host_domain::Delivery::Delivered(landed)),
    );
    world.settle();
    let expected =
        smith_domain::run::Delivered::new(Box::from([
            smith_domain::run::Receipt::new(0, Box::from(*b"commit-1")).expect("bounded receipt")
        ]))
        .expect("one receipt");
    assert!(world.observations().contains(&Observation::AgentDelivered {
        owner,
        delivery: Box::new(smith_domain::run::Delivery::Delivered(expected)),
    }));
    let later = skein_lib::Token::new(51);
    world.agent_delivers(smith_domain::run::CallName { activation: 1, completion: 2, position: 0 }, later);
    world.settle();
    world.host_answers(
        skein_lib::Token::new(2),
        smith_host_domain::channel::Reply::Delivery(smith_host_domain::Delivery::Stale),
    );
    world.settle();
    assert!(world.observations().contains(&Observation::AgentDelivered {
        owner: later,
        delivery: Box::new(smith_domain::run::Delivery::Stale),
    }));
}

#[test]
fn a_fact_projects_its_emission_time_relative_to_the_activation_start() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    let fact = smith_domain::Fact::Run {
        opening: None,
        fact: smith_domain::run::facts::Fact {
            at: skein_lib::Time::from_nanos(91),
            kind: smith_domain::run::facts::FactKind::CheckStarted {
                run: skein_lib::Token::new(1),
                deadline: skein_lib::Time::from_nanos(101),
            },
        },
    };
    assert!(world.agent_sends_fact(&fact, skein_lib::Time::from_nanos(83)));
    world.settle();
    assert!(world.observations().contains(&Observation::HostFact {
        kind: smith_channel::FactKind::CheckStarted,
        elapsed: skein_lib::Duration::from_nanos(8),
        count: 1,
    }));
}

#[cfg(test)]
mod charter_round_trip {
    use skein_lib::{List, Reader};
    use smith_protocol_channel::{Endpoints, encode_charter};

    #[test]
    fn complete_charter_policy_round_trips_without_local_contract_assumptions() {
        let bytes = include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_full.bin");
        let wire =
            smith_charter::Charter::decode(&smith_charter::CEILINGS, &mut Reader::new(bytes)).expect("full golden");
        let mut names = List::with_capacity(16);
        names
            .push(smith_protocol_channel::Endpoint { name: Box::from(wire.main().endpoint()), number: 0, dialect: 0 })
            .expect("main endpoint");
        for model in wire.models() {
            if model.endpoint() != wire.main().endpoint() {
                let number = names.len();
                names
                    .push(smith_protocol_channel::Endpoint { name: Box::from(model.endpoint()), number, dialect: 0 })
                    .expect("child endpoint");
            }
        }
        let endpoints = Endpoints::new(names);
        let policy = smith_protocol_channel::decode_charter(bytes, &smith_charter::CEILINGS, &endpoints)
            .expect("typed full policy");
        let encoded = encode_charter(&policy, &smith_charter::CEILINGS, &endpoints).expect("full policy encodes");
        assert_eq!(smith_protocol_channel::decode_charter(&encoded, &smith_charter::CEILINGS, &endpoints), Ok(policy));
    }
}

#[test]
fn facts_outside_actual_admission_are_counted_without_failing_the_channel() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    let fact = smith_domain::Fact::Run {
        opening: None,
        fact: smith_domain::run::facts::Fact {
            at: skein_lib::Time::ZERO,
            kind: smith_domain::run::facts::FactKind::CheckFinished {
                run: skein_lib::Token::new(1),
                exit: smith_domain::run::Exit::Code { code: 0 },
            },
        },
    };
    assert!(!world.agent_sends_fact(&fact, skein_lib::Time::ZERO));
    assert_eq!(world.lost_facts(), 1);
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    assert!(!world.agent_sends_fact(&fact, skein_lib::Time::ZERO));
    assert_eq!(world.lost_facts(), 2);
    world.agent_admits();
    world.settle();
    assert!(world.agent_sends_fact(&fact, skein_lib::Time::ZERO));
    world.settle();
    assert_eq!(world.lost_facts(), 2);
    world.agent_answers(smith_domain::run::Answer::Failed {
        failure: smith_domain::run::Failure::Cancelled,
        spent: smith_domain::run::Spend::ZERO,
        turns: 0,
    });
    assert!(!world.agent_sends_fact(&fact, skein_lib::Time::ZERO));
    assert_eq!(world.lost_facts(), 3);
    world.settle();
    assert!(!world.agent_sends_fact(&fact, skein_lib::Time::ZERO));
    assert_eq!(world.lost_facts(), 4);
}

#[test]
fn overlapping_kinds_have_exactly_one_terminal_per_begin() {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(Box::from(
        &include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
    ));
    world.settle();
    world.agent_admits();
    world.settle();
    assert!(!world.agent_try_ends_long(), "no span has begun");
    for operation in [smith_domain::Operation::Completion, smith_domain::Operation::Command] {
        world.agent_starts_long(operation, skein_lib::Duration::from_secs(30));
        world.settle();
        assert!(
            world
                .observations()
                .contains(&Observation::HostLong { operation, span: skein_lib::Duration::from_secs(30) })
        );
    }
    for _ in 0..2 {
        world.agent_ends_long();
        world.settle();
    }
    assert!(!world.agent_try_ends_long(), "the two begun operations are exhausted");
    assert_eq!(world.observations().iter().filter(|seen| **seen == Observation::HostLongDone).count(), 2);
}

fn last_window_turns(world: &World) -> Box<[Box<[u8]>]> {
    let saved: Vec<_> = world
        .observations()
        .iter()
        .filter_map(|observation| {
            if let Observation::HostTurn { opens_window, body, .. } = observation {
                Some((*opens_window, body.clone()))
            } else {
                None
            }
        })
        .collect();
    let last = saved.iter().rposition(|(flag, _)| *flag).expect("window boundary");
    saved[last..].iter().map(|(_, body)| body.clone()).collect::<Vec<_>>().into_boxed_slice()
}

#[test]
fn a_host_resumes_from_its_last_window_turn_alone() {
    use smith_domain_session::{llm, record};
    let charter: Box<[u8]> =
        include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin").as_slice().into();
    let mut turn = window_turn_fixture();
    let affinity = llm::Affinity { key: [0; 16], thread: 0 };
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start(charter.clone());
    world.settle();
    world.agent_admits();
    world.settle();
    for place in 1..=5 {
        turn.sequence = place;
        turn.kind = match place {
            1 => record::Kind::First(Box::new(record::Opener {
                affinity,
                account: None,
                backend: record::Backend::Direct,
            })),
            3 | 5 => record::Kind::Window(Box::new(record::Opener {
                affinity,
                account: None,
                backend: record::Backend::Direct,
            })),
            _ => record::Kind::Ordinary,
        };
        world.agent_tells_turn(place, None, &turn);
        world.settle();
        world.host_acknowledges(place);
        world.settle();
    }
    let history = last_window_turns(&world);
    let mut resumed = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    resumed.settle();
    resumed.send_start_with_turns(charter, Some(history));
    resumed.settle();
    assert!(
        resumed.observations().contains(&Observation::AgentHistory { turns: 1, place: 5 }),
        "{:?}",
        resumed.observations()
    );
}

#[path = "stories/limits.rs"]
mod limits;

fn account_fault_record(fault: smith_domain::run::Fault) -> smith_channel::Answer {
    let answer = smith_domain::run::Answer::Failed {
        failure: smith_domain::run::Failure::Model(fault),
        spent: smith_domain::run::Spend::ZERO,
        turns: 0,
    };
    smith_protocol_channel::answer_record(
        skein_lib::Time::from_nanos(20_000_000),
        answer,
        None,
        &CEILINGS,
        &smith_charter::CEILINGS,
    )
    .expect("bounded failure")
}

#[test]
fn a_limited_account_is_written_with_its_reset_at_the_answer() {
    let now = skein_lib::Time::from_nanos(20_000_000);
    for (reset_at, expected) in [
        (Some(skein_lib::Time::from_nanos(100_000_000)), Some(skein_lib::Duration::from_millis(80))),
        (Some(now), Some(skein_lib::Duration::ZERO)),
        (None, None),
    ] {
        let record = account_fault_record(smith_domain::run::Fault::Limited { account: 7, reset_at });
        let smith_channel::RunResult::Failed(failed) = record.result() else { panic!("failed answer") };
        let smith_channel::RunFailure::Model(model) = failed.reason() else { panic!("model failure") };
        let smith_channel::ModelFault::Limited(limited) = model.value() else { panic!("limited account") };
        let received = Some((limited.account(), *limited.reset()));
        assert_eq!(received, Some((7, expected)), "{reset_at:?}");
    }
}

#[test]
fn a_refused_account_is_written_with_its_code() {
    for (code, expected) in [(b"not-eligible".as_slice(), b"not-eligible".as_slice()), (b"".as_slice(), b"".as_slice())]
    {
        let record = account_fault_record(smith_domain::run::Fault::Refused { account: 7, code: code.into() });
        let smith_channel::RunResult::Failed(failed) = record.result() else { panic!("failed answer") };
        let smith_channel::RunFailure::Model(model) = failed.reason() else { panic!("model failure") };
        let smith_channel::ModelFault::Refused(refused) = model.value() else { panic!("refused account") };
        let received = Some((refused.account(), refused.code()));
        assert_eq!(received, Some((7, expected)), "code length {}", code.len());
    }
}

fn grant_shape(value: Option<smith_protocol_channel::GrantValue>) -> Option<(&'static str, usize, usize)> {
    use smith_protocol_channel::GrantValue;
    value.map(|value| match value {
        GrantValue::Bearer(bytes) => ("bearer", bytes.len(), 0),
        GrantValue::Key(bytes) => ("key", bytes.len(), 0),
        GrantValue::Program(name) => ("program", name.len(), 0),
        GrantValue::Withdrawn { account_id, bearer } => ("withdrawn", bearer.len(), account_id.len()),
    })
}

#[test]
fn each_grant_form_reaches_the_table_by_its_tag() {
    use smith_protocol_channel::GrantValue;
    // Sizes follow the fixture words below.
    const VALUE_BYTES: usize = 5;
    const SOURCE_BYTES: usize = 6;
    const ACCOUNT_BYTES: usize = 7;
    let rows = [
        (GrantValue::Bearer(Box::from(&b"value"[..])), Some(("bearer", VALUE_BYTES, 0))),
        (GrantValue::Key(Box::from(&b"value"[..])), Some(("key", VALUE_BYTES, 0))),
        (GrantValue::Program(Box::from(&b"source"[..])), Some(("program", SOURCE_BYTES, 0))),
        (
            GrantValue::Withdrawn { account_id: Box::from(&b"account"[..]), bearer: Box::from(&b"value"[..]) },
            Some(("withdrawn", VALUE_BYTES, ACCOUNT_BYTES)),
        ),
    ];
    let mut world = world_with_initial_grant();
    for (index, (value, expected)) in rows.into_iter().enumerate() {
        let generation = u64::try_from(index).expect("four rows").checked_add(5).expect("generation");
        world.host_refreshes_grant(3, generation, value);
        world.settle();
        assert_eq!(grant_shape(world.agent_grant_value(3, generation)), expected, "row {index}");
    }
}

#[test]
fn a_grant_values_debug_shows_no_bytes() {
    use smith_protocol_channel::GrantValue;
    const HIDDEN: &str = "[redacted grant value]";
    let rows = [
        (GrantValue::Bearer(Box::from(&b"secret"[..])), HIDDEN),
        (GrantValue::Key(Box::from(&b"secret"[..])), HIDDEN),
        (GrantValue::Program(Box::from(&b"source"[..])), HIDDEN),
        (GrantValue::Withdrawn { account_id: Box::from(&b"account"[..]), bearer: Box::from(&b"secret"[..]) }, HIDDEN),
    ];
    for (index, (value, expected)) in rows.into_iter().enumerate() {
        assert_eq!(format!("{value:?}"), expected, "row {index}");
    }
}

fn world_with_initial_grant() -> World {
    let mut world = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    world.settle();
    world.send_start_with_context(
        Box::from(&include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..]),
        smith_host_domain::SavedReply::TooLarge,
    );
    world.settle();
    world.agent_admits();
    world.settle();
    world
}
