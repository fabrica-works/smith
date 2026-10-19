//! Version four job outcomes preserve what live rendering receives.

use smith_domain_session::{llm, record};
use smith_domain_tools::{BytesClamp, Exit, JobEnd, JobNumber, Outcome, TimeoutClamp};

fn round_trip(outcome: Outcome) -> Result<Outcome, record::Refusal> {
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0 }).unwrap();
    let endpoints = crate::Endpoints::new(configured);
    let limits = smith_transcript::CEILINGS;
    let mut turn = crate::decode_turn(&super::saved_turn(1, 0), 1, &limits, &endpoints).unwrap();
    turn.messages = Box::new([llm::Message {
        role: llm::Role::User,
        content: Box::new([llm::Block::ToolResult {
            id: b"job-call".as_slice().into(),
            result: llm::Returned::Owned { outcome },
        }]),
    }]);
    let bytes = crate::encode_turn(&turn, &limits, &endpoints).unwrap();
    let decoded = crate::decode_turn(&bytes, 1, &limits, &endpoints)?;
    match &decoded.messages[0].content[0] {
        llm::Block::ToolResult { result: llm::Returned::Owned { outcome }, .. } => Ok(outcome.clone()),
        other @ (llm::Block::Opaque { .. }
        | llm::Block::Refusal { .. }
        | llm::Block::Text { .. }
        | llm::Block::ToolCall { .. }
        | llm::Block::ToolResult { .. }) => panic!("expected job outcome: {other:?}"),
    }
}

fn read(end: JobEnd) -> Outcome {
    Outcome::Job {
        job: JobNumber(7),
        head: b"new".as_slice().into(),
        tail: b"last".as_slice().into(),
        dropped: 11,
        output: BytesClamp::Clamped { asked: 20, ceiling: 10 },
        wait: TimeoutClamp::Clamped {
            asked: skein_lib::Duration::from_secs(90),
            ceiling: skein_lib::Duration::from_secs(30),
        },
        end,
    }
}

#[test]
fn a_running_commands_number_and_output_survive_resume() {
    let outcome = Outcome::Running {
        timeout: TimeoutClamp::Within,
        job: JobNumber(3),
        head: b"start".as_slice().into(),
        tail: b"end".as_slice().into(),
        dropped: 8,
        output: BytesClamp::Within,
        after: TimeoutClamp::Within,
    };
    assert_eq!(
        round_trip(outcome),
        Ok(Outcome::Running {
            timeout: TimeoutClamp::Within,
            job: JobNumber(3),
            head: b"start".as_slice().into(),
            tail: b"end".as_slice().into(),
            dropped: 8,
            output: BytesClamp::Within,
            after: TimeoutClamp::Within,
        })
    );
}

#[test]
fn a_running_job_read_keeps_its_clamps_on_resume() {
    assert_eq!(
        round_trip(read(JobEnd::Running)),
        Ok(Outcome::Job {
            job: JobNumber(7),
            head: b"new".as_slice().into(),
            tail: b"last".as_slice().into(),
            dropped: 11,
            output: BytesClamp::Clamped { asked: 20, ceiling: 10 },
            wait: TimeoutClamp::Clamped {
                asked: skein_lib::Duration::from_secs(90),
                ceiling: skein_lib::Duration::from_secs(30)
            },
            end: JobEnd::Running,
        })
    );
}

#[test]
fn a_job_exit_code_and_unread_output_survive_resume() {
    assert_eq!(
        round_trip(read(JobEnd::Ended(Exit::Code { code: 17 }))),
        Ok(Outcome::Job {
            job: JobNumber(7),
            head: b"new".as_slice().into(),
            tail: b"last".as_slice().into(),
            dropped: 11,
            output: BytesClamp::Clamped { asked: 20, ceiling: 10 },
            wait: TimeoutClamp::Clamped {
                asked: skein_lib::Duration::from_secs(90),
                ceiling: skein_lib::Duration::from_secs(30)
            },
            end: JobEnd::Ended(Exit::Code { code: 17 }),
        })
    );
}

#[test]
fn a_job_signal_and_unread_output_survive_resume() {
    assert_eq!(
        round_trip(read(JobEnd::Ended(Exit::Signal { signal: 9 }))),
        Ok(Outcome::Job {
            job: JobNumber(7),
            head: b"new".as_slice().into(),
            tail: b"last".as_slice().into(),
            dropped: 11,
            output: BytesClamp::Clamped { asked: 20, ceiling: 10 },
            wait: TimeoutClamp::Clamped {
                asked: skein_lib::Duration::from_secs(90),
                ceiling: skein_lib::Duration::from_secs(30)
            },
            end: JobEnd::Ended(Exit::Signal { signal: 9 }),
        })
    );
}

#[test]
fn a_jobs_deadline_end_and_unread_output_survive_resume() {
    assert_eq!(
        round_trip(read(JobEnd::Ended(Exit::TimedOut))),
        Ok(Outcome::Job {
            job: JobNumber(7),
            head: b"new".as_slice().into(),
            tail: b"last".as_slice().into(),
            dropped: 11,
            output: BytesClamp::Clamped { asked: 20, ceiling: 10 },
            wait: TimeoutClamp::Clamped {
                asked: skein_lib::Duration::from_secs(90),
                ceiling: skein_lib::Duration::from_secs(30)
            },
            end: JobEnd::Ended(Exit::TimedOut),
        })
    );
}

#[test]
fn a_stopped_job_and_unread_output_survive_resume() {
    assert_eq!(
        round_trip(read(JobEnd::Stopped)),
        Ok(Outcome::Job {
            job: JobNumber(7),
            head: b"new".as_slice().into(),
            tail: b"last".as_slice().into(),
            dropped: 11,
            output: BytesClamp::Clamped { asked: 20, ceiling: 10 },
            wait: TimeoutClamp::Clamped {
                asked: skein_lib::Duration::from_secs(90),
                ceiling: skein_lib::Duration::from_secs(30)
            },
            end: JobEnd::Stopped,
        })
    );
}

#[test]
fn a_cancelled_job_and_unread_output_survive_resume() {
    assert_eq!(
        round_trip(read(JobEnd::Cancelled)),
        Ok(Outcome::Job {
            job: JobNumber(7),
            head: b"new".as_slice().into(),
            tail: b"last".as_slice().into(),
            dropped: 11,
            output: BytesClamp::Clamped { asked: 20, ceiling: 10 },
            wait: TimeoutClamp::Clamped {
                asked: skein_lib::Duration::from_secs(90),
                ceiling: skein_lib::Duration::from_secs(30)
            },
            end: JobEnd::Cancelled,
        })
    );
}

#[test]
fn a_job_number_zero_is_refused_on_resume() {
    let outcome = Outcome::Running {
        timeout: TimeoutClamp::Within,
        job: JobNumber(0),
        head: Box::default(),
        tail: Box::default(),
        dropped: 0,
        output: BytesClamp::Within,
        after: TimeoutClamp::Within,
    };
    assert_eq!(round_trip(outcome), Err(record::Refusal::Malformed));
}

#[test]
fn a_running_commands_deadline_clamp_survives_its_resume() {
    let outcome = Outcome::Running {
        job: JobNumber(3),
        head: Box::default(),
        tail: Box::default(),
        dropped: 0,
        timeout: TimeoutClamp::Clamped {
            asked: skein_lib::Duration::from_secs(180),
            ceiling: skein_lib::Duration::from_secs(120),
        },
        output: BytesClamp::Within,
        after: TimeoutClamp::Within,
    };
    assert_eq!(
        round_trip(outcome),
        Ok(Outcome::Running {
            job: JobNumber(3),
            head: Box::default(),
            tail: Box::default(),
            dropped: 0,
            timeout: TimeoutClamp::Clamped {
                asked: skein_lib::Duration::from_secs(180),
                ceiling: skein_lib::Duration::from_secs(120)
            },
            output: BytesClamp::Within,
            after: TimeoutClamp::Within,
        })
    );
}

fn round_trip_problem(problem: llm::Problem) -> llm::Problem {
    let mut configured = skein_lib::List::with_capacity(1);
    configured.push(crate::Endpoint { name: Box::default(), number: 3, dialect: 0 }).unwrap();
    let endpoints = crate::Endpoints::new(configured);
    let limits = smith_transcript::CEILINGS;
    let mut turn = crate::decode_turn(&super::saved_turn(1, 0), 1, &limits, &endpoints).unwrap();
    turn.messages = Box::new([llm::Message {
        role: llm::Role::User,
        content: Box::new([llm::Block::ToolResult {
            id: b"invalid".as_slice().into(),
            result: llm::Returned::Invalid { problem },
        }]),
    }]);
    let bytes = crate::encode_turn(&turn, &limits, &endpoints).unwrap();
    let decoded = crate::decode_turn(&bytes, 1, &limits, &endpoints).unwrap();
    match &decoded.messages[0].content[0] {
        llm::Block::ToolResult { result: llm::Returned::Invalid { problem }, .. } => problem.clone(),
        other @ (llm::Block::Opaque { .. }
        | llm::Block::Refusal { .. }
        | llm::Block::Text { .. }
        | llm::Block::ToolCall { .. }
        | llm::Block::ToolResult { .. }) => panic!("expected invalid result: {other:?}"),
    }
}

#[test]
fn a_full_job_tables_invalid_problem_survives_its_resume() {
    assert_eq!(round_trip_problem(llm::Problem::JobsFull { jobs: 4 }), llm::Problem::JobsFull { jobs: 4 });
}

#[test]
fn an_unknown_jobs_invalid_problem_survives_its_resume() {
    assert_eq!(round_trip_problem(llm::Problem::UnknownJob { job: 17 }), llm::Problem::UnknownJob { job: 17 });
}
