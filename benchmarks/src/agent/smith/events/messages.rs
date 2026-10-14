//! Native message identities settle admission refusals or one read/unread event.
//! A refused duplicate name is its own admission attempt, never the terminal
//! of an already admitted message (protocol/events.md, section3.7).

use std::collections::{BTreeMap, BTreeSet};

use smith_events::Event;

use crate::Measure;

use super::{Observer, Session};

pub(super) fn terminals(session: &Session, require_read: bool) -> Vec<String> {
    let mut active = BTreeSet::new();
    let mut problems = final_fences(session);
    for record in &session.records {
        match &record.event {
            Event::MessageReceived(value) => {
                if value.refused.is_none() && !active.insert((value.run, value.name)) {
                    problems.push(format!("duplicate admitted message {}/{}", value.run, value.name));
                }
            }
            Event::MessageRead(value) => {
                if !active.remove(&(value.run, value.name)) {
                    problems.push(format!("read without one admission {}/{}", value.run, value.name));
                }
            }
            Event::MessageUnread(value) => {
                if !active.remove(&(value.run, value.name)) || require_read {
                    problems.push(format!("unread message {}/{}", value.run, value.name));
                }
            }
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
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::Notice(_) => {}
        }
    }
    problems.extend(active.into_iter().map(|(run, name)| format!("message lacks terminal {run}/{name}")));
    problems
}

fn final_fences(session: &Session) -> Vec<String> {
    let mut read = BTreeMap::new();
    let mut unread = BTreeMap::<u64, u64>::new();
    let mut completed = Vec::new();
    for record in &session.records {
        match &record.event {
            Event::MessageRead(value) => {
                read.insert(value.run, value.name);
            }
            Event::MessageUnread(value) => {
                let count = unread.entry(value.run).or_default();
                *count = count.checked_add(1).expect("retained terminal count fits");
            }
            Event::RunCompleted(value) => completed.push(value),
            Event::MessageReceived(_)
            | Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::RunStarted(_)
            | Event::ConversationOpened(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::Notice(_) => {}
        }
    }
    let mut problems = Vec::new();
    for value in completed {
        if value.read != read.get(&value.run).copied() {
            problems.push(format!("final read fence disagrees for run {}", value.run));
        }
        if value.unread != unread.get(&value.run).copied().unwrap_or(0) {
            problems.push(format!("final unread count disagrees for run {}", value.run));
        }
    }
    problems
}

fn before_receipt(captured: bool) -> Measure<bool> {
    if captured {
        Measure::Observed { value: false }
    } else {
        Measure::unavailable("task receipt text was not captured before the response or wait")
    }
}

pub(super) fn before_task(observer: &Observer, task: &str) -> Measure<bool> {
    if task.trim_end().is_empty() {
        return Measure::unavailable("the task text is empty");
    }
    let mut captured = false;
    for session in &observer.sessions {
        for record in &session.records {
            match &record.event {
                Event::MessageReceived(value) => {
                    if let Some(text) = &value.text {
                        captured = true;
                        if text
                            .as_ref()
                            .windows(task.trim_end().len())
                            .any(|window| window == task.trim_end().as_bytes())
                        {
                            return Measure::Observed { value: true };
                        }
                    }
                }
                Event::ResponseStarted(_) | Event::ResponseCompleted(_) => return before_receipt(captured),
                Event::ToolStarted(value) if value.tool.as_ref() == b"wait" => {
                    return before_receipt(captured);
                }
                Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Budget(_)
                | Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunStarted(_)
                | Event::RunCompleted(_)
                | Event::ConversationOpened(_)
                | Event::ConversationClosed(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::JobEnded(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::Notice(_) => {}
            }
        }
    }
    if captured {
        Measure::Observed { value: false }
    } else {
        Measure::unavailable("task receipt text was not captured")
    }
}

pub(super) fn per_exchange(observer: &Observer) -> Measure<bool> {
    let mut messages = BTreeMap::<(usize, u64), u64>::new();
    let mut completions = BTreeMap::<(usize, u64), u64>::new();
    let mut main = BTreeSet::new();
    let mut completed = BTreeSet::new();
    let mut person_seen = false;
    let mut uncaptured = false;
    for (index, session) in observer.sessions.iter().enumerate() {
        for record in &session.records {
            match &record.event {
                Event::MessageReceived(value) => {
                    if value.label.as_deref() == Some(b"person") && value.refused.is_none() {
                        person_seen = true;
                        *messages.entry((index, value.run)).or_default() += 1;
                    }
                    uncaptured |= value.refused.is_none() && value.label.is_none();
                }
                Event::ConversationOpened(value) if value.kind == smith_events::ConversationKind::Main => {
                    main.insert((index, value.run, value.conversation));
                }
                Event::ResponseCompleted(value) if value.outcome == smith_events::Outcome::Completed => {
                    if main.contains(&(index, value.run, value.conversation))
                        && completed.insert((index, value.run, value.response))
                    {
                        *completions.entry((index, value.run)).or_default() += 1;
                    }
                }
                Event::MessageRead(_)
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
                | Event::ToolStarted(_)
                | Event::JobEnded(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::Notice(_) => {}
            }
        }
    }
    if uncaptured {
        Measure::unavailable("person labels were not captured for every received exchange")
    } else {
        Measure::Observed { value: person_seen && messages == completions }
    }
}

pub(super) fn counts(sessions: &[Session]) -> BTreeMap<String, Measure<u64>> {
    let missing: Vec<_> = sessions.iter().filter_map(super::usage::incomplete).collect();
    let mut counts = BTreeMap::<String, u64>::new();
    for session in sessions {
        for record in &session.records {
            let terminal = match &record.event {
                Event::MessageReceived(value) if value.refused.is_some() => Some("refused"),
                Event::MessageRead(_) => Some("read"),
                Event::MessageUnread(_) => Some("unread"),
                Event::MessageReceived(_)
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
                | Event::ToolStarted(_)
                | Event::JobEnded(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::Notice(_) => None,
            };
            if let Some(name) = terminal {
                let count = counts.entry(name.into()).or_default();
                *count = count.checked_add(1).expect("in-memory terminal count fits u64");
            }
        }
    }
    counts
        .into_iter()
        .map(|(name, value)| {
            (
                name,
                if missing.is_empty() {
                    Measure::Observed { value }
                } else {
                    Measure::LowerBound { value, missing: missing.clone() }
                },
            )
        })
        .collect()
}
