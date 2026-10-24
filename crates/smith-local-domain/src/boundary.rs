//! The local host's typed terminal, store, credential and agent boundary
//! (domain/host.md, sections 2 and 7–9). The store owns encoding; this domain
//! keeps only records. Every `Load`, `SaveState`, `SaveTurn` and `Credential` request
//! receives one terminal, including after a cancellation.

use alloc::boxed::Box;
use skein_lib::{Token, Wall};
use smith_domain::{self as agent, run};

/// One writable directory recorded before a delivery can commit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IntentDirectory {
    /// Position in workspace order.
    pub directory: u32,
    /// Whether the checked tree differs from its starting state.
    pub changed: bool,
    /// Git head before delivery; absent for a plain directory.
    pub head: Option<Box<[u8]>>,
    /// Remote target selected before this delivery began, if configured.
    pub push: Option<crate::PushTarget>,
}

/// The durable pre-effect record for a delivery.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeliveryIntent {
    /// All writable directories in workspace order.
    pub directories: Box<[IntentDirectory]>,
}

/// An intent awaiting reconciliation, or its final answer.
#[derive(Clone, PartialEq, Eq, Debug)]
#[expect(clippy::large_enum_variant, reason = "the sealed delivery diagnostic is held inline in the durable record")]
pub enum DeliveryState {
    /// The effect may have begun, and must be found before the next run.
    Intent(DeliveryIntent),
    /// The answer to replay for this call name.
    Answer(run::Delivery),
}

/// One durable intent or decision for an agent delivery call name.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeliveryRecord {
    /// Transcript-derived name decided once by this host.
    pub name: run::CallName,
    /// Pre-effect intent or terminal to replay when the agent asks again.
    pub state: DeliveryState,
    /// Effects found or observed before the saved answer, in workspace order.
    pub landed: Box<[run::Receipt]>,
}

impl DeliveryRecord {
    /// Return the first saved terminal only for its original call name.
    #[must_use]
    pub fn answer(&self, name: run::CallName) -> Option<run::Delivery> {
        if self.name != name {
            return None;
        }
        match &self.state {
            DeliveryState::Intent(_) => None,
            DeliveryState::Answer(delivery) => Some(delivery.clone()),
        }
    }
}

/// A typed operation on one configured git working tree.
#[derive(Debug)]
pub enum GitOp {
    /// Read the current local head before the run starts.
    Head,
    /// Discover changes and merge-conflicted paths.
    Status,
    /// Inspect the current head and whether its commit carries this delivery's name.
    Inspect { name: run::CallName },
    /// Check original conflict paths for markers still present.
    Markers { paths: Box<[Box<[u8]>]> },
    /// Commit the checked tree with the configured result message.
    Commit { message: Box<[u8]> },
    /// Conditionally push the new commit to the configured branch.
    Push { remote: Box<[u8]>, branch: Box<[u8]> },
}

/// One terminal for a typed git operation.
#[derive(Debug)]
pub enum GitResult {
    /// Current local head before the run starts.
    Head { head: Box<[u8]> },
    /// Current working-tree state and any original merge conflicts.
    Status { changed: bool, merging: Option<Box<[Box<[u8]>]>>, head: Box<[u8]> },
    /// Current head and whether its commit message has the call-name trailer.
    Inspected { head: Box<[u8]>, named: bool },
    /// First original conflict file still holding a marker, if any.
    Markers { first: Option<Box<[u8]>> },
    /// The new commit identity to show the agent and person.
    Committed { receipt: Box<[u8]>, head: Box<[u8]> },
    /// The configured branch now names the commit.
    Pushed,
    /// The remote branch moved since this checkout began.
    Stale,
    /// A write made no commit; the host may fail with this diagnostic.
    NoEffect { reason: run::DeliveryReason, diagnostic: Box<run::Diagnostic> },
    /// A write may have landed; the host must inspect its saved delivery name.
    Uncertain { reason: run::DeliveryReason, diagnostic: Box<run::Diagnostic> },
    /// A read failed with a bounded diagnostic tail.
    Failed { reason: run::DeliveryReason, diagnostic: Box<run::Diagnostic> },
}

/// Chat metadata kept beside durable turns.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChatState {
    pub identity: run::Identity,
    /// Last activation saved before it started.
    pub activation: u64,
    /// Last person message name saved before it was sent.
    pub next_message: u64,
    /// Last person message read in a told turn.
    pub read: Option<Token>,
}

/// The stored family whose layout the Store refused before loading records.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StoreFamily {
    /// Metadata, turn envelopes, deliveries and the marker, checked first.
    Store,
    /// Local transcript payloads, checked after the store family.
    Transcript,
}

/// The Store's older-build refusal, carried to the operator and exit 1.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Older {
    /// A nonempty directory whose layouts were never recorded.
    Unrecorded,
    /// A malformed marker or a recorded family layout different from this build.
    Layout(StoreFamily),
}

impl Older {
    /// Static operator pieces, with the chat name inserted between them (shell.md, 5.3).
    #[must_use]
    pub fn words(&self) -> &'static [&'static str] {
        match self {
            Self::Unrecorded => &[
                "Chat `",
                "` was kept by an older build of smith, from before chats recorded their layouts, and this start has changed nothing in its directory. Before the first release a chat is not translated, so every start refuses this one until it is moved. To start afresh, stop each smith using it, move its directory under smith's state directory aside, whole, and start again: nothing is deleted, and the next start makes a new, empty chat, without this chat's history or its deliveries' answers.",
            ],
            Self::Layout(StoreFamily::Store) => &[
                "Chat `",
                "` was kept by an older build of smith, whose records' layout differs from this build's, and this start has changed nothing in its directory. Before the first release a chat is not translated, so every start refuses this one until it is moved. To start afresh, stop each smith using it, move its directory under smith's state directory aside, whole, and start again: nothing is deleted, and the next start makes a new, empty chat, without this chat's history or its deliveries' answers.",
            ],
            Self::Layout(StoreFamily::Transcript) => &[
                "Chat `",
                "` was kept by an older build of smith, whose turns' layout differs from this build's, and this start has changed nothing in its directory. Before the first release a chat is not translated, so every start refuses this one until it is moved. To start afresh, stop each smith using it, move its directory under smith's state directory aside, whole, and start again: nothing is deleted, and the next start makes a new, empty chat, without this chat's history or its deliveries' answers.",
            ],
        }
    }
}

/// Why the store could not keep or load this chat.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StoreFailure {
    /// This build cannot use the chat layouts; the Store sends it before mutation.
    OlderBuild { why: Older },
    /// The record could not be read.
    Read,
    /// The record could not be made durable.
    Write,
}

/// Why the configured account has no usable grant (domain/host.md, section 8;
/// shell.md, section 4.3); sent by the service to a failed start or account notice.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CredentialFailure {
    /// The account has not been signed in.
    Missing,
    /// The account could not be refreshed.
    Refresh,
    /// A borrowed access token is approaching expiry.
    BorrowedExpiring { at: Wall },
    /// The program's status says this account is signed out; no run starts.
    ProgramNotSignedIn,
    /// The program names another payment method; no run starts.
    ProgramSignedInOtherwise { method: smith_host_domain::ProgramMethod },
    /// The program check failed with bounded stderr detail; no run starts.
    ProgramFailed(smith_host_domain::ProgramFailure),
    /// The person interrupted the program check; its request ends without a run.
    ProgramInterrupted,
}

/// How this invocation ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExitStatus {
    /// The run accepted its result, or the person left an idle chat.
    Accepted,
    /// The run or a required host operation failed.
    RunFailed,
    /// The run exhausted its admitted allowance.
    Budget,
    /// This account needs a usable credential before work can continue.
    Credentials { account: u32 },
    /// The run's settings or start were refused as invalid.
    Refused,
    /// The headless run parked because it needed the person's input.
    NeedsInput,
    /// The run was dropped before answering at the person's ask or after cancellation grace.
    Interrupted,
}

/// One event from the person or a typed lower boundary.
#[derive(Debug)]
pub enum Event {
    /// Person text, bounded by `Limits::line_bytes` before retention.
    Line { text: Box<[u8]> },
    /// A bounded local command; it never becomes a run message.
    Slash(crate::Slash),
    /// The person asks to cancel a live run or leave an idle chat.
    Interrupt,
    /// A second interrupt asks the selected child to drop now; `Gone` follows settlement.
    Drop,
    /// The terminal closed; finish outstanding work, then leave.
    Closed,
    /// Select another bounded chat between activations; `ChatSelected` follows.
    SelectChat { name: Box<[u8]>, create: bool },
    /// The idle store selected a chat, or refused it without changing its selection.
    ChatSelected { name: Box<[u8]>, result: Result<(), ChatRefusal> },
    /// Terminal for Load, including a possibly empty saved chat.
    Loaded { state: Option<ChatState>, transcript: Option<agent::Transcript>, deliveries: Box<[DeliveryRecord]> },
    /// Terminal for `SaveState`; names and activation are now durable.
    StateSaved,
    /// Terminal for `SaveTurn`; the named turn is now durable.
    TurnSaved { number: u32 },
    /// Terminal for a saved delivery decision.
    DeliverySaved { name: run::CallName },
    /// Terminal for one workspace git operation.
    Git { owner: Token, result: GitResult },
    /// Terminal for a plain directory's changed-state query.
    PlainStatus { owner: Token, changed: bool },
    /// Terminal for any outstanding store request that failed.
    StoreFailed {
        /// The transcript sequence whose `SaveTurn` write failed; other store operations carry None.
        turn: Option<u32>,
        reason: StoreFailure,
    },
    /// Terminal for one Credential request.
    Credential { grant: agent::Grant, form: smith_host_domain::Form },
    /// Terminal for one Credential request that could not be served.
    NoCredential { account: u32, reason: CredentialFailure },
    /// One terminal below an inline run, routed under its agent handle.
    Below { agent: Token, terminal: smith_inline_agent::Below },
    /// One process terminal from the adapter, consumed by the local host child.
    Process(smith_host_domain::process::Event),
}

/// Why the person's request to select a chat could not be served.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatRefusal {
    /// The current chat still owns activation, line or store work.
    Busy,
    /// The chat is not a bounded single directory name.
    Name,
    /// No private directory with that name exists.
    Missing,
    /// A private ephemeral store cannot select another chat.
    Ephemeral,
    /// The directory operation failed.
    Io,
}

/// Whether the terminal may demand more person input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reading {
    /// There is room to receive another line.
    Open,
    /// Retained lines must be read before input resumes.
    Held,
}

/// A local host notice rendered by the terminal (protocol/hosts.md, section 5.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Notice {
    /// A local chat selection was refused, without sending a message to the run.
    ChatRefused { name: Box<[u8]>, reason: ChatRefusal },
    /// The run is waiting for a person message.
    Waiting,
    /// The run's checks began or ended.
    Checking(Checks),
    /// A person line could not reach the run.
    NotDelivered { line: Option<Token>, why: Undelivered },
    /// A local host operation failed.
    Failure(Failure),
    /// The kit forced an answered agent to exit; its answer still stands.
    Warning(Warning),
}

/// The run reports a checking span to the person.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Checks {
    /// The checks began.
    Started,
    /// The checks ended.
    Ended,
}

/// Why a person line did not reach the run.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Undelivered {
    /// The line exceeds the terminal bound and has no name.
    TooLong,
    /// The run ended without reading this line.
    Dropped,
    /// The kit or run refused the named line at admission.
    Refused(smith_host_domain::MessageRefusal),
}

/// A host failure sent to the terminal for the person.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Failure {
    /// The chat's increasing names ran out.
    NamesExhausted,
    /// A configured account could not serve the run.
    Account(AccountFailure),
    /// A repository head could not be read.
    HeadUnavailable { directory: Box<[u8]> },
    /// The answered run failed with its last diagnostic, for the person and operator.
    Run { failure: smith_host_domain::RunFailure, detail: Box<[u8]> },
    /// The run refused its start, naming the refusal.
    Refused(smith_host_domain::Refusal),
    /// The kit refused before an agent existed.
    Unstarted(smith_host_domain::End),
    /// The agent failed before answering, carrying its settled exit and diagnostic.
    Agent { failure: AgentFailure, end: smith_host_domain::End, detail: Box<[u8]> },
    /// This account needs a credential from the person.
    Credential { account: u32, reason: CredentialFailure },
    /// The chat could not be kept; a turn save names its turn.
    Store { turn: Option<u32>, reason: StoreFailure },
    /// A committed delivery's answer could not be kept.
    DeliveryUnsaved,
}

/// An account's terminal failure shown to the person.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AccountFailure {
    /// No usable account is available.
    Unavailable,
    /// The account could not refresh.
    NotRefreshed,
    /// The account reached its usage limit, with a reset only when supplied.
    Limited { account: u32, reset_at: Option<Wall> },
    /// The provider refused the account, keeping its diagnostic.
    Refused { account: u32, code: Box<[u8]>, reason: Box<[u8]> },
}

/// Why the child failed before it could answer (domain/host.md, section 4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AgentFailure {
    /// The process or its opening could not start.
    NotStarted,
    /// The kit observed this process or channel failure.
    Fault(smith_host_domain::Fault),
}

/// An operator warning that leaves the run's answer unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Warning {
    /// The kit sent a signal after the answer; its settled exit is retained.
    ForcedExit { exit: Option<smith_host_domain::Exit>, forced: smith_host_domain::Forced },
}

/// One request to the person, store, credential source or agent IO.
#[derive(Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "typed process payloads are owned directly and priced by the request queue"
)]
pub enum Request {
    /// Open or hold the terminal input demand.
    Input { reading: Reading },
    /// Bounded text shown to the person.
    Show { text: Box<[u8]> },
    /// A typed local host notice for the terminal to render.
    Notice { notice: Notice },
    /// Load this configured chat's metadata and concrete history.
    Load,
    /// Select one idle chat directory; answered by `ChatSelected`.
    SelectChat { name: Box<[u8]>, create: bool },
    /// Save the updated activation or person-message counter before using it.
    /// A fresh start replaces incompatible old history in the same store operation.
    SaveState { state: ChatState, fresh: bool },
    /// Append one numbered concrete turn, its read fence and window flag atomically; answered by `TurnSaved` or `StoreFailed`.
    SaveTurn { number: u32, read: Option<Token>, opens_window: bool, turn: agent::Turn },
    /// Save a pre-effect intent or terminal; the child hears only the terminal.
    SaveDelivery { record: Box<DeliveryRecord> },
    /// Operate on one git directory within the delivery deadline.
    Git { owner: Token, directory: u32, op: GitOp, deadline: skein_lib::Time },
    /// Compare a plain directory with its start snapshot.
    PlainStatus { owner: Token, directory: u32, deadline: skein_lib::Time },
    /// Fetch or refresh one account; answered by `Credential` or `NoCredential`.
    Credential { account: u32 },
    /// One operation below an inline run; its terminal returns as Below.
    Lower { agent: Token, request: smith_inline_agent::Lower },
    /// One process effect for the adapter; its terminal returns as Process.
    Process(smith_host_domain::process::Request),
    /// The shown answer moves to the service for its run terminal; owes no terminal.
    Answered { answer: smith_host_domain::Answer },
    /// The agent went without answering; the service writes its run terminal. Owes no terminal.
    Unanswered { cause: crate::EndCause, turns: u32, spent: u64, read: Option<Token> },
    /// The caller may terminate this invocation.
    Exit { status: ExitStatus },
}
