//! Offline controls for the pair's deciding evidence and process overlap.
use super::*;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

#[test]
fn a_refresh_pair_requires_two_completions_with_a_new_generation() {
    let mut pair = [End::Completed, End::Completed];
    assert_eq!(
        proof(&pair, &Ok(Some(4)), &Ok(Some(5)), Path::new("synthetic-record")),
        Measure::Observed { value: true }
    );
    assert_eq!(
        proof(&pair, &Ok(Some(4)), &Ok(Some(4)), Path::new("synthetic-record")),
        Measure::Observed { value: false }
    );
    pair[1] = End::Refused { setup: "refresh reused".into() };
    assert_eq!(
        proof(&pair, &Ok(Some(4)), &Ok(Some(5)), Path::new("synthetic-record")),
        Measure::Observed { value: false }
    );
}

#[test]
fn an_unreadable_generation_never_passes_the_pair() {
    assert!(matches!(
        proof(
            &[End::Completed, End::Completed],
            &Err("cannot inspect generation".into()),
            &Ok(Some(5)),
            Path::new("synthetic-record")
        ),
        Measure::Unavailable { .. }
    ));
    for (before, after) in [(Ok(None), Ok(Some(5))), (Ok(Some(4)), Ok(None))] {
        assert_eq!(
            proof(&[End::Completed, End::Completed], &before, &after, Path::new("synthetic-record")),
            Measure::unavailable("missing harness sign-in record: synthetic-record"),
        );
    }
}

fn isolated(name: &str) -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("SMITH_BENCH_PAIR_ROOT") {
        return Some(root.into());
    }
    let root = std::env::temp_dir().join(format!("smith-bench-pair-{}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&root).expect("private synthetic root");
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", name, "--nocapture"])
        .env_clear()
        .env("SMITH_BENCH_PAIR_ROOT", &root)
        .env("HOME", root.join("home"))
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_STATE_HOME", root.join("state"))
        .output()
        .expect("isolated pair control");
    std::fs::remove_dir_all(root).expect("synthetic cleanup");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    None
}

fn record(generation: u64) -> Box<[u8]> {
    let record = skein_oauth::SavedToken {
        key: 0,
        generation,
        access_token: b"unissued-access".as_slice().into(),
        refresh_token: Some(b"unissued-refresh".as_slice().into()),
        expires_at: skein_lib::Wall::from_nanos(1_000_000_000_000),
        issued_client_id: None,
        granted_scopes: None,
        subject: None,
        id_token: None,
        earliest_refresh_at: None,
        metadata: None,
    };
    skein_oauth::encode_record(&record, &smith_local_shell::local_host::token_limits()).expect("synthetic token")
}

#[test]
fn two_attempt_processes_meet_before_either_can_finish() {
    if let Some(request) = std::env::var_os("SMITH_BENCH_PAIR_REQUEST") {
        fake_attempt(Path::new(&request));
        return;
    }
    let Some(root) = isolated("batch::credentials::tests::two_attempt_processes_meet_before_either_can_finish") else {
        return;
    };
    let state = root.join("state/smith-bench");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(state.join("logins/smith/tokens"))
        .expect("synthetic login");
    let token = state.join("logins/smith/tokens/chatgpt");
    assert_eq!(record_generation(&token).expect("missing synthetic record"), None);
    std::fs::write(&token, record(1)).expect("synthetic record");
    std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).expect("private record");
    std::fs::write(state.join("rotated.token"), record(2)).expect("rotated synthetic record");
    std::fs::set_permissions(state.join("rotated.token"), std::fs::Permissions::from_mode(0o600))
        .expect("private rotated record");
    let fifo = state.join("rendezvous");
    let status = std::process::Command::new("/usr/bin/mkfifo")
        .arg(&fifo)
        .env_clear()
        .env("HOME", root.join("home"))
        .status()
        .expect("control rendezvous");
    assert!(status.success());
    let guard = Guard::new(&root.join("home"), &root.join("config"), &root.join("state"), &[]).expect("guard");
    let machine = Machine { path: "/usr/bin:/bin".into(), rustup_home: None };
    let mut result =
        crate::read_result(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json"))
            .expect("template");
    result.identity.run = "credentials-control".into();
    result.identity.attempt = "01".into();
    result.identity.task = "probes/credentials".into();
    result.identity.agent = crate::Agent::Smith;
    result.identity.provider = crate::Provider::Codex;
    result.outcome.deciding = vec!["credential-refresh".into()];
    result.outcome.checks = vec![crate::CheckVerdict {
        check: "credential-refresh".into(),
        passed: Measure::unavailable("pair pending"),
        evidence: vec![],
    }];
    std::fs::write(state.join("template.json"), crate::render_result(&result).expect("template"))
        .expect("template file");
    let request = Prepared {
        task: Path::new(env!("CARGO_MANIFEST_DIR")).join("tasks/probes/credentials/task.toml"),
        identity: result.identity,
        program: "/nonexecuted".into(),
        arguments: vec![],
        codex_home: Some(state.join("logins/codex")),
        configuration: None,
        teardown_ms: 100,
        grace_ms: 100,
        settlement_ms: 1000,
        setup_deadline_seconds: 5,
        max_stdout_bytes: 4096,
        max_stderr_bytes: 4096,
        disk_estimate_bytes: 0,
        force_walk: true,
    };
    let program = state.join("peer");
    std::os::unix::fs::symlink(std::env::current_exe().expect("control executable"), state.join("peer.test"))
        .expect("control executable alias");
    std::fs::write(&program, PEER).expect("fake attempt executable");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).expect("executable");
    let pair = run(&guard, &machine, &state, &program, &request, 5_000_000_000).expect("two settled processes");
    assert_eq!(pair.len(), 2);
    assert!(pair.iter().all(|result| result.outcome.checks[0].passed == Measure::Observed { value: true }));
    assert_eq!(record_generation(&token).expect("shared record generation"), Some(2));
    assert_ne!(pair[0].identity.attempt, pair[1].identity.attempt);
}

fn fake_attempt(file: &Path) {
    use std::io::{Read, Write};
    let request = crate::run::read_prepared(file).expect("synthetic request");
    let state = file.parent().and_then(Path::parent).expect("synthetic state");
    let peer = request.identity.attempt.ends_with("-peer");
    let mut signal =
        std::fs::OpenOptions::new().read(!peer).write(peer).open(state.join("rendezvous")).expect("FIFO rendezvous");
    if peer {
        signal.write_all(b"x").expect("peer reached the rendezvous");
        std::fs::copy(state.join("rotated.token"), state.join("logins/smith/tokens/chatgpt")).expect("fake refresh");
    } else {
        signal.read_exact(&mut [0]).expect("peer reached the rendezvous");
    }
    let mut result = crate::read_result(&state.join("template.json")).expect("synthetic result");
    result.identity = request.identity;
    let directory = state.join("runs").join(&result.identity.run).join(&result.identity.attempt);
    std::fs::create_dir_all(&directory).expect("synthetic result directory");
    std::fs::write(directory.join("result.json"), crate::render_result(&result).expect("synthetic result"))
        .expect("result file");
}

// Each FIFO open waits for the other process. A serial dispatcher cannot finish.
const PEER: &str = r#"#!/bin/sh
SMITH_BENCH_PAIR_REQUEST="$2" exec "$0.test" --exact batch::credentials::tests::two_attempt_processes_meet_before_either_can_finish --nocapture
"#;
