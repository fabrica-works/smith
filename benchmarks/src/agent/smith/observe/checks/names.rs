//! Frozen Debug names are mapped explicitly, without interpreting diagnostics.

use super::super::Value;

pub(super) fn name(value: &Value) -> Option<&str> {
    match value {
        Value::Name(name) | Value::Tuple { name, .. } | Value::Struct { name, .. } => Some(name),
        Value::Number(_) | Value::String(_) | Value::Bytes(_) | Value::Sequence(_) => None,
    }
}

pub(super) fn stop(value: &Value) -> Option<&str> {
    match name(value)? {
        "EndTurn" => Some("end"),
        "ToolUse" => Some("tools"),
        "MaxTokens" => Some("max_tokens"),
        "Refusal" => Some("refusal"),
        _ => None,
    }
}

pub(super) fn failure(value: &Value) -> Option<&str> {
    match name(value)? {
        "Overloaded" => Some("overloaded"),
        "RateLimited" => Some("rate_limited"),
        "Exhausted" => Some("exhausted"),
        "Unavailable" => Some("unavailable"),
        "TimedOut" => Some("timed_out"),
        "ContextTooLong" => Some("context_too_long"),
        "Invalid" => Some("invalid"),
        "Unauthorized" => Some("unauthorized"),
        "Limit" => Some("limit"),
        "Protocol" => Some("protocol"),
        "Cancelled" => Some("cancelled"),
        _ => None,
    }
}

pub(super) fn verdict(value: &Value) -> Option<&str> {
    match name(value)? {
        "Read" => Some("read"),
        "Listed" => Some("listed"),
        "Found" => Some("found"),
        "Written" => Some("written"),
        "Edited" => Some("edited"),
        "Exited" => Some("exited"),
        "Conflict" => Some("conflict"),
        "Missing" => Some("missing"),
        "TooLarge" => Some("too_large"),
        "TimedOut" => Some("timed_out"),
        "Failed" => Some("failed"),
        "Unchanged" => Some("unchanged"),
        "BadPath" => Some("bad_path"),
        "NoMatch" => Some("no_match"),
        "Ambiguous" => Some("ambiguous"),
        _ => None,
    }
}

pub(super) fn answer(value: &Value) -> Option<(&str, Option<&str>)> {
    match value {
        Value::Name(name) if name == "Accepted" => Some(("accepted", None)),
        Value::Name(name) if name == "Parked" => Some(("parked", None)),
        Value::Tuple { name, values } if values.len() == 1 => {
            let status = match name.as_str() {
                "Failed" => "failed",
                "Refused" => "refused",
                _ => return None,
            };
            let class = match self::name(&values[0])? {
                "Model" => "model",
                "Budget" => "budget",
                "Policy" => "policy",
                "Cancelled" => "cancelled",
                "Stale" => "stale",
                "Transcript" => "transcript",
                "Busy" => "busy",
                "Invalid" => "invalid",
                _ => return None,
            };
            Some((status, Some(class)))
        }
        Value::Name(_)
        | Value::Number(_)
        | Value::String(_)
        | Value::Bytes(_)
        | Value::Sequence(_)
        | Value::Tuple { .. }
        | Value::Struct { .. } => None,
    }
}
