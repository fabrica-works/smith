//! One prepared attempt, retaining scoped evidence and private archives only.
//! Setup and grading run outside the measured invocation. The shared supervisor
//! owns every process tree; no process discovery or resource formula lives here.
//! Entry point `attempt` follows benchmarks.md, sections 6, 8 and 10.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skein_lib::{Duration, Time};
use skein_shell::Clock;
use skein_world::end_to_end::{Command, Method, PeakScope};

use crate::guard::Guard;
use crate::isolate::{Machine, Private};
use crate::process::{Bounds, Capture, ProcessResult, execute};
use crate::{
    Artifact, AttemptResult, End, Exit, Forced, Health, Identity, Measure, Outcome, ResourceScope, Resources, Spend,
    SpendBasis, Task, Timing,
};

/// An adapter's prepared pipe invocation and author-supplied harness policies.
/// Arguments already name the planned scratch paths; the runner owns its cwd.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Prepared {
    pub task: PathBuf,
    pub identity: Identity,
    pub program: PathBuf,
    pub arguments: Vec<String>,
    /// Only Codex may name its existing read-only borrowed login directory.
    pub codex_home: Option<PathBuf>,
    pub teardown_ms: u64,
    pub grace_ms: u64,
    pub settlement_ms: u64,
    pub setup_deadline_seconds: u64,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    /// Explicit required free bytes, including writable caches and raw archives.
    pub disk_estimate_bytes: u64,
    pub force_walk: bool,
}

/// The recorded result and whether preparation left the suite incomplete.
#[derive(Debug)]
pub struct Recorded {
    pub result: AttemptResult,
    pub directory: PathBuf,
    pub incomplete: bool,
}

fn milliseconds(span: Duration) -> u64 {
    span.as_nanos() / 1_000_000
}
fn elapsed(start: Time, end: Time) -> u64 {
    end.as_nanos().checked_sub(start.as_nanos()).expect("one monotonic attempt clock") / 1_000_000
}

fn blank(identity: Identity) -> AttemptResult {
    let missing = "inference did not start";
    AttemptResult {
        version: crate::RESULT_VERSION,
        identity,
        timing: Timing {
            process_wall_ms: Measure::unavailable(missing),
            task_wall_ms: Measure::unavailable(missing),
            teardown_ms: Measure::unavailable(missing),
            grading_wall_ms: Measure::unavailable("no grader ran"),
            first_byte_ms: Measure::unavailable("not exposed by the selected stream"),
            largest_gap_ms: Measure::unavailable("not exposed by the selected stream"),
            longest_completion_ms: Measure::unavailable("not exposed by the selected stream"),
            agent_t_ms: Measure::unavailable("not reported by the agent"),
        },
        tokens: Vec::new(),
        counts: crate::Counts {
            responses: Measure::unavailable(missing),
            provider_attempts: Measure::unavailable(missing),
            retries: Measure::unavailable(missing),
            completions: Measure::unavailable(missing),
            tool_calls: BTreeMap::new(),
            tool_failures: BTreeMap::new(),
            children: Measure::unavailable(missing),
            conversations: Measure::unavailable(missing),
            compactions: Measure::unavailable(missing),
            message_terminals: BTreeMap::new(),
        },
        resources: Resources {
            cpu_scope: ResourceScope::Unavailable,
            cpu_ms: Measure::unavailable(missing),
            peak_memory_scope: ResourceScope::Unavailable,
            peak_rss_bytes: Measure::unavailable(missing),
        },
        spend: Spend {
            basis: SpendBasis::Priced,
            usd: Measure::unavailable("adapter has not supplied scoped usage and prices"),
        },
        outcome: Outcome {
            end: End::Refused { setup: "preparation incomplete".into() },
            exit: Measure::unavailable(missing),
            forced: Forced::No,
            warnings: Vec::new(),
            checks: Vec::new(),
            grade: None,
            protected_files_changed: Measure::unavailable(missing),
            writes_outside_workspace: Measure::unavailable("calls have not been observed"),
        },
        health: Health { loss_per_sink: BTreeMap::new(), headroom: BTreeMap::new(), passive_checks: Vec::new() },
        artifacts: Vec::new(),
    }
}

pub(crate) fn private_parent(guard: &Guard, directory: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let directory = guard.write_path(directory)?;
    if !directory.exists() {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(&directory).map_err(|error| error.to_string())?;
    }
    let metadata = fs::symlink_metadata(&directory).map_err(|error| error.to_string())?;
    if !metadata.is_dir() || metadata.is_symlink() || metadata.permissions().mode() & 0o077 != 0 {
        return Err("attempt parents must be private directories".into());
    }
    for ancestor in directory.ancestors() {
        if ancestor.join(".git").exists() {
            return Err("attempt data must be outside repositories".into());
        }
    }
    Ok(())
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Resolve the planned scratch root without materialising it.
pub fn scratch_root(state: &Path, prepared: &Prepared) -> Result<PathBuf, String> {
    if !valid_name(&prepared.identity.run) || !valid_name(&prepared.identity.attempt) {
        return Err("run and attempt ids must be ordinary directory names".into());
    }
    Ok(state.join("scratch").join(format!("{}-{}", prepared.identity.run, prepared.identity.attempt)))
}

fn raw(
    clock: &Clock,
    machine: &Machine,
    private: &Private,
    arguments: &[String],
    deadline: u64,
    prepared: &Prepared,
) -> Result<ProcessResult, String> {
    raw_environment(clock, machine, private, arguments, prepared, (deadline, &BTreeMap::new()))
}

fn raw_environment(
    clock: &Clock,
    machine: &Machine,
    private: &Private,
    arguments: &[String],
    prepared: &Prepared,
    policy: (u64, &BTreeMap<String, String>),
) -> Result<ProcessResult, String> {
    let (deadline, environment) = policy;
    let (program, arguments) = arguments.split_first().ok_or("empty author command")?;
    let command = Command {
        program: crate::isolate::program(machine, &private.workspace, Path::new(program))?,
        arguments: arguments.iter().map(Into::into).collect(),
        environment: {
            let mut values = crate::isolate::environment(machine, private, environment)?;
            values.push(("GIT_CONFIG_NOSYSTEM".into(), "1".into()));
            values.push(("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()));
            values
        },
        directory: private.workspace.clone(),
    };
    execute(
        clock,
        command,
        Box::new([]),
        Capture::Raw,
        Bounds {
            hard: Duration::from_secs(deadline),
            teardown: Duration::ZERO,
            grace: Duration::from_millis(prepared.grace_ms),
            settlement: Duration::from_millis(prepared.settlement_ms),
            stdout: prepared.max_stdout_bytes,
            stderr: prepared.max_stderr_bytes,
            force_walk: prepared.force_walk,
        },
    )
}

fn succeeded(result: &ProcessResult) -> bool {
    result.exit == Exit::Code(0) && result.forced == Forced::No && result.error.is_none()
}

fn free_space(clock: &Clock, machine: &Machine, private: &Private, prepared: &Prepared) -> Result<(), String> {
    if prepared.disk_estimate_bytes == 0 {
        return Err("a positive disk estimate is required".into());
    }
    let command =
        vec!["df".into(), "-Pk".into(), "--".into(), private.root.to_str().ok_or("scratch path must be UTF-8")?.into()];
    let result = raw(clock, machine, private, &command, prepared.setup_deadline_seconds, prepared)?;
    if !succeeded(&result) {
        return Err("free-space observation failed".into());
    }
    let output = std::str::from_utf8(&result.stdout).map_err(|error| error.to_string())?;
    let bytes = output
        .lines()
        .last()
        .and_then(|line| line.split_whitespace().nth(3))
        .ok_or("df did not report available space")?
        .parse::<u64>()
        .map_err(|error| error.to_string())?
        .checked_mul(1024)
        .ok_or("free space overflow")?;
    if bytes < prepared.disk_estimate_bytes {
        return Err(format!(
            "insufficient disk space: {bytes} bytes available, {} required",
            prepared.disk_estimate_bytes
        ));
    }
    Ok(())
}

fn resources(process: &ProcessResult) -> Result<Resources, String> {
    let cpu = process.counts.user.checked_add(process.counts.system).ok_or("tree CPU overflow")?;
    let cpu_scope = match process.counts.method {
        Method::Cgroup => ResourceScope::Cgroup,
        Method::Walk => ResourceScope::PidfdWalk,
    };
    let (peak_memory_scope, peak_rss_bytes) = match process.counts.peak_scope {
        PeakScope::Tree => (ResourceScope::Cgroup, Measure::Observed { value: process.counts.peak_rss_bytes }),
        PeakScope::LargestProcess { baseline_bytes } => {
            if process.counts.peak_rss_bytes > baseline_bytes {
                (
                    ResourceScope::LargestProcess,
                    Measure::LowerBound {
                        value: process.counts.peak_rss_bytes,
                        missing: vec![
                            "pidfd walk retains the largest process peak, not simultaneous tree memory".into(),
                        ],
                    },
                )
            } else {
                (
                    ResourceScope::Unavailable,
                    Measure::unavailable("prior setup children establish the walk's unreducible peak baseline"),
                )
            }
        }
    };
    Ok(Resources {
        cpu_scope,
        cpu_ms: Measure::Observed { value: milliseconds(cpu) },
        peak_memory_scope,
        peak_rss_bytes,
    })
}

fn archive(
    guard: &Guard,
    directory: &Path,
    result: &mut AttemptResult,
    name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    result.artifacts.push(crate::grade::archive(guard, directory, name, bytes)?);
    Ok(())
}

fn seed_commit(
    clock: &Clock,
    guard: &Guard,
    machine: &Machine,
    private: &Private,
    prepared: &Prepared,
) -> Result<(), String> {
    let path = guard.write_path(&private.workspace.join(".git"))?;
    if path.exists() {
        return Err("seed must not contain git metadata".into());
    }
    for arguments in [
        vec!["init", "--quiet"],
        vec!["add", "--", "."],
        vec!["commit", "--quiet", "--allow-empty", "-m", "Frozen task seed"],
    ] {
        let mut command: Vec<String> = [
            "git",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.autocrlf=false",
            "-c",
            "gc.auto=0",
            "-c",
            "maintenance.auto=false",
            "-c",
            "user.name=Smith benchmark",
            "-c",
            "user.email=smith-bench@example.invalid",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        command.extend(arguments.into_iter().map(String::from));
        let result = raw(clock, machine, private, &command, prepared.setup_deadline_seconds, prepared)?;
        if !succeeded(&result) {
            return Err("bounded seed git setup failed".into());
        }
    }
    Ok(())
}

fn copy_cache(
    clock: &Clock,
    guard: &Guard,
    machine: &Machine,
    private: &Private,
    prepared: &Prepared,
) -> Result<(), String> {
    for (source, destination) in [(&private.warm_cargo, &private.cargo_home), (&private.warm_target, &private.target)] {
        // A source has been warmed by this attempt's own setup, never borrowed
        // from a user's Cargo home. Validate before cp can follow anything.
        let files = crate::grade::snapshot_all(source)?;
        if files
            .values()
            .any(|entry| !matches!(entry, crate::grade::Entry::File { .. } | crate::grade::Entry::Directory))
        {
            return Err("warm caches contain a link or special node".into());
        }
        guard.write_path(destination)?;
        let command = vec![
            "cp".into(),
            "--recursive".into(),
            "--reflink=auto".into(),
            "--no-preserve=ownership".into(),
            "--".into(),
            source.join(".").to_str().ok_or("cache path must be UTF-8")?.into(),
            destination.to_str().ok_or("cache path must be UTF-8")?.into(),
        ];
        let result = raw(clock, machine, private, &command, prepared.setup_deadline_seconds, prepared)?;
        if !succeeded(&result) {
            return Err("private writable cache copy failed".into());
        }
        crate::isolate::restrict_tree(guard, destination)?;
    }
    Ok(())
}

fn task_for(prepared: &Prepared) -> Result<Task, String> {
    let mut task = crate::check_task(&prepared.task, &Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/design"))
        .map_err(|error| error.to_string())?;
    let kind = match task.kind {
        crate::Kind::Probe => "probes",
        crate::Kind::Fixture => "fixtures",
        crate::Kind::Repository => "repository",
    };
    let base = format!("{kind}/{}", task.id);
    if prepared.identity.task != base {
        let name = prepared.identity.task.strip_prefix(&format!("{base}/")).ok_or("prepared task identity differs")?;
        let variant = task.variant.iter().find(|variant| variant.name == name).ok_or("unknown task variant")?.clone();
        if let Some(prompt) = variant.prompt {
            task.prompt = prompt;
        }
        task.environment.extend(variant.environment);
        task.outcome.extend(variant.outcome);
        task.event.extend(variant.event);
        task.waives.extend(variant.waives);
        task.smith.extend(variant.smith);
        if let Some(budget) = variant.budget {
            task.budget = budget;
        }
    }
    Ok(task)
}

fn prepare(
    clock: &Clock,
    guard: &Guard,
    machine: &Machine,
    task: &Task,
    task_directory: &Path,
    root: &Path,
    prepared: &Prepared,
) -> Result<(Private, String), String> {
    if task.version != prepared.identity.task_version || task.seed_sha256 != prepared.identity.seed_sha256 {
        return Err("prepared identity differs from the frozen task".into());
    }
    let prompt = task.prompt.as_bytes().to_vec();
    if prompt.is_empty()
        || prompt.len() > 4088
        || !prompt.ends_with(b"\n")
        || prompt[..prompt.len() - 1].contains(&b'\n')
    {
        return Err("prompt must be one paragraph ending in a newline, under 4088 bytes".into());
    }
    if format!("{:x}", Sha256::digest(&prompt)) != prepared.identity.prompt_sha256 {
        return Err("prompt digest differs".into());
    }
    if crate::arm::binary_digest(&prepared.program)? != prepared.identity.binary_sha256 {
        return Err("prepared binary digest differs".into());
    }
    if prepared.identity.agent != crate::Agent::Codex {
        return Err(
            "this prepared pipe face requires the Codex observer; smith driving is the legacy adapter increment".into(),
        );
    }
    if prepared.setup_deadline_seconds == 0 || task.deadline_seconds == 0 {
        return Err("setup and task require positive author deadlines".into());
    }
    for seconds in std::iter::once(prepared.setup_deadline_seconds)
        .chain(std::iter::once(task.deadline_seconds))
        .chain(task.grade.iter().map(|grade| grade.deadline_seconds))
    {
        if seconds == 0 || seconds.checked_mul(1_000_000_000).is_none() {
            return Err("author deadline is outside the clock's range".into());
        }
    }
    for milliseconds in [prepared.teardown_ms, prepared.grace_ms, prepared.settlement_ms] {
        if milliseconds.checked_mul(1_000_000).is_none() {
            return Err("supervision policy is outside the clock's range".into());
        }
    }
    let private = crate::isolate::prepare(guard, &task_directory.join("seed"), &task.seed_sha256, root)?;
    free_space(clock, machine, &private, prepared)?;
    crate::isolate::environment(machine, &private, &task.environment)?;
    seed_commit(clock, guard, machine, &private, prepared)?;
    let commit = raw(
        clock,
        machine,
        &private,
        &["git".into(), "rev-parse".into(), "HEAD".into()],
        prepared.setup_deadline_seconds,
        prepared,
    )?;
    if !succeeded(&commit) {
        return Err("cannot retain frozen seed commit".into());
    }
    let revision = String::from_utf8(commit.stdout).map_err(|error| error.to_string())?.trim().to_owned();
    let mut warming = private.clone();
    warming.cargo_home.clone_from(&private.warm_cargo);
    warming.target.clone_from(&private.warm_target);
    for command in &task.setup {
        let result = raw_environment(
            clock,
            machine,
            &warming,
            command,
            prepared,
            (prepared.setup_deadline_seconds, &task.environment),
        )?;
        if !succeeded(&result) {
            return Err(format!("setup command failed: {:?}, {:?}", result.exit, result.error));
        }
    }
    copy_cache(clock, guard, machine, &private, prepared)?;
    if !task.grade.is_empty() {
        let mut grading = crate::isolate::empty(guard, &private.root.join("grading"))?;
        grading.warm_cargo.clone_from(&private.warm_cargo);
        grading.warm_target.clone_from(&private.warm_target);
        copy_cache(clock, guard, machine, &grading, prepared)?;
    }
    Ok((private, revision))
}

fn lexical(base: &Path, path: &Path) -> PathBuf {
    use std::path::Component;
    let absolute = base.join(path);
    let mut normal = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::ParentDir => {
                normal.pop();
            }
            Component::CurDir => {}
            Component::RootDir | Component::Normal(_) | Component::Prefix(_) => normal.push(part),
        }
    }
    normal
}

fn scope(process: &ProcessResult, workspace: &Path) -> Measure<Vec<PathBuf>> {
    let mut outside = Vec::new();
    for item in process.observer.completed_items() {
        match item {
            crate::agent::codex::Item::FileChange { changes, .. } => {
                for change in changes {
                    let path = lexical(workspace, Path::new(&change.path));
                    if !path.starts_with(workspace.parent().unwrap_or(workspace)) {
                        outside.push(path);
                    }
                }
            }
            crate::agent::codex::Item::AgentMessage { .. }
            | crate::agent::codex::Item::CommandExecution { .. }
            | crate::agent::codex::Item::CollabToolCall { .. } => {}
        }
    }
    outside.sort();
    outside.dedup();
    Measure::LowerBound {
        value: outside,
        missing: vec![
            "JSON lines omit non-root calls and shell filesystem effects; complete adapter collection required".into(),
        ],
    }
}

fn observe(
    guard: &Guard,
    directory: &Path,
    workspace: &Path,
    result: &mut AttemptResult,
    process: &ProcessResult,
) -> Result<(), String> {
    result.identity.started_utc =
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(process.start_wall.as_nanos()))
            .map_err(|error| error.to_string())?
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|error| error.to_string())?;
    let classification = process.observer.classify(process.exit, process.forced);
    result.outcome.end = classification.end;
    result.outcome.warnings = classification.warnings;
    result.outcome.exit = Measure::Observed { value: process.exit };
    result.outcome.forced = process.forced;
    if let Some(error) = &process.error {
        result.outcome.end = End::HarnessError { what: error.clone() };
    }
    result.timing.process_wall_ms = Measure::Observed { value: elapsed(process.start, process.end) };
    if let Some(answered) = process.answered {
        result.timing.task_wall_ms = Measure::Observed { value: elapsed(process.start, answered) };
        result.timing.teardown_ms = Measure::Observed { value: elapsed(answered, process.end) };
    }
    result.resources = resources(process)?;
    result.counts = process.observer.counts();
    result.tokens = match process.observer.thread() {
        Some(_) => process.observer.token_records()?,
        None => Vec::new(),
    };
    result.outcome.writes_outside_workspace = scope(process, workspace);
    if let Measure::LowerBound { value, missing: _ } = &result.outcome.writes_outside_workspace {
        result.health.passive_checks.push(crate::CheckVerdict {
            check: "scope".into(),
            passed: if value.is_empty() {
                Measure::unavailable("the selected stream does not show every filesystem effect")
            } else {
                Measure::Observed { value: false }
            },
            evidence: value.iter().map(|path| path.display().to_string()).collect(),
        });
    }
    let ended_cleanly =
        process.tree.live_at_leader_exit.as_ref().is_some_and(Vec::is_empty) && process.forced == Forced::No;
    result.health.passive_checks.push(crate::CheckVerdict {
        check: "process-tree-ended".into(),
        passed: Measure::Observed { value: ended_cleanly },
        evidence: vec![format!("{:?}", process.tree)],
    });
    archive(guard, directory, result, "stdout.log", &process.stdout)?;
    archive(guard, directory, result, "stderr.log", &process.stderr)?;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct Work<'a> {
    clock: &'a Clock,
    guard: &'a Guard,
    machine: &'a Machine,
    private: &'a Private,
    task: &'a Task,
    prepared: &'a Prepared,
    directory: &'a Path,
}

fn afterwards(
    work: Work<'_>,
    seed: &crate::grade::Snapshot,
    revision: String,
    result: &mut AttemptResult,
) -> Result<(), String> {
    let Work { clock, guard, machine, private, task, prepared, directory } = work;
    let after = crate::grade::snapshot(&private.workspace)?;
    result.outcome.checks = crate::grade::checks(&private.workspace, seed, &after, &task.outcome);
    let changed: Vec<_> =
        task.protected.iter().filter(|path| crate::grade::changed(seed, &after, path)).cloned().collect();
    result.outcome.protected_files_changed = Measure::Observed { value: changed };
    let git_files = crate::grade::snapshot_all(&private.workspace.join(".git"))?;
    if git_files
        .values()
        .any(|entry| !matches!(entry, crate::grade::Entry::File { .. } | crate::grade::Entry::Directory))
    {
        return Err("agent replaced git metadata with a link or special file".into());
    }
    let include = raw(
        clock,
        machine,
        private,
        &[
            "git".into(),
            "-c".into(),
            "core.hooksPath=/dev/null".into(),
            "add".into(),
            "--intent-to-add".into(),
            "--".into(),
            ".".into(),
        ],
        prepared.setup_deadline_seconds,
        prepared,
    )?;
    if !succeeded(&include) {
        return Err("cannot include untracked files in seed diff".into());
    }
    let diff = raw(
        clock,
        machine,
        private,
        &[
            "git".into(),
            "-c".into(),
            "core.hooksPath=/dev/null".into(),
            "diff".into(),
            "--no-ext-diff".into(),
            "--no-textconv".into(),
            "--binary".into(),
            revision,
            "--".into(),
        ],
        prepared.setup_deadline_seconds,
        prepared,
    )?;
    if !succeeded(&diff) {
        return Err("seed diff collection failed".into());
    }
    archive(guard, directory, result, "diff.patch", &diff.stdout)?;
    if !task.grade.is_empty() {
        let grading_private = Private::paths(&private.root.join("grading"));
        let grading_root = &grading_private.root;
        let grading = &grading_private.workspace;
        crate::isolate::copy_into(guard, &private.workspace, grading)?;
        let grader = prepared.task.parent().ok_or("task needs a directory")?.join("grader");
        if grader.exists() {
            crate::isolate::copy_directory(guard, &grader, &grading_root.join("grader"))?;
        }
        let grade_start = clock.now().now;
        let mut passed = true;
        for (index, grade) in task.grade.iter().enumerate() {
            let judgment = crate::grade::command(
                clock,
                machine,
                &grading_private,
                grading,
                grade,
                Bounds {
                    hard: Duration::ZERO,
                    teardown: Duration::ZERO,
                    grace: Duration::from_millis(prepared.grace_ms),
                    settlement: Duration::from_millis(prepared.settlement_ms),
                    stdout: prepared.max_stdout_bytes,
                    stderr: prepared.max_stderr_bytes,
                    force_walk: prepared.force_walk,
                },
                &task.environment,
            )?;
            archive(guard, directory, result, &format!("grade-{index}-stdout.log"), &judgment.stdout)?;
            archive(guard, directory, result, &format!("grade-{index}-stderr.log"), &judgment.stderr)?;
            match judgment.end {
                crate::grade::GradeEnd::Passed => {}
                crate::grade::GradeEnd::Failed => passed = false,
                crate::grade::GradeEnd::Fault { reason } => {
                    result.outcome.grade = Some(Measure::unavailable(reason.clone()));
                    result.timing.grading_wall_ms = Measure::Observed { value: elapsed(grade_start, clock.now().now) };
                    return Err(reason);
                }
            }
        }
        result.outcome.grade = Some(Measure::Observed { value: passed });
        result.timing.grading_wall_ms = Measure::Observed { value: elapsed(grade_start, clock.now().now) };
    }
    Ok(())
}

/// Run exactly one prepared invocation in this process and archive its result.
/// Setup failures are recorded as refusals and make the enclosing suite incomplete.
pub fn attempt(guard: &Guard, machine: &Machine, state: &Path, prepared: &Prepared) -> Result<Recorded, String> {
    let root = scratch_root(state, prepared)?;
    private_parent(guard, root.parent().expect("scratch parent"))?;
    let archive_root = state.join("runs").join(&prepared.identity.run);
    private_parent(guard, &archive_root)?;
    let directory = crate::isolate::create_directory(guard, &archive_root.join(&prepared.identity.attempt))?;
    let clock = Clock::new();
    let existed = root.exists();
    let mut result = blank(prepared.identity.clone());
    let mut inference = false;
    let work = (|| {
        if existed {
            return Err("attempt scratch root already exists; setup refused".into());
        }
        let task = task_for(prepared)?;
        archive(
            guard,
            &directory,
            &mut result,
            "prepared.json",
            &serde_json::to_vec_pretty(prepared).map_err(|error| error.to_string())?,
        )?;
        archive(
            guard,
            &directory,
            &mut result,
            "task.toml",
            &fs::read(&prepared.task).map_err(|error| error.to_string())?,
        )?;
        archive(guard, &directory, &mut result, "prompt.txt", task.prompt.as_bytes())?;
        let task_directory = prepared.task.parent().ok_or("task needs a directory")?;
        let (private, revision) = prepare(&clock, guard, machine, &task, task_directory, &root, prepared)?;
        let seed = crate::grade::snapshot(&task_directory.join("seed"))?;
        let mut environment = crate::isolate::environment(machine, &private, &task.environment)?;
        if let Some(home) = &prepared.codex_home {
            environment.push(("CODEX_HOME".into(), home.as_os_str().into()));
        }
        let command = Command {
            program: prepared.program.clone(),
            arguments: prepared.arguments.iter().map(Into::into).collect(),
            environment,
            directory: private.workspace.clone(),
        };
        let prompt = task.prompt.as_bytes().to_vec();
        let process = execute(
            &clock,
            command,
            prompt.into_boxed_slice(),
            Capture::Codex,
            Bounds {
                hard: Duration::from_secs(task.deadline_seconds),
                teardown: Duration::from_millis(prepared.teardown_ms),
                grace: Duration::from_millis(prepared.grace_ms),
                settlement: Duration::from_millis(prepared.settlement_ms),
                stdout: prepared.max_stdout_bytes,
                stderr: prepared.max_stderr_bytes,
                force_walk: prepared.force_walk,
            },
        )?;
        inference = true;
        observe(guard, &directory, &private.workspace, &mut result, &process)?;
        afterwards(
            Work { clock: &clock, guard, machine, private: &private, task: &task, prepared, directory: &directory },
            &seed,
            revision,
            &mut result,
        )?;
        Ok(())
    })();
    if let Err(error) = work {
        result.outcome.end = if inference { End::HarnessError { what: error } } else { End::Refused { setup: error } };
    }
    if !existed && root.exists() {
        let deletion =
            guard.write_path(&root).and_then(|root| fs::remove_dir_all(root).map_err(|error| error.to_string()));
        if let Err(error) = deletion {
            result.outcome.end = End::HarnessError { what: format!("owned workspace deletion failed: {error}") };
        }
    }
    let incomplete = matches!(result.outcome.end, End::Refused { .. });
    let bytes = crate::render_result(&result)?;
    let _: Artifact = crate::grade::archive(guard, &directory, "result.json", bytes.as_bytes())?;
    Ok(Recorded { result, directory, incomplete })
}

/// Archive a dispatcher-observed failure without inventing agent measurements.
pub(crate) fn recover(
    guard: &Guard,
    state: &Path,
    prepared: &Prepared,
    process: &ProcessResult,
    reason: String,
) -> Result<AttemptResult, String> {
    let directory = state.join("runs").join(&prepared.identity.run).join(&prepared.identity.attempt);
    private_parent(guard, &directory)?;
    let mut result = blank(prepared.identity.clone());
    result.outcome.end = End::HarnessError { what: reason };
    result.outcome.forced = process.forced;
    result.health.passive_checks.push(crate::CheckVerdict {
        check: "dispatcher-tree-settled".into(),
        passed: Measure::Observed { value: true },
        evidence: vec![format!("{:?}", process.tree)],
    });
    archive(guard, &directory, &mut result, "dispatcher-stdout.log", &process.stdout)?;
    archive(guard, &directory, &mut result, "dispatcher-stderr.log", &process.stderr)?;
    let bytes = crate::render_result(&result)?;
    crate::grade::archive(guard, &directory, "result.json", bytes.as_bytes())?;
    Ok(result)
}

/// Read an adapter-produced request strictly, without reading any login.
pub fn read_prepared(path: &Path) -> Result<Prepared, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut decoder = serde_json::Deserializer::from_str(&text);
    let request =
        serde_path_to_error::deserialize(&mut decoder).map_err(|error| format!("{}: {error}", path.display()))?;
    decoder.end().map_err(|error| error.to_string())?;
    Ok(request)
}

/// Execute one already-prepared request in this fresh harness process.
pub fn attempt_file(path: &Path) -> Result<Recorded, String> {
    let guard = Guard::from_environment()?;
    let machine = Machine::from_environment()?;
    let state = crate::arm::state_root(&guard)?;
    attempt(&guard, &machine, &state, &read_prepared(path)?)
}
