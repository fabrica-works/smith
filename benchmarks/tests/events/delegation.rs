//! Delegation retains only the event face's exact shares, ends and spans.

use smith_bench::{ChildStop, Measure, Share, ShareBound, SharePart};
use smith_events::{Event, Record};

use super::{balanced, child_run, observe, record};

pub(super) fn a_childs_share_stop_turns_and_time_are_recorded() {
    let delegation = observe(&child_run()).delegation();
    let Measure::Observed { value: children } = delegation.children else { panic!("children observed") };
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].share, Share { turns: 2, time_ms: 1000, spend: 10 });
    assert_eq!(children[0].stop, ChildStop::Budget);
    assert_eq!((children[0].turns, children[0].time_ms), (2, 100));
}

pub(super) fn a_clamped_share_names_its_part_and_bound() {
    let mut records = child_run();
    if let Event::ConversationOpened(child) = &mut records[4].event {
        child.clamped = skein_lib::List::with_capacity(1);
        child.requested = Some(smith_events::Requested { turns: Some(5), seconds: Some(3) });
        child
            .clamped
            .push(smith_events::Clamp { part: smith_events::SharePart::Time, by: smith_events::ShareBound::Deadline })
            .expect("clamp capacity");
    }
    let Measure::Observed { value: children } = observe(&records).delegation().children else { panic!("observed") };
    assert_eq!(children[0].requested, Some(smith_bench::Requested { turns: Some(5), seconds: Some(3) }));
    assert_eq!(children[0].clamped, [smith_bench::Clamp { part: SharePart::Time, by: ShareBound::Deadline }]);
}

fn span(call: &str, time: u64, started: bool) -> Record {
    let mut record = if started {
        record(include_str!("../../../crates/smith-events/golden/v4/tool_started_everything.jsonl"))
    } else {
        record(include_str!("../../../crates/smith-events/golden/v4/tool_completed_everything.jsonl"))
    };
    record.t_ms = time;
    if let Event::ToolStarted(value) = &mut record.event {
        value.run = 0;
        value.conversation = 0;
        value.call = call.as_bytes().into();
        value.tool = b"sub_agent".as_slice().into();
    } else if let Event::ToolCompleted(value) = &mut record.event {
        value.run = 0;
        value.conversation = 0;
        value.call = call.as_bytes().into();
        value.tool = b"sub_agent".as_slice().into();
    } else {
        panic!("tool fixture");
    }
    record
}

pub(super) fn parent_blocked_time_is_the_union_of_sub_agent_spans() {
    let mut records = balanced();
    let mut tail = records.split_off(5);
    records.extend([
        span("a", 10, true),
        span("b", 20, true),
        span("a", 40, false),
        span("b", 50, false),
        span("c", 60, true),
        span("c", 70, false),
    ]);
    for (index, record) in tail.iter_mut().enumerate() {
        record.t_ms = 80 + u64::try_from(index).expect("tail");
    }
    records.extend(tail);
    assert_eq!(observe(&records).delegation().parent_blocked_ms, Measure::Observed { value: 50 });
}

pub(super) fn event_loss_leaves_delegation_unavailable() {
    let mut records = child_run();
    if let Event::SessionEnded(ended) = &mut records.last_mut().expect("session terminal").event {
        ended.loss.events = 1;
    }
    let delegation = observe(&records).delegation();
    assert!(matches!(delegation.turns, Measure::Unavailable { .. }));
    assert!(matches!(delegation.children, Measure::Unavailable { .. }));
    assert!(matches!(delegation.parent_blocked_ms, Measure::Unavailable { .. }));
}

pub(super) fn an_unknown_child_stop_leaves_children_unavailable() {
    let mut records = child_run();
    if let Event::ToolCompleted(completed) = &mut records[5].event {
        completed.child.as_mut().expect("child terminal").stop =
            smith_events::ChildStop::Unknown(b"future".as_slice().into());
    }
    assert!(matches!(observe(&records).delegation().children,
        Measure::Unavailable { reason } if reason.contains("future")));
}
