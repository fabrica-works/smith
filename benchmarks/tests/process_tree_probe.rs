//! Process-tree controls exercise the harness codec, not a provider or product run.

use smith_bench::agent::smith::events::Observer;
use smith_bench::{EventCheck, Measure};
use smith_events::{Capture, Event, Record};
use std::path::PathBuf;

pub mod common;

const COMMAND: &str = "sh -c 'sleep 600 & wait'";

#[test]
fn nested_process_task_freezes_the_command_profile_and_tree_criterion() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&root.join("tasks/probes/process-tree/task.toml"), &root.join("../docs/design"))
        .expect("frozen seed and task");
    assert_eq!(task.version, 2);
    assert!(task.prompt.contains(COMMAND));
    assert!(task.deciding.iter().any(|name| name == "tree-ended"));
    assert!(task.event.contains(&EventCheck::ToolTimeout {
        requested_seconds: 1,
        deadline_ms: 1000,
        clamped: false,
        grace_ms: 1000,
        command: Some(COMMAND.into()),
    }));
    let suite_file = root.join("suites/probes.toml");
    let mut suite = smith_bench::read_suite(&suite_file).expect("suite");
    suite.tasks = vec!["probes/process-tree".into()];
    suite.agents.retain(|agent| agent.tasks.iter().any(|name| name == "probes/process-tree"));
    suite.agents[0].tasks.clone_from(&suite.tasks);
    assert_eq!(suite.agents[0].config, "everything");
    let catalogue = smith_bench::catalogue(&root.join("tasks"), &root.join("../docs/design")).expect("catalogue");
    let tiers = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("tiers");
    assert_eq!(
        smith_bench::validate_suite(&suite_file, &suite, &catalogue, &root.join("agents"), &tiers)
            .expect("selected task and arm")
            .len(),
        1
    );
}

#[test]
fn nested_command_requires_retained_input_and_its_later_exact_terminal() {
    let check = EventCheck::ToolTimeout {
        requested_seconds: 1,
        deadline_ms: 1000,
        clamped: false,
        grace_ms: 250,
        command: Some(COMMAND.into()),
    };
    let mut values = common::events::balanced();
    let mut start =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_started_everything.jsonl"));
    let mut end =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
    if let Event::ToolStarted(value) = &mut start.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"nested-call"[..]);
        value.tool = Box::from(&b"shell"[..]);
        let input =
            serde_json::to_vec(&serde_json::json!({"command": COMMAND, "timeout_seconds": 1})).expect("fixture");
        value.input_bytes = u64::try_from(input.len()).expect("bounded");
        value.input = Some(input.into_boxed_slice());
        value.deadline_ms = Some(1000);
    }
    if let Event::ToolCompleted(value) = &mut end.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"nested-call"[..]);
        value.tool = Box::from(&b"shell"[..]);
        value.verdict = smith_events::Verdict::TimedOut;
        value.clamped = false;
        value.duration_ms = 1001;
        value.child = None;
        value.delivery = None;
    }
    values.splice(5..5, [start, end]);
    for (index, record) in values.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("bounded");
    }
    common::events::inventory(&mut values);
    assert!(matches!(observed(&values, &check, &Capture::None), Measure::Unavailable { .. }));
    for capture in [Capture::Calls, Capture::Everything] {
        assert_eq!(observed(&values, &check, &capture), Measure::Observed { value: true });
        for command in [Some("sleep 600"), Some("sh -c 'sleep 600'"), Some(""), None] {
            let mut wrong = values.clone();
            if let Event::ToolStarted(value) = &mut wrong[5].event {
                let input = match command {
                    Some(command) => serde_json::json!({"command": command, "timeout_seconds": 1}),
                    None => serde_json::json!({"timeout_seconds": 1}),
                };
                value.input = Some(serde_json::to_vec(&input).expect("fixture").into_boxed_slice());
            }
            let actual = observed(&wrong, &check, &capture);
            if command.is_some() {
                assert_eq!(actual, Measure::Observed { value: false });
            } else {
                assert!(matches!(actual, Measure::Unavailable { .. }));
            }
        }
        for field in ["run", "conversation", "call", "tool"] {
            let mut wrong = values.clone();
            if let Event::ToolCompleted(value) = &mut wrong[6].event {
                match field {
                    "run" => value.run = 1,
                    "conversation" => value.conversation = 1,
                    "call" => value.call = Box::from(&b"other-call"[..]),
                    "tool" => value.tool = Box::from(&b"read"[..]),
                    _ => unreachable!("fixture selector"),
                }
            }
            assert!(matches!(observed(&wrong, &check, &capture), Measure::Unavailable { .. }), "{field}");
        }
        let mut wrong = values.clone();
        wrong.swap(5, 6);
        for (index, record) in wrong.iter_mut().enumerate() {
            record.t_ms = u64::try_from(index).expect("bounded");
        }
        assert!(matches!(observed(&wrong, &check, &capture), Measure::Unavailable { .. }));
        let mut wrong = values.clone();
        if let Event::ToolStarted(value) = &mut wrong[5].event {
            value.input = None;
        }
        assert!(matches!(observed(&wrong, &check, &capture), Measure::Unavailable { .. }));
    }
}

fn observed(records: &[Record], check: &EventCheck, capture: &Capture) -> Measure<bool> {
    let mut observer = Observer::new(&common::events::LIMITS).expect("actual reader");
    for record in records {
        let line =
            smith_events::write(record, capture, &common::events::LIMITS).expect("actual writer").expect("record");
        observer.feed(&line).expect("actual codec input");
    }
    observer.finish().expect("complete stream");
    observer.check(check, "task").passed
}
