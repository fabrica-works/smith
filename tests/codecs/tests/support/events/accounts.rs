//! Account and program failure fixtures for the version-four vocabulary.
use super::*;
use smith_events::AccountState;

pub(super) fn samples() -> Vec<(&'static str, Record, Capture)> {
    struct NoticeSample {
        name: &'static str,
        kind: NoticeKind,
        reset: Option<u64>,
        code: Option<&'static [u8]>,
    }
    struct FailureSample {
        name: &'static str,
        state: AccountState,
        reset: Option<u64>,
        code: Option<&'static [u8]>,
    }
    let mut samples = Vec::new();
    for row in [
        NoticeSample {
            name: "notice_account_limited_unknown_reset",
            kind: NoticeKind::AccountLimited,
            reset: None,
            code: None,
        },
        NoticeSample {
            name: "notice_account_refused",
            kind: NoticeKind::AccountRefused,
            reset: None,
            code: Some(b"not_allowed"),
        },
    ] {
        let mut value = value_notice("", "");
        value.kind = row.kind;
        value.reset_ms = row.reset;
        value.code = row.code.map(Box::from);
        samples.push((row.name, Record { t_ms: 0, event: Event::Notice(value) }, Capture::Everything));
    }
    for row in [
        FailureSample { name: "answer_account_limited", state: AccountState::Limited, reset: Some(73), code: None },
        FailureSample {
            name: "answer_account_limited_unknown_reset",
            state: AccountState::Limited,
            reset: None,
            code: None,
        },
        FailureSample {
            name: "answer_account_refused",
            state: AccountState::Refused,
            reset: None,
            code: Some(b"not_allowed"),
        },
        FailureSample {
            name: "answer_account_unauthorized",
            state: AccountState::Unauthorized,
            reset: None,
            code: None,
        },
        FailureSample {
            name: "answer_account_unknown",
            state: AccountState::Unknown(b"future".as_slice().into()),
            reset: None,
            code: None,
        },
    ] {
        let mut run = value_run_completed("Status", "failed");
        let value = run.failure.as_mut().expect("failed run fixture");
        value.class = AnswerClass::Model;
        value.state = Some(row.state);
        value.reset_ms = row.reset;
        value.code = row.code.map(Box::from);
        value.completion = None;
        samples.push((row.name, Record { t_ms: 0, event: Event::RunCompleted(run) }, Capture::Everything));
    }
    for (name, capture, exit) in [
        ("answer_program_none", Capture::None, None),
        ("answer_program_calls", Capture::Calls, Some(AgentExit::Code(i64::MIN))),
        ("answer_program_everything", Capture::Everything, Some(AgentExit::Signal(u64::MAX))),
    ] {
        let mut run = value_run_completed("Status", "failed");
        let value = run.failure.as_mut().expect("failed run fixture");
        value.class = AnswerClass::Model;
        value.account = None;
        value.program = Some(b"claude_code".as_slice().into());
        value.exit = exit;
        value.detail = Some(b"program content".as_slice().into());
        value.completion = None;
        samples.push((name, Record { t_ms: 0, event: Event::RunCompleted(run) }, capture));
    }
    samples
}
