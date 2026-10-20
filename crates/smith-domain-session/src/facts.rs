//! What the sessions tell whoever watches the agent (domain/session.md,
//! sections 3–5): a fact for each thing that happened: bounded provider identity, counts and
//! classifications, never prompts, tool inputs or results, in a bounded
//! queue the parent drains at its own pace.
//!
//! Facts are a separate step output: the parent reserves [`crate::max_facts`]
//! slots before each entrance and drains them before taking more work.

use alloc::boxed::Box;

use skein_lib::{Duration, Queue, Time, Token};
use smith_domain_tools as tools;

use crate::boundary::{End, Yield};
use crate::llm::{Failure, Stop, Usage};

/// An observation stamped with the injected time of its emitting step.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Fact {
    /// Emission time, preserved when parents drain this observation later.
    pub at: Time,
    /// The content-free observation.
    pub kind: FactKind,
    /// Provider call metadata for a workspace or delegated call observation.
    pub call: Option<ToolCall>,
    /// Conversation-local completion identity and its exact accepted charge.
    pub response: Option<ResponseInfo>,
}

/// Content-free tool selection on the actual session completion request (protocol/llm.md, section 5).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Choice {
    /// Any offered tool may be called.
    Auto,
    /// No tool may be called.
    None,
    /// Only the selected offered descriptors may be called.
    Only,
}

/// Whether a summarised window was replaced or its compaction failed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Compacted {
    /// The compaction told a window turn and opened the next window.
    Opened,
    /// The compaction ended without opening a window.
    Failed,
}

/// Something that happened in the session opened for `opener`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum FactKind {
    /// The session was admitted.
    Opened { opener: Token },
    /// A summary completion is replacing this conversation's current window.
    CompactionStarted {
        opener: Token,
        window: u32,
        model: [u8; 128],
        model_bytes: u32,
        model_window: u32,
        output: u32,
        effort: Option<([u8; 32], u32)>,
    },
    /// The one compaction of this window has settled.
    CompactionEnded { opener: Token, window: u32, end: Compacted },
    /// A completion was asked for, after `attempt` retries, with `messages`
    /// messages and room for `max_tokens` in its answer.
    CompletionStarted {
        opener: Token,
        /// Current context window, numbered from one.
        window: u32,
        /// Retry number for the pending provider completion.
        attempt: u32,
        /// Oldest-first conversation messages, with provider call/result pairing preserved.
        messages: u32,
        /// Maximum output tokens requested for one completion.
        max_tokens: u32,
        /// Logical selection; provider rendering may use its ordinary automatic mode.
        choice: Choice,
    },
    /// The completion came back with an answer of `blocks` blocks, `calls` of
    /// them tool calls, of which `invalid` could not be decoded.
    CompletionAnswered {
        opener: Token,
        stop: Stop,
        /// Count of provider answer blocks, independent of their content.
        blocks: u32,
        /// Maximum provider calls held at once.
        calls: u32,
        /// Count of calls which could not be decoded or admitted.
        invalid: u32,
    },
    /// An opted-in reasoning item was omitted from this completion and history.
    ReasoningDropped { opener: Token, bytes: u64 },
    /// The completion produced no answer.
    CompletionFailed {
        opener: Token,
        failure: Failure,

        /// Content-free lower transport evidence; diagnostic text is not retained.
        evidence: crate::llm::Evidence,
    },
    /// The completion was abandoned.
    CompletionCancelled { opener: Token },
    /// The completion that failed is tried again after `delay`, as retry
    /// `attempt`.
    CompletionRetried {
        opener: Token,
        /// Retry number for the pending provider completion.
        attempt: u32,
        /// Injected backoff interval before retrying the failed completion.
        delay: Duration,
    },
    /// What the session's own tools told, of the kit the session has: its
    /// calls starting and being answered, and the kit opening and closing.
    /// The tools name the session by its own token; `opener` names its
    /// conversation, as every other fact does.
    Tools {
        opener: Token,
        /// Content-free child observation, preserved in reserved output room.
        fact: tools::FactKind,
    },
    /// A program starts its built-in call at `block` (protocol/programs.md, 5.4–5.5).
    BuiltinStarted { opener: Token, block: u32 },
    /// A program's result, or its retained stand-in, settles that call once.
    BuiltinAnswered { opener: Token, bytes: u64, error: bool },
    /// The tool call at `block` of the last message was delegated to the
    /// opener.
    DelegateStarted {
        opener: Token,
        /// Index of the delegated tool-call block in its completion.
        block: u32,
    },
    /// The opener answered, with `bytes`; `error` marks a failure.
    DelegateAnswered {
        opener: Token,
        /// Owned payload bytes charged against the enclosing session limit.
        bytes: u64,
        /// Whether the returned tool result represents a failure.
        error: bool,
    },
    /// The delegated call was withdrawn.
    DelegateCancelled { opener: Token },
    /// The opener parked its own wait without a historical result.
    DelegateWithheld { opener: Token },
    /// An unanswered job ended before its session closed; its provider identity is in Fact.call.
    JobEnded { opener: Token, job: tools::JobNumber, end: tools::JobEnd, unread_bytes: u64, started: Time },
    /// The session yielded.
    Yielded { opener: Token, stop: Yield },
    /// A completion came back, and used one turn and `usage`.
    Used { opener: Token, usage: Usage },
    /// The session ended, or was refused at the entrance.
    Ended {
        opener: Token,
        /// Terminal classification after everything started beneath this entity has settled.
        end: End,
        /// Completion count, bounded across the enclosing run or session.
        turns: u32,
        usage: Usage,
    },
}

/// Reserved step observations not yet drained by the parent.
#[derive(Debug)]
pub(crate) struct Facts {
    queue: Queue<Fact>,
    now: Time,
}

impl Facts {
    pub(crate) fn with_capacity(capacity: u32) -> Facts {
        Facts { queue: Queue::with_capacity(capacity), now: Time::ZERO }
    }

    pub(crate) const fn now(&self) -> Time {
        self.now
    }

    pub(crate) fn begin(&mut self, now: Time) {
        self.now = now;
    }

    /// Keeps one fact in the room the parent reserved for this entry point.
    pub(crate) fn push(&mut self, kind: FactKind) {
        let fact = Fact { at: self.now, kind, call: None, response: None };
        self.queue.push(fact);
    }

    pub(crate) fn push_at(&mut self, at: Time, kind: FactKind) {
        self.queue.push(Fact { at, kind, call: None, response: None });
    }

    pub(crate) fn push_call(&mut self, at: Time, kind: FactKind, call: &ToolCall) {
        assert!(self.queue.room() > 0, "parent reserved call observation before cloning");
        self.queue.push(Fact { at, kind, call: Some(call.clone()), response: None });
    }

    pub(crate) fn push_response(&mut self, kind: FactKind, number: u64, spent: Option<u64>) {
        self.queue.push(Fact { at: self.now, kind, call: None, response: Some(ResponseInfo { number, spent }) });
    }

    pub(crate) fn pop(&mut self) -> Option<Fact> {
        self.queue.pop()
    }

    pub(crate) fn room(&self) -> u32 {
        self.queue.room()
    }
}

/// The owner of a tool declaration, supplied by the conversation opener.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ToolSource {
    /// The session's checkout tools.
    Workspace,
    /// A run-owned tool such as `sub_agent` or finish.
    Run,
    /// An opaque tool declared by the host.
    Host,
}

/// Exact provider call identity and content-free start measurements.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ToolCall {
    /// Session-issued call identity, also used by a run opening its child.
    pub owner: Token,
    pub id: Box<[u8]>,
    pub name: Box<[u8]>,
    pub source: ToolSource,
    pub effect: tools::Effect,
    pub deadline: Time,
    /// Exact raw provider argument bytes, independent of decoded storage.
    pub input_bytes: u64,
}

/// A completion's number within its conversation and its accepted host-unit charge.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ResponseInfo {
    /// One-based completed turn, reused by retries of that turn.
    pub number: u64,
    /// Accepted charge in the opener's price unit; None before accounting.
    pub spent: Option<u64>,
}

impl FactKind {
    /// Copy admitted model identity into a bounded compaction observation.
    #[must_use]
    pub fn compaction_started(
        opener: Token,
        window: u32,
        model: &[u8],
        model_window: u32,
        output: u32,
        effort: Option<&[u8]>,
    ) -> Self {
        let mut name = [0; 128];
        assert!(model.len() <= name.len(), "admitted model name");
        for (slot, byte) in name.iter_mut().zip(model) {
            *slot = *byte;
        }
        let effort = match effort {
            Some(value) => {
                let mut bytes = [0; 32];
                assert!(value.len() <= bytes.len(), "admitted effort name");
                for (slot, byte) in bytes.iter_mut().zip(value) {
                    *slot = *byte;
                }
                Some((bytes, u32::try_from(value.len()).expect("bounded effort")))
            }
            None => None,
        };
        Self::CompactionStarted {
            opener,
            window,
            model: name,
            model_bytes: u32::try_from(model.len()).expect("bounded model"),
            model_window,
            output,
            effort,
        }
    }
}
