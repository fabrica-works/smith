//! Captured write arguments are compared with settled bytes, not the seed.
//! Missing capture or incomplete streams stay unavailable; exact call identities
//! join terminals. No provider text or remote inference state is known here.
//! Contracts: benchmarks.md, section11.1; domain/tools.md, sections3/4.

use std::path::{Component, Path, PathBuf};

use smith_events::{Event, Verdict};

use crate::{CheckVerdict, Measure};

use super::Observer;

pub(super) fn relative(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

pub(super) fn check(observer: &Observer, path: &Path, minimum: u64, workspace: &Path) -> CheckVerdict {
    let mut evidence = Vec::new();
    let passed = evaluate(observer, path, minimum, workspace, &mut evidence);
    CheckVerdict { check: "written-file".into(), passed, evidence }
}

fn evaluate(
    observer: &Observer,
    path: &Path,
    minimum: u64,
    workspace: &Path,
    evidence: &mut Vec<String>,
) -> Measure<bool> {
    let measured = |value| Measure::Observed { value };
    if !observer.finished
        || observer.fault.is_some()
        || observer.unknown
        || observer.sessions.is_empty()
        || observer.sessions.iter().any(|session| session.unknown || super::usage::incomplete(session).is_some())
    {
        return Measure::unavailable("incomplete event stream prevents a complete write/file comparison");
    }
    let mut writes = Vec::new();
    for (session_index, session) in observer.sessions.iter().enumerate() {
        for record in &session.records {
            let Event::ToolStarted(start) = &record.event else {
                continue;
            };
            if start.tool.as_ref() != b"write" {
                continue;
            }
            if start.source != smith_events::Source::Workspace || start.effect != smith_events::Effect::Write {
                return Measure::unavailable("write tool source/effect is not the native workspace contract");
            }
            let Some(input) = &start.input else {
                return Measure::unavailable("captured write argument content is unavailable");
            };
            let Ok(input) = serde_json::from_slice::<serde_json::Value>(input) else {
                return Measure::unavailable("captured write arguments are not decoded JSON");
            };
            let Some(actual_path) = input.get("path").and_then(serde_json::Value::as_str) else {
                return Measure::unavailable("captured write path is absent");
            };
            let actual_path = Path::new(actual_path);
            let actual_path = actual_path.strip_prefix(workspace).unwrap_or(actual_path);
            let Some(actual_path) = relative(actual_path) else {
                return Measure::unavailable("captured write namespace cannot be resolved by this file predicate");
            };
            if actual_path != path {
                continue;
            }
            let Some(content) = input.get("content").and_then(serde_json::Value::as_str) else {
                return Measure::unavailable("captured write content is absent");
            };
            let ends: Vec<_> = session
                .records
                .iter()
                .filter_map(|record| {
                    let Event::ToolCompleted(end) = &record.event else {
                        return None;
                    };
                    (end.run == start.run
                        && end.conversation == start.conversation
                        && end.call == start.call
                        && end.tool.as_ref() == b"write")
                        .then_some(end)
                })
                .collect();
            if ends.len() != 1 || ends[0].verdict != Verdict::Written {
                return measured(false);
            }
            evidence.push(format!(
                "session={session_index}/run={}/conversation={}/call={}: content_bytes={}",
                start.run,
                start.conversation,
                String::from_utf8_lossy(&start.call),
                content.len()
            ));
            writes.push(content.as_bytes().to_vec());
        }
    }
    if writes.len() != 1 || u64::try_from(writes[0].len()).expect("captured bytes fit u64") < minimum {
        return measured(false);
    }
    match crate::grade::file(workspace, path).and_then(|file| std::fs::read(file).map_err(|error| error.to_string())) {
        Ok(bytes) => {
            evidence.push(format!("saved_bytes={}; saved_equals_call_content={}", bytes.len(), bytes == writes[0]));
            measured(bytes == writes[0])
        }
        Err(error) => {
            evidence.push(error);
            measured(false)
        }
    }
}
