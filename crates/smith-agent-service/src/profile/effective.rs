//! Names and units retained for startup comparisons and effective-limit reporting.
//! This module keeps no configuration or derived state. Contract:
//! protocol/limits.md, sections 4, 8 and 9.

use skein_lib::Duration;

use super::Fraction;

mod boundaries;
mod channel;
mod header;
mod list;
mod llm;

pub use header::header;
pub use list::effective;

#[cfg(test)]
mod tests;

/// One limit's layer, field and declaration scope, sent in startup refusals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitName {
    pub layer: Layer,
    pub field: &'static str,
    pub per: Per,
}

/// Where an effective limit's name starts, supplied by its deriving owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// A deployment or model declaration, sent by the configuration.
    Declared,
    /// A profile policy value, sent by the configuration.
    Policy,
    /// The native LLM path's derived limit, sent by its adapter.
    Llm,
    /// A rendered result's derived bound, sent by the adapter.
    Render,
    /// A session's derived limit, sent by its parent.
    Session,
    /// A run's derived limit, sent by its parent.
    Run,
    /// A workspace tool's derived limit, sent by its parent.
    Tools,
    /// The machine's derived limit, sent by the service.
    Machine,
    /// A derived transcript limit, sent by the channel.
    Transcript,
    /// A derived channel limit, sent by the service.
    Channel,
    /// An event sink limit, sent by the service.
    Events,
    /// An IO limit, sent by the service.
    Io,
    /// The root domain's derived limit, sent by its service.
    Domain,
    /// The service's own limit, sent by startup.
    Service,
    /// The local domain's derived limit, sent by its service.
    Local,
    /// The inline agent's derived limit, sent by its host.
    Inline,
    /// The spawned host and process adapter's derived limit, sent by startup.
    Host,
}

impl Layer {
    /// The layer's prefix in an effective limit's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Policy => "policy",
            Self::Llm => "llm",
            Self::Render => "render",
            Self::Session => "session",
            Self::Run => "run",
            Self::Tools => "tools",
            Self::Machine => "machine",
            Self::Transcript => "transcript",
            Self::Channel => "channel",
            Self::Events => "events",
            Self::Io => "io",
            Self::Domain => "domain",
            Self::Service => "service",
            Self::Local => "local",
            Self::Inline => "inline",
            Self::Host => "host",
        }
    }
}

/// What one limit applies to, supplied by its declaration's owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Per {
    /// The deployment as a whole.
    Deployment,
    /// One configured endpoint, identified by its number.
    Endpoint(u32),
    /// One model's position within its configured endpoint.
    Model { endpoint: u32, model: u32 },
}

/// An effective value in its unit, sent in a comparison or startup report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    /// A byte count.
    Bytes(u64),
    /// A count of items.
    Count(u64),
    /// A token count.
    Tokens(u64),
    /// Spend in the host's unit.
    Spend(u64),
    /// A duration.
    Duration(Duration),
    /// An exact share of a window.
    Fraction(Fraction),
}

/// One named limit and value, sent as a startup refusal's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limit {
    pub name: LimitName,
    pub value: Value,
}

impl LimitName {
    /// Name one whole-deployment value for a derivation or comparison.
    #[must_use]
    pub const fn deployment(layer: Layer, field: &'static str) -> Self {
        Self { layer, field, per: Per::Deployment }
    }

    /// Attach this name to a value in its unit.
    #[must_use]
    pub const fn value(self, value: Value) -> Limit {
        Limit { name: self, value }
    }
}
