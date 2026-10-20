//! Job observations retain their original call identity (protocol/events.md, section 3.6).
use super::{Event, Record, Scope, Time, command_exit, millis, session, tools, verdict_kind};

pub(super) fn number(job: Option<tools::JobNumber>) -> Option<u64> {
    Some(u64::from(job?.0))
}

pub(super) fn ended(origin: Time, fact: &session::Fact, scope: Scope, metadata: &session::ToolCall) -> Record {
    let (job, end, unread_bytes, started) = match fact.kind {
        session::FactKind::JobEnded { job, end, unread_bytes, started, .. } => (job, end, unread_bytes, started),
        session::FactKind::CompactionStarted { .. }
        | session::FactKind::CompactionEnded { .. }
        | session::FactKind::Opened { .. }
        | session::FactKind::CompletionStarted { .. }
        | session::FactKind::CompletionAnswered { .. }
        | session::FactKind::CompletionFailed { .. }
        | session::FactKind::CompletionCancelled { .. }
        | session::FactKind::CompletionRetried { .. }
        | session::FactKind::ReasoningDropped { .. }
        | session::FactKind::Yielded { .. }
        | session::FactKind::Used { .. }
        | session::FactKind::Ended { .. }
        | session::FactKind::Tools { .. }
        | session::FactKind::BuiltinStarted { .. }
        | session::FactKind::BuiltinAnswered { .. }
        | session::FactKind::DelegateStarted { .. }
        | session::FactKind::DelegateAnswered { .. }
        | session::FactKind::DelegateCancelled { .. }
        | session::FactKind::DelegateWithheld { .. } => unreachable!("only a job end reaches this projection"),
    };
    let verdict = match end {
        tools::JobEnd::Ended(exit) => tools::Verdict::Exited { exit },
        tools::JobEnd::Stopped => tools::Verdict::Stopped,
        tools::JobEnd::Cancelled => tools::Verdict::Cancelled,
        tools::JobEnd::Running => unreachable!("a closing job has settled"),
    };
    Record {
        t_ms: millis(fact.at, origin),
        event: Event::JobEnded(smith_events::JobEnded {
            run: scope.run,
            conversation: scope.conversation,
            job: u64::from(job.0),
            call: metadata.id.clone(),
            verdict: verdict_kind(verdict),
            exit: command_exit(verdict),
            unread_bytes,
            duration_ms: millis(fact.at, started),
        }),
    }
}
