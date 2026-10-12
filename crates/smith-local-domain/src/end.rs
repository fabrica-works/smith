//! The first cause of a run's unanswering end (protocol/events.md, section 3.10;
//! shell.md, section 4.3). Local policy keeps the cause through settlement;
//! process exit evidence is not known until the host kit supplies it.

use crate::{ExitStatus, StoreFailure};

/// The first cause recorded by local policy; settlement determines when the host exits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EndCause {
    /// The person asked to drop or the inline cancellation grace expired.
    Cancelled,
    /// The store failed while the run went on; its turn is absent for a non-turn write.
    Host { turn: Option<u32>, reason: StoreFailure },
    /// The child ended without an answer; the kit's exit evidence is added by teardown.
    Agent,
}

pub(crate) const fn exit_status(cause: Option<EndCause>) -> ExitStatus {
    match cause {
        Some(EndCause::Cancelled) => ExitStatus::Interrupted,
        Some(EndCause::Host { .. } | EndCause::Agent) | None => ExitStatus::Failed,
    }
}
