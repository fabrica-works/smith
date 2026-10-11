//! Before/after evidence uses verified frozen arms and the prepared suite dispatcher.
//! Builds are an explicit heavy preparation step; live dispatch never builds or
//! reads agent credentials itself (benchmarks.md sections4.2/10; 00.12 sketches).

use std::fmt::Write;
use std::path::Path;

use crate::arm::{ArmRef, Build};
use crate::{AttemptResult, End, Judgment, judge, judge_suite};

mod prepare;

/// One verified arm's actual probe run, supplied by the common dispatcher.
#[derive(Clone, Debug)]
pub struct ProbeRun {
    pub arm: ArmRef,
    pub run: String,
    pub verdict: Judgment,
    /// Setup refusal is distinct from an expected product entrance refusal.
    pub setup_refused: bool,
}

/// The before and after findings supplied to a change's author.
#[derive(Clone, Debug)]
pub struct FixReport {
    pub probe: String,
    pub before: ProbeRun,
    pub after: ProbeRun,
    pub shown: Shown,
}

/// What the actual deciding observations establish about a fix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// Deciding checks failed before and passed after.
    Effect,
    /// The before already satisfies the probe.
    PassesBefore,
    /// The after has a deciding failure.
    FailsAfter,
    /// Before inference, the parent refused its preparation.
    BeforeRefused,
    /// At least one deciding observation is unavailable.
    Unjudged,
}

fn shown(before: &ProbeRun, after: &ProbeRun) -> Shown {
    if after.verdict == Judgment::Fail {
        return Shown::FailsAfter;
    }
    if before.setup_refused {
        return Shown::BeforeRefused;
    }
    match (before.verdict, after.verdict) {
        (Judgment::Fail, Judgment::Pass) => Shown::Effect,
        (Judgment::Pass, Judgment::Pass) => Shown::PassesBefore,
        (Judgment::Incomplete, _) | (_, Judgment::Incomplete) => Shown::Unjudged,
        (_, Judgment::Fail) => Shown::FailsAfter,
    }
}

/// Classify supplied actual runs without discarding failure or unavailable evidence.
#[must_use]
pub fn report(probe: String, before: ProbeRun, after: ProbeRun) -> FixReport {
    FixReport { probe, shown: shown(&before, &after), before, after }
}

fn arm_name(arm: &ArmRef) -> String {
    match arm {
        ArmRef::Built(build) => format!("{} sha256={}", build.arm.commit, build.arm.sha256),
        ArmRef::Installed { program } => format!("installed {}", program.display()),
    }
}

/// Print copyable commit-body evidence with both run identifiers and truthful findings.
#[must_use]
pub fn body(report: &FixReport) -> String {
    format!(
        "Probe: {}\nBefore: {} run={} {:?}\nAfter: {} run={} {:?}\nShown: {:?}\n",
        report.probe,
        arm_name(&report.before.arm),
        report.before.run,
        report.before.verdict,
        arm_name(&report.after.arm),
        report.after.run,
        report.after.verdict,
        report.shown
    )
}

/// Build both frozen arms only during the caller's heavy preparation step.
pub fn build(repository: &Path, parent: &str, fix: &str) -> Result<(Build, Build), String> {
    Ok((crate::arm::build(repository, parent)?, crate::arm::build(repository, fix)?))
}

fn actual(build: &Build, results: &[AttemptResult], batch: &crate::batch::Batch) -> Result<ProbeRun, String> {
    if results.is_empty() {
        return Err("probe arm produced no results".into());
    }
    if results.iter().any(|result| {
        result.identity.agent_version != build.arm.commit || result.identity.binary_sha256 != build.arm.sha256
    }) {
        return Err("probe result differs from its verified binary".into());
    }
    let run = results
        .iter()
        .map(|result| format!("{}/{}", result.identity.run, result.identity.attempt))
        .collect::<Vec<_>>()
        .join(",");
    let quarantined = results.iter().any(|result| {
        matches!(
            batch.probes.get(&format!("{}/{}", result.identity.task, result.identity.arm)),
            None | Some(crate::ProbeVerdict::Rerun | crate::ProbeVerdict::Quarantined)
        )
    });
    Ok(ProbeRun {
        arm: ArmRef::Built(build.clone()),
        run,
        verdict: if quarantined { Judgment::Incomplete } else { judge_suite(results, false) },
        setup_refused: results
            .iter()
            .any(|result| matches!(result.outcome.end, End::Refused { .. }) && result.outcome.checks.is_empty()),
    })
}

/// Run prepared before/after requests on verified arms; never prepare an unstated invocation.
pub fn run(probe: &str, parent: &str, fix: &str, prepared: &Path, seed: u64) -> Result<FixReport, String> {
    let suite = Path::new(env!("CARGO_MANIFEST_DIR")).join("suites/calibrate.toml");
    let requests = crate::batch::probe_requests(&suite, prepared, seed, probe, 1)?;
    prepare::inputs(&requests)?;
    let mut before = None;
    let mut after = None;
    for request in &requests {
        let build = prepare::build(request)?;
        if build.arm.commit == parent {
            before = Some(build);
        } else if build.arm.commit == fix {
            after = Some(build);
        } else {
            return Err("prepared fix has an arm outside the stated before/after".into());
        }
    }
    let before = before.ok_or("prepared fix lacks the parent arm")?;
    let after = after.ok_or("prepared fix lacks the changed arm")?;
    if parent == fix {
        return Err("fix before and after require distinct commits".into());
    }
    let batch = crate::batch::run_probe(&suite, prepared, seed, probe, 1)?;
    let before_results =
        batch.results.iter().filter(|result| result.identity.agent_version == parent).cloned().collect::<Vec<_>>();
    let after_results =
        batch.results.iter().filter(|result| result.identity.agent_version == fix).cloned().collect::<Vec<_>>();
    let mut report =
        report(probe.into(), actual(&before, &before_results, &batch)?, actual(&after, &after_results, &batch)?);
    if batch.incomplete && report.shown == Shown::Effect {
        report.shown = Shown::Unjudged;
    }
    Ok(report)
}

/// Judge ten changed-arm repetitions and a failing control without claiming calibration from unavailable.
pub fn calibrate(probe: &str, changed: &Path, control: &Path, seed: u64) -> Result<(Judgment, String), String> {
    let suite = Path::new(env!("CARGO_MANIFEST_DIR")).join("suites/calibrate.toml");
    let after_requests = crate::batch::probe_requests(&suite, changed, seed, probe, 10)?;
    let before_requests = crate::batch::probe_requests(&suite, control, seed, probe, 1)?;
    let after_arm = prepare::one_arm(&after_requests, 10)?;
    let before_arm = prepare::one_arm(&before_requests, 1)?;
    if !prepare::same_input(&after_requests[0].identity, &before_requests[0].identity) {
        return Err("calibration control changes frozen task inputs".into());
    }
    if after_arm.arm == before_arm.arm {
        return Err("calibration control must differ from the changed arm".into());
    }
    let after = crate::batch::run_probe(&suite, changed, seed, probe, 10)?;
    let before = crate::batch::run_probe(&suite, control, seed, probe, 1)?;
    let after_verdict = actual(&after_arm, &after.results, &after)?.verdict;
    let before_verdict = actual(&before_arm, &before.results, &before)?.verdict;
    let verdict = match (after_verdict, before_verdict) {
        (Judgment::Pass, Judgment::Fail) if after.results.len() == 10 && !after.incomplete && !before.incomplete => {
            Judgment::Pass
        }
        (Judgment::Incomplete, _) | (_, Judgment::Incomplete) => Judgment::Incomplete,
        (_, _) => Judgment::Fail,
    };
    let mut text = format!("Calibration: {probe} {verdict:?}\n");
    for (phase, batch) in [("after", after), ("control", before)] {
        for result in batch.results {
            writeln!(
                text,
                "{phase}: {} sha256={} run={}/{} {:?}",
                result.identity.agent_version,
                result.identity.binary_sha256,
                result.identity.run,
                result.identity.attempt,
                judge(&result)
            )
            .expect("string formatting");
        }
    }
    Ok((verdict, text))
}

#[cfg(test)]
pub(crate) fn probe_cases() {
    prepare::comparison_case();
    tests::rerun_quarantine_cannot_be_evidence_of_a_fix_or_calibration();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProbeVerdict;
    use std::collections::BTreeMap;

    pub(super) fn rerun_quarantine_cannot_be_evidence_of_a_fix_or_calibration() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut value = crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("fixture");
        value.outcome.end = End::Completed;
        value.outcome.grade = None;
        value.outcome.deciding = vec!["answer".into()];
        value.outcome.checks = vec![crate::CheckVerdict {
            check: "answer".into(),
            passed: crate::Measure::Observed { value: false },
            evidence: vec![],
        }];
        let build = Build {
            directory: "/nonexecuted-identity-fixture".into(),
            arm: crate::arm::BuiltArm {
                version: 1,
                commit: value.identity.agent_version.clone(),
                sha256: value.identity.binary_sha256.clone(),
                toolchain: "not a build; report identity fixture".into(),
                binaries: vec![],
            },
        };
        let key = format!("{}/{}", value.identity.task, value.identity.arm);
        let mut batch =
            crate::batch::Batch { results: vec![value.clone()], incomplete: false, probes: BTreeMap::new() };
        assert_eq!(actual(&build, &batch.results, &batch).expect("unknown rerun").verdict, Judgment::Incomplete);
        for verdict in [ProbeVerdict::Rerun, ProbeVerdict::Quarantined] {
            batch.probes.insert(key.clone(), verdict);
            assert_eq!(actual(&build, &batch.results, &batch).expect("quarantine").verdict, Judgment::Incomplete);
        }
        batch.probes.insert(key.clone(), ProbeVerdict::Regression);
        assert_eq!(actual(&build, &batch.results, &batch).expect("two actual failures").verdict, Judgment::Fail);
        batch.results[0].outcome.checks[0].passed = crate::Measure::Observed { value: true };
        batch.probes.insert(key, ProbeVerdict::Passed);
        assert_eq!(actual(&build, &batch.results, &batch).expect("pass").verdict, Judgment::Pass);
        batch.results[0].identity.binary_sha256.push('0');
        assert!(
            actual(&build, &batch.results, &batch).is_err(),
            "a changed binary identity is never reported as a pass"
        );
    }
}

