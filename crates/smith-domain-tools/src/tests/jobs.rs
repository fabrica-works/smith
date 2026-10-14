//! Content-free observations of the job outcomes used by live calls.

use crate::{BytesClamp, Exit, FactKind, JobEnd, JobNumber, Outcome, TimeoutClamp, Tool, Verdict};
use skein_lib::{Duration, Token};

fn observe(end: JobEnd) -> FactKind {
    crate::facts::answered(
        Token::new(1),
        Tool::Shell,
        &Outcome::Job {
            job: JobNumber(2),
            head: b"abc".as_slice().into(),
            tail: b"de".as_slice().into(),
            dropped: 7,
            output: BytesClamp::Clamped { asked: 90, ceiling: 5 },
            wait: TimeoutClamp::Within,
            end,
        },
        crate::facts::Observation { omitted_bytes: 0, clamped: false },
    )
}

#[test]
fn a_running_commands_fact_keeps_its_loss_and_clamp() {
    let outcome = Outcome::Running {
        job: JobNumber(2),
        head: b"abc".as_slice().into(),
        tail: b"de".as_slice().into(),
        dropped: 7,
        output: BytesClamp::Within,
        after: TimeoutClamp::Clamped { asked: Duration::from_secs(90), ceiling: Duration::from_secs(30) },
    };
    assert_eq!(
        crate::facts::answered(
            Token::new(1),
            Tool::Shell,
            &outcome,
            crate::facts::Observation { omitted_bytes: 0, clamped: false }
        ),
        FactKind::Answered {
            session: Token::new(1),
            tool: Tool::Shell,
            verdict: Verdict::Running,
            bytes: 5,
            omitted_bytes: 7,
            clamped: true
        }
    );
}

#[test]
fn a_running_jobs_fact_keeps_its_loss_and_clamp() {
    assert_eq!(
        observe(JobEnd::Running),
        FactKind::Answered {
            session: Token::new(1),
            tool: Tool::Shell,
            verdict: Verdict::Running,
            bytes: 5,
            omitted_bytes: 7,
            clamped: true
        }
    );
}

#[test]
fn an_exited_jobs_fact_keeps_its_observed_exit() {
    assert_eq!(
        observe(JobEnd::Ended(Exit::Code { code: 17 })),
        FactKind::Answered {
            session: Token::new(1),
            tool: Tool::Shell,
            verdict: Verdict::Exited { exit: Exit::Code { code: 17 } },
            bytes: 5,
            omitted_bytes: 7,
            clamped: true
        }
    );
}

#[test]
fn a_stopped_jobs_fact_keeps_its_loss_and_clamp() {
    assert_eq!(
        observe(JobEnd::Stopped),
        FactKind::Answered {
            session: Token::new(1),
            tool: Tool::Shell,
            verdict: Verdict::Stopped,
            bytes: 5,
            omitted_bytes: 7,
            clamped: true
        }
    );
}

#[test]
fn a_cancelled_jobs_fact_keeps_its_loss_and_clamp() {
    assert_eq!(
        observe(JobEnd::Cancelled),
        FactKind::Answered {
            session: Token::new(1),
            tool: Tool::Shell,
            verdict: Verdict::Cancelled,
            bytes: 5,
            omitted_bytes: 7,
            clamped: true
        }
    );
}
