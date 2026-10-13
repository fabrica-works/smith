//! Catalog inheritance and the run's model/account selection (shell.md, 5.2).
//! Selection retains settings and account descriptors, never credentials. `select`
//! opens no endpoint; composition and relationship checks follow selection.
use super::{Settings, Sourced};
use std::collections::{BTreeMap, BTreeSet};

mod document;
mod inherit;
mod input;
mod local;
mod models;
pub(super) mod relationships;

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

/// Native agent and local policy values, with only the run's account descriptors.
pub struct Resolved {
    pub agent: smith_agent_shell::config::Configuration,
    pub local: smith_local_domain::Config,
    pub accounts: Vec<Account>,
    pub limits: smith_local_domain::Limits,
    pub settings: Settings,
    pub outside_models: BTreeMap<String, &'static str>,
}

/// Compose selected models through the native configuration and local policy owners.
pub fn resolve(settings: &Settings, _command: &crate::command::Command) -> Result<Resolved, super::Refusal> {
    let Selection { models: active, accounts, mut settings } = select(settings)?;
    let models = models::resolve(&mut settings, &active)?;
    let mut outside_models = BTreeMap::new();
    let children = input::sourced(&settings, "sub_agents")?.value.as_bool().expect("decoded delegation");
    for model in catalog(&settings) {
        if !active.contains(&model) {
            outside_models.insert(model, if children { "account has no source" } else { "sub_agents is off" });
        }
    }
    let profile_document = document::profile(&settings)?;
    let profile = smith_agent_shell::config::profile_from_value(profile_document.clone())
        .map_err(|why| relationships::native(&settings, &models, why))?;
    relationships::budget(&settings, &profile)?;
    let endpoints = document::endpoints(&settings, &models, &accounts)?;
    let mut generated = serde_json::json!({"profile":profile_document,"endpoints":endpoints,
        "environment":document::environment(&settings)?});
    let trace: BTreeMap<_, _> = settings
        .values
        .iter()
        .filter_map(|(key, value)| key.strip_prefix("trace.").map(|field| (field, &value.value)))
        .collect();
    if !trace.is_empty() {
        generated
            .as_object_mut()
            .expect("configuration object")
            .insert("trace".into(), serde_json::to_value(trace).expect("trace settings"));
    }
    let agent = smith_agent_shell::config::from_value(generated)
        .map_err(|why| relationships::native(&settings, &models, why))?;
    smith_local_service::profile::host_limits(&agent.profile, &agent.service.limits)
        .map_err(|why| relationships::profile(&settings, &models, why))?;
    let limits = local::limits(&settings, &models, &agent)?;
    let local = local::build(&settings, &models, &agent, &accounts)?;
    local.validate(&limits).map_err(|why| relationships::local(&settings, &local, &limits, why))?;
    let resolved = Resolved { agent, local, accounts, limits, settings, outside_models };
    for account in &resolved.accounts {
        if account.fields.get("source").and_then(|value| value.value.as_str()) == Some("sign_in") {
            crate::credentials::registration(
                account,
                crate::credentials::product_codex(&resolved, account.number),
                &resolved.settings.workspace,
            )?;
        }
    }
    Ok(resolved)
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
