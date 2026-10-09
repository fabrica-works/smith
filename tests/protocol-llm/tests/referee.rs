//! Negative controls for the native stable-prefix contract (protocol/llm.md, section 10).
use skein_lib::{Time, Token};
use skein_world::domain::{Referee, Verdict};
use smith_protocol_llm_world::referee::{Meeting, Seen, NativeRequest};

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
    let mut changed = request(); changed.system = "different".into();
    reject(changed, "native system bytes stay fixed within a window");
}

#[test]
fn changed_native_tool_schema_is_rejected() {
    let mut changed = request(); changed.tools = "different".into();
    reject(changed, "native tool bytes stay fixed within a window");
}

#[test]
fn edited_native_history_item_is_rejected() {
    let mut changed = request(); changed.history[0] = "different".into();
    reject(changed, "native history bytes extend within a window");
}
