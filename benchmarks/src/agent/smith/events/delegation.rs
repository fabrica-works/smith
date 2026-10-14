//! Reported delegation only, from protocol/events.md, sections 3.4 and 3.6.

use std::collections::{BTreeMap, BTreeSet};

use smith_events::{ConversationKind, ConversationOpened, Event};

use crate::{Child, ChildStop, Clamp, Delegation, Measure, Requested, Share, ShareBound, SharePart};

use super::{Observer, Session};

fn child(opened: &ConversationOpened, session: &Session) -> Result<Child, String> {
    let share = opened.share.as_ref().ok_or("child opening did not report a share")?;
    let mut clamped = Vec::new();
    for clamp in &opened.clamped {
        let part = match &clamp.part {
            smith_events::SharePart::Turns => SharePart::Turns,
            smith_events::SharePart::Time => SharePart::Time,
            smith_events::SharePart::Unknown(value) => {
                return Err(format!("unknown share part {}", String::from_utf8_lossy(value)));
            }
        };
        let by = match &clamp.by {
            smith_events::ShareBound::Reserve => ShareBound::Reserve,
            smith_events::ShareBound::Deadline => ShareBound::Deadline,
            smith_events::ShareBound::Unknown(value) => {
                return Err(format!("unknown share bound {}", String::from_utf8_lossy(value)));
            }
        };
        clamped.push(Clamp { part, by });
    }
    let mut terminals = session.records.iter().filter_map(|record| match &record.event {
        Event::ToolCompleted(tool)
            if tool.run == opened.run
                && Some(tool.conversation) == opened.parent
                && tool.child.as_ref().is_some_and(|child| child.conversation == opened.conversation)
                && opened.call.as_deref() == Some(tool.call.as_ref()) =>
        {
            tool.child.as_ref()
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
        | Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
        | Event::Notice(_) => None,
    });
    let terminal = terminals.next().ok_or("child tool terminal not reported")?;
    if terminals.next().is_some() {
        return Err("duplicate child tool terminal".into());
    }
    let stop = match &terminal.stop {
        smith_events::ChildStop::End => ChildStop::End,
        smith_events::ChildStop::Budget => ChildStop::Budget,
        smith_events::ChildStop::Failed => ChildStop::Failed,
        smith_events::ChildStop::Closed => ChildStop::Closed,
        smith_events::ChildStop::Unknown(value) => {
            return Err(format!("unknown child stop {}", String::from_utf8_lossy(value)));
        }
    };
    Ok(Child {
        conversation: opened.conversation,
        requested: opened.requested.as_ref().map(|ask| Requested { turns: ask.turns, seconds: ask.seconds }),
        share: Share { turns: share.turns, time_ms: share.time_ms, spend: share.spend },
        clamped,
        stop,
        turns: terminal.turns,
        time_ms: terminal.time_ms,
    })
}

fn children(observer: &Observer) -> Result<Vec<Child>, String> {
    let mut children = Vec::new();
    for session in &observer.sessions {
        let mut seen = BTreeSet::new();
        for record in &session.records {
            if let Event::ConversationOpened(opened) = &record.event {
                match &opened.kind {
                    ConversationKind::Main | ConversationKind::Compaction => {}
                    ConversationKind::Child => {
                        if !seen.insert((opened.run, opened.conversation)) {
                            return Err("duplicate child opening".into());
                        }
                        children.push(child(opened, session)?);
                    }
                    ConversationKind::Unknown(value) => {
                        return Err(format!("unknown conversation kind {}", String::from_utf8_lossy(value)));
                    }
                }
            }
        }
    }
    Ok(children)
}

fn turns(observer: &Observer) -> Result<u64, String> {
    let mut total = 0_u64;
    let mut saw_run = false;
    for session in &observer.sessions {
        let mut seen = BTreeSet::new();
        for record in &session.records {
            if let Event::RunCompleted(run) = &record.event {
                if !seen.insert(run.run) {
                    return Err("duplicate authoritative run turns".into());
                }
                saw_run = true;
                total = total.checked_add(run.turns).ok_or("main turns overflow")?;
            }
        }
    }
    if saw_run { Ok(total) } else { Err("run.completed turns not reported".into()) }
}

fn spans(session: &Session) -> Result<u64, String> {
    let mains: BTreeSet<_> = session
        .records
        .iter()
        .filter_map(|record| match &record.event {
            Event::ConversationOpened(opened) if opened.kind == ConversationKind::Main => {
                Some((opened.run, opened.conversation))
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
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => None,
        })
        .collect();
    let mut started = BTreeMap::new();
    let mut spans = Vec::new();
    for record in &session.records {
        match &record.event {
            Event::ToolStarted(tool)
                if tool.tool.as_ref() == b"sub_agent" && mains.contains(&(tool.run, tool.conversation)) =>
            {
                let previous = started.insert((tool.run, tool.conversation, tool.call.clone()), record.t_ms);
                if previous.is_some() {
                    return Err("duplicate main sub_agent start".into());
                }
            }
            Event::ToolCompleted(tool)
                if tool.tool.as_ref() == b"sub_agent" && mains.contains(&(tool.run, tool.conversation)) =>
            {
                let start = started
                    .remove(&(tool.run, tool.conversation, tool.call.clone()))
                    .ok_or("main sub_agent start not reported")?;
                if start > record.t_ms {
                    return Err("main sub_agent timestamps reversed".into());
                }
                spans.push((start, record.t_ms));
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
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => {}
        }
    }
    if !started.is_empty() {
        return Err("main sub_agent terminal not reported".into());
    }
    Ok(blocked_ms(&mut spans))
}

/// Union of [start, end) spans on this process stream's monotonic clock.
fn blocked_ms(spans: &mut [(u64, u64)]) -> u64 {
    spans.sort_unstable();
    let (mut total, mut open) = (0_u64, None::<(u64, u64)>);
    for &(start, end) in spans.iter() {
        open = match open {
            Some((from, to)) if start <= to => Some((from, to.max(end))),
            Some((from, to)) => {
                total += to - from;
                Some((start, end))
            }
            None => Some((start, end)),
        };
    }
    total + open.map_or(0, |(from, to)| to - from)
}

pub(crate) fn delegation(observer: &Observer) -> Delegation {
    if observer.fault.is_some()
        || observer.unknown
        || observer.sessions.is_empty()
        || observer.sessions.iter().any(|session| super::usage::incomplete(session).is_some())
    {
        return Delegation::unavailable("incomplete or lost event stream prevents exact delegation");
    }
    let children = children(observer).map_or_else(Measure::unavailable, |value| Measure::Observed { value });
    let turns = turns(observer).map_or_else(Measure::unavailable, |value| Measure::Observed { value });
    let blocked = observer.sessions.iter().try_fold(0_u64, |total, session| {
        total.checked_add(spans(session)?).ok_or_else(|| "parent blocked time overflow".to_string())
    });
    Delegation {
        turns,
        children,
        parent_blocked_ms: blocked.map_or_else(Measure::unavailable, |value| Measure::Observed { value }),
    }
}
