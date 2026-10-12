//! Actual native provider terminals qualify the slow-completion exercise.
//! Content-free opaque observations are durable-envelope sizes, not raw item sizes.
//! Missing capture and ambiguous dropped items remain unavailable; preset half
//! warnings never fail the task. Contracts: benchmarks.md 11.1; protocol/events.md
//! 3.5/5.3; protocol/limits.md 2.1. No clock or provider is invoked here.

use super::Observer;
use crate::{CheckVerdict, Measure};
use smith_events::{Block, Event, NoticeKind, Outcome};

fn verdict(passed: Measure<bool>, evidence: Vec<String>) -> CheckVerdict {
    CheckVerdict { check: "slow-completion".into(), passed, evidence }
}

fn half(value: u64, bound: u64) -> bool {
    u128::from(value) * 2 > u128::from(bound)
}

fn reasoning(response: &smith_events::ResponseCompleted) -> Result<(u64, u64), String> {
    let Some(completion) = &response.completion else {
        return Err("completion content was not captured; largest reasoning item is unavailable".into());
    };
    let mut raw = 0;
    let mut encoded = 0;
    for block in &completion.blocks {
        match block {
            Block::Opaque { bytes } => {
                // capture::blocks emits Replay::to_bytes for every retained item.
                let Some(item) = bytes.checked_sub(u64::from(skein_llm::REPLAY_HEADER_BYTES)) else {
                    return Err("opaque size is smaller than its native durable envelope".into());
                };
                raw = raw.max(item);
                encoded = encoded.max(*bytes);
            }
            Block::Unknown { .. } => {
                return Err("unknown captured block prevents measuring the largest reasoning item".into());
            }
            Block::Text { .. }
            | Block::Refusal { .. }
            | Block::Call { .. }
            | Block::Oversized { .. }
            | Block::Cut { .. }
            | Block::Result { .. } => {}
        }
    }
    Ok((raw, encoded))
}

pub(super) fn check(observer: &Observer, floor: u64, head: u64, idle: u64, reasoning_bound: u64) -> CheckVerdict {
    let mut evidence = Vec::new();
    let mut missing = Vec::new();
    let mut longest = None;
    let mut failed = false;
    for (session_number, session) in observer.sessions.iter().enumerate() {
        let dropped = session.records.iter().any(|record| {
            matches!(&record.event,
            Event::Notice(value) if value.kind == NoticeKind::ReasoningDropped)
        });
        for (end_at, record) in session.records.iter().enumerate() {
            let Event::ResponseCompleted(response) = &record.event else {
                continue;
            };
            let key = format!(
                "session-{session_number}/run-{}/conversation-{}/response-{}",
                response.run, response.conversation, response.response
            );
            let start = session.records[..end_at].iter().find_map(|record| {
                if let Event::ResponseStarted(start) = &record.event
                    && start.run == response.run
                    && start.conversation == response.conversation
                    && start.response == response.response
                {
                    Some(start)
                } else {
                    None
                }
            });
            let Some(start) = start else {
                missing.push(format!("{key}: no earlier actual provider start"));
                continue;
            };
            longest = Some(longest.unwrap_or(0_u64).max(response.duration_ms));
            failed |= response.outcome != Outcome::Completed || response.failure.is_some();
            evidence.push(format!(
                "{key}: provider={} model={} call_length_ms={}",
                String::from_utf8_lossy(&start.model.endpoint),
                String::from_utf8_lossy(&start.model.model),
                response.duration_ms
            ));
            for (name, value, bound) in
                [("first_byte_ms", response.first_byte_ms, head), ("largest_gap_ms", response.largest_gap_ms, idle)]
            {
                match value {
                    Some(value) => evidence.push(format!(
                        "{key}: {name}={value} preset_bound={bound} preset_finding={}",
                        half(value, bound)
                    )),
                    None => missing.push(format!("{key}: {name} was not measured")),
                }
            }
            if dropped {
                missing.push(format!(
                    "{key}: dropped raw and retained envelope sizes cannot be distinguished per response"
                ));
                continue;
            }
            match reasoning(response) {
                Ok((raw, encoded)) => evidence.push(format!("{key}: largest_reasoning_item_bytes={raw} largest_opaque_envelope_bytes={encoded} preset_bound={reasoning_bound} preset_finding={}", half(raw, reasoning_bound))),
                Err(reason) => missing.push(format!("{key}: {reason}")),
            }
        }
    }
    if failed {
        return verdict(Measure::Observed { value: false }, evidence);
    }
    evidence.extend(missing.iter().cloned());
    let Some(longest) = longest else {
        return verdict(Measure::unavailable("no joined actual provider call"), evidence);
    };
    if longest <= floor {
        evidence.push(format!(
            "NotExercised: longest_call_ms={longest} required_over_ms={floor}; count like a model deviation"
        ));
        return verdict(Measure::unavailable("not exercised: call did not exceed the required duration"), evidence);
    }
    if !missing.is_empty() {
        return verdict(Measure::unavailable(missing.join("; ")), evidence);
    }
    verdict(Measure::Observed { value: true }, evidence)
}
