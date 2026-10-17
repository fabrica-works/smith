//! Commands held under their original token, on the real kernel (protocol/agent.md, 2).

use skein_lib::{Duration, Time, Token};
use smith_domain::tools;
use smith_machine_world::World;
use smith_protocol_machine::{FromDomain, ToDomain};

fn spawn(root: Token, command: &[u8], head: u32, tail: u32, deadline: Time) -> FromDomain {
    FromDomain::Op {
        owner: Token::new(10),
        op: tools::Op::Spawn {
            cwd: tools::Place { root, path: Box::default() },
            command: command.into(),
            head,
            tail,
            after: Some(Duration::from_millis(30)),
        },
        deadline,
    }
}

fn read(head: u32, tail: u32, wait: Duration) -> FromDomain {
    FromDomain::Job { owner: Token::new(10), act: tools::JobAct::Read { head, tail, wait } }
}

fn done(terminal: &ToDomain) -> tools::Done {
    match terminal {
        ToDomain::Done { owner, done } => {
            assert_eq!(*owner, Token::new(10));
            done.clone()
        }
        ToDomain::Read { .. } | ToDomain::Probed { .. } | ToDomain::Checked { .. } | ToDomain::Aborted { .. } => {
            panic!("job owes Done")
        }
    }
}

fn far() -> Time {
    Time::from_nanos(5_000_000_000)
}

#[test]
fn a_command_past_its_yield_is_held_and_read_to_its_end() {
    let mut world = World::new(71, &[]);
    let replies = world.jobs(vec![
        spawn(world.root(), b"printf one; sleep 0.08; printf two", 8, 8, far()),
        read(8, 8, Duration::from_secs(1)),
    ]);
    assert_eq!(
        done(&replies[0]),
        tools::Done::Held { head: b"one".as_slice().into(), tail: Box::default(), dropped: 0 }
    );
    assert_eq!(
        done(&replies[1]),
        tools::Done::Job {
            head: b"two".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            end: tools::JobEnd::Ended(tools::Exit::Code { code: 0 })
        }
    );
}

#[test]
fn a_jobs_output_past_its_kept_bound_is_head_tail_and_counted_once() {
    let mut world = World::new(72, &[]);
    let command = b"printf abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz; sleep 0.08; printf end";
    let replies = world.jobs(vec![
        spawn(world.root(), command, 4, 4, far()),
        read(4, 4, Duration::ZERO),
        read(4, 4, Duration::from_secs(1)),
    ]);
    assert_eq!(
        done(&replies[0]),
        tools::Done::Held { head: b"abcd".as_slice().into(), tail: b"wxyz".as_slice().into(), dropped: 70 }
    );
    assert_eq!(
        done(&replies[1]),
        tools::Done::Job { head: Box::default(), tail: Box::default(), dropped: 0, end: tools::JobEnd::Running }
    );
    assert_eq!(
        done(&replies[2]),
        tools::Done::Job {
            head: b"end".as_slice().into(),
            tail: Box::default(),
            dropped: 0,
            end: tools::JobEnd::Ended(tools::Exit::Code { code: 0 })
        }
    );
}

#[test]
fn a_stopped_job_answers_stopped() {
    let mut world = World::new(73, &[]);
    let replies = world.jobs(vec![
        spawn(world.root(), b"sleep 5", 8, 8, far()),
        FromDomain::Job { owner: Token::new(10), act: tools::JobAct::Stop { head: 8, tail: 8 } },
    ]);
    assert_eq!(
        done(&replies[1]),
        tools::Done::Job { head: Box::default(), tail: Box::default(), dropped: 0, end: tools::JobEnd::Stopped }
    );
}

#[test]
fn a_job_past_its_deadline_answers_timed_out() {
    let mut world = World::new(74, &[]);
    let replies = world.jobs(vec![
        spawn(world.root(), b"sleep 5", 8, 8, Time::from_nanos(60_000_000)),
        read(8, 8, Duration::from_secs(1)),
    ]);
    assert_eq!(
        done(&replies[1]),
        tools::Done::Job {
            head: Box::default(),
            tail: Box::default(),
            dropped: 0,
            end: tools::JobEnd::Ended(tools::Exit::TimedOut)
        }
    );
}

#[test]
fn a_job_held_at_the_machines_close_is_closed_within_one_close() {
    let mut world = World::new(75, &[]);
    let replies = world.jobs(vec![spawn(world.root(), b"sleep 5", 8, 8, far())]);
    assert_eq!(done(&replies[0]), tools::Done::Held { head: Box::default(), tail: Box::default(), dropped: 0 });
    assert!(world.now() < Time::from_nanos(1_030_000_000), "yield plus one io close");
}

#[test]
fn a_cancelled_held_job_answers_cancelled() {
    for (seed, command, delay, end) in [
        (76, b"sleep 5".as_slice(), Duration::ZERO, tools::JobEnd::Cancelled),
        (77, b"sleep 0.06".as_slice(), Duration::from_millis(80), tools::JobEnd::Ended(tools::Exit::Code { code: 0 })),
    ] {
        let mut world = World::new(seed, &[]);
        let replies = world.jobs_after(
            vec![spawn(world.root(), command, 8, 8, far()), FromDomain::Cancel { owner: Token::new(10) }],
            vec![delay],
        );
        assert_eq!(
            done(&replies[1]),
            tools::Done::Job { head: Box::default(), tail: Box::default(), dropped: 0, end },
            "seed {seed}"
        );
    }
}

#[test]
fn a_hastened_read_answers_at_once() {
    const OUTPUT_BYTES: u32 = 8;
    const WAIT_SECONDS: u64 = 1;
    const BEFORE_END_NANOS: u64 = 500_000_000;
    let mut world = World::new(78, &[]);
    let replies = world.jobs(vec![
        spawn(world.root(), b"sleep 1", OUTPUT_BYTES, OUTPUT_BYTES, far()),
        read(OUTPUT_BYTES, OUTPUT_BYTES, Duration::from_secs(WAIT_SECONDS)),
        FromDomain::Job { owner: Token::new(10), act: tools::JobAct::Hasten },
    ]);
    assert_eq!(
        done(&replies[1]),
        tools::Done::Job { head: Box::default(), tail: Box::default(), dropped: 0, end: tools::JobEnd::Running }
    );
    assert!(world.now() < Time::from_nanos(BEFORE_END_NANOS), "the read ends before its wait and command");
}
