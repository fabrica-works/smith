//! Synthetic codec records exercise the two visibility deciders, never a live run.

use smith_bench::{EventCheck, Measure};
use smith_events::{Event, Record};

pub mod common;

fn records() -> Vec<Record> {
    static TEMPLATE: std::sync::LazyLock<Vec<Record>> = std::sync::LazyLock::new(build_records);
    TEMPLATE.clone()
}

fn build_records() -> Vec<Record> {
    let baseline = common::events::balanced();
    let mut values = baseline[..5].to_vec();
    let budget = Record {
        t_ms: 5,
        event: Event::Budget(smith_events::BudgetStanding {
            call: None,
            run: 0,
            stage: smith_events::BudgetStage::Ordinary,
            turns: smith_events::Quantity { used: 2, limit: 10 },
            spend: smith_events::Quantity { used: 3, limit: 100 },
            time_ms: smith_events::Quantity { used: 1500, limit: 10000 },
            reserve: smith_events::Reserve { turns: 2, time_ms: 2000, spend: 10 },
        }),
    };
    let mut opened = baseline[2].clone();
    if let Event::ConversationOpened(value) = &mut opened.event {
        value.kind = smith_events::ConversationKind::Child;
        value.conversation = 1;
        value.parent = Some(0);
        value.call = Some(b"child0".as_slice().into());
        value.share = Some(smith_events::Share { turns: 2, time_ms: 1000, spend: 10 });
    }
    let mut returned =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"));
    if let Event::ToolCompleted(value) = &mut returned.event {
        value.run = 0;
        value.conversation = 0;
        value.call = b"child0".as_slice().into();
        value.child = Some(smith_events::ChildCompleted {
            conversation: 1,
            stop: smith_events::ChildStop::End,
            turns: 1,
            time_ms: 100,
            spent: 0,
            usage: smith_events::Usage {
                input_tokens: Some(0),
                cache_read_tokens: Some(0),
                cache_write_tokens: Some(0),
                output_tokens: Some(0),
                reasoning_tokens: None,
            },
        });
    }
    let mut request = baseline[3].clone();
    if let Event::ResponseStarted(value) = &mut request.event {
        value.response = 1;
        let prompt = value.prompt.as_mut().expect("captured request");
        prompt.messages = skein_lib::List::with_capacity(1);
        let mut blocks = skein_lib::List::with_capacity(1);
        blocks.push(smith_events::Block::Result {
            call: b"child0".as_slice().into(), error: false,
            text: b"Survey done.\n\nChild usage: used 1 turns and less than a second; stopped for budget: no; ordinary left 6 turns and 6 seconds.".as_slice().into(),
        }).expect("one returned child result");
        prompt.messages.push(smith_events::Message { role: smith_events::Role::User, blocks }).expect("one message");
    }
    let mut response = baseline[4].clone();
    if let Event::ResponseCompleted(value) = &mut response.event {
        value.response = 1;
    }
    values.extend([budget, opened, returned, request, response]);
    values.extend_from_slice(&baseline[5..]);
    report_text(&mut values, "ALPHA 6 6");
    values
}

fn report_text(records: &mut [Record], text: &str) {
    for record in records {
        if let Event::RunCompleted(value) = &mut record.event {
            value.result.as_mut().expect("accepted result").text = Some(text.as_bytes().into());
        }
    }
}

fn told_text(records: &mut [Record], turns: u64, time: &str) {
    for record in records {
        if let Event::ResponseStarted(value) = &mut record.event
            && value.response == 1
        {
            let block = value
                .prompt
                .as_mut()
                .expect("request")
                .messages
                .get_mut(0)
                .expect("message")
                .blocks
                .get_mut(0)
                .expect("result");
            if let smith_events::Block::Result { text, .. } = block {
                *text = format!("Survey done.\n\nChild usage: used 1 turns and less than a second; stopped for budget: no; ordinary left {turns} turns and {time}.").as_bytes().into();
            }
        }
    }
}

fn sequence(records: &mut [Record]) {
    for (position, record) in records.iter_mut().enumerate() {
        record.t_ms = u64::try_from(position).expect("small fixture");
    }
}

fn observed(records: &[Record], check: &EventCheck) -> Measure<bool> {
    let bytes = common::events::bytes(records);
    let ceiling = u32::try_from(bytes.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("records"))
        .expect("bounded fixture lines");
    let mut observer = smith_bench::agent::smith::events::Observer::with_ceiling(&common::events::LIMITS, ceiling)
        .expect("reader with the exact encoded ceiling and unchanged semantic limits");
    observer.feed(&bytes).expect("outside codec records");
    observer.finish().expect("whole stream");
    observer.check(check, "task").passed
}

fn checked(records: &mut [Record], check: &EventCheck) -> Measure<bool> {
    sequence(records);
    observed(records, check)
}

fn writing_budget(records: &mut [Record]) {
    if let Event::Budget(value) = &mut records[5].event {
        value.call = Some(b"child0".as_slice().into());
    }
}

#[test]
fn told_values_equal_their_writing_steps_budget() {
    for (used_ms, time) in [(1500, "6 seconds"), (7000, "1 second"), (7999, "less than a second")] {
        let mut values = records();
        writing_budget(&mut values);
        if let Event::Budget(value) = &mut values[5].event {
            value.time_ms.used = used_ms;
        }
        told_text(&mut values, 6, time);
        assert_eq!(checked(&mut values, &EventCheck::BudgetTold), Measure::Observed { value: true });
    }
}

#[test]
fn a_told_value_off_its_writing_step_fails() {
    for (turns, time) in [(7, "6 seconds"), (6, "7 seconds")] {
        let mut values = records();
        writing_budget(&mut values);
        told_text(&mut values, turns, time);
        report_text(&mut values, &format!("ALPHA {turns} {}", if turns == 7 { 6 } else { 7 }));
        assert_eq!(checked(&mut values, &EventCheck::BudgetTold), Measure::Observed { value: false });
        assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: true });
    }
}

#[test]
fn a_return_without_the_join_stays_unavailable() {
    for field in ["call", "run", "prompt", "usage"] {
        let mut values = records();
        writing_budget(&mut values);
        match field {
            "call" => {
                if let Event::Budget(value) = &mut values[5].event {
                    value.call = Some(b"foreign".as_slice().into());
                }
            }
            "run" => {
                if let Event::Budget(value) = &mut values[5].event {
                    value.run = 1;
                }
            }
            "prompt" => {
                if let Event::ResponseStarted(value) = &mut values[8].event {
                    value.prompt = None;
                }
            }
            "usage" => told_text(&mut values, 6, "unknown"),
            _ => unreachable!("fixture selector"),
        }
        assert!(matches!(checked(&mut values, &EventCheck::BudgetTold), Measure::Unavailable { .. }), "{field}");
    }
}

#[test]
fn model_report_compares_actual_last_told_values_without_a_budget_inference() {
    let mut values = records();
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: true });
    assert!(matches!(checked(&mut values, &EventCheck::BudgetTold), Measure::Unavailable { .. }));
    report_text(&mut values, "ALPHA 7 6");
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: false });
    told_text(&mut values, 7, "6 seconds");
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: true });
    values.retain(|record| !matches!(record.event, Event::Budget(_)));
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: true });
    assert!(matches!(checked(&mut values, &EventCheck::BudgetTold), Measure::Unavailable { .. }));
}

#[test]
fn writing_step_absence_stale_and_coincident_snapshots_cannot_prove_smith_told_values() {
    for change in ["same-rounded", "stale", "same-time", "absent", "wrong-run", "later"] {
        let mut values = records();
        match change {
            "same-rounded" | "same-time" => {}
            "stale" => {
                if let Event::Budget(value) = &mut values[5].event {
                    value.time_ms.used = 1499;
                }
            }
            "absent" => values.retain(|record| !matches!(record.event, Event::Budget(_))),
            "wrong-run" => {
                if let Event::Budget(value) = &mut values[5].event {
                    value.run = 1;
                }
            }
            "later" => {
                let budget = values.remove(5);
                values.insert(values.len() - 1, budget);
            }
            _ => unreachable!("fixture selector"),
        }
        sequence(&mut values);
        if change == "same-time" {
            values[5].t_ms = values[7].t_ms;
            values[6].t_ms = values[7].t_ms;
        }
        assert!(matches!(observed(&values, &EventCheck::BudgetTold), Measure::Unavailable { .. }), "{change}");
        assert_eq!(observed(&values, &EventCheck::BudgetReport), Measure::Observed { value: true }, "{change}");
    }
}

#[test]
fn model_report_keeps_the_last_told_value_and_excludes_post_report_budget_changes() {
    let mut values = records();
    told_text(&mut values, 6, "less than a second");
    report_text(&mut values, "ALPHA 6 0");
    let mut later = values[5].clone();
    if let Event::Budget(value) = &mut later.event {
        value.turns.used = 9;
        value.time_ms.used = 9000;
    }
    values.insert(values.len() - 1, later);
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: true });
    told_text(&mut values, 6, "1 second");
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: false });
}

#[test]
fn model_report_requires_actual_child_identity_captured_usage_and_result() {
    for field in ["call", "run", "parent", "child", "prompt", "report", "usage"] {
        let mut values = records();
        for record in &mut values {
            match &mut record.event {
                Event::ToolCompleted(value) => match field {
                    "call" => value.call = b"foreign".as_slice().into(),
                    "run" => value.run = 1,
                    "parent" => value.conversation = 1,
                    "child" => value.child.as_mut().expect("child").conversation = 2,
                    _ => {}
                },
                Event::ResponseStarted(value) if value.response == 1 && field == "prompt" => value.prompt = None,
                Event::RunCompleted(value) if field == "report" => value.result.as_mut().expect("result").text = None,
                Event::Budget(_)
                | Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunStarted(_)
                | Event::RunCompleted(_)
                | Event::ConversationOpened(_)
                | Event::ConversationClosed(_)
                | Event::ResponseStarted(_)
                | Event::ResponseCompleted(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::JobEnded(_)
                | Event::Notice(_) => {}
            }
        }
        if field == "usage" {
            told_text(&mut values, 6, "unknown");
        }
        assert!(matches!(checked(&mut values, &EventCheck::BudgetReport), Measure::Unavailable { .. }), "{field}");
    }
}

#[test]
fn model_report_uses_the_last_of_two_actual_child_returns() {
    let mut values = records();
    let mut opened = values[6].clone();
    if let Event::ConversationOpened(value) = &mut opened.event {
        value.conversation = 2;
        value.call = Some(b"child1".as_slice().into());
    }
    let mut returned = values[7].clone();
    if let Event::ToolCompleted(value) = &mut returned.event {
        value.call = b"child1".as_slice().into();
        value.child.as_mut().expect("child").conversation = 2;
    }
    let mut second = records();
    told_text(&mut second, 5, "4 seconds");
    let mut request = second[8].clone();
    if let Event::ResponseStarted(value) = &mut request.event {
        value.response = 2;
        let block = value
            .prompt
            .as_mut()
            .expect("prompt")
            .messages
            .get_mut(0)
            .expect("message")
            .blocks
            .get_mut(0)
            .expect("block");
        if let smith_events::Block::Result { call, .. } = block {
            *call = b"child1".as_slice().into();
        }
    }
    let mut response = values[9].clone();
    if let Event::ResponseCompleted(value) = &mut response.event {
        value.response = 2;
    }
    values.splice(10..10, [opened, returned, request, response]);
    report_text(&mut values, "ALPHA 5 4");
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: true });
    report_text(&mut values, "ALPHA 6 6");
    assert_eq!(checked(&mut values, &EventCheck::BudgetReport), Measure::Observed { value: false });
}

#[test]
fn model_report_requires_capture_of_the_told_text() {
    let mut values = records();
    sequence(&mut values);
    for capture in [smith_events::Capture::None, smith_events::Capture::Calls, smith_events::Capture::Everything] {
        let mut bytes = Vec::new();
        for record in &values {
            let limits = smith_events::Limits { items: 64, ..common::events::LIMITS };
            if let Some(line) = smith_events::write(record, &capture, &limits).expect("typed record") {
                bytes.extend_from_slice(&line);
            }
        }
        let mut observer = smith_bench::agent::smith::events::Observer::new(&common::events::LIMITS).expect("reader");
        observer.feed(&bytes).expect("outside codec records");
        observer.finish().expect("whole stream");
        let report = observer.check(&EventCheck::BudgetReport, "task").passed;
        match capture {
            smith_events::Capture::Everything => assert_eq!(report, Measure::Observed { value: true }),
            smith_events::Capture::None | smith_events::Capture::Calls => {
                assert!(matches!(report, Measure::Unavailable { .. }));
            }
            smith_events::Capture::Unknown(_) => unreachable!("known fixture capture"),
        }
    }
}
