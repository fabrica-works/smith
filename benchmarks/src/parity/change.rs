//! A change's criteria use its parent commit as the reference (benchmarks.md 16.6).

mod inputs;
pub use inputs::{Input, Inputs, attach, command, verify};

use serde::{Deserialize, Serialize};

use super::{ArmKey, Criterion, TASKS, criteria, passed};
use crate::{Measure, RunSummary};

/// A change run's report against its parent.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChangeSection {
    pub baseline_run: Option<Input>,
    pub landed: Vec<Input>,
    pub projection: Vec<Projection>,
    pub stop: Option<Stop>,
    pub change_commit: String,
    pub parent_commit: String,
    pub ratios: Vec<Ratio>,
    pub outcome: Outcome,
    pub beside: Vec<Beside>,
    /// Parent lower bounds taken as observed in the ratios.
    pub reference_lower_bounds: Vec<String>,
}

/// One criterion's change-to-parent ratio.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Ratio {
    pub name: String,
    pub value: Criterion,
}

/// Each arm's passes and the parent's exact interval's lower bound.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub change_passes: u32,
    pub parent_passes: u32,
    pub attempts: u32,
    pub parent_lower: f64,
}

/// Medians beside the criteria, deciding nothing.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Beside {
    pub task: String,
    pub arm: String,
    pub responses: Measure<f64>,
    pub requests_s: Measure<f64>,
    pub compaction_s: Measure<f64>,
    pub tools_s: Measure<f64>,
    pub fresh_tokens: Measure<f64>,
}

/// Recompute a change report with its reference selected by source commit.
pub fn from_summary(summary: &RunSummary, parent_commit: &str) -> Result<ChangeSection, String> {
    if summary.suite != "parity-change" || summary.tier != "working" || summary.design != crate::Design::Interleaved {
        return Err("change needs its working, interleaved suite".into());
    }
    let commits: std::collections::BTreeSet<_> =
        summary.groups.iter().map(|group| group.agent_version.as_str()).collect();
    if commits.len() != 2
        || !commits.contains(parent_commit)
        || summary.groups.iter().any(|group| group.agent != crate::Agent::Smith)
        || commits.iter().any(|commit| commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err("change needs two smith source commits including its parent".into());
    }
    let change_commit = *commits.iter().find(|commit| **commit != parent_commit).expect("two commits");
    let rows = super::identified_rows(summary)?;
    if rows.len() > 40 || rows.iter().any(|row| !TASKS.contains(&row.task.as_str())) {
        return Err("change needs its four tasks and at most forty attempts".into());
    }
    let subject = ArmKey { agent: "smith".into(), commit: change_commit.into() };
    let reference = ArmKey { agent: "smith".into(), commit: parent_commit.into() };
    let values = criteria(rows, &subject, &reference, summary.seed)?;
    let count = |arm: &ArmKey, pass: bool| {
        u32::try_from(rows.iter().filter(|row| arm.contains(row) && (!pass || passed(row))).count())
            .expect("bounded attempts")
    };
    let attempts = count(&reference, false);
    let outcome = Outcome {
        change_passes: count(&subject, true),
        parent_passes: count(&reference, true),
        attempts,
        parent_lower: values[0].interval.0,
    };
    let ratios = values.into_iter().skip(1).map(|value| Ratio { name: value.name.clone(), value }).collect();
    let beside = summary.groups.iter().map(|group| beside(rows, group)).collect();
    let reference_lower_bounds = rows
        .iter()
        .filter(|row| reference.contains(row))
        .flat_map(|row| {
            [
                ("task_wall_ms", matches!(row.task_wall_ms, Measure::LowerBound { .. })),
                ("priced_usd", matches!(row.priced_usd, Measure::LowerBound { .. })),
            ]
            .into_iter()
            .filter(|(_, lower)| *lower)
            .map(|(metric, _)| format!("{}: {metric}", row.attempt))
        })
        .collect();
    Ok(ChangeSection {
        baseline_run: None,
        landed: Vec::new(),
        projection: Vec::new(),
        stop: Some(Stop::Incomplete { why: "baseline run missing".into() }),
        reference_lower_bounds,
        change_commit: change_commit.into(),
        parent_commit: parent_commit.into(),
        ratios,
        outcome,
        beside,
    })
}

fn median(values: impl Iterator<Item = Measure<f64>>) -> Measure<f64> {
    let observed: Option<Vec<_>> = values
        .map(|value| match value {
            Measure::Observed { value } => Some(value),
            Measure::LowerBound { .. } | Measure::Unavailable { .. } => None,
        })
        .collect();
    observed
        .and_then(|values| crate::median(&values).ok())
        .map_or_else(|| Measure::unavailable("attempt measurement missing"), |value| Measure::Observed { value })
}

fn seconds(value: &Measure<u64>) -> Measure<f64> {
    match crate::summary::measure_u64(value) {
        Measure::Observed { value } => Measure::Observed { value: value / 1000.0 },
        Measure::LowerBound { value, missing } => Measure::LowerBound { value: value / 1000.0, missing },
        Measure::Unavailable { reason } => Measure::unavailable(reason),
    }
}

fn tools(row: &crate::AttemptRow) -> Measure<f64> {
    let values: Option<Vec<_>> = row
        .time_record
        .tools_ms
        .iter()
        .map(|(_, value)| match seconds(value) {
            Measure::Observed { value } => Some(value),
            Measure::LowerBound { .. } | Measure::Unavailable { .. } => None,
        })
        .collect();
    values.map_or_else(
        || Measure::unavailable("tool duration missing"),
        |values| Measure::Observed { value: values.iter().sum() },
    )
}

fn beside(rows: &[crate::AttemptRow], group: &crate::ArmSummary) -> Beside {
    let selected: Vec<_> = rows.iter().filter(|row| row.task == group.task && row.arm == group.arm).collect();
    Beside {
        task: group.task.clone(),
        arm: group.arm.clone(),
        responses: median(selected.iter().map(|row| crate::summary::measure_u64(&row.responses))),
        requests_s: median(selected.iter().map(|row| seconds(&row.time_record.requests_ms))),
        compaction_s: median(selected.iter().map(|row| seconds(&row.time_record.compaction_ms))),
        tools_s: median(selected.iter().map(|row| tools(row))),
        fresh_tokens: median(selected.iter().map(|row| crate::summary::measure_u64(&row.fresh_tokens))),
    }
}

/// A criterion projected onto the baseline, supplied beside its original interval.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub name: String,
    #[serde(with = "super::number")]
    pub statistic: f64,
    #[serde(with = "super::number::interval")]
    pub baseline_interval: (f64, f64),
}

/// The first reason a change cannot proceed, supplied to the suite's exit.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Stop {
    /// Evidence required for the report is missing.
    Incomplete { why: String },
    /// One criterion moved past its baseline or parent's bound.
    Moved { criterion: String },
}

impl ChangeSection {
    /// The suite's stable exit code, independent of comparison significance.
    #[must_use]
    pub const fn code(&self) -> u8 {
        match self.stop {
            None => 0,
            Some(Stop::Moved { .. }) => 1,
            Some(Stop::Incomplete { .. }) => 2,
        }
    }
}

/// Recompute projections from the named run summaries, never from drift baselines.
pub fn project(
    summary: &RunSummary,
    parent_commit: &str,
    baseline: &RunSummary,
    landed: &[RunSummary],
) -> Result<ChangeSection, String> {
    project_report(summary, from_summary(summary, parent_commit)?, baseline, landed)
}

fn project_report(
    summary: &RunSummary,
    mut report: ChangeSection,
    baseline: &RunSummary,
    landed: &[RunSummary],
) -> Result<ChangeSection, String> {
    let baseline = baseline
        .parity
        .as_ref()
        .filter(|_| baseline.suite == "parity")
        .ok_or("baseline run must be a parity summary with a parity section")?;
    let landed: Vec<_> = landed
        .iter()
        .map(|summary| {
            summary
                .change
                .as_ref()
                .filter(|_| summary.suite == "parity-change")
                .ok_or("landed input must be a parity-change summary with a change section")
        })
        .collect::<Result<_, _>>()?;
    for ratio in &report.ratios {
        let baseline =
            baseline.criteria.iter().find(|value| value.name == ratio.name).ok_or("baseline criterion missing")?;
        let mut statistic = baseline.statistic * ratio.value.statistic;
        for previous in &landed {
            statistic *= previous
                .ratios
                .iter()
                .find(|value| value.name == ratio.name)
                .ok_or("landed criterion missing")?
                .value
                .statistic;
        }
        report.projection.push(Projection {
            name: ratio.name.clone(),
            statistic,
            baseline_interval: baseline.interval,
        });
    }
    report.stop = stop(summary, &report);
    Ok(report)
}

fn stop(summary: &RunSummary, report: &ChangeSection) -> Option<Stop> {
    let unavailable = report.ratios.iter().any(|ratio| {
        !ratio.value.statistic.is_finite() || !ratio.value.interval.0.is_finite() || !ratio.value.interval.1.is_finite()
    }) || report.projection.iter().any(|value| {
        !value.statistic.is_finite() || !value.baseline_interval.0.is_finite() || !value.baseline_interval.1.is_finite()
    });
    let no_pass = TASKS.iter().any(|task| {
        [&report.change_commit, &report.parent_commit].iter().any(|commit| {
            !summary.attempts.iter().any(|row| row.task == *task && row.agent_version == **commit && passed(row))
        })
    });
    let unequal = summary.attempts.iter().filter(|row| row.agent_version == report.change_commit).count()
        != usize::try_from(report.outcome.attempts).expect("bounded attempts");
    if unavailable
        || unequal
        || no_pass
        || summary.attempts.iter().any(|row| !matches!(row.end.as_str(), "completed" | "failed" | "timeout" | "budget"))
    {
        return Some(Stop::Incomplete { why: "criterion unavailable or an arm has no pass on a task".into() });
    }
    for value in &report.projection {
        if value.statistic > value.baseline_interval.1 {
            return Some(Stop::Moved { criterion: value.name.clone() });
        }
    }
    if f64::from(report.outcome.change_passes) < report.outcome.parent_lower * f64::from(report.outcome.attempts) {
        return Some(Stop::Moved { criterion: "O".into() });
    }
    None
}
