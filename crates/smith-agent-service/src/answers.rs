//! Pure projection of the exact answer sent by the agent, without host or provider IO.
//! Only an accepted answer owns result content. The caller supplies its run fence,
//! wind-down and elapsed time; no observation of stdout establishes acceptance.
//! Contract: protocol/events.md, sections 3.3 and 3.10.

#![expect(clippy::manual_map, reason = "the strict subset requires explicit matches instead of closures")]

use skein_lib::{Duration, List};
use smith_domain::run;
use smith_events::{
    AnswerClass, AnswerFailure, CompletionFailure, Evidence, FailureClass, Form, RunCompleted, RunResult, Status, Usage,
};

use crate::sinks::Capture;

/// The run owner's final fence and measurements, supplied with its actual answer.
#[derive(Clone, Copy, Debug)]
pub struct AnswerMeta {
    pub run: u64,
    pub wind_down: bool,
    pub read: Option<u64>,
    pub unread: u64,
    pub elapsed: Duration,
}

/// Project the exact terminal the service sends, preserving its accepted counters.
#[must_use]
pub fn answer_record(meta: AnswerMeta, answer: &run::Answer, capture: Capture) -> RunCompleted {
    let (status, failure, result, turns, spent) = match answer {
        run::Answer::Parked { spent, turns } => (Status::Parked, None, None, *turns, *spent),
        run::Answer::Refused(refusal) => (Status::Refused, Some(refused(*refusal)), None, 0, run::Spend::ZERO),
        run::Answer::Accepted { outcome, spent, turns } => {
            (Status::Accepted, None, Some(accepted(outcome, capture)), *turns, *spent)
        }
        run::Answer::Failed { failure, spent, turns } => (Status::Failed, Some(failed(*failure)), None, *turns, *spent),
    };
    RunCompleted {
        run: meta.run,
        wind_down: meta.wind_down,
        status,
        failure,
        result,
        turns: u64::from(turns),
        spent: spent.units,
        usage: Usage {
            input_tokens: spent.input,
            cache_read_tokens: spent.cache_read,
            cache_write_tokens: spent.cache_write,
            output_tokens: spent.output,
            reasoning_tokens: spent.reasoning,
        },
        read: meta.read,
        unread: meta.unread,
        duration_ms: meta.elapsed.as_nanos() / 1_000_000,
    }
}

fn accepted(outcome: &run::outcome::Declared, capture: Capture) -> RunResult {
    let (form, label, text, fields) = match outcome {
        run::outcome::Declared::Change(change) => (Form::Change, None, None, &change.fields),
        run::outcome::Declared::Verdict(verdict) => {
            (Form::Verdict, Some(verdict.name.clone()), Some(&verdict.text), &verdict.fields)
        }
        run::outcome::Declared::Report(report) => (Form::Report, None, Some(&report.text), &report.fields),
        run::outcome::Declared::Failure(failure) => (Form::Failure, None, Some(&failure.reason), &failure.fields),
    };
    let (text, fields) = match capture {
        Capture::None | Capture::Calls => (None, None),
        Capture::Everything => {
            let mut copied = List::with_capacity(u32::try_from(fields.len()).expect("admitted result field count"));
            for field in fields {
                copied
                    .push(smith_events::Field { name: field.name.clone(), text: field.value.clone() })
                    .expect("admitted result fields");
            }
            let text = match text {
                Some(text) => Some(text.clone()),
                None => None,
            };
            (text, Some(copied))
        }
    };
    RunResult { form, label, text, fields }
}

fn failure(class: AnswerClass, which: Option<&[u8]>) -> AnswerFailure {
    AnswerFailure {
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

fn refused(refusal: run::Refusal) -> AnswerFailure {
    match refusal {
        run::Refusal::Busy => failure(AnswerClass::Busy, None),
        run::Refusal::Invalid(invalid) => failure(AnswerClass::Invalid, Some(invalid_name(invalid))),
    }
}

fn invalid_name(invalid: run::Invalid) -> &'static [u8] {
    match invalid {
        run::Invalid::HostPart { .. } => b"host_part",
        run::Invalid::Room { .. } => b"room",
        run::Invalid::Messages => b"messages",
        run::Invalid::CharterVersion => b"charter_version",
        run::Invalid::MalformedCharter => b"malformed_charter",
        run::Invalid::Endpoint => b"endpoint",
        run::Invalid::Activation => b"activation",
        run::Invalid::Window => b"window",
        run::Invalid::Conventions => b"conventions",
        run::Invalid::TooLarge => b"too_large",
        run::Invalid::Workspace => b"workspace",
        run::Invalid::Grants => b"grants",
        run::Invalid::Outcome => b"outcome",
        run::Invalid::Budget(_) => b"budget",
        run::Invalid::Price(_) => b"price",
        run::Invalid::Llm => b"llm",
        run::Invalid::Conversation => b"conversation",
    }
}

fn failed(why: run::Failure) -> AnswerFailure {
    match why {
        run::Failure::Model(fault) => match fault {
            run::Fault::Completion { failure: why, evidence } => {
                let mut result = failure(AnswerClass::Model, None);
                result.completion = Some(completion(why, evidence));
                result
            }
            run::Fault::Exhausted => failure(AnswerClass::Model, Some(b"exhausted")),
            run::Fault::Provider => failure(AnswerClass::Model, Some(b"provider")),
            run::Fault::ContextFull => failure(AnswerClass::Model, Some(b"context_full")),
            run::Fault::Refused => failure(AnswerClass::Model, Some(b"refused")),
            run::Fault::Truncated => failure(AnswerClass::Model, Some(b"truncated")),
            run::Fault::Malformed => failure(AnswerClass::Model, Some(b"malformed")),
        },
        run::Failure::Budget(exhausted) => {
            let which: &[u8] = match exhausted {
                run::Exhausted::Turns => b"turns",
                run::Exhausted::Spend => b"spend",
                run::Exhausted::Time => b"time",
                run::Exhausted::Tokens(receiving) => match receiving {
                    run::ReceivingLimit::Input => b"input_tokens",
                    run::ReceivingLimit::Output => b"output_tokens",
                    run::ReceivingLimit::CacheRead => b"cache_read_tokens",
                    run::ReceivingLimit::CacheWrite => b"cache_write_tokens",
                },
                run::Exhausted::Overflow(overflow) => match overflow {
                    run::Overflow::Spend => b"spend_overflow",
                    run::Overflow::Usage => b"usage_overflow",
                },
            };
            failure(AnswerClass::Budget, Some(which))
        }
        run::Failure::Policy(run::Policy::Unfinished { .. }) => failure(AnswerClass::Policy, None),
        run::Failure::Cancelled => failure(AnswerClass::Cancelled, None),
        run::Failure::Stale => failure(AnswerClass::Stale, None),
        run::Failure::Transcript(refusal) => {
            let which: &[u8] = match refusal {
                run::TranscriptRefusal::Version => b"version",
                run::TranscriptRefusal::Endpoint => b"endpoint",
                run::TranscriptRefusal::Dialect => b"dialect",
                run::TranscriptRefusal::Malformed => b"malformed",
                run::TranscriptRefusal::Unresolved => b"unresolved",
                run::TranscriptRefusal::TooLarge => b"too_large",
            };
            let mut result = failure(AnswerClass::Transcript, None);
            result.reason = Some(which.into());
            result
        }
    }
}

fn completion(why: run::CompletionFailure, evidence: run::CompletionEvidence) -> CompletionFailure {
    let (class, retry_after_ms) = match why {
        run::CompletionFailure::Limit => (FailureClass::Limit, None),
        run::CompletionFailure::Protocol => (FailureClass::Protocol, None),
        run::CompletionFailure::Cancelled => (FailureClass::Cancelled, None),
        run::CompletionFailure::Overloaded => (FailureClass::Overloaded, None),
        run::CompletionFailure::Unavailable => (FailureClass::Unavailable, None),
        run::CompletionFailure::TimedOut => (FailureClass::TimedOut, None),
        run::CompletionFailure::ContextTooLong => (FailureClass::ContextTooLong, None),
        run::CompletionFailure::Invalid => (FailureClass::Invalid, None),
        run::CompletionFailure::Unauthorized => (FailureClass::Unauthorized, None),
        run::CompletionFailure::RateLimited { retry_after } => {
            (FailureClass::RateLimited, Some(retry_after.as_nanos() / 1_000_000))
        }
        run::CompletionFailure::Exhausted { retry_after } => {
            (FailureClass::Exhausted, Some(retry_after.as_nanos() / 1_000_000))
        }
    };
    let evidence = match evidence {
        run::CompletionEvidence::Unsent => Evidence::Unsent,
        run::CompletionEvidence::Unknown => Evidence::MaybeSent,
        run::CompletionEvidence::Response => Evidence::Response,
    };
    CompletionFailure { class, evidence, retry_after_ms }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> AnswerMeta {
        AnswerMeta { run: 7, wind_down: false, read: Some(13), unread: 1, elapsed: Duration::from_millis(17) }
    }

    fn cases(spent: run::Spend) -> [(run::Answer, Status, Option<AnswerClass>, u64); 9] {
        [
            (run::Answer::Parked { spent, turns: 2 }, Status::Parked, None, 19),
            (run::Answer::Refused(run::Refusal::Busy), Status::Refused, Some(AnswerClass::Busy), 0),
            (
                run::Answer::Refused(run::Refusal::Invalid(run::Invalid::Workspace)),
                Status::Refused,
                Some(AnswerClass::Invalid),
                0,
            ),
            (
                run::Answer::Failed { failure: run::Failure::Cancelled, spent, turns: 2 },
                Status::Failed,
                Some(AnswerClass::Cancelled),
                19,
            ),
            (
                run::Answer::Failed { failure: run::Failure::Stale, spent, turns: 2 },
                Status::Failed,
                Some(AnswerClass::Stale),
                19,
            ),
            (
                run::Answer::Failed { failure: run::Failure::Budget(run::Exhausted::Spend), spent, turns: 2 },
                Status::Failed,
                Some(AnswerClass::Budget),
                19,
            ),
            (
                run::Answer::Failed {
                    failure: run::Failure::Policy(run::Policy::Unfinished { nudges: 3, rejected: 5 }),
                    spent,
                    turns: 2,
                },
                Status::Failed,
                Some(AnswerClass::Policy),
                19,
            ),
            (
                run::Answer::Failed {
                    failure: run::Failure::Transcript(run::TranscriptRefusal::Unresolved),
                    spent,
                    turns: 2,
                },
                Status::Failed,
                Some(AnswerClass::Transcript),
                19,
            ),
            (
                run::Answer::Failed {
                    failure: run::Failure::Model(run::Fault::Completion {
                        failure: run::CompletionFailure::RateLimited { retry_after: Duration::from_millis(23) },
                        evidence: run::CompletionEvidence::Response,
                    }),
                    spent,
                    turns: 2,
                },
                Status::Failed,
                Some(AnswerClass::Model),
                19,
            ),
        ]
    }

    #[test]
    fn every_actual_answer_kind_preserves_its_terminal_and_accounting() {
        let spent = run::Spend {
            turns: 2,
            input: Some(3),
            cache_read: Some(5),
            cache_write: Some(7),
            output: Some(11),
            reasoning: Some(2),
            units: 19,
        };
        for (actual, expected_status, expected_class, expected_spend) in cases(spent) {
            for capture in [Capture::None, Capture::Calls, Capture::Everything] {
                let projected = answer_record(meta(), &actual, capture);
                assert_eq!(projected.status, expected_status);
                assert_eq!(
                    match &projected.failure {
                        Some(why) => Some(&why.class),
                        None => None,
                    },
                    expected_class.as_ref()
                );
                assert_eq!(projected.spent, expected_spend);
                assert_eq!(projected.usage.input_tokens, if expected_spend == 0 { None } else { Some(3) });
                assert_eq!(projected.result, None);
                assert_eq!(
                    (projected.run, projected.read, projected.unread, projected.duration_ms),
                    (7, Some(13), 1, 17)
                );
            }
        }
        let why = failed(run::Failure::Model(run::Fault::Completion {
            failure: run::CompletionFailure::RateLimited { retry_after: Duration::from_millis(23) },
            evidence: run::CompletionEvidence::Response,
        }));
        assert_eq!(
            why.completion,
            Some(CompletionFailure {
                class: FailureClass::RateLimited,
                evidence: Evidence::Response,
                retry_after_ms: Some(23),
            })
        );
        assert_eq!(
            failed(run::Failure::Transcript(run::TranscriptRefusal::Unresolved)).reason.as_deref(),
            Some(b"unresolved".as_slice())
        );
    }
}
