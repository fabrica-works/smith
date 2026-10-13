//! Native preparation and trace replay use unissued credentials and immutable fixtures.

pub mod common;
use common::{unexecuted_arm, unissued_auth};

use std::path::{Path, PathBuf};

use skein_lib::Duration;
use skein_shell::Clock;
use skein_world::end_to_end::Command;
use smith_bench::agent::smith::{Credential, FaceSince, Observing, Prepared};
use smith_bench::process::{Bounds, Capture, execute};
use smith_bench::{End, Forced, Measure};

#[test]
fn the_first_request_pin_renders_only_the_probes_empty_instructions() {
    use smith_bench::agent::Context;
    use smith_bench::arm::ArmRef;

    let directory = root("first-request-settings");
    let source = directory.join("user/.codex");
    std::fs::create_dir_all(&source).expect("synthetic login directory");
    let auth = unissued_auth();
    std::fs::write(source.join("auth.json"), &auth).expect("synthetic source only");
    let guard = smith_bench::guard::Guard::new(
        &directory.join("user"),
        &directory.join("config"),
        &directory.join("state"),
        &[],
    )
    .expect("fixture guard");
    let private = smith_bench::isolate::empty(&guard, &directory.join("scratch")).expect("private fixture");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite = smith_bench::read_suite(&project.join("suites/probes.toml")).expect("dedicated suite");
    let agent = suite
        .agents
        .iter()
        .find(|agent| agent.tasks.iter().any(|name| name == "probes/first-request"))
        .expect("probe pin");
    assert_eq!(agent.config, "first-request");
    let pin_file = project.join("agents/smith").join(format!("{}.pin.toml", agent.config));
    let pin = smith_bench::read_configuration(&pin_file).expect("selected committed pin");
    let task = smith_bench::read_task(&project.join("tasks/probes/first-request/task.toml")).expect("original task");
    let model = smith_bench::ModelChoice { model: Some("selected-model".into()), effort: Some("low".into()) };
    let arm = ArmRef::Built(unexecuted_arm(&directory, project.parent().expect("repository")));
    let environment = vec![("NAMED".into(), "exact-value".into())];
    let mut context = Context {
        guard: &guard,
        private: &private,
        pin: &pin,
        pin_file: &pin_file,
        task: &task,
        variant: None,
        model: &model,
        arm: &arm,
        installed_version: match &arm {
            ArmRef::Built(build) => &build.arm.commit,
            ArmRef::Installed { .. } => unreachable!("fixture arm"),
        },
        home: &source,
        wall: skein_lib::Wall::from_nanos(100_000_000_000),
        lifetime: Duration::from_secs(20),
        environment: &environment,
    };
    let first = smith_bench::agent::smith::prepare(&context).expect("actual first-request native preparation");
    assert_eq!(first.rendered["instructions"], "", "empty brief must not demand AGENTS/read calls");
    let converted = match first.credential {
        Credential::Record(Some(borrowed)) => borrowed,
        Credential::Record(None) => panic!("synthetic credential prepared"),
    };
    assert!(converted.finish(&guard).expect("no process was started").source_unchanged);
    std::fs::remove_dir_all(private.root.join("smith")).expect("remove first unused preparation");
    let old_file = project.join("agents/smith/smoke.pin.toml");
    let old = smith_bench::read_configuration(&old_file).expect("unchanged smoke pin");
    context.pin = &old;
    context.pin_file = &old_file;
    let prepared = smith_bench::agent::smith::prepare(&context).expect("old native preparation");
    let mut expected = prepared.rendered;
    expected["instructions"] = serde_json::json!("");
    assert_eq!(first.rendered, expected, "all other native fields remain identical");
    assert_eq!(
        smith_bench::read_suite(&project.join("suites/calibrate.toml")).expect("other probes").agents[0].config,
        "smoke"
    );
    match prepared.credential {
        Credential::Record(Some(borrowed)) => {
            assert!(borrowed.finish(&guard).expect("unused preparation").source_unchanged);
        }
        Credential::Record(None) => panic!("synthetic credential prepared"),
    }
    assert_eq!(std::fs::read(source.join("auth.json")).expect("source unchanged"), auth);
    std::fs::remove_dir_all(directory).expect("remove fixture");
}

#[test]
fn a_terminal_peer_can_close_its_stdio_before_the_leader_exits_without_hangup() {
    let directory = root("terminal-before-exit");
    std::fs::create_dir(&directory).expect("private fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("empty append file");
    let recording = directory.join("recording.jsonl");
    std::fs::write(&recording, common::events::bytes(&common::events::balanced())).expect("event recording");
    let command = Command {
        program: env!("CARGO_BIN_EXE_test-peer").into(),
        arguments: vec!["terminal".into(), recording.into_os_string(), trace.clone().into_os_string()],
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
    std::env::temp_dir().join(format!("smith-bench-events-{}-{name}", std::process::id()))
}

fn observing(trace: &Path) -> Observing {
    let face = smith_bench::agent::smith::events::Shape::LocalTrace;
    Observing::new(Prepared {
        face,
        settings: trace.with_extension("settings"),
        credential: Credential::Record(None),
        trace: Some(trace.into()),
        registry: FaceSince {
            commit: "30259d89b88834ed3edaaa08b3f320d2252bbb9a".into(),
            shape: face,
            session_ended_aborted: true,
            receiving: serde_json::from_value(serde_json::json!({"string":524_288,"content":4096,"items":32,"encoded_line":16384})).expect("fixture bounds"),
        },
        rendered: serde_json::json!({"models":[{"input_price":10,"cached_price":1,"output_price":80,"price_unit":8_000_000}]}),
        program: "/bin/sh".into(),
        state: trace.with_extension("state"),
        teardown: Duration::from_millis(40),
    }).expect("event observing")
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
    let face = smith_bench::agent::smith::events::Shape::LocalTrace;
    let prepared = Prepared {
        face,
        settings: directory.join("settings.json"),
        credential: Credential::Record(None),
        trace: Some(trace.clone()),
        registry: FaceSince {
            commit: "30259d89b88834ed3edaaa08b3f320d2252bbb9a".into(),
            shape: face,
            session_ended_aborted: true,
            receiving: serde_json::from_value(serde_json::json!({
                "string":524_288,"content":4096,"items":32,
                "encoded_line":records.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("record lines")
            })).expect("writer-derived receiving ceiling"),
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
fn an_earlier_event_face_keeps_the_absent_teardown_field_unknown() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let faces = smith_bench::agent::smith::read_faces(&directory.join("agents/smith/faces.toml"))
        .expect("frozen face registry");
    let before =
        faces.iter().find(|face| face.commit == "b86b022a83c16194cf716015fe0dff4db9569163").expect("pre-aborted face");
    let current = faces
        .iter()
        .find(|face| face.commit == "8d2ec9b878ae85240575788809c44fe0028e6a4a")
        .expect("aborted field's first writer");
    assert!(!before.session_ended_aborted);
    assert!(current.session_ended_aborted);
    let recording = include_str!("recorded/smith-before-aborted/terminal.jsonl");
    assert!(!recording.contains("\"aborted\""));
    let mut prepared = observing(Path::new("/unused-before-aborted")).prepared;
    prepared.face = before.shape;
    prepared.registry = before.clone();
    let mut historical = Observing::new(prepared).expect("historical layout");
    for line in recording.lines() {
        historical.feed_line(line).expect("old writer's recording");
    }
    {
        let observer = &historical.observer;
        let records = &observer.sessions().last().expect("recorded session").records;
        let smith_events::Event::SessionEnded(ended) = &records.last().expect("recorded terminal").event else {
            panic!("recording ends its stream")
        };
        assert_eq!(ended.aborted, None);
        assert_eq!(
            smith_events::write_with_ceiling(
                records.last().expect("terminal"),
                &smith_events::Capture::Everything,
                &before.receiving.limits().expect("semantic caps"),
                before.receiving.encoded_line,
            ),
            Err(smith_events::Error::Shape)
        );
    }
    historical.restart_trace().expect("restart retains the arm's layout");
    for line in recording.lines() {
        historical.feed_line(line).expect("same historical layout after restart");
    }
    historical.prepared.registry = current.clone();
    historical.restart_trace().expect("current layout");
    let mut lines = recording.lines();
    historical.feed_line(lines.next().expect("startup")).expect("current startup");
    assert!(historical.feed_line(lines.next().expect("terminal")).is_err());
}

#[test]
fn a_terminal_person_closes_after_main_acceptance_and_drains_the_final_file_record() {
    let directory = root("terminal");
    std::fs::create_dir(&directory).expect("fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("empty append file");
    let recording = directory.join("recording.jsonl");
    std::fs::write(&recording, common::events::bytes(&common::events::balanced())).expect("event recording");
    let command = Command { program:"/bin/sh".into(),arguments:vec!["-c".into(),
        "IFS= read -r prompt; head -n -1 \"$1\" >> \"$2\"; if IFS= read -r extra; then exit 9; fi; tail -n 1 \"$1\" >> \"$2\"".into(),
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
    assert!(String::from_utf8_lossy(&result.trace).contains("session.ended"));
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
    let recording = directory.join("recording.jsonl");
    std::fs::write(&recording, failed_recording()).expect("budget terminal");
    let command = Command {
        program: "/bin/sh".into(),
        arguments: vec![
            "-c".into(),
            "IFS= read -r prompt; cat \"$1\" >> \"$2\"; if IFS= read -r extra; then exit 9; fi".into(),
            "replay".into(),
            recording.into_os_string(),
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

fn failed_recording() -> Vec<u8> {
    let mut records = common::events::balanced();
    let failed = common::events::record(include_str!("../../crates/smith-events/golden/v1/answer_class_budget.jsonl"));
    let smith_events::Event::RunCompleted(mut failed) = failed.event else { panic!("run terminal") };
    failed.run = 0;
    failed.failure.as_mut().expect("budget").which = Some(Box::from(&b"time"[..]));
    for record in &mut records {
        if let smith_events::Event::SessionStarted(value) = &mut record.event {
            value.mode = smith_events::Mode::Chat;
        }
        if let smith_events::Event::RunCompleted(value) = &mut record.event {
            value.status = smith_events::Status::Failed;
            value.result = None;
            value.failure.clone_from(&failed.failure);
        }
    }
    common::events::bytes(&records)
}

fn person_capture(trace: &Path) -> Capture {
    let mut prepared = observing(trace).prepared;
    prepared.face = smith_bench::agent::smith::events::Shape::LocalTrace;
    prepared.registry.shape = prepared.face;
    prepared.registry.receiving = serde_json::from_value(serde_json::json!({
        "string":128,"content":1024,"items":16,"encoded_line":4096
    }))
    .expect("fixture receiving limits");
    Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(Observing::new(prepared).expect("event observer"))))
}

fn person_recording() -> Vec<u8> {
    let mut records = common::events::balanced();
    let mut started =
        common::events::record(include_str!("../../crates/smith-events/golden/v1/tool_started_everything.jsonl"));
    started.t_ms = 4;
    if let smith_events::Event::ToolStarted(value) = &mut started.event {
        value.run = 0;
        value.conversation = 0;
        value.call = Box::from(&b"finish"[..]);
        value.tool = Box::from(&b"finish"[..]);
        value.deadline_ms = Some(100);
        value.input_bytes = 0;
        value.input = None;
    }
    records.insert(4, started);
    common::events::bytes(&records)
}

#[test]
fn a_task_person_line_follows_its_tool_cue_at_the_terminal() {
    let directory = root("task-cue");
    std::fs::create_dir(&directory).expect("fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("append file");
    let replay = directory.join("records.jsonl");
    std::fs::write(&replay, person_recording()).expect("tool cue");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut task = smith_bench::read_task(&project.join("tests/recorded/tasks/valid/task.toml")).expect("task");
    task.person = vec![
        toml::from_str("when = { tool-call = { tool = 'finish' } }\ntext = \"second line\\n\"\n")
            .expect("authored cue"),
    ];
    task.input_end = Some(smith_bench::agent::Close::AfterLastCue);
    let command=Command {program:"/bin/sh".into(),arguments:vec!["-c".into(),
        "IFS= read -r prompt; cat \"$1\" >> \"$2\"; IFS= read -r reply || exit 7; test \"$reply\" = 'second line' || exit 8; if IFS= read -r extra; then exit 9; fi".into(),
        "replay".into(),replay.into_os_string(),trace.clone().into_os_string()],
        environment:vec![("PATH".into(),"/usr/bin:/bin".into())],directory:directory.clone()};
    let capture = person_capture(&trace);
    let result = smith_bench::process::execute_script(
        &Clock::new(),
        command,
        smith_bench::run::script(&task, &capture),
        capture,
        bounds(),
    )
    .expect("settled task script");
    assert_eq!(result.exit, smith_bench::Exit::Code(0), "{:?}", result.error);
    assert_eq!(result.forced, Forced::No);
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(result.tree.live_pids.is_empty());
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn a_later_person_line_waits_for_its_named_tool_cue_and_eof_follows_that_line() {
    use smith_bench::agent::{Close, Cue, Script, Trigger};
    let directory = root("cue");
    std::fs::create_dir(&directory).expect("fixture directory");
    let trace = directory.join("trace.jsonl");
    std::fs::write(&trace, []).expect("append file");
    let recording = directory.join("recording.jsonl");
    std::fs::write(&recording, person_recording()).expect("event recording");
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
#[expect(clippy::too_many_lines, reason = "one bounded native mount admission and refusal matrix")]
fn path_mounts_reach_native_policy_and_escaped_mounts_refuse_before_credentials() {
    use smith_bench::agent::Context;
    use smith_bench::arm::ArmRef;
    let directory = root("path-settings");
    let source = directory.join("user/.codex");
    std::fs::create_dir_all(&source).expect("synthetic source");
    let auth = unissued_auth();
    std::fs::write(source.join("auth.json"), &auth).expect("unissued credential");
    let guard = smith_bench::guard::Guard::new(
        &directory.join("user"),
        &directory.join("config"),
        &directory.join("state"),
        &[],
    )
    .expect("fixture guard");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&project.join("tasks/probes/paths/task.toml"), &project.join("../docs/design"))
        .expect("frozen task");
    let private = smith_bench::isolate::prepare(
        &guard,
        &project.join("tasks/probes/paths/seed"),
        &task.seed_sha256,
        &directory.join("scratch"),
    )
    .expect("actual frozen seed materialisation");
    let pin_file = project.join("agents/smith/first-request.pin.toml");
    let pin = smith_bench::read_configuration(&pin_file).expect("native pin");
    let arm = ArmRef::Built(unexecuted_arm(&directory, project.parent().expect("repository")));
    let installed_version = match &arm {
        ArmRef::Built(build) => &build.arm.commit,
        ArmRef::Installed { .. } => unreachable!("fixture arm"),
    };
    let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
    let context = Context {
        guard: &guard,
        private: &private,
        pin: &pin,
        pin_file: &pin_file,
        task: &task,
        variant: None,
        model: &model,
        arm: &arm,
        installed_version,
        home: &source,
        wall: skein_lib::Wall::from_nanos(100_000_000),
        lifetime: Duration::from_secs(200),
        environment: &[],
    };
    for variant in [None, Some(&task.variant[0])] {
        let context = Context { variant, ..context };
        let prepared = smith_bench::agent::smith::prepare(&context).expect("actual native renderer");
        assert_eq!(prepared.rendered["instructions"], "");
        let resolved = native_product(&context);
        let local = resolved.local;
        local.validate(&resolved.limits).expect("actual mounted run policy");
        let mounts = &local.workspace.as_ref().expect("workspace").directories;
        assert_eq!(mounts.len(), if variant.is_some() { 1 } else { 2 });
        assert_eq!(mounts[0].name.as_ref(), b"app");
        assert!(mounts[0].writable);
        assert!(!mounts[0].git);
        assert_eq!(mounts[0].host.as_ref(), private.workspace.join("app").as_os_str().as_encoded_bytes());
        if variant.is_none() {
            assert_eq!(mounts[1].name.as_ref(), b"spec");
            assert!(!mounts[1].writable);
            assert_eq!(mounts[1].host.as_ref(), private.workspace.join("spec").as_os_str().as_encoded_bytes());
        }
        match prepared.credential {
            Credential::Record(Some(borrowed)) => {
                assert!(borrowed.finish(&guard).expect("unused arm").source_unchanged);
            }
            Credential::Record(None) => panic!("synthetic credential prepared"),
        }
        std::fs::remove_dir_all(private.root.join("smith")).expect("reset unused preparation");
    }
    let mut default_mount = task.clone();
    default_mount.smith.remove("directories");
    let default_context = Context { task: &default_mount, ..context };
    let default_prepared = smith_bench::agent::smith::prepare(&default_context).expect("default task working mount");
    assert_eq!(default_prepared.rendered["directories"].as_array().expect("default mount").len(), 1);
    assert_eq!(default_prepared.rendered["directories"][0]["path"], serde_json::json!(private.workspace.join("app")));
    match default_prepared.credential {
        Credential::Record(Some(borrowed)) => {
            assert!(borrowed.finish(&guard).expect("unused default mount").source_unchanged);
        }
        Credential::Record(None) => panic!("synthetic credential prepared"),
    }
    std::fs::remove_dir_all(private.root.join("smith")).expect("reset default mount");
    let outside = directory.join("outside");
    std::fs::create_dir(&outside).expect("owned outside fixture");
    std::os::unix::fs::symlink(&outside, private.workspace.join("escaped")).expect("owned symlink escape");
    let refusals = [
        "[]",
        "[{ name = 'app', path = '../outside', writable = true, git = false }]",
        "[{ name = 'app', path = '/tmp', writable = true, git = false }]",
        "[{ name = 'app', path = 'escaped', writable = true, git = false }]",
        "[{ name = 'app', path = 'absent', writable = true, git = false }]",
        "[{ name = 'app', path = 'spec', writable = true, git = false }]",
        "[{ name = 'app', path = 'app/input.txt', writable = true, git = false }]",
        "[{ name = '.', path = 'app', writable = true, git = false }]",
        "[{ name = 'a/b', path = 'app', writable = true, git = false }]",
        "[{ name = 'app', path = 'app', writable = true, git = false }, { name = 'app', path = 'spec', writable = false, git = false }]",
        "[{ name = 'app', path = 'app', writable = true, git = false }, { name = 'alias', path = './app', writable = false, git = false }]",
        "[{ name = 'root', path = '.', writable = true, git = false }, { name = 'app', path = 'app', writable = true, git = false }]",
        "[{ name = 'app', path = 'app', writable = true, git = false, extra = 1 }]",
        "[{ name = 'app', path = 'app', git = false }]",
    ];
    for refusal in refusals {
        let mut refused_task = task.clone();
        let value: toml::Value = toml::from_str(&format!("directories = {refusal}")).expect("authored TOML");
        refused_task.smith.insert("directories".into(), value["directories"].clone());
        let context = Context { task: &refused_task, home: &directory.join("missing-login"), ..context };
        let refusal = smith_bench::agent::smith::prepare(&context).expect_err("refusal before source login lookup");
        assert_eq!(refusal.step, smith_bench::isolate::SetupStep::Configuration);
        assert!(!private.root.join("smith/tokens").exists(), "no credential conversion");
        std::fs::remove_dir_all(private.root.join("smith")).expect("reset refused dry rendering");
    }
    assert_eq!(std::fs::read(source.join("auth.json")).expect("synthetic source"), auth);
    std::fs::remove_dir_all(directory).expect("private fixture cleanup");
}

#[test]
fn tool_output_profiles_render_and_pass_the_actual_public_startup_derivation() {
    use smith_bench::agent::Context;
    use smith_bench::arm::ArmRef;
    let directory = root("tool-output-settings");
    let source = directory.join("user/.codex");
    std::fs::create_dir_all(&source).expect("synthetic source");
    std::fs::write(source.join("auth.json"), unissued_auth()).expect("unissued credential");
    let guard = smith_bench::guard::Guard::new(
        &directory.join("user"),
        &directory.join("config"),
        &directory.join("state"),
        &[],
    )
    .expect("fixture guard");
    let private = smith_bench::isolate::empty(&guard, &directory.join("scratch")).expect("private fixture");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task =
        smith_bench::check_task(&project.join("tasks/probes/tool-output/task.toml"), &project.join("../docs/design"))
            .expect("frozen task");
    let pin_file = project.join("agents/smith/first-request.pin.toml");
    let pin = smith_bench::read_configuration(&pin_file).expect("native pin");
    let arm = ArmRef::Built(unexecuted_arm(&directory, project.parent().expect("repository")));
    let installed_version = match &arm {
        ArmRef::Built(build) => &build.arm.commit,
        ArmRef::Installed { .. } => unreachable!("fixture arm"),
    };
    let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
    for variant in &task.variant {
        let context = Context {
            guard: &guard,
            private: &private,
            pin: &pin,
            pin_file: &pin_file,
            task: &task,
            variant: Some(variant),
            model: &model,
            arm: &arm,
            installed_version,
            home: &source,
            wall: skein_lib::Wall::from_nanos(100_000_000),
            lifetime: Duration::from_secs(200),
            environment: &[],
        };
        let mut prepared = smith_bench::agent::smith::prepare(&context).expect("actual native renderer");
        let endpoint = prepared.rendered["agent"]["endpoints"][0].as_object_mut().expect("fixture endpoint");
        endpoint.insert("address".into(), serde_json::json!("127.0.0.1:443"));
        endpoint.insert("transport".into(), serde_json::json!("plaintext"));
        endpoint.remove("server_name");
        let settings_path = private.root.join("settings.json");
        std::fs::write(&settings_path, serde_json::to_vec(&prepared.rendered).expect("rendered JSON"))
            .expect("private settings");
        let resolved = native_product(&context);
        let local = resolved.local;
        let configured = resolved.agent;
        if variant.name == "timeout" {
            assert_eq!(configured.service.limits.domain.session.tools.tool_deadline, Duration::from_secs(1));
            assert_eq!(configured.service.limits.domain.session.tools.shell_timeout, Duration::from_secs(1));
            assert_eq!(configured.service.limits.machine.group_stop, Duration::from_secs(1));
        }
        let allowances = configured
            .service
            .domain
            .models
            .iter()
            .map(|model| model.protocol_allowance)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        assert_eq!(local.models.len(), 2);
        assert_eq!(allowances.len(), 2);
        local.validate(&resolved.limits).expect("actual run policy admitted");
        match prepared.credential {
            Credential::Record(Some(borrowed)) => {
                assert!(borrowed.finish(&guard).expect("never executed").source_unchanged);
            }
            Credential::Record(None) => panic!("synthetic credential prepared"),
        }
        std::fs::remove_dir_all(private.root.join("smith")).expect("reset unused preparation");
    }
    std::fs::remove_dir_all(directory).expect("private fixture cleanup");
}

#[test]
fn process_tree_profile_renders_and_passes_the_actual_public_startup_derivation() {
    use smith_bench::agent::Context;
    use smith_bench::arm::ArmRef;
    let directory = root("process-tree-settings");
    let source = directory.join("user/.codex");
    std::fs::create_dir_all(&source).expect("synthetic source");
    std::fs::write(source.join("auth.json"), unissued_auth()).expect("unissued credential");
    let guard = smith_bench::guard::Guard::new(
        &directory.join("user"),
        &directory.join("config"),
        &directory.join("state"),
        &[],
    )
    .expect("fixture guard");
    let private = smith_bench::isolate::empty(&guard, &directory.join("scratch")).expect("private fixture");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task =
        smith_bench::check_task(&project.join("tasks/probes/process-tree/task.toml"), &project.join("../docs/design"))
            .expect("frozen task");
    let pin_file = project.join("agents/smith/first-request.pin.toml");
    let pin = smith_bench::read_configuration(&pin_file).expect("native pin");
    let arm = ArmRef::Built(unexecuted_arm(&directory, project.parent().expect("repository")));
    let installed_version = match &arm {
        ArmRef::Built(build) => &build.arm.commit,
        ArmRef::Installed { .. } => unreachable!("fixture arm"),
    };
    let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
    {
        let variant = None;
        let context = Context {
            guard: &guard,
            private: &private,
            pin: &pin,
            pin_file: &pin_file,
            task: &task,
            variant,
            model: &model,
            arm: &arm,
            installed_version,
            home: &source,
            wall: skein_lib::Wall::from_nanos(100_000_000),
            lifetime: Duration::from_secs(200),
            environment: &[],
        };
        let prepared = smith_bench::agent::smith::prepare(&context).expect("actual native renderer");
        let settings_path = private.root.join("settings.json");
        std::fs::write(&settings_path, serde_json::to_vec(&prepared.rendered).expect("rendered JSON"))
            .expect("private settings");
        let resolved = native_product(&context);
        let local = resolved.local;
        let configured = resolved.agent;
        {
            assert_eq!(configured.service.limits.domain.session.tools.tool_deadline, Duration::from_secs(1));
            assert_eq!(configured.service.limits.domain.session.tools.shell_timeout, Duration::from_secs(1));
            assert_eq!(configured.service.limits.machine.group_stop, Duration::from_secs(1));
        }
        let allowances = configured
            .service
            .domain
            .models
            .iter()
            .map(|model| model.protocol_allowance)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        assert_eq!(local.models.len(), 2);
        assert_eq!(allowances.len(), 2);
        local.validate(&resolved.limits).expect("actual run policy admitted");
        match prepared.credential {
            Credential::Record(Some(borrowed)) => {
                assert!(borrowed.finish(&guard).expect("never executed").source_unchanged);
            }
            Credential::Record(None) => panic!("synthetic credential prepared"),
        }
        std::fs::remove_dir_all(private.root.join("smith")).expect("reset unused preparation");
    }
    std::fs::remove_dir_all(directory).expect("private fixture cleanup");
}

#[test]
fn slow_completion_profile_renders_and_passes_the_actual_public_startup_derivation() {
    use smith_bench::agent::Context;
    use smith_bench::arm::ArmRef;
    let directory = root("slow-completion-settings");
    let source = directory.join("user/.codex");
    std::fs::create_dir_all(&source).expect("synthetic source");
    let mut auth: serde_json::Value = serde_json::from_slice(&unissued_auth()).expect("synthetic credential");
    let claims = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, b"{\"exp\":5000}");
    auth["tokens"]["access_token"] = serde_json::json!(format!("header.{claims}.signature"));
    std::fs::write(source.join("auth.json"), serde_json::to_vec(&auth).expect("fixture"))
        .expect("unissued credential lasts the authored offline lifetime");
    let guard = smith_bench::guard::Guard::new(
        &directory.join("user"),
        &directory.join("config"),
        &directory.join("state"),
        &[],
    )
    .expect("fixture guard");
    let private = smith_bench::isolate::empty(&guard, &directory.join("scratch")).expect("private fixture");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(
        &project.join("tasks/probes/slow-completion/task.toml"),
        &project.join("../docs/design"),
    )
    .expect("frozen task");
    let pin_file = project.join("agents/smith/first-request.pin.toml");
    let pin = smith_bench::read_configuration(&pin_file).expect("native pin");
    let arm = ArmRef::Built(unexecuted_arm(&directory, project.parent().expect("repository")));
    let installed_version = match &arm {
        ArmRef::Built(build) => &build.arm.commit,
        ArmRef::Installed { .. } => unreachable!("fixture arm"),
    };
    let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("high".into()) };
    {
        let variant = None;
        let context = Context {
            guard: &guard,
            private: &private,
            pin: &pin,
            pin_file: &pin_file,
            task: &task,
            variant,
            model: &model,
            arm: &arm,
            installed_version,
            home: &source,
            wall: skein_lib::Wall::from_nanos(100_000_000),
            lifetime: Duration::from_secs(1000),
            environment: &[],
        };
        let prepared = smith_bench::agent::smith::prepare(&context).expect("actual native renderer");
        let settings_path = private.root.join("settings.json");
        std::fs::write(&settings_path, serde_json::to_vec(&prepared.rendered).expect("rendered JSON"))
            .expect("private settings");
        let resolved = native_product(&context);
        let local = resolved.local;
        let configured = resolved.agent;
        assert_eq!(local.progress[0].head, Duration::from_millis(60000));
        assert_eq!(local.progress[0].idle, Duration::from_millis(300_000));
        assert_eq!(configured.declarations.endpoints[0].models[0].reasoning_item, 65536);
        assert_eq!(
            configured.declarations.endpoints[0].models[0].oversized_reasoning,
            smith_agent_service::profile::OversizedReasoning::Fail
        );
        assert_eq!(prepared.rendered["effort"], "high");
        let allowances = configured
            .service
            .domain
            .models
            .iter()
            .map(|model| model.protocol_allowance)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        assert_eq!(local.models.len(), 2);
        assert_eq!(allowances.len(), 2);
        local.validate(&resolved.limits).expect("actual run policy admitted");
        match prepared.credential {
            Credential::Record(Some(borrowed)) => {
                assert!(borrowed.finish(&guard).expect("never executed").source_unchanged);
            }
            Credential::Record(None) => panic!("synthetic credential prepared"),
        }
        std::fs::remove_dir_all(private.root.join("smith")).expect("reset unused preparation");
    }
    std::fs::remove_dir_all(directory).expect("private fixture cleanup");
}

#[path = "smith_adapter/exec.rs"]
mod exec;

#[test]
fn an_exec_arm_reads_its_prompt_to_eof_without_a_trace_file() {
    let fixture = exec::ExecFixture::new("exec-eof", 0);
    let context = fixture.context();
    let prepared = smith_bench::agent::smith::prepare(&context).expect("checked Exec");
    assert!(prepared.trace.is_none());
    let invocation = prepared.command();
    let result = execute(
        &Clock::new(),
        Command {
            program: invocation.program,
            arguments: invocation.arguments,
            environment: context.environment.to_vec(),
            directory: context.private.workspace.clone(),
        },
        b"read this prompt to EOF\n".as_slice().into(),
        Capture::Agent(Box::new(smith_bench::agent::Observing::Smith(Observing::new(prepared).expect("observer")))),
        bounds(),
    )
    .expect("Exec pipes settle");
    assert_eq!(result.exit, smith_bench::Exit::Code(0));
    assert_eq!(result.forced, Forced::No);
    assert!(result.error.is_none() && result.tree.live_pids.is_empty());
    fixture.clean();
}

#[test]
fn the_current_source_arm_selects_the_exec_face() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = project.parent().expect("repository");
    let output = std::process::Command::new("git")
        .args(["-C", repository.to_str().expect("path"), "rev-parse", "HEAD"])
        .output()
        .expect("current source identity");
    assert!(output.status.success());
    let commit = String::from_utf8(output.stdout).expect("commit").trim().to_owned();
    let arm = smith_bench::arm::BuiltArm {
        version: 1,
        commit,
        sha256: "0".repeat(64),
        toolchain: "selection only".into(),
        binaries: vec![],
    };
    let entries = smith_bench::agent::smith::read_faces(&project.join("agents/smith/faces.toml")).expect("registry");
    let selected = smith_bench::agent::smith::face(&arm, &entries, repository).expect("source face");
    assert_eq!(selected.shape, smith_bench::agent::smith::events::Shape::Chat);
    assert_eq!(selected.credential(), smith_bench::agent::smith::CredentialShape::Borrowed);
}

/// Resolve the authored pin through the product's settings and native startup owners.
fn native_product(context: &smith_bench::agent::Context<'_>) -> smith_local_shell::settings::resolve::Resolved {
    let path = context.pin_file.parent().expect("product fixture").join(&context.pin.pin.configuration);
    let mut base: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).expect("product fixture")).expect("product fixture");
    base["agent"]["endpoints"][0]["address"] = serde_json::json!("127.0.0.1:443");
    let mut task = context.task.clone();
    let mut variant = context.variant.cloned();
    for fields in std::iter::once(&mut task.smith).chain(variant.iter_mut().map(|variant| &mut variant.smith)) {
        if fields.get("group_stop").and_then(toml::Value::as_integer) == Some(250) {
            fields.insert("group_stop".into(), toml::Value::Integer(1000));
        }
    }
    let context = smith_bench::agent::Context { task: &task, variant: variant.as_ref(), ..*context };
    let product = smith_bench::agent::smith::product_settings(&context, &base, &context.private.root.join("auth.json"))
        .expect("product renderer");
    resolve_product(product, &context.private.root)
}

fn resolve_product(
    mut document: serde_json::Value,
    workspace: &std::path::Path,
) -> smith_local_shell::settings::resolve::Resolved {
    omit_null(&mut document);
    let value: toml::Value = serde_json::from_value(document).expect("product TOML values");
    let source = smith_local_shell::settings::Source {
        layer: smith_local_shell::settings::Layer::User,
        path: workspace.join("fixture.toml"),
        text: toml::to_string(&value).expect("product fixture"),
    };
    let settings = smith_local_shell::settings::from_sources(
        &smith_local_shell::command::Shared::default(),
        workspace.into(),
        &[source],
    )
    .unwrap_or_else(|why| panic!("{}", why.line()));
    smith_local_shell::settings::resolve::resolve_startup(&settings).unwrap_or_else(|why| panic!("{}", why.line())).0
}

fn omit_null(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.retain(|_, value| !value.is_null());
            for value in object.values_mut() {
                omit_null(value);
            }
        }
        serde_json::Value::Array(array) => {
            for value in array {
                omit_null(value);
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}
