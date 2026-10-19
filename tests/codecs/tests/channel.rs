//! Channel kind and body checks (protocol/channel.md, sections 2-4).

use skein_channel::Direction;
use skein_lib::Reader;
use smith_channel::{Answer, CEILINGS, Start, V4, schema};

#[test]
fn channel_goldens_decode_at_their_measured_length() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-channel/golden/v4");
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
    let version = table.version(4).expect("version four");
    assert_eq!(version.kinds.len(), 20);
    for (index, rule) in V4.iter().enumerate() {
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
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-channel/golden/v4");
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
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v4/record_start_smallest.bin")),
    )
    .expect("absent identity golden");
    assert_eq!(absent.identity(), &None);
    let present = Start::decode(
        &CEILINGS,
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v4/record_start_full.bin")),
    )
    .expect("present identity golden");
    assert_eq!(present.identity(), &Some([165; 16]));
}

#[test]
fn channel_turn_goldens_keep_the_declared_window_flag() {
    let first = smith_channel::Turn::decode(
        &CEILINGS,
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v4/record_turn_full.bin")),
    )
    .expect("window flag golden");
    assert!(first.opens_window());
    let ordinary = smith_channel::Turn::decode(
        &CEILINGS,
        &mut Reader::new(include_bytes!("../../../crates/smith-channel/golden/v4/record_turn_smallest.bin")),
    )
    .expect("ordinary flag golden");
    assert!(!ordinary.opens_window());
}

#[test]
fn live_goldens_decode_at_their_measured_length() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-channel/golden/v4");
    for name in ["record_live_smallest.bin", "record_live_full.bin"] {
        let bytes = std::fs::read(root.join(name)).expect("live golden");
        let value = smith_channel::Live::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("live decodes");
        assert_eq!(u32::try_from(bytes.len()).expect("golden length fits"), value.measure());
    }
}

#[test]
fn each_grant_value_form_reads_back_by_its_tag() {
    use smith_channel::{GrantValue, Scheme};
    let rows = [
        (credential_value(Scheme::Bearer), (0, Some(Scheme::Bearer))),
        (credential_value(Scheme::Key), (0, Some(Scheme::Key))),
        (program_value(b"program").expect("bounded program"), (1, None)),
        (withdrawn_value(b"account", 1).expect("bounded withdrawn"), (2, None)),
    ];
    for (value, expected) in rows {
        let mut writer = skein_lib::Writer::new(usize::try_from(value.measure()).expect("fixture size"));
        value.encode(&mut writer).expect("measured grant");
        let bytes = writer.finish();
        let read = GrantValue::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("grant decodes");
        let scheme = match read {
            GrantValue::Credential(value) => Some(value.scheme().clone()),
            GrantValue::Program(_) | GrantValue::Withdrawn(_) => None,
        };
        assert_eq!((bytes[0], scheme), expected);
    }
}

fn credential_value(scheme: smith_channel::Scheme) -> smith_channel::GrantValue {
    smith_channel::GrantValue::Credential(
        smith_channel::CredentialValue::new(
            &CEILINGS,
            smith_channel::CredentialValueParts { scheme, bytes: skein_lib::Secret::new(Box::from(*b"fixture")) },
        )
        .expect("bounded credential"),
    )
}

fn program_value(name: &[u8]) -> Result<smith_channel::GrantValue, smith_channel::v4::Problem> {
    smith_channel::ProgramValue::new(&CEILINGS, smith_channel::ProgramValueParts { name: Box::from(name) })
        .map(smith_channel::GrantValue::Program)
}

fn withdrawn_value(id: &[u8], bearer_length: usize) -> Result<smith_channel::GrantValue, smith_channel::v4::Problem> {
    smith_channel::WithdrawnValue::new(
        &CEILINGS,
        smith_channel::WithdrawnValueParts {
            id: Box::from(id),
            bearer: skein_lib::Secret::new(vec![b'x'; bearer_length].into_boxed_slice()),
        },
    )
    .map(smith_channel::GrantValue::Withdrawn)
}

#[test]
fn a_grant_value_past_its_bounds_is_refused() {
    use skein_codec::Reason;
    let program_bound = usize::try_from(CEILINGS.program_value_name).expect("program ceiling");
    let id_bound = usize::try_from(CEILINGS.withdrawn_value_id).expect("id ceiling");
    let bearer_bound = usize::try_from(CEILINGS.withdrawn_value_bearer).expect("bearer ceiling");
    let rows = [
        ("long program", program_value(&vec![b'a'; program_bound + 1]), Reason::Bound),
        ("invalid program text", program_value(&[255]), Reason::Utf8),
        ("long id", withdrawn_value(&vec![b'a'; id_bound + 1], 1), Reason::Bound),
        ("long bearer", withdrawn_value(b"account", bearer_bound + 1), Reason::Bound),
    ];
    for (name, result, reason) in rows {
        assert_eq!(result.err().map(|problem| problem.reason), Some(reason), "{name}");
    }
}

#[test]
fn grant_values_at_their_bounds_are_admitted() {
    let program_bound = usize::try_from(CEILINGS.program_value_name).expect("program ceiling");
    let id_bound = usize::try_from(CEILINGS.withdrawn_value_id).expect("id ceiling");
    let bearer_bound = usize::try_from(CEILINGS.withdrawn_value_bearer).expect("bearer ceiling");
    for (name, result) in [
        ("program", program_value(&vec![b'a'; program_bound])),
        ("withdrawn", withdrawn_value(&vec![b'a'; id_bound], bearer_bound)),
    ] {
        assert!(result.is_ok(), "{name}");
    }
}

#[test]
fn limited_resets_distinguish_zero_one_and_not_said() {
    use skein_lib::{Duration, Writer};
    for reset in [Some(Duration::ZERO), Some(Duration::from_nanos(1)), None] {
        let value = smith_channel::Limited::new(&CEILINGS, smith_channel::LimitedParts { account: 7, reset })
            .expect("bounded limited notice");
        let mut writer = Writer::new(usize::try_from(value.measure()).expect("notice size"));
        value.encode(&mut writer).expect("measured notice");
        let bytes = writer.finish();
        let read = smith_channel::Limited::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("limited decodes");
        assert_eq!((read.account(), *read.reset()), (7, reset));
    }
}

#[test]
fn refused_notice_fields_read_back_at_both_bounds() {
    let code_bound = usize::try_from(CEILINGS.account_refused_code).expect("code ceiling");
    let reason_bound = usize::try_from(CEILINGS.account_refused_reason).expect("reason ceiling");
    for (code_length, reason_length) in [(0, 0), (code_bound, reason_bound)] {
        let value = smith_channel::AccountRefused::new(
            &CEILINGS,
            smith_channel::AccountRefusedParts {
                account: 7,
                code: vec![b'c'; code_length].into_boxed_slice(),
                reason: vec![b'r'; reason_length].into_boxed_slice(),
            },
        )
        .expect("bounded refusal");
        let mut writer = skein_lib::Writer::new(usize::try_from(value.measure()).expect("notice size"));
        value.encode(&mut writer).expect("measured notice");
        let bytes = writer.finish();
        let read = smith_channel::AccountRefused::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("refusal decodes");
        assert_eq!((read.account(), read.code().len(), read.reason().len()), (7, code_length, reason_length));
    }
}
