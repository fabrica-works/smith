//! Native preparation and trace replay use unissued credentials and immutable fixtures.

pub mod common;

use std::path::{Path, PathBuf};

use skein_lib::Duration;
use skein_shell::Clock;
use skein_world::end_to_end::Command;
use smith_bench::agent::smith::{FaceSince, Observing, Prepared};
use smith_bench::process::{Bounds, Capture, execute};
use smith_bench::{End, Forced, Measure};

#[path = "smith_adapter/retried.rs"]
mod retried;

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
    let face = smith_bench::agent::smith::events::Shape::Chat;
    Observing::new(Prepared {
        face,
        settings: trace.with_extension("settings"),
        trace: Some(trace.into()),
        registry: FaceSince {
            commit: "30259d89b88834ed3edaaa08b3f320d2252bbb9a".into(),
            shape: face,
            receiving: serde_json::from_value(serde_json::json!({"string":524_288,"content":4096,"items":32,"encoded_line":16384})).expect("fixture bounds"),
        },
        rendered: serde_json::json!({"models":[{"input_price":10,"cached_price":1,"output_price":80,"price_unit":8_000_000}]}),
        program: "/bin/sh".into(),
        state: trace.with_extension("state"),
        serial: None,
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
    let face = smith_bench::agent::smith::events::Shape::Chat;
    let prepared = Prepared {
        face,
        settings: directory.join("settings.json"),
        trace: Some(trace.clone()),
        registry: FaceSince {
            commit: "30259d89b88834ed3edaaa08b3f320d2252bbb9a".into(),
            shape: face,
            receiving: serde_json::from_value(serde_json::json!({
                "string":524_288,"content":4096,"items":32,
                "encoded_line":records.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("record lines")
            })).expect("writer-derived receiving ceiling"),
        },
        rendered: serde_json::json!({}),
        program: "/bin/sh".into(),
        state: directory.join("state"),
        serial: None,
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
    let failed = common::events::record(include_str!("../../crates/smith-events/golden/v4/answer_class_budget.jsonl"));
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
    prepared.face = smith_bench::agent::smith::events::Shape::Chat;
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
        common::events::record(include_str!("../../crates/smith-events/golden/v4/tool_started_everything.jsonl"));
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
fn path_mounts_reach_native_policy_and_escaped_mounts_refuse_before_credentials() {
    let fixture = exec::ExecFixture::new("path-settings", 0);
    let mut task = authored_task("paths");
    let context = fixture.context();
    let private = context.private;
    for name in ["app", "spec"] {
        std::fs::create_dir(private.workspace.join(name)).expect("mount fixture");
    }
    for variant in [None, Some(&task.variant[0])] {
        let context = smith_bench::agent::Context { task: &task, variant, ..context };
        let resolved = resolved_product(&context);
        let mounts = &resolved.local.workspace.as_ref().expect("workspace").directories;
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
    }
    task.smith.remove("workspace.directories");
    let resolved = resolved_product(&smith_bench::agent::Context { task: &task, ..context });
    let mounts = &resolved.local.workspace.as_ref().expect("default workspace").directories;
    assert_eq!(mounts.len(), 1);
    assert_eq!(mounts[0].host.as_ref(), private.workspace.join("app").as_os_str().as_encoded_bytes());
    mount_refusals(&context, &mut task);
    assert!(context.home.join("tokens/chatgpt").is_file());
    fixture.clean();
}

fn mount_refusals(context: &smith_bench::agent::Context<'_>, task: &mut smith_bench::Task) {
    let outside = context.private.root.join("outside");
    std::fs::create_dir(&outside).expect("owned outside fixture");
    std::os::unix::fs::symlink(&outside, context.private.workspace.join("escaped")).expect("escape fixture");
    std::fs::write(context.private.workspace.join("app/input.txt"), []).expect("file mount fixture");
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
    for row in refusals {
        let value: toml::Value = toml::from_str(&format!("directories = {row}")).expect("authored TOML");
        task.smith.insert("workspace.directories".into(), value["directories"].clone());
        let context = smith_bench::agent::Context { task, home: &outside, ..*context };
        let refusal = product_arguments(&context).expect_err("refusal before login");
        assert_eq!(refusal.step, smith_bench::isolate::SetupStep::Configuration, "{row}");
        assert!(!context.private.root.join("smith/tokens").exists());
    }
}

#[test]
fn every_probe_tasks_overrides_pass_the_actual_public_startup_derivation() {
    let fixture = exec::ExecFixture::new("all-probe-settings", 0);
    for name in ["app", "spec"] {
        std::fs::create_dir_all(fixture.context().private.workspace.join(name)).expect("probe mount");
    }
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tasks/probes");
    let mut paths = std::fs::read_dir(directory)
        .expect("probe tasks")
        .map(|entry| entry.expect("probe directory").path().join("task.toml"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    paths.sort();
    assert!(!paths.is_empty());
    for path in paths {
        let task = smith_bench::read_task(&path).expect("authored probe");
        let context = smith_bench::agent::Context { task: &task, ..fixture.context() };
        let resolved = resolved_product(&context);
        resolved.local.validate(&resolved.limits).expect("probe product policy");
    }
    fixture.clean();
}

#[test]
fn tool_output_profiles_render_and_pass_the_actual_public_startup_derivation() {
    let fixture = exec::ExecFixture::new("tool-output-settings", 0);
    let task = authored_task("tool-output");
    for variant in &task.variant {
        let context = smith_bench::agent::Context { task: &task, variant: Some(variant), ..fixture.context() };
        let resolved = resolved_product(&context);
        if variant.name == "timeout" {
            assert_eq!(resolved.agent.service.limits.domain.session.tools.tool_deadline, Duration::from_secs(1));
            assert_eq!(resolved.agent.service.limits.domain.session.tools.shell_timeout, Duration::from_secs(1));
            assert_eq!(resolved.agent.service.limits.machine.group_stop, Duration::from_secs(1));
        }
        assert_eq!(resolved.local.models.len(), 3);
        assert_eq!(resolved.agent.service.domain.models.len(), 3);
        resolved.local.validate(&resolved.limits).expect("mounted product policy");
    }
    fixture.clean();
}

#[test]
fn process_tree_profile_renders_and_passes_the_actual_public_startup_derivation() {
    let fixture = exec::ExecFixture::new("process-tree-settings", 0);
    let task = authored_task("process-tree");
    let context = smith_bench::agent::Context { task: &task, ..fixture.context() };
    let resolved = resolved_product(&context);
    assert_eq!(resolved.agent.service.limits.domain.session.tools.tool_deadline, Duration::from_secs(1));
    assert_eq!(resolved.agent.service.limits.domain.session.tools.shell_timeout, Duration::from_secs(1));
    assert_eq!(resolved.agent.service.limits.machine.group_stop, Duration::from_secs(1));
    assert_eq!(resolved.local.models.len(), 3);
    assert_eq!(resolved.agent.service.domain.models.len(), 3);
    resolved.local.validate(&resolved.limits).expect("product policy");
    fixture.clean();
}

#[test]
fn slow_completion_profile_renders_and_passes_the_actual_public_startup_derivation() {
    let fixture = exec::ExecFixture::new("slow-completion-settings", 0);
    let task = authored_task("slow-completion");
    let context = smith_bench::agent::Context { task: &task, ..fixture.context() };
    let resolved = resolved_product(&context);
    assert_eq!(resolved.local.progress[0].head, Duration::from_millis(60000));
    assert_eq!(resolved.local.progress[0].idle, Duration::from_millis(300_000));
    assert_eq!(resolved.agent.declarations.endpoints[0].models[0].reasoning_item, 65536);
    assert_eq!(
        resolved.agent.declarations.endpoints[0].models[0].oversized_reasoning,
        smith_agent_service::profile::OversizedReasoning::Fail
    );
    assert_eq!(resolved.local.models.len(), 3);
    assert_eq!(resolved.agent.service.domain.models.len(), 3);
    resolved.local.validate(&resolved.limits).expect("product policy");
    fixture.clean();
}

fn authored_task(name: &str) -> smith_bench::Task {
    smith_bench::read_task(&Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tasks/probes/{name}/task.toml")))
        .expect("authored task")
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
fn a_binary_older_than_the_product_face_is_refused() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = project.parent().expect("repository");
    let arm = smith_bench::arm::BuiltArm {
        version: 1,
        commit: "4d06c1106b45b12280bad141cacb2806c514fe26".into(),
        sha256: "0".repeat(64),
        toolchain: "selection only".into(),
        binaries: vec![],
    };
    let entries = smith_bench::agent::smith::read_faces(&project.join("agents/smith/faces.toml")).expect("registry");
    let refusal = smith_bench::agent::smith::face(&arm, &entries, repository).expect_err("pre-product arm");
    assert_eq!(refusal.step, smith_bench::isolate::SetupStep::Configuration);
    assert_eq!(
        refusal.reason,
        "Smith arm 4d06c1106b45b12280bad141cacb2806c514fe26 predates the first supported product face"
    );
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
}

fn product_rendered(context: &smith_bench::agent::Context<'_>) -> serde_json::Value {
    let path = context.pin_file.parent().expect("pin directory").join(&context.pin.pin.configuration);
    let base: toml::Value = toml::from_str(&std::fs::read_to_string(path).expect("user layer")).expect("TOML");
    smith_bench::agent::smith::product_settings(context, &serde_json::to_value(base).expect("layer"), context.home)
        .expect("product layer")
}

fn product_arguments(context: &smith_bench::agent::Context<'_>) -> Result<Vec<String>, smith_bench::isolate::Setup> {
    let path = context.pin_file.parent().expect("pin directory").join(&context.pin.pin.configuration);
    let base: toml::Value = toml::from_str(&std::fs::read_to_string(path).expect("user layer")).expect("TOML");
    let rendered = smith_bench::agent::smith::product_settings(
        context,
        &serde_json::to_value(base).expect("layer"),
        context.home,
    )?;
    Ok(rendered["invocation"]
        .as_array()
        .expect("prepared flags")
        .iter()
        .map(|value| value.as_str().expect("argument").to_owned())
        .collect())
}

fn product_layer(context: &smith_bench::agent::Context<'_>) -> serde_json::Value {
    let mut layer = product_rendered(context);
    layer.as_object_mut().expect("layer").remove("invocation");
    layer
}

fn product_sources(context: &smith_bench::agent::Context<'_>) -> smith_local_shell::settings::Settings {
    let value: toml::Value = serde_json::from_value(product_layer(context)).expect("layer values");
    let source = smith_local_shell::settings::Source {
        layer: smith_local_shell::settings::Layer::User,
        path: context.private.config.join("smith/config.toml"),
        text: toml::to_string(&value).expect("layer TOML"),
    };
    let arguments = product_arguments(context).expect("product arguments");
    let command = smith_local_shell::command::parse(arguments.into_iter().map(Into::into).collect()).expect("flags");
    let smith_local_shell::command::Command::Interactive { shared, .. } = command else { panic!("shared flags") };
    smith_local_shell::settings::from_sources(&shared, shared.directory.clone().expect("workspace flag"), &[source])
        .unwrap_or_else(|why| panic!("{}", why.line()))
}

fn resolved_product(context: &smith_bench::agent::Context<'_>) -> smith_local_shell::settings::resolve::Resolved {
    let mut settings = product_sources(context);
    let trust = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/protocol-llm/fixtures/root.der");
    // These startup stories open no TLS connection. Keep every native quantity
    // and memory check, loading the bounded test root instead of the host bundle.
    for endpoint in ["codex", "anthropic", "openai"] {
        for (field, value) in [
            ("system", toml::Value::Boolean(false)),
            ("der", toml::Value::Array(vec![toml::Value::String(trust.display().to_string())])),
        ] {
            settings.values.insert(
                format!("endpoints.{endpoint}.trust.{field}"),
                smith_local_shell::settings::Sourced { value, origin: smith_local_shell::settings::Origin::BuiltIn },
            );
        }
    }
    smith_local_shell::settings::resolve::resolve_startup(&settings).unwrap_or_else(|why| panic!("{}", why.line())).0
}

#[test]
fn every_smith_pin_is_a_product_settings_layer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("agents/smith");
    for name in ["standard", "everything"] {
        let pin = smith_bench::read_configuration(&root.join(format!("{name}.pin.toml"))).expect("pin");
        assert_eq!(pin.pin.configuration, PathBuf::from(format!("{name}.toml")));
        let layer: toml::Value =
            toml::from_str(&std::fs::read_to_string(root.join(&pin.pin.configuration)).expect("TOML layer"))
                .expect("product TOML");
        assert_eq!(layer.as_table().expect("layer").keys().map(String::as_str).collect::<Vec<_>>(), ["trace"]);
        assert_eq!(layer["trace"]["capture"].as_str(), Some(if name == "standard" { "calls" } else { "everything" }));
    }
}

#[test]
fn a_time_breakdown_sums_to_task_wall_within_one_second() {
    let limits = smith_events::Limits { string: 524_288, content: 4096, items: 512 };
    let mut observer =
        smith_bench::agent::smith::events::Observer::with_ceiling(&limits, 1_048_576).expect("recorded stream bounds");
    observer.feed(include_bytes!("recorded/smith-time.jsonl")).expect("recorded smith trace");
    observer.finish().expect("complete trace");
    let mut timing = smith_bench::parse_result(Path::new("fixture"), include_str!("recorded/results/attempt.json"))
        .expect("result fixture")
        .timing;
    observer.timing(&mut timing);
    let record = &timing.record;
    let value = |measure: &Measure<u64>| match measure {
        Measure::Observed { value } => *value,
        other @ (Measure::LowerBound { .. } | Measure::Unavailable { .. }) => panic!("missing trace span: {other:?}"),
    };
    let sum = value(&record.requests_ms)
        + value(&record.compaction_ms)
        + value(&record.children_ms)
        + value(&record.rest_ms)
        + record.tools_ms.iter().map(|(_, span)| value(span)).sum::<u64>();
    assert!(sum.abs_diff(764_602) < 1000, "recorded task wall: {sum}");
    assert!((380_000..381_000).contains(&value(&record.requests_ms)));
    assert!((93_000..94_000).contains(&value(&record.compaction_ms)));
    assert!((32_000..33_000).contains(&value(&timing.first_byte_ms)));
    assert_eq!(timing.first_byte_ms, record.first_byte_ms);
    assert_eq!(timing.largest_gap_ms, record.largest_gap_ms);
    assert!(value(&timing.longest_completion_ms) > 10_000);
}

#[test]
fn a_task_override_reaches_smith_as_a_product_setting() {
    let fixture = exec::ExecFixture::new("task-setting", 0);
    let mut task = authored_task("first-request");
    task.smith.insert("models.\"{model}\".output".into(), toml::Value::Integer(8192));
    let context = smith_bench::agent::Context { task: &task, ..fixture.context() };
    let arguments = product_arguments(&context).expect("overrides");
    assert!(arguments.contains(&"models.\"openai/gpt-6-luna\".output=8192".into()));
    assert_eq!(resolved_product(&context).agent.declarations.endpoints[0].models[0].output, 8192);
    fixture.clean();
}

#[test]
fn a_comparison_pin_keeps_every_preset_value_and_the_default_role() {
    let fixture = exec::ExecFixture::new("comparison-settings", 0);
    let mut task = authored_task("first-request");
    task.smith.clear();
    task.budget.turns = None;
    task.budget.seconds = None;
    task.budget.spend = None;
    let context = smith_bench::agent::Context { task: &task, ..fixture.context() };
    let settings = product_sources(&context);
    for (key, value) in &settings.values {
        if key.starts_with("models.")
            || (key.starts_with("endpoints.") && key != "endpoints.openai.account")
            || key.starts_with("deployment.")
            || key == "instructions"
        {
            assert!(
                !matches!(
                    value.origin,
                    smith_local_shell::settings::Origin::File { layer: smith_local_shell::settings::Layer::User, .. }
                ),
                "{key}"
            );
        }
    }
    let recorded: serde_json::Value =
        serde_json::from_str(include_str!("recorded/smith-product-check.json")).expect("recorded product check");
    assert_eq!(recorded["version"], 2);
    for (key, leaf) in recorded["settings"]["values"].as_object().expect("recorded leaves") {
        if key.starts_with("models.")
            || (key.starts_with("endpoints.") && key != "endpoints.openai.account")
            || key.starts_with("deployment.")
            || key == "instructions"
        {
            assert_eq!(leaf["origin"]["kind"], "built_in", "{key}");
            assert_eq!(
                serde_json::to_value(&settings.values[key].value).expect("setting value"),
                leaf["value"],
                "{key}"
            );
        }
    }
    let resolved = resolved_product(&context);
    assert!(!resolved.local.instructions.is_empty());
    fixture.clean();
}

#[test]
fn a_task_without_a_budget_renders_the_products_defaults() {
    let fixture = exec::ExecFixture::new("default-budget", 0);
    let mut task = authored_task("first-request");
    task.smith.remove("budget.reserve.seconds");
    task.budget.turns = None;
    task.budget.seconds = None;
    task.budget.spend = None;
    let context = smith_bench::agent::Context { task: &task, ..fixture.context() };
    let settings = product_sources(&context);
    for (key, expected) in [("budget.turns", 512), ("budget.seconds", 3600)] {
        assert_eq!(settings.values[key].value.as_integer(), Some(expected));
        assert!(matches!(settings.values[key].origin, smith_local_shell::settings::Origin::BuiltIn));
    }
    assert!(
        !product_arguments(&context).expect("default flags").iter().any(|argument| argument.starts_with("budget."))
    );
    fixture.clean();
}
