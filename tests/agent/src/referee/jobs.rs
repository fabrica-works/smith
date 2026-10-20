//! Job terminals are observed before the session's close (protocol/events.md, section 5).
use super::{Judge, Meeting};
use smith_domain::{session::FactKind, tools};

impl Meeting {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the referee consumes the observed fact at its dispatch boundary"
    )]
    pub(super) fn job_fact(&mut self, fact: FactKind, judge: &mut Judge<&'static str, ()>) {
        match fact {
            FactKind::Tools { opener, fact: tools::FactKind::Answered { tool, job: Some(job), verdict, .. } } => {
                if tool == tools::Tool::Shell && verdict == tools::Verdict::Running {
                    judge
                        .check(self.jobs.insert((opener, job.0), false).is_none(), "a running shell names a fresh job");
                } else {
                    let ended = self.jobs.get_mut(&(opener, job.0)).expect("a job answer follows its running shell");
                    judge.check(!*ended, "no job answer follows its terminal");
                    if verdict != tools::Verdict::Running {
                        *ended = true;
                    }
                }
            }
            FactKind::JobEnded { opener, job, .. } => {
                let ended = self.jobs.get_mut(&(opener, job.0)).expect("an ended job follows its running shell");
                judge.check(!*ended, "a job's end is told once");
                *ended = true;
            }
            FactKind::Ended { opener, .. } => {
                judge.check(
                    self.jobs.iter().all(|((owner, _), ended)| *owner != opener || *ended),
                    "every job terminal precedes its conversation's close",
                );
            }
            FactKind::Tools { .. }
            | FactKind::Opened { .. }
            | FactKind::CompactionStarted { .. }
            | FactKind::CompactionEnded { .. }
            | FactKind::CompletionStarted { .. }
            | FactKind::CompletionAnswered { .. }
            | FactKind::ReasoningDropped { .. }
            | FactKind::CompletionFailed { .. }
            | FactKind::CompletionCancelled { .. }
            | FactKind::CompletionRetried { .. }
            | FactKind::BuiltinStarted { .. }
            | FactKind::BuiltinAnswered { .. }
            | FactKind::DelegateStarted { .. }
            | FactKind::DelegateAnswered { .. }
            | FactKind::DelegateCancelled { .. }
            | FactKind::DelegateWithheld { .. }
            | FactKind::Yielded { .. }
            | FactKind::Used { .. } => {}
        }
    }
}
