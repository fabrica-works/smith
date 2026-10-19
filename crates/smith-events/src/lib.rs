//! Versioned JSON-line event codec, with owned bounded records and no domain dependencies.
//! No process state or credential is known here. `write` measures and encodes a line;
//! `write_with_ceiling` uses its owner's joint bound with the same semantic caps.
//! `read` checks its version and decodes a line without retaining stream state.
//! Contract: protocol/events.md, sections 3, 5.3, 6 and 8.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod limits;
mod read;
mod records;
mod words;
mod write;

pub use limits::{Limits, largest_record, largest_record_for, largest_size, worst_case};
pub use read::{Line, Reader, read, read_with_ceiling};
pub use records::{
    AccountState, Agent, AgentExit, AnswerClass, AnswerFailure, Backend, Block, Budget, BudgetStage, BudgetStanding,
    Capture, CheckCompleted, CheckExit, CheckStarted, ChildCompleted, ChildStop, Clamp, CommandExit, Completion,
    CompletionFailure, Containment, ConversationClosed, ConversationEnd, ConversationKind, ConversationOpened, Count,
    Cpu, Delivered, Delivery, Effect, EffectiveLimit, Event, Evidence, FailureClass, FailureLimit, FailurePhase,
    Family, Field, Forced, Form, JobEnded, Level, LimitValue, Loss, Message, MessageRead, MessageReceived,
    MessageRefusal, MessageUnread, Mode, Model, Notice, NoticeKind, Outcome, PeakRss, Prompt, Quantity, Record,
    Requested, Reserve, ResponseCompleted, ResponseStarted, Role, RunCompleted, RunResult, RunStarted, SessionEnded,
    SessionStarted, Share, ShareBound, SharePart, Source, Status, Stop, TextDelta, ToolChoice, ToolCompleted,
    ToolStarted, Tools, Usage, Verdict, Versions,
};
pub use words::{DETAIL_BYTES, clip, clock, effort_words, words};
pub use write::{write, write_with_ceiling};

/// The event vocabulary version emitted and accepted by this codec.
pub const VERSION: u64 = 4;

/// A codec refusal naming unsupported versions and malformed or oversized values.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Error {
    /// A line names a different vocabulary version.
    Version(u64),
    /// A line is not a complete JSON object.
    Malformed,
    /// A known field has a missing or invalid value.
    Shape,
    /// Input or configured bounds cannot be honored.
    Limits,
    /// A caller supplied content that is not UTF-8.
    Text,
}

#[cfg(test)]
mod tests;
