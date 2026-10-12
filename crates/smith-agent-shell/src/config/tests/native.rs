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

#[test]
fn trust_arrays_accept_der_and_pem_with_system_roots_disabled() {
    use skein_lib::{
        Writer,
        base64::{self, Alphabet},
    };
    let der = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/protocol-llm/fixtures/root.der"));
    let encoded = base64::encoded_len(Alphabet::Standard, u32::try_from(der.len()).expect("root bytes"))
        .expect("encoded root length");
    let mut writer = Writer::new(usize::try_from(encoded).expect("encoded root fits"));
    base64::encode(Alphabet::Standard, der, &mut writer).expect("root encoding");
    let pem = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
        String::from_utf8(writer.finish().into_vec()).expect("base64 text")
    );
    let path = std::env::temp_dir().join(format!("smith-native-trust-{}.pem", std::process::id()));
    std::fs::write(&path, pem).expect("PEM fixture");
    let mut document = shaped_document();
    document["endpoints"][0]["transport"] = serde_json::json!("tls");
    document["endpoints"][0]["server_name"] = serde_json::json!("example.test");
    document["endpoints"][0]["trust"] = serde_json::json!({"system":false,
        "der":[concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/protocol-llm/fixtures/root.der")],
        "pem":[path]});
    let built = from_value(document);
    std::fs::remove_file(path).expect("remove PEM fixture");
    assert!(built.is_ok(), "mixed explicit trust roots accepted");
}

#[test]
fn trust_arrays_with_system_off_and_no_roots_are_refused() {
    let mut document = shaped_document();
    document["endpoints"][0]["transport"] = serde_json::json!("tls");
    document["endpoints"][0]["server_name"] = serde_json::json!("example.test");
    document["endpoints"][0]["trust"] = serde_json::json!({"system":false,"der":[],"pem":[]});
    let refusal = from_value(document).err().expect("no trusted roots").detail();
    assert_eq!(refusal, "TLS trust : Roots");
}

#[test]
fn trust_arrays_keep_one_combined_certificate_bound() {
    let path = std::env::temp_dir().join(format!("smith-combined-trust-{}.pem", std::process::id()));
    std::fs::write(&path, b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n".repeat(256))
        .expect("bounded PEM fixture");
    let mut document = shaped_document();
    document["endpoints"][0]["transport"] = serde_json::json!("tls");
    document["endpoints"][0]["server_name"] = serde_json::json!("example.test");
    document["endpoints"][0]["trust"] = serde_json::json!({"system":false,
        "der":[concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/protocol-llm/fixtures/root.der")],
        "pem":[path,path]});
    let built = from_value(document);
    std::fs::remove_file(path).expect("remove PEM fixture");
    assert_eq!(built.err().expect("combined root bound").detail(), "trust roots exceed their combined bound");
}
