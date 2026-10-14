//! Frozen path tasks and codec-derived controls; no provider or native run.

use std::path::PathBuf;

use smith_bench::agent::smith::events::Observer;
use smith_bench::{EventCheck, Measure};
use smith_events::{Capture, Effect, Event, Record, Source, ToolCompleted, ToolStarted, Verdict};

pub mod common;

use common::events::{LIMITS, balanced};

fn path_check() -> EventCheck {
    EventCheck::ToolPath {
        tool: "read".into(),
        path: "../spec/rules.txt".into(),
        aliases: vec!["/spec/rules.txt".into()],
        verdict: "read".into(),
    }
}

fn calls() -> Vec<Record> {
    let mut records = balanced();
    let mut call = records[0].clone();
    call.event = Event::ToolStarted(ToolStarted {
        run: 0,
        conversation: 0,
        call: Box::from(&b"path-read"[..]),
        tool: Box::from(&b"read"[..]),
        source: Source::Workspace,
        effect: Effect::Read,
        deadline_ms: Some(1000),
        input_bytes: 26,
        input: Some(Box::from(&b"{\"path\":\"/spec/rules.txt\"}"[..])),
    });
    records.insert(5, call);
    let mut terminal = records[0].clone();
    terminal.event = Event::ToolCompleted(ToolCompleted {
        run: 0,
        conversation: 0,
        call: Box::from(&b"path-read"[..]),
        tool: Box::from(&b"read"[..]),
        verdict: Verdict::Read,
        exit: None,
        job: None,
        bytes: 150,
        omitted_bytes: 0,
        clamped: false,
        duration_ms: 2,
        child: None,
        delivery: None,
    });
    records.insert(6, terminal);
    for (index, record) in records.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("small controls");
    }
    records
}

fn observed(records: &[Record], capture: &Capture) -> Observer {
    let lines: Vec<_> = records
        .iter()
        .map(|record| smith_events::write(record, capture, &LIMITS).expect("bounded controls").expect("metadata"))
        .collect();
    let ceiling = u32::try_from(lines.iter().map(|line| line.len()).max().expect("session")).expect("bounded");
    let mut observer = Observer::with_ceiling(&LIMITS, ceiling).expect("actual outside reader");
    for line in lines {
        observer.feed(&line).expect("actual captured codec");
    }
    observer.finish().expect("finished stream");
    observer
}

#[test]
fn a_path_requires_captured_input_and_its_exact_successful_terminal() {
    let records = calls();
    for capture in [Capture::Calls, Capture::Everything] {
        assert_eq!(observed(&records, &capture).check(&path_check(), "").passed, Measure::Observed { value: true });
    }
    assert!(matches!(observed(&records, &Capture::None).check(&path_check(), "").passed, Measure::Unavailable { .. }));
    for path in ["/spec/./rules.txt", "/spec//rules.txt", "../spec/rules.txt", "./../spec/rules.txt"] {
        let mut records = records.clone();
        if let Event::ToolStarted(start) = &mut records[5].event {
            start.input = Some(serde_json::to_vec(&serde_json::json!({"path":path})).expect("JSON").into());
        }
        assert_eq!(
            observed(&records, &Capture::Everything).check(&path_check(), "").passed,
            Measure::Observed { value: true }
        );
    }
    for path in ["spec/rules.txt", "/app/rules.txt", "/spec/../spec/rules.txt", "/spec/rules.txt.extra"] {
        let mut records = records.clone();
        if let Event::ToolStarted(start) = &mut records[5].event {
            start.input = Some(serde_json::to_vec(&serde_json::json!({"path":path})).expect("JSON").into());
        }
        assert_eq!(
            observed(&records, &Capture::Everything).check(&path_check(), "").passed,
            Measure::Observed { value: false }
        );
    }
    for input in [b"{bad".as_slice(), b"{\"path\":7}", b"{}"] {
        let mut records = records.clone();
        if let Event::ToolStarted(start) = &mut records[5].event {
            start.input = Some(input.into());
        }
        assert_eq!(
            observed(&records, &Capture::Everything).check(&path_check(), "").passed,
            Measure::Observed { value: false }
        );
    }
    for mutation in 0..5 {
        let mut records = records.clone();
        if let Event::ToolCompleted(end) = &mut records[6].event {
            match mutation {
                0 => end.run = 1,
                1 => end.conversation = 1,
                2 => end.call = Box::from(&b"another-call"[..]),
                3 => end.tool = Box::from(&b"write"[..]),
                4 => end.verdict = Verdict::Outside,
                _ => unreachable!("bounded test cases"),
            }
        }
        let actual = observed(&records, &Capture::Everything).check(&path_check(), "").passed;
        if mutation == 4 {
            assert_eq!(actual, Measure::Observed { value: false });
        } else {
            assert!(matches!(actual, Measure::Unavailable { .. }), "{mutation}: {actual:?}");
        }
    }
    let mut no_terminal = records.clone();
    no_terminal.remove(6);
    assert!(matches!(
        observed(&no_terminal, &Capture::Everything).check(&path_check(), "").passed,
        Measure::Unavailable { .. }
    ));
    let mut reversed = records.clone();
    reversed.swap(5, 6);
    for (index, record) in reversed.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("small controls");
    }
    assert!(matches!(
        observed(&reversed, &Capture::Everything).check(&path_check(), "").passed,
        Measure::Unavailable { .. }
    ));
    assert_eq!(
        observed(&balanced(), &Capture::Everything).check(&path_check(), "").passed,
        Measure::Observed { value: false }
    );
    let mut other_session = balanced();
    let mut unrelated = records[6].clone();
    unrelated.t_ms = other_session[6].t_ms;
    other_session.insert(6, unrelated);
    no_terminal.extend(other_session);
    assert!(matches!(
        observed(&no_terminal, &Capture::Everything).check(&path_check(), "").passed,
        Measure::Unavailable { .. }
    ));
    let mut relative = records;
    if let Event::ToolStarted(start) = &mut relative[5].event {
        start.input = Some(Box::from(&b"{\"path\":\"./input.txt\"}"[..]));
    }
    let relative_check =
        EventCheck::ToolPath { tool: "read".into(), path: "input.txt".into(), aliases: vec![], verdict: "read".into() };
    assert_eq!(
        observed(&relative, &Capture::Everything).check(&relative_check, "").passed,
        Measure::Observed { value: true }
    );
}

#[test]
fn the_frozen_path_tasks_keep_namespace_authority_and_two_completion_bounds() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&project.join("tasks/probes/paths/task.toml"), &project.join("../docs/design"))
        .expect("actual task validation");
    let suite = smith_bench::read_suite(&project.join("suites/probes.toml")).expect("native suite");
    let agent =
        suite.agents.iter().find(|agent| agent.tasks.iter().any(|name| name == "probes/paths")).expect("probe pin");
    assert_eq!(agent.config, "everything");
    assert_eq!(task.protected.len(), 2);
    assert_eq!(task.working_directory, Some(PathBuf::from("app")));
    assert!(task.prompt.contains("../spec/rules.txt"));
    assert!(!task.prompt.split_whitespace().any(|word| word.starts_with("/spec/")));
    assert_eq!(task.variant.len(), 1);
    assert_eq!(task.variant[0].name, "single");
    assert!(task.variant[0].replace_checks);
    assert!(!task.variant[0].prompt.as_ref().expect("single prompt").contains("app"));
    assert_eq!(task.smith["workspace.directories"].as_array().expect("mounts").len(), 2);
    assert_eq!(task.variant[0].smith["workspace.directories"].as_array().expect("single mount").len(), 1);
    assert!(task.event.iter().any(|check| matches!(check, EventCheck::ToolPath { path, aliases, .. }
        if path == "../spec/rules.txt" && aliases == &["/spec/rules.txt".to_owned()])));
    for checks in [&task.event, &task.variant[0].event] {
        assert!(checks.iter().any(|check| matches!(
            check,
            EventCheck::Completions { count: smith_bench::Count::AtMost(2), before: smith_bench::Before::Finish }
        )));
        assert!(checks.iter().any(|check| matches!(check, EventCheck::NoVerdict { verdicts } if verdicts.contains(&"outside".to_owned()) && verdicts.contains(&"not_found".to_owned()))));
    }
}
