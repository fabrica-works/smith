//! Authored write probes retain the overflow and successful piecewise repair.

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use smith_bench::agent::smith::events::Observer;
use smith_bench::{Count, EventCheck, Measure, OutcomeCheck};
use smith_events::{Effect, Event, Source, Verdict};

mod common;

#[test]
fn version_three_copies_all_prose_lines_before_finishing() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let file = root.join("tasks/probes/large-write/task.toml");
    let task = smith_bench::check_task(&file, &root.join("../docs/design")).expect("frozen task and seed");
    let oversized = task.with_variant("oversized").expect("oversized variant");
    assert_eq!(task.version, 3);
    assert_eq!(task.budget, oversized.budget);
    assert_eq!(task.smith, oversized.smith);
    assert_eq!(task.budget.seconds, Some(300));
    assert_eq!(task.deadline_seconds, 330);
    assert_eq!(task.smith["read_window"].as_integer(), Some(32768));
    assert_eq!(task.smith["tool_payload"].as_integer(), Some(40960));
    for (name, expected, windows, lines, prompt) in
        [("source.txt", 32768, 1, 552, &task.prompt), ("oversized-source.txt", 50000, 2, 827, &oversized.prompt)]
    {
        let bytes =
            std::fs::read(file.parent().expect("task file has a parent").join("seed").join(name)).expect("seed prose");
        assert_eq!(bytes.len(), expected);
        assert!(bytes.is_ascii());
        assert!(bytes.ends_with(b"This is the end of the source report.\n"));
        assert_eq!(bytes.len().div_ceil(32768), windows);
        assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), lines);
        assert!(bytes.split(|byte| *byte == b'\n').all(|line| line.len() <= 76));
        assert!(prompt.contains(&format!("ALL {lines} source lines")));
        assert!(prompt.contains("remove only those display additions"));
        assert!(prompt.contains("do not skip or deduplicate any"));
        assert!(prompt.contains("The final source line is This is the end of the source report."));
    }
    assert!(task.prompt.contains("source.txt") && task.prompt.contains("exactly one write"));
    assert!(oversized.prompt.contains("oversized-source.txt") && oversized.prompt.contains("two read"));
    assert!(task.prompt.contains("finish with ALPHA only after the write reports written"));
    assert!(oversized.prompt.contains("finish before both repair tools report success"));
    assert!(task.event.contains(&EventCheck::ToolCall {
        tool: "write".into(),
        count: Some(Count::Exactly(1)),
        input_bytes_at_least: Some(32768),
        verdict: Some("written".into()),
        exit: None,
        clamped: None,
    }));
}

#[test]
fn oversized_write_requires_the_overflow_and_both_accepted_repair_tools() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&root.join("tasks/probes/large-write/task.toml"), &root.join("../docs/design"))
        .expect("manifest and unchanged seed");
    let oversized = task.with_variant("oversized").expect("replacement checks");
    for (value, name) in [(&task, "source.txt"), (&oversized, "oversized-source.txt")] {
        let source = std::fs::read(root.join("tasks/probes/large-write/seed").join(name)).expect("plain prose seed");
        let digest = format!("{:x}", Sha256::digest(source));
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
