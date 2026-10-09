//! Stable opaque run identity (domain/run.md, section 3.2; programming-model.md,
//! section 9). This module retains no state and knows no credentials or host
//! naming policy. Hosts construct identities; admission draws absent ones from
//! its injected random state.

use skein_lib::Rng;

/// Sixteen opaque bytes supplied by the host or drawn at run admission.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Identity([u8; 16]);

impl Identity {
    /// Preserve the host's exact bytes, including an all-zero identity.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Identity {
        Identity(bytes)
    }

    /// The exact opaque bytes for another boundary representation.
    #[must_use]
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }

    pub(crate) fn draw(rng: &mut Rng) -> Identity {
        let low = u128::from(rng.next_u64());
        let high = u128::from(rng.next_u64()).checked_shl(64).expect("fixed upper half shift");
        Identity((low | high).to_le_bytes())
    }
}
