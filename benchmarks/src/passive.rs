//! Passive names are a closed author vocabulary, never arbitrary waived text.
//! `name` preserves the check's wire spelling (benchmarks.md, section11.2).

use serde::Deserialize;

/// A passive contract check the task author may explicitly waive.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Passive {
    /// The attempt's complete process tree ended.
    TreeEnded,
    /// Protected seed files retained their digests.
    ProtectedUnchanged,
    /// No write escaped the workspace and private directories.
    NothingOutside,
    /// No benchmark-tree read appeared in reported calls.
    BenchmarkUnread,
    /// Authoritative process and run boundaries are complete.
    StreamWhole,
    /// The actual exit agrees with the answer.
    ExitAgrees,
    /// Authoritative records survive and sink loss respects delivery.
    NothingLost,
    /// Teardown stays within its bound without harness force.
    Teardown,
    /// Every started entity has exactly one matching terminal.
    Paired,
    /// Every heard message has exactly one terminal.
    MessageTerminals,
    /// Charges per response sum exactly to each run's spent.
    SpendSums,
    /// No unexpected completion limit failure occurs.
    NoLimitFailure,
    /// One measured quantity stays below its named effective bound.
    Headroom(HeadroomOf),
}

/// The measured quantity held against a startup limit.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HeadroomOf {
    /// Encoded request bytes against the endpoint request capacity.
    RequestBytes,
    /// Complete prompt tokens against the model window.
    PromptTokens,
    /// Milliseconds to the first response byte against head duration.
    FirstByte,
    /// Milliseconds between response events against idle duration.
    LargestGap,
    /// Charged completions against the run's budget.
    Turns,
    /// Charged spend against the run's notional ceiling.
    Spend,
    /// Resident process memory against the process's declared memory.
    PeakRss,
}

impl Passive {
    /// Identify a waiver by the same typed name its result uses.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TreeEnded => "tree-ended",
            Self::ProtectedUnchanged => "protected-unchanged",
            Self::NothingOutside => "nothing-outside",
            Self::BenchmarkUnread => "benchmark-unread",
            Self::StreamWhole => "stream-whole",
            Self::ExitAgrees => "exit-agrees",
            Self::NothingLost => "nothing-lost",
            Self::Teardown => "teardown",
            Self::Paired => "paired",
            Self::MessageTerminals => "message-terminals",
            Self::SpendSums => "spend-sums",
            Self::NoLimitFailure => "no-limit-failure",
            Self::Headroom(HeadroomOf::RequestBytes) => "headroom/request-bytes",
            Self::Headroom(HeadroomOf::PromptTokens) => "headroom/prompt-tokens",
            Self::Headroom(HeadroomOf::FirstByte) => "headroom/first-byte",
            Self::Headroom(HeadroomOf::LargestGap) => "headroom/largest-gap",
            Self::Headroom(HeadroomOf::Turns) => "headroom/turns",
            Self::Headroom(HeadroomOf::Spend) => "headroom/spend",
            Self::Headroom(HeadroomOf::PeakRss) => "headroom/peak-rss",
        }
    }
}

/// Name every missing limit comparison without inventing a bound or a join.
pub(crate) fn unavailable_headroom(health: &mut crate::Health, reason: &str) {
    for quantity in [
        HeadroomOf::RequestBytes,
        HeadroomOf::PromptTokens,
        HeadroomOf::FirstByte,
        HeadroomOf::LargestGap,
        HeadroomOf::Turns,
        HeadroomOf::Spend,
        HeadroomOf::PeakRss,
    ] {
        let check = Passive::Headroom(quantity).name();
        health.headroom.insert(
            check.trim_start_matches("headroom/").into(),
            crate::Headroom { fraction_used: crate::Measure::unavailable(reason), warning: false },
        );
        health.passive_checks.push(crate::CheckVerdict {
            check: check.into(),
            passed: crate::Measure::unavailable(reason),
            evidence: Vec::new(),
        });
    }
}

/// Keep raw health unchanged while selecting the task's unwaived judgments.
/// A forced teardown after an accepted answer remains a warning (8.3).
pub(crate) fn judge(health: &crate::Health, waived: &[Passive], outcome: &crate::Outcome) -> Vec<crate::CheckVerdict> {
    health
        .passive_checks
        .iter()
        .filter(|check| {
            let explicitly_waived = waived.iter().any(|waiver| waiver.name() == check.check);
            let force_warning = check.check == Passive::Teardown.name()
                && outcome.end == crate::End::Completed
                && outcome.forced != crate::Forced::No;
            !(explicitly_waived || force_warning)
        })
        .cloned()
        .collect()
}
