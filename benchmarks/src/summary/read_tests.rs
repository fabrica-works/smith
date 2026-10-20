//! Summary decoding keeps the value decoder's decisions without its ordinary allocation pass.

use super::{VALUE_DECODES, decode_summary, decode_summary_value};
use std::path::Path;

const SUMMARY: &str = r#"{
    "version":4,"result_version":4,"priced_at":"recorded",
    "run":"sample","suite":"sample","tier":"small","design":"single",
    "seed":17,"date":"2026-10-20","attempts":[],"groups":[],
    "comparisons":[],"audits":[],"drift_notices":[]
}"#;

#[test]
fn a_supported_summary_needs_no_value_tree() {
    let file = Path::new("summary.json");
    let expected = decode_summary_value(file, SUMMARY).expect("supported summary");
    VALUE_DECODES.with(|count| count.set(0));
    let summary = decode_summary(file, SUMMARY).expect("supported summary");
    assert_eq!(VALUE_DECODES.with(std::cell::Cell::get), 0, "no intermediate value decode");
    assert_eq!(summary, expected);
}

#[test]
fn summary_fallback_keeps_the_value_decoders_decisions() {
    let file = Path::new("summary.json");
    let mut unknown = serde_json::from_str::<serde_json::Value>(SUMMARY).expect("summary object");
    unknown["version"] = 2.into();
    unknown.as_object_mut().expect("object").remove("attempts").expect("removed attempts");
    let mut malformed = serde_json::from_str::<serde_json::Value>(SUMMARY).expect("summary object");
    malformed["attempts"] = false.into();
    // Duplicate keys are deliberate raw JSON: a Value would erase the duplicate.
    let duplicate = format!("{{\"run\":\"overwritten\",{}", SUMMARY.strip_prefix('{').expect("object"));
    for text in ["{".into(), "[]".into(), unknown.to_string(), malformed.to_string(), duplicate] {
        let expected = decode_summary_value(file, &text);
        let decoded = decode_summary(file, &text);
        match (decoded, expected) {
            (Ok(decoded), Ok(expected)) => assert_eq!(decoded, expected),
            (Err(decoded), Err(expected)) => {
                assert_eq!(decoded.file, expected.file);
                assert_eq!(decoded.key, expected.key);
                assert_eq!(decoded.reason, expected.reason);
            }
            (decoded, expected) => panic!("different summary decisions: {decoded:?}, {expected:?}"),
        }
    }
}
