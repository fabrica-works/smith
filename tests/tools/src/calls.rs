//! Calls as the protocol layer would decode them from what an LLM wrote.

use skein_lib::Duration;
use smith_domain_tools::Call;

use crate::translate::path;

/// Reads the file at `at` whole, as far as a read goes.
///
/// World contract: domain/tools.md, sections 7 and 9; testing-strategy.md, section 2.2.
#[must_use]
pub fn read(at: &[u8]) -> Call {
    Call::Read { path: path(at), skip: 0, lines: None }
}

/// Reads `lines` lines of the file at `at`, after the first `skip`.
///
/// World contract: domain/tools.md, sections 7 and 9; testing-strategy.md, section 2.2.
#[must_use]
pub fn read_lines(at: &[u8], skip: u32, lines: u32) -> Call {
    Call::Read { path: path(at), skip, lines: Some(lines) }
}

/// Lists the directory at `at`.
///
/// World contract: domain/tools.md, sections 7 and 9; testing-strategy.md, section 2.2.
#[must_use]
pub fn list(at: &[u8]) -> Call {
    Call::List { path: path(at) }
}

/// Makes the file at `at` hold `content`.
///
/// World contract: domain/tools.md, sections 7 and 9; testing-strategy.md, section 2.2.
#[must_use]
pub fn write(at: &[u8], content: &[u8]) -> Call {
    Call::Write { path: path(at), content: content.into() }
}

/// Replaces `old` with `new` in the file at `at`: its one occurrence, or every
/// one if `all`.
///
/// World contract: domain/tools.md, sections 7 and 9; testing-strategy.md, section 2.2.
#[must_use]
pub fn edit(at: &[u8], old: &[u8], new: &[u8], all: bool) -> Call {
    edits(at, &[(old, new, all)])
}

/// One ordered replacement list in the existing tools world.
#[must_use]
pub fn edits(at: &[u8], replacements: &[(&[u8], &[u8], bool)]) -> Call {
    let edits = replacements
        .iter()
        .map(|(old, new, all)| smith_domain_tools::Replacement {
            old: (*old).into(),
            new: (*new).into(),
            occurrences: if *all {
                smith_domain_tools::Occurrences::All
            } else {
                smith_domain_tools::Occurrences::Once
            },
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Call::Edit { path: path(at), edits }
}

/// Runs `command` with the shell, for at most `timeout` if given.
///
/// World contract: domain/tools.md, sections 7 and 9; testing-strategy.md, section 2.2.
#[must_use]
pub fn shell(command: &[u8], timeout: Option<Duration>) -> Call {
    Call::Shell { command: command.into(), timeout }
}

/// Searches the files at and beneath `at` for `pattern`, in those whose names
/// match `glob` if given.
///
/// World contract: domain/tools.md, sections 7 and 9; testing-strategy.md, section 2.2.
#[must_use]
pub fn search(at: &[u8], pattern: &[u8], glob: Option<&[u8]>) -> Call {
    Call::Search { path: path(at), pattern: pattern.into(), glob: glob.map(Into::into) }
}
