use smith_domain_session::{self as session, llm, record};
use smith_session_world::recorded::{self, World, opening, scenario, transcript};

#[test]
fn a_domain_turn_round_trips_its_account_and_backend() {
    let first = scenario(30, 256);
    let affinity = llm::Affinity { key: [0x42; 16], thread: 7 };
    for (kind, sequence) in [
        (
            record::Kind::First(Box::new(record::Opener {
                affinity,
                account: Some(7),
                backend: record::Backend::Program { name: b"claude_code".as_slice().into() },
            })),
            1,
        ),
        (
            record::Kind::Window(Box::new(record::Opener {
                affinity,
                account: Some(7),
                backend: record::Backend::Program { name: b"claude_code".as_slice().into() },
            })),
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
    let mut world = World::new(SEED, FACTS);
    let mut start = opening(history, BUDGET);
    start.window_llm = previous;
    start.spec.window = window;
    start.spec.output = OUTPUT;
    start.spec.model = model.into();
    start.spec.account = Some(ACCOUNT);
    world.open(start);
    world
}

#[test]
fn each_compactions_window_turn_names_the_charters_llm() {
    struct Case {
        name: &'static str,
        world: World,
        expected: Window,
    }
    let rows = [
        Case { name: "threshold", world: threshold_compaction(), expected: expected_window(b"model") },
        Case { name: "model change", world: model_change_compaction(), expected: expected_window(NEXT_MODEL) },
    ];
    for row in rows {
        let mut world = row.world;
        summarise(&mut world);
        assert_eq!(window(world.turns.last().expect("window turn")), row.expected, "{}", row.name);
        world.close();
    }
}

fn round_trip(turn: &record::Turn) -> record::Turn {
    let mut configured = skein_lib::List::with_capacity(1);
    configured
        .push(smith_protocol_channel::Endpoint {
            name: b"provider".as_slice().into(),
            number: turn.endpoint.expect("named endpoint").0,
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
    endpoint: Option<llm::Endpoint>,
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
    let saved = same_window();
    compaction_world(
        Some(saved),
        Some(record::Llm {
            model: b"model".as_slice().into(),
            effort: None,
            window: MODEL_WINDOW,
            output: 128,
            prices: recorded::PRICES,
        }),
        MODEL_WINDOW,
        b"another-model",
    )
}

const ACCOUNT: u32 = 7;
const OTHER_ACCOUNT: u32 = ACCOUNT + 1;
const ENDPOINT: llm::Endpoint = llm::Endpoint(7);
const OTHER_ENDPOINT: llm::Endpoint = llm::Endpoint(8);
const DIALECT: u32 = 23;
const OTHER_DIALECT: u32 = DIALECT + 1;
const SEED: u64 = 33;
const FACTS: u32 = 256;
const MODEL_WINDOW: u32 = 65_536;
const OUTPUT: u32 = 512;
const BUDGET: u64 = 10_000;
const SUMMARY: &[u8] = b"portable summary";
const NEXT_MODEL: &[u8] = b"another-model";

struct CarryCase {
    name: &'static str,
    history: record::Transcript,
    expected: CarryOutcome,
}

#[derive(Debug, PartialEq, Eq)]
struct CarryOutcome {
    endpoint: llm::Endpoint,
    model: Box<[u8]>,
    choice: llm::ToolChoice,
    metadata: Vec<Box<[u8]>>,
    window: Window,
}

fn expected_carry() -> CarryOutcome {
    CarryOutcome {
        endpoint: ENDPOINT,
        model: NEXT_MODEL.into(),
        choice: llm::ToolChoice::None,
        metadata: vec![],
        window: expected_window(NEXT_MODEL),
    }
}

fn same_window() -> record::Transcript {
    let mut history = transcript(&scenario(SEED, FACTS));
    let affinity = llm::Affinity { key: [0; 16], thread: 0 };
    history.turns[0].kind = record::Kind::First(Box::new(record::Opener {
        affinity,
        account: Some(ACCOUNT),
        backend: record::Backend::Direct,
    }));
    for turn in &mut history.turns {
        for message in &mut turn.messages {
            for block in &mut message.content {
                match block {
                    llm::Block::Text { replay, .. }
                    | llm::Block::Refusal { replay, .. }
                    | llm::Block::ToolCall { replay, .. }
                    | llm::Block::ToolResult { result: llm::Returned::Text { replay, .. }, .. } => {
                        *replay = Some(llm::Replay { bytes: b"saved metadata".as_slice().into() });
                    }
                    llm::Block::Opaque { .. } | llm::Block::ToolResult { .. } => {}
                }
            }
        }
    }
    history
}

fn carried_windows() -> Vec<CarryCase> {
    let base = same_window();
    let mut endpoint = base.clone();
    endpoint.endpoint = Some(OTHER_ENDPOINT);
    let mut dialect = base.clone();
    dialect.dialect = OTHER_DIALECT;
    for turn in &mut endpoint.turns {
        turn.endpoint = Some(OTHER_ENDPOINT);
    }
    for turn in &mut dialect.turns {
        turn.dialect = OTHER_DIALECT;
    }
    let affinity = llm::Affinity { key: [0; 16], thread: 0 };
    let mut account = base.clone();
    account.turns[0].kind = record::Kind::First(Box::new(record::Opener {
        affinity,
        account: Some(OTHER_ACCOUNT),
        backend: record::Backend::Direct,
    }));
    let mut backend = base;
    backend.turns[0].kind = record::Kind::First(Box::new(record::Opener {
        affinity,
        account: Some(ACCOUNT),
        backend: record::Backend::Program { name: b"claude_code".as_slice().into() },
    }));
    vec![
        CarryCase { name: "endpoint", history: endpoint, expected: expected_carry() },
        CarryCase { name: "dialect", history: dialect, expected: expected_carry() },
        CarryCase { name: "account", history: account, expected: expected_carry() },
        CarryCase { name: "backend", history: backend, expected: expected_carry() },
    ]
}

fn expected_window(model: &[u8]) -> Window {
    Window {
        kind: record::Kind::Window(Box::new(record::Opener {
            affinity: llm::Affinity { key: [0; 16], thread: 0 },
            account: Some(ACCOUNT),
            backend: record::Backend::Direct,
        })),
        endpoint: Some(ENDPOINT),
        dialect: DIALECT,
        model: model.into(),
    }
}

fn window(turn: &record::Turn) -> Window {
    Window { kind: turn.kind.clone(), endpoint: turn.endpoint, dialect: turn.dialect, model: turn.model.clone() }
}

fn metadata(prompt: &llm::Prompt) -> Vec<Box<[u8]>> {
    prompt
        .messages
        .iter()
        .flat_map(|message| &message.content)
        .filter_map(|block| match block {
            llm::Block::Opaque { bytes } => Some(bytes.clone()),
            llm::Block::Text { replay, .. }
            | llm::Block::Refusal { replay, .. }
            | llm::Block::ToolCall { replay, .. }
            | llm::Block::ToolResult { result: llm::Returned::Text { replay, .. }, .. } => {
                replay.as_ref().map(|value| value.bytes.clone())
            }
            llm::Block::ToolResult { .. } => None,
        })
        .collect()
}

fn summarise(world: &mut World) {
    world.complete(
        Box::new([llm::Block::Text { text: SUMMARY.into(), replay: None }]),
        llm::Stop::EndTurn,
        llm::Usage::ZERO,
    );
}

#[test]
fn a_window_from_elsewhere_is_carried_over() {
    for row in carried_windows() {
        let mut world = compaction_world(Some(row.history), None, MODEL_WINDOW, NEXT_MODEL);
        let prompt = world.prompts.first().expect("carry-over compaction");
        let endpoint = prompt.endpoint;
        let model = prompt.model.clone();
        let choice = prompt.choice.clone();
        let metadata = metadata(prompt);
        summarise(&mut world);
        let result = CarryOutcome {
            endpoint,
            model,
            choice,
            metadata,
            window: window(world.turns.first().expect("window turn")),
        };
        assert_eq!(result, row.expected, "{}", row.name);
        world.close();
    }
}

#[test]
fn a_resume_after_a_carry_over_does_not_compact_again() {
    let row = carried_windows().remove(0);
    let mut carried = compaction_world(Some(row.history), None, MODEL_WINDOW, NEXT_MODEL);
    summarise(&mut carried);
    let last = carried.turns.first().expect("window turn").clone();
    carried.close();
    let history = record::Transcript {
        version: record::VERSION,
        endpoint: last.endpoint,
        dialect: last.dialect,
        turns: Box::new([last]),
    };
    let mut resumed = compaction_world(Some(history), None, MODEL_WINDOW, NEXT_MODEL);
    assert_eq!(resumed.prompts.first().expect("resumed prompt").choice, llm::ToolChoice::Auto);
    resumed.close();
}

#[test]
fn a_same_window_with_another_model_compacts_on_its_own_model() {
    let mut world = model_change_compaction();
    assert_eq!(world.prompts.first().expect("old model compaction").model.as_ref(), b"model");
    summarise(&mut world);
    assert_eq!(window(world.turns.first().expect("new model window")), expected_window(NEXT_MODEL));
    world.close();
}
