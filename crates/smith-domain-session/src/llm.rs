//! The provider-neutral vocabulary of a conversation with an LLM.
//!
//! The domain speaks this to the provider-neutral shared Client. Smith's
//! protocol translates this vocabulary and terminals; Skein owns wire
//! formats and failure classification. Smith owns the application schemas of
//! the tools a prompt offers, decodes the JSON the LLM writes as a tool's input
//! into a typed call, or into the [`Problem`] that keeps it from being one, and
//! renders what comes of a call as the text the LLM reads. The domain never
//! parses: it keeps a call's input as the bytes the LLM wrote, to send back
//! verbatim, beside what was decoded from it.
//!
//! Contract: domain/session.md, section 4; programming-model.md, sections 4.4 and 6.3.

use alloc::boxed::Box;

use skein_lib::{Duration, Token};
use smith_domain_tools as tools;

/// A provider endpoint the protocol layer is configured with: which provider,
/// where, with which credentials. The domain only names it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Endpoint(
    /// Host-configured numeric provider endpoint name, echoed without address resolution or authority inference.
    pub u32,
);

/// A complete shared-client replay envelope, preserved opaquely by the domain.
/// Skein owns its format; Smith's protocol attests its size before delivering a block.
/// This wrapper and all bytes count against completion and transcript ownership.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Replay {
    /// Complete bounded tagged envelope, including unknown provider extensions.
    /// The domain neither parses it nor selects a provider from it.
    pub bytes: Box<[u8]>,
}

/// Who wrote a message.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Role {
    /// The agent's side: the task, and the results of tool calls.
    User,
    /// The LLM.
    Assistant,
}

/// A piece of a message. Text is UTF-8, checked by the protocol layer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Block {
    /// Provider-owned reasoning, preserved in position and replayed verbatim.
    Opaque {
        /// Owned payload bytes charged against the enclosing session limit.
        bytes: Box<[u8]>,
    },
    /// Explicit provider refusal, distinct from ordinary text and preserved in place.
    Refusal {
        /// Provider-attested UTF-8 refusal bytes, bounded with enclosing content.
        text: Box<[u8]>,
        /// Complete optional provider replay envelope, preserved opaquely.
        replay: Option<Replay>,
    },
    /// Owned bounded text in its original provider position.
    Text { text: Box<[u8]>, replay: Option<Replay> },
    /// The LLM asks for a tool to run. `id` is the provider's name for this
    /// call, which its result echoes; `name` and `input` are what the LLM
    /// wrote, the input a JSON object; `call` is what the protocol layer
    /// decoded from them.
    ToolCall {
        id: Box<[u8]>,
        name: Box<[u8]>,
        /// Provider arguments, or canonical `{}` for an oversized or cut call;
        /// counted with their enclosing message against session ownership limits.
        input: Box<[u8]>,
        /// Full decoded classification: owned tool call, opener-served ticket,
        /// invalid-input problem, or non-executable concrete-history replay marker.
        call: Decoded,
        replay: Option<Replay>,
    },
    /// What came of the tool call `id`.
    ToolResult {
        id: Box<[u8]>,
        /// The one terminal value for the enclosing call; ownership passes to its receiver.
        result: Returned,
    },
}

/// A tool call, as the protocol layer decoded it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Decoded {
    /// A program-owned built-in call; the program reports its result later (protocol/programs.md, 5.3–5.5).
    Builtin { effect: tools::Effect },
    /// A call to one of the tools the session owns.
    Owned {
        /// Typed checkout-tool call decoded below the domain.
        call: tools::Call,
    },
    /// A call to one of the tools the opener serves, with `effect`: the typed
    /// call is the opener's, kept under `ticket` (a ticket names a value the
    /// session cannot, and the layer that holds it resolves it).
    Delegated {
        /// Declaration owner supplied by the protocol decoder.
        source: crate::ToolSource,
        /// Opener-issued opaque name for a tool or delegated call.
        ticket: Token,
        /// Descriptor ticket identifying the offered tool, independent of this call.
        descriptor: Token,
        /// Whether this delegated call may write, used to serialize writers.
        effect: tools::Effect,
    },
    /// A call that is no call: the session answers it with its problem, and
    /// runs nothing.
    Invalid {
        /// Offered delegated descriptor, retained even when its arguments are invalid.
        descriptor: Option<Token>,
        /// Typed reason a call could not become an admitted tool operation.
        problem: Problem,
    },
    /// A prior call, kept by provider name and input in its enclosing block.
    /// It is replayed to the provider and never executed again.
    Historical,
}

/// What came of a tool call, for the protocol layer to render as the text the
/// LLM reads.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Returned {
    /// The tools' outcome: a success, or a failure, one that ran out of time
    /// included.
    Owned {
        /// Typed checkout-tool success or failure returned by the tools child;
        /// it is not a declared run result and confers no new authority.
        outcome: tools::Outcome,
    },
    /// The call was malformed, and this is why.
    Invalid {
        /// Typed reason a call could not become an admitted tool operation.
        problem: Problem,
    },
    /// Nothing ran for the call: the LLM stopped for another reason than
    /// calling tools, its answer cut short or its turn ended.
    NotRun { why: NotRun },
    /// An opener's concrete rendered answer, without a local ticket.
    Text {
        /// Opaque opener-provided result bytes, preserved in concrete history.
        /// The constructor does not validate UTF-8; session ownership limits bound retention.
        text: Box<[u8]>,
        /// Whether the returned tool result represents a failure.
        error: bool,
        replay: Option<Replay>,
    },
    /// A delegated call whose withdrawal won its terminal race.
    Withdrawn,
}

/// Why a concrete tool call ran nothing (domain/session.md, section 5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum NotRun {
    /// A previous failed call stopped this completion's remaining changes.
    AfterFailedWrite { call: u16 },
    /// The provider stopped without asking for execution.
    Stopped,
    /// The call was outside the completion's retained choice.
    OutsideChoice { choice: Option<Box<[Box<[u8]>]>> },
}

/// The offered descriptors this completion may call; definitions stay unchanged.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ToolChoice {
    /// Any offered tool, or none.
    Auto,
    /// No tool may run.
    None,
    /// Only the opener descriptors named by these tickets may run.
    Only(Box<[Token]>),
}

/// A tool the opener serves, which the prompt offers the LLM: `ticket` names
/// it, and its schema is the opener's. Calls to it read or write as `effect`
/// says, and are scheduled so.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Descriptor {
    /// Opener-issued opaque name for a tool or delegated call.
    pub ticket: Token,
    /// Whether this delegated call may write, used to serialize writers.
    pub effect: tools::Effect,
}

/// Why a tool call is no call: the protocol layer could not decode it, or the
/// agent could not hold what it decoded.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Problem {
    /// A nesting request refused by the protocol; a sub-agent does its work itself.
    Nesting,

    /// No tool the prompt offered has the call's name.
    UnknownTool,
    /// The input is not a JSON object.
    NotAnObject,
    /// The input has no `field`, which the tool needs.
    Missing {
        /// Name of the required input member which failed validation.
        field: Box<[u8]>,
    },
    /// The input's `field` has the wrong type.
    WrongType {
        /// Name of the required input member which failed validation.
        field: Box<[u8]>,
    },
    /// The input's `field` has a value the tool cannot take: a path with an
    /// empty name or a NUL in it, a number out of range.
    BadValue {
        /// Name of the required input member which failed validation.
        field: Box<[u8]>,
    },
    /// A yielding shell call would exceed its session's job bound.
    JobsFull { jobs: u32 },
    /// A job number was never answered running, or its end was already answered.
    UnknownJob { job: u32 },
    /// The call holds more than the agent takes at once.
    TooLarge,
    /// The received call exceeded its argument bound; no input was retained.
    Oversize { bytes: u64, bound: u32 },
    /// The provider stopped while writing the call's input; no call runs.
    CutOff { bytes: u64 },
}

/// One ordered conversation message, with sender and owned bounded content.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Message {
    /// Sender of this message in the provider-neutral conversation.
    pub role: Role,
    /// Ordered provider-neutral blocks; boxed storage and owned payloads count
    /// against the receiving session's aggregate ownership limits.
    pub content: Box<[Block]>,
}

/// The opener's run identity and conversation ordinal, retained across windows
/// and resumes (domain/session.md, section 2).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Affinity {
    pub key: [u8; 16],
    pub thread: u32,
}

/// Why the session asks for a completion (domain/session.md, section 8.2).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Purpose {
    /// An ordinary completion, guarded when its window has a reported last prompt.
    Ordinary { guard: Option<Guard> },
    /// A summary completion shares this room in bytes among its owed results.
    Compaction { room: u64 },
}

/// What the protocol layer checks before an ordinary request (domain/session.md, section 8.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Guard {
    /// The conversation's line, in tokens.
    pub line: u64,
    /// The window's last reported prompt, in tokens.
    pub last_prompt: u64,
    /// The message index where the last answer starts.
    pub from: usize,
}

/// One call to an LLM: everything it needs to produce the next assistant
/// message.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Prompt {
    pub purpose: Purpose,
    pub endpoint: Endpoint,
    pub affinity: Affinity,
    /// The opening's measured protocol contribution, included in input pricing.
    pub protocol_allowance: u64,
    /// The provider's name for the model.
    pub model: Box<[u8]>,
    /// Reasoning effort requested by this activation, absent for the provider default.
    pub effort: Option<Box<[u8]>>,
    /// System instructions supplied by the opener, retained within the enclosing byte cap.
    pub system: Box<[u8]>,
    /// The families of tools the LLM may call, whose schemas the protocol
    /// layer offers it, and the tools the opener serves.
    pub tools: tools::Grants,
    /// Path presentation fixed from the conversation's workspace authority.
    pub namespace: tools::Namespace,
    /// Opener-owned tool descriptors; tickets are resolved only by the opener.
    pub delegated: Box<[Descriptor]>,
    pub choice: ToolChoice,
    /// The conversation so far, oldest first, ending with a user message.
    pub messages: Box<[Message]>,
    /// The most tokens the answer may take.
    pub max_tokens: u32,
}

impl Prompt {
    /// Checked bytes of the owned domain request before protocol rendering.
    /// Fixed cells and payloads are counted; schemas and provider framing are
    /// added by the parent or covered by the configured protocol allowance.
    #[must_use]
    pub fn input_bytes(&self) -> Option<u64> {
        use core::mem::size_of;

        let mut bytes = u64::try_from(size_of::<Self>())
            .ok()?
            .checked_add(u64::try_from(self.model.len()).ok()?)?
            .checked_add(u64::try_from(self.system.len()).ok()?)?
            .checked_add(
                u64::try_from(size_of::<Descriptor>()).ok()?.checked_mul(u64::try_from(self.delegated.len()).ok()?)?,
            )?
            .checked_add(
                u64::try_from(size_of::<Message>()).ok()?.checked_mul(u64::try_from(self.messages.len()).ok()?)?,
            )?;
        bytes = bytes.checked_add(self.namespace.owned_bytes()?)?.checked_add(self.protocol_allowance)?;
        if let Some(effort) = &self.effort {
            bytes = bytes.checked_add(u64::try_from(effort.len()).ok()?)?;
        }
        match &self.choice {
            ToolChoice::Auto | ToolChoice::None => {}
            ToolChoice::Only(tickets) => {
                bytes = bytes.checked_add(
                    u64::try_from(tickets.len()).ok()?.checked_mul(u64::try_from(size_of::<Token>()).ok()?)?,
                )?;
            }
        }
        for message in &self.messages {
            bytes = bytes.checked_add(crate::session::content_cost(&message.content)?)?;
        }
        Some(bytes)
    }
}

/// The session's name for a pending completion (domain/session.md, sections 3–4).
/// Retries keep the sequence and advance only the retry; accepted completions
/// advance the transcript sequence before another completion begins.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CompletionName {
    /// Next concrete transcript sequence, starting at one; scoped by the session owner token.
    pub sequence: u32,
    /// Provider retry number, starting at zero and bounded by the session's retry limit.
    pub retry: u32,
}

/// The next assistant message.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Completion {
    /// Bytes of each opted-in oversized reasoning item, never retained in history.
    pub reasoning_dropped: Box<[u64]>,
    /// Ordered provider-neutral blocks; boxed storage and owned payloads count
    /// against the receiving session's aggregate ownership limits.
    pub content: Box<[Block]>,
    pub stop: Stop,
    pub usage: Usage,
}

/// A native LLM path cap, sent by the protocol in a terminal failure.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cap {
    /// The configured bound on encoded request body.
    Request,
    /// The configured bound on encoded request head.
    RequestHead,
    /// The configured bound on response head.
    ResponseHead,
    /// The configured bound on HTTP header field count.
    ResponseFields,
    /// Scanned wire bytes of one event.
    Skip,
    /// The configured bound on an SSE event type or identifier.
    Field,
    /// The configured bound on retained decoded text.
    Retained,
    /// The configured bound on retained JSON tokens.
    Tokens,
    /// The configured bound on JSON nesting depth.
    Depth,
    /// The configured bound on retained string.
    Strings,
    /// The fixed tokenizer bound on a numeral.
    Number,
    /// The configured bound on provider reasoning replay.
    Reasoning,
    /// The configured bound on completion items.
    OutputItems,
    /// The configured bound on other replay metadata.
    Metadata,
    /// The configured bound on offered tools.
    Tools,
    /// The configured bound on history messages and blocks.
    HistoryItems,
    /// The endpoint model's declared completion ceiling in tokens.
    Output,
    /// The configured bound on completion answer.
    Answer,
    /// The configured bound on provider error body.
    ErrorBody,
    /// The configured bound on access token.
    AccessToken,
    /// The configured bound on account identifier.
    AccountId,
}

/// The owner of a bounded value, sent in a terminal limit failure.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Limit {
    /// The requested reservation exceeds the entire native connection pool.
    Pool { reservation: u64 },
    /// The native provider path refused its named cap.
    Path(Cap),
    /// The session's retained window refused its history bound.
    History,
    /// The usable model window could not make its first ordinary completion.
    Window,
    /// One received tool call refused its argument bound.
    Call,
    /// One injected message refused its receiving bound.
    Message,
}

/// Why the LLM stopped.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Stop {
    /// It finished its turn.
    EndTurn,
    /// It wants the tool calls in its message run, and their results back.
    ToolUse,
    /// It ran out of its token budget mid-answer.
    MaxTokens,
    /// It declined to answer.
    Refusal,
}

/// Raw provider counts (domain/session.md, section 6); absence stays distinct
/// from reported zero in facts and concrete history. Reasoning is within output.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
}

impl Usage {
    /// Exact empty prefix before any completion has been accepted.
    pub const ZERO: Usage = Usage {
        input_tokens: Some(0),
        output_tokens: Some(0),
        cache_read_tokens: Some(0),
        cache_write_tokens: Some(0),
        reasoning_tokens: Some(0),
    };

    /// No counts were reported by the provider.
    pub const NONE: Usage = Usage {
        input_tokens: None,
        output_tokens: None,
        cache_read_tokens: None,
        cache_write_tokens: None,
        reasoning_tokens: None,
    };
}

/// What the transport terminal proves about a provider operation.
/// It is supplied by the shared client, never inferred from failure wording.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Evidence {
    /// The lower proves no request bytes were sent.
    Unsent,

    /// The operation may have reached the peer; no response proves its outcome.
    Unknown,

    /// The peer response was received, including a refused response.
    Response { status: Option<u16> },
}

/// The protocol's elapsed call phase, carried losslessly in timeout terminals
/// (protocol/llm.md, sections 6 and 10).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Phase {
    /// The connection's socket admission deadline.
    Connect,
    /// The connection's TLS handshake deadline.
    Handshake,
    /// The completion's first response-byte deadline.
    Head,
    /// The completion's deadline between response events.
    Idle,
    /// The completion's absolute whole-call deadline.
    Whole,
}

/// Bounded provider evidence, kept outside history and content-free facts (domain/session.md, section 4).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Report {
    pub code: Box<[u8]>,
    pub request: Box<[u8]>,
    pub field: Box<[u8]>,
}

impl Report {
    /// Three provider fields, each at the vocabulary's 128-byte bound (protocol/limits.md, 2.4).
    pub const BYTES: u64 = 3 * 128;

    /// No provider evidence was supplied.
    #[must_use]
    pub fn none() -> Self {
        Self { code: Box::default(), request: Box::default(), field: Box::default() }
    }
}

/// Why a call produced no message, as the protocol layer classifies the
/// provider's answer, or the lack of one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Failure {
    /// The shared client's bounded receiving allowance was exceeded.
    /// This terminal is nonretryable; it is distinct from provider rejection.
    Limit { which: Limit, bound: u64 },

    /// Application translation refused its existing aggregate contract before native admission.
    ApplicationLimit,

    /// The peer response violated the shared protocol contract, nonretryably.
    Protocol,

    /// An unsolicited lower cancellation, not acknowledgement of a
    /// requested Cancel. This failure is nonretryable and keeps transport evidence.
    Cancelled,

    /// The provider is overloaded (HTTP 529, 503). Transient.
    Overloaded,
    /// The provider limits our rate (HTTP 429) and asks us to wait
    /// `retry_after`, zero if it did not say. Transient.
    RateLimited {
        /// Optional provider cooldown, retained in the representation of this boundary.
        retry_after: Duration,
    },
    /// The provider could not be reached, or failed (a connection error,
    /// HTTP 500). Transient.
    Unavailable,
    /// A phase deadline ended the call; only the whole phase ends the session's time without retry.
    TimedOut { phase: Phase },
    /// The conversation does not fit the model's context window.
    ContextTooLong,
    /// The provider rejected the call as malformed or unsupported (HTTP 400,
    /// 404, 422).
    Invalid,
    /// The provider refused our credentials (HTTP 401, 403).
    Unauthorized,
    /// The program source is signed out; a new grant cannot repair it.
    SignedOut,
    /// The backend program failed, with its exit code when known.
    Program { exit: Option<u32> },
    /// The program cannot resume this window; the session carries it over once.
    SessionGone,
    /// The provider refused the account; this call is never retried.
    Refused,
    /// The provider will not take a request field; this call is never retried.
    Unsupported,
    /// The account has spent its allowance; this call is never retried.
    Exhausted {
        /// Optional provider cooldown, retained in the representation of this boundary.
        retry_after: Option<Duration>,
    },
}
