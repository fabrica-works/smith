//! Both phases must be admitted before dispatch; frozen inputs cannot silently differ.
//! No private workspace or credential is created by these checks (benchmarks.md3.2/4.2).

use crate::Identity;
use crate::arm::Build;
use crate::run::Prepared;

pub(super) fn same_input(left: &Identity, right: &Identity) -> bool {
    left.task == right.task
        && left.task_version == right.task_version
        && left.seed_sha256 == right.seed_sha256
        && left.prompt_sha256 == right.prompt_sha256
        && left.configuration_sha256 == right.configuration_sha256
        && left.agent == right.agent
        && left.provider == right.provider
        && left.model.requested_model == right.model.requested_model
        && left.model.requested_effort == right.model.requested_effort
        && left.harness_commit == right.harness_commit
        && left.host == right.host
        && left.cpus == right.cpus
}

pub(super) fn inputs(requests: &[Prepared]) -> Result<(), String> {
    let first = requests.first().ok_or("probe has no prepared requests")?;
    if requests.iter().any(|request| !same_input(&first.identity, &request.identity)) {
        return Err("before/after must use the same frozen task, seed, prompt, configuration, model and host".into());
    }
    Ok(())
}

pub(super) fn build(request: &Prepared) -> Result<Build, String> {
    let build = crate::arm::read_build(&request.program)?;
    if build.arm.commit != request.identity.agent_version || build.arm.sha256 != request.identity.binary_sha256 {
        return Err("prepared probe differs from its verified built arm".into());
    }
    Ok(build)
}

pub(super) fn one_arm(requests: &[Prepared], repetitions: usize) -> Result<Build, String> {
    inputs(requests)?;
    if requests.len() != repetitions {
        return Err(format!("calibration requires {repetitions} physical requests for this logical repetition count"));
    }
    let first = build(&requests[0])?;
    for request in &requests[1..] {
        if build(request)?.arm != first.arm || request.identity.arm != requests[0].identity.arm {
            return Err("one calibration phase must name one immutable arm".into());
        }
    }
    Ok(first)
}

#[cfg(test)]
pub(super) fn comparison_case() {
    tests::comparison_inputs_keep_task_seed_model_and_configuration_immutable();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    pub(super) fn comparison_inputs_keep_task_seed_model_and_configuration_immutable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let value = crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("fixture").identity;
        let mut other = value.clone();
        other.agent_version = "another verified arm".into();
        other.binary_sha256 = "another binary digest".into();
        other.run = "another run".into();
        other.attempt = "another attempt".into();
        assert!(same_input(&value, &other), "arm/run identity is recorded separately");
        for field in
            ["task", "version", "seed", "prompt", "configuration", "model", "effort", "harness", "host", "cpus"]
        {
            let mut changed = other.clone();
            match field {
                "task" => changed.task.push_str("/different"),
                "version" => changed.task_version += 1,
                "seed" => changed.seed_sha256.push('0'),
                "prompt" => changed.prompt_sha256.push('0'),
                "configuration" => changed.configuration_sha256.push('0'),
                "model" => changed.model.requested_model.push_str("-different"),
                "effort" => changed.model.requested_effort.push_str("-different"),
                "harness" => changed.harness_commit.push('0'),
                "host" => changed.host.push_str("-different"),
                "cpus" => changed.cpus.push(100),
                _ => panic!("named comparison field"),
            }
            assert!(!same_input(&value, &changed), "{field}");
        }
    }
}
