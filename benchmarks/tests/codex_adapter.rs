//! The named-thread collector reads only this attempt and preserves every gap.

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

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
fn codex_refuses_later_person_lines_before_reading_a_login() {
    let root = private("person");
    std::fs::create_dir(&root).expect("fixture root");
    let guard = smith_bench::guard::Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[])
        .expect("guard");
    let scratch = smith_bench::isolate::empty(&guard, &root.join("scratch")).expect("private scratch");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pin_file = project.join("agents/codex/standard.pin.toml");
    let pin = smith_bench::read_configuration(&pin_file).expect("Codex pin");
    let mut task = smith_bench::read_task(&project.join("tests/recorded/tasks/valid/task.toml")).expect("task");
    task.person = vec![smith_bench::agent::Cue { when: smith_bench::agent::Trigger::Answer, text: "later\n".into() }];
    let arm = smith_bench::arm::ArmRef::Installed { program: "/bin/true".into() };
    let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
    let context = smith_bench::agent::Context {
        guard: &guard,
        private: &scratch,
        pin: &pin,
        pin_file: &pin_file,
        task: &task,
        variant: None,
        model: &model,
        arm: &arm,
        installed_version: "codex-cli 0.160.0",
        home: &root.join("missing-login"),
        wall: skein_lib::Wall::EPOCH,
        lifetime: skein_lib::Duration::from_secs(1),
        environment: &[],
    };
    let refusal = smith_bench::agent::codex::prepare(&context).expect_err("interactive task refused before login");
    assert_eq!(refusal.step, smith_bench::isolate::SetupStep::Configuration);
    assert!(refusal.reason.contains("later person lines"));
    assert!(!scratch.root.join("codex").exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

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

fn the_pinned_adapter_refuses_version_drift_and_isolates_the_user_home() {
    use smith_bench::agent::{Context, Prepared};
    use smith_bench::guard::Guard;
    use smith_bench::isolate::{SetupStep, empty};
    use smith_bench::{Agent, ModelChoice, read_task};

    let root = private("prepare");
    let home = root.join("logins/codex");
    std::fs::create_dir_all(&home).expect("fake user home");
    let configuration = b"model=\"user-choice\"\n";
    std::fs::write(home.join("config.toml"), configuration).expect("unrelated user configuration");
    let auth = synthetic_auth(500);
    std::fs::write(home.join("auth.json"), &auth).expect("synthetic source login");
    std::fs::write(home.join("AGENTS.md"), "UNRELATED_GLOBAL_INSTRUCTION").expect("global marker");
    let guard = Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), std::slice::from_ref(&home))
        .expect("guard");
    let private = empty(&guard, &root.join("scratch")).expect("private scratch");
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (pin_file, pin, arm) = codex_pin_fixture(&root);
    let task = read_task(&directory.join("tests/recorded/tasks/valid/task.toml")).expect("sample task");
    let model = ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
    let mut context = Context {
        environment: &[],
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
        wall: skein_lib::Wall::from_nanos(100_000_000_000),
        lifetime: skein_lib::Duration::from_secs(20),
    };
    assert_eq!(Agent::Codex.prepare(&context).expect_err("version drift").step, SetupStep::Configuration);
    context.installed_version = "codex-cli 0.160.0";
    let prepared = Agent::Codex.prepare(&context).expect("pinned version");
    let Prepared::Codex(codex) = &prepared else {
        panic!("Codex preparation");
    };
    assert_eq!(codex.home, private.root.join("codex"));
    assert_eq!(std::fs::read_dir(&codex.home).expect("private home").count(), 1);
    let proof = codex.borrowed.as_ref().expect("converted credentials").finish(&guard).expect("settled cleanup");
    assert!(proof.source_unchanged);
    assert!(!codex.credential.exists());
    assert_eq!(std::fs::read(home.join("auth.json")).expect("source unchanged"), auth);
    let invocation = prepared.command();
    let arguments: Vec<_> = invocation.arguments.iter().map(|argument| argument.to_str().expect("UTF-8")).collect();
    assert!(!arguments.contains(&"--ignore-user-config"));
    assert!(!arguments.contains(&"--ignore-rules"));
    let (_, projects) = codex.overrides.iter().find(|(key, _)| key == "projects").expect("whole trust table");
    let parsed: toml::Value = toml::from_str(&format!("projects={projects}")).expect("dynamic path TOML");
    assert_eq!(parsed["projects"][private.workspace.to_str().expect("path")]["trust_level"].as_str(), Some("trusted"));
    assert_eq!(std::fs::read(home.join("config.toml")).expect("unmodified config"), configuration);
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

fn synthetic_auth(expires: u64) -> Vec<u8> {
    use base64::Engine;
    let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("{{\"exp\":{expires}}}"));
    serde_json::to_vec(&serde_json::json!({"auth_mode":"chatgpt", "last_refresh":"2000-01-01T00:00:00Z",
        "tokens":{"id_token":"UNISSUED_ID", "access_token":format!("header.{claims}.signature"),
        "account_id":"UNISSUED_ACCOUNT", "refresh_token":"UNISSUED_REFRESH_MUST_NOT_COPY"}}))
    .expect("fixture")
}

fn access_only_credentials_are_private_and_never_copy_a_refresh_or_other_home_file() {
    use std::os::unix::fs::PermissionsExt;
    let root = private("credentials");
    std::fs::DirBuilder::new().mode(0o700).create(&root).expect("private fixture root");
    let source = root.join("logins/codex");
    std::fs::create_dir_all(&source).expect("fixture source");
    let original = synthetic_auth(500);
    std::fs::write(source.join("auth.json"), &original).expect("source auth");
    for name in ["AGENTS.md", "config.toml", "skills", "memories", "plugins"] {
        std::fs::write(source.join(name), "UNRELATED_HOME_MARKER").expect("source marker");
    }
    let guard = smith_bench::guard::Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[])
        .expect("guard");
    let home = root.join("attempt-codex");
    let borrowed = smith_bench::credential::codex(
        &guard,
        &source,
        &home,
        skein_lib::Wall::from_nanos(100_000_000_000),
        skein_lib::Duration::from_secs(20),
    )
    .expect("access-only conversion");
    assert_eq!(std::fs::metadata(&home).expect("home mode").permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(&borrowed.credential).expect("auth mode").permissions().mode() & 0o777, 0o600);
    assert_eq!(std::fs::read_dir(&home).expect("initial home").count(), 1);
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&borrowed.credential).expect("private auth")).expect("auth shape");
    assert_eq!(record["auth_mode"], "chatgpt");
    assert_eq!(record["tokens"]["refresh_token"], "");
    assert_eq!(record["tokens"]["id_token"], "UNISSUED_ID");
    assert_eq!(record["tokens"]["account_id"], "UNISSUED_ACCOUNT");
    assert_eq!(record["last_refresh"], "1970-01-01T00:01:40Z");
    assert!(!serde_json::to_string(&record).expect("record").contains("UNISSUED_REFRESH"));
    assert!(!format!("{borrowed:?}").contains("UNISSUED"));
    let proof = borrowed.finish(&guard).expect("after tree settlement");
    assert!(proof.source_unchanged && proof.credential_removed && proof.private_refresh_empty);
    assert_eq!(std::fs::read(source.join("auth.json")).expect("source unchanged"), original);
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

fn credentials_expiring_before_the_hard_deadline_refuse_before_creating_a_home() {
    let root = private("expiry");
    std::fs::DirBuilder::new().mode(0o700).create(&root).expect("private fixture root");
    let source = root.join("logins/codex");
    std::fs::create_dir_all(&source).expect("fixture source");
    std::fs::write(source.join("auth.json"), synthetic_auth(110)).expect("source auth");
    let guard = smith_bench::guard::Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[])
        .expect("guard");
    let home = root.join("attempt-codex");
    let refusal = smith_bench::credential::codex(
        &guard,
        &source,
        &home,
        skein_lib::Wall::from_nanos(100_000_000_000),
        skein_lib::Duration::from_secs(20),
    )
    .expect_err("lapsed token");
    assert_eq!(refusal.step, smith_bench::isolate::SetupStep::Credentials);
    assert!(refusal.reason.contains(&format!("run `CODEX_HOME={} codex` once", source.display())));
    assert!(!refusal.reason.contains("UNISSUED"));
    assert!(!home.exists());
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

fn changed_user_auth_is_reported_after_private_credentials_are_removed() {
    let root = private("changed-source");
    std::fs::DirBuilder::new().mode(0o700).create(&root).expect("private fixture root");
    let source = root.join("logins/codex");
    std::fs::create_dir_all(&source).expect("fixture source");
    std::fs::write(source.join("auth.json"), synthetic_auth(500)).expect("source auth");
    let guard = smith_bench::guard::Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[])
        .expect("guard");
    let borrowed = smith_bench::credential::codex(
        &guard,
        &source,
        &root.join("attempt-codex"),
        skein_lib::Wall::from_nanos(100_000_000_000),
        skein_lib::Duration::from_secs(20),
    )
    .expect("conversion");
    std::fs::write(source.join("auth.json"), synthetic_auth(501)).expect("user refresh control");
    assert!(borrowed.finish(&guard).expect_err("source changed").contains("stop before further inference"));
    assert!(!borrowed.credential.exists());
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

fn repeated_session_metadata_keeps_its_parent_and_refuses_conflicts() {
    let stream = rollout("root", Some("parent"));
    let text = String::from_utf8(stream).expect("UTF-8");
    let metadata = text.lines().next().expect("metadata");
    let repeated = format!("{text}{metadata}\n");
    assert_eq!(
        parse_rollout(repeated.as_bytes(), "root").expect("same identity"),
        parse_rollout(text.as_bytes(), "root").expect("original")
    );
    let conflicting = format!("{text}{}\n", metadata.replace("\"parent\"", "\"foreign\""));
    assert!(
        parse_rollout(conflicting.as_bytes(), "root")
            .expect_err("parent changed")
            .to_string()
            .contains("conflicting repeated metadata")
    );
}

fn malformed_named_rollouts_remain_available_for_archive_and_keep_the_cached_fault() {
    let home = private("raw-fault");
    std::fs::create_dir_all(home.join("sessions")).expect("private fixture");
    let file = home.join("sessions/root.jsonl");
    let raw = b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"root\"}}\n{invalid JSON}\n";
    std::fs::write(&file, raw).expect("known malformed rollout");
    let connection = Connection::open(home.join("state_5.sqlite")).expect("fixture database");
    connection.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY, rollout_path TEXT); CREATE TABLE thread_spawn_edges(parent_thread_id TEXT, child_thread_id TEXT);").expect("schema");
    connection
        .execute("INSERT INTO threads VALUES(?1,?2)", ["root", file.to_str().expect("path")])
        .expect("named path");
    drop(connection);
    let prepared = smith_bench::agent::Prepared::Codex(smith_bench::agent::codex::Prepared {
        overrides: Vec::new(),
        credential: home.join("auth.json"),
        expires: skein_lib::Wall::EPOCH,
        borrowed: None,
        home: home.clone(),
        final_message: home.join("final.txt"),
        model: smith_bench::ModelChoice { model: Some("offline".into()), effort: Some("low".into()) },
        program: "/bin/false".into(),
        workspace: home.clone(),
        teardown: skein_lib::Duration::ZERO,
    });
    let smith_bench::agent::Observing::Codex(mut observing) = prepared.observe().expect("prepared Codex observer")
    else {
        panic!("Codex observer");
    };
    observing.observer.observe_line(r#"{"type":"thread.started","thread_id":"root"}"#).expect("named root");
    let first = observing.collect().expect_err("known shape fault").what;
    assert!(first.contains("thread root"));
    let retained = observing.collected.as_ref().expect("raw retained");
    assert_eq!(retained.files.len(), 1);
    assert_eq!(retained.files[0].bytes, raw);
    assert_eq!(observing.collect().expect_err("cached fault").what, first);
    std::fs::remove_dir_all(home).expect("fixture cleanup");
}

fn a_childs_inherited_parent_header_never_replaces_the_childs_ownership() {
    let child = String::from_utf8(rollout("child", Some("root"))).expect("child");
    let inherited = r#"{"type":"session_meta","payload":{"id":"root","parent_thread_id":null}}"#;
    let stream = format!("{child}{inherited}\n");
    let (link, usage) = parse_rollout(stream.as_bytes(), "child").expect("native inherited context");
    assert_eq!(link, Link::Linked { parent: "root".into() });
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].response, "response-child");
    assert!(parse_rollout(stream.replace("\"id\":\"root\"", "\"id\":\"foreign\"").as_bytes(), "child").is_err());
}

fn captured_native_child_headers_and_usage_keep_each_response_in_its_own_scope() {
    let root = "01a121d2-5c3c-74b1-865b-66d8c6f41c3a";
    let child = "01a121d2-6e5a-7e30-abd0-133d08c8c960";
    let (_, responses) =
        parse_rollout(include_bytes!("recorded/codex/live-root-rollout.jsonl"), root).expect("actual root");
    let (link, child_responses) = parse_rollout(include_bytes!("recorded/codex/live-child-rollout.jsonl"), child)
        .expect("actual inherited context");
    assert_eq!(link, Link::Linked { parent: root.into() });
    assert_eq!(responses.len(), 4);
    assert_eq!(child_responses.len(), 2);
    let collected = Rollouts {
        responses,
        children: vec![ChildThread { thread: child.into(), link, responses: child_responses }],
        files: vec![
            smith_bench::agent::codex::RolloutFile {
                thread: root.into(),
                bytes: include_bytes!("recorded/codex/live-root-rollout.jsonl").to_vec(),
            },
            smith_bench::agent::codex::RolloutFile {
                thread: child.into(),
                bytes: include_bytes!("recorded/codex/live-child-rollout.jsonl").to_vec(),
            },
        ],
    };
    let tokens = collected.token_records(root).expect("actual scope totals");
    assert!(matches!(
        tokens.iter().find(|r| r.role == ScopeKind::Total).expect("total").usage.output,
        Measure::Observed { value: 186 }
    ));
    let mut counts = smith_bench::agent::codex::Observer::default().counts();
    collected.counts(root, &mut counts).expect("native counts");
    assert_eq!(counts.responses, Measure::Observed { value: 6 });
}

#[test]
fn pinned_adapters_usage_collection_and_private_credentials() {
    immutable_response_ids_deduplicate_without_adding_turn_totals();
    an_unlinked_child_keeps_its_scope_unavailable_and_the_total_a_lower_bound();
    known_usage_shape_changes_and_foreign_thread_records_refuse();
    read_only_collection_follows_only_the_named_roots_descendants();
    a_named_path_cannot_read_credentials_or_an_unrelated_file();
    the_pinned_adapter_refuses_version_drift_and_isolates_the_user_home();
    access_only_credentials_are_private_and_never_copy_a_refresh_or_other_home_file();
    credentials_expiring_before_the_hard_deadline_refuse_before_creating_a_home();
    changed_user_auth_is_reported_after_private_credentials_are_removed();
    repeated_session_metadata_keeps_its_parent_and_refuses_conflicts();
    malformed_named_rollouts_remain_available_for_archive_and_keep_the_cached_fault();
    a_childs_inherited_parent_header_never_replaces_the_childs_ownership();
    captured_native_child_headers_and_usage_keep_each_response_in_its_own_scope();
}

#[test]
fn paths_use_the_declared_working_directory_and_refuse_escaped_paths_before_credentials() {
    use smith_bench::agent::Context;
    use smith_bench::guard::Guard;
    use smith_bench::isolate::SetupStep;
    use std::os::unix::fs::PermissionsExt;

    let root = private("paths-prepare");
    let home = root.join("logins/codex");
    std::fs::create_dir_all(&home).expect("unissued source directory");
    let auth = synthetic_auth(500);
    std::fs::write(home.join("auth.json"), &auth).expect("unissued source record");
    let guard = Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[]).expect("guard");
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&project.join("tasks/probes/paths/task.toml"), &project.join("../docs/design"))
        .expect("frozen paths task");
    let private = smith_bench::isolate::prepare(
        &guard,
        &project.join("tasks/probes/paths/seed"),
        &task.seed_sha256,
        &root.join("scratch"),
    )
    .expect("actual seed materialisation");
    for command in &task.setup {
        assert!(
            std::process::Command::new(&command[0])
                .args(&command[1..])
                .current_dir(&private.workspace)
                .status()
                .expect("exact authored offline setup")
                .success()
        );
    }
    for path in [private.workspace.join("spec"), private.workspace.join("spec/rules.txt")] {
        assert_eq!(std::fs::metadata(path).expect("read-only layout").permissions().mode() & 0o222, 0);
    }
    let (pin_file, pin, arm) = codex_pin_fixture(&root);
    let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
    let context = Context {
        environment: &[],
        guard: &guard,
        private: &private,
        pin: &pin,
        pin_file: &pin_file,
        task: &task,
        variant: None,
        model: &model,
        arm: &arm,
        installed_version: "codex-cli 0.160.0",
        home: &home,
        wall: skein_lib::Wall::from_nanos(100_000_000_000),
        lifetime: skein_lib::Duration::from_secs(20),
    };
    for task in [&task, &task.with_variant("single").expect("single task")] {
        let context = Context { task, ..context };
        let prepared = smith_bench::agent::codex::prepare(&context).expect("actual Codex preparation, no invocation");
        assert_eq!(prepared.workspace, private.workspace.join("app"));
        let invocation = prepared.command();
        let index = invocation.arguments.iter().position(|argument| argument == "-C").expect("working directory flag");
        assert_eq!(invocation.arguments[index + 1], private.workspace.join("app").as_os_str());
        let (_, projects) = prepared.overrides.iter().find(|(name, _)| name == "projects").expect("trust table");
        let parsed: toml::Value = toml::from_str(&format!("projects={projects}")).expect("TOML trust table");
        assert_eq!(
            parsed["projects"][private.workspace.join("app").to_str().expect("path")]["trust_level"].as_str(),
            Some("trusted")
        );
        assert!(
            prepared
                .borrowed
                .as_ref()
                .expect("unissued conversion")
                .finish(&guard)
                .expect("unused cleanup")
                .source_unchanged
        );
        std::fs::remove_dir_all(private.root.join("codex")).expect("reset unused adapter");
    }
    let outside = root.join("outside");
    std::fs::create_dir(&outside).expect("owned outside fixture");
    std::os::unix::fs::symlink(&outside, private.workspace.join("escaped")).expect("owned escape fixture");
    for path in ["../outside", "/tmp", "absent", "app/input.txt", "escaped"] {
        let mut refused_task = task.clone();
        refused_task.working_directory = Some(path.into());
        let context = Context { task: &refused_task, home: &root.join("missing-source-login"), ..context };
        assert_eq!(
            smith_bench::agent::codex::prepare(&context).expect_err("working directory refused before login").step,
            SetupStep::Configuration
        );
        assert!(!private.root.join("codex/auth.json").exists());
    }
    assert_eq!(std::fs::read(home.join("auth.json")).expect("unissued source unchanged"), auth);
    smith_bench::isolate::remove_owned_tree(&guard, &private.root).expect("read-only owned layout cleaned");
    std::fs::remove_dir_all(root).expect("unissued source fixture cleaned");
}

fn recorded_time(bytes: &[u8]) -> smith_bench::Timing {
    let thread = "01a13785-a5a8-7543-ad07-90745b2ada69";
    let rollouts = Rollouts {
        responses: vec![],
        children: vec![],
        files: vec![smith_bench::agent::codex::RolloutFile { thread: thread.into(), bytes: bytes.into() }],
    };
    let mut timing = smith_bench::parse_result(Path::new("fixture"), include_str!("recorded/results/attempt.json"))
        .expect("result fixture")
        .timing;
    rollouts.timing(thread, &mut timing).expect("recorded timestamps");
    timing
}

#[test]
fn rollout_threads_give_request_tool_and_command_overlap() {
    let timing = recorded_time(include_bytes!("recorded/codex/repository-04-time.jsonl"));
    let record = &timing.record;
    let value = |measure: &Measure<u64>| match measure {
        Measure::Observed { value } => *value,
        other @ (Measure::LowerBound { .. } | Measure::Unavailable { .. }) => panic!("missing rollout span: {other:?}"),
    };
    assert!((388_000..389_000).contains(&value(&record.requests_ms)));
    let exec = record.tools_ms.iter().find(|(tool, _)| tool == "exec").expect("root exec calls");
    assert!((46_000..47_000).contains(&value(&exec.1)));
    assert!(value(&record.commands_outside_calls_ms) > 110_000);
    assert_eq!(record.compaction_ms, Measure::Observed { value: 0 });
    assert_eq!(record.children_ms, Measure::Observed { value: 0 });
    assert!(matches!(record.first_byte_ms, Measure::Unavailable { .. }));
    assert!(matches!(record.largest_gap_ms, Measure::Unavailable { .. }));
}

#[test]
fn a_stream_without_spans_leaves_the_record_unavailable() {
    for bytes in [b"".as_slice(), include_bytes!("recorded/codex/completed.jsonl").as_slice()] {
        let timing = recorded_time(bytes);
        assert_eq!(timing.record.requests_ms, Measure::unavailable("root rollout lacks complete timestamped spans"));
        assert!(matches!(timing.record.rest_ms, Measure::Unavailable { .. }));
        assert!(matches!(timing.record.commands_outside_calls_ms, Measure::Unavailable { .. }));
    }
}

#[test]
fn a_summary_keeps_the_time_records_medians() {
    let mut results = Vec::new();
    for value in [100_u64, 300, 200] {
        let mut result = smith_bench::parse_result(Path::new("fixture"), include_str!("recorded/results/attempt.json"))
            .expect("result fixture");
        result.identity.attempt = value.to_string();
        let record = &mut result.timing.record;
        record.requests_ms = Measure::Observed { value };
        record.first_byte_ms = Measure::Observed { value: value / 10 };
        record.largest_gap_ms = Measure::Observed { value: value / 2 };
        record.compaction_ms = Measure::Observed { value: 0 };
        record.children_ms = Measure::Observed { value: 0 };
        record.rest_ms = Measure::Observed { value: value / 5 };
        record.tools_ms = vec![("exec".into(), Measure::Observed { value: value / 4 })];
        results.push(result);
    }
    let summary = smith_bench::summarise(&results, "timing", "working", smith_bench::Design::Single, 17, None, None)
        .expect("summary");
    for (metric, expected) in [
        ("requests_ms", 200.0),
        ("first_byte_ms", 20.0),
        ("largest_gap_ms", 100.0),
        ("compaction_ms", 0.0),
        ("children_ms", 0.0),
        ("rest_ms", 40.0),
        ("tools/exec_ms", 50.0),
    ] {
        assert_eq!(
            summary.groups[0].metrics[&format!("time/{metric}")].median,
            Measure::Observed { value: expected },
            "{metric}"
        );
    }
    assert!(matches!(summary.groups[0].metrics["time/commands_outside_calls_ms"].median, Measure::Unavailable { .. }));
}

fn repository_threads() -> (String, Rollouts) {
    let root = "01a13785-a5a8-7543-ad07-90745b2ada69";
    let sources = [
        (root, include_bytes!("recorded/codex/repository-04-root.jsonl").as_slice()),
        (
            "01a13785-fa70-7333-8992-f27e7720f4fb",
            include_bytes!("recorded/codex/repository-04-child-one.jsonl").as_slice(),
        ),
        (
            "01a13786-12c9-7381-a0de-7de1cbc6eb1d",
            include_bytes!("recorded/codex/repository-04-child-two.jsonl").as_slice(),
        ),
    ];
    let mut rollouts = Rollouts { responses: vec![], children: vec![], files: vec![] };
    for (thread, bytes) in sources {
        // Supply the model context omitted by this usage-only projection.
        let context = b"{\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-6.1-sol\"}}\n";
        let bytes = [context.as_slice(), bytes].concat();
        let (link, responses) = parse_rollout(&bytes, thread).expect("archived native rollout");
        if thread == root {
            rollouts.responses = responses;
        } else {
            rollouts.children.push(ChildThread { thread: thread.into(), link, responses });
        }
        rollouts.files.push(smith_bench::agent::codex::RolloutFile { thread: thread.into(), bytes });
    }
    (root.into(), rollouts)
}

#[test]
fn linked_codex_children_are_counted_from_their_rollouts() {
    let (root, mut rollouts) = repository_threads();
    let child = &rollouts.children[0].thread;
    rollouts.children.push(ChildThread {
        thread: "grandchild".into(),
        link: Link::Linked { parent: child.clone() },
        responses: vec![],
    });
    rollouts.children.push(ChildThread {
        thread: "unrelated".into(),
        link: Link::Linked { parent: "another-root".into() },
        responses: vec![],
    });
    rollouts.children.push(ChildThread { thread: "unlinked".into(), link: Link::Unlinked, responses: vec![] });
    let mut counts = smith_bench::agent::codex::Observer::default().counts();
    rollouts.counts(&root, &mut counts).expect("native counts");
    assert_eq!(counts.children, Measure::Observed { value: 3 });
    assert_eq!(counts.conversations, Measure::Observed { value: 4 });
}

#[test]
fn codex_responses_per_thread_come_from_token_counts() {
    let (root, rollouts) = repository_threads();
    let counts = rollouts.responses_per_thread(&root).expect("native token-count records");
    for (thread, responses) in [(&root, 55), (&rollouts.children[0].thread, 30), (&rollouts.children[1].thread, 27)] {
        assert_eq!(counts[thread], Measure::Observed { value: responses }, "{thread}");
    }
    let mut total = smith_bench::agent::codex::Observer::default().counts();
    rollouts.counts(&root, &mut total).expect("counts");
    assert_eq!(total.responses, Measure::Observed { value: 112 });
}

fn codex_pin_fixture(root: &Path) -> (PathBuf, smith_bench::PinnedConfiguration, smith_bench::arm::ArmRef) {
    let directory = root.join("pin");
    std::fs::create_dir(&directory).expect("fixture pin directory");
    let metadata = include_str!("../agents/codex/standard.pin.toml").replace(
        "12eb3e81114588aca3b7998f4f19e8997b056aca08e57a7ca7c8a3ec8c652aad",
        "0d63d62cbfd277a91299c0335296a03bcb6a02c2642a90e2b5815caf71726a8d",
    );
    let pin_file = directory.join("standard.pin.toml");
    std::fs::write(&pin_file, metadata).expect("fixture pin");
    std::fs::write(directory.join("overrides.toml"), include_str!("../agents/codex/overrides.toml"))
        .expect("fixture overrides");
    let program = root.join("tools/codex-0.160.0/bin/codex");
    std::fs::create_dir_all(program.parent().expect("bin directory")).expect("fixture package");
    std::fs::write(&program, b"unexecuted Codex fixture\n").expect("fixture executable bytes");
    let pin = smith_bench::read_configuration(&pin_file).expect("fixture pin");
    (pin_file, pin, smith_bench::arm::ArmRef::Installed { program })
}

struct CodexAttempt {
    root: PathBuf,
    guard: smith_bench::guard::Guard,
    private: smith_bench::isolate::Private,
    pin_file: PathBuf,
    pin: smith_bench::PinnedConfiguration,
    task: smith_bench::Task,
    model: smith_bench::ModelChoice,
    arm: smith_bench::arm::ArmRef,
    home: PathBuf,
}

impl CodexAttempt {
    fn new(name: &str) -> Self {
        let root = private(name);
        let home = root.join("logins/codex");
        std::fs::create_dir_all(&home).expect("unissued login directory");
        std::fs::write(home.join("auth.json"), synthetic_auth(500)).expect("unissued login");
        let guard = smith_bench::guard::Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[])
            .expect("fixture guard");
        let private = smith_bench::isolate::empty(&guard, &root.join("scratch")).expect("fixture attempt");
        let (pin_file, pin, arm) = codex_pin_fixture(&root);
        let task = smith_bench::read_task(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/tasks/valid/task.toml"),
        )
        .expect("fixture task");
        let model = smith_bench::ModelChoice { model: Some("gpt-6-luna".into()), effort: Some("low".into()) };
        Self { root, guard, private, pin_file, pin, task, model, arm, home }
    }

    fn context(&self) -> smith_bench::agent::Context<'_> {
        smith_bench::agent::Context {
            guard: &self.guard,
            private: &self.private,
            pin: &self.pin,
            pin_file: &self.pin_file,
            task: &self.task,
            variant: None,
            model: &self.model,
            arm: &self.arm,
            installed_version: "codex-cli 0.160.0",
            home: &self.home,
            wall: skein_lib::Wall::from_nanos(100_000_000_000),
            lifetime: skein_lib::Duration::from_secs(20),
            environment: &[],
        }
    }

    fn finish(self, prepared: smith_bench::agent::codex::Prepared) {
        prepared.borrowed.expect("private credential").finish(&self.guard).expect("settled cleanup");
        std::fs::remove_dir_all(self.root).expect("fixture cleanup");
    }
}

#[test]
fn a_codex_binary_whose_digest_differs_from_its_pin_is_refused() {
    let fixture = CodexAttempt::new("digest");
    let smith_bench::arm::ArmRef::Installed { program } = &fixture.arm else {
        panic!("fixture package");
    };
    std::fs::write(program, b"different executable\n").expect("changed executable");
    let context = smith_bench::agent::Context { home: &fixture.root.join("absent-login"), ..fixture.context() };
    let refusal = smith_bench::agent::codex::prepare(&context).expect_err("digest drift before credentials");
    assert_eq!(refusal.step, smith_bench::isolate::SetupStep::Configuration);
    assert!(refusal.reason.contains("differs from its pin"));
    assert!(!fixture.private.root.join("codex/auth.json").exists());
    std::fs::remove_dir_all(fixture.root).expect("fixture cleanup");
}

#[test]
fn the_codex_pin_names_only_the_keys_an_attempt_must_own() {
    let pin_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("agents/codex/standard.pin.toml");
    let pin = smith_bench::read_configuration(&pin_file).expect("shipped pin");
    assert_eq!(pin.pin.sha256.as_deref(), Some("12eb3e81114588aca3b7998f4f19e8997b056aca08e57a7ca7c8a3ec8c652aad"));
    assert_eq!(pin.pin.package.as_deref(), Some(Path::new("tools/codex-0.160.0")));
    let document: toml::Table =
        toml::from_str(include_str!("../agents/codex/overrides.toml")).expect("shipped overrides");
    let keys: Vec<_> = document.keys().map(String::as_str).collect();
    assert_eq!(keys, ["approval_policy", "check_for_update_on_startup", "harness", "mcp_servers", "sandbox_mode"]);
}

#[test]
fn codex_writable_roots_hold_the_attempts_caches() {
    let fixture = CodexAttempt::new("writable");
    let prepared = smith_bench::agent::codex::prepare(&fixture.context()).expect("private preparation");
    let (_, roots) = prepared
        .overrides
        .iter()
        .find(|(key, _)| key == "sandbox_workspace_write.writable_roots")
        .expect("cache roots");
    let document: toml::Table = toml::from_str(&format!("roots={roots}")).expect("TOML roots");
    let paths: Vec<_> = document["roots"]
        .as_array()
        .expect("root array")
        .iter()
        .map(|value| PathBuf::from(value.as_str().expect("root path")))
        .collect();
    assert_eq!(
        paths,
        [fixture.private.cargo_home.clone(), fixture.private.target.clone(), fixture.private.temporary.clone()]
    );
    fixture.finish(prepared);
}

#[test]
fn codex_trusts_the_attempt_root() {
    let fixture = CodexAttempt::new("trusted");
    let prepared = smith_bench::agent::codex::prepare(&fixture.context()).expect("private preparation");
    let (_, projects) = prepared.overrides.iter().find(|(key, _)| key == "projects").expect("trust table");
    let document: toml::Table = toml::from_str(&format!("projects={projects}")).expect("TOML projects");
    assert_eq!(
        document["projects"][fixture.private.workspace.to_str().expect("attempt root")]["trust_level"].as_str(),
        Some("trusted")
    );
    fixture.finish(prepared);
}

#[test]
fn codex_responses_are_priced_from_their_usage_records() {
    let (root, rollouts) = repository_threads();
    let ledger = rollouts.token_ledger(&root).expect("response usage");
    assert_eq!(ledger.responses().len(), 112);
    let value: f64 = ledger
        .responses()
        .iter()
        .map(|response| {
            let prices = smith_local_shell::presets::list_prices("codex", &response.model, response.prompt_tokens())
                .expect("model price");
            let cost = u128::from(response.fresh - response.cache_write) * u128::from(prices.input)
                + u128::from(response.cache_read) * u128::from(prices.cached)
                + u128::from(response.cache_write) * u128::from(prices.cache_write.expect("write price"))
                + u128::from(response.output) * u128::from(prices.output);
            cost.to_string().parse::<f64>().expect("cost") / (f64::from(prices.unit) * 1_000_000.0)
        })
        .sum();
    assert!((value - 1.75468).abs() < 1e-12);
}
