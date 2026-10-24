//! Crate-local real-directory Store/FileIo adapter for `store` and `qa_store`.
//! No fake, Host, application trait or generic test framework is used.
use skein_io::{
    file,
    file_layer::{self, FileIo},
    kernel::{Complete, Submit},
};
use skein_lib::{Duration, List, Queue, Token};
use skein_scratch::Scratch;
use skein_shell::{Backend, Clock, Config, Kernel, Wait};
use smith_local_domain as local;
use smith_local_protocol::{Action, Store, StoreLimits, StoreRequest};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

/// The whole-file request kinds observed at the Store boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Store asks IO to load a bounded whole file.
    Load,
    /// Store asks IO for a bounded directory scan.
    Scan,
    /// Store asks IO for a durable conditional replacement.
    Store,
    /// Store asks IO to remove one known name.
    Remove,
    /// Store asks IO to sync its directory.
    Sync,
    /// Store asks IO to close an owned root.
    Close,
    /// Store asks IO to open a selected directory.
    Open,
    /// Store asks IO to create a selected private directory.
    Make,
}

/// One immediate directory entry retained by a sorted, no-follow snapshot.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: std::ffi::OsString,
    pub inode: u64,
    pub kind: fs::FileType,
    /// Regular-file contents only; empty for other kinds.
    pub bytes: Vec<u8>,
    pub target: Option<PathBuf>,
}

/// Snapshot names, inodes, kinds, regular-file bytes and symlink targets.
#[must_use]
pub fn snapshot(path: &Path) -> Vec<Entry> {
    let mut entries: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let metadata = fs::symlink_metadata(entry.path()).unwrap();
            let kind = metadata.file_type();
            Entry {
                name: entry.file_name(),
                inode: metadata.ino(),
                kind,
                bytes: if kind.is_file() { fs::read(entry.path()).unwrap() } else { Vec::new() },
                target: if kind.is_symlink() { Some(fs::read_link(entry.path()).unwrap()) } else { None },
            }
        })
        .collect();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

/// Tiny bounds, including capacity for the current marker and interrupted residue.
#[must_use]
pub fn limits() -> StoreLimits {
    StoreLimits { file_bytes: 4096, turns: 2, deliveries: 2, scan_bytes: 4096 }
}

/// The fixture's single main endpoint: number 2, dialect 1, without an address.
#[must_use]
pub fn endpoints() -> smith_protocol_channel::Endpoints {
    let mut entries = List::with_capacity(1);
    entries.push(smith_protocol_channel::Endpoint { name: b"main".as_slice().into(), number: 2, dialect: 1 }).unwrap();
    smith_protocol_channel::Endpoints::new(entries)
}

/// A concrete real directory and the explicitly closed Store/`FileIo`/kernel stack.
#[derive(Debug)]
pub struct Disk {
    scratch: Scratch,
    store: Store,
    limits: StoreLimits,
    io: FileIo,
    kernel: Kernel,
    clock: Clock,
    actions: Queue<Action>,
    events: Queue<file::Event>,
    submissions: Queue<Submit>,
    completions: Queue<Complete>,
    requests: Vec<Kind>,
    faults: Vec<smith_local_protocol::StoreFault>,
    closed: bool,
}

impl Disk {
    /// Create a fresh scratch directory with tiny Store limits on the real readiness backend.
    #[must_use]
    #[expect(
        clippy::new_without_default,
        reason = "a real-directory fixture is constructed and explicitly closed, never defaulted"
    )]
    pub fn new() -> Self {
        Self::bounded(limits())
    }

    /// Create a fresh scratch directory with caller-selected tiny Store bounds.
    #[must_use]
    pub fn bounded(limits: StoreLimits) -> Self {
        let scratch = Scratch::new("smith-store");
        let fd = skein_shell::open_root(scratch.path()).unwrap();
        let scan = limits
            .turns
            .checked_add(limits.deliveries)
            .and_then(|count| count.checked_add(4))
            .expect("tiny scan capacity");
        let mut io = FileIo::with_whole_limit(4, limits.file_bytes, scan, limits.file_bytes, Duration::from_secs(30));
        io.seed_randomness(713);
        let root = io.adopt_root(fd).unwrap();
        let endpoints = endpoints();
        let store = Store::new(Token::new(1), root, fd, limits, endpoints).unwrap();
        Self {
            scratch,
            store,
            io,
            limits,
            kernel: Kernel::open_on(Config { operations: 8 }, Backend::Readiness).expect("real readiness backend"),
            clock: Clock::new(),
            actions: Queue::with_capacity(3),
            events: Queue::with_capacity(1),
            submissions: Queue::with_capacity(8),
            completions: Queue::with_capacity(8),
            requests: Vec::new(),
            faults: Vec::new(),
            closed: false,
        }
    }

    /// The owned scratch path; fixtures can lay files only between serial requests.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.scratch.path()
    }

    /// Close and reopen the scratch root with a fresh unchecked Store instance.
    pub fn reopen(&mut self) {
        self.close();
        let fd = skein_shell::open_root(self.path()).unwrap();
        let scan = self
            .limits
            .turns
            .checked_add(self.limits.deliveries)
            .and_then(|count| count.checked_add(4))
            .expect("tiny scan capacity");
        self.io =
            FileIo::with_whole_limit(4, self.limits.file_bytes, scan, self.limits.file_bytes, Duration::from_secs(30));
        self.io.seed_randomness(714);
        let root = self.io.adopt_root(fd).unwrap();
        self.store = Store::new(Token::new(1), root, fd, self.limits, endpoints()).unwrap();
        self.closed = false;
        self.clear_requests();
    }

    /// Recorded whole-file request kinds since construction or the last clear.
    #[must_use]
    pub fn requests(&self) -> &[Kind] {
        &self.requests
    }

    /// Content-free Store faults observed while driving real IO.
    #[must_use]
    pub fn faults(&self) -> &[smith_local_protocol::StoreFault] {
        &self.faults
    }

    /// Clear boundary request observations before an independently asserted operation.
    pub fn clear_requests(&mut self) {
        self.requests.clear();
    }

    /// Drive one request to exactly one domain terminal through real `FileIo` and kernel.
    pub fn drive(&mut self, request: StoreRequest) -> local::Event {
        assert!(!self.closed);
        self.store.request(request, self.clock.now().now, &mut self.actions);
        self.pump().expect("one domain terminal")
    }

    /// Explicitly drain Store close, both owned roots and all kernel operations.
    pub fn close(&mut self) {
        self.store.close(self.clock.now().now, &mut self.actions);
        assert!(self.pump().is_none());
        assert!(self.closed);
        assert_eq!(self.io.open_files(), 0);
        assert_eq!(self.kernel.in_flight(), 0);
    }

    fn pump(&mut self) -> Option<local::Event> {
        let deadline = self.clock.now().now.saturating_add(Duration::from_secs(5));
        loop {
            let now = self.clock.now().now;
            assert!(now < deadline, "a tiny Store operation settles promptly");
            self.kernel.reap(&mut self.completions);
            while let Some(complete) = self.completions.pop() {
                file_layer::up(&mut self.io, complete, &mut self.events, &mut self.submissions);
            }
            while let Some(event) = self.events.pop() {
                if let file::Event::Opened { file, .. } = &event {
                    self.store.opened_directory_descriptor(self.io.descriptor(*file).unwrap());
                }
                self.store.from_io(event, now, &mut self.actions);
            }
            while let Some(action) = self.actions.pop() {
                match action {
                    Action::Io { request, deadline } => {
                        self.requests.push(kind(&request));
                        file_layer::down_until(
                            &mut self.io,
                            deadline,
                            request,
                            &mut self.events,
                            &mut self.submissions,
                        );
                    }
                    Action::Domain(event) => {
                        assert!(self.actions.is_empty());
                        assert!(self.submissions.is_empty());
                        assert_eq!(self.kernel.in_flight(), 0);
                        return Some(event);
                    }
                    Action::Fault(fault) => self.faults.push(fault),
                    Action::Closed => {
                        self.closed = true;
                        return None;
                    }
                    Action::CleanupWarning { error } => panic!("unexpected cleanup warning {error:?}"),
                }
            }
            if self.io.is_due(now) {
                file_layer::expire(&mut self.io, now, &mut self.submissions);
            }
            let wait = if self.events.is_empty() { Wait::Until(deadline) } else { Wait::No };
            self.kernel.submit(&mut self.submissions, wait);
        }
    }
}

fn kind(request: &file::Request) -> Kind {
    match request {
        file::Request::Load { .. } => Kind::Load,
        file::Request::Scan { .. } => Kind::Scan,
        file::Request::Store { .. } => Kind::Store,
        file::Request::Remove { .. } => Kind::Remove,
        file::Request::SyncDirectory { .. } => Kind::Sync,
        file::Request::Close { .. } => Kind::Close,
        file::Request::OpenDirectory { .. } => Kind::Open,
        file::Request::MakeDirectory { .. } => Kind::Make,
        file::Request::Lock { .. }
        | file::Request::OpenPrivate { .. }
        | file::Request::Create { .. }
        | file::Request::CreateNoFollow { .. }
        | file::Request::OpenRead { .. }
        | file::Request::OpenReadNoFollow { .. }
        | file::Request::OpenWrite { .. }
        | file::Request::OpenWriteNoFollow { .. }
        | file::Request::Stat { .. }
        | file::Request::WriteAt { .. }
        | file::Request::ReadAt { .. }
        | file::Request::Cut { .. }
        | file::Request::Sync { .. }
        | file::Request::Rename { .. }
        | file::Request::List { .. }
        | file::Request::ListPage { .. } => panic!("unexpected Store request {request:?}"),
    }
}
