//! Recorded second compaction of history/small-window attempt 05, and responses
//! 27 to 29 of base history attempt 03, from the c2498a9d calibration runs.
//! Excerpts omit system text, tool schemas and retry prompts the checks do not read.

use smith_bench::{CheckVerdict, EventCheck, Measure};
use smith_events::{Block, Capture, Event, Limits, Record};

const LIMITS: Limits = Limits { string: 16_384, content: 16_384, items: 16 };

fn records(source: &str) -> Vec<Record> {
    let excerpt: Vec<_> = source
        .lines()
        .map(|line| {
            smith_events::read_with_ceiling(
                line.as_bytes(),
                &LIMITS,
                u32::try_from(line.len() + 1).expect("record length"),
            )
            .expect("recorded excerpt")
            .expect("known event")
        })
        .collect();
    let mut records = super::balanced();
    let last = excerpt.last().expect("excerpt").t_ms;
    for (index, record) in records[5..].iter_mut().enumerate() {
        record.t_ms = last + 1 + u64::try_from(index).expect("boundaries");
    }
    records.splice(5..5, excerpt);
    records
}

fn compaction() -> Vec<Record> {
    records(include_str!("../recorded/compaction-carried-c2498a9d-05.jsonl"))
}

fn check(records: &[Record], check: &EventCheck) -> CheckVerdict {
    observer(records).check(check, "task")
}

fn observer(records: &[Record]) -> smith_bench::agent::smith::events::Observer {
    let mut observer =
        smith_bench::agent::smith::events::Observer::with_ceiling(&LIMITS, 65_536).expect("excerpt ceiling");
    for record in records {
        let line = smith_events::write_with_ceiling(record, &Capture::Everything, &LIMITS, 65_536)
            .expect("bounded excerpt")
            .expect("captured record");
        observer.feed(&line).expect("excerpt stream");
    }
    observer.finish().expect("complete excerpt");
    observer
}

#[test]
fn compaction_carried_passes_when_the_owed_read_and_the_summary_cross() {
    assert_eq!(check(&compaction(), &EventCheck::CompactionCarried).passed, Measure::Observed { value: true });
}

#[test]
fn compaction_carried_fails_a_request_without_its_owed_result() {
    let mut records = compaction();
    for record in &mut records {
        if let Event::ResponseStarted(value) = &mut record.event
            && value.conversation == 2
        {
            let messages = &mut value.prompt.as_mut().expect("captured prompt").messages;
            for index in 0..messages.len() {
                let message = messages.get_mut(index).expect("message index");
                let mut retained = skein_lib::List::with_capacity(message.blocks.len());
                for block in &message.blocks {
                    if !matches!(block, Block::Result { .. }) {
                        retained.push(block.clone()).expect("same bound");
                    }
                }
                message.blocks = retained;
            }
        }
    }
    let verdict = check(&records, &EventCheck::CompactionCarried);
    assert_eq!(verdict.passed, Measure::Observed { value: false });
    assert_eq!(verdict.evidence, ["run=1 compaction=2: missing result for call call_92815a0182aa48a5a3f3783d061b6601"]);
}

#[test]
fn compaction_carried_fails_a_window_without_its_summary() {
    let mut records = compaction();
    for record in &mut records {
        if let Event::ResponseStarted(value) = &mut record.event
            && value.response == 35
        {
            value
                .prompt
                .as_mut()
                .expect("captured next window")
                .messages
                .get_mut(0)
                .expect("first message")
                .blocks
                .clear();
        }
    }
    let verdict = check(&records, &EventCheck::CompactionCarried);
    assert_eq!(verdict.passed, Measure::Observed { value: false });
    assert_eq!(verdict.evidence, ["run=1 compaction=2: missing summary in next window's first message"]);
}

#[test]
fn a_failed_request_that_was_retried_leaves_its_checks_observed() {
    let records = records(include_str!("../recorded/retried-request-c2498a9d-03.jsonl"));
    let observer = observer(&records);
    assert_eq!(
        observer.check(&EventCheck::CallsPerResponse { tool: "read".into(), at_most: 1 }, "task").passed,
        Measure::Observed { value: true }
    );
    assert_eq!(observer.check(&EventCheck::PromptWithinWindow, "task").passed, Measure::Observed { value: true });
}

#[test]
fn compaction_carried_passes_when_no_window_was_compacted() {
    assert_eq!(check(&super::balanced(), &EventCheck::CompactionCarried).passed, Measure::Observed { value: true });
}

#[test]
fn compaction_carried_leaves_an_uncaptured_compaction_prompt_unavailable() {
    let mut records = compaction();
    for record in &mut records {
        if let Event::ResponseStarted(value) = &mut record.event
            && value.conversation == 2
        {
            value.prompt = None;
        }
    }
    assert_eq!(
        check(&records, &EventCheck::CompactionCarried).passed,
        Measure::unavailable("run=1 compaction=2: compaction prompt was not captured")
    );
}

#[test]
fn compaction_carried_needs_no_summary_without_a_parent_request() {
    let mut records = compaction();
    records.retain(|record| !matches!(&record.event, Event::ResponseStarted(value) if value.response == 35));
    assert_eq!(check(&records, &EventCheck::CompactionCarried).passed, Measure::Observed { value: true });
}

#[test]
fn an_unretried_failed_request_leaves_its_prompt_window_unavailable() {
    let mut records = records(include_str!("../recorded/retried-request-c2498a9d-03.jsonl"));
    records.retain(|record| {
        !matches!(&record.event,
        Event::ResponseStarted(value) if value.response == 29)
    });
    assert_eq!(
        check(&records, &EventCheck::PromptWithinWindow).passed,
        Measure::unavailable("provider did not report fresh prompt input")
    );
}

#[test]
fn call_count_passes_for_the_exact_named_tool_starts() {
    assert_eq!(
        check(&compaction(), &EventCheck::CallCount { tool: "read".into(), count: smith_bench::Count::Exactly(1) })
            .passed,
        Measure::Observed { value: true }
    );
}

#[test]
fn call_count_fails_when_an_unasked_read_was_made() {
    assert_eq!(
        check(&compaction(), &EventCheck::CallCount { tool: "read".into(), count: smith_bench::Count::Exactly(0) })
            .passed,
        Measure::Observed { value: false }
    );
}
