//! Handed-in accounts and read-only login loads (protocol/hosts.md, 5.4).
//! The driver owns records and holds. This owner keeps source descriptors,
//! account identifiers and reads, never a refresh token or a writable login.
use alloc::boxed::Box;
use core::mem::size_of;
use skein_io::{file, file_layer, kernel};
use skein_lib::{CryptoRng, Env, List, Queue, Secret, Time, Token, Wall};
use skein_oauth_accounts as oauth;
use smith_local_domain as local;
use smith_local_protocol::{Borrowed, LoginFormat, decode_login};

/// One access-only account source handed to the local service by its shell.
#[expect(missing_debug_implementations, reason = "environment records contain credentials")]
pub enum Source {
    /// A login under an adopted directory, loaded at startup and for later grants.
    Borrow { account: u32, root: kernel::Fd, path: Box<[u8]>, format: LoginFormat },
    /// A variable read once by the shell, with its endpoint's account identifier.
    Env { account: u32, borrowed: Borrowed },
}

struct Account {
    number: u32,
    source: Origin,
    identifier: Box<[u8]>,
    generation: u64,
    expires: Wall,
    startup: bool,
    loading: bool,
    wanted: bool,
}
enum Origin {
    Borrow { root: Option<Token>, path: Box<[u8]>, format: LoginFormat },
    Env { initial: Option<Borrowed> },
}

/// The service's handed-in account driver and read-only file owner.
#[expect(missing_debug_implementations, reason = "the driver contains credentials")]
pub struct Accounts {
    driver: oauth::Component,
    limits: oauth::Limits,
    entries: List<Account>,
    files: file_layer::FileIo,
    events: Queue<file::Event>,
    failed: Option<(u32, local::CredentialFailure)>,
    submissions: Queue<kernel::Submit>,
    out: Queue<oauth::Event>,
    closed: bool,
    closing: bool,
}

/// The driver, source descriptors, file operations and bounded output queues.
#[must_use]
pub fn accounts_worst_case(limits: &oauth::Limits) -> Option<u64> {
    oauth::worst_case(limits)?
        .checked_add(smith_local_protocol::login_worst_case(&limits.client.document)?)?
        .checked_add(file_layer::FileIo::worst_case(
            limits.accounts.checked_mul(2)?,
            4096,
            1,
            limits.client.document.document_bytes,
        )?)?
        .checked_add(List::<Account>::worst_case(limits.accounts)?)?
        .checked_add(List::<Source>::worst_case(limits.accounts)?)?
        .checked_add(
            u64::from(limits.accounts).checked_mul(
                4095_u64.checked_add(u64::from(limits.client.document.record_bytes))?.checked_add(1024)?,
            )?,
        )?
        .checked_add(Queue::<file::Event>::worst_case(3)?)?
        .checked_add(u64::from(limits.client.document.document_bytes).checked_mul(3)?)?
        .checked_add(Queue::<oauth::Event>::worst_case(3)?)?
        .checked_add(u64::from(limits.client.document.token_bytes).checked_mul(3)?)?
        .checked_add(Queue::<kernel::Submit>::worst_case(8)?)?
        .checked_add(u64::try_from(size_of::<Accounts>()).ok()?)
}

fn number(source: &Source) -> u32 {
    match source {
        Source::Borrow { account, .. } | Source::Env { account, .. } => *account,
    }
}

impl Accounts {
    /// Validate sources and adopt their read-only directory descriptors.
    pub fn new(sources: List<Source>, limits: oauth::Limits, entropy: [u8; 32]) -> Result<Self, crate::Error> {
        accounts_worst_case(&limits).ok_or(crate::Error::Memory)?;
        if sources.len() > limits.accounts || limits.client.document.document_bytes == 0 {
            return Err(crate::Error::Memory);
        }
        let mut handed = List::with_capacity(limits.accounts);
        let mut accounts = List::<Account>::with_capacity(limits.accounts);
        for index in 0..sources.len() {
            let source = sources.get(index).expect("source index");
            for prior in 0..index {
                if number(source) == number(sources.get(prior).expect("prior source")) {
                    return Err(crate::Error::Process);
                }
            }
            match source {
                Source::Borrow { path, .. } if path.is_empty() || path.len() > 4095 => {
                    return Err(crate::Error::Memory);
                }
                Source::Env { borrowed, .. }
                    if borrowed.record.refresh_token.is_some()
                        || borrowed.account_id.len() > 1024
                        || skein_oauth::encode_record(&borrowed.record, &limits.client.document).is_err() =>
                {
                    return Err(crate::Error::Process);
                }
                Source::Borrow { .. } | Source::Env { .. } => {}
            }
            match handed.push(oauth::Account::HandedIn) {
                Ok(()) => {}
                Err(_) => return Err(crate::Error::Memory),
            }
        }
        let crypto = CryptoRng::new(&Secret::new(Box::from(entropy)), 0).ok_or(crate::Error::Memory)?;
        let Ok(driver) = oauth::Component::new(handed, &limits, crypto) else {
            return Err(crate::Error::Process);
        };
        let mut files = file_layer::FileIo::with_whole_limit(
            limits.accounts.checked_mul(2).ok_or(crate::Error::Memory)?,
            4096,
            1,
            limits.client.document.document_bytes,
            limits.file_stall,
        );
        for source in sources.into_boxed() {
            let (number, origin) = match source {
                Source::Borrow { account, root, path, format } => {
                    let root = files.adopt_root(root).expect("validated source count fits");
                    (account, Origin::Borrow { root: Some(root), path, format })
                }
                Source::Env { account, borrowed } => (account, Origin::Env { initial: Some(borrowed) }),
            };
            match accounts.push(Account {
                number,
                source: origin,
                identifier: Box::new([]),
                generation: 0,
                expires: Wall::EPOCH,
                startup: true,
                loading: false,
                wanted: false,
            }) {
                Ok(()) => {}
                Err(_) => return Err(crate::Error::Memory),
            }
        }
        Ok(Self {
            driver,
            limits,
            entries: accounts,
            files,
            events: Queue::with_capacity(3),
            failed: None,
            submissions: Queue::with_capacity(8),
            out: Queue::with_capacity(3),
            closed: false,
            closing: false,
        })
    }

    fn down(&mut self, request: oauth::Request, now: Time, wall: Wall) {
        let env = Env { now, wall, limits: self.limits };
        let mut io = Queue::with_capacity(3);
        let mut files = Queue::with_capacity(1);
        self.driver.down(&env, request, &mut self.out, &mut io, &mut files);
        assert!(io.is_empty() && files.is_empty(), "handed-in accounts perform no exchange or keep");
    }

    /// Request this account's grant; the next advance emits its terminal.
    pub fn request(&mut self, number: u32) -> bool {
        if self.closing || self.closed {
            return false;
        }
        for index in 0..self.entries.len() {
            let account = self.entries.get_mut(index).expect("account index");
            if account.number == number {
                assert!(!account.wanted, "one grant request per account");
                account.wanted = true;
                return true;
            }
        }
        false
    }

    fn hand_in(&mut self, index: u32, mut borrowed: Borrowed, now: Time, wall: Wall) -> bool {
        let account = self.entries.get_mut(index).expect("account index");
        if borrowed.account_id.len() > 1024 {
            self.fail(index, local::CredentialFailure::Missing);
            return false;
        }
        let Some(generation) = account.generation.checked_add(1) else {
            self.fail(index, local::CredentialFailure::Missing);
            return false;
        };
        borrowed.record.key = index;
        borrowed.record.generation = generation;
        account.generation = generation;
        account.expires = borrowed.record.expires_at;
        account.identifier = borrowed.account_id;
        self.down(oauth::Request::HandIn { account: index, record: borrowed.record }, now, wall);
        self.out.is_empty()
    }

    fn fail(&mut self, index: u32, reason: local::CredentialFailure) {
        let account = self.entries.get_mut(index).expect("account index");
        if account.wanted {
            assert!(self.failed.is_none(), "one serial file failure");
            self.failed = Some((account.number, reason));
            account.wanted = false;
        }
    }

    fn begin(&mut self, index: u32, now: Time, wall: Wall) {
        let account = self.entries.get_mut(index).expect("account index");
        account.startup = false;
        match &mut account.source {
            Origin::Borrow { .. } => {
                account.loading = true;
                self.down(oauth::Request::Release { account: index }, now, wall);
                let account = self.entries.get(index).expect("account index");
                match &account.source {
                    Origin::Borrow { root, path, .. } => file_layer::down(
                        &mut self.files,
                        now,
                        file::Request::Load {
                            owner: Token::new(u64::from(index)),
                            root: root.expect("live login root"),
                            path: path.clone(),
                            max: self.limits.client.document.document_bytes,
                            no_follow: true,
                        },
                        &mut self.events,
                        &mut self.submissions,
                    ),
                    Origin::Env { .. } => unreachable!("borrowed source"),
                }
            }
            Origin::Env { initial } => {
                let initial = initial.take();
                if let Some(borrowed) = initial
                    && !self.hand_in(index, borrowed, now, wall)
                {
                    return;
                }
                if self.entries.get(index).expect("account index").wanted {
                    self.down(oauth::Request::Release { account: index }, now, wall);
                    self.down(oauth::Request::Grant { account: index }, now, wall);
                }
            }
        }
    }

    fn progress(&mut self, now: Time, wall: Wall) {
        if !self.files.takes() || !self.out.is_empty() {
            return;
        }
        for index in 0..self.entries.len() {
            let account = self.entries.get_mut(index).expect("account index");
            if self.closing {
                match &mut account.source {
                    Origin::Borrow { root, .. } => {
                        if let Some(root) = root.take() {
                            file_layer::down(
                                &mut self.files,
                                now,
                                file::Request::Close { owner: Token::new(u64::from(index)), file: root },
                                &mut self.events,
                                &mut self.submissions,
                            );
                            return;
                        }
                    }
                    Origin::Env { .. } => {}
                }
            } else if !account.loading && (account.startup || account.wanted) {
                self.begin(index, now, wall);
                return;
            }
        }
    }

    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "external file IO terminals other than Loaded or Closed end a failed load"
    )]
    fn filed(&mut self, event: file::Event, now: Time, wall: Wall) {
        match event {
            file::Event::Loaded { owner, bytes } => {
                let index = u32::try_from(owner.raw()).expect("account index");
                let account = self.entries.get_mut(index).expect("account index");
                account.loading = false;
                let format = match account.source {
                    Origin::Borrow { format, .. } => format,
                    Origin::Env { .. } => unreachable!("env is read once"),
                };
                match decode_login(format, &bytes, &self.limits.client.document) {
                    Ok(borrowed) if !self.closing => {
                        if self.hand_in(index, borrowed, now, wall)
                            && self.entries.get(index).expect("account index").wanted
                        {
                            self.down(oauth::Request::Grant { account: index }, now, wall);
                        }
                    }
                    Ok(_) | Err(_) => self.fail(index, local::CredentialFailure::Missing),
                }
            }
            file::Event::Closed { .. } => {}
            _ => {
                for index in 0..self.entries.len() {
                    let account = self.entries.get_mut(index).expect("account index");
                    if account.loading {
                        account.loading = false;
                        self.fail(index, local::CredentialFailure::Missing);
                    }
                }
            }
        }
    }

    /// Route one terminal of this driver's kernel operations.
    pub fn complete(&mut self, complete: kernel::Complete) {
        file_layer::up(&mut self.files, complete, &mut self.events, &mut self.submissions);
    }

    /// The earliest driver or read-only file deadline.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Time> {
        match self.driver.next_deadline() {
            Some(driver) => match self.files.next_deadline() {
                Some(file) => Some(driver.min(file)),
                None => Some(driver),
            },
            None => self.files.next_deadline(),
        }
    }

    /// The driver and every adopted directory have reported closed.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed && self.files.open_files() == 0 && self.events.is_empty() && self.submissions.is_empty()
    }

    /// Whether an advance can make progress before another kernel completion.
    #[must_use]
    pub fn work_pending(&self, now: Time) -> bool {
        if self.closed() {
            return false;
        }
        if !self.events.is_empty() || !self.out.is_empty() || self.failed.is_some() || !self.submissions.is_empty() {
            return true;
        }
        if match self.next_deadline() {
            Some(deadline) => deadline <= now,
            None => false,
        } {
            return true;
        }
        if self.files.takes() {
            for account in &self.entries {
                if !self.closing && !account.loading && (account.startup || account.wanted) {
                    return true;
                }
                match account.source {
                    Origin::Borrow { root: Some(_), .. } if self.closing => return true,
                    Origin::Borrow { .. } | Origin::Env { .. } => {}
                }
            }
        }
        false
    }
}

/// Caller-reserved room for one driver advance, including its file failures.
pub const ACCOUNTS_MAX_OUT: u32 = 6;

impl Accounts {
    /// Directory identifiers accompany grants without putting them in traces.
    #[must_use]
    pub fn account_id(&self, number: u32) -> Option<&[u8]> {
        for account in &self.entries {
            if account.number == number {
                return Some(&account.identifier);
            }
        }
        None
    }

    /// Kernel requests retain this owner's tokens until their terminals arrive.
    pub fn submissions(&mut self) -> &mut Queue<kernel::Submit> {
        &mut self.submissions
    }

    /// End the run's holds while keeping the source records for the next run.
    pub fn release_all(&mut self, now: Time, wall: Wall) {
        for index in 0..self.entries.len() {
            self.down(oauth::Request::Release { account: index }, now, wall);
        }
    }

    /// Close the driver and cancel outstanding login reads before closing roots.
    pub fn close(&mut self, close: smith_agent_service::Close, now: Time, wall: Wall) {
        assert!(self.out.is_empty(), "drain driver events before close");
        if !self.closing && !self.closed {
            self.closing = true;
            let request = match close {
                smith_agent_service::Close::Graceful => oauth::Request::Close,
                smith_agent_service::Close::Aborted => oauth::Request::Abort,
            };
            self.down(request, now, wall);
            for index in 0..self.entries.len() {
                if self.entries.get(index).expect("account index").loading {
                    file_layer::cancel(&mut self.files, Token::new(u64::from(index)), &mut self.submissions);
                }
            }
        }
    }

    /// Advance serial file reads and translate driver events, reserving `ACCOUNTS_MAX_OUT`.
    pub fn advance(&mut self, now: Time, wall: Wall, out: &mut Queue<smith_local_protocol::CredentialRequest>) {
        assert!(out.room() >= ACCOUNTS_MAX_OUT, "caller reserves a complete account advance");
        if self.closed() {
            return;
        }
        if self.files.is_due(now) {
            file_layer::expire(&mut self.files, now, &mut self.submissions);
        }
        for _ in 0..3_u32 {
            match self.events.pop() {
                Some(event) => self.filed(event, now, wall),
                None => break,
            }
            if !self.out.is_empty() {
                break;
            }
        }
        self.progress(now, wall);
        if self.out.is_empty() && !self.closing {
            let env = Env { now, wall, limits: self.limits };
            self.driver.fire(&env, &mut self.out, &mut Queue::with_capacity(3), &mut Queue::with_capacity(1));
        }
        self.emit(out);
        if self.closed() && self.closing {
            self.closing = false;
            out.push(smith_local_protocol::CredentialRequest::Closed);
        }
    }

    fn emit(&mut self, out: &mut Queue<smith_local_protocol::CredentialRequest>) {
        use smith_local_protocol::CredentialRequest;
        if let Some((account, reason)) = self.failed.take() {
            out.push(CredentialRequest::Failed { event: local::Event::NoCredential { account, reason } });
        }
        for _ in 0..3_u32 {
            match self.out.pop() {
                Some(oauth::Event::Granted { account: index, token, generation, valid }) => {
                    let account = self.entries.get_mut(index).expect("account index");
                    account.wanted = false;
                    out.push(CredentialRequest::Ready {
                        event: local::Event::Credential {
                            grant: smith_domain::Grant {
                                name: smith_domain::GrantName { account: account.number, generation },
                                valid,
                            },
                        },
                        value: token,
                    });
                }
                Some(oauth::Event::Expiring { account, .. }) => {
                    let account = self.entries.get(account).expect("account index");
                    out.push(CredentialRequest::Notice {
                        account: account.number,
                        reason: local::CredentialFailure::BorrowedExpiring { at: account.expires },
                    });
                }
                Some(oauth::Event::Failed { account, .. } | oauth::Event::Refused { account, .. }) => {
                    let account = self.entries.get_mut(account).expect("account index");
                    account.wanted = false;
                    let reason = match account.source {
                        Origin::Borrow { .. } => local::CredentialFailure::BorrowedExpired { at: account.expires },
                        Origin::Env { .. } => local::CredentialFailure::Missing,
                    };
                    out.push(CredentialRequest::Failed {
                        event: local::Event::NoCredential { account: account.number, reason },
                    });
                }
                Some(oauth::Event::Closed) => self.closed = true,
                Some(oauth::Event::Visit { .. } | oauth::Event::SignedIn { .. } | oauth::Event::Keep { .. }) => {
                    unreachable!("handed-in accounts do not sign in")
                }
                None => break,
            }
        }
    }
}

#[cfg(test)]
mod tests;
