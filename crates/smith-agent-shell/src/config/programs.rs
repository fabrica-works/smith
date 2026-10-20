//! Program declarations and endpoint bindings (protocol/agent.md, section 4).
//! Keeps paths and adapter names only; it never opens or runs a program.

use std::{collections::BTreeMap, path::PathBuf};

use serde::Deserialize;
use smith_agent_service::profile::{Layer, LimitName, Per, Refusal};

use super::{ConfigurationRefusal, Endpoint};

/// A program the agent may run as a backend (protocol/agent.md, section 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgramName {
    /// Claude Code, selected by the configuration for program sources.
    ClaudeCode,
}

/// A program's executable and directory of source configurations (programs.md, 4.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub path: PathBuf,
    pub directory: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProgramDocument {
    path: Option<String>,
    directory: Option<String>,
}

fn name(value: &str, field: &str) -> Result<ProgramName, ConfigurationRefusal> {
    match value {
        "claude_code" => Ok(ProgramName::ClaudeCode),
        _ => Err(format!("{field}: unknown program {value}").into()),
    }
}

fn path(value: Option<String>, field: &'static str) -> Result<PathBuf, ConfigurationRefusal> {
    let value = value.ok_or_else(|| ConfigurationRefusal::Input(format!("{field}: required path")))?;
    if value.len() > 4096 || value.contains('\0') || !std::path::Path::new(&value).is_absolute() {
        return Err(format!("{field}: must be an absolute path of at most 4096 bytes without NUL").into());
    }
    Ok(value.into())
}

pub(super) fn build(
    documents: BTreeMap<String, ProgramDocument>,
) -> Result<BTreeMap<ProgramName, Program>, ConfigurationRefusal> {
    let mut programs = BTreeMap::new();
    for (key, document) in documents {
        let name = name(&key, &format!("programs.{key}"))?;
        programs.insert(
            name,
            Program {
                path: path(document.path, "programs.claude_code.path")?,
                directory: path(document.directory, "programs.claude_code.directory")?,
            },
        );
    }
    Ok(programs)
}

#[expect(
    clippy::zero_sized_map_values,
    reason = "Endpoint selections retain the adapter name; Claude Code is currently the sole adapter."
)]
pub(super) fn endpoints(
    endpoints: &[Endpoint],
    programs: &BTreeMap<ProgramName, Program>,
) -> Result<BTreeMap<u32, ProgramName>, ConfigurationRefusal> {
    let mut selections = BTreeMap::new();
    for endpoint in endpoints {
        match &endpoint.program {
            None => {}
            Some(value) => {
                let program = name(value, &format!("endpoints[{}].program", endpoint.number))?;
                if !programs.contains_key(&program) {
                    return Err(ConfigurationRefusal::Profile(Refusal::Field {
                        limit: LimitName {
                            layer: Layer::Declared,
                            field: "program",
                            per: Per::Endpoint(endpoint.number),
                        },
                    }));
                }
                selections.insert(endpoint.number, program);
            }
        }
    }
    Ok(selections)
}
