//! Bounded turn records; this crate retains no runtime state and knows no
//! session, provider configuration or host policy. Generated constructors,
//! encoders and decoders are the entry points. Contract:
//! `protocol/transcript.md`, sections 2-7.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[rustfmt::skip]
#[path = "generated/v5.rs"]
pub mod v5;

pub use v5::{
    Affinity, AffinityParts, AfterFailedWrite, AfterFailedWriteParts, AllowedCalls, AllowedCallsParts, Ambiguous,
    AmbiguousParts, Block, ByteClamp, ByteClampParts, BytesClamp, CEILINGS, Call, CallName, CallNameParts, CallParts,
    CallProblem, CommandEnd, CommandEndParts, CommandRunning, CommandRunningParts, CutProblem, CutProblemParts,
    DurationClamp, DurationClampParts, EditFailed, EditFailedParts, Edited, EditedParts, Entry, EntryParts, Exit,
    ExitCode, ExitCodeParts, ExitSignal, ExitSignalParts, Failed, FailedParts, Fault, FieldProblem, FieldProblemParts,
    Found, FoundParts, Hit, HitParts, Invalid, InvalidParts, JobEnd, JobExit, JobExitParts, JobRead, JobReadParts,
    JobsFullProblem, JobsFullProblemParts, Kind, LineNumber, LineNumberParts, Listed, ListedParts, Message,
    MessageParts, Mismatch, Next, NotRun, NotRunParts, NotRunWhy, Opaque, OpaqueParts, OtherPath, OtherPathParts,
    Outcome, OutsideChoice, OutsideChoiceParts, OutsidePath, OutsidePathParts, OversizeProblem, OversizeProblemParts,
    OwnedOutcome, OwnedOutcomeParts, Read, ReadCut, ReadParts, Replay, ReplayParts, Resolved, ResolvedFailure,
    ResolvedFailureParts, Returned, Role, Said, SaidParts, SizePart, Text, TextParts, TimeoutClamp, TooLarge,
    TooLargeParts, ToolResult, ToolResultParts, Turn, TurnKind, TurnParts, UnknownJobProblem, UnknownJobProblemParts,
    Usage, UsageParts, WorkingPath, WorkingPathParts, Written, WrittenParts,
};

impl Turn {
    /// The wire format version this codec writes and reads.
    pub const FORMAT_VERSION: u64 = 5;
}
