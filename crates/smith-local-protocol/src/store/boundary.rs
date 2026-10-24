//! Store requests, terminals and bounded record names (protocol/hosts.md, 5.3).
use alloc::boxed::Box;
use skein_io::{file, kernel};
use skein_lib::{Time, Token};
use smith_local_domain::{self as local, ChatState, DeliveryRecord};

/// Bounds on one file, the current window, deliveries and directory scan bytes.
#[derive(Clone, Copy, Debug)]
pub struct StoreLimits {
    /// Must hold the generated Layouts marker as one whole file.
    pub file_bytes: u32,
    pub turns: u32,
    pub deliveries: u32,
    /// Includes one entry cell and the six-byte marker name; absence scans reserve them.
    pub scan_bytes: u64,
}

/// Whether saving metadata keeps the earlier transcript or starts a fresh chat.
#[derive(Clone, Copy, Debug)]
pub enum History {
    /// Keep the existing turn and delivery files.
    Kept,
    /// Journal a fresh chat before removing the earlier records.
    Fresh,
}

/// One local-domain request; the store emits its one domain terminal.
#[derive(Debug)]
pub enum StoreRequest {
    /// Select one safe chat directory while idle; `create` makes a new private directory.
    SelectChat { name: Box<[u8]>, create: bool },
    /// Check the selected root layouts, then recover fresh and load the last window.
    Load,
    /// Durably reserve the chat's next activation or reset its history.
    SaveState { state: ChatState, history: History },
    /// Store one conversation turn with `Expect::Absent` and its read fence.
    SaveTurn { number: u32, read: Option<Token>, opens_window: bool, turn: smith_domain::Turn },
    /// Replace a delivery intent or its terminal decision.
    SaveDelivery { record: Box<DeliveryRecord> },
}

/// The bounded file named in an operator-facing store fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordName {
    /// Chat metadata, including the next activation.
    State,
    /// A numbered conversation turn stored only where its file is absent.
    Turn(u32),
    /// One delivery intent or answer under its stable call name.
    Delivery(smith_domain::run::CallName),
    /// The fresh-chat recovery journal.
    Journal,
}

/// Why a record operation failed; the domain receives `StoreFailed` separately.
#[derive(Debug)]
pub enum StoreFault {
    /// The file operation ended with a kernel fault.
    Io { file: RecordName, error: kernel::Error },
    /// Another writer changed the expected version.
    Conflict { file: RecordName },
    /// A record or the scan exceeded its admitted bound.
    TooLarge { file: RecordName },
    /// A retained or supplied record was malformed.
    Malformed { file: RecordName },
    /// The one retry still could not establish durability.
    NotDurable { file: RecordName },
}

/// One file request, domain terminal, diagnostic or the close's terminal.
#[derive(Debug)]
pub enum Action {
    /// IO settles this request before the next store transition.
    Io { request: file::Request, deadline: Time },
    /// Exactly one terminal for an admitted local-domain store request.
    Domain(local::Event),
    /// Content-free detail behind `StoreFailed`.
    Fault(StoreFault),
    /// Cleanup kept the private directory or could not finish its parent close.
    CleanupWarning { error: kernel::Error },
    /// The root descriptor and all operations have closed.
    Closed,
}
