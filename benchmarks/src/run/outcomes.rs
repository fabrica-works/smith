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
        | OutcomeCheck::CredentialRefresh => true,
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

    pub(super) fn program(&mut self) {
        self.deciding.retain(|name| name != "tool-call");
        if !self.deciding.iter().any(|name| name == "stream-whole") {
            self.deciding.push("stream-whole".into());
        }
    }

    pub(super) fn observed(
        &mut self,
        observed: Result<Ended, String>,
        process: &ProcessResult,
        outcome: &Outcome,
    ) -> Result<&Ended, String> {
        if let Some(crate::agent::Observing::Smith(observing)) = process.observing.as_deref()
            && observing.prepared.program_source()
        {
            self.program();
        }
        match observed {
            Ok(ended) => {
                self.ended = Some(ended);
                Ok(self.ended.as_ref().expect("stored settled evidence"))
            }
            Err(error) => {
                self.ended = Some(super::settled_outcome(
                    process,
                    outcome,
                    Measure::unavailable("the credentials pair has no complete proof"),
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
            credential_refresh: Measure::unavailable("the credentials pair has not settled"),
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
        if !self.checks.iter().any(|check| matches!(check, OutcomeCheck::End { .. })) {
            result.outcome.checks.push(crate::CheckVerdict {
                check: "end".into(),
                passed: Measure::Observed { value: result.outcome.end == crate::End::Completed },
                evidence: Vec::new(),
            });
        }
        if let Some(grade) = &result.outcome.grade {
            result.outcome.checks.push(crate::CheckVerdict {
                check: "grade".into(),
                passed: grade.clone(),
                evidence: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod cache_pair_tests {
    use super::*;

    #[test]
    fn program_bash_call_passes_smoke_tools_deciding_checks() {
        let task: Task = toml::from_str(include_str!("../../tasks/probes/smoke-tools/task.toml")).expect("smoke task");
        let mut observing = crate::agent::smith::recorded_program();
        assert_eq!(observing.counts().tool_calls.get("Bash"), Some(&Measure::Observed { value: 1 }));
        let mut result: AttemptResult =
            serde_json::from_str(include_str!("../../tests/recorded/results/attempt.json")).expect("result fixture");
        result.outcome.end = crate::End::Completed;
        result.outcome.exit = Measure::Observed { value: crate::Exit::Code(0) };
        result.outcome.checks = observing.checks(&task.event, &task.prompt);
        result.health = observing.health(crate::Exit::Code(0), crate::Forced::No).expect("health");
        let mut neutral = Neutral::default();
        neutral.task(&task, crate::Agent::Smith);
        neutral.program();
        // These workspace and tree observations were settled by the fixture's caller.
        for check in ["pattern:smoke-env.txt", "tree-ended", "protected-unchanged"] {
            result.outcome.checks.push(crate::CheckVerdict {
                check: check.into(),
                passed: Measure::Observed { value: true },
                evidence: Vec::new(),
            });
        }
        neutral.ended = Some(Ended {
            end: crate::End::Completed,
            exit: result.outcome.exit.clone(),
            last_line: None,
            answer: Some("ALPHA".into()),
            credential_refresh: Measure::unavailable("not a pair"),
        });
        neutral.finish(&mut result);
        assert_eq!(
            result.outcome.deciding,
            task.deciding.iter().filter(|name| *name != "tool-call").cloned().collect::<Vec<_>>()
        );
        let tools: Vec<_> = result.outcome.checks.iter().filter(|check| check.check == "tool-call").collect();
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().all(|check| check.passed == Measure::unavailable("Claude Code's built-in tools")));
        assert_eq!(crate::judge(&result), crate::Judgment::Pass, "{:?}", result.outcome.checks);
        for reason in ["agent trace open: missing", "agent trace read: denied"] {
            observing.agent_trace(Err(reason.into()));
            result.outcome.checks.retain(|check| check.check != "stream-whole");
            let whole = observing
                .health(crate::Exit::Code(0), crate::Forced::No)
                .expect("health")
                .passive_checks
                .into_iter()
                .find(|check| check.check == "stream-whole")
                .expect("required trace");
            result.outcome.checks.push(whole);
            assert_eq!(crate::judge(&result), crate::Judgment::Incomplete);
        }
    }

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
