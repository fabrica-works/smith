//! Startup endpoint names and their resolved transport, dialect and options.
//! This table retains no credential. The service gives the destinations to
//! skein's connection component and keeps their actual HTTP heads for native
//! validation before copying an observation, using names through this mapping.
//! Contract: protocol/agent.md, section 4; protocol/llm.md, sections 2 and 6.

use alloc::boxed::Box;
use core::mem::size_of;

use skein_lib::{Duration, List, Map};
use skein_llm::{Prompt, Provider};
use skein_llm_connection as connection;
use smith_domain::llm;

/// Optional provider identity prepended to the endpoint's instructions.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum IdentityProfile {
    /// Send only the domain's system instructions.
    Plain,
    /// Opt in to skein's archived Claude Code identity blocks.
    ClaudeCode,
}

/// A model's policy for a reasoning item past its bound (protocol/limits.md, section 2.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum OversizedReasoning {
    /// Refuse the completion with the reasoning item's typed limit; the default.
    Fail,
    /// Keep the completion without the item and emit its drop as a fact.
    Drop,
}

/// Measure actual endpoint options and provider identity for startup's own part.
/// Field punctuation belongs to the separately measured provider framing.
/// Contract: protocol/limits.md, sections 3.2 and 3.9.
pub fn endpoint_own(identity: IdentityProfile) -> Result<u64, crate::Error> {
    let mut bytes = 0_u64;
    match identity {
        IdentityProfile::Plain => {}
        IdentityProfile::ClaudeCode => {
            let instructions = match skein_llm::anthropic::identity::instructions(&[]) {
                Ok(instructions) => instructions,
                Err(error) => return Err(crate::Error::from_shared(error)),
            };
            bytes = bytes
                .checked_add(u64::try_from(instructions.len()).or(Err(crate::Error::Limit))?)
                .ok_or(crate::Error::Limit)?;
        }
    }
    Ok(bytes)
}

/// One declared model's per-call policy, supplied by startup and retained by its endpoint.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfiguredModel {
    pub name: Box<[u8]>,
    pub oversized_reasoning: OversizedReasoning,
}

/// One startup-resolved endpoint, named as the domain names it.
/// The address, TLS trust/name, dialect, path and headers live in destination.
#[derive(Debug)]
pub struct ConfiguredEndpoint {
    /// Numeric name used by a charter and a completion's prompt.
    pub name: llm::Endpoint,
    /// Resolved transport and LLM endpoint given to skein's connection pool.
    pub destination: connection::Endpoint,
    pub connect: Duration,
    pub handshake: Duration,
    /// Optional provider system identity selected at startup.
    pub identity: IdentityProfile,
    /// The models whose per-call reasoning policy this endpoint serves.
    pub models: Box<[ConfiguredModel]>,
}

/// Per-name options kept after the connection component takes destinations.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EndpointOptions {
    /// Index in skein's configured destination list.
    pub index: u32,
    pub connect: Duration,
    pub handshake: Duration,
    /// Dialect that owns replay and provider-specific request rules.
    pub provider: Provider,
    /// Actual destination head used to validate content before its observation is copied.
    pub endpoint: skein_llm::Endpoint,
    /// Native per-endpoint bounds, including its transport pieces and measured head.
    pub client: skein_llm::client::Limits,
    /// Optional provider system identity selected at startup.
    pub identity: IdentityProfile,
    /// The models whose per-call reasoning policy this endpoint serves.
    pub models: Box<[ConfiguredModel]>,
}

impl EndpointOptions {
    /// Select the declared model's per-call policy; an undeclared name keeps the default failure.
    #[must_use]
    pub fn drop_reasoning(&self, requested_model: &[u8]) -> bool {
        for model in &self.models {
            if model.name.as_ref() == requested_model {
                return match model.oversized_reasoning {
                    OversizedReasoning::Fail => false,
                    OversizedReasoning::Drop => true,
                };
            }
        }
        false
    }
}

/// Why startup endpoint registration cannot be admitted.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EndpointError {
    /// More configured endpoints than the component's bound.
    TooMany,
    /// Two destinations use the same numeric name.
    Duplicate,
    /// A prompt named no configured destination.
    Unknown,
    /// The selected identity is incompatible with the dialect or bound.
    Identity,
}

/// Checked numeric names and resolved skein destinations.
#[derive(Debug)]
pub struct Endpoints {
    destinations: List<connection::Endpoint>,
    options: Map<u32, EndpointOptions>,
}

impl Endpoints {
    /// Validate names before a service starts its loop.
    pub fn new(configured: Box<[ConfiguredEndpoint]>, endpoint_limit: u32) -> Result<Endpoints, EndpointError> {
        if configured.len() > usize::try_from(endpoint_limit).expect("u32 fits usize") {
            return Err(EndpointError::TooMany);
        }
        let mut destinations = List::with_capacity(endpoint_limit);
        let mut options = Map::with_capacity(endpoint_limit);
        for (index, configured) in configured.into_iter().enumerate() {
            let index = u32::try_from(index).or(Err(EndpointError::TooMany))?;
            if options.contains_key(&configured.name.0) {
                return Err(EndpointError::Duplicate);
            }
            let provider = configured.destination.llm.provider;
            if configured.identity == IdentityProfile::ClaudeCode && provider != Provider::Anthropic {
                return Err(EndpointError::Identity);
            }
            let client = configured.destination.limits;
            let endpoint = configured.destination.llm.clone();
            destinations.push(configured.destination).or(Err(EndpointError::TooMany))?;
            let option = EndpointOptions {
                index,
                connect: configured.connect,
                handshake: configured.handshake,
                provider,
                endpoint,
                client,
                identity: configured.identity,
                models: configured.models,
            };
            options.insert(configured.name.0, option).or(Err(EndpointError::TooMany))?;
        }
        Ok(Endpoints { destinations, options })
    }

    /// The configured endpoint named in a domain request.
    #[must_use]
    pub fn resolve(&self, name: llm::Endpoint) -> Option<&EndpointOptions> {
        self.options.get(&name.0)
    }

    /// Apply the startup options to the neutral prompt given to skein.
    /// Its dialect, target and headers are already in the destination.
    pub fn apply(&self, name: llm::Endpoint, prompt: &mut Prompt) -> Result<u32, EndpointError> {
        let options = self.resolve(name).ok_or(EndpointError::Unknown)?;
        match options.identity {
            IdentityProfile::Plain => {}
            IdentityProfile::ClaudeCode => {
                prompt.instructions = skein_llm::anthropic::identity::instructions(&prompt.instructions)
                    .or(Err(EndpointError::Identity))?;
            }
        }
        Ok(options.index)
    }

    /// Borrow configured destination limits and credential ceilings for startup accounting.
    #[must_use]
    pub fn destinations(&self) -> &List<connection::Endpoint> {
        &self.destinations
    }

    /// Bytes owned by startup options beyond the map's inline cells.
    #[must_use]
    pub fn option_bytes(&self) -> Option<u64> {
        let mut held = 0_u64;
        for (_, option) in &self.options {
            held = held.checked_add(u64::try_from(option.endpoint.authority.len()).ok()?)?;
            held = held.checked_add(u64::try_from(option.endpoint.target.len()).ok()?)?;
            held = held.checked_add(u64::try_from(size_of_val(option.endpoint.headers.as_ref())).ok()?)?;
            for header in &option.endpoint.headers {
                held = held.checked_add(u64::try_from(header.name.len()).ok()?)?;
                held = held.checked_add(u64::try_from(header.value.len()).ok()?)?;
            }
            for model in &option.models {
                held = held.checked_add(u64::try_from(size_of::<ConfiguredModel>()).ok()?)?;
                held = held.checked_add(u64::try_from(model.name.len()).ok()?)?;
            }
        }
        Some(held)
    }

    /// Split names from resolved destinations for the owning component.
    #[must_use]
    pub fn into_parts(self) -> (List<connection::Endpoint>, Map<u32, EndpointOptions>) {
        (self.destinations, self.options)
    }
}
