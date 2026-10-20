//! Source paths and explicit environments (protocol/programs.md, section 4.2).
//! Retains no state or login; the caller supplies paths and whole environments.

use alloc::boxed::Box;
use skein_lib::List;

/// A host or agent's absolute programs directory and Claude executable (protocol/agent.md, section 4).
#[derive(Debug)]
pub struct Programs {
    pub directory: Box<[u8]>,
    pub claude: Box<[u8]>,
}

/// A refused source binding, returned to the caller before any child starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The source name is outside protocol/channel.md, section 6's set.
    Name,
    /// The programs directory is empty, relative or contains NUL.
    Directory,
    /// An environment entry is malformed or the resulting environment is too large.
    Environment,
}

/// Resolve the source's named configuration directory (protocol/programs.md, 4.2).
pub fn configuration_directory(programs: &Programs, source: &[u8]) -> Result<Box<[u8]>, Error> {
    if source.is_empty() || source.len() > 32 {
        return Err(Error::Name);
    }
    for byte in source {
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && *byte != b'-' {
            return Err(Error::Name);
        }
    }
    if !programs.directory.starts_with(b"/") || programs.directory.contains(&0) {
        return Err(Error::Directory);
    }
    let separator = !programs.directory.ends_with(b"/");
    let length = programs
        .directory
        .len()
        .checked_add(usize::from(separator))
        .ok_or(Error::Directory)?
        .checked_add(source.len())
        .ok_or(Error::Directory)?;
    let mut path = List::with_capacity(match u32::try_from(length) {
        Ok(length) => length,
        Err(_) => return Err(Error::Directory),
    });
    for byte in &programs.directory {
        path.push(*byte).expect("sized path");
    }
    if separator {
        path.push(b'/').expect("sized separator");
    }
    for byte in source {
        path.push(*byte).expect("sized source name");
    }
    Ok(path.into_boxed())
}

/// Whether a caller's variable stays in a source's environment (programs.md, 4.2).
#[must_use]
pub fn keeps(name: &[u8]) -> bool {
    name != b"CLAUDE_CONFIG_DIR"
        && !name.starts_with(b"ANTHROPIC_")
        && name != b"CLAUDE_CODE_OAUTH_TOKEN"
        && !name.starts_with(b"CLAUDE_CODE_USE_")
        && name != b"AWS_BEARER_TOKEN_BEDROCK"
}

/// Maximum filtered environment bytes supplied to a source (protocol/programs.md, 4.2).
pub const ENVIRONMENT_BYTES: usize = 4096;

/// Maximum filtered environment entries supplied to a source (protocol/programs.md, 4.2).
pub const ENVIRONMENT_ENTRIES: u32 = 64;

/// Replace competing login variables with the source's configuration directory.
/// The result holds at most 64 entries and 4096 bytes (programs.md, 4.2).
pub fn environment(base: &[Box<[u8]>], configuration: &[u8]) -> Result<Box<[Box<[u8]>]>, Error> {
    if configuration.contains(&0) {
        return Err(Error::Environment);
    }
    let mut bytes = b"CLAUDE_CONFIG_DIR=".len().checked_add(configuration.len()).ok_or(Error::Environment)?;
    let mut entries = 1_u32;
    for entry in base {
        let equal = skein_lib::bytes::find(entry, b"=").ok_or(Error::Environment)?;
        if entry.contains(&0) {
            return Err(Error::Environment);
        }
        if keeps(entry.get(..equal).expect("entry name")) {
            bytes = bytes.checked_add(entry.len()).ok_or(Error::Environment)?;
            entries = entries.checked_add(1).ok_or(Error::Environment)?;
        }
    }
    if bytes > ENVIRONMENT_BYTES || entries > ENVIRONMENT_ENTRIES {
        return Err(Error::Environment);
    }
    let mut result = List::with_capacity(entries);
    for entry in base {
        let equal = skein_lib::bytes::find(entry, b"=").expect("validated entry");
        if keeps(entry.get(..equal).expect("entry name")) {
            result.push(entry.clone()).expect("counted entries");
        }
    }
    let mut setting = List::with_capacity(
        u32::try_from(b"CLAUDE_CONFIG_DIR=".len().checked_add(configuration.len()).expect("bounded setting"))
            .expect("bounded setting"),
    );
    for byte in b"CLAUDE_CONFIG_DIR=".iter().chain(configuration) {
        setting.push(*byte).expect("sized setting");
    }
    result.push(setting.into_boxed()).expect("configuration entry reserved");
    Ok(result.into_boxed())
}
