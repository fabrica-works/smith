use super::*;
use skein_lib::stream;
use smith_host_domain::{FailureReason, ProgramFailure, ProgramMethod};

mod fixture;
use fixture::*;
mod ending;
mod passes;
mod queue;

#[test]
fn debug_omits_base_environment_values() {
    let mut fixture = Fixture::new();
    fixture.checks.base = Box::new([b"PATH=/hidden-path".as_slice().into()]);
    let expected = b"Checks { accounts: 3, waiting: 0, base_entries: 1, .. }";
    let mut rendered = DebugOutput(skein_lib::Writer::new(expected.len()));
    core::fmt::write(&mut rendered, format_args!("{:?}", fixture.checks)).unwrap();
    assert_eq!(rendered.0.finish().as_ref(), expected);
}

struct DebugOutput(skein_lib::Writer);

impl core::fmt::Write for DebugOutput {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        match self.0.put(text.as_bytes()) {
            Ok(()) => Ok(()),
            Err(_) => Err(core::fmt::Error),
        }
    }
}

#[test]
fn startup_coverage_is_spent_once() {
    let mut fixture = Fixture::new();
    fixture.request(1);
    assert!(fixture.below.is_empty());
    fixture.shell_account(1);
    fixture.request(1);
    fixture.spawn();
    fixture.finish(br#"{"loggedIn":true,"authMethod":"claude.ai"}"#, 0);
    assert!(fixture.events.is_empty());
    fixture.shell_account(1);
    assert!(!fixture.checks.request(99, Time::ZERO, &mut fixture.below, &mut fixture.shell));
}

#[test]
fn each_failed_check_returns_its_reason() {
    for (bytes, code, reason) in [
        (b"".as_slice(), 1, local::CredentialFailure::ProgramNotSignedIn),
        (
            br#"{"loggedIn":true,"authMethod":"api"}"#.as_slice(),
            0,
            local::CredentialFailure::ProgramSignedInOtherwise { method: ProgramMethod::new(b"api").unwrap() },
        ),
        (
            b"bad".as_slice(),
            0,
            local::CredentialFailure::ProgramFailed(ProgramFailure {
                reason: FailureReason::Unreadable,
                detail: Box::new([]),
            }),
        ),
    ] {
        let mut fixture = Fixture::new();
        fixture.request(1);
        fixture.shell.pop().unwrap();
        fixture.request(1);
        fixture.spawn();
        fixture.finish(bytes, code);
        assert!(fixture.shell.is_empty());
        fixture.refused(1, reason);
        assert!(fixture.events.is_empty());
    }
}
