//! Neutral checks read the settled end and bounded text without interpreting
//! agent diagnostics (benchmarks.md, sections3.2/8.3; shell.md, section4.3).

use crate::{CheckVerdict, End, Exit, Measure, OutcomeCheck};

use super::Ended;

fn pattern(actual: Option<&str>, expected: &str, missing: &str) -> Measure<bool> {
    let Some(actual) = actual else {
        return Measure::unavailable(missing);
    };
    match regex::Regex::new(expected) {
        Ok(pattern) => Measure::Observed { value: pattern.is_match(actual) },
        Err(error) => Measure::unavailable(format!("invalid outcome pattern: {error}")),
    }
}

pub(super) fn check(check: &OutcomeCheck, ended: &Ended) -> CheckVerdict {
    let (name, passed) = match check {
        OutcomeCheck::End { end } => {
            let actual = match &ended.end {
                End::Completed => "completed",
                End::Failed { .. } => "failed",
                End::Budget { .. } => "budget",
                End::Refused { .. } => "refused",
                End::Timeout => "timeout",
                End::HarnessError { .. } => "harness-error",
            };
            ("end", Measure::Observed { value: actual == end })
        }
        OutcomeCheck::ExitCode { code } => (
            "exit-code",
            match &ended.exit {
                Measure::Observed { value } => Measure::Observed { value: *value == Exit::Code(*code) },
                Measure::LowerBound { .. } | Measure::Unavailable { .. } => {
                    Measure::unavailable("no exact settled process exit")
                }
            },
        ),
        OutcomeCheck::LastLine { pattern: expected } => {
            ("last-line", pattern(ended.last_line.as_deref(), expected, "no complete stderr line was retained"))
        }
        OutcomeCheck::Answer { pattern: expected } => {
            ("answer", pattern(ended.answer.as_deref(), expected, "the selected face did not retain final result text"))
        }
        OutcomeCheck::CredentialRefresh => ("credential-refresh", ended.credential_refresh.clone()),
        OutcomeCheck::FileContent { .. }
        | OutcomeCheck::FileSize { .. }
        | OutcomeCheck::FileDigest { .. }
        | OutcomeCheck::Absent { .. }
        | OutcomeCheck::Pattern { .. }
        | OutcomeCheck::Protected { .. } => unreachable!("file checks use their snapshot"),
    };
    CheckVerdict { check: name.into(), passed, evidence: Vec::new() }
}
