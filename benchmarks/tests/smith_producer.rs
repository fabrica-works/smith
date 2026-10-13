//! The published events face receives the actual frozen control's writer bounds.
//! This crosses the local settings translation, startup derivation and registry;
//! it assumes no global range for task-selected model or policy overrides.

use smith_bench::agent::smith::Receiving;

#[test]
fn the_published_face_preserves_the_native_control_producers_joint_bounds() {
    let mut source: serde_json::Value =
        serde_json::from_str(include_str!("../agents/smith/smoke.json")).expect("frozen native template");
    source["agent"] = serde_json::json!({
        "profile":{"name":"standard","declared":{
            "conversations":6,"tool_payload":40960,"file_bytes":1_048_576,"calls_per_response":16,
            "read_window":32768,"guide":16384,"shell_output":16384,"shell_head":16384,"shell_tail":49152,
            "search_hits":100,"search_bytes":16384,"list_entries":512,"llm_pool":134_217_728,"memory":2_147_483_648_u64},
            "policy":{"group_stop":250,"host_tools":2}},
        "endpoints":[{"name":"codex","number":1,"dialect":1,"account":0,"provider":"codex",
            "address":"127.0.0.1:443","transport":"tls","server_name":"chatgpt.com",
            "connect":30000,"handshake":30000}],
        "environment":[],"trace":{"path":"/tmp/unused-registry-derivation","capture":"everything"}
    });
    source["models"] = serde_json::json!([{"endpoint":"codex","name":"gpt-6-luna",
        "window":272_000,"output":32000,"reasoning_item":65536,"head":60000,"idle":300_000,
        "input_price":1_250_000,"cached_price":125_000,"output_price":10_000_000,"price_unit":1_000_000}]);
    source["accounts"] = serde_json::json!([{"number":0,"account_id":"opaque-test-account"}]);
    source["budget"] = serde_json::json!({"turns":4096,"spend":1_000_000_000_u64,"seconds":300,
        "reserve":{"turns":2,"seconds":150}});
    let local = serde_json::from_value(source).expect("actual local settings shape");
    let document = smith_local_shell::local_settings::agent_document(&local).expect("native agent document");
    let configured = smith_agent_shell::config::parse(&document).expect("admitted producer configuration");
    let producer = configured.service.limits.events.expect("configured stream writer");
    let registry: toml::Value = toml::from_str(include_str!("../agents/smith/faces.toml")).expect("published registry");
    let entry = registry["face"].as_array().expect("face inventory").last().expect("writing face");
    assert_eq!(entry["shape"].as_str(), Some("exec"));
    let receiving: Receiving = entry["receiving"].clone().try_into().expect("required receiving values");
    assert_eq!(receiving.limits().expect("semantic caps"), producer.sinks.records);
    assert_eq!(receiving.encoded_line, producer.sinks.line_ceiling(producer.capture).expect("joint writer ceiling"));
    smith_events::Reader::with_ceiling(&receiving.limits().expect("preserved caps"), receiving.encoded_line)
        .expect("actual receiving values construct without a Cartesian fallback");
}
