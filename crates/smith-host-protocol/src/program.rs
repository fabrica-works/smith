//! Program source operations (protocol/hosts.md, section 2; protocol/programs.md,
//! sections 3 and 4.1). Keeps one child command and a single answer right; never
//! reads a login file. `new`, `from_io` and `fire` drive the command; `answer`
//! translates its settled record into the host domain's vocabulary.

use alloc::boxed::Box;
use core::mem::size_of;
use skein_io::{Event as IoEvent, Request as IoRequest, kernel};
use skein_lib::{Queue, Time, Token};
use smith_host_domain::{FailureReason, ProgramCheck, ProgramFailure, ProgramMethod, ProgramSignOut, SourceName};
use smith_protocol_program::{self as program, Command, Ended, Programs, Status};

/// A host's request for its half to inspect or sign out a program source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceAsk {
    /// Read the program's status and return a `ProgramCheck`.
    Check,
    /// Run the program's own sign-out and return a `ProgramSignOut`.
    SignOut,
}

/// The half's single terminal answer to a source request from its host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceAnswer {
    /// The source's status, answering Check.
    Check(ProgramCheck),
    /// The source's sign-out result, answering `SignOut`.
    SignOut(ProgramSignOut),
}

/// One program source request from the host, answered after its child settles.
#[derive(Debug)]
pub struct ProgramSource {
    command: Command,
    ask: SourceAsk,
    answered: bool,
}

/// One source command, its host adapter and the returned stderr detail.
#[must_use]
pub fn program_source_worst_case(detail_bytes: u32) -> Option<u64> {
    program::command_worst_case(detail_bytes)?
        .checked_add(u64::from(detail_bytes))?
        .checked_add(u64::try_from(size_of::<ProgramSource>()).ok()?)
}

impl ProgramSource {
    /// Spawn the requested operation in the source's explicit environment.
    #[expect(clippy::too_many_arguments, reason = "a host's source binding and deadline cross together")]
    pub fn new(
        io_owner: Token,
        ask: SourceAsk,
        programs: &Programs,
        source: SourceName,
        base: &[Box<[u8]>],
        root: kernel::Fd,
        directory: Box<[u8]>,
        deadline: Time,
        detail_bytes: u32,
        below: &mut Queue<IoRequest>,
    ) -> Result<Self, program::Error> {
        let configuration = program::configuration_directory(programs, source.as_bytes())?;
        let environment = program::environment(base, &configuration)?;
        let arguments = match ask {
            SourceAsk::Check => program::STATUS,
            SourceAsk::SignOut => program::SIGN_OUT,
        };
        let launch = program::Launch {
            program: programs.claude.clone(),
            arguments: Box::new([
                Box::from(arguments.first().copied().expect("auth argument")),
                Box::from(arguments.get(1).copied().expect("operation argument")),
            ]),
            environment,
            root,
            directory,
        };
        Ok(Self { command: Command::new(io_owner, launch, deadline, detail_bytes, below), ask, answered: false })
    }

    /// Whether this I/O owner belongs to the source command.
    #[must_use]
    pub fn owns(&self, owner: Token) -> bool {
        self.command.owns(owner)
    }

    /// Drive one child or output-pipe event.
    pub fn from_io(&mut self, event: IoEvent, below: &mut Queue<IoRequest>) {
        self.command.from_io(event, below);
    }

    /// Enforce the host's deadline while the child is active.
    pub fn fire(&mut self, now: Time, below: &mut Queue<IoRequest>) {
        self.command.fire(now, below);
    }

    /// Cancel the host's source operation, answering only after its resources settle.
    pub fn cancel(&mut self, below: &mut Queue<IoRequest>) {
        self.command.cancel(below);
    }

    /// The source command's next host deadline.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Time> {
        self.command.next_deadline()
    }

    /// Take the answer once, after the child and its output pipes have settled.
    pub fn answer(&mut self) -> Option<SourceAnswer> {
        if self.answered {
            return None;
        }
        let ended = self.command.ended()?;
        let answer = translate(self.ask, ended);
        self.answered = true;
        Some(answer)
    }
}

fn translate(ask: SourceAsk, ended: Ended<'_>) -> SourceAnswer {
    match ended {
        Ended::Unspawned { detail } => failure(ask, FailureReason::Missing, detail),
        Ended::Deadline { detail } => failure(ask, FailureReason::Deadline, detail),
        Ended::Exited { exit, stdout, detail } => match ask {
            SourceAsk::Check => match exit {
                kernel::Exit::Code(code @ (0 | 1)) => match stdout {
                    Some(stdout) => match program::status(code, stdout) {
                        Status::Usable => SourceAnswer::Check(ProgramCheck::Usable),
                        Status::NotSignedIn => SourceAnswer::Check(ProgramCheck::NotSignedIn),
                        Status::Otherwise(method) => SourceAnswer::Check(ProgramCheck::SignedInOtherwise(
                            ProgramMethod::new(method.as_bytes()).expect("same printable method bounds"),
                        )),
                        Status::Unreadable => failure(ask, FailureReason::Unreadable, detail),
                    },
                    None => failure(ask, FailureReason::Unreadable, detail),
                },
                kernel::Exit::Code(_) | kernel::Exit::Signal(_) => failure(ask, FailureReason::Exited, detail),
            },
            SourceAsk::SignOut => match exit {
                kernel::Exit::Code(0) => SourceAnswer::SignOut(ProgramSignOut::SignedOut),
                kernel::Exit::Code(_) | kernel::Exit::Signal(_) => failure(ask, FailureReason::Exited, detail),
            },
        },
    }
}

fn failure(ask: SourceAsk, reason: FailureReason, detail: &[u8]) -> SourceAnswer {
    let failure = ProgramFailure { reason, detail: Box::from(detail) };
    match ask {
        SourceAsk::Check => SourceAnswer::Check(ProgramCheck::Failed(failure)),
        SourceAsk::SignOut => SourceAnswer::SignOut(ProgramSignOut::Failed(failure)),
    }
}

#[cfg(test)]
mod tests;
