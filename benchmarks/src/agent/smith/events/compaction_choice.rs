//! Compaction conversations retain the tool choice of every actual request.
//! No conversation means the predicate was not exercised; the first request
//! that permits tools supplies failure evidence (benchmarks.md, section 11.1).

use std::collections::BTreeSet;

use smith_events::{ConversationKind, Event, ToolChoice};

use crate::{CheckVerdict, Measure};

use super::Observer;

pub(super) fn check(observer: &Observer) -> CheckVerdict {
    let mut exercised = false;
    for session in &observer.sessions {
        let conversations: BTreeSet<_> = session
            .records
            .iter()
            .filter_map(|record| {
                if let Event::ConversationOpened(opening) = &record.event
                    && opening.kind == ConversationKind::Compaction
                {
                    Some((opening.run, opening.conversation))
                } else {
                    None
                }
            })
            .collect();
        let mut requested = BTreeSet::new();
        for record in &session.records {
            if let Event::ResponseStarted(request) = &record.event
                && conversations.contains(&(request.run, request.conversation))
            {
                requested.insert((request.run, request.conversation));
                exercised = true;
                match request.tool_choice {
                    ToolChoice::None => {}
                    ToolChoice::Auto | ToolChoice::Only => {
                        return CheckVerdict {
                            check: "compaction-choice".into(),
                            passed: Measure::Observed { value: false },
                            evidence: vec![format!(
                                "run={} conversation={} response={} attempt={} tool_choice={:?}",
                                request.run,
                                request.conversation,
                                request.response,
                                request.attempt,
                                request.tool_choice
                            )],
                        };
                    }
                    ToolChoice::Unknown(_) => return unavailable("compaction request tool choice is unknown"),
                }
            }
        }
        if conversations != requested {
            return unavailable("compaction conversation has no retained request");
        }
    }
    if exercised {
        CheckVerdict { check: "compaction-choice".into(), passed: Measure::Observed { value: true }, evidence: vec![] }
    } else {
        match super::threshold::passed_line(observer) {
            Measure::Observed { value: false } => unavailable("not exercised: no request passed the compaction line"),
            Measure::Unavailable { reason } => unavailable(&reason),
            Measure::Observed { value: true } | Measure::LowerBound { .. } => {
                unavailable("no compaction request was observed")
            }
        }
    }
}

fn unavailable(reason: &str) -> CheckVerdict {
    CheckVerdict { check: "compaction-choice".into(), passed: Measure::unavailable(reason), evidence: vec![] }
}
