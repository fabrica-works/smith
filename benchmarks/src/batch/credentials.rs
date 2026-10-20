//! One logical credentials probe is two simultaneous attempt processes.
//! Only generation numbers survive the product's bounded private-record reader
//! and codec; no token value reaches a diagnostic or result (benchmarks.md, 11.1).

use std::path::{Path, PathBuf};

use smith_local_shell::credentials::record_generation;

use super::{AttemptResult, Clock, End, Guard, Machine, Measure, Prepared, dispatch_command, dispatch_result};

pub(super) fn selected(request: &Prepared) -> bool {
    request.identity.task == "probes/credentials"
}

fn record_path(request: &Prepared) -> Result<PathBuf, String> {
    if request.identity.agent != crate::Agent::Smith || request.identity.provider != crate::Provider::Codex {
        return Err("the credentials pair requires smith's chatgpt account".into());
    }
    Ok(request
        .codex_home
        .as_ref()
        .and_then(|home| home.parent())
        .ok_or("credentials pair needs the harness login parent")?
        .join("smith/tokens/chatgpt"))
}

fn proof(
    ends: &[End; 2],
    before: &Result<Option<u64>, String>,
    after: &Result<Option<u64>, String>,
    record: &Path,
) -> Measure<bool> {
    match (before, after) {
        (Ok(Some(before)), Ok(Some(after))) => {
            Measure::Observed { value: ends.iter().all(|end| *end == End::Completed) && after > before }
        }
        (Ok(None), _) | (_, Ok(None)) => {
            Measure::unavailable(format!("missing harness sign-in record: {}", record.display()))
        }
        (Err(reason), _) | (_, Err(reason)) => Measure::unavailable(reason),
    }
}

pub(super) fn run(
    guard: &Guard,
    machine: &Machine,
    state: &Path,
    executable: &Path,
    request: &Prepared,
    remaining: u64,
) -> Result<Vec<AttemptResult>, String> {
    let path = record_path(request)?;
    let before = record_generation(&path);
    let mut peer = request.clone();
    peer.identity.attempt.push_str("-peer");
    let (left, left_existed) = dispatch_command(guard, machine, state, executable, request)?;
    let (right, right_existed) = dispatch_command(guard, machine, state, executable, &peer)?;
    let command = skein_world::end_to_end::Command {
        program: crate::isolate::program(machine, state, Path::new("sh"))?,
        arguments: vec![
            "-c".into(),
            PAIR.into(),
            "credentials-pair".into(),
            executable.as_os_str().into(),
            left.arguments[1].clone(),
            right.arguments[1].clone(),
        ],
        environment: left.environment,
        directory: left.directory,
    };
    let process = crate::process::execute(
        &Clock::new(),
        command,
        Box::new([]),
        crate::process::Capture::Raw,
        crate::process::Bounds {
            hard: skein_lib::Duration::from_nanos(remaining),
            teardown: skein_lib::Duration::ZERO,
            grace: skein_lib::Duration::from_millis(request.grace_ms),
            settlement: skein_lib::Duration::from_millis(request.settlement_ms),
            stdout: request.max_stdout_bytes.checked_mul(2).ok_or("pair stdout bound overflow")?,
            stderr: request.max_stderr_bytes.checked_mul(2).ok_or("pair stderr bound overflow")?,
            force_walk: request.force_walk,
        },
    )?;
    let mut results = [
        dispatch_result(guard, state, request, &process, left_existed)?,
        dispatch_result(guard, state, &peer, &process, right_existed)?,
    ];
    let after = record_generation(&path);
    let passed = proof(&[results[0].outcome.end.clone(), results[1].outcome.end.clone()], &before, &after, &path);
    for result in &mut results {
        let check = result
            .outcome
            .checks
            .iter_mut()
            .find(|check| check.check == "credential-refresh")
            .ok_or("credentials attempt omitted its deciding refresh check")?;
        check.passed = passed.clone();
        if let (Ok(Some(before)), Ok(Some(after))) = (&before, &after) {
            check.evidence = vec![format!("generation {before} -> {after}; two settled concurrent attempts")];
        }
        let directory = state.join("runs").join(&result.identity.run).join(&result.identity.attempt);
        let file = guard.write_path(&directory.join("result.json"))?;
        std::fs::write(file, crate::render_result(result)?).map_err(|_| "cannot persist credentials pair verdict")?;
    }
    Ok(results.into())
}

// Each attempt owns its supervisor in a separate harness process. The shell
// waits for both, including a nonzero first exit, under one outer tree owner.
const PAIR: &str = r#""$1" attempt "$2" &
left_pid=$!
"$1" attempt "$3" &
right_pid=$!
wait "$left_pid"
left_status=$?
wait "$right_pid"
right_status=$?
if [ "$left_status" -ne 0 ]; then exit "$left_status"; fi
exit "$right_status"
"#;

pub(super) fn validate(task: &crate::Task) -> Result<(), String> {
    if task.smith.get("deployment.refresh_lead").and_then(toml::Value::as_integer) != Some(34_560_000)
        || !task.outcome.contains(&crate::OutcomeCheck::CredentialRefresh)
    {
        return Err("credentials probe must force refresh and decide on the pair".into());
    }
    for (ends, before, after, expected) in [
        ([End::Completed, End::Completed], 4, 4, false),
        ([End::Completed, End::Completed], 4, 5, true),
        ([End::Completed, End::Refused { setup: "refresh reused".into() }], 4, 5, false),
    ] {
        let refresh = proof(&ends, &Ok(Some(before)), &Ok(Some(after)), Path::new("synthetic-record"));
        let ended = crate::grade::Ended {
            end: ends[0].clone(),
            exit: Measure::Observed { value: crate::Exit::Code(0) },
            answer: Some("CREDENTIALS_REFRESH_OK".into()),
            last_line: None,
            credential_refresh: refresh,
        };
        let snapshot = crate::grade::Snapshot::new();
        let checks = crate::grade::checks(Path::new("."), &snapshot, &snapshot, &task.outcome, &ended);
        let passed = task.deciding.iter().all(|name| {
            checks.iter().any(|check| check.check == *name && check.passed == (Measure::Observed { value: true }))
        });
        if passed != expected {
            return Err("credentials probe's offline pair control differs".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
