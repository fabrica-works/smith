//! Named command variables and build caches (shell.md, section 8). Values feed
//! the agent's existing environment encoder; check reports names and origins.
use super::{Origin, Problem, Refusal, Settings};
use std::{collections::BTreeMap, path::PathBuf};

pub(crate) fn values(settings: &Settings) -> Result<BTreeMap<String, String>, Refusal> {
    let mut values = BTreeMap::new();
    let inherited = settings.values.get("environment.inherit").expect("built-in inheritance");
    let names = inherited.value.as_array().expect("schema names");
    for name in names {
        let name = name.as_str().expect("schema name");
        if !valid_name(name) {
            return Err(refusal(settings, "environment.inherit", "expected a variable name, never a pattern"));
        }
        inherit(&mut values, name, settings)?;
    }
    if let Some(preset) = settings.values.get("build.preset") {
        let homes = crate::presets::build_homes(preset.value.as_str().expect("schema preset"))
            .ok_or_else(|| refusal(settings, "build.preset", "unknown build preset"))?;
        for name in homes {
            inherit(&mut values, name, settings)?;
        }
    }
    for (key, value) in &settings.values {
        if let Some(name) = key.strip_prefix("environment.set.") {
            let name = serde_json::from_str::<String>(name).unwrap_or_else(|_| name.into());
            if !valid_name(&name) {
                return Err(refusal(settings, key, "expected a variable name, never a pattern"));
            }
            values.insert(name, value.value.as_str().expect("schema value").into());
        }
    }
    Ok(values)
}

fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn inherit(values: &mut BTreeMap<String, String>, name: &str, settings: &Settings) -> Result<(), Refusal> {
    let value = match name {
        "HOME" => settings.paths.home.clone(),
        "XDG_STATE_HOME" => settings.paths.state_home.clone(),
        "PATH" => settings.paths.path.clone(),
        _ => std::env::var_os(name),
    };
    if let Some(value) = value {
        let value = value
            .into_string()
            .map_err(|_| refusal(settings, "environment.inherit", "inherited value is not UTF-8"))?;
        values.insert(name.into(), value);
    }
    Ok(())
}

pub(crate) fn caches(settings: &Settings, values: &BTreeMap<String, String>) -> Vec<PathBuf> {
    let mut caches = Vec::new();
    if settings.values.contains_key("build.preset") {
        let home = values
            .get("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| values.get("HOME").map(|home| PathBuf::from(home).join(".cargo")));
        if let Some(home) = home {
            for directory in ["registry", "git"] {
                caches.push(settings.workspace.join(&home).join(directory));
            }
        }
    }
    if let Some(paths) = settings.values.get("build.caches") {
        for path in paths.value.as_array().expect("schema paths") {
            caches.push(settings.workspace.join(path.as_str().expect("schema path")));
        }
    }
    caches.sort();
    caches.dedup();
    caches
}

pub(crate) fn origin(settings: &Settings, name: &str) -> Origin {
    if let Some(value) = settings.values.get(&format!("environment.set.{}", super::name(name))) {
        return value.origin.clone();
    }
    let inherited = settings.values.get("environment.inherit").expect("built-in inheritance");
    if inherited.value.as_array().expect("schema names").iter().any(|value| value.as_str() == Some(name)) {
        return inherited.origin.clone();
    }
    settings.values.get("build.preset").map_or(Origin::BuiltIn, |value| value.origin.clone())
}

fn refusal(settings: &Settings, key: &str, reason: &str) -> Refusal {
    Refusal {
        origin: settings.values.get(key).map_or(Origin::BuiltIn, |value| value.origin.clone()),
        key: key.into(),
        problem: Problem::Input(reason.into()),
    }
}
