//! Strict task documents, retaining only the manifest and no runtime state.
//! `read_task` refuses unknown keys with a file and key path; task checks are
//! closed enums, following benchmarks.md, sections 3.2 and 5.2.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A task's judgment, supplied by its author to the runner.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A forced behaviour whose checks must all pass.
    Probe,
    /// A small coding task judged by its external grader.
    Fixture,
    /// A frozen repository task judged by its external grader.
    Repository,
}

/// An agent-neutral outcome the task author asks the harness to inspect.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "check", rename_all = "kebab-case", deny_unknown_fields)]
pub enum OutcomeCheck {
    /// The named file must have exactly these UTF-8 bytes.
    FileContent { path: PathBuf, content: String },
    /// The named file must have this SHA-256 digest.
    FileDigest { path: PathBuf, sha256: String },
    /// The named regular file must contain at least this many bytes.
    FileSize { path: PathBuf, bytes_at_least: u64 },
    /// The named workspace path must be absent.
    Absent { path: PathBuf },
    /// The named file must match the given pattern.
    Pattern { path: PathBuf, pattern: String },
    /// The named seed file must retain its original bytes.
    Protected { path: PathBuf },
    /// The settled attempt's terminal classification must have this name.
    End { end: String },
    /// The settled process must return this code, rather than a signal.
    ExitCode { code: i32 },
    /// Standard error's last retained complete line must match the pattern.
    LastLine { pattern: String },
    /// The actual final result text must match the pattern.
    Answer { pattern: String },
    /// The borrowed source login's digest must remain unchanged.
    LoginUnchanged,
}

/// A typed check over smith records, supplied by a task author.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "check", rename_all = "kebab-case", deny_unknown_fields)]
pub enum EventCheck {
    /// One successful captured write supplies the exact settled file bytes.
    WrittenFile { path: PathBuf, content_bytes_at_least: u64 },
    /// Exactly one write is refused by the authored protocol or native payload bound.
    OversizedWrite {
        path: PathBuf,
        content_bytes_above: u64,
        /// Select protocol-only refusal against the actual stated call-input bound.
        protocol_input_bytes_above: Option<u64>,
    },
    /// An oversized write is repaired by a first write and captured extending edits.
    FilePieces {
        path: PathBuf,
        bytes_above: u64,
        /// The authored native `file_bytes` bound, also bounding replay scratch.
        file_bytes: u64,
    },
    /// A captured path call has its exact joined successful terminal.
    ToolPath { tool: String, path: String, verdict: String },
