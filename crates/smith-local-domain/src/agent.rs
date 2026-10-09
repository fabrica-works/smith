//! Agent composition and typed parent commands (domain/host.md, sections8
//! and9.4). This module retains the child and its receiving bounds; it knows
//! no process descriptors, credentials, wire bytes or durable store format.
//! Commands and terminals enter the spawned child; its outputs remain ordered
//! for the local policy to consume. The existing root child stays until3.6.

use crate::Invalid;
use alloc::boxed::Box;
use skein_lib::List;
use skein_lib::{Env, Queue, Time};
use smith_domain as root;
use smith_domain::{Answered, run};
use smith_host_domain::{self as host, channel};

/// A sealed local terminal did not fit the host's matching sealed vocabulary.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BridgeError {
    /// A sealed terminal could not be constructed.
    Terminal,
    /// The charter exceeds the receiving bound.
    Charter,
    /// The concrete transcript exceeds its receiving bound.
    Transcript,
    /// A count does not fit the receiving vocabulary.
    Values,
}

/// Bound the typed local policy and history while moving them into the host Start.
pub(crate) fn prepare_start(source: RunStart, limits: &host::Limits) -> Result<host::Start, BridgeError> {
    let (workspace, directories) = match source.workspace {
        Some(workspace) => {
            let Ok(count) = u32::try_from(workspace.directories.len()) else {
                return Err(BridgeError::Values);
            };
            let mut directories = List::with_capacity(count);
            for directory in workspace.directories {
                let item = channel::Directory {
                    root: directory.root,
                    name: directory.name,
                    writable: directory.writable,
                    git: directory.git,
                    conflicts: directory.conflicts,
                };
                if directories.push(item).is_err() {
                    return Err(BridgeError::Values);
                }
            }
            (Some(skein_lib::Token::new(1)), directories.into_boxed())
        }
        None => (None, Box::<[channel::Directory]>::default()),
    };
    let charter = host::Charter::new(source.charter, limits.charter_bytes).ok_or(BridgeError::Charter)?;
    let transcript = match source.transcript {
        Some(history) => Some(host::Transcript::new(history, limits.transcript_bytes).ok_or(BridgeError::Transcript)?),
        None => None,
    };
    let Ok(count) = u32::try_from(source.answered.len()) else {
        return Err(BridgeError::Terminal);
    };
    let mut answered = List::with_capacity(count);
    for call in source.answered {
        let record = channel::AnsweredCall {
            name: name_to_host(call.name),
            tool: call.tool,
            reply: saved_reply_to_host(call.answer)?,
        };
        if answered.push(record).is_err() {
            return Err(BridgeError::Terminal);
        }
    }
    let Ok(count) = u32::try_from(source.grants.len()) else {
        return Err(BridgeError::Values);
    };
    let mut grants = List::with_capacity(count);
    for grant in source.grants {
        let item =
            channel::Grant { account: grant.name.account, generation: grant.name.generation, valid: grant.valid };
        if grants.push(item).is_err() {
            return Err(BridgeError::Values);
        }
    }
    Ok(host::Start {
        messages: Box::default(),
        logical_run: skein_lib::Token::new(1),
        activation: source.activation,
        workspace,
        charter,
        transcript,
        answered: answered.into_boxed(),
        directories,
        grants: grants.into_boxed(),
    })
}

/// Preserve one transcript-derived name across the local and host domains.
#[must_use]
pub(crate) const fn name_to_host(name: run::CallName) -> channel::CallName {
    channel::CallName { activation: name.activation, completion: name.completion, position: name.position }
}

/// Carry a settled local answer into the host's saved-call record.
pub(crate) fn saved_reply_to_host(answer: Answered) -> Result<channel::SavedReply, BridgeError> {
    match answer {
        Answered::Host(answer) => {
            Ok(channel::SavedReply::Host { error: answer.error(), body: Box::from(answer.text()) })
        }
        Answered::Delivery(delivery) => Ok(channel::SavedReply::Delivery(Box::new(delivery_to_host(*delivery)?))),
        Answered::TooLarge => Ok(channel::SavedReply::TooLarge),
    }
}

/// Preserve a durable local delivery terminal at the spawned-agent boundary.
pub(crate) fn delivery_to_host(delivery: run::Delivery) -> Result<host::Delivery, BridgeError> {
    match delivery {
        run::Delivery::Delivered(delivered) => {
            let Ok(count) = u32::try_from(delivered.receipts().len()) else {
                return Err(BridgeError::Terminal);
            };
            let mut receipts = List::with_capacity(count);
            for receipt in delivered.receipts() {
                let item =
                    host::Receipt::new(receipt.directory(), Box::from(receipt.text())).ok_or(BridgeError::Terminal)?;
                if receipts.push(item).is_err() {
                    return Err(BridgeError::Terminal);
                }
            }
            let value = host::Delivered::new(receipts.into_boxed()).ok_or(BridgeError::Terminal)?;
            Ok(host::Delivery::Delivered(value))
        }
        run::Delivery::Nothing => Ok(host::Delivery::Nothing),
        run::Delivery::Refused(refused) => {
            let marker = match refused.marker() {
                Some(marker) => {
                    Some(host::Marker::new(marker.directory(), Box::from(marker.path())).ok_or(BridgeError::Terminal)?)
                }
                None => None,
            };
            let value =
                host::DeliveryRefusal::new(marker, Box::from(refused.explanation())).ok_or(BridgeError::Terminal)?;
            Ok(host::Delivery::Refused(value))
        }
        run::Delivery::Failed(failed) => {
            let reason = match failed.reason {
                run::DeliveryReason::Unreachable => host::DeliveryReason::Unreachable,
                run::DeliveryReason::RefusedByTarget => host::DeliveryReason::RefusedByTarget,
                run::DeliveryReason::TimedOut => host::DeliveryReason::TimedOut,
                run::DeliveryReason::Broken => host::DeliveryReason::Broken,
                run::DeliveryReason::TooLarge => host::DeliveryReason::TooLarge,
                run::DeliveryReason::Missing => host::DeliveryReason::Missing,
                run::DeliveryReason::Busy => host::DeliveryReason::Busy,
                run::DeliveryReason::Unavailable => host::DeliveryReason::Unavailable,
                run::DeliveryReason::Cancelled => host::DeliveryReason::Cancelled,
                run::DeliveryReason::Unknown => host::DeliveryReason::Unknown,
            };
            Ok(host::Delivery::Failed(host::DeliveryFailure {
                directory: failed.directory,
                reason,
                diagnostic: host::Diagnostic::new(failed.diagnostic.output(), failed.diagnostic.cut()),
            }))
        }
        run::Delivery::Stale => Ok(host::Delivery::Stale),
    }
}

/// Local-owned agent, keeping the existing root until the inline adoption.
#[derive(Debug)]
pub(crate) enum Agent {
    /// Smith's existing root, receiving direct typed events in this increment.
    InProcess(root::Domain),
    /// The host kit, whose ordered process outputs leave through the parent.
    Spawned { domain: host::Domain, limits: host::Limits, out: Queue<host::Output> },
}

impl Agent {
    pub(crate) fn spawned(limits: host::Limits, output_cap: u32) -> Result<Self, Invalid> {
        let required = match host::max_out(&limits).checked_mul(2) {
            Some(required) => required,
            None => return Err(Invalid::Limits),
        };
        if host::worst_case(&limits).is_none() || required > output_cap {
            return Err(Invalid::Limits);
        }
        Ok(Self::Spawned { domain: host::Domain::new(&limits), limits, out: Queue::with_capacity(output_cap) })
    }

    pub(crate) fn root(&self) -> &root::Domain {
        match self {
            Self::InProcess(domain) => domain,
            Self::Spawned { .. } => unreachable!("root work belongs to the in-process kind"),
        }
    }

    pub(crate) fn root_mut(&mut self) -> &mut root::Domain {
        match self {
            Self::InProcess(domain) => domain,
            Self::Spawned { .. } => unreachable!("root work belongs to the in-process kind"),
        }
    }

    pub(crate) fn bounds(&self) -> &host::Limits {
        match self {
            Self::InProcess(_) => unreachable!("host bounds belong to the spawned kind"),
            Self::Spawned { limits, .. } => limits,
        }
    }

    pub(crate) fn deadline(&self) -> Option<Time> {
        match self {
            Self::InProcess(domain) => domain.next_deadline(),
            Self::Spawned { domain, .. } => domain.next_deadline(),
        }
    }

    pub(crate) fn process(&mut self, now: Time, wall: skein_lib::Wall, event: host::process::Event) {
        match self {
            Self::InProcess(_) => unreachable!("process terminals belong to spawned kind"),
            Self::Spawned { domain, limits, out } => {
                host::step(domain, &Env { now, wall, limits: *limits }, host::Input::Process(event), out);
            }
        }
    }

    pub(crate) fn fire(&mut self, now: Time, wall: skein_lib::Wall) {
        match self {
            Self::InProcess(_) => unreachable!("direct root fire has its own typed queue"),
            Self::Spawned { domain, limits, out } => host::fire(domain, &Env { now, wall, limits: *limits }, out),
        }
    }

    pub(crate) fn pop(&mut self) -> Option<host::Output> {
        match self {
            Self::InProcess(_) => None,
            Self::Spawned { out, .. } => out.pop(),
        }
    }

    pub(crate) fn reclaim(&mut self) {
        match self {
            Self::InProcess(domain) => domain.reclaim(),
            Self::Spawned { domain, .. } => domain.reclaim(),
        }
    }
}

pub(crate) fn command(agent: &mut Agent, now: Time, wall: skein_lib::Wall, event: host::parent::Event) {
    match agent {
        Agent::InProcess(_) => unreachable!("direct root commands remain unchanged until3.6"),
        Agent::Spawned { domain, limits, out } => {
            host::step(domain, &Env { now, wall, limits: *limits }, host::Input::Parent(event), out);
        }
    }
}

/// The local policy's owned run values before sealing at the typed parent face.
#[derive(Debug)]
pub(crate) struct RunStart {
    pub(crate) activation: u64,
    pub(crate) charter: run::charter::Charter,
    pub(crate) workspace: Option<run::Workspace>,
    pub(crate) transcript: Option<root::Transcript>,
    pub(crate) answered: Box<[root::AnsweredCall]>,
    pub(crate) grants: Box<[root::Grant]>,
}

/// Preserve today's display words while retaining the host-owned typed answer.
pub(crate) fn answer_text(answer: host::Answer) -> Box<[u8]> {
    match answer.result {
        host::RunResult::Accepted { outcome } => match outcome.into_value() {
            run::outcome::Declared::Report(report) => report.text,
            run::outcome::Declared::Failure(failure) => failure.reason,
            run::outcome::Declared::Verdict(verdict) => verdict.text,
            run::outcome::Declared::Change(_) => Box::from(&b"Change delivered"[..]),
        },
        host::RunResult::Parked => Box::from(&b"Chat parked"[..]),
        host::RunResult::Refused { .. } => Box::from(&b"The agent refused this run"[..]),
        host::RunResult::Failed { failure } => match failure {
            host::RunFailure::Cancelled => Box::from(&b"Run cancelled"[..]),
            host::RunFailure::Transcript(_) => Box::from(&b"Saved transcript was refused"[..]),
            host::RunFailure::Model(_)
            | host::RunFailure::Budget(_)
            | host::RunFailure::Policy(_)
            | host::RunFailure::Stale => Box::from(&b"Run failed"[..]),
        },
    }
}
