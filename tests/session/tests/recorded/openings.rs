use smith_domain_session::{self as session, llm, record};
use smith_session_world::recorded::{self, World, opening, scenario, transcript};

#[test]
fn a_domain_turn_round_trips_its_account_and_backend() {
    let first = scenario(30, 256);
    let affinity = llm::Affinity { key: [0x42; 16], thread: 7 };
    for (kind, sequence) in [
        (
            record::Kind::First {
                affinity,
                account: Some(7),
                backend: record::Backend::Program { name: b"claude_code".as_slice().into() },
            },
            1,
        ),
        (
            record::Kind::Window {
                affinity,
                account: Some(7),
                backend: record::Backend::Program { name: b"claude_code".as_slice().into() },
            },
            3,
        ),
    ] {
        let mut turn = first.turns[0].clone();
        turn.kind = kind;
        turn.sequence = sequence;
        let decoded = round_trip(&turn);
        assert_eq!(decoded, turn, "sequence {sequence}");
    }
}

fn compaction_world(
    history: Option<record::Transcript>,
    previous: Option<record::Llm>,
    window: u32,
    model: &[u8],
) -> World {
    let mut world = World::new(32, 256);
    let mut start = opening(history, 10_000);
    start.window_llm = previous;
    start.spec.window = window;
    start.spec.output = 512;
    start.spec.model = model.into();
    start.spec.account = Some(7);
    world.open(start);
    world
}

#[test]
fn each_compactions_window_turn_names_the_charters_llm() {
    let threshold = threshold_compaction();
    let model_change = model_change_compaction();
    for (name, mut world, model) in
        [("threshold", threshold, b"model".as_slice()), ("model change", model_change, b"another-model".as_slice())]
    {
        world.complete(
            Box::new([llm::Block::Text { text: b"summary".as_slice().into(), replay: None }]),
            llm::Stop::EndTurn,
            llm::Usage::ZERO,
        );
        let turn = world.turns.last().expect("window turn");
        let seen = Window {
            kind: turn.kind.clone(),
            endpoint: turn.endpoint,
            dialect: turn.dialect,
            model: turn.model.clone(),
        };
        let expected = Window {
            kind: record::Kind::Window {
                affinity: llm::Affinity { key: [0; 16], thread: 0 },
                account: Some(7),
                backend: record::Backend::Direct,
            },
            endpoint: llm::Endpoint(7),
            dialect: 23,
            model: model.into(),
        };
        assert_eq!(seen, expected, "{name}");
        world.close();
    }
}

fn round_trip(turn: &record::Turn) -> record::Turn {
    let mut configured = skein_lib::List::with_capacity(1);
    configured
        .push(smith_protocol_channel::Endpoint {
            name: b"provider".as_slice().into(),
            number: turn.endpoint.0,
            dialect: turn.dialect,
        })
        .expect("endpoint");
    let endpoints = smith_protocol_channel::Endpoints::new(configured);
    let bytes =
        smith_protocol_channel::encode_turn(turn, &smith_transcript::CEILINGS, &endpoints).expect("encode turn");
    smith_protocol_channel::decode_turn(&bytes, turn.sequence, &smith_transcript::CEILINGS, &endpoints)
        .expect("decode turn")
}

#[derive(Debug, PartialEq, Eq)]
struct Window {
    kind: record::Kind,
    endpoint: llm::Endpoint,
    dialect: u32,
    model: Box<[u8]>,
}

fn threshold_compaction() -> World {
    let mut threshold = compaction_world(None, None, 2_000, b"model");
    threshold.complete(
        Box::new([llm::Block::Text { text: b"answer".as_slice().into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage { input_tokens: Some(1_300), output_tokens: Some(300), ..llm::Usage::ZERO },
    );
    threshold.step(session::Event::Continue {
        session: threshold.session.expect("opened"),
        content: b"go on".as_slice().into(),
    });
    threshold
}

fn model_change_compaction() -> World {
    let saved = scenario(31, 256);
    compaction_world(
        Some(transcript(&saved)),
        Some(record::Llm {
            model: b"model".as_slice().into(),
            effort: None,
            window: 65_536,
            output: 128,
            prices: recorded::PRICES,
        }),
        65_536,
        b"another-model",
    )
}
