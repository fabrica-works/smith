//! Cache outcomes are logical triplets of canonical, like-input repetitions.
//! One repetition keeps exact reported counts; only three repetitions decide
//! the median and pooled miss rate (benchmarks.md, sections 11.1 and 11.5).

use serde::{Deserialize, Serialize};

use crate::{AttemptResult, Judgment, Measure};

/// One measured repetition supplied by the adapter, never a triplet verdict.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CacheObservation {
    pub cache_read: u64,
    pub predecessor_prompt: u64,
    pub eligible: u64,
    pub full_misses: u64,
    pub median_at_least: f64,
    pub full_misses_per_ten_at_most: u32,
}

impl CacheObservation {
    /// The exact sums' ratio for one repetition, supplied to reports.
    #[must_use]
    pub fn reuse(&self) -> f64 {
        let reads = self.cache_read.to_string().parse::<f64>().expect("u64 is finite");
        let prompts = self.predecessor_prompt.to_string().parse::<f64>().expect("u64 is finite");
        reads / prompts
    }
}

/// Three repetition IDs and their shared judgment, supplied to summaries.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CacheTriplet {
    pub repetitions: Vec<String>,
    pub reuse: Vec<Measure<f64>>,
    pub median_reuse: Measure<f64>,
    pub eligible: Measure<u64>,
    pub full_misses: Measure<u64>,
    pub judgment: String,
}

pub(crate) fn selected(result: &AttemptResult) -> bool {
    result.outcome.deciding.iter().any(|name| name == "cache-reuse")
        || result.outcome.deciding.is_empty() && result.outcome.checks.iter().any(|check| check.check == "cache-reuse")
}

pub(crate) fn repetition(result: &AttemptResult) -> Judgment {
    if !selected(result) {
        return crate::judge(result);
    }
    let mut value = result.clone();
    value.outcome.deciding.retain(|name| name != "cache-reuse");
    value.outcome.checks.retain(|check| check.check != "cache-reuse");
    crate::judge(&value)
}

fn numeric(results: &[&AttemptResult]) -> Result<(Vec<f64>, u64, u64, f64, u32), String> {
    let mut reuse = Vec::new();
    let mut eligible = 0_u64;
    let mut misses = 0_u64;
    let mut thresholds = None;
    for result in results {
        let Some(Measure::Observed { value }) = &result.cache else {
            return Err("a repetition has no exact cache measurement".into());
        };
        if value.predecessor_prompt == 0
            || value.eligible == 0
            || value.full_misses > value.eligible
            || !value.median_at_least.is_finite()
            || !(0.0..=1.0).contains(&value.median_at_least)
        {
            return Err("invalid cache observation".into());
        }
        let current = (value.median_at_least, value.full_misses_per_ten_at_most);
        if thresholds.is_some_and(|prior| prior != current) {
            return Err("triplet thresholds differ".into());
        }
        thresholds = Some(current);
        reuse.push(value.reuse());
        eligible = eligible.checked_add(value.eligible).ok_or("cache eligible sum overflow")?;
        misses = misses.checked_add(value.full_misses).ok_or("cache full-miss sum overflow")?;
    }
    let (threshold, miss_limit) = thresholds.ok_or("no cache observations")?;
    Ok((reuse, eligible, misses, threshold, miss_limit))
}

pub(crate) fn triplet(results: &[&AttemptResult]) -> (Judgment, CacheTriplet) {
    let missing = || Measure::unavailable("a complete like-input triplet is required");
    let mut summary = CacheTriplet {
        repetitions: results.iter().map(|result| result.identity.attempt.clone()).collect(),
        reuse: results
            .iter()
            .map(|result| match &result.cache {
                Some(Measure::Observed { value }) => Measure::Observed { value: value.reuse() },
                Some(Measure::LowerBound { missing, .. }) => Measure::unavailable(missing.join("; ")),
                Some(Measure::Unavailable { reason }) => Measure::unavailable(reason),
                None => Measure::unavailable("cache observation absent"),
            })
            .collect(),
        median_reuse: missing(),
        eligible: Measure::unavailable("incomplete triplet"),
        full_misses: Measure::unavailable("incomplete triplet"),
        judgment: "incomplete".into(),
    };
    let judged = verdict(results, &mut summary);
    summary.judgment = match judged {
        Judgment::Pass => "pass",
        Judgment::Fail => "fail",
        Judgment::Incomplete => "incomplete",
    }
    .into();
    (judged, summary)
}

fn verdict(results: &[&AttemptResult], summary: &mut CacheTriplet) -> Judgment {
    let failed = results.iter().any(|result| repetition(result) == Judgment::Fail);
    if results.len() != 3
        || results.iter().any(|result| {
            !selected(result)
                || result.identity.run != results[0].identity.run
                || result.identity.task != results[0].identity.task
                || result.identity.arm != results[0].identity.arm
                || !crate::summary::same_inputs(&results[0].identity, &result.identity)
        })
    {
        return if failed { Judgment::Fail } else { Judgment::Incomplete };
    }
    let unique: std::collections::BTreeSet<_> = results.iter().map(|result| &result.identity.attempt).collect();
    if unique.len() != 3 {
        return Judgment::Incomplete;
    }
    let Ok((reuse, eligible, misses, threshold, miss_limit)) = numeric(results) else {
        return if failed { Judgment::Fail } else { Judgment::Incomplete };
    };
    let median = crate::median(&reuse).expect("three finite ratios");
    summary.median_reuse = Measure::Observed { value: median };
    summary.eligible = Measure::Observed { value: eligible };
    summary.full_misses = Measure::Observed { value: misses };
    if failed {
        return Judgment::Fail;
    }
    if results.iter().any(|result| repetition(result) == Judgment::Incomplete) {
        return Judgment::Incomplete;
    }
    if median >= threshold && u128::from(misses) * 10 <= u128::from(miss_limit) * u128::from(eligible) {
        Judgment::Pass
    } else {
        Judgment::Fail
    }
}

pub(crate) fn triplets(results: &[&AttemptResult]) -> Vec<(Judgment, CacheTriplet)> {
    let mut ordered = results.to_vec();
    ordered.sort_by(|left, right| left.identity.attempt.cmp(&right.identity.attempt));
    ordered.chunks(3).map(triplet).collect()
}

pub(crate) fn control_failure(results: &[AttemptResult]) -> bool {
    if results.iter().any(|result| repetition(result) != Judgment::Pass) {
        return false;
    }
    let references: Vec<_> = results.iter().collect();
    let groups = triplets(&references);
    groups.len() == 1 && groups[0].0 == Judgment::Fail && matches!(groups[0].1.median_reuse, Measure::Observed { .. })
}

pub(crate) fn notes(results: &[AttemptResult]) -> String {
    use std::fmt::Write;
    let references: Vec<_> = results.iter().collect();
    let mut text = String::new();
    for (verdict, summary) in triplets(&references) {
        writeln!(
            text,
            "triplet={} {verdict:?}; R={:?}; median={:?}; eligible={:?}; full_misses={:?}",
            summary.repetitions.join(","),
            summary.reuse,
            summary.median_reuse,
            summary.eligible,
            summary.full_misses
        )
        .expect("string formatting");
    }
    text
}
