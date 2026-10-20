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

mod credentials;

pub(crate) fn validate_credentials(task: &crate::Task) -> Result<(), String> {
    credentials::validate(task)
}

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

pub(crate) fn ordered(suite: &Suite, requests: Vec<Prepared>, seed: u64) -> Result<Vec<Prepared>, String> {
    order(suite, requests, seed, false)
}

fn order(suite: &Suite, requests: Vec<Prepared>, seed: u64, logical: bool) -> Result<Vec<Prepared>, String> {
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
    for (name, arms) in groups {
        let mut arms: Vec<_> = arms.into_values().collect();
        for attempts in &mut arms {
            attempts.sort_by(|left, right| left.identity.attempt.cmp(&right.identity.attempt));
        }
        let authored = crate::read_task(&arms[0][0].task).map_err(|error| error.to_string())?.repetitions;
        let repetitions = suite.task_repetitions(&name, authored, logical)?;
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

/// Append settings after the task's settings and before its positional input.
pub(crate) fn settings_arguments(arguments: &mut Vec<std::ffi::OsString>, overrides: &[String]) {
    let end =
        if arguments.last().is_some_and(|argument| argument == "-") { arguments.len() - 1 } else { arguments.len() };
    arguments.splice(end..end, overrides.iter().map(Into::into));
}

fn observed_checks(result: &AttemptResult, names: &[&str]) -> bool {
    names.iter().all(|name| {
        result.outcome.deciding.iter().any(|deciding| deciding == *name)
            && result
                .outcome
                .checks
                .iter()
                .any(|check| check.check == *name && check.passed == Measure::Observed { value: true })
    })
}

/// A task's expected refused end passes only with every deciding check observed true.
pub(crate) fn expected_refusal(result: &AttemptResult) -> bool {
    matches!(result.outcome.end, End::Refused { .. }) && crate::judge(result) == crate::Judgment::Pass
}

/// A selected startup refusal passes every check of its empty-run story.
pub(crate) fn expected_startup_refusal(result: &AttemptResult) -> bool {
    expected_refusal(result) && observed_checks(result, &["no-run", "end", "exit-code", "last-line", "tree-ended"])
}

fn observed_no_request(result: &AttemptResult) -> bool {
    observed_checks(result, &["no-request", "stream-whole"]) || expected_startup_refusal(result)
}

fn rejected_before_output(result: &AttemptResult) -> bool {
    matches!(result.counts.provider_attempts, Measure::Observed { value } if value > 0)
        && result.health.passive_checks.iter().any(|check| {
            check.check == "provider-rejected-before-output" && check.passed == Measure::Observed { value: true }
        })
}

fn known_tokens(result: &AttemptResult) -> Option<u64> {
    let total = result.tokens.iter().find(|record| record.role == ScopeKind::Total)?;
    // Cache write is already within fresh input; reasoning is within output.
    let mut used = 0_u64;
    for count in [&total.usage.fresh, &total.usage.cache_read, &total.usage.output] {
        used = used.checked_add(crate::tokens::reported(count)?)?;
    }
    Some(used)
}

fn configuration_matches(pin: &crate::PinnedConfiguration, request: &Prepared) -> Result<bool, String> {
    if pin.pin.agent != request.identity.agent
        || pin.pin.provider != request.identity.provider
        || pin.sha256 != request.identity.configuration_sha256
    {
        return Ok(false);
    }
    match pin.pin.agent {
        crate::Agent::Smith if pin.pin.version == "built-arm" => {
            let build = crate::arm::read_build(&request.program)?;
            Ok(build.arm.commit == request.identity.agent_version && build.arm.sha256 == request.identity.binary_sha256)
        }
        crate::Agent::Smith | crate::Agent::Codex | crate::Agent::ClaudeCode => {
            Ok(pin.pin.version == request.identity.agent_version)
        }
    }
}

fn selected_configuration_matches(
    agents: &[crate::SuiteAgent],
    pins: &[crate::PinnedConfiguration],
    request: &Prepared,
) -> Result<bool, String> {
    let mut matched = false;
    for (agent, pin) in agents.iter().zip(pins) {
        if agent.tasks.is_empty() || agent.tasks.contains(&request.identity.task) {
            matched |= configuration_matches(pin, request)?;
        }
    }
    Ok(matched)
}

fn allowed_agents(suite: &Suite) -> Vec<crate::SuiteAgent> {
    let mut allowed = suite.agents.clone();
    for arm in &suite.arms {
        match arm {
            crate::Arm::Agent { agent, provider, config, .. } => {
                allowed.push(crate::SuiteAgent {
                    agent: *agent,
                    provider: *provider,
                    config: config.clone(),
                    tasks: Vec::new(),
                });
            }
            crate::Arm::Binary { .. } | crate::Arm::Override { .. } => {}
        }
    }
    allowed
}

fn check_override(suite: &Suite, request: &Prepared) -> Result<(), String> {
    if !suite.arms.is_empty() && matches!(suite.arms[0], crate::Arm::Override { .. }) {
        let fields = suite
            .arms
            .iter()
            .find_map(|arm| match arm {
                crate::Arm::Override { name, smith } if name == &request.identity.arm => Some(smith),
                crate::Arm::Binary { .. } | crate::Arm::Override { .. } | crate::Arm::Agent { .. } => None,
            })
            .ok_or("prepared request names an undeclared override arm")?;
        if request.arguments != crate::suite::override_arguments(fields) {
            return Err("prepared override settings differ from the suite".into());
        }
    }
    Ok(())
}

fn check_requests(suite_file: &Path, suite: &Suite, requests: &[Prepared]) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalogue =
        crate::catalogue(&root.join("tasks"), &root.join("../docs/design")).map_err(|error| error.to_string())?;
    let models = crate::read_model_tiers(&root.join("agents/models.toml")).map_err(|error| error.to_string())?;
    let selected = crate::validate_suite(suite_file, suite, &catalogue, &root.join("agents"), &models)
        .map_err(|error| error.to_string())?;
    let asked: Vec<String> = requests
        .iter()
        .map(|request| &request.identity.task)
        .filter(|name| !selected.contains(name))
        .cloned()
        .collect();
    let selected = crate::include(suite_file, suite, &selected, &asked).map_err(|error| error.to_string())?;
    let selected =
        if suite.name == "smoke" { crate::probe::gating::smoke_tasks(&catalogue, &selected)? } else { selected };
    let allowed = allowed_agents(suite);
    let mut pins = Vec::new();
    for agent in &allowed {
        let path = root.join("agents").join(agent.agent.directory()).join(format!("{}.pin.toml", agent.config));
        pins.push(crate::read_configuration(&path).map_err(|error| error.to_string())?);
    }
    for request in requests {
        if !selected.contains(&request.identity.task) {
            return Err("prepared request is not a selected suite task".into());
        }
        check_override(suite, request)?;
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
        if !selected_configuration_matches(&allowed, &pins, request)? {
            return Err("prepared agent configuration differs from the suite's pins".into());
        }
        let model = models
            .lookup(&root.join("agents/models.toml"), &suite.tier, request.identity.provider)
            .map_err(|error| error.to_string())?;
        let task = if request.identity.task == entry.name {
            entry.task.clone()
        } else {
            let variant = request
                .identity
                .task
                .strip_prefix(&format!("{}/", entry.name))
                .ok_or("task variant identity differs")?;
            entry.task.with_variant(variant)?
        };
        let expected_model =
            task.requested_model(request.identity.agent, model.model.as_deref().ok_or("suite tier has no model")?)?;
        if expected_model != request.identity.model.requested_model
            || task.effort.as_ref().or(model.effort.as_ref()) != Some(&request.identity.model.requested_effort)
        {
            return Err("prepared model/effort differs from the suite's tier".into());
        }
    }
    for task in &selected {
        for agent in &suite.agents {
            if !agent.tasks.is_empty() && !agent.tasks.contains(task) {
                continue;
            }
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
        if suite.change {
            for name in ["change", "parent"] {
                if !requests.iter().any(|request| request.identity.task == *task && request.identity.arm == name) {
                    return Err("prepared requests omit a change suite arm".into());
                }
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

fn dispatch_command(
    guard: &Guard,
    machine: &Machine,
    state: &Path,
    executable: &Path,
    request: &Prepared,
) -> Result<(Command, bool), String> {
    let root = state.join("requests");
    crate::run::private_parent(guard, &root)?;
    let name = format!("{}-{}.request", request.identity.run, request.identity.attempt);
    // scratch_root validates ids before either the request or agent can write.
    let scratch = crate::run::scratch_root(state, request)?;
    let existed = scratch.exists();
    let document = serde_json::to_vec_pretty(request).map_err(|error| error.to_string())?;
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
    Ok((
        Command {
            program: executable.to_path_buf(),
            arguments: vec!["attempt".into(), root.join(&name).into_os_string()],
            environment,
            directory: state.to_path_buf(),
        },
        existed,
    ))
}

fn dispatch_result(
    guard: &Guard,
    state: &Path,
    request: &Prepared,
    process: &crate::process::ProcessResult,
    existed: bool,
) -> Result<AttemptResult, String> {
    let scratch = crate::run::scratch_root(state, request)?;
    let result_file =
        state.join("runs").join(&request.identity.run).join(&request.identity.attempt).join("result.json");
    let mut result = match crate::read_result(&result_file) {
        Ok(result) => result,
        Err(error) if !result_file.exists() => crate::run::recover(
            guard,
            state,
            request,
            process,
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

fn dispatch(
    guard: &Guard,
    machine: &Machine,
    state: &Path,
    clock: &Clock,
    executable: &Path,
    request: &Prepared,
    remaining: u64,
) -> Result<AttemptResult, String> {
    let (command, existed) = dispatch_command(guard, machine, state, executable, request)?;
    let process = execute(
        clock,
        command,
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
    dispatch_result(guard, state, request, &process, existed)
}

/// Dispatch adapter-prepared requests under the suite's aggregate wall budget.
/// Requests are immutable JSON documents; agent adapters add them when ready.
pub fn run(suite_file: &Path, prepared_directory: &Path, seed: u64) -> Result<Batch, String> {
    let suite = crate::read_suite(suite_file).map_err(|error| error.to_string())?;
    run_suite(suite_file, &suite, prepared_directory, seed, false, None)
}

/// Run one authored probe with explicit repetitions, retaining the common suite supervisor.
pub fn run_probe(
    suite_file: &Path,
    prepared_directory: &Path,
    seed: u64,
    probe: &str,
    repetitions: u32,
) -> Result<Batch, String> {
    let suite = probe_suite(suite_file, probe, repetitions)?;
    run_suite(suite_file, &suite, prepared_directory, seed, true, None)
}

/// Dispatch calibration with the common aggregate budgets and stop at enough passes or impossibility.
pub fn run_calibration(
    suite_file: &Path,
    prepared_directory: &Path,
    seed: u64,
    probe: &str,
    required: u32,
    ceiling: u32,
) -> Result<Batch, String> {
    let mut suite = probe_suite(suite_file, probe, ceiling)?;
    suite.rerun_failed = 0;
    run_suite(suite_file, &suite, prepared_directory, seed, true, Some((required, ceiling)))
}

fn probe_suite(suite_file: &Path, probe: &str, repetitions: u32) -> Result<Suite, String> {
    if !probe.starts_with("probes/") || repetitions == 0 {
        return Err("probe dispatch requires a probe selector and positive repetitions".into());
    }
    let mut suite = crate::read_suite(suite_file).map_err(|error| error.to_string())?;
    suite.agents.retain(|agent| agent.tasks.is_empty() || agent.tasks.iter().any(|name| name == probe));
    for agent in &mut suite.agents {
        agent.tasks = vec![probe.into()];
    }
    suite.tasks = vec![probe.into()];
    suite.select.clear();
    suite.repetitions = repetitions;
    Ok(suite)
}

pub(crate) fn probe_requests(
    suite_file: &Path,
    prepared_directory: &Path,
    seed: u64,
    probe: &str,
    repetitions: u32,
) -> Result<Vec<Prepared>, String> {
    let suite = probe_suite(suite_file, probe, repetitions)?;
    let requests = requests(prepared_directory)?;
    check_requests(suite_file, &suite, &requests)?;
    order(&suite, requests, seed, true)
}

fn dispatch_order(
    suite: &Suite,
    requests: Vec<Prepared>,
    seed: u64,
    logical: bool,
) -> Result<(VecDeque<Prepared>, usize), String> {
    let ordered = if logical { order(suite, requests, seed, true)? } else { ordered(suite, requests, seed)? };
    let physical = ordered.first().map_or(Ok(1), |request| {
        if credentials::selected(request) {
            return Ok(2);
        }
        crate::read_task(&request.task)
            .map(|task| usize::try_from(task.repetitions.unwrap_or(1)).expect("u32 fits usize"))
            .map_err(|error| error.to_string())
    })?;
    let mut requests: VecDeque<_> = ordered.into();
    if requests.iter().any(|request| {
        credentials::selected(request)
            && requests
                .iter()
                .any(|candidate| candidate.identity.attempt == format!("{}-peer", request.identity.attempt))
    }) {
        return Err("prepared attempt ids collide with the reserved credentials peer".into());
    }
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
        .try_fold(0_u64, |sum, request| {
            let multiplier = if credentials::selected(request) { 2 } else { 1 };
            sum.checked_add(request.disk_estimate_bytes.checked_mul(multiplier)?)
        })
        .and_then(|bytes| bytes.checked_mul(u64::from(suite.rerun_failed) + 1))
        .ok_or("suite disk estimate overflow")?;
    if let Some(first) = requests.front_mut() {
        first.disk_estimate_bytes = disk;
    }
    Ok((requests, physical))
}

fn run_suite(
    suite_file: &Path,
    suite: &Suite,
    prepared_directory: &Path,
    seed: u64,
    logical: bool,
    calibration: Option<(u32, u32)>,
) -> Result<Batch, String> {
    let requests = requests(prepared_directory)?;
    let uncalibrated = crate::probe::gating::suite_tasks(suite, logical, &requests)?;
    check_requests(suite_file, suite, &requests)?;
    let (mut requests, physical) = dispatch_order(suite, requests, seed, logical)?;
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
    'attempts: while let Some(request) = requests.pop_front() {
        if let Some((required, ceiling)) = calibration
            && results.len() % physical == 0
        {
            let count = crate::probe::count(&results, required, ceiling);
            if count.calibrated() || !count.reachable() {
                break;
            }
        }
        let remaining = deadline.as_nanos().checked_sub(clock.now().now.as_nanos());
        let Some(remaining) = remaining.filter(|span| *span > 0) else {
            println!("suite max_wall_seconds reached: incomplete");
            incomplete = true;
            break;
        };
        if used >= suite.max_tokens {
            println!("suite max_tokens reached: incomplete");
            incomplete = true;
            break;
        }
        let paired = credentials::selected(&request);
        let settled = if paired {
            credentials::run(&guard, &machine, &state, &executable, &request, remaining)?
        } else {
            vec![dispatch(&guard, &machine, &state, &clock, &executable, &request, remaining)?]
        };
        let pair_passed =
            settled.iter().try_fold(true, |passed, result| crate::summary::passed(result).map(|next| passed && next));
        for (part, result) in settled.into_iter().enumerate() {
            let deviation = calibration.is_some() && crate::probe::model_deviation(&result);
            incomplete |= !deviation && crate::cache::repetition(&result) == crate::Judgment::Incomplete;
            let key = format!("{}/{}", request.identity.task, request.identity.arm);
            if request.identity.task.starts_with("probes/")
                && !crate::cache::selected(&result)
                && (!paired || part == 0)
            {
                if let Some(passed) = if paired { pair_passed } else { crate::summary::passed(&result) } {
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
                } else if !deviation {
                    incomplete = true;
                }
            }
            let tokens = known_tokens(&result);
            let zero_charge = observed_no_request(&result) || rejected_before_output(&result);
            println!("{}", crate::probe::line(&result));
            results.push(result);
            if let Some(count) = tokens {
                used = used.checked_add(count).ok_or("suite token count overflow")?;
                incomplete |= used > suite.max_tokens;
            } else if !zero_charge {
                println!("suite token usage unavailable: incomplete");
                incomplete = true;
                break 'attempts;
            }
        }
    }
    cache_verdicts(&results, &mut incomplete, &mut probes);
    Ok(crate::probe::gating::report(Batch { results, incomplete, probes }, &uncalibrated))
}

#[cfg(test)]
pub(crate) fn probe_cases() {
    tests::no_request_requires_selected_complete_observations_without_inventing_tokens();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn retried_total() -> AttemptResult {
        let mut result =
            crate::read_result(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json"))
                .expect("result fixture");
        let total = result.tokens.iter_mut().find(|record| record.role == ScopeKind::Total).expect("total");
        total.usage.fresh =
            Measure::LowerBound { value: 10, missing: vec!["root: response-3: retried failure".into()] };
        total.usage.cache_read =
            Measure::LowerBound { value: 20, missing: vec!["root: response-3: retried failure".into()] };
        total.usage.cache_write =
            Measure::LowerBound { value: 0, missing: vec!["root: response-3: retried failure".into()] };
        total.usage.output =
            Measure::LowerBound { value: 30, missing: vec!["root: response-3: retried failure".into()] };
        result.identity.model.resolved_model = Measure::Observed { value: "gpt-6.1-sol".into() };
        result.identity.model.resolved_window = Measure::Observed { value: 272_000 };
        result
    }

    #[test]
    fn a_total_missing_only_retried_failures_charges_its_lower_bound() {
        assert_eq!(known_tokens(&retried_total()), Some(60));
    }

    #[test]
    fn a_total_missing_another_count_charges_nothing_known() {
        let mut result = retried_total();
        let total = result.tokens.iter_mut().find(|record| record.role == ScopeKind::Total).expect("total");
        total.usage.output = Measure::LowerBound { value: 30, missing: vec!["root: missing response".into()] };
        assert_eq!(known_tokens(&result), None);
    }

    fn priced_response() -> crate::ResponseTokens {
        crate::ResponseTokens {
            scope: "response-4".into(),
            model: "gpt-6.1-sol".into(),
            fresh: 10,
            cache_read: 20,
            cache_write: 0,
            output: 30,
        }
    }

    #[test]
    fn pricing_reported_tokens_keeps_the_retried_failure_lower_bound() {
        assert_eq!(
            crate::result::priced_spend(&retried_total(), &[priced_response()]),
            Measure::LowerBound { value: 0.000_322, missing: vec!["root: response-3: retried failure".into()] }
        );
    }

    #[test]
    fn pricing_a_total_with_another_gap_stays_unavailable() {
        let mut result = retried_total();
        let total = result.tokens.iter_mut().find(|record| record.role == ScopeKind::Total).expect("total");
        total.usage.output = Measure::LowerBound { value: 30, missing: vec!["root: missing response".into()] };
        assert_eq!(
            crate::result::priced_spend(&result, &[priced_response()]),
            Measure::unavailable("total output tokens are a lower bound: root: missing response")
        );
    }

    pub(super) fn no_request_requires_selected_complete_observations_without_inventing_tokens() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut value = crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("fixture");
        value.tokens.clear();
        value.outcome.deciding = vec!["no-request".into(), "stream-whole".into()];
        value.outcome.checks = value
            .outcome
            .deciding
            .iter()
            .map(|name| crate::CheckVerdict {
                check: name.clone(),
                passed: Measure::Observed { value: true },
                evidence: vec![],
            })
            .collect();
        assert!(observed_no_request(&value));
        assert_eq!(known_tokens(&value), None, "NoRequest is evidence, not invented numeric usage");
        value.outcome.checks[1].passed = Measure::unavailable("missing authoritative terminal");
        assert!(!observed_no_request(&value));
        value.outcome.checks[1].passed = Measure::LowerBound { value: true, missing: vec!["loss".into()] };
        assert!(!observed_no_request(&value));
        value.outcome.checks[1].passed = Measure::Observed { value: true };
        value.outcome.deciding.pop();
        assert!(!observed_no_request(&value), "an advisory no-request observation cannot lift a token gap");
    }

    fn startup_refusal_result() -> AttemptResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut result = crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("fixture");
        result.tokens.clear();
        result.identity.agent = crate::Agent::Smith;
        result.outcome.end = End::Refused { setup: "expected invalid setting".into() };
        result.outcome.exit = Measure::Observed { value: crate::Exit::Code(2) };
        result.outcome.forced = crate::Forced::No;
        result.outcome.deciding = ["no-run", "end", "exit-code", "last-line", "tree-ended"].map(str::to_owned).to_vec();
        result.outcome.checks = result
            .outcome
            .deciding
            .iter()
            .map(|name| crate::CheckVerdict {
                check: name.clone(),
                passed: Measure::Observed { value: true },
                evidence: vec![],
            })
            .collect();
        result
    }

    #[test]
    fn expected_startup_refusal_keeps_a_suite_running_without_token_totals() {
        let result = startup_refusal_result();
        let mut batch = Batch { results: vec![result], incomplete: false, probes: BTreeMap::new() };
        assert!(observed_no_request(&batch.results[0]));
        assert_eq!(known_tokens(&batch.results[0]), None);
        assert_eq!(crate::probe::judgment(&batch), crate::Judgment::Pass);
        for index in 0..batch.results[0].outcome.checks.len() {
            batch.results[0].outcome.checks[index].passed =
                Measure::LowerBound { value: true, missing: vec!["missing".into()] };
            assert!(!observed_no_request(&batch.results[0]));
            assert_eq!(crate::probe::judgment(&batch), crate::Judgment::Incomplete);
            batch.results[0].outcome.checks[index].passed = Measure::Observed { value: true };
        }
        batch.results[0].outcome.deciding.pop();
        assert!(!observed_no_request(&batch.results[0]));
        assert_eq!(crate::probe::judgment(&batch), crate::Judgment::Pass);
    }

    #[test]
    fn an_expected_in_run_refusal_passes_only_when_every_deciding_check_passes() {
        let mut result = startup_refusal_result();
        result.outcome.deciding =
            ["end", "run-completed", "no-request", "stream-whole", "tree-ended"].map(str::to_owned).to_vec();
        result.outcome.checks = result
            .outcome
            .deciding
            .iter()
            .map(|name| crate::CheckVerdict {
                check: name.clone(),
                passed: Measure::Observed { value: true },
                evidence: vec![],
            })
            .collect();
        let mut batch = Batch { results: vec![result], incomplete: false, probes: BTreeMap::new() };
        assert!(!expected_startup_refusal(&batch.results[0]));
        assert_eq!(crate::probe::judgment(&batch), crate::Judgment::Pass);
        for index in 0..batch.results[0].outcome.checks.len() {
            batch.results[0].outcome.checks[index].passed = Measure::Observed { value: false };
            assert!(!expected_refusal(&batch.results[0]));
            assert_eq!(crate::probe::judgment(&batch), crate::Judgment::Incomplete);
            batch.results[0].outcome.checks[index].passed = Measure::unavailable("missing deciding observation");
            assert!(!expected_refusal(&batch.results[0]));
            batch.results[0].outcome.checks[index].passed = Measure::Observed { value: true };
        }
    }

    #[test]
    fn a_proved_provider_rejection_charges_no_tokens_without_changing_usage_or_requests() {
        let mut result = startup_refusal_result();
        result.outcome.end = End::Failed { reason: crate::FailureReason::Model, detail: "HTTP 400".into() };
        result.counts.provider_attempts = Measure::Observed { value: 1 };
        let health = &mut result.health;
        health.passive_checks.push(crate::CheckVerdict {
            check: "provider-rejected-before-output".into(),
            passed: Measure::Observed { value: true },
            evidence: vec![],
        });
        assert!(rejected_before_output(&result));
        assert!(!observed_no_request(&result));
        assert_eq!(known_tokens(&result), None);
        assert_eq!(result.counts.provider_attempts, Measure::Observed { value: 1 });
        result.counts.provider_attempts = Measure::LowerBound { value: 1, missing: vec!["loss".into()] };
        assert!(!rejected_before_output(&result));
        result.counts.provider_attempts = Measure::Observed { value: 1 };
        result.health.passive_checks.last_mut().expect("proof").passed =
            Measure::Unavailable { reason: "unobserved".into() };
        assert!(!rejected_before_output(&result));
    }

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
                    configuration: None,
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

        // The fake fresh attempt creates its scratch, then exits without a
        // result. Settlement must precede recovery and owned scratch deletion;
        // dispatcher resource counts never become agent measurements.
        let request = &first[0];
        let state = std::env::temp_dir().join(format!("smith-bench-dispatch-{}", std::process::id()));
        let guard = Guard::new(&state.join("user-home"), &state.join("config"), &state.join("state"), &[])
            .expect("synthetic guard");
        crate::isolate::create_directory(&guard, &state).expect("private state");
        let scratch = crate::run::scratch_root(&state, request).expect("planned scratch");
        let relative = scratch.strip_prefix(&state).expect("private scratch").display();
        let archived = format!("runs/{}/{}", request.identity.run, request.identity.attempt);
        fs::write(
            state.join("attempt"),
            format!("umask 077; mkdir -p '{relative}' '{archived}'; cp \"$1\" '{archived}/prepared.json'; printf transient > '{relative}/transient'; printf interrupted; exit 1\n"),
        )
        .expect("fake attempt program");
        let machine = Machine { path: "/usr/bin:/bin".into(), rustup_home: None };
        let recovered = dispatch(&guard, &machine, &state, &Clock::new(), Path::new("/bin/sh"), request, 1_000_000_000)
            .expect("settled missing result");
        assert!(matches!(recovered.outcome.end, End::HarnessError { .. }));
        assert!(matches!(recovered.resources.cpu_ms, Measure::Unavailable { .. }));
        let directory = state.join("runs").join(&request.identity.run).join(&request.identity.attempt);
        assert_eq!(fs::read(directory.join("dispatcher-stdout.log")).expect("retained output"), b"interrupted");
        assert!(!scratch.exists());
        assert!(recovered.artifacts.iter().any(|artifact| {
            artifact.path == Path::new("prepared.json")
                && artifact.sha256 == crate::arm::binary_digest(&directory.join(&artifact.path)).expect("digest")
        }));
        fs::remove_dir_all(state).expect("remove dispatch control");
    }
}

#[cfg(test)]
mod native_suite_pin_tests {
    use super::*;
    use sha2::Digest;

    #[test]
    fn native_built_suite_pin_verifies_full_arm_commit_and_binary() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pin_file = root.join("agents/smith/everything.pin.toml");
        let pin = crate::read_configuration(&pin_file).expect("native built-arm pin");
        let task_file = root.join("tasks/probes/first-request/task.toml");
        let task = crate::read_task(&task_file).expect("frozen author input");
        let mut suite = crate::read_suite(&root.join("suites/calibrate.toml")).expect("suite");
        suite.tasks = vec!["probes/first-request".into()];
        let mut identity =
            crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("identity fixture").identity;
        let commit = "a".repeat(40);
        let bytes = b"synthetic identity fixture; never executed\n";
        let digest = format!("{:x}", sha2::Sha256::digest(bytes));
        let directory = std::env::temp_dir()
            .join(format!("smith-native-suite-pin-{}", std::process::id()))
            .join(crate::arm::arm_name(&commit, &digest).expect("arm name"));
        fs::create_dir_all(&directory).expect("isolated identity fixture");
        fs::write(directory.join("smith"), bytes).expect("non-executed fixture");
        let arm = crate::arm::BuiltArm {
            version: 1,
            commit: commit.clone(),
            sha256: digest.clone(),
            toolchain: "synthetic identity control".into(),
            binaries: vec![crate::arm::ArmBinary { name: "smith".into(), sha256: digest.clone() }],
        };
        fs::write(directory.join("arm.json"), serde_json::to_vec(&arm).expect("arm manifest"))
            .expect("private test manifest");
        identity.agent = crate::Agent::Smith;
        identity.provider = crate::Provider::Codex;
        identity.agent_version = commit;
        identity.binary_sha256 = digest.clone();
        identity.configuration_sha256 = pin.sha256.clone();
        identity.task = "probes/first-request".into();
        identity.task_version = task.version;
        identity.seed_sha256 = task.seed_sha256;
        identity.model.requested_model = "gpt-6-luna".into();
        identity.model.requested_effort = "low".into();
        let mut request = Prepared {
            task: task_file,
            identity,
            program: directory.join("smith"),
            arguments: vec![],
            codex_home: None,
            configuration: Some(pin_file),
            teardown_ms: 1,
            grace_ms: 1,
            settlement_ms: 1,
            setup_deadline_seconds: 1,
            max_stdout_bytes: 1,
            max_stderr_bytes: 1,
            disk_estimate_bytes: 1,
            force_walk: true,
        };
        let suite_file = root.join("suites/calibrate.toml");
        let valid = check_requests(&suite_file, &suite, std::slice::from_ref(&request));
        // Do not execute or read credentials; this is the real admission seam.
        assert!(valid.is_ok(), "verified native commit must match built-arm pin: {valid:?}");
        request.identity.binary_sha256 = "0".repeat(64);
        assert_eq!(configuration_matches(&pin, &request), Ok(false), "changed binary identity refused by admission");
        request.identity.binary_sha256 = digest;
        unknown_model_admission(root, &mut request);
        fs::remove_dir_all(directory.parent().expect("fixture owner")).expect("cleanup");
    }

    fn unknown_model_admission(root: &Path, request: &mut Prepared) {
        let suite_file = root.join("suites/probes.toml");
        let mut suite = probe_suite(&suite_file, "probes/failures/unknown-model", 1).expect("failure suite");
        suite.tasks = vec!["probes/failures/unknown-model".into()];
        let pin_file = root.join("agents/smith/everything.pin.toml");
        let pin = crate::read_configuration(&pin_file).expect("failure pin");
        request.task = root.join("tasks/probes/failures/task.toml");
        let task =
            crate::read_task(&request.task).expect("failure task").with_variant("unknown-model").expect("variant");
        request.configuration = Some(pin_file);
        request.identity.configuration_sha256 = pin.sha256;
        request.identity.task = "probes/failures/unknown-model".into();
        request.identity.task_version = task.version;
        request.identity.seed_sha256 = task.seed_sha256;
        // The actual closed admission seam refuses a tier-labelled secret override.
        assert!(check_requests(&suite_file, &suite, std::slice::from_ref(request)).is_err());
        request.identity.model.requested_model = "codex/smith-bench-nonexistent-model-2026-10-11".into();
        let valid = check_requests(&suite_file, &suite, std::slice::from_ref(request));
        assert!(valid.is_ok(), "truthful authored provider rejection identity admitted: {valid:?}");
    }
}

fn cache_verdicts(
    results: &[AttemptResult],
    incomplete: &mut bool,
    probes: &mut BTreeMap<String, crate::ProbeVerdict>,
) {
    let mut caches: BTreeMap<String, Vec<&AttemptResult>> = BTreeMap::new();
    for result in results {
        if crate::cache::selected(result) {
            caches.entry(format!("{}/{}", result.identity.task, result.identity.arm)).or_default().push(result);
        }
    }
    for (key, group) in caches {
        let verdicts = crate::cache::triplets(&group);
        *incomplete |= verdicts.iter().any(|(verdict, _)| *verdict == crate::Judgment::Incomplete);
        let pass = verdicts.iter().all(|(verdict, _)| *verdict == crate::Judgment::Pass);
        probes.insert(key, if pass { crate::ProbeVerdict::Passed } else { crate::ProbeVerdict::Regression });
    }
}

#[cfg(test)]
mod cache_order_tests {
    use super::*;

    #[test]
    fn logical_calibration_requires_thirty_runs_but_one_suite_triplet_requires_three() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut suite = crate::read_suite(&root.join("suites/probes.toml")).expect("cache suite");
        suite.repetitions = 10;
        let result = crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("fixture");
        let requests: Vec<_> = (0..30)
            .map(|position| {
                let mut identity = result.identity.clone();
                identity.attempt = format!("{position:02}");
                Prepared {
                    task: root.join("tasks/probes/cache-reuse/task.toml"),
                    identity,
                    program: "/nonexecuted".into(),
                    arguments: vec![],
                    codex_home: None,
                    configuration: None,
                    teardown_ms: 0,
                    grace_ms: 1,
                    settlement_ms: 1,
                    setup_deadline_seconds: 1,
                    max_stdout_bytes: 4096,
                    max_stderr_bytes: 4096,
                    disk_estimate_bytes: 4096,
                    force_walk: true,
                }
            })
            .collect();
        assert_eq!(order(&suite, requests.clone(), 1, true).expect("ten triplets").len(), 30);
        assert!(order(&suite, requests[..29].to_vec(), 1, true).is_err());
        assert!(ordered(&suite, requests.clone(), 1).is_err(), "suite task's authored repetition count is three");
        assert_eq!(ordered(&suite, requests[..3].to_vec(), 1).expect("one triplet").len(), 3);
        suite.repetitions = 1;
        assert_eq!(order(&suite, requests[..3].to_vec(), 1, true).expect("one control triplet").len(), 3);
    }
}

#[cfg(test)]
mod agent_task_tests {
    use super::*;

    #[test]
    fn a_prepared_pin_runs_only_its_selected_tasks() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let suite_file = root.join("suites/probes.toml");
        let mut suite = crate::read_suite(&suite_file).expect("suite");
        suite.tasks = vec!["probes/smoke-tools".into(), "probes/large-write".into()];
        suite.agents.retain(|agent| agent.agent == crate::Agent::Codex);
        suite.agents[0].tasks = vec!["probes/large-write".into()];
        let pin_file = root.join("agents/codex/standard.pin.toml");
        let pin = crate::read_configuration(&pin_file).expect("pin");
        let mut identity =
            crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("identity fixture").identity;
        identity.task = "probes/large-write".into();
        identity.agent = crate::Agent::Codex;
        identity.provider = crate::Provider::Codex;
        identity.agent_version = pin.pin.version.clone();
        identity.configuration_sha256 = pin.sha256.clone();
        identity.model.requested_model = "gpt-6-luna".into();
        identity.model.requested_effort = "low".into();
        let mut request = Prepared {
            task: root.join("tasks/probes/large-write/task.toml"),
            identity,
            program: "/nonexecuted".into(),
            arguments: vec![],
            codex_home: None,
            configuration: Some(pin_file),
            teardown_ms: 0,
            grace_ms: 1,
            settlement_ms: 1,
            setup_deadline_seconds: 1,
            max_stdout_bytes: 1,
            max_stderr_bytes: 1,
            disk_estimate_bytes: 1,
            force_walk: true,
        };
        assert_eq!(selected_configuration_matches(&suite.agents, std::slice::from_ref(&pin), &request), Ok(true));
        request.identity.task = "probes/smoke-tools".into();
        request.task = root.join("tasks/probes/smoke-tools/task.toml");
        assert_eq!(selected_configuration_matches(&suite.agents, &[pin], &request), Ok(false));
    }
}
