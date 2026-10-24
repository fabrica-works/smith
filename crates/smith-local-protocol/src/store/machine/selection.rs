//! One chat directory selected between serial store operations (protocol/hosts.md, section 5.3).
//! The old root stays owned until its replacement opens; both roots settle on close.

use super::{Action, Box, Phase, Queue, Store, Time, Token, file, kernel, local};
use skein_lib::Map;

#[derive(Debug)]
enum Selecting {
    Making,
    Opening,
    Retiring,
}

#[derive(Debug)]
/// One candidate directory kept until its open and the old root's close settle.
pub(super) struct Selection {
    name: Box<[u8]>,
    step: Selecting,
    directory: Option<(Token, kernel::Fd)>,
}

impl Store {
    /// Supply the descriptor of the directory token reported by the shared file layer.
    pub fn opened_directory_descriptor(&mut self, descriptor: kernel::Fd) {
        assert!(self.selection.is_some(), "the store only opens directories during selection");
        self.opened_descriptor = Some(descriptor);
    }

    pub(super) fn select_chat(&mut self, name: Box<[u8]>, create: bool, now: Time, out: &mut Queue<Action>) {
        let refused = if self.ephemeral.is_some() {
            Some(local::ChatRefusal::Ephemeral)
        } else if name.len() > 255 || !kernel::is_name(&name) {
            Some(local::ChatRefusal::Name)
        } else {
            None
        };
        if let Some(reason) = refused {
            out.push(Action::Domain(local::Event::ChatSelected { name, result: Err(reason) }));
            return;
        }
        self.phase = Phase::Selecting;
        self.selection = Some(Selection {
            name,
            step: if create { Selecting::Making } else { Selecting::Opening },
            directory: None,
        });
        if create {
            Self::io(
                file::Request::MakeDirectory {
                    owner: self.owner,
                    root: self.chats_root,
                    path: self.selection.as_ref().expect("selected name").name.clone(),
                    mode: 0o700,
                },
                now,
                out,
            );
        } else {
            self.open_selected(now, out);
        }
    }

    fn open_selected(&self, now: Time, out: &mut Queue<Action>) {
        Self::io(
            file::Request::OpenDirectory {
                owner: self.owner,
                root: self.chats_descriptor,
                name: self.selection.as_ref().expect("selected name").name.clone(),
                no_follow: true,
            },
            now,
            out,
        );
    }

    pub(super) fn selection_event(&mut self, event: file::Event, now: Time, out: &mut Queue<Action>) {
        match event {
            file::Event::Made { .. } => {
                match self.selection.as_ref().expect("selection owns its terminal").step {
                    Selecting::Making => {}
                    Selecting::Opening | Selecting::Retiring => unreachable!("only directory creation emits Made"),
                }
                self.selection.as_mut().expect("selection owns its terminal").step = Selecting::Opening;
                self.open_selected(now, out);
            }
            file::Event::Opened { file, .. } => {
                match self.selection.as_ref().expect("selection owns its terminal").step {
                    Selecting::Opening => {}
                    Selecting::Making | Selecting::Retiring => unreachable!("only selection's open emits Opened"),
                }
                let selection = self.selection.as_mut().expect("selection owns its terminal");
                selection.directory =
                    Some((file, self.opened_descriptor.take().expect("file layer's opened descriptor")));
                if self.root == self.chats_root {
                    self.selected(now, out);
                } else {
                    selection.step = Selecting::Retiring;
                    Self::io(file::Request::Close { owner: self.owner, file: self.root }, now, out);
                }
            }
            file::Event::Closed { .. } => match self.selection.as_ref().expect("selection owns its terminal").step {
                Selecting::Retiring => self.selected(now, out),
                Selecting::Making | Selecting::Opening => unreachable!("only the previous root emits Closed"),
            },
            file::Event::Failed { error, .. } => {
                let reason = match error {
                    kernel::Error::NotFound => local::ChatRefusal::Missing,
                    kernel::Error::Refused
                    | kernel::Error::Busy
                    | kernel::Error::Reset
                    | kernel::Error::BrokenPipe
                    | kernel::Error::NotConnected
                    | kernel::Error::AddressInUse
                    | kernel::Error::AddressNotAvailable
                    | kernel::Error::Unreachable
                    | kernel::Error::TimedOut
                    | kernel::Error::TooManyOpenFiles
                    | kernel::Error::NoBufferSpace
                    | kernel::Error::Cancelled
                    | kernel::Error::TooLate
                    | kernel::Error::Exists
                    | kernel::Error::NotADirectory
                    | kernel::Error::IsADirectory
                    | kernel::Error::NotEmpty
                    | kernel::Error::Permission
                    | kernel::Error::NoSpace
                    | kernel::Error::ReadOnly
                    | kernel::Error::TooManyLinks
                    | kernel::Error::NameTooLong
                    | kernel::Error::Escape
                    | kernel::Error::NotAFile
                    | kernel::Error::InvalidArgument
                    | kernel::Error::Other(_) => local::ChatRefusal::Io,
                };
                self.selection_failed(reason, now, out);
            }
            file::Event::Cancelled { .. } | file::Event::Refused { .. } => {
                self.selection_failed(local::ChatRefusal::Io, now, out);
            }
            file::Event::Locked { .. }
            | file::Event::Busy { .. }
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
            | file::Event::Removed { .. }
            | file::Event::Listed { .. } => unreachable!("selection only creates, opens and retires directories"),
        }
    }

    fn selected(&mut self, now: Time, out: &mut Queue<Action>) {
        let selection = self.selection.take().expect("selection owns its terminal");
        let (root, descriptor) = selection.directory.expect("selection opened its new root");
        self.root = root;
        self.descriptor = descriptor;
        self.checked = false;
        self.state = None;
        self.fresh = None;
        self.window = Box::new([]);
        self.entries = Box::new([]);
        self.turns.clear();
        self.deliveries.clear();
        self.versions = Map::with_capacity(self.versions.capacity());
        self.finish(local::Event::ChatSelected { name: selection.name, result: Ok(()) }, now, out);
    }

    fn selection_failed(&mut self, reason: local::ChatRefusal, now: Time, out: &mut Queue<Action>) {
        let selection = self.selection.take().expect("selection owns its terminal");
        if let Some((root, descriptor)) = selection.directory {
            self.checked = false;
            self.root = root;
            self.descriptor = descriptor;
            out.push(Action::Domain(local::Event::StoreFailed { turn: None, reason: local::StoreFailure::Write }));
        }
        self.opened_descriptor = None;
        self.finish(local::Event::ChatSelected { name: selection.name, result: Err(reason) }, now, out);
    }
}
