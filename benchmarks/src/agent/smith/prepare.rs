//! Preparation writes guarded native settings and a converted access-only record.
//! The verified arm and append-only face registry choose the codec; user login
//! bytes are read only by credential.rs (benchmarks.md, sections 6.2 and 10).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::agent::{Context, Invocation};
use crate::isolate::{Setup, SetupStep};

use super::{CredentialShape, Face, FaceSince, faces, settings};

/// The private native invocation and its immutable outside face.
#[derive(Debug)]
pub struct Prepared {
    pub face: Face,
    pub settings: PathBuf,
    pub credential: Credential,
    pub trace: Option<PathBuf>,
    pub registry: FaceSince,
    pub rendered: serde_json::Value,
    pub program: PathBuf,
    pub state: PathBuf,
    pub teardown: skein_lib::Duration,
}

/// The attempt's credential, removed only after the process tree settles.
#[derive(Debug)]
pub enum Credential {
    /// A private converted record with no usable refresh grant.
    Record(Option<crate::credential::Borrowed>),
}

fn configuration(reason: impl Into<String>) -> Setup {
    Setup { step: SetupStep::Configuration, reason: reason.into() }
}

fn write(context: &Context<'_>, path: &Path, bytes: &[u8]) -> Result<(), Setup> {
    let result: Result<(), String> = (|| {
        let path = context.guard.write_path(path)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).and_then(|()| file.sync_all()).map_err(|error| error.to_string())
    })();
    result.map_err(configuration)
}

/// Render the selected native arm before borrowing its existing source login.
pub fn prepare(context: &Context<'_>) -> Result<Prepared, Setup> {
    if context.pin.pin.agent != crate::Agent::Smith
        || context.pin.pin.provider != crate::Provider::Codex
        || context.pin.pin.version != "built-arm"
    {
        return Err(configuration("Smith requires the codex built-arm configuration pin"));
    }
    let build = match context.arm {
        crate::arm::ArmRef::Built(build) => build,
        crate::arm::ArmRef::Installed { .. } => return Err(configuration("Smith requires a verified built arm")),
    };
    let program = build.directory.join("smith");
    let verified = crate::arm::read_build(&program).map_err(configuration)?;
    if verified != *build || context.installed_version != build.arm.commit {
        return Err(configuration("Smith arm differs from its planned full source identity"));
    }
    let pin = crate::read_configuration(context.pin_file).map_err(|error| configuration(error.to_string()))?;
    if pin != *context.pin {
        return Err(configuration("Smith configuration changed since planning"));
    }
    let directory = context.pin_file.parent().ok_or_else(|| configuration("Smith pin needs a directory"))?;
    let repository = directory
        .ancestors()
        .find(|path| path.join(".git").exists())
        .ok_or_else(|| configuration("Smith pin needs its source repository for arm ancestry"))?;
    let registry = faces::face(&build.arm, &faces::read(&directory.join("faces.toml"))?, repository)?;
    let shape = match registry.face {
        Face::Legacy(shape) => shape,
        Face::Events(super::events::Shape::LocalTrace) => super::legacy::Shape::LocalReserved,
        Face::Events(super::events::Shape::Exec | super::events::Shape::Chat) => {
            return Err(configuration("this event front end awaits its shipped product invocation"));
        }
    };
    let bytes = fs::read(directory.join(&pin.pin.configuration)).map_err(|error| configuration(error.to_string()))?;
    if bytes.len() > 65_536 {
        return Err(configuration("Smith settings template exceeds its bound"));
    }
    let base = serde_json::from_slice(&bytes).map_err(|error| configuration(error.to_string()))?;
    let root = context.private.root.join("smith");
    crate::run::private_parent(context.guard, &root).map_err(configuration)?;
    let tokens = root.join("tokens");
    let trace = root.join("trace.jsonl");
    let settings_path = root.join("settings.json");
    let dry = settings::settings(context, shape, &base, "prepared-account", &trace, &tokens)?;
    let teardown = settings::teardown(shape, &dry)?;
    let lifetime =
        context.lifetime.checked_add(teardown).ok_or_else(|| configuration("credential lifetime overflow"))?;
    let CredentialShape::Converted(token_shape) = registry.credential;
    let converted =
        crate::credential::smith(context.guard, context.home, &tokens, context.wall, lifetime, token_shape)?;
    let rendered = settings::settings(context, shape, &base, &converted.account_id, &trace, &tokens)?;
    let encoded = serde_json::to_vec_pretty(&rendered).map_err(|error| configuration(error.to_string()))?;
    write(context, &settings_path, &encoded)?;
    write(context, &trace, &[])?;
    let state = root.join("state");
    crate::run::private_parent(context.guard, &state).map_err(configuration)?;
    Ok(Prepared {
        face: registry.face,
        settings: settings_path,
        credential: Credential::Record(Some(converted.borrowed)),
        trace: Some(trace),
        registry,
        rendered,
        program,
        state,
        teardown,
    })
}

impl Prepared {
    /// Invoke the frozen native local front end with its own private state.
    #[must_use]
    pub fn command(&self) -> Invocation {
        Invocation {
            program: self.program.clone(),
            arguments: vec![
                "local".into(),
                self.settings.clone().into_os_string(),
                self.state.clone().into_os_string(),
            ],
            environment: Vec::new(),
            mode: skein_world::end_to_end::Mode::Terminal,
            teardown: self.teardown,
        }
    }
}
