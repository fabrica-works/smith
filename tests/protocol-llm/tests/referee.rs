//! Negative controls for the native stable-prefix contract (protocol/llm.md, section 10).
use skein_lib::{Time, Token};
use skein_world::domain::{Referee, Verdict};
use smith_protocol_llm_world::referee::{Meeting, NativeRequest, Seen};

fn request() -> NativeRequest {
    NativeRequest { system: "system".into(), tools: "tools".into(), history: vec!["first".into()] }
}

fn reject(changed: NativeRequest, reason: &str) {
    let mut referee = Referee::new(Meeting::default());
    referee.observe(Time::ZERO, Seen::Request { owner: Token::new(1), request: request() }, &mut Vec::new());
    referee.observe(Time::ZERO, Seen::Request { owner: Token::new(1), request: changed }, &mut Vec::new());
    let Verdict::Failed(failure) = referee.verdict() else { panic!("broken prefix must fail") };
    assert_eq!(failure.why, reason);
}

#[test]
fn changed_native_system_is_rejected() {
    let mut changed = request();
    changed.system = "different".into();
    reject(changed, "native system bytes stay fixed within a window");
}

#[test]
fn changed_native_tool_schema_is_rejected() {
    let mut changed = request();
    changed.tools = "different".into();
    reject(changed, "native tool bytes stay fixed within a window");
}

#[test]
fn edited_native_history_item_is_rejected() {
    let mut changed = request();
    changed.history[0] = "different".into();
    reject(changed, "native history bytes extend within a window");
}

fn http(body: &str) -> Vec<u8> {
    [b"POST /fixture HTTP/1.1\r\n\r\n".as_slice(), body.as_bytes()].concat()
}

#[test]
fn only_the_moving_anthropic_marker_is_removed_from_history() {
    let previous = NativeRequest::from_http(&http(
        r#"{"system":[{"type":"text","text":"fixed","cache_control":{"type":"ephemeral"}}],"tools":[],"messages":[{"role":"user","content":[{"type":"text","text":"first","cache_control":{"type":"ephemeral"}}]}]}"#,
    ));
    let next = NativeRequest::from_http(&http(
        r#"{"system":[{"type":"text","text":"fixed","cache_control":{"type":"ephemeral"}}],"tools":[],"messages":[{"role":"user","content":[{"type":"text","text":"first"}]},{"role":"assistant","content":[{"type":"text","text":"answer","cache_control":{"type":"ephemeral"}}]}]}"#,
    ));
    assert_eq!(previous.system, next.system);
    assert!(previous.system.contains("cache_control"), "the fixed system marker stays literal");
    assert!(next.history.starts_with(&previous.history));
    assert_eq!(previous.history[0], r#"{"role":"user","content":[{"type":"text","text":"first"}]}"#);
}

#[test]
#[should_panic(expected = "at most one moving Anthropic tail marker")]
fn a_second_anthropic_tail_marker_is_rejected() {
    let _request = NativeRequest::from_http(&http(
        r#"{"system":[],"tools":[],"messages":[{"role":"user","content":[{"type":"text","text":"first","cache_control":{"type":"ephemeral"}},{"type":"text","text":"second","cache_control":{"type":"ephemeral"}}]}]}"#,
    ));
}

#[test]
fn codex_history_and_escaped_marker_words_stay_literal() {
    let codex = NativeRequest::from_http(&http(
        r#"{"instructions":"fixed","tools":[],"input":[{"type":"message","cache_control":{"type":"ephemeral"}}]}"#,
    ));
    assert_eq!(codex.history[0], r#"{"type":"message","cache_control":{"type":"ephemeral"}}"#);
    let quoted = NativeRequest::from_http(&http(
        r#"{"system":[],"tools":[],"messages":[{"role":"user","content":[{"type":"text","text":"literal ,\"cache_control\":{\"type\":\"ephemeral\"}"}]}]}"#,
    ));
    assert_eq!(
        quoted.history[0],
        r#"{"role":"user","content":[{"type":"text","text":"literal ,\"cache_control\":{\"type\":\"ephemeral\"}"}]}"#
    );
}
