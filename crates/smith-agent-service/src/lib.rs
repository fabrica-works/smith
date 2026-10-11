//! One agent run composed from its domain, host channel, LLM and machine.
//! The service keeps the four machines, bounded routing tables, and the queues
//! between them. It never knows provider wire syntax, charter policy decisions,
//! or the meaning of a host's bearer token. [`iterate`] is its only progression
//! entry point; [`Service::completions`] and [`Service::submissions`] meet a
//! shell or simulated kernel. Contracts: protocol/agent.md, sections 4–6;
//! protocol/README.md, sections 5–6.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod answers;
mod events;
mod observations;
mod responses;
mod service;

pub mod profile;
pub mod sinks;

#[cfg(test)]
mod tests;

pub use observations::{
    ChildShare, budget_record, check_exit, child_completed, child_share, opening_model, reported_usage,
    request_tool_choice,
};

pub use events::EventLimits;

pub use sinks::Destination;

pub use answers::{AnswerMeta, answer_record};

pub use service::{
    Config, ConfigError, Effects, Failure, Limits, Service, done, effects_worst_case, failure, iterate, next_deadline,
    work_pending, worst_case,
};
