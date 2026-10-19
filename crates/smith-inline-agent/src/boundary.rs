//! Lower routing for an inline run (domain/host.md, sections 9.1 and 9.4).
//! These stateless records retain owned requests and exact callback owners;
//! no operation is answered here and each provider request owes one terminal.

use alloc::boxed::Box;
use skein_lib::{Time, Token};
use smith_domain as smith;
use smith_host_domain::parent;

/// One provider terminal sent by the root to its generation-safe agent handle.
#[derive(PartialEq, Eq, Debug)]
pub enum Below {
    /// A provider supplied its bounded completion; ends one Complete.
    Completed { owner: Token, completion: smith::llm::Completion },
    /// A provider failed with actual transport evidence; ends one Complete.
    Failed {
        owner: Token,
        failure: smith::llm::Failure,
        evidence: smith::llm::Evidence,
        detail: Box<[u8]>,
        report: smith::llm::Report,
    },
    /// A tool IO terminal sent by the machine; ends one Io.
    Done { owner: Token, done: smith::tools::Done },
    /// A guide read terminal sent by the machine; ends one Read.
    Read { owner: Token, read: smith::run::Read },
    /// An executable probe sent by the machine; ends one Probe.
    Probed { owner: Token, executable: bool },
    /// A contained check terminal sent by the machine; ends one Check.
    Checked { owner: Token, ran: smith::run::Ran },
    /// A stopped contained check sent by the machine; ends one Check.
    Aborted { owner: Token },
    /// A provider confirmed cancellation; ends one Complete.
    Cancelled { owner: Token },
}

/// Ordered requests sent to the parent or its lower protocol components.
#[derive(PartialEq, Eq, Debug)]
pub enum Output {
    /// A notice in the same parent vocabulary as the spawned capability.
    Parent(parent::Request),
    /// A root request scoped by its agent handle; its original owner is unchanged.
    Lower { agent: Token, request: Lower },
}

/// An operation for the owner to route to its provider component.
#[derive(PartialEq, Eq, Debug)]
pub enum Lower {
    /// A completion requested by a run; ends with Completed, Failed or Cancelled.
    Complete {
        owner: Token,
        name: smith::llm::CompletionName,
        grant: Option<smith::GrantName>,
        prompt: smith::llm::Prompt,
        deadlines: smith::session::Deadlines,
        max_completion_bytes: u64,
        max_completion_blocks: u32,
        max_failure_bytes: u32,
        decoded_call_bytes: u64,
    },
    /// A tool operation routed to the machine; ends with Done.
    Io { owner: Token, op: smith::tools::Op, deadline: Time },
    /// A held command's read or stop, routed to the machine; ends with Done.
    Job { owner: Token, act: smith::tools::JobAct },
    /// Cancel a tool operation; its Done remains owed.
    CancelIo { owner: Token },
    /// Read a bounded guide from a mounted root; ends with Read.
    Read { owner: Token, at: smith::run::Place, max: u32, deadline: Time },
    /// Probe the check executable; ends with Probed.
    Probe { owner: Token, at: smith::run::Place, deadline: Time },
    /// Run contained checks; ends with Checked or Aborted.
    Check { owner: Token, program: smith::run::Place, deadline: Time, tail: u32 },
    /// Stop contained checks; their existing terminal remains owed.
    Abort { owner: Token },
    /// Cancel a completion; its existing terminal remains owed.
    Cancel { owner: Token },
}

/// A parent command or lower terminal sent by the owning host.
#[derive(PartialEq, Eq, Debug)]
#[expect(clippy::large_enum_variant, reason = "the parent face moves values without an extra allocation")]
pub enum Input {
    /// A command in the face shared by both agent kinds.
    Parent(parent::Event),
    /// One terminal for a lower request, scoped to the agent that asked.
    Below { agent: Token, terminal: Below },
}
