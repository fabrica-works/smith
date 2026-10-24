//! Private chat cleanup by this store's written names (shell.md, section 4.2).
//! No directory content is learned. A failed removal keeps the directory;
//! closing the two roots reports the warning without changing the answer.
use super::{Action, Box, Phase, Queue, Store, Time, Token, file, kernel};

#[derive(Debug)]
enum Step {
    Files,
    Root,
    Directory,
    Parent,
}

#[derive(Debug)]
pub(super) struct Ephemeral {
    parent: Token,
    descriptor: kernel::Fd,
    name: Box<[u8]>,
    step: Step,
    warning: Option<kernel::Error>,
}

impl Store {
    /// Mark a fresh private root for cleanup, with its independently owned parent.
    pub fn ephemeral(&mut self, parent: Token, descriptor: kernel::Fd, name: Box<[u8]>) {
        assert!(
            self.takes() && self.ephemeral.is_none() && kernel::is_name(&name),
            "private cleanup is configured before loading"
        );
        self.checked = true;
        self.ephemeral = Some(Ephemeral { parent, descriptor, name, step: Step::Files, warning: None });
    }

    /// Settle an outstanding operation before closing the roots once.
    pub fn close(&mut self, now: Time, out: &mut Queue<Action>) {
        self.closing = true;
        match self.phase {
            Phase::Idle if self.ephemeral.is_some() => {
                self.phase = Phase::Cleaning;
                self.cleanup_next(now, out);
            }
            Phase::Idle => {
                self.phase = Phase::Closing;
                Self::io(file::Request::Close { owner: self.owner, file: self.root }, now, out);
            }
            Phase::Selecting
            | Phase::Format
            | Phase::Admit
            | Phase::CreateFormat
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
            | Phase::Closed => {}
        }
    }

    fn cleanup_next(&mut self, now: Time, out: &mut Queue<Action>) {
        let cleanup = self.ephemeral.as_mut().expect("private close keeps its roots");
        let request = match cleanup.step {
            Step::Files => match self.versions.iter().next() {
                Some((name, _)) => file::Request::Remove {
                    owner: self.owner,
                    root: self.descriptor,
                    name: name.clone(),
                    directory: false,
                },
                None => {
                    cleanup.step = Step::Root;
                    file::Request::Close { owner: self.owner, file: self.root }
                }
            },
            Step::Root => file::Request::Close { owner: self.owner, file: self.root },
            Step::Directory => file::Request::Remove {
                owner: self.owner,
                root: cleanup.descriptor,
                name: cleanup.name.clone(),
                directory: true,
            },
            Step::Parent => file::Request::Close { owner: self.owner, file: cleanup.parent },
        };
        Self::io(request, now, out);
    }

    pub(super) fn cleanup_event(&mut self, event: file::Event, now: Time, out: &mut Queue<Action>) {
        let error = match event {
            file::Event::Removed { .. } | file::Event::Closed { .. } => None,
            file::Event::Failed { error, .. } => Some(error),
            file::Event::Cancelled { .. } => Some(kernel::Error::Cancelled),
            file::Event::Locked { .. }
            | file::Event::Busy { .. }
            | file::Event::Made { .. }
            | file::Event::Refused { .. }
            | file::Event::Opened { .. }
            | file::Event::Loaded { .. }
            | file::Event::Scanned { .. }
            | file::Event::Stored { .. }
            | file::Event::Conflict { .. }
            | file::Event::TooLarge { .. }
            | file::Event::Stated { .. }
            | file::Event::Written { .. }
            | file::Event::Read { .. }
            | file::Event::Cut { .. }
            | file::Event::Synced { .. }
            | file::Event::Renamed { .. }
            | file::Event::Listed { .. } => {
                unreachable!("private cleanup only removes and closes")
            }
        };
        let cleanup = self.ephemeral.as_mut().expect("private close keeps its roots");
        if cleanup.warning.is_none() {
            cleanup.warning = error;
        }
        cleanup.step = match cleanup.step {
            Step::Files => {
                let name = self.versions.iter().next().expect("one removal was submitted").0.clone();
                self.versions.remove(&name);
                Step::Files
            }
            Step::Root if cleanup.warning.is_none() => Step::Directory,
            Step::Root | Step::Directory => Step::Parent,
            Step::Parent => {
                if let Some(error) = cleanup.warning {
                    out.push(Action::CleanupWarning { error });
                }
                self.phase = Phase::Closed;
                out.push(Action::Closed);
                return;
            }
        };
        self.cleanup_next(now, out);
    }

    pub(super) fn retain_written(&mut self, name: Box<[u8]>, digest: skein_io::digest::Digest) {
        if self.ephemeral.is_some() {
            self.versions.insert(name, digest).expect("private writes reserve their known name before IO");
        }
    }
}
