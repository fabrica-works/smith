//! The shell's optional role reserves the guides at their declared bound.

use super::{Check, Layer, LimitName, Refusal, Value, declarations, profile, validate};

const ROLE_BOUND: u32 = 4096;
const HOST_BELOW_ROLE_AND_GUIDES: u64 = 37_525;

#[test]
fn a_host_part_below_the_role_and_its_guides_is_refused() {
    let selected = profile();
    let mut configuration = declarations();
    let mut limits = crate::profile::derive(&selected, &configuration).expect("profile without role").domain.run;
    configuration.role_bound = Some(ROLE_BOUND);
    limits.host_part = HOST_BELOW_ROLE_AND_GUIDES;
    assert_eq!(
        validate::host_part_holds_the_role(&selected, &configuration, &limits),
        Err(Refusal::Broken {
            check: Check::HostPartHoldsTheRole,
            left: LimitName::deployment(Layer::Run, "role_and_guides").value(Value::Bytes(37_526)),
            right: LimitName::deployment(Layer::Run, "host_part").value(Value::Bytes(37_525)),
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
    let refusal = crate::profile::derive(&profile(), &configuration).expect_err("role and full guides exceed host");
    assert_eq!(refusal.check(), Some(Check::HostPartHoldsTheRole));
}
