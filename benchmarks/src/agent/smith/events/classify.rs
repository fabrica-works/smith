//! Authoritative run answers and the reported process exit are reconciled.
//! Native failure metadata supplies budget and credential classes; codes alone
//! never manufacture them (shell.md, section4.3; benchmarks.md, section8.3).

use smith_events::{AnswerClass, AnswerFailure, Event, FailureClass, Mode, Status};

use crate::{BudgetLimit, Classification, End, Exit, FailureReason, Forced};

use super::Observer;

fn error(what: impl Into<String>) -> Classification {
    Classification { end: End::HarnessError { what: what.into() }, warnings: Vec::new() }
}

fn failed(failure: &AnswerFailure) -> Result<(End, i32), String> {
    let detail = failure
        .reason
        .as_ref()
        .map_or_else(|| "typed run failure".into(), |value| String::from_utf8_lossy(value).into_owned());
    let (end, exit) = match &failure.class {
        AnswerClass::Model => {
            let unauthorized =
                failure.completion.as_ref().is_some_and(|value| value.class == FailureClass::Unauthorized);
            (End::Failed { reason: FailureReason::Model, detail }, if unauthorized { 3 } else { 1 })
        }
        AnswerClass::Budget => {
            let which = match failure.which.as_deref() {
                Some(b"turns") => BudgetLimit::Turns,
                Some(b"time") => BudgetLimit::Time,
                Some(b"spend") => BudgetLimit::Spend,
                Some(_) | None => return Err("budget failure lacks a known which".into()),
            };
            (End::Budget { which }, 4)
        }
        AnswerClass::Policy => (End::Failed { reason: FailureReason::Policy, detail }, 1),
        AnswerClass::Cancelled => (End::Failed { reason: FailureReason::Cancelled, detail }, 130),
        AnswerClass::Agent => (End::Failed { reason: FailureReason::Agent, detail }, 1),
        AnswerClass::Host => (End::Failed { reason: FailureReason::Host, detail }, 1),
        AnswerClass::Stale => (End::Failed { reason: FailureReason::Stale, detail }, 1),
        AnswerClass::Transcript => (End::Failed { reason: FailureReason::Transcript, detail }, 1),
        AnswerClass::Busy => (End::Refused { setup: detail }, 1),
        AnswerClass::Invalid => (End::Refused { setup: detail }, 2),
        AnswerClass::Unknown(value) => {
            return Err(format!("unknown answer failure class {}", String::from_utf8_lossy(value)));
        }
    };
    Ok((end, exit))
}

fn pending(session: &super::Session) -> bool {
    let pending = session.records.iter().rev().find_map(|record| match &record.event {
        Event::RunStarted(run) => Some(run.run),
        Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
        | Event::Budget(_)
        | Event::SessionStarted(_)
        | Event::SessionEnded(_)
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
    });
    pending.is_some_and(|run| {
        !session.records.iter().any(|record| {
            matches!(&record.event,
        Event::RunCompleted(terminal) if terminal.run == run)
        })
    })
}

pub(super) fn classify(observer: &Observer, exit: Exit, forced: Forced) -> Classification {
    if let Some(error) = &observer.fault {
        return self::error(error.clone());
    }
    let Some(session) = observer.sessions.last() else {
        return error("no event session was observed");
    };
    if pending(session) {
        return if forced == Forced::No {
            error("last admitted run has no authoritative answer")
        } else {
            Classification { end: End::Timeout, warnings: Vec::new() }
        };
    }
    let last = session.records.iter().rev().find_map(|record| match &record.event {
        Event::RunCompleted(value) => Some(value),
        Event::MessageReceived(_)
        | Event::MessageRead(_)
        | Event::MessageUnread(_)
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
        | Event::Notice(_) => None,
    });
    let Some(run) = last else {
        return if forced == Forced::No {
            error("no run.completed was observed")
        } else {
            Classification { end: End::Timeout, warnings: Vec::new() }
        };
    };
    let terminal = match &run.status {
        Status::Accepted => {
            if run.result.is_none() || run.failure.is_some() {
                return error("accepted run has no result or carries a failure");
            }
            Ok((End::Completed, 0))
        }
        Status::Parked => {
            Ok((End::Failed { reason: FailureReason::InputNeeded, detail: "run parked awaiting input".into() }, 5))
        }
        Status::Failed | Status::Refused => match &run.failure {
            Some(failure) => failed(failure),
            None => Err("failed or refused run has no typed answer failure".into()),
        },
        Status::Unknown(value) => Err(format!("unknown run status {}", String::from_utf8_lossy(value))),
    };
    let (end, mut expected) = match terminal {
        Ok(value) => value,
        Err(what) => return error(what),
    };
    match &session.started.mode {
        Mode::Agent | Mode::Exec | Mode::Chat => {}
        Mode::Unknown(value) => return error(format!("unknown session mode {}", String::from_utf8_lossy(value))),
    }
    if forced != Forced::No {
        return if run.status == Status::Accepted {
            Classification { end, warnings: vec![format!("forced teardown after accepted answer: {forced:?}")] }
        } else {
            Classification { end: End::Timeout, warnings: Vec::new() }
        };
    }
    let Some(ended) = &session.ended else {
        return error("session.ended was not observed");
    };
    // The agent process succeeds once it sent any answer (protocol/agent.md,
    // section6); the product's exit still classifies the answer itself.
    if session.started.mode == Mode::Agent {
        expected = i32::from(!ended.answered);
    }
    // Interactive chat reports run failures as records; shell.md section4.3
    // assigns class-specific process codes to the headless Exec face only.
    if session.started.mode == Mode::Chat {
        expected = match i32::try_from(ended.exit) {
            Ok(code) => code,
            Err(_) => return error("interactive session exit exceeds the process code range"),
        };
    }
    if ended.exit != i64::from(expected) || exit != Exit::Code(expected) {
        return error(format!(
            "run.completed expects exit {expected}, session reports {}, tree reports {exit:?}",
            ended.exit
        ));
    }
    Classification { end, warnings: Vec::new() }
}
