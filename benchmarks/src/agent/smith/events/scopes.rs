//! Scope checks join actual conversation identities and emitted child returns.
//! Prompt tokens are the disjoint input/cache counts; absent values stay unknown
//! (protocol/events.md, section3.10; benchmarks.md, section11.1).

use std::collections::{BTreeMap, BTreeSet};

use smith_events::{ChildStop, ConversationKind, Event, Outcome, Status};

use crate::Measure;

use super::Observer;

pub(super) fn prompt_within_window(observer: &Observer) -> Measure<bool> {
    let mut measured = false;
    for session in &observer.sessions {
        let mut models = BTreeMap::new();
        let mut requests = BTreeMap::new();
        for (position, record) in session.records.iter().enumerate() {
            if let Event::ConversationOpened(value) = &record.event {
                models.insert((value.run, value.conversation), &value.model);
            }
            if let Event::ResponseStarted(value) = &record.event {
                if models.get(&(value.run, value.conversation)) != Some(&&value.model) {
                    return Measure::unavailable("request model does not match its actual conversation opening");
                }
                requests.insert((value.run, value.conversation, value.response), value);
            }
            if let Event::ResponseCompleted(value) = &record.event {
                let Some(request) = requests.get(&(value.run, value.conversation, value.response)) else {
                    return Measure::unavailable("response lacks its request's actual model window");
                };
                if value.outcome == Outcome::Failed && value.blocks == 0
                    && session.records[position + 1..].iter().any(|record| matches!(&record.event,
                        Event::ResponseStarted(retry) if retry.run == value.run && retry.conversation == value.conversation
                            && retry.turn == request.turn && retry.attempt > request.attempt))
                {
                    continue;
                }
                let Some(input) = value.usage.input_tokens else {
                    return Measure::unavailable("provider did not report fresh prompt input");
                };
                let Some(read) = value.usage.cache_read_tokens else {
                    return Measure::unavailable("provider did not report cached prompt input");
                };
                let Some(write) = value.usage.cache_write_tokens else {
                    return Measure::unavailable("provider did not report written prompt input");
                };
                measured = true;
                if u128::from(input) + u128::from(read) + u128::from(write) > u128::from(request.model.window_tokens) {
                    return Measure::Observed { value: false };
                }
            }
        }
    }
    Measure::Observed { value: measured }
}

pub(super) fn child_share(observer: &Observer, turns_at_most: u32) -> Measure<bool> {
    let mut measured = false;
    for session in &observer.sessions {
        let mut children = BTreeMap::new();
        let mut returned = BTreeMap::new();
        for record in &session.records {
            if let Event::ConversationOpened(value) = &record.event
                && value.kind == ConversationKind::Child
            {
                let Some(_) = &value.share else {
                    return Measure::unavailable("child opening lacks its effective share");
                };
                children.insert((value.run, value.conversation), value);
            }
            if let Event::ToolCompleted(value) = &record.event
                && let Some(child) = &value.child
            {
                returned.insert((value.run, child.conversation), (value, child));
            }
        }
        for (identity, opening) in children {
            let Some((tool, child)) = returned.get(&identity) else {
                return Measure::Observed { value: false };
            };
            if opening.parent != Some(tool.conversation) || opening.call.as_ref() != Some(&tool.call) {
                return Measure::unavailable("child return does not match its actual parent call");
            }
            let share = opening.share.as_ref().expect("checked effective share");
            measured = true;
            match &child.stop {
                ChildStop::End | ChildStop::Budget => {}
                ChildStop::Failed | ChildStop::Closed => return Measure::Observed { value: false },
                ChildStop::Unknown(_) => return Measure::unavailable("child stop classification is unknown"),
            }
            if child.turns > u64::from(turns_at_most)
                || child.turns > share.turns
                || share.turns > u64::from(turns_at_most)
            {
                return Measure::Observed { value: false };
            }
        }
    }
    Measure::Observed { value: measured }
}

pub(super) fn main_after_child(observer: &Observer) -> Measure<bool> {
    let mut measured = false;
    for session in &observer.sessions {
        let mut main = BTreeSet::new();
        let mut children = BTreeMap::new();
        let mut budgets = BTreeMap::new();
        let mut latest_main = BTreeMap::new();
        let mut latest_child = BTreeMap::new();
        let mut returned_children = BTreeSet::new();
        let mut terminals = BTreeMap::new();
        for (position, record) in session.records.iter().enumerate() {
            if let Event::RunStarted(value) = &record.event {
                budgets.insert(value.run, value.budget.turns);
            }
            if let Event::ConversationOpened(value) = &record.event
                && value.kind == ConversationKind::Main
            {
                main.insert((value.run, value.conversation));
            }
            if let Event::ConversationOpened(value) = &record.event
                && value.kind == ConversationKind::Child
            {
                children.insert((value.run, value.conversation), value);
            }
            if let Event::ResponseCompleted(value) = &record.event
                && value.outcome == Outcome::Completed
                && main.contains(&(value.run, value.conversation))
            {
                latest_main.insert(value.run, position);
            }
            if let Event::ToolCompleted(value) = &record.event
                && value.child.is_some()
            {
                let child = value.child.as_ref().expect("checked child return");
                let Some(opening) = children.get(&(value.run, child.conversation)) else {
                    return Measure::unavailable("child return lacks its actual opening");
                };
                let opening: &&smith_events::ConversationOpened = opening;
                if opening.parent != Some(value.conversation) || opening.call.as_ref() != Some(&value.call) {
                    return Measure::unavailable("child return does not match its actual parent call");
                }
                returned_children.insert((value.run, child.conversation));
                latest_child.insert(value.run, position);
            }
            if let Event::RunCompleted(value) = &record.event {
                terminals.insert(value.run, value);
            }
        }
        if children.keys().any(|identity| !returned_children.contains(identity)) {
            return Measure::Observed { value: false };
        }
        for (run, returned) in latest_child {
            measured = true;
            let Some(limit) = budgets.get(&run) else {
                return Measure::unavailable("child run lacks its opening turn budget");
            };
            if latest_main.get(&run).is_none_or(|position| *position <= returned)
                || terminals.get(&run).is_none_or(|value| value.status != Status::Accepted || value.turns > *limit)
            {
                return Measure::Observed { value: false };
            }
        }
    }
    Measure::Observed { value: measured }
}
