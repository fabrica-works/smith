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

/// Count each logical attempt, including cache triplets, without passing missing evidence.
#[must_use]
pub fn count(results: &[AttemptResult], required: u32, ceiling: u32) -> Counted {
    let mut counted = Counted { passed: 0, failed: 0, deviations: 0, unjudged: 0, required, ceiling };
    let physical = if results.first().is_some_and(crate::cache::selected) { 3 } else { 1 };
    for attempt in results.chunks(physical) {
        match crate::judge_suite(attempt, attempt.len() != physical) {
            Judgment::Pass => counted.passed += 1,
            Judgment::Fail | Judgment::Incomplete if attempt.iter().all(model_deviation) => counted.deviations += 1,
            Judgment::Fail => counted.failed += 1,
            Judgment::Incomplete => counted.unjudged += 1,
        }
    }
    counted
}

/// A completed answer may deviate from the task while smith's contracts hold.
#[must_use]
pub fn model_deviation(result: &AttemptResult) -> bool {
    if result.outcome.end != End::Completed {
        return false;
    }
    let mut deviation = false;
    for check in &result.outcome.checks {
        if check.passed == (Measure::Observed { value: true }) {
            continue;
        }
        let model = check.check == "answer"
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
        codex_home: Some(PathBuf::from(std::env::var_os("HOME").ok_or("HOME is required")?).join(".codex")),
        identity: Identity {
            run: run.0.into(),
            attempt: format!("{attempt:02}"),
            arm: format!("{}-{}-{}", agent.agent.directory(), agent.config, &digest[..8]),
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
        setup_deadline_seconds: 15,
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
    arm: &ArmRef,
    run: (&str, &str, Option<crate::SmithFace>, &str),
    repetitions: u32,
    requests: &mut Vec<crate::run::Prepared>,
) -> Result<(), String> {
    for agent in &suite.agents {
        if !agent.tasks.is_empty() && !agent.tasks.contains(&entry.name) {
            continue;
        }
        let installed;
        let selected = if agent.agent == Agent::Codex && matches!(arm, ArmRef::Built(_)) {
            installed = codex()?;
            &installed
        } else {
            arm
        };
        let overrides: Vec<_> = suite
            .arms
            .iter()
            .filter_map(|arm| match arm {
                crate::Arm::Override { name, smith } => Some((name, smith)),
                crate::Arm::Binary { .. } | crate::Arm::Agent { .. } => None,
            })
            .collect();
        let executions = overrides.len().max(1);
        for index in 0..executions {
            for _ in 0..repetitions {
                let mut request = request(repository, entry, suite, agent, selected, run, requests.len() + 1)?;
                if let Some((name, fields)) = overrides.get(index) {
                    request.identity.arm.clone_from(name);
                    request.arguments = crate::suite::override_arguments(fields);
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
    if order.agent == Agent::Codex {
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
    prepare(repository, &order.probe, &value, arm, true, &[])
}

fn prepare(
    repository: &Path,
    name: &str,
    value: &toml::Value,
    arm: &ArmRef,
    logical: bool,
    asked: &[String],
) -> Result<Planned, String> {
    let harness_commit = crate::arm::commit(repository, "HEAD")?;
    let suite: Suite = value.clone().try_into().map_err(|error| error.to_string())?;
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
    let face = match arm {
        ArmRef::Installed { .. } => None,
        ArmRef::Built(build) => {
            let faces =
                crate::agent::smith::read_faces(&root.join("agents/smith/faces.toml")).map_err(|error| error.reason)?;
            crate::agent::smith::face(&build.arm, &faces, repository).map_err(|error| error.reason)?;
            Some(crate::SmithFace::Events)
        }
    };
    let mut requests = Vec::new();
    for name in names {
        let entry = task(&entries, &name)?;
        let task = &entry.task;
        let repetitions = if logical {
            suite.repetitions.checked_mul(task.repetitions.unwrap_or(1)).ok_or("repetition overflow")?
        } else {
            task.repetitions.unwrap_or(suite.repetitions)
        };
        task_requests(
            repository,
            &entry,
            &suite,
            arm,
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

fn codex() -> Result<ArmRef, String> {
    let machine = crate::isolate::Machine::from_environment()?;
    Ok(ArmRef::Installed { program: crate::isolate::program(&machine, Path::new("/"), Path::new("codex"))? })
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

/// Execute a parsed command with frozen arms, printing each settled attempt and summary.
pub fn command(command: &str, arguments: &[String]) -> Result<Judgment, String> {
    let (name, flags) = arguments.split_first().ok_or("a task or suite name is required")?;
    if flags.len() % 2 != 0 {
        return Err("options require values".into());
    }
    let allowed: &[&str] = match command {
        "probe" => &["--agent", "--attempts", "--commit", "--arm"],
        "calibrate" => &["--attempts", "--passes", "--control", "--commit", "--arm"],
        "fix" => &["--parent", "--commit"],
        "suite" => &["--seed", "--commit", "--arm", "--include"],
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
    let agent = match options.get("--agent").copied().unwrap_or("smith") {
        "smith" => Agent::Smith,
        "codex" if command == "probe" => Agent::Codex,
        _ => return Err("this command requires smith (probe also accepts codex)".into()),
    };
    let source = options.get("--commit").map_or(Source::Head, |commit| Source::Commit((*commit).into()));
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().ok_or("workspace missing")?;
    crate::arm::resolve_commit(repository, "HEAD")?;
    let arm = if agent == Agent::Codex {
        codex()?
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
        "suite" => {
            let file = repository.join("benchmarks/suites").join(format!("{name}.toml"));
            let value = crate::formats::read_document(&file).map_err(|error| error.to_string())?;
            let planned = prepare(repository, name, &value, &arm, false, &asked)?;
            let batch = crate::batch::run(&planned.suite, &planned.directory, seed)?;
            Ok(summary(&planned, &batch))
        }
        "calibrate" | "fix" => comparison(command, repository, &order, &arm, &options, seed),
        _ => Err("unknown probe command".into()),
    }
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
    let physical = usize::try_from(task.repetitions.unwrap_or(1)).map_err(|error| error.to_string())?;
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
        let passes: Vec<_> = results
            .chunks(physical)
            .filter(|attempt| crate::judge_suite(attempt, false) == Judgment::Pass)
            .map(|attempt| format!("{}/{}", planned.run, attempt[0].identity.attempt))
            .collect();
        let deviations = results
            .chunks(physical)
            .filter(|attempt| attempt.iter().all(model_deviation))
            .map(|attempt| format!("{}/{}", planned.run, attempt[0].identity.attempt))
            .collect();
        let first = &results[0].identity;
        let block = crate::Calibration {
            binary: first.agent_version.clone(),
            binary_sha256: first.binary_sha256.clone(),
            date: first.started_utc.clone(),
            summary: format!("summaries/{}.json", planned.run),
            attempts: counted.passed + counted.deviations,
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
