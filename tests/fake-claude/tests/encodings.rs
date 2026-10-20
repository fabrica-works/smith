//! Story and observation records (protocol/programs.md, section 8;
//! testing.md, section 4).

use smith_fake_claude::{Login, Logout, Observation, Status, Story, observations};

#[test]
fn a_story_reads_back_as_written() {
    for story in [
        Story::default(),
        Story {
            status: Status::Unreadable,
            login: Login::Code("api\"key".into()),
            logout: Logout::Fails,
            ..Story::default()
        },
        Story { status: Status::Silent, login: Login::Refused, logout: Logout::Succeeds, ..Story::default() },
    ] {
        assert_eq!(serde_json::from_slice::<Story>(&story.encode()).expect("story encoding"), story);
    }
}

#[test]
fn observations_read_back_as_written() {
    let expected = [
        Observation { arguments: vec!["auth".into(), "status".into()], environment: vec!["HOME".into()] },
        Observation { arguments: vec!["quote\"\n".into()], environment: vec!["CLAUDE_CONFIG_DIR".into()] },
    ];
    let mut bytes = Vec::new();
    for record in &expected {
        bytes.extend(serde_json::to_vec(record).expect("observation encoding"));
        bytes.push(b'\n');
    }
    assert_eq!(observations(&bytes), expected);
    assert!(observations(b"").is_empty());
}

#[test]
fn a_stage_one_story_still_decodes() {
    assert_eq!(
        serde_json::from_slice::<Story>(
            br#"{"status":"Truthful","login":{"Immediate":"claude.ai"},"logout":"Succeeds"}"#
        )
        .expect("stage one story"),
        Story::default()
    );
}

#[test]
fn a_server_script_reads_back_as_written() {
    use smith_fake_claude::{Answer, ServerScript, Tool, Verdict};
    let script = ServerScript {
        tools: vec![
            Tool { name: "answer".into(), answer: Answer::Text("quote\"\n".into()) },
            Tool { name: "error".into(), answer: Answer::Error("failed".into()) },
            Tool { name: "never".into(), answer: Answer::Never },
        ],
        verdicts: vec![Verdict::Allow, Verdict::Deny("outside".into())],
        ..ServerScript::default()
    };
    assert_eq!(serde_json::from_str::<ServerScript>(&script.encode()).expect("script"), script);
    assert_eq!(serde_json::from_str::<ServerScript>("{}").expect("default script"), ServerScript::default());
}
