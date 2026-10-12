//! Pinned model tiers retain only choices in agents/models.toml.
//! `lookup` refuses an unresolved working tier rather than guessing the user's
//! model (benchmarks.md, section 7).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::{Provider, Refusal};

/// A model choice supplied by the tier author, absent while awaiting its owner.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelChoice {
    pub model: Option<String>,
    pub effort: Option<String>,
}

/// Committed model choices indexed by tier and provider.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ModelTiers(pub BTreeMap<String, BTreeMap<Provider, ModelChoice>>);

impl ModelTiers {
    /// Resolve a tier, refusing unknown or not-yet-chosen entries by name.
    pub fn lookup(&self, file: &Path, tier: &str, provider: Provider) -> Result<&ModelChoice, Refusal> {
        let choice = self
            .0
            .get(tier)
            .and_then(|providers| providers.get(&provider))
            .ok_or_else(|| Refusal::new(file, tier, format!("tier has no {provider:?} entry")))?;
        if choice.model.as_ref().is_none_or(String::is_empty) || choice.effort.as_ref().is_none_or(String::is_empty) {
            return Err(Refusal::new(
                file,
                tier,
                format!("{provider:?} working model and effort await the user's choice"),
            ));
        }
        Ok(choice)
    }
}

/// Read model tiers without selecting a default for an unresolved entry.
pub fn read_model_tiers(file: &Path) -> Result<ModelTiers, Refusal> {
    crate::formats::read_document(file)
}

/// List prices by model from smith's presets; notional (shell.md, section 6.1).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PriceTable {
    pub models: BTreeMap<String, Prices>,
}

/// One model's micro-dollars per million normalised tokens (benchmarks.md, 8.2).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Prices {
    pub fresh: u64,
    pub cache_read: u64,
    pub output: u64,
}

/// Read the committed price basis, refusing nonpositive rates.
pub fn read_prices(file: &Path) -> Result<PriceTable, Refusal> {
    let table: PriceTable = crate::formats::read_document(file)?;
    for (model, prices) in &table.models {
        if model.is_empty() || [prices.fresh, prices.cache_read, prices.output].contains(&0) {
            return Err(Refusal::new(file, &format!("models.{model}"), "model and positive rates required"));
        }
    }
    Ok(table)
}
