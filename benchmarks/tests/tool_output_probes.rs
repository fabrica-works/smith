//! Tool-output fixtures enter the actual codec; none are provider evidence.

use smith_bench::agent::smith::events::Observer;
use smith_bench::{EventCheck, Measure};
use smith_events::{Event, JobEnded, Record, Verdict};
use std::path::PathBuf;

pub mod common;

#[test]
fn tool_output_inputs_and_deciders_match_the_authored_cases() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&root.join("tasks/probes/tool-output/task.toml"), &root.join("../docs/design"))
        .expect("frozen inputs and checks");
    let suite_file = root.join("suites/probes.toml");
    let mut suite = smith_bench::read_suite(&suite_file).expect("four authored variants");
    suite.tasks.retain(|name| name.starts_with("probes/tool-output/") && !name.ends_with("/background"));
    suite.agents.retain(|agent| agent.tasks.iter().any(|name| name == "probes/tool-output"));
    suite.agents[0].tasks.clone_from(&suite.tasks);
    let catalogue =
        smith_bench::catalogue(&root.join("tasks"), &root.join("../docs/design")).expect("frozen catalogue");
    let models = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("tiers");
    assert_eq!(
        smith_bench::validate_suite(&suite_file, &suite, &catalogue, &root.join("agents"), &models)
            .expect("actual selectors")
            .len(),
        4
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
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_started_everything.jsonl"));
    let mut end =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
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

#[test]
fn timeouts_require_the_native_equal_bound_exact_request_clamp_and_complete_join() {
    let check = EventCheck::ToolTimeout {
        requested_seconds: 9999,
        deadline_ms: 1000,
        clamped: true,
        grace_ms: 250,
        command: None,
    };
    let original = common::events::balanced();
    let mut values = original.clone();
    let mut start =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_started_everything.jsonl"));
    let mut end =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
    if let Event::ToolStarted(value) = &mut start.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"timeout-call"[..]);
        value.tool = Box::from(&b"shell"[..]);
        let input = br#"{"command":"sleep 600","timeout_seconds":9999}"#;
        value.input = Some(Box::from(input.as_slice()));
        value.input_bytes = u64::try_from(input.len()).expect("small input");
        value.deadline_ms = Some(1000);
    }
    if let Event::ToolCompleted(value) = &mut end.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"timeout-call"[..]);
        value.tool = Box::from(&b"shell"[..]);
        value.verdict = smith_events::Verdict::TimedOut;
        value.clamped = true;
        value.duration_ms = 1001;
        value.child = None;
        value.delivery = None;
    }
    values.splice(5..5, [start, end]);
    for (index, record) in values.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("small fixture") + if index >= 6 { 1000 } else { 0 };
    }
    common::events::inventory(&mut values);
    for capture in [smith_events::Capture::Calls, smith_events::Capture::Everything] {
        let verdict = observed_capture(&values, &check, &capture);
        assert_eq!(verdict.passed, Measure::Observed { value: true });
        assert!(verdict.evidence[0].contains("requested_seconds=9999 emitted_bound_ms=1000 duration_ms=1001"));
        for duration in [0, 999, 1251] {
            let mut changed = values.clone();
            if let Event::ToolCompleted(value) = &mut changed[6].event {
                value.duration_ms = duration;
            }
            assert_eq!(observed_capture(&changed, &check, &capture).passed, Measure::Observed { value: false });
        }
        let mut changed = values.clone();
        if let Event::ToolStarted(value) = &mut changed[5].event {
            value.deadline_ms = None;
        }
        assert!(matches!(observed_capture(&changed, &check, &capture).passed, Measure::Unavailable { .. }));
        if let Event::ToolStarted(value) = &mut changed[5].event {
            value.deadline_ms = Some(2000);
        }
        assert_eq!(observed_capture(&changed, &check, &capture).passed, Measure::Observed { value: false });
        changed = values.clone();
        if let Event::ToolCompleted(value) = &mut changed[6].event {
            value.clamped = false;
        }
        assert_eq!(observed_capture(&changed, &check, &capture).passed, Measure::Observed { value: false });
        if let Event::ToolCompleted(value) = &mut changed[6].event {
            value.clamped = true;
            value.run = 1;
        }
        assert!(matches!(observed_capture(&changed, &check, &capture).passed, Measure::Unavailable { .. }));
        changed = values.clone();
        if let Event::ToolStarted(value) = &mut changed[5].event {
            value.input = Some(Box::from(&br#"{"command":"sleep 600","timeout_seconds":1}"#[..]));
        }
        assert_eq!(observed_capture(&changed, &check, &capture).passed, Measure::Observed { value: false });
        let unclamped = EventCheck::ToolTimeout {
            requested_seconds: 1,
            deadline_ms: 1000,
            clamped: false,
            grace_ms: 250,
            command: None,
        };
        if let Event::ToolCompleted(value) = &mut changed[6].event {
            value.clamped = false;
        }
        assert_eq!(observed_capture(&changed, &unclamped, &capture).passed, Measure::Observed { value: true });
        if let Event::ToolStarted(value) = &mut changed[5].event {
            value.input = Some(Box::from(&br#"{"command":"sleep 600","timeout_seconds":1,"output_bytes":99999}"#[..]));
        }
        assert_eq!(observed_capture(&changed, &unclamped, &capture).passed, Measure::Observed { value: false });
    }
    assert!(matches!(
        observed_capture(&values, &check, &smith_events::Capture::None).passed,
        Measure::Unavailable { .. }
    ));
}

fn observed_capture(
    records: &[Record],
    check: &EventCheck,
    capture: &smith_events::Capture,
) -> smith_bench::CheckVerdict {
    let mut observer = Observer::new(&common::events::LIMITS).expect("reader");
    for record in records {
        let line =
            smith_events::write(record, capture, &common::events::LIMITS).expect("actual Writer").expect("record");
        observer.feed(&line).expect("actual Reader");
    }
    observer.finish().expect("complete stream");
    observer.check(check, "task")
}

const JOB_NUMBER: u64 = 1;
const OTHER_JOB_NUMBER: u64 = 2;
const OTHER_CONVERSATION: u64 = 2;
const FIRST_REQUEST_AT: usize = 3;
const FIRST_RESPONSE_AT: usize = 4;
const FIRST_TOOL_AT: usize = 5;
const SHELL_ANSWER_AT: usize = 6;
const JOB_TERMINAL_AT: usize = 10;

fn shell_records() -> (Record, Record) {
    let mut shell =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_started_everything.jsonl"));
    let mut running =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
    if let Event::ToolStarted(value) = &mut shell.event {
        value.run = 0;
        value.conversation = 0;
        value.call = b"shell-call".as_slice().into();
        value.tool = b"shell".as_slice().into();
    }
    if let Event::ToolCompleted(value) = &mut running.event {
        value.run = 0;
        value.conversation = 0;
        value.call = b"shell-call".as_slice().into();
        value.tool = b"shell".as_slice().into();
        value.job = Some(JOB_NUMBER);
        value.verdict = Verdict::Running;
        value.exit = None;
        value.child = None;
        value.delivery = None;
    }
    (shell, running)
}

fn story() -> Vec<Record> {
    let mut records = common::events::balanced();
    let (shell, running) = shell_records();
    let mut request = records[FIRST_REQUEST_AT].clone();
    if let Event::ResponseStarted(value) = &mut request.event {
        value.response = 1;
    }
    let mut response = records[FIRST_RESPONSE_AT].clone();
    if let Event::ResponseCompleted(value) = &mut response.event {
        value.response = 1;
    }

    let mut poll = shell.clone();
    if let Event::ToolStarted(value) = &mut poll.event {
        value.call = b"poll-call".as_slice().into();
        value.tool = b"job".as_slice().into();
    }
    let mut end = running.clone();
    if let Event::ToolCompleted(value) = &mut end.event {
        value.call = b"poll-call".as_slice().into();
        value.tool = b"job".as_slice().into();
        value.verdict = Verdict::Exited;
        value.exit = Some(smith_events::CommandExit::Code { code: 0 });
    }
    records.splice(FIRST_TOOL_AT..FIRST_TOOL_AT, [shell, running, request, response, poll, end]);
    for (index, record) in records.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("tiny story");
    }
    common::events::inventory(&mut records);
    records
}

#[test]
fn job_running_passes_on_a_shell_call_answered_running() {
    assert_eq!(observed(&story(), &EventCheck::JobRunning), Measure::Observed { value: true });
}

#[test]
fn job_running_fails_when_the_shell_finished_before_answering() {
    let mut records = story();
    if let Event::ToolCompleted(value) = &mut records[SHELL_ANSWER_AT].event {
        value.verdict = Verdict::Exited;
        value.job = None;
    }
    assert_eq!(observed(&records, &EventCheck::JobRunning), Measure::Observed { value: false });
}

#[test]
fn completion_during_job_passes_on_an_overlapping_request() {
    assert_eq!(observed(&story(), &EventCheck::CompletionDuringJob), Measure::Observed { value: true });
}

#[test]
fn completion_during_job_fails_when_the_session_only_waited() {
    let mut records = story();
    records.retain(|record| {
        !matches!(&record.event, Event::ResponseStarted(_) | Event::ResponseCompleted(_))
            || record.t_ms < u64::try_from(SHELL_ANSWER_AT).expect("small time")
    });
    common::events::inventory(&mut records);
    assert_eq!(observed(&records, &EventCheck::CompletionDuringJob), Measure::Observed { value: false });
}

#[test]
fn completion_during_job_ignores_another_conversations_overlapping_request() {
    let mut records = story();
    for record in records.iter_mut().skip(SHELL_ANSWER_AT) {
        if let Event::ResponseStarted(value) = &mut record.event {
            value.conversation = OTHER_CONVERSATION;
        }
    }
    assert_eq!(observed(&records, &EventCheck::CompletionDuringJob), Measure::Observed { value: false });
}

#[test]
fn job_terminal_passes_when_its_poll_reports_the_end() {
    assert_eq!(observed(&story(), &EventCheck::JobTerminal), Measure::Observed { value: true });
}

fn closing_terminal() -> Event {
    Event::JobEnded(JobEnded {
        run: 0,
        conversation: 0,
        job: JOB_NUMBER,
        call: b"shell-call".as_slice().into(),
        verdict: Verdict::Cancelled,
        exit: None,
        unread_bytes: 0,
        duration_ms: 0,
    })
}

#[test]
fn job_terminal_passes_when_its_closing_session_reports_cancellation() {
    let mut records = story();
    records[JOB_TERMINAL_AT].event = closing_terminal();
    common::events::inventory(&mut records);
    assert_eq!(observed(&records, &EventCheck::JobTerminal), Measure::Observed { value: true });
}

#[test]
fn job_terminal_fails_on_a_job_told_twice() {
    let mut records = story();
    records.insert(
        JOB_TERMINAL_AT,
        Record { t_ms: u64::try_from(JOB_TERMINAL_AT).expect("small time"), event: closing_terminal() },
    );
    common::events::inventory(&mut records);
    assert_eq!(observed(&records, &EventCheck::JobTerminal), Measure::Observed { value: false });
}

#[test]
fn job_terminal_ignores_another_jobs_reported_terminal() {
    let mut records = story();
    if let Event::ToolCompleted(value) = &mut records[JOB_TERMINAL_AT].event {
        value.job = Some(OTHER_JOB_NUMBER);
    }
    assert_eq!(observed(&records, &EventCheck::JobTerminal), Measure::Observed { value: false });
}
