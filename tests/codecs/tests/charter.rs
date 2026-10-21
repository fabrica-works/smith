//! Charter and result wire boundaries (protocol/charter.md, sections 3-6).

use skein_codec::Reason;
use skein_lib::Reader;
use smith_charter::v4::{CEILINGS, Charter, Path, RunResult};

#[test]
fn charter_and_result_golden_records_decode() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/smith-charter/golden/v4");
    for name in ["record_charter_smallest.bin", "record_charter_full.bin"] {
        let bytes = std::fs::read(root.join(name)).expect("charter golden");
        let value = Charter::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("charter decodes");
        assert_eq!(u32::try_from(bytes.len()).expect("golden length fits"), value.measure());
    }
    for name in ["record_run_result_smallest.bin", "record_run_result_full.bin"] {
        let bytes = std::fs::read(root.join(name)).expect("result golden");
        let value = RunResult::decode(&CEILINGS, &mut Reader::new(&bytes)).expect("result decodes");
        assert_eq!(u32::try_from(bytes.len()).expect("golden length fits"), value.measure());
    }
}

#[test]
fn charter_rejects_an_unsupported_version_before_fields() {
    let problem = Charter::decode(&CEILINGS, &mut Reader::new(&[0, 1])).expect_err("version one");
    assert_eq!((problem.path, problem.reason), (Path::CharterVersion, Reason::Version));
}

#[test]
fn llm_goldens_keep_the_window_and_output_as_separate_quantities() {
    use smith_charter::v4::Llm;
    let full = include_bytes!("../../../crates/smith-charter/golden/v4/record_llm_full.bin");
    let value = Llm::decode(&CEILINGS, &mut Reader::new(full)).expect("full LLM entry");
    assert_eq!((value.window(), value.output()), (u32::MAX, u32::MAX));
    let smallest = include_bytes!("../../../crates/smith-charter/golden/v4/record_llm_smallest.bin");
    let value = Llm::decode(&CEILINGS, &mut Reader::new(smallest)).expect("smallest LLM entry");
    assert_eq!((value.window(), value.output()), (0, 0));
}

#[test]
fn prices_goldens_keep_an_optional_separate_cache_write_rate() {
    use smith_charter::v4::Prices;
    let full = include_bytes!("../../../crates/smith-charter/golden/v4/record_prices_full.bin");
    let decoded = Prices::decode(&CEILINGS, &mut Reader::new(full)).expect("full prices");
    assert_eq!(*decoded.cache_write(), Some(u64::MAX));
    let smallest = include_bytes!("../../../crates/smith-charter/golden/v4/record_prices_smallest.bin");
    let decoded = Prices::decode(&CEILINGS, &mut Reader::new(smallest)).expect("smallest prices");
    assert_eq!(*decoded.cache_write(), None, "absent write rate inherits input in the domain");
}

#[test]
fn llm_goldens_carry_an_account_or_none() {
    for (bytes, expected) in [
        (include_bytes!("../../../crates/smith-charter/golden/v4/record_llm_smallest.bin").as_slice(), None),
        (include_bytes!("../../../crates/smith-charter/golden/v4/record_llm_full.bin").as_slice(), Some(u32::MAX)),
    ] {
        let value = smith_charter::Llm::decode(&CEILINGS, &mut Reader::new(bytes)).expect("LLM golden");
        assert_eq!(*value.account(), expected);
    }
}

#[test]
fn earlier_charter_and_result_versions_are_refused() {
    for version in [1, 2, 3] {
        let problem = Charter::decode(&CEILINGS, &mut Reader::new(&[0, version])).expect_err("older charter");
        assert_eq!((problem.path, problem.reason), (Path::CharterVersion, Reason::Version));
        let problem = RunResult::decode(&CEILINGS, &mut Reader::new(&[0, version])).expect_err("older result");
        assert_eq!((problem.path, problem.reason), (Path::RunResultVersion, Reason::Version));
    }
}

#[test]
fn both_modes_round_trip_in_the_version_four_charter() {
    use skein_lib::Writer;
    use smith_charter::Mode;
    let smallest = include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin");
    for (bytes, mode) in [
        (include_bytes!("../../../crates/smith-charter/golden/v4/enum_mode_finite_full.bin").as_slice(), Mode::Finite),
        (include_bytes!("../../../crates/smith-charter/golden/v4/enum_mode_open_full.bin").as_slice(), Mode::Open),
    ] {
        assert_eq!(Mode::decode(&CEILINGS, &mut Reader::new(bytes)).expect("mode golden"), mode);
        let mut writer = Writer::new(usize::try_from(mode.measure()).expect("bounded mode"));
        mode.encode(&mut writer).expect("measured mode");
        assert_eq!(writer.finish().as_ref(), bytes);
        let mut parts = Charter::decode(&CEILINGS, &mut Reader::new(smallest)).expect("smallest charter").into_parts();
        parts.mode = mode;
        let charter = Charter::new(&CEILINGS, parts).expect("charter mode");
        let mut writer = Writer::new(usize::try_from(charter.measure()).expect("bounded encoding"));
        charter.encode(&mut writer).expect("measured bytes");
        assert_eq!(Charter::decode(&CEILINGS, &mut Reader::new(&writer.finish())), Ok(charter));
    }
}
