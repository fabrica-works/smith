//! The shell's optional role reserves one guide at its declared bound.

use super::{Check, Layer, LimitName, Refusal, Value, declarations, profile, validate};

const ROLE_BOUND: u32 = 4096;
// 4096 role bytes + 16,384 guide bytes + 95 rendered framing bytes, less one.
const HOST_BELOW_ROLE_AND_GUIDE: u64 = 20_574;

#[test]
fn a_host_part_below_the_role_and_its_guide_is_refused() {
    let selected = profile();
    let mut configuration = declarations();
    let mut limits = crate::profile::derive(&selected, &configuration).expect("profile without role").domain.run;
    configuration.role_bound = Some(ROLE_BOUND);
    limits.host_part = HOST_BELOW_ROLE_AND_GUIDE;
    assert_eq!(
        validate::host_part_holds_the_role(&selected, &configuration, &limits),
        Err(Refusal::Broken {
            check: Check::HostPartHoldsTheRole,
            left: LimitName::deployment(Layer::Run, "role_and_guides").value(Value::Bytes(20_575)),
            right: LimitName::deployment(Layer::Run, "host_part").value(Value::Bytes(20_574)),
        }),
    );
}

#[test]
fn a_profile_without_the_shells_role_skips_the_role_check() {
    let selected = profile();
    let configuration = declarations();
    let limits = crate::profile::derive(&selected, &configuration).expect("small host without role").domain.run;
    assert_eq!(validate::host_part_holds_the_role(&selected, &configuration, &limits), Ok(()));
}

#[test]
fn deriving_a_small_host_with_the_shells_role_refuses() {
    let mut configuration = declarations();
    configuration.role_bound = Some(ROLE_BOUND);
    let refusal = crate::profile::derive(&profile(), &configuration).expect_err("role and one full guide exceed host");
    assert_eq!(refusal.check(), Some(Check::HostPartHoldsTheRole));
}

#[test]
fn the_standard_profile_holds_the_role_and_one_whole_guide_with_four_directories_available() {
    let selected = crate::profile::standard();
    let mut configuration = declarations();
    configuration.role_bound = Some(ROLE_BOUND);
    configuration.endpoints[0].models[0].window = 272_000;
    configuration.endpoints[0].models[0].output = 32_000;
    let limits = crate::profile::derive(&selected, &configuration).expect("standard role and guide").domain.run;
    assert_eq!(limits.directories, 4);
    assert_eq!(limits.guide_bytes, selected.declared.guide);
    assert_eq!(validate::host_part_holds_the_role(&selected, &configuration, &limits), Ok(()));
}
