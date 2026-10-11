pub mod support;

use smith_local_process_world::{Placement, referee::CheckoutRead};
use smith_real_world::{Scratch, World};

#[test]
fn the_shipped_local_and_agent_answer_on_a_terminal() {
    let scratch = Scratch::new();
    let scenario = World::new(31, &scratch, Placement::Spawned, b"First answer").scenario;
    let seen = support::run(&scratch, scenario);
    assert_eq!(seen.queries.len(), 1);
}

#[test]
fn a_first_run_signs_in_and_ends_with_a_commit_in_place() {
    let scratch = Scratch::new();
    let mut scenario = World::changed(32, &scratch, Placement::Spawned).scenario;
    scenario.launch.authenticated = true;
    scenario.authentication = Some(smith_local_process_world::world::Authentication::SignIn);
    std::fs::remove_file(scenario.launch.token_directory.join("0.json")).expect("first sign-in has no token");
    let seen = support::run(&scratch, scenario);
    let oauth = seen.oauth.expect("issuer and browser observations");
    assert_eq!(oauth.posts, 1);
    assert_eq!(oauth.pages, 1);
    assert!(oauth.browser_replied && oauth.saved_before_query);
    let checkout = scratch.checkout();
    let head = checkout.head().expect("committed head");
    let files = checkout.files(&head);
    assert_eq!(files[b"original.txt".as_slice()], b"edited\n");
    assert_eq!(files[b"result.txt".as_slice()], b"new\n");
    assert_eq!(files[b"command.txt".as_slice()], b"ran\n");
    assert_eq!(files[b"checks-ran.txt".as_slice()], b"checked\n");
}

#[test]
fn a_second_run_resumes_the_chat_from_its_files() {
    let scratch = Scratch::new();
    let first = support::run(&scratch, World::new(33, &scratch, Placement::Spawned, b"First answer").scenario);
    let second = support::run(&scratch, World::new(34, &scratch, Placement::Spawned, b"Second answer").scenario);
    assert!(second.queries[0].messages.len() > first.queries[0].messages.len());
    assert!(
        second.queries[0].messages.iter().flat_map(|message| message.parts.iter()).any(|part| {
            matches!(part, skein_fake_llm_domain::api::Part::ToolCall { name, arguments, .. }
            if name.as_ref() == b"finish" && arguments.windows(12).any(|bytes| bytes == b"First answer"))
        }),
        "the second actual agent received the first run's durable history"
    );
}

#[test]
fn the_startup_refusals_of_both_commands_are_observed_outside() {
    let scratch = Scratch::new();
    for command in ["agent", "local"] {
        let arguments = if command == "agent" {
            vec![command.into(), "".into()]
        } else {
            vec![command.into(), "".into(), scratch.path().into()]
        };
        let errors = support::refusal(&scratch, arguments);
        assert!(String::from_utf8_lossy(&errors).contains(if command == "agent" {
            "configuration metadata"
        } else {
            "settings metadata"
        }));
    }
    let path = scratch.path().join("agent-config.json");
    std::fs::write(
        &path,
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776,"conversations":1,"calls_per_response":2,"tool_payload":256,"read_window":256},"policy":{"group_stop":10}},"endpoints":[{"name":"fake","number":0,"dialect":0,"account":0,"provider":"codex","address":"127.0.0.1:8080","transport":"plaintext","connect":1000,"handshake":1000,"models":[{"name":"fake","window":65536,"output":512,"reasoning_item":64,"head":1000,"idle":1000}]}],"environment":[]}"#,
    )
    .expect("valid agent fixture");
    let errors = support::refusal(&scratch, vec!["agent".into(), path.clone().into()]);
    assert_eq!(
        String::from_utf8_lossy(&errors).lines().last(),
        Some("smith: the run could not answer: Some(ChannelEnded)")
    );
    std::fs::write(&path, br#"{"profile":{"name":"standard","declared":{"memory":1099511627776,"conversations":1,"calls_per_response":2,"tool_payload":256,"read_window":256},"policy":{"group_stop":10}},"endpoints":[{"name":"invalid","number":1,"dialect":1,"account":0,"provider":"codex","connect":10000,"handshake":10000,"models":[{"name":"fake","window":65536,"output":512,"reasoning_item":64,"head":60000,"idle":30000}],"address":"192.0.2.1:8080","transport":"plaintext"}],"environment":[]}"#).expect("invalid transport fixture");
    let errors = support::refusal(&scratch, vec!["agent".into(), path.into()]);
    assert_eq!(
        String::from_utf8_lossy(&errors).lines().last(),
        Some("smith: plaintext endpoint address must be loopback")
    );
    assert!(!scratch.path().join("chat").exists(), "refusals leave no chat");
}

#[test]
fn the_live_outcome_referee_runs_on_the_shared_fake_binary_harness() {
    let report = Scratch::new();
    support::live_run::run_fake(&report, &World::new(35, &report, Placement::Spawned, b"First answer").scenario);
    let change = Scratch::new();
    support::live_run::run_fake(&change, &World::changed(36, &change, Placement::Spawned).scenario);
}

#[test]
fn smith_agent_refuses_a_malformed_configuration_before_its_channel_opens() {
    let scratch = Scratch::new();
    let path = scratch.path().join("malformed-agent.json");
    std::fs::write(&path, br#"{"profile":"standard"}"#).expect("malformed configuration fixture");
    let errors =
        support::refusal_program(std::path::Path::new(env!("CARGO_BIN_EXE_smith-agent")), &scratch, vec![path.into()]);
    let line = std::str::from_utf8(&errors).expect("UTF-8 refusal");
    assert!(line.starts_with("smith: configuration JSON:"));
    assert_eq!(line.lines().count(), 1, "startup writes one refusal");
}

#[test]
fn smith_agent_names_a_broken_relationship_and_exits_1_before_its_channel_opens() {
    let scratch = Scratch::new();
    let path = scratch.path().join("read-past-payload.json");
    std::fs::write(&path,
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776,"read_window":1025,"tool_payload":1024}},"endpoints":[{"name":"fake","number":0,"dialect":0,"account":0,"provider":"codex","address":"127.0.0.1:8080","transport":"plaintext","connect":1000,"handshake":1000,"models":[{"name":"fake","window":65536,"output":512,"reasoning_item":64,"head":1000,"idle":1000}]}],"environment":[]}"#,
    ).expect("broken relationship fixture");
    let errors =
        support::refusal_program(std::path::Path::new(env!("CARGO_BIN_EXE_smith-agent")), &scratch, vec![path.into()]);
    assert_eq!(
        std::str::from_utf8(&errors).expect("UTF-8 refusal"),
        "smith: read-in-payload: declared.read_window=1025 bytes; declared.tool_payload=1024 bytes\n"
    );
}

#[test]
fn smith_agent_names_a_repeated_environment_entry_before_its_channel_opens() {
    let scratch = Scratch::new();
    let path = scratch.path().join("repeated-environment.json");
    std::fs::write(&path,
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776,"tool_payload":256,"read_window":256}},"endpoints":[{"name":"fake","number":0,"dialect":0,"account":0,"provider":"codex","address":"127.0.0.1:8080","transport":"plaintext","connect":1000,"handshake":1000,"models":[{"name":"fake","window":65536,"output":512,"reasoning_item":64,"head":1000,"idle":1000}]}],"environment":[{"name":"A","value":"one"},{"name":"A","value":"two"}]}"#,
    ).expect("repeated environment fixture");
    let errors =
        support::refusal_program(std::path::Path::new(env!("CARGO_BIN_EXE_smith-agent")), &scratch, vec![path.into()]);
    assert_eq!(
        std::str::from_utf8(&errors).expect("UTF-8 refusal"),
        "smith: environment: entry 1 has an invalid or repeated name\n"
    );
}

#[test]
fn smith_agent_refuses_empty_endpoints_by_name_before_its_channel_opens() {
    let scratch = Scratch::new();
    let path = scratch.path().join("empty-endpoints.json");
    std::fs::write(
        &path,
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776,"tool_payload":256,"read_window":256}},"endpoints":[],"environment":[]}"#,
    )
    .expect("empty endpoints fixture");
    let errors =
        support::refusal_program(std::path::Path::new(env!("CARGO_BIN_EXE_smith-agent")), &scratch, vec![path.into()]);
    assert_eq!(
        std::str::from_utf8(&errors).expect("UTF-8 refusal"),
        "smith: field: declared.endpoints must not be empty\n"
    );
}

#[test]
fn standard_profile_runs_a_large_coding_write_and_the_following_checks() {
    use skein_fake_llm_domain::api;
    use smith_local_process_world::referee::CheckoutRead;

    let scratch = Scratch::new();
    let scenario = World::changed(37, &scratch, Placement::Spawned).scenario;
    let large = serde_json::to_vec(&serde_json::json!({
        "path": "result.txt", "content": "// coding regression\n".repeat(160)
    }))
    .expect("large coding tool arguments");
    assert!(large.len() > 2048, "coding write exceeds the former input limit");
    let mut turns = vec![(b"write".as_slice(), large)];
    for (name, arguments) in [
        ("read", r#"{"path":"result.txt"}"#),
        ("read", r#"{"path":"original.txt"}"#),
        ("edit", r#"{"path":"original.txt","edits":[{"old":"original","new":"edited"}]}"#),
        ("write", r#"{"path":"result.txt","content":"new\n"}"#),
        ("shell", r#"{"command":"printf 'ran\n' > command.txt"}"#),
        (
            "finish",
            r#"{"form":"change","text":"","fields":{"title":"Updated result","body":"Created the result file"}}"#,
        ),
    ] {
        turns.push((name.as_bytes(), arguments.as_bytes().to_vec()));
    }
    let peer = smith_agent_process_world::fake::configured_transport(
        Box::new([api::Script {
            cue: b"@local-shell".as_slice().into(),
            turns: turns
                .into_iter()
                .map(|(name, arguments)| api::Turn {
                    lines: Box::new([api::Line::Call { name: name.into(), arguments: arguments.into() }]),
                    finish: api::Finish::ToolCalls,
                    tokens: 1,
                })
                .collect(),
        }]),
        skein_llm::Credential { access_token: b"token".as_slice().into(), account_id: b"acc".as_slice().into() },
        skein_lib::Duration::ZERO,
        skein_fake_peers::Transport::Tls,
    );
    let seen = support::run_with_provider(&scratch, scenario, Some(peer));
    assert_eq!(seen.queries.len(), 7, "large write and every subsequent call completed");
    assert!(
        seen.queries.iter().skip(1).any(|query| query
            .messages
            .iter()
            .any(|message| { message.parts.iter().any(|part| matches!(part, api::Part::ToolOutput { .. })) })),
        "follow-up provider prompts contain actual workspace answers"
    );
    let checkout = scratch.checkout();
    let files = checkout.files(&checkout.head().expect("committed head"));
    assert_eq!(files[b"result.txt".as_slice()], b"new\n");
    assert_eq!(files[b"original.txt".as_slice()], b"edited\n");
    assert_eq!(files[b"checks-ran.txt".as_slice()], b"checked\n");
}

fn refusal_settings(output: u32, read_window: u32) -> serde_json::Value {
    serde_json::json!({
        "agent": {"profile": {"name":"standard", "declared":{
            "memory":2_147_483_648_u64,"conversations":6,"calls_per_response":16,
            "tool_payload":40960,"file_bytes":1_048_576,"read_window":read_window}},
            "endpoints":[{"name":"fake","number":1,"dialect":1,"account":0,"provider":"codex",
                "address":"127.0.0.1:1","transport":"tls","server_name":"fixture.invalid",
                "connect":10000,"handshake":10000}],"environment":[]},
        "chat":"refused","instructions":"No run may start",
        "models":[{"endpoint":"fake","name":"fake","window":272_000,"output":output,
            "reasoning_item":65536,"head":60000,"idle":300_000,
            "input_price":1_250_000,"cached_price":125_000,"output_price":10_000_000,"price_unit":1_000_000}],
        "budget":{"turns":4096,"spend":1_000_000_000_u64,"seconds":300,"reserve":{"turns":2,"seconds":150}},
        "waiting_seconds":30,"contract":{"form":"report","max":4096},"in_process":false
    })
}

fn local_refusal_output(settings: &serde_json::Value) -> (Vec<u8>, Scratch) {
    let scratch = Scratch::new();
    let path = scratch.path().join("settings.json");
    std::fs::write(&path, serde_json::to_vec(settings).expect("settings fixture")).expect("write settings");
    let state = scratch.path().join("absent-state");
    let output = support::refusal_code(&scratch, vec!["local".into(), path.into(), state.clone().into()], 2);
    assert!(!state.exists(), "configuration refusal precedes chat/state and a run");
    (output, scratch)
}

#[test]
fn local_bad_setting_exits_2_and_names_output_before_a_run() {
    let (output, _scratch) = local_refusal_output(&refusal_settings(0, 32768));
    let stderr = std::str::from_utf8(&output).expect("refusal is UTF-8");
    assert!(stderr.contains("models[0].output"), "{stderr}");
    assert!(stderr.contains('0') && stderr.contains("positive"), "{stderr}");
}

#[test]
fn local_derived_limit_exits_2_and_names_both_values_before_a_run() {
    let (output, _scratch) = local_refusal_output(&refusal_settings(32000, 40961));
    let stderr = std::str::from_utf8(&output).expect("refusal is UTF-8");
    assert!(stderr.contains("read-in-payload"), "{stderr}");
    assert!(stderr.contains("40961") && stderr.contains("40960"), "{stderr}");
}

#[test]
fn local_host_io_failure_keeps_exit_1() {
    let scratch = Scratch::new();
    let settings_path = scratch.path().join("settings.json");
    std::fs::write(&settings_path, serde_json::to_vec(&refusal_settings(32_000, 32_768)).expect("fixture JSON"))
        .expect("write settings");
    let state = scratch.path().join("blocked-state");
    std::fs::write(&state, b"a file cannot be the state directory").expect("block state directory");
    let output = support::refusal_code(&scratch, vec!["local".into(), settings_path.into(), state.clone().into()], 1);
    assert!(String::from_utf8_lossy(&output).contains("state directory"));
    assert_eq!(std::fs::read(state).expect("unchanged state file"), b"a file cannot be the state directory");
}

#[test]
fn local_chat_store_io_failure_keeps_exit_1() {
    let scratch = Scratch::new();
    let settings_path = scratch.path().join("settings.json");
    let mut settings = refusal_settings(32_000, 32_768);
    settings["token_directory"] = serde_json::json!(scratch.path().join("tokens"));
    std::fs::write(&settings_path, serde_json::to_vec(&settings).expect("fixture JSON")).expect("write settings");
    let state = scratch.path().join("state");
    std::fs::create_dir(&state).expect("state root");
    let chat = state.join("refused");
    std::fs::write(&chat, b"a file cannot be the chat directory").expect("block chat directory");
    let output = support::refusal_code(&scratch, vec!["local".into(), settings_path.into(), state.clone().into()], 1);
    assert!(String::from_utf8_lossy(&output).contains("chat store"), "{}", String::from_utf8_lossy(&output));
    assert_eq!(std::fs::read(chat).expect("unchanged chat file"), b"a file cannot be the chat directory");
}

#[test]
fn a_one_turn_local_run_is_refused_by_the_actor_before_any_completion() {
    let scratch = Scratch::new();
    let scenario = World::new(63, &scratch, Placement::Spawned, b"Unused").scenario;
    support::settings(&scratch, &scenario);
    let mut settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(scratch.path().join("settings.json")).expect("fixture settings"))
            .expect("fixture JSON");
    settings["agent"]["profile"]["declared"] = serde_json::json!({
        "memory":2_147_483_648_u64,"conversations":6,"calls_per_response":16,
        "tool_payload":40960,"file_bytes":1_048_576,"read_window":32768});
    settings["models"][0]["window"] = serde_json::json!(272_000);
    settings["models"][0]["output"] = serde_json::json!(32_000);
    settings["models"][0]["reasoning_item"] = serde_json::json!(65_536);
    settings["models"][0]["idle"] = serde_json::json!(300_000);
    settings["budget"] = serde_json::json!({"turns":1,"spend":1_000_000_000_u64,"seconds":600,
        "reserve":{"turns":2,"seconds":300}});
    settings["agent"]["trace"] = serde_json::json!({"path":scratch.path().join("agent-trace.jsonl"),"capture":"calls"});
    std::fs::write(scratch.path().join("settings.json"), serde_json::to_vec(&settings).expect("settings fixture"))
        .expect("write local settings");
    let records = support::refused_run(&scratch);
    let completed: Vec<_> = records
        .iter()
        .filter_map(|record| match &record.event {
            smith_events::Event::RunCompleted(completed) => Some(completed),
            smith_events::Event::Budget(_)
            | smith_events::Event::SessionStarted(_)
            | smith_events::Event::SessionEnded(_)
            | smith_events::Event::RunStarted(_)
            | smith_events::Event::ConversationOpened(_)
            | smith_events::Event::ConversationClosed(_)
            | smith_events::Event::ResponseStarted(_)
            | smith_events::Event::ResponseCompleted(_)
            | smith_events::Event::TextDelta(_)
            | smith_events::Event::ToolStarted(_)
            | smith_events::Event::ToolCompleted(_)
            | smith_events::Event::CheckStarted(_)
            | smith_events::Event::CheckCompleted(_)
            | smith_events::Event::MessageReceived(_)
            | smith_events::Event::MessageRead(_)
            | smith_events::Event::MessageUnread(_)
            | smith_events::Event::Notice(_) => None,
        })
        .collect();
    assert_eq!(completed.len(), 1, "one genuine actor terminal");
    let completed = completed.first().expect("actual refused run");
    assert_eq!(completed.status, smith_events::Status::Refused);
    let failure = completed.failure.as_ref().expect("typed reserve refusal");
    assert_eq!(failure.class, smith_events::AnswerClass::Invalid);
    assert!(failure.which.as_ref().is_some_and(|name| name.windows(7).any(|part| part == b"reserve")), "{failure:?}");
    assert!(
        !records.iter().any(|record| matches!(&record.event, smith_events::Event::ResponseStarted(_))),
        "reserve refusal starts no completion or provider request"
    );
}
