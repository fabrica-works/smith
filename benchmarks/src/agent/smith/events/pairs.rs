//! Pairing keeps observed contradictions separate from absent records.
//! Missing terminals may be lost; duplicate response identities and changed
//! tool names cannot be repaired by another record (protocol/events.md section3).

use std::collections::{BTreeMap, BTreeSet};

use smith_events::{Event, Record};

use super::Session;

/// Outside pairing evidence used by the passive reader.
pub(super) struct Pairing {
    pub violations: Vec<String>,
    pub missing: Vec<String>,
}

fn pair_key(record: &Record) -> Option<(String, bool)> {
    match &record.event {
        Event::ResponseStarted(value) => {
            Some((format!("response/{}/{}/{}", value.run, value.conversation, value.response), true))
        }
        Event::ResponseCompleted(value) => {
            Some((format!("response/{}/{}/{}", value.run, value.conversation, value.response), false))
        }
        Event::ConversationOpened(value) => Some((format!("conversation/{}/{}", value.run, value.conversation), true)),
        Event::ConversationClosed(value) => Some((format!("conversation/{}/{}", value.run, value.conversation), false)),
        Event::ToolStarted(value) => {
            Some((format!("tool/{}/{}/{}", value.run, value.conversation, String::from_utf8_lossy(&value.call)), true))
        }
        Event::ToolCompleted(value) => {
            Some((format!("tool/{}/{}/{}", value.run, value.conversation, String::from_utf8_lossy(&value.call)), false))
        }
        Event::CheckStarted(value) => Some((format!("check/{}", value.run), true)),
        Event::CheckCompleted(value) => Some((format!("check/{}", value.run), false)),
        Event::JobEnded(_)
        | Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
        | Event::Budget(_)
        | Event::SessionStarted(_)
        | Event::SessionEnded(_)
        | Event::RunStarted(_)
        | Event::RunCompleted(_)
        | Event::TextDelta(_)
        | Event::Notice(_) => None,
    }
}

pub(super) fn check(session: &Session) -> Pairing {
    let mut active = BTreeSet::new();
    let mut completed = BTreeSet::new();
    let mut tools = BTreeMap::new();
    let mut problems = Pairing { violations: Vec::new(), missing: Vec::new() };
    for record in &session.records {
        if let Some((key, started)) = pair_key(record) {
            if started {
                if !active.insert(key.clone()) || (completed.contains(&key) && !key.starts_with("check/")) {
                    if key.starts_with("check/") {
                        problems.missing.push(format!("check start before observed terminal {key}"));
                    } else {
                        problems.violations.push(format!("duplicate start {key}"));
                    }
                }
            } else {
                if !active.remove(&key) {
                    if completed.contains(&key) && !key.starts_with("check/") {
                        problems.violations.push(format!("duplicate terminal {key}"));
                    } else {
                        problems.missing.push(format!("terminal without its start {key}"));
                    }
                }
                completed.insert(key);
            }
        }
        match &record.event {
            Event::ToolStarted(value) => {
                tools.insert((value.run, value.conversation, value.call.clone()), value.tool.clone());
            }
            Event::ToolCompleted(value) => {
                if let Some(tool) = tools.get(&(value.run, value.conversation, value.call.clone()))
                    && tool != &value.tool
                {
                    problems.violations.push("tool terminal changed its call's tool".into());
                }
            }
            Event::JobEnded(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::RunCompleted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::Notice(_) => {}
        }
    }
    problems.missing.extend(active.into_iter().map(|key| format!("start without terminal {key}")));
    problems
}
