//! Compaction openings and requests from reliability's 12.6 attempt 02.
//! The excerpt omits captured prompts; each check keeps the recorded identities
//! and choices, and adds the shared fixture's complete stream boundaries.

use smith_bench::{EventCheck, Measure};
use smith_events::{Event, Record, ToolChoice};

fn compaction_records() -> Vec<Record> {
    let mut records = super::balanced();
    let excerpt = include_str!("../recorded/compaction-8a9747fa-02.jsonl");
    let compactions: Vec<_> = excerpt.lines().map(super::record).collect();
    let last = compactions.last().expect("recorded requests").t_ms;
    for (index, record) in records[5..].iter_mut().enumerate() {
        record.t_ms = last + 1 + u64::try_from(index).expect("stream boundaries");
    }
    records.splice(5..5, compactions);
    records
}

#[test]
fn compaction_choice_fails_an_auto_compaction_request() {
    let records = compaction_records();
    let check = super::observe(&records).check(&EventCheck::CompactionChoice, "task");
    assert_eq!(check.passed, Measure::Observed { value: false });
    assert_eq!(check.evidence, ["run=1 conversation=1 response=17 attempt=0 tool_choice=Auto"]);
}

#[test]
fn compaction_choice_passes_when_every_compaction_asks_none() {
    let mut records = compaction_records();
    for record in &mut records {
        if let Event::ResponseStarted(request) = &mut record.event
            && request.conversation != 0
        {
            request.tool_choice = ToolChoice::None;
        }
    }
    assert_eq!(
        super::observe(&records).check(&EventCheck::CompactionChoice, "task").passed,
        Measure::Observed { value: true }
    );
}

#[test]
fn compaction_choice_is_unavailable_without_a_compaction() {
    assert_eq!(
        super::observe(&super::balanced()).check(&EventCheck::CompactionChoice, "task").passed,
        Measure::unavailable("no compaction request was observed")
    );
}
