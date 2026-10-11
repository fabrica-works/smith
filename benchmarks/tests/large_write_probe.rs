//! Authored write probes retain the overflow and successful piecewise repair.

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use smith_bench::agent::smith::events::Observer;
use smith_bench::{Count, EventCheck, Measure, OutcomeCheck};
use smith_events::{Effect, Event, Source, Verdict};

mod common;

#[test]
fn oversized_write_requires_the_overflow_and_both_accepted_repair_tools() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&root.join("tasks/probes/large-write/task.toml"), &root.join("../docs/design"))
        .expect("manifest and unchanged seed");
    let oversized = task.with_variant("oversized").expect("replacement checks");
    for (value, size) in [(&task, 32768), (&oversized, 50000)] {
        let digest = format!("{:x}", Sha256::digest(vec![b'A'; size]));
        assert!(value.outcome.contains(&OutcomeCheck::FileDigest { path: "payload.txt".into(), sha256: digest }));
    }
    let mut records = common::events::balanced();
    if let Event::ResponseCompleted(value) = &mut records[4].event {
        value.oversized = 1;
    }
    add_tool(&mut records, "write", Verdict::Written);
    add_tool(&mut records, "edit", Verdict::Edited);
    let checks: Vec<_> = oversized.event.iter().filter(|check| matches!(check, EventCheck::ToolCall { .. })).collect();
    assert_eq!(checks.len(), 2);
    for check in &checks {
        assert!(matches!(check, EventCheck::ToolCall { count: Some(Count::AtLeast(1)), .. }));
        assert_eq!(observe(&records).check(check, "task").passed, Measure::Observed { value: true });
    }
    let valid = records.clone();
    records.retain(|record| {
        !matches!(&record.event,
        Event::ToolStarted(value) if value.tool.as_ref() == b"edit")
    });
    assert_eq!(observe(&records).check(checks[1], "task").passed, Measure::Observed { value: false });
    records = valid;
    let check = EventCheck::Blocks { oversized: Some(Count::Exactly(1)), cut: None };
    assert_eq!(observe(&records).check(&check, "task").passed, Measure::Observed { value: true });
    if let Event::ResponseCompleted(value) = &mut records[4].event {
        value.oversized = 0;
    }
    assert_eq!(observe(&records).check(&check, "task").passed, Measure::Observed { value: false });
}

fn add_tool(records: &mut Vec<smith_events::Record>, tool: &str, verdict: Verdict) {
    let mut started =
        common::events::record(include_str!("../../crates/smith-events/golden/v1/tool_started_everything.jsonl"));
    let mut completed =
        common::events::record(include_str!("../../crates/smith-events/golden/v1/tool_completed_everything.jsonl"));
    started.t_ms = 4;
    completed.t_ms = 4;
    if let Event::ToolStarted(value) = &mut started.event {
        value.run = 0;
        value.conversation = 0;
        value.call = tool.as_bytes().into();
        value.tool = tool.as_bytes().into();
        value.source = Source::Workspace;
        value.effect = Effect::Write;
        value.input_bytes = 16;
        value.input = None;
    }
    if let Event::ToolCompleted(value) = &mut completed.event {
        value.run = 0;
        value.conversation = 0;
        value.call = tool.as_bytes().into();
        value.tool = tool.as_bytes().into();
        value.verdict = verdict;
    }
    records.insert(5, started);
    records.insert(6, completed);
}

fn observe(records: &[smith_events::Record]) -> Observer {
    let mut observer = Observer::new(&common::events::LIMITS).expect("reader");
    observer.feed(&common::events::bytes(records)).expect("synthetic codec records");
    observer.finish().expect("complete framing");
    observer
}
