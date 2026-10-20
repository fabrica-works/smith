//! Account names and numbers (protocol/hosts.md, 5.2; shell.md, 5.2 and 7.4).
//! Keeps no state; `check` checks every declared name before selection.
use super::super::{Layer, Origin, Refusal, Settings};
use std::collections::BTreeMap;

/// First four SHA-256 bytes of the account's name, read big-endian.
pub(crate) fn number(name: &str) -> u32 {
    let digest = skein_lib::sha256::digest(name.as_bytes());
    u32::from_be_bytes(digest.get(..4).expect("SHA-256 prefix").try_into().expect("four bytes"))
}

fn refuse(settings: &Settings, name: &str, problem: &str) -> Refusal {
    let key = format!("accounts.{}", super::super::name(name));
    let prefix = format!("{key}.");
    let mut refusal = super::input::refuse(settings, &key, problem);
    if let Some(origin) = settings
        .values
        .iter()
        .filter(|(key, _)| key.starts_with(&prefix))
        .map(|(_, field)| &field.origin)
        .min_by_key(|origin| position(origin))
    {
        refusal.origin = origin.clone();
    }
    refusal
}

fn position(origin: &Origin) -> (u8, u32) {
    match origin {
        Origin::BuiltIn => (0, 0),
        Origin::File { layer: Layer::User, line, .. } => (1, *line),
        Origin::File { layer: Layer::Repository, line, .. } => (2, *line),
        Origin::Override { index } => (3, *index),
        Origin::Flag { .. } => (4, 0),
    }
}

fn refuse_collisions(settings: &Settings, numbered: &[(String, u32)]) -> Result<(), Refusal> {
    let mut seen = BTreeMap::new();
    for (name, number) in numbered {
        if name.is_empty()
            || name.len() > 32
            || !name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(refuse(settings, name, "account name must be 1 to 32 of a-z, 0-9 or -"));
        }
        if let Some(prior) = seen.insert(*number, name) {
            return Err(refuse(settings, name, &format!("accounts.{prior} and accounts.{name} share number {number}")));
        }
    }
    Ok(())
}

pub(super) fn check(settings: &Settings) -> Result<(), Refusal> {
    let mut names = std::collections::BTreeSet::new();
    for key in settings.values.keys() {
        if let Some(rest) = key.strip_prefix("accounts.") {
            let (name, _) = super::super::named_field(rest)
                .map_err(|()| super::input::refuse(settings, key, "invalid account name encoding"))?;
            names.insert(name);
        }
    }
    let numbered: Vec<_> = names
        .into_iter()
        .map(|name| {
            let number = number(&name);
            (name, number)
        })
        .collect();
    refuse_collisions(settings, &numbered)
}

#[cfg(test)]
mod tests;
