//! Shared local shell pass (protocol/hosts.md, sections 5.3–5.6). It keeps
//! chat and token file owners, the terminal writer, and kernel routing. It
//! knows no simulator or checkout state. `Local::new` takes all directories
//! and terminal descriptors; `Host::iterate` performs the service and shell
//! passes. Every store acknowledgement follows file and directory sync.

use std::io::Write;
use std::path::PathBuf;

use skein_io::{self as io, kernel};
use skein_lib::stream::{OutputDown, OutputOutcome, OutputUp};
use skein_lib::{Env, Map, Queue, Time, Token, Wall};
use skein_shell::Host;
use smith_agent_service as agent;
use smith_local_domain as domain;
use smith_local_service as service;

use crate::{
    local_auth::{self, Auth},
    local_settings::Account,
    local_tokens::Tokens,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Route {
    Service(Token),
    Writer(Token),
    Launch,
    TraceRoot,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Announcement {
    BeforeStart,
    Running,
    Ended,
}

/// Paths and inherited descriptors owned by one invocation's shared shell.
pub struct Resources {
    pub state_root: kernel::Fd,
    pub token_directory: PathBuf,
    pub input: kernel::Fd,
    pub output: kernel::Fd,
    pub signals: kernel::Fd,
    pub delivery_roots: Box<[kernel::Fd]>,
    pub effect_roots: Box<[kernel::Fd]>,
    pub delivery_environment: Box<[Box<[u8]>]>,
    pub accounts: Box<[Account]>,
    pub seed: u64,
    /// Unpredictable bytes from the binary shell; worlds supply deterministic bytes.
    pub oauth_entropy: [u8; 32],
}

/// The local service and its shell effects, independent of the driving loop.
pub struct Local {
    service: service::Service,
    tokens: Tokens,
    accounts: Box<[Auth]>,
    writer: io::Io,
    env: Env<io::Limits>,
    output: Token,
    launch_root: Option<kernel::Fd>,
    launch_closing: bool,
    routes: Map<Token, Route>,
    operation: u64,
    pending: Option<(Token, Box<[u8]>)>,
    right: u64,
    events: Queue<io::Event>,
    requests: Queue<io::Request>,
    store_completions: Queue<kernel::Complete>,
    store_submissions: Queue<kernel::Submit>,
    completions: Queue<kernel::Complete>,
    submissions: Queue<kernel::Submit>,
    exit: Option<domain::ExitStatus>,
    closing: bool,
    error: Option<String>,
    errors: Box<dyn Write>,
    announcement: Announcement,
    worst: u64,
    operations: u32,
}

impl Local {
    /// Compose either agent kind with the same file and terminal shell pass.
    pub fn new(
        config: service::Config,
        lower: Option<agent::Config>,
        resources: Resources,
        errors: Box<dyn Write>,
    ) -> Result<Self, String> {
        Self::within_memory(config, lower, resources, errors, u64::MAX).map_err(|failure| failure.detail().to_owned())
    }

    /// Refuse the complete shell footprint above the caller's memory before stream adoption.
    /// Contract: protocol/limits.md, section 4.
    #[expect(
        clippy::too_many_lines,
        reason = "startup validates the shared store, terminal, credential and selected lower owners"
    )]
    pub fn within_memory(
        mut config: service::Config,
        lower: Option<agent::Config>,
        resources: Resources,
        errors: Box<dyn Write>,
        available_bytes: u64,
    ) -> Result<Self, crate::local_shell::Failure> {
        let lower_routes = lower.as_ref().map_or(0, |configuration| configuration.limits.routes);
        config.agent = match lower {
            Some(configuration) => {
                config.limits.local.agent = domain::AgentLimits::inline(
                    config.limits.local.smith,
                    config.limits.local.unsaved,
                    config.local.budget.time,
                    match config.limits.local.agent {
                        domain::AgentLimits::Inline(limits) => limits.cancel_grace,
                        domain::AgentLimits::Spawned(limits) => limits.cancel_grace,
                    },
                )
                .ok_or_else(|| configuration_failure(derivation_refusal("local_worst_case")))?;
                config.limits.inline = Some(
                    service::InlineLimits::new(&configuration)
                        .ok_or_else(|| configuration_failure(derivation_refusal("local_worst_case")))?,
                );
                service::AgentConfig::Inline { configuration, roots: resources.effect_roots }
            }
            None if resources.effect_roots.is_empty() => service::AgentConfig::Spawned,
            None => return Err(configuration_failure("spawned mode cannot adopt inline roots".into())),
        };
        if resources.accounts.len() > usize::try_from(config.limits.local.smith.accounts).expect("account bound fits")
            || resources.accounts.iter().any(|account| account.account_id.len() > 1024)
        {
            return Err(configuration_failure("local accounts exceed their configured bounds".into()));
        }
        for (position, account) in resources.accounts.iter().enumerate() {
            if resources
                .accounts
                .get(..position)
                .expect("enumerated prior accounts")
                .iter()
                .any(|prior| prior.number == account.number)
            {
                return Err(configuration_failure("local account numbers must be unique".into()));
            }
        }
        let launch_root = config.launch.root;
        let queue = config.limits.queue;
        let outer_queue =
            queue.checked_mul(2).ok_or_else(|| configuration_failure(derivation_refusal("local_queue")))?;
        let io_limits = config.limits.process.io;
        if !resources.accounts.is_empty() && !crate::local_auth_http::fits(&io_limits) {
            return Err(configuration_failure("local IO limits cannot carry OAuth TLS records".into()));
        }
        let operations = io::operations(&io_limits)
            .and_then(|count| count.checked_mul(2))
            .and_then(|count| count.checked_add(64))
            .and_then(|count| count.checked_add(lower_routes))
            .ok_or_else(|| configuration_failure(derivation_refusal("local_routes")))?;
        let worst = accounted_memory(
            &config.limits,
            u32::try_from(resources.accounts.len())
                .map_err(|_| configuration_failure(derivation_refusal("account_worst_case")))?,
            lower_routes,
        )
        .map_err(configuration_failure)?;
        smith_agent_service::profile::validate::memory(worst, available_bytes)
            .map_err(profile_refusal)
            .map_err(configuration_failure)?;
        let mut service = service::Service::new(config, resources.seed)
            .map_err(|error| configuration_failure(format!("local service: {error:?}")))?;
        service.adopt_store(resources.state_root).map_err(|error| host_failure(format!("chat store: {error:?}")))?;
        let tokens = Tokens::new(&resources.token_directory, token_limits()).map_err(host_failure)?;
        let mut writer = io::Io::new(&io_limits);
        let output = writer
            .adopt_write_pipe(resources.output)
            .map_err(|_| host_failure("terminal output cannot be adopted".into()))?;
        let requests = Queue::with_capacity(queue);
        let accounts = resources
            .accounts
            .into_vec()
            .into_iter()
            .enumerate()
            .map(|(position, account)| Auth::new(account, position, resources.oauth_entropy))
            .collect::<Result<Vec<_>, _>>()
            .map_err(host_failure)?;
        let mut local = Self {
            service,
            tokens,
            accounts: accounts.into(),
            writer,
            env: Env { now: Time::ZERO, wall: Wall::EPOCH, limits: io_limits },
            output,
            launch_root: Some(launch_root),
            launch_closing: false,
            routes: Map::with_capacity(operations),
            operation: 1,
            pending: None,
            right: 1,
            events: Queue::with_capacity(queue),
            requests,
            store_completions: Queue::with_capacity(queue),
            store_submissions: Queue::with_capacity(queue),
            completions: Queue::with_capacity(outer_queue),
            submissions: Queue::with_capacity(outer_queue),
            exit: None,
            closing: false,
            error: None,
            errors,
            announcement: Announcement::BeforeStart,
            worst,
            operations,
        };
        local
            .service
            .adopt_terminal(resources.input, resources.signals)
            .map_err(|fd| host_failure(format!("local terminal {} cannot be adopted", fd.raw())))?;
        local
            .service
            .adopt_delivery_roots(resources.delivery_roots, resources.delivery_environment)
            .map_err(|error| host_failure(format!("delivery directories: {error:?}")))?;
        Ok(local)
    }

    /// Price and configure the spawned product's bounded trace producer.
    pub fn prepare_trace(
        &mut self,
        configuration: &agent::Config,
        available: u64,
    ) -> Result<(), crate::local_shell::Failure> {
        let bytes = agent::events::worst_case(&configuration.limits)
            .ok_or_else(|| configuration_failure(derivation_refusal("local_trace")))?;
        let worst =
            self.worst.checked_add(bytes).ok_or_else(|| configuration_failure(derivation_refusal("local_trace")))?;
        agent::profile::validate::memory(worst, available).map_err(profile_refusal).map_err(configuration_failure)?;
        self.service.prepare_trace(configuration);
        self.worst = worst;
        Ok(())
    }

    /// Adopt an opened product trace before the first loop pass.
    pub fn adopt_trace(
        &mut self,
        fd: kernel::Fd,
        root: Option<kernel::Fd>,
        now: Time,
        header: smith_events::SessionStarted,
        delivery: agent::sinks::Delivery,
    ) -> Result<(), kernel::Fd> {
        self.service.adopt_trace(fd, now, header, delivery)?;
        if let Some(fd) = root {
            self.submit(kernel::Submit { op: Token::new(0), kind: kernel::Op::Close { fd } }, Route::TraceRoot);
        }
        Ok(())
    }

    /// Price the selected local service and every outer shell owner before adopting resources.
    /// `accounts` counts actual configured OAuth owners; `lower_routes` is zero for a spawned child.
    /// Contract: protocol/limits.md, sections 3 and 4.
    #[must_use]
    pub fn required_memory(limits: &service::Limits, accounts: u32, lower_routes: u32) -> Option<u64> {
        accounted_memory(limits, accounts, lower_routes).ok()
    }

    /// Content-free service facts available to the caller's independent referee.
    pub fn pop_fact(&mut self) -> Option<domain::Fact> {
        self.service.pop_fact()
    }

    /// The completed invocation's status after every lower close settles.
    #[must_use]
    pub fn result(&self) -> Option<Result<domain::ExitStatus, &str>> {
        if !self.is_empty() {
            return None;
        }
        self.exit.map(|status| match &self.error {
            Some(error) => Err(error.as_str()),
            None => Ok(status),
        })
    }

    fn files(&mut self) {
        for _ in 0..self.service.shell_requests().capacity() {
            let Some(request) = self.service.shell_requests().pop() else { break };
            match request {
                domain::Request::Credential { account } => {
                    self.credential(account);
                }
                domain::Request::Exit { status } => {
                    self.exit = Some(status);
                }
                domain::Request::Load
                | domain::Request::SaveState { .. }
                | domain::Request::SaveTurn { .. }
                | domain::Request::SaveDelivery { .. }
                | domain::Request::Notice { .. }
                | domain::Request::Answered { .. }
                | domain::Request::Show { .. }
                | domain::Request::Git { .. }
                | domain::Request::PlainStatus { .. }
                | domain::Request::Lower { .. }
                | domain::Request::Process(_)
                | domain::Request::Input { .. } => unreachable!("service owns lower effects"),
            }
        }
    }

    fn credential(&mut self, account: u32) {
        if let Some(auth) = self.accounts.iter_mut().find(|auth| auth.account() == account) {
            auth.request();
        } else {
            self.service
                .local_event(domain::Event::NoCredential { account, reason: domain::CredentialFailure::Missing });
        }
    }

    #[expect(
        clippy::single_match_else,
        reason = "10-product replaces the shell token-store driver with skein OAuth io"
    )]
    fn authentication(&mut self) {
        for auth in &mut self.accounts {
            if self.exit.is_some() {
                auth.close(self.env.now, self.env.wall, &mut self.requests);
            }
            auth.due(self.env.now);
            if auth.wants() {
                match self.tokens.load(auth.account()) {
                    Ok(saved) => auth.begin(saved, self.env.now, self.env.wall),
                    Err(_) => {
                        auth.load_failed();
                        self.service.local_event(domain::Event::NoCredential {
                            account: auth.account(),
                            reason: domain::CredentialFailure::Missing,
                        });
                    }
                }
            }
            auth.tick(self.env.now, self.env.wall, &mut self.requests);
            if let Some(url) = auth.shown_url() {
                let mut text = b"Sign in: ".to_vec();
                text.extend_from_slice(&url);
                text.push(b'\n');
                self.service.output().push(text.into());
            }
            for _ in 0..8 {
                let Some(request) = auth.take() else { break };
                match request {
                    smith_local_protocol::CredentialRequest::Visit { url } => auth.visit(url, &mut self.requests),
                    smith_local_protocol::CredentialRequest::Http(request) => auth.http(request),
                    smith_local_protocol::CredentialRequest::Save { account, bytes } => {
                        match self.tokens.save(account, &bytes) {
                            Ok(()) => auth.stored(self.env.wall),
                            Err(_) => auth.store_failed(),
                        }
                    }
                    smith_local_protocol::CredentialRequest::Ready {
                        event: domain::Event::Credential { grant },
                        value,
                    } => {
                        let length = u16::try_from(auth.account_id().len()).expect("bounded account identifier");
                        let mut envelope = Vec::with_capacity(
                            auth.account_id()
                                .len()
                                .checked_add(value.len())
                                .and_then(|bytes| bytes.checked_add(2))
                                .expect("bounded credential envelope"),
                        );
                        envelope.extend_from_slice(&length.to_be_bytes());
                        envelope.extend_from_slice(auth.account_id());
                        envelope.extend_from_slice(&value);
                        auth.lent(grant.valid, self.env.now);
                        self.service.credential(grant, envelope.into());
                    }
                    smith_local_protocol::CredentialRequest::Failed { event } => self.service.local_event(event),
                    smith_local_protocol::CredentialRequest::Ready { .. } => {
                        unreachable!("credential translator lends only a grant")
                    }
                }
            }
        }
    }

    fn writer_up(&mut self) {
        while self.writer.is_ready() {
            io::resume(&mut self.writer, &self.env, &mut self.events, &mut self.store_submissions);
        }
        for _ in 0..self.store_completions.capacity() {
            let Some(complete) = self.store_completions.pop() else { break };
            io::up(&mut self.writer, &self.env, complete, &mut self.events, &mut self.store_submissions);
        }
        for _ in 0..self.env.limits.sockets {
            if self.writer.is_ready() {
                io::resume(&mut self.writer, &self.env, &mut self.events, &mut self.store_submissions);
            }
            if self.writer.is_due(self.env.now) {
                io::fire(&mut self.writer, &self.env, &mut self.events, &mut self.store_submissions);
            }
        }
        while let Some(event) = self.events.pop() {
            let owner = local_auth::event_owner(&event);
            if let Some(auth) = self.accounts.iter_mut().find(|auth| auth.owns(owner)) {
                auth.event(event, self.env.now, self.env.wall, &mut self.requests);
                continue;
            }
            match event {
                io::Event::Output { owner, up: OutputUp::Settled { right, outcome } } if owner == self.output => {
                    let (wanted, text) = self.pending.take().expect("terminal has one output right");
                    assert_eq!(right, wanted, "terminal settled its sole output right");
                    match outcome {
                        OutputOutcome::Granted => self.requests.push(io::Request::Output {
                            stream: self.output,
                            down: OutputDown::Send { right, bytes: text },
                        }),
                        OutputOutcome::Cancelled | OutputOutcome::Failed(_) => self.write_failed(),
                    }
                }
                io::Event::Failed { .. } | io::Event::Stream { up: skein_lib::stream::Up::Failed(_), .. } => {
                    self.write_failed();
                }
                io::Event::Closed { .. } => {}
                other @ (io::Event::Listening { .. }
                | io::Event::Accepted { .. }
                | io::Event::Connecting { .. }
                | io::Event::Connected { .. }
                | io::Event::Stream { .. }
                | io::Event::Output { .. }
                | io::Event::Spawned { .. }
                | io::Event::Exited { .. }
                | io::Event::Usage { .. }
                | io::Event::Shutdown { .. }) => panic!("unexpected terminal writer event: {other:?}"),
            }
        }
    }

    fn write_failed(&mut self) {
        self.error = Some("terminal output failed".into());
        self.service.local_event(domain::Event::StoreFailed { turn: None, reason: domain::StoreFailure::Write });
    }

    fn writer_down(&mut self) {
        if self.pending.is_none()
            && !self.closing
            && let Some(text) = self.service.output().pop()
        {
            let right = Token::new(self.right);
            self.right = self.right.checked_add(1).expect("terminal output names remain representable");
            let bytes = u32::try_from(text.len()).expect("bounded show text");
            if bytes > 0 {
                self.pending = Some((right, text));
                self.requests
                    .push(io::Request::Output { stream: self.output, down: OutputDown::Room { right, bytes } });
            }
        }
        if self.exit.is_some() && !self.closing && self.pending.is_none() && self.service.output().is_empty() {
            self.closing = true;
            self.requests.push(io::Request::Close { entity: self.output });
            let fd = self.launch_root.take().expect("one launch root");
            self.launch_closing = true;
            self.submit(kernel::Submit { op: Token::new(0), kind: kernel::Op::Close { fd } }, Route::Launch);
        }
        while self.writer.takes() {
            let Some(request) = self.requests.pop() else { break };
            io::down(&mut self.writer, &self.env, request, &mut self.store_submissions);
        }
        self.writer.reclaim();
    }
    fn submit(&mut self, submit: kernel::Submit, route: Route) {
        let kind = match submit.kind {
            kernel::Op::Cancel { target } => {
                let wanted = match route {
                    Route::Service(_) => Route::Service(target),
                    Route::Writer(_) => Route::Writer(target),
                    Route::Launch | Route::TraceRoot => unreachable!("startup directory only closes"),
                };
                let target = self
                    .routes
                    .iter()
                    .find_map(|(global, active)| (*active == wanted).then_some(*global))
                    .unwrap_or(Token::new(u64::MAX));
                kernel::Op::Cancel { target }
            }
            other @ (kernel::Op::Socket { .. }
            | kernel::Op::Bind { .. }
            | kernel::Op::Listen { .. }
            | kernel::Op::Accept { .. }
            | kernel::Op::Connect { .. }
            | kernel::Op::Recv { .. }
            | kernel::Op::Send { .. }
            | kernel::Op::Shutdown { .. }
            | kernel::Op::Close { .. }
            | kernel::Op::Open { .. }
            | kernel::Op::Append { .. }
            | kernel::Op::Read { .. }
            | kernel::Op::Write { .. }
            | kernel::Op::Sync { .. }
            | kernel::Op::Stat { .. }
            | kernel::Op::Rename { .. }
            | kernel::Op::Remove { .. }
            | kernel::Op::MakeDirectory { .. }
            | kernel::Op::List { .. }
            | kernel::Op::Spawn { .. }
            | kernel::Op::Wait { .. }
            | kernel::Op::Signal { .. }
            | kernel::Op::ReadSignal { .. }
            | kernel::Op::PipeRead { .. }
            | kernel::Op::PipeWrite { .. }
            | kernel::Op::Usage) => other,
        };
        let op = Token::new(self.operation);
        self.operation = self.operation.checked_add(1).expect("shell operation names remain representable");
        self.routes.insert(op, route).expect("configured operation routes");
        self.submissions.push(kernel::Submit { op, kind });
    }
}

impl Host for Local {
    fn drain(&mut self) {
        if self.announcement == Announcement::BeforeStart {
            self.announcement = Announcement::Running;
            drop(writeln!(self.errors, "smith: local host started; worst case {} bytes", self.worst));
        }
        if self.is_empty() && self.announcement != Announcement::Ended {
            self.announcement = Announcement::Ended;
            let failure = match self.result() {
                Some(Err(why)) => Some(why.to_owned()),
                Some(Ok(
                    domain::ExitStatus::RunFailed
                    | domain::ExitStatus::Budget
                    | domain::ExitStatus::Credentials { .. }
                    | domain::ExitStatus::Refused
                    | domain::ExitStatus::NeedsInput,
                )) => Some(match self.service.end_cause() {
                    Some(domain::EndCause::Host { turn: Some(turn), reason }) => {
                        let reason = match reason {
                            domain::StoreFailure::Read => "read failed",
                            domain::StoreFailure::Write => "write failed",
                        };
                        format!(
                            "run failed: chat {}: turn {turn} could not be kept: {reason}",
                            String::from_utf8_lossy(self.service.chat_name())
                        )
                    }
                    Some(
                        domain::EndCause::Host { turn: None, .. }
                        | domain::EndCause::Agent
                        | domain::EndCause::Cancelled,
                    )
                    | None => "local chat ended with a failure".into(),
                }),
                Some(Ok(domain::ExitStatus::Interrupted)) => {
                    Some("interrupted: the run was dropped before it answered".into())
                }
                Some(Ok(domain::ExitStatus::Accepted)) | None => None,
            };
            if let Some(why) = failure {
                drop(writeln!(self.errors, "smith: {why}"));
            }
        }
    }

    fn iterate(&mut self, now: Time, wall: Wall) {
        self.env.now = now;
        self.env.wall = wall;
        while let Some(complete) = self.completions.pop() {
            match self.routes.remove(&complete.op).expect("each operation has one lower owner") {
                Route::Service(op) => self.service.completions().push(kernel::Complete {
                    op,
                    kind: complete.kind,
                    result: complete.result,
                }),
                Route::Writer(op) => {
                    self.store_completions.push(kernel::Complete { op, kind: complete.kind, result: complete.result });
                }
                Route::TraceRoot => {
                    if complete.result.is_err() {
                        self.error = Some("trace directory close failed".into());
                    }
                }
                Route::Launch => {
                    self.launch_closing = false;
                    if complete.result.is_err() {
                        self.error = Some("launch directory close failed".into());
                    }
                }
            }
        }
        self.writer_up();
        service::iterate(&mut self.service, now, wall);
        self.files();
        self.authentication();
        self.writer_down();
        while let Some(submit) = self.service.submissions().pop() {
            let op = submit.op;
            self.submit(submit, Route::Service(op));
        }
        while let Some(submit) = self.store_submissions.pop() {
            let op = submit.op;
            self.submit(submit, Route::Writer(op));
        }
    }
    fn completions(&mut self) -> &mut Queue<kernel::Complete> {
        &mut self.completions
    }
    fn submissions(&mut self) -> &mut Queue<kernel::Submit> {
        &mut self.submissions
    }
    fn work_pending(&self, now: Time) -> bool {
        self.service.work_pending(now)
            || self.accounts.iter().any(Auth::pending)
            || self.writer.is_ready()
            || self.writer.is_due(now)
            || !self.completions.is_empty()
            || !self.store_completions.is_empty()
            || !self.store_submissions.is_empty()
            || !self.events.is_empty()
            || !self.requests.is_empty()
    }
    fn next_deadline(&self) -> Option<Time> {
        [self.service.next_deadline(), self.writer.next_deadline()]
            .into_iter()
            .flatten()
            .chain(self.accounts.iter().filter_map(Auth::deadline))
            .min()
    }

    /// The service's and the accounts' deadlines; the terminal writer's io
    /// deadline is no policy deadline.
    fn next_policy_deadline(&self) -> Option<Time> {
        self.service.next_deadline().into_iter().chain(self.accounts.iter().filter_map(Auth::deadline)).min()
    }
    fn is_empty(&self) -> bool {
        self.exit.is_some()
            && self.closing
            && self.writer.is_empty()
            && self.pending.is_none()
            && self.routes.is_empty()
            && !self.launch_closing
            && self.launch_root.is_none()
            && self.completions.is_empty()
            && self.submissions.is_empty()
            && self.store_completions.is_empty()
            && self.store_submissions.is_empty()
            && self.events.is_empty()
            && self.requests.is_empty()
    }
    fn exit(&self) -> Option<kernel::Exit> {
        self.result().map(|result| {
            kernel::Exit::Code(match result {
                Ok(domain::ExitStatus::Accepted) => 0,
                Ok(domain::ExitStatus::RunFailed) | Err(_) => 1,
                Ok(domain::ExitStatus::Refused) => 2,
                Ok(domain::ExitStatus::Credentials { .. }) => 3,
                Ok(domain::ExitStatus::Budget) => 4,
                Ok(domain::ExitStatus::NeedsInput) => 5,
                Ok(domain::ExitStatus::Interrupted) => 130,
            })
        })
    }

    fn worst_case(&self) -> u64 {
        self.worst
    }
    fn operations(&self) -> u32 {
        self.operations
    }
}

/// Bounded saved-token documents shared by both placements and the shell store.
#[must_use]
pub fn token_limits() -> skein_oauth::Limits {
    skein_oauth::Limits {
        document_bytes: 16_384,
        string_bytes: 8192,
        token_bytes: 8192,
        client_bytes: 1024,
        detail_bytes: 1024,
        record_bytes: 32_768,
        depth: 16,
        tokens: 1024,
    }
}

fn profile_refusal(error: smith_agent_service::profile::Refusal) -> String {
    String::from_utf8(smith_agent_service::profile::words(&error).into_vec()).expect("refusal words are UTF-8")
}

fn derivation_refusal(field: &'static str) -> String {
    profile_refusal(smith_agent_service::profile::Refusal::Derived {
        limit: smith_agent_service::profile::LimitName::deployment(smith_agent_service::profile::Layer::Service, field),
    })
}

/// The constructor and offline startup checks share one complete shell accounting entrance.
fn accounted_memory(limits: &service::Limits, accounts: u32, lower_routes: u32) -> Result<u64, String> {
    let mut worst = service::worst_case(limits).ok_or_else(|| derivation_refusal("local_worst_case"))?;
    let queue = limits.queue;
    let show_bytes = limits.local.show_bytes;
    let io_limits = limits.process.io;
    worst = worst
        .checked_add(
            local_auth::worst_case()
                .and_then(|bound| bound.checked_mul(u64::from(accounts)))
                .ok_or_else(|| derivation_refusal("account_worst_case"))?,
        )
        .ok_or_else(|| derivation_refusal("account_worst_case"))?;
    let operations = io::operations(&io_limits)
        .and_then(|count| count.checked_mul(2))
        .and_then(|count| count.checked_add(64))
        .and_then(|count| count.checked_add(lower_routes))
        .ok_or_else(|| derivation_refusal("local_routes"))?;
    worst = worst
        .checked_add(io::worst_case(&io_limits).ok_or_else(|| derivation_refusal("local_worst_case"))?)
        .and_then(|count| count.checked_add(u64::from(queue).checked_mul(u64::from(show_bytes.max(8192)))?))
        .and_then(|count| count.checked_add(Queue::<kernel::Complete>::worst_case(queue)?.checked_mul(3)?))
        .and_then(|count| count.checked_add(Queue::<kernel::Submit>::worst_case(queue)?.checked_mul(3)?))
        .and_then(|count| count.checked_add(Queue::<io::Event>::worst_case(queue)?))
        .and_then(|count| count.checked_add(Queue::<io::Request>::worst_case(queue)?))
        .and_then(|count| count.checked_add(Map::<Token, Route>::worst_case(operations)?))
        .and_then(|count| count.checked_add(u64::try_from(size_of::<Local>()).ok()?))
        .ok_or_else(|| derivation_refusal("local_worst_case"))?;
    Ok(worst)
}

fn configuration_failure(detail: String) -> crate::local_shell::Failure {
    crate::local_shell::Failure::Configuration { detail }
}

fn host_failure(detail: String) -> crate::local_shell::Failure {
    crate::local_shell::Failure::Host { detail }
}
