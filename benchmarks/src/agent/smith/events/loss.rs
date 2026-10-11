//! The emitted inventory reconciles known type counts with destination loss.
//! Unknown record types remain skipped; their per-type cardinalities cannot be
//! reconstructed. Contract: protocol/events.md sections3.2/5.1/5.2.

use std::collections::BTreeMap;

use crate::Measure;

use super::Session;

pub(super) fn compare(session: &Session) -> Measure<bool> {
    let Some(ended) = &session.ended else {
        return Measure::unavailable("session.ended did not report emitted cardinalities");
    };
    let mut read = BTreeMap::<&[u8], u64>::new();
    for record in &session.records {
        *read.entry(super::names::event(&record.event)).or_default() += 1;
    }
    let mut emitted = BTreeMap::new();
    for count in &ended.emitted {
        if emitted.insert(count.record.as_ref(), count.count).is_some() {
            return Measure::Observed { value: false };
        }
    }
    for (name, count) in &read {
        if emitted.get(name).copied().unwrap_or(0) < *count {
            return Measure::Observed { value: false };
        }
    }
    let mut missing = 0_u128;
    let mut unknown = session.unknown;
    for (name, made) in emitted {
        let received = read.get(name).copied().unwrap_or(0);
        if received > made {
            return Measure::Observed { value: false };
        }
        if matches!(name, b"session.started" | b"run.completed" | b"session.ended") && received != made {
            return Measure::Observed { value: false };
        }
        let known = matches!(
            name,
            b"session.started"
                | b"session.ended"
                | b"run.started"
                | b"run.completed"
                | b"conversation.opened"
                | b"conversation.closed"
                | b"response.started"
                | b"response.completed"
                | b"text.delta"
                | b"tool.started"
                | b"tool.completed"
                | b"check.started"
                | b"check.completed"
                | b"message.received"
                | b"message.read"
                | b"message.unread"
                | b"budget"
                | b"notice"
        );
        if known {
            missing += u128::from(made - received);
        } else {
            unknown = true;
        }
    }
    if missing > u128::from(ended.loss.events) {
        return Measure::Observed { value: false };
    }
    if unknown {
        Measure::unavailable("unknown record types prevent a complete per-type emitted/loss comparison")
    } else {
        Measure::Observed { value: missing == u128::from(ended.loss.events) }
    }
}
