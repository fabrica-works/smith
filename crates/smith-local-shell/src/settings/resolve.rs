//! Catalog inheritance and the run's model/account selection (shell.md, 5.2).
//! Selection retains settings and account descriptors, never credentials. `select`
//! opens no endpoint; composition and relationship checks follow selection.
use super::{Settings, Sourced};
use std::collections::{BTreeMap, BTreeSet};

mod inherit;
mod input;

/// Main first, followed by eligible child models, with their needed accounts.
#[derive(Debug)]
pub struct Selection {
    pub models: Vec<String>,
    pub accounts: Vec<Account>,
    pub settings: Settings,
}

/// An account's named source and fields, without any borrowed or refreshed value.
#[derive(Debug)]
pub struct Account {
    pub name: String,
    pub number: u32,
    pub fields: BTreeMap<String, Sourced>,
}

/// Expand preset defaults, then select only models whose account names a source.
pub fn select(settings: &Settings) -> Result<Selection, super::Refusal> {
    let mut settings = settings.clone();
    inherit::apply(&mut settings)?;
    let main = input::text(&settings, "model")?.to_string();
    let main_account = account(&settings, &main)?;
    input::text(&settings, &format!("accounts.{}.source", super::name(&main_account)))?;
    let children = input::sourced(&settings, "sub_agents")?.value.as_bool().expect("validated delegation");
    let mut models = vec![main.clone()];
    if children {
        for model in catalog(&settings) {
            if model == main {
                continue;
            }
            let account = account(&settings, &model)?;
            if settings.values.contains_key(&format!("accounts.{}.source", super::name(&account))) {
                models.push(model);
            }
        }
    }
    let mut accounts = Vec::<Account>::new();
    for model in &models {
        let name = account(&settings, model)?;
        if accounts.iter().any(|account| account.name == name) {
            continue;
        }
        let prefix = format!("accounts.{}.", super::name(&name));
        let fields = settings
            .values
            .iter()
            .filter_map(|(key, value)| key.strip_prefix(&prefix).map(|field| (field.into(), value.clone())))
            .collect();
        accounts.push(Account {
            name,
            number: u32::try_from(accounts.len()).expect("accounts fit settings input"),
            fields,
        });
    }
    Ok(Selection { models, accounts, settings })
}

pub(super) fn catalog(settings: &Settings) -> BTreeSet<String> {
    settings
        .values
        .keys()
        .filter_map(|key| {
            key.strip_prefix("models.")
                .and_then(|path| path.rsplit_once('.'))
                .map(|(name, _)| serde_json::from_str(name).unwrap_or_else(|_| name.into()))
        })
        .collect()
}

fn account(settings: &Settings, model: &str) -> Result<String, super::Refusal> {
    let (endpoint, _) =
        model.split_once('/').ok_or_else(|| input::refuse(settings, "model", "expected endpoint/model"))?;
    input::text(settings, &format!("endpoints.{}.account", super::name(endpoint))).map(str::to_owned)
}
