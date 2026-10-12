//! Wrapper configuration sources and name-only credential warnings (shell.md, 8).
use super::Warning;
use crate::settings::{Origin, Problem, Refusal, Settings, environment};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(crate) fn collect(
    settings: &Settings,
    values: &BTreeMap<String, String>,
    caches: &[PathBuf],
) -> Result<Vec<Warning>, Refusal> {
    let mut warnings = Vec::new();
    let mut writable = Vec::new();
    for cache in caches {
        let reason = match skein_shell::directory_access(cache) {
            skein_shell::DirectoryAccess::Writable => {
                writable.push(cache.clone());
                continue;
            }
            skein_shell::DirectoryAccess::Absent => "absent".into(),
            skein_shell::DirectoryAccess::NotDirectory => "not a directory".into(),
            skein_shell::DirectoryAccess::Denied(errno) => std::io::Error::from_raw_os_error(errno).to_string(),
        };
        warnings.push(Warning::CacheAccess { cache: cache.clone(), reason });
    }
    for name in values.keys() {
        let upper = name.to_ascii_uppercase();
        if ["TOKEN", "PASSWORD", "PASSWD", "SECRET", "CREDENTIAL", "API_KEY", "PRIVATE_KEY"]
            .iter()
            .any(|part| upper.contains(part))
        {
            warnings
                .push(Warning::CredentialLikeName { name: name.clone(), origin: environment::origin(settings, name) });
        }
    }
    if !settings.values.contains_key("build.preset") {
        return Ok(warnings);
    }
    let mut wrappers = BTreeMap::<String, (String, String, Origin)>::new();
    let home = values
        .get("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| values.get("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    if let Some(home) = home {
        read_wrappers(&settings.workspace.join(home), &mut wrappers)?;
    }
    for ancestor in settings.workspace.ancestors().collect::<Vec<_>>().into_iter().rev() {
        read_wrappers(&ancestor.join(".cargo"), &mut wrappers)?;
    }
    for (key, names) in [
        ("rustc-wrapper", ["CARGO_BUILD_RUSTC_WRAPPER", "RUSTC_WRAPPER"]),
        ("rustc-workspace-wrapper", ["CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"]),
    ] {
        for name in names {
            if let Some(value) = values.get(name) {
                wrappers.insert(key.into(), (value.clone(), name.into(), environment::origin(settings, name)));
            }
        }
    }
    for (_, (wrapper, configured_by, origin)) in wrappers {
        if wrapper.is_empty() {
            continue;
        }
        let cache = wrapper_cache(&wrapper, values).map(|path| settings.workspace.join(path));
        if cache.as_ref().is_none_or(|path| !writable.contains(path)) {
            warnings.push(Warning::WrapperCache { configured_by, origin, cache });
        }
    }
    Ok(warnings)
}

fn read_wrappers(directory: &Path, wrappers: &mut BTreeMap<String, (String, String, Origin)>) -> Result<(), Refusal> {
    for name in ["config", "config.toml"] {
        let path = directory.join(name);
        let bytes = match skein_shell::read_file(&path, 1 << 20) {
            Ok(bytes) => bytes,
            Err(skein_shell::Unread::Absent) => continue,
            Err(_) => {
                return Err(Refusal {
                    origin: Origin::BuiltIn,
                    key: path.display().to_string(),
                    problem: Problem::Input("could not read Rust wrapper configuration".into()),
                });
            }
        };
        let text = std::str::from_utf8(&bytes).map_err(|_| config_refusal(&path))?;
        let document: toml::Value = toml::from_str(text).map_err(|_| config_refusal(&path))?;
        for key in ["rustc-wrapper", "rustc-workspace-wrapper"] {
            if let Some(value) = document.get("build").and_then(|build| build.get(key)) {
                let wrapper = value.as_str().ok_or_else(|| config_refusal(&path))?;
                wrappers.insert(key.into(), (wrapper.into(), path.display().to_string(), Origin::BuiltIn));
            }
        }
        // Cargo prefers config over config.toml when both exist.
        break;
    }
    Ok(())
}

fn config_refusal(path: &Path) -> Refusal {
    Refusal {
        origin: Origin::BuiltIn,
        key: path.display().to_string(),
        problem: Problem::Input("expected TOML Rust wrapper configuration".into()),
    }
}

fn wrapper_cache(wrapper: &str, values: &BTreeMap<String, String>) -> Option<PathBuf> {
    let name = Path::new(wrapper).file_name()?.to_str()?;
    let variable = match name {
        "sccache" => "SCCACHE_DIR",
        "ccache" => "CCACHE_DIR",
        _ => return None,
    };
    values
        .get(variable)
        .map(PathBuf::from)
        .or_else(|| values.get("XDG_CACHE_HOME").map(|home| PathBuf::from(home).join(name)))
        .or_else(|| values.get("HOME").map(|home| PathBuf::from(home).join(".cache").join(name)))
}

pub(crate) fn line(warning: &Warning) -> String {
    match warning {
        Warning::Credential { account, credential } => format!("warning: {}", credential.line(account)),
        Warning::CacheAccess { cache, reason } => format!("warning: cache {}: {reason}", cache.display()),
        Warning::WrapperCache { configured_by, origin, cache } => format!(
            "warning: wrapper from {configured_by} (from {}) has no listed writable cache{}",
            origin.description(),
            cache.as_ref().map_or_else(String::new, |path| format!(": {}", path.display()))
        ),
        Warning::CredentialLikeName { name, origin } => {
            format!("warning: credential-like environment name {name} (from {})", origin.description())
        }
    }
}
