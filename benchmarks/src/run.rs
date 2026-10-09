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
        counts: crate::agent::codex::Observer::default().counts(),
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

fn private_parent(guard: &Guard, directory: &Path) -> Result<(), String> {
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
    let (program, arguments) = arguments.split_first().ok_or("empty author command")?;
    let command = Command {
        program: program.into(),
        arguments: arguments.iter().map(Into::into).collect(),
        environment: crate::isolate::environment(machine, private, &BTreeMap::new())?,
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
                (ResourceScope::LargestProcess, Measure::Observed { value: process.counts.peak_rss_bytes })
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

fn prepare(
    clock: &Clock,
    guard: &Guard,
    machine: &Machine,
    task: &Task,
    task_directory: &Path,
    root: &Path,
    prepared: &Prepared,
) -> Result<Private, String> {
    if task.id != prepared.identity.task
        || task.version != prepared.identity.task_version
        || task.seed_sha256 != prepared.identity.seed_sha256
    {
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
    if prepared.setup_deadline_seconds == 0 {
        return Err("setup requires an author deadline".into());
    }
    let private = crate::isolate::prepare(guard, &task_directory.join("seed"), &task.seed_sha256, root)?;
    free_space(clock, machine, &private, prepared)?;
    crate::isolate::environment(machine, &private, &task.environment)?;
    for command in &task.setup {
        let result = raw(clock, machine, &private, command, prepared.setup_deadline_seconds, prepared)?;
        if !succeeded(&result) {
            return Err(format!("setup command failed: {:?}, {:?}", result.exit, result.error));
        }
    }
    crate::isolate::commit_seed(guard, machine, &private)?;
    Ok(private)
}

fn observe(guard: &Guard, directory: &Path, result: &mut AttemptResult, process: &ProcessResult) -> Result<(), String> {
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

fn afterwards(
    clock: &Clock,
    guard: &Guard,
    machine: &Machine,
    private: &Private,
    task: &Task,
    seed: &crate::grade::Snapshot,
    revision: String,
    prepared: &Prepared,
    directory: &Path,
    result: &mut AttemptResult,
) -> Result<(), String> {
    let after = crate::grade::snapshot(&private.workspace)?;
    result.outcome.checks = crate::grade::checks(&private.workspace, seed, &after, &task.outcome);
    let changed: Vec<_> = task.protected.iter().filter(|path| seed.get(*path) != after.get(*path)).cloned().collect();
    result.outcome.protected_files_changed = Measure::Observed { value: changed };
    // The adapter's complete call observation must supply outside-write evidence.
    // A settled filesystem alone cannot establish that no outside write happened.
    result.outcome.writes_outside_workspace =
        Measure::unavailable("complete call scope observation awaits the prepared agent adapter");
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
        let grading = private.root.join("grading");
        crate::isolate::copy_directory(guard, &private.workspace, &grading)?;
        let grade_start = clock.now().now;
        let mut passed = true;
        for (index, grade) in task.grade.iter().enumerate() {
            let judgment = crate::grade::command(
                clock,
                machine,
                private,
                &grading,
                grade,
                Duration::from_millis(prepared.grace_ms),
            )?;
            passed &= judgment.passed;
            archive(guard, directory, result, &format!("grade-{index}-stdout.log"), &judgment.stdout)?;
            archive(guard, directory, result, &format!("grade-{index}-stderr.log"), &judgment.stderr)?;
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
    let work = (|| {
        if existed {
            return Err("attempt scratch root already exists; setup refused".into());
        }
        let task = crate::read_task(&prepared.task).map_err(|error| error.to_string())?;
        let task_directory = prepared.task.parent().ok_or("task needs a directory")?;
        let private = prepare(&clock, guard, machine, &task, task_directory, &root, prepared)?;
        let seed = crate::grade::snapshot(&private.workspace)?;
        let seed_revision = raw(
            &clock,
            machine,
            &private,
            &["git".into(), "rev-parse".into(), "HEAD".into()],
            prepared.setup_deadline_seconds,
            prepared,
        )?;
        if !succeeded(&seed_revision) {
            return Err("cannot retain seed commit".into());
        }
        let revision = String::from_utf8(seed_revision.stdout).map_err(|error| error.to_string())?.trim().to_owned();
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
                stdout: prepared.max_stdout_bytes,
                stderr: prepared.max_stderr_bytes,
                force_walk: prepared.force_walk,
            },
        )?;
        observe(guard, &directory, &mut result, &process)?;
        afterwards(&clock, guard, machine, &private, &task, &seed, revision, prepared, &directory, &mut result)?;
        Ok(())
    })();
    if let Err(error) = work {
        match result.outcome.exit {
            Measure::Unavailable { .. } => result.outcome.end = End::Refused { setup: error },
            Measure::Observed { .. } | Measure::LowerBound { .. } => {
                result.outcome.end = End::HarnessError { what: error }
            }
        }
    }
    if !existed && root.exists() {
        fs::remove_dir_all(guard.write_path(&root)?).map_err(|error| error.to_string())?;
    }
    let incomplete = matches!(result.outcome.end, End::Refused { .. });
    let bytes = crate::render_result(&result)?;
    let _: Artifact = crate::grade::archive(guard, &directory, "result.json", bytes.as_bytes())?;
    Ok(Recorded { result, directory, incomplete })
}
