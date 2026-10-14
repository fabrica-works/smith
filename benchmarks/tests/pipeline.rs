//! Preparation, grading and archival around a fake prepared agent invocation.
use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use smith_bench::guard::Guard;
use smith_bench::isolate::Machine;
use smith_bench::run::{Prepared, attempt, scratch_root};
use smith_bench::{End, Measure};

fn root(story: &str) -> PathBuf {
    std::env::temp_dir().join(format!("smith-bench-pipeline-{story}-{}", std::process::id()))
}

fn fixture(story: &str) -> (Guard, Machine, PathBuf, Prepared) {
    use std::os::unix::fs::DirBuilderExt;
    let root = root(story);
    fs::DirBuilder::new().mode(0o700).create(&root).expect("fresh story root");
    let task = root.join("task");
    fs::create_dir(&task).expect("task");
    fs::create_dir(task.join("seed")).expect("seed");
    fs::write(task.join("seed/protected.txt"), "frozen\n").expect("protected seed");
    let digest = smith_bench::seed_digest(&task.join("seed")).expect("digest");
    let prompt = "Complete the replay task.\n";
    fs::write(task.join("task.toml"), format!("id='recorded'\nversion=1\nkind='probe'\ntier='small'\ndeadline_seconds=1\nprompt={prompt:?}\nseed_sha256='{digest}'\ndeciding=['end','grade','file-content:answer.txt','protected-unchanged','tree-ended','nothing-outside','exit-code','answer','last-line']\nprotected=['protected.txt']\nenvironment={{TASK_MARKER='explicit'}}\nsetup=[['sh','-c','test \"$TASK_MARKER\" = explicit && printf warmed > \"$CARGO_HOME/warm\" && printf target > \"$CARGO_TARGET_DIR/warm\" && printf setup > setup.txt && git add setup.txt && git -c user.name=setup -c user.email=setup@example.invalid -c gc.auto=0 -c maintenance.auto=false commit -qm setup']]\n[[outcome]]\ncheck='file-content'\npath='answer.txt'\ncontent=\"changed\\n\"\n[[grade]]\ncommand=['sh','-c','test -f answer.txt && test ! -e \"$HOME/agent-only\" && test \"$(cat \"$CARGO_HOME/warm\")\" = warmed && test \"$(cat \"$CARGO_TARGET_DIR/warm\")\" = target && printf grader-only > changed-by-grader']\ndeadline_seconds=1\nmax_output_bytes=4096\n")).expect("manifest");
    {
        use std::io::Write;
        fs::OpenOptions::new().append(true).open(task.join("task.toml")).expect("append neutral checks")
            .write_all(b"\n[[outcome]]\ncheck='end'\nend='completed'\n[[outcome]]\ncheck='exit-code'\ncode=0\n[[outcome]]\ncheck='answer'\npattern='bounded saturating arithmetic'\n[[outcome]]\ncheck='last-line'\npattern='^settled replay$'\n").expect("neutral task checks");
    }
    let mut identity = smith_bench::read_result(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json"),
    )
    .expect("template")
    .identity;
    identity.run = story.into();
    identity.attempt = "one".into();
    identity.task = "probes/recorded".into();
    identity.seed_sha256 = digest;
    identity.prompt_sha256 = format!("{:x}", Sha256::digest(prompt));
    identity.binary_sha256 = smith_bench::arm::binary_digest(&PathBuf::from("/bin/dash")).expect("shell digest");
    let recording = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/codex/completed.jsonl");
    let prepared = Prepared { task: task.join("task.toml"), identity, program: "/bin/dash".into(), arguments: vec!["-c".into(), "cat >/dev/null; test \"$(cat \"$CARGO_HOME/warm\")\" = warmed; test \"$(cat \"$CARGO_TARGET_DIR/warm\")\" = target; printf 'changed\\n' > answer.txt; git add answer.txt; git -c user.name=replay -c user.email=replay@example.invalid -c gc.auto=0 -c maintenance.auto=false commit -qm 'Agent commit'; printf untracked > new.txt; printf poisoned > \"$CARGO_HOME/warm\"; printf poisoned > \"$CARGO_TARGET_DIR/warm\"; printf marker > \"$HOME/agent-only\"; printf 'settled replay\\n' >&2; cat \"$1\"".into(), "replay".into(), recording.to_str().expect("recording UTF-8").into()], codex_home: None, configuration: None, teardown_ms: 100, grace_ms: 30, settlement_ms: 200, setup_deadline_seconds: 1, max_stdout_bytes: 16384, max_stderr_bytes: 4096, disk_estimate_bytes: 65536, force_walk: true };
    let guard =
        Guard::new(&root.join("user-home"), &root.join("user-config"), &root.join("user-state"), &[]).expect("guard");
    let machine = Machine { path: "/usr/bin:/bin".into(), rustup_home: None };
    (guard, machine, root, prepared)
}

#[test]
fn grading_and_archives_keep_agent_commits_and_untracked_files_outside_the_timer() {
    let (guard, machine, root, prepared) = fixture("archives");
    let request = root.join("prepared.json");
    fs::write(&request, serde_json::to_vec(&prepared).expect("prepared JSON")).expect("request");
    let clock = skein_shell::Clock::new();
    let process = smith_bench::process::execute(
        &clock,
        skein_world::end_to_end::Command {
            program: PathBuf::from(env!("CARGO_BIN_EXE_smith-bench")),
            arguments: vec!["attempt".into(), request.into_os_string()],
            environment: vec![
                ("HOME".into(), root.join("user-home").into_os_string()),
                ("PATH".into(), "/usr/bin:/bin".into()),
                ("XDG_STATE_HOME".into(), root.join("state").into_os_string()),
            ],
            directory: root.clone(),
        },
        Box::new([]),
        smith_bench::process::Capture::Raw,
        smith_bench::process::Bounds {
            hard: skein_lib::Duration::from_secs(2),
            teardown: skein_lib::Duration::ZERO,
            grace: skein_lib::Duration::from_millis(30),
            settlement: skein_lib::Duration::from_millis(200),
            stdout: 16384,
            stderr: 16384,
            force_walk: true,
        },
    )
    .expect("fresh attempt process");
    // The replay proves grading/archive behavior, but its root-only records
    // include a historical absolute FileChange outside this fresh workspace.
    assert_eq!(process.exit, smith_bench::Exit::Code(1), "{:?}", process.stderr);
    let directory = root.join("state/smith-bench/runs/archives/one");
    let result = smith_bench::read_result(&directory.join("result.json")).expect("attempt archive");
    let recorded = smith_bench::run::Recorded { result, directory, incomplete: smith_bench::run::Incomplete::No };
    assert_eq!(recorded.result.outcome.end, End::Completed);
    assert_eq!(recorded.incomplete, smith_bench::run::Incomplete::No);
    assert_eq!(recorded.result.outcome.grade, Some(Measure::Observed { value: true }));
    assert_eq!(recorded.result.outcome.protected_files_changed, Measure::Observed { value: Vec::new() });
    check_replay_judgments(&recorded.result);
    let diff = fs::read_to_string(recorded.directory.join("diff.patch")).expect("seed diff");
    assert!(diff.contains("answer.txt") && diff.contains("new.txt") && diff.contains("setup.txt"));
    assert!(!diff.contains("changed-by-grader"));
    for artifact in &recorded.result.artifacts {
        assert_eq!(
            smith_bench::arm::binary_digest(&recorded.directory.join(&artifact.path)).expect("artifact hash"),
            artifact.sha256
        );
    }
    assert_eq!(
        smith_bench::read_result(&recorded.directory.join("result.json")).expect("strict result"),
        recorded.result
    );
    assert!(!scratch_root(&root.join("state/smith-bench"), &prepared).expect("scratch").exists());
    // A rejecting grader is an ordinary grade failure; a crashed grader
    // preserves its bounded output and remains a harness fault.
    let private = smith_bench::isolate::empty(&guard, &root.join("crash-grader")).expect("private grader");
    let judgment = smith_bench::grade::command(
        &clock,
        &machine,
        &private,
        &private.workspace,
        &smith_bench::Grade {
            command: vec!["sh".into(), "-c".into(), "printf crash-evidence; kill -KILL $$".into()],
            deadline_seconds: 1,
            max_output_bytes: 4096,
        },
        smith_bench::process::Bounds {
            hard: skein_lib::Duration::ZERO,
            teardown: skein_lib::Duration::ZERO,
            grace: skein_lib::Duration::from_millis(30),
            settlement: skein_lib::Duration::from_millis(200),
            stdout: 4096,
            stderr: 4096,
            force_walk: true,
        },
        &std::collections::BTreeMap::new(),
    )
    .expect("settled crashed grader");
    assert!(matches!(judgment.end, smith_bench::grade::GradeEnd::Fault { .. }));
    assert_eq!(judgment.stdout, b"crash-evidence");
    fs::remove_dir_all(root).expect("remove synthetic story");
}

#[test]
fn login_shells_keep_machine_path_in_setup_agent_and_grading() {
    use std::os::unix::fs::PermissionsExt;

    let (guard, mut machine, root, mut prepared) = fixture("login-path");
    let tools = root.join("machine's tools $literal");
    fs::create_dir(&tools).expect("machine tool directory");
    let tool = tools.join("smith-login-path-fixture");
    fs::write(&tool, "#!/bin/sh\nexit 0\n").expect("unique machine tool");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable tool");
    machine.path =
        std::env::join_paths([tools.as_path(), std::path::Path::new("/usr/bin"), std::path::Path::new("/bin")])
            .expect("machine PATH");
    let check = "test \"$(command -v smith-login-path-fixture)\" = \"$TASK_TOOL\" && smith-login-path-fixture";
    let task = fs::read_to_string(&prepared.task).expect("task");
    let task = task
        .replace(
            "TASK_MARKER='explicit'",
            &format!("TASK_MARKER='explicit',TASK_TOOL={:?}", tool.to_str().expect("tool path")),
        )
        .replace("setup=[", &format!("setup=[['/bin/bash','-lc',{check:?}],"))
        .replace("command=['sh','-c',", "command=['/bin/bash','-lc',")
        .replace("test -f answer.txt &&", &format!("{check} && test -f answer.txt &&"));
    fs::write(&prepared.task, task).expect("login-shell setup and grading");
    prepared.program = "/bin/bash".into();
    prepared.identity.binary_sha256 = smith_bench::arm::binary_digest(&prepared.program).expect("bash digest");
    prepared.arguments[0] = "-lc".into();
    prepared.arguments[1] = format!("{check} || exit 17; {}", prepared.arguments[1]);
    let recorded = attempt(&guard, &machine, &root.join("state"), &prepared).expect("settled fake attempt");
    assert_eq!(recorded.result.outcome.end, End::Completed, "{:?}", recorded.result.outcome);
    assert_eq!(recorded.result.outcome.grade, Some(Measure::Observed { value: true }));
    assert!(!scratch_root(&root.join("state"), &prepared).expect("scratch").exists());
    fs::remove_dir_all(root).expect("remove synthetic story");
}

#[test]
fn an_existing_scratch_root_is_refused_and_never_deleted() {
    let (guard, machine, root, prepared) = fixture("existing");
    let scratch = scratch_root(&root.join("state"), &prepared).expect("scratch");
    fs::create_dir_all(&scratch).expect("existing directory");
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(scratch.parent().expect("parent"), fs::Permissions::from_mode(0o700))
            .expect("private parent");
    }
    fs::write(scratch.join("sentinel"), "keep").expect("sentinel");
    let recorded = attempt(&guard, &machine, &root.join("state"), &prepared).expect("record setup refusal");
    assert!(matches!(recorded.result.outcome.end, End::Refused { .. }));
    assert_eq!(recorded.incomplete, smith_bench::run::Incomplete::Setup(smith_bench::isolate::SetupStep::Scratch));
    assert_eq!(fs::read_to_string(scratch.join("sentinel")).expect("retained original"), "keep");
    fs::remove_dir_all(root).expect("remove synthetic story");
}

fn check_replay_judgments(result: &smith_bench::AttemptResult) {
    assert!(
        result
            .health
            .passive_checks
            .iter()
            .any(|check| check.check == "protected-unchanged" && check.passed == Measure::Observed { value: true })
    );
    assert!(
        result
            .health
            .passive_checks
            .iter()
            .any(|check| check.check == "benchmark-unread" && matches!(check.passed, Measure::Unavailable { .. }))
    );
    assert!(
        result
            .outcome
            .checks
            .iter()
            .filter(|check| !result.health.passive_checks.iter().any(|passive| passive.check == check.check))
            .all(|check| check.passed == Measure::Observed { value: true })
    );
    assert!(
        result
            .outcome
            .checks
            .iter()
            .any(|check| check.check == "nothing-outside" && check.passed == Measure::Observed { value: false })
    );
    assert!(
        result
            .health
            .passive_checks
            .iter()
            .any(|check| check.check == "nothing-outside" && check.passed == Measure::Observed { value: false })
    );
}
