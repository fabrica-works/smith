//! Manifest check parameters use the event vocabulary's documented spellings.
//! Unknown names refuse before an agent runs (benchmarks.md, section3.2).

use std::path::Path;

use crate::{EventCheck, Refusal};

const VERDICTS: &[&str] = &[
    "read",
    "listed",
    "found",
    "written",
    "edited",
    "exited",
    "conflict",
    "missing",
    "too_large",
    "timed_out",
    "failed",
    "cancelled",
    "ambiguous",
    "result",
    "error",
    "invalid",
    "not_run",
    "withdrawn",
    "not_granted",
    "outside",
    "read_only",
    "too_long",
    "not_found",
    "not_file",
    "linked",
    "protected",
    "not_directory",
    "not_read",
    "stale",
    "no_match",
    "unchanged",
    "busy",
    "nul_byte",
];

pub(crate) const FAILURES: &[&str] = &[
    "overloaded",
    "rate_limited",
    "exhausted",
    "unavailable",
    "timed_out",
    "context_too_long",
    "invalid",
    "unauthorized",
    "limit",
    "protocol",
    "cancelled",
];

const ANSWERS: &[&str] = &["model", "budget", "policy", "cancelled", "stale", "transcript", "busy", "invalid"];

fn named(file: &Path, key: &str, value: &str, allowed: &[&str]) -> Result<(), Refusal> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(Refusal::new(file, key, format!("unknown event vocabulary name {value}")))
    }
}

pub(super) fn checks(file: &Path, key: &str, checks: &[EventCheck]) -> Result<(), Refusal> {
    for (index, check) in checks.iter().enumerate() {
        let key = format!("{key}[{index}]");
        match check {
            EventCheck::ToolCall { tool, verdict, .. } => {
                if tool.trim().is_empty() {
                    return Err(Refusal::new(file, &format!("{key}.tool"), "tool name is empty"));
                }
                if let Some(value) = verdict {
                    named(file, &format!("{key}.verdict"), value, VERDICTS)?;
                }
            }
            EventCheck::NoVerdict { verdicts } => {
                for value in verdicts {
                    named(file, &format!("{key}.verdicts"), value, VERDICTS)?;
                }
            }
            EventCheck::NoStop { stop } => {
                named(file, &format!("{key}.stop"), stop, &["end", "tools", "max_tokens", "refusal"])?;
            }
            EventCheck::ConversationOpened { kind, .. } => {
                named(file, &format!("{key}.kind"), kind, &["main", "child", "compaction"])?;
            }
            EventCheck::CacheReuse { median_at_least, .. } => {
                if !median_at_least.is_finite() || !(0.0..=1.0).contains(median_at_least) {
                    return Err(Refusal::new(
                        file,
                        &format!("{key}.median_at_least"),
                        "reuse share must be finite and within zero to one",
                    ));
                }
            }
            EventCheck::SpendWithin { ceiling_usd } => {
                if !ceiling_usd.is_finite() || *ceiling_usd <= 0.0 {
                    return Err(Refusal::new(
                        file,
                        &format!("{key}.ceiling_usd"),
                        "spend ceiling must be finite and positive",
                    ));
                }
            }
            EventCheck::NoFailure { class } => named(file, &format!("{key}.class"), class, FAILURES)?,
            EventCheck::RunCompleted { status, class, reason } => {
                named(file, &format!("{key}.status"), status, &["accepted", "parked", "failed", "refused"])?;
                if let Some(value) = class {
                    named(file, &format!("{key}.class"), value, ANSWERS)?;
                }
                if let Some(pattern) = reason {
                    regex::Regex::new(pattern)
                        .map_err(|error| Refusal::new(file, &format!("{key}.reason"), error.to_string()))?;
                }
            }
            EventCheck::CallsInOneResponse { tool, at_least } => {
                if tool.trim().is_empty() || *at_least == 0 {
                    return Err(Refusal::new(file, &key, "tool and cardinality must be nonempty"));
                }
            }
            EventCheck::ChildrenOverlap { at_least } | EventCheck::ChildShare { turns_at_most: at_least } => {
                if *at_least == 0 {
                    return Err(Refusal::new(file, &key, "cardinality must be positive"));
                }
            }
            EventCheck::UsageSums
            | EventCheck::Completions { .. }
            | EventCheck::FirstRequestHoldsTask
            | EventCheck::NothingBeforeTask
            | EventCheck::HandledOnce
            | EventCheck::CompletionPerExchange
            | EventCheck::Blocks { .. }
            | EventCheck::PromptWithinWindow
            | EventCheck::MainAfterChild
            | EventCheck::FinalTurn
            | EventCheck::BudgetVisible
            | EventCheck::NoRequest
            | EventCheck::NoRun => {}
        }
    }
    Ok(())
}
