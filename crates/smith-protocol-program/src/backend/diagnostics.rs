//! Private payload boundaries (programs.md, sections 5.6 and 9). Debug keeps
//! lengths; upward diagnostics replace the secret and private run names.

use super::{Backend, Below};
use alloc::boxed::Box;
use skein_io::{self as io, file};
use skein_lib::stream;

impl core::fmt::Debug for Below {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (kind, bytes) = match self {
            Below::Io(request) => (
                "Io",
                match request {
                    io::Request::Spawn { spawn, .. } => {
                        let mut bytes = spawn.program.len().saturating_add(spawn.dir.len());
                        for entry in &spawn.args {
                            bytes = bytes.saturating_add(entry.len());
                        }
                        for entry in &spawn.env {
                            bytes = bytes.saturating_add(entry.len());
                        }
                        bytes
                    }
                    io::Request::Output { down, .. } => match down {
                        stream::OutputDown::Send { bytes, .. } => bytes.len(),
                        stream::OutputDown::Room { .. }
                        | stream::OutputDown::Release { .. }
                        | stream::OutputDown::Cancel { .. } => 0,
                    },
                    io::Request::Stream { down, .. } => match down {
                        stream::Down::Send(bytes) => bytes.len(),
                        stream::Down::Demand { .. } | stream::Down::Finish => 0,
                    },
                    io::Request::Listen { .. }
                    | io::Request::Connect { .. }
                    | io::Request::Bind { .. }
                    | io::Request::Reject { .. }
                    | io::Request::Signal { .. }
                    | io::Request::Usage { .. }
                    | io::Request::Window { .. }
                    | io::Request::Close { .. }
                    | io::Request::Abort { .. } => 0,
                },
            ),
            Below::File { request, .. } => (
                "File",
                match request {
                    file::Request::Store { bytes, .. } | file::Request::WriteAt { bytes, .. } => bytes.len(),
                    file::Request::Lock { .. }
                    | file::Request::MakeDirectory { .. }
                    | file::Request::OpenPrivate { .. }
                    | file::Request::Create { .. }
                    | file::Request::CreateNoFollow { .. }
                    | file::Request::OpenRead { .. }
                    | file::Request::OpenReadNoFollow { .. }
                    | file::Request::OpenWrite { .. }
                    | file::Request::OpenWriteNoFollow { .. }
                    | file::Request::OpenDirectory { .. }
                    | file::Request::Load { .. }
                    | file::Request::Scan { .. }
                    | file::Request::Stat { .. }
                    | file::Request::ReadAt { .. }
                    | file::Request::Cut { .. }
                    | file::Request::Sync { .. }
                    | file::Request::Close { .. }
                    | file::Request::SyncDirectory { .. }
                    | file::Request::Rename { .. }
                    | file::Request::Remove { .. }
                    | file::Request::List { .. }
                    | file::Request::ListPage { .. } => 0,
                },
            ),
        };
        formatter.debug_struct(kind).field("payload_bytes", &bytes).finish_non_exhaustive()
    }
}

impl Backend {
    pub(super) fn private_diagnostic(&self, bytes: &[u8]) -> Box<[u8]> {
        let mut bytes = super::diagnostic(bytes, self.settings.receiving.max_failure_bytes);
        self.redact_private(&mut bytes);
        bytes
    }

    pub(super) fn redact_private(&self, bytes: &mut [u8]) {
        let entropy: &[u8; 32] = self.names.mcp.expose().try_into().expect("MCP entropy");
        let secret = crate::mcp::secret_text(entropy);
        let instructions = self.run_path(b"instructions.md");
        let mcp = self.run_path(b"mcp.json");
        for pattern in [secret.as_slice(), instructions.as_ref(), mcp.as_ref(), self.names.run.as_slice()] {
            redact(bytes, pattern);
        }
    }
}

fn redact(bytes: &mut [u8], pattern: &[u8]) {
    if pattern.is_empty() {
        return;
    }
    for start in 0..bytes.len() {
        let end = start.saturating_add(pattern.len());
        if bytes.get(start..end) == Some(pattern) {
            for byte in bytes.get_mut(start..end).expect("matched range") {
                *byte = b'*';
            }
        }
    }
    // The stderr ring or diagnostic ceiling can split a private value.
    // Hide a recognizable prefix/suffix too, without retaining another tail.
    for length in 8..pattern.len() {
        if bytes.get(..length) == pattern.get(pattern.len().saturating_sub(length)..) {
            for byte in bytes.get_mut(..length).expect("matched suffix") {
                *byte = b'*';
            }
        }
        if let Some(start) = bytes.len().checked_sub(length)
            && bytes.get(start..) == pattern.get(..length)
        {
            for byte in bytes.get_mut(start..).expect("matched prefix") {
                *byte = b'*';
            }
        }
    }
}

impl core::fmt::Debug for super::Settings {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Settings")
            .field("input_bytes", &self.input_bytes)
            .field("line_bytes", &self.line_bytes)
            .finish_non_exhaustive()
    }
}

impl core::fmt::Debug for super::BelowEvent {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(event) => {
                formatter.debug_struct("IoEvent").field("kind", &core::mem::discriminant(event)).finish_non_exhaustive()
            }
            Self::File { event, .. } => formatter
                .debug_struct("FileEvent")
                .field("kind", &core::mem::discriminant(event))
                .finish_non_exhaustive(),
        }
    }
}
