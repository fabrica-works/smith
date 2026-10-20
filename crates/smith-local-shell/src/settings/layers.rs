//! Bounded startup reads and strict key-by-key precedence (shell.md, 5.1–5.3).
use super::{Layer, Origin, Problem, Refusal, Settings, Source, Sourced, schema};
use crate::command::Shared;
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};
use toml::{
    Value,
    de::{DeTable, DeValue},
};

const FILE_BYTES: u32 = 1 << 20;
const BUILT_IN: &str = "profile='standard'\nchat='default'\nsub_agents=false\nwaiting_seconds=900\n[budget]\nspend=25.0\nseconds=3600\nturns=512\n[budget.reserve]\nturns=2\nseconds=300\n[environment]\ninherit=['PATH','HOME','USER','LOGNAME','LANG','LC_ALL','LC_CTYPE','TZ','TERM','TMPDIR']\n";

/// Read the startup layers, ordered overrides and flags without opening a run.
pub fn read(shared: &Shared) -> Result<Settings, Refusal> {
    read_in(shared, &std::env::vars_os().collect::<Vec<_>>())
}

pub(crate) fn read_in(
    shared: &Shared,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
) -> Result<Settings, Refusal> {
    let variable = |name: &str| environment.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone());
    let current = std::env::current_dir().map_err(|why| refusal(Origin::BuiltIn, "workspace", &why.to_string()))?;
    let workspace = shared.directory.as_ref().map_or_else(|| current.clone(), |path| current.join(path));
    let path_environment =
        super::Paths { home: variable("HOME"), state_home: variable("XDG_STATE_HOME"), path: variable("PATH") };
    let user = variable("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| path_environment.home.as_ref().map(|home| PathBuf::from(home).join(".config")));
    let mut sources = Vec::new();
    let mut root = workspace.clone();
    for ancestor in workspace.ancestors() {
        if ancestor.join(".git").exists() {
            root = ancestor.into();
            break;
        }
    }
    let paths = user
        .into_iter()
        .map(|path| (Layer::User, path.join("smith/config.toml")))
        .chain(std::iter::once((Layer::Repository, root.join(".smith/config.toml"))));
    for (layer, path) in paths {
        let origin = Origin::File { layer, path: path.clone(), line: 1 };
        let bytes = match skein_shell::read_file(&path, FILE_BYTES) {
            Ok(bytes) => bytes,
            Err(skein_shell::Unread::Absent) => continue,
            Err(skein_shell::Unread::NotAFile) => return Err(refusal(origin, "settings", "expected a regular file")),
            Err(skein_shell::Unread::TooLarge { .. }) => return Err(refusal(origin, "settings", "file exceeds 1 MiB")),
            Err(skein_shell::Unread::Unsafe(_) | skein_shell::Unread::Kernel(_)) => {
                return Err(refusal(origin, "settings", "kernel could not read settings"));
            }
        };
        let text = String::from_utf8(bytes.into_vec()).map_err(|_| refusal(origin, "settings", "expected UTF-8"))?;
        sources.push(Source { layer, path, text });
    }
    let mut shared = shared.clone();
    for dir in &mut shared.dirs {
        dir.path = current.join(&dir.path);
    }
    let mut settings = merge(&shared, workspace, &sources)?;
    settings.paths = path_environment;
    Ok(settings)
}

pub(crate) fn merge(shared: &Shared, workspace: PathBuf, sources: &[Source]) -> Result<Settings, Refusal> {
    let mut settings = Settings { paths: super::Paths::default(), workspace, values: BTreeMap::new() };
    document(&mut settings, BUILT_IN, &Origin::BuiltIn)?;
    for preset in [crate::presets::Preset::OpenAi, crate::presets::Preset::Codex, crate::presets::Preset::Anthropic] {
        document(&mut settings, crate::presets::built_in(preset), &Origin::BuiltIn)?;
    }
    for source in sources {
        let origin = Origin::File { layer: source.layer, path: source.path.clone(), line: 1 };
        document(&mut settings, &source.text, &origin)?;
    }
    for (index, text) in shared.overrides.iter().enumerate() {
        let origin = Origin::Override {
            index: u32::try_from(index).expect("arguments fit u32").checked_add(1).expect("argument index fits"),
        };
        let (key, raw) =
            text.split_once('=').ok_or_else(|| refusal(origin.clone(), "override", "expected KEY=VALUE"))?;
        DeValue::parse(raw).map_err(|why| refusal(origin.clone(), key, why.message()))?;
        document(&mut settings, text, &origin)?;
    }
    for (key, flag, value) in [
        ("model", "-m", shared.model.as_ref()),
        ("effort", "--effort", shared.effort.as_ref()),
        ("chat", "--chat", shared.chat.as_ref()),
    ] {
        if let Some(value) = value {
            let value = Value::String(value.clone());
            let origin = Origin::Flag { flag: flag.into() };
            schema::check(&value, schema::kind(&[key.into()]).expect("named flag")).map_err(|problem| Refusal {
                origin: origin.clone(),
                key: key.into(),
                problem,
            })?;
            settings.values.insert(key.into(), Sourced { value, origin });
        }
    }
    if let Some(path) = &shared.trace {
        let text = path
            .to_str()
            .ok_or_else(|| refusal(Origin::Flag { flag: "--trace".into() }, "trace.path", "expected UTF-8 path"))?;
        let value = Value::String(text.into());
        let origin = Origin::Flag { flag: "--trace".into() };
        schema::check(&value, schema::kind(&["trace".into(), "path".into()]).expect("trace flag"))
            .map_err(|problem| Refusal { origin: origin.clone(), key: "trace.path".into(), problem })?;
        settings.values.insert("trace.path".into(), Sourced { value, origin });
    }
    if !shared.dirs.is_empty() {
        let origin = Origin::Flag { flag: "--dir".into() };
        let mut rows = Vec::new();
        for dir in &shared.dirs {
            let path = dir
                .path
                .to_str()
                .ok_or_else(|| refusal(origin.clone(), "workspace.directories", "expected UTF-8 path"))?;
            rows.push(Value::Table(toml::Table::from_iter([
                ("name".into(), Value::String(dir.name.clone())),
                ("path".into(), Value::String(path.into())),
                ("writable".into(), Value::Boolean(dir.writable)),
            ])));
        }
        let value = Value::Array(rows);
        schema::check(&value, schema::kind(&["workspace".into(), "directories".into()]).expect("dir flag"))
            .map_err(|problem| Refusal { origin: origin.clone(), key: "workspace.directories".into(), problem })?;
        settings.values.insert("workspace.directories".into(), Sourced { value, origin });
    }
    crate::presets::apply_prices(&mut settings);
    Ok(settings)
}

fn document(settings: &mut Settings, text: &str, origin: &Origin) -> Result<(), Refusal> {
    if text.len() > usize::try_from(FILE_BYTES).expect("file bound fits") {
        return Err(refusal(origin.clone(), "settings", "file exceeds 1 MiB"));
    }
    let spanned = DeTable::parse(text)
        .map_err(|why| refusal(at(origin, text, why.span().map_or(0, |span| span.start)), "settings", why.message()))?;
    let decoded = toml::Table::deserialize(toml::de::Deserializer::from(spanned.clone())).expect("already parsed TOML");
    leaves(settings, text, origin, spanned.get_ref(), &Value::Table(decoded), &mut Vec::new())
}

fn leaves(
    settings: &mut Settings,
    text: &str,
    origin: &Origin,
    table: &DeTable<'_>,
    decoded: &Value,
    path: &mut Vec<String>,
) -> Result<(), Refusal> {
    for (key, spanned) in table {
        path.push(key.get_ref().to_string());
        let named = path.iter().map(|part| super::name(part)).collect::<Vec<_>>().join(".");
        let location = at(origin, text, key.span().start);
        if let Origin::File { layer: Layer::Repository, .. } = &location
            && schema::authority(path)
        {
            return Err(Refusal { origin: location, key: named, problem: Problem::RepositoryAuthority });
        }
        let kind = schema::kind(path).ok_or_else(|| Refusal {
            origin: location.clone(),
            key: named.clone(),
            problem: Problem::UnknownKey,
        })?;
        let value = decoded.get(key.get_ref().as_ref()).expect("same parsed table");
        schema::check(value, kind).map_err(|problem| Refusal {
            origin: location.clone(),
            key: named.clone(),
            problem,
        })?;
        if let Origin::File { layer: Layer::Repository, .. } = &location
            && path.first().is_some_and(|part| part == "budget")
            && !value.is_table()
        {
            let above = settings
                .values
                .get(&named)
                .is_some_and(|user| number(value).zip(number(&user.value)).is_some_and(|(new, prior)| new > prior));
            if above {
                return Err(Refusal { origin: location, key: named, problem: Problem::RepositoryBudget });
            }
        }
        if let DeValue::Array(array) = spanned.get_ref() {
            for (index, item) in array.iter().enumerate() {
                if let DeValue::Table(fields) = item.get_ref() {
                    let row = value.as_array().expect("same array").get(index).expect("same row");
                    for (field, _) in fields {
                        let key = field.get_ref().as_ref();
                        let full = format!("{named}[{index}].{key}");
                        let place = at(origin, text, field.span().start);
                        let expected = schema::field(kind, key).ok_or_else(|| Refusal {
                            origin: place.clone(),
                            key: full.clone(),
                            problem: Problem::UnknownKey,
                        })?;
                        schema::check(row.get(key).expect("same field"), expected).map_err(|problem| Refusal {
                            origin: place,
                            key: full,
                            problem,
                        })?;
                    }
                }
            }
        }
        if let DeValue::Table(child) = spanned.get_ref() {
            leaves(settings, text, origin, child, value, path)?;
        } else {
            let prefix = format!("{named}.");
            // Descendants form one ordered range; unrelated settings need no visit.
            while let Some((key, _)) = settings.values.range(prefix.clone()..).next() {
                if !key.starts_with(&prefix) {
                    break;
                }
                let key = key.clone();
                drop(settings.values.remove(&key));
            }
            settings.values.insert(named, Sourced { value: value.clone(), origin: location });
        }
        drop(path.pop());
    }
    Ok(())
}

fn at(origin: &Origin, text: &str, offset: usize) -> Origin {
    match origin {
        Origin::File { layer, path, .. } => Origin::File {
            layer: *layer,
            path: path.clone(),
            line: u32::try_from(
                text.get(..offset).expect("parser byte span").bytes().filter(|byte| *byte == b'\n').count(),
            )
            .expect("bounded lines")
            .checked_add(1)
            .expect("bounded line"),
        },
        Origin::BuiltIn | Origin::Override { .. } | Origin::Flag { .. } => origin.clone(),
    }
}

fn refusal(origin: Origin, key: &str, problem: &str) -> Refusal {
    Refusal { origin, key: key.into(), problem: Problem::Input(problem.into()) }
}

fn number(value: &Value) -> Option<f64> {
    value.as_float().or_else(|| value.as_integer().and_then(|n| n.to_string().parse().ok()))
}
