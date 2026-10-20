//! Program paths and source bindings (protocol/hosts.md, sections 5.2 and 5.4).
//! Resolution opens no login and runs no program; `locate` uses supplied paths.
use super::super::{Refusal, Settings};
use super::input;
use crate::credentials::Directory;
use serde_json::{Value, json};
use std::{ffi::OsStr, os::unix::fs::PermissionsExt, path::PathBuf};

pub(crate) fn locate(configured: Option<&str>, path: Option<&OsStr>) -> Result<PathBuf, ()> {
    let executable = |path: &PathBuf| {
        path.is_absolute()
            && std::fs::metadata(path)
                .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    };
    match configured {
        Some(value) => {
            let path = PathBuf::from(value);
            if executable(&path) { Ok(path) } else { Err(()) }
        }
        None => std::env::split_paths(path.ok_or(())?).map(|base| base.join("claude")).find(executable).ok_or(()),
    }
}

pub(super) fn check_sources(settings: &Settings) -> Result<(), Refusal> {
    if let Some(value) = settings.values.get("credentials")
        && !std::path::Path::new(value.value.as_str().expect("schema path")).is_absolute()
    {
        return Err(input::refuse(settings, "credentials", "expected an absolute credentials directory"));
    }
    for (key, value) in &settings.values {
        let Some(endpoint) = key.strip_prefix("endpoints.").and_then(|rest| rest.strip_suffix(".account")) else {
            continue;
        };
        let endpoint_name = serde_json::from_str::<String>(endpoint).unwrap_or_else(|_| endpoint.into());
        if super::super::name(&endpoint_name) != endpoint {
            continue;
        }
        let prefix = key.strip_suffix("account").expect("endpoint account suffix");
        let account = value.value.as_str().expect("schema account");
        let source = format!("accounts.{}.source", super::super::name(account));
        if settings.values.get(&source).is_some_and(|field| field.value.as_str() == Some("program"))
            && !settings.values.contains_key(&format!("{prefix}program"))
        {
            return Err(input::refuse(
                settings,
                &source,
                &format!("accounts.{account} uses a program source but {prefix}program is absent"),
            ));
        }
    }
    Ok(())
}

pub(super) fn document(settings: &Settings, endpoints: &[Value]) -> Result<Option<Value>, Refusal> {
    if !endpoints.iter().any(|endpoint| endpoint.get("program").is_some()) {
        return Ok(None);
    }
    let directory = Directory::of(settings).ok_or_else(|| {
        input::refuse(settings, "credentials", "a program requires an absolute credentials directory")
    })?;
    let configured = settings.values.get("programs.claude_code").and_then(|field| field.value.as_str());
    let program = locate(configured, settings.paths.path.as_deref()).map_err(|()| {
        input::refuse(settings, "programs.claude_code", "expected an absolute executable file, or claude on PATH")
    })?;
    let path =
        program.to_str().ok_or_else(|| input::refuse(settings, "programs.claude_code", "expected a UTF-8 path"))?;
    let programs = directory.programs();
    let directory = programs.to_str().ok_or_else(|| input::refuse(settings, "credentials", "expected a UTF-8 path"))?;
    Ok(Some(json!({"claude_code":{"path":path,"directory":directory}})))
}
