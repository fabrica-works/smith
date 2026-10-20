//! Private, unrefreshed key files (domain/host.md, section 7; shell.md, 7.1).
//! Startup keeps `read`'s bytes for in-memory lends; `inspect` reports usability.
use super::Credential;
use std::path::Path;

/// A key inspection's fixed failure class, sent to startup and check without a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyProblem {
    /// Private-file inspection refused group or other mode bits.
    Mode,
    /// The key is empty or exceeds the token bound.
    Size,
    /// The file or its key bytes could not be read as a key.
    Unreadable,
}

impl KeyProblem {
    pub(super) const fn words(self) -> &'static str {
        match self {
            Self::Mode => "has unsafe mode",
            Self::Size => "has invalid size",
            Self::Unreadable => "is unreadable",
        }
    }
}

pub(crate) fn read(file: &Path) -> Result<Box<[u8]>, Credential> {
    if !file.is_absolute() {
        return Err(Credential::NotSignedIn);
    }
    let bound = crate::local_host::token_limits().token_bytes;
    let bytes = skein_shell::read_private_file(
        file,
        bound.checked_add(1).expect("key newline bound"),
        skein_shell::effective_user(),
    )
    .map_err(|why| match why {
        skein_shell::Unread::Absent => Credential::NotSignedIn,
        skein_shell::Unread::TooLarge { .. } => Credential::KeyUnreadable { problem: KeyProblem::Size },
        skein_shell::Unread::Unsafe(skein_io::file::Unsafe::Mode(_)) => {
            Credential::KeyUnreadable { problem: KeyProblem::Mode }
        }
        skein_shell::Unread::NotAFile | skein_shell::Unread::Kernel(_) | skein_shell::Unread::Unsafe(_) => {
            Credential::KeyUnreadable { problem: KeyProblem::Unreadable }
        }
    })?;
    value(bytes.expose())
}

pub(super) fn value(bytes: &[u8]) -> Result<Box<[u8]>, Credential> {
    let bound = crate::local_host::token_limits().token_bytes;
    let value = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if value.is_empty() || u64::try_from(value.len()).expect("key length fits u64") > u64::from(bound) {
        return Err(Credential::KeyUnreadable { problem: KeyProblem::Size });
    }
    if !super::valid_key(value) {
        return Err(Credential::KeyUnreadable { problem: KeyProblem::Unreadable });
    }
    Ok(value.into())
}

pub(super) fn inspect(file: &Path) -> Credential {
    match read(file) {
        Ok(_) => Credential::Usable { until: None },
        Err(credential) => credential,
    }
}

#[cfg(test)]
mod tests;
