//! Strict product layers and each value's source (shell.md, sections 5.1–5.3).
//! Startup retains only decoded settings and origins; it reads no credentials.
//! `read` merges bounded files, ordered overrides and flags before resolution.
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

pub(crate) mod environment;
mod layers;
#[cfg(test)]
mod providers_tests;
pub mod resolve;
mod schema;
#[cfg(test)]
mod sources_tests;
#[cfg(test)]
mod tests;

pub(crate) use layers::merge;
pub use layers::read;
pub(crate) use layers::read_in;

/// Resolve supplied startup layers through the same strict merge as file startup.
pub fn from_sources(
    shared: &crate::command::Shared,
    workspace: PathBuf,
    sources: &[Source],
) -> Result<Settings, Refusal> {
    merge(shared, workspace, sources)
}

pub(crate) fn name(part: &str) -> String {
    if part.is_empty() || part.contains(['.', '"', '\\']) {
        serde_json::to_string(part).expect("setting name")
    } else {
        part.into()
    }
}

/// Split one encoded setting name from its field without dropping malformed input.
pub(crate) fn named_field(path: &str) -> Result<(String, &str), ()> {
    if path.starts_with('"') {
        let mut parts = serde_json::Deserializer::from_str(path).into_iter::<String>();
        let name = parts.next().ok_or(())?.map_err(|_| ())?;
        let field = path.get(parts.byte_offset()..).and_then(|rest| rest.strip_prefix('.')).ok_or(())?;
        Ok((name, field))
    } else {
        path.split_once('.').map(|(name, field)| (name.into(), field)).ok_or(())
    }
}

/// Startup's supplied path environment (protocol/hosts.md, section 5.2).
#[derive(Clone, Default)]
pub struct Paths {
    pub home: Option<std::ffi::OsString>,
    pub state_home: Option<std::ffi::OsString>,
    pub path: Option<std::ffi::OsString>,
}

impl std::fmt::Debug for Paths {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Paths")
    }
}

/// One file's precedence below command overrides and flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// The person's user configuration.
    User,
    /// The workspace repository's configuration.
    Repository,
}

/// Where a resolved leaf came from, used by check and refusals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// The product's built-in value.
    BuiltIn,
    /// A file layer's exact key location.
    File { layer: Layer, path: PathBuf, line: u32 },
    /// An ordered command override, numbered from one.
    Override { index: u32 },
    /// A named command flag.
    Flag { flag: String },
}

/// A decoded leaf and its surviving origin after tables merge.
#[derive(Clone, Debug, Serialize)]
pub struct Sourced {
    pub value: toml::Value,
    pub origin: Origin,
}

/// One bounded file layer, supplied by startup or a deterministic fixture.
#[derive(Clone, Debug)]
pub struct Source {
    pub layer: Layer,
    pub path: PathBuf,
    pub text: String,
}

/// Decoded leaves by their dotted paths, before run resolution.
#[derive(Clone, Serialize)]
pub struct Settings {
    #[serde(skip)]
    pub paths: Paths,
    pub workspace: PathBuf,
    pub values: BTreeMap<String, Sourced>,
}

impl std::fmt::Debug for Settings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Settings")
    }
}

/// A strict layer refusal, naming the key and the source that supplied it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub origin: Origin,
    pub key: String,
    pub problem: Problem,
}

/// The startup rule a setting or a command tool could not meet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// An input could not be decoded or read; only its diagnostic is retained.
    Input(String),
    /// A leaf has the wrong type or exceeds the representation's bound.
    WrongValue,
    /// The schema has no such key.
    UnknownKey,
    /// A repository supplied authority reserved to the user layer.
    RepositoryAuthority,
    /// A repository raised the user's budget.
    RepositoryBudget,
    /// The commands' PATH contains no executable for this tool.
    MissingTool,
    /// A budget refusal with the requested value and the other side's source.
    Budget(Box<BudgetComparison>),
    /// A native comparison's typed sides, with their setting paths and sources.
    Relationship(Box<Relationship>),
}

/// The local budget's typed failure and both sides' values and sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetComparison {
    pub reason: smith_domain::run::BudgetRefusal,
    pub requested: smith_agent_service::profile::Value,
    pub other: smith_agent_service::profile::Value,
    pub other_key: String,
    pub other_origin: Origin,
    pub profile: String,
}

/// A profile comparison retains its native check and each side's origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relationship {
    pub check: smith_agent_service::profile::Check,
    pub left: smith_agent_service::profile::Value,
    pub right: smith_agent_service::profile::Value,
    pub right_key: String,
    pub right_origin: Origin,
}

impl Refusal {
    /// One operator line without the refused value or credentials.
    #[must_use]
    pub fn line(&self) -> String {
        if let Problem::Budget(comparison) = &self.problem {
            return resolve::relationships::budget_line(self, comparison).replace(['\n', '\r'], "\\n");
        }
        if let Problem::Relationship(comparison) = &self.problem {
            let Relationship { check, left, right, right_key, right_origin } = comparison.as_ref();
            return format!(
                "smith: {}: {} = {} (from {}); {right_key} = {} (from {})",
                check.name(),
                self.key,
                resolve::relationships::value(*left),
                self.origin.description(),
                resolve::relationships::value(*right),
                right_origin.description()
            )
            .replace(['\n', '\r'], "\\n");
        }
        let place = match &self.origin {
            Origin::BuiltIn => "built-in settings".into(),
            Origin::File { path, line, .. } => format!("{}:{line}", path.display()),
            Origin::Override { index } => format!("-c override {index}"),
            Origin::Flag { flag } => flag.clone(),
        };
        let problem = match &self.problem {
            Problem::Input(message) => message.as_str(),
            Problem::WrongValue => "wrong type or value past its representable bound",
            Problem::UnknownKey => "unknown key",
            Problem::RepositoryAuthority => "repository layer cannot set authority",
            Problem::RepositoryBudget => "repository budget exceeds the user's value",
            Problem::MissingTool => "executable is not found in the commands' PATH",
            Problem::Relationship(_) => unreachable!("relationship rendered above"),
            Problem::Budget(_) => unreachable!("budget rendered above"),
        };
        format!("smith: {place}: {}: {problem}", self.key).replace(['\n', '\r'], "\\n")
    }
}

impl Origin {
    /// The surviving source of one value, without using Rust's debug formatting.
    pub(crate) fn description(&self) -> String {
        match self {
            Self::BuiltIn => "built-in".into(),
            Self::File { layer, path, line } => {
                let layer = match layer {
                    Layer::User => "user",
                    Layer::Repository => "repository",
                };
                format!("{layer} {}:{line}", path.display())
            }
            Self::Override { .. } => "-c".into(),
            Self::Flag { flag } => flag.clone(),
        }
    }
}
