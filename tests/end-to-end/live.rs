//! Shipped smith on its own live accounts (testing.md, section 2.3).
//! The live profile selects these stories serially, without retries.

#[path = "support/live.rs"]
pub mod live;
pub mod support;

use smith_events::{Block, Event};
use smith_local_process_world::referee::CheckoutRead;
use smith_real_world::Scratch;
use support::live_run::Expectation;

const CHANGE: &str = "In repository repo, read original.txt, replace its content with exactly edited followed by one newline, create result.txt containing exactly new followed by one newline, and use the command tool to create command.txt containing exactly ran followed by one newline. Run the configured .smith-test/check checks without modifying that script. Finish with a change result, a short title and body, so the host delivers the checked commit. Do not run git commit or push yourself.";

fn changed_files() -> Vec<(Vec<u8>, Vec<u8>)> {
    [("original.txt", "edited\n"), ("result.txt", "new\n"), ("command.txt", "ran\n"), ("checks-ran.txt", "checked\n")]
        .into_iter()
        .map(|(path, content)| (path.as_bytes().to_vec(), content.as_bytes().to_vec()))
        .collect()
}

#[test]
fn chatgpt_delivers_a_commit_in_place() {
    commit("chatgpt");
}

#[test]
fn a_run_refreshes_its_grant_at_the_issuer() {
    let mut environment = live::Environment::load(&["chatgpt"]);
    environment.refresh_lead = Some(34_560_000);
    let before = environment.generation();
    let scratch = Scratch::new();
    change(&environment, "chatgpt", &scratch, CHANGE, None, false);
    assert!(environment.generation() > before, "the issuer refresh was kept; generations withheld");
}

#[test]
fn claude_delivers_a_commit_in_place() {
    commit("claude");
}

#[test]
fn an_anthropic_key_delivers_a_commit_in_place() {
    commit("anthropic-key");
}

fn commit(account: &str) {
    let environment = live::Environment::load(&[account]);
    let scratch = Scratch::new();
    change(&environment, account, &scratch, CHANGE, None, false);
}

#[test]
fn a_command_reads_the_whole_configured_environment() {
    let environment = live::Environment::load(&["chatgpt"]);
    let scratch = Scratch::new();
    run(
        &environment,
        "chatgpt",
        &scratch,
        "Use shell to run `printf '%s\\n' \"$SMITH_TEST_ENVIRONMENT\" > configured.txt`. Do not use a file-writing tool or substitute a literal value. Finish with a report.",
        Expectation { before: None, files: vec![(b"configured.txt".to_vec(), b"configured\n".to_vec())] },
        false,
        false,
    );
}

#[test]
fn a_second_run_resumes_the_chat_from_its_files() {
    let environment = live::Environment::load(&["chatgpt"]);
    let scratch = Scratch::new();
    let marker = format!("smith-memory-{}", std::process::id());
    run(
        &environment,
        "chatgpt",
        &scratch,
        &format!(
            "Remember this code for our next invocation: {marker}. Do not write files. Call finish with a report."
        ),
        Expectation { before: None, files: vec![] },
        false,
        false,
    );
    let before = turns(&scratch);
    assert!(!before.is_empty(), "first invocation wrote durable turns");
    assert!(!scratch.path().join("repo/remembered.txt").exists(), "code is kept only in the conversation");
    change(
        &environment,
        "chatgpt",
        &scratch,
        &format!(
            "{CHANGE} Also create remembered.txt containing exactly the code I asked you to remember in the previous invocation, with no newline. Obtain it from the saved conversation."
        ),
        Some((b"remembered.txt".to_vec(), marker.into_bytes())),
        true,
    );
    let after = turns(&scratch);
    assert!(after.len() > before.len(), "resume appended durable history");
    assert!(before.iter().all(|entry| after.contains(entry)), "earlier turns remain unchanged");
}

#[test]
fn a_chat_crosses_from_chatgpt_to_claude_then_back() {
    let environment = live::Environment::load(&["chatgpt", "claude"]);
    let scratch = Scratch::new();
    for (account, filename, content, resumed) in [
        ("chatgpt", "first.txt", "first\n", false),
        ("claude", "second.txt", "second\n", true),
        ("chatgpt", "third.txt", "third\n", true),
    ] {
        run(
            &environment,
            account,
            &scratch,
            &format!(
                "Create {filename} in repository repo containing exactly {} followed by one newline. Finish with a report.",
                content.trim()
            ),
            Expectation { before: None, files: vec![(filename.as_bytes().to_vec(), content.as_bytes().to_vec())] },
            false,
            resumed,
        );
    }
    for (file, content) in
        [("first.txt", b"first\n".as_slice()), ("second.txt", b"second\n"), ("third.txt", b"third\n")]
    {
        assert!(
            std::fs::read(scratch.path().join("repo").join(file)).is_ok_and(|bytes| bytes == content),
            "each completed run keeps its file"
        );
    }
}

fn turns(scratch: &Scratch) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut workspaces = std::fs::read_dir(scratch.path().join("state/smith/chats")).expect("workspace store");
    let directory = workspaces.next().expect("one workspace").expect("workspace entry").path().join("chat");
    assert!(workspaces.next().is_none(), "only this story's workspace");
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .expect("durable chat")
        .map(|entry| entry.expect("turn entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "turn"))
        .map(|path| {
            let bytes = std::fs::read(&path).expect("durable turn");
            (path, bytes)
        })
        .collect();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

fn change(
    environment: &live::Environment,
    account: &str,
    scratch: &Scratch,
    prompt: &str,
    extra: Option<(Vec<u8>, Vec<u8>)>,
    resumed: bool,
) {
    let mut files = changed_files();
    files.extend(extra);
    run(
        environment,
        account,
        scratch,
        prompt,
        Expectation { before: scratch.checkout().head(), files },
        false,
        resumed,
    );
}

#[expect(clippy::disallowed_types, reason = "independent live checkout and remote observations")]
fn git(path: &std::path::Path, arguments: &[&str]) -> std::process::Output {
    std::process::Command::new("/usr/bin/git")
        .env_clear()
        .env("HOME", path)
        .env("PATH", std::env::var_os("PATH").expect("PATH"))
        .arg("-C")
        .arg(path)
        .args(arguments)
        .output()
        .expect("live git command")
}

#[test]
fn a_configured_push_lands() {
    let environment = live::Environment::load(&["chatgpt"]);
    let Some(remote) = std::env::var_os("SMITH_TEST_LIVE_GIT_REMOTE").filter(|value| !value.is_empty()) else {
        eprintln!("configured push not run: SMITH_TEST_LIVE_GIT_REMOTE is absent");
        return;
    };
    let remote = remote.to_str().expect("remote is UTF-8");
    let scratch = Scratch::new();
    let repo = scratch.path().join("repo");
    let branch = format!("smith-live-{}-{}", std::process::id(), skein_shell::Clock::new().now().now.as_nanos());
    assert!(git(&repo, &["checkout", "-q", "-b", &branch]).status.success(), "unique live branch");
    assert!(git(&repo, &["remote", "add", "origin", remote]).status.success(), "selected remote");
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(
            &environment,
            "chatgpt",
            &scratch,
            CHANGE,
            Expectation { before: scratch.checkout().head(), files: changed_files() },
            true,
            false,
        );
        let seen = git(&repo, &["ls-remote", "origin", &format!("refs/heads/{branch}")]);
        assert!(seen.status.success(), "inspect remote branch; diagnostics withheld");
        let head = scratch.checkout().head().expect("delivered head");
        assert!(
            seen.stdout.split(u8::is_ascii_whitespace).next() == Some(head.as_slice()),
            "push landed at the delivered commit"
        );
    }));
    let cleanup = git(&repo, &["push", "origin", &format!(":refs/heads/{branch}")]);
    assert!(cleanup.status.success(), "remove remote branch even on story failure; diagnostics withheld");
    if let Err(failure) = outcome {
        std::panic::resume_unwind(failure);
    }
}

fn run(
    environment: &live::Environment,
    account: &str,
    scratch: &Scratch,
    prompt: &str,
    expectation: Expectation,
    push: bool,
    resumed: bool,
) {
    use skein_lib::Duration;
    use skein_world::end_to_end::Command;
    use smith_bench::process::{Bounds, Capture, execute};
    let clock = skein_shell::Clock::new();
    let mut process_environment = vec![
        ("HOME".into(), scratch.path().join("home").into()),
        ("XDG_CONFIG_HOME".into(), environment.configuration().into()),
        ("XDG_STATE_HOME".into(), scratch.path().join("state").into()),
        ("PATH".into(), std::env::var_os("PATH").expect("live PATH")),
    ];
    std::fs::create_dir_all(scratch.path().join("home")).expect("private home");
    if account == "anthropic-key" {
        let key = std::env::var_os("ANTHROPIC_API_KEY").filter(|value| !value.is_empty()).expect(live::KEY);
        process_environment.push(("ANTHROPIC_API_KEY".into(), key));
    }
    let command = |arguments| Command {
        program: env!("CARGO_BIN_EXE_smith").into(),
        arguments,
        environment: process_environment.clone(),
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
    let change = expectation.before.is_some();
    let mut arguments = live::arguments(account, scratch.path(), change, push);
    if let Some(lead) = environment.refresh_lead {
        let at = arguments.len().checked_sub(1).expect("prompt argument");
        arguments.splice(at..at, ["-c".into(), format!("deployment.refresh_lead={lead}").into()]);
    }
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
    assert!(check.exit == smith_bench::Exit::Code(0), "live check must pass; diagnostic bytes withheld");
    let teardown = smith_bench::agent::smith::teardown_from_check(&check.stdout).expect("effective teardown bound");
    let _configuration = environment.configuration();
    let mut result = execute(
        &clock,
        command(arguments),
        format!("{prompt}\n").into_bytes().into(),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(observer(environment, scratch, teardown)))),
        bounds(Duration::from_secs(130), teardown),
    )
    .expect("shipped live exec settles");
    assert_exec(&mut result, scratch, prompt, expectation, account, resumed);
}

fn observer(
    environment: &live::Environment,
    scratch: &Scratch,
    teardown: skein_lib::Duration,
) -> smith_bench::agent::smith::Observing {
    let registry = smith_bench::agent::smith::FaceSince {
        commit: String::new(),
        shape: smith_bench::agent::smith::events::Shape::Exec,
        receiving: smith_bench::agent::smith::Receiving {
            string: 256_000,
            content: 13_056_000,
            items: 52_778,
            encoded_line: 19_817_472,
        },
    };
    let prepared = smith_bench::agent::smith::Prepared {
        face: registry.shape,
        registry,
        settings: environment.root.join("config/smith/config.toml"),
        credential: smith_bench::agent::smith::Credential::Record(None),
        trace: None,
        rendered: serde_json::Value::Null,
        program: env!("CARGO_BIN_EXE_smith").into(),
        state: scratch.path().join("state"),
        teardown,
    };
    smith_bench::agent::smith::Observing::new(prepared).expect("outside observer")
}

fn assert_exec(
    result: &mut smith_bench::process::ProcessResult,
    scratch: &Scratch,
    prompt: &str,
    expectation: Expectation,
    account: &str,
    resumed: bool,
) {
    let change = expectation.before.is_some();
    assert!(
        result.error.is_none() && result.forced == smith_bench::Forced::No && result.tree.live_pids.is_empty(),
        "live process tree settles; diagnostics withheld"
    );
    assert!(result.exit == smith_bench::Exit::Code(0), "live exec succeeds; diagnostics withheld");
    let Some(smith_bench::agent::Observing::Smith(observing)) = result.observing.as_deref_mut() else {
        panic!("exec observer")
    };
    observing.observer.finish().expect("complete typed event stream");
    assert!(
        matches!(observing.observer.classify(result.exit, result.forced).end, smith_bench::End::Completed),
        "typed answer and exit agree"
    );
    let records: Vec<_> = observing.observer.sessions().iter().flat_map(|session| &session.records).collect();
    assert!(
        records.iter().any(|record| matches!(&record.event, Event::RunStarted(start) if start.resumed == resumed)),
        "run reports the expected resume state"
    );
    assert!(records.iter().any(|record| matches!(&record.event, Event::ResponseStarted(start) if start.prompt.as_ref().is_some_and(|captured| captured.messages.iter().any(|message| message.blocks.iter().any(|block| matches!(block, Block::Text { text } if text.windows(prompt.len()).any(|part| part == prompt.as_bytes()))))))), "the task appears in the typed prompt");
    if account == "claude" {
        program_contract(&records);
    }
    if change {
        assert!(records.iter().any(|record| matches!(&record.event, Event::ToolCompleted(tool) if tool.tool.as_ref() == b"finish" && tool.delivery == Some(smith_events::Delivered::Delivered))), "finish delivered the change");
        assert!(
            records.iter().any(|record| matches!(&record.event, Event::CheckCompleted(check) if check.passed)),
            "the configured checks passed"
        );
        let head = scratch.checkout().head().expect("delivered commit");
        assert!(Some(&head) != expectation.before.as_ref(), "a new commit was delivered");
        assert!(
            scratch.checkout().message(&head).windows(15).any(|part| part == b"Smith-Delivery:"),
            "commit carries its delivery identity"
        );
        let files = scratch.checkout().files(&head);
        let original = scratch.checkout().files(expectation.before.as_ref().expect("initial commit"));
        assert!(
            files.get(b".smith-test/check".as_slice()) == original.get(b".smith-test/check".as_slice()),
            "the prescribed check stays unchanged"
        );
        for (path, content) in &expectation.files {
            assert!(files.get(path) == Some(content), "committed file matches its prescribed content");
        }
    }
    for (path, content) in expectation.files {
        assert!(
            std::fs::read(scratch.path().join("repo").join(std::str::from_utf8(&path).expect("test path")))
                .is_ok_and(|bytes| bytes == content),
            "checkout file matches its prescribed content"
        );
    }
}

fn program_contract(records: &[&smith_events::Record]) {
    let started = records
        .iter()
        .find_map(|record| if let Event::RunStarted(start) = &record.event { Some(start) } else { None })
        .expect("program admission event");
    assert!(started.main.backend == smith_events::Backend::ClaudeCode, "the product spawned the program backend");
    assert!(
        started.tools.families.as_slice()
            == [smith_events::Family::Inspect, smith_events::Family::Modify, smith_events::Family::Shell],
        "the program has exactly the granted capabilities"
    );
    let tools = records
        .iter()
        .find_map(|record| {
            if let Event::ResponseStarted(start) = &record.event {
                start.prompt.as_ref().and_then(|prompt| prompt.tools.as_ref()).filter(|tools| !tools.is_empty())
            } else {
                None
            }
        })
        .expect("the accepted init's typed tool inventory");
    for name in ["Read", "Glob", "Grep", "Write", "Edit", "Bash", "permission_prompt", "finish"] {
        assert!(
            tools.iter().any(|tool| tool.as_ref() == name.as_bytes()),
            "init provides a required built in or smith tool"
        );
    }
    assert!(tools.iter().all(|tool| tool.as_ref() != b"sub_agent"), "init offers no sub agent capability");
}
