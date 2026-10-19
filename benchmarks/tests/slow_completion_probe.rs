//! Synthetic native records test measurement semantics; no live duration is claimed.

use smith_bench::agent::smith::events::Observer;
use smith_bench::{CheckVerdict, EventCheck, Measure};
use smith_events::{Block, Capture, Event, Record};
use std::path::PathBuf;

pub mod common;

fn check() -> EventCheck {
    EventCheck::SlowCompletion { longer_than_ms: 60000, head_ms: 60000, idle_ms: 300_000, reasoning_bytes: 65536 }
}

fn fixture() -> Vec<Record> {
    let mut values = common::events::balanced();
    if let Event::ResponseCompleted(response) = &mut values[4].event {
        response.first_byte_ms = Some(31000);
        response.largest_gap_ms = Some(151_000);
        response.duration_ms = 61000;
        let mut blocks = skein_lib::List::with_capacity(1);
        blocks.push(Block::Opaque { bytes: 32769 + u64::from(skein_llm::REPLAY_HEADER_BYTES) }).expect("fixture block");
        response.completion = Some(smith_events::Completion { blocks });
    }
    common::events::inventory(&mut values);
    values
}

fn observed(records: &[Record], capture: &Capture) -> CheckVerdict {
    let mut observer = Observer::new(&common::events::LIMITS).expect("actual reader");
    for record in records {
        let line =
            smith_events::write(record, capture, &common::events::LIMITS).expect("actual writer").expect("record");
        observer.feed(&line).expect("actual input");
    }
    observer.finish().expect("complete stream");
    observer.check(&check(), "task")
}

#[test]
fn native_measurements_export_the_raw_item_and_preset_findings_without_failing() {
    let values = fixture();
    let verdict = observed(&values, &Capture::Everything);
    assert_eq!(verdict.passed, Measure::Observed { value: true });
    let evidence = verdict.evidence.join("\n");
    for expected in [
        "provider=endpoint model=model call_length_ms=61000",
        "first_byte_ms=31000",
        "largest_gap_ms=151000",
        "largest_reasoning_item_bytes=32769",
        "largest_opaque_envelope_bytes=32776",
        "preset_finding=true",
    ] {
        assert!(evidence.contains(expected), "{expected}: {evidence}");
    }
    for capture in [Capture::None, Capture::Calls] {
        let verdict = observed(&values, &capture);
        assert!(matches!(verdict.passed, Measure::Unavailable { .. }));
        assert!(verdict.evidence.iter().any(|line| line.contains("call_length_ms=61000")));
    }
}

#[test]
fn exercise_floor_is_strict_and_short_calls_are_not_exercised() {
    for length in [1, 59999, 60000] {
        let mut values = fixture();
        if let Event::ResponseCompleted(response) = &mut values[4].event {
            response.duration_ms = length;
        }
        let verdict = observed(&values, &Capture::Everything);
        assert!(matches!(verdict.passed, Measure::Unavailable { .. }));
        assert!(verdict.evidence.iter().any(|line| line.contains("NotExercised") && line.contains("model deviation")));
    }
    let mut values = fixture();
    if let Event::ResponseCompleted(response) = &mut values[4].event {
        response.duration_ms = 60001;
    }
    assert_eq!(observed(&values, &Capture::Everything).passed, Measure::Observed { value: true });
}

#[test]
fn absent_transport_content_wrong_join_and_failed_terminal_cannot_pass() {
    for field in ["first", "gap", "content", "response", "run", "conversation", "envelope", "unknown"] {
        let mut values = fixture();
        if let Event::ResponseCompleted(response) = &mut values[4].event {
            match field {
                "first" => response.first_byte_ms = None,
                "gap" => response.largest_gap_ms = None,
                "content" => response.completion = None,
                "response" => response.response = 1,
                "run" => response.run = 1,
                "conversation" => response.conversation = 1,
                "envelope" | "unknown" => {
                    let mut blocks = skein_lib::List::with_capacity(1);
                    blocks
                        .push(if field == "envelope" {
                            Block::Opaque { bytes: 6 }
                        } else {
                            Block::Unknown { kind: Box::from(&b"future"[..]) }
                        })
                        .expect("fixture");
                    response.completion = Some(smith_events::Completion { blocks });
                }
                _ => unreachable!("fixture selector"),
            }
        }
        assert!(matches!(observed(&values, &Capture::Everything).passed, Measure::Unavailable { .. }), "{field}");
    }
    for outcome in [smith_events::Outcome::Failed, smith_events::Outcome::Cancelled] {
        let mut values = fixture();
        if let Event::ResponseCompleted(response) = &mut values[4].event {
            response.outcome = outcome;
        }
        assert_eq!(observed(&values, &Capture::Everything).passed, Measure::Observed { value: false });
    }
    let mut values = fixture();
    let mut notice =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/notice_everything.jsonl"));
    if let Event::Notice(notice) = &mut notice.event {
        notice.kind = smith_events::NoticeKind::ReasoningDropped;
        notice.run = Some(0);
        notice.bytes = Some(65537);
    }
    values.insert(5, notice);
    for (index, record) in values.iter_mut().enumerate() {
        record.t_ms = u64::try_from(index).expect("bounded");
    }
    common::events::inventory(&mut values);
    assert!(matches!(observed(&values, &Capture::Everything).passed, Measure::Unavailable { .. }));
}

#[test]
fn frozen_slow_task_uses_high_effort_native_only_and_the_declared_preset_bounds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task =
        smith_bench::check_task(&root.join("tasks/probes/slow-completion/task.toml"), &root.join("../docs/design"))
            .expect("frozen task");
    assert_eq!(task.version, 1);
    assert_eq!(task.effort.as_deref(), Some("high"));
    assert!(task.event.contains(&check()));
    assert!(task.prompt.contains("reasoning alone") && task.prompt.contains("Do not simulate slowness"));
    let suite_file = root.join("suites/probes.toml");
    let mut suite = smith_bench::read_suite(&suite_file).expect("native-only suite");
    suite.tasks = vec!["probes/slow-completion".into()];
    suite.agents.retain(|agent| agent.tasks.iter().any(|name| name == "probes/slow-completion"));
    suite.agents[0].tasks.clone_from(&suite.tasks);
    assert_eq!(suite.agents.len(), 1);
    assert_eq!(suite.agents[0].agent, smith_bench::Agent::Smith);
    let catalogue = smith_bench::catalogue(&root.join("tasks"), &root.join("../docs/design")).expect("catalogue");
    let tiers = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("tiers");
    assert_eq!(
        smith_bench::validate_suite(&suite_file, &suite, &catalogue, &root.join("agents"), &tiers)
            .expect("selectors")
            .len(),
        1
    );
    let problem = std::fs::read_to_string(root.join("tasks/probes/slow-completion/seed/problem.txt")).expect("seed");
    assert_eq!(problem.lines().count(), 38);
}
