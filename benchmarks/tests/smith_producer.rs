//! The published events face receives the actual frozen control's writer bounds.
//! This crosses the local settings translation, startup derivation and registry;
//! it assumes no global range for task-selected model or policy overrides.

use smith_bench::agent::smith::Receiving;

#[test]
fn the_published_face_preserves_the_native_control_producers_joint_bounds() {
    let source = smith_local_shell::settings::Source {
        layer: smith_local_shell::settings::Layer::User,
        path: "producer.toml".into(),
        text: r"model='openai/gpt-6-luna'
credentials='/synthetic/logins/smith'
[accounts.chatgpt]
provider='openai'
source='sign_in'
[endpoints.openai]
account='chatgpt'
address='127.0.0.1:443'
[deployment]
conversations=6
tool_payload=40960
file_bytes=1048576
calls_per_response=16
read_window=32768
guide=16384
shell_output=16384
shell_head=16384
shell_tail=49152
search_hits=100
search_bytes=16384
list_entries=512
llm_pool=134217728
memory=2281701376
host_tools=2
[models.'openai/gpt-6-luna']
input_price=1.25
cached_price=0.125
output_price=10.0
price_unit=1000000
[budget]
turns=4096
spend=1000.0
seconds=300
[budget.reserve]
turns=2
seconds=150
[trace]
path='/tmp/unused-registry-derivation'
capture='everything'
"
        .into(),
    };
    let settings = smith_local_shell::settings::from_sources(
        &smith_local_shell::command::Shared::default(),
        "/synthetic".into(),
        &[source],
    )
    .unwrap_or_else(|why| panic!("{}", why.line()));
    let (resolved, _document) =
        smith_local_shell::settings::resolve::resolve_startup(&settings).unwrap_or_else(|why| panic!("{}", why.line()));
    let configured = resolved.agent;
    let producer = configured.service.limits.events.expect("configured stream writer");
    let registry: toml::Value = toml::from_str(include_str!("../agents/smith/faces.toml")).expect("published registry");
    let entry = registry["face"].as_array().expect("face inventory").last().expect("writing face");
    assert_eq!(entry["shape"].as_str(), Some("chat"));
    let receiving: Receiving = entry["receiving"].clone().try_into().expect("required receiving values");
    assert_eq!(receiving.limits().expect("semantic caps"), producer.sinks.records);
    assert_eq!(receiving.encoded_line, producer.sinks.line_ceiling(producer.capture).expect("joint writer ceiling"));
    smith_events::Reader::with_ceiling(&receiving.limits().expect("preserved caps"), receiving.encoded_line)
        .expect("actual receiving values construct without a Cartesian fallback");
}
