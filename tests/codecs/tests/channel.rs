//! Channel kind and body checks (protocol/channel.md, sections 2-4).

use skein_channel::Direction;
use skein_lib::Reader;
use smith_channel::{Answer, CEILINGS, Start, V3, schema};

#[test]
fn channel_goldens_decode_at_their_measured_length() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-channel/golden/v3");
    for name in ["record_start_smallest.bin", "record_start_full.bin"] {
        let bytes = std::fs::read(root.join(name)).expect("start golden");
        let value = Start::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("start decodes");
        assert_eq!(u32::try_from(bytes.len()).expect("golden length fits"), value.measure());
    }
    for name in ["record_answer_smallest.bin", "record_answer_full.bin"] {
        let bytes = std::fs::read(root.join(name)).expect("answer golden");
        let value = Answer::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("answer decodes");
        assert_eq!(u32::try_from(bytes.len()).expect("golden length fits"), value.measure());
    }
}

#[test]
fn every_required_kind_has_its_direction_and_bound_in_the_schema() {
    let table = schema(&CEILINGS).expect("schema bounds fit");
    assert_eq!(table.magic, *b"smth");
    assert_eq!(table.versions.len(), 1);
    let version = table.version(3).expect("version three");
    assert_eq!(version.kinds.len(), 18);
    for (index, rule) in V3.iter().enumerate() {
        assert!(rule.required, "all initial kinds are required");
        let kind = version.kind(rule.kind).expect("required kind is listed");
        assert_eq!(kind.direction, rule.direction);
        if [0x0105, 0x0106, 0x010c].contains(&rule.kind) {
            assert_eq!(kind.largest, 0);
        } else {
            assert!(kind.largest > 0);
        }
        assert_eq!(usize::from(rule.kind), 0x0100 + index);
        if index < 6 {
            assert_eq!(rule.direction, Direction::FromInitiator);
        } else {
            assert_eq!(rule.direction, Direction::FromResponder);
        }
    }
}

#[test]
fn message_refusal_goldens_keep_each_typed_reason() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-channel/golden/v3");
    for name in ["record_message_refused_smallest.bin", "record_message_refused_full.bin"] {
        let bytes = std::fs::read(root.join(name)).expect("refusal golden");
        let value =
            smith_channel::MessageRefused::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("refusal decodes");
        assert_eq!(u32::try_from(bytes.len()).expect("golden length fits"), value.measure());
    }
}

#[test]
fn channel_start_goldens_seal_the_optional_fixed_sixteen_byte_identity() {
    let absent = Start::decode(
        &CEILINGS,
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v3/record_start_smallest.bin")),
    )
    .expect("absent identity golden");
    assert_eq!(absent.identity(), &None);
    let present = Start::decode(
        &CEILINGS,
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v3/record_start_full.bin")),
    )
    .expect("present identity golden");
    assert_eq!(present.identity(), &Some([165; 16]));
}

#[test]
fn channel_turn_goldens_keep_the_declared_window_flag() {
    let first = smith_channel::Turn::decode(
        &CEILINGS,
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v3/record_turn_full.bin")),
    )
    .expect("window flag golden");
    assert!(first.opens_window());
    let ordinary = smith_channel::Turn::decode(
        &CEILINGS,
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v3/record_turn_smallest.bin")),
    )
    .expect("ordinary flag golden");
    assert!(!ordinary.opens_window());
}
