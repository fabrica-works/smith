//! Before/after evidence uses verified frozen arms and the prepared suite dispatcher.
//! Builds are an explicit heavy preparation step; live dispatch never builds or
//! reads agent credentials itself (benchmarks.md sections4.2/10; 00.12 sketches).

use std::path::Path;

use crate::arm::{ArmRef, Build};
use crate::batch::Batch;
use crate::{AttemptResult, End, Judgment, judge_suite};

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

/// Admit one complete prepared arm before either comparison phase dispatches.
pub(crate) fn prepared(suite: &Path, requests: &Path, seed: u64, probe: &str, attempts: u32) -> Result<Build, String> {
    let prepared = crate::batch::probe_requests(suite, requests, seed, probe, attempts)?;
    prepare::one_arm(&prepared, prepared.len())
}

pub(crate) fn actual(build: &Build, results: &[AttemptResult], batch: &Batch) -> Result<ProbeRun, String> {
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
        verdict: if quarantined
            || results.iter().any(|result| matches!(result.outcome.end, End::HarnessError { .. } | End::Refused { .. }))
        {
            Judgment::Incomplete
        } else {
            judge_suite(results, false)
        },
        setup_refused: results.iter().any(|result| {
            matches!(result.outcome.end, End::Refused { .. })
                && matches!(result.timing.process_wall_ms, crate::Measure::Unavailable { .. })
        }),
    })
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
        batch.results[0].outcome.end = End::Refused { setup: "named refusal".into() };
        batch.results[0].timing.process_wall_ms = crate::Measure::unavailable("product invocation did not run");
        assert!(
            actual(&build, &batch.results, &batch).expect("setup refusal").setup_refused,
            "neutral named checks survive even when setup refuses"
        );
        batch.results[0].timing.process_wall_ms = crate::Measure::Observed { value: 1 };
        assert!(!actual(&build, &batch.results, &batch).expect("measured product refusal").setup_refused);
        batch.results[0].identity.binary_sha256.push('0');
        assert!(
            actual(&build, &batch.results, &batch).is_err(),
            "a changed binary identity is never reported as a pass"
        );
    }
}
