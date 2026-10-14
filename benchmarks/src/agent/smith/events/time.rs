//! Main spans use matching event identities, never child model time.
//! Completed streams account for overlap (benchmarks.md, section 8.1).

use super::{Observer, Session};
use crate::time_record::{Breakdown, observed};
use crate::{Measure, Timing};
use smith_events::{ConversationKind, ConversationOpened, Event};
use std::collections::BTreeMap;

impl Observer {
    /// Fill main-conversation durations only from a complete, lossless event stream.
    pub fn timing(&self, timing: &mut Timing) {
        timing.record = crate::TimeRecord::unavailable("main event spans not reported completely");
        let Some(session) = self.sessions.last() else { return };
        if self.unknown || session.unknown || session.ended.as_ref().is_none_or(|end| end.loss.events != 0) {
            timing.record = crate::TimeRecord::unavailable("event stream is incomplete or has loss");
            return;
        }
        let Some((main, start)) = main(session) else { return };
        let mut spans = Breakdown::default();
        let mut requests = BTreeMap::new();
        let mut calls = BTreeMap::new();
        let mut compactions = BTreeMap::new();
        let mut first_byte = Some(0_u64);
        let mut largest_gap = Some(0_u64);
        let mut longest = 0;
        let mut end = None;
        for record in &session.records {
            match &record.event {
                Event::ResponseStarted(response)
                    if response.run == main.run && response.conversation == main.conversation =>
                {
                    requests.insert(response.response, record.t_ms);
                }
                Event::ResponseCompleted(response)
                    if response.run == main.run && response.conversation == main.conversation =>
                {
                    let Some(start) = requests.remove(&response.response) else { return };
                    spans.requests.push((start, record.t_ms));
                    first_byte = first_byte.and_then(|sum| sum.checked_add(response.first_byte_ms?));
                    largest_gap = largest_gap.and_then(|sum| sum.checked_add(response.largest_gap_ms?));
                    longest = longest.max(response.duration_ms);
                }
                Event::ToolStarted(tool) if tool.run == main.run && tool.conversation == main.conversation => {
                    let name = String::from_utf8_lossy(&tool.tool).into_owned();
                    spans.tool(&name, (record.t_ms, record.t_ms));
                    calls.insert(tool.call.clone(), record.t_ms);
                }
                Event::ToolCompleted(tool) if tool.run == main.run && tool.conversation == main.conversation => {
                    let Some(start) = calls.remove(&tool.call) else { return };
                    let name = String::from_utf8_lossy(&tool.tool);
                    if name == "sub_agent" {
                        spans.children.push((start, record.t_ms));
                    }
                    spans.tool(&name, (start, record.t_ms));
                }
                Event::ConversationOpened(opened)
                    if opened.run == main.run
                        && opened.parent == Some(main.conversation)
                        && opened.kind == ConversationKind::Compaction =>
                {
                    compactions.insert(opened.conversation, record.t_ms);
                }
                Event::ConversationClosed(closed) if closed.run == main.run => {
                    if let Some(start) = compactions.remove(&closed.conversation) {
                        spans.compaction.push((start, record.t_ms));
                    }
                }
                Event::RunCompleted(run) if run.run == main.run => end = Some(record.t_ms),
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
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::JobEnded(_)
                | Event::Notice(_) => {}
            }
        }
        let Some(end) = end else { return };
        if !requests.is_empty() || !calls.is_empty() || !compactions.is_empty() {
            return;
        }
        timing.record = spans.finish((start, end));
        timing.record.first_byte_ms = observed(first_byte);
        timing.record.largest_gap_ms = observed(largest_gap);
        timing.record.commands_outside_calls_ms = Measure::unavailable("command process spans not reported");
        timing.first_byte_ms = timing.record.first_byte_ms.clone();
        timing.largest_gap_ms = timing.record.largest_gap_ms.clone();
        timing.longest_completion_ms = Measure::Observed { value: longest };
        timing.agent_t_ms = Measure::Observed { value: end };
    }
}

fn main(session: &Session) -> Option<(&ConversationOpened, u64)> {
    session.records.iter().find_map(|record| match &record.event {
        Event::ConversationOpened(opened) if opened.kind == ConversationKind::Main => Some((opened, record.t_ms)),
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
        | Event::ToolCompleted(_)
        | Event::CheckStarted(_)
        | Event::CheckCompleted(_)
        | Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
        | Event::JobEnded(_)
        | Event::Notice(_) => None,
    })
}
