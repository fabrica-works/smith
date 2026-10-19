//! Public canonical feedback controls with independently specified exact bytes.
//! Contract: domain/run.md, sections 5.2, 7, 8 and 10;
//! testing-strategy.md, section 7.

use skein_lib::Duration;
use smith_domain::{Feedback, FeedbackRefusal, feedback, run};

const USED: run::facts::Used = run::facts::Used { turns: 0, time: Duration::ZERO, spend: 0 };
const LEFT: run::Budget = run::Budget { turns: 0, time: Duration::ZERO, spend: 0 };
const SUFFIX: &str =
    "\n\nChild usage: used 0 turns and 0 seconds; stopped for budget: no; ordinary left 0 turns and 0 seconds.";

fn bytes(text: &[u8]) -> Box<[u8]> {
    text.into()
}

#[test]
fn host_moves_without_copy_and_child_prefix_precedes_usage() {
    for error in [false, true] {
        let text = bytes("actual ✓\n".as_bytes());
        let pointer = text.as_ptr();
        let answer = run::HostAnswer::new(text, error).expect("sealed host text");
        let got = feedback(run::Returned::HostAnswered(answer), 64).expect("compatible cap");
        assert_eq!(got.text.as_ref(), "actual ✓\n".as_bytes());
        assert_eq!(got.text.as_ptr(), pointer);
        assert_eq!(got.error, error);
    }
    let text = bytes(b"child\n");
    let got = feedback(run::Returned::Answered { text, cut: 0, stop: run::Stop::EndTurn, used: USED, left: LEFT }, 256)
        .expect("ordinary child");
    assert_eq!(got.text.as_ref(), format!("child\n{SUFFIX}").as_bytes());
    assert!(!got.error);
}

#[test]
fn child_cut_and_every_stop_survive_canonical_feedback() {
    for (stop, name) in [
        (run::Stop::EndTurn, "end-turn"),
        (run::Stop::MaxTokens, "max-tokens"),
        (run::Stop::Refusal, "refusal"),
        (run::Stop::NoCalls, "no-calls"),
    ] {
        for cut in [0, u64::MAX] {
            let got =
                feedback(run::Returned::Answered { text: bytes(b"child"), cut, stop, used: USED, left: LEFT }, 256)
                    .expect("compatible bound");
            let expected = if cut == 0 && stop == run::Stop::EndTurn {
                "child".to_owned()
            } else {
                format!("child\n[child-result cut={cut} stop={name}]")
            };
            assert_eq!(got.text.as_ref(), format!("{expected}{SUFFIX}").as_bytes());
            assert!(!got.error);
        }
    }
}

#[test]
fn opaque_receipts_diagnostics_markers_and_check_tail_are_lossless_ascii() {
    let opaque = [0xff, b'"', b'\\', 0, b'\n', b' ', b'a'];
    let encoded = br#""\xff\x22\x5c\x00\x0a a""#;
    let receipts = run::Delivered::new(Box::new([run::Receipt::new(2, bytes(&opaque)).expect("bounded receipt")]))
        .expect("actual sealed receipts");
    let got = feedback(run::Returned::Delivered(receipts), 128).expect("full evidence fits");
    assert_eq!(got.text.as_ref(), [b"delivered\nreceipt directory=2 text=".as_slice(), encoded].concat());
    assert!(!got.error);
    let got = feedback(
        run::Returned::DeliveryFailed {
            failure: run::DeliveryFailure {
                directory: 0,
                reason: run::DeliveryReason::Unknown,
                diagnostic: run::Diagnostic::new(&opaque, u64::MAX),
            },
        },
        128,
    )
    .expect("diagnostic");
    assert_eq!(
        got.text.as_ref(),
        [b"delivery-failed directory=0 reason=unknown cut=18446744073709551615 tail=".as_slice(), encoded].concat()
    );
    assert!(got.error);
    let marker = run::Marker::new(1, bytes(&[b'd', b'/', 0xff, b'"'])).expect("bounded marker");
    let refused = run::DeliveryRefusal::new(Some(marker), bytes(&opaque)).expect("bounded refusal");
    let got = feedback(run::Returned::DeliveryRefused(refused), 128).expect("named feedback");
    assert_eq!(
        got.text.as_ref(),
        [b"delivery-refused explanation=".as_slice(), encoded, br#" marker-directory=1 marker-path="d/\xff\x22""#]
            .concat()
    );
    let got = feedback(
        run::Returned::ChecksFailed {
            repository: bytes(&opaque),
            ran: run::Ran { exit: run::Exit::Code { code: 255 }, output: bytes(&opaque), cut: u64::MAX },
        },
        256,
    )
    .expect("checks");
    assert_eq!(
        got.text.as_ref(),
        [b"checks-failed repository=".as_slice(), encoded, b" exit=code:255 cut=18446744073709551615 tail=", encoded]
            .concat()
    );
    assert!(got.error);
    assert!(got.text.is_ascii());
}

#[test]
fn exact_text_cap_accepts_whole_result_and_one_byte_less_refuses() {
    assert_eq!(feedback(run::Returned::Accepted, 7), Err(FeedbackRefusal::TooLarge));
    assert_eq!(feedback(run::Returned::Accepted, 8), Ok(Feedback { text: bytes(b"accepted"), error: false }));
}

#[test]
fn child_scalar_receiving_and_unknown_prefix_ends_remain_distinct_literal_feedback() {
    for (end, expected) in [
        (run::End::Budget(run::Exhausted::Turns), "unanswered end=budget:turns"),
        (run::End::Budget(run::Exhausted::Spend), "unanswered end=budget:spend"),
        (run::End::Budget(run::Exhausted::Time), "unanswered end=budget:time"),
        (run::End::PriceOverflow, "unanswered end=price-overflow"),
        (run::End::UsageOverflow, "unanswered end=usage-overflow"),
        (run::End::Receiving(run::ReceivingLimit::Input), "unanswered end=receiving:input"),
        (run::End::Receiving(run::ReceivingLimit::Output), "unanswered end=receiving:output"),
        (run::End::Receiving(run::ReceivingLimit::CacheRead), "unanswered end=receiving:cache-read"),
        (run::End::Receiving(run::ReceivingLimit::CacheWrite), "unanswered end=receiving:cache-write"),
    ] {
        let budget = if expected.starts_with("unanswered end=budget:") { "yes" } else { "no" };
        let expected = format!(
            "{expected}\n\nChild usage: used 0 turns and 0 seconds; stopped for budget: {budget}; ordinary left 0 turns and 0 seconds."
        );
        let received = feedback(
            run::Returned::Unanswered { end: end.clone(), used: USED, left: LEFT },
            u64::try_from(expected.len()).expect("bounded bytes"),
        )
        .expect("exact complete cap");
        assert_eq!(received.text.as_ref(), expected.as_bytes());
        assert!(received.error);
        assert!(
            feedback(
                run::Returned::Unanswered { end, used: USED, left: LEFT },
                u64::try_from(expected.len() - 1).expect("short label")
            )
            .is_err()
        );
    }
}

#[test]
fn child_usage_has_exact_scalar_values_flooring_and_complete_cap() {
    let used = run::facts::Used { turns: 3, time: Duration::from_nanos(750_000_000), spend: 77 };
    let left = run::Budget { turns: 5, time: Duration::from_nanos(1_999_000_000), spend: 89 };
    let expected = b"answer\n[child-result cut=14 stop=end-turn]\n\nChild usage: used 3 turns and less than a second; stopped for budget: no; ordinary left 5 turns and 1 second.";
    for cap in [expected.len(), expected.len() - 1] {
        let got = feedback(
            run::Returned::Answered { text: bytes(b"answer"), cut: 14, stop: run::Stop::EndTurn, used, left },
            u64::try_from(cap).expect("fixture cap"),
        );
        if cap == expected.len() {
            assert_eq!(got.expect("exact cap").text.as_ref(), expected);
        } else {
            assert_eq!(got, Err(FeedbackRefusal::TooLarge));
        }
    }
    let expected = b"unanswered end=budget:time\n\nChild usage: used 3 turns and less than a second; stopped for budget: yes; ordinary left 5 turns and 1 second.";
    assert_eq!(
        feedback(
            run::Returned::Unanswered { end: run::End::Budget(run::Exhausted::Time), used, left },
            u64::try_from(expected.len()).expect("fixture cap")
        )
        .expect("exact terminal")
        .text
        .as_ref(),
        expected
    );
}

#[test]
fn largest_usage_suffix_and_answer_fit_the_declared_feedback_bound() {
    let mut limits = smith_agent_world::TIGHT.run;
    limits.answer_bytes = 8192;
    let cap = smith_domain::feedback_worst_case(&limits).expect("checked bound");
    let used = run::facts::Used { turns: u32::MAX, time: Duration::from_nanos(u64::MAX), spend: u64::MAX };
    let left = run::Budget { turns: u32::MAX, time: Duration::from_nanos(u64::MAX), spend: u64::MAX };
    let answer = || run::Returned::Answered {
        text: vec![b'x'; 8192].into_boxed_slice(),
        cut: u64::MAX,
        stop: run::Stop::MaxTokens,
        used,
        left,
    };
    let received = feedback(answer(), cap).expect("largest child fields fit bound");
    assert!(received.text.ends_with(b"Child usage: used 4294967295 turns and 18446744073 seconds; stopped for budget: no; ordinary left 4294967295 turns and 18446744073 seconds."));
    let measured = u64::try_from(received.text.len()).expect("bounded feedback");
    assert_eq!(feedback(answer(), measured - 1), Err(FeedbackRefusal::TooLarge));
}
