//! A capped child's returned text is joined to its last native request.
//! Tool choice and captured answer are observations, never inferred from turns
//! (benchmarks.md, section 11.1; protocol/events.md, sections 3.5 and 3.6).

use std::collections::BTreeMap;

use smith_events::{Block, ChildStop, ConversationKind, Event, Outcome, ToolChoice};

use crate::Measure;

use super::Observer;

pub(super) fn check(observer: &Observer) -> Measure<bool> {
    let mut measured = false;
    for session in &observer.sessions {
        let mut children = BTreeMap::new();
        let mut requests = BTreeMap::new();
        let mut responses = BTreeMap::new();
        for record in &session.records {
            if let Event::ConversationOpened(value) = &record.event
                && value.kind == ConversationKind::Child
            {
                children.insert((value.run, value.conversation), value);
            }
            if let Event::ResponseStarted(value) = &record.event {
                requests.insert((value.run, value.conversation), value);
            }
            if let Event::ResponseCompleted(value) = &record.event {
                responses.insert((value.run, value.conversation), value);
            }
            if let Event::ToolCompleted(value) = &record.event
                && let Some(child) = &value.child
            {
                match &child.stop {
                    ChildStop::Budget => {}
                    ChildStop::End | ChildStop::Failed | ChildStop::Closed => continue,
                    ChildStop::Unknown(_) => return Measure::unavailable("child stop classification is unknown"),
                }
                let identity = (value.run, child.conversation);
                let Some(opening) = children.get(&identity) else {
                    return Measure::unavailable("capped child lacks its actual opening");
                };
                if opening.parent != Some(value.conversation) || opening.call.as_ref() != Some(&value.call) {
                    return Measure::unavailable("capped child return does not match its parent call");
                }
                let Some(request) = requests.get(&identity) else {
                    return Measure::Observed { value: false };
                };
                match &request.tool_choice {
                    ToolChoice::None => {}
                    ToolChoice::Auto | ToolChoice::Only => return Measure::Observed { value: false },
                    ToolChoice::Unknown(_) => return Measure::unavailable("final request tool choice is unknown"),
                }
                let Some(response) = responses.get(&identity) else {
                    return Measure::Observed { value: false };
                };
                if request.response != response.response || response.outcome != Outcome::Completed {
                    return Measure::Observed { value: false };
                }
                let Some(completion) = &response.completion else {
                    return Measure::unavailable("capped child answer text was not captured");
                };
                let text = completion.blocks.iter().any(|block| match block {
                    Block::Text { text } | Block::Refusal { text } => !text.is_empty(),
                    Block::Unknown { .. }
                    | Block::Call { .. }
                    | Block::Oversized { .. }
                    | Block::Cut { .. }
                    | Block::Result { .. }
                    | Block::Opaque { .. } => false,
                });
                if !text {
                    return Measure::Observed { value: false };
                }
                measured = true;
            }
        }
    }
    Measure::Observed { value: measured }
}
