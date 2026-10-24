//! The pre-recovery layout check (domain/host.md, section 8).
//! Only complete empty scans and regular reserved marker residue permit creation.
use super::{Action, Box, Phase, Queue, RecordName, Store, StoreFault, Time, file, kernel, local};

impl Store {
    pub(super) fn load_current(&mut self, now: Time, out: &mut Queue<Action>) {
        self.phase = Phase::Journal;
        self.load(b"fresh".as_slice().into(), now, out);
    }

    pub(super) fn compatibility_event(&mut self, event: file::Event, now: Time, out: &mut Queue<Action>) {
        match event {
            file::Event::Loaded { bytes, .. } => match crate::format::check(&bytes) {
                Ok(()) => {
                    self.checked = true;
                    self.load_current(now, out);
                }
                Err(why) => self.older(why, now, out),
            },
            file::Event::Failed { error: kernel::Error::NotFound, committed: false, .. } => match self.phase {
                Phase::Format => {
                    self.phase = Phase::Admit;
                    self.scan(now, out);
                }
                Phase::Admit => self.older(local::Older::Unrecorded, now, out),
                Phase::CreateFormat => self.marker_failed(kernel::Error::NotFound, now, out),
                Phase::Idle
                | Phase::Selecting
                | Phase::Journal
                | Phase::State
                | Phase::Scan
                | Phase::Headers
                | Phase::Records
                | Phase::Write
                | Phase::Remove
                | Phase::SyncOld
                | Phase::RemoveJournal
                | Phase::SyncJournal
                | Phase::Cleaning
                | Phase::Closing
                | Phase::Closed => unreachable!("compatibility phases only"),
            },
            file::Event::Scanned { entries, more, .. } => {
                if more != 0 {
                    self.older(local::Older::Unrecorded, now, out);
                    return;
                }
                for entry in &entries {
                    if entry.kind != kernel::Kind::File || !marker_residue(&entry.name) {
                        self.older(local::Older::Unrecorded, now, out);
                        return;
                    }
                }
                let marker = crate::format::file();
                self.phase = Phase::CreateFormat;
                Self::io(
                    file::Request::Store {
                        owner: self.owner,
                        root: self.root,
                        path: marker.name,
                        bytes: marker.bytes,
                        expected: file::Expect::Absent,
                        no_follow: true,
                    },
                    now,
                    out,
                );
            }
            file::Event::Stored { .. } => {
                self.checked = true;
                self.finish(local::Event::Loaded { state: None, transcript: None, deliveries: Box::new([]) }, now, out);
            }
            file::Event::Failed { error, .. } => self.marker_failed(error, now, out),
            file::Event::Conflict { .. } => {
                self.fail(RecordName::State, StoreFault::Conflict { file: RecordName::State }, now, out);
            }
            file::Event::TooLarge { .. } => {
                self.marker_fault(StoreFault::TooLarge { file: RecordName::State }, now, out);
            }
            file::Event::Cancelled { .. } => self.marker_failed(kernel::Error::TimedOut, now, out),
            file::Event::Refused { .. } => self.marker_failed(kernel::Error::NotAFile, now, out),
            file::Event::Locked { .. }
            | file::Event::Busy { .. }
            | file::Event::Made { .. }
            | file::Event::Opened { .. }
            | file::Event::Stated { .. }
            | file::Event::Written { .. }
            | file::Event::Read { .. }
            | file::Event::Renamed { .. }
            | file::Event::Cut { .. }
            | file::Event::Listed { .. }
            | file::Event::Removed { .. }
            | file::Event::Synced { .. }
            | file::Event::Closed { .. } => unreachable!("marker loads, scans and stores have whole-file terminals"),
        }
    }

    fn marker_failed(&mut self, error: kernel::Error, now: Time, out: &mut Queue<Action>) {
        self.marker_fault(StoreFault::Io { file: RecordName::State, error }, now, out);
    }

    fn marker_fault(&mut self, fault: StoreFault, now: Time, out: &mut Queue<Action>) {
        match self.phase {
            Phase::Admit => self.older(local::Older::Unrecorded, now, out),
            Phase::Format | Phase::CreateFormat => {
                self.fail(RecordName::State, fault, now, out);
            }
            Phase::Idle
            | Phase::Selecting
            | Phase::Journal
            | Phase::State
            | Phase::Scan
            | Phase::Headers
            | Phase::Records
            | Phase::Write
            | Phase::Remove
            | Phase::SyncOld
            | Phase::RemoveJournal
            | Phase::SyncJournal
            | Phase::Cleaning
            | Phase::Closing
            | Phase::Closed => unreachable!("compatibility failures only"),
        }
    }

    fn older(&mut self, why: local::Older, now: Time, out: &mut Queue<Action>) {
        self.finish(
            local::Event::StoreFailed { turn: None, reason: local::StoreFailure::OlderBuild { why } },
            now,
            out,
        );
    }
}

fn marker_residue(name: &[u8]) -> bool {
    if name.len() != 29 || !name.starts_with(b"format.skein-") {
        return false;
    }
    for byte in name.get(13..).expect("reserved prefix length") {
        if !byte.is_ascii_digit() && !(b'a'..=b'f').contains(byte) {
            return false;
        }
    }
    true
}
