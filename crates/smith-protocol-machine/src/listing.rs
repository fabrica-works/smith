//! A bounded tree prefix from sorted, NUL-delimited `rg --files` output.
//! Only selected files and their ancestors exist here; no filesystem, ignore
//! rule or glob is interpreted. Sorted files let one previous path prove which
//! ancestors are new. Selected paths compare name by name, each name as bytes.
//! Depth filters emitted entries, not the selected files.
//! Contract: domain/tools.md, section 3; programming-model.md, sections 4.4 and 6.3.

use alloc::boxed::Box;
use core::{
    cmp::Ordering,
    mem::{replace, size_of},
};

use skein_io::kernel;
use skein_lib::List;
use smith_domain::tools;

/// One listing process's selected paths, ordered retained prefix and exact count.
#[derive(Debug)]
pub struct Listing {
    entries: List<Option<tools::Entry>>,
    kept: u32,
    maximum: u64,
    total: u64,
    depth: tools::Depth,
    line: List<u8>,
    previous: Box<[u8]>,
    cutoff: Option<Box<[u8]>>,
    failed: bool,
}

impl Listing {
    /// Allocate entry slots and one record buffer within the supplied inclusive caps.
    /// Depth levels count direct children as one; levels must be positive.
    #[must_use]
    pub fn new(entries: u32, bytes: u64, depth: tools::Depth, line_bytes: u32) -> Listing {
        assert!(depth != tools::Depth::Levels(0), "listing depth is positive");
        let mut slots = List::with_capacity(entries);
        for _ in 0..entries {
            slots.push(None).expect("one slot per entry");
        }
        Listing {
            entries: slots,
            kept: 0,
            maximum: bytes,
            total: 0,
            depth,
            line: List::with_capacity(line_bytes),
            previous: Box::new([]),
            cutoff: None,
            failed: false,
        }
    }

    /// Consume one sorted selected-file path byte; NUL ends each record.
    pub fn push(&mut self, byte: u8) {
        if self.failed {
            return;
        }
        if byte == 0 {
            self.end_path();
            self.line.clear();
        } else if self.line.push(byte).is_err() {
            // A truncated pathname cannot prove unique ancestors or a count.
            self.failed = true;
        }
    }

    fn end_path(&mut self) {
        let raw = self.line.as_slice();
        let path: Box<[u8]> = match raw.strip_prefix(b"./") {
            Some(path) => Box::from(path),
            None => Box::from(raw),
        };
        if path.is_empty() || compare_paths(&path, &self.previous).is_le() || !relative(&path) {
            self.failed = true;
            return;
        }
        if in_git(&path) {
            return;
        }
        let levels = match self.depth {
            tools::Depth::Whole => u32::MAX,
            tools::Depth::Levels(levels) => levels,
        };
        let mut shared = 0_usize;
        for (left, right) in self.previous.iter().zip(path.iter()) {
            if left != right {
                break;
            }
            shared = shared.checked_add(1).expect("within a bounded path");
        }
        let mut level = 1_u32;
        for (index, byte) in path.iter().enumerate() {
            if *byte == b'/' {
                if index >= shared && level <= levels {
                    self.retain(Box::from(path.get(..index).expect("ancestor prefix")), tools::Kind::Directory);
                }
                level = level.checked_add(1).expect("bounded pathname components");
            }
        }
        if level <= levels {
            self.retain(path.clone(), tools::Kind::File);
        }
        self.previous = path;
    }

    fn retain(&mut self, name: Box<[u8]>, kind: tools::Kind) {
        self.total = self.total.checked_add(1).expect("stream entry count fits u64");
        if let Some(cutoff) = &self.cutoff
            && name.as_ref() >= cutoff.as_ref()
        {
            return;
        }
        let mut at = self.kept;
        for index in 0..self.kept {
            let entry = self.entries.get(index).expect("retained slot").as_ref().expect("retained entry");
            if name.as_ref() < entry.name.as_ref() {
                at = index;
                break;
            }
        }
        let mut carrying = Some(tools::Entry { name, kind });
        for index in at..self.entries.len() {
            carrying = replace(self.entries.get_mut(index).expect("bounded entry slot"), carrying);
            if carrying.is_none() {
                break;
            }
        }
        match carrying {
            Some(entry) => self.cutoff = Some(entry.name),
            None => self.kept = self.kept.checked_add(1).expect("one free entry slot"),
        }
        let mut used = 0_u64;
        let mut keep = self.kept;
        for index in 0..self.kept {
            let entry = self.entries.get(index).expect("retained slot").as_ref().expect("retained entry");
            let cost = u64::try_from(size_of::<tools::Entry>())
                .expect("entry cell fits")
                .checked_add(u64::try_from(entry.name.len()).expect("bounded entry name"))
                .expect("bounded entry cost");
            let next = used.checked_add(cost).expect("bounded entry prefix");
            if next > self.maximum {
                keep = index;
                break;
            }
            used = next;
        }
        for index in keep..self.kept {
            let entry = self.entries.get_mut(index).expect("retained slot").take().expect("retained entry");
            if index == keep {
                self.cutoff = Some(entry.name);
            }
        }
        self.kept = keep;
    }

    /// End the child output after exit and closure, yielding a bounded typed terminal.
    #[must_use]
    pub fn finish(mut self, exit: Option<kernel::Exit>, timed_out: bool) -> tools::Done {
        if timed_out {
            return tools::Done::TimedOut;
        }
        if self.failed || !self.line.is_empty() {
            return other();
        }
        match exit {
            Some(kernel::Exit::Code(0 | 1)) => {}
            Some(kernel::Exit::Code(_) | kernel::Exit::Signal(_)) | None => return other(),
        }
        let mut entries = List::with_capacity(self.kept);
        for index in 0..self.kept {
            entries
                .push(self.entries.get_mut(index).expect("retained slot").take().expect("retained entry"))
                .expect("one result per retained slot");
        }
        let more = self.total.checked_sub(u64::from(self.kept)).expect("retained prefix belongs to the count");
        tools::Done::Listed { entries: entries.into_boxed(), more }
    }
}

fn compare_paths(left: &[u8], right: &[u8]) -> Ordering {
    for (left, right) in left.iter().zip(right.iter()) {
        if left != right {
            if *left == b'/' {
                return Ordering::Less;
            }
            if *right == b'/' {
                return Ordering::Greater;
            }
            return left.cmp(right);
        }
    }
    left.len().cmp(&right.len())
}

fn relative(path: &[u8]) -> bool {
    let mut start = 0_usize;
    for (index, byte) in path.iter().enumerate() {
        if *byte == b'/' {
            if !component(path.get(start..index).expect("path component")) {
                return false;
            }
            start = index.checked_add(1).expect("path byte index");
        }
    }
    component(path.get(start..).expect("last component"))
}

fn component(name: &[u8]) -> bool {
    !name.is_empty() && name != b"." && name != b".."
}

fn in_git(path: &[u8]) -> bool {
    let mut start = 0_usize;
    for (index, byte) in path.iter().enumerate() {
        if *byte == b'/' {
            if path.get(start..index).expect("path component").eq_ignore_ascii_case(b".git") {
                return true;
            }
            start = index.checked_add(1).expect("path byte index");
        }
    }
    path.get(start..).expect("last component").eq_ignore_ascii_case(b".git")
}

fn other() -> tools::Done {
    tools::Done::Failed { fault: tools::Fault::Other }
}
