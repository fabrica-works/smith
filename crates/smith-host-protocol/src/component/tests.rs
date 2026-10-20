//! Grant-form checks before any channel output (protocol/hosts.md, section 2).

use super::{Component, Phase};
use crate::{Error, Values, grant_form};
use alloc::boxed::Box;
use skein_channel::StreamMode;
use skein_lib::{Queue, Token};
use smith_host_domain::{Form, Scheme, SourceName};
use smith_protocol_channel::GrantValue;

fn mismatched_values() -> [GrantValue; 2] {
    [GrantValue::Key(Box::from(b"key".as_slice())), GrantValue::Program(Box::from(b"claude-code".as_slice()))]
}

fn channel(phase: Phase) -> (Component, Queue<crate::OpenEvent>, Queue<skein_channel::Lower>) {
    let limits = crate::tests::process_limits();
    let mut component = Component::new(&limits, StreamMode::Two, crate::tests::endpoints()).expect("channel limits");
    component.phase = phase;
    (component, Queue::with_capacity(limits.channel.output_frames), Queue::with_capacity(limits.channel.kinds))
}

#[test]
fn each_value_lends_its_own_form() {
    for (value, form) in [
        (GrantValue::Bearer(Box::from(b"bearer".as_slice())), Form::Credential(Scheme::Bearer)),
        (GrantValue::Key(Box::from(b"key".as_slice())), Form::Credential(Scheme::Key)),
        (
            GrantValue::Program(Box::from(b"claude-code".as_slice())),
            Form::Program(SourceName::new(b"claude-code").expect("channel name")),
        ),
        (
            GrantValue::Withdrawn {
                account_id: Box::from(b"account".as_slice()),
                bearer: Box::from(b"token".as_slice()),
            },
            Form::Withdrawn,
        ),
    ] {
        assert_eq!(grant_form(&value), Ok(form));
    }
}

#[test]
fn a_program_value_without_a_source_name_has_no_form() {
    for name in [b"".as_slice(), b"A".as_slice(), b"abcdefghijklmnopqrstuvwxyz012345-".as_slice()] {
        assert_eq!(grant_form(&GrantValue::Program(Box::from(name))), Err(Error::Grants));
    }
}

#[test]
fn a_start_whose_value_disagrees_with_its_grant_is_refused() {
    let (mut component, mut to_service, mut below) = channel(Phase::Opened);
    for value in mismatched_values() {
        let mut start = smith_host_world::start();
        start.grants[0].form = Form::Credential(Scheme::Bearer);
        let result = component.send_start(
            start,
            smith_host_domain::Window { turns: 1, bytes: 4096 },
            Values { paths: Box::default(), credentials: Box::from([value]) },
            Token::new(1),
            &mut to_service,
            &mut below,
        );
        assert_eq!(result, Err(Error::Grants));
        assert!(to_service.is_empty());
        assert!(below.is_empty());
        assert_eq!(component.phase, Phase::Opened);
    }
}

#[test]
fn a_refresh_whose_value_disagrees_with_its_grant_is_refused() {
    let (mut component, mut to_service, mut below) = channel(Phase::Admitted);
    for value in mismatched_values() {
        let grant = smith_host_domain::Grant {
            account: 1,
            generation: 2,
            valid: skein_lib::Duration::from_secs(60),
            form: Form::Credential(Scheme::Bearer),
        };
        assert_eq!(component.send_grant(grant, value, Token::new(1), &mut to_service, &mut below), Err(Error::Grants));
        assert!(to_service.is_empty());
        assert!(below.is_empty());
    }
}
