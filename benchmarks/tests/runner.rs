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
fn a_prepared_adapter_observes_the_answer_through_the_common_supervisor() {
    let prepared = smith_bench::agent::Prepared::Codex(smith_bench::agent::codex::Prepared {
        overrides: Vec::new(),
        credential: "/tmp/smith-codex-capture-no-login/auth.json".into(),
        expires: skein_lib::Wall::EPOCH,
        borrowed: None,
        home: "/tmp/smith-codex-capture-no-login".into(),
        final_message: "/tmp/smith-codex-capture-final.txt".into(),
        model: smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) },
        program: "/bin/sh".into(),
        workspace: "/tmp".into(),
        teardown: bounds().teardown,
    });
    let result = execute(
        &Clock::new(),
        replay("exit 0"),
        Box::from(&b"offline replay\n"[..]),
        Capture::Agent(Box::new(prepared.observe().expect("prepared replay observer"))),
        bounds(),
    )
    .expect("paired replay settles");
    assert!(result.error.is_none(), "{:?}", result.error);
    let observing = result.observing.expect("prepared observer retained");
    assert_eq!(observing.classify(result.exit, result.forced).end, End::Completed);
    assert!(result.answered.is_some_and(|answer| answer >= result.start && answer <= result.end));
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

#[test]
fn an_overflowing_stderr_retains_its_prefix_and_settles_the_tree() {
    let clock = Clock::new();
    let mut command = replay("");
    command.arguments = vec!["-c".into(), "cat >/dev/null; trap '' TERM; yes error >&2".into()];
    let result = execute(&clock, command, Box::from(&b"answer\n"[..]), Capture::Codex, bounds())
        .expect("overflowing tree settles");
    assert_eq!(result.error.as_deref(), Some("stderr exceeds the attempt log bound"));
    assert_eq!(result.stderr.len(), bounds().stderr);
    assert_eq!(result.forced, Forced::Killed);
    assert!(result.answered.is_none());
    assert!(result.tree.live_pids.is_empty());
    assert!(result.end.as_nanos() - result.start.as_nanos() < 180_000_000);
}

#[test]
fn an_overflowing_stdout_retains_its_prefix_and_settles_the_tree() {
    let clock = Clock::new();
    let mut command = replay("");
    command.arguments = vec!["-c".into(), "cat >/dev/null; trap '' TERM; yes output".into()];
    let result = execute(&clock, command, Box::new([]), Capture::Raw, bounds()).expect("overflowing tree settles");
    assert_eq!(result.error.as_deref(), Some("stdout exceeds the attempt log bound"));
    assert_eq!(result.stdout.len(), bounds().stdout);
    assert_eq!(result.forced, Forced::Killed);
    assert!(result.tree.live_pids.is_empty());
}

#[test]
fn stderr_at_its_bound_is_complete_and_a_settled_overflow_still_fails() {
    let clock = Clock::new();
    for (text, error) in [("12345678", None), ("123456789", Some("stderr exceeds the attempt log bound"))] {
        let mut command = replay("");
        command.arguments = vec!["-c".into(), format!("cat >/dev/null; printf {text} >&2").into()];
        let mut policy = bounds();
        policy.stderr = 8;
        let result = execute(&clock, command, Box::new([]), Capture::Raw, policy).expect("short stderr settles");
        assert_eq!(result.error.as_deref(), error);
        assert_eq!(result.stderr, b"12345678");
        assert!(result.tree.live_pids.is_empty());
    }
}

#[test]
fn an_answer_near_the_hard_deadline_receives_its_full_teardown_allowance() {
    let clock = Clock::new();
    let mut command = replay("sleep 0.12; exit 0");
    command.arguments[1] =
        command.arguments[1].to_string_lossy().replace("cat \"$1\"", "sleep 0.08; cat \"$1\"").into();
    let mut policy = bounds();
    policy.hard = Duration::from_millis(160);
    policy.teardown = Duration::from_millis(180);
    let result = execute(&clock, command, Box::new([]), Capture::Codex, policy).expect("accepted answer settles");
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.forced, Forced::No);
    assert_eq!(result.observer.classify(result.exit, result.forced).end, End::Completed);
    assert!(result.answered.is_some());
}
