use super::*;
use crate::credentials::tests::Files;
use std::os::unix::fs::PermissionsExt;

#[test]
fn the_report_lists_each_program_a_run_may_use() {
    let files = Files::new();
    let executable = files.0.join("claude");
    std::fs::write(&executable, b"").expect("scratch executable");
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).expect("executable mode");
    let certificate = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/protocol-llm/fixtures/root.der");
    let text = format!(
        "model='codex/gpt-6-luna'\ncredentials='{}'\n[environment]\ninherit=['PATH']\n[programs]\nclaude_code='{}'\n[accounts.codex]\nsource='key'\n[endpoints.codex]\naddress='127.0.0.1:443'\nprogram='claude_code'\ntrust.system=false\ntrust.der=['{certificate}']\n",
        files.0.display(),
        executable.display()
    );
    let mut settings = settings::merge(
        &command::Shared::default(),
        files.0.clone(),
        &[settings::Source { layer: settings::Layer::User, path: files.0.join("config.toml"), text }],
    )
    .unwrap_or_else(|why| panic!("{}", why.line()));
    settings.paths.path = Some("/usr/bin:/bin".into());
    let report = match report(&settings, skein_lib::Wall::from_nanos(0), &skein_shell::Clock::new()) {
        Verdict::Ready(report) => report,
        Verdict::Refused(why) => panic!("{}", why.line()),
    };
    let json: serde_json::Value = serde_json::from_str(&render_json(&report)).expect("check document");
    assert_eq!(
        json["programs"],
        serde_json::json!({"claude_code":executable}),
        "the builder resolves each declared program path"
    );
    assert_eq!(json["version"], 2, "check emits version 2");
    assert_eq!(json["formats"]["check"], 2, "formats names the same version");
    let text = render_text(&report);
    let program_line = format!("program claude_code = {}", executable.display());
    assert_eq!(
        text.lines().skip_while(|line| !line.starts_with("sh = ")).nth(1),
        Some(program_line.as_str()),
        "programs immediately follow the tools"
    );
    assert!(text.contains(&format!("{program_line}\nenvironment:")), "programs precede the environment");
    assert!(!files.0.join("programs").exists(), "building the report never prepares the program source");
}
