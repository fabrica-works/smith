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

pub(super) fn checks(
    file: &Path,
    key: &str,
    checks: &[EventCheck],
    native_file_bytes: Option<u64>,
    native_tool_payload: Option<u64>,
) -> Result<(), Refusal> {
    for (index, check) in checks.iter().enumerate() {
        let key = format!("{key}[{index}]");
        if file_check(file, &key, check, native_file_bytes, native_tool_payload)? {
            continue;
        }
        match check {
            EventCheck::ToolPath { tool, path, aliases, verdict } => {
                if tool.trim().is_empty() || path.is_empty() || path.contains('\0')
                    || aliases.iter().any(|alias| alias.is_empty() || alias.contains('\0')) {
                    return Err(Refusal::new(file, &key, "tool and path must be nonempty namespace text"));
                }
                named(file, &format!("{key}.verdict"), verdict, VERDICTS)?;
            }
            EventCheck::WrittenFile { .. } | EventCheck::FilePieces { .. } | EventCheck::OversizedWrite { .. } => {
                unreachable!("file checks return above")
            }
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
            EventCheck::RunCompleted { status, class, which, reason } => {
                named(file, &format!("{key}.status"), status, &["accepted", "parked", "failed", "refused"])?;
                if let Some(value) = class {
                    named(file, &format!("{key}.class"), value, ANSWERS)?;
                }
                if which.as_ref().is_some_and(|value| value.trim().is_empty()) {
                    return Err(Refusal::new(file, &format!("{key}.which"), "typed failure identity must be nonempty"));
                }
                if let Some(pattern) = reason {
                    regex::Regex::new(pattern)
                        .map_err(|error| Refusal::new(file, &format!("{key}.reason"), error.to_string()))?;
                }
            }
            EventCheck::ProviderFailure { status_at_least, status_at_most, detail_bytes_at_most } => {
                if *status_at_least < 100
                    || *status_at_most > 599
                    || status_at_least > status_at_most
                    || *detail_bytes_at_most == 0
                {
                    return Err(Refusal::new(file, &key, "HTTP status range and diagnostic bound must be valid"));
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

fn file_check(
    file: &Path,
    key: &str,
    check: &EventCheck,
    native_file_bytes: Option<u64>,
    native_tool_payload: Option<u64>,
) -> Result<bool, Refusal> {
    if let EventCheck::OversizedWrite { path, content_bytes_above, protocol_input_bytes_above } = check {
        super::check_path(file, &format!("{key}.path"), path)?;
        if *content_bytes_above == 0 || native_tool_payload != Some(*content_bytes_above) {
            return Err(Refusal::new(
                file,
                key,
                "oversized threshold must equal the positive authored native tool_payload",
            ));
        }
        if protocol_input_bytes_above.is_some_and(|bound| bound <= *content_bytes_above) {
            return Err(Refusal::new(file, key, "protocol call bound must exceed the authored payload"));
        }
        return Ok(true);
    }
    if let EventCheck::WrittenFile { path, content_bytes_at_least } = check {
        super::check_path(file, &format!("{key}.path"), path)?;
        if *content_bytes_at_least == 0 {
            return Err(Refusal::new(file, key, "content byte minimum must be positive"));
        }
        return Ok(true);
    }
    if let EventCheck::FilePieces { path, bytes_above, file_bytes } = check {
        super::check_path(file, &format!("{key}.path"), path)?;
        if *bytes_above == 0 || bytes_above >= file_bytes {
            return Err(Refusal::new(file, key, "piece threshold must be positive and below the declared file bound"));
        }
        if native_file_bytes != Some(*file_bytes) {
            return Err(Refusal::new(file, key, "piece replay bound must match the authored native file_bytes"));
        }
        return Ok(true);
    }
    Ok(false)
}
