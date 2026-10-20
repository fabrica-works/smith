//! Checked ownership bound for local policy and its in-process agent
//! (domain/host.md, sections 6 and 8; programming-model.md, section 6.3).

use alloc::boxed::Box;
use core::mem::size_of;
use skein_lib::{Map, Queue};
use smith_domain::{self as agent, run};

use crate::{DeliveryRecord, Fact, IntentDirectory, Request};

/// Capacity of one local chat and its child agent.
#[derive(Clone, Debug)]
pub struct Limits {
    /// Native typed charter, transcript, turn and delivery receiving bounds for either kind.
    pub smith: agent::Limits,
    /// Child kind and its own routing and lifecycle allowances.
    pub agent: AgentLimits,
    /// Endpoint names permitted by the child agent's configuration.
    pub endpoints: Box<[run::charter::Endpoint]>,
    /// Maximum chat store-key bytes.
    pub chat_bytes: u32,
    /// Maximum local instructions and brief text bytes.
    pub text_bytes: u32,
    /// Maximum model choices, main included.
    pub models: u32,
    /// Maximum terminal line bytes.
    pub line_bytes: u32,
    /// Maximum rendered text bytes per notice.
    pub show_bytes: u32,
    /// Maximum turns awaiting durable acknowledgement.
    pub unsaved: u32,
    /// Maximum content-free observations.
    pub facts: u32,
}

/// The child selected by the host, sharing the same typed parent vocabulary.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "fixed receiving profiles remain copyable and are included in Domain ownership"
)]
pub enum AgentLimits {
    /// Native roots with LLM and machine effects delegated in this process.
    Inline(smith_inline_agent::Limits),
    /// An agent process supervised by the host kit.
    Spawned(smith_host_domain::Limits),
}

impl AgentLimits {
    /// Build the single inline child with enough credit for one complete native turn.
    #[must_use]
    pub fn inline(
        smith: agent::Limits,
        turns: u32,
        wall_time: skein_lib::Duration,
        cancel_grace: skein_lib::Duration,
    ) -> Option<Self> {
        let limits = smith_inline_agent::Limits {
            slots: 1,
            smith,
            window: agent::Window { turns, bytes: agent::max_turn_bytes(&smith)? },
            wall_time,
            cancel_grace,
            facts: 32,
        };
        smith_inline_agent::worst_case(&limits)?;
        Some(Self::Inline(limits))
    }

    pub(crate) fn max_out(self) -> u32 {
        match self {
            Self::Inline(limits) => smith_inline_agent::max_out(&limits),
            Self::Spawned(limits) => smith_host_domain::max_out(&limits),
        }
    }

    fn worst_case(self) -> Option<u64> {
        match self {
            Self::Inline(limits) => smith_inline_agent::worst_case(&limits),
            Self::Spawned(limits) => smith_host_domain::worst_case(&limits),
        }
    }
}

/// Largest terminal line that fits the run's rendered message bound (domain/host.md, section 8).
#[must_use]
pub fn max_line_bytes(message_bytes: u32) -> Option<u32> {
    let label_bytes = u32::try_from(crate::person::LABEL.len()).ok()?;
    message_bytes.checked_sub(label_bytes)?.checked_sub(2)
}

/// Bound on owned local state, including the child domain.
#[must_use]
pub fn worst_case(limits: &Limits) -> Option<u64> {
    if limits.models == 0 || limits.unsaved == 0 || limits.show_bytes < limits.chat_bytes.checked_add(52)?.max(64) {
        return None;
    }
    agent::worst_case(&limits.smith)?;
    let agent = limits.agent.worst_case()?;
    let endpoints = u64::from(limits.models)
        .checked_mul(u64::try_from(size_of::<run::charter::Llm>()).ok()?)?
        .checked_add(u64::from(limits.models).checked_mul(u64::try_from(size_of::<agent::session::Progress>()).ok()?)?)?
        .checked_add(u64::from(limits.models).checked_mul(u64::try_from(size_of::<u64>()).ok()?)?)?
        .checked_add(u64::from(limits.models).checked_mul(u64::from(limits.text_bytes))?)?;
    let accounts = u64::from(limits.smith.accounts).checked_mul(u64::try_from(size_of::<u32>()).ok()?)?;
    let targets = u64::from(limits.smith.run.directories).checked_mul(u64::from(limits.text_bytes))?.checked_mul(2)?;
    let configuration = u64::from(limits.chat_bytes)
        .checked_add(limits.smith.configured_model_bytes)?
        .checked_add(u64::from(limits.text_bytes).checked_mul(2)?)?
        .checked_add(endpoints)?
        .checked_add(accounts)?
        .checked_add(targets)?
        .checked_add(limits.smith.run.run_bytes)?;
    let inbox = limits.smith.run.messages.checked_add(1)?;
    let lines = Queue::<crate::person::Line>::worst_case(inbox)?
        .checked_add(u64::from(inbox).checked_mul(u64::from(limits.line_bytes))?.checked_mul(2)?)?;
    // Each retained turn can open a window, owning one opener box at most.
    let record = limits
        .smith
        .session
        .history_bytes
        .checked_add(u64::try_from(size_of::<agent::session::record::Opener>()).ok()?)?
        .checked_add(
            u64::from(limits.smith.session.history_messages)
                .checked_mul(u64::try_from(size_of::<agent::Turn>()).ok()?)?,
        )?;
    let unsaved = u64::from(limits.unsaved).checked_mul(record)?;
    let detail = match limits.agent {
        AgentLimits::Inline(_) => u64::from(limits.smith.session.failure_bytes),
        AgentLimits::Spawned(child) => u64::from(child.detail_bytes),
    };
    let out = Queue::<Request>::worst_case(crate::max_out(limits))?
        .checked_add(u64::from(limits.show_bytes))?
        .checked_add(detail.checked_mul(2)?)?;
    let child_out = match limits.agent {
        AgentLimits::Inline(_) => Queue::<smith_inline_agent::Output>::worst_case(crate::max_out(limits))?,
        AgentLimits::Spawned(_) => Queue::<smith_host_domain::Output>::worst_case(crate::max_out(limits))?,
    };
    let held = Queue::<crate::domain::Held>::worst_case(1)?.checked_add(limits.smith.session.completion_bytes)?;
    let intent = skein_lib::List::<IntentDirectory>::worst_case(run::MAX_DIRECTORIES)?
        .checked_add(u64::from(run::MAX_DIRECTORIES).checked_mul(u64::try_from(run::Receipt::CAPACITY).ok()?)?)?
        .checked_add(targets)?;
    let answered = Map::<run::CallName, DeliveryRecord>::worst_case(limits.smith.run.answered_calls)?.checked_add(
        u64::from(limits.smith.run.answered_calls).checked_mul(run::Delivered::worst_case().max(intent))?,
    )?;
    let heads = Map::<u32, Box<[u8]>>::worst_case(limits.smith.run.directories)?
        .checked_add(u64::from(limits.smith.run.directories).checked_mul(u64::try_from(run::Receipt::CAPACITY).ok()?)?)?
        .checked_mul(2)?;
    let delivery = skein_lib::List::<run::Receipt>::worst_case(run::MAX_DIRECTORIES)?
        .checked_add(u64::try_from(size_of::<crate::delivery::InPlace>()).ok()?)?
        .checked_add(run::Delivered::worst_case().checked_mul(4)?)?
        .checked_add(limits.smith.run.outcome_bytes.checked_mul(2)?)?
        .checked_add(skein_lib::List::<IntentDirectory>::worst_case(run::MAX_DIRECTORIES)?.checked_mul(3)?)?
        .checked_add(targets.checked_mul(3)?)?
        .checked_add(
            u64::from(run::MAX_DIRECTORIES).checked_mul(u64::try_from(run::Receipt::CAPACITY).ok()?)?.checked_mul(3)?,
        )?
        .checked_add(run::Delivered::worst_case().checked_mul(2)?)?;
    agent
        .checked_add(configuration)?
        .checked_add(lines)?
        .checked_add(skein_lib::List::<smith_host_domain::Grant>::worst_case(limits.smith.accounts)?)?
        .checked_add(u64::from(limits.line_bytes.max(20)))?
        .checked_add(32)?
        .checked_add(Queue::<smith_host_domain::Message>::worst_case(inbox)?)?
        .checked_add(u64::from(inbox).checked_mul(u64::try_from(crate::person::LABEL.len()).ok()?)?)?
        .checked_add(unsaved)?
        .checked_add(out)?
        .checked_add(child_out)?
        .checked_add(smith_host_domain::ModelFault::BYTES.checked_mul(3)?)?
        .checked_add(held)?
        .checked_add(delivery)?
        .checked_add(answered)?
        .checked_add(heads)?
        .checked_add(Queue::<Fact>::worst_case(limits.facts)?)
}
