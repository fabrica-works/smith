//! Suites retain task selections, pins and budgets, never live attempt state.
//! `validate_suite` resolves references; `choose_guards` selects affordable
//! guarded tasks from estimates until summaries supply medians
//! (benchmarks.md, sections 4.2 and 5.3).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Agent, Kind, ModelTiers, Provider, Refusal, Task, check_task, read_configuration};

/// The attempt ordering requested by the suite author.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Design {
    /// Each task runs once per repetition and configuration.
    Single,
    /// Each block runs all arms in an order drawn from its seed.
    Interleaved,
}

/// A catalogue selector supplied by the suite author.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub kind: Kind,
    #[serde(default)]
    pub behaviours: Vec<String>,
}

/// A pinned agent configuration named by the suite author.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SuiteAgent {
    pub agent: Agent,
    pub provider: Provider,
    pub config: String,
    /// Empty selects every suite task; otherwise these exact tasks and variants.
    #[serde(default)]
    pub tasks: Vec<String>,
}

/// A comparison arm supplied by the suite author to the runner.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "source", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Arm {
    /// A smith binary built from a recorded commit.
    Binary { name: String, commit: String },
    /// A smith settings override compared with the suite's base settings.
    Override { name: String, smith: BTreeMap<String, toml::Value> },
    /// Another pinned agent configuration compared on the same task.
    Agent { name: String, agent: Agent, provider: Provider, config: String },
}

pub(crate) fn override_arguments(fields: &BTreeMap<String, toml::Value>) -> Vec<String> {
    let mut arguments = Vec::new();
    for (key, value) in fields {
        arguments.extend(["-c".into(), format!("{key}={value}")]);
    }
    arguments
}

/// A committed run specification supplied by its author to the runner.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    pub name: String,
    pub tier: String,
    pub design: Design,
    #[serde(default)]
    pub reference: Option<String>,
    /// Compare two smith builds supplied at invocation (benchmarks.md, 16.6).
    #[serde(default)]
    pub change: bool,
    pub repetitions: u32,
    /// An invocation may replace every task's attempts per arm.
    #[serde(default)]
    pub attempts: Option<u32>,
    /// Suite-authored counts for exact task and variant names (benchmarks.md, 5.3).
    #[serde(default)]
    pub repetitions_by_task: BTreeMap<String, u32>,
    /// Zero or one rerun after a failed attempt.
    pub rerun_failed: u32,
    pub max_wall_seconds: u64,
    pub max_tokens: u64,
    #[serde(default)]
    pub tasks: Vec<String>,
    /// Tasks that run only when a run names them with `--include` (benchmarks.md, 5.3).
    #[serde(default)]
    pub when_asked: Vec<String>,
    #[serde(default)]
    pub select: Vec<Selection>,
    pub agents: Vec<SuiteAgent>,
    #[serde(default, rename = "arm")]
    pub arms: Vec<Arm>,
}

impl Suite {
    /// The reference selected by the suite, including a change suite’s parent.
    #[must_use]
    pub fn reference_arm(&self) -> Option<&str> {
        if self.change { Some("parent") } else { self.reference.as_deref() }
    }

    pub(crate) fn task_repetitions(&self, name: &str, authored: Option<u32>, logical: bool) -> Result<u32, String> {
        if let Some(attempts) = self.attempts {
            return Ok(attempts);
        }
        let repetitions = self.repetitions_by_task.get(name).copied().unwrap_or(self.repetitions);
        if logical {
            repetitions.checked_mul(authored.unwrap_or(1)).ok_or_else(|| "probe repetition count overflow".into())
        } else {
            Ok(self.repetitions_by_task.get(name).copied().or(authored).unwrap_or(repetitions))
        }
    }
}

/// A validated task and its catalogue name, supplied to suite resolution.
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogueTask {
    pub name: String,
    pub manifest: PathBuf,
    pub task: Task,
}

/// Affordable guarded task names and the tasks omitted from the budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardSelection {
    pub selected: Vec<String>,
    pub omitted: Vec<(String, String)>,
    pub estimated_seconds: u64,
    pub estimated_tokens: u64,
}

/// Read a suite strictly; resolving its references is a separate offline check.
pub fn read_suite(file: &Path) -> Result<Suite, Refusal> {
    crate::formats::read_document(file)
}

/// Read frozen tasks from a catalogue and give each its kind/id name.
pub fn catalogue(tasks: &Path, design: &Path) -> Result<Vec<CatalogueTask>, Refusal> {
    let mut manifests = Vec::new();
    crate::check::find_manifests(tasks, &mut manifests)?;
    manifests.sort();
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    for manifest in manifests {
        let task = check_task(&manifest, design)?;
        let kind = match task.kind {
            Kind::Probe => "probes",
            Kind::Fixture => "fixtures",
            Kind::Repository => "repository",
        };
        let name = format!("{kind}/{}", task.id);
        if !names.insert(name.clone()) {
            return Err(Refusal::new(&manifest, "id", format!("duplicate catalogue task {name}")));
        }
        entries.push(CatalogueTask { name, manifest, task });
    }
    Ok(entries)
}

/// Validate a suite's quantities, task names, selectors, model tiers and pins.
pub fn validate_suite(
    file: &Path,
    suite: &Suite,
    tasks: &[CatalogueTask],
    agents: &Path,
    models: &ModelTiers,
) -> Result<Vec<String>, Refusal> {
    if suite.name.is_empty()
        || suite.repetitions == 0
        || suite.attempts == Some(0)
        || suite.rerun_failed > 1
        || suite.max_wall_seconds == 0
        || suite.max_tokens == 0
        || suite.agents.is_empty()
    {
        return Err(Refusal::new(
            file,
            "suite",
            "name, agents and positive budgets/repetitions are required; at most one failure rerun",
        ));
    }
    validate_change(file, suite)?;
    let mut selected = BTreeSet::new();
    for name in &suite.tasks {
        if !task_exists(name, tasks) {
            return Err(Refusal::new(file, "tasks", format!("unknown task or variant {name}")));
        }
        selected.insert(name.clone());
    }
    for name in &suite.when_asked {
        if !task_exists(name, tasks) || suite.tasks.contains(name) {
            return Err(Refusal::new(file, "when_asked", format!("unknown or default task or variant {name}")));
        }
    }
    for selector in &suite.select {
        let matches: Vec<&CatalogueTask> = tasks
            .iter()
            .filter(|entry| {
                entry.task.kind == selector.kind
                    && selector.behaviours.iter().all(|behaviour| entry.task.behaviours.contains(behaviour))
            })
            .collect();
        if matches.is_empty() {
            return Err(Refusal::new(file, "select", "selector names no existing task"));
        }
        for task in matches {
            if !suite.when_asked.contains(&task.name) {
                selected.insert(task.name.clone());
            }
        }
    }
    if selected.is_empty() {
        return Err(Refusal::new(file, "tasks/select", "suite must name an existing task"));
    }
    validate_repetitions(file, suite, &selected)?;
    for agent in &suite.agents {
        if agent.tasks.iter().any(|name| !selected.contains(name)) {
            return Err(Refusal::new(file, "agents.tasks", "agent task must be selected by the suite"));
        }
        validate_agent(file, agent, &suite.tier, agents, models)?;
    }
    let mut arm_names = BTreeSet::new();
    if suite.arms.iter().any(|arm| matches!(arm, Arm::Override { .. }))
        && (suite.agents.len() != 1
            || suite.agents[0].agent != Agent::Smith
            || suite.arms.iter().any(|arm| !matches!(arm, Arm::Override { .. })))
    {
        return Err(Refusal::new(file, "arm", "override arms require one smith agent and only override arms"));
    }
    for arm in &suite.arms {
        let name = match arm {
            Arm::Binary { name, commit } => {
                if commit.is_empty() {
                    return Err(Refusal::new(file, "arm.commit", "binary arm needs a commit"));
                }
                name
            }
            Arm::Override { name, smith: _ } => name,
            Arm::Agent { name, agent, provider, config } => {
                validate_agent(
                    file,
                    &SuiteAgent { agent: *agent, provider: *provider, config: config.clone(), tasks: Vec::new() },
                    &suite.tier,
                    agents,
                    models,
                )?;
                name
            }
        };
        if name.is_empty() || !arm_names.insert(name) {
            return Err(Refusal::new(file, "arm.name", "arm names must be nonempty and unique"));
        }
    }
    let has_codex = suite.agents.iter().any(|agent| agent.agent == Agent::Codex)
        || suite.arms.iter().any(|arm| matches!(arm, Arm::Agent { agent: Agent::Codex, .. }));
    if suite.design == Design::Interleaved && !has_codex && !suite.change {
        let Some(reference) = &suite.reference else {
            return Err(Refusal::new(file, "reference", "a comparison without Codex must name its reference arm"));
        };
        if !arm_names.contains(reference) {
            return Err(Refusal::new(file, "reference", format!("unknown reference arm {reference}")));
        }
    }
    Ok(selected.into_iter().collect())
}

fn validate_change(file: &Path, suite: &Suite) -> Result<(), Refusal> {
    if suite.change
        && (!suite.arms.is_empty()
            || suite.reference.is_some()
            || suite.agents.len() != 1
            || suite.agents[0].agent != Agent::Smith
            || suite.design != Design::Interleaved
            || suite.rerun_failed != 0)
    {
        return Err(Refusal::new(
            file,
            "change",
            "change suites require one smith agent, interleaving, no arms, reference or rerun",
        ));
    }
    Ok(())
}

fn validate_repetitions(file: &Path, suite: &Suite, selected: &BTreeSet<String>) -> Result<(), Refusal> {
    for (name, repetitions) in &suite.repetitions_by_task {
        if *repetitions == 0 || !(selected.contains(name) || suite.when_asked.contains(name)) {
            return Err(Refusal::new(file, "repetitions_by_task", "positive counts must name suite tasks"));
        }
    }
    Ok(())
}

/// The default selection plus asked tasks; names outside `when_asked` are refused.
pub fn include(file: &Path, suite: &Suite, selected: &[String], asked: &[String]) -> Result<Vec<String>, Refusal> {
    let mut names: BTreeSet<String> = selected.iter().cloned().collect();
    for name in asked {
        if !suite.when_asked.contains(name) {
            return Err(Refusal::new(file, "include", format!("task is not under when_asked: {name}")));
        }
        names.insert(name.clone());
    }
    Ok(names.into_iter().collect())
}

fn validate_agent(
    file: &Path,
    agent: &SuiteAgent,
    tier: &str,
    agents: &Path,
    models: &ModelTiers,
) -> Result<(), Refusal> {
    crate::check::check_path(file, "agents.config", Path::new(&agent.config))?;
    if agent.config.contains('/') {
        return Err(Refusal::new(file, "agents.config", "configuration is a name within the agent directory"));
    }
    let pin_file = agents.join(agent.agent.directory()).join(format!("{}.pin.toml", agent.config));
    let pin = read_configuration(&pin_file)?;
    if pin.pin.agent != agent.agent || pin.pin.provider != agent.provider || pin.pin.name != agent.config {
        return Err(Refusal::new(file, "agents", "suite agent/provider/name disagrees with its configuration pin"));
    }
    models.lookup(&agents.join("models.toml"), tier, agent.provider)?;
    Ok(())
}

fn task_exists(name: &str, tasks: &[CatalogueTask]) -> bool {
    tasks.iter().any(|entry| {
        if name == entry.name {
            return true;
        }
        let Some(variant) = name.strip_prefix(&format!("{}/", entry.name)) else {
            return false;
        };
        entry.task.variant.iter().any(|entry| entry.name == variant)
    })
}

/// Select guarded tasks cheapest first within the suite's aggregate budgets.
/// Missing estimates are omitted with a reason, never treated as zero cost.
#[must_use]
pub fn choose_guards(suite: &Suite, tasks: &[CatalogueTask], sections: &[String]) -> GuardSelection {
    choose_guards_with_costs(suite, tasks, sections, &BTreeMap::new())
}

/// Select guards using matching committed medians before task estimates.
#[must_use]
pub fn choose_guards_with_costs(
    suite: &Suite,
    tasks: &[CatalogueTask],
    sections: &[String],
    costs: &BTreeMap<String, crate::Estimate>,
) -> GuardSelection {
    let mut candidates = Vec::new();
    let mut selection =
        GuardSelection { selected: Vec::new(), omitted: Vec::new(), estimated_seconds: 0, estimated_tokens: 0 };
    let arms = u64::try_from(suite.arms.len().max(1)).expect("arm count fits u64");
    for entry in tasks {
        if !entry.task.guards.iter().any(|guard| sections.contains(guard)) {
            continue;
        }
        let names = std::iter::once(entry.name.clone())
            .chain(entry.task.variant.iter().map(|variant| format!("{}/{}", entry.name, variant.name)));
        for name in names {
            let repetitions = u64::from(suite.task_repetitions(&name, entry.task.repetitions, false).expect("count"));
            let agents =
                suite.agents.iter().filter(|agent| agent.tasks.is_empty() || agent.tasks.contains(&name)).count();
            let agents = u64::try_from(agents).expect("agent count fits u64");
            let multiplier = repetitions
                .checked_mul(u64::from(suite.rerun_failed) + 1)
                .and_then(|count| count.checked_mul(agents))
                .and_then(|count| count.checked_mul(arms));
            let Some(estimate) = costs.get(&name).or(entry.task.estimate.as_ref()) else {
                selection.omitted.push((name, "cost unavailable".into()));
                continue;
            };
            let cost = multiplier
                .and_then(|count| Some((estimate.seconds.checked_mul(count)?, estimate.tokens.checked_mul(count)?)));
            if let Some((seconds, tokens)) = cost {
                candidates.push((seconds, tokens, name));
            } else {
                selection.omitted.push((name, "cost overflow".into()));
            }
        }
    }
    candidates.sort();
    for (seconds, tokens, name) in candidates {
        let wall = selection.estimated_seconds.checked_add(seconds);
        let tokens = selection.estimated_tokens.checked_add(tokens);
        if let Some(wall) = wall
            && let Some(tokens) = tokens
            && wall <= suite.max_wall_seconds
            && tokens <= suite.max_tokens
        {
            selection.selected.push(name);
            selection.estimated_seconds = wall;
            selection.estimated_tokens = tokens;
        } else {
            selection.omitted.push((name, "suite wall or token budget".into()));
        }
    }
    selection
}
