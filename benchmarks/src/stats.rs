//! Seeded statistics retain the sampled values and never infer missing metrics.
//! `interleave`, `bootstrap_ratio`, `pass_interval` and `audit_sample` implement
//! benchmarks.md, section 9, using skein's recorded seed for every draw.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use skein_lib::Rng;

use crate::Measure;

/// A repository comparison's range claim (benchmarks.md, sections 3.1 and 9).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub enum LargeEffect {
    /// Every candidate value lies above or below every reference value.
    Separated { above: bool },
    /// The observed ranges overlap, including a shared endpoint.
    None,
    /// An incomplete range keeps the reason that prevents a claim.
    Unclaimed { reason: String },
}

fn observed_range(measures: &[Measure<f64>]) -> Result<Interval, String> {
    let mut values = Vec::new();
    let mut reasons = Vec::new();
    for measure in measures {
        match measure {
            Measure::Observed { value } => values.push(*value),
            Measure::LowerBound { missing, .. } => reasons.push(format!("lower bound: {}", missing.join("; "))),
            Measure::Unavailable { reason } => reasons.push(reason.clone()),
        }
    }
    if !reasons.is_empty() {
        return Err(reasons.join("; "));
    }
    validate(&values)?;
    Ok(Interval {
        lower: values.iter().copied().fold(f64::INFINITY, f64::min),
        upper: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    })
}

/// Compare complete observed ranges without inferring statistical significance.
#[must_use]
pub fn large_effect(reference: &[Measure<f64>], candidate: &[Measure<f64>]) -> LargeEffect {
    match (observed_range(reference), observed_range(candidate)) {
        (Ok(reference), Ok(candidate)) if candidate.lower > reference.upper => LargeEffect::Separated { above: true },
        (Ok(reference), Ok(candidate)) if candidate.upper < reference.lower => LargeEffect::Separated { above: false },
        (Ok(_), Ok(_)) => LargeEffect::None,
        (Err(reference), Err(candidate)) => {
            LargeEffect::Unclaimed { reason: format!("reference: {reference}; candidate: {candidate}") }
        }
        (Err(reason), Ok(_)) => LargeEffect::Unclaimed { reason: format!("reference: {reason}") },
        (Ok(_), Err(reason)) => LargeEffect::Unclaimed { reason: format!("candidate: {reason}") },
    }
}

/// The rerun decision supplied by the probe's observed checks.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeVerdict {
    /// Every check passed in the first attempt or both failures were absent.
    Passed,
    /// The first failure needs its one allowed rerun.
    Rerun,
    /// Both attempts failed their checks.
    Regression,
    /// One of two attempts passed, so calibration must be repeated.
    Quarantined,
}

/// Decide a probe's one-rerun rule from its first check verdict and optional rerun.
#[must_use]
pub const fn probe_verdict(first_passed: bool, rerun_passed: Option<bool>) -> ProbeVerdict {
    match rerun_passed {
        None => {
            if first_passed {
                ProbeVerdict::Passed
            } else {
                ProbeVerdict::Rerun
            }
        }
        Some(second) => {
            if first_passed && second {
                ProbeVerdict::Passed
            } else if !first_passed && !second {
                ProbeVerdict::Regression
            } else {
                ProbeVerdict::Quarantined
            }
        }
    }
}

/// A confidence interval supplied by the statistical calculation.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

/// A comparison's observed ratio, interval and detection decision.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    /// Candidate median divided by the reference median.
    pub ratio: f64,
    pub interval_95: Interval,
    pub minimum_detectable_effect: f64,
    pub detected: bool,
    pub resamples: u32,
}

fn validate(values: &[f64]) -> Result<(), String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return Err("statistics need nonempty finite nonnegative measurements".into());
    }
    Ok(())
}

/// Return the median of every supplied observation, regardless of its end.
pub fn median(values: &[f64]) -> Result<f64, String> {
    validate(values)?;
    let mut values = values.to_vec();
    Ok(sample_median(&mut values))
}

// Input and resampled values are validated before entering this calculation.
fn sample_median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) { values[middle - 1] / 2.0 + values[middle] / 2.0 } else { values[middle] }
}

/// Compute sample standard deviation divided by the arithmetic mean.
pub fn coefficient_of_variation(values: &[f64]) -> Result<f64, String> {
    validate(values)?;
    if values.len() < 2 {
        return Err("CV needs at least two observations".into());
    }
    let count = f64::from(u32::try_from(values.len()).map_err(|_| "too many CV observations")?);
    let scale = values.iter().copied().fold(0.0, f64::max);
    if scale == 0.0 {
        return Err("CV is unavailable for a zero mean".into());
    }
    let mean = values.iter().map(|value| value / scale).sum::<f64>() / count;
    // Scale before squaring to avoid overflow on valid large measurements.
    let variance = values.iter().map(|value| ((value / scale) / mean - 1.0).powi(2)).sum::<f64>() / (count - 1.0);
    let cv = variance.sqrt();
    if !cv.is_finite() {
        return Err("CV exceeded finite precision".into());
    }
    Ok(cv)
}

/// The relative effect resolvable with this variation and attempts per arm.
pub fn minimum_detectable_effect(cv: f64, attempts: u32) -> Result<f64, String> {
    if !cv.is_finite() || cv < 0.0 || attempts == 0 {
        return Err("minimum detectable effect needs finite CV and positive n".into());
    }
    let effect = 2.8 * cv * (2.0 / f64::from(attempts)).sqrt();
    if !effect.is_finite() {
        return Err("minimum detectable effect exceeded finite precision".into());
    }
    Ok(effect)
}

fn shuffle(values: &mut [usize], random: &mut Rng) {
    for upper in (1..values.len()).rev() {
        let bound = u64::try_from(upper + 1).expect("slice length fits u64");
        let selected = usize::try_from(random.below(bound)).expect("draw fits slice index");
        values.swap(upper, selected);
    }
}

/// Each block contains every arm once, in an order drawn from the recorded seed.
#[must_use]
pub fn interleave(arms: usize, repetitions: u32, seed: u64) -> Vec<Vec<usize>> {
    let mut random = Rng::new(seed);
    (0..repetitions)
        .map(|_| {
            let mut block: Vec<usize> = (0..arms).collect();
            shuffle(&mut block, &mut random);
            block
        })
        .collect()
}

fn resample(values: &[f64], random: &mut Rng, sample: &mut Vec<f64>) {
    let bound = u64::try_from(values.len()).expect("observation count fits u64");
    sample.clear();
    for _ in 0..values.len() {
        sample.push(values[usize::try_from(random.below(bound)).expect("draw fits index")]);
    }
}

/// The fixture's paired value and pass observations, retained in its own stratum.
pub(crate) struct Stratum {
    pub reference: Vec<(f64, bool)>,
    pub candidate: Vec<(f64, bool)>,
}

fn solved_ratio(reference: (f64, u32), candidate: (f64, u32)) -> f64 {
    if reference.1 == 0 || candidate.1 == 0 || reference.0 == 0.0 {
        f64::INFINITY
    } else {
        (candidate.0 / f64::from(candidate.1)) / (reference.0 / f64::from(reference.1))
    }
}

fn draw_solved(values: &[(f64, bool)], random: &mut Rng, resample: bool) -> (f64, u32) {
    let mut sum = 0.0;
    let mut passes = 0;
    let bound = u64::try_from(values.len()).expect("stratum size");
    for index in 0..values.len() {
        let index = if resample { usize::try_from(random.below(bound)).expect("draw") } else { index };
        sum += values[index].0;
        passes += u32::from(values[index].1);
    }
    (sum, passes)
}

fn pooled_solved(strata: &[Stratum], random: &mut Rng, resample: bool) -> f64 {
    let mut reference = (0.0, 0);
    let mut candidate = (0.0, 0);
    for stratum in strata {
        let observed = draw_solved(&stratum.reference, random, resample);
        reference.0 += observed.0;
        reference.1 += observed.1;
        let observed = draw_solved(&stratum.candidate, random, resample);
        candidate.0 += observed.0;
        candidate.1 += observed.1;
    }
    solved_ratio(reference, candidate)
}

/// Resample each fixture and arm separately, preserving value/pass pairing.
pub(crate) fn bootstrap_per_solved(strata: &[Stratum], seed: u64, resamples: u32) -> Result<(f64, (f64, f64)), String> {
    if strata.is_empty()
        || resamples < 1000
        || strata.iter().any(|stratum| {
            stratum.reference.iter().chain(&stratum.candidate).any(|(value, _)| !value.is_finite() || *value < 0.0)
        })
    {
        return Err("per-solved bootstrap needs finite nonnegative strata and 1000 resamples".into());
    }
    let mut random = Rng::new(seed);
    let ratio = pooled_solved(strata, &mut random, false);
    let constant = strata.iter().all(|stratum| {
        [&stratum.reference, &stratum.candidate]
            .iter()
            .all(|values| values.first().is_none_or(|first| values.iter().all(|value| value == first)))
    });
    if constant {
        return Ok((ratio, (ratio, ratio)));
    }
    let mut ratios = Vec::with_capacity(usize::try_from(resamples).expect("resamples"));
    for _ in 0..resamples {
        ratios.push(pooled_solved(strata, &mut random, true));
    }
    ratios.sort_by(f64::total_cmp);
    let lower = usize::try_from(u64::from(resamples) * 25 / 1000).expect("percentile");
    let upper = usize::try_from(u64::from(resamples) * 975 / 1000).expect("percentile");
    Ok((ratio, (ratios[lower], ratios[upper.min(ratios.len() - 1)])))
}

/// Bootstrap the ratio of medians; at least five complete observations per arm.
/// Detection needs both an interval excluding one and an effect above the MDE.
pub fn bootstrap_ratio(reference: &[f64], candidate: &[f64], seed: u64, resamples: u32) -> Result<Comparison, String> {
    validate(reference)?;
    validate(candidate)?;
    if reference.len() < 5 || candidate.len() < 5 || resamples < 1000 {
        return Err("comparison needs at least five attempts per arm and 1000 resamples".into());
    }
    if reference.contains(&0.0) {
        return Err("reference zero can make a bootstrap ratio undefined".into());
    }
    let ratio = median(candidate)? / median(reference)?;
    // Constant arms make every percentile draw the same ratio.
    if reference.iter().all(|value| value.to_bits() == reference[0].to_bits())
        && candidate.iter().all(|value| value.to_bits() == candidate[0].to_bits())
        && candidate[0] > 0.0
        && ratio.is_finite()
    {
        return Ok(Comparison {
            ratio,
            interval_95: Interval { lower: ratio, upper: ratio },
            minimum_detectable_effect: 0.0,
            detected: ratio.to_bits() != 1.0_f64.to_bits(),
            resamples,
        });
    }
    let mut random = Rng::new(seed);
    let mut ratios = Vec::with_capacity(usize::try_from(resamples).expect("resamples fit usize"));
    let mut candidate_sample = Vec::with_capacity(candidate.len());
    let mut reference_sample = Vec::with_capacity(reference.len());
    for _ in 0..resamples {
        resample(candidate, &mut random, &mut candidate_sample);
        resample(reference, &mut random, &mut reference_sample);
        ratios.push(sample_median(&mut candidate_sample) / sample_median(&mut reference_sample));
    }
    if !ratio.is_finite() || ratios.iter().any(|ratio| !ratio.is_finite()) {
        return Err("ratio exceeded finite precision".into());
    }
    ratios.sort_by(f64::total_cmp);
    let lower = usize::try_from(u64::from(resamples) * 25 / 1000).expect("percentile fits usize");
    let upper =
        usize::try_from(u64::from(resamples) * 975 / 1000).expect("percentile fits usize").min(ratios.len() - 1);
    let interval = Interval { lower: ratios[lower], upper: ratios[upper] };
    let attempts =
        u32::try_from(reference.len().min(candidate.len())).map_err(|_| "too many comparison observations")?;
    let cv = coefficient_of_variation(reference)?.max(coefficient_of_variation(candidate)?);
    let effect = minimum_detectable_effect(cv, attempts)?;
    Ok(Comparison {
        ratio,
        interval_95: interval,
        minimum_detectable_effect: effect,
        detected: (interval.upper < 1.0 || interval.lower > 1.0) && (ratio - 1.0).abs() > effect,
        resamples,
    })
}

fn binomial_cdf(n: u32, maximum: u32, probability: f64) -> f64 {
    if probability == 0.0 || maximum >= n {
        return 1.0;
    }
    if probability >= 1.0 {
        return 0.0;
    }
    let mut log_mass = f64::from(n) * (-probability).ln_1p();
    let mut sum = log_mass.exp();
    let log_odds = probability.ln() - (-probability).ln_1p();
    for successes in 1..=maximum {
        log_mass += f64::from(n - successes + 1).ln() - f64::from(successes).ln() + log_odds;
        sum += log_mass.exp();
    }
    sum.clamp(0.0, 1.0)
}

/// The equal-tailed 95% Clopper–Pearson interval, by binomial-tail inversion.
/// Empty counts have no rate; the computational bound refuses corrupt counts.
pub fn pass_interval(passed: u32, total: u32) -> Result<Interval, String> {
    if total == 0 || passed > total || total > 1_000_000 {
        return Err("pass interval needs 0 <= passed <= total <= 1000000, with positive total".into());
    }
    let mut lower = 0.0;
    let mut upper = 1.0;
    if passed > 0 {
        let mut low = 0.0;
        let mut high = 1.0;
        for _ in 0..64 {
            let middle = f64::midpoint(low, high);
            if binomial_cdf(total, total - passed, 1.0 - middle) < 0.025 {
                low = middle;
            } else {
                high = middle;
            }
        }
        lower = f64::midpoint(low, high);
    }
    if passed < total {
        let mut low = 0.0;
        let mut high = 1.0;
        for _ in 0..64 {
            let middle = f64::midpoint(low, high);
            if binomial_cdf(total, passed, middle) > 0.025 {
                low = middle;
            } else {
                high = middle;
            }
        }
        upper = f64::midpoint(low, high);
    }
    Ok(Interval { lower, upper })
}

/// Draw distinct audit indices, always including check/grader disagreements.
pub fn audit_sample(attempts: usize, count: usize, disagreements: &[usize], seed: u64) -> Result<Vec<usize>, String> {
    if disagreements.iter().any(|index| *index >= attempts) {
        return Err("audit disagreement index is outside the attempts".into());
    }
    let mut indices: Vec<usize> = (0..attempts).collect();
    shuffle(&mut indices, &mut Rng::new(seed));
    let mut chosen: BTreeSet<usize> = indices.into_iter().take(count).collect();
    chosen.extend(disagreements);
    Ok(chosen.into_iter().collect())
}
