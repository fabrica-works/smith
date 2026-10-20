//! Provider registrations and endpoint account defaults (shell.md, 5.2, 7.1).
//! Checks declarations only; neither host identifiers nor records are read.
use super::{super::Settings, input};
use std::collections::BTreeSet;

pub(super) fn check(settings: &mut Settings) -> Result<(), super::super::Refusal> {
    let names: BTreeSet<_> = settings
        .values
        .keys()
        .filter_map(|key| {
            key.strip_prefix("accounts.").and_then(|rest| super::super::named_field(rest).ok()).map(|(name, _)| name)
        })
        .collect();
    for name in &names {
        let prefix = format!("accounts.{}.", super::super::name(name));
        let source = settings.values.get(&format!("{prefix}source")).and_then(|value| value.value.as_str());
        let provider = settings.values.get(&format!("{prefix}provider")).and_then(|value| value.value.as_str());
        let withdrawn = ["file", "format", "program"]
            .into_iter()
            .find(|field| settings.values.contains_key(&format!("{prefix}{field}")))
            .or_else(|| (source == Some("borrow")).then_some("source"));
        if let Some(field) = withdrawn {
            return Err(input::refuse(
                settings,
                &format!("{prefix}{field}"),
                &format!("declare `provider = \"openai\"` and `source = \"sign_in\"`, then run `smith login {name}`"),
            ));
        }
        if source != Some("sign_in") {
            continue;
        }
        if provider == Some("anthropic") {
            return Err(input::refuse(
                settings,
                &format!("{prefix}source"),
                "the anthropic preset has no registration",
            ));
        }
        let fields: &[&str] = if provider.is_some() {
            &["client_id", "redirect_uri", "scope", "token_format"]
        } else {
            &["issuer", "keys_url", "revocation_url", "redirect_port"]
        };
        for field in fields {
            let key = format!("{prefix}{field}");
            if settings.values.contains_key(&key) {
                return Err(input::refuse(
                    settings,
                    &key,
                    if provider.is_some() {
                        "the provider's preset gives the registration"
                    } else {
                        "needs `provider`"
                    },
                ));
            }
        }
        for field in ["issuer", "authorization_url", "token_url", "keys_url", "revocation_url"] {
            let key = format!("{prefix}{field}");
            if let Some(value) = settings.values.get(&key) {
                crate::credentials::sign_in::issuer(value.value.as_str().expect("schema URL"))
                    .map_err(|why| input::refuse(settings, &key, why))?;
            }
        }
        let port = format!("{prefix}redirect_port");
        if settings.values.get(&port).is_some_and(|value| value.value.as_integer() == Some(0)) {
            return Err(input::refuse(settings, &port, "expected a port in 1..=65535"));
        }
    }
    let endpoints: Vec<_> = settings
        .values
        .iter()
        .filter(|(key, value)| {
            key.starts_with("endpoints.") && key.ends_with(".preset") && value.value.as_str() == Some("openai")
        })
        .map(|(key, _)| key.trim_end_matches("preset").to_owned())
        .collect();
    for prefix in endpoints {
        let key = format!("{prefix}account");
        if settings.values.contains_key(&key) {
            continue;
        }
        let candidates: Vec<_> = names
            .iter()
            .filter(|name| {
                let prefix = format!("accounts.{}.", super::super::name(name));
                settings
                    .values
                    .get(&format!("{prefix}provider"))
                    .is_some_and(|value| value.value.as_str() == Some("openai"))
                    && settings.values.contains_key(&format!("{prefix}source"))
            })
            .collect();
        if let [name] = candidates.as_slice() {
            input::default(settings, key, toml::Value::String((*name).clone()));
        }
    }
    Ok(())
}

pub(super) fn missing(settings: &Settings, endpoint: &str) -> super::super::Refusal {
    let candidates: Vec<_> = settings
        .values
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("accounts.")
                .and_then(|rest| rest.strip_suffix(".provider"))
                .filter(|_| value.value.as_str() == Some("openai"))
        })
        .collect();
    input::refuse(
        settings,
        &format!("endpoints.{}.account", super::super::name(endpoint)),
        &format!("an account is required; candidates: {}", candidates.join(", ")),
    )
}
