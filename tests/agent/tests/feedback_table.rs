//! Golden bytes for each run call terminal and its nested refusal vocabulary.
//! Contract: domain/run.md, sections 5.2, 7, 8 and 10; testing-strategy.md, section 7.

use skein_lib::Duration;
use smith_domain::{feedback, run};

fn bytes(text: &[u8]) -> Box<[u8]> {
    text.into()
}

fn unanswered(end: run::End) -> run::Returned {
    run::Returned::Unanswered {
        end,
        used: run::facts::Used { turns: 0, time: Duration::ZERO, spend: 0 },
        left: run::Budget { turns: 0, time: Duration::ZERO, spend: 0 },
    }
}

fn expected_rendering(expected: &str, error: bool) -> (Box<[u8]>, bool) {
    let expected = if expected.starts_with("unanswered ") || expected == "child" {
        let budget = if expected.starts_with("unanswered end=budget:") { "yes" } else { "no" };
        format!(
            "{expected}\n\nChild usage: used 0 turns and 0 seconds; stopped for budget: {budget}; ordinary left 0 turns and 0 seconds."
        )
    } else {
        expected.to_owned()
    };
    (expected.as_bytes().into(), error)
}

fn rendering(returned: run::Returned) -> (Box<[u8]>, bool) {
    let result = feedback(returned, 4096).expect("bounded complete feedback");
    (result.text, result.error)
}

#[test]
fn every_run_call_terminal_has_canonical_feedback() {
    let receipt = run::Receipt::new(1, bytes(b"r")).expect("receipt");
    let delivered = run::Delivered::new(Box::new([receipt])).expect("delivery");
    let refusal = run::DeliveryRefusal::new(None, bytes(b"review")).expect("refusal");
    let host = run::HostAnswer::new(bytes(b"host text"), false).expect("host text");
    let rows = [
        (run::Returned::Waited { text: Box::from(b"messages arrived".as_slice()) }, "messages arrived", false),
        (run::Returned::Crossed { text: b"messages came".as_slice().into() }, "messages came", false),
        (run::Returned::HostAnswered(host), "host text", false),
        (
            run::Returned::HostTooLarge { bytes: 9312, max: 8192 },
            "host-decided answer-too-large bytes=9312 max=8192 answer-not-shown",
            true,
        ),
        (run::Returned::HostReportedTooLarge, "host-decided answer-too-large answer-not-shown", true),
        (run::Returned::HostUnknown, "host-unknown", true),
        (run::Returned::HostRejected(run::HostProblem::Undeclared), "host-rejected reason=undeclared", true),
        (run::Returned::Delivered(delivered), "delivered\nreceipt directory=1 text=\"r\"", false),
        (run::Returned::Nothing, "nothing", true),
        (run::Returned::DeliveryRefused(refusal), "delivery-refused explanation=\"review\" marker=none", true),
        (run::Returned::Accepted, "accepted", false),
        (
            run::Returned::Rejected { problems: run::outcome::Problems { listed: Box::new([]), more: 2 } },
            "rejected more=2",
            true,
        ),
        (
            run::Returned::ChecksFailed {
                repository: bytes(b"repo"),
                ran: run::Ran { exit: run::Exit::TimedOut, output: bytes(b"tail"), cut: 3 },
            },
            "checks-failed repository=\"repo\" exit=timed-out cut=3 tail=\"tail\"",
            true,
        ),
        (run::Returned::Stale, "stale", true),
        (
            run::Returned::DeliveryFailed { failure: run::DeliveryFailure::new(0, run::DeliveryReason::Unknown) },
            "delivery-failed directory=0 reason=unknown cut=0 tail=\"\"",
            true,
        ),
        (run::Returned::Cancelled, "cancelled", true),
        (run::Returned::TimedOut, "timed-out", true),
        (run::Returned::Busy, "busy", true),
        (
            run::Returned::Answered {
                text: bytes(b"child"),
                cut: 0,
                stop: run::Stop::EndTurn,
                used: run::facts::Used { turns: 0, time: Duration::ZERO, spend: 0 },
                left: run::Budget { turns: 0, time: Duration::ZERO, spend: 0 },
            },
            "child",
            false,
        ),
        (unanswered(run::End::Closed), "unanswered end=closed", true),
        (run::Returned::Refused { refusal: run::AskRefusal::NotGranted }, "refused reason=not-granted", true),
    ];
    for (returned, expected, error) in rows {
        assert_eq!(rendering(returned), expected_rendering(expected, error));
    }
}

#[test]
fn host_and_ask_refusal_variants_keep_their_names() {
    for (problem, label) in [
        (run::HostProblem::Undeclared, "undeclared"),
        (run::HostProblem::Effect, "effect"),
        (run::HostProblem::TooLarge, "too-large"),
    ] {
        assert_eq!(
            rendering(run::Returned::HostRejected(problem)),
            expected_rendering(&format!("host-rejected reason={label}"), true)
        );
    }
    for (refusal, label) in [
        (run::AskRefusal::Name, "name"),
        (run::AskRefusal::NotGranted, "not-granted"),
        (run::AskRefusal::TooMany, "too-many"),
        (run::AskRefusal::UnknownLlm, "unknown-llm"),
        (run::AskRefusal::Unworkable, "unworkable"),
        (run::AskRefusal::Over, "over"),
    ] {
        assert_eq!(
            rendering(run::Returned::Refused { refusal }),
            expected_rendering(&format!("refused reason={label}"), true)
        );
    }
}

#[test]
fn every_contract_problem_renders_its_values_and_field_scope() {
    use run::outcome::{Form, Problem, Problems};

    let rows = [
        (Problem::TooLarge { max: 9 }, "too-large max=9"),
        (Problem::ChangeNotAllowed, "change-not-allowed"),
        (Problem::VerdictNotAllowed, "verdict-not-allowed"),
        (Problem::ReportNotAllowed, "report-not-allowed"),
        (Problem::FailureNotAllowed, "failure-not-allowed"),
        (Problem::UnknownVerdict, "unknown-verdict"),
        (Problem::TextTooLarge { form: Form::Report, max: 3 }, "text-too-large form=report max=3"),
        (Problem::TextTooLarge { form: Form::Verdict, max: 4 }, "text-too-large form=verdict max=4"),
        (Problem::TextTooLarge { form: Form::Failure, max: 5 }, "text-too-large form=failure max=5"),
        (Problem::TooFewItems { min: 2 }, "too-few-items min=2"),
        (Problem::TooManyItems { max: 6 }, "too-many-items max=6"),
        (Problem::KindNotAllowed { item: 7 }, "kind-not-allowed item=7"),
        (Problem::MissingField { item: None, field: bytes(b"name") }, "missing-field item=none field=\"name\""),
        (Problem::EmptyField { item: Some(2), field: bytes(b"name") }, "empty-field item=2 field=\"name\""),
        (Problem::RepeatedField { item: None, field: bytes(b"name") }, "repeated-field item=none field=\"name\""),
        (
            Problem::FieldTooLarge { item: Some(3), field: bytes(b"name"), max: 8 },
            "field-too-large item=3 field=\"name\" max=8",
        ),
    ];
    for (problem, expected) in rows {
        assert_eq!(
            rendering(run::Returned::Rejected { problems: Problems { listed: Box::new([problem]), more: 0 } }),
            expected_rendering(&format!("rejected more=0\nproblem {expected}"), true)
        );
    }
}

#[test]
fn every_delivery_reason_and_check_exit_is_visible() {
    for (reason, label) in [
        (run::DeliveryReason::Unreachable, "unreachable"),
        (run::DeliveryReason::RefusedByTarget, "refused-by-target"),
        (run::DeliveryReason::TimedOut, "timed-out"),
        (run::DeliveryReason::Broken, "broken"),
        (run::DeliveryReason::TooLarge, "too-large"),
        (run::DeliveryReason::Missing, "missing"),
        (run::DeliveryReason::Busy, "busy"),
        (run::DeliveryReason::Unavailable, "unavailable"),
        (run::DeliveryReason::Cancelled, "cancelled"),
        (run::DeliveryReason::Unknown, "unknown"),
    ] {
        assert_eq!(
            rendering(run::Returned::DeliveryFailed { failure: run::DeliveryFailure::new(0, reason) }),
            expected_rendering(&format!("delivery-failed directory=0 reason={label} cut=0 tail=\"\""), true)
        );
    }
    assert_eq!(
        rendering(run::Returned::DeliveryFailed {
            failure: run::DeliveryFailure::new(2, run::DeliveryReason::Unreachable)
        }),
        expected_rendering("delivery-failed directory=2 reason=unreachable cut=0 tail=\"\"", true)
    );
    for (exit, label) in [
        (run::Exit::Code { code: 7 }, "code:7"),
        (run::Exit::Signalled { signal: 15 }, "signalled"),
        (run::Exit::TimedOut, "timed-out"),
        (run::Exit::Unstarted, "unstarted"),
        (run::Exit::Cancelled, "cancelled"),
    ] {
        assert_eq!(
            rendering(run::Returned::ChecksFailed {
                repository: bytes(b"r"),
                ran: run::Ran { exit, output: bytes(b""), cut: 0 }
            }),
            expected_rendering(&format!("checks-failed repository=\"r\" exit={label} cut=0 tail=\"\""), true)
        );
    }
}

#[test]
fn every_unanswered_end_and_transcript_refusal_is_visible() {
    let rows = [
        (run::End::PriceOverflow, "price-overflow"),
        (run::End::UsageOverflow, "usage-overflow"),
        (run::End::Receiving(run::ReceivingLimit::Input), "receiving:input"),
        (run::End::Receiving(run::ReceivingLimit::Output), "receiving:output"),
        (run::End::Receiving(run::ReceivingLimit::CacheRead), "receiving:cache-read"),
        (run::End::Receiving(run::ReceivingLimit::CacheWrite), "receiving:cache-write"),
        (run::End::Closed, "closed"),
        (run::End::Busy, "busy"),
        (run::End::Invalid, "invalid"),
        (run::End::Budget(run::Exhausted::Turns), "budget:turns"),
        (run::End::Budget(run::Exhausted::Spend), "budget:spend"),
        (run::End::Budget(run::Exhausted::Time), "budget:time"),
    ];
    for (end, label) in rows {
        assert_eq!(rendering(unanswered(end)), expected_rendering(&format!("unanswered end={label}"), true));
    }
    for (reason, label) in [
        (run::TranscriptRefusal::Version, "version"),
        (run::TranscriptRefusal::Malformed, "malformed"),
        (run::TranscriptRefusal::Unresolved, "unresolved"),
        (run::TranscriptRefusal::TooLarge, "too-large"),
    ] {
        assert_eq!(
            rendering(unanswered(run::End::TranscriptRefused { reason })),
            expected_rendering(&format!("unanswered end=transcript-refused:{label}"), true)
        );
    }
}

#[test]
fn every_model_fault_and_completion_failure_keeps_its_evidence() {
    for (fault, label) in [
        (run::Fault::Limited { account: 0, reset_at: None }, "limited"),
        (run::Fault::Refused { account: 0, code: Box::default() }, "account-refused"),
        (run::Fault::Provider, "provider"),
        (run::Fault::Declined, "refused"),
        (run::Fault::Truncated, "truncated"),
        (run::Fault::Malformed, "malformed"),
    ] {
        assert_eq!(
            rendering(unanswered(run::End::Fault(fault))),
            expected_rendering(&format!("unanswered end=fault:{label}"), true)
        );
    }
    let cooldown = Duration::from_nanos(19);
    for (failure, label) in [
        (run::CompletionFailure::Limit { which: run::Limit::Path(run::Cap::Answer), bound: 512 }, "limit"),
        (run::CompletionFailure::Protocol, "protocol"),
        (run::CompletionFailure::Cancelled, "unsolicited-cancelled"),
        (run::CompletionFailure::Overloaded, "overloaded"),
        (run::CompletionFailure::Unavailable, "unavailable"),
        (run::CompletionFailure::TimedOut { phase: run::Phase::Whole }, "timed-out"),
        (run::CompletionFailure::ContextTooLong, "context-too-long"),
        (run::CompletionFailure::Invalid, "invalid"),
        (run::CompletionFailure::Refused, "refused"),
        (run::CompletionFailure::Unsupported, "unsupported"),
        (run::CompletionFailure::Unauthorized, "unauthorized"),
        (run::CompletionFailure::RateLimited { retry_after: cooldown }, "rate-limited retry-after-ns=19"),
        (run::CompletionFailure::Exhausted { retry_after: Some(cooldown) }, "exhausted retry-after-ns=19"),
    ] {
        for (evidence, evidence_label) in [
            (run::CompletionEvidence::Unsent, "unsent"),
            (run::CompletionEvidence::Unknown, "unknown"),
            (run::CompletionEvidence::Response { status: None }, "response"),
        ] {
            assert_eq!(
                rendering(unanswered(run::End::Fault(run::Fault::Completion {
                    failure,
                    evidence,
                    model: 0,
                    attempts: 1,
                    report: run::Report::none(),
                }))),
                expected_rendering(&format!("unanswered end=fault:completion:{label} evidence={evidence_label}"), true)
            );
        }
    }
}

fn failure_fields() -> Vec<(smith_domain::llm::Failure, run::CompletionFailure)> {
    use smith_domain::llm;
    let retry_after = Duration::from_millis(7);
    vec![
        (
            llm::Failure::Limit { which: llm::Limit::Pool { reservation: 73 }, bound: 64 },
            run::CompletionFailure::Limit { which: run::Limit::Pool { reservation: 73 }, bound: 64 },
        ),
        (
            llm::Failure::Limit { which: llm::Limit::Path(llm::Cap::Answer), bound: 512 },
            run::CompletionFailure::Limit { which: run::Limit::Path(run::Cap::Answer), bound: 512 },
        ),
        (llm::Failure::ApplicationLimit, run::CompletionFailure::ApplicationLimit),
        (llm::Failure::Protocol, run::CompletionFailure::Protocol),
        (llm::Failure::Cancelled, run::CompletionFailure::Cancelled),
        (llm::Failure::Overloaded, run::CompletionFailure::Overloaded),
        (llm::Failure::Unavailable, run::CompletionFailure::Unavailable),
        (
            llm::Failure::TimedOut { phase: llm::Phase::Head },
            run::CompletionFailure::TimedOut { phase: run::Phase::Head },
        ),
        (llm::Failure::Invalid, run::CompletionFailure::Invalid),
        (llm::Failure::Refused, run::CompletionFailure::Refused),
        (llm::Failure::Unsupported, run::CompletionFailure::Unsupported),
        (llm::Failure::Unauthorized, run::CompletionFailure::Unauthorized),
        (llm::Failure::RateLimited { retry_after }, run::CompletionFailure::RateLimited { retry_after }),
        (
            llm::Failure::Exhausted { retry_after: Some(retry_after) },
            run::CompletionFailure::Exhausted { retry_after: Some(retry_after) },
        ),
        (
            llm::Failure::Exhausted { retry_after: Some(Duration::ZERO) },
            run::CompletionFailure::Exhausted { retry_after: Some(Duration::ZERO) },
        ),
        (llm::Failure::Exhausted { retry_after: None }, run::CompletionFailure::Exhausted { retry_after: None }),
    ]
}

#[test]
fn every_completion_failure_reaches_the_answer_with_its_fields() {
    completion_answer_fields();
    budget_answer_fields();
}

fn completion_answer_fields() {
    use smith_agent_world::{Settings, World};
    use smith_domain::llm;
    for (source, expected) in failure_fields() {
        let mut settings = Settings::calm(6807);
        settings.limits.session.retries = 0;
        let charter = smith_agent_world::scripted_charter(&settings);
        let mut world = World::with_failure(settings, source, llm::Evidence::Response { status: Some(503) });
        world.failure_report(smith_domain::llm::Report {
            code: Box::from(b"provider-code".as_slice()),
            request: Box::from(b"request-id".as_slice()),
            field: Box::from(b"request-field".as_slice()),
        });
        world.run(20_000);
        let fault = expected_fault(expected, &world);
        let resolved =
            if let run::Answer::Failed { failure: run::Failure::Model(run::Fault::Completion { model, .. }), .. } =
                world.answer()
            {
                charter.model(*model).map(|value| (&value.endpoint, &value.model))
            } else {
                None
            };
        let expected_model = if let run::Fault::Completion { .. } = &fault {
            Some((&charter.llm.endpoint, &charter.llm.model))
        } else {
            None
        };
        assert_eq!(
            (world.answer(), resolved),
            (
                &run::Answer::Failed { failure: run::Failure::Model(fault), spent: run::Spend::ZERO, turns: 0 },
                expected_model
            )
        );
    }
}

fn expected_fault(expected: run::CompletionFailure, world: &smith_agent_world::World) -> run::Fault {
    match expected {
        run::CompletionFailure::Exhausted { retry_after } => {
            let at = world.completions()[0].terminal.expect("provider failed").0;
            run::Fault::Limited { account: 0, reset_at: retry_after.map(|reset| at.saturating_add(reset)) }
        }
        run::CompletionFailure::Refused => {
            run::Fault::Refused { account: 0, code: Box::from(b"provider-code".as_slice()) }
        }
        run::CompletionFailure::Unsupported
        | run::CompletionFailure::Limit { .. }
        | run::CompletionFailure::ApplicationLimit
        | run::CompletionFailure::Protocol
        | run::CompletionFailure::Cancelled
        | run::CompletionFailure::Overloaded
        | run::CompletionFailure::Unavailable
        | run::CompletionFailure::TimedOut { .. }
        | run::CompletionFailure::ContextTooLong
        | run::CompletionFailure::Invalid
        | run::CompletionFailure::Unauthorized
        | run::CompletionFailure::RateLimited { .. } => run::Fault::Completion {
            failure: expected,
            evidence: run::CompletionEvidence::Response { status: Some(503) },
            model: 0,
            attempts: 1,
            report: run::Report {
                code: Box::from(b"provider-code".as_slice()),
                request: Box::from(b"request-id".as_slice()),
                field: Box::from(b"request-field".as_slice()),
            },
        },
    }
}

fn budget_answer_fields() {
    use smith_agent_world::{Job, Settings, World};
    let mut settings = Settings::calm(4);
    settings.job = Job::Spending;
    settings.budget.turns = 8;
    let mut world = World::new(settings);
    world.run(20_000);
    let run::Answer::Failed { failure, .. } = world.answer() else { panic!("shared turns spent") };
    assert_eq!(
        failure.clone(),
        run::Failure::Budget { exhausted: run::Exhausted::Turns, used: Some(7), bound: Some(8) }
    );
}
