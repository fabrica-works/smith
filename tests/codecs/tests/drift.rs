//! Frozen generated code and bytes follow codec.md, section 6.

use std::{fs, path::Path};

use skein_codegen::{generate, parse};

#[test]
fn charter_schema_matches_generated_code_and_all_goldens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-charter");
    let source = fs::read_to_string(root.join("schema/charter-v3.schema")).expect("charter schema");
    let schema = parse(&source).expect("schema valid");
    let output = generate(&schema);
    let committed = fs::read_to_string(root.join("src/generated/v3.rs")).expect("generated code");
    assert_eq!(committed, output.rust, "regenerate charter codec with skein-codegen");
    for golden in output.goldens {
        let committed = fs::read(root.join("golden/v3").join(&golden.name)).expect("golden bytes");
        assert_eq!(committed, golden.bytes, "golden {} drifted", golden.name);
    }
}

#[test]
fn transcript_schema_matches_generated_code_and_all_goldens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-transcript");
    let source = fs::read_to_string(root.join("schema/transcript-v6.schema")).expect("transcript schema");
    let schema = parse(&source).expect("schema valid");
    let output = generate(&schema);
    let committed = fs::read_to_string(root.join("src/generated/v6.rs")).expect("generated code");
    assert_eq!(committed, output.rust, "regenerate transcript codec with skein-codegen");
    for golden in output.goldens {
        let committed = fs::read(root.join("golden/v6").join(&golden.name)).expect("golden bytes");
        assert_eq!(committed, golden.bytes, "golden {} drifted", golden.name);
    }
}

#[test]
fn channel_schema_matches_generated_code_and_all_goldens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-channel");
    let source = fs::read_to_string(root.join("schema/channel-v4.schema")).expect("channel schema");
    let schema = parse(&source).expect("schema valid");
    let output = generate(&schema);
    let committed = fs::read_to_string(root.join("src/generated/v4.rs")).expect("generated code");
    assert_eq!(committed, output.rust, "regenerate channel codec with skein-codegen");
    for golden in output.goldens {
        let committed = fs::read(root.join("golden/v4").join(&golden.name)).expect("golden bytes");
        assert_eq!(committed, golden.bytes, "golden {} drifted", golden.name);
    }
}

#[path = "support/events.rs"]
mod events;

#[test]
fn event_records_and_every_listed_value_match_frozen_version_three_goldens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-events/golden/v3");
    let limits = smith_events::Limits { string: 64, content: 1024, items: 8 };
    let samples = events::samples();
    assert_eq!(fs::read_dir(&root).expect("event goldens").count(), samples.len(), "every golden is covered");
    check_event_goldens(&root, &limits, &samples);
}

fn check_event_goldens(
    root: &Path,
    limits: &smith_events::Limits,
    rows: &[(&str, smith_events::Record, smith_events::Capture)],
) {
    for (name, record, capture) in rows {
        let frozen = fs::read(root.join(format!("{name}.jsonl"))).expect("frozen event golden");
        let ceiling = u32::try_from(frozen.len()).expect("golden length");
        let encoded = smith_events::write_with_ceiling(record, capture, limits, ceiling)
            .expect("bounded fixture")
            .expect("event");
        assert_eq!(encoded.as_ref(), frozen, "event {name} drifted");
        let decoded =
            smith_events::read_with_ceiling(&frozen, limits, ceiling).expect("valid golden").expect("known record");
        let rewritten = smith_events::write_with_ceiling(&decoded, capture, limits, ceiling)
            .expect("bounded decoded fixture")
            .expect("event");
        assert_eq!(rewritten.as_ref(), frozen, "golden {name} reads back");
    }
}
