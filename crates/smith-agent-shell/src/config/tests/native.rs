use super::shaped_document;
use crate::config::{ConfigurationRefusal, from_value, profile_from_value};
use smith_agent_service::profile::{Check, Layer, LimitName, Refusal, Value};

#[test]
fn generated_values_build_the_native_agent_without_a_file() {
    let configuration = from_value(shaped_document()).expect("native configuration");
    assert_eq!(configuration.profile.declared.tool_payload, 256);
    assert_eq!(configuration.declarations.endpoints[0].models[0].window, 65_536);
}

#[test]
fn a_native_value_keeps_both_sides_of_a_profile_refusal() {
    let mut document = shaped_document();
    document["profile"]["declared"]["read_window"] = serde_json::json!(257);
    let refusal = from_value(document).err().expect("read exceeds payload");
    match refusal {
        ConfigurationRefusal::Profile(Refusal::Broken { check, left, right }) => {
            assert_eq!(check, Check::ReadInPayload);
            assert_eq!(left.name, LimitName::deployment(Layer::Declared, "read_window"));
            assert_eq!(left.value, Value::Bytes(257));
            assert_eq!(right.name, LimitName::deployment(Layer::Declared, "tool_payload"));
            assert_eq!(right.value, Value::Bytes(256));
        }
        other @ (ConfigurationRefusal::Input(_) | ConfigurationRefusal::Profile(_)) => {
            panic!("expected typed relationship, got {other:?}");
        }
    }
}

#[test]
fn a_native_value_keeps_a_missing_list_as_a_typed_field() {
    let mut document = shaped_document();
    document["endpoints"] = serde_json::json!([]);
    let refusal = from_value(document).err().expect("missing endpoints");
    assert!(
        matches!(refusal, ConfigurationRefusal::Profile(Refusal::Field { limit })
        if limit == LimitName::deployment(Layer::Declared, "endpoints")),
        "typed missing field"
    );
}

#[test]
fn profile_values_take_check_and_wrap_up_durations() {
    let profile = profile_from_value(serde_json::json!({
        "name":"standard", "policy":{"check_timeout":1234,"wrap_up":5678}
    }))
    .expect("profile values");
    assert_eq!(profile.policy.check_timeout.as_nanos(), 1_234_000_000);
    assert_eq!(profile.policy.wrap_up.as_nanos(), 5_678_000_000);
}
