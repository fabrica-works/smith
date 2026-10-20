use skein_fake_checkout::Checkout;
use skein_fake_llm_domain::api::{Finish, Line, Script, Turn};
use skein_lib::{Time, Token};
use smith_agent_world::{Job, Settings, World, scripted_charter};
use smith_domain::{
    run,
    session::{llm, record},
};

const ITERATIONS: u32 = 10_000;
const DATA_BYTES: usize = 3_700;

fn history(input: u64) -> smith_domain::Transcript {
    let content = |text: &[u8]| Box::new([llm::Block::Text { text: text.into(), replay: None }]);
    smith_domain::Transcript {
        version: record::VERSION,
        endpoint: Some(llm::Endpoint(0)),
        dialect: 1,
        turns: Box::new([record::Turn {
            model: b"fake-1".as_slice().into(),
            effort: None,
            version: record::VERSION,
            kind: record::Kind::First(Box::new(record::Opener {
                affinity: llm::Affinity { key: [0; 16], thread: 0 },
                account: Some(0),
                backend: record::Backend::Direct,
            })),
            endpoint: Some(llm::Endpoint(0)),
            dialect: 1,
            sequence: 1,
            usage: llm::Usage { input_tokens: Some(input), ..llm::Usage::ZERO },
            spent: 0,
            jobs: 0,
            messages: Box::new([
                llm::Message { role: llm::Role::User, content: content(&[b't'; 200]) },
                llm::Message { role: llm::Role::Assistant, content: content(b"saved answer") },
            ]),
        }]),
    }
}

fn world(refused: bool, immediate: bool) -> World {
    world_with_history(refused, immediate, history(if immediate { 6_000 } else { 0 }))
}

fn world_with_history(refused: bool, immediate: bool, saved: smith_domain::Transcript) -> World {
    let mut settings = Settings::calm(4801);
    settings.window = 6_144;
    settings.job = Job::Reporting;
    settings.resume = true;
    let mut charter = scripted_charter(&settings);
    charter.llm.output = 256;
    charter.models = Box::new([]);
    let mut disk = Checkout::new();
    disk.mkdir(b"work");
    disk.write(b"work/data.txt", &[b'd'; DATA_BYTES]);
    let workspace = run::Workspace {
        directories: Box::new([run::Directory {
            host: b"/host".as_slice().into(),
            name: b"work".as_slice().into(),
            root: Token::new(disk.root(b"work")),
            writable: false,
            git: false,
            conflicts: Box::new([]),
        }]),
    };
    let finish = Turn {
        finish: Finish::ToolCalls,
        tokens: 32,
        lines: Box::new([Line::Call {
            name: b"finish".as_slice().into(),
            arguments: br#"{"report":"finished","source":"compaction"}"#.as_slice().into(),
        }]),
    };
    let read = Turn {
        finish: Finish::ToolCalls,
        tokens: 32,
        lines: Box::new([Line::Call {
            name: b"read".as_slice().into(),
            arguments: br#"{"path":"data.txt"}"#.as_slice().into(),
        }]),
    };
    let summary = Turn {
        finish: if refused { Finish::ContentFilter } else { Finish::Stop },
        tokens: 40,
        lines: Box::new([Line::Text { text: b"Keep working.".as_slice().into() }]),
    };
    let turns = if immediate {
        Box::new([finish, summary]) as Box<[Turn]>
    } else {
        // The first read is cut to what fits below the line (session.md, 8.1); the second finds no room left, so
        // its result crosses the line and the next turn is the compaction.
        Box::new([finish, read.clone(), read, summary]) as Box<[Turn]>
    };
    World::with_workspace_scripts_charter(
        settings,
        Some(saved),
        Some(workspace),
        disk,
        Box::new([Script { cue: b"@report".as_slice().into(), turns }]),
        charter,
    )
}

#[test]
fn a_compaction_counts_as_a_turn_and_as_spend_and_keeps_the_system_text() {
    let mut world = world(false, false);
    world.run(ITERATIONS);
    let run::Answer::Accepted { turns, .. } = world.answer() else { panic!("accepted: {:?}", world.answer()) };
    let opens: Vec<bool> = world.turns().iter().map(|turn| turn.kind.opens_window()).collect();
    let window = opens.iter().skip(1).position(|opens| *opens).expect("window turn after first") + 1;
    assert_eq!(*turns, u32::try_from(opens.len()).expect("few turns"));
    let spent: Vec<u64> = world.turn_metadata().iter().map(|(_, _, spent)| spent.units).collect();
    assert!(spent[window] > spent[window - 1], "the compaction is charged");
    let systems: Vec<&[u8]> = world.prompts().iter().map(|query| &*query.system).collect();
    assert!(systems.windows(2).all(|pair| pair[0] == pair[1]), "the system text never changes");
}

#[test]
fn a_message_held_back_by_a_compaction_is_read_by_the_window_turn() {
    let mut world = world(false, true);
    world.message_at(
        Time::ZERO.saturating_add(skein_lib::Duration::from_millis(1)),
        Token::new(9),
        b"person".as_slice().into(),
        b"go on".as_slice().into(),
    );
    world.run(ITERATIONS);
    let window =
        world.turns().iter().position(|turn| matches!(turn.kind, record::Kind::Window(_))).expect("compaction turn");
    assert!(world.turn_metadata()[..window].iter().all(|(_, read, _)| read.is_none()), "held until the window");
    assert_eq!(world.turn_metadata()[window].1, Some(Token::new(9)));
}

#[test]
fn a_refused_compaction_fails_the_run_as_refused_its_message_unread() {
    let mut world = world(true, true);
    world.message_at(
        Time::ZERO.saturating_add(skein_lib::Duration::from_millis(1)),
        Token::new(9),
        b"person".as_slice().into(),
        b"unread".as_slice().into(),
    );
    world.run(ITERATIONS);
    assert!(matches!(world.answer(), run::Answer::Failed { failure: run::Failure::Model(run::Fault::Declined), .. }));
    assert!(world.turns().is_empty(), "no summary turn is told");
    assert!(!world.messages_seen().iter().any(|(_, seen)| matches!(seen,
        smith_agent_world::messages_referee::Seen::Turn { read: Some(token), .. } if *token == Token::new(9))));
}

#[test]
fn a_window_from_an_unnamed_endpoint_is_carried_over() {
    let mut world = world_with_history(false, true, unnamed_window());
    world.run(ITERATIONS);
    let first = world.prompts().first().expect("carry-over compaction");
    assert_eq!(first.model.as_ref(), b"fake-1");
    assert_eq!(first.choice, skein_fake_llm_domain::api::ToolChoice::None);
    let metadata: Vec<_> = first
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .filter_map(|block| match block {
            skein_fake_llm_domain::api::Part::Opaque { bytes } => Some(bytes.as_ref()),
            skein_fake_llm_domain::api::Part::Text { .. }
            | skein_fake_llm_domain::api::Part::ToolCall { .. }
            | skein_fake_llm_domain::api::Part::ToolOutput { .. } => None,
        })
        .collect();
    assert_eq!(metadata, Vec::<&[u8]>::new());
    let window = world.turns().first().expect("carried window");
    assert_eq!(
        window.kind,
        record::Kind::Window(Box::new(record::Opener {
            affinity: llm::Affinity { key: [0; 16], thread: 0 },
            account: Some(0),
            backend: record::Backend::Direct,
        }))
    );
    assert_eq!(window.endpoint, Some(llm::Endpoint(0)));
    assert!(matches!(world.answer(), run::Answer::Accepted { .. }));
}

fn unnamed_window() -> smith_domain::Transcript {
    let mut saved = history(0);
    saved.endpoint = None;
    saved.turns[0].endpoint = None;
    saved.turns[0].model = b"removed-model".as_slice().into();
    saved.turns[0].messages[1].content = Box::new([
        llm::Block::Opaque { bytes: b"foreign provider block".as_slice().into() },
        llm::Block::Text {
            text: b"saved answer".as_slice().into(),
            replay: Some(llm::Replay { bytes: b"foreign provider replay".as_slice().into() }),
        },
    ]);
    saved
}
