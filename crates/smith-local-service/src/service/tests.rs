//! Service-owned values supply the domain's forms (domain/host.md, section 7).

use super::{Service, local};
use alloc::boxed::Box;
use skein_lib::Duration;
use smith_host_domain::{Form, Scheme};
use smith_protocol_channel::GrantValue;

fn grant() -> smith_domain::Grant {
    smith_domain::Grant { name: smith_domain::GrantName { account: 7, generation: 1 }, valid: Duration::from_secs(60) }
}

#[test]
fn a_lent_value_sets_its_credential_events_form() {
    let mut service = Service::new(crate::tests::config(), 7).expect("bounded service");
    for (value, form) in [
        (GrantValue::Bearer(Box::from(b"token".as_slice())), Form::Credential(Scheme::Bearer)),
        (
            GrantValue::Withdrawn { account_id: Box::from(b"acc".as_slice()), bearer: Box::from(b"token".as_slice()) },
            Form::Withdrawn,
        ),
    ] {
        service.credential(grant(), value);
        let Some(local::Event::Credential { grant: lent, form: lent_form }) = service.local_events.pop() else {
            panic!("credential terminal");
        };
        assert_eq!(lent, grant());
        assert_eq!(lent_form, form);
    }
}

#[test]
fn a_value_without_a_form_answers_missing() {
    let mut service = Service::new(crate::tests::config(), 7).expect("bounded service");
    service.credential(grant(), GrantValue::Program(Box::from(b"Bad_Name".as_slice())));
    let Some(local::Event::NoCredential { account, reason }) = service.local_events.pop() else {
        panic!("missing credential terminal");
    };
    assert_eq!(account, 7);
    assert_eq!(reason, local::CredentialFailure::Missing);
    assert!(service.values.is_empty());
}
