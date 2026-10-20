//! Private keeper requests, queued cancellation and owner settlement.
use super::{Accounts, Origin, oauth};
use skein_io::{file, file_layer, kernel};
use skein_lib::{Time, Token, Wall};

impl Accounts {
    /// Start this account's explicit login, without asking for a run's grant.
    pub fn sign_in(&mut self, number: u32) -> bool {
        if self.closing.is_some() || self.closed {
            return false;
        }
        for index in 0..self.entries.len() {
            let account = self.entries.get_mut(index).expect("account index");
            if account.number == number {
                match &mut account.source {
                    Origin::SignIn { sign_in, signing, .. } if !*signing && !account.loading => {
                        *sign_in = true;
                        account.startup = true;
                        return true;
                    }
                    Origin::SignIn { .. } | Origin::Env { .. } => return false,
                }
            }
        }
        false
    }

    pub(super) fn settle_close(&mut self, now: Time, wall: Wall) {
        if self.closing.is_none() || self.close_sent {
            return;
        }
        if self.closing == Some(smith_agent_service::Close::Aborted) {
            self.close_sent = true;
            self.down(oauth::Request::Abort, now, wall);
            return;
        }
        for index in 0..self.entries.len() {
            let account = self.entries.get_mut(index).expect("account index");
            match &mut account.source {
                Origin::SignIn { signing, sign_in, .. } => {
                    *sign_in = false;
                    if *signing {
                        *signing = false;
                        self.down(oauth::Request::Cancel { account: index }, now, wall);
                        return;
                    }
                }
                Origin::Env { .. } => {}
            }
        }
        self.close_sent = true;
        self.down(oauth::Request::Close, now, wall);
    }

    pub(super) fn private_files(&mut self, now: Time, wall: Wall) {
        if self.events.room() < 3 || self.submissions.room() < 2 {
            return;
        }
        for _ in 0..self.file_requests.len() {
            match self.file_requests.pop().expect("queued keeper request") {
                oauth::FileLower::Request { request, deadline } => {
                    if deadline <= now {
                        self.events.push(file::Event::Failed {
                            owner: owner(&request),
                            error: kernel::Error::TimedOut,
                            committed: false,
                            residue: None,
                        });
                        return;
                    }
                    if self.record_request(&request, now, wall) {
                        return;
                    }
                    if self.files.takes() {
                        file_layer::down_until(
                            &mut self.files,
                            deadline,
                            request,
                            &mut self.events,
                            &mut self.submissions,
                        );
                        return;
                    }
                    self.file_requests.push(oauth::FileLower::Request { request, deadline });
                }
                oauth::FileLower::Remove { owner, root, name, directory, deadline } => {
                    if deadline <= now {
                        self.events.push(file::Event::Failed {
                            owner,
                            error: kernel::Error::TimedOut,
                            committed: false,
                            residue: None,
                        });
                        return;
                    }
                    if self.files.takes() {
                        let root = self.files.descriptor(root).expect("private removal root");
                        file_layer::down_until(
                            &mut self.files,
                            deadline,
                            file::Request::Remove { owner, root, name, directory },
                            &mut self.events,
                            &mut self.submissions,
                        );
                        return;
                    }
                    self.file_requests.push(oauth::FileLower::Remove { owner, root, name, directory, deadline });
                }
                oauth::FileLower::Cancel { owner: cancelled } => {
                    file_layer::cancel(&mut self.files, cancelled, &mut self.submissions);
                    for _ in 0..self.file_requests.len() {
                        match self.file_requests.pop().expect("queued keeper work") {
                            oauth::FileLower::Request { request, deadline } => {
                                if owner(&request) == cancelled {
                                    self.events.push(file::Event::Cancelled { owner: cancelled });
                                } else {
                                    self.file_requests.push(oauth::FileLower::Request { request, deadline });
                                }
                            }
                            oauth::FileLower::Remove { owner, root, name, directory, deadline } => {
                                if owner == cancelled {
                                    self.events.push(file::Event::Cancelled { owner });
                                } else {
                                    self.file_requests.push(oauth::FileLower::Remove {
                                        owner,
                                        root,
                                        name,
                                        directory,
                                        deadline,
                                    });
                                }
                            }
                            oauth::FileLower::Cancel { owner } => {
                                self.file_requests.push(oauth::FileLower::Cancel { owner });
                            }
                        }
                    }
                    return;
                }
            }
        }
    }
}

#[expect(clippy::wildcard_enum_match_arm, reason = "private keepers emit only open, load, store and close")]
fn owner(request: &file::Request) -> Token {
    match request {
        file::Request::OpenPrivate { owner, .. }
        | file::Request::Load { owner, .. }
        | file::Request::Store { owner, .. }
        | file::Request::Close { owner, .. }
        | file::Request::Remove { owner, .. } => *owner,
        _ => unreachable!("private keeper file vocabulary"),
    }
}
