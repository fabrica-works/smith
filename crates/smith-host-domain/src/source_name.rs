//! Validated program source names (protocol/channel.md, section 6).
//! Keeps only the public name, never credentials. `new` validates the channel's
//! byte set and bound; `as_bytes` returns the name without its zero padding.

/// A host's program source name: 1 to 32 bytes of `a-z`, `0-9` and `-`
/// (protocol/channel.md, section 6).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceName {
    len: u8,
    bytes: [u8; Self::BYTES],
}

impl SourceName {
    /// The channel's maximum program source name length in bytes.
    pub const BYTES: usize = 32;

    /// Refuse an empty name, one past `BYTES`, or a byte outside the channel's set.
    #[must_use]
    pub fn new(name: &[u8]) -> Option<Self> {
        if name.is_empty() || name.len() > Self::BYTES {
            return None;
        }
        for byte in name {
            if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'-' {
                return None;
            }
        }
        let mut bytes = [0; Self::BYTES];
        for (target, byte) in bytes.iter_mut().zip(name) {
            *target = *byte;
        }
        Some(Self { len: u8::try_from(name.len()).ok()?, bytes })
    }

    /// The validated name's bytes, without padding.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..usize::from(self.len)).expect("validated source name length")
    }
}

impl core::fmt::Debug for SourceName {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("SourceName(\"")?;
        for byte in self.as_bytes() {
            // Each validated ASCII source byte occupies one encoded byte.
            let mut character = [0; 1];
            let text = char::from(*byte).encode_utf8(&mut character);
            formatter.write_str(text)?;
        }
        formatter.write_str("\")")
    }
}
