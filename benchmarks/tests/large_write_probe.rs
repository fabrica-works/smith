//! Historical write cases remain intact; current predicates judge delivered bytes.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use smith_bench::agent::smith::events::Observer;
use smith_bench::{Count, EventCheck, Measure, OutcomeCheck};
use smith_events::{Effect, Event, Source, Verdict};

pub mod common;

#[test]
fn oversized_write_without_a_call_past_the_bound_is_not_exercised() {
    let mut records = common::events::balanced();
    let input = serde_json::json!({"path":"payload.txt","content":"Line 0001"});
    piece(&mut records, "write", "first", &input, Verdict::Written);
    let workspace = std::env::temp_dir();
    let verdict = observe(&records).oversized_write(std::path::Path::new("payload.txt"), 2048, Some(13848), &workspace);
    assert!(matches!(&verdict.passed, Measure::Unavailable { reason } if reason.starts_with("not exercised")));
    assert!(verdict.evidence.iter().any(|line| line.starts_with("NotExercised")));
}

#[test]
fn payload_refusal_within_the_call_bound_is_not_exercised() {
    let mut records = common::events::balanced();
    let input = serde_json::json!({"path":"payload.txt","content":"x".repeat(20)});
    piece(&mut records, "write", "first", &input, Verdict::TooLarge);
    let workspace = std::env::temp_dir();
    let verdict = observe(&records).oversized_write(std::path::Path::new("payload.txt"), 10, Some(13848), &workspace);
    assert!(matches!(&verdict.passed, Measure::Unavailable { reason } if reason.starts_with("not exercised")));
    assert!(verdict.evidence.iter().any(|line| line.starts_with("NotExercised")));
}

#[test]
fn protocol_probe_judges_accepted_pieces_without_inventing_a_refusal() {
    let directory = std::env::temp_dir().join(format!("smith-pieces-unexercised-{}", std::process::id()));
    fs::create_dir(&directory).expect("fresh workspace");
    fs::write(directory.join("payload.txt"), "hello world").expect("actual settled pieces");
    let mut records = common::events::balanced();
    piece(
        &mut records,
        "write",
        "first",
        &serde_json::json!({"path":"payload.txt","content":"hello"}),
        Verdict::Written,
    );
    piece(
        &mut records,
        "edit",
        "append",
        &serde_json::json!({"path":"payload.txt","edits":[{"old":"hello","new":"hello world"}]}),
        Verdict::Edited,
    );
    let checked = |records: &[smith_events::Record]| {
        observe(records)
            .file_pieces_for_protocol_probe(std::path::Path::new("payload.txt"), 10, 64, 15, &directory)
            .passed
    };
    assert_eq!(checked(&records), Measure::Observed { value: true });
    assert!(matches!(
        observe(&records).oversized_write(std::path::Path::new("payload.txt"), 10, Some(15), &directory).passed,
        Measure::Unavailable { reason } if reason.starts_with("not exercised")
    ));
    // Historical v7/v8 semantics and assertions are unchanged.
    assert_eq!(
        observe(&records).file_pieces(std::path::Path::new("payload.txt"), 10, 64, &directory).passed,
        Measure::Observed { value: false }
    );
    let original = records.clone();
    records.retain(|record| {
        !matches!(&record.event, Event::ToolStarted(start) if start.tool.as_ref() == b"edit")
            && !matches!(&record.event, Event::ToolCompleted(end) if end.tool.as_ref() == b"edit")
    });
    assert_eq!(checked(&records), Measure::Observed { value: false });
    records = original.clone();
    if let Event::ToolStarted(start) = &mut records[7].event {
        start.input = None;
    }
    assert!(matches!(checked(&records), Measure::Unavailable { .. }));
    records = original;
    fs::write(directory.join("payload.txt"), "altered").expect("saved file mismatch");
    assert_eq!(checked(&records), Measure::Observed { value: false });
    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn version_nine_exercises_the_actual_call_bound_with_small_repair_pieces() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let normal =
        smith_bench::check_task(&root.join("tests/fixtures/large-write/v7/task.toml"), &root.join("../docs/design"))
            .expect("unchanged normal v7");
    let task = smith_bench::check_task(&root.join("tasks/probes/large-write/task.toml"), &root.join("../docs/design"))
        .expect("current v9");
    assert_eq!(task.version, 9);
    assert_eq!(task.effort.as_deref(), Some("low"));
    assert_eq!(task.prompt, normal.prompt);
    assert_eq!(task.budget, normal.budget);
    let variant = task.with_variant("oversized").expect("small-payload variant");
    let medium = smith_bench::check_task(
        &root.join("tests/fixtures/large-write/v9/oversized-v9-medium.toml"),
        &root.join("../docs/design"),
    )
    .expect("named medium document");
    assert_eq!(medium.version, 9);
    assert_eq!(medium.effort.as_deref(), Some("medium"));
    let medium = medium.with_variant("oversized").expect("same medium variant");
    assert_eq!(medium.prompt, variant.prompt);
    assert_eq!(medium.budget, variant.budget);
    let mut historical_settings = medium.smith.clone();
    let mut live_settings = variant.smith.clone();
    assert_eq!(historical_settings.remove("deployment.conversations"), Some(toml::Value::Integer(6)));
    assert_eq!(live_settings.remove("deployment.conversations"), Some(toml::Value::Integer(5)));
    assert_eq!(historical_settings, live_settings);
    assert_eq!(medium.seed_sha256, variant.seed_sha256);
    assert_eq!(medium.event, variant.event);
    assert_eq!(medium.outcome, variant.outcome);
    assert_eq!(medium.deciding, variant.deciding);
    assert_eq!(variant.smith["deployment.tool_payload"].as_integer(), Some(2048));
    assert_eq!(variant.smith["deployment.read_window"].as_integer(), Some(2048));
    assert!(variant.prompt.contains("MEANT TO BE REFUSED"));
    assert!(!variant.prompt.contains("Do not read"));
    assert!(variant.event.contains(&EventCheck::OversizedWrite {
        path: "payload.txt".into(),
        content_bytes_above: 2048,
        protocol_input_bytes_above: Some(13848),
    }));
    let lines = |first, last| {
        let mut result = String::new();
        for index in first..=last {
            writeln!(&mut result, "Line {index:04}: The quick brown fox jumps over the lazy dog.").expect("line");
        }
        result
    };
    let first = lines(1, 280);
    let input =
        serde_json::to_vec(&serde_json::json!({"path":"payload.txt","content":first})).expect("actual JSON framing");
    assert_eq!(input.len(), 15995);
    assert!(input.len() * 100 >= 13848 * 115);
    assert_eq!(lines(1, 60).len(), 3360);
    assert_eq!(lines(1, 30).len(), 1680);
    assert!(lines(30, 60).len() <= 2048);
    assert!(!variant.deciding.iter().any(|name| name == "file-digest:payload.txt"));
}

#[test]
fn protocol_only_oversized_write_records_identity_and_elided_path() {
    let directory = std::env::temp_dir().join(format!("smith-protocol-oversized-{}", std::process::id()));
    fs::create_dir(&directory).expect("fresh workspace");
    let mut records = common::events::balanced();
    if let Event::ResponseCompleted(value) = &mut records[4].event {
        value.oversized = 1;
        let mut blocks = skein_lib::List::with_capacity(1);
        blocks
            .push(smith_events::Block::Oversized {
                call: b"refused".as_slice().into(),
                tool: b"write".as_slice().into(),
                bytes: 15995,
            })
            .expect("one protocol block");
        value.completion = Some(smith_events::Completion { blocks });
    }
    let checked = |records: &[smith_events::Record]| {
        observe(records).oversized_write(std::path::Path::new("payload.txt"), 2048, Some(13848), &directory)
    };
    let positive = checked(&records);
    assert_eq!(positive.passed, Measure::Observed { value: true });
    assert!(positive.evidence.iter().any(|item| item.contains("not carried (arguments elided by design)")));
    assert!(positive.evidence.iter().any(|item| item.contains("call=refused")));
    let original = records.clone();
    for case in ["at-bound", "wrong-tool", "absent-content", "counter", "loss"] {
        records = original.clone();
        if case == "loss" {
            if let Event::SessionEnded(end) = &mut records.last_mut().expect("ended").event {
                end.loss.events = 1;
            }
        } else if let Event::ResponseCompleted(response) = &mut records[4].event {
            match case {
                "absent-content" => response.completion = None,
                "counter" => response.oversized = 2,
                "at-bound" | "wrong-tool" => {
                    if let smith_events::Block::Oversized { bytes, tool, .. } =
                        response.completion.as_mut().expect("captured").blocks.get_mut(0).expect("one block")
                    {
                        if case == "at-bound" {
                            *bytes = 13848;
                        } else {
                            *tool = b"edit".as_slice().into();
                        }
                    }
                }
                _ => unreachable!("named negative"),
            }
        }
        if case == "absent-content" || case == "loss" {
            assert!(matches!(checked(&records).passed, Measure::Unavailable { .. }));
        } else {
            assert_eq!(checked(&records).passed, Measure::Observed { value: false });
        }
    }
    records = common::events::balanced();
    piece(
        &mut records,
        "write",
        "native",
        &serde_json::json!({"path":"payload.txt","content":"x".repeat(20)}),
        Verdict::TooLarge,
    );
    assert_eq!(
        observe(&records).oversized_write(std::path::Path::new("payload.txt"), 10, None, &directory).passed,
        Measure::Observed { value: true }
    );
    assert!(matches!(
        observe(&records).oversized_write(std::path::Path::new("payload.txt"), 10, Some(13848), &directory).passed,
        Measure::Unavailable { reason } if reason.starts_with("not exercised")
    ));
    fs::remove_dir(directory).expect("cleanup");
}

#[test]
fn version_seven_generates_visible_numbered_lines_at_low_with_unchanged_deciders() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let design = root.join("../docs/design");
    let current = smith_bench::check_task(&root.join("tests/fixtures/large-write/v7/task.toml"), &design)
        .expect("preserved v7 task");
    let old =
        smith_bench::check_task(&root.join("tests/fixtures/large-write/v6/task.toml"), &design).expect("preserved v6");
    assert_eq!(current.version, 7);
    assert_eq!(current.effort.as_deref(), Some("low"));
    assert!(current.prompt.contains("Line 0643"));
    assert!(current.prompt.contains("Do not read"));
    assert_eq!(current.tier, old.tier);
    assert_eq!(current.budget, old.budget);
    assert_eq!(current.smith, old.smith);
    assert_eq!(current.event, old.event);
    assert_eq!(current.deciding, old.deciding);
    for (now, before) in current.outcome.iter().zip(&old.outcome) {
        match now {
            OutcomeCheck::FileDigest { .. } => (),
            OutcomeCheck::FileContent { .. }
            | OutcomeCheck::FileSize { .. }
            | OutcomeCheck::Absent { .. }
            | OutcomeCheck::Pattern { .. }
            | OutcomeCheck::Protected { .. }
            | OutcomeCheck::End { .. }
            | OutcomeCheck::ExitCode { .. }
            | OutcomeCheck::LastLine { .. }
            | OutcomeCheck::Answer { .. }
            | OutcomeCheck::CredentialRefresh => assert_eq!(now, before),
        }
    }
    let piece = current.with_variant("oversized").expect("v7 oversized");
    let old_piece = old.with_variant("oversized").expect("v6 oversized");
    assert!(piece.prompt.contains("Line 0893"));
    assert!(piece.prompt.contains("Do not read"));
    assert_eq!(piece.smith, old_piece.smith);
    assert_eq!(piece.budget, old_piece.budget);
    assert_eq!(piece.event, old_piece.event);
    assert_eq!(piece.deciding, old_piece.deciding);
    for (now, before) in piece.outcome.iter().zip(&old_piece.outcome) {
        match now {
            OutcomeCheck::FileDigest { .. } => (),
            OutcomeCheck::FileContent { .. }
            | OutcomeCheck::FileSize { .. }
            | OutcomeCheck::Absent { .. }
            | OutcomeCheck::Pattern { .. }
            | OutcomeCheck::Protected { .. }
            | OutcomeCheck::End { .. }
            | OutcomeCheck::ExitCode { .. }
            | OutcomeCheck::LastLine { .. }
            | OutcomeCheck::Answer { .. }
            | OutcomeCheck::CredentialRefresh => assert_eq!(now, before),
        }
    }
    for (file, last, bytes) in [("source.txt", 643, 36_008), ("oversized-source.txt", 893, 50_008)] {
        let mut expected = String::new();
        for line in 1..=last {
            writeln!(&mut expected, "Line {line:04}: The quick brown fox jumps over the lazy dog.")
                .expect("string formatting");
        }
        assert_eq!(expected.len(), bytes);
        assert_eq!(
            fs::read(root.join("tests/fixtures/large-write/v7/seed").join(file)).expect("numbered measurement seed"),
            expected.as_bytes()
        );
    }
}

#[test]
fn version_six_names_medium_after_three_low_no_write_deviations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let design = root.join("../docs/design");
    let current = smith_bench::check_task(&root.join("tests/fixtures/large-write/v6/task.toml"), &design)
        .expect("preserved v6 task");
    let old = smith_bench::check_task(&root.join("tests/fixtures/large-write/v5/task.toml"), &design)
        .expect("preserved v5 task");
    assert_eq!(current.version, 6);
    assert_eq!(old.version, 5);
    assert_eq!(current.effort.as_deref(), Some("medium"));
    assert_eq!(old.effort.as_deref(), Some("low"));
    assert_eq!(current.tier, old.tier);
    assert_eq!(current.prompt, old.prompt);
    assert_eq!(current.budget, old.budget);
    assert_eq!(current.smith, old.smith);
    assert_eq!(current.seed_sha256, old.seed_sha256);
    assert_eq!(current.event, old.event);
    assert_eq!(current.outcome, old.outcome);
    assert_eq!(current.deciding, old.deciding);
    let current_piece = current.with_variant("oversized").expect("current variant");
    let old_piece = old.with_variant("oversized").expect("old variant");
    assert_eq!(current_piece.prompt, old_piece.prompt);
    assert_eq!(current_piece.smith, old_piece.smith);
    assert_eq!(current_piece.budget, old_piece.budget);
    assert_eq!(current_piece.event, old_piece.event);
    assert_eq!(current_piece.outcome, old_piece.outcome);
    assert_eq!(current_piece.deciding, old_piece.deciding);
}

#[test]
fn version_five_judges_saved_call_bytes_and_measures_seed_fidelity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let file = root.join("tests/fixtures/large-write/v5/task.toml");
    let task = smith_bench::check_task(&file, &root.join("../docs/design")).expect("v5 frozen task");
    let oversized = task.with_variant("oversized").expect("oversized variant");
    assert_eq!(task.version, 5);
    assert_eq!(task.budget, oversized.budget);
    assert_eq!(task.smith, oversized.smith);
    assert_eq!(task.budget.seconds, Some(300));
    assert_eq!(task.deadline_seconds, 330);
    assert_eq!(task.smith["deployment.read_window"].as_integer(), Some(32768));
    assert_eq!(task.smith["deployment.tool_payload"].as_integer(), Some(40960));
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
    static START: std::sync::LazyLock<smith_events::Record> = std::sync::LazyLock::new(|| {
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_started_everything.jsonl"))
    });
    static END: std::sync::LazyLock<smith_events::Record> = std::sync::LazyLock::new(|| {
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"))
    });
    let mut started = START.clone();
    let mut completed = END.clone();
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
    assert_eq!(task.smith["deployment.read_window"].as_integer(), Some(32768));
    assert_eq!(task.smith["deployment.tool_payload"].as_integer(), Some(40960));
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
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_started_everything.jsonl"));
    let mut completed =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
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
    let bytes = common::events::bytes(records);
    let ceiling = bytes.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("fixture lines");
    let ceiling = u32::try_from(ceiling).expect("bounded fixture line");
    let mut observer = Observer::with_ceiling(&common::events::LIMITS, ceiling).expect("reader");
    observer.feed(&bytes).expect("synthetic codec records");
    observer.finish().expect("complete framing");
    observer
}

fn tail_repetitions(file: &std::path::Path) -> usize {
    let content = std::fs::read_to_string(file).expect("ASCII prose seed");
    content.lines().rev().skip(1).take_while(|line| *line == "The work continued.").count()
}

#[test]
fn payload_too_large_write_then_actual_pieces_is_a_valid_oversized_refusal() {
    let directory = std::env::temp_dir().join(format!("smith-native-refusal-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("fresh workspace");
    std::fs::write(directory.join("payload.txt"), "hello world").expect("saved actual pieces");
    let mut records = common::events::balanced();
    piece(
        &mut records,
        "write",
        "large",
        &serde_json::json!({"path":"payload.txt","content":"x".repeat(20)}),
        Verdict::TooLarge,
    );
    piece(&mut records, "write", "w", &serde_json::json!({"path":"payload.txt","content":"hello"}), Verdict::Written);
    piece(
        &mut records,
        "edit",
        "e",
        &serde_json::json!({"path":"payload.txt","edits":[{"old":"hello","new":"hello world"}]}),
        Verdict::Edited,
    );
    for record in &mut records {
        if let Event::ToolStarted(start) = &mut record.event
            && let Some(input) = &start.input
        {
            start.input_bytes = u64::try_from(input.len()).expect("fixture bytes");
        }
    }
    let actual = observe(&records).file_pieces(std::path::Path::new("payload.txt"), 10, 64, &directory).passed;
    std::fs::remove_dir_all(directory).expect("cleanup workspace before assertion");
    assert_eq!(actual, Measure::Observed { value: true });
}

#[test]
fn corrected_refusal_checks_keep_normal_low_and_only_oversized_v8_medium() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let design = root.join("../docs/design");
    let normal = smith_bench::check_task(&root.join("tests/fixtures/large-write/v7-corrected/task.toml"), &design)
        .expect("normal v7");
    let original =
        smith_bench::check_task(&root.join("tests/fixtures/large-write/v7/task.toml"), &design).expect("original v7");
    assert_eq!(normal.version, 7);
    assert_eq!(normal.effort.as_deref(), Some("low"));
    assert_eq!(normal.prompt, original.prompt);
    assert_eq!(normal.event, original.event);
    assert_eq!(normal.outcome, original.outcome);
    assert_eq!(normal.deciding, original.deciding);
    let low = normal.with_variant("oversized").expect("corrected low");
    let medium = smith_bench::check_task(&root.join("tests/fixtures/large-write/v8/task.toml"), &design)
        .expect("explicit oversized v8")
        .with_variant("oversized")
        .expect("selected v8");
    assert_eq!(medium.version, 8);
    assert_eq!(medium.effort.as_deref(), Some("medium"));
    assert_eq!(low.prompt, medium.prompt);
    assert_eq!(low.smith, medium.smith);
    assert_eq!(low.budget, medium.budget);
    assert_eq!(low.seed_sha256, medium.seed_sha256);
    assert_eq!(low.event, medium.event);
    assert_eq!(low.outcome, medium.outcome);
    assert_eq!(low.deciding, medium.deciding);
    assert!(medium.deciding.iter().any(|name| name == "oversized-write"));
    assert!(!medium.deciding.iter().any(|name| name == "blocks"));
    assert!(medium.event.contains(&EventCheck::Blocks { oversized: Some(Count::Exactly(1)), cut: None }));
}

#[test]
fn native_oversized_refusal_rejects_wrong_pairs_small_content_loss_and_absent_capture() {
    let directory = std::env::temp_dir().join(format!("smith-refusal-negatives-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("fresh workspace");
    let mut records = common::events::balanced();
    piece(
        &mut records,
        "write",
        "large",
        &serde_json::json!({"path":"payload.txt","content":"x".repeat(20)}),
        Verdict::TooLarge,
    );
    let checked = |rows: &[smith_events::Record]| {
        observe(rows).oversized_write(std::path::Path::new("payload.txt"), 10, None, &directory).passed
    };
    assert_eq!(checked(&records), Measure::Observed { value: true });
    let original = records.clone();
    for value in ["run", "conversation", "call", "terminal", "small", "capture", "effect", "loss", "duplicate"] {
        records = original.clone();
        match value {
            "run" | "conversation" | "call" | "terminal" => {
                if let Event::ToolCompleted(end) = &mut records[6].event {
                    match value {
                        "run" => end.run = 9,
                        "conversation" => end.conversation = 9,
                        "call" => end.call = b"foreign".as_slice().into(),
                        "terminal" => end.verdict = Verdict::Written,
                        _ => unreachable!("bounded negative"),
                    }
                }
                assert_eq!(checked(&records), Measure::Observed { value: false });
            }
            "small" | "capture" | "effect" => {
                if let Event::ToolStarted(start) = &mut records[5].event {
                    match value {
                        "small" => {
                            start.input = Some(
                                serde_json::to_vec(&serde_json::json!({"path":"payload.txt","content":"x".repeat(10)}))
                                    .expect("JSON")
                                    .into(),
                            );
                        }
                        "capture" => start.input = None,
                        "effect" => start.effect = Effect::Read,
                        _ => unreachable!("bounded negative"),
                    }
                }
                if value == "small" {
                    assert_eq!(checked(&records), Measure::Observed { value: false });
                } else {
                    assert!(matches!(checked(&records), Measure::Unavailable { .. }));
                }
            }
            "loss" => {
                if let Event::SessionEnded(end) = &mut records.last_mut().expect("ended").event {
                    end.loss.events = 1;
                }
                assert!(matches!(checked(&records), Measure::Unavailable { .. }));
            }
            "duplicate" => {
                records.insert(7, records[6].clone());
                assert_eq!(checked(&records), Measure::Observed { value: false });
            }
            _ => unreachable!("named negatives"),
        }
    }
    // Protocol counters remain protocol-only: this native refusal creates no oversized response block.
    assert_eq!(
        observe(&original).check(&EventCheck::Blocks { oversized: Some(Count::Exactly(0)), cut: None }, "task").passed,
        Measure::Observed { value: true }
    );
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn oversized_refusal_threshold_must_equal_the_native_tool_payload() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("tests/fixtures/large-write/v8");
    let directory = std::env::temp_dir().join(format!("smith-refusal-declaration-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("fresh task");
    std::fs::create_dir(directory.join("seed")).expect("seed directory");
    for name in ["source.txt", "oversized-source.txt", "README.md"] {
        std::fs::copy(source.join("seed").join(name), directory.join("seed").join(name)).expect("same seed bytes");
    }
    let document = std::fs::read_to_string(source.join("task.toml"))
        .expect("v8 task")
        .replace("content_bytes_above = 40960", "content_bytes_above = 40959");
    std::fs::write(directory.join("task.toml"), document).expect("wrong threshold fixture");
    let refused = smith_bench::check_task(&directory.join("task.toml"), &root.join("../docs/design"))
        .expect_err("native bound mismatch refuses");
    std::fs::remove_dir_all(directory).expect("cleanup");
    assert!(refused.to_string().contains("authored native tool_payload"));
}
