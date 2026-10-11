//! Neutral author checks run after final settlement, grading and scratch
//! cleanup, so an eventual harness fault cannot retain a completed end check.

use std::path::Path;

use crate::grade::{Ended, Snapshot};
use crate::process::ProcessResult;
use crate::{AttemptResult, Measure, Outcome, OutcomeCheck, Task};

pub(super) const fn neutral(check: &OutcomeCheck) -> bool {
    match check {
        OutcomeCheck::End { .. }
        | OutcomeCheck::ExitCode { .. }
        | OutcomeCheck::LastLine { .. }
        | OutcomeCheck::Answer { .. }
        | OutcomeCheck::LoginUnchanged => true,
        OutcomeCheck::FileContent { .. }
        | OutcomeCheck::FileDigest { .. }
        | OutcomeCheck::Absent { .. }
        | OutcomeCheck::Pattern { .. }
        | OutcomeCheck::Protected { .. } => false,
    }
}

#[derive(Default)]
pub(super) struct Neutral {
    checks: Vec<OutcomeCheck>,
    deciding: Vec<String>,
    waived: Vec<crate::Passive>,
    ended: Option<Ended>,
}

impl Neutral {
    pub(super) fn task(&mut self, task: &Task) {
        self.checks = task.outcome.iter().filter(|check| neutral(check)).cloned().collect();
        self.waived.clone_from(&task.waives);
        self.deciding.clone_from(&task.deciding);
    }

    pub(super) fn observed(
        &mut self,
        observed: Result<Ended, String>,
        process: &ProcessResult,
        outcome: &Outcome,
    ) -> Result<&Ended, String> {
        match observed {
            Ok(ended) => {
                self.ended = Some(ended);
                Ok(self.ended.as_ref().expect("stored settled evidence"))
            }
            Err(error) => {
                self.ended = Some(super::settled_outcome(
                    process,
                    outcome,
                    Measure::unavailable("credential comparison did not return a complete proof"),
                ));
                Err(error)
            }
        }
    }

    pub(super) fn finish(self, result: &mut AttemptResult) {
        result.outcome.deciding = self.deciding;
        result.health.passive_checks.push(super::passive::protected(&result.outcome.protected_files_changed));
        result.health.passive_checks.push(super::passive::benchmark_unread());
        result.outcome.checks.extend(crate::passive::judge(&result.health, &self.waived, &result.outcome));
        let mut ended = self.ended.unwrap_or_else(|| Ended {
            end: result.outcome.end.clone(),
            exit: result.outcome.exit.clone(),
            last_line: None,
            answer: None,
            login_unchanged: Measure::unavailable("this attempt did not complete a source login comparison"),
        });
        ended.end.clone_from(&result.outcome.end);
        ended.exit.clone_from(&result.outcome.exit);
        result.outcome.checks.extend(crate::grade::checks(
            Path::new("."),
            &Snapshot::new(),
            &Snapshot::new(),
            &self.checks,
            &ended,
        ));
    }
}
