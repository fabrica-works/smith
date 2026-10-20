#[global_allocator]
static HEAP: skein_heap::Counting = skein_heap::Counting;

use skein_lib::{Reader, Writer};
use smith_agent_process_world::{
    charter, program,
    program_world::{self, Case},
};
use smith_fake_claude::{End, Seen, Step, Story, Turn};

#[test]
fn a_program_wait_holds_until_the_host_message_reaches_its_reply() {
    let mut parts = smith_charter::Charter::decode(&smith_charter::CEILINGS, &mut Reader::new(&charter()))
        .expect("charter")
        .into_parts();
    parts.waiting = skein_lib::Duration::from_secs(1);
    let mut tools = parts.tools.into_parts();
    tools.wait = true;
    parts.tools = smith_charter::Tools::new(&smith_charter::CEILINGS, tools).expect("wait granted");
    let mut budget = parts.budget.into_parts();
    budget.turns = 8;
    budget.spend = 20;
    parts.budget = smith_charter::Budget::new(&smith_charter::CEILINGS, budget).expect("budget");
    let charter = smith_charter::Charter::new(&smith_charter::CEILINGS, parts).expect("wait charter");
    let mut body = Writer::new(charter.measure() as usize);
    charter.encode(&mut body).expect("charter bytes");
    let story = Story {
        turns: vec![Turn {
            steps: vec![
                Step::CallChecked {
                    tool: "wait".into(),
                    input: serde_json::json!({}),
                    contains: "The next instruction".into(),
                },
                Step::Call {
                    tool: "finish".into(),
                    input: serde_json::json!({"form":"report", "text":"done", "fields":{}}),
                },
            ],
            end: End::default(),
        }],
        ..Story::default()
    };
    let run = program_world::run(Case::Waiting, &story, Some(b"claude.ai"), &body.finish(), &program(b"source"), None);
    assert!(
        matches!(run.answer().expect("answer").result(), smith_channel::RunResult::Accepted(_)),
        "answer={:?}; frames={:?}; errors={}",
        run.answer(),
        run.frames.iter().map(|frame| frame.kind).collect::<Vec<_>>(),
        String::from_utf8_lossy(&run.errors)
    );
    assert!(run.seen.iter().any(|seen| matches!(seen, Seen::Called { tool, error: false } if tool == "wait")));
    let invocation = run
        .invocations
        .iter()
        .find(|invocation| invocation.arguments.first().is_some_and(|argument| argument == "-p"))
        .expect("program invocation");
    assert!(invocation.environment.iter().any(|name| name == "MCP_TOOL_TIMEOUT"));
}
