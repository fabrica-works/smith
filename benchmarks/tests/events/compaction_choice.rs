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
        Measure::unavailable("not exercised: session.started lacks run.compaction_material")
    );
}

// Six responses from history/small-window attempt 03 at 8a5bb9ec. The
// excerpt keeps usage, model windows and the effective compaction material,
// omitting captures and startup limits these checks do not read.
fn threshold_records() -> Vec<Record> {
    include_str!("../recorded/compaction-threshold-8a5bb9ec-03.jsonl").lines().map(super::record).collect()
}

fn compaction_opened() -> EventCheck {
    EventCheck::ConversationOpened { kind: "compaction".into(), count: smith_bench::Count::AtLeast(1) }
}

#[test]
fn compaction_checks_are_not_exercised_below_the_threshold() {
    let observer = super::observe(&threshold_records());
    for check in [compaction_opened(), EventCheck::CompactionChoice] {
        assert_eq!(
            observer.check(&check, "task").passed,
            Measure::unavailable("not exercised: no request passed the compaction line")
        );
    }
}

#[test]
fn a_prompt_past_the_line_without_a_compaction_fails() {
    // 65_536 - 8_192 - 982 = 56_362; fresh input is 416.
    const READS: u64 = 55_946;
    const WRITES: u64 = 1;
    let records = edited_threshold_records(|record| {
        if record["type"] != "response.completed" || record["response"] != 6 {
            return false;
        }
        record["usage"]["cache_read_tokens"] = READS.into();
        record["usage"]["cache_write_tokens"] = WRITES.into();
        true
    });
    assert_eq!(super::observe(&records).check(&compaction_opened(), "task").passed, Measure::Observed { value: false });
}

#[test]
fn a_prompt_between_the_threshold_and_the_line_is_no_failure() {
    const ABOVE_THRESHOLD_READS: u64 = 52_013; // 416 + 52_013 > 65_536 * 4 / 5.
    const AT_LINE_READS: u64 = 55_946; // 416 + 55_946 = 65_536 - 8_192 - 982.
    for reads in [ABOVE_THRESHOLD_READS, AT_LINE_READS] {
        let records = edited_threshold_records(|record| {
            if record["type"] != "response.completed" || record["response"] != 6 {
                return false;
            }
            record["usage"]["cache_read_tokens"] = reads.into();
            true
        });
        let observer = super::observe(&records);
        for check in [compaction_opened(), EventCheck::CompactionChoice] {
            assert_eq!(
                observer.check(&check, "task").passed,
                Measure::unavailable("not exercised: no request passed the compaction line"),
                "reads={reads}"
            );
        }
    }
}

#[test]
fn a_stream_without_the_material_leaves_the_condition_unexercised() {
    let records = edited_threshold_records(|record| {
        record["type"] == "session.started"
            && record["limits"].as_object_mut().expect("effective limits").remove("run.compaction_material").is_some()
    });
    let observer = super::observe(&records);
    for check in [compaction_opened(), EventCheck::CompactionChoice] {
        assert_eq!(
            observer.check(&check, "task").passed,
            Measure::unavailable("not exercised: session.started lacks run.compaction_material")
        );
    }
}

fn edited_threshold_records(mut edit: impl FnMut(&mut serde_json::Value) -> bool) -> Vec<Record> {
    let mut edited = 0;
    let records = include_str!("../recorded/compaction-threshold-8a5bb9ec-03.jsonl")
        .lines()
        .map(|line| {
            let mut record = serde_json::from_str(line).expect("recorded event");
            edited += usize::from(edit(&mut record));
            super::record(&serde_json::to_string(&record).expect("edited record"))
        })
        .collect();
    assert_eq!(edited, 1);
    records
}
