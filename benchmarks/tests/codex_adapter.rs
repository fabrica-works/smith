//! The named-thread collector reads only this attempt and preserves every gap.

use std::path::PathBuf;

use rusqlite::Connection;
use smith_bench::agent::codex::{ChildThread, Link, Rollouts, collect_rollouts, parse_rollout};
use smith_bench::{Measure, ScopeKind};

fn rollout(thread: &str, parent: Option<&str>) -> Vec<u8> {
    let metadata = serde_json::json!({"type":"session_meta", "payload":{"id":thread, "parent_thread_id":parent}});
    let usage = serde_json::json!({"type":"token_usage_record", "payload":{"thread_id":thread, "response_id":format!("response-{thread}"),
        "usage":{"input_tokens":20,"cached_input_tokens":5,"cache_write_input_tokens":0,"output_tokens":8,
        "reasoning_output_tokens":3,"total_tokens":28}}});
    format!("{metadata}\n{usage}\n{usage}\n").into_bytes()
}

#[test]
fn immutable_response_ids_deduplicate_without_adding_turn_totals() {
    let (_, responses) = parse_rollout(&rollout("root", None), "root").expect("pinned shape");
    assert_eq!(responses.len(), 1);
    let (link, child_usage) = parse_rollout(&rollout("child", Some("root")), "child").expect("linked child");
    let records = Rollouts {
        responses,
        children: vec![ChildThread { thread: "child".into(), link, responses: child_usage }],
        files: Vec::new(),
    }
    .token_records("root")
    .expect("scoped ledger");
    let total = records.iter().find(|record| record.role == ScopeKind::Total).expect("total");
    assert_eq!(total.usage.fresh, Measure::Observed { value: 30 });
    assert_eq!(total.usage.output, Measure::Observed { value: 16 });
    assert_eq!(total.usage.reasoning, Measure::Observed { value: 6 });
}

#[test]
fn an_unlinked_child_keeps_its_scope_unavailable_and_the_total_a_lower_bound() {
    let (_, responses) = parse_rollout(&rollout("root", None), "root").expect("root");
    let (_, child_usage) = parse_rollout(&rollout("child", None), "child").expect("child");
    let records = Rollouts {
        responses,
        children: vec![ChildThread { thread: "child".into(), link: Link::Unlinked, responses: child_usage }],
        files: Vec::new(),
    }
    .token_records("root")
    .expect("honest ledger");
    let child = records.iter().find(|record| record.scope == "child").expect("unavailable child scope");
    assert_eq!(child.role, ScopeKind::UnlinkedChild);
    assert!(matches!(child.usage.fresh, Measure::Unavailable { .. }));
    let total = records.iter().find(|record| record.role == ScopeKind::Total).expect("total");
    assert!(matches!(total.usage.fresh, Measure::LowerBound { value: 15, .. }));
}

#[test]
fn known_usage_shape_changes_and_foreign_thread_records_refuse() {
    let text = String::from_utf8(rollout("root", None)).expect("JSON UTF-8");
    assert!(parse_rollout(text.replace("\"cached_input_tokens\":5,", "").as_bytes(), "root").is_err());
    assert!(parse_rollout(text.as_bytes(), "foreign").is_err());
    assert!(parse_rollout(text.replace("\"output_tokens\":8", "\"output_tokens\":-8").as_bytes(), "root").is_err());
    let conflicting = format!(
        "{text}{}",
        text.lines()
            .nth(1)
            .expect("usage")
            .replace("\"output_tokens\":8", "\"output_tokens\":9")
            .replace("\"total_tokens\":28", "\"total_tokens\":29")
    );
    assert!(parse_rollout(conflicting.as_bytes(), "root").is_err());
}

fn private(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("smith-codex-adapter-{}-{name}", std::process::id()))
}

#[test]
fn read_only_collection_follows_only_the_named_roots_descendants() {
    let home = private("named");
    std::fs::create_dir_all(home.join("sessions")).expect("private fixture");
    let database = home.join("state_5.sqlite");
    let connection = Connection::open(&database).expect("fixture database");
    connection.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY, rollout_path TEXT); CREATE TABLE thread_spawn_edges(parent_thread_id TEXT, child_thread_id TEXT);").expect("pinned schema");
    for (thread, parent) in [("root", None), ("child", Some("root")), ("foreign", None)] {
        let file = home.join("sessions").join(format!("{thread}.jsonl"));
        std::fs::write(&file, if thread == "foreign" { b"not JSON".to_vec() } else { rollout(thread, parent) })
            .expect("own fixture");
        connection
            .execute("INSERT INTO threads VALUES(?1,?2)", [thread, file.to_str().expect("fixture path")])
            .expect("thread row");
        if let Some(parent) = parent {
            connection.execute("INSERT INTO thread_spawn_edges VALUES(?1,?2)", [parent, thread]).expect("native edge");
        }
    }
    drop(connection);
    let before = std::fs::read(&database).expect("database snapshot");
    let collected = collect_rollouts(&home, "root").expect("named collection ignores foreign file");
    assert_eq!(collected.children.len(), 1);
    assert_eq!(collected.files.len(), 2);
    assert_eq!(collected.children[0].link, Link::Linked { parent: "root".into() });
    assert_eq!(std::fs::read(database).expect("unchanged database"), before);
    std::fs::remove_dir_all(home).expect("fixture cleanup");
}

#[test]
fn a_named_path_cannot_read_credentials_or_an_unrelated_file() {
    let home = private("path");
    std::fs::create_dir_all(&home).expect("fixture directory");
    let connection = Connection::open(home.join("state_5.sqlite")).expect("fixture database");
    connection.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY, rollout_path TEXT); CREATE TABLE thread_spawn_edges(parent_thread_id TEXT, child_thread_id TEXT);").expect("schema");
    let forbidden = home.join("auth.json");
    std::fs::write(&forbidden, "CREDENTIAL_CONTROL_SENTINEL").expect("unissued fixture sentinel");
    connection
        .execute("INSERT INTO threads VALUES(?1,?2)", ["root", forbidden.to_str().expect("path")])
        .expect("invalid native path");
    drop(connection);
    let error = collect_rollouts(&home, "root").expect_err("not a rollout directory");
    assert!(!error.to_string().contains("CREDENTIAL_CONTROL_SENTINEL"));
    std::fs::remove_dir_all(home).expect("fixture cleanup");
}

#[test]
fn the_pinned_adapter_refuses_version_drift_and_never_rewrites_user_configuration() {
    use smith_bench::agent::{Context, Prepared};
    use smith_bench::arm::ArmRef;
    use smith_bench::guard::Guard;
    use smith_bench::isolate::{SetupStep, empty};
    use smith_bench::{Agent, ModelChoice, read_configuration, read_task};

    let root = private("prepare");
    let home = root.join("user/.codex");
    std::fs::create_dir_all(&home).expect("fake user home");
    let configuration = b"model=\"user-choice\"\n[agents]\nmax_threads=99\n";
    std::fs::write(home.join("config.toml"), configuration).expect("unrelated user configuration");
    let guard = Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), std::slice::from_ref(&home))
        .expect("guard");
    let private = empty(&guard, &root.join("scratch")).expect("private scratch");
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pin_file = directory.join("agents/codex/standard.pin.toml");
    let pin = read_configuration(&pin_file).expect("committed Codex pin");
    let task = read_task(&directory.join("tests/recorded/tasks/valid/task.toml")).expect("sample task");
    let model = ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
    let arm = ArmRef::Installed { program: "/bin/false".into() };
    let mut context = Context {
        guard: &guard,
        private: &private,
        pin: &pin,
        pin_file: &pin_file,
        task: &task,
        variant: None,
        model: &model,
        arm: &arm,
        installed_version: "codex-cli 0.159.0",
        home: &home,
    };
    assert_eq!(Agent::Codex.prepare(&context).expect_err("version drift").step, SetupStep::Configuration);
    context.installed_version = "codex-cli 0.160.0";
    let prepared = Agent::Codex.prepare(&context).expect("pinned version");
    let Prepared::Codex(codex) = &prepared;
    assert!(matches!(codex.user_configuration, Measure::Observed { .. }));
    assert!(codex.overrides.contains(&("agents.max_threads".into(), "6".into())));
    let invocation = prepared.command();
    let arguments: Vec<_> = invocation.arguments.iter().map(|argument| argument.to_str().expect("UTF-8")).collect();
    assert!(arguments.contains(&"--ignore-user-config"));
    assert!(arguments.contains(&"--ignore-rules"));
    assert!(
        arguments
            .iter()
            .any(|argument| argument.starts_with("projects.") && argument.ends_with(".trust_level=\"trusted\""))
    );
    assert_eq!(std::fs::read(home.join("config.toml")).expect("unmodified config"), configuration);
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}
