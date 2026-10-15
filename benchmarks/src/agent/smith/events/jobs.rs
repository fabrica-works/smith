//! Jobs are joined within one process, run and conversation, by their number.
//! Only reported terminals bound overlap; no command lifetime is inferred from
//! result text (benchmarks.md, section 11.1; protocol/events.md, section 3.6).

use smith_events::{Event, Verdict};

use crate::{EventCheck, Measure};

use super::Observer;

pub(super) fn check(observer: &Observer, check: &EventCheck) -> Measure<bool> {
    let mut found = false;
    let mut overlap = false;
    let mut unique = true;
    for session in &observer.sessions {
        let mut jobs = std::collections::BTreeSet::new();
        for (index, record) in session.records.iter().enumerate() {
            let Event::ToolCompleted(start) = &record.event else { continue };
            if start.tool.as_ref() != b"shell" || start.verdict != Verdict::Running {
                continue;
            }
            let Some(job) = start.job else { return Measure::Observed { value: false } };
            let paired = session.records[..index].iter().any(|record| {
                matches!(&record.event,
                Event::ToolStarted(value) if value.run == start.run && value.conversation == start.conversation
                    && value.call == start.call && value.tool == start.tool)
            });
            if !paired {
                return Measure::Observed { value: false };
            }
            found = true;
            unique &= jobs.insert((start.run, start.conversation, job));
            let ends: Vec<_> = session
                .records
                .iter()
                .filter(|record| {
                    if let Event::ToolCompleted(end) = &record.event {
                        end.run == start.run
                            && end.conversation == start.conversation
                            && end.job == Some(job)
                            && end.tool.as_ref() == b"job"
                            && matches!(
                                end.verdict,
                                Verdict::Exited | Verdict::TimedOut | Verdict::Stopped | Verdict::Cancelled
                            )
                    } else if let Event::JobEnded(end) = &record.event {
                        end.run == start.run
                            && end.conversation == start.conversation
                            && end.job == job
                            && end.call == start.call
                            && matches!(end.verdict, Verdict::Exited | Verdict::TimedOut | Verdict::Cancelled)
                    } else {
                        false
                    }
                })
                .collect();
            unique &= ends.len() == 1 && ends[0].t_ms > record.t_ms;
            if let [end] = ends.as_slice() {
                overlap |= session.records.iter().any(|request| {
                    matches!(&request.event,
                    Event::ResponseStarted(value) if value.run == start.run && value.conversation == start.conversation
                        && request.t_ms > record.t_ms && request.t_ms < end.t_ms)
                });
            }
        }
    }
    let passed = if *check == EventCheck::JobRunning {
        found
    } else if *check == EventCheck::CompletionDuringJob {
        found && overlap && unique
    } else {
        found && unique
    };
    Measure::Observed { value: passed }
}
