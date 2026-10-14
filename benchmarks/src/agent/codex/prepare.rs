//! Codex preparation retains pinned overrides and private credential paths.
//! It never writes to the source login. The command's settings are
//! explicit; its observer retains only root stream evidence until collection.
//! Contracts: benchmarks.md, sections 6, 8.2, 10 and 14.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::agent::{Context, HarnessFault, Invocation, Seen};
use crate::isolate::{Setup, SetupStep};
use crate::{Agent, ModelChoice, Provider};

use super::{Item, Observation, Observer};

/// The pinned standalone Codex invocation, supplied to the common supervisor.
#[derive(Debug)]
pub struct Prepared {
    pub overrides: Vec<(String, String)>,
    pub credential: PathBuf,
    pub expires: skein_lib::Wall,
    pub borrowed: Option<crate::credential::Borrowed>,
    pub home: PathBuf,
    pub final_message: PathBuf,
    pub model: ModelChoice,
    pub program: PathBuf,
    pub workspace: PathBuf,
    pub teardown: skein_lib::Duration,
}

/// The observer and exact configuration selected for this attempt.
#[derive(Debug)]
pub struct Observing {
    pub prepared: Box<Prepared>,
    pub observer: Observer,
    pub collected: Option<super::Rollouts>,
    pub collection_error: Option<String>,
}

/// Prepare a version-matched Codex command without touching its existing login.
pub fn prepare(context: &Context<'_>) -> Result<Prepared, Setup> {
    let configuration = (|| {
        if !context.task.person.is_empty() {
            return Err("Codex does not accept later person lines".into());
        }
        if context.pin.pin.agent != Agent::Codex || context.pin.pin.provider != Provider::Codex {
            return Err("Codex requires its own codex configuration pin".into());
        }
        let installed =
            context.installed_version.trim().strip_prefix("codex-cli ").unwrap_or(context.installed_version.trim());
        if installed != context.pin.pin.version {
            return Err(format!("installed Codex {installed} differs from pinned {}", context.pin.pin.version));
        }
        if context.model.model.as_ref().is_none_or(String::is_empty)
            || context.model.effort.as_ref().is_none_or(String::is_empty)
        {
            return Err("Codex model and effort must be explicitly selected".into());
        }
        let program = match context.arm {
            crate::arm::ArmRef::Installed { program } => program.clone(),
            crate::arm::ArmRef::Built(_) => {
                return Err("Codex requires its installed executable, not a smith build".into());
            }
        };
        let reread = crate::read_configuration(context.pin_file).map_err(|error| error.to_string())?;
        if reread != *context.pin {
            return Err("Codex configuration changed since the attempt was planned".into());
        }
        let digest = crate::arm::binary_digest(&program)?;
        if Some(digest.as_str()) != context.pin.pin.sha256.as_deref() {
            return Err(format!("Codex executable digest {digest} differs from its pin"));
        }
        let overrides_path = context
            .pin_file
            .parent()
            .ok_or("configuration pin needs a directory")?
            .join(&context.pin.pin.configuration);
        let bytes = std::fs::read(&overrides_path).map_err(|error| error.to_string())?;
        let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
        let document: std::collections::BTreeMap<String, toml::Value> =
            toml::from_str(text).map_err(|error| error.to_string())?;
        let milliseconds = document
            .get("harness")
            .and_then(|value| value.get("teardown_ms"))
            .and_then(toml::Value::as_integer)
            .ok_or("Codex pin requires harness.teardown_ms")?;
        let nanos = u64::try_from(milliseconds)
            .ok()
            .and_then(|value| value.checked_mul(1_000_000))
            .filter(|value| *value > 0)
            .ok_or("Codex teardown must fit a positive duration")?;
        let teardown = skein_lib::Duration::from_nanos(nanos);
        let mut overrides: Vec<_> = document
            .into_iter()
            .filter(|(key, _)| key != "harness")
            .map(|(key, value)| (key, value.to_string()))
            .collect();
        let workspace = crate::isolate::working_directory(context.guard, context.private, context.task)?;
        // Codex splits dotted override paths literally; quote the dynamic path
        // within a top-level TOML table value, including dots in the workspace.
        let project = toml::Table::from_iter([("trust_level".into(), toml::Value::String("trusted".into()))]);
        let projects = toml::Table::from_iter([(workspace.display().to_string(), toml::Value::Table(project))]);
        overrides.push(("projects".into(), toml::Value::Table(projects).to_string()));
        let writable = [&context.private.cargo_home, &context.private.target, &context.private.temporary]
            .into_iter()
            .map(|path| toml::Value::String(path.display().to_string()))
            .collect();
        overrides.push(("sandbox_workspace_write.writable_roots".into(), toml::Value::Array(writable).to_string()));
        overrides.push((
            "model_reasoning_effort".into(),
            toml::Value::String(context.model.effort.clone().expect("validated effort")).to_string(),
        ));
        Ok(Prepared {
            overrides,
            credential: PathBuf::new(),
            expires: skein_lib::Wall::EPOCH,
            borrowed: None,
            home: context.private.root.join("codex"),
            final_message: context.private.root.join("final.txt"),
            model: context.model.clone(),
            program,
            workspace,
            teardown,
        })
    })();
    let mut prepared = configuration.map_err(|reason| Setup { step: SetupStep::Configuration, reason })?;
    let borrowed = crate::credential::codex(
        context.guard,
        context.home,
        &prepared.home,
        context.wall,
        context.lifetime.checked_add(prepared.teardown).ok_or_else(|| Setup {
            step: SetupStep::Configuration,
            reason: "credential teardown lifetime overflow".into(),
        })?,
    )?;
    prepared.credential.clone_from(&borrowed.credential);
    prepared.expires = borrowed.expires;
    prepared.borrowed = Some(borrowed);
    Ok(prepared)
}

impl Prepared {
    /// Render the standalone JSON-lines command with exact pinned settings.
    #[must_use]
    pub fn command(&self) -> Invocation {
        let mut arguments: Vec<OsString> =
            ["--no-daemon", "exec", "--json", "-m"].into_iter().map(Into::into).collect();
        arguments.push(self.model.model.as_ref().expect("prepared model").into());
        for (key, value) in &self.overrides {
            arguments.extend(["-c".into(), format!("{key}={value}").into()]);
        }
        arguments.extend([
            "-C".into(),
            self.workspace.as_os_str().into(),
            "-o".into(),
            self.final_message.as_os_str().into(),
            "-".into(),
        ]);
        Invocation {
            program: self.program.clone(),
            arguments,
            environment: vec![("CODEX_HOME".into(), self.home.as_os_str().into())],
            mode: skein_world::end_to_end::Mode::Pipes,
            teardown: self.teardown,
        }
    }
}

impl Observing {
    /// Observe known root records once, returning neutral timing cues.
    pub fn feed_line(&mut self, line: &str) -> Result<Vec<Seen>, HarnessFault> {
        let answered = self.observer.answer().is_some();
        let turns = self.observer.usages.len();
        let record = self.observer.observe_line(line).map_err(|error| HarnessFault { what: error.to_string() })?;
        let mut seen = Vec::new();
        match record {
            Some(Observation::TurnCompleted { .. }) => {
                if self.observer.usages.len() != turns {
                    seen.push(Seen::Response);
                }
            }
            Some(Observation::ItemStarted { item }) => match item {
                Item::CommandExecution { .. } => seen.push(Seen::ToolCall { tool: "command_execution".into() }),
                Item::FileChange { .. } => seen.push(Seen::ToolCall { tool: "file_change".into() }),
                Item::CollabToolCall { tool, .. } => {
                    if tool == "spawn_agent" {
                        seen.push(Seen::Child);
                    }
                    seen.push(Seen::ToolCall { tool });
                }
                Item::AgentMessage { .. } => {}
            },
            Some(
                Observation::ThreadStarted { .. }
                | Observation::TurnStarted
                | Observation::ItemUpdated { .. }
                | Observation::ItemCompleted { .. }
                | Observation::TurnFailed { .. }
                | Observation::Error { .. },
            )
            | None => {}
        }
        if !answered && self.observer.answer().is_some() {
            seen.push(Seen::Answer);
        }
        Ok(seen)
    }

    /// Read only this completed attempt's native root and child records.
    pub fn collect(&mut self) -> Result<&super::Rollouts, HarnessFault> {
        if let Some(what) = &self.collection_error {
            return Err(HarnessFault { what: what.clone() });
        }
        if self.collected.is_none() {
            let thread =
                self.observer.thread().ok_or_else(|| HarnessFault { what: "Codex stream has no named root".into() })?;
            let mut children = Vec::new();
            for item in self.observer.completed_items() {
                if let Item::CollabToolCall { tool, receiver_thread_ids, .. } = item
                    && tool == "spawn_agent"
                {
                    children.extend(receiver_thread_ids.iter().cloned());
                }
            }
            children.sort();
            children.dedup();
            self.collected = Some(super::Rollouts { responses: Vec::new(), children: Vec::new(), files: Vec::new() });
            if let Err(error) = super::rollouts::collect_named(
                &self.prepared.home,
                thread,
                &children,
                self.collected.as_mut().expect("raw collection retained"),
            ) {
                let what = error.to_string();
                self.collection_error = Some(what.clone());
                return Err(HarnessFault { what });
            }
        }
        Ok(self.collected.as_ref().expect("collected above"))
    }
}
