//! Explicit startup probes classify the genuine refusal and empty native capture.
//! Other empty streams remain errors; no record is synthesized (benchmarks.md 11.1).

use crate::agent::smith::events;
use crate::{End, EventCheck, Exit, Forced, OutcomeCheck, Task};

pub(super) fn finish(process: &mut crate::process::ProcessResult) -> Result<(), String> {
    if let Some(crate::agent::Observing::Smith(observing)) = process.observing.as_deref_mut() {
        observing.observer.finish().map_err(|error| error.what)?;
    }
    Ok(())
}

pub(super) fn classify(process: &crate::process::ProcessResult, task: &Task) -> Option<End> {
    if process.error.is_some() {
        return None;
    }
    let crate::agent::Observing::Smith(observing) = process.observing.as_deref()? else {
        return None;
    };
    refused(task, &observing.observer, process.exit, process.forced, &process.stderr)
}

fn refused(task: &Task, observer: &events::Observer, exit: Exit, forced: Forced, stderr: &[u8]) -> Option<End> {
    if !task.event.contains(&EventCheck::NoRun)
        || !observer.ended_without_run()
        || exit != Exit::Code(2)
        || forced != Forced::No
    {
        return None;
    }
    let line = std::str::from_utf8(stderr).ok()?.lines().last()?;
    let diagnostic = task.outcome.iter().any(|check| match check {
        OutcomeCheck::LastLine { pattern } => regex::Regex::new(pattern).is_ok_and(|pattern| pattern.is_match(line)),
        OutcomeCheck::FileContent { .. }
        | OutcomeCheck::FileSize { .. }
        | OutcomeCheck::FileDigest { .. }
        | OutcomeCheck::Absent { .. }
        | OutcomeCheck::Pattern { .. }
        | OutcomeCheck::Protected { .. }
        | OutcomeCheck::End { .. }
        | OutcomeCheck::ExitCode { .. }
        | OutcomeCheck::Answer { .. }
        | OutcomeCheck::CredentialRefresh => false,
    });
    diagnostic.then(|| End::Refused { setup: line.to_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn startup_refusal_requires_explicit_scope_complete_empty_capture_code_and_diagnostic() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let task = crate::read_task(&root.join("tasks/probes/failures/task.toml")).expect("authored startup variants");
        let mut task = task.with_variant("bad-setting").expect("explicit startup scope");
        let limits = smith_events::Limits { string: 128, content: 1024, items: 16 };
        let mut observer = events::Observer::new(&limits).expect("reader");
        let diagnostic = b"smith: models[0].output must be positive\n";
        assert!(refused(&task, &observer, Exit::Code(2), Forced::No, diagnostic).is_none());
        observer.finish().expect("fully read empty capture");
        assert_eq!(
            refused(&task, &observer, Exit::Code(2), Forced::No, diagnostic),
            Some(End::Refused { setup: "smith: models[0].output must be positive".into() })
        );
        for exit in [Exit::Code(0), Exit::Code(1), Exit::Signal(15)] {
            assert!(refused(&task, &observer, exit, Forced::No, diagnostic).is_none());
        }
        assert!(refused(&task, &observer, Exit::Code(2), Forced::Terminated, diagnostic).is_none());
        assert!(refused(&task, &observer, Exit::Code(2), Forced::No, b"unrelated error\n").is_none());
        task.event.clear();
        assert!(refused(&task, &observer, Exit::Code(2), Forced::No, diagnostic).is_none());
    }
}
