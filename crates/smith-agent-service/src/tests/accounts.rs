//! Account notices and run answers retain their supplied reset and state.
use super::assert_stream_record;
use skein_lib::{Duration, Time};
use smith_domain::{Request, run};

#[test]
fn each_account_notice_is_written_when_raised() {
    struct Row {
        name: &'static str,
        request: Request,
        expected: &'static [u8],
    }
    let mut service = crate::service::tests::account_service();
    for row in [
        Row { name: "unauthorized", request: Request::Unauthorized { grant: smith_domain::GrantName { account: 7, generation: 1 } }, expected: br#"{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"credential_unauthorized","account":7}"# },
        Row { name: "limited", request: Request::Limited { account: 7, reset: Some(Duration::from_millis(73)) }, expected: br#"{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"account_limited","account":7,"reset_ms":73}"# },
        Row { name: "limited unknown", request: Request::Limited { account: 7, reset: None }, expected: br#"{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"account_limited","account":7}"# },
        Row { name: "refused", request: Request::Refused { account: 7, code: b"not_allowed".as_slice().into(), reason: b"provider content".as_slice().into() }, expected: br#"{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"account_refused","account":7,"code":"not_allowed"}"# },
    ] {
        let record = crate::service::tests::account_request(&mut service, row.request);
        assert_stream_record(&record, row.expected);
        let line = smith_events::write(&record, &smith_events::Capture::None, &smith_events::Limits { string: 256, content: 1024, items: 8 }).expect(row.name).unwrap();
        assert_eq!(line.as_ref(), [row.expected, b"\n"].concat(), "{}", row.name);
    }
}

#[test]
fn a_limited_run_answers_with_reset_ms() {
    struct Row {
        name: &'static str,
        reset_at: Option<Time>,
        expected: &'static [u8],
    }
    for row in [
        Row { name: "reset", reset_at: Some(Time::from_nanos(75_000_000)), expected: br#"{"v":4,"type":"run.completed","t_ms":2,"run":7,"wind_down":false,"status":"failed","failure":{"class":"model","account":7,"state":"limited","reset_ms":73},"turns":0,"spent":0,"usage":{"input_tokens":null,"cache_read_tokens":null,"cache_write_tokens":null,"output_tokens":null,"reasoning_tokens":null},"read":null,"unread":0,"duration_ms":2}"# },
        Row { name: "unknown", reset_at: None, expected: br#"{"v":4,"type":"run.completed","t_ms":2,"run":7,"wind_down":false,"status":"failed","failure":{"class":"model","account":7,"state":"limited"},"turns":0,"spent":0,"usage":{"input_tokens":null,"cache_read_tokens":null,"cache_write_tokens":null,"output_tokens":null,"reasoning_tokens":null},"read":null,"unread":0,"duration_ms":2}"# },
    ] {
        let meta = crate::AnswerMeta { now: Time::from_nanos(2_000_000), run: 7, wind_down: false, read: None, unread: 0, elapsed: Duration::from_millis(2) };
        let answer = run::Answer::Failed { failure: run::Failure::Model(run::Fault::Limited { account: 7, reset_at: row.reset_at }), spent: run::Spend::ZERO, turns: 0 };
        let record = smith_events::Record { t_ms: 2, event: smith_events::Event::RunCompleted(crate::answer_record(meta, &answer, crate::sinks::Capture::None)) };
        assert_stream_record(&record, row.expected);
        assert_eq!(record.event, smith_events::read(row.expected, &smith_events::Limits { string: 256, content: 1024, items: 8 }).expect(row.name).unwrap().event, "{}", row.name);
    }
}
