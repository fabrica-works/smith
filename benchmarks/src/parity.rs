//! Offline parity retains the five criteria, the price basis and the smith arm.
//! `recompute` judges attempt rows; `verify` checks the committed section and
//! arm identity. Missing observations never establish parity (benchmarks.md, 16).

pub mod change;
mod number;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{AttemptRow, Measure, RunSummary};

fn passed(row: &AttemptRow) -> bool {
    row.passed && row.end == "completed"
}

const TASKS: [&str; 4] =
    ["fixtures/01-backoff", "fixtures/02-line-decoder", "fixtures/03-planner", "repository/shell-cwd"];

/// One parity criterion supplied by the offline calculation to the summary.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub name: String,
    #[serde(with = "number")]
    pub statistic: f64,
    #[serde(with = "number::interval")]
    pub interval: (f64, f64),
    pub margin: f64,
    pub holds: bool,
}

/// The run's non-inferiority verdict, supplied to the command's process exit.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// Every criterion holds on a complete run.
    OnPar,
    /// A complete run fails a criterion or smith solves no task in a pool.
    NotOnPar,
    /// Setup, observation or reference evidence is missing.
    Incomplete,
}

impl Verdict {
    /// The parity command's stable process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::OnPar => 0,
            Self::NotOnPar => 1,
            Self::Incomplete => 2,
        }
    }
}

/// A committed parity section supplied to the offline verification command.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Parity {
    pub reference_arm: String,
    pub smith_commit: String,
    pub seed: u64,
    pub priced_at: String,
    pub criteria: Vec<Criterion>,
    pub verdict: Verdict,
    pub reported: BTreeMap<String, BTreeMap<String, Measure<f64>>>,
}

fn criterion(name: &str, statistic: f64, interval: (f64, f64), margin: f64) -> Criterion {
    Criterion { name: name.into(), statistic, interval, margin, holds: interval.1 < margin }
}

/// An arm selected by its agent and recorded source version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmKey {
    pub agent: String,
    pub commit: String,
}

impl ArmKey {
    fn contains(&self, row: &AttemptRow) -> bool {
        row.agent == self.agent && row.agent_version == self.commit
    }
}

fn measured(row: &AttemptRow, cost: bool, reference: &ArmKey) -> Option<f64> {
    let measurement = if cost {
        row.priced_usd.clone()
    } else {
        match row.task_wall_ms {
            Measure::Observed { value } => Measure::Observed { value: wall(value) },
            Measure::LowerBound { value, ref missing } => {
                Measure::LowerBound { value: wall(value), missing: missing.clone() }
            }
            Measure::Unavailable { ref reason } => Measure::unavailable(reason),
        }
    };
    let value = match measurement {
        Measure::Observed { value } => value,
        Measure::LowerBound { value, .. } if reference.contains(row) => value,
        Measure::LowerBound { value, missing } if cost && crate::tokens::retried_failures(&missing) => value,
        Measure::LowerBound { .. } | Measure::Unavailable { .. } => return None,
    };
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn wall(value: u64) -> f64 {
    f64::from(u32::try_from(value >> 32).expect("high half")) * 4_294_967_296.0
        + f64::from(u32::try_from(value & 0xffff_ffff).expect("low half"))
}

fn metric(rows: &[&AttemptRow], cost: bool, reference: &ArmKey) -> Option<Vec<f64>> {
    rows.iter().map(|row| measured(row, cost, reference)).collect()
}

fn fixture_criterion(
    rows: &[&AttemptRow],
    cost: bool,
    seed: u64,
    subject: &ArmKey,
    reference: &ArmKey,
) -> Result<Criterion, String> {
    let name = if cost { "FC" } else { "FT" };
    if rows.iter().any(|row| !matches!(row.end.as_str(), "completed" | "failed" | "timeout" | "budget")) {
        return Ok(criterion(name, f64::INFINITY, (f64::INFINITY, f64::INFINITY), 1.20));
    }
    let mut strata = Vec::new();
    for task in &TASKS[..3] {
        let mut arms = Vec::new();
        for arm in [reference, subject] {
            let selected: Vec<_> = rows.iter().copied().filter(|row| row.task == *task && arm.contains(row)).collect();
            let Some(values) = metric(&selected, cost, reference) else {
                return Ok(criterion(name, f64::INFINITY, (f64::INFINITY, f64::INFINITY), 1.20));
            };
            arms.push(values.into_iter().zip(selected.iter().map(|row| passed(row))).collect());
        }
        strata.push(crate::stats::Stratum { reference: arms.remove(0), candidate: arms.remove(0) });
    }
    let (ratio, interval) = crate::stats::bootstrap_per_solved(&strata, seed, 10_000)?;
    Ok(criterion(name, ratio, interval, 1.20))
}

fn repository_criterion(
    rows: &[&AttemptRow],
    cost: bool,
    seed: u64,
    subject: &ArmKey,
    reference: &ArmKey,
) -> Result<Criterion, String> {
    let name = if cost { "RC" } else { "RT" };
    let margin = if cost { 1.50 } else { 1.20 };
    let arms: Vec<_> = [reference, subject]
        .into_iter()
        .map(|arm| metric(&rows.iter().copied().filter(|row| arm.contains(row)).collect::<Vec<_>>(), cost, reference))
        .collect();
    let (Some(reference), Some(candidate)) = (&arms[0], &arms[1]) else {
        return Ok(criterion(name, f64::INFINITY, (f64::INFINITY, f64::INFINITY), margin));
    };
    if reference.len() < 5 || candidate.len() < 5 || reference.contains(&0.0) {
        return Ok(criterion(name, f64::INFINITY, (f64::INFINITY, f64::INFINITY), margin));
    }
    if candidate.iter().all(|value| *value == 0.0) {
        return Ok(criterion(name, 0.0, (0.0, 0.0), margin));
    }
    let result = crate::bootstrap_ratio(reference, candidate, seed, 10_000)?;
    Ok(criterion(name, result.ratio, (result.interval_95.lower, result.interval_95.upper), margin))
}

fn reported(rows: &[AttemptRow]) -> BTreeMap<String, BTreeMap<String, Measure<f64>>> {
    let mut values: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    for row in rows {
        let scope = match row.peak_rss_scope {
            crate::PeakScope::Process => "process",
            crate::PeakScope::Tree => "tree",
        };
        for (name, measure) in
            [("responses".into(), &row.responses), (format!("peak_rss_bytes/{scope}"), &row.peak_rss_bytes)]
        {
            let samples = values.entry(row.arm.clone()).or_default().entry(name).or_default();
            if let Measure::Observed { value } = crate::summary::measure_u64(measure) {
                samples.push(value);
            }
        }
    }
    values
        .into_iter()
        .map(|(arm, metrics)| {
            (
                arm,
                metrics
                    .into_iter()
                    .map(|(name, samples)| {
                        let median = crate::median(&samples)
                            .map_or_else(Measure::unavailable, |value| Measure::Observed { value });
                        (name, median)
                    })
                    .collect(),
            )
        })
        .collect()
}

fn criteria(rows: &[AttemptRow], subject: &ArmKey, reference: &ArmKey, seed: u64) -> Result<Vec<Criterion>, String> {
    let fixtures: Vec<_> = rows.iter().filter(|row| row.task != TASKS[3]).collect();
    let repository: Vec<_> = rows.iter().filter(|row| row.task == TASKS[3]).collect();
    let pass_count = |arm: &ArmKey| {
        u32::try_from(rows.iter().filter(|row| arm.contains(row) && passed(row)).count()).expect("attempt count")
    };
    let codex_count = u32::try_from(rows.iter().filter(|row| reference.contains(row)).count()).expect("attempt count");
    let interval = if codex_count == 0 {
        crate::Interval { lower: 0.0, upper: 1.0 }
    } else {
        crate::pass_interval(pass_count(reference), codex_count)?
    };
    let threshold = (interval.lower * f64::from(codex_count)).ceil();
    let outcome = Criterion {
        name: "O".into(),
        statistic: f64::from(pass_count(subject)),
        interval: (interval.lower, interval.upper),
        margin: threshold,
        holds: f64::from(pass_count(subject)) >= threshold,
    };
    Ok(vec![
        outcome,
        fixture_criterion(&fixtures, false, seed, subject, reference)?,
        fixture_criterion(&fixtures, true, seed, subject, reference)?,
        repository_criterion(&repository, false, seed, subject, reference)?,
        repository_criterion(&repository, true, seed, subject, reference)?,
    ])
}

/// Recompute criteria from rows and the smith commit retained by the arm groups.
pub fn recompute(rows: &[AttemptRow], seed: u64, priced_at: &str, smith_commit: &str) -> Result<Parity, String> {
    if rows.len() > 68 {
        return Err("parity has more than 68 attempts".into());
    }
    let mut arms: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for row in rows {
        if !TASKS.contains(&row.task.as_str())
            || !["smith", "codex"].contains(&row.agent.as_str())
            || !ids.insert(&row.attempt)
        {
            return Err("parity needs its four tasks, smith and Codex, and unique attempts".into());
        }
        arms.entry(&row.agent).or_default().insert(&row.arm);
    }
    if arms.values().any(|names| names.len() != 1) {
        return Err("parity needs one arm per agent".into());
    }
    let reference_arm = arms.get("codex").and_then(|names| names.first()).copied().unwrap_or("");
    let subject = ArmKey { agent: "smith".into(), commit: smith_commit.into() };
    let reference = ArmKey {
        agent: "codex".into(),
        commit: rows.iter().find(|row| row.agent == "codex").map_or("", |row| row.agent_version.as_str()).into(),
    };
    let criteria = criteria(rows, &subject, &reference, seed)?;
    let fixtures: Vec<_> = rows.iter().filter(|row| row.task != TASKS[3]).collect();
    let repository: Vec<_> = rows.iter().filter(|row| row.task == TASKS[3]).collect();
    let sizes_complete = TASKS.iter().enumerate().all(|(index, task)| {
        ["smith", "codex"].iter().all(|agent| {
            rows.iter().filter(|row| row.task == *task && row.agent == *agent).count()
                == if index == 3 { 10 } else { 8 }
        })
    });
    let complete = sizes_complete
        && !rows.iter().any(|row| {
            !matches!(row.end.as_str(), "completed" | "failed" | "timeout" | "budget")
                || measured(row, false, &reference).is_none()
                || measured(row, true, &reference).is_none()
        })
        && [&fixtures, &repository].iter().all(|pool| pool.iter().any(|row| row.agent == "codex" && passed(row)));
    let verdict = if !complete {
        Verdict::Incomplete
    } else if criteria.iter().all(|criterion| criterion.holds)
        && [&fixtures, &repository].iter().all(|pool| pool.iter().any(|row| row.agent == "smith" && passed(row)))
    {
        Verdict::OnPar
    } else {
        Verdict::NotOnPar
    };
    Ok(Parity {
        reference_arm: reference_arm.into(),
        smith_commit: smith_commit.into(),
        seed,
        priced_at: priced_at.into(),
        criteria,
        verdict,
        reported: reported(rows),
    })
}

fn identified_rows(summary: &RunSummary) -> Result<&[AttemptRow], String> {
    let mut ids = BTreeSet::new();
    for row in &summary.attempts {
        if !ids.insert(&row.attempt)
            || !summary.groups.iter().any(|group| {
                group.task == row.task
                    && group.arm == row.arm
                    && group.agent.directory() == row.agent
                    && group.agent_version == row.agent_version
                    && group.attempts.contains(&row.attempt)
            })
        {
            return Err("parity attempt has no matching arm identity".into());
        }
    }
    Ok(&summary.attempts)
}

/// Build the section from attempt rows and their recorded arm identity.
pub fn from_summary(summary: &RunSummary) -> Result<Parity, String> {
    if summary.suite != "parity" || summary.tier != "working" || summary.design != crate::Design::Interleaved {
        return Err("parity needs its working, interleaved suite".into());
    }
    identified_rows(summary)?;
    let commits: BTreeSet<_> = summary
        .groups
        .iter()
        .filter(|group| group.agent == crate::Agent::Smith)
        .map(|group| group.agent_version.as_str())
        .collect();
    if commits.len() != 1 {
        return Err("parity needs one smith source commit".into());
    }
    let commit = commits.first().expect("one commit");
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("parity needs the smith arm's full source commit".into());
    }
    recompute(&summary.attempts, summary.seed, &summary.priced_at, commits.first().expect("one commit"))
}

/// Refuse a conflicting committed section or an arm built from another commit.
pub fn verify(summary: &RunSummary, commit: &str) -> Result<Parity, String> {
    let parity = from_summary(summary)?;
    if parity.smith_commit != commit {
        return Err("summary smith arm was built from another commit".into());
    }
    if summary.parity.as_ref() != Some(&parity) {
        return Err("committed parity section disagrees with attempt rows".into());
    }
    Ok(parity)
}
