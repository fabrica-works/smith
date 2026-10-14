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
        timeout: TimeoutClamp::Within,
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
            job: Some(JobNumber(2)),
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
            job: Some(JobNumber(2)),
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
            job: Some(JobNumber(2)),
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
            job: Some(JobNumber(2)),
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
            job: Some(JobNumber(2)),
            clamped: true
        }
    );
}

#[test]
fn a_running_commands_fact_preserves_its_deadline_clamp_alone() {
    let outcome = Outcome::Running {
        job: JobNumber(2),
        head: Box::default(),
        tail: Box::default(),
        dropped: 0,
        output: BytesClamp::Within,
        after: TimeoutClamp::Within,
        timeout: TimeoutClamp::Clamped { asked: Duration::from_secs(180), ceiling: Duration::from_secs(120) },
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
            bytes: 0,
            omitted_bytes: 0,
            job: Some(JobNumber(2)),
            clamped: true
        }
    );
}

#[test]
fn parallel_job_reads_reserve_each_calls_capture_budget() {
    let limits = crate::Limits {
        read_bytes: 1,
        tool_payload: 1,
        list_bytes: 1,
        search_hits: 0,
        search_bytes: 0,
        path_bytes: 1,
        ..super::LIMITS
    };
    assert_eq!(crate::batch_result_worst_case(&limits, 3), Some(576));
}

#[test]
fn a_job_read_has_the_read_effect() {
    let call =
        crate::Call::Job { job: JobNumber(1), wait: crate::Wait::Default, output: crate::Budget::Default, stop: false };
    assert_eq!(crate::effect(&call), crate::Effect::Read);
}

#[test]
fn a_job_stop_has_the_write_effect() {
    let call =
        crate::Call::Job { job: JobNumber(1), wait: crate::Wait::Default, output: crate::Budget::Default, stop: true };
    assert_eq!(crate::effect(&call), crate::Effect::Write);
}

#[test]
fn a_resumed_kits_first_job_follows_its_saved_highest() {
    use super::{ALL, Harness, LIMITS, authority, bytes};
    use crate::{Budget, Call, Event, Request, Timeout, Yield};
    let mut harness = Harness::new(LIMITS);
    let opened = harness.step(Event::Open { session: Token::new(1), jobs: 3, authority: authority(ALL) });
    let Some(Request::Opened { kit, .. }) = opened else { panic!("kit admitted") };
    let spawned = harness.send(
        kit,
        1,
        Call::Shell {
            command: bytes(b"cargo build"),
            timeout: Timeout::Default,
            output: Budget::Default,
            after: Yield::Asked(Duration::from_secs(1)),
        },
    );
    let Some(Request::Io { owner, .. }) = spawned else { panic!("shell spawned") };
    assert_eq!(
        harness.step(Event::Done {
            owner,
            done: crate::Done::Held { head: Box::default(), tail: Box::default(), dropped: 0 }
        }),
        Some(Request::Answer {
            to: skein_lib::ReplyTo::new(Token::new(1)),
            outcome: Outcome::Running {
                job: JobNumber(4),
                head: Box::default(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within,
            }
        })
    );
}
