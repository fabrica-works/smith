//! Serial prepared suite dispatch; every attempt gets a fresh harness process.
//! An adapter owns request preparation, not this dispatcher. The shared kit owns
//! the child tree and the suite wall bound (benchmarks.md, sections 5.3 and 10).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::Path;

use skein_lib::Duration;
use skein_shell::Clock;
use skein_world::end_to_end::Command;

use crate::guard::Guard;
use crate::isolate::Machine;
use crate::process::{Bounds, Capture, execute};
use crate::run::{Prepared, read_prepared};
use crate::{AttemptResult, Design, End, Measure, ScopeKind, Suite};

/// A completed dispatch retains all results, including setup refusals.
#[derive(Debug)]
pub struct Batch {
    pub results: Vec<AttemptResult>,
    pub incomplete: bool,
    pub probes: BTreeMap<String, crate::ProbeVerdict>,
}

fn requests(directory: &Path) -> Result<Vec<Prepared>, String> {
    let mut paths = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    paths.sort_by_key(fs::DirEntry::file_name);
    paths.into_iter().map(|entry| read_prepared(&entry.path())).collect()
}

fn ordered(suite: &Suite, requests: Vec<Prepared>, seed: u64) -> Result<Vec<Prepared>, String> {
    let mut groups: BTreeMap<String, BTreeMap<String, Vec<Prepared>>> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let run = requests.first().map(|request| request.identity.run.clone());
    for request in requests {
        if Some(&request.identity.run) != run.as_ref() {
            return Err("prepared attempts must belong to one run".into());
        }
        if !ids.insert((request.identity.run.clone(), request.identity.attempt.clone())) {
            return Err("prepared attempts must have unique run/attempt ids".into());
        }
        groups
            .entry(request.identity.task.clone())
            .or_default()
            .entry(request.identity.arm.clone())
            .or_default()
            .push(request);
    }
    let mut ordered = Vec::new();
    for arms in groups.into_values() {
        let mut arms: Vec<_> = arms.into_values().collect();
        for attempts in &mut arms {
            attempts.sort_by(|left, right| left.identity.attempt.cmp(&right.identity.attempt));
        }
        let repetitions = crate::read_task(&arms[0][0].task)
            .map_err(|error| error.to_string())?
            .repetitions
            .unwrap_or(suite.repetitions);
        if arms.iter().any(|attempts| attempts.len() != usize::try_from(repetitions).expect("u32 fits usize")) {
            return Err("each prepared task/arm must supply its declared repetition count".into());
        }
        let order = match suite.design {
            Design::Single => (0..repetitions).map(|_| (0..arms.len()).collect()).collect(),
            Design::Interleaved => crate::interleave(arms.len(), repetitions, seed),
        };
        for (repetition, block) in order.into_iter().enumerate() {
            for arm in block {
                ordered.push(arms[arm][repetition].clone());
            }
        }
    }
    if ordered.is_empty() {
        return Err("prepared suite has no requests".into());
    }
    Ok(ordered)
}

fn known_tokens(result: &AttemptResult) -> Option<u64> {
    let total = result.tokens.iter().find(|record| record.role == ScopeKind::Total)?;
    // Cache write is already within fresh input; reasoning is within output.
    let mut used = 0_u64;
    for count in [&total.usage.fresh, &total.usage.cache_read, &total.usage.output] {
        match count {
            Measure::Observed { value } => used = used.checked_add(*value)?,
            Measure::LowerBound { .. } | Measure::Unavailable { .. } => return None,
        }
    }
    Some(used)
}

fn check_requests(suite_file: &Path, suite: &Suite, requests: &[Prepared]) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalogue =
        crate::catalogue(&root.join("tasks"), &root.join("../docs/design")).map_err(|error| error.to_string())?;
    let models = crate::read_model_tiers(&root.join("agents/models.toml")).map_err(|error| error.to_string())?;
    let selected = crate::validate_suite(suite_file, suite, &catalogue, &root.join("agents"), &models)
        .map_err(|error| error.to_string())?;
    let mut allowed = suite.agents.clone();
    for arm in &suite.arms {
        match arm {
            crate::Arm::Agent { agent, provider, config, .. } => {
                allowed.push(crate::SuiteAgent { agent: *agent, provider: *provider, config: config.clone() });
            }
            crate::Arm::Binary { .. } | crate::Arm::Override { .. } => {}
        }
    }
    let mut pins = Vec::new();
    for agent in &allowed {
        let path = root.join("agents").join(agent.agent.directory()).join(format!("{}.pin.toml", agent.config));
        pins.push(crate::read_configuration(&path).map_err(|error| error.to_string())?);
    }
    for request in requests {
        if !selected.contains(&request.identity.task) {
            return Err("prepared request is not a selected suite task".into());
        }
        let entry = catalogue
            .iter()
            .find(|entry| {
                request.identity.task == entry.name || request.identity.task.starts_with(&format!("{}/", entry.name))
            })
            .ok_or("prepared task is not in the catalogue")?;
        if request.task.canonicalize().map_err(|error| error.to_string())?
            != entry.manifest.canonicalize().map_err(|error| error.to_string())?
        {
            return Err("prepared request names a different task manifest".into());
        }
        if !pins.iter().any(|pin| {
            pin.pin.agent == request.identity.agent
                && pin.pin.provider == request.identity.provider
                && pin.pin.version == request.identity.agent_version
                && pin.sha256 == request.identity.configuration_sha256
        }) {
            return Err("prepared agent configuration differs from the suite's pins".into());
        }
        let model = models
            .lookup(&root.join("agents/models.toml"), &suite.tier, request.identity.provider)
            .map_err(|error| error.to_string())?;
        if model.model.as_ref() != Some(&request.identity.model.requested_model)
            || entry.task.effort.as_ref().or(model.effort.as_ref()) != Some(&request.identity.model.requested_effort)
        {
            return Err("prepared model/effort differs from the suite's tier".into());
        }
    }
    for task in &selected {
        for agent in &suite.agents {
            let pin = pins
                .iter()
                .find(|pin| {
                    pin.pin.agent == agent.agent && pin.pin.provider == agent.provider && pin.pin.name == agent.config
                })
                .expect("validated base pin");
            if !requests.iter().any(|request| {
                request.identity.task == *task
                    && request.identity.agent == agent.agent
                    && request.identity.provider == agent.provider
                    && request.identity.configuration_sha256 == pin.sha256
            }) {
                return Err("prepared requests omit a selected task/configuration".into());
            }
        }
        for arm in &suite.arms {
            let name = match arm {
                crate::Arm::Binary { name, .. }
                | crate::Arm::Override { name, .. }
                | crate::Arm::Agent { name, .. } => name,
            };
            if !requests.iter().any(|request| request.identity.task == *task && request.identity.arm == *name) {
                return Err("prepared requests omit a selected comparison arm".into());
            }
        }
    }
    Ok(())
}

fn dispatch(
    guard: &Guard,
    machine: &Machine,
    state: &Path,
    clock: &Clock,
    executable: &Path,
    request: &Prepared,
    remaining: u64,
) -> Result<AttemptResult, String> {
    let root = state.join("requests");
    crate::run::private_parent(guard, &root)?;
    let name = format!("{}-{}.request", request.identity.run, request.identity.attempt);
    // scratch_root validates ids before either the request or agent can write.
    let scratch = crate::run::scratch_root(state, request)?;
    let existed = scratch.exists();
    let document = serde_json::to_vec_pretty(&request).map_err(|error| error.to_string())?;
    crate::grade::archive(guard, &root, &name, &document)?;
    let home = std::env::var_os("HOME").ok_or("suite requires the user's directory choices")?;
    let mut environment = vec![
        ("HOME".into(), home),
        ("PATH".into(), machine.path.clone()),
        ("XDG_STATE_HOME".into(), state.parent().ok_or("state needs a parent")?.as_os_str().into()),
    ];
    for name in ["XDG_CONFIG_HOME", "CODEX_HOME", "CLAUDE_CONFIG_DIR", "RUSTUP_HOME"] {
        if let Some(value) = std::env::var_os(name) {
            environment.push((name.into(), value));
        }
    }
    let process = execute(
        clock,
        Command {
            program: executable.to_path_buf(),
            arguments: vec!["attempt".into(), root.join(&name).into_os_string()],
            environment,
            directory: state.to_path_buf(),
        },
        Box::new([]),
        Capture::Raw,
        Bounds {
            hard: Duration::from_nanos(remaining),
            teardown: Duration::ZERO,
            grace: Duration::from_millis(request.grace_ms),
            settlement: Duration::from_millis(request.settlement_ms),
            stdout: request.max_stdout_bytes,
            stderr: request.max_stderr_bytes,
            force_walk: request.force_walk,
        },
    )?;
    let result_file =
        state.join("runs").join(&request.identity.run).join(&request.identity.attempt).join("result.json");
    let mut result = match crate::read_result(&result_file) {
        Ok(result) => result,
        Err(error) if !result_file.exists() => crate::run::recover(
            guard,
            state,
            request,
            &process,
            format!("attempt ended {:?} without a result: {error}", process.exit),
        )?,
        Err(error) => return Err(format!("attempt's existing result is invalid: {error}")),
    };
    if !existed && scratch.exists() {
        let deletion =
            guard.write_path(&scratch).and_then(|path| fs::remove_dir_all(path).map_err(|error| error.to_string()));
        if let Err(error) = deletion {
            result.outcome.end = End::HarnessError { what: format!("dispatcher workspace deletion failed: {error}") };
            let path = guard.write_path(&result_file)?;
            fs::write(path, crate::render_result(&result)?).map_err(|error| error.to_string())?;
        }
    }
    Ok(result)
}

/// Dispatch adapter-prepared requests under the suite's aggregate wall budget.
/// Requests are immutable JSON documents; agent adapters add them when ready.
pub fn run(suite_file: &Path, prepared_directory: &Path, seed: u64) -> Result<Batch, String> {
    let suite = crate::read_suite(suite_file).map_err(|error| error.to_string())?;
    let requests = requests(prepared_directory)?;
    check_requests(suite_file, &suite, &requests)?;
    let mut requests: VecDeque<_> = ordered(&suite, requests, seed)?.into();
    if requests.iter().any(|request| {
        requests.iter().any(|candidate| {
            candidate.identity.run == request.identity.run
                && candidate.identity.attempt == format!("{}-rerun", request.identity.attempt)
        })
    }) {
        return Err("prepared attempt ids collide with the reserved failure rerun".into());
    }
    let disk = requests
        .iter()
        .try_fold(0_u64, |sum, request| sum.checked_add(request.disk_estimate_bytes))
        .and_then(|bytes| bytes.checked_mul(u64::from(suite.rerun_failed) + 1))
        .ok_or("suite disk estimate overflow")?;
    if let Some(first) = requests.front_mut() {
        first.disk_estimate_bytes = disk;
    }
    let guard = Guard::from_environment()?;
    let machine = Machine::from_environment()?;
    let state = crate::arm::state_root(&guard)?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let clock = Clock::new();
    let deadline = clock
        .now()
        .now
        .checked_add(Duration::from_nanos(
            suite.max_wall_seconds.checked_mul(1_000_000_000).ok_or("suite deadline overflow")?,
        ))
        .ok_or("suite deadline overflow")?;
    let mut results = Vec::new();
    let mut incomplete = false;
    let mut used = 0_u64;
    let mut probes = BTreeMap::new();
    let mut reruns = BTreeMap::new();
    while let Some(request) = requests.pop_front() {
        let remaining = deadline.as_nanos().checked_sub(clock.now().now.as_nanos());
        let Some(remaining) = remaining.filter(|span| *span > 0) else {
            incomplete = true;
            break;
        };
        if used >= suite.max_tokens {
            incomplete = true;
            break;
        }
        let result = dispatch(&guard, &machine, &state, &clock, &executable, &request, remaining)?;
        incomplete |= matches!(result.outcome.end, End::Refused { .. });
        let key = format!("{}/{}", request.identity.task, request.identity.arm);
        if request.identity.task.starts_with("probes/") {
            if let Some(passed) = crate::summary::passed(&result) {
                if let Some(first) = reruns.remove(&request.identity.attempt) {
                    probes.insert(key, crate::probe_verdict(first, Some(passed)));
                } else {
                    let verdict = crate::probe_verdict(passed, None);
                    probes.insert(key, verdict);
                    if verdict == crate::ProbeVerdict::Rerun && suite.rerun_failed == 1 {
                        let mut retry = request.clone();
                        retry.identity.attempt.push_str("-rerun");
                        reruns.insert(retry.identity.attempt.clone(), passed);
                        requests.push_front(retry);
                    }
                }
            } else {
                incomplete = true;
            }
        }
        let tokens = known_tokens(&result);
        results.push(result);
        if let Some(count) = tokens {
            used = used.checked_add(count).ok_or("suite token count overflow")?;
            incomplete |= used > suite.max_tokens;
        } else {
            incomplete = true;
            break;
        }
    }
    Ok(Batch { results, incomplete, probes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn prepared_blocks_preserve_seeded_order_unique_ids_and_token_gaps() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut suite = crate::read_suite(&root.join("tests/recorded/suites/sample.toml")).expect("suite");
        suite.design = Design::Interleaved;
        suite.repetitions = 2;
        let mut result = crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("result");
        let mut requests = Vec::new();
        for arm in ["a", "b"] {
            for repetition in 0..2 {
                let mut identity = result.identity.clone();
                identity.arm = arm.into();
                identity.attempt = format!("{arm}-{repetition}");
                requests.push(Prepared {
                    task: root.join("tests/recorded/tasks/valid/task.toml"),
                    identity,
                    program: "/bin/sh".into(),
                    arguments: Vec::new(),
                    codex_home: None,
                    teardown_ms: 50,
                    grace_ms: 20,
                    settlement_ms: 200,
                    setup_deadline_seconds: 1,
                    max_stdout_bytes: 4096,
                    max_stderr_bytes: 4096,
                    disk_estimate_bytes: 4096,
                    force_walk: true,
                });
            }
        }
        let first = ordered(&suite, requests.clone(), 71).expect("interleaving");
        let second = ordered(&suite, requests.clone(), 71).expect("same seed");
        assert_eq!(
            first.iter().map(|request| &request.identity.attempt).collect::<Vec<_>>(),
            second.iter().map(|request| &request.identity.attempt).collect::<Vec<_>>()
        );
        for block in first.chunks_exact(2) {
            assert_ne!(block[0].identity.arm, block[1].identity.arm);
        }
        let mut duplicate = requests.clone();
        duplicate.push(requests[0].clone());
        assert!(ordered(&suite, duplicate, 71).expect_err("duplicate").contains("unique"));
        let mut mixed = requests.clone();
        mixed[0].identity.run = "another-run".into();
        assert!(ordered(&suite, mixed, 71).expect_err("mixed runs").contains("one run"));
        requests.pop();
        assert!(ordered(&suite, requests, 71).expect_err("missing repetition").contains("repetition"));
        let total = result.tokens.iter_mut().find(|record| record.role == ScopeKind::Total).expect("total");
        total.usage.fresh = Measure::Observed { value: 10 };
        total.usage.cache_read = Measure::Observed { value: 20 };
        total.usage.output = Measure::Observed { value: 30 };
        assert_eq!(known_tokens(&result), Some(60));
        let total = result.tokens.iter_mut().find(|record| record.role == ScopeKind::Total).expect("total");
        total.usage.fresh = Measure::unavailable("missing child");
        assert_eq!(known_tokens(&result), None);
    }
}
