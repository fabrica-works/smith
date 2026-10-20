//! Route per-account lock state without reading a record. `lock_event` settles
//! each attempt or schedules a retry (shell.md, 7.3; oauth.md, 6.7).
use super::{LOCK_FILE, RETRY, State};
use crate::credentials::{Accounts, oauth};
use skein_io::file;
use skein_lib::{Time, Wall};

impl Accounts {
    pub(in crate::credentials) fn lock_event(&mut self, event: file::Event, now: Time, wall: Wall) {
        let index = u32::try_from(event.owner().raw() & !LOCK_FILE).expect("lock index");
        let lock = self.locks.get_mut(index).expect("lock index");
        let mut answer = None;
        match event {
            file::Event::Opened { file, .. } => {
                lock.root = Some(file);
                lock.state = State::Take;
            }
            file::Event::Locked { .. } => {
                lock.state = if self.closed { State::Close } else { State::Held };
                answer = Some(oauth::Locked::Held { record: None });
            }
            file::Event::Closed { .. } => {
                lock.state = if lock.again { State::Open } else { State::Idle };
                lock.again = false;
            }
            file::Event::Busy { .. } if lock.until > Some(now) => {
                let until = lock.until.expect("waiting lock deadline");
                lock.state = State::Waiting { retry: now.saturating_add(RETRY).min(until) };
            }
            file::Event::Busy { .. }
            | file::Event::Failed { .. }
            | file::Event::Refused { .. }
            | file::Event::Cancelled { .. } => {
                lock.state = if lock.root.is_some() { State::Close } else { State::Idle };
                answer = Some(oauth::Locked::Busy);
            }
            file::Event::Made { .. }
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
            | file::Event::Listed { .. } => unreachable!("lock vocabulary"),
        }
        let number = lock.number;
        if !self.closed
            && let Some(answer) = answer
        {
            for account in 0..self.entries.len() {
                if self.entries.get(account).expect("account index").number == number {
                    self.down(oauth::Request::Locked { account, answer }, now, wall);
                    return;
                }
            }
        }
    }
}
