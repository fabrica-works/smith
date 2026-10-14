//! Application deadlines and live parsing on the shared process-tree supervisor.
use std::path::PathBuf;

use skein_lib::Duration;
use skein_shell::Clock;
use skein_world::end_to_end::{Command, Method};
use smith_bench::process::{Bounds, Capture, execute};
use smith_bench::{End, Forced};

fn isolated_replay(name: &str) -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("SMITH_REPOSITORY_REPLAY_ROOT") {
        return Some(PathBuf::from(root));
    }
    let root = std::env::temp_dir().join(format!("smith-bench-{name}-{}", std::process::id()));
    std::fs::create_dir(&root).expect("fixture root");
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", name, "--nocapture"])
        .env("SMITH_REPOSITORY_REPLAY_ROOT", &root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_STATE_HOME", root.join("state"))
        .output()
        .expect("private replay process");
    std::fs::remove_dir_all(root).expect("remove private replay");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    None
}

fn repository_request(root: &std::path::Path, guard: &smith_bench::guard::Guard) -> smith_bench::run::Prepared {
    use sha2::{Digest, Sha256};
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded");
    let state = root.join("state/smith-bench");
    std::fs::create_dir_all(state.join("bundles")).expect("bundles parent");
    smith_bench::isolate::copy_directory(guard, &fixtures.join("bundles/tiny"), &state.join("bundles/tiny"))
        .expect("private recorded bundle");
    let task_root = root.join("task");
    smith_bench::isolate::copy_directory(guard, &fixtures.join("tasks/repository-seed"), &task_root).expect("task");
    let manifest = task_root.join("task.toml");
    let text = std::fs::read_to_string(&manifest)
        .expect("manifest")
        .replace("deciding = [\"tree-ended\"]", "deciding = ['end', 'grade', 'protected-unchanged', 'tree-ended']")
        .replace("[[grade]]", "protected = ['README', 'Cargo.toml']\n\n[[grade]]");
    std::fs::write(&manifest, text).expect("protected paths");
    let task = smith_bench::read_task(&manifest).expect("task");
    let mut identity = smith_bench::read_result(&fixtures.join("results/attempt.json")).expect("identity").identity;
    identity.agent = smith_bench::Agent::Codex;
    identity.task = "repository/repository-seed".into();
    identity.task_version = task.version;
    identity.seed_sha256 = task.seed_sha256;
    identity.prompt_sha256 = format!("{:x}", Sha256::digest(task.prompt.as_bytes()));
    let program = std::fs::canonicalize("/bin/sh").expect("shell executable");
    identity.binary_sha256 = smith_bench::arm::binary_digest(&program).expect("replay digest");
    identity.run = "repository-replay".into();
    smith_bench::run::Prepared {
        task: manifest,
        identity,
        program,
        arguments: Vec::new(),
        codex_home: None,
        configuration: None,
        teardown_ms: 100,
        grace_ms: 20,
        settlement_ms: 200,
        setup_deadline_seconds: 3,
        max_stdout_bytes: 16384,
        max_stderr_bytes: 4096,
        disk_estimate_bytes: 1,
        force_walk: true,
    }
}

#[test]
fn a_repository_attempt_compares_protected_files_with_its_bundle() {
    let Some(root) = isolated_replay("a_repository_attempt_compares_protected_files_with_its_bundle") else { return };
    let guard = smith_bench::guard::Guard::new(&root.join("home"), &root.join("config"), &root.join("state"), &[])
        .expect("guard");
    let machine = smith_bench::isolate::Machine { path: "/usr/bin:/bin".into(), rustup_home: None };
    let mut prepared = repository_request(&root, &guard);
    for (name, tail, expected) in [
        ("unchanged", "", Vec::<PathBuf>::new()),
        ("changed", "printf changed > Cargo.toml;", vec![PathBuf::from("Cargo.toml")]),
    ] {
        prepared.identity.attempt = name.into();
        let mut command = replay(tail);
        command.arguments[1] = format!("cat >/dev/null; {tail} cat \"$1\"").into();
        prepared.arguments = command.arguments.iter().map(|value| value.to_str().expect("argument").into()).collect();
        let recorded = smith_bench::run::attempt(&guard, &machine, &root.join("state/smith-bench"), &prepared)
            .expect("repository replay");
        assert_eq!(recorded.result.outcome.end, End::Completed, "{name}");
        assert_eq!(
            recorded.result.outcome.protected_files_changed,
            smith_bench::Measure::Observed { value: expected },
            "{name}"
        );
    }
}

#[test]
fn a_file_left_in_tmp_by_an_attempt_is_a_write_outside() {
    let Some(root) = isolated_replay("a_file_left_in_tmp_by_an_attempt_is_a_write_outside") else { return };
    let guard = smith_bench::guard::Guard::new(&root.join("home"), &root.join("config"), &root.join("state"), &[])
        .expect("guard");
    let machine = smith_bench::isolate::Machine { path: "/usr/bin:/bin".into(), rustup_home: None };
    let mut prepared = repository_request(&root, &guard);
    let path = PathBuf::from(format!("/tmp/smith-bench-test-{}", std::process::id()));
    assert!(!path.exists(), "fresh test file");
    let mut command = replay("");
    command.arguments[1] = format!("cat >/dev/null; printf escaped > {}; cat \"$1\"", path.display()).into();
    prepared.arguments = command.arguments.iter().map(|value| value.to_str().expect("argument").into()).collect();
    let recorded = smith_bench::run::attempt(&guard, &machine, &root.join("state/smith-bench"), &prepared)
        .expect("repository replay");
    let check =
        recorded.result.outcome.checks.iter().find(|check| check.check == "nothing-outside").expect("scope check");
    assert_eq!(check.passed, smith_bench::Measure::Observed { value: false });
    assert!(check.evidence.contains(&path.display().to_string()), "{:?}", check.evidence);
    std::fs::remove_file(path).expect("remove test's temporary file");
}

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
