//! Agent identities and pinned configuration records; no login is read here.
//! `read_configuration` checks the pin and hashes its metadata and exact
//! configuration bytes (benchmarks.md, sections 5.4, 6 and 10).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Refusal;

pub mod codex;

pub mod smith;

mod person;

pub use person::{Close, Cue, Script, Trigger};

pub(crate) use person::Person;

/// The frozen inputs an adapter prepares inside one private attempt.
#[derive(Debug)]
pub struct Context<'a> {
    pub guard: &'a crate::guard::Guard,
    pub private: &'a crate::isolate::Private,
    pub pin: &'a PinnedConfiguration,
    /// The pin's file, resolving its relative configuration document.
    pub pin_file: &'a Path,
    pub task: &'a crate::Task,
    pub variant: Option<&'a crate::Variant>,
    pub model: &'a crate::ModelChoice,
    pub arm: &'a crate::arm::ArmRef,
    /// Observed outside the attempt timer from the selected executable.
    pub installed_version: &'a str,
    /// Harness account directory: smith credentials or the source Codex home.
    pub home: &'a Path,
    pub wall: skein_lib::Wall,
    /// Hard deadline plus the grace and settlement spans; adapter adds teardown.
    pub lifetime: skein_lib::Duration,
    /// The runner's exact allow-listed environment, also used by Smith tools.
    pub environment: &'a [(std::ffi::OsString, std::ffi::OsString)],
}

/// The adapter's prepared invocation; unsupported agents refuse before this value.
#[derive(Debug)]
pub enum Prepared {
    /// Codex's pinned standalone command, in an attempt-owned home.
    Codex(codex::Prepared),
    /// Smith's verified built arm and native outside face.
    Smith(smith::Prepared),
}

/// The invocation supplied to the common process supervisor.
#[derive(Debug)]
pub struct Invocation {
    pub program: PathBuf,
    pub arguments: Vec<std::ffi::OsString>,
    pub environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    pub mode: skein_world::end_to_end::Mode,
    pub teardown: skein_lib::Duration,
}

/// A neutral stream observation for the runner's clock and scripted person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seen {
    /// A provider response or reported turn ended.
    Response,
    /// A named tool call started.
    ToolCall { tool: String },
    /// A child thread was opened.
    Child,
    /// The accepted answer ends task wall time and begins teardown.
    Answer,
}

/// An observer stays paired with the preparation that selected its format.
#[derive(Debug)]
pub enum Observing {
    /// Codex's root JSON stream and subsequent read-only collection.
    Codex(codex::Observing),
    /// Smith's native trace, read as the process writes it.
    Smith(smith::Observing),
}

/// A stream or collection fault, naming its persisted field or file.
#[derive(Debug)]
pub struct HarnessFault {
    pub what: String,
}

impl Agent {
    /// Prepare a supported adapter; missing faces or versions refuse as setup.
    pub fn prepare(self, context: &Context<'_>) -> Result<Prepared, crate::isolate::Setup> {
        match self {
            Self::Codex => codex::prepare(context).map(Prepared::Codex),
            Self::Smith => smith::prepare(context).map(Prepared::Smith),
            Self::ClaudeCode => Err(crate::isolate::Setup {
                step: crate::isolate::SetupStep::Configuration,
                reason: "Claude Code belongs to the next comparison pass".into(),
            }),
        }
    }
}

impl Prepared {
    /// Build the command with its pinned configuration overrides.
    #[must_use]
    pub fn command(&self) -> Invocation {
        match self {
            Self::Codex(prepared) => prepared.command(),
            Self::Smith(prepared) => prepared.command(),
        }
    }

    /// Move preparation into its matching observer before execution.
    pub fn observe(self) -> Result<Observing, HarnessFault> {
        match self {
            Self::Codex(prepared) => Ok(Observing::Codex(codex::Observing {
                prepared: Box::new(prepared),
                observer: codex::Observer::default(),
                collected: None,
                collection_error: None,
            })),
            Self::Smith(prepared) => smith::Observing::new(prepared).map(Observing::Smith),
        }
    }
}

impl Observing {
    /// Feed one complete line; known malformed records fault before classification.
    pub fn feed_line(&mut self, line: &str) -> Result<Vec<Seen>, HarnessFault> {
        match self {
            Self::Codex(observing) => observing.feed_line(line),
            Self::Smith(observing) => observing.feed_line(line),
        }
    }

    /// The actual root final message, where the selected stream retains it.
    #[must_use]
    pub fn answer(&self) -> Option<String> {
        match self {
            Self::Codex(observing) => observing.observer.answer().map(str::to_owned),
            Self::Smith(observing) => observing
                .observer
                .sessions()
                .last()?
                .records
                .iter()
                .rev()
                .find_map(|record| {
                    if let smith_events::Event::RunCompleted(value) = &record.event {
                        Some(
                            value
                                .result
                                .as_ref()
                                .and_then(|result| result.text.as_deref())
                                .and_then(|text| std::str::from_utf8(text).ok())
                                .map(str::to_owned),
                        )
                    } else {
                        None
                    }
                })
                .flatten(),
        }
    }

    /// Classify from the selected face, its accepted answer and genuine tree exit.
    #[must_use]
    pub fn classify(&self, exit: crate::Exit, forced: crate::Forced) -> crate::Classification {
        match self {
            Self::Codex(observing) => observing.observer.classify(exit, forced),
            Self::Smith(observing) => observing.observer.classify(exit, forced),
        }
    }
}

/// The adapter selected by a suite author for each attempt.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    /// smith, observed through the face its binary offers.
    Smith,
    /// Codex, observed through JSON lines and its named rollouts.
    Codex,
    /// Standalone Claude Code identity; probes select the smith program arm.
    ClaudeCode,
}

impl Agent {
    /// The directory holding this adapter's pinned configurations.
    #[must_use]
    pub const fn directory(self) -> &'static str {
        match self {
            Self::Smith => "smith",
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
        }
    }
}

/// The provider route selected by a suite author.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    /// The `OpenAI` model tier table, retained under its codex name.
    Codex,
    /// The Anthropic route.
    Anthropic,
}

/// A committed configuration pin supplied to one agent adapter.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub name: String,
    pub agent: Agent,
    pub provider: Provider,
    pub version: String,
    /// A smith pin may require a source ancestor for its account policy.
    pub minimum_commit: Option<String>,
    /// The private package executable's digest, required for Codex.
    pub sha256: Option<String>,
    /// Codex's whole package, relative to the harness state root.
    pub package: Option<PathBuf>,
    pub configuration: PathBuf,
}

/// The validated pin and digest carried into an attempt's identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinnedConfiguration {
    pub pin: Configuration,
    pub sha256: String,
}

/// Read a committed pin without reading a user's home or credentials.
pub fn read_configuration(file: &Path) -> Result<PinnedConfiguration, Refusal> {
    let pin: Configuration = crate::formats::read_document(file)?;
    if pin.name.is_empty() || pin.version.is_empty() {
        return Err(Refusal::new(file, "name/version", "configuration name and agent version must be pinned"));
    }
    crate::check::check_path(file, "configuration", &pin.configuration)?;
    match pin.agent {
        Agent::Smith => {
            if let Some(commit) = &pin.minimum_commit
                && (commit.len() != 40
                    || !commit.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
            {
                return Err(Refusal::new(file, "minimum_commit", "expected a full lowercase source commit"));
            }
            if pin.name == "smith-claude-code" && pin.provider != Provider::Anthropic {
                return Err(Refusal::new(file, "provider", "smith-claude-code requires the anthropic tier"));
            }
        }
        Agent::Codex => {
            if pin.provider != Provider::Codex {
                return Err(Refusal::new(file, "provider", "Codex requires its codex route"));
            }
            let digest = pin
                .sha256
                .as_deref()
                .ok_or_else(|| Refusal::new(file, "sha256", "Codex requires an executable digest"))?;
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(Refusal::new(file, "sha256", "expected 64 lowercase hexadecimal digits"));
            }
            let package = pin
                .package
                .as_ref()
                .ok_or_else(|| Refusal::new(file, "package", "Codex requires a private package"))?;
            crate::check::check_path(file, "package", package)?;
        }
        Agent::ClaudeCode => {
            if pin.provider != Provider::Anthropic {
                return Err(Refusal::new(file, "provider", "Claude Code requires its anthropic route"));
            }
        }
    }
    let directory = file.parent().ok_or_else(|| Refusal::new(file, "configuration", "pin needs a directory"))?;
    let metadata = std::fs::read(file).map_err(|error| Refusal::new(file, "document", error.to_string()))?;
    let configuration = directory.join(&pin.configuration);
    let config_metadata = std::fs::symlink_metadata(&configuration)
        .map_err(|error| Refusal::new(file, "configuration", error.to_string()))?;
    if !config_metadata.is_file() || config_metadata.is_symlink() {
        return Err(Refusal::new(file, "configuration", "expected a committed regular configuration file"));
    }
    let bytes =
        std::fs::read(&configuration).map_err(|error| Refusal::new(file, "configuration", error.to_string()))?;
    let mut digest = Sha256::new();
    digest.update(u64::try_from(metadata.len()).expect("pin fits u64").to_be_bytes());
    digest.update(metadata);
    digest.update(u64::try_from(bytes.len()).expect("configuration fits u64").to_be_bytes());
    digest.update(bytes);
    Ok(PinnedConfiguration { pin, sha256: format!("{:x}", digest.finalize()) })
}
