//! Application deadlines and live parsing on the shared process-tree supervisor.
use std::path::PathBuf;

use skein_lib::Duration;
use skein_shell::Clock;
use skein_world::end_to_end::{Command, Method};
use smith_bench::process::{Bounds, Capture, execute};
use smith_bench::{End, Forced};

fn replay(tail: &str) -> Command {
    let recording = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/codex/completed.jsonl");
    Command {
        program: "/bin/sh".into(),
        arguments: vec![
            "-c".into(),
            format!("cat >/dev/null; cat \"$1\"; {tail}").into(),
            "replay".into(),
            recording.into_os_string(),
        ],
        environment: vec![("PATH".into(), "/usr/bin:/bin".into()), ("LANG".into(), "C.UTF-8".into())],
        directory: "/tmp".into(),
    }
}

fn bounds() -> Bounds {
    Bounds {
        hard: Duration::from_millis(180),
        teardown: Duration::from_millis(50),
        grace: Duration::from_millis(20),
        settlement: Duration::from_millis(200),
        stdout: 16384,
        stderr: 4096,
        force_walk: true,
    }
}

#[test]
fn a_recorded_answer_is_observed_before_the_tree_settles() {
    let clock = Clock::new();
    let result = execute(&clock, replay("exit 0"), Box::from(&b"answer the task\n"[..]), Capture::Codex, bounds())
        .expect("settled replay");
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.observer.classify(result.exit, result.forced).end, End::Completed);
    assert_eq!(result.forced, Forced::No);
    assert!(result.answered.is_some_and(|answer| answer >= result.start && answer <= result.end));
    assert_eq!(result.counts.method, Method::Walk);
    assert!(result.tree.live_pids.is_empty());
}

#[test]
fn an_answer_with_a_lingering_grandchild_keeps_its_forced_teardown() {
    let clock = Clock::new();
    // The leader stays long enough for the generic keeper to see the child.
    let result = execute(
        &clock,
        replay("(trap '' TERM; sleep 30) & sleep 0.04; exit 0"),
        Box::from(&b"answer\n"[..]),
        Capture::Codex,
        bounds(),
    )
    .expect("settled replay tree");
    assert_eq!(result.observer.classify(result.exit, result.forced).end, End::Completed);
    assert_eq!(result.forced, Forced::Killed);
    assert!(!result.observer.classify(result.exit, result.forced).warnings.is_empty());
    assert!(result.tree.live_at_leader_exit.as_ref().is_some_and(|pids| !pids.is_empty()));
    assert!(result.tree.live_pids.is_empty());
    assert!(!result.tree.killed_pids.is_empty());
}

#[test]
fn a_program_without_an_answer_ends_at_the_hard_deadline() {
    let clock = Clock::new();
    let mut command = replay("");
    command.arguments = vec!["-c".into(), "cat >/dev/null; trap '' TERM; sleep 30".into()];
    let result = execute(&clock, command, Box::from(&b"answer\n"[..]), Capture::Codex, bounds())
        .expect("settled timed-out tree");
    assert_eq!(result.observer.classify(result.exit, result.forced).end, End::Timeout);
    assert_eq!(result.forced, Forced::Killed);
    assert!(result.answered.is_none());
    assert!(result.tree.live_pids.is_empty());
}
