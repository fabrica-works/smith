//! Cache probes keep a fixed read schedule and sequential child returns.
//! Checks require captured calls and parent-return identities
//! (benchmarks.md, section 11.1).

use std::collections::{BTreeMap, BTreeSet};

use smith_events::{Block, ConversationKind, Event};

use crate::Measure;

use super::Observer;

pub(super) fn calls_per_response(observer: &Observer, tool: &str, at_most: u32) -> Measure<bool> {
    let mut captured = false;
    for session in &observer.sessions {
        for record in &session.records {
            if let Event::ResponseCompleted(value) = &record.event {
                if value.outcome == smith_events::Outcome::Failed && value.blocks == 0 {
                    captured = true;
                    continue;
                }
                let Some(completion) = &value.completion else {
                    return Measure::unavailable("response call blocks were not captured");
                };
                captured = true;
                if completion.blocks.iter().any(|block| matches!(block, Block::Unknown { .. })) {
                    return Measure::unavailable("an unknown response block prevents exact call counts");
                }
                let count = completion
                    .blocks
                    .iter()
                    .filter(|block| {
                        matches!(block,
                    Block::Call { tool: actual, .. } if actual.as_ref() == tool.as_bytes())
                    })
                    .count();
                if u64::try_from(count).expect("blocks fit") > u64::from(at_most) {
                    return Measure::Observed { value: false };
                }
            }
        }
    }
    if captured { Measure::Observed { value: true } } else { Measure::unavailable("no captured provider response") }
}

pub(super) fn children_in_turn(observer: &Observer, count: u32) -> Measure<bool> {
    let mut opened_count = 0_u64;
    for session in &observer.sessions {
        let mut openings = BTreeMap::new();
        let mut waiting = BTreeSet::new();
        for record in &session.records {
            if let Event::ConversationOpened(value) = &record.event
                && value.kind == ConversationKind::Child
            {
                if !waiting.is_empty() {
                    return Measure::Observed { value: false };
                }
                let identity = (value.run, value.conversation);
                if openings.insert(identity, value).is_some() {
                    return Measure::unavailable("child opening was repeated");
                }
                waiting.insert(identity);
                opened_count += 1;
            }
            if let Event::ToolCompleted(value) = &record.event
                && let Some(child) = &value.child
            {
                let identity = (value.run, child.conversation);
                let Some(opening) = openings.get(&identity) else {
                    return Measure::unavailable("child return lacks its actual opening");
                };
                let opening: &&smith_events::ConversationOpened = opening;
                if opening.parent != Some(value.conversation)
                    || opening.call.as_ref() != Some(&value.call)
                    || !waiting.remove(&identity)
                {
                    return Measure::Observed { value: false };
                }
            }
        }
        if !waiting.is_empty() {
            return Measure::Observed { value: false };
        }
    }
    Measure::Observed { value: opened_count == u64::from(count) }
}
