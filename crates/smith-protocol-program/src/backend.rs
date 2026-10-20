//! One lazy program conversation (protocol/programs.md, sections 3 and 5).
//! Keeps a source grant, private run files, status child, MCP listener and
//! program pipes; never reads login contents. `from_domain` starts on demand,
//! `from_io` and `fire` advance startup, and `close` settles every resource.

use alloc::boxed::Box;
use skein_io::{self as io, file, kernel};
use skein_lib::{Duration, Env, List, Queue, Secret, Time, Token};
use smith_domain::{GrantName, llm};
use smith_protocol_llm::{self as protocol, FromDomain, ToDomain};

use crate::{Command, Programs, mcp};

mod diagnostics;
mod files;
mod start;
mod stream;

#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod tests;

/// An adopted programs root or a workspace supplied by the owning service.
#[derive(Clone, Copy, Debug)]
pub struct Root {
    pub file: Token,
    pub descriptor: kernel::Fd,
}

/// Fixed run startup and receiving bounds supplied by the owning service (5.1).
pub struct Settings {
    pub program: Programs,
    pub directory: Option<Root>,
    pub workspace: Option<Root>,
    pub relay: Box<[u8]>,
    pub environment: Box<[Box<[u8]>]>,
    pub mcp: mcp::Limits,
    pub adapter: protocol::Limits,
    pub receiving: protocol::Receiving,
    pub prompt_bytes: u64,
    pub contract_bytes: u32,
    pub line_bytes: u32,
    pub input_bytes: u32,
    pub max_turns: u32,
    pub grace: Duration,
    pub startup: Duration,
}

/// A host's program grant, installed or renewed before its lapse (section 3).
#[derive(Debug)]
pub struct Grant {
    pub name: GrantName,
    pub source: Box<[u8]>,
    pub lapses: Time,
}

/// A grant does not renew this account and source, or has an invalid name.
#[derive(Debug)]
pub struct Refused;

/// One lower effect, routed by the service to native I/O or its file driver.
pub enum Below {
    /// A process, pipe or MCP operation, ending through `BelowEvent::Io`.
    Io(io::Request),
    /// A file operation, ending through `BelowEvent::File`.
    File { request: file::Request, deadline: Time },
}

/// A lower terminal supplied by the service to this backend (sections 3 and 5).
pub enum BelowEvent {
    /// A child, pipe or listener event belonging to this backend.
    Io(io::Event),
    /// One terminal for a private file operation.
    File { event: file::Event, descriptor: Option<kernel::Fd> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Checking,
    Starting,
    Running,
    Ending,
    Ended,
}

/// The run's program adapter, driven by the service (programs.md, sections 3, 5).
pub struct Backend {
    first: Token,
    settings: Settings,
    names: start::Names,
    grant: Option<Grant>,
    phase: Phase,
    owner: Option<Token>,
    request: Option<FromDomain>,
    check: Option<Command>,
    server: Option<mcp::Server>,
    server_ended: bool,
    files: files::Files,
    child: Option<Token>,
    spawning: bool,
    pipes: [Option<Token>; 3],
    outgoing: Option<Box<[u8]>>,
    written: u32,
    exit: Option<u32>,
    deadline: Option<Time>,
    stopping: u8,
    line: List<u8>,
    stdout_demanding: bool,
    calls: List<mcp::Up>,
    stderr: List<u8>,
    tail: u32,
    io: Queue<io::Request>,
}

impl core::fmt::Debug for Backend {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Backend")
            .field("phase", &self.phase)
            .field("owner", &self.owner)
            .finish_non_exhaustive()
    }
}

/// Complete additional protocol ownership, including transient copies (5.1–5.6).
/// Native I/O tables and the shared file driver are priced by the service.
#[must_use]
pub fn backend_worst_case(settings: &Settings) -> Option<u64> {
    let mut paths = u64::try_from(settings.program.directory.len().max(4096)).ok()?;
    paths = paths
        .checked_add(u64::try_from(settings.program.claude.len()).ok()?)?
        .checked_add(u64::try_from(settings.relay.len().max(4096)).ok()?)?;
    let mut environment = 0_u64;
    for entry in &settings.environment {
        environment = environment
            .checked_add(u64::try_from(entry.len()).ok()?)?
            .checked_add(u64::try_from(size_of::<Box<[u8]>>()).ok()?)?;
    }
    u64::try_from(size_of::<Backend>())
        .ok()?
        // Launch peak: settings paths; executable; instructions and MCP path
        // temporaries; their two argv copies; relative cwd; source directory.
        .checked_add(paths.checked_mul(8)?)?
        // Settings' command entries and the spawn's owned environment. The
        // service prices its machine environment independently. Fixed room
        // covers the synthesized config-directory entry, secret and argv cells.
        .checked_add(environment.checked_mul(2)?.checked_add(32_768)?)?
        .checked_add(
            crate::command_worst_case(settings.receiving.max_failure_bytes)?
                .checked_sub(u64::try_from(size_of::<Command>()).ok()?)?,
        )?
        .checked_add(mcp::mcp_worst_case(&settings.mcp)?.checked_sub(u64::try_from(size_of::<mcp::Server>()).ok()?)?)?
        .checked_add(List::<mcp::Up>::worst_case(u32::from(settings.mcp.calls))?)?
        .checked_add(u64::from(settings.mcp.message_bytes).checked_mul(u64::from(settings.mcp.calls))?)?
        // Held neutral prompt and the instructions Store's system-text copy.
        // Tool/schema inventories are already in the MCP bound, user encoding
        // in the line terms; neither is a third neutral prompt allocation.
        .checked_add(settings.prompt_bytes.checked_mul(2)?)?
        // Held outcome/delivery contracts, finish_schema's bytes and their
        // inline argv copy coexist during launch. MCP's schemas/list/escaping
        // are in mcp_worst_case, rather than another thirteen contract copies.
        .checked_add(u64::from(settings.contract_bytes).checked_mul(3)?)?
        // Four queued completions plus the message currently being assembled.
        .checked_add(settings.receiving.max_completion_bytes.checked_mul(5)?)?
        // Stderr ring, chronological tail, and sanitized failure terminal.
        // The sanitizer's List becomes that terminal Box without a fourth copy;
        // the status command's capture/scratch is in command_worst_case.
        .checked_add(u64::from(settings.receiving.max_failure_bytes).checked_mul(3)?)?
        // Input encoding precedes the child and stream collection. The JSON
        // encoder's body and the newline copy overlap; the larger workspace
        // is reserved once, beside the prepared input and raw stdout line.
        .checked_add(
            stream::worst_case(settings.line_bytes, settings.adapter.client.depth, settings.adapter.client.tokens)?
                .max(skein_json::writer::worst_case(&skein_json::writer::Limits {
                    depth: 8,
                    length: settings.input_bytes,
                })?),
        )?
        .checked_add(u64::from(settings.line_bytes))?
        .checked_add(u64::from(settings.input_bytes))?
        .checked_add(4096)? // a failed Store's bounded temporary leaf
        .checked_add(Queue::<io::Request>::worst_case(32)?)?
        .checked_add(Queue::<Below>::worst_case(32)?)
}

impl Backend {
    /// Retain settings and names; emit nothing until the first Complete (section 3).
    #[must_use]
    pub fn new(first: Token, settings: Settings, entropy: &Secret) -> Self {
        let failure_bytes = settings.receiving.max_failure_bytes;
        let line = List::with_capacity(settings.line_bytes);
        let calls = List::with_capacity(u32::from(settings.mcp.calls));
        Self {
            first,
            settings,
            names: start::Names::new(entropy),
            grant: None,
            phase: Phase::Idle,
            owner: None,
            request: None,
            check: None,
            server: None,
            server_ended: false,
            files: files::Files::new(),
            child: None,
            spawning: false,
            pipes: [None; 3],
            outgoing: None,
            written: 0,
            exit: None,
            deadline: None,
            stopping: 0,
            line,
            stdout_demanding: false,
            calls,
            stderr: List::with_capacity(failure_bytes),
            tail: 0,
            io: Queue::with_capacity(32),
        }
    }

    /// Adopt the shell's programs directory; its close belongs to this backend.
    pub fn directory(&mut self, root: Root) {
        assert_eq!(self.phase, Phase::Idle, "programs root is supplied before startup");
        self.settings.directory = Some(root);
    }

    /// Supply the first admitted workspace directory, borrowed until backend close.
    pub fn workspace(&mut self, root: Root) {
        assert_eq!(self.phase, Phase::Idle, "workspace is supplied before startup");
        self.settings.workspace = Some(root);
    }

    /// Install the source or renew the same account's generation and lapse (section 3).
    pub fn grant(&mut self, grant: Grant) -> Result<(), Refused> {
        crate::configuration_directory(&self.settings.program, &grant.source).or(Err(Refused))?;
        match &self.grant {
            Some(held)
                if held.name.account != grant.name.account
                    || held.source != grant.source
                    || grant.name.generation < held.name.generation =>
            {
                return Err(Refused);
            }
            Some(_) | None => {}
        }
        self.grant = Some(grant);
        Ok(())
    }

    /// Whether the service should route this listener, child, pipe or file owner here.
    #[must_use]
    pub fn owns(&self, owner: Token) -> bool {
        owner == self.token(3)
            || owner == self.token(4)
            || owner == self.token(5)
            || self.pipes.contains(&Some(owner))
            || match &self.check {
                Some(check) => check.owns(owner),
                None => false,
            }
            || match &self.server {
                Some(server) => server.owns(owner),
                None => false,
            }
    }

    /// Set the admitted charter's turn guard before its first completion (5.7).
    pub fn turn_limit(&mut self, turns: u32) {
        self.settings.max_turns = turns;
    }

    /// Whether a cancellation belongs to the outstanding program completion.
    #[must_use]
    pub fn pending(&self, owner: Token) -> bool {
        self.owner == Some(owner)
    }

    /// Whether a completion grant belongs to the installed program account.
    #[must_use]
    pub fn holds(&self, name: GrantName) -> bool {
        match &self.grant {
            Some(grant) => grant.name.account == name.account,
            None => false,
        }
    }

    /// Begin the lazy startup or cancel its outstanding completion (sections 3, 5.7).
    pub fn from_domain(
        &mut self,
        env: &Env<protocol::ComponentLimits>,
        request: FromDomain,
        up: &mut Queue<ToDomain>,
        below: &mut Queue<Below>,
    ) {
        match request {
            request @ FromDomain::Complete { .. } => self.start(env, request, up, below),
            FromDomain::Cancel { owner } => {
                if self.owner == Some(owner) {
                    if let Some(child) = self.child {
                        self.io.push(io::Request::Signal {
                            child,
                            to: kernel::Target::Group,
                            signal: kernel::Signal::Interrupt,
                        });
                    }
                    self.owner = None;
                    up.push(ToDomain::Cancelled { owner });
                    self.close(env, up, below);
                }
            }
        }
        self.flush(below);
    }

    /// Consume one lower terminal and advance startup or resource settlement (5.1).
    pub fn from_io(
        &mut self,
        env: &Env<protocol::ComponentLimits>,
        event: BelowEvent,
        up: &mut Queue<ToDomain>,
        below: &mut Queue<Below>,
    ) {
        match event {
            BelowEvent::File { event, descriptor } => self.file_event(env, event, descriptor, up, below),
            BelowEvent::Io(event) => self.io_event(env, event, up, below),
        }
        self.progress(env, up, below);
        self.flush(below);
    }

    /// Advance the check, startup deadline and close grace at the supplied clock.
    pub fn fire(&mut self, env: &Env<protocol::ComponentLimits>, up: &mut Queue<ToDomain>, below: &mut Queue<Below>) {
        if let Some(check) = &mut self.check {
            check.fire(env.now, &mut self.io);
        }
        if let Some(server) = &mut self.server {
            server.fire(env.now, &mut self.io);
        }
        let due = match self.deadline {
            Some(deadline) => env.now >= deadline,
            None => false,
        };
        if due {
            self.deadline = None;
            match self.phase {
                Phase::Starting => self.fail(
                    env,
                    llm::Failure::Program { exit: self.exit },
                    b"smith: the program did not initialize",
                    up,
                    below,
                ),
                Phase::Ending => self.terminate(env.now),
                Phase::Idle | Phase::Checking | Phase::Running | Phase::Ended => {}
            }
        }
        self.progress(env, up, below);
        self.flush(below);
    }

    fn terminate(&mut self, now: Time) {
        self.stopping = self.stopping.saturating_add(1).min(2);
        if let Some(child) = self.child {
            self.io.push(io::Request::Signal {
                child,
                to: kernel::Target::Group,
                signal: if self.stopping == 1 { kernel::Signal::Terminate } else { kernel::Signal::Kill },
            });
            if self.stopping == 2 {
                for pipe in self.pipes.into_iter().flatten() {
                    self.io.push(io::Request::Abort { entity: pipe });
                }
                self.io.push(io::Request::Close { entity: child });
            }
        }
        if self.child.is_some() || self.spawning {
            self.deadline = Some(now.saturating_add(self.settings.grace));
        }
    }

    /// Earliest check, server authentication, startup or close deadline.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Time> {
        if self.closed() {
            return None;
        }
        let mut earliest = self.deadline;
        for at in [
            match &self.check {
                Some(check) => check.next_deadline(),
                None => None,
            },
            match &self.server {
                Some(server) => server.next_deadline(),
                None => None,
            },
        ]
        .into_iter()
        .flatten()
        {
            earliest = Some(match earliest {
                Some(held) => held.min(at),
                None => at,
            });
        }
        earliest
    }

    /// End the conversation, closing input before terminating at its grace (5.7).
    pub fn close(&mut self, env: &Env<protocol::ComponentLimits>, up: &mut Queue<ToDomain>, below: &mut Queue<Below>) {
        match self.phase {
            Phase::Ending | Phase::Ended => return,
            Phase::Idle | Phase::Checking | Phase::Starting | Phase::Running => {}
        }
        self.phase = Phase::Ending;
        if let Some(owner) = self.owner.take() {
            up.push(ToDomain::Cancelled { owner });
        }
        self.request = None;
        self.outgoing = None;
        self.deadline = if self.child.is_some() || self.spawning {
            Some(env.now.saturating_add(self.settings.grace))
        } else {
            None
        };
        if let Some(input) = self.pipes.first().copied().flatten() {
            self.io.push(io::Request::Close { entity: input });
        }
        self.read_stdout();
        if let Some(check) = &mut self.check {
            check.fire(Time::from_nanos(u64::MAX), &mut self.io);
        }
        if let Some(server) = &mut self.server {
            server.close(&mut self.io);
        }
        self.progress(env, up, below);
        self.flush(below);
    }

    /// Every owned resource, including the programs root, has reported closed.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.phase == Phase::Ended
    }

    fn token(&self, offset: u64) -> Token {
        Token::new(self.first.raw().checked_add(offset).expect("backend token range"))
    }

    fn flush(&mut self, below: &mut Queue<Below>) {
        for _ in 0..self.io.capacity() {
            match self.io.pop() {
                Some(request) => below.push(Below::Io(request)),
                None => break,
            }
        }
    }

    fn fail(
        &mut self,
        env: &Env<protocol::ComponentLimits>,
        failure: llm::Failure,
        detail: &[u8],
        up: &mut Queue<ToDomain>,
        below: &mut Queue<Below>,
    ) {
        if let Some(owner) = self.owner.take() {
            let detail = self.private_diagnostic(detail);
            up.push(ToDomain::Failed {
                owner,
                failure,
                evidence: llm::Evidence::Unsent,
                detail,
                report: llm::Report::none(),
            });
        }
        self.close(env, up, below);
    }
}

// The channel's program detail is text. Preserve valid bytes and replace only
// malformed sequences, keeping the same byte ceiling even at a split UTF-8 tail.
#[expect(
    clippy::disallowed_methods,
    reason = "protocol/programs.md, section 5.6: the protocol validates stderr at its text boundary"
)]
fn diagnostic(bytes: &[u8], maximum: u32) -> Box<[u8]> {
    let length = bytes.len().min(usize::try_from(maximum).expect("failure cap"));
    let mut output = List::with_capacity(u32::try_from(length).expect("failure cap"));
    let mut from = 0_usize;
    for _ in 0..=length {
        let rest = bytes.get(from..length).expect("diagnostic cursor");
        match core::str::from_utf8(rest) {
            Ok(_) => {
                for byte in rest {
                    output.push(*byte).expect("diagnostic room");
                }
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                for byte in rest.get(..valid).expect("valid UTF-8 prefix") {
                    output.push(*byte).expect("diagnostic room");
                }
                from = from.checked_add(valid).expect("bounded diagnostic cursor");
                match error.error_len() {
                    Some(count) => {
                        output.push(b'?').expect("one byte replaces at least one malformed byte");
                        from = from.checked_add(count).expect("bounded diagnostic cursor");
                    }
                    None => break,
                }
            }
        }
    }
    output.into_boxed()
}
