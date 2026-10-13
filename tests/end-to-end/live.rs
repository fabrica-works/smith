//! Opt-in shipped-binary stories against real backends (testing.md, section 2.3).
//! The `live` nextest profile selects this binary; the merge gate excludes it.

#[path = "support/live.rs"]
pub mod live;
pub mod support;

#[test]
fn bootstrap_settings() {
    if !live::enabled() {
        return;
    }
    let Some(directory) = live::setting("SMITH_TEST_LIVE_BOOTSTRAP_DIR") else {
        return;
    };
    let directory =
        smith_bench::guard::write_path(std::path::Path::new(&directory)).expect("guarded bootstrap directory");
    assert!(directory.is_absolute(), "SMITH_TEST_LIVE_BOOTSTRAP_DIR must be absolute");
    let environment = live::Environment::load(true);
    std::fs::create_dir_all(&directory).expect("bootstrap settings directory");
    for backend in environment.backends {
        let state = smith_bench::guard::write_path(&directory.join(&backend.name)).expect("guarded bootstrap state");
        std::fs::create_dir_all(&state).expect("bootstrap state directory");
        let config = state.join("config/smith");
        std::fs::create_dir_all(&config).expect("bootstrap user layer");
        let settings: toml::Value = serde_json::from_value(backend.settings(&state, false)).expect("public settings");
        std::fs::write(
            smith_bench::guard::write_path(&config.join("config.toml")).expect("guarded bootstrap settings"),
            toml::to_string(&settings).expect("bootstrap TOML"),
        )
        .expect("bootstrap settings");
    }
}

use smith_local_process_world::referee::CheckoutRead;
use smith_real_world::Scratch;
use support::live_run::Expectation;

const CHANGE: &str = "In repository repo, read original.txt, replace its content with exactly edited followed by one newline, create result.txt containing exactly new followed by one newline, and use the command tool to create command.txt containing exactly ran followed by one newline. Run the configured .smith-test/check checks without modifying that script. Finish with a change result, a short title and body, so the host delivers the checked commit. Do not run git commit or push yourself.";

/// The spike's configured-variable story remains opt-in and uses the shipped binary.
#[test]
fn a_command_reads_the_whole_configured_environment() {
    if !live::enabled() {
        return;
    }
    for backend in live::Environment::load(false).backends {
        let scratch = Scratch::new();
        let mut settings = backend.settings(scratch.path(), false);
        settings["environment"]["set"]["SMITH_TEST_ENVIRONMENT"] = serde_json::json!("configured");
        backend.write_product(&scratch, &settings);
        run(
            &backend,
            &scratch,
            "Use shell to run `printf '%s\\n' \"$SMITH_TEST_ENVIRONMENT\" > configured.txt`. Do not use a file-writing tool or substitute a literal value. Finish with a report.",
            false,
            Expectation { before: None, files: vec![(b"configured.txt".to_vec(), b"configured\n".to_vec())] },
        );
    }
}

fn changed_files() -> Vec<(Vec<u8>, Vec<u8>)> {
    [("original.txt", "edited\n"), ("result.txt", "new\n"), ("command.txt", "ran\n"), ("checks-ran.txt", "checked\n")]
        .into_iter()
        .map(|(path, content)| (path.as_bytes().to_vec(), content.as_bytes().to_vec()))
        .collect()
}

#[expect(
    clippy::disallowed_types,
    reason = "10-product replaces the live command stories and their independent command checks"
)]
fn change(backend: &live::Backend, scratch: &Scratch, prompt: &str, extra: Option<(Vec<u8>, Vec<u8>)>) {
    let mut files = changed_files();
    if let Some(extra) = extra {
        files.push(extra);
    }
    run(backend, scratch, prompt, true, Expectation { before: scratch.checkout().head(), files });
    assert!(
        std::process::Command::new("/bin/sh")
            .arg(".smith-test/check")
            .current_dir(scratch.path().join("repo"))
            .status()
            .expect("independent check execution")
            .success(),
        "prescribed check passes independently"
    );
    if backend.borrowed.is_none() {
        let tokens = read_record(backend);
        let trace = std::fs::read(scratch.path().join("agent-trace.jsonl")).expect("outside trace");
        for secret in [Some(tokens.access_token.as_ref()), tokens.refresh_token.as_deref()].into_iter().flatten() {
            assert!(
                !secret.is_empty() && !trace.windows(secret.len()).any(|bytes| bytes == secret),
                "trace contains no credential value"
            );
        }
    }
}

#[test]
fn a_run_refreshes_its_grant_at_the_issuer() {
    if !live::enabled() {
        return;
    }
    let Some(environment) = live::Environment::issuer() else {
        return;
    };
    for backend in environment.backends {
        eprintln!("live refresh provider={}", backend.name);
        let mut saved = read_record(&backend);
        assert!(
            saved.refresh_token.as_ref().is_some_and(|token| !token.is_empty()),
            "dedicated test grant must support refresh"
        );
        let generation = saved.generation;
        saved.expires_at = skein_lib::Wall::EPOCH;
        std::fs::write(
            smith_bench::guard::write_path(&live::record_path(&backend.tokens, &backend.name))
                .expect("guarded test record"),
            skein_oauth::encode_record(&saved, &smith_local_shell::local_host::token_limits())
                .expect("dedicated expired record"),
        )
        .expect("force only the test-owned grant to refresh");
        let scratch = Scratch::new();
        backend.configure(&scratch, false, None);
        run(
            &backend,
            &scratch,
            "Give a brief greeting and call finish with a report result.",
            false,
            Expectation { before: None, files: vec![] },
        );
        let refreshed = read_record(&backend);
        assert!(refreshed.generation > generation, "real issuer exchange advanced durable token generation");
        assert!(
            refreshed.expires_at > skein_shell::Clock::new().now().wall && !refreshed.access_token.is_empty(),
            "the refreshed grant is usable and persists for the next run"
        );
    }
}

#[test]
fn a_chat_ends_with_a_commit_in_place() {
    if !live::enabled() {
        return;
    }
    for backend in live::Environment::load(false).backends {
        eprintln!("live commit provider={}", backend.name);
        let scratch = Scratch::new();
        backend.configure(&scratch, true, None);
        change(&backend, &scratch, CHANGE, None);
    }
}

fn turns(backend: &live::Backend, scratch: &Scratch) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let directory = {
        let mut workspaces = std::fs::read_dir(if backend.borrowed.is_some() {
            scratch.path().join("state/smith/chats")
        } else {
            backend.tokens.join("smith/chats")
        })
        .expect("workspace store");
        let directory = workspaces.next().expect("one workspace").expect("workspace path").path().join("chat");
        assert!(workspaces.next().is_none(), "only the story's workspace");
        directory
    };
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .expect("durable chat files")
        .map(|entry| entry.expect("turn directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "turn"))
        .map(|path| {
            let bytes = std::fs::read(&path).expect("durable turn");
            (path, bytes)
        })
        .collect();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

#[test]
fn a_second_run_resumes_the_chat_from_its_files() {
    if !live::enabled() {
        return;
    }
    for backend in live::Environment::load(false).backends {
        eprintln!("live durable resume provider={}", backend.name);
        let scratch = Scratch::new();
        let marker =
            format!("smith-live-memory-{}-{}", std::process::id(), skein_shell::Clock::new().now().now.as_nanos());
        backend.configure(&scratch, false, None);
        run(
            &backend,
            &scratch,
            &format!(
                "Remember this code for our next invocation: {marker}. Do not write files. Call finish with a short report result."
            ),
            false,
            Expectation { before: None, files: vec![] },
        );
        let before = turns(&backend, &scratch);
        assert!(!before.is_empty(), "first binary wrote durable turns");
        assert!(
            !scratch.path().join("repo/remembered.txt").exists(),
            "the first invocation keeps the code only in conversation history"
        );
        backend.configure(&scratch, true, None);
        let prompt = format!(
            "{CHANGE} Also create remembered.txt containing exactly the code I asked you to remember in the previous invocation, with no newline. Obtain it from the saved conversation, not from an invented value."
        );
        change(&backend, &scratch, &prompt, Some((b"remembered.txt".to_vec(), marker.into_bytes())));
        let after = turns(&backend, &scratch);
        assert!(after.len() > before.len(), "second binary appended durable history");
        assert!(before.iter().all(|entry| after.contains(entry)), "earlier durable turns remain unchanged");
    }
}

#[expect(clippy::disallowed_types, reason = "10-product replaces the live command stories and their remote checks")]
fn git(path: &std::path::Path, arguments: &[&str]) -> std::process::Output {
    std::process::Command::new("/usr/bin/git").arg("-C").arg(path).args(arguments).output().expect("live git command")
}

#[test]
fn a_configured_push_lands() {
    if !live::enabled() {
        return;
    }
    let environment = live::Environment::load(false);
    let Some(remote) = environment.remote else {
        eprintln!("configured push not run: SMITH_TEST_LIVE_GIT_REMOTE is absent");
        return;
    };
    for backend in environment.backends {
        let scratch = Scratch::new();
        let repo = scratch.path().join("repo");
        let branch = format!(
            "smith-live-{}-{}-{}",
            backend.name,
            std::process::id(),
            skein_shell::Clock::new().now().now.as_nanos()
        );
        assert!(git(&repo, &["checkout", "-q", "-b", &branch]).status.success(), "create unique live push branch");
        assert!(
            git(&repo, &["remote", "add", if backend.borrowed.is_some() { "origin" } else { "live" }, &remote])
                .status
                .success(),
            "configure caller-selected test remote"
        );
        backend.configure(&scratch, true, Some(("live", &branch)));
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            change(&backend, &scratch, CHANGE, None);
            let reference = format!("refs/heads/{branch}");
            let seen = git(&repo, &["ls-remote", "live", &reference]);
            assert!(seen.status.success(), "inspect configured remote branch; diagnostics withheld");
            let head = scratch.checkout().head().expect("delivered head");
            assert!(
                seen.stdout.split(u8::is_ascii_whitespace).next() == Some(head.as_slice()),
                "configured push lands at the delivered commit"
            );
        }));
        let reference = format!(":refs/heads/{branch}");
        let cleanup = git(&repo, &["push", "live", &reference]);
        assert!(cleanup.status.success(), "remove live test branch even when its story fails; diagnostics withheld");
        if let Err(failure) = outcome {
            std::panic::resume_unwind(failure);
        }
    }
}

fn run(backend: &live::Backend, scratch: &Scratch, prompt: &str, change: bool, expectation: Expectation) {
    run_exec(backend, scratch, prompt, change, expectation);
}

fn run_exec(backend: &live::Backend, scratch: &Scratch, prompt: &str, change: bool, expectation: Expectation) {
    use skein_lib::Duration;
    use skein_world::end_to_end::Command;
    use smith_bench::process::{Bounds, Capture, execute};
    let clock = skein_shell::Clock::new();
    let environment = vec![
        ("HOME".into(), scratch.path().into()),
        ("XDG_CONFIG_HOME".into(), scratch.path().join("config").into()),
        (
            "XDG_STATE_HOME".into(),
            if backend.borrowed.is_some() { scratch.path().join("state") } else { backend.tokens.clone() }.into(),
        ),
        ("PATH".into(), live::setting("PATH").expect("live PATH").into()),
    ];
    let workspace = scratch.path().join("repo");
    let command = |arguments| Command {
        program: env!("CARGO_BIN_EXE_smith").into(),
        arguments,
        environment: environment.clone(),
        directory: scratch.path().into(),
    };
    let bounds = |hard, teardown| Bounds {
        hard,
        teardown,
        grace: Duration::from_secs(1),
        settlement: Duration::from_secs(5),
        stdout: 20_000_000,
        stderr: 65_536,
        force_walk: true,
    };
    let mut arguments =
        vec!["exec".into(), "--json".into(), "-C".into(), workspace.clone().into(), "--chat".into(), "chat".into()];
    if change {
        arguments.push("--deliver".into());
    }
    arguments.push("-".into());
    let mut check_arguments = vec!["check".into(), "--json".into()];
    check_arguments.extend(arguments.clone());
    let check = execute(
        &clock,
        command(check_arguments),
        Box::default(),
        Capture::Raw,
        bounds(Duration::from_secs(15), Duration::ZERO),
    )
    .expect("live check settles");
    assert_eq!(check.exit, smith_bench::Exit::Code(0), "live check must pass; credential values withheld");
    let teardown = smith_bench::agent::smith::teardown_from_check(&check.stdout).expect("effective bound");
    let borrowed = backend.borrowed.as_ref().map(|file| {
        smith_bench::credential::smith_borrowed(
            file,
            clock.now().wall,
            Duration::from_secs(140).checked_add(teardown).expect("live credential lifetime"),
        )
        .expect("caller credential outlives story")
    });
    let registry = smith_bench::agent::smith::FaceSince {
        commit: String::new(),
        shape: smith_bench::agent::smith::events::Shape::Exec,
        receiving: smith_bench::agent::smith::Receiving {
            string: 256_000,
            content: 13_056_000,
            items: 52_778,
            encoded_line: 19_817_472,
        },
        session_ended_aborted: true,
    };
    let prepared = smith_bench::agent::smith::Prepared {
        face: registry.shape,
        registry,
        settings: scratch.path().join("config/smith/config.toml"),
        credential: smith_bench::agent::smith::Credential::Record(None),
        trace: None,
        rendered: serde_json::Value::Null,
        program: env!("CARGO_BIN_EXE_smith").into(),
        state: scratch.path().join("state"),
        teardown,
    };
    let mut result = execute(
        &clock,
        command(arguments),
        format!("{prompt}\n").into_bytes().into(),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(
            smith_bench::agent::smith::Observing::new(prepared).expect("outside observer"),
        ))),
        bounds(Duration::from_secs(130), teardown),
    )
    .expect("shipped live exec settles");
    assert_exec(&mut result, scratch, expectation);
    if let Some(borrowed) = borrowed {
        assert!(
            borrowed
                .finish(&smith_bench::guard::Guard::from_environment().expect("credential guard"))
                .expect("source file unchanged")
                .source_unchanged,
            "smith leaves its borrowed login file unchanged"
        );
    }
}

fn assert_exec(result: &mut smith_bench::process::ProcessResult, scratch: &Scratch, expectation: Expectation) {
    assert!(
        result.error.is_none() && result.forced == smith_bench::Forced::No && result.tree.live_pids.is_empty(),
        "live exec and its tree must settle; credential values withheld"
    );
    let Some(smith_bench::agent::Observing::Smith(observing)) = result.observing.as_deref_mut() else {
        panic!("exec observer")
    };
    observing.observer.finish().expect("complete event stream");
    assert!(
        matches!(observing.observer.classify(result.exit, result.forced).end, smith_bench::End::Completed),
        "typed answer and process exit agree"
    );
    let checkout = scratch.checkout();
    if let Some(before) = expectation.before {
        let head = checkout.head().expect("delivered commit");
        assert_ne!(head, before, "exec delivered a new commit");
        let files = checkout.files(&head);
        for (path, content) in expectation.files {
            assert_eq!(files.get(&path), Some(&content), "committed file matches the story");
        }
    } else {
        for (path, content) in expectation.files {
            assert_eq!(
                std::fs::read(scratch.path().join("repo").join(std::str::from_utf8(&path).expect("path")))
                    .expect("requested file"),
                content,
                "requested file matches the story"
            );
        }
    }
}

#[test]
fn a_borrowed_login_lends_a_grant_and_keeps_its_file() {
    if !live::enabled() {
        return;
    }
    for backend in live::Environment::load(false).backends {
        if backend.borrowed.is_none() {
            continue;
        }
        let scratch = Scratch::new();
        backend.configure(&scratch, false, None);
        run(
            &backend,
            &scratch,
            "Give a short greeting and finish with a report.",
            false,
            Expectation { before: None, files: vec![] },
        );
    }
}

fn read_record(backend: &live::Backend) -> skein_oauth::SavedToken {
    let bytes = skein_shell::read_private_file(
        &live::record_path(&backend.tokens, &backend.name),
        smith_local_shell::local_host::token_limits().record_bytes,
        skein_shell::effective_user(),
    )
    .expect("dedicated private record");
    skein_oauth::decode_record(bytes.expose(), &smith_local_shell::local_host::token_limits())
        .expect("native private record")
}
