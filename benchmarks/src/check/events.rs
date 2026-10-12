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
            EventCheck::WrittenFile { .. } | EventCheck::FilePieces { .. } | EventCheck::OversizedWrite { .. } => {
                unreachable!("file checks return above")
            }
            EventCheck::ToolPath { tool, path, verdict } => {
                if tool.trim().is_empty() || path.is_empty() || path.contains('\0') {
                    return Err(Refusal::new(file, &key, "tool and path must be nonempty namespace text"));
                }
                named(file, &format!("{key}.verdict"), verdict, VERDICTS)?;
