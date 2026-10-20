//! Commands prepare frozen requests and print settled evidence; the shared
//! dispatcher owns execution and judging (benchmarks.md, sections 5.1 and 11.5).

use std::fmt::{self, Display};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::arm::ArmRef;
use crate::guard::Guard;
use crate::{Agent, AttemptResult, End, Identity, Judgment, Measure, ModelIdentity, Suite};

pub mod gating;

/// The committed source selected by the person running a command.
#[derive(Clone, Debug)]
pub enum Source {
    /// The clean checkout's committed tip.
    Head,
    /// An explicitly named source commit.
    Commit(String),
}

/// One probe command's task, agent, attempt ceiling and committed source.
#[derive(Debug)]
pub struct Order {
    pub probe: String,
    pub agent: Agent,
    pub attempts: u32,
    pub source: Source,
}

/// Immutable requests prepared outside the checkout for the dispatcher.
#[derive(Debug)]
pub struct Planned {
    pub run: String,
    pub suite: PathBuf,
    pub directory: PathBuf,
    pub requests: Vec<crate::run::Prepared>,
}

/// The settled attempt's printable outcome and measured wall and spend.
#[derive(Debug)]
pub struct Line<'a>(&'a AttemptResult);

fn verdict_name(verdict: Judgment) -> &'static str {
    match verdict {
        Judgment::Pass => "pass",
        Judgment::Fail => "fail",
        Judgment::Incomplete => "incomplete",
    }
}

impl Display for Line<'_> {
    fn fmt(&self, text: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = self.0;
        let end = match result.outcome.end {
            End::Completed => "completed",
            End::Failed { .. } => "failed",
            End::Budget { .. } => "budget",
            End::Refused { .. } => "refused",
            End::Timeout => "timeout",
            End::HarnessError { .. } => "harness-error",
        };
        write!(
            text,
            "{}/{} end={end} verdict={} wall=",
            result.identity.run,
            result.identity.attempt,
            verdict_name(crate::judge(result))
        )?;
        match result.timing.task_wall_ms {
            Measure::Observed { value } => write!(text, "{}.{:03}s", value / 1000, value % 1000)?,
            Measure::LowerBound { .. } | Measure::Unavailable { .. } => text.write_str("-")?,
        }
        text.write_str(" spend=")?;
        match result.spend.usd {
            Measure::Observed { value } => write!(text, "${value:.6}"),
            Measure::LowerBound { .. } | Measure::Unavailable { .. } => text.write_str("-"),
        }
    }
}

/// Retain unavailable measurements while preparing one settled line.
#[must_use]
pub fn line(result: &AttemptResult) -> Line<'_> {
    Line(result)
}

/// Calibration counts supplied by settled logical attempts.
#[derive(Debug, PartialEq, Eq)]
pub struct Counted {
    pub passed: u32,
    pub failed: u32,
    pub deviations: u32,
    pub unjudged: u32,
    pub required: u32,
    pub ceiling: u32,
}

impl Counted {
    /// Enough passes arrived within the command's attempt ceiling.
    #[must_use]
    pub const fn calibrated(&self) -> bool {
        self.passed >= self.required
            && self.failed == 0
            && self.unjudged == 0
            && self.passed + self.deviations <= self.ceiling
    }

    /// Remaining attempts can still attain the required number of passes.
    #[must_use]
    pub fn reachable(&self) -> bool {
        self.failed == 0
            && self.unjudged == 0
            && self.ceiling.saturating_sub(self.deviations) >= self.required
            && self.passed + self.failed + self.deviations + self.unjudged < self.ceiling
    }
}

fn physical_size(results: &[AttemptResult]) -> usize {
    if results.first().is_some_and(crate::cache::selected) {
        3
    } else if results.first().is_some_and(|result| result.identity.task == "probes/credentials") {
        2
    } else {
        1
    }
}

fn calibration_attempts(results: &[AttemptResult], run: &str) -> (u32, Vec<String>, Vec<String>) {
    let mut passes = Vec::new();
    let mut deviations = Vec::new();
    let physical = physical_size(results);
    for attempt in results.chunks(physical) {
        if attempt.len() != physical {
            continue;
        }
        let id = format!("{run}/{}", attempt[0].identity.attempt);
        if crate::judge_suite(attempt, false) == Judgment::Pass {
            passes.push(id);
        } else if attempt.iter().all(model_deviation) {
            deviations.push(id);
        }
    }
    (u32::try_from(passes.len() + deviations.len()).expect("bounded calibration attempts"), passes, deviations)
}

/// Count each logical attempt, including cache triplets, without passing missing evidence.
#[must_use]
pub fn count(results: &[AttemptResult], required: u32, ceiling: u32) -> Counted {
    let mut counted = Counted { passed: 0, failed: 0, deviations: 0, unjudged: 0, required, ceiling };
    let physical = physical_size(results);
    for attempt in results.chunks(physical) {
        if attempt.len() != physical {
            counted.unjudged += 1;
            continue;
        }
        match crate::judge_suite(attempt, false) {
            Judgment::Pass => counted.passed += 1,
            Judgment::Fail | Judgment::Incomplete if attempt.iter().all(model_deviation) => counted.deviations += 1,
            Judgment::Fail if attempt.iter().all(|result| deviation(result, true)) => counted.unjudged += 1,
            Judgment::Fail => counted.failed += 1,
            Judgment::Incomplete => counted.unjudged += 1,
        }
    }
    counted
}

/// A completed answer may deviate from the task while smith's contracts hold.
#[must_use]
pub fn model_deviation(result: &AttemptResult) -> bool {
    deviation(result, false)
}

fn deviation(result: &AttemptResult, permit_unavailable: bool) -> bool {
    if result.outcome.end != End::Completed {
        return false;
    }
    if !permit_unavailable
        && result.outcome.deciding.iter().any(|name| !result.outcome.checks.iter().any(|check| check.check == *name))
    {
        return false;
    }
    let mut deviation = false;
    for check in &result.outcome.checks {
        if !result.outcome.deciding.contains(&check.check) || check.passed == (Measure::Observed { value: true }) {
            continue;
        }
        if permit_unavailable && matches!(&check.passed, Measure::Unavailable { .. }) {
            continue;
        }
        let model = check.check == "answer"
            || check.check == "call-count"
            || check.check == "budget-report"
            || ["file-content:", "file-digest:", "pattern:"].iter().any(|prefix| check.check.starts_with(prefix))
            || matches!(&check.passed, Measure::Unavailable { reason } if reason.starts_with("not exercised:"));
        if !model {
            return false;
        }
        deviation = true;
    }
    deviation
}

fn write(guard: &Guard, path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(guard.write_path(path)?)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())
}

fn utc() -> Result<(String, String), String> {
    let wall = skein_shell::Clock::new().now().wall;
    let time = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(wall.as_nanos()))
        .map_err(|error| error.to_string())?;
    let started = time
        .replace_nanosecond(0)
        .map_err(|error| error.to_string())?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|error| error.to_string())?;
    Ok((started.replace(['-', ':'], ""), started))
}

fn cpus() -> Result<Vec<u32>, String> {
    let status = fs::read_to_string("/proc/self/status").map_err(|error| error.to_string())?;
    let affinity =
        status.lines().find_map(|line| line.strip_prefix("Cpus_allowed_list:")).ok_or("missing CPU affinity")?;
    let mut cpus = Vec::new();
    for range in affinity.trim().split(',') {
        let (first, last) = range.split_once('-').unwrap_or((range, range));
        cpus.extend(
            first.parse::<u32>().map_err(|error| error.to_string())?
                ..=last.parse::<u32>().map_err(|error| error.to_string())?,
        );
    }
    Ok(cpus)
}

fn task(entries: &[crate::CatalogueTask], name: &str) -> Result<crate::CatalogueTask, String> {
    for entry in entries {
        let mut selected = entry.clone();
        if entry.name == name {
            return Ok(selected);
        }
        if let Some(variant) = name.strip_prefix(&format!("{}/", entry.name)) {
            selected.name = name.into();
            selected.task = entry.task.with_variant(variant)?;
            return Ok(selected);
        }
    }
    Err(format!("unknown task: {name}"))
}

fn request(
    repository: &Path,
    entry: &crate::CatalogueTask,
    suite: &Suite,
    agent: &crate::SuiteAgent,
    arm: &ArmRef,
    run: (&str, &str, Option<crate::SmithFace>, &str),
    attempt: usize,
) -> Result<crate::run::Prepared, String> {
    let root = repository.join("benchmarks");
    let task = &entry.task;
    let pin_file = root.join("agents").join(agent.agent.directory()).join(format!("{}.pin.toml", agent.config));
    let pin = crate::read_configuration(&pin_file).map_err(|error| error.to_string())?;
    let models_file = root.join("agents/models.toml");
    let models = crate::read_model_tiers(&models_file).map_err(|error| error.to_string())?;
    let model = models.lookup(&models_file, &suite.tier, agent.provider).map_err(|error| error.to_string())?;
    let (program, version, digest) = match arm {
        ArmRef::Built(build) => (build.directory.join("smith"), build.arm.commit.clone(), build.arm.sha256.clone()),
        ArmRef::Installed { program } => {
            (program.clone(), pin.pin.version.clone(), crate::arm::binary_digest(program)?)
        }
    };
    Ok(crate::run::Prepared {
        task: entry.manifest.clone(),
        program,
        arguments: Vec::new(),
        configuration: Some(pin_file),
        codex_home: Some(crate::arm::state_root(&Guard::from_environment()?)?.join("logins/codex")),
        identity: Identity {
            run: run.0.into(),
            attempt: format!("{attempt:02}"),
            arm: if agent.config == "smith-claude-code" {
                format!("smith-claude-code-{}", &digest[..8])
            } else {
                format!("{}-{}-{}", agent.agent.directory(), agent.config, &digest[..8])
            },
            task: entry.name.clone(),
            task_version: task.version,
            seed_sha256: task.seed_sha256.clone(),
            prompt_sha256: format!("{:x}", Sha256::digest(task.prompt.as_bytes())),
            agent: agent.agent,
            agent_version: version,
            binary_sha256: digest,
            smith_face: if agent.agent == Agent::Smith { run.2 } else { None },
            configuration_sha256: pin.sha256,
            user_configuration_sha256: Measure::unavailable("user configuration is isolated"),
            provider: agent.provider,
            model: ModelIdentity {
                requested_model: task
                    .requested_model(agent.agent, model.model.as_deref().ok_or("unresolved tier")?)?
                    .into(),
                requested_effort: task.effort.clone().or_else(|| model.effort.clone()).ok_or("unresolved effort")?,
                resolved_model: Measure::unavailable("not run"),
                resolved_window: Measure::unavailable("not run"),
                resolved_effort: Measure::unavailable("not run"),
            },
            harness_commit: run.3.into(),
            host: fs::read_to_string("/proc/sys/kernel/hostname").map_err(|error| error.to_string())?.trim().into(),
            cpus: cpus()?,
            started_utc: run.1.into(),
        },
        teardown_ms: 10_000,
        grace_ms: 1_000,
        settlement_ms: 3_000,
        setup_deadline_seconds: 900,
        max_stdout_bytes: 2 * 1024 * 1024,
        max_stderr_bytes: 2 * 1024 * 1024,
        disk_estimate_bytes: 16 * 1024 * 1024,
        force_walk: true,
    })
}

fn task_requests(
    repository: &Path,
    entry: &crate::CatalogueTask,
    suite: &Suite,
    arms: (&ArmRef, &[(String, ArmRef)]),
    run: (&str, &str, Option<crate::SmithFace>, &str),
    repetitions: u32,
    requests: &mut Vec<crate::run::Prepared>,
) -> Result<(), String> {
    let (arm, binaries) = arms;
    for agent in &suite.agents {
        if !agent.tasks.is_empty() && !agent.tasks.contains(&entry.name) {
            continue;
        }
        let installed;
        let selected = if agent.agent == Agent::Codex && matches!(arm, ArmRef::Built(_)) {
            installed = codex(repository, &agent.config)?;
            &installed
        } else {
            arm
        };
        let mut executions = Vec::new();
        if suite.change {
            for (name, binary) in binaries {
                executions.push((Some(name), binary, Vec::new()));
            }
        }
        for comparison in &suite.arms {
            match comparison {
                crate::Arm::Binary { name, .. } => {
                    let (_, binary) = binaries
                        .iter()
                        .find(|(built_name, _)| built_name == name)
                        .ok_or("binary arm has no prepared build")?;
                    executions.push((Some(name), binary, Vec::new()));
                }
                crate::Arm::Override { name, smith } => {
                    executions.push((Some(name), selected, crate::suite::override_arguments(smith)));
                }
                crate::Arm::Agent { .. } => {}
            }
        }
        if executions.is_empty() {
            executions.push((None, selected, Vec::new()));
        }
        for (name, program, arguments) in executions {
            for _ in 0..repetitions {
                let mut request = request(repository, entry, suite, agent, program, run, requests.len() + 1)?;
                if let Some(name) = name {
                    request.identity.arm.clone_from(name);
                    request.arguments.clone_from(&arguments);
                }
                requests.push(request);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/overrides/suite.rs"]
mod override_tests;

fn narrowed(file: &Path, order: &Order) -> Result<toml::Value, String> {
    let mut value: toml::Value = crate::formats::read_document(file).map_err(|error| error.to_string())?;
    let fields = value.as_table_mut().ok_or("suite must be a TOML table")?;
    fields.insert("tasks".into(), toml::Value::Array(vec![toml::Value::String(order.probe.clone())]));
    fields.insert("select".into(), toml::Value::Array(vec![]));
    fields.insert("arm".into(), toml::Value::Array(vec![]));
    fields.insert("repetitions".into(), toml::Value::Integer(i64::from(order.attempts)));
    fields.insert("rerun_failed".into(), toml::Value::Integer(i64::from(order.attempts == 1)));
    if order.probe == "probes/credentials" && order.agent != Agent::Smith {
        return Err("the credentials probe requires smith's chatgpt account".into());
    }
    if order.agent == Agent::ClaudeCode {
        let program = toml::from_str::<toml::Value>(
            "[[agents]]\nagent='smith'\nprovider='anthropic'\nconfig='smith-claude-code'",
        )
        .map_err(|error| error.to_string())?;
        fields.insert("agents".into(), program["agents"].clone());
    } else if order.agent == Agent::Codex {
        let codex = toml::from_str::<toml::Value>("[[agents]]\nagent='codex'\nprovider='codex'\nconfig='standard'")
            .map_err(|error| error.to_string())?;
        fields.insert("agents".into(), codex["agents"].clone());
    } else {
        let agents = fields.get_mut("agents").and_then(toml::Value::as_array_mut).ok_or("suite agents")?;
        agents.retain(|agent| {
            agent["agent"].as_str() == Some("smith")
                && agent["tasks"].as_array().is_none_or(|tasks| {
                    tasks.is_empty() || tasks.iter().any(|task| task.as_str() == Some(&order.probe))
                })
        });
        for agent in agents {
            agent["tasks"] = toml::Value::Array(vec![toml::Value::String(order.probe.clone())]);
        }
    }
    Ok(value)
}

/// Prepare a probe's authored pin and tier, without invoking a compiler or agent.
pub fn plan(repository: &Path, order: &Order, arm: &ArmRef) -> Result<Planned, String> {
    if !order.probe.starts_with("probes/") || order.attempts == 0 {
        return Err("invalid probe order".into());
    }
    let value = narrowed(&repository.join("benchmarks/suites/probes.toml"), order)?;
    prepare(repository, &order.probe, &value, (arm, None), true, &[])
}

fn prepare(
    repository: &Path,
    name: &str,
    value: &toml::Value,
    arms: (&ArmRef, Option<&ArmRef>),
    logical: bool,
    asked: &[String],
) -> Result<Planned, String> {
    let harness_commit = crate::arm::commit(repository, "HEAD")?;
    let suite: Suite = value.clone().try_into().map_err(|error| error.to_string())?;
    let (arm, parent_arm) = arms;
    check_parent(&suite, parent_arm.is_some())?;
    let (stamp, started) = utc()?;
    let source = match arm {
        ArmRef::Built(build) => build.arm.commit.clone(),
        ArmRef::Installed { program } => crate::arm::binary_digest(program)?,
    };
    let run = format!("{}-{}-{stamp}", name.replace('/', "-"), &source[..8]);
    let guard = Guard::from_environment()?;
    let parent = crate::arm::state_root(&guard)?.join("requests");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(guard.write_path(&parent)?)
        .map_err(|error| error.to_string())?;
    let directory = parent.join(&run);
    if directory.exists() {
        return Err("run already exists; wait until the next UTC second".into());
    }
    crate::isolate::create_directory(&guard, &directory)?;
    let suite_file = parent.join(format!("{run}.toml"));
    write(&guard, &suite_file, toml::to_string(value).map_err(|error| error.to_string())?.as_bytes())?;
    let root = repository.join("benchmarks");
    let entries = if logical {
        let base = name.split('/').take(2).collect::<Vec<_>>().join("/");
        if base.split('/').any(|part| part.is_empty() || part == ".." || part == ".") {
            return Err("invalid task name".into());
        }
        let manifest = root.join("tasks").join(&base).join("task.toml");
        vec![crate::CatalogueTask {
            name: base,
            task: crate::check_task(&manifest, &root.join("../docs/design")).map_err(|error| error.to_string())?,
            manifest,
        }]
    } else {
        crate::catalogue(&root.join("tasks"), &root.join("../docs/design")).map_err(|error| error.to_string())?
    };
    let models = crate::read_model_tiers(&root.join("agents/models.toml")).map_err(|error| error.to_string())?;
    let names = crate::validate_suite(&suite_file, &suite, &entries, &root.join("agents"), &models)
        .map_err(|error| error.to_string())?;
    let names = crate::include(&suite_file, &suite, &names, asked).map_err(|error| error.to_string())?;
    let names = if name == "smoke" { gating::smoke_tasks(&entries, &names)? } else { names };
    let face = arm_face(repository, arm)?;
    let mut binaries = Vec::new();
    if let Some(parent_arm) = parent_arm {
        arm_face(repository, parent_arm)?;
        binaries.push(("change".into(), arm.clone()));
        binaries.push(("parent".into(), parent_arm.clone()));
    }
    for comparison in &suite.arms {
        if let crate::Arm::Binary { name, commit } = comparison {
            let binary = archived(repository, &Source::Commit(commit.clone()))?;
            arm_face(repository, &binary)?;
            binaries.push((name.clone(), binary));
        }
    }
    let mut requests = Vec::new();
    for name in names {
        let entry = task(&entries, &name)?;
        let task = &entry.task;
        let repetitions = suite.task_repetitions(&name, task.repetitions, logical)?;
        task_requests(
            repository,
            &entry,
            &suite,
            (arm, &binaries),
            (&run, &started, face, &harness_commit),
            repetitions,
            &mut requests,
        )?;
    }
    for request in &requests {
        write(
            &guard,
            &directory.join(format!("{}.json", request.identity.attempt)),
            &serde_json::to_vec_pretty(request).map_err(|error| error.to_string())?,
        )?;
    }
    Ok(Planned { run, suite: suite_file, directory, requests })
}

fn arm_face(repository: &Path, arm: &ArmRef) -> Result<Option<crate::SmithFace>, String> {
    match arm {
        ArmRef::Installed { .. } => Ok(None),
        ArmRef::Built(build) => {
            let faces = crate::agent::smith::read_faces(&repository.join("benchmarks/agents/smith/faces.toml"))
                .map_err(|error| error.reason)?;
            crate::agent::smith::face(&build.arm, &faces, repository).map_err(|error| error.reason)?;
            Ok(Some(crate::SmithFace::Events))
        }
    }
}

fn codex(repository: &Path, configuration: &str) -> Result<ArmRef, String> {
    let pin = crate::read_configuration(
        &repository.join("benchmarks/agents/codex").join(format!("{configuration}.pin.toml")),
    )
    .map_err(|error| error.to_string())?;
    let guard = Guard::from_environment()?;
    let program = crate::arm::state_root(&guard)?
        .join(pin.pin.package.ok_or("Codex requires a private package")?)
        .join("bin/codex");
    Ok(ArmRef::Installed { program })
}

fn archived(repository: &Path, source: &Source) -> Result<ArmRef, String> {
    let reference = match source {
        Source::Head => "HEAD",
        Source::Commit(commit) => commit,
    };
    let commit = crate::arm::resolve_commit(repository, reference)?;
    let guard = Guard::from_environment()?;
    let arms = crate::arm::state_root(&guard)?.join("arms");
    for entry in fs::read_dir(arms).map_err(|_| format!("no archived arm for {commit}; run build {reference} first"))? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_name().to_string_lossy().starts_with(&format!("{commit}-")) {
            return crate::arm::read_build(&entry.path().join("smith")).map(ArmRef::Built);
        }
    }
    Err(format!("no archived arm for {commit}; run build {reference} first"))
}

fn check_parent(suite: &Suite, has_parent: bool) -> Result<(), String> {
    if suite.change != has_parent {
        return Err(if suite.change {
            "change suite requires --parent or --parent-arm"
        } else {
            "only change suites accept a parent"
        }
        .into());
    }
    Ok(())
}

/// Execute a parsed command with frozen arms, printing each settled attempt and summary.
pub fn command(command: &str, arguments: &[String]) -> Result<Judgment, String> {
    let (name, flags) = arguments.split_first().ok_or("a task or suite name is required")?;
    let (change_inputs, flags) = if command == "suite" {
        crate::parity::change::Inputs::parse(flags)?
    } else {
        (crate::parity::change::Inputs::default(), flags.to_vec())
    };
    if flags.len() % 2 != 0 {
        return Err("options require values".into());
    }
    let allowed: &[&str] = match command {
        "probe" => &["--agent", "--attempts", "--commit", "--arm"],
        "calibrate" => &["--attempts", "--passes", "--control", "--commit", "--arm"],
        "fix" => &["--parent", "--commit"],
        "suite" => &["--attempts", "--seed", "--commit", "--arm", "--include", "--parent", "--parent-arm"],
        _ => return Err("unknown probe command".into()),
    };
    let mut options = std::collections::BTreeMap::new();
    let mut asked = Vec::new();
    for pair in flags.chunks_exact(2) {
        if command == "suite" && pair[0] == "--include" {
            asked.push(pair[1].clone());
            continue;
        }
        if !allowed.contains(&pair[0].as_str()) || options.insert(pair[0].as_str(), pair[1].as_str()).is_some() {
            return Err("unknown or repeated command option".into());
        }
    }
    if options.contains_key("--commit") && options.contains_key("--arm") {
        return Err("choose commit or arm".into());
    }
    if options.contains_key("--parent") && options.contains_key("--parent-arm") {
        return Err("choose parent commit or parent arm".into());
    }
    let agent = match options.get("--agent").copied().unwrap_or("smith") {
        "smith" => Agent::Smith,
        "codex" if command == "probe" => Agent::Codex,
        "smith-claude-code" if command == "probe" => Agent::ClaudeCode,
        _ => return Err("this command requires smith (probe also accepts codex or smith-claude-code)".into()),
    };
    let source = options.get("--commit").map_or(Source::Head, |commit| Source::Commit((*commit).into()));
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().ok_or("workspace missing")?;
    if command == "suite" && name == "smoke" {
        let guard = Guard::from_environment()?;
        let directory = crate::arm::state_root(&guard)?.join("logins/smith");
        crate::credential::logins::smith(&directory, "chatgpt").map_err(|refusal| refusal.reason)?;
    }
    if command == "suite" {
        let suite = crate::read_suite(&repository.join("benchmarks/suites").join(format!("{name}.toml")))
            .map_err(|error| error.to_string())?;
        check_parent(&suite, options.contains_key("--parent") || options.contains_key("--parent-arm"))?;
        if !suite.change && !change_inputs.is_empty() {
            return Err("named run inputs require a change suite".into());
        }
        if suite.change && !options.contains_key("--commit") && !options.contains_key("--arm") {
            return Err("change suite requires --commit or --arm".into());
        }
    }
    if agent == Agent::Codex || !options.contains_key("--arm") {
        crate::arm::resolve_commit(repository, "HEAD")?;
    }
    let arm = if agent == Agent::Codex {
        codex(repository, "standard")?
    } else if let Some(directory) = options.get("--arm") {
        ArmRef::Built(crate::arm::read_build(&Path::new(directory).join("smith"))?)
    } else {
        archived(repository, &source)?
    };
    let seed = options.get("--seed").map_or_else(
        || Ok(skein_shell::Clock::new().now().wall.as_nanos()),
        |seed| seed.parse::<u64>().map_err(|error| error.to_string()),
    )?;
    println!("seed={seed}");
    let attempts = options.get("--attempts").map_or(Ok(if command == "calibrate" { 12 } else { 1 }), |value| {
        value.parse::<u32>().map_err(|error| error.to_string())
    })?;
    if attempts == 0 || attempts > 12 {
        return Err("attempts must be between one and twelve".into());
    }
    let order = Order { probe: name.clone(), agent, attempts, source };
    match command {
        "probe" => {
            let planned = plan(repository, &order, &arm)?;
            let batch = crate::batch::run_probe(&planned.suite, &planned.directory, seed, name, attempts)?;
            Ok(summary(&planned, &batch))
        }
        "suite" => suite_command(repository, name, &arm, &options, (&asked, &change_inputs), seed, attempts),
        "calibrate" | "fix" => comparison(command, repository, &order, &arm, &options, seed),
        _ => Err("unknown probe command".into()),
    }
}

fn suite_attempts(value: &mut toml::Value, attempts: u32) -> Result<(), String> {
    value
        .as_table_mut()
        .ok_or("suite must be a table")?
        .insert("attempts".into(), toml::Value::Integer(i64::from(attempts)));
    Ok(())
}

fn suite_command(
    repository: &Path,
    name: &str,
    arm: &ArmRef,
    options: &std::collections::BTreeMap<&str, &str>,
    selection: (&[String], &crate::parity::change::Inputs),
    seed: u64,
    attempts: u32,
) -> Result<Judgment, String> {
    let (asked, inputs) = selection;
    let file = repository.join("benchmarks/suites").join(format!("{name}.toml"));
    let mut value: toml::Value = crate::formats::read_document(&file).map_err(|error| error.to_string())?;
    let suite: Suite = value.clone().try_into().map_err(|error| error.to_string())?;
    if options.contains_key("--attempts") {
        suite_attempts(&mut value, attempts)?;
    }
    let parent = if let Some(directory) = options.get("--parent-arm") {
        Some(ArmRef::Built(crate::arm::read_build(&Path::new(directory).join("smith"))?))
    } else if let Some(commit) = options.get("--parent") {
        Some(archived(repository, &Source::Commit((*commit).into()))?)
    } else {
        None
    };
    let planned = prepare(repository, name, &value, (arm, parent.as_ref()), false, asked)?;
    if name == "smoke" && planned.requests.is_empty() {
        println!("summary {} attempts=0 verdict=incomplete", planned.run);
        return Ok(Judgment::Incomplete);
    }
    let small = planned.requests.iter().try_fold(false, |small, request| -> Result<bool, String> {
        let task = crate::read_task(&request.task).map_err(|error| error.to_string())?;
        Ok(small
            || options.contains_key("--attempts")
                && attempts < suite.task_repetitions(&request.identity.task, task.repetitions, false)?)
    })?;
    let batch = crate::batch::run(&planned.suite, &planned.directory, seed)?;
    if small || suite.change {
        let guard = Guard::from_environment()?;
        let directory = crate::arm::state_root(&guard)?.join("runs").join(&planned.run);
        return if suite.change {
            change_summary(
                &batch,
                &suite,
                seed,
                &directory,
                inputs,
                small || batch.results.len() < planned.requests.len(),
            )
        } else {
            small_summary(&batch, &suite, seed, &directory)
        };
    }
    if name == "parity" {
        return parity_summary(repository, &batch, seed);
    }
    if name == "probes" {
        let verdict = gating::judgment(&batch, &gating::uncalibrated(&planned.requests)?);
        println!("summary {} attempts={} verdict={}", planned.run, batch.results.len(), verdict_name(verdict));
        return Ok(verdict);
    }
    Ok(summary(&planned, &batch))
}

fn small_summary(batch: &crate::batch::Batch, suite: &Suite, seed: u64, directory: &Path) -> Result<Judgment, String> {
    let summary =
        crate::summarise(&batch.results, &suite.name, &suite.tier, suite.design, seed, None, suite.reference_arm())?;
    let path = directory.join("summary.json");
    fs::create_dir_all(path.parent().ok_or("run directory missing")?).map_err(|error| error.to_string())?;
    crate::write_summary(&path, &summary)?;
    println!("{}", path.display());
    println!("summary {} attempts={} verdict=incomplete", summary.run, batch.results.len());
    Ok(Judgment::Incomplete)
}

fn change_summary(
    batch: &crate::batch::Batch,
    suite: &Suite,
    seed: u64,
    directory: &Path,
    inputs: &crate::parity::change::Inputs,
    small: bool,
) -> Result<Judgment, String> {
    let summary =
        crate::summarise(&batch.results, &suite.name, &suite.tier, suite.design, seed, None, suite.reference_arm())?;
    settle_change(summary, directory, inputs, small)
}

fn settle_change(
    mut summary: crate::RunSummary,
    directory: &Path,
    inputs: &crate::parity::change::Inputs,
    small: bool,
) -> Result<Judgment, String> {
    crate::parity::change::attach(&mut summary, inputs)?;
    let path = directory.join("summary.json");
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    crate::write_summary(&path, &summary)?;
    let code = summary.change.as_ref().expect("change section").code();
    let verdict = if small {
        Judgment::Incomplete
    } else {
        match code {
            0 => Judgment::Pass,
            1 => Judgment::Fail,
            _ => Judgment::Incomplete,
        }
    };
    println!("{}", path.display());
    println!("summary {} attempts={} verdict={}", summary.run, summary.attempts.len(), verdict_name(verdict));
    Ok(verdict)
}

fn parity_summary(repository: &Path, batch: &crate::batch::Batch, seed: u64) -> Result<Judgment, String> {
    let summary = crate::summarise(&batch.results, "parity", "working", crate::Design::Interleaved, seed, None, None)?;
    let directory = repository.join("benchmarks/summaries");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    crate::write_summary(&directory.join(format!("{}.json", summary.run)), &summary)?;
    let parity = summary.parity.expect("parity suite section");
    let verdict = match parity.verdict {
        crate::parity::Verdict::OnPar => Judgment::Pass,
        crate::parity::Verdict::NotOnPar => Judgment::Fail,
        crate::parity::Verdict::Incomplete => Judgment::Incomplete,
    };
    println!("summary {} attempts={} verdict={}", summary.run, batch.results.len(), verdict_name(verdict));
    Ok(verdict)
}

fn comparison(
    command: &str,
    repository: &Path,
    order: &Order,
    arm: &ArmRef,
    options: &std::collections::BTreeMap<&str, &str>,
    seed: u64,
) -> Result<Judgment, String> {
    let required =
        options.get("--passes").map_or(Ok(10), |passes| passes.parse::<u32>().map_err(|error| error.to_string()))?;
    if command == "calibrate" && (required != 10 || required > order.attempts) {
        return Err("calibration requires ten passes within twelve attempts".into());
    }
    let control = options.get(if command == "fix" { "--parent" } else { "--control" });
    if command == "fix" && control.is_none() {
        return Err("fix requires --parent COMMIT".into());
    }
    let planned = plan(repository, order, arm)?;
    let after_build = crate::fix::prepared(&planned.suite, &planned.directory, seed, &order.probe, order.attempts)?;
    let before = control
        .map(|commit| -> Result<_, String> {
            let before_arm = archived(repository, &Source::Commit((*commit).into()))?;
            if let (ArmRef::Built(before), ArmRef::Built(after)) = (&before_arm, arm)
                && before.arm.commit == after.arm.commit
            {
                return Err("control must differ from the changed arm".into());
            }
            let ceiling = if command == "calibrate" { 2 } else { 1 };
            let one =
                Order { probe: order.probe.clone(), agent: Agent::Smith, attempts: ceiling, source: Source::Head };
            let planned = plan(repository, &one, &before_arm)?;
            let before_build = crate::fix::prepared(&planned.suite, &planned.directory, seed, &order.probe, ceiling)?;
            let batch = if command == "calibrate" {
                crate::batch::run_calibration(&planned.suite, &planned.directory, seed, &order.probe, 2, 2)?
            } else {
                crate::batch::run_probe(&planned.suite, &planned.directory, seed, &order.probe, 1)?
            };
            Ok((before_build, planned, batch))
        })
        .transpose()?;
    if command == "fix" {
        let batch = crate::batch::run_probe(&planned.suite, &planned.directory, seed, &order.probe, 1)?;
        let (before_build, _, before_batch) = before.ok_or("fix lacks parent")?;
        let finding = crate::fix::report(
            order.probe.clone(),
            crate::fix::actual(&before_build, &before_batch.results, &before_batch)?,
            crate::fix::actual(&after_build, &batch.results, &batch)?,
        );
        print!("{}", crate::fix::body(&finding));
        return Ok(match finding.shown {
            crate::fix::Shown::Effect => Judgment::Pass,
            crate::fix::Shown::FailsAfter | crate::fix::Shown::PassesBefore => Judgment::Fail,
            crate::fix::Shown::BeforeRefused | crate::fix::Shown::Unjudged => Judgment::Incomplete,
        });
    }
    if before.as_ref().is_some_and(|(_, _, batch)| judgment(batch) == Judgment::Pass) {
        println!("control passed both attempts; no calibration written");
        return Ok(Judgment::Fail);
    }
    finish_calibration(repository, order, &planned, before.as_ref(), required, seed)
}

fn finish_calibration(
    repository: &Path,
    order: &Order,
    planned: &Planned,
    before: Option<&(crate::arm::Build, Planned, crate::batch::Batch)>,
    required: u32,
    seed: u64,
) -> Result<Judgment, String> {
    let task = selected_task(&planned.requests[0].task, &order.probe)?;
    let batch = crate::batch::run_calibration(
        &planned.suite,
        &planned.directory,
        seed,
        &order.probe,
        required,
        order.attempts,
    )?;
    let results = batch.results;
    if results.first().is_some_and(crate::cache::selected) {
        print!("{}", crate::cache::notes(&results));
    }
    let counted = count(&results, required, order.attempts);
    println!(
        "summary {} passed={} failed={} deviations={} unjudged={} required={} ceiling={}",
        planned.run,
        counted.passed,
        counted.failed,
        counted.deviations,
        counted.unjudged,
        counted.required,
        counted.ceiling
    );
    let control_failed = before.as_ref().is_none_or(|(_, _, batch)| {
        judgment(batch) == Judgment::Fail
            && (!batch.results.iter().any(crate::cache::selected) || crate::cache::control_failure(&batch.results))
    });
    if counted.calibrated() && control_failed && !batch.incomplete {
        let failed_before = before.as_ref().map_or_else(Vec::new, |(_, _, batch)| {
            batch
                .results
                .iter()
                .filter(|result| crate::judge(result) == Judgment::Fail)
                .map(|result| format!("{}/{}", result.identity.run, result.identity.attempt))
                .collect()
        });
        let (attempts, passes, deviations) = calibration_attempts(&results, &planned.run);
        let first = &results[0].identity;
        let block = crate::Calibration {
            binary: first.agent_version.clone(),
            binary_sha256: first.binary_sha256.clone(),
            date: first.started_utc.clone(),
            summary: format!("summaries/{}.json", planned.run),
            attempts,
            passes,
            deviations,
            control: before.as_ref().map_or_else(
                || {
                    if order.probe == "probes/first-request" {
                        "none: the behaviour predates the oldest runnable face".into()
                    } else {
                        "none: no runnable control was named".into()
                    }
                },
                |(build, _, _)| build.arm.commit.clone(),
            ),
            failed_before,
            prompt_sha256: first.prompt_sha256.clone(),
            seed_sha256: first.seed_sha256.clone(),
            checks_sha256: task.checks_sha256()?,
        };
        record_calibration(repository, &planned.requests[0].task, &order.probe, &block, &results, seed)?;
        println!("{}", toml::to_string(&block).map_err(|error| error.to_string())?);
        Ok(Judgment::Pass)
    } else if counted.unjudged > 0
        || batch.incomplete
        || before.as_ref().is_some_and(|(_, _, batch)| judgment(batch) == Judgment::Incomplete)
    {
        Ok(Judgment::Incomplete)
    } else {
        Ok(Judgment::Fail)
    }
}

fn selected_task(file: &Path, name: &str) -> Result<crate::Task, String> {
    let task = crate::read_task(file).map_err(|error| error.to_string())?;
    name.splitn(3, '/').nth(2).map_or(Ok(task.clone()), |variant| task.with_variant(variant))
}

/// Persist settled calibration evidence and its task or variant block, without committing.
pub fn record_calibration(
    repository: &Path,
    file: &Path,
    name: &str,
    block: &crate::Calibration,
    results: &[AttemptResult],
    seed: u64,
) -> Result<(), String> {
    let mut document: toml::Value = crate::formats::read_document(file).map_err(|error| error.to_string())?;
    let target = if let Some(name) = name.splitn(3, '/').nth(2) {
        document
            .get_mut("variant")
            .and_then(toml::Value::as_array_mut)
            .and_then(|variants| variants.iter_mut().find(|variant| variant["name"].as_str() == Some(name)))
            .ok_or("missing calibration variant")?
    } else {
        &mut document
    };
    target
        .as_table_mut()
        .ok_or("task table required")?
        .insert("calibration".into(), toml::Value::try_from(block).map_err(|error| error.to_string())?);
    let text = toml::to_string(&document).map_err(|error| error.to_string())?;
    let task = selected_task(file, name)?;
    let summary = crate::summarise(results, "calibrate", &task.tier, crate::Design::Single, seed, None, None)?;
    let path = repository.join("benchmarks").join(&block.summary);
    fs::create_dir_all(path.parent().ok_or("summary directory required")?).map_err(|error| error.to_string())?;
    crate::write_summary(&path, &summary)?;
    fs::write(file, text).map_err(|error| error.to_string())
}

pub(crate) fn judgment(batch: &crate::batch::Batch) -> Judgment {
    if batch.results.iter().any(|result| {
        matches!(result.outcome.end, End::HarnessError { .. })
            || (matches!(result.outcome.end, End::Refused { .. }) && !crate::batch::expected_refusal(result))
    }) || batch.probes.values().any(|verdict| matches!(verdict, crate::ProbeVerdict::Quarantined))
    {
        Judgment::Incomplete
    } else {
        crate::judge_suite(&batch.results, batch.incomplete)
    }
}

fn summary(planned: &Planned, batch: &crate::batch::Batch) -> Judgment {
    let verdict = judgment(batch);
    println!("summary {} attempts={} verdict={}", planned.run, batch.results.len(), verdict_name(verdict));
    verdict
}

#[cfg(test)]
mod calibration_tests {
    #[test]
    fn ten_credentials_pairs_form_a_valid_calibration_block() {
        use sha2::{Digest, Sha256};
        let fixture: crate::AttemptResult =
            serde_json::from_str(include_str!("../tests/recorded/results/attempt.json")).expect("result fixture");
        let results: Vec<_> = (0..20)
            .map(|index| {
                let mut result = fixture.clone();
                result.identity.task = "probes/credentials".into();
                result.identity.attempt = format!("{index:02}");
                result
            })
            .collect();
        assert!(super::count(&results, 10, 12).calibrated());
        let (attempts, passes, deviations) = super::calibration_attempts(&results, "pairs");
        assert_eq!(attempts, 10);
        assert_eq!(passes.len(), 10);
        assert_eq!(super::count(&results[..19], 10, 12).unjudged, 1);
        let mut task: crate::Task =
            toml::from_str(include_str!("../tasks/probes/credentials/task.toml")).expect("task");
        task.calibration = Some(crate::Calibration {
            binary: "a".repeat(40),
            binary_sha256: "b".repeat(64),
            date: "2026-10-20".into(),
            summary: "summaries/pairs.json".into(),
            attempts,
            passes,
            deviations,
            control: "none: synthetic control".into(),
            failed_before: Vec::new(),
            prompt_sha256: format!("{:x}", Sha256::digest(task.prompt.as_bytes())),
            seed_sha256: task.seed_sha256.clone(),
            checks_sha256: task.checks_sha256().expect("checks"),
        });
        crate::check::calibration(std::path::Path::new("synthetic-task.toml"), "calibration", &task)
            .expect("accepted calibration");
        assert!(task.calibrated().expect("matching inputs"));
    }
}
