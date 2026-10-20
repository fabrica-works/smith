//! Program-account paths through the shipped product (shell.md, sections 4 and 9).
mod records;

use super::support;
use serde_json::{Value, json};
use smith_real_world::Scratch;

const ANSWER: &str = "Program answer";

fn signed_in(story: &Value) -> Scratch {
    let scratch = Scratch::new();
    let executable = std::path::Path::new(env!("CARGO_BIN_EXE_smith")).with_file_name("fake-claude");
    let settings = format!(
        "model='anthropic/claude-opus-5-5'\n[programs]\nclaude_code='{}'\n\
         [accounts.work]\nprovider='anthropic'\nsource='program'\n[endpoints.anthropic]\naccount='work'\nprogram='claude_code'\n",
        executable.display()
    );
    let _file = support::user_layer(&scratch, settings.as_bytes());
    let login = support::run_product(&scratch, &["login", "work"], b"", None);
    assert_eq!(login.code, 0, "{}", String::from_utf8_lossy(&login.stderr));
    std::fs::write(
        scratch.path().join("state/smith/programs/work/fake-claude.story"),
        serde_json::to_vec(&json!({
            "status":"Truthful", "login":{"Immediate":"claude.ai"}, "logout":"Succeeds",
            "turns":[story]
        }))
        .expect("program story"),
    )
    .expect("scratch story file");
    scratch
}

fn answer() -> Value {
    json!({"steps":[{"Call":{"tool":"finish","input":{"form":"report","text":ANSWER,"fields":{}}}}]})
}

fn invocations(scratch: &Scratch) -> Vec<Value> {
    std::fs::read(scratch.path().join("state/smith/programs/work/fake-claude.observations"))
        .expect("fake observations")
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).expect("observation record"))
        .filter_map(|record| record.get("arguments").cloned())
        .collect()
}

#[test]
fn exec_spawns_one_print_run_on_its_program() {
    let scratch = signed_in(&answer());
    let ran = support::run_product(&scratch, &["exec", "Ask first"], b"", None);
    assert_eq!(ran.code, 0, "{}", String::from_utf8_lossy(&ran.stderr));
    assert_eq!(ran.stdout, ANSWER.as_bytes());
    let calls = invocations(&scratch);
    let print: Vec<_> = calls.iter().filter(|arguments| arguments[0] == "-p").collect();
    assert_eq!(print.len(), 1);
    assert_eq!(
        &print[0].as_array().expect("print arguments")[..10],
        json!([
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--setting-sources",
            "user",
            "--model"
        ])
        .as_array()
        .expect("expected arguments")
    );
    assert!(print[0].as_array().unwrap().windows(2).any(|pair| pair == [json!("--model"), json!("claude-opus-5-5")]));
    assert_eq!(
        calls.iter().filter(|arguments| arguments[0] == "auth").collect::<Vec<_>>(),
        vec![
            &json!(["auth", "login"]),
            &json!(["auth", "status"]),
            &json!(["auth", "status"]),
            &json!(["auth", "status"])
        ],
        "login, its status, the startup status and the agent source check"
    );
}

#[test]
fn a_program_chat_answers_before_input_ends() {
    use smith_local_process_world::terminal::Action;
    let scratch = signed_in(&answer());
    let ran = support::run_product_scripted(
        &scratch,
        &["--plain"],
        vec![Action::Send(b"Ask first\n".as_slice().into()), Action::Wait(ANSWER.as_bytes().into()), Action::Eof],
    );
    assert_eq!(ran.code, 0, "{}", String::from_utf8_lossy(&ran.stderr));
    assert!(String::from_utf8_lossy(&ran.stdout).contains(ANSWER));
}

#[test]
fn program_json_keeps_each_process_in_its_stream() {
    use smith_events::{Event, Status};
    let scratch = signed_in(&answer());
    let ran = support::run_product(
        &scratch,
        &["exec", "--json", "--trace", "events.jsonl", "-c", "trace.capture='everything'", "Ask first"],
        b"",
        None,
    );
    assert_eq!(ran.code, 0, "{}", String::from_utf8_lossy(&ran.stderr));
    let product = records::read(&ran.stdout);
    let completed = records::answer(&product);
    assert_eq!(completed.status, Status::Accepted);
    assert_eq!(completed.result.as_ref().expect("accepted result").text.as_deref(), Some(ANSWER.as_bytes()));
    let trace = std::fs::read(scratch.path().join("events.jsonl")).expect("product trace");
    assert_eq!(records::read(&trace), product);
    let trace = std::fs::read(scratch.path().join("events.jsonl.agent")).expect("agent trace");
    let agent = records::read(&trace);
    let observations = std::fs::read(scratch.path().join("state/smith/programs/work/fake-claude.observations"))
        .expect("received program input");
    let received: Vec<Value> = observations
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).expect("observation record"))
        .filter_map(|record| record.get("received").cloned())
        .collect();
    smith_agent_process_world::assert_program_event_conformance(&agent, &received);
    assert!(agent.iter().any(|record| matches!(record.event, Event::RunStarted(_))));
    assert!(agent.iter().any(|record| matches!(record.event, Event::ResponseCompleted(_))));
    assert!(agent.iter().any(|record| matches!(record.event, Event::RunCompleted(_))));
}

#[test]
fn a_program_turn_exit_keeps_the_run_failure() {
    let scratch = signed_in(&json!({"steps":[],"end":{"Exit":7}}));
    let ran = support::run_product(
        &scratch,
        &["exec", "--json", "--trace", "failure.jsonl", "-c", "trace.capture='none'", "Ask first"],
        b"",
        None,
    );
    assert_eq!(ran.code, 1, "{}", String::from_utf8_lossy(&ran.stderr));
    assert_eq!(
        String::from_utf8_lossy(&ran.stderr).lines().last(),
        Some("smith: run failed: claude exited before its answer: ")
    );
    let product = records::read(&ran.stdout);
    let completed = records::answer(&product);
    assert_eq!(completed.status, smith_events::Status::Failed);
    let failure = completed.failure.as_ref().expect("program failure");
    assert_eq!(failure.class, smith_events::AnswerClass::Model);
    assert_eq!(failure.program.as_deref(), Some(b"claude_code".as_slice()));
    assert_eq!(failure.exit, Some(smith_events::AgentExit::Code(7)));
    assert_eq!(failure.detail, None);
}
