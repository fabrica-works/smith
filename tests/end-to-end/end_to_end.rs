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
    std::fs::write(&path, br#"{"profile":{"name":"standard","declared":{"memory":1099511627776},"policy":{"group_stop":10}},"endpoints":[{"name":"invalid","number":1,"dialect":1,"account":0,"provider":"codex","connect":10000,"handshake":10000,"models":[{"name":"fake","window":8192,"output":1024,"reasoning_item":2048,"head":60000,"idle":30000}],"address":"192.0.2.1:8080","transport":"plaintext"}],"environment":[]}"#).expect("invalid transport fixture");
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
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776,"read_window":1025,"tool_payload":1024}},"endpoints":[{"name":"fake","number":0,"dialect":0,"account":0,"provider":"codex","address":"127.0.0.1:8080","transport":"plaintext","connect":1000,"handshake":1000,"models":[{"name":"fake","window":65536,"output":32,"reasoning_item":2048,"head":1000,"idle":1000}]}],"environment":[]}"#,
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
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776}},"endpoints":[{"name":"fake","number":0,"dialect":0,"account":0,"provider":"codex","address":"127.0.0.1:8080","transport":"plaintext","connect":1000,"handshake":1000,"models":[{"name":"fake","window":65536,"output":32,"reasoning_item":2048,"head":1000,"idle":1000}]}],"environment":[{"name":"A","value":"one"},{"name":"A","value":"two"}]}"#,
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
        br#"{"profile":{"name":"standard","declared":{"memory":1099511627776}},"endpoints":[],"environment":[]}"#,
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
        ("edit", r#"{"path":"original.txt","old":"original","new":"edited"}"#),
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
