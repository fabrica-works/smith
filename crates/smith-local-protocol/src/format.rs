//! Per-chat layout marker (domain/host.md, section 8; protocol/hosts.md, 5.3).
//! No state or runtime schema parser is retained. `layouts`, `file` and `check`
//! return the committed layouts, their generated record and its refusal.
use crate::{File, local_store_v1 as wire};
use skein_lib::{Reader, Writer};
use smith_local_domain::{Older, StoreFamily};

#[rustfmt::skip]
#[path = "generated/layouts.rs"]
mod committed;

/// The generated family's complete fixed-size Layouts encoding bound.
#[must_use]
pub fn max_bytes() -> u32 {
    u32::try_from(wire::Layouts::worst_case_bytes(&wire::CEILINGS).expect("fixed layouts"))
        .expect("layout encoding fits u32")
}

/// The committed current store and transcript layouts, sent by the drift generator.
#[must_use]
pub fn layouts() -> wire::Layouts {
    wire::Layouts::new(
        &wire::CEILINGS,
        wire::LayoutsParts { store: committed::STORE, transcript: committed::TRANSCRIPT },
    )
    .expect("fixed layouts fit")
}

/// The no-replace marker written only in an admitted empty directory.
#[must_use]
pub fn file() -> File {
    let value = layouts();
    let mut writer = Writer::new(usize::try_from(value.measure()).expect("measured bytes fit usize"));
    value.encode(&mut writer).expect("measured layouts");
    File { name: b"format".as_slice().into(), bytes: writer.finish() }
}

/// Check store first, then transcript; malformed records are store-layout refusals.
pub fn check(bytes: &[u8]) -> Result<(), Older> {
    let value = wire::Layouts::decode(&wire::CEILINGS, &mut Reader::new(bytes))
        .or(Err(Older::Layout(StoreFamily::Store)))?
        .into_parts();
    let current = layouts().into_parts();
    if value.store != current.store {
        return Err(Older::Layout(StoreFamily::Store));
    }
    if value.transcript != current.transcript {
        return Err(Older::Layout(StoreFamily::Transcript));
    }
    Ok(())
}
