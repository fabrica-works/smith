//! Native preparation and trace replay use unissued credentials and immutable fixtures.

use std::path::{Path, PathBuf};

use skein_lib::Duration;
use skein_shell::Clock;
use skein_world::end_to_end::Command;
use smith_bench::agent::smith::legacy::Shape;
use smith_bench::agent::smith::{Credential, CredentialShape, Face, FaceSince, Observing, Prepared};
use smith_bench::credential::TokenShape;
use smith_bench::process::{Bounds, Capture, execute};
use smith_bench::{End, Forced, Measure, ScopeKind};

mod common;

#[test]
fn a_terminal_peer_can_close_its_stdio_before_the_leader_exits_without_hangup() {
    let directory = root("terminal-before-exit");
    std::fs::create_dir(&directory).expect("private fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("empty append file");
    let recording = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/smith-legacy/completed.jsonl");
    let script = "import os,signal,sys,time; signal.signal(signal.SIGHUP,signal.SIG_DFL); sys.stdin.readline(); open(sys.argv[2],'wb').write(open(sys.argv[1],'rb').read()); sys.stdin.readline(); os.close(0); os.close(1); os.write(2,b'tty stdio closed before leader exit\\n'); time.sleep(0.005)";
    let command = Command {
        program: "/usr/bin/python3".into(),
        arguments: vec!["-c".into(), script.into(), recording.into_os_string(), trace.clone().into_os_string()],
        environment: vec![("PATH".into(), "/usr/bin:/bin".into())],
        directory: directory.clone(),
    };
    let result = execute(
        &Clock::new(),
        command,
        Box::from(&b"finish the task\n"[..]),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(observing(&trace)))),
        bounds(),
    )
    .expect("tree settles");
    std::fs::remove_dir_all(directory).expect("fixture cleanup");
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.forced, Forced::No);
    assert!(result.answered.is_some());
    assert!(result.tree.live_pids.is_empty());
    assert_eq!(result.exit, smith_bench::Exit::Code(0));
}

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("smith-bench-legacy-{}-{name}", std::process::id()))
}

fn observing(trace: &Path) -> Observing {
    let face = Face::Legacy(Shape::LocalJson);
    Observing::new(Prepared {
        face,
        settings: trace.with_extension("settings"),
        credential: Credential::Record(None),
        trace: Some(trace.into()),
        registry: FaceSince {
            commit: "30259d89b88834ed3edaaa08b3f320d2252bbb9a".into(),
            face,
            credential: CredentialShape::Converted(TokenShape::Legacy),
            facts: smith_bench::agent::smith::legacy::FactShape::Flags,
            receiving: None,
        },
        rendered: serde_json::json!({"models":[{"input_price":10,"cached_price":1,"output_price":80,"price_unit":8_000_000}]}),
        program: "/bin/sh".into(),
        state: trace.with_extension("state"),
        teardown: Duration::from_millis(40),
    }).expect("legacy observing")
}

fn bounds() -> Bounds {
    Bounds {
        hard: Duration::from_millis(150),
        teardown: Duration::from_millis(40),
        grace: Duration::from_millis(20),
        settlement: Duration::from_millis(200),
        stdout: 16384,
        stderr: 4096,
        force_walk: true,
    }
}

#[test]
fn the_selected_event_face_observes_the_answer_and_drains_its_native_boundaries() {
    let directory = root("events");
    std::fs::create_dir(&directory).expect("private fixture directory");
    let trace = directory.join("trace.jsonl");
    let replay = directory.join("codec-control.jsonl");
    let records = common::events::bytes(&common::events::balanced());
    std::fs::write(&trace, []).expect("empty append stream");
    std::fs::write(&replay, &records).expect("synthetic codec records");
    let face = Face::Events(smith_bench::agent::smith::events::Shape::LocalTrace);
    let prepared = Prepared {
        face,
        settings: directory.join("settings.json"),
        credential: Credential::Record(None),
        trace: Some(trace.clone()),
        registry: FaceSince {
            commit: "30259d89b88834ed3edaaa08b3f320d2252bbb9a".into(),
            face,
            credential: CredentialShape::Converted(TokenShape::AccessOnly),
            facts: smith_bench::agent::smith::legacy::FactShape::Kinds,
            receiving: Some(serde_json::from_value(serde_json::json!({
                "string":524_288,"content":4096,"items":32,
                "encoded_line":records.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("record lines")
            })).expect("immutable writer-derived receiving ceiling")),
        },
        rendered: serde_json::json!({}),
        program: "/bin/sh".into(),
        state: directory.join("state"),
        teardown: bounds().teardown,
    };
    let observer = Observing::new(prepared).expect("checked event receiving bounds");
    let command = Command {
        program: "/bin/sh".into(),
        arguments: vec![
            "-c".into(),
            "IFS= read -r prompt; cat \"$1\" >> \"$2\"; if IFS= read -r extra; then exit 9; fi".into(),
            "codec-replay".into(),
            replay.into_os_string(),
            trace.into_os_string(),
        ],
        environment: vec![("PATH".into(), "/usr/bin:/bin".into())],
        directory: directory.clone(),
    };
    let result = execute(
        &Clock::new(),
        command,
        Box::from(&b"task\n"[..]),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(observer))),
        bounds(),
    )
    .expect("real replay tree settles");
    assert_eq!(result.forced, Forced::No);
    assert_eq!(result.trace, records);
    assert!(result.answered.is_some());
    assert_eq!(result.error, None);
    let Some(smith_bench::agent::Observing::Smith(native)) = result.observing.as_deref() else {
        panic!("selected native face retained");
    };
    assert_eq!(native.observer.classify(result.exit, result.forced).end, End::Completed);
    assert_eq!(native.counts().responses, Measure::Observed { value: 1 });
    let health = native.health(result.exit, result.forced).expect("typed passive evidence");
    for check in &health.passive_checks {
        if check.check == "teardown"
            || (check.check.starts_with("headroom/")
                && !matches!(check.check.as_str(), "headroom/prompt-tokens" | "headroom/spend"))
        {
            assert!(matches!(check.passed, Measure::Unavailable { .. }), "{}", check.check);
        } else {
            assert_eq!(check.passed, Measure::Observed { value: true }, "{}", check.check);
        }
    }
    assert_eq!(health.headroom.len(), 7);
    let spend = native.spend(&native.tokens().expect("exact response usage")).expect("reported notional spend");
    assert_eq!(spend.basis, smith_bench::SpendBasis::Notional);
    assert_eq!(spend.usd, Measure::Observed { value: 0.000_003 });
    std::fs::remove_dir_all(directory).expect("replay fixture removed");
}

#[test]
fn a_terminal_person_closes_after_main_acceptance_and_drains_the_final_file_record() {
    let directory = root("terminal");
    std::fs::create_dir(&directory).expect("fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("empty append file");
    let recording = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/smith-legacy/completed.jsonl");
    let command = Command { program:"/bin/sh".into(),arguments:vec!["-c".into(),
        "IFS= read -r prompt; cat \"$1\" >> \"$2\"; if IFS= read -r extra; then exit 9; fi; printf '%s\\n' '{\"type\":\"fact\",\"at_ns\":77,\"fact\":\"Run { fact: Answered { run: Token(0), answer: Accepted } }\"}' >> \"$2\"".into(),
        "replay".into(),recording.into_os_string(),trace.clone().into_os_string()],
        environment:vec![("PATH".into(),"/usr/bin:/bin".into())],directory:directory.clone() };
    let result = execute(
        &Clock::new(),
        command,
        Box::from(&b"finish the task\n"[..]),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(observing(&trace)))),
        bounds(),
    )
    .expect("tree settles");
    assert!(result.error.is_none(), "{:?}", result.error);
    assert_eq!(result.forced, Forced::No);
    assert!(result.answered.is_some());
    assert!(String::from_utf8_lossy(&result.trace).contains("Answered { run"));
    assert_eq!(result.observing.expect("paired observer").classify(result.exit, result.forced).end, End::Completed);
    assert!(result.tree.live_pids.is_empty());
    std::fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn an_incomplete_file_record_faults_after_genuine_tree_settlement_and_retains_its_prefix() {
    let directory = root("partial");
    std::fs::create_dir(&directory).expect("fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, b"{\"type\":").expect("partial fixture");
    let command =
        Command { program: "/bin/true".into(), arguments: vec![], environment: vec![], directory: directory.clone() };
    let result = execute(
        &Clock::new(),
        command,
        Box::new([]),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(observing(&trace)))),
        bounds(),
    )
    .expect("settled tree");
    assert!(result.error.as_ref().is_some_and(|error| error.contains("partial record")));
    assert_eq!(result.trace, b"{\"type\":");
    assert!(result.tree.live_pids.is_empty());
    std::fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn a_failed_main_answer_also_closes_person_input_and_preserves_its_budget_end() {
    let directory = root("failed-answer");
    std::fs::create_dir(&directory).expect("fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("append file");
    let command = Command {
        program: "/bin/sh".into(),
        arguments: vec![
            "-c".into(),
            "IFS= read -r prompt; printf '%s\\n' '{\"type\":\"fact\",\"at_ns\":1,\"fact\":\"Run { fact: Answered { run: Token(0), answer: Failed(Budget(Time)) } }\"}' >> \"$1\"; if IFS= read -r extra; then exit 9; fi".into(),
            "replay".into(),
            trace.clone().into_os_string(),
        ],
        environment: vec![("PATH".into(), "/usr/bin:/bin".into())],
        directory: directory.clone(),
    };
    let result = execute(
        &Clock::new(),
        command,
        Box::from(&b"finish the task\n"[..]),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(observing(&trace)))),
        bounds(),
    )
    .expect("settled failure");
    assert_eq!(result.exit, smith_bench::Exit::Code(0));
    assert_eq!(result.forced, Forced::No);
    assert!(result.answered.is_some());
    assert_eq!(
        result.observing.expect("paired observer").classify(result.exit, result.forced).end,
        End::Budget { which: smith_bench::BudgetLimit::Time }
    );
    assert!(result.tree.live_pids.is_empty());
    std::fs::remove_dir_all(directory).expect("cleanup");
}

fn fact(observing: &mut Observing, text: &str) {
    let line = serde_json::json!({"type":"fact","at_ns":1,"fact":text}).to_string();
    observing.feed_line(&line).expect("native fact");
}

#[test]
fn a_later_person_line_waits_for_its_named_tool_cue_and_eof_follows_that_line() {
    use smith_bench::agent::{Close, Cue, Script, Trigger};
    let directory = root("cue");
    std::fs::create_dir(&directory).expect("fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("append file");
    let recording = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/smith-legacy/completed.jsonl");
    let command=Command {program:"/bin/sh".into(),arguments:vec!["-c".into(),
        "IFS= read -r prompt; cat \"$1\" >> \"$2\"; IFS= read -r reply || exit 7; test \"$reply\" = 'second line' || exit 8; if IFS= read -r extra; then exit 9; fi".into(),
        "replay".into(),recording.into_os_string(),trace.clone().into_os_string()],
        environment:vec![("PATH".into(),"/usr/bin:/bin".into())],directory:directory.clone()};
    let script = Script {
        cues: vec![
            Cue { when: Trigger::Start, text: "first line\n".into() },
            Cue { when: Trigger::ToolCall { tool: "finish".into() }, text: "second line\n".into() },
        ],
        close: Close::AfterLastCue,
    };
    let result = smith_bench::process::execute_script(
        &Clock::new(),
        command,
        script,
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(observing(&trace)))),
        bounds(),
    )
    .expect("settled script");
    assert_eq!(result.exit, smith_bench::Exit::Code(0));
    assert_eq!(result.forced, Forced::No);
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(result.tree.live_pids.is_empty());
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn response_content_counts_once_and_aggregate_used_facts_do_not_charge_again() {
    let mut observer = observing(Path::new("/unused/offline-trace"));
    fact(&mut observer, "Run { fact: Opened { run: Token(0), conversation: Token(0), child: false } }");
    fact(&mut observer, "Session { fact: Opened { opener: Token(0) } }");
    let usage = "Usage { input_tokens: 20, output_tokens: 8, cache_read_tokens: 5, cache_write_tokens: 2 }";
    for _ in 0..2 {
        fact(
            &mut observer,
            "Session { fact: CompletionAnswered { opener: Token(0), stop: ToolUse, blocks: 1, calls: 1, invalid: 0 } }",
        );
        fact(&mut observer, &format!("Session {{ fact: Used {{ opener: Token(0), usage: {usage} }} }}"));
        let line = serde_json::json!({"type":"usage","at_ns":2,"owner":0,"usage":usage}).to_string();
        observer.feed_line(&line).expect("content usage");
        observer.feed_line(&line).expect("identical repeated content");
    }
    let records = observer.tokens().expect("per-response ledger");
    assert!(matches!(observer.counts().responses, Measure::LowerBound { value: 2, .. }));
    let total = records.iter().find(|record| record.role == ScopeKind::Total).expect("total");
    assert!(matches!(total.usage.fresh, Measure::LowerBound { value: 44, .. }));
    assert!(matches!(total.usage.output, Measure::LowerBound { value: 16, .. }));
    let priced = observer.spend(&records).expect("notional prices");
    let Measure::LowerBound { value, .. } = priced.usd else {
        panic!("loss remains a gap");
    };
    assert!((value - 0.000_216_25).abs() < 1.0e-14);
    let line=serde_json::json!({"type":"usage","at_ns":2,"owner":0,"usage":usage.replace("output_tokens: 8","output_tokens: 9")}).to_string();
    observer.feed_line(&line).expect("well formed conflicting record");
    assert!(observer.tokens().expect_err("conflicting repeat").contains("conflicting repeated"));
}

#[test]
fn native_kind_parent_links_keep_child_response_usage_in_its_actual_scope() {
    use skein_lib::Token;
    use smith_bench::agent::smith::legacy::FactShape;
    use smith_domain_run::facts::{FactKind, Kind};
    use smith_domain_run::{Granted, Set, Share};
    let mut prepared = observing(Path::new("/unused-native-kind-trace")).prepared;
    prepared.registry.facts = FactShape::Kinds;
    let mut observer = Observing::new(prepared).expect("prepared legacy observer");
    for native in [
        FactKind::Opened { run: Token::new(0), conversation: Token::new(0), kind: Kind::Main },
        FactKind::Opened {
            run: Token::new(0),
            conversation: Token::new(2),
            kind: Kind::Child {
                parent: Token::new(0),
                call: Token::new(9),
                asked: Share::UNASKED,
                granted: Granted {
                    turns: 2,
                    time: Duration::from_secs(1),
                    spend: 1,
                    turns_set: Set::Pool,
                    time_set: Set::Pool,
                },
            },
        },
    ] {
        let line = serde_json::json!({"type":"fact", "at_ns":1,
            "fact":format!("Run {{ fact: {native:?} }}")})
        .to_string();
        observer.feed_line(&line).expect("native opening");
    }
    let usage = "Usage { input_tokens: 20, output_tokens: 8, cache_read_tokens: 5, cache_write_tokens: 2 }";
    for owner in [0, 2] {
        for native in [
            format!("Opened {{ opener: Token({owner}) }}"),
            format!("CompletionAnswered {{ opener: Token({owner}), stop: ToolUse, blocks: 1, calls: 1, invalid: 0 }}"),
        ] {
            let line = serde_json::json!({"type":"fact", "at_ns":2,
                "fact":format!("Session {{ fact: {native} }}")})
            .to_string();
            observer.feed_line(&line).expect("session boundary");
        }
        let line = serde_json::json!({"type":"usage", "at_ns":3,"owner":owner,"usage":usage}).to_string();
        observer.feed_line(&line).expect("scoped content");
        observer.feed_line(&line).expect("immutable duplicate");
    }
    let records = observer.tokens().expect("linked child ledger");
    let child = records.iter().find(|record| record.scope == "conversation-2").expect("child");
    assert_eq!(child.role, ScopeKind::Child { parent: "conversation-0".into() });
    assert!(matches!(child.usage.output, Measure::LowerBound { value: 8, .. }));
    let total = records.iter().find(|record| record.role == ScopeKind::Total).expect("total");
    assert!(matches!(total.usage.output, Measure::LowerBound { value: 16, .. }));
}
