//! Account notices and run answers retain their supplied reset and state.
use super::{assert_stream_record, stream_limits};
use skein_lib::{Duration, Time};
use smith_domain::{Request, run};

const ACCOUNT: u32 = 7;

const RUN: u64 = 7;

const RESET_AT: Time = Time::from_nanos(75_000_000);

const NOW: Time = Time::from_nanos(2_000_000);

const RESET_DELAY: Duration = RESET_AT.saturating_since(NOW);

const RESET_MS: u64 = RESET_DELAY.as_nanos() / Duration::from_millis(1).as_nanos();

const ELAPSED: Duration = NOW.saturating_since(Time::ZERO);

const NOW_MS: u64 = ELAPSED.as_nanos() / Duration::from_millis(1).as_nanos();

#[test]
fn each_account_notice_is_written_when_raised() {
    struct Row {
        name: &'static str,
        request: Request,
        expected: String,
    }
    let mut service = crate::service::tests::account_service();
    for row in [
        Row {
            name: "unauthorized",
            request: Request::Unauthorized { grant: smith_domain::GrantName { account: ACCOUNT, generation: 1 } },
            expected: alloc::format!(
                concat!(
                    r#"{{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"credential_unauthorized","#,
                    r#""account":{account}}}"#,
                ),
                account = ACCOUNT,
            ),
        },
        Row {
            name: "limited",
            request: Request::Limited { account: ACCOUNT, reset: Some(RESET_DELAY) },
            expected: alloc::format!(
                concat!(
                    r#"{{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"account_limited","#,
                    r#""account":{account},"reset_ms":{reset_ms}}}"#,
                ),
                account = ACCOUNT,
                reset_ms = RESET_MS,
            ),
        },
        Row {
            name: "limited unknown",
            request: Request::Limited { account: ACCOUNT, reset: None },
            expected: alloc::format!(
                concat!(
                    r#"{{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"account_limited","#,
                    r#""account":{account}}}"#,
                ),
                account = ACCOUNT,
            ),
        },
        Row {
            name: "refused",
            request: Request::Refused {
                account: ACCOUNT,
                code: b"not_allowed".as_slice().into(),
                reason: b"provider content".as_slice().into(),
            },
            expected: alloc::format!(
                concat!(
                    r#"{{"v":4,"type":"notice","t_ms":0,"level":"warning","kind":"account_refused","#,
                    r#""account":{account},"code":"not_allowed"}}"#,
                ),
                account = ACCOUNT,
            ),
        },
    ] {
        let record = crate::service::tests::account_request(&mut service, row.request);
        let expected = row.expected.as_bytes();
        assert_stream_record(&record, expected);
        let line =
            smith_events::write(&record, &smith_events::Capture::None, &stream_limits()).expect(row.name).unwrap();
        assert_eq!(line.as_ref(), [expected, b"\n"].concat(), "{}", row.name);
    }
}

#[test]
fn a_limited_run_answers_with_reset_ms() {
    struct Row {
        name: &'static str,
        reset_at: Option<Time>,
        expected: String,
    }
    for row in [
        Row {
            name: "reset",
            reset_at: Some(RESET_AT),
            expected: alloc::format!(
                concat!(
                    r#"{{"v":4,"type":"run.completed","t_ms":{now_ms},"run":{run},"wind_down":false,"#,
                    r#""status":"failed","failure":{{"class":"model","account":{account},"state":"limited","#,
                    r#""reset_ms":{reset_ms}}},"turns":0,"spent":0,"#,
                    r#""usage":{{"input_tokens":null,"cache_read_tokens":null,"cache_write_tokens":null,"#,
                    r#""output_tokens":null,"reasoning_tokens":null}},"#,
                    r#""read":null,"unread":0,"duration_ms":{now_ms}}}"#,
                ),
                account = ACCOUNT,
                run = RUN,
                reset_ms = RESET_MS,
                now_ms = NOW_MS,
            ),
        },
        Row {
            name: "unknown",
            reset_at: None,
            expected: alloc::format!(
                concat!(
                    r#"{{"v":4,"type":"run.completed","t_ms":{now_ms},"run":{run},"wind_down":false,"#,
                    r#""status":"failed","failure":{{"class":"model","account":{account},"state":"limited"}},"#,
                    r#""turns":0,"spent":0,"#,
                    r#""usage":{{"input_tokens":null,"cache_read_tokens":null,"cache_write_tokens":null,"#,
                    r#""output_tokens":null,"reasoning_tokens":null}},"#,
                    r#""read":null,"unread":0,"duration_ms":{now_ms}}}"#,
                ),
                account = ACCOUNT,
                run = RUN,
                now_ms = NOW_MS,
            ),
        },
    ] {
        let meta = crate::AnswerMeta { now: NOW, run: RUN, wind_down: false, read: None, unread: 0, elapsed: ELAPSED };
        let answer = run::Answer::Failed {
            failure: run::Failure::Model(run::Fault::Limited { account: ACCOUNT, reset_at: row.reset_at }),
            spent: run::Spend::ZERO,
            turns: 0,
        };
        let record = smith_events::Record {
            t_ms: NOW_MS,
            event: smith_events::Event::RunCompleted(crate::answer_record(meta, &answer, crate::sinks::Capture::None)),
        };
        let expected = row.expected.as_bytes();
        assert_stream_record(&record, expected);
        assert_eq!(
            record.event,
            smith_events::read(expected, &stream_limits()).expect(row.name).unwrap().event,
            "{}",
            row.name
        );
    }
}
