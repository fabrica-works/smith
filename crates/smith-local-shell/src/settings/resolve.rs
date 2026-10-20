//! Catalog inheritance and the run's model/account selection (shell.md, 5.2).
//! Selection retains settings and account descriptors, never credentials. `select`
//! opens no endpoint; composition and relationship checks follow selection.
use super::{Settings, Sourced};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod document;
mod inherit;
mod input;
mod local;
pub(super) mod models;
pub(crate) mod numbers;
pub(crate) mod programs;
mod providers;
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

impl Account {
    /// The source's configured name, empty when absent.
    #[must_use]
    pub fn source(&self) -> &str {
        self.fields.get("source").and_then(|field| field.value.as_str()).unwrap_or_default()
    }
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
    build(select(settings)?, false).map(|(resolved, _document)| resolved)
}

/// Resolve startup once and return its generated agent document for a spawned child.
pub fn resolve_startup(settings: &Settings) -> Result<(Resolved, Box<[u8]>), super::Refusal> {
    build(select(settings)?, true)
}

pub(crate) fn build(selection: Selection, document: bool) -> Result<(Resolved, Box<[u8]>), super::Refusal> {
    let Selection { models: active, accounts, mut settings } = selection;
    let models = models::resolve(&mut settings, &active)?;
    let mut outside_models = BTreeMap::new();
    let children = input::sourced(&settings, "sub_agents")?.value.as_bool().expect("decoded delegation");
    for model in catalog(&settings) {
        if !active.contains(&model)
            && (!model.starts_with("openai/") || settings.values.contains_key("endpoints.openai.account"))
        {
            let program = account(&settings, &model).ok().is_some_and(|name| {
                settings
                    .values
                    .get(&format!("accounts.{}.source", super::name(&name)))
                    .is_some_and(|field| field.value.as_str() == Some("program"))
            });
            outside_models.insert(
                model,
                if program {
                    "a program runs no sub-agent"
                } else if children {
                    "account has no source"
                } else {
                    "sub_agents is off"
                },
            );
        }
    }
    let profile_document = document::profile(&settings)?;
    let profile = smith_agent_shell::config::profile_from_value(profile_document.clone())
        .map_err(|why| relationships::native(&settings, &models, why))?;
    relationships::budget(&settings, &profile)?;
    let endpoints = document::endpoints(&settings, &models)?;
    let programs = programs::document(&settings, &endpoints)?;
    let mut generated = serde_json::json!({"profile":profile_document,"endpoints":endpoints,
        "environment":document::environment(&settings)?});
    if let Some(programs) = programs {
        generated.as_object_mut().expect("configuration object").insert("programs".into(), programs);
    }
    let trace: BTreeMap<_, _> = settings
        .values
        .iter()
        .filter_map(|(key, value)| key.strip_prefix("trace.").map(|field| (field, &value.value)))
        .collect();
    if trace.contains_key("path") {
        generated
            .as_object_mut()
            .expect("configuration object")
            .insert("trace".into(), serde_json::to_value(trace).expect("trace settings"));
    }
    let bytes = if document {
        serde_json::to_vec(&generated).expect("generated agent document").into_boxed_slice()
    } else {
        Box::new([])
    };
    let mut agent = smith_agent_shell::config::from_value(generated)
        .map_err(|why| relationships::native(&settings, &models, why))?;
    agent.declarations.role_bound = if settings.values.contains_key("instructions") {
        None
    } else {
        Some(u32::try_from(crate::role::TASK_ROLE.len()).expect("the task role fits four kibibytes"))
    };
    smith_agent_service::profile::validate::host_part_holds_the_role(
        &agent.profile,
        &agent.declarations,
        &agent.service.limits.domain.run,
    )
    .map_err(|why| relationships::profile(&settings, &models, why))?;
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
    Ok((resolved, bytes))
}

/// Check the declarations shared by login and run startup without selecting a model.
pub(crate) fn checked(settings: &Settings) -> Result<Settings, super::Refusal> {
    let mut settings = settings.clone();
    inherit::apply(&mut settings)?;
    numbers::check(&settings)?;
    providers::check(&mut settings)?;
    programs::check_sources(&settings)?;
    Ok(settings)
}

/// Expand preset defaults, then select only models whose account names a source.
pub fn select(settings: &Settings) -> Result<Selection, super::Refusal> {
    let settings = checked(settings)?;
    let main = input::text(&settings, "model")?.to_string();
    let main_account = account(&settings, &main)?;
    input::text(&settings, &format!("accounts.{}.source", super::name(&main_account)))?;
    let children = input::sourced(&settings, "sub_agents")?.value.as_bool().expect("validated delegation");
    let mut models = vec![main.clone()];
    let main_endpoint = main.split_once('/').expect("validated main model").0;
    for model in catalog(&settings) {
        if children || model.split_once('/').is_some_and(|(endpoint, _)| endpoint == main_endpoint) {
            if model == main {
                continue;
            }
            let Ok(account) = account(&settings, &model) else { continue };
            let source = settings.values.get(&format!("accounts.{}.source", super::name(&account)));
            if model.split_once('/').is_some_and(|(endpoint, _)| endpoint != main_endpoint)
                && source.is_some_and(|field| field.value.as_str() == Some("program"))
            {
                continue;
            }
            if source.is_some() {
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
        let number = numbers::number(&name);
        accounts.push(Account { name, number, fields });
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
    input::text(settings, &format!("endpoints.{}.account", super::name(endpoint)))
        .map(str::to_owned)
        .map_err(|why| if endpoint == "openai" { providers::missing(settings, endpoint) } else { why })
}
