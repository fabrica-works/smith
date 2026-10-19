//! The first cause of a run's unanswering end (protocol/events.md, section 3.10;
//! shell.md, section 4.3). Local policy keeps the cause through settlement;
//! process exit evidence is not known until the host kit supplies it.

use crate::{ExitStatus, StoreFailure};

/// The last turn told in this activation, independently of its durable save.
#[derive(Debug)]
pub(crate) struct Told {
    pub(crate) turns: u32,
    pub(crate) spent: u64,
    pub(crate) read: Option<skein_lib::Token>,
}

impl Told {
    pub(crate) const fn new() -> Self {
        Self { turns: 0, spent: 0, read: None }
    }
}

/// The first cause recorded by local policy; settlement determines when the host exits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EndCause {
    /// The person asked to drop or the inline cancellation grace expired.
    Cancelled,
    /// The store failed while the run went on; its turn is absent for a non-turn write.
    Host { turn: Option<u32>, reason: StoreFailure },
    /// The child ended without answering; Gone supplies any process exit the kit observed.
    Agent { ended: Option<smith_events::AgentExit> },
}

pub(crate) const fn exit_status(cause: Option<EndCause>) -> ExitStatus {
    match cause {
        Some(EndCause::Cancelled) => ExitStatus::Interrupted,
        Some(EndCause::Host { .. } | EndCause::Agent { .. }) | None => ExitStatus::RunFailed,
    }
}

/// Classify the actual answer before rendering moves its concrete result.
pub(crate) fn answer_status(answer: &smith_host_domain::Answer, account: Option<u32>) -> ExitStatus {
    use smith_host_domain::{Refusal, RunFailure, RunResult};
    match &answer.result {
        RunResult::Accepted { .. } => ExitStatus::Accepted,
        RunResult::Parked => ExitStatus::NeedsInput,
        RunResult::Refused { refusal } => match refusal {
            Refusal::Busy => ExitStatus::RunFailed,
            Refusal::Invalid(_) => ExitStatus::Refused,
        },
        RunResult::Failed { failure } => match failure {
            RunFailure::Budget { exhausted: _, .. } => ExitStatus::Budget,
            RunFailure::Cancelled => ExitStatus::Interrupted,
            RunFailure::Model(fault) => match fault {
                smith_host_domain::ModelFault::Completion {
                    failure: smith_host_domain::CompletionFailure::Unauthorized,
                    ..
                } => match account {
                    Some(account) => ExitStatus::Credentials { account },
                    None => ExitStatus::RunFailed,
                },
                smith_host_domain::ModelFault::Refused { account, .. }
                | smith_host_domain::ModelFault::Unauthorized { account } => {
                    ExitStatus::Credentials { account: *account }
                }
                smith_host_domain::ModelFault::Completion { .. }
                | smith_host_domain::ModelFault::Limited { .. }
                | smith_host_domain::ModelFault::Provider
                | smith_host_domain::ModelFault::Declined
                | smith_host_domain::ModelFault::Program { .. }
                | smith_host_domain::ModelFault::Truncated
                | smith_host_domain::ModelFault::Malformed => ExitStatus::RunFailed,
            },
            RunFailure::Transcript(_) | RunFailure::Policy(_) | RunFailure::Stale => ExitStatus::RunFailed,
        },
    }
}
