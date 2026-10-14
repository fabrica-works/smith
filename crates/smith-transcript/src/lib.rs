//! Bounded turn records; this crate retains no runtime state and knows no
//! session, provider configuration or host policy. Generated constructors,
//! encoders and decoders are the entry points. Contract:
//! `protocol/transcript.md`, sections 2-7.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[rustfmt::skip]
#[path = "generated/v4.rs"]
pub mod v4;

pub use v4::{
    Affinity, AffinityParts, Ambiguous, AmbiguousParts, Block, ByteClamp, ByteClampParts, BytesClamp, CEILINGS, Call,
    CallParts, CallProblem, CommandEnd, CommandEndParts, CommandRunning, CommandRunningParts, CutProblem,
    CutProblemParts, DurationClamp, DurationClampParts, EditFailed, EditFailedParts, Edited, EditedParts, Entry,
    EntryParts, Exit, ExitCode, ExitCodeParts, ExitSignal, ExitSignalParts, Failed, FailedParts, Fault, FieldProblem,
    FieldProblemParts, Found, FoundParts, Hit, HitParts, Invalid, InvalidParts, JobEnd, JobExit, JobExitParts, JobRead,
    JobReadParts, Kind, LineNumber, LineNumberParts, Listed, ListedParts, Message, MessageParts, Mismatch, Next,
    Opaque, OpaqueParts, OtherPath, OtherPathParts, Outcome, OutsidePath, OutsidePathParts, OversizeProblem,
    OversizeProblemParts, OwnedOutcome, OwnedOutcomeParts, Read, ReadCut, ReadParts, Replay, ReplayParts, Resolved,
    ResolvedFailure, ResolvedFailureParts, Returned, Role, Said, SaidParts, SizePart, Text, TextParts, TimeoutClamp,
    TooLarge, TooLargeParts, ToolResult, ToolResultParts, Turn, TurnKind, TurnParts, Usage, UsageParts, WorkingPath,
    WorkingPathParts, Written, WrittenParts,
};

impl Turn {
    /// The wire format version this codec writes and reads.
    pub const FORMAT_VERSION: u64 = 4;
}
