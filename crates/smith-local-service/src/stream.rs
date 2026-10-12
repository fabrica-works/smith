//! The local writer's terminal from the parent face's answer, for either kind.
//! Only accepted results carry content, under the configured capture policy.
//! Contract: protocol/events.md, sections 3.3 and 4.

#![expect(clippy::manual_map, reason = "the strict subset uses exhaustive matches without closures")]

use smith_agent_service::{Answered, sinks::Capture};
use smith_events::{AnswerClass, AnswerFailure, CompletionFailure, Evidence, FailureClass, Status};
use smith_host_domain as host;

/// Parts of `run.completed` from the exact answer the local host meets.
#[must_use]
pub fn answered(answer: &host::Answer, capture: Capture) -> Answered {
    let (status, failure, result) = match &answer.result {
        host::RunResult::Parked => (Status::Parked, None, None),
        host::RunResult::Refused { refusal } => (Status::Refused, Some(refused(*refusal)), None),
        host::RunResult::Failed { failure } => (Status::Failed, Some(failed(*failure)), None),
        host::RunResult::Accepted { outcome } => {
            (Status::Accepted, None, Some(smith_agent_service::accepted(outcome.value(), capture)))
        }
    };
    Answered {
        status,
        failure,
        result,
        turns: u64::from(answer.turns),
        spent: answer.spent,
        read: match answer.read {
            Some(read) => Some(read.raw()),
            None => None,
        },
    }
}

fn failure(class: AnswerClass, which: Option<&[u8]>) -> AnswerFailure {
    AnswerFailure {
        dropped: None,
        ended: None,
        turn: None,
        endpoint: None,
        model: None,
        attempts: None,
        class,
        completion: None,
        account: None,
        which: match which {
            Some(which) => Some(which.into()),
            None => None,
        },
        reason: None,
    }
}

fn refused(refusal: host::Refusal) -> AnswerFailure {
    match refusal {
        host::Refusal::Busy => failure(AnswerClass::Busy, None),
        host::Refusal::Invalid(invalid) => failure(AnswerClass::Invalid, Some(invalid_name(invalid))),
    }
}

fn invalid_name(invalid: host::RunInvalid) -> &'static [u8] {
    match invalid {
        host::RunInvalid::HostPart { .. } => b"host_part",
        host::RunInvalid::Room { .. } => b"room",
        host::RunInvalid::Messages => b"messages",
        host::RunInvalid::CharterVersion => b"charter_version",
        host::RunInvalid::MalformedCharter => b"malformed_charter",
        host::RunInvalid::Endpoint => b"endpoint",
        host::RunInvalid::Activation => b"activation",
        host::RunInvalid::Window => b"window",
        host::RunInvalid::Conventions => b"conventions",
        host::RunInvalid::TooLarge => b"too_large",
        host::RunInvalid::Workspace => b"workspace",
        host::RunInvalid::Grants => b"grants",
        host::RunInvalid::Outcome => b"outcome",
        host::RunInvalid::Budget(why) => match why {
            host::BudgetRefusal::Reserve(part) => match part {
                host::Part::Turns => b"budget_reserve_turns",
                host::Part::Spend => b"budget_reserve_spend",
                host::Part::Time => b"budget_reserve_time",
            },
            host::BudgetRefusal::Ceiling(_)
            | host::BudgetRefusal::Empty(_)
            | host::BudgetRefusal::Checks { .. }
            | host::BudgetRefusal::Completion { .. } => b"budget",
        },
        host::RunInvalid::Price(_) => b"price",
        host::RunInvalid::Llm => b"llm",
        host::RunInvalid::Conversation => b"conversation",
    }
}

fn failed(why: host::RunFailure) -> AnswerFailure {
    match why {
        host::RunFailure::Model(fault) => match fault {
            host::ModelFault::Completion { failure: why, evidence } => {
                let mut result = failure(AnswerClass::Model, None);
                result.completion = Some(completion(why, evidence));
                result
            }
            host::ModelFault::Exhausted => failure(AnswerClass::Model, Some(b"exhausted")),
            host::ModelFault::Provider => failure(AnswerClass::Model, Some(b"provider")),
            host::ModelFault::ContextFull => failure(AnswerClass::Model, Some(b"context_full")),
            host::ModelFault::Refused => failure(AnswerClass::Model, Some(b"refused")),
            host::ModelFault::Truncated => failure(AnswerClass::Model, Some(b"truncated")),
            host::ModelFault::Malformed => failure(AnswerClass::Model, Some(b"malformed")),
        },
        host::RunFailure::Budget(exhausted) => {
            let which: &[u8] = match exhausted {
                host::Exhausted::Turns => b"turns",
                host::Exhausted::Spend => b"spend",
                host::Exhausted::Time => b"time",
                host::Exhausted::Tokens(receiving) => match receiving {
                    host::ReceivingLimit::Input => b"input_tokens",
                    host::ReceivingLimit::Output => b"output_tokens",
                    host::ReceivingLimit::CacheRead => b"cache_read_tokens",
                    host::ReceivingLimit::CacheWrite => b"cache_write_tokens",
                },
                host::Exhausted::Overflow(overflow) => match overflow {
                    host::Overflow::Spend => b"spend_overflow",
                    host::Overflow::Usage => b"usage_overflow",
                },
            };
            failure(AnswerClass::Budget, Some(which))
        }
        host::RunFailure::Policy(host::Policy::Unfinished { .. }) => failure(AnswerClass::Policy, None),
        host::RunFailure::Cancelled => {
            let mut answer = failure(AnswerClass::Cancelled, None);
            answer.dropped = Some(false);
            answer
        }
        host::RunFailure::Stale => failure(AnswerClass::Stale, None),
        host::RunFailure::Transcript(refusal) => {
            let which: &[u8] = match refusal {
                host::TranscriptRefusal::Version => b"version",
                host::TranscriptRefusal::Endpoint => b"endpoint",
                host::TranscriptRefusal::Dialect => b"dialect",
                host::TranscriptRefusal::Malformed => b"malformed",
                host::TranscriptRefusal::Unresolved => b"unresolved",
                host::TranscriptRefusal::TooLarge => b"too_large",
            };
            let mut result = failure(AnswerClass::Transcript, None);
            result.reason = Some(which.into());
            result
        }
    }
}

fn completion(why: host::CompletionFailure, evidence: host::CompletionEvidence) -> CompletionFailure {
    let (class, retry_after_ms) = match why {
        host::CompletionFailure::Limit => (FailureClass::Limit, None),
        host::CompletionFailure::Protocol => (FailureClass::Protocol, None),
        host::CompletionFailure::Cancelled => (FailureClass::Cancelled, None),
        host::CompletionFailure::Overloaded => (FailureClass::Overloaded, None),
        host::CompletionFailure::Unavailable => (FailureClass::Unavailable, None),
        host::CompletionFailure::TimedOut => (FailureClass::TimedOut, None),
        host::CompletionFailure::ContextTooLong => (FailureClass::ContextTooLong, None),
        host::CompletionFailure::Invalid => (FailureClass::Invalid, None),
        host::CompletionFailure::Unauthorized => (FailureClass::Unauthorized, None),
        host::CompletionFailure::RateLimited { retry_after } => {
            (FailureClass::RateLimited, Some(retry_after.as_nanos() / 1_000_000))
        }
        host::CompletionFailure::Exhausted { retry_after } => {
            (FailureClass::Exhausted, Some(retry_after.as_nanos() / 1_000_000))
        }
    };
    let evidence = match evidence {
        host::CompletionEvidence::Unsent => Evidence::Unsent,
        host::CompletionEvidence::Unknown => Evidence::MaybeSent,
        host::CompletionEvidence::Response => Evidence::Response,
    };
    CompletionFailure { class, evidence, retry_after_ms, phase: None, limit: None, status: None, detail: None }
}
