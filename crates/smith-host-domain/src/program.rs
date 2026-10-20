//! Program source answers (protocol/hosts.md, section 2; protocol/programs.md,
//! sections 3 and 4.1). Keeps typed results and bounded stderr detail; knows
//! no login contents, executable paths or I/O. The half returns these to its host.

use alloc::boxed::Box;

/// A program source check returned by the half to its host (programs.md, section 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramCheck {
    /// The program is signed in through the person's subscription.
    Usable,
    /// The source is not signed in.
    NotSignedIn,
    /// The source is signed in through the named other method.
    SignedInOtherwise(ProgramMethod),
    /// The check could not establish a readable status.
    Failed(ProgramFailure),
}

/// A program sign-out returned by the half to its host (programs.md, section 4.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramSignOut {
    /// The program's sign-out exited successfully.
    SignedOut,
    /// The program's sign-out failed.
    Failed(ProgramFailure),
}

/// A failed source operation returned to the host, with its bounded stderr tail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramFailure {
    pub reason: FailureReason,
    /// Standard error only, bounded by the host's `detail_bytes`.
    pub detail: Box<[u8]>,
}

/// Why the half could not complete the source operation (programs.md, section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    /// The program could not start.
    Missing,
    /// The program did not settle within the host's deadline.
    Deadline,
    /// The status output did not describe a bounded, readable answer.
    Unreadable,
    /// The program exited with an unexpected code or signal.
    Exited,
}

/// A status's authMethod: 1 to 32 printable ASCII bytes, returned to the host.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProgramMethod {
    len: u8,
    bytes: [u8; 32],
}

impl ProgramMethod {
    /// Refuse empty, oversized or nonprintable authentication methods.
    #[must_use]
    pub fn new(method: &[u8]) -> Option<Self> {
        if method.is_empty() || method.len() > 32 {
            return None;
        }
        for byte in method {
            if !(0x21..=0x7e).contains(byte) {
                return None;
            }
        }
        let mut bytes = [0; 32];
        for (target, byte) in bytes.iter_mut().zip(method) {
            *target = *byte;
        }
        Some(Self { len: u8::try_from(method.len()).ok()?, bytes })
    }

    /// The validated method, without its padding.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..usize::from(self.len)).expect("validated method length")
    }
}

impl core::fmt::Debug for ProgramMethod {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("ProgramMethod(\"")?;
        for byte in self.as_bytes() {
            let mut character = [0; 1];
            formatter.write_str(char::from(*byte).encode_utf8(&mut character))?;
        }
        formatter.write_str("\")")
    }
}

#[cfg(test)]
mod tests {
    use super::ProgramMethod;

    #[test]
    fn a_program_method_takes_printable_ascii() {
        for bytes in [b"claude.ai".as_slice(), b"!~", &[b'x'; 32]] {
            let method = ProgramMethod::new(bytes).expect("printable bounded method");
            assert_eq!(method.as_bytes(), bytes);
        }
        for bytes in [b"".as_slice(), b"two words", b"line\n", b"\x7f", b"\x80", &[b'x'; 33]] {
            assert!(ProgramMethod::new(bytes).is_none(), "invalid method {bytes:?}");
        }
    }
}
