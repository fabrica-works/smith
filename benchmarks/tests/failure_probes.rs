//! Failure probes inspect outside terminals; synthetic records are not live proof.

use std::path::PathBuf;

use smith_bench::agent::smith::events::Observer;
use smith_bench::{End, EventCheck, Exit, Forced, Measure, OutcomeCheck};
use smith_events::{AnswerClass, AnswerFailure, Event, Status};

mod common;

#[test]
fn failure_variants_keep_their_class_codes_and_do_not_invent_startup_terminals() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&root.join("tasks/probes/failures/task.toml"), &root.join("../docs/design"))
        .expect("authored task and unchanged seed");
    for (name, code) in [("unknown-model", 1), ("derived-limit", 2), ("bad-setting", 2), ("one-turn", 2)] {
        let variant = task.with_variant(name).expect("authored failure variant");
        assert!(variant.outcome.contains(&OutcomeCheck::ExitCode { code }));
        assert!(variant.deciding.iter().any(|name| name == "last-line"));
    }
    let unknown = task.with_variant("unknown-model").expect("provider failure");
    assert_eq!(unknown.smith["model"].as_str(), Some("smith-bench-nonexistent-model-2026-10-11"));
    let derived = task.with_variant("derived-limit").expect("named relationship");
    assert_eq!(derived.smith["read_window"].as_integer(), Some(40961));
    assert_eq!(derived.smith["tool_payload"].as_integer(), Some(40960));
    assert!(!derived.event.iter().any(|check| matches!(check, EventCheck::RunCompleted { .. })));
    let bad = task.with_variant("bad-setting").expect("invalid native setting");
    assert_eq!(bad.smith["output"].as_integer(), Some(0));
    assert!(bad.event.contains(&EventCheck::NoRequest));
    let mut empty = Observer::new(&common::events::LIMITS).expect("reader");
    empty.finish().expect("empty native capture framed completely");
    assert!(matches!(empty.check(&EventCheck::NoRequest, "task").passed, Measure::Unavailable { .. }));
    assert!(matches!(empty.classify(Exit::Code(2), Forced::No).end, End::HarnessError { .. }));
}

#[test]
fn model_failure_requires_the_run_class_and_the_actual_matching_exit() {
    let mut records = common::events::balanced();
    let failure = AnswerFailure {
        class: AnswerClass::Model,
        completion: None,
        account: None,
        which: None,
        reason: Some(Box::from(&b"unknown model"[..])),
    };
    if let Event::RunCompleted(value) = &mut records[6].event {
        value.status = Status::Failed;
        value.result = None;
        value.failure = Some(failure);
    }
    if let Event::SessionEnded(value) = &mut records[7].event {
        value.exit = 1;
    }
    let check = EventCheck::RunCompleted { status: "failed".into(), class: Some("model".into()), reason: None };
    let observer = observe(&records);
    assert_eq!(observer.check(&check, "task").passed, Measure::Observed { value: true });
    assert!(matches!(observer.classify(Exit::Code(1), Forced::No).end, End::Failed { .. }));
    assert!(matches!(observer.classify(Exit::Code(0), Forced::No).end, End::HarnessError { .. }));
    if let Event::RunCompleted(value) = &mut records[6].event {
        value.failure.as_mut().expect("failure").class = AnswerClass::Policy;
    }
    assert_eq!(observe(&records).check(&check, "task").passed, Measure::Observed { value: false });
}

fn observe(records: &[smith_events::Record]) -> Observer {
    let mut observer = Observer::new(&common::events::LIMITS).expect("reader");
    observer.feed(&common::events::bytes(records)).expect("synthetic codec records");
    observer.finish().expect("complete framing");
    observer
}
