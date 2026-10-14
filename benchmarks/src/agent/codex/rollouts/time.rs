//! Root rollout timestamps account for requests, calls and command overlap.
//! First-byte and silent-gap measures are absent (benchmarks.md, section 8.1).

use super::{ParseError, Rollouts, bad, required_string};
use crate::time_record::Breakdown;
use crate::{TimeRecord, Timing};
use std::collections::{BTreeMap, BTreeSet};

fn timestamp(value: &serde_json::Value) -> Option<u64> {
    let text = value.get("timestamp")?.as_str()?;
    let time = time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339).ok()?;
    u64::try_from(time.unix_timestamp_nanos() / 1_000_000).ok()
}

fn record(bytes: &[u8], thread: &str) -> Result<(TimeRecord, Option<u64>), ParseError> {
    let text = std::str::from_utf8(bytes).map_err(|error| bad("rollout", "UTF-8", error.to_string()))?;
    let mut spans = Breakdown::default();
    let mut anchor = None;
    let mut start = None;
    let mut end = None;
    let mut calls = BTreeMap::new();
    let mut responses = BTreeSet::new();
    let mut commands = BTreeSet::new();
    let mut longest = None;
    let missing = || (TimeRecord::unavailable("root rollout lacks complete timestamped spans"), None);
    for line in text.lines() {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|error| bad("rollout", "JSON", error.to_string()))?;
        let Some(payload) = value.get("payload") else { continue };
        let kind = payload.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
        match value.get("type").and_then(serde_json::Value::as_str).unwrap_or("") {
            "token_usage_record" if payload.get("thread_id").and_then(serde_json::Value::as_str) == Some(thread) => {
                let id = required_string(payload, "response_id")?;
                if responses.insert(id.to_owned()) {
                    let (Some(from), Some(to)) = (anchor, timestamp(&value)) else { return Ok(missing()) };
                    let Some(duration) = to.checked_sub(from) else { return Ok(missing()) };
                    spans.requests.push((from, to));
                    longest = Some(longest.unwrap_or(0).max(duration));
                }
            }
            "response_item" if matches!(kind, "custom_tool_call" | "function_call") => {
                let Some(at) = timestamp(&value) else { return Ok(missing()) };
                let id = required_string(payload, "call_id")?;
                let name = required_string(payload, "name")?;
                spans.tool(name, (at, at));
                calls.insert(id.to_owned(), (name.to_owned(), at));
            }
            "response_item" if matches!(kind, "custom_tool_call_output" | "function_call_output") => {
                let Some(at) = timestamp(&value) else { return Ok(missing()) };
                let id = required_string(payload, "call_id")?;
                let Some((name, from)) = calls.remove(id) else { return Ok(missing()) };
                if from > at {
                    return Ok(missing());
                }
                spans.tool(&name, (from, at));
                anchor = Some(at);
                if name.ends_with("wait_agent") {
                    spans.children.push((from, at));
                }
            }
            "event_msg" if kind == "task_started" => {
                let Some(at) = timestamp(&value) else { return Ok(missing()) };
                start.get_or_insert(at);
                anchor = Some(at);
            }
            "event_msg" if kind == "task_complete" => end = timestamp(&value),
            "event_msg" if kind == "item_completed" => {
                let Some(item) = payload
                    .get("item")
                    .filter(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("CommandExecution"))
                else {
                    continue;
                };
                let id = required_string(item, "id")?;
                if commands.insert(id.to_owned()) {
                    let (Some(at), Some(secs), Some(nanos)) = (
                        timestamp(&value),
                        item.pointer("/duration/secs").and_then(serde_json::Value::as_u64),
                        item.pointer("/duration/nanos").and_then(serde_json::Value::as_u64),
                    ) else {
                        return Ok(missing());
                    };
                    let duration = secs.checked_mul(1000).and_then(|ms| ms.checked_add(nanos / 1_000_000));
                    let Some(from) = duration.and_then(|duration| at.checked_sub(duration)) else {
                        return Ok(missing());
                    };
                    spans.commands.push((from, at));
                }
            }
            // Native compaction records lack an opening timestamp for a matched span.
            "compacted" => return Ok((TimeRecord::unavailable("Codex compaction span not reported"), longest)),
            _ => {}
        }
    }
    let (Some(start), Some(end)) = (start, end) else { return Ok(missing()) };
    if start > end || !calls.is_empty() || responses.is_empty() {
        return Ok(missing());
    }
    Ok((spans.finish((start, end)), longest))
}

impl Rollouts {
    /// Fill root-thread time from its archived rollout, excluding child work.
    pub fn timing(&self, thread: &str, timing: &mut Timing) -> Result<(), ParseError> {
        if let Some(file) = self.files.iter().find(|file| file.thread == thread) {
            let (record, longest) = record(&file.bytes, thread)?;
            timing.record = record;
            timing.longest_completion_ms = crate::time_record::observed(longest);
        }
        Ok(())
    }
}
