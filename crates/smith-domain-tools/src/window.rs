//! Byte windows selected by a read caller, bounded by the tools' payload ceiling.
//! It keeps no file state and knows no version; window selects the configured
//! bytes and read slices a loaded file into whole lines, or one cut line.
//! Contract: domain/tools.md, sections 3 and 4; programming-model.md, sections 4.4 and 6.3.

use crate::{Budget, BytesClamp, Cut, Limits, Next, Outcome};
use alloc::boxed::Box;
use skein_lib::bytes::{self, copy_of};

/// A read's effective byte budget and the requested budget's clamp.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Window {
    pub(crate) bytes: u32,
    pub(crate) clamp: BytesClamp,
}

/// The read's one-based starting line and its already selected byte window.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Span {
    pub(crate) offset: u32,
    pub(crate) window: Window,
}

/// Apply the default or clamp a requested byte window to the payload ceiling.
pub(crate) fn window(asked: Budget, limits: &Limits) -> Window {
    let bytes = match asked {
        Budget::Default => limits.read_bytes,
        Budget::Asked(bytes) => bytes,
    };
    if bytes > limits.tool_payload {
        Window { bytes: limits.tool_payload, clamp: BytesClamp::Clamped { asked: bytes, ceiling: limits.tool_payload } }
    } else {
        Window { bytes, clamp: BytesClamp::Within }
    }
}

/// The selected tool answer and the suffix its own window omitted, before rendering.
pub(crate) struct Read {
    pub(crate) outcome: Outcome,
    pub(crate) omitted_bytes: u64,
}

/// Return whole lines within the effective window, or the cut first line.
pub(crate) fn read(content: Box<[u8]>, span: Span) -> Read {
    let total = count(&content);
    let skipped = span.offset.checked_sub(1).expect("admitted one-based offset").min(total);
    let mut start = 0_usize;
    for _ in 0..skipped {
        start = next_line(&content, start);
    }
    let rest = content.get(start..).expect("line starts within content");
    let max = usize::try_from(span.window.bytes).expect("u32 fits usize");
    let mut end = 0_usize;
    let mut taken = 0_u32;
    for _ in 0..total {
        if end >= rest.len() {
            break;
        }
        let line_end = next_line(rest, end);
        if line_end > max {
            break;
        }
        end = line_end;
        taken = taken.checked_add(1).expect("file bounded by u32");
    }
    let cut = if taken == 0 && !rest.is_empty() {
        end = max;
        taken = 1;
        Cut::Line
    } else {
        Cut::Whole
    };
    let after = skipped.checked_add(taken).expect("returned lines fit file count");
    let next = if after < total { Next::Line(after.checked_add(1).expect("next line within file")) } else { Next::End };
    let first = skipped.checked_add(1).expect("admitted one-based offset fits u32");
    let omitted_bytes = u64::try_from(rest.len().checked_sub(end).expect("selected window within remaining file"))
        .expect("bounded file bytes fit u64");
    let content = if start == 0 && end == content.len() {
        content
    } else {
        copy_of(rest.get(..end).expect("window within file"))
    };
    Read {
        outcome: Outcome::Read { content, first, lines: taken, total, next, cut, window: span.window.clamp },
        omitted_bytes,
    }
}

fn count(content: &[u8]) -> u32 {
    let ended = bytes::count(content, b"\n", u32::MAX);
    match content.last() {
        Some(b'\n') | None => ended,
        Some(_) => ended.checked_add(1).expect("file bounded by u32"),
    }
}

fn next_line(content: &[u8], start: usize) -> usize {
    match bytes::find_from(content, b"\n", start) {
        Some(at) => at.checked_add(1).expect("newline within bounded content"),
        None => content.len(),
    }
}
