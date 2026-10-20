//! Carried windows cross the shipped channel, service and program boundary (programs.md 5.8).
#[global_allocator]
static HEAP: skein_heap::Counting = skein_heap::Counting;
use skein_fake_llm_domain::api;
use smith_agent_process_world::{
    World, program,
    program_world::{self, Case},
};
use smith_channel::RunResult;
use smith_fake_claude::{Step, Story, Turn};

fn charter() -> Box<[u8]> {
    let mut parts = smith_charter::Charter::decode(
        &smith_charter::CEILINGS,
        &mut skein_lib::Reader::new(&smith_agent_process_world::charter()),
    )
    .expect("charter")
    .into_parts();
    parts.resume = true;
    let mut budget = parts.budget.into_parts();
    budget.turns = 4;
    budget.spend = 20;
    parts.budget = smith_charter::Budget::new(&smith_charter::CEILINGS, budget).expect("budget");
    let charter = smith_charter::Charter::new(&smith_charter::CEILINGS, parts).expect("charter");
    let mut writer = skein_lib::Writer::new(charter.measure() as usize);
    charter.encode(&mut writer).expect("charter");
    writer.finish()
}

fn finish() -> api::Turn {
    api::Turn {
        lines: Box::new([api::Line::Call {
            name: b"finish".as_slice().into(),
            arguments: br#"{"form":"report","text":"done","fields":{}}"#.as_slice().into(),
        }]),
        finish: api::Finish::ToolCalls,
        tokens: 1,
    }
}

fn story() -> Story {
    Story {
        summary: "portable summary".into(),
        turns: vec![Turn {
            steps: vec![Step::Call {
                tool: "finish".into(),
                input: serde_json::json!({"form":"report","text":"done","fields":{}}),
            }],
            ..Turn::default()
        }],
        ..Story::default()
    }
}

fn direct() -> World {
    let mut world = World::new(7, &charter());
    world.script(Box::new([finish()]));
    assert!(world.settle());
    assert!(matches!(world.answer().expect("answer").result(), RunResult::Accepted(_)));
    world
}

#[test]
fn a_conversation_is_carried_over_from_the_direct_backend() {
    let old = direct();
    let turns = program_world::from_frames(old.observed());
    assert!(!turns.is_empty());
    let run = program_world::run_with_window(&story(), &charter(), &turns);
    assert!(
        matches!(run.answer().expect("answer").result(), RunResult::Accepted(_)),
        "{:?} {}",
        run.answer(),
        String::from_utf8_lossy(&run.errors)
    );
    let print: Vec<_> = run.invocations.iter().filter(|invocation| invocation.arguments[0] == "-p").collect();
    assert_eq!(print.len(), 2, "summary then ordinary window");
    assert!(!print[0].arguments.contains(&"--mcp-config".into()));
    assert!(print[1].arguments.contains(&"--mcp-config".into()));
    let text = run.received[0]["message"]["content"].as_str().expect("summary input");
    assert!(text.contains("assistant:\nfinish:"));
    assert!(
        run.received[1]["message"]["content"]
            .as_array()
            .expect("window blocks")
            .iter()
            .any(|block| block["text"] == "portable summary")
    );
}

#[test]
fn a_conversation_is_carried_over_to_the_direct_backend() {
    let old = program_world::run(Case::Turns, &story(), Some(b"claude.ai"), &charter(), &program(b"source"), None);
    assert!(matches!(old.answer().expect("answer").result(), RunResult::Accepted(_)));
    let turns = program_world::from_frames(&old.frames);
    assert!(!turns.is_empty());
    let mut world = World::new(7, &charter());
    world.restore(turns);
    world.script(Box::new([
        finish(),
        api::Turn {
            lines: Box::new([api::Line::Text { text: b"direct summary".as_slice().into() }]),
            finish: api::Finish::Stop,
            tokens: 1,
        },
    ]));
    assert!(world.settle());
    assert!(matches!(world.answer().expect("answer").result(), RunResult::Accepted(_)), "{:?}", world.answer());
    assert_eq!(world.peer_queries().len(), 2, "summary precedes ordinary completion");
}

#[test]
fn a_compaction_runs_its_own_program() {
    let old = program_world::run(Case::Turns, &story(), Some(b"claude.ai"), &charter(), &program(b"source"), None);
    let turns = program_world::from_frames(&old.frames);
    let run = program_world::run_with_window(&story(), &charter(), &turns);
    assert!(
        matches!(run.answer().expect("answer").result(), RunResult::Accepted(_)),
        "{:?} {}",
        run.answer(),
        String::from_utf8_lossy(&run.errors)
    );
    let print: Vec<_> = run.invocations.iter().filter(|invocation| invocation.arguments[0] == "-p").collect();
    assert_eq!(print.len(), 2);
    let ids: Vec<_> = print
        .iter()
        .map(|invocation| {
            invocation.arguments.windows(2).find(|pair| pair[0] == "--session-id").expect("new session")[1].clone()
        })
        .collect();
    assert_ne!(ids[0], ids[1]);
    assert!(run.run_directories.is_empty());
    assert!(run.exits.iter().all(|exit| *exit == skein_io::kernel::Exit::Code(0)), "{:?}", run.exits);
}
