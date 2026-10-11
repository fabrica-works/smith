//! Owned event values supplied by the service, without domain dependencies.
//! Contract: protocol/events.md, sections 3 and 6.

use alloc::boxed::Box;
use skein_lib::List;

/// `Capture` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Capture {
    /// The `none` classification.
    None,
    /// The `calls` classification.
    Calls,
    /// The `everything` classification.
    Everything,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Delivery` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Delivery {
    /// The `complete` classification.
    Complete,
    /// The `best_effort` classification.
    BestEffort,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Mode` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Mode {
    /// The `agent` classification.
    Agent,
    /// The `exec` classification.
    Exec,
    /// The `chat` classification.
    Chat,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// The machine containment that the shell records at startup (protocol/agent.md, section 3).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Containment {
    /// Commands lead process groups and remain uncontained.
    Groups,
    /// Commands belong to contained process trees.
    Trees,
    /// A value this reader does not know, retained as bounded UTF-8.
    Unknown(Box<[u8]>),
}

/// `Family` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Family {
    /// The `inspect` classification.
    Inspect,
    /// The `modify` classification.
    Modify,
    /// The `shell` classification.
    Shell,
    /// The `sub_agents` classification.
    SubAgents,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Form` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Form {
    /// The `report` classification.
    Report,
    /// The `verdict` classification.
    Verdict,
    /// The `change` classification.
    Change,
    /// The `failure` classification.
    Failure,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Status` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Status {
    /// The `accepted` classification.
    Accepted,
    /// The `parked` classification.
    Parked,
    /// The `failed` classification.
    Failed,
    /// The `refused` classification.
    Refused,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `ConversationKind` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ConversationKind {
    /// The `main` classification.
    Main,
    /// The `child` classification.
    Child,
    /// The `compaction` classification.
    Compaction,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `ConversationEnd` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ConversationEnd {
    /// The `closed` classification.
    Closed,
    /// The `budget` classification.
    Budget,
    /// The `failed` classification.
    Failed,
    /// The `refused` classification.
    Refused,
    /// The `transcript` classification.
    Transcript,
    /// The `overflow` classification.
    Overflow,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// Provider tool-selection metadata the stream producer sends (protocol/events.md, section 3.5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ToolChoice {
    /// The provider may select tools automatically.
    Auto,
    /// The provider is asked to answer without tools.
    None,
    /// The provider may choose only among the specified tools.
    Only,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Outcome` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Outcome {
    /// The `completed` classification.
    Completed,
    /// The `failed` classification.
    Failed,
    /// The `cancelled` classification.
    Cancelled,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Stop` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Stop {
    /// The `end` classification.
    End,
    /// The `tools` classification.
    Tools,
    /// The `max_tokens` classification.
    MaxTokens,
    /// The `refusal` classification.
    Refusal,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Source` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Source {
    /// The `workspace` classification.
    Workspace,
    /// The `run` classification.
    Run,
    /// The `host` classification.
    Host,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Effect` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// The `read` classification.
    Read,
    /// The `write` classification.
    Write,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Verdict` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Verdict {
    /// The `read` classification.
    Read,
    /// The `listed` classification.
    Listed,
    /// The `found` classification.
    Found,
    /// The `written` classification.
    Written,
    /// The `edited` classification.
    Edited,
    /// The `exited` classification.
    Exited,
    /// The `conflict` classification.
    Conflict,
    /// The `missing` classification.
    Missing,
    /// The `too_large` classification.
    TooLarge,
    /// The `timed_out` classification.
    TimedOut,
    /// The `failed` classification.
    Failed,
    /// The `cancelled` classification.
    Cancelled,
    /// The `ambiguous` classification.
    Ambiguous,
    /// The `result` classification.
    Result,
    /// The `error` classification.
    Error,
    /// The `invalid` classification.
    Invalid,
    /// The `not_run` classification.
    NotRun,
    /// The `withdrawn` classification.
    Withdrawn,
    /// The `not_granted` classification.
    NotGranted,
    /// The `outside` classification.
    Outside,
    /// The `read_only` classification.
    ReadOnly,
    /// The `too_long` classification.
    TooLong,
    /// The `not_found` classification.
    NotFound,
    /// The `not_file` classification.
    NotFile,
    /// The `linked` classification.
    Linked,
    /// The `protected` classification.
    Protected,
    /// The `not_directory` classification.
    NotDirectory,
    /// The `not_read` classification.
    NotRead,
    /// The `stale` classification.
    Stale,
    /// The `no_match` classification.
    NoMatch,
    /// The `unchanged` classification.
    Unchanged,
    /// The `busy` classification.
    Busy,
    /// The `nul_byte` classification.
    NulByte,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Delivered` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Delivered {
    /// The `delivered` classification.
    Delivered,
    /// The `nothing` classification.
    Nothing,
    /// The `refused` classification.
    Refused,
    /// The `failed` classification.
    Failed,
    /// The `stale` classification.
    Stale,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `FailureClass` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum FailureClass {
    /// The `overloaded` classification.
    Overloaded,
    /// The `rate_limited` classification.
    RateLimited,
    /// The `exhausted` classification.
    Exhausted,
    /// The `unavailable` classification.
    Unavailable,
    /// The `timed_out` classification.
    TimedOut,
    /// The `context_too_long` classification.
    ContextTooLong,
    /// The `invalid` classification.
    Invalid,
    /// The `unauthorized` classification.
    Unauthorized,
    /// The `limit` classification.
    Limit,
    /// The `protocol` classification.
    Protocol,
    /// The `cancelled` classification.
    Cancelled,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Evidence` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Evidence {
    /// The `unsent` classification.
    Unsent,
    /// The `maybe_sent` classification.
    MaybeSent,
    /// The `response` classification.
    Response,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `AnswerClass` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum AnswerClass {
    /// The `model` classification.
    Model,
    /// The `budget` classification.
    Budget,
    /// The `policy` classification.
    Policy,
    /// The `cancelled` classification.
    Cancelled,
    /// The `stale` classification.
    Stale,
    /// The `transcript` classification.
    Transcript,
    /// The `busy` classification.
    Busy,
    /// The `invalid` classification.
    Invalid,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Level` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Level {
    /// The `info` classification.
    Info,
    /// The `warning` classification.
    Warning,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `NoticeKind` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum NoticeKind {
    /// The service reports one reasoning item omitted by the model's opt-in.
    ReasoningDropped,
    /// The `credential_rejected` classification.
    CredentialRejected,
    /// The `account_exhausted` classification.
    AccountExhausted,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// `Role` classification sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Role {
    /// The `user` classification.
    User,
    /// The `assistant` classification.
    Assistant,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// The build identity the shell supplies at process startup.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Agent {
    pub name: Box<[u8]>,
    pub version: Box<[u8]>,
    pub build: Box<[u8]>,
}

/// The format versions the shell announces at process startup.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Versions {
    pub events: u64,
    pub channel: u64,
    pub charter: u64,
    pub transcript: u64,
}

/// One per-type emission count the event sink reports at process end.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Count {
    pub record: Box<[u8]>,
    pub count: u64,
}

/// The event sink and channel loss counts the service reports at process end.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Loss {
    pub events: u64,
    pub channel: Option<u64>,
}

/// Own and reaped-child CPU milliseconds measured by the shell at process end.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Cpu {
    pub user: u64,
    pub system: u64,
    pub children_user: u64,
    pub children_system: u64,
}

/// Own and reaped-child peak resident bytes measured by the shell at process end.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PeakRss {
    pub self_bytes: u64,
    pub children: u64,
}

/// The endpoint, model and charter window/output allowances the service records for a conversation.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Model {
    pub endpoint: Box<[u8]>,
    pub model: Box<[u8]>,
    pub window_tokens: u64,
    pub output_tokens: u64,
    pub effort: Option<Box<[u8]>>,
}

/// Provider-reported token counts the service records, with unknown counts kept nullable.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
}

/// The run allowances the service records at admission.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Budget {
    pub turns: u64,
    pub spend: u64,
    pub time_ms: u64,
    pub reserve: Reserve,
}

/// Optional child allowances requested by the model, sent in opening metadata.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Requested {
    pub turns: Option<u64>,
    pub seconds: Option<u64>,
}

/// A child's effective allowance, sent by the run's event producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Share {
    pub turns: u64,
    /// Exact effective time in milliseconds.
    pub time_ms: u64,
    pub spend: u64,
}

/// One child allowance lowered by the run, sent in opening metadata.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Clamp {
    pub part: SharePart,
    pub by: ShareBound,
}

/// A child's allowance part named by an event producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum SharePart {
    /// Its completion allowance.
    Turns,
    /// Its wall-clock allowance.
    Time,
    /// A future part retained by this reader.
    Unknown(Box<[u8]>),
}

/// The bound that lowered a child's allowance, sent by an event producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ShareBound {
    /// The parent's ordinary pool above its reserve.
    Reserve,
    /// The asker's call deadline.
    Deadline,
    /// A future bound retained by this reader.
    Unknown(Box<[u8]>),
}

/// The run's budget stage named by its event producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BudgetStage {
    /// Work draws on the ordinary pool.
    Ordinary,
    /// Main finishes on the reserved pool.
    WindDown,
    /// A future stage retained by this reader.
    Unknown(Box<[u8]>),
}

/// A budget part's counted usage and ceiling, sent in a budget snapshot.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Quantity {
    pub used: u64,
    pub limit: u64,
}

/// The finishing allowances held by the run, sent in its budget snapshot.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Reserve {
    pub turns: u64,
    pub time_ms: u64,
    pub spend: u64,
}

/// A cumulative run budget observation, sent once at each actual charge.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BudgetStanding {
    pub run: u64,
    pub stage: BudgetStage,
    pub turns: Quantity,
    pub spend: Quantity,
    pub time_ms: Quantity,
    pub reserve: Reserve,
}

/// The tool families and host tool names the service records at admission.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Tools {
    pub families: List<Family>,
    pub wait: bool,
    pub deliver: bool,
    pub host: List<Box<[u8]>>,
}

/// The provider failure classification and transport evidence the service records.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CompletionFailure {
    pub class: FailureClass,
    pub evidence: Evidence,
    pub retry_after_ms: Option<u64>,
}

/// The typed failed or refused answer the service records at run completion.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AnswerFailure {
    pub class: AnswerClass,
    pub completion: Option<CompletionFailure>,
    pub account: Option<u64>,
    pub which: Option<Box<[u8]>>,
    pub reason: Option<Box<[u8]>>,
}

/// One named result field the service records under full content capture.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Field {
    pub name: Box<[u8]>,
    pub text: Box<[u8]>,
}

/// The accepted result form and optional content the service records at run completion.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RunResult {
    pub form: Form,
    pub label: Option<Box<[u8]>>,
    pub text: Option<Box<[u8]>>,
    pub fields: Option<List<Field>>,
}

/// One prompt message the service records with ordered content blocks.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Message {
    pub role: Role,
    pub blocks: List<Block>,
}

/// The new part of a provider request the service records under full capture.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Prompt {
    pub system: Option<Box<[u8]>>,
    pub tools: Option<List<Box<[u8]>>>,
    pub messages: List<Message>,
}

/// The provider answer blocks the service records under full capture.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Completion {
    pub blocks: List<Block>,
}

/// A command's code or signal, sent by the service with its tool terminal.
/// Contract: protocol/events.md, section 3.6.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CommandExit {
    /// The leader exited with this code, recorded by the service.
    Code { code: i64 },
    /// This signal ended the leader, recorded by the service.
    Signal { signal: u64 },
}

/// A workspace check's terminal, sent by the service after its start.
/// Contract: protocol/events.md, sections 3.6 and 6.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CheckExit {
    /// The checks exited with this code; only zero passes.
    Code { code: i64 },
    /// The machine reported this signal ending the checks.
    Signal { signal: u64 },
    /// The checks' deadline passed and their tree was stopped.
    TimedOut,
    /// The checks could not start; their spawn detail is an operator notice.
    Unstarted,
    /// The run withdrew the checks and stopped their tree before they ended.
    Cancelled,
    /// An unrecognized kind retained by a reader, never an inferred terminal.
    Unknown(Box<[u8]>),
}

/// One effective bound sent in the startup header; the key names its layer and unit.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EffectiveLimit {
    pub name: Box<[u8]>,
    pub value: LimitValue,
}

/// An exact numeric effective value, supplied by startup and retained by the reader.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LimitValue {
    /// An integer in the named limit's unit, sent by startup without floating conversion.
    Integer(u64),
    /// A policy share, sent by startup as an exact pair with a nonzero denominator.
    Fraction { numerator: u64, denominator: u64 },
}

/// The shell startup record that opens one process stream.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SessionStarted {
    pub wall_ms: u64,
    pub agent: Agent,
    pub mode: Mode,
    pub containment: Containment,
    pub pid: u64,
    pub profile: Box<[u8]>,
    pub capture: Capture,
    pub delivery: Delivery,
    pub versions: Versions,
    /// Named effective values, ordered by the service; names are unique.
    pub limits: List<EffectiveLimit>,
}

/// The shell exit and resource record that terminates one process stream.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SessionEnded {
    pub exit: i64,
    pub answered: bool,
    pub teardown_ms: Option<u64>,
    pub emitted: List<Count>,
    pub loss: Loss,
    pub cpu_ms: Option<Cpu>,
    pub peak_rss_bytes: Option<PeakRss>,
}

/// The admitted run record the service sends before its conversation records.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RunStarted {
    pub run: u64,
    pub resumed: bool,
    pub messages: u64,
    pub main: Model,
    pub budget: Budget,
    pub tools: Tools,
    pub contract: List<Form>,
}

/// The authoritative answer record the service sends once to terminate a run.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RunCompleted {
    pub run: u64,
    pub wind_down: bool,
    pub status: Status,
    pub failure: Option<AnswerFailure>,
    pub result: Option<RunResult>,
    pub turns: u64,
    pub spent: u64,
    pub usage: Usage,
    /// Final told-turn message fence; None is encoded as null when no message was read.
    pub read: Option<u64>,
    pub unread: u64,
    pub duration_ms: u64,
}

/// The conversation identity and model the service records at its opening.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConversationOpened {
    pub run: u64,
    pub conversation: u64,
    pub kind: ConversationKind,
    pub parent: Option<u64>,
    pub call: Option<Box<[u8]>>,
    /// Affinity ordinal carried by this conversation's requests.
    pub thread: u64,
    pub model: Model,
    pub requested: Option<Requested>,
    pub share: Option<Share>,
    pub clamped: List<Clamp>,
}

/// The conversation terminal and totals the service records at its close.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConversationClosed {
    pub run: u64,
    pub conversation: u64,
    pub end: ConversationEnd,
    pub which: Option<Box<[u8]>>,
    pub failure: Option<CompletionFailure>,
    pub turns: u64,
    pub usage: Usage,
    pub spent: u64,
    pub duration_ms: u64,
}

/// The provider attempt identity and request metadata the service records at its start.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ResponseStarted {
    pub run: u64,
    pub conversation: u64,
    pub response: u64,
    pub turn: u64,
    pub attempt: u64,
    /// Conversation window, from one and incremented after each compaction.
    pub window: u64,
    pub model: Model,
    pub messages: u64,
    pub output_tokens: u64,
    pub tool_choice: ToolChoice,
    pub prompt: Option<Prompt>,
}

/// The provider attempt terminal, charge and measures the service records at its end.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ResponseCompleted {
    pub run: u64,
    pub conversation: u64,
    pub response: u64,
    pub outcome: Outcome,
    pub stop: Option<Stop>,
    pub blocks: u64,
    pub calls: u64,
    pub invalid: u64,
    pub oversized: u64,
    pub cut: u64,
    pub usage: Usage,
    pub spent: u64,
    pub request_bytes: Option<u64>,
    pub first_byte_ms: Option<u64>,
    pub largest_gap_ms: Option<u64>,
    pub duration_ms: u64,
    pub failure: Option<CompletionFailure>,
    pub retry_ms: Option<u64>,
    pub completion: Option<Completion>,
}

/// One live provider text piece the protocol layer sends under full capture.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TextDelta {
    pub run: u64,
    pub conversation: u64,
    pub response: u64,
    pub block: u64,
    pub text: Box<[u8]>,
}

/// The provider call identity, authority and input size the service records at its start.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ToolStarted {
    pub run: u64,
    pub conversation: u64,
    pub call: Box<[u8]>,
    pub tool: Box<[u8]>,
    pub source: Source,
    pub effect: Effect,
    pub deadline_ms: Option<u64>,
    pub input_bytes: u64,
    pub input: Option<Box<[u8]>>,
}

/// The provider call terminal and result size the service records at its end.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ToolCompleted {
    pub run: u64,
    pub conversation: u64,
    pub call: Box<[u8]>,
    pub tool: Box<[u8]>,
    pub verdict: Verdict,
    pub exit: Option<CommandExit>,
    pub bytes: u64,
    /// Bytes the tool's own read window or command capture omitted, before rendering.
    pub omitted_bytes: u64,
    /// Whether the requested budget or deadline was lowered to its ceiling.
    pub clamped: bool,
    pub duration_ms: u64,
    pub child: Option<ChildCompleted>,
    pub delivery: Option<Delivered>,
}

/// The settled child result the service joins to its enclosing tool terminal.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ChildCompleted {
    pub conversation: u64,
    pub stop: ChildStop,
    pub turns: u64,
    /// Exact elapsed child time in milliseconds.
    pub time_ms: u64,
    pub spent: u64,
    pub usage: Usage,
}

/// How a child ended, sent with its enclosing tool terminal.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ChildStop {
    /// The child chose to answer.
    End,
    /// The child's share ended its answer.
    Budget,
    /// The child failed without answering.
    Failed,
    /// The child closed without answering.
    Closed,
    /// A value this reader does not know, retained as UTF-8.
    Unknown(Box<[u8]>),
}

/// The workspace check deadline the service records at its start.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CheckStarted {
    pub run: u64,
    pub deadline_ms: Option<u64>,
}

/// The workspace check exit and duration the service records at its end.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CheckCompleted {
    pub run: u64,
    pub exit: CheckExit,
    pub passed: bool,
    pub duration_ms: u64,
}

/// The message admission or sole refusal terminal the producer records (protocol/events.md, section 3.7).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MessageReceived {
    pub run: u64,
    pub name: u64,
    /// Rendered message size in bytes, retained under every capture policy.
    pub bytes: u64,
    pub refused: Option<MessageRefusal>,
    pub label: Option<Box<[u8]>>,
    pub text: Option<Box<[u8]>>,
}

/// A message admission refusal the producer records as its sole terminal.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MessageRefusal {
    /// The rendered message exceeded the receiving bound.
    TooLarge,
    /// The receiving inbox had no room.
    InboxFull,
    /// The receiving inbox already owned this name.
    NameInUse,
    /// The run had already ended message admission.
    Ending,
    /// The start carrying this message was refused at the entrance.
    StartRefused,
    /// A value this reader does not know, retained as bounded UTF-8.
    Unknown(Box<[u8]>),
}

/// The message read terminal the producer sends for the told turn covering its name.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MessageRead {
    pub run: u64,
    pub name: u64,
    pub turn: u64,
}

/// The message unread terminal the producer sends when the run answers without reading it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MessageUnread {
    pub run: u64,
    pub name: u64,
}

/// An operator notice the service records for an account or a dropped reasoning item.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Notice {
    pub level: Level,
    pub kind: NoticeKind,
    pub run: Option<u64>,
    pub account: Option<u64>,
    pub wait_ms: Option<u64>,
    /// Actual size of the reasoning item omitted from history, in bytes.
    pub bytes: Option<u64>,
}

/// A content block supplied by a provider or a prompt owner.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Block {
    /// A content block kind this reader does not know; its fields are skipped.
    Unknown { kind: Box<[u8]> },
    /// The `text` block.
    Text { text: Box<[u8]> },
    /// The `refusal` block.
    Refusal { text: Box<[u8]> },
    /// The `call` block.
    Call { call: Box<[u8]>, tool: Box<[u8]>, input: Box<[u8]> },
    /// The `oversized` block.
    Oversized { call: Box<[u8]>, tool: Box<[u8]>, bytes: u64 },
    /// The `cut` block.
    Cut { call: Box<[u8]>, tool: Box<[u8]>, input: Box<[u8]> },
    /// The `result` block.
    Result { call: Box<[u8]>, error: bool, text: Box<[u8]> },
    /// The `opaque` block.
    Opaque { bytes: u64 },
}

/// One timestamped event emitted by a service; session end terminates its stream.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Record {
    /// Milliseconds since process start, at emission.
    pub t_ms: u64,
    pub event: Event,
}

/// A record body sent by a stream producer.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Event {
    /// The `budget` record.
    Budget(BudgetStanding),
    /// The `session.started` record.
    SessionStarted(SessionStarted),
    /// The `session.ended` record.
    SessionEnded(SessionEnded),
    /// The `run.started` record.
    RunStarted(RunStarted),
    /// The `run.completed` record.
    RunCompleted(RunCompleted),
    /// The `conversation.opened` record.
    ConversationOpened(ConversationOpened),
    /// The `conversation.closed` record.
    ConversationClosed(ConversationClosed),
    /// The `response.started` record.
    ResponseStarted(ResponseStarted),
    /// The `response.completed` record.
    ResponseCompleted(ResponseCompleted),
    /// The `text.delta` record.
    TextDelta(TextDelta),
    /// The `tool.started` record.
    ToolStarted(ToolStarted),
    /// The `tool.completed` record.
    ToolCompleted(ToolCompleted),
    /// The `check.started` record.
    CheckStarted(CheckStarted),
    /// The `check.completed` record.
    CheckCompleted(CheckCompleted),
    /// The message admission or sole refusal terminal.
    MessageReceived(MessageReceived),
    /// The told-turn message read terminal.
    MessageRead(MessageRead),
    /// The answered-run message unread terminal.
    MessageUnread(MessageUnread),
    /// The `notice` record.
    Notice(Notice),
}
