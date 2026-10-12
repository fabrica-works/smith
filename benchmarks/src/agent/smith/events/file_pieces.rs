//! One immutable refusal identity precedes the successful first piece and edits.
//! `oversized_check` and `check` join native payload `TooLarge` with captured content
//! or preserve attested protocol Oversized blocks. They never infer an unrecorded
//! argument or namespace, and never relabel the protocol block counter.
//! Protocol-only judgments use the authored actual call-input bound; the elided
//! refusal path is never known. Accepted piece paths remain independently judged.
//! In protocol-probe mode no trigger is a separate not-exercised observation;
//! accepted pieces still replay on one actual run/conversation scope.
//! Captured arguments reconstruct only reported successful mutations; a failed
//! edit is retained as evidence and changes nothing. Missing content stays
//! unavailable. Replay scratch is bounded by the task's unchanged file cap.
//! Contracts: benchmarks.md, section11.1; domain/tools.md, sections3/4.

use std::path::Path;

use smith_events::{Block, Event, Verdict};

use crate::{CheckVerdict, Measure};

use super::{Observer, Session};

struct Refusal {
    session: usize,
    run: u64,
    conversation: u64,
    call: Box<[u8]>,
    at: usize,
}

pub(super) fn oversized_check(
    observer: &Observer,
    path: &Path,
    above: u64,
    protocol_input_bytes_above: Option<u64>,
    workspace: &Path,
) -> CheckVerdict {
    let mut evidence = Vec::new();
    let passed = if complete(observer) {
        if let Some(bound) = protocol_input_bytes_above {
            let mut refused = Vec::new();
            evidence.push(format!("refusal_path=protocol_oversized; stated_call_input={bound}; path=not carried (arguments elided by design)"));
            match protocol_refusals(observer, bound, &mut evidence, &mut refused) {
                Ok(()) => protocol_verdict(refused.len(), bound, &mut evidence),
                Err(measure) => measure,
            }
        } else {
            match refusal(observer, path, above, workspace, &mut evidence) {
                Ok(_) => Measure::Observed { value: true },
                Err(measure) => measure,
            }
        }
    } else {
        Measure::unavailable("incomplete event stream prevents oversized call pairing")
    };
    CheckVerdict { check: "oversized-write".into(), passed, evidence }
}

/// No call past the bound is not exercised (benchmarks.md, section 11.1).
fn protocol_verdict(refused: usize, bound: u64, evidence: &mut Vec<String>) -> Measure<bool> {
    if refused == 0 {
        evidence.push(format!("NotExercised: no call past the stated call_input={bound}; bound honoured"));
        return Measure::unavailable("not exercised: no call past the call's bound");
    }
    Measure::Observed { value: refused == 1 }
}

fn complete(observer: &Observer) -> bool {
    observer.finished
        && observer.fault.is_none()
        && !observer.unknown
        && !observer.sessions.is_empty()
        && !observer.sessions.iter().any(|session| session.unknown || super::usage::incomplete(session).is_some())
}

pub(super) fn check(observer: &Observer, path: &Path, above: u64, maximum: u64, workspace: &Path) -> CheckVerdict {
    let mut evidence = Vec::new();
    let passed = evaluate(observer, path, above, maximum, None, workspace, &mut evidence);
    CheckVerdict { check: "file-pieces".into(), passed, evidence }
}

pub(super) fn check_protocol(
    observer: &Observer,
    path: &Path,
    above: u64,
    maximum: u64,
    protocol_bound: u64,
    workspace: &Path,
) -> CheckVerdict {
    let mut evidence = Vec::new();
    let passed = evaluate(observer, path, above, maximum, Some(protocol_bound), workspace, &mut evidence);
    CheckVerdict { check: "file-pieces".into(), passed, evidence }
}

fn evaluate(
    observer: &Observer,
    path: &Path,
    above: u64,
    maximum: u64,
    protocol_bound: Option<u64>,
    workspace: &Path,
    evidence: &mut Vec<String>,
) -> Measure<bool> {
    let measured = |value| Measure::Observed { value };
    if !complete(observer) {
        return Measure::unavailable("incomplete event stream prevents a complete piece/file comparison");
    }
    let mut scope = match repair_scope(observer, path, above, protocol_bound, workspace, evidence) {
        Ok(scope) => scope,
        Err(measure) => return measure,
    };
    let has_refusal = scope.is_some();
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
            if protocol_bound.is_some() && !has_refusal && verdict == Verdict::TooLarge {
                continue;
            }
            match mutation_scope(&mut scope, session_index, start, at, &verdict, has_refusal) {
                Ok(false) => continue,
                Ok(true) => (),
                Err(measure) => return measure,
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

fn repair_scope(
    observer: &Observer,
    path: &Path,
    above: u64,
    protocol_bound: Option<u64>,
    workspace: &Path,
    evidence: &mut Vec<String>,
) -> Result<Option<Refusal>, Measure<bool>> {
    match protocol_bound {
        Some(bound) => {
            let mut refused = Vec::new();
            protocol_refusals(observer, bound, evidence, &mut refused)?;
            if refused.len() > 1 {
                return Err(Measure::Observed { value: false });
            }
            evidence.push(format!("protocol_triggers={}; accepted pieces judged independently", refused.len()));
            Ok(refused.pop())
        }
        None => refusal(observer, path, above, workspace, evidence).map(Some),
    }
}

fn mutation_scope(
    scope: &mut Option<Refusal>,
    session: usize,
    start: &smith_events::ToolStarted,
    at: usize,
    verdict: &Verdict,
    has_refusal: bool,
) -> Result<bool, Measure<bool>> {
    if scope.is_none() {
        *scope =
            Some(Refusal { session, run: start.run, conversation: start.conversation, call: start.call.clone(), at });
    }
    let scope = scope.as_ref().expect("first accepted piece or observed refusal supplies scope");
    if session != scope.session || start.run != scope.run || start.conversation != scope.conversation {
        return Err(Measure::unavailable("piece mutations span independent call scopes"));
    }
    if has_refusal && at <= scope.at {
        if start.call == scope.call && *verdict == Verdict::TooLarge {
            return Ok(false);
        }
        return Err(Measure::Observed { value: false });
    }
    Ok(true)
}

fn refusal(
    observer: &Observer,
    path: &Path,
    above: u64,
    workspace: &Path,
    evidence: &mut Vec<String>,
) -> Result<Refusal, Measure<bool>> {
    let mut refusals = Vec::new();
    protocol_refusals(observer, above, evidence, &mut refusals)?;
    native_refusals(observer, path, above, workspace, evidence, &mut refusals)?;
    if refusals.len() != 1 {
        return Err(Measure::Observed { value: false });
    }
    Ok(refusals.remove(0))
}

fn protocol_refusals(
    observer: &Observer,
    above: u64,
    evidence: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) -> Result<(), Measure<bool>> {
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
                if *bytes <= above {
                    return Err(Measure::Observed { value: false });
                }
                refusals.push(Refusal {
                    session: index,
                    run: response.run,
                    conversation: response.conversation,
                    call: call.clone(),
                    at,
                });
            }
        }
    }
    Ok(())
}

fn native_refusals(
    observer: &Observer,
    path: &Path,
    above: u64,
    workspace: &Path,
    evidence: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) -> Result<(), Measure<bool>> {
    for (index, session) in observer.sessions.iter().enumerate() {
        for (at, record) in session.records.iter().enumerate() {
            let Event::ToolCompleted(end) = &record.event else {
                continue;
            };
            if end.tool.as_ref() != b"write" || end.verdict != Verdict::TooLarge {
                continue;
            }
            let starts: Vec<_> = session
                .records
                .iter()
                .enumerate()
                .filter_map(|(before, record)| {
                    let Event::ToolStarted(start) = &record.event else {
                        return None;
                    };
                    (start.run == end.run
                        && start.conversation == end.conversation
                        && start.call == end.call
                        && start.tool == end.tool)
                        .then_some((before, start))
                })
                .collect();
            if starts.len() != 1 || starts[0].0 >= at {
                return Err(Measure::Observed { value: false });
            }
            let Some((input, Verdict::TooLarge)) = mutation(session, starts[0].1, path, workspace, evidence)? else {
                return Err(Measure::Observed { value: false });
            };
            let Some(content) = input.get("content").and_then(serde_json::Value::as_str) else {
                return Err(Measure::unavailable("oversized write content is unavailable"));
            };
            let bytes = u64::try_from(content.len()).expect("captured input fits");
            if bytes <= above {
                return Err(Measure::Observed { value: false });
            }
            evidence.push(format!("payload-refused session={index}/run={}/conversation={}/call={}; content_bytes={bytes}; payload={above}", end.run, end.conversation, String::from_utf8_lossy(&end.call)));
            refusals.push(Refusal {
                session: index,
                run: end.run,
                conversation: end.conversation,
                call: end.call.clone(),
                at,
            });
        }
    }
    Ok(())
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
