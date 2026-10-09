//! Opening entry points (protocol/channel.md, section 2).
use skein_channel::{Lower, StreamMode};
use skein_lib::{Queue, stream};

use crate::{Component, Limits, OpenEvent};

#[test]
fn the_agent_demands_the_open_before_the_domain_hears_anything() {
    let limits = Limits {
        bodies: smith_channel::CEILINGS,
        charter: smith_charter::CEILINGS,
        transcript: smith_transcript::CEILINGS,
        endpoints: 0,
        calls: 0,
        turns: 0,
        fact_reserve_frames: 1,
        fact_reserve_bytes: 128,
        grants: 0,
        channel: skein_channel::Limits {
            chunk: 8,
            credential: 0,
            skip: 8,
            output_bytes: u32::MAX,
            output_frames: 2,
            kinds: 18,
        },
    };
    let mut component =
        Component::new(&limits, StreamMode::Two, crate::Endpoints::new(skein_lib::List::with_capacity(0)))
            .expect("checked schema");
    let mut to_service = Queue::<OpenEvent>::with_capacity(2);
    let mut below = Queue::<Lower>::with_capacity(8);
    component.fire(&mut to_service, &mut below);
    assert!(to_service.is_empty());
    match below.pop() {
        Some(Lower::Read(stream::Down::Demand { read: stream::Read::Fill(1), room: 0 })) => {}
        other => panic!("expected Open read: {other:?}"),
    }
}

fn saved_turn(place: u32, dialect: &[u8]) -> Box<[u8]> {
    use smith_transcript as wire;
    let limits = wire::CEILINGS;
    let result = wire::ToolResult::new(
        &limits,
        wire::ToolResultParts {
            id: Box::from(*b"call-1"),
            returned: wire::Returned::Invalid(
                wire::Invalid::new(&limits, wire::InvalidParts { problem: wire::CallProblem::UnknownTool })
                    .expect("bounded invalid result"),
            ),
        },
    )
    .expect("bounded result");
    let mut blocks = skein_lib::List::with_capacity(2);
    blocks
        .push(wire::Block::Call(
            wire::Call::new(
                &limits,
                wire::CallParts {
                    id: Box::from(*b"call-1"),
                    name: Box::from(*b"missing"),
                    input: Box::from(*b"{}"),
                    replay: None,
                },
            )
            .expect("bounded call"),
        ))
        .expect("first block");
    blocks.push(wire::Block::Result(result)).expect("second block");
    let mut messages = skein_lib::List::with_capacity(1);
    messages
        .push(
            wire::Message::new(&limits, wire::MessageParts { role: wire::Role::Assistant, blocks })
                .expect("bounded message"),
        )
        .expect("first message");
    let turn = wire::Turn::new(
        &limits,
        wire::TurnParts {
            kind: wire::TurnKind::Ordinary,
            affinity: if place == 1 {
                Some(
                    wire::Affinity::new(&limits, wire::AffinityParts { key: [0; 16], thread: 0 })
                        .expect("fixture affinity"),
                )
            } else {
                None
            },
            endpoint: Box::default(),
            dialect: Box::from(dialect),
            place,
            usage: wire::Usage::new(
                &limits,
                wire::UsageParts {
                    input: Some(2),
                    output: Some(3),
                    cache_read: Some(4),
                    cache_write: Some(5),
                    reasoning: None,
                },
            )
            .expect("bounded usage"),
            spent: 7,
            messages,
        },
    )
    .expect("bounded turn");
    let mut writer = skein_lib::Writer::new(usize::try_from(turn.measure()).expect("bounded turn length"));
    turn.encode(&mut writer).expect("measured turn");
    writer.finish()
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "test reports the observed value")]
fn transcript_decodes_history_with_one_invalid_call_problem() {
    use smith_domain_session::{llm, record};
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0, account: 5 }).expect("one endpoint");
    let endpoints = crate::Endpoints::new(configured);
    let bytes = Box::from([saved_turn(1, b"00000000")]);
    let transcript = crate::decode_transcript(&bytes, &smith_transcript::CEILINGS, &endpoints)
        .expect("valid transcript")
        .expect("saved history");
    assert_eq!(transcript.version, record::VERSION);
    assert_eq!(transcript.endpoint, llm::Endpoint(3));
    assert_eq!(transcript.turns[0].sequence, 1);
    assert_eq!(transcript.turns[0].spent, 7);
    assert_eq!(transcript.turns[0].usage.cache_read_tokens, Some(4));
    match &transcript.turns[0].messages[0].content[0] {
        llm::Block::ToolCall { call: llm::Decoded::Historical, .. } => {}
        other => panic!("expected historical call: {other:?}"),
    }
    match &transcript.turns[0].messages[0].content[1] {
        llm::Block::ToolResult { result: llm::Returned::Invalid { problem: llm::Problem::UnknownTool }, .. } => {}
        other => panic!("expected single invalid problem: {other:?}"),
    }
}

#[test]
fn transcript_refuses_a_gap_and_a_different_dialect() {
    use smith_domain_session::record;
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0, account: 5 }).expect("one endpoint");
    let endpoints = crate::Endpoints::new(configured);
    assert_eq!(
        crate::decode_transcript(&[saved_turn(2, b"00000000")], &smith_transcript::CEILINGS, &endpoints),
        Err(record::Refusal::Malformed)
    );
    assert_eq!(
        crate::decode_transcript(&[saved_turn(1, b"other")], &smith_transcript::CEILINGS, &endpoints),
        Err(record::Refusal::Dialect)
    );
}

#[test]
fn a_live_turn_decodes_at_its_number_without_the_saved_prefix() {
    use smith_domain_session::record;
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0, account: 5 }).expect("endpoint");
    let endpoints = crate::Endpoints::new(configured);
    let bytes = saved_turn(4, b"00000000");
    let turn = crate::decode_turn(&bytes, 4, &smith_transcript::CEILINGS, &endpoints).expect("live turn");
    assert_eq!(turn.sequence, 4);
    assert_eq!(turn.spent, 7);
    assert_eq!(crate::decode_turn(&bytes, 3, &smith_transcript::CEILINGS, &endpoints), Err(record::Refusal::Malformed));
}

#[test]
fn optional_usage_round_trips_without_turning_absence_into_zero() {
    use smith_domain_session::llm;
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0, account: 5 }).expect("endpoint");
    let endpoints = crate::Endpoints::new(configured);
    let bytes = saved_turn(1, b"00000000");
    let mut turn = crate::decode_turn(&bytes, 1, &smith_transcript::CEILINGS, &endpoints).expect("saved turn");
    for usage in [
        llm::Usage::NONE,
        llm::Usage::ZERO,
        llm::Usage {
            input_tokens: None,
            output_tokens: Some(0),
            cache_read_tokens: Some(9),
            cache_write_tokens: None,
            reasoning_tokens: Some(7),
        },
    ] {
        turn.usage = usage;
        let encoded = crate::encode_turn(&turn, &smith_transcript::CEILINGS, &endpoints).expect("turn encodes");
        let decoded = crate::decode_turn(&encoded, 1, &smith_transcript::CEILINGS, &endpoints).expect("turn decodes");
        assert_eq!(decoded.usage, usage);
    }
}

#[test]
fn resolved_failures_round_trip_in_version_three_and_refuse_unnormalized_paths() {
    use smith_domain_session::{llm, record};
    use smith_domain_tools::{Name, Outcome, Resolved};
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0, account: 5 }).expect("endpoint");
    let endpoints = crate::Endpoints::new(configured);
    let mut turn = crate::decode_turn(&saved_turn(1, b"00000000"), 1, &smith_transcript::CEILINGS, &endpoints)
        .expect("saved turn");
    let cases = [
        Outcome::NotFound { path: Resolved::Working { path: "src/café.rs".as_bytes().into() } },
        Outcome::NotDirectory { path: Resolved::Working { path: Box::new([]) } },
        Outcome::ReadOnly {
            path: Resolved::Other {
                directory: Name::new(b"docs".as_slice().into()).expect("name"),
                path: b"guide.md".as_slice().into(),
            },
        },
        Outcome::Outside { path: Resolved::Outside { path: b"other/file".as_slice().into() } },
    ];
    for outcome in cases {
        turn.messages = Box::new([llm::Message {
            role: llm::Role::User,
            content: Box::new([llm::Block::ToolResult {
                id: b"path-result".as_slice().into(),
                result: llm::Returned::Owned { outcome },
            }]),
        }]);
        let bytes = crate::encode_turn(&turn, &smith_transcript::CEILINGS, &endpoints).expect("bounded turn");
        assert_eq!(&bytes[..2], &[0, 3]);
        let decoded = crate::decode_turn(&bytes, 1, &smith_transcript::CEILINGS, &endpoints).expect("saved failure");
        assert_eq!(decoded.messages, turn.messages, "typed descriptors survive durable translation");
    }
    for path in [b"../file".as_slice(), b"a/./b", b"a//b", b"/file", b"file/", b"a\0b"] {
        turn.messages = Box::new([llm::Message {
            role: llm::Role::User,
            content: Box::new([llm::Block::ToolResult {
                id: b"path-result".as_slice().into(),
                result: llm::Returned::Owned {
                    outcome: Outcome::NotFound { path: Resolved::Working { path: path.into() } },
                },
            }]),
        }]);
        let bytes = crate::encode_turn(&turn, &smith_transcript::CEILINGS, &endpoints).expect("wire byte bound");
        assert_eq!(
            crate::decode_turn(&bytes, 1, &smith_transcript::CEILINGS, &endpoints),
            Err(record::Refusal::Malformed),
            "{path:?}"
        );
    }
}

fn with_recorded_affinity(bytes: &[u8], affinity: Option<smith_transcript::Affinity>) -> Box<[u8]> {
    let limits = smith_transcript::CEILINGS;
    let mut parts =
        smith_transcript::Turn::decode(&limits, &mut skein_lib::Reader::new(bytes)).expect("fixture turn").into_parts();
    parts.affinity = affinity;
    let turn = smith_transcript::Turn::new(&limits, parts).expect("bounded fixture turn");
    let mut writer = skein_lib::Writer::new(usize::try_from(turn.measure()).expect("fixture length"));
    turn.encode(&mut writer).expect("fixture room");
    writer.finish()
}

#[test]
fn a_transcript_records_affinity_only_in_its_opening_turn() {
    use smith_domain_session::record;
    let limits = smith_transcript::CEILINGS;
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0, account: 5 }).expect("endpoint");
    let endpoints = crate::Endpoints::new(configured);
    let affinity =
        smith_transcript::Affinity::new(&limits, smith_transcript::AffinityParts { key: [0x42; 16], thread: 7 })
            .expect("fixed affinity");
    let first = with_recorded_affinity(&saved_turn(1, b"00000000"), Some(affinity));
    let ordinary = saved_turn(2, b"00000000");
    let decoded = crate::decode_transcript(&[first.clone(), ordinary.clone()], &limits, &endpoints)
        .expect("opening and ordinary")
        .expect("transcript");
    assert_eq!(
        decoded.turns[0].kind,
        record::Kind::First { affinity: smith_domain_session::llm::Affinity { key: [0x42; 16], thread: 7 } }
    );
    assert_eq!(decoded.turns[1].kind, record::Kind::Ordinary);
    assert_eq!(
        crate::decode_transcript(&[with_recorded_affinity(&first, None)], &limits, &endpoints),
        Err(record::Refusal::Malformed)
    );
    let extra =
        smith_transcript::Affinity::new(&limits, smith_transcript::AffinityParts { key: [0x42; 16], thread: 7 })
            .expect("fixed affinity");
    assert_eq!(
        crate::decode_transcript(&[first, with_recorded_affinity(&ordinary, Some(extra))], &limits, &endpoints),
        Err(record::Refusal::Malformed)
    );
}

#[test]
fn a_window_suffix_round_trips_its_noninitial_place_and_rejects_a_second_window() {
    use smith_domain_session::{llm, record};
    let limits = smith_transcript::CEILINGS;
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0, account: 5 }).expect("endpoint");
    let endpoints = crate::Endpoints::new(configured);
    let window = record::Turn {
        version: record::VERSION,
        kind: record::Kind::Window { affinity: llm::Affinity { key: [0x25; 16], thread: 0 } },
        endpoint: llm::Endpoint(3),
        dialect: 0,
        sequence: 8,
        usage: llm::Usage::ZERO,
        spent: 9,
        messages: Box::new([llm::Message {
            role: llm::Role::User,
            content: Box::new([llm::Block::Text { text: b"task and summary".as_slice().into(), replay: None }]),
        }]),
    };
    let first = crate::encode_turn(&window, &limits, &endpoints).expect("window turn");
    let decoded = crate::decode_transcript(&[first.clone(), saved_turn(9, b"00000000")], &limits, &endpoints)
        .expect("window suffix")
        .expect("transcript");
    assert_eq!(decoded.turns[0], window);
    assert!(decoded.turns[0].kind.opens_window());
    assert!(!decoded.turns[1].kind.opens_window());
    let mut second = window.clone();
    second.sequence = 9;
    let second = crate::encode_turn(&second, &limits, &endpoints).expect("second window bytes");
    assert_eq!(crate::decode_transcript(&[first, second], &limits, &endpoints), Err(record::Refusal::Malformed));
}
