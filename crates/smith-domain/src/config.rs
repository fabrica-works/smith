//! Endpoint and model declarations retained by the root at startup. This module
//! knows no destinations or credentials. Configuration is checked before the
//! root starts; each charter is checked against its declared model quantities.
//! Contracts: domain/run.md, section 4; protocol/limits.md, section 2.1.

use alloc::boxed::Box;
use core::mem::size_of;

use skein_lib::Duration;

use smith_domain_run::{
    Invalid,
    charter::{Endpoint, Llm},
};

use crate::Limits;

/// One configured model, supplied at startup and retained until root teardown.
#[derive(Clone, Debug)]
pub struct ConfiguredModel {
    pub endpoint: Endpoint,
    pub model: Box<[u8]>,
    /// Accepted effort names; an empty list leaves them to the provider.
    pub efforts: Box<[smith_domain_run::charter::Effort]>,
    /// Startup-resolved rates for compacting a saved window.
    pub prices: smith_domain_run::Prices,
    /// Declared usable input tokens.
    pub window: u32,
    /// Declared maximum completion tokens, thinking included.
    pub output: u32,
    /// Endpoint-specific fixed schemas, provider framing and configured option bytes.
    pub protocol_allowance: u64,
    /// First response byte deadline for each call.
    pub head: Duration,
    /// Maximum gap between response events for each call.
    pub idle: Duration,
}

impl ConfiguredModel {
    /// Owned declaration, model name and effort-name cells and bytes.
    #[must_use]
    pub fn worst_case(model_bytes: usize, efforts: &[smith_domain_run::charter::Effort]) -> Option<u64> {
        let mut bytes = u64::try_from(size_of::<Self>()).ok()?.checked_add(u64::try_from(model_bytes).ok()?)?;
        for effort in efforts {
            if effort.is_empty() || effort.len() > 32 {
                return None;
            }
            bytes = bytes
                .checked_add(u64::try_from(size_of::<Box<[u8]>>()).ok()?)?
                .checked_add(u64::try_from(effort.len()).ok()?)?;
        }
        Some(bytes)
    }

    /// Whether an optional effort fits the model's declaration and text bound.
    #[must_use]
    pub fn accepts_effort(efforts: &[smith_domain_run::charter::Effort], effort: Option<&[u8]>) -> bool {
        match effort {
            None => true,
            Some(value) => {
                if value.len() > 32 {
                    return false;
                }
                if efforts.is_empty() {
                    return true;
                }
                for effort in efforts {
                    if effort.as_ref() == value {
                        return true;
                    }
                }
                false
            }
        }
    }
}

/// Endpoint names and served models, supplied at startup separately from limits.
#[derive(Clone, Debug)]
pub struct Config {
    /// Unique endpoint names, at most `Limits::endpoints`.
    pub endpoints: Box<[Endpoint]>,
    /// Endpoints whose LLM a program runs (protocol/programs.md 3), each once.
    pub programs: Box<[Program]>,
    /// Unique models per endpoint; model and program declarations together fit `Limits::configured_model_bytes`.
    pub models: Box<[ConfiguredModel]>,
}

/// One endpoint's program name, supplied at startup for turns and events (programs.md 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub endpoint: Endpoint,
    /// The backend's name in turns and events: `claude_code`.
    pub name: Box<[u8]>,
}

impl Config {
    /// Whether declarations fit their capacities and names are unique.
    #[must_use]
    #[expect(clippy::manual_let_else, reason = "strict subset keeps checked construction exhaustive")]
    pub fn valid(&self, limits: &Limits) -> bool {
        let count = match u32::try_from(self.endpoints.len()) {
            Ok(count) => count,
            Err(_) => return false,
        };
        if count > limits.endpoints {
            return false;
        }
        for (index, endpoint) in self.endpoints.iter().enumerate() {
            for other in self.endpoints.get(index.saturating_add(1)..).unwrap_or_default() {
                if endpoint == other {
                    return false;
                }
            }
        }
        let mut bytes = 0_u64;
        for (index, program) in self.programs.iter().enumerate() {
            if !self.contains(program.endpoint) || program.name.is_empty() {
                return false;
            }
            for other in self.programs.get(index.saturating_add(1)..).unwrap_or_default() {
                if program.endpoint == other.endpoint {
                    return false;
                }
            }
            let owned = match u64::try_from(program.name.len()) {
                Ok(length) => length.checked_add(u64::try_from(size_of::<Program>()).expect("program cell")),
                Err(_) => return false,
            };
            let owned = match owned {
                Some(owned) => owned,
                None => return false,
            };
            bytes = match bytes.checked_add(owned) {
                Some(bytes) => bytes,
                None => return false,
            };
        }
        for (index, model) in self.models.iter().enumerate() {
            if model.prices.check().is_err()
                || model.model.is_empty()
                || model.window == 0
                || model.output == 0
                || model.protocol_allowance > limits.session.protocol_allowance
                || model.head == Duration::ZERO
                || model.idle == Duration::ZERO
                || !self.contains(model.endpoint)
            {
                return false;
            }
            let owned = match ConfiguredModel::worst_case(model.model.len(), &model.efforts) {
                Some(owned) => owned,
                None => return false,
            };
            bytes = match bytes.checked_add(owned) {
                Some(bytes) => bytes,
                None => return false,
            };
            if bytes > limits.configured_model_bytes {
                return false;
            }
            for other in self.models.get(index.saturating_add(1)..).unwrap_or_default() {
                if model.endpoint == other.endpoint && model.model == other.model {
                    return false;
                }
            }
        }
        bytes <= limits.configured_model_bytes
    }

    /// The program configured for an endpoint, if any (protocol/programs.md 3).
    #[must_use]
    #[expect(clippy::manual_find, reason = "programming-model.md 10.1 keeps domain lookups free of closures")]
    pub fn program(&self, endpoint: Endpoint) -> Option<&Program> {
        for program in &self.programs {
            if program.endpoint == endpoint {
                return Some(program);
            }
        }
        None
    }

    /// Whether this endpoint name was configured.
    pub(crate) fn contains(&self, endpoint: Endpoint) -> bool {
        self.endpoints.contains(&endpoint)
    }

    /// Resolve a saved window's declared model and startup rates for its compaction.
    pub(crate) fn window_llm(&self, endpoint: Endpoint, name: &[u8]) -> Option<crate::session::record::Llm> {
        for model in &self.models {
            if model.endpoint == endpoint && model.model.as_ref() == name {
                let prices = model.prices;
                return Some(crate::session::record::Llm {
                    effort: None,
                    model: model.model.clone(),
                    window: model.window,
                    output: model.output,
                    prices: crate::translate::prices(prices),
                });
            }
        }
        None
    }

    /// Progress deadlines for an admitted model, retained only by the root.
    pub(crate) fn progress(&self, llm: &Llm) -> crate::session::Progress {
        for model in &self.models {
            if llm.endpoint == model.endpoint && llm.model == model.model {
                return crate::session::Progress { head: model.head, idle: model.idle };
            }
        }
        unreachable!("admission checked the declared model")
    }

    pub(crate) fn protocol_allowance(&self, llm: &Llm) -> u64 {
        for model in &self.models {
            if llm.endpoint == model.endpoint && llm.model == model.model {
                return model.protocol_allowance;
            }
        }
        unreachable!("admission checked the declared model")
    }

    /// Check a charter's effective quantities against its served model.
    pub(crate) fn check_model(&self, llm: &Llm) -> Result<(), Invalid> {
        if !self.contains(llm.endpoint) {
            return Err(Invalid::Endpoint);
        }
        for model in &self.models {
            if llm.endpoint == model.endpoint && llm.model == model.model {
                if llm.window == 0 || llm.output == 0 || llm.window > model.window || llm.output > model.output {
                    return Err(Invalid::Llm);
                }
                if !ConfiguredModel::accepts_effort(&model.efforts, llm.effort.as_deref()) {
                    return Err(Invalid::Effort);
                }
                return Ok(());
            }
        }
        Err(Invalid::Llm)
    }
}
