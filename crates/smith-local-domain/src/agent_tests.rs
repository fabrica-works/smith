//! Typed values retained by the local parent and its spawned child.
use super::agent::{RunStart, answer_text, delivery_to_host, name_to_host, prepare_start, saved_reply_to_host};
use alloc::boxed::Box;
use smith_domain::run;
use smith_host_domain as host;

#[test]
fn a_spawned_last_word_keeps_its_local_display_class() {
    assert_eq!(
        answer_text(&host::Answer { read: None, turns: 3, spent: 29, result: host::RunResult::Parked }).as_ref(),
        b"Chat parked"
    );
    assert_eq!(
        answer_text(&host::Answer {
            read: None,
            turns: 1,
            spent: 7,
            result: host::RunResult::Failed { failure: host::RunFailure::Cancelled }
        })
        .as_ref(),
        b""
    );
}

#[test]
fn a_local_start_keeps_its_concrete_charter_and_run_identity() {
    let settings = smith_agent_world::Settings::calm(67);
    let charter = smith_agent_world::scripted_charter(&settings);
    let identity = run::Identity::new([0x83; 16]);
    for live_text in [false, true] {
        let source = RunStart {
            live_text,
            messages: Box::new([]),
            identity,
            activation: 7,
            charter: smith_agent_world::scripted_charter(&settings),
            workspace: None,
            transcript: None,
            answered: Box::default(),
            grants: Box::default(),
        };
        let prepared = prepare_start(
            source,
            crate::tests::host_limits().charter_bytes,
            crate::tests::host_limits().transcript_bytes,
        )
        .expect("bounded concrete start");
        match prepared.live {
            Some(ask) => {
                assert!(live_text);
                assert_eq!((ask.text, ask.reasoning, ask.input), (true, false, false));
            }
            None => assert!(!live_text),
        }
        assert_eq!(prepared.activation, 7);
        assert_eq!(prepared.identity, Some(identity));
        assert_eq!(prepared.logical_run, skein_lib::Token::new(1));
        assert_eq!(prepared.charter.into_value(), charter);
        assert!(prepared.transcript.is_none());
    }
}

#[test]
fn a_saved_delivery_retains_its_name_receipt_and_failure_tail() {
    let name = run::CallName { activation: 9, completion: 3, position: 1 };
    assert_eq!(name_to_host(name), host::channel::CallName { activation: 9, completion: 3, position: 1 });
    let receipt = run::Receipt::new(0, Box::from(&b"commit abc"[..])).expect("receipt");
    let delivered = run::Delivered::new(Box::from([receipt])).expect("delivery");
    let saved = saved_reply_to_host(smith_domain::Answered::Delivery(Box::new(run::Delivery::Delivered(delivered))))
        .expect("saved answer");
    let host::channel::SavedReply::Delivery(answer) = saved else { panic!("delivery answer") };
    let host::Delivery::Delivered(answer) = *answer else { panic!("landed terminal") };
    assert_eq!(answer.receipts()[0].text(), b"commit abc");

    let failed = run::DeliveryFailure {
        directory: 1,
        reason: run::DeliveryReason::TimedOut,
        diagnostic: run::Diagnostic::new(b"git timed out", 7),
    };
    let converted = delivery_to_host(run::Delivery::Failed(failed)).expect("failed terminal");
    let host::Delivery::Failed(converted) = converted else { panic!("failure") };
    assert_eq!(converted.reason, host::DeliveryReason::TimedOut);
    assert_eq!(converted.diagnostic.output(), b"git timed out");
    assert_eq!(converted.diagnostic.cut(), 7);
}
