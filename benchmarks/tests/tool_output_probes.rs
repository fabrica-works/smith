//! Tool-output fixtures enter the actual codec; none are provider evidence.

use smith_bench::agent::smith::events::Observer;
use smith_bench::{EventCheck, Measure};
use smith_events::{Event, Record};
use std::path::PathBuf;

mod common;

#[test]
fn tool_output_inputs_and_deciders_match_the_authored_cases() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&root.join("tasks/probes/tool-output/task.toml"), &root.join("../docs/design"))
        .expect("frozen inputs and checks");
    let suite_file = root.join("suites/tool-output.toml");
    let suite = smith_bench::read_suite(&suite_file).expect("three independent variants only");
    let catalogue =
        smith_bench::catalogue(&root.join("tasks"), &root.join("../docs/design")).expect("frozen catalogue");
    let models = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("tiers");
    assert_eq!(
        smith_bench::validate_suite(&suite_file, &suite, &catalogue, &root.join("agents"), &models)
            .expect("actual selectors")
            .len(),
        3
    );
    let output = std::process::Command::new("sh")
        .arg(root.join("tasks/probes/tool-output/seed/test.sh"))
        .output()
        .expect("credential-free seed command");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.len() > 10 * 1024);
    assert!(output.stdout.ends_with(b"ASSERT budget_total: expected 17, observed 19\n"));
    let document = std::fs::read_to_string(root.join("tasks/probes/tool-output/seed/manual.txt")).expect("document");
    assert!(document.len() > 30_000);
    assert_eq!(
        document.lines().nth(699),
        Some("QUOTE: The copper heron carries seven blue lanterns across the silent harbor.")
    );
    let test = task.with_variant("test-output").expect("one shell");
    assert!(test.event.contains(&EventCheck::ToolResultBytes { tool: "shell".into(), at_least: 10241 }));
    let quote = task.with_variant("quote").expect("read windows");
    assert!(!quote.event.iter().any(|check| matches!(check, EventCheck::ToolResultBytes { .. })));
    let utf = task.with_variant("utf-8").expect("first edit");
    assert!(!utf.event.iter().any(|check| matches!(check, EventCheck::ToolResultBytes { .. })));
}

#[test]
fn payload_bytes_come_from_the_actual_matching_terminal_not_input_or_another_run() {
    let check = EventCheck::ToolResultBytes { tool: "shell".into(), at_least: 10241 };
    let original = common::events::balanced();
    let mut values = original.clone();
    let mut start =
        common::events::record(include_str!("../../crates/smith-events/golden/v1/tool_started_everything.jsonl"));
    let mut end =
        common::events::record(include_str!("../../crates/smith-events/golden/v1/tool_completed_everything.jsonl"));
    if let Event::ToolStarted(value) = &mut start.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"shell-call"[..]);
        value.tool = Box::from(&b"shell"[..]);
        value.input_bytes = 100_000;
    }
    if let Event::ToolCompleted(value) = &mut end.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"shell-call"[..]);
        value.tool = Box::from(&b"shell"[..]);
        value.bytes = 10241;
        value.child = None;
        value.delivery = None;
    }
    values.splice(5..5, [start, end]);
    for (index, record) in values.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("small fixture");
    }
    common::events::inventory(&mut values);
    assert_eq!(observed(&values, &check), Measure::Observed { value: true });
    if let Event::ToolCompleted(value) = &mut values[6].event {
        value.bytes = 10240;
    }
    assert_eq!(observed(&values, &check), Measure::Observed { value: false });
    if let Event::ToolCompleted(value) = &mut values[6].event {
        value.bytes = 10241;
        value.run = 1;
    }
    assert!(matches!(observed(&values, &check), Measure::Unavailable { .. }));
    assert_eq!(observed(&original, &check), Measure::Observed { value: false });
}

fn observed(records: &[Record], check: &EventCheck) -> Measure<bool> {
    let mut previous = None;
    for capture in [smith_events::Capture::None, smith_events::Capture::Calls, smith_events::Capture::Everything] {
        let mut observer = Observer::new(&common::events::LIMITS).expect("reader");
        for record in records {
            let line = smith_events::write(record, &capture, &common::events::LIMITS)
                .expect("actual Writer")
                .expect("scalar metadata retained");
            observer.feed(&line).expect("actual Reader");
        }
        observer.finish().expect("complete stream");
        let actual = observer.check(check, "task").passed;
        if let Some(previous) = &previous {
            assert_eq!(*previous, actual, "payload metadata survives every capture");
        }
        previous = Some(actual);
    }
    previous.expect("three captures checked")
}
