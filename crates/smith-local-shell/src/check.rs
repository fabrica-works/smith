//! Startup's check document with value origins; credential values are redacted
//! (shell.md, sections 4.4, 5.3 and 7). It starts no run or sign-in.
use crate::{
    command::{self, Command},
    settings::{self, Origin, Problem, Refusal, Settings},
};
use serde::Serialize;
use std::{collections::BTreeMap, ffi::OsString, os::unix::fs::PermissionsExt, path::PathBuf};

/// The version of check's JSON document, read by callers and the harness.
pub const CHECK_VERSION: u64 = 1;

/// One startup check's document; only names and sources describe environment.
#[derive(Debug, Serialize)]
pub struct Report {
    pub version: u64,
    pub settings: Settings,
    pub formats: BTreeMap<&'static str, u64>,
    pub build: serde_json::Value,
    pub tools: BTreeMap<&'static str, PathBuf>,
    pub environment: Vec<String>,
    pub outside_models: BTreeMap<String, String>,
}

/// Startup either supplies the document or names its configuration problem.
#[derive(Debug)]
pub enum Verdict {
    /// The decoded settings and tools pass this stage's checks.
    Ready(Report),
    /// A setting or tool prevents startup, rendered with exit 2.
    Refused(Refusal),
}

/// Build the document without reading or reporting a credential.
#[must_use]
pub fn report(settings: &Settings) -> Verdict {
    let command = Command::Check { shared: command::Shared::default(), json: false };
    let resolved = match settings::resolve::resolve(settings, &command) {
        Ok(resolved) => resolved,
        Err(why) => return Verdict::Refused(why),
    };
    let settings = &resolved.settings;
    let inherited = settings.values.get("environment.inherit").and_then(|value| value.value.as_array());
    let inherits_path = inherited.is_some_and(|names| names.iter().any(|name| name.as_str() == Some("PATH")));
    let path = settings
        .values
        .get("environment.set.PATH")
        .and_then(|value| value.value.as_str())
        .map(OsString::from)
        .or_else(|| inherits_path.then(|| std::env::var_os("PATH")).flatten());
    let mut tools = BTreeMap::new();
    for name in ["sh", "rg", "git"] {
        let resolved = path.as_ref().and_then(|path| {
            std::env::split_paths(path).map(|directory| settings.workspace.join(directory).join(name)).find(|file| {
                std::fs::metadata(file)
                    .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            })
        });
        let Some(resolved) = resolved else {
            return Verdict::Refused(Refusal {
                origin: Origin::BuiltIn,
                key: format!("tools.{name}"),
                problem: Problem::MissingTool,
            });
        };
        tools.insert(name, resolved);
    }
    let mut environment: Vec<_> = inherited
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .filter(|name| std::env::var_os(name).is_some())
        .map(str::to_owned)
        .collect();
    let mut redacted = settings.clone();
    for (key, value) in &mut redacted.values {
        if let Some(name) = key.strip_prefix("environment.set.") {
            environment.push(serde_json::from_str(name).unwrap_or_else(|_| name.into()));
            value.value = toml::Value::String("<redacted>".into());
        } else if key.starts_with("endpoints.") && key.contains(".headers.") {
            value.value = toml::Value::String("<redacted>".into());
        }
    }
    environment.sort();
    environment.dedup();
    let identity = smith_agent_shell::identity();
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    Verdict::Ready(Report {
        version: CHECK_VERSION,
        settings: redacted,
        formats: BTreeMap::from([
            ("channel", smith_channel::KindRule::FORMAT_VERSION),
            ("charter", smith_charter::Charter::FORMAT_VERSION),
            ("transcript", smith_transcript::Turn::FORMAT_VERSION),
            ("events", smith_events::VERSION),
            ("chat_store", smith_local_protocol::Store::FORMAT_VERSION),
            ("check", CHECK_VERSION),
        ]),
        build: serde_json::json!({"name":text(&identity.name),"version":text(&identity.version),"build":text(&identity.build)}),
        tools,
        environment,
        outside_models: resolved.outside_models.into_iter().map(|(model, reason)| (model, reason.into())).collect(),
    })
}

/// Render the same document for scripts as one JSON object.
#[must_use]
pub fn render_json(report: &Report) -> String {
    serde_json::to_string(report).expect("check document contains JSON values")
}

/// Render settings and their layers with environment values omitted.
#[must_use]
pub fn render_text(report: &Report) -> String {
    let mut lines = Vec::new();
    for (key, value) in &report.settings.values {
        let source = value.origin.description();
        lines.push(format!("{key} = {} (from {source})", value.value));
    }
    lines.push(format!("build {}", report.build));
    lines.push(format!("formats {}", serde_json::to_string(&report.formats).expect("format names")));
    for (name, path) in &report.tools {
        lines.push(format!("{name} = {}", path.display()));
    }
    lines.push(format!("environment: {}", report.environment.join(", ")));
    for (model, reason) in &report.outside_models {
        lines.push(format!("model {model} is outside the run: {reason}"));
    }
    lines.join("\n")
}

/// Handle product arguments, leaving legacy invocations to their existing glue.
#[must_use]
pub fn command(arguments: &[OsString]) -> Option<u8> {
    let (shared, json) = match command::parse(arguments.iter().skip(1).cloned().collect()) {
        Ok(Command::Legacy(_)) => return None,
        Ok(Command::Check { shared, json }) => (shared, json),
        Err(usage) => {
            eprintln!("smith: usage: {}", usage.0);
            return Some(2);
        }
    };
    let verdict = match settings::read(&shared) {
        Ok(settings) => report(&settings),
        Err(why) => Verdict::Refused(why),
    };
    match verdict {
        Verdict::Ready(report) => {
            println!("{}", if json { render_json(&report) } else { render_text(&report) });
            Some(0)
        }
        Verdict::Refused(why) => {
            eprintln!("{}", why.line());
            Some(2)
        }
    }
}
