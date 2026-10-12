//! Oversized-call identity precedes the successful first piece and edits.
//! Captured arguments reconstruct only reported successful mutations; a failed
//! edit is retained as evidence and changes nothing. Missing content stays
//! unavailable. Replay scratch is bounded by the task's unchanged file cap.
//! Contracts: benchmarks.md, section11.1; domain/tools.md, sections3/4.

use std::path::Path;

use smith_events::{Block, Event, Verdict};

use crate::{CheckVerdict, Measure};

use super::{Observer, Session};

pub(super) fn check(observer: &Observer, path: &Path, above: u64, maximum: u64, workspace: &Path) -> CheckVerdict {
    let mut evidence = Vec::new();
    let passed = evaluate(observer, path, above, maximum, workspace, &mut evidence);
    CheckVerdict { check: "file-pieces".into(), passed, evidence }
}

fn evaluate(
    observer: &Observer,
    path: &Path,
    above: u64,
    maximum: u64,
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
        return Measure::unavailable("incomplete event stream prevents a complete piece/file comparison");
    }
    let (index, run, conversation, refused_at) = match refusal(observer, evidence) {
        Ok(scope) => scope,
        Err(measure) => return measure,
    };
    let mut content: Option<String> = None;
    let mut edits = 0;
    for (session_index, session) in observer.sessions.iter().enumerate() {
        for (at, record) in session.records.iter().enumerate() {
            let Event::ToolStarted(start) = &record.event else {
                continue;
            };
            if start.tool.as_ref() != b"write" && start.tool.as_ref() != b"edit" {
                continue;
            }
            let (input, verdict) = match mutation(session, start, path, workspace, evidence) {
                Ok(Some(value)) => value,
                Ok(None) => continue,
                Err(measure) => return measure,
            };
            if start.tool.as_ref() == b"edit" && verdict != Verdict::Edited {
                continue;
            }
            if session_index != index || start.run != run || start.conversation != conversation {
                return Measure::unavailable("piece mutations span independent call scopes");
            }
            if at <= refused_at {
                return measured(false);
            }
            if let Err(measure) = apply(&mut content, &start.tool, &input, &verdict, maximum, &mut edits) {
                return measure;
            }
        }
    }
    let Some(content) = content else {
        return measured(false);
    };
    if edits == 0 || u64::try_from(content.len()).expect("bounded replay") <= above {
        return measured(false);
    }
    match crate::grade::file(workspace, path).and_then(|file| std::fs::read(file).map_err(|error| error.to_string())) {
        Ok(bytes) => {
            evidence.push(format!(
                "successful_edits={edits}; replay_bytes={}; saved_equals_call_pieces={}",
                content.len(),
                bytes == content.as_bytes()
            ));
            measured(bytes == content.as_bytes())
        }
        Err(error) => {
            evidence.push(error);
            measured(false)
        }
    }
}

fn refusal(observer: &Observer, evidence: &mut Vec<String>) -> Result<(usize, u64, u64, usize), Measure<bool>> {
    let mut refusals = Vec::new();
    for (index, session) in observer.sessions.iter().enumerate() {
        for (at, record) in session.records.iter().enumerate() {
            let Event::ResponseCompleted(response) = &record.event else {
                continue;
            };
            if response.oversized == 0 {
                continue;
            }
            let Some(completion) = &response.completion else {
                return Err(Measure::unavailable("oversized call identity needs capture Everything"));
            };
            let blocks: Vec<_> = completion
                .blocks
                .iter()
                .filter_map(|block| {
                    if let Block::Oversized { call, tool, bytes } = block { Some((call, tool, bytes)) } else { None }
                })
                .collect();
            if u64::try_from(blocks.len()).expect("bounded blocks") != response.oversized {
                return Err(Measure::Observed { value: false });
            }
            for (call, tool, bytes) in blocks {
                if tool.as_ref() != b"write" {
                    return Err(Measure::Observed { value: false });
                }
                evidence.push(format!(
                    "oversized session={index}/run={}/conversation={}/call={}; bytes={bytes}",
                    response.run,
                    response.conversation,
                    String::from_utf8_lossy(call)
                ));
                refusals.push((index, response.run, response.conversation, at));
            }
        }
    }
    if refusals.len() != 1 {
        return Err(Measure::Observed { value: false });
    }
    Ok(refusals[0])
}

fn mutation(
    session: &Session,
    start: &smith_events::ToolStarted,
    path: &Path,
    workspace: &Path,
    evidence: &mut Vec<String>,
) -> Result<Option<(serde_json::Value, Verdict)>, Measure<bool>> {
    if start.source != smith_events::Source::Workspace || start.effect != smith_events::Effect::Write {
        return Err(Measure::unavailable("piece tool source/effect is not the native workspace contract"));
    }
    let Some(input) = &start.input else {
        return Err(Measure::unavailable("captured piece arguments are unavailable"));
    };
    let Ok(input) = serde_json::from_slice::<serde_json::Value>(input) else {
        return Err(Measure::unavailable("piece arguments are not decoded JSON"));
    };
    let Some(actual_path) = input.get("path").and_then(serde_json::Value::as_str) else {
        return Err(Measure::unavailable("piece path is absent"));
    };
    let actual_path = Path::new(actual_path);
    let actual_path = actual_path.strip_prefix(workspace).unwrap_or(actual_path);
    let Some(actual_path) = super::written_file::relative(actual_path) else {
        return Err(Measure::unavailable("piece namespace cannot be resolved by this file predicate"));
    };
    if actual_path != path {
        return Ok(None);
    }
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
                && end.tool == start.tool)
                .then_some(end)
        })
        .collect();
    if ends.len() != 1 {
        return Err(Measure::Observed { value: false });
    }
    evidence.push(format!("piece call={}; terminal={:?}", String::from_utf8_lossy(&start.call), ends[0].verdict));
    Ok(Some((input, ends[0].verdict.clone())))
}

fn apply(
    content: &mut Option<String>,
    tool: &[u8],
    input: &serde_json::Value,
    verdict: &Verdict,
    maximum: u64,
    edits: &mut usize,
) -> Result<(), Measure<bool>> {
    if tool == b"write" {
        if *verdict != Verdict::Written || content.is_some() {
            return Err(Measure::Observed { value: false });
        }
        let Some(first) = input.get("content").and_then(serde_json::Value::as_str) else {
            return Err(Measure::unavailable("first piece content is absent"));
        };
        if u64::try_from(first.len()).expect("bounded first piece") > maximum {
            return Err(Measure::Observed { value: false });
        }
        *content = Some(first.to_owned());
    } else {
        let Some(current) = content.as_mut() else {
            return Err(Measure::Observed { value: false });
        };
        let Some(replacements) = input.get("edits").and_then(serde_json::Value::as_array) else {
            return Err(Measure::unavailable("successful edit pieces are absent"));
        };
        let before = current.len();
        for replacement in replacements {
            let (Some(old), Some(new)) = (
                replacement.get("old").and_then(serde_json::Value::as_str),
                replacement.get("new").and_then(serde_json::Value::as_str),
            ) else {
                return Err(Measure::unavailable("successful edit old/new bytes are absent"));
            };
            if old.is_empty() {
                return Err(Measure::Observed { value: false });
            }
            let all = replacement.get("all").and_then(serde_json::Value::as_bool).unwrap_or(false);
            let count = current.matches(old).count();
            if count == 0 || (!all && count != 1) {
                return Err(Measure::Observed { value: false });
            }
            let Some(next) = count
                .checked_mul(old.len())
                .and_then(|removed| current.len().checked_sub(removed))
                .and_then(|length| length.checked_add(count.checked_mul(new.len())?))
            else {
                return Err(Measure::Observed { value: false });
            };
            if u64::try_from(next).expect("replay length fits u64") > maximum {
                return Err(Measure::Observed { value: false });
            }
            *current = if all { current.replace(old, new) } else { current.replacen(old, new, 1) };
        }
        if current.len() <= before {
            return Err(Measure::Observed { value: false });
        }
        *edits += 1;
    }
    Ok(())
}
