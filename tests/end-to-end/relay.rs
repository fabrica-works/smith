//! The shipped binary's relay dispatch (protocol/programs.md, section 9).
use smith_real_world::Scratch;
use std::path::Path;
#[test]
fn the_agent_binary_refuses_a_relay_without_its_secret() {
    let scratch = Scratch::new();
    let stderr = crate::support::refusal_program(
        Path::new(env!("CARGO_BIN_EXE_smith-agent")),
        &scratch,
        vec!["--mcp-relay".into(), "127.0.0.1:9".into()],
    );
    assert!(stderr.starts_with(b"smith-agent: usage: smith-agent --mcp-relay"));
    assert_eq!(skein_lib::bytes::count(&stderr, b"\n", 2), 1);
}
