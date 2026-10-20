use super::*;

#[test]
fn the_report_lists_each_program_a_run_may_use() {
    let report = Report {
        version: CHECK_VERSION,
        settings: Settings {
            paths: settings::Paths::default(),
            workspace: "/workspace".into(),
            values: BTreeMap::new(),
        },
        formats: BTreeMap::from([("check", CHECK_VERSION)]),
        build: serde_json::Value::Null,
        tools: BTreeMap::from([("sh", "/bin/sh".into())]),
        programs: BTreeMap::from([("claude_code", "/programs/claude".into())]),
        environment: Vec::new(),
        limits: Vec::new(),
        named_limits: Vec::new(),
        caches: Vec::new(),
        warnings: Vec::new(),
        outside_models: BTreeMap::new(),
        credentials: BTreeMap::new(),
    };
    let json: serde_json::Value = serde_json::from_str(&render_json(&report)).expect("check document");
    assert_eq!(
        json["programs"],
        serde_json::json!({"claude_code":"/programs/claude"}),
        "the reported state matches the contract"
    );
    assert_eq!(json["version"], 2, "the reported state matches the contract");
    assert_eq!(json["formats"]["check"], 2, "the reported state matches the contract");
    assert!(
        render_text(&report).contains("sh = /bin/sh\nprogram claude_code = /programs/claude\nenvironment:"),
        "the reported state matches the contract"
    );
}
