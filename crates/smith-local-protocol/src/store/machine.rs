//! Serial durable chat operations (protocol/hosts.md, 5.3; skein io.md, 5.2).
//! The owner retains only record versions and bounded loading buffers. It knows
//! neither the conversation's policy nor whether a turn was acknowledged.
use super::boundary::{Action, History, RecordName, StoreFault, StoreLimits, StoreRequest};
use super::{File, STATE_BYTES, decode_state, decode_turns, save_state, save_turn, turn_opens_window};
use alloc::boxed::Box;
use core::mem::{replace, size_of};
use skein_io::{
    digest::{Digest, digest},
    file::{self, Expect},
    kernel,
};
use skein_lib::{Duration, List, Map, Queue, Time, Token};
use smith_local_domain::{self as local, ChatState, DeliveryRecord};
use smith_protocol_channel::Endpoints;

#[derive(Debug)]
enum Phase {
    Idle,
    Journal,
    State,
    Scan,
    Headers,
    Records,
    Write,
    Remove,
    SyncOld,
    RemoveJournal,
    SyncJournal,
    Closing,
    Closed,
}

#[derive(Debug)]
struct Writing {
    record: RecordName,
    file: File,
    expected: Expect,
    terminal: local::Event,
    retried: bool,
}

/// One serial chat store over an independently adopted root.
#[derive(Debug)]
pub struct Store {
    owner: Token,
    root: Token,
    descriptor: kernel::Fd,
    limits: StoreLimits,
    endpoints: Endpoints,
    phase: Phase,
    loading: bool,
    closing: bool,
    fresh: Option<ChatState>,
    state: Option<ChatState>,
    entries: Box<[file::Entry]>,
    index: usize,
    window: Box<[u8]>,
    total: u64,
    turns: List<Box<[u8]>>,
    deliveries: List<DeliveryRecord>,
    versions: Map<Box<[u8]>, Digest>,
    writing: Option<Writing>,
}

impl Store {
    /// The stored record format version this codec writes and reads.
    pub const FORMAT_VERSION: u64 = super::CHAT_STORE_VERSION;

    /// Adopt a chat directory's token and descriptor; the store closes it explicitly.
    #[must_use]
    pub fn new(
        owner: Token,
        root: Token,
        descriptor: kernel::Fd,
        limits: StoreLimits,
        endpoints: Endpoints,
    ) -> Option<Self> {
        Self::worst_case(limits)?;
        if limits.file_bytes == 0 || limits.turns == 0 || limits.deliveries == 0 || limits.scan_bytes == 0 {
            return None;
        }
        let records = limits.turns.checked_add(limits.deliveries)?.checked_add(2)?;
        Some(Self {
            owner,
            root,
            descriptor,
            limits,
            endpoints,
            phase: Phase::Idle,
            loading: false,
            closing: false,
            fresh: None,
            state: None,
            entries: Box::new([]),
            index: 0,
            window: Box::new([]),
            total: 0,
            turns: List::with_capacity(limits.turns),
            deliveries: List::with_capacity(limits.deliveries),
            versions: Map::with_capacity(records),
            writing: None,
        })
    }

    /// Loading, decoding and a retry's owned replacement can coexist.
    #[must_use]
    pub fn worst_case(limits: StoreLimits) -> Option<u64> {
        let records = limits.turns.checked_add(limits.deliveries)?.checked_add(2)?;
        u64::from(limits.file_bytes)
            .checked_mul(4)?
            .checked_add(limits.scan_bytes)?
            .checked_add(Map::<Box<[u8]>, Digest>::worst_case(records)?)?
            .checked_add(u64::from(records).checked_mul(42)?)?
            .checked_add(List::<Box<[u8]>>::worst_case(limits.turns)?)?
            .checked_add(List::<DeliveryRecord>::worst_case(limits.deliveries)?)?
            .checked_add(u64::try_from(size_of::<Self>()).ok()?)
    }

    /// Room for exactly one serial domain request.
    #[must_use]
    pub fn takes(&self) -> bool {
        self.idle() && !self.closing
    }

    fn idle(&self) -> bool {
        match self.phase {
            Phase::Idle => true,
            Phase::Journal
            | Phase::State
            | Phase::Scan
            | Phase::Headers
            | Phase::Records
            | Phase::Write
            | Phase::Remove
            | Phase::SyncOld
            | Phase::RemoveJournal
            | Phase::SyncJournal
            | Phase::Closing
            | Phase::Closed => false,
        }
    }

    /// Admit one request while idle; its IO requests carry a bounded deadline.
    pub fn request(&mut self, request: StoreRequest, now: Time, out: &mut Queue<Action>) {
        assert!(self.takes(), "the service reserves a serial store slot");
        self.loading = false;
        match request {
            StoreRequest::Load => {
                self.loading = true;
                self.total = 0;
                self.window = Box::new([]);
                self.state = None;
                self.fresh = None;
                self.turns.clear();
                self.deliveries.clear();
                self.versions = Map::with_capacity(self.versions.capacity());
                self.phase = Phase::Journal;
                self.load(b"fresh".as_slice().into(), now, out);
            }
            StoreRequest::SaveState { state, history } => match history {
                History::Kept => self.write(RecordName::State, save_state(state), local::Event::StateSaved, now, out),
                History::Fresh => {
                    self.fresh = Some(state);
                    let mut journal = save_state(state);
                    journal.name = b"fresh".as_slice().into();
                    self.write(RecordName::Journal, journal, local::Event::StateSaved, now, out);
                }
            },
            StoreRequest::SaveTurn { number, read, opens_window, turn } => {
                let record = RecordName::Turn(turn.sequence);
                match save_turn(turn.sequence, read, opens_window, &turn, &smith_transcript::CEILINGS, &self.endpoints)
                {
                    Ok(file) => self.write(record, file, local::Event::TurnSaved { number }, now, out),
                    Err(_) => self.fail(record, StoreFault::Malformed { file: record }, now, out),
                }
            }
            StoreRequest::SaveDelivery { record } => {
                let name = RecordName::Delivery(record.name);
                match crate::save_delivery(&record, self.limits.file_bytes.min(1_u32 << 20_u32)) {
                    Ok(file) => self.write(name, file, local::Event::DeliverySaved { name: record.name }, now, out),
                    Err(_) => self.fail(name, StoreFault::Malformed { file: name }, now, out),
                }
            }
        }
    }

    /// Consume one terminal of the last file request, reserving three output slots.
    pub fn from_io(&mut self, event: file::Event, now: Time, out: &mut Queue<Action>) {
        assert_eq!(event.owner(), self.owner, "store IO echoes its owner");
        let record = self.record();
        match event {
            file::Event::Loaded { bytes, .. } => self.loaded(Some(bytes), now, out),
            file::Event::Scanned { entries, more, .. } => {
                if more != 0 {
                    self.fail(record, StoreFault::TooLarge { file: record }, now, out);
                    return;
                }
                self.entries = entries;
                self.index = 0;
                self.phase = match self.fresh {
                    Some(_) => Phase::Remove,
                    None => Phase::Headers,
                };
                self.advance(now, out);
            }
            file::Event::Stored { digest, .. } => self.stored(digest, now, out),
            file::Event::Failed { error: kernel::Error::NotFound, committed: false, .. } => match self.phase {
                Phase::Journal | Phase::State => self.loaded(None, now, out),
                Phase::Idle
                | Phase::Scan
                | Phase::Headers
                | Phase::Records
                | Phase::Write
                | Phase::Remove
                | Phase::SyncOld
                | Phase::RemoveJournal
                | Phase::SyncJournal
                | Phase::Closing
                | Phase::Closed => {
                    self.fail(record, StoreFault::Io { file: record, error: kernel::Error::NotFound }, now, out);
                }
            },
            file::Event::Failed { committed: true, .. } => {
                let writing = self.writing.as_mut().expect("committed belongs to Store");
                if writing.retried {
                    self.fail(record, StoreFault::NotDurable { file: record }, now, out);
                } else {
                    writing.retried = true;
                    writing.expected = Expect::Digest(digest(&writing.file.bytes));
                    self.send_write(now, out);
                }
            }
            file::Event::Failed { error, .. } => self.fail(record, StoreFault::Io { file: record, error }, now, out),
            file::Event::Conflict { .. } => self.fail(record, StoreFault::Conflict { file: record }, now, out),
            file::Event::TooLarge { .. } => self.fail(record, StoreFault::TooLarge { file: record }, now, out),
            file::Event::Cancelled { .. } => {
                self.fail(record, StoreFault::Io { file: record, error: kernel::Error::TimedOut }, now, out);
            }
            file::Event::Refused { .. } => self.fail(record, StoreFault::Malformed { file: record }, now, out),
            file::Event::Removed { .. } => self.removed(now, out),
            file::Event::Synced { .. } => self.synced(now, out),
            file::Event::Closed { .. } => {
                self.phase = Phase::Closed;
                out.push(Action::Closed);
            }
            file::Event::Made { .. }
            | file::Event::Opened { .. }
            | file::Event::Stated { .. }
            | file::Event::Written { .. }
            | file::Event::Read { .. }
            | file::Event::Renamed { .. }
            | file::Event::Cut { .. }
            | file::Event::Listed { .. } => unreachable!("store requests have whole-file terminals"),
        }
    }

    fn removed(&mut self, now: Time, out: &mut Queue<Action>) {
        match self.phase {
            Phase::Remove => {
                self.index = self.index.checked_add(1).expect("bounded scan");
                self.advance(now, out);
            }
            Phase::RemoveJournal => {
                self.versions.remove(&Box::from(&b"fresh"[..]));
                self.phase = Phase::SyncJournal;
                self.sync(now, out);
            }
            Phase::Idle
            | Phase::Journal
            | Phase::State
            | Phase::Scan
            | Phase::Headers
            | Phase::Records
            | Phase::Write
            | Phase::SyncOld
            | Phase::SyncJournal
            | Phase::Closing
            | Phase::Closed => unreachable!("remove terminal belongs to fresh history"),
        }
    }

    fn synced(&mut self, now: Time, out: &mut Queue<Action>) {
        match self.phase {
            Phase::SyncOld => {
                self.turns.clear();
                self.deliveries.clear();
                let state = self.fresh.expect("fresh history has metadata");
                self.write(RecordName::State, save_state(state), local::Event::StateSaved, now, out);
            }
            Phase::SyncJournal => {
                self.fresh = None;
                if self.loading {
                    self.phase = Phase::Scan;
                    self.scan(now, out);
                } else {
                    self.finish(local::Event::StateSaved, now, out);
                }
            }
            Phase::Idle
            | Phase::Journal
            | Phase::State
            | Phase::Scan
            | Phase::Headers
            | Phase::Records
            | Phase::Write
            | Phase::Remove
            | Phase::RemoveJournal
            | Phase::Closing
            | Phase::Closed => unreachable!("sync terminal belongs to fresh history"),
        }
    }

    /// Settle an outstanding operation before closing the owned root once.
    pub fn close(&mut self, now: Time, out: &mut Queue<Action>) {
        self.closing = true;
        match self.phase {
            Phase::Idle => {
                self.phase = Phase::Closing;
                Self::io(file::Request::Close { owner: self.owner, file: self.root }, now, out);
            }
            Phase::Journal
            | Phase::State
            | Phase::Scan
            | Phase::Headers
            | Phase::Records
            | Phase::Write
            | Phase::Remove
            | Phase::SyncOld
            | Phase::RemoveJournal
            | Phase::SyncJournal
            | Phase::Closing
            | Phase::Closed => {}
        }
    }

    fn io(request: file::Request, now: Time, out: &mut Queue<Action>) {
        out.push(Action::Io { request, deadline: now.saturating_add(Duration::from_secs(30)) });
    }

    fn load(&self, path: Box<[u8]>, now: Time, out: &mut Queue<Action>) {
        let max = match self.phase {
            Phase::Journal | Phase::State => u32::try_from(STATE_BYTES).expect("fixed metadata"),
            Phase::Idle
            | Phase::Scan
            | Phase::Headers
            | Phase::Records
            | Phase::Write
            | Phase::Remove
            | Phase::SyncOld
            | Phase::RemoveJournal
            | Phase::SyncJournal
            | Phase::Closing
            | Phase::Closed => self.limits.file_bytes,
        };
        Self::io(file::Request::Load { owner: self.owner, root: self.root, path, max, no_follow: true }, now, out);
    }

    fn scan(&self, now: Time, out: &mut Queue<Action>) {
        let max = self
            .limits
            .turns
            .checked_add(self.limits.deliveries)
            .expect("admitted scan count")
            .checked_add(2)
            .expect("admitted metadata count");
        Self::io(
            file::Request::Scan {
                owner: self.owner,
                root: self.root,
                path: b".".as_slice().into(),
                max,
                max_bytes: self.limits.scan_bytes,
                no_follow: true,
            },
            now,
            out,
        );
    }

    fn loaded(&mut self, bytes: Option<Box<[u8]>>, now: Time, out: &mut Queue<Action>) {
        let record = self.record();
        match self.phase {
            Phase::Journal | Phase::State => self.metadata(bytes, now, out),
            Phase::Headers => {
                let bytes = bytes.expect("record load exists");
                match turn_opens_window(&bytes) {
                    Ok(true) => self.window = self.path(),
                    Ok(false) => {}
                    Err(_) => {
                        self.fail(record, StoreFault::Malformed { file: record }, now, out);
                        return;
                    }
                }
                self.index = self.index.checked_add(1).expect("bounded scan");
                self.advance(now, out);
            }
            Phase::Records => {
                let bytes = bytes.expect("record load exists");
                self.total = match self.total.checked_add(u64::try_from(bytes.len()).expect("file bytes fit")) {
                    Some(total) if total <= u64::from(self.limits.file_bytes) => total,
                    Some(_) | None => {
                        self.fail(record, StoreFault::TooLarge { file: record }, now, out);
                        return;
                    }
                };
                match record {
                    RecordName::Turn(_) => {
                        if self.turns.push(bytes).is_err() {
                            self.fail(record, StoreFault::TooLarge { file: record }, now, out);
                            return;
                        }
                    }
                    RecordName::Delivery(_) => {
                        if self.versions.insert(self.path(), digest(&bytes)).is_err() {
                            self.fail(record, StoreFault::TooLarge { file: record }, now, out);
                            return;
                        }
                        match crate::decode_delivery(&bytes, self.limits.file_bytes.min(1_u32 << 20_u32)) {
                            Ok(value) => {
                                if self.deliveries.push(value).is_err() {
                                    self.fail(record, StoreFault::TooLarge { file: record }, now, out);
                                    return;
                                }
                            }
                            Err(_) => {
                                self.fail(record, StoreFault::Malformed { file: record }, now, out);
                                return;
                            }
                        }
                    }
                    RecordName::State | RecordName::Journal => unreachable!("record scan selects turns and deliveries"),
                }
                self.index = self.index.checked_add(1).expect("bounded scan");
                self.advance(now, out);
            }
            Phase::Idle
            | Phase::Scan
            | Phase::Write
            | Phase::Remove
            | Phase::SyncOld
            | Phase::RemoveJournal
            | Phase::SyncJournal
            | Phase::Closing
            | Phase::Closed => unreachable!("load terminal belongs to a read"),
        }
    }

    fn metadata(&mut self, bytes: Option<Box<[u8]>>, now: Time, out: &mut Queue<Action>) {
        let record = self.record();
        let state = match bytes {
            None => None,
            Some(bytes) => match decode_state(&bytes) {
                Ok(state) => {
                    self.versions.insert(self.path(), digest(&bytes)).expect("bounded metadata versions");
                    Some(state)
                }
                Err(_) => {
                    self.fail(record, StoreFault::Malformed { file: record }, now, out);
                    return;
                }
            },
        };
        match self.phase {
            Phase::Journal => {
                self.fresh = state;
                self.phase = Phase::State;
                self.load(b"state".as_slice().into(), now, out);
            }
            Phase::State => {
                self.state = state;
                self.phase = Phase::Scan;
                self.scan(now, out);
            }
            Phase::Idle
            | Phase::Scan
            | Phase::Headers
            | Phase::Records
            | Phase::Write
            | Phase::Remove
            | Phase::SyncOld
            | Phase::RemoveJournal
            | Phase::SyncJournal
            | Phase::Closing
            | Phase::Closed => unreachable!("metadata phase"),
        }
    }

    fn advance(&mut self, now: Time, out: &mut Queue<Action>) {
        for _ in 0_u8..2_u8 {
            for index in self.index..self.entries.len() {
                self.index = index;
                let entry = self.entries.get(index).expect("bounded scan index");
                let record = record_name(&entry.name);
                let selected = match self.phase {
                    Phase::Headers => match record {
                        Some(RecordName::Turn(_)) => true,
                        Some(RecordName::State | RecordName::Journal | RecordName::Delivery(_)) | None => false,
                    },
                    Phase::Records => match record {
                        Some(RecordName::Turn(_)) => entry.name >= self.window,
                        Some(RecordName::Delivery(_)) => true,
                        Some(RecordName::State | RecordName::Journal) | None => false,
                    },
                    Phase::Remove => match record {
                        Some(RecordName::Turn(_) | RecordName::Delivery(_)) => true,
                        Some(RecordName::State | RecordName::Journal) | None => false,
                    },
                    Phase::Idle
                    | Phase::Journal
                    | Phase::State
                    | Phase::Scan
                    | Phase::Write
                    | Phase::SyncOld
                    | Phase::RemoveJournal
                    | Phase::SyncJournal
                    | Phase::Closing
                    | Phase::Closed => unreachable!("scan traversal phase"),
                };
                if selected {
                    if self.fresh.is_some() {
                        self.versions.remove(&entry.name);
                        Self::io(
                            file::Request::Remove {
                                owner: self.owner,
                                root: self.descriptor,
                                name: entry.name.clone(),
                            },
                            now,
                            out,
                        );
                    } else {
                        self.load(entry.name.clone(), now, out);
                    }
                    return;
                }
            }
            match self.phase {
                Phase::Headers => {
                    self.phase = Phase::Records;
                    self.index = 0;
                    continue;
                }
                Phase::Records => {
                    match decode_turns(self.turns.as_slice(), &smith_transcript::CEILINGS, &self.endpoints) {
                        Ok((transcript, read)) => {
                            if transcript.is_some() {
                                match &mut self.state {
                                    Some(state) => state.read = read,
                                    None => {
                                        self.fail(
                                            RecordName::State,
                                            StoreFault::Malformed { file: RecordName::State },
                                            now,
                                            out,
                                        );
                                        return;
                                    }
                                }
                            }
                            let deliveries =
                                replace(&mut self.deliveries, List::with_capacity(self.limits.deliveries)).into_boxed();
                            self.turns.clear();
                            self.entries = Box::new([]);
                            self.finish(local::Event::Loaded { state: self.state, transcript, deliveries }, now, out);
                        }
                        Err(_) => {
                            self.fail(RecordName::State, StoreFault::Malformed { file: RecordName::State }, now, out);
                        }
                    }
                }
                Phase::Remove => {
                    self.entries = Box::new([]);
                    self.phase = Phase::SyncOld;
                    self.sync(now, out);
                }
                Phase::Idle
                | Phase::Journal
                | Phase::State
                | Phase::Scan
                | Phase::Write
                | Phase::SyncOld
                | Phase::RemoveJournal
                | Phase::SyncJournal
                | Phase::Closing
                | Phase::Closed => unreachable!("scan exhausted"),
            }
            return;
        }
    }

    fn write(&mut self, record: RecordName, file: File, terminal: local::Event, now: Time, out: &mut Queue<Action>) {
        if file.bytes.len() > usize::try_from(self.limits.file_bytes).expect("file bound fits") {
            self.fail(record, StoreFault::TooLarge { file: record }, now, out);
            return;
        }
        let expected = match record {
            RecordName::Turn(_) => Expect::Absent,
            RecordName::State | RecordName::Delivery(_) | RecordName::Journal => match self.versions.get(&file.name) {
                Some(version) => Expect::Digest(*version),
                None => Expect::Absent,
            },
        };
        self.writing = Some(Writing { record, file, expected, terminal, retried: false });
        self.phase = Phase::Write;
        self.send_write(now, out);
    }

    fn send_write(&self, now: Time, out: &mut Queue<Action>) {
        let writing = self.writing.as_ref().expect("write retains its retry bytes");
        Self::io(
            file::Request::Store {
                owner: self.owner,
                root: self.root,
                path: writing.file.name.clone(),
                bytes: writing.file.bytes.clone(),
                expected: writing.expected,
                no_follow: true,
            },
            now,
            out,
        );
    }

    fn stored(&mut self, digest: Digest, now: Time, out: &mut Queue<Action>) {
        let writing = self.writing.take().expect("stored terminal belongs to a write");
        let retained = match writing.record {
            RecordName::Turn(_) => true,
            RecordName::State | RecordName::Journal | RecordName::Delivery(_) => {
                self.versions.insert(writing.file.name, digest).is_ok()
            }
        };
        if !retained {
            self.fail(writing.record, StoreFault::TooLarge { file: writing.record }, now, out);
            return;
        }
        match writing.record {
            RecordName::Journal => {
                self.phase = Phase::Scan;
                self.scan(now, out);
            }
            RecordName::State if self.fresh.is_some() => {
                self.state = self.fresh;
                self.phase = Phase::RemoveJournal;
                Self::io(
                    file::Request::Remove {
                        owner: self.owner,
                        root: self.descriptor,
                        name: b"fresh".as_slice().into(),
                    },
                    now,
                    out,
                );
            }
            RecordName::State | RecordName::Turn(_) | RecordName::Delivery(_) => {
                self.finish(writing.terminal, now, out);
            }
        }
    }

    fn sync(&self, now: Time, out: &mut Queue<Action>) {
        Self::io(file::Request::SyncDirectory { owner: self.owner, root: self.descriptor }, now, out);
    }

    fn finish(&mut self, event: local::Event, now: Time, out: &mut Queue<Action>) {
        self.phase = Phase::Idle;
        out.push(Action::Domain(event));
        if self.closing {
            self.close(now, out);
        }
    }

    fn fail(&mut self, record: RecordName, fault: StoreFault, now: Time, out: &mut Queue<Action>) {
        out.push(Action::Fault(fault));
        self.writing = None;
        match self.phase {
            Phase::Closing => {
                self.phase = Phase::Closed;
                out.push(Action::Closed);
                return;
            }
            Phase::Idle
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
            | Phase::Closed => {}
        }
        let turn = match record {
            RecordName::Turn(number) => Some(number),
            RecordName::State | RecordName::Delivery(_) | RecordName::Journal => None,
        };
        let reason = if self.loading { local::StoreFailure::Read } else { local::StoreFailure::Write };
        self.finish(local::Event::StoreFailed { turn, reason }, now, out);
    }

    fn path(&self) -> Box<[u8]> {
        match self.phase {
            Phase::Journal | Phase::RemoveJournal => b"fresh".as_slice().into(),
            Phase::Headers | Phase::Records | Phase::Remove => {
                self.entries.get(self.index).expect("selected entry").name.clone()
            }
            Phase::Write => self.writing.as_ref().expect("pending write").file.name.clone(),
            Phase::Idle
            | Phase::State
            | Phase::Scan
            | Phase::SyncOld
            | Phase::SyncJournal
            | Phase::Closing
            | Phase::Closed => b"state".as_slice().into(),
        }
    }

    fn record(&self) -> RecordName {
        record_name(&self.path()).unwrap_or(RecordName::State)
    }
}

fn decimal(bytes: &[u8]) -> Option<u64> {
    let mut value = 0_u64;
    for digit in bytes {
        if !digit.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u64::from(digit.checked_sub(b'0')?))?;
    }
    Some(value)
}

fn record_name(name: &[u8]) -> Option<RecordName> {
    if name == b"state" {
        return Some(RecordName::State);
    }
    if name == b"fresh" {
        return Some(RecordName::Journal);
    }
    if name.len() == 15 && name.ends_with(b".turn") {
        return Some(RecordName::Turn(u32::try_from(decimal(name.get(..10)?)?).ok()?));
    }
    if name.len() == 42 && name.get(20) == Some(&b'.') && name.get(31) == Some(&b'.') {
        return Some(RecordName::Delivery(smith_domain::run::CallName {
            activation: decimal(name.get(..20)?)?,
            completion: u32::try_from(decimal(name.get(21..31)?)?).ok()?,
            position: u32::try_from(decimal(name.get(32..)?)?).ok()?,
        }));
    }
    None
}
