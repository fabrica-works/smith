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
        | OutcomeCheck::FileSize { .. }
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
    pub(super) fn task(&mut self, task: &Task, agent: crate::Agent) {
        self.checks = task.outcome.iter().filter(|check| neutral(check)).cloned().collect();
        self.waived.clone_from(&task.waives);
        self.deciding.clone_from(&task.deciding);
        if agent != crate::Agent::Smith {
            self.deciding.retain(|name| {
                !["stream-whole", "paired", "spend-sums", "nothing-lost"].contains(&name.as_str())
                    && !task.event.iter().any(|check| check.name() == name && check.name() != "cache-reuse")
            });
        }
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

#[cfg(test)]
mod cache_pair_tests {
    use super::*;

    #[test]
    fn a_codex_outcome_half_keeps_only_outcome_checks_deciding() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let task = crate::read_task(&root.join("tasks/probes/smoke-tools/task.toml")).expect("task");
        let mut neutral = Neutral::default();
        neutral.task(&task, crate::Agent::Codex);
        assert!(!neutral.deciding.contains(&"tool-call".into()));
        assert!(neutral.deciding.contains(&"pattern:smoke-env.txt".into()));
        assert_eq!(
            neutral.deciding,
            ["end", "exit-code", "answer", "pattern:smoke-env.txt", "tree-ended", "protected-unchanged"]
        );
    }

    #[test]
    fn codex_cache_pair_keeps_neutral_outcomes_and_triplet_statistics() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let task = crate::check_task(&root.join("tasks/probes/cache-reuse/task.toml"), &root.join("../docs/design"))
            .expect("task");
        let mut neutral = Neutral::default();
        neutral.task(&task, crate::Agent::Codex);
        assert_eq!(neutral.deciding, ["end", "exit-code", "answer", "cache-reuse"]);
        neutral.task(&task, crate::Agent::Smith);
        assert_eq!(neutral.deciding, task.deciding);
    }
}
