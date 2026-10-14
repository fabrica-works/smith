//! Deciding observations produce pass, fail or incomplete; raw advisory evidence
//! stays archived. Contract: benchmarks.md sections3.2/11 and the 00.12 exit-code steer.

use crate::{AttemptResult, End, Measure};

/// A task's deciding observations, supplied to both command front ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judgment {
    /// Every deciding observation is true.
    Pass,
    /// A deciding observation is false or the attempt failed.
    Fail,
    /// A deciding observation is absent or unavailable.
    Incomplete,
}

impl Judgment {
    /// The stable process exit code, shared by attempt and suite commands.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Fail => 1,
            Self::Incomplete => 2,
        }
    }
}

/// Inspect exact deciding observations, without turning unavailable into success.
#[must_use]
pub fn judge(result: &AttemptResult) -> Judgment {
    let outcome = &result.outcome;
    let expected_end = outcome.deciding.iter().any(|name| name == "end")
        && outcome.checks.iter().any(|check| check.check == "end" && check.passed == Measure::Observed { value: true });
    match outcome.end {
        End::Completed => {}
        End::Failed { .. } | End::Budget { .. } | End::Refused { .. } if expected_end => {}
        End::Failed { .. } | End::Budget { .. } | End::Timeout | End::HarnessError { .. } | End::Refused { .. } => {
            return Judgment::Fail;
        }
    }
    let mut incomplete = outcome.deciding.is_empty();
    for name in &outcome.deciding {
        incomplete |= !outcome.checks.iter().any(|check| check.check == *name);
    }
    for check in &outcome.checks {
        if !outcome.deciding.contains(&check.check) {
            continue;
        }
        match check.passed {
            Measure::Observed { value: false } => return Judgment::Fail,
            Measure::Observed { value: true } => {}
            Measure::LowerBound { .. } | Measure::Unavailable { .. } => incomplete = true,
        }
    }
    if incomplete || crate::cache::selected(result) { Judgment::Incomplete } else { Judgment::Pass }
}

/// A suite fails on any failed run, then reports any remaining incompleteness.
#[must_use]
pub fn judge_suite(results: &[AttemptResult], incomplete: bool) -> Judgment {
    let mut missing = incomplete || results.is_empty();
    let mut caches: std::collections::BTreeMap<(&str, &str), Vec<&AttemptResult>> = std::collections::BTreeMap::new();
    for result in results {
        if crate::cache::selected(result) {
            caches.entry((&result.identity.task, &result.identity.arm)).or_default().push(result);
        } else {
            match judge(result) {
                Judgment::Fail => return Judgment::Fail,
                Judgment::Incomplete => missing = true,
                Judgment::Pass => {}
            }
        }
    }
    for group in caches.values() {
        for (verdict, _) in crate::cache::triplets(group) {
            match verdict {
                Judgment::Fail => return Judgment::Fail,
                Judgment::Incomplete => missing = true,
                Judgment::Pass => {}
            }
        }
    }
    if missing { Judgment::Incomplete } else { Judgment::Pass }
}
