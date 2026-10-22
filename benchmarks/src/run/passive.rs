//! Passive judgments consume the supervisor and collected path evidence only.
//! A reported escape is a violation; missing effects never establish scope.
//! The shared kit owns all process discovery and genuine settlement.
//! Contract: benchmarks.md section11.2.

use std::path::PathBuf;

use crate::process::ProcessResult;
use crate::{CheckVerdict, Measure, Passive};

#[cfg(test)]
use crate::Forced;

fn paths(check: Passive, effects: &Measure<Vec<PathBuf>>, missing: &str) -> CheckVerdict {
    let (passed, evidence) = match effects {
        Measure::Observed { value } => (Measure::Observed { value: value.is_empty() }, value),
        Measure::LowerBound { value, .. } => {
            let passed =
                if value.is_empty() { Measure::unavailable(missing) } else { Measure::Observed { value: false } };
            (passed, value)
        }
        Measure::Unavailable { reason } => {
            return CheckVerdict {
                check: check.name().into(),
                passed: Measure::unavailable(reason.clone()),
                evidence: Vec::new(),
            };
        }
    };
    CheckVerdict {
        check: check.name().into(),
        passed,
        evidence: evidence.iter().map(|path| path.display().to_string()).collect(),
    }
}

pub(super) fn scope(effects: &Measure<Vec<PathBuf>>) -> CheckVerdict {
    paths(Passive::NothingOutside, effects, "the selected stream does not show every filesystem effect")
}

pub(super) fn temporary(result: &mut crate::AttemptResult, changes: Result<Vec<(PathBuf, String)>, String>) {
    let mut check = scope(&result.outcome.writes_outside_workspace);
    match changes {
        Ok(changes) => {
            if !changes.is_empty() {
                let (outside, evidence): (Vec<_>, Vec<_>) = changes.into_iter().unzip();
                let effects = &mut result.outcome.writes_outside_workspace;
                if let Measure::Unavailable { reason } = effects {
                    *effects = Measure::LowerBound { value: Vec::new(), missing: vec![reason.clone()] };
                }
                match effects {
                    Measure::Observed { value } | Measure::LowerBound { value, .. } => {
                        value.extend(outside);
                        value.sort();
                        value.dedup();
                    }
                    Measure::Unavailable { .. } => unreachable!("converted unavailable scope above"),
                }
                check = scope(effects);
                check.evidence.extend(evidence);
            }
            check.evidence.push("User-owned /tmp entries listed before and after the attempt".into());
        }
        Err(reason) => check.evidence.push(format!("User-owned /tmp listing unavailable: {reason}")),
    }
    result.health.passive_checks.retain(|check| check.check != Passive::NothingOutside.name());
    result.health.passive_checks.push(check);
}

pub(super) fn tree(process: &ProcessResult) -> CheckVerdict {
    CheckVerdict {
        check: Passive::TreeEnded.name().into(),
        passed: process.tree.live_at_leader_exit.as_ref().map_or_else(
            || Measure::unavailable("the supervisor did not report children live at leader exit"),
            |live| Measure::Observed { value: live.is_empty() },
        ),
        evidence: vec![format!("{:?}", process.tree)],
    }
}

pub(super) fn protected(changed: &Measure<Vec<PathBuf>>) -> CheckVerdict {
    paths(Passive::ProtectedUnchanged, changed, "protected-file comparison is incomplete")
}

pub(super) fn benchmark_unread() -> CheckVerdict {
    CheckVerdict {
        check: Passive::BenchmarkUnread.name().into(),
        passed: Measure::unavailable("reported calls do not provide complete resolved benchmark-tree read evidence"),
        evidence: Vec::new(),
    }
}

#[cfg(test)]
#[test]
fn protected_and_teardown_measurements_preserve_counterexamples_and_missing_proof() {
    assert_eq!(protected(&Measure::Observed { value: Vec::new() }).passed, Measure::Observed { value: true });
    let changed = Measure::LowerBound { value: vec![PathBuf::from("Cargo.toml")], missing: vec!["cut".into()] };
    assert_eq!(protected(&changed).passed, Measure::Observed { value: false });
    let missing = Measure::LowerBound { value: Vec::new(), missing: vec!["cut".into()] };
    assert!(matches!(protected(&missing).passed, Measure::Unavailable { .. }));
}

#[cfg(test)]
#[test]
fn tree_leftovers_fail_even_after_supervisor_cleanup_and_missing_observation_is_unavailable() {
    use skein_lib::{Duration, Time, Wall};
    use skein_world::end_to_end::{Counts, Method, PeakScope, TreeStatus};
    let mut process = ProcessResult {
        exit: crate::Exit::Code(0),
        forced: Forced::No,
        start: Time::from_nanos(0),
        start_wall: Wall::from_nanos(0),
        end: Time::from_nanos(1),
        answered: None,
        stdout: Vec::new(),
        stderr: Vec::new(),
        trace: Vec::new(),
        observer: crate::agent::codex::Observer::default(),
        observing: None,
        error: None,
        counts: Counts {
            method: Method::Walk,
            user: Duration::ZERO,
            system: Duration::ZERO,
            peak_rss_bytes: 0,
            peak_scope: PeakScope::LargestProcess { baseline_bytes: 0 },
        },
        tree: TreeStatus {
            live_pids: Vec::new(),
            live_at_leader_exit: Some(Vec::new()),
            interrupted_pids: Vec::new(),
            stopped_pids: Vec::new(),
            continued_pids: Vec::new(),
            terminated_pids: Vec::new(),
            killed_pids: Vec::new(),
            forced_cleanup: false,
        },
    };
    assert_eq!(tree(&process).passed, Measure::Observed { value: true });
    process.tree.live_at_leader_exit = Some(vec![42]);
    process.tree.killed_pids = vec![42];
    process.tree.forced_cleanup = true;
    assert_eq!(tree(&process).passed, Measure::Observed { value: false });
    assert!(tree(&process).evidence[0].contains("42"));
    process.tree.live_at_leader_exit = None;
    assert!(matches!(tree(&process).passed, Measure::Unavailable { .. }));
}
