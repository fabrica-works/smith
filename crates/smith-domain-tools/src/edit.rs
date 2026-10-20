//! Ordered edits over current bytes loaded by io. Each replacement matches
//! the result of its predecessors; scratch bytes never change the file. The
//! caller stores the final result once, only over the version it loaded,
//! and only if the edits together changed those bytes.
//! This module knows no earlier reads or filesystem state.
//!
//! Contract: domain/tools.md, sections 3 and 4; programming-model.md, sections 4.4 and 6.3.

use alloc::boxed::Box;

use skein_lib::bytes::{count, find_from};
use skein_lib::{List, Writer};

use crate::call::{Mismatch, Occurrences, Outcome, Replacement};
use crate::limits::Limits;

/// `content` with `edit` made, and how many occurrences it replaced; or the
/// indexed mismatch that refuses it, or `TooLarge` if its scratch result
/// would outgrow the file bound.
pub(crate) fn apply(
    content: &[u8],
    edit: &Replacement,
    number: u32,
    limits: &Limits,
) -> Result<(Box<[u8]>, u32), Outcome> {
    if edit.old.is_empty() {
        return Err(Outcome::EditFailed { edit: number, why: Mismatch::NoMatch });
    }
    let matches = count(content, &edit.old, u32::MAX);
    if matches == 0 {
        return Err(Outcome::EditFailed { edit: number, why: Mismatch::NoMatch });
    }
    let once = match edit.occurrences {
        Occurrences::Once => true,
        Occurrences::All => false,
    };
    if matches > 1 && once {
        return Err(Outcome::EditFailed {
            edit: number,
            why: Mismatch::Ambiguous { count: matches, lines: lines(content, &edit.old, limits.match_lines) },
        });
    }
    let size = size(content.len(), matches, edit.old.len(), edit.new.len()).unwrap_or(u64::MAX);
    if size > u64::from(limits.file_bytes) {
        return Err(Outcome::TooLarge { what: crate::SizePart::EditResult, size, bound: limits.file_bytes });
    }
    let mut writer = Writer::new(usize::try_from(size).expect("no larger than the file limit"));
    let mut from: usize = 0;
    for _ in 0..matches {
        let at = find_from(content, &edit.old, from).expect("counted above");
        writer.put(content.get(from..at).expect("a match is past the last")).expect("sized above");
        writer.put(&edit.new).expect("sized above");
        from = at.checked_add(edit.old.len()).expect("a match is within the content");
    }
    writer.put(content.get(from..).expect("the last match is within the content")).expect("sized above");
    Ok((writer.finish(), matches))
}

/// Apply the ordered list, retaining the loaded bytes for the final comparison
/// without copying them. Only the caller's final store changes the file.
pub(crate) fn apply_edits(
    loaded: Box<[u8]>,
    edits: &[Replacement],
    limits: &Limits,
) -> Result<(Box<[u8]>, u32), Outcome> {
    let mut content: Option<Box<[u8]>> = None;
    let mut number = 0_u32;
    let mut replaced = 0_u32;
    for edit in edits {
        number = number.checked_add(1).ok_or(Outcome::TooLarge {
            what: crate::SizePart::EditItems,
            size: u64::from(number).checked_add(1).expect("u32 increment fits u64"),
            bound: limits.tool_payload,
        })?;
        let (next, count) = apply(content.as_deref().unwrap_or(&loaded), edit, number, limits)?;
        replaced = replaced.checked_add(count).ok_or(Outcome::TooLarge {
            what: crate::SizePart::EditReplacements,
            size: u64::from(replaced).checked_add(u64::from(count)).expect("two u32 counts fit u64"),
            bound: u32::MAX,
        })?;
        content = Some(next);
    }
    if content.as_deref().unwrap_or(&loaded) == loaded.as_ref() {
        return Err(Outcome::EditFailed { edit: number, why: Mismatch::Unchanged });
    }
    Ok((content.expect("changed bytes require at least one edit"), replaced))
}

/// The size of a file of `len` bytes with `matches` occurrences of `old`
/// bytes replaced by `new` bytes, or `None` past a `u64`.
fn size(len: usize, matches: u32, old: usize, new: usize) -> Option<u64> {
    let matches = u64::from(matches);
    let removed = u64::try_from(old).ok()?.checked_mul(matches)?;
    let added = u64::try_from(new).ok()?.checked_mul(matches)?;
    u64::try_from(len).ok()?.checked_sub(removed)?.checked_add(added)
}

/// The numbers of the lines, counting from 1, where the first `max`
/// occurrences of `old` in `content` start.
fn lines(content: &[u8], old: &[u8], max: u32) -> Box<[u32]> {
    let mut lines = List::with_capacity(max);
    let mut line: u32 = 1;
    let mut from: usize = 0;
    let mut counted: usize = 0;
    for _ in 0..max {
        let Some(at) = find_from(content, old, from) else {
            break;
        };
        let before = content.get(counted..at).expect("a match is past the last");
        line = line.saturating_add(count(before, b"\n", u32::MAX));
        counted = at;
        lines.push(line).expect("room for `max` lines");
        from = at.saturating_add(old.len());
    }
    lines.into_boxed()
}
