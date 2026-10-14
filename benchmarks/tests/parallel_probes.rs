//! Synthetic codec fixtures decide concurrency; they are not live measurements.

use smith_bench::agent::smith::events::Observer;
use smith_bench::{EventCheck, Measure};
use smith_events::{ConversationKind, Event, Record};

pub mod common;

#[test]
fn children_overlap_requires_positive_time_in_the_same_run() {
    let check = EventCheck::ChildrenOverlap { at_least: 2 };
    assert_eq!(
        observe(&children([(10, 30, 0), (20, 40, 0)])).check(&check, "task").passed,
        Measure::Observed { value: true }
    );
    for intervals in [[(10, 20, 0), (20, 30, 0)], [(10, 10, 0), (10, 10, 0)], [(10, 30, 0), (20, 40, 1)]] {
        assert_eq!(observe(&children(intervals)).check(&check, "task").passed, Measure::Observed { value: false });
    }
    let mut missing = children([(10, 30, 0), (20, 40, 0)]);
    missing.retain(|record| !matches!(&record.event, Event::ConversationClosed(value) if value.conversation == 2));
    common::events::inventory(&mut missing);
    assert!(matches!(observe(&missing).check(&check, "task").passed, Measure::Unavailable { .. }));
}

#[test]
fn parallel_tasks_are_frozen_and_decide_actual_call_and_child_evidence() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let read =
        smith_bench::check_task(&root.join("tasks/probes/parallel-read/task.toml"), &root.join("../docs/design"))
            .expect("parallel reads");
    assert!(read.event.contains(&EventCheck::CallsInOneResponse { tool: "read".into(), at_least: 20 }));
    assert_eq!(read.smith["deployment.calls_per_response"].as_integer(), Some(20));
    let children =
        smith_bench::check_task(&root.join("tasks/probes/parallel-children/task.toml"), &root.join("../docs/design"))
            .expect("parallel children");
    for (task, count) in [(children.clone(), 4), (children.with_variant("reviews").expect("reviews"), 3)] {
        assert!(task.event.contains(&EventCheck::ChildrenOverlap { at_least: 2 }));
        assert!(task.event.contains(&EventCheck::ConversationOpened {
            kind: "child".into(),
            count: smith_bench::Count::Exactly(count)
        }));
        assert_eq!(task.event.iter().filter(|check| matches!(check, EventCheck::ConversationOpened { .. })).count(), 1);
        assert!(
            task.event
                .iter()
                .any(|check| matches!(check, EventCheck::ToolCall { verdict: Some(value), .. } if value == "result"))
        );
    }
}

#[test]
fn twenty_reads_must_be_in_one_response_and_content_absence_is_unknown() {
    let check = EventCheck::CallsInOneResponse { tool: "read".into(), at_least: 20 };
    let mut records = common::events::balanced();
    set_calls(&mut records[4], 20);
    assert_eq!(observe_wide(&records).check(&check, "task").passed, Measure::Observed { value: true });
    set_calls(&mut records[4], 10);
    let mut second = records[4].clone();
    if let Event::ResponseCompleted(value) = &mut second.event {
        value.response = 1;
    }
    records.insert(5, second);
    common::events::inventory(&mut records);
    assert_eq!(observe_wide(&records).check(&check, "task").passed, Measure::Observed { value: false });
    for record in &mut records {
        if let Event::ResponseCompleted(value) = &mut record.event {
            value.completion = None;
        }
    }
    assert!(matches!(observe_wide(&records).check(&check, "task").passed, Measure::Unavailable { .. }));
}

fn set_calls(record: &mut Record, count: u32) {
    if let Event::ResponseCompleted(value) = &mut record.event {
        let mut blocks = skein_lib::List::with_capacity(count);
        for index in 0..count {
            blocks
                .push(smith_events::Block::Call {
                    call: format!("read-{index}").into_bytes().into_boxed_slice(),
                    tool: Box::from(&b"read"[..]),
                    input: format!("{{\"path\":\"part{:02}.txt\"}}", index + 1).into_bytes().into_boxed_slice(),
                })
                .expect("twenty bounded calls");
        }
        value.completion = Some(smith_events::Completion { blocks });
        value.calls = u64::from(count);
        value.blocks = u64::from(count);
    }
}

fn observe_wide(records: &[Record]) -> Observer {
    let limits = smith_events::Limits { items: 64, ..common::events::LIMITS };
    let mut observer = Observer::new(&limits).expect("twenty-call fixture");
    for record in records {
        let line = smith_events::write(record, &smith_events::Capture::Everything, &limits)
            .expect("typed fixture")
            .expect("captured fixture");
        observer.feed(&line).expect("real codec");
    }
    observer.finish().expect("complete framing");
    observer
}

fn children(intervals: [(u64, u64, u64); 2]) -> Vec<Record> {
    let mut records = common::events::balanced();
    let opened = records[2].clone();
    let closed = records[5].clone();
    for (index, (start, end, run)) in intervals.into_iter().enumerate() {
        let mut opening = opened.clone();
        opening.t_ms = start;
        if let Event::ConversationOpened(value) = &mut opening.event {
            value.run = run;
            value.conversation = u64::try_from(index + 1).expect("two children");
            value.kind = ConversationKind::Child;
            value.parent = Some(0);
            value.call = Some(format!("child-{index}").into_bytes().into_boxed_slice());
        }
        let mut closure = closed.clone();
        closure.t_ms = end;
        if let Event::ConversationClosed(value) = &mut closure.event {
            value.run = run;
            value.conversation = u64::try_from(index + 1).expect("two children");
        }
        records.extend([opening, closure]);
    }
    // Retain actual session framing around the synthetic intervals.
    let mut terminal = records.remove(7);
    terminal.t_ms = 100;
    records.sort_by_key(|record| record.t_ms);
    records.push(terminal);
    common::events::inventory(&mut records);
    records
}

fn observe(records: &[Record]) -> Observer {
    let mut observer = Observer::new(&common::events::LIMITS).expect("reader");
    observer.feed(&common::events::bytes(records)).expect("real codec");
    observer.finish().expect("complete framing");
    observer
}
