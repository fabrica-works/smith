//! Historical write cases remain intact; current predicates judge delivered bytes.

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use smith_bench::agent::smith::events::Observer;
use smith_bench::{Count, EventCheck, Measure, OutcomeCheck};
use smith_events::{Effect, Event, Source, Verdict};

mod common;

#[test]
fn version_five_judges_saved_call_bytes_and_measures_seed_fidelity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let file = root.join("tasks/probes/large-write/task.toml");
    let task = smith_bench::check_task(&file, &root.join("../docs/design")).expect("v5 frozen task");
    let oversized = task.with_variant("oversized").expect("oversized variant");
    assert_eq!(task.version, 5);
    assert_eq!(task.budget, oversized.budget);
    assert_eq!(task.smith, oversized.smith);
    assert_eq!(task.budget.seconds, Some(300));
    assert_eq!(task.deadline_seconds, 330);
    assert_eq!(task.smith["read_window"].as_integer(), Some(32768));
    assert_eq!(task.smith["tool_payload"].as_integer(), Some(40960));
    for (name, size, lines) in [("source.txt", 36000, 600), ("oversized-source.txt", 50000, 834)] {
        let text =
            std::fs::read_to_string(file.parent().expect("task directory").join("seed").join(name)).expect("prose");
        assert_eq!(text.len(), size);
        assert_eq!(text.lines().count(), lines);
        assert!(text.is_ascii());
        assert!(text.ends_with("This is the end of the source report.\n"));
        assert!(text.lines().all(|line| line.len() <= 76));
        assert_eq!(text.lines().collect::<std::collections::BTreeSet<_>>().len(), lines);
        assert_eq!(size.div_ceil(32768), 2);
    }
    assert!(
        task.event.contains(&EventCheck::WrittenFile { path: "payload.txt".into(), content_bytes_at_least: 32768 })
    );
    assert!(oversized.event.contains(&EventCheck::FilePieces {
        path: "payload.txt".into(),
        bytes_above: 40960,
        file_bytes: 1_048_576
    }));
    assert!(!task.deciding.iter().chain(&oversized.deciding).any(|name| name == "file-digest:payload.txt"));
    assert!(
        task.outcome
            .iter()
            .chain(&oversized.outcome)
            .filter(|check| matches!(check, OutcomeCheck::FileDigest { .. }))
            .count()
            == 2
    );
}

#[test]
fn saved_write_checks_content_size_pairing_and_capture_not_seed() {
    let directory = std::env::temp_dir().join(format!("smith-written-file-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("fresh workspace");
    let content = "A line.\nAnother line.\n";
    std::fs::write(directory.join("payload.txt"), content).expect("saved actual write");
    let mut records = common::events::balanced();
    piece(&mut records, "write", "w", &serde_json::json!({"path":"./payload.txt","content":content}), Verdict::Written);
    let checked = |records: &[smith_events::Record], minimum| {
        observe(records).written_file(std::path::Path::new("payload.txt"), minimum, &directory).passed
    };
    assert_eq!(
        checked(&records, u64::try_from(content.len()).expect("small bytes")),
        Measure::Observed { value: true }
    );
    assert_eq!(
        checked(&records, 32768),
        Measure::Observed { value: false },
        "encoded input size cannot replace content size"
    );
    let original = records.clone();
    if let Event::ToolStarted(value) = &mut records[5].event {
        value.input = None;
    }
    assert!(matches!(checked(&records, 1), Measure::Unavailable { .. }));
    records = original.clone();
    if let Event::ToolCompleted(value) = &mut records[6].event {
        value.conversation = 7;
    }
    assert_eq!(checked(&records, 1), Measure::Observed { value: false });
    records = original.clone();
    if let Event::ToolStarted(value) = &mut records[5].event {
        value.effect = Effect::Read;
    }
    assert!(matches!(checked(&records, 1), Measure::Unavailable { .. }));
    records = original.clone();
    records.insert(7, records[6].clone());
    assert_eq!(checked(&records, 1), Measure::Observed { value: false });
    records = original.clone();
    if let Event::SessionEnded(value) = &mut records.last_mut().expect("ended").event {
        value.loss.events = 1;
    }
    assert!(matches!(checked(&records, 1), Measure::Unavailable { .. }));
    std::fs::write(directory.join("payload.txt"), "altered").expect("tampered saved file");
    assert_eq!(checked(&original, 1), Measure::Observed { value: false });
    std::fs::remove_dir_all(directory).expect("cleanup workspace");
}

#[test]
fn oversized_pieces_replay_successful_edits_and_keep_failed_edit_measurement() {
    let directory = std::env::temp_dir().join(format!("smith-file-pieces-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("fresh workspace");
    std::fs::write(directory.join("payload.txt"), "hello wide world").expect("saved actual pieces");
    let mut records = common::events::balanced();
    if let Event::ResponseCompleted(value) = &mut records[4].event {
        value.oversized = 1;
        let mut blocks = skein_lib::List::with_capacity(1);
        blocks
            .push(smith_events::Block::Oversized {
                call: b"large".as_slice().into(),
                tool: b"write".as_slice().into(),
                bytes: 20,
            })
            .expect("one refusal");
        value.completion = Some(smith_events::Completion { blocks });
    }
    piece(&mut records, "write", "w", &serde_json::json!({"path":"payload.txt","content":"hello"}), Verdict::Written);
    piece(
        &mut records,
        "edit",
        "bad",
        &serde_json::json!({"path":"payload.txt","edits":[{"old":"absent","new":"ignored"}]}),
        Verdict::NoMatch,
    );
    piece(
        &mut records,
        "edit",
        "e1",
        &serde_json::json!({"path":"payload.txt","edits":[{"old":"hello","new":"hello world"}]}),
        Verdict::Edited,
    );
    piece(
        &mut records,
        "edit",
        "e2",
        &serde_json::json!({"path":"payload.txt","edits":[{"old":"world","new":"wide world"}]}),
        Verdict::Edited,
    );
    let checked = |records: &[smith_events::Record]| {
        observe(records).file_pieces(std::path::Path::new("payload.txt"), 10, 64, &directory).passed
    };
    assert_eq!(checked(&records), Measure::Observed { value: true });
    let original = records.clone();
    if let Event::ResponseCompleted(value) = &mut records[4].event {
        value.completion = None;
    }
    assert!(matches!(checked(&records), Measure::Unavailable { .. }));
    records = original.clone();
    if let Event::ResponseCompleted(value) = &mut records[4].event {
        value.oversized = 0;
    }
    assert_eq!(checked(&records), Measure::Observed { value: false });
    records = original.clone();
    if let Event::ToolCompleted(value) = &mut records[12].event {
        value.call = b"foreign".as_slice().into();
    }
    assert_eq!(checked(&records), Measure::Observed { value: false });
    records = original.clone();
    if let Event::ToolStarted(value) = &mut records[11].event {
        value.input = None;
    }
    assert!(matches!(checked(&records), Measure::Unavailable { .. }));
    records = original.clone();
    if let Event::ToolStarted(value) = &mut records[9].event {
        value.input = Some(br#"{"path":"payload.txt","edits":[{"old":"missing","new":"wrong"}]}"#.as_slice().into());
    }
    assert_eq!(checked(&records), Measure::Observed { value: false });
    std::fs::write(directory.join("payload.txt"), "different pieces").expect("tampered saved file");
    assert_eq!(checked(&original), Measure::Observed { value: false });
    std::fs::remove_dir_all(directory).expect("cleanup workspace");
}

fn piece(records: &mut Vec<smith_events::Record>, tool: &str, call: &str, input: &serde_json::Value, verdict: Verdict) {
    let mut started =
        common::events::record(include_str!("../../crates/smith-events/golden/v1/tool_started_everything.jsonl"));
    let mut completed =
        common::events::record(include_str!("../../crates/smith-events/golden/v1/tool_completed_everything.jsonl"));
    started.t_ms = 4;
    completed.t_ms = 4;
    if let Event::ToolStarted(value) = &mut started.event {
        value.run = 0;
        value.conversation = 0;
        value.call = call.as_bytes().into();
        value.tool = tool.as_bytes().into();
        value.source = Source::Workspace;
        value.effect = Effect::Write;
        value.input_bytes = 32768;
        value.input = Some(serde_json::to_vec(input).expect("captured arguments").into());
    }
    if let Event::ToolCompleted(value) = &mut completed.event {
        value.run = 0;
        value.conversation = 0;
        value.call = call.as_bytes().into();
        value.tool = tool.as_bytes().into();
        value.verdict = verdict;
    }
    let at = records.len() - 3;
    records.insert(at, started);
    records.insert(at + 1, completed);
}

#[test]
fn version_four_copies_all_prose_lines_and_exact_tail() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let file = root.join("tests/fixtures/large-write/task-v4.toml");
    let task = smith_bench::check_task(&file, &root.join("../docs/design")).expect("frozen task and seed");
    let oversized = task.with_variant("oversized").expect("oversized variant");
    assert_eq!(task.version, 4);
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
        assert_eq!(bytes.split(|byte| *byte == b'\n').count(), lines + 1);
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
    assert!(task.prompt.contains("TWENTY-TWO consecutive lines"));
    assert!(oversized.prompt.contains("NINE consecutive lines"));
    assert_eq!(tail_repetitions(&root.join("tests/fixtures/large-write/seed/source.txt")), 22);
    assert_eq!(tail_repetitions(&root.join("tests/fixtures/large-write/seed/oversized-source.txt")), 9);
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
    let task =
        smith_bench::check_task(&root.join("tests/fixtures/large-write/task-v4.toml"), &root.join("../docs/design"))
            .expect("manifest and unchanged seed");
    let oversized = task.with_variant("oversized").expect("replacement checks");
    for (value, name) in [(&task, "source.txt"), (&oversized, "oversized-source.txt")] {
        let source = std::fs::read(root.join("tests/fixtures/large-write/seed").join(name)).expect("plain prose seed");
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

fn tail_repetitions(file: &std::path::Path) -> usize {
    let content = std::fs::read_to_string(file).expect("ASCII prose seed");
    content.lines().rev().skip(1).take_while(|line| *line == "The work continued.").count()
}
