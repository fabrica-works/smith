//! Turn the wire last word into the host domain's concrete result (protocol/channel.md, sections 4 and 5;
//! domain/host.md, sections 2 and 6).
//!
//! This module keeps no state, never learns policy or credential values, and
//! enters through `decode_answer` after the channel body decoder succeeds.

use smith_channel as wire;
use smith_host_domain::channel as host;

use crate::Error;

#[expect(clippy::manual_map, clippy::manual_let_else, reason = "exhaustive result and fence decoding")]
pub(crate) fn decode_answer(record: &wire::Answer, limits: &smith_charter::v4::Limits) -> Result<host::Answer, Error> {
    let result = match record.result() {
        wire::RunResult::Refused(refused) => {
            let refusal = match refused.reason() {
                wire::StartRefusal::Busy => host::Refusal::Busy,
                wire::StartRefusal::Invalid(invalid) => host::Refusal::Invalid(invalid_start(invalid.value())),
            };
            host::RunResult::Refused { refusal }
        }
        wire::RunResult::Accepted(accepted) => {
            let value = match smith_protocol_channel::decode_declared(accepted.result(), limits) {
                Ok(value) => value,
                Err(_) => return Err(Error::InvalidAnswer),
            };
            host::RunResult::Accepted {
                outcome: smith_host_domain::Declared::new(value, u64::MAX).ok_or(Error::InvalidAnswer)?,
            }
        }
        wire::RunResult::Parked => host::RunResult::Parked,
        wire::RunResult::Failed(failed) => host::RunResult::Failed { failure: failure(failed.reason())? },
    };
    Ok(host::Answer {
        read: match record.last_read() {
            Some(name) => Some(skein_lib::Token::new(*name)),
            None => None,
        },
        turns: record.turns(),
        spent: record.spent(),
        result,
    })
}

fn invalid_start(invalid: &wire::InvalidStart) -> host::RunInvalid {
    match invalid {
        wire::InvalidStart::CharterVersion => host::RunInvalid::CharterVersion,
        wire::InvalidStart::MalformedCharter => host::RunInvalid::MalformedCharter,
        wire::InvalidStart::Activation => host::RunInvalid::Activation,
        wire::InvalidStart::Window => host::RunInvalid::Window,
        wire::InvalidStart::Conventions => host::RunInvalid::Conventions,
        wire::InvalidStart::TooLarge => host::RunInvalid::TooLarge,
        wire::InvalidStart::Workspace => host::RunInvalid::Workspace,
        wire::InvalidStart::Grants => host::RunInvalid::Grants,
        wire::InvalidStart::Outcome => host::RunInvalid::Outcome,
        wire::InvalidStart::Budget(record) => host::RunInvalid::Budget(budget_refusal(record.value())),
        wire::InvalidStart::Price(record) => host::RunInvalid::Price(price_rate(record.value())),
        wire::InvalidStart::Llm => host::RunInvalid::Llm,
        wire::InvalidStart::Effort => host::RunInvalid::Effort,
        wire::InvalidStart::Conversation => host::RunInvalid::Conversation,
        wire::InvalidStart::HostPart(bound) => host::RunInvalid::HostPart { bound: bound.bound() },
        wire::InvalidStart::Room(bound) => host::RunInvalid::Room { bound: bound.bound() },
        wire::InvalidStart::Endpoint => host::RunInvalid::Endpoint,
        wire::InvalidStart::Messages => host::RunInvalid::Messages,
    }
}

fn failure(source: &wire::RunFailure) -> Result<host::RunFailure, Error> {
    let result = match source {
        wire::RunFailure::Transcript(record) => {
            let reason = match record.value() {
                wire::TranscriptRefusal::Version => host::TranscriptRefusal::Version,
                wire::TranscriptRefusal::Malformed => host::TranscriptRefusal::Malformed,
                wire::TranscriptRefusal::Unresolved => host::TranscriptRefusal::Unresolved,
                wire::TranscriptRefusal::TooLarge => host::TranscriptRefusal::TooLarge,
            };
            host::RunFailure::Transcript(reason)
        }
        wire::RunFailure::Model(record) => host::RunFailure::Model(model_fault(record)?),
        wire::RunFailure::Budget(record) => {
            let exhausted = budget(record.value());
            match exhausted {
                host::Exhausted::Turns | host::Exhausted::Spend | host::Exhausted::Time => {
                    if record.used().is_none() || record.bound().is_none() {
                        return Err(Error::InvalidAnswer);
                    }
                }
                host::Exhausted::Tokens(_) | host::Exhausted::Overflow(_) => {
                    if record.used().is_some() || record.bound().is_some() {
                        return Err(Error::InvalidAnswer);
                    }
                }
            }
            host::RunFailure::Budget { exhausted, used: *record.used(), bound: *record.bound() }
        }
        wire::RunFailure::Policy(record) => {
            host::RunFailure::Policy(host::Policy::Unfinished { nudges: record.nudges(), rejected: record.rejected() })
        }
        wire::RunFailure::Cancelled => host::RunFailure::Cancelled,
        wire::RunFailure::Stale => host::RunFailure::Stale,
    };
    Ok(result)
}

fn model_fault(record: &wire::ModelFaultValue) -> Result<host::ModelFault, Error> {
    let result = match record.value() {
        wire::ModelFault::Completion(record) => {
            let failure = match record.failure() {
                wire::CompletionFailure::Limit(record) => {
                    host::CompletionFailure::Limit { which: completion_limit(record.which()), bound: record.bound() }
                }
                wire::CompletionFailure::ApplicationLimit => host::CompletionFailure::ApplicationLimit,
                wire::CompletionFailure::Protocol => host::CompletionFailure::Protocol,
                wire::CompletionFailure::Cancelled => host::CompletionFailure::Cancelled,
                wire::CompletionFailure::Overloaded => host::CompletionFailure::Overloaded,
                wire::CompletionFailure::Unavailable => host::CompletionFailure::Unavailable,
                wire::CompletionFailure::TimedOut(record) => {
                    host::CompletionFailure::TimedOut { phase: completion_phase(record.phase()) }
                }
                wire::CompletionFailure::ContextTooLong => host::CompletionFailure::ContextTooLong,
                wire::CompletionFailure::Refused => host::CompletionFailure::Refused,
                wire::CompletionFailure::Unsupported => host::CompletionFailure::Unsupported,
                wire::CompletionFailure::Invalid => host::CompletionFailure::Invalid,
                wire::CompletionFailure::Unauthorized => host::CompletionFailure::Unauthorized,
                wire::CompletionFailure::RateLimited => host::CompletionFailure::RateLimited {
                    retry_after: (*record.retry_after()).ok_or(Error::InvalidAnswer)?,
                },
                wire::CompletionFailure::Exhausted => {
                    host::CompletionFailure::Exhausted { retry_after: *record.retry_after() }
                }
            };
            let evidence = match record.evidence() {
                wire::CompletionEvidence::Unsent => host::CompletionEvidence::Unsent,
                wire::CompletionEvidence::Unknown => host::CompletionEvidence::Unknown,
                wire::CompletionEvidence::Response(record) => {
                    host::CompletionEvidence::Response { status: *record.status() }
                }
            };
            host::ModelFault::Completion {
                failure,
                evidence,
                model: record.model(),
                attempts: record.attempts(),
                report: host::Report {
                    code: record.code().into(),
                    request: record.request_id().into(),
                    field: record.field().into(),
                },
            }
        }
        wire::ModelFault::Limited(value) => {
            host::ModelFault::Limited { account: value.account(), reset: *value.reset() }
        }
        wire::ModelFault::Refused(value) => {
            host::ModelFault::Refused { account: value.account(), code: value.code().into() }
        }
        wire::ModelFault::Unauthorized(value) => host::ModelFault::Unauthorized { account: value.account() },
        wire::ModelFault::Program(value) => {
            host::ModelFault::Program { name: value.name().into(), exit: *value.exit(), detail: value.detail().into() }
        }
        wire::ModelFault::Provider => host::ModelFault::Provider,
        wire::ModelFault::Declined => host::ModelFault::Declined,
        wire::ModelFault::Truncated => host::ModelFault::Truncated,
        wire::ModelFault::Malformed => host::ModelFault::Malformed,
    };
    Ok(result)
}

fn budget(source: &wire::BudgetFailure) -> host::Exhausted {
    match source {
        wire::BudgetFailure::Turns => host::Exhausted::Turns,
        wire::BudgetFailure::Spend => host::Exhausted::Spend,
        wire::BudgetFailure::Time => host::Exhausted::Time,
        wire::BudgetFailure::Tokens(record) => {
            let limit = match record.value() {
                wire::ReceivingLimit::Input => host::ReceivingLimit::Input,
                wire::ReceivingLimit::Output => host::ReceivingLimit::Output,
                wire::ReceivingLimit::CacheRead => host::ReceivingLimit::CacheRead,
                wire::ReceivingLimit::CacheWrite => host::ReceivingLimit::CacheWrite,
            };
            host::Exhausted::Tokens(limit)
        }
        wire::BudgetFailure::Overflow(record) => {
            let overflow = match record.value() {
                wire::BudgetOverflow::Spend => host::Overflow::Spend,
                wire::BudgetOverflow::Usage => host::Overflow::Usage,
            };
            host::Exhausted::Overflow(overflow)
        }
    }
}

fn price_rate(rate: &wire::PriceRate) -> host::Rate {
    match rate {
        wire::PriceRate::Input => host::Rate::Input,
        wire::PriceRate::Cached => host::Rate::Cached,
        wire::PriceRate::CacheWrite => host::Rate::CacheWrite,
        wire::PriceRate::Output => host::Rate::Output,
    }
}

fn budget_part(part: &wire::BudgetPart) -> host::Part {
    match part {
        wire::BudgetPart::Turns => host::Part::Turns,
        wire::BudgetPart::Spend => host::Part::Spend,
        wire::BudgetPart::Time => host::Part::Time,
    }
}

fn budget_refusal(refusal: &wire::BudgetRefusal) -> host::BudgetRefusal {
    match refusal {
        wire::BudgetRefusal::Ceiling(record) => host::BudgetRefusal::Ceiling(match record.value() {
            wire::BudgetCeiling::Turns(record) => host::Ceiling::Turns(record.value()),
            wire::BudgetCeiling::Spend(record) => host::Ceiling::Spend(record.value()),
            wire::BudgetCeiling::Time(record) => host::Ceiling::Time(record.value()),
        }),
        wire::BudgetRefusal::Empty(record) => host::BudgetRefusal::Empty(budget_part(record.value())),
        wire::BudgetRefusal::Reserve(record) => host::BudgetRefusal::Reserve(budget_part(record.value())),
        wire::BudgetRefusal::Completion(record) => host::BudgetRefusal::Completion { most: record.most() },
        wire::BudgetRefusal::Checks(record) => {
            host::BudgetRefusal::Checks { reserve: record.reserve(), checks: record.checks() }
        }
    }
}

fn completion_limit(which: &wire::CompletionLimit) -> host::Limit {
    match which {
        wire::CompletionLimit::Pool(record) => host::Limit::Pool { reservation: record.reservation() },
        wire::CompletionLimit::Path(record) => host::Limit::Path(completion_cap(record.value())),
        wire::CompletionLimit::History => host::Limit::History,
        wire::CompletionLimit::Window => host::Limit::Window,
        wire::CompletionLimit::Call => host::Limit::Call,
        wire::CompletionLimit::Message => host::Limit::Message,
    }
}

const fn completion_cap(cap: &wire::Cap) -> host::Cap {
    match cap {
        wire::Cap::Request => host::Cap::Request,
        wire::Cap::RequestHead => host::Cap::RequestHead,
        wire::Cap::ResponseHead => host::Cap::ResponseHead,
        wire::Cap::ResponseFields => host::Cap::ResponseFields,
        wire::Cap::Skip => host::Cap::Skip,
        wire::Cap::Field => host::Cap::Field,
        wire::Cap::Retained => host::Cap::Retained,
        wire::Cap::Tokens => host::Cap::Tokens,
        wire::Cap::Depth => host::Cap::Depth,
        wire::Cap::Strings => host::Cap::Strings,
        wire::Cap::Number => host::Cap::Number,
        wire::Cap::Reasoning => host::Cap::Reasoning,
        wire::Cap::OutputItems => host::Cap::OutputItems,
        wire::Cap::Metadata => host::Cap::Metadata,
        wire::Cap::Tools => host::Cap::Tools,
        wire::Cap::HistoryItems => host::Cap::HistoryItems,
        wire::Cap::Output => host::Cap::Output,
        wire::Cap::Answer => host::Cap::Answer,
        wire::Cap::ErrorBody => host::Cap::ErrorBody,
        wire::Cap::AccessToken => host::Cap::AccessToken,
        wire::Cap::AccountId => host::Cap::AccountId,
    }
}

const fn completion_phase(phase: &wire::CompletionPhase) -> host::Phase {
    match phase {
        wire::CompletionPhase::Connect => host::Phase::Connect,
        wire::CompletionPhase::Handshake => host::Phase::Handshake,
        wire::CompletionPhase::Head => host::Phase::Head,
        wire::CompletionPhase::Idle => host::Phase::Idle,
        wire::CompletionPhase::Whole => host::Phase::Whole,
    }
}
