//! Tagged wire fixtures shared by the process stories.
use skein_lib::{Secret, Writer};
use smith_channel::{CEILINGS, GrantValue, Scheme};

/// A host's bearer wire value.
#[must_use]
pub fn bearer(value: &[u8]) -> GrantValue {
    credential(value, Scheme::Bearer)
}

/// A host's key wire value.
#[must_use]
pub fn key(value: &[u8]) -> GrantValue {
    credential(value, Scheme::Key)
}

fn credential(value: &[u8], scheme: Scheme) -> GrantValue {
    GrantValue::Credential(
        smith_channel::CredentialValue::new(
            &CEILINGS,
            smith_channel::CredentialValueParts { scheme, bytes: Secret::new(Box::from(value)) },
        )
        .expect("bounded credential fixture"),
    )
}

/// A host's withdrawn wire value, removed by accounts-sources.
#[must_use]
pub fn withdrawn(id: &[u8], value: &[u8]) -> GrantValue {
    GrantValue::Withdrawn(
        smith_channel::WithdrawnValue::new(
            &CEILINGS,
            smith_channel::WithdrawnValueParts { id: Box::from(id), bearer: Secret::new(Box::from(value)) },
        )
        .expect("bounded withdrawn fixture"),
    )
}

/// A host's program source wire value.
#[must_use]
pub fn program(name: &[u8]) -> GrantValue {
    GrantValue::Program(
        smith_channel::ProgramValue::new(&CEILINGS, smith_channel::ProgramValueParts { name: Box::from(name) })
            .expect("bounded program fixture"),
    )
}

pub(crate) fn refresh(value: GrantValue) -> skein_channel::Frame {
    let record = smith_channel::GrantRefresh::new(
        &CEILINGS,
        smith_channel::GrantRefreshParts {
            grant: smith_channel::Grant::new(
                &CEILINGS,
                smith_channel::GrantParts {
                    account: 0,
                    generation: 2,
                    valid: skein_lib::Duration::from_secs(7200),
                    value,
                },
            )
            .expect("bounded grant"),
        },
    )
    .expect("refresh");
    let mut body = Writer::new(usize::try_from(record.measure()).expect("body length"));
    record.encode(&mut body).expect("measured refresh");
    let mut frame = skein_channel::frame_writer(0x0104, record.measure()).expect("refresh frame");
    frame.put(&body.finish()).expect("frame body");
    frame.finish().expect("refresh frame")
}
