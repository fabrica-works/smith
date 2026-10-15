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
        if file_check(file, &key, check, native_file_bytes, native_tool_payload)? || identity_check(file, &key, check)?
        {
            continue;
        }
        match check {
            EventCheck::ToolPath { .. }
            | EventCheck::RunCompleted { .. }
            | EventCheck::ProviderFailure { .. }
            | EventCheck::SlowCompletion { .. } => {
                unreachable!("identity checks return above")
            }
            EventCheck::WrittenFile { .. } | EventCheck::FilePieces { .. } | EventCheck::OversizedWrite { .. } => {
                unreachable!("file checks return above")
            }
            EventCheck::ToolCall { tool, verdict, .. } => {
                nonempty_tool(file, &key, tool)?;
                if let Some(value) = verdict {
                    named(file, &format!("{key}.verdict"), value, VERDICTS)?;
                }
            }
            EventCheck::ToolResultBytes { tool, at_least } => {
                if tool.trim().is_empty() || *at_least == 0 {
                    return Err(Refusal::new(file, &key, "tool and result byte bound must be nonempty"));
                }
            }
            EventCheck::ToolTimeout { requested_seconds, deadline_ms, grace_ms, command, .. } => {
                timeout(file, &key, *requested_seconds, *deadline_ms, *grace_ms, command.as_deref())?;
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
            EventCheck::CallsInOneResponse { tool, at_least } => {
                if tool.trim().is_empty() || *at_least == 0 {
                    return Err(Refusal::new(file, &key, "tool and cardinality must be nonempty"));
                }
            }
            EventCheck::CallCount { tool, .. } | EventCheck::CallsPerResponse { tool, .. } => {
                if tool.trim().is_empty() {
                    return Err(Refusal::new(file, &key, "tool name is empty"));
                }
            }
            EventCheck::ChildrenOverlap { at_least }
            | EventCheck::ChildrenInTurn { count: at_least }
            | EventCheck::ChildShare { turns_at_most: at_least } => {
                if *at_least == 0 {
                    return Err(Refusal::new(file, &key, "cardinality must be positive"));
                }
            }
            EventCheck::UsageSums
            | EventCheck::JobRunning
            | EventCheck::CompletionDuringJob
            | EventCheck::JobTerminal
            | EventCheck::Completions { .. }
            | EventCheck::FirstRequestHoldsTask
            | EventCheck::NothingBeforeTask
            | EventCheck::HandledOnce
            | EventCheck::CompletionPerExchange
            | EventCheck::Blocks { .. }
            | EventCheck::PromptWithinWindow
            | EventCheck::MainAfterChild
            | EventCheck::FinalTurn
            | EventCheck::CompactionChoice
            | EventCheck::CompactionCarried
            | EventCheck::BudgetVisible
            | EventCheck::BudgetTold
            | EventCheck::BudgetReport
            | EventCheck::NoRequest
            | EventCheck::NoRun => {}
        }
    }
    Ok(())
}

fn identity_check(file: &Path, key: &str, check: &EventCheck) -> Result<bool, Refusal> {
    if let EventCheck::SlowCompletion { longer_than_ms, head_ms, idle_ms, reasoning_bytes } = check {
        if [*longer_than_ms, *head_ms, *idle_ms, *reasoning_bytes].contains(&0) {
            return Err(Refusal::new(file, key, "exercise floor and preset bounds must be positive"));
        }
        return Ok(true);
    }
    if let EventCheck::ToolPath { tool, path, aliases, verdict } = check {
        if tool.trim().is_empty()
            || path.is_empty()
            || path.contains('\0')
            || aliases.iter().any(|alias| alias.is_empty() || alias.contains('\0'))
        {
            return Err(Refusal::new(file, key, "tool and path must be nonempty namespace text"));
        }
        named(file, &format!("{key}.verdict"), verdict, VERDICTS)?;
        return Ok(true);
    }
    if let EventCheck::RunCompleted { status, class, which, reason } = check {
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
        return Ok(true);
    }
    if let EventCheck::ProviderFailure { status_at_least, status_at_most, detail_bytes_at_most } = check {
        if *status_at_least < 100
            || *status_at_most > 599
            || status_at_least > status_at_most
            || *detail_bytes_at_most == 0
        {
            return Err(Refusal::new(file, key, "HTTP status range and diagnostic bound must be valid"));
        }
        return Ok(true);
    }
    Ok(false)
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

fn timeout(
    file: &Path,
    key: &str,
    requested: u32,
    deadline: u64,
    grace: u64,
    command: Option<&str>,
) -> Result<(), Refusal> {
    if command.is_some_and(|value| value.trim().is_empty()) {
        return Err(Refusal::new(file, key, "command identity must be nonempty when supplied"));
    }
    if requested == 0 || deadline == 0 || deadline.checked_add(grace).is_none() {
        return Err(Refusal::new(file, key, "timeout request, bound and grace must fit positive durations"));
    }
    Ok(())
}

fn nonempty_tool(file: &Path, key: &str, tool: &str) -> Result<(), Refusal> {
    if tool.trim().is_empty() {
        return Err(Refusal::new(file, &format!("{key}.tool"), "tool name is empty"));
    }
    Ok(())
}
