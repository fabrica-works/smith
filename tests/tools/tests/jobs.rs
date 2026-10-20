//! Commands yield into numbered jobs without retaining their batch place.
use skein_fake_checkout::Exit;
use skein_lib::{Duration, Time};
use smith_domain_tools::{
    self as tools, BytesClamp, Call, Grants, JobEnd, JobNumber, JobProblem, Outcome, TimeoutClamp,
};
use smith_tools_world::calls::{edit, job, read, shell, shell_after};
use smith_tools_world::fixture::{Fixture, LIB, program};
use smith_tools_world::{Settings, Span, Step, World};

fn run(
    settings: Settings,
    fixture: Fixture,
    writes: &[(Duration, &[u8])],
    script: Vec<Step>,
) -> (Vec<Result<Outcome, JobProblem>>, World) {
    let authority = fixture.authority(Grants { inspect: true, modify: true, shell: true });
    let mut world = World::new(settings, fixture.checkout);
    world.job_output(b"cargo build", writes);
    let session = world.session(Time::ZERO, authority, script);
    world.run(10_000);
    (world.results(session).into_iter().cloned().collect(), world)
}

fn settings() -> Settings {
    let mut settings = Settings::calm(1);
    settings.think = Span::millis(0, 0);
    settings
}

fn build() -> [(Duration, &'static [u8]); 2] {
    [(Duration::ZERO, b"Compiling\n"), (Duration::from_secs(2), b"Finished\n")]
}

#[test]
fn a_command_past_its_yield_is_answered_running_then_read_to_its_end() {
    let script = vec![Step::Calls(vec![shell_after(b"cargo build", 1)]), Step::Calls(vec![job(1, 5)])];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
}

// Distinct small durations exercise domain/tools.md, section 5, Reading it.
fn poll_with_separate_default_and_ceiling(wait: tools::Wait) -> Vec<Result<Outcome, JobProblem>> {
    let mut settings = settings();
    settings.tools.job_poll = Duration::from_secs(1);
    settings.tools.job_wait = Duration::from_secs(3);
    let poll = Call::Job { job: JobNumber(1), wait, output: tools::Budget::Default, stop: false };
    let script = vec![Step::Calls(vec![shell_after(b"cargo build", 0)]), Step::Calls(vec![poll])];
    let (answers, _) = run(settings, Fixture::new(), &build(), script);
    answers
}

#[test]
fn an_unasked_wait_uses_job_poll_before_the_job_ends() {
    let answers = poll_with_separate_default_and_ceiling(tools::Wait::Default);
    assert_eq!(
        answers.get(1),
        Some(&Ok(Outcome::Job {
            job: JobNumber(1),
            head: Box::default(),
            tail: Box::default(),
            dropped: 0,
            output: BytesClamp::Within,
            wait: TimeoutClamp::Within,
            end: JobEnd::Running,
        }))
    );
}

#[test]
fn a_wait_between_job_poll_and_job_wait_reaches_the_end() {
    let answers = poll_with_separate_default_and_ceiling(tools::Wait::Asked(Duration::from_secs(2)));
    assert_eq!(
        answers.get(1),
        Some(&Ok(Outcome::Job {
            job: JobNumber(1),
            head: b"Finished\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            output: BytesClamp::Within,
            wait: TimeoutClamp::Within,
            end: JobEnd::Ended(tools::Exit::Code { code: 0 }),
        }))
    );
}

fn read_with_dropped_output() -> Vec<Result<Outcome, JobProblem>> {
    let mut fixture = Fixture::new();
    fixture.checkout.program(b"cargo build", program(4_000, b"start\nabcdefghijklmnopend\n", Exit::Code(0), &[]));
    let mut poll = job(1, 1);
    if let Call::Job { output, .. } = &mut poll {
        *output = tools::Budget::Asked(6);
    }
    let writes = [
        (Duration::ZERO, b"start\n".as_slice()),
        (Duration::from_secs(2), b"abcdefghijklmnop"),
        (Duration::from_secs(4), b"end\n"),
    ];
    let (answers, _) = run(
        settings(),
        fixture,
        &writes,
        vec![Step::Calls(vec![shell_after(b"cargo build", 1)]), Step::Calls(vec![poll]), Step::Calls(vec![job(1, 5)])],
    );
    answers
}

#[test]
fn a_jobs_read_past_its_kept_ceiling_counts_the_dropped_bytes_once() {
    let answers = read_with_dropped_output();
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"start\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"ab".as_slice().into(),
                tail: b"mnop".as_slice().into(),
                dropped: 10,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Running
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"end\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
}

fn yield_and_read_with_clamps() -> Vec<Result<Outcome, JobProblem>> {
    let mut settings = settings();
    settings.tools.job_poll = Duration::from_secs(1);
    settings.tools.job_wait = Duration::from_secs(1);
    let mut fixture = Fixture::new();
    fixture.checkout.program(b"cargo build", program(3_000, b"Compiling\nFinished\n", Exit::Code(0), &[]));
    let writes = [(Duration::ZERO, b"Compiling\n".as_slice()), (Duration::from_secs(3), b"Finished\n")];
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 2)]),
        Step::Calls(vec![job(1, 2)]),
        Step::Calls(vec![job(1, 1)]),
    ];
    let (answers, _) = run(settings, fixture, &writes, script);
    answers
}

#[test]
fn a_yield_and_a_wait_past_job_wait_are_clamped_and_said() {
    let answers = yield_and_read_with_clamps();
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Clamped { asked: Duration::from_secs(2), ceiling: Duration::from_secs(1) }
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: Box::default(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Clamped { asked: Duration::from_secs(2), ceiling: Duration::from_secs(1) },
                end: JobEnd::Running
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
}

fn calls_beside_a_job() -> (Vec<Result<Outcome, JobProblem>>, World) {
    let mut settings = settings();
    settings.tools.calls = 1;
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![read(b"src/lib.rs")]),
        Step::Calls(vec![edit(b"src/lib.rs", b"one", b"four", false)]),
        Step::Calls(vec![shell(b"kill -9 $$", None)]),
        Step::Calls(vec![job(1, 5)]),
    ];
    let (answers, world) = run(settings, Fixture::new(), &build(), script);
    (answers, world)
}

#[test]
fn reads_edits_and_commands_run_while_a_job_runs() {
    let (answers, world) = calls_beside_a_job();
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Ok(Outcome::Read {
                content: LIB.into(),
                first: 1,
                lines: 3,
                total: 3,
                next: tools::Next::End,
                cut: tools::Cut::Whole,
                window: BytesClamp::Within
            }),
            Ok(Outcome::Edited { replaced: 1 }),
            Ok(Outcome::Exited {
                exit: tools::Exit::Signal { signal: 9 },
                head: Box::default(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
    assert_eq!(world.stats().commands, 2);
}

#[test]
fn a_yield_while_jobs_jobs_run_is_an_invalid_call() {
    let mut settings = settings();
    settings.tools.jobs = 1;
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![job(1, 5)]),
    ];
    let (answers, world) = run(settings, Fixture::new(), &build(), script);
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Err(JobProblem::Full { jobs: 1 }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
    assert_eq!(world.stats().commands, 1);
}

#[test]
fn a_job_call_naming_an_unknown_or_answered_job_is_invalid() {
    let script = vec![
        Step::Calls(vec![job(1, 5)]),
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![job(1, 5)]),
        Step::Calls(vec![job(1, 5)]),
    ];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers,
        vec![
            Err(JobProblem::Unknown { job: JobNumber(1) }),
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
            Err(JobProblem::Unknown { job: JobNumber(1) }),
        ]
    );
}

#[test]
fn two_reads_of_one_job_wait_their_turn() {
    let script = vec![Step::Calls(vec![shell_after(b"cargo build", 1)]), Step::Calls(vec![job(1, 0), job(1, 5)])];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: Box::default(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Running
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
}

fn stop(number: u32) -> Call {
    let mut call = job(number, 5);
    if let Call::Job { stop, .. } = &mut call {
        *stop = true;
    }
    call
}

#[test]
fn a_job_stopped_and_one_stopped_after_it_exited() {
    let script = vec![Step::Calls(vec![shell_after(b"cargo build", 1)]), Step::Calls(vec![stop(1)])];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: Box::default(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Stopped
            }),
        ]
    );
}

#[test]
fn a_job_stopped_after_its_exit_preserves_that_exit() {
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Sleep(Duration::from_secs(2)),
        Step::Calls(vec![stop(1)]),
    ];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers,
        vec![
            Ok(Outcome::Running {
                job: JobNumber(1),
                head: b"Compiling\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                timeout: TimeoutClamp::Within,
                output: BytesClamp::Within,
                after: TimeoutClamp::Within
            }),
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
}

#[test]
fn a_close_with_a_job_running_settles_it_before_closed() {
    let (answers, world) =
        run(settings(), Fixture::new(), &build(), vec![Step::Calls(vec![shell_after(b"cargo build", 1)])]);
    assert_eq!(
        answers,
        vec![Ok(Outcome::Running {
            job: JobNumber(1),
            head: b"Compiling\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            timeout: TimeoutClamp::Within,
            output: BytesClamp::Within,
            after: TimeoutClamp::Within
        })]
    );
    assert_eq!(world.stats().cancels, 1);
}

#[test]
fn a_yielded_command_keeps_its_original_deadline_clamp() {
    let mut settings = settings();
    settings.tools.tool_deadline = Duration::from_secs(60);
    let mut call = shell_after(b"cargo build", 1);
    if let Call::Shell { timeout, .. } = &mut call {
        *timeout = tools::Timeout::Asked(Duration::from_secs(180));
    }
    let (answers, _) =
        run(settings, Fixture::new(), &build(), vec![Step::Calls(vec![call]), Step::Calls(vec![job(1, 5)])]);
    assert_eq!(
        answers[0],
        Ok(Outcome::Running {
            job: JobNumber(1),
            head: b"Compiling\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            timeout: TimeoutClamp::Clamped { asked: Duration::from_secs(180), ceiling: Duration::from_secs(60) },
            output: BytesClamp::Within,
            after: TimeoutClamp::Within,
        })
    );
}

#[test]
fn different_jobs_can_be_read_in_the_same_batch() {
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![job(1, 5), job(2, 5)]),
    ];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers[2..],
        [
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
            Ok(Outcome::Job {
                job: JobNumber(2),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Ended(tools::Exit::Code { code: 0 })
            }),
        ]
    );
}

#[test]
fn a_command_ending_before_its_yield_answers_its_exit() {
    let (answers, world) =
        run(settings(), Fixture::new(), &build(), vec![Step::Calls(vec![shell_after(b"cargo build", 5)])]);
    assert_eq!(
        answers,
        vec![Ok(Outcome::Exited {
            exit: tools::Exit::Code { code: 0 },
            head: b"Compiling\nFinished\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            timeout: TimeoutClamp::Within,
            output: BytesClamp::Within,
        })]
    );
    assert_eq!(world.stats().cancels, 0);
}

#[test]
fn an_exited_unread_job_still_occupies_its_slot() {
    let mut settings = settings();
    settings.tools.jobs = 1;
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Sleep(Duration::from_secs(2)),
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![job(1, 5)]),
    ];
    let (answers, world) = run(settings, Fixture::new(), &build(), script);
    assert_eq!(answers[1], Err(JobProblem::Full { jobs: 1 }));
    assert_eq!(world.stats().commands, 1);
}

#[test]
fn a_finished_job_number_is_not_used_again() {
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![job(1, 5)]),
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Calls(vec![job(2, 5)]),
    ];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers[2],
        Ok(Outcome::Running {
            job: JobNumber(2),
            head: b"Compiling\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            timeout: TimeoutClamp::Within,
            output: BytesClamp::Within,
            after: TimeoutClamp::Within,
        })
    );
}

#[test]
fn a_job_past_its_deadline_answers_timed_out() {
    let mut fixture = Fixture::new();
    fixture.checkout.program(b"cargo build", program(4_000, b"Compiling\nFinished\n", Exit::Code(0), &[]));
    let mut call = shell_after(b"cargo build", 1);
    if let Call::Shell { timeout, .. } = &mut call {
        *timeout = tools::Timeout::Asked(Duration::from_secs(2));
    }
    let writes = [(Duration::ZERO, b"Compiling\n".as_slice()), (Duration::from_secs(4), b"Finished\n")];
    let (answers, _) = run(settings(), fixture, &writes, vec![Step::Calls(vec![call]), Step::Calls(vec![job(1, 5)])]);
    assert_eq!(
        answers[1],
        Ok(Outcome::Job {
            job: JobNumber(1),
            head: Box::default(),
            tail: Box::default(),
            dropped: 0,
            output: BytesClamp::Within,
            wait: TimeoutClamp::Within,
            end: JobEnd::Ended(tools::Exit::TimedOut)
        })
    );
}

fn cancel_builds(reads: Vec<Call>) -> (Vec<Result<Outcome, JobProblem>>, World) {
    let mut fixture = Fixture::new();
    fixture.checkout.program(b"cargo build", program(30_000, b"Compiling\nFinished\n", Exit::Code(0), &[]));
    let writes = [(Duration::ZERO, b"Compiling\n".as_slice()), (Duration::from_secs(3), b"Finished\n")];
    run(
        settings(),
        fixture,
        &writes,
        vec![
            Step::Calls(vec![shell_after(b"cargo build", 1)]),
            Step::Calls(vec![shell_after(b"cargo build", 1)]),
            Step::Sleep(Duration::from_secs(2)),
            Step::CancelJobs,
            Step::Calls(reads),
        ],
    )
}

#[test]
fn jobs_cancelled_for_a_check_answer_cancelled_at_their_next_read() {
    let (answers, world) = cancel_builds(vec![job(1, 5), job(2, 5)]);
    assert_eq!(
        answers[2..],
        [
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Cancelled
            }),
            Ok(Outcome::Job {
                job: JobNumber(2),
                head: b"Finished\n".as_slice().into(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Cancelled
            }),
        ]
    );
    assert_eq!(world.stats().cancels, 2);
}

#[test]
fn a_cancelled_jobs_read_applies_its_own_output_budget() {
    let mut poll = job(1, 5);
    if let Call::Job { output, .. } = &mut poll {
        *output = tools::Budget::Asked(6);
    }
    let (answers, _) = cancel_builds(vec![poll, job(2, 5)]);
    assert_eq!(
        answers[2],
        Ok(Outcome::Job {
            job: JobNumber(1),
            head: b"Fi".as_slice().into(),
            tail: b"hed\n".as_slice().into(),
            dropped: 3,
            output: BytesClamp::Within,
            wait: TimeoutClamp::Within,
            end: JobEnd::Cancelled
        })
    );
}

#[test]
fn a_close_with_one_job_running_and_one_exited_unread_tells_each_once() {
    let script =
        vec![Step::Calls(vec![shell_after(b"cargo build", 1)]), Step::Calls(vec![shell_after(b"cargo build", 1)])];
    let (_, world) = run(settings(), Fixture::new(), &build(), script);
    let facts: Vec<_> = world
        .sessions()
        .flat_map(|session| world.job_facts(session))
        .map(|fact| {
            let tools::FactKind::JobEnded { job, end, unread_bytes, started, .. } = fact.kind else { unreachable!() };
            (job, end, unread_bytes, started, fact.at)
        })
        .collect();
    assert_eq!(
        facts,
        vec![
            (
                JobNumber(1),
                JobEnd::Ended(tools::Exit::Code { code: 0 }),
                9,
                Time::ZERO,
                Time::ZERO.saturating_add(Duration::from_secs(2))
            ),
            (
                JobNumber(2),
                JobEnd::Cancelled,
                0,
                Time::ZERO.saturating_add(Duration::from_secs(1)),
                Time::ZERO.saturating_add(Duration::from_secs(2))
            ),
        ]
    );
}

#[test]
fn cancellation_settles_an_outstanding_read_before_it_acknowledges() {
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Send(vec![job(1, 5)]),
        Step::CancelJobs,
        Step::Calls(vec![job(1, 5)]),
    ];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers[1..],
        [
            Ok(Outcome::Job {
                job: JobNumber(1),
                head: Box::default(),
                tail: Box::default(),
                dropped: 0,
                output: BytesClamp::Within,
                wait: TimeoutClamp::Within,
                end: JobEnd::Cancelled
            }),
            Err(JobProblem::Unknown { job: JobNumber(1) }),
        ]
    );
}

#[test]
fn cancelling_an_already_exited_job_preserves_its_observed_exit() {
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", 1)]),
        Step::Sleep(Duration::from_secs(2)),
        Step::CancelJobs,
        Step::Calls(vec![job(1, 5)]),
    ];
    let (answers, _) = run(settings(), Fixture::new(), &build(), script);
    assert_eq!(
        answers[1],
        Ok(Outcome::Job {
            job: JobNumber(1),
            head: b"Finished\n".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            output: BytesClamp::Within,
            wait: TimeoutClamp::Within,
            end: JobEnd::Ended(tools::Exit::Code { code: 0 })
        })
    );
}

fn read_ended_early(
    polls: Vec<Call>,
    index: usize,
    before: Duration,
    elapsed: Duration,
) -> Vec<Result<Outcome, JobProblem>> {
    const COMMAND_MILLIS: u64 = 4_000;
    const YIELD_SECONDS: u64 = 1;
    const SETTLE_SECONDS: u64 = 5;
    let mut fixture = Fixture::new();
    fixture.checkout.program(b"cargo build", program(COMMAND_MILLIS, b"Compiling\nFinished\n", Exit::Code(0), &[]));
    let script = vec![
        Step::Calls(vec![shell_after(b"cargo build", YIELD_SECONDS)]),
        Step::Sleep(before),
        Step::Send(polls),
        Step::Sleep(elapsed),
        Step::EndWait(index),
        Step::Sleep(Duration::from_secs(SETTLE_SECONDS)),
    ];
    run(settings(), fixture, &build(), script).0
}

fn job_answer(head: &[u8], end: JobEnd) -> Outcome {
    Outcome::Job {
        job: JobNumber(1),
        head: head.into(),
        tail: Box::default(),
        dropped: 0,
        output: BytesClamp::Within,
        wait: TimeoutClamp::Within,
        end,
    }
}

#[test]
fn a_wait_ended_early_answers_running_with_the_output_so_far() {
    const WAIT_SECONDS: u64 = 5;
    const ELAPSED_MILLIS: u64 = 1_100;
    const CALL_INDEX: usize = 1;
    let answers =
        read_ended_early(vec![job(1, WAIT_SECONDS)], CALL_INDEX, Duration::ZERO, Duration::from_millis(ELAPSED_MILLIS));
    assert_eq!(answers[CALL_INDEX], Ok(job_answer(b"Finished\n", JobEnd::Running)));
}

#[test]
fn a_wait_ended_after_the_job_ended_answers_its_end() {
    const WAIT_SECONDS: u64 = 5;
    const BEFORE_SECONDS: u64 = 5;
    const CALL_INDEX: usize = 1;
    let answers =
        read_ended_early(vec![job(1, WAIT_SECONDS)], CALL_INDEX, Duration::from_secs(BEFORE_SECONDS), Duration::ZERO);
    assert_eq!(answers[CALL_INDEX], Ok(job_answer(b"Finished\n", JobEnd::Ended(tools::Exit::Code { code: 0 }))));
}

#[test]
fn a_queued_read_ended_early_answers_when_it_starts() {
    const FIRST_WAIT_SECONDS: u64 = 1;
    const QUEUED_WAIT_SECONDS: u64 = 5;
    const CALL_INDEX: usize = 2;
    let answers = read_ended_early(
        vec![job(1, FIRST_WAIT_SECONDS), job(1, QUEUED_WAIT_SECONDS)],
        CALL_INDEX,
        Duration::ZERO,
        Duration::ZERO,
    );
    assert_eq!(answers[CALL_INDEX], Ok(job_answer(b"", JobEnd::Running)));
}
