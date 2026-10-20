//! Account metadata for check and startup warnings (shell.md, 4.4 and 7.4;
//! protocol/hosts.md, sections 5.2 and 5.4).
//! Keeps provider, source, number and place, never credential values. `report`
//! inspects the catalog; startup warnings omit program sources not checked.
use super::{Account, Credential, Directory, Source, inspect, product, program_check::ProgramStates, source};
use crate::settings::{self, resolve::Resolved};
use skein_lib::Wall;
use std::{collections::BTreeMap, fmt::Write, path::PathBuf};

/// A catalog account's redacted state and location, returned by check.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AccountReport {
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub source: &'static str,
    pub number: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept: Option<Kept>,
    #[serde(flatten)]
    pub credential: Credential,
}

/// The place check reports for an account, without reading its value.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind", content = "at", rename_all = "snake_case")]
pub enum Kept {
    /// The account's saved file, reported by the shell.
    File(PathBuf),
    /// The program's configuration directory, reported by the shell.
    Directory(PathBuf),
    /// The environment variable's name, reported by the shell.
    Variable(String),
}

impl AccountReport {
    /// The catalog metadata line preceding the account's state.
    #[must_use]
    pub(crate) fn line(&self, account: &str) -> String {
        let mut line = format!("account {account}: ");
        if let Some(provider) = &self.provider {
            write!(line, "provider {provider}, ").expect("String formatting");
        }
        write!(line, "source {}, number {}", self.source, self.number).expect("String formatting");
        if let Some(place) = &self.kept {
            let at = match place {
                Kept::File(path) | Kept::Directory(path) => path.display().to_string(),
                Kept::Variable(variable) => variable.clone(),
            };
            write!(line, ", kept at {at}").expect("String formatting");
        }
        line
    }
}

pub(crate) fn accounts(resolved: &Resolved) -> BTreeMap<String, Account> {
    let mut accounts = BTreeMap::<String, Account>::new();
    for (key, value) in &resolved.settings.values {
        if let Some(rest) = key.strip_prefix("accounts.") {
            let (name, field) = settings::named_field(rest).expect("validated account key");
            let number = settings::resolve::numbers::number(&name);
            accounts
                .entry(name.clone())
                .or_insert_with(|| Account { name, number, fields: BTreeMap::new() })
                .fields
                .insert(field.into(), value.clone());
        }
    }
    accounts
}

/// Inspect all declared accounts using the caller's program checks and wall time.
#[must_use]
pub fn report(resolved: &Resolved, wall: Wall, programs: &ProgramStates) -> BTreeMap<String, AccountReport> {
    catalog(resolved, wall, programs, true)
}

pub(crate) fn unused_warnings(resolved: &Resolved, wall: Wall, programs: &ProgramStates) -> Vec<crate::check::Warning> {
    catalog(resolved, wall, programs, false)
        .into_iter()
        .filter_map(|(name, account)| {
            (!account.credential.usable())
                .then_some(crate::check::Warning::Credential { account: name, credential: account.credential })
        })
        .collect()
}

fn catalog(
    resolved: &Resolved,
    wall: Wall,
    programs: &ProgramStates,
    include_required: bool,
) -> BTreeMap<String, AccountReport> {
    let settings = &resolved.settings;
    let directory = Directory::of(settings);
    accounts(resolved)
        .into_iter()
        .filter_map(|(name, account)| {
            let needed = resolved.accounts.iter().find(|needed| needed.name == name);
            if needed.is_some() && !include_required {
                return None;
            }
            let descriptor = source(
                &account.fields,
                settings,
                needed.is_some_and(|needed| product::codex(resolved, needed.number)),
                &name,
                directory.as_ref(),
            );
            let (source, kept) = match &descriptor {
                Source::SignIn { file, .. } => ("sign_in", directory.as_ref().map(|_| Kept::File(file.clone()))),
                Source::Key { file } => ("key", directory.as_ref().map(|_| Kept::File(file.clone()))),
                Source::Env { variable, .. } => ("env", Some(Kept::Variable(variable.clone()))),
                Source::Program { .. } => {
                    if !include_required && !programs.contains_key(&name) {
                        return None;
                    }
                    ("program", directory.as_ref().map(|directory| Kept::Directory(directory.program(&name))))
                }
                Source::Nothing => ("none", None),
            };
            let credential = inspect(&descriptor, wall, programs);
            let provider = account.fields.get("provider").and_then(|field| field.value.as_str()).map(str::to_owned);
            Some((
                name,
                AccountReport {
                    required: needed.is_some(),
                    provider,
                    source,
                    number: account.number,
                    kept,
                    credential,
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests;
