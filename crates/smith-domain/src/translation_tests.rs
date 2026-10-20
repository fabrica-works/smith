//! Exact tables at the session to run sibling boundary.
//! Contract: domain/session.md, sections 3–6; domain/run.md, sections 5, 9 and 13.

use skein_lib::Duration;
use smith_domain_run as run;
use smith_domain_session::{self as session, llm};

use crate::translate;

#[test]
fn every_session_yield_and_usage_counter_maps_to_the_run() {
    for (yielded, stop) in [
        (session::Yield::Done, run::Stop::EndTurn),
        (session::Yield::Truncated, run::Stop::MaxTokens),
        (session::Yield::Refused, run::Stop::Refusal),
        (session::Yield::Malformed, run::Stop::NoCalls),
    ] {
        assert_eq!(translate::stop(yielded), stop);
    }
    let usage = llm::Usage {
        input_tokens: Some(2),
        output_tokens: Some(3),
        cache_read_tokens: Some(5),
        cache_write_tokens: Some(7),
        reasoning_tokens: Some(2),
    };
    assert_eq!(
        translate::spend(11, usage),
        run::Spend {
            turns: 11,
            input: Some(2),
            output: Some(3),
            cache_read: Some(5),
            cache_write: Some(7),
            units: 0,
            reasoning: Some(2)
        }
    );
}

#[test]
fn every_session_end_and_budget_dimension_keeps_its_class() {
    let rows = [
        (session::End::PriceOverflow, run::End::PriceOverflow),
        (session::End::UsageOverflow, run::End::UsageOverflow),
        (session::End::Busy, run::End::Busy),
        (session::End::Invalid, run::End::Invalid),
        (session::End::Closed, run::End::Closed),
        (session::End::Budget { spent: session::Dimension::Unit }, run::End::Budget(run::Exhausted::Spend)),
        (session::End::Budget { spent: session::Dimension::Turns }, run::End::Budget(run::Exhausted::Turns)),
        (session::End::Budget { spent: session::Dimension::Time }, run::End::Budget(run::Exhausted::Time)),
        (session::End::Budget { spent: session::Dimension::Input }, run::End::Receiving(run::ReceivingLimit::Input)),
        (session::End::Budget { spent: session::Dimension::Output }, run::End::Receiving(run::ReceivingLimit::Output)),
        (
            session::End::Budget { spent: session::Dimension::CacheRead },
            run::End::Receiving(run::ReceivingLimit::CacheRead),
        ),
        (
            session::End::Budget { spent: session::Dimension::CacheWrite },
            run::End::Receiving(run::ReceivingLimit::CacheWrite),
        ),
    ];
    for (from, to) in rows {
        assert_eq!(translate::end(from, 0, Some(0), None, llm::Report::none()), to);
    }
}

#[test]
fn every_transcript_refusal_keeps_its_reason() {
    for (from, to) in [
        (session::record::Refusal::Version, run::TranscriptRefusal::Version),
        (session::record::Refusal::Malformed, run::TranscriptRefusal::Malformed),
        (session::record::Refusal::Unresolved, run::TranscriptRefusal::Unresolved),
        (session::record::Refusal::TooLarge, run::TranscriptRefusal::TooLarge),
    ] {
        assert_eq!(
            translate::end(session::End::TranscriptRefused { reason: from }, 0, Some(0), None, llm::Report::none()),
            run::End::TranscriptRefused { reason: to }
        );
    }
}

#[test]
fn every_completion_failure_and_transport_evidence_maps_without_inference() {
    let cooldown = Duration::from_nanos(23);
    let failures = [
        (llm::Failure::ApplicationLimit, run::CompletionFailure::ApplicationLimit),
        (llm::Failure::Protocol, run::CompletionFailure::Protocol),
        (llm::Failure::Cancelled, run::CompletionFailure::Cancelled),
        (llm::Failure::Overloaded, run::CompletionFailure::Overloaded),
        (llm::Failure::Unavailable, run::CompletionFailure::Unavailable),
        (
            llm::Failure::TimedOut { phase: llm::Phase::Whole },
            run::CompletionFailure::TimedOut { phase: run::Phase::Whole },
        ),
        (llm::Failure::ContextTooLong, run::CompletionFailure::ContextTooLong),
        (llm::Failure::Invalid, run::CompletionFailure::Invalid),
        (llm::Failure::Unauthorized, run::CompletionFailure::Unauthorized),
        (
            llm::Failure::RateLimited { retry_after: cooldown },
            run::CompletionFailure::RateLimited { retry_after: cooldown },
        ),
        (
            llm::Failure::Exhausted { retry_after: Some(cooldown) },
            run::CompletionFailure::Exhausted { retry_after: Some(cooldown) },
        ),
    ];
    for (failure, mapped) in failures {
        for (evidence, mapped_evidence) in [
            (llm::Evidence::Unsent, run::CompletionEvidence::Unsent),
            (llm::Evidence::Unknown, run::CompletionEvidence::Unknown),
            (llm::Evidence::Response { status: None }, run::CompletionEvidence::Response { status: None }),
        ] {
            assert_eq!(
                translate::end(
                    session::End::Failed { detail: Box::default(), failure, evidence, attempts: 1 },
                    0,
                    Some(0),
                    None,
                    llm::Report::none()
                ),
                run::End::Fault(match mapped {
                    run::CompletionFailure::Exhausted { .. } => run::Fault::Limited { account: 0, reset_at: None },
                    run::CompletionFailure::Limit { .. }
                    | run::CompletionFailure::ApplicationLimit
                    | run::CompletionFailure::Protocol
                    | run::CompletionFailure::Cancelled
                    | run::CompletionFailure::Overloaded
                    | run::CompletionFailure::Unavailable
                    | run::CompletionFailure::TimedOut { .. }
                    | run::CompletionFailure::ContextTooLong
                    | run::CompletionFailure::Refused
                    | run::CompletionFailure::Unsupported
                    | run::CompletionFailure::Invalid
                    | run::CompletionFailure::Unauthorized
                    | run::CompletionFailure::RateLimited { .. } => run::Fault::Completion {
                        failure: mapped,
                        evidence: mapped_evidence,
                        model: 0,
                        attempts: 1,
                        report: run::Report::none(),
                    },
                })
            );
        }
    }
}
