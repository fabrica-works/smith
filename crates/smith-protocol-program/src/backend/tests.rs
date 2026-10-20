#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    reason = "unit test harnesses use ordinary Rust per programming-model.md, section 10.2"
)]

extern crate std;

use super::{files, mcp, start::Names, stream};
use alloc::boxed::Box;
use skein_lib::Secret;

#[test]
fn entropy_names_have_separate_domains() {
    let names = Names::new(&Secret::new(Box::new([7; 32])));
    let repeated = Names::new(&Secret::new(Box::new([7; 32])));
    assert!(names.mcp.expose() == repeated.mcp.expose());
    assert!(names.run == repeated.run);
    assert!(names.mcp.expose() != names.run);
}

#[test]
fn settings_require_each_builtin_to_ask() {
    let value: serde_json::Value = serde_json::from_slice(files::SETTINGS).expect("settings JSON");
    assert_eq!(
        value,
        serde_json::json!({"permissions":{"allow":["mcp__smith"],
        "ask":["Read","Glob","Grep","Write","Edit","Bash"]}})
    );
}

fn tool() -> mcp::Tool {
    mcp::Tool {
        name: b"finish".as_slice().into(),
        description: b"Finish".as_slice().into(),
        input_schema: br#"{"type":"object"}"#.as_slice().into(),
    }
}

#[test]
fn init_requires_the_connected_smith_server() {
    let bytes = br#"{"type":"system","subtype":"init","mcp_servers":[{"name":"smith","status":"connected"}],"tools":["mcp__smith__finish","mcp__smith__permission_prompt"]}"#;
    assert!(stream::init(bytes, &[tool()], 32, 256));
    let missing = br#"{"type":"system","subtype":"init","mcp_servers":[],"tools":["mcp__smith__finish","mcp__smith__permission_prompt"]}"#;
    assert!(!stream::init(missing, &[tool()], 32, 256));
}

#[test]
fn init_refuses_an_ungranted_builtin() {
    let bytes = br#"{"type":"system","subtype":"init","mcp_servers":[{"name":"smith","status":"connected"}],"tools":["mcp__smith__finish","mcp__smith__permission_prompt","Read"]}"#;
    assert!(!stream::init(bytes, &[tool()], 32, 256));
}

#[test]
fn init_refuses_a_repeated_tool() {
    let bytes = br#"{"type":"system","subtype":"init","mcp_servers":[{"name":"smith","status":"connected"}],"tools":["mcp__smith__finish","mcp__smith__finish"]}"#;
    assert!(!stream::init(bytes, &[tool()], 32, 256));
}

#[test]
fn diagnostics_stay_within_the_utf8_byte_cap() {
    assert_eq!(super::diagnostic(b"tail\xff", 5).as_ref(), b"tail?");
    assert_eq!(super::diagnostic("tailé".as_bytes(), 5).as_ref(), b"tail");
}

#[test]
fn command_keeps_the_inline_schema_in_one_argument() {
    let arguments = super::start::arguments(
        b"chosen-model",
        Some(b"high"),
        b"/run/instructions.md",
        b"/run/mcp.json",
        br#"{"type":"object","description":"spaces stay inline"}"#,
        9,
        b"9213b8f4-a680-416e-a34c-6718a9575807",
        b"",
    )
    .expect("bounded command");
    let flag = arguments.iter().position(|argument| argument.as_ref() == b"--json-schema").expect("schema flag");
    assert_eq!(arguments[flag + 1].as_ref(), br#"{"type":"object","description":"spaces stay inline"}"#);
    let flag = arguments.iter().position(|argument| argument.as_ref() == b"--effort").expect("effort flag");
    assert_eq!(arguments[flag + 1].as_ref(), b"high");
    let flag = arguments.iter().position(|argument| argument.as_ref() == b"--tools").expect("tools flag");
    assert!(arguments[flag + 1].is_empty());
}

#[test]
fn mcp_file_keeps_the_secret_in_the_relay_environment() {
    let bytes = files::mcp_configuration(b"/relay/quoted\"name", 1234, &[b'a'; 64]);
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("MCP file JSON");
    let server = &value["mcpServers"]["smith"];
    assert_eq!(server["type"], "stdio");
    assert_eq!(server["command"], "/relay/quoted\"name");
    assert_eq!(server["args"], serde_json::json!(["--mcp-relay", "127.0.0.1:1234"]));
    assert_eq!(server["env"]["SMITH_MCP_SECRET"].as_str().expect("token").len(), 64);
}
