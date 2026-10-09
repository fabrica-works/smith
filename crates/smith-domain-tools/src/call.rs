//! The calls a session's LLM makes and what comes of them: the vocabulary the
//! session and the protocol layer share with the tools.
//!
//! The protocol layer owns each tool's schema, decodes the JSON the LLM wrote
//! into a [`Call`], valid by construction, and renders an [`Outcome`] as the
//! text the LLM reads. A malformed call never gets here: the session answers
//! it. What an outcome holds, such as how much of a file comes back, is the
//! tools' decision.
//!
//! Contract: domain/tools.md, sections 3 and 4; programming-model.md, sections 4.4 and 6.3.

use alloc::boxed::Box;

use skein_lib::Duration;

use crate::path::{Part, Path, Resolved};

/// A call the LLM made to one of the tools.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Call {
    /// Read the file at `path`: its lines after the first `skip`, at most
    /// `lines` of them if given, within the tools' limit on what a read
    /// answers with. Any read counts as reading the file at the version io
    /// loaded, however little of it the window shows: a change needs the
    /// current version seen, not all of it.
    Read {
        path: Path,
        /// Number of whole lines skipped before the returned read window.
        skip: u32,
        /// Maximum number of lines to return when present.
        lines: Option<u32>,
    },
    /// List matching files and their directories beneath `path`, within the requested depth.
    List { path: Path, depth: Depth, glob: Option<Box<[u8]>> },
    /// Search the files at and beneath `path` for `pattern`, a regular
    /// expression as rg reads it, in those whose names match `glob` if given.
    Search {
        path: Path,
        /// Search expression passed as data to the process-group search.
        pattern: Box<[u8]>,
        /// Optional search filename filter, passed as data rather than an option.
        glob: Option<Box<[u8]>>,
    },
    /// Make the file at `path` hold `content`, creating it if there is none.
    Write {
        path: Path,
        /// Owned content blocks or file bytes, bounded by the enclosing record's byte cap.
        content: Box<[u8]>,
    },
    /// Apply replacements in order, store once over the loaded version, or apply none.
    /// No earlier read is required; ends in `Edited`, `EditFailed` or a file terminal.
    Edit { path: Path, edits: Box<[Replacement]> },
    /// Run `command` with the shell, in the working directory, for at most
    /// `timeout` if given and within the tools' own limit.
    Shell {
        /// Owned shell command interpreted only by the process-group layer.
        command: Box<[u8]>,
        /// Maximum time allowed for this completion or command.
        timeout: Option<Duration>,
    },
}

/// One ordered replacement sent by the protocol layer with an edit call.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Replacement {
    pub old: Box<[u8]>,
    pub new: Box<[u8]>,
    pub occurrences: Occurrences,
}

/// The matching rule sent with one replacement; the tools enforce it on the current bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Occurrences {
    /// Exactly one match is required; ends in a replacement or an ambiguity refusal.
    Once,
    /// Every non-overlapping match is replaced; at least one must exist.
    All,
}

/// The depth a listing caller asks for, counting direct children as one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Depth {
    /// List the whole matching tree.
    Whole,
    /// List at most this many levels; the caller supplies at least one.
    Levels(u32),
}

/// Why the tools refused one replacement without storing any part of the edit.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Mismatch {
    /// The old text is absent, including an empty snippet.
    NoMatch,
    /// A unique replacement matched count times; retained lines count from one.
    Ambiguous { count: u32, lines: Box<[u32]> },
    /// The old and new bytes are identical.
    Unchanged,
}

/// Which tool a call is for, without its arguments.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tool {
    /// Read-only call or successful read fact, according to the enclosing classification.
    Read,
    /// Directory-listing tool.
    List,
    /// Contained-search tool.
    Search,
    /// A call that may modify state and must run alone.
    Write,
    /// Version-aware file-edit tool.
    Edit,
    /// Contained shell-command tool.
    Shell,
}

/// The tool `call` is for.
#[must_use]
pub const fn tool(call: &Call) -> Tool {
    match call {
        Call::Read { .. } => Tool::Read,
        Call::List { .. } => Tool::List,
        Call::Search { .. } => Tool::Search,
        Call::Write { .. } => Tool::Write,
        Call::Edit { .. } => Tool::Edit,
        Call::Shell { .. } => Tool::Shell,
    }
}

/// What a call does to the checkout. Calls that only read may run side by
/// side; a call that writes runs alone.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// Read-only call or successful read fact, according to the enclosing classification.
    Read,
    /// A call that may modify state and must run alone.
    Write,
}

/// The effect of `call`. A command may do anything, so it writes.
#[must_use]
pub const fn effect(call: &Call) -> Effect {
    match call {
        Call::Read { .. } | Call::List { .. } | Call::Search { .. } => Effect::Read,
        Call::Write { .. } | Call::Edit { .. } | Call::Shell { .. } => Effect::Write,
    }
}

/// What came of a call: exactly one per call.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Outcome {
    /// A read: `content` is the lines after the first `skipped`, `lines` of
    /// them, of the `total` the file has. They are whole lines, but for a
    /// line longer than what a read answers with, which is `cut` at that
    /// limit; a read moves by whole lines, so the rest of a cut line cannot
    /// be read (a search, or a command, can reach it).
    Read {
        /// Owned content blocks or file bytes, bounded by the enclosing record's byte cap.
        content: Box<[u8]>,
        /// Whole file lines skipped before this read window.
        skipped: u32,
        /// Number of lines returned in `content`.
        lines: u32,
        /// Total file lines observed at the loaded version.
        total: u32,
        /// Bytes omitted outside the retained output window.
        cut: bool,
    },
    /// A listing: the selected tree's first entries in path order, and how many
    /// `more` it has.
    Listed {
        /// First bounded directory entries in name order.
        entries: Box<[Entry]>,
        /// Problems or matches omitted after the retained bound.
        more: u64,
    },
    /// A search: the first lines that matched, in path order, and how many
    /// `more` did; `timed_out` if the search ran out of time, and these are
    /// what it had found by then.
    Found {
        /// Bounded search results, or the caller's maximum retained count.
        hits: Box<[Hit]>,
        /// Problems or matches omitted after the retained bound.
        more: u64,
        /// Whether IO ended the process or search at its deadline.
        timed_out: bool,
    },
    /// The file holds what was written; it was `created` if there was none.
    Written {
        /// Whether this write created a previously absent file.
        created: bool,
    },
    /// The edit `replaced` that many occurrences.
    Edited {
        /// Number of edit occurrences successfully replaced.
        replaced: u32,
    },
    /// The command ended so, having written its output (standard output and
    /// standard error together, as they came): the first bytes of it in
    /// `head`, the last in `tail`, and `dropped` bytes between them that were
    /// not kept.
    Exited {
        /// Terminal process classification; only a zero exit code passes checks.
        exit: Exit,
        /// First output bytes retained under the caller's cap.
        head: Box<[u8]>,
        /// Last output bytes retained under the caller's cap.
        tail: Box<[u8]>,
        /// Bytes omitted between the retained process output head and tail.
        dropped: u64,
    },

    /// The kit was not granted the call's family of tools.
    NotGranted,
    /// The path is outside every repository of the checkout, or a symbolic
    /// link on the way to it leads out of its repository.
    Outside { path: Resolved },
    /// The path is in a repository the kit may not write.
    ReadOnly { path: Resolved },
    /// The path is longer than the tools take.
    TooLong,
    /// Nothing is at the path.
    NotFound { path: Resolved },
    /// What is at the path is not a regular file: a directory, a device.
    NotFile,
    /// The path of a write or an edit goes through a symbolic link, which
    /// they do not follow, so that a change lands only in the repository its
    /// path names. Reads follow links that stay inside the path's own
    /// repository.
    Linked,
    /// The path of a write or an edit is in a repository's git directory: it
    /// has `.git`, in any ASCII case, among its names. The tools do not know
    /// git directories under other names (a `.git` file pointing elsewhere, a
    /// separate git directory, hooks configured to live in the tree), so the
    /// worker must not trust a repository's own configuration either.
    Protected,
    /// What is at the path, or a directory on the way to it, is not a
    /// directory.
    NotDirectory { path: Resolved },
    /// The file, or the content to write, is `size` bytes: more than the
    /// tools load or store.
    TooLarge {
        /// Observed file size which exceeds the receiving byte cap.
        size: u64,
    },
    /// A write names an existing file whose version the LLM has not read.
    NotRead,
    /// A write or edit lost the version it read or matched before its store.
    Stale,
    /// The first failed replacement, counting from one; no edit was stored.
    EditFailed { edit: u32, why: Mismatch },
    /// io failed.
    Failed {
        /// Typed IO failure; this boundary carries no OS error text.
        fault: Fault,
    },
    /// The call did not finish by its deadline.
    TimedOut,
    /// The kit closed while the call was running.
    Cancelled,
    /// The kit has as many calls running as it may.
    Busy,
    /// The command, the pattern or the glob holds a NUL byte, which no
    /// argument to a process can.
    NulByte,
}

/// A line a search found.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Hit {
    /// The file it is in, beneath the path searched: names joined by `/`,
    /// empty if the path searched is the file.
    pub path: Box<[u8]>,
    /// Its number, counting from 1.
    pub line: u32,
    /// The line, without its end, cut where the search's byte limit fell.
    pub text: Box<[u8]>,
}

/// A selected file or its ancestor, relative to the directory listed.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Entry {
    /// A nonempty relative path beneath the listed directory, joined by `/`.
    pub name: Box<[u8]>,
    /// Typed entry classification or byte label required by the enclosing contract.
    pub kind: Kind,
}

/// What an entry is, as io found it, without following symbolic links.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symbolic link.
    Link,
    /// A device, a socket, a pipe.
    Other,
}

/// How a command ended.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Exit {
    /// It exited with `code`.
    Code { code: u8 },
    /// A signal killed it.
    Signal {
        /// Signal number which ended the process.
        signal: u8,
    },
    /// It ran past its deadline, and io killed it, with every process it
    /// started.
    TimedOut,
}

/// Why io failed, as the protocol layer classifies the error.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Fault {
    /// Permission denied.
    Denied,
    /// No space left, or over a quota.
    NoSpace,
    /// Anything else.
    Other,
}

impl Call {
    /// Checked complete ownership of this decoded call: inline cell plus every
    /// Path/Part/Name wrapper and byte payload, content, pattern, glob or command.
    /// The protocol/root reuse this before effects; None is arithmetic overflow.
    #[must_use]
    pub fn owned_bytes(&self) -> Option<u64> {
        let payload = match self {
            Call::Read { path, skip: _, lines: _ } => owned_path(path),
            Call::List { path, depth: _, glob } => {
                let filter = match glob {
                    Some(filter) => owned_len(filter)?,
                    None => 0,
                };
                owned_path(path)?.checked_add(filter)
            }
            Call::Search { path, pattern, glob } => {
                let glob = match glob {
                    Some(glob) => owned_len(glob)?,
                    None => 0,
                };
                owned_path(path)?.checked_add(owned_len(pattern)?)?.checked_add(glob)
            }
            Call::Write { path, content } => owned_path(path)?.checked_add(owned_len(content)?),
            Call::Edit { path, edits } => {
                let mut cost = owned_path(path)?.checked_add(
                    u64::try_from(size_of::<Replacement>()).ok()?.checked_mul(u64::try_from(edits.len()).ok()?)?,
                )?;
                for replacement in edits {
                    cost = cost.checked_add(owned_len(&replacement.old)?)?.checked_add(owned_len(&replacement.new)?)?;
                }
                Some(cost)
            }
            Call::Shell { command, timeout: _ } => owned_len(command),
        };
        u64::try_from(size_of::<Self>()).ok()?.checked_add(payload?)
    }
}

fn owned_path(path: &Path) -> Option<u64> {
    let parts = u64::try_from(size_of::<Part>()).ok()?.checked_mul(u64::try_from(path.parts.len()).ok()?)?;
    let mut cost = parts;
    for part in &path.parts {
        match part {
            Part::Name { name } => cost = cost.checked_add(owned_len(name.as_bytes())?)?,
            Part::Current | Part::Parent => {}
        }
    }
    Some(cost)
}

fn owned_len(bytes: &[u8]) -> Option<u64> {
    u64::try_from(bytes.len()).ok()
}
