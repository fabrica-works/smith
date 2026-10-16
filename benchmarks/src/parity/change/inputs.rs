//! Named run inputs retain their path and content hash for offline recomputation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{ChangeSection, Stop};
use crate::RunSummary;

/// A named run file supplied to a change report; missing files have no digest.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub path: PathBuf,
    pub sha256: Option<String>,
}

/// The caller's baseline run and landed change runs, in their supplied order.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    pub baseline_run: Option<PathBuf>,
    pub landed: Vec<PathBuf>,
}

impl Inputs {
    /// Remove the named run options, retaining other command options for their owner.
    pub fn parse(flags: &[String]) -> Result<(Self, Vec<String>), String> {
        if !flags.len().is_multiple_of(2) {
            return Err("options require values".into());
        }
        let mut inputs = Self::default();
        let mut rest = Vec::new();
        for pair in flags.chunks_exact(2) {
            match pair[0].as_str() {
                "--baseline-run" => {
                    if inputs.baseline_run.replace(PathBuf::from(&pair[1])).is_some() {
                        return Err("baseline run given more than once".into());
                    }
                }
                "--landed" => inputs.landed.push(PathBuf::from(&pair[1])),
                _ => rest.extend_from_slice(pair),
            }
        }
        Ok((inputs, rest))
    }

    /// Whether a caller supplied any change-only input.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.baseline_run.is_none() && self.landed.is_empty()
    }
}

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("benchmark repository")
}

fn record(path: &Path) -> Input {
    let absolute =
        path.canonicalize().unwrap_or_else(|_| if path.is_absolute() { path.into() } else { repository().join(path) });
    let sha256 = crate::arm::binary_digest(&absolute).ok();
    // Repository-relative paths survive a checkout move; external inputs keep their absolute path.
    Input { path: absolute.strip_prefix(repository()).unwrap_or(&absolute).into(), sha256 }
}

fn read(input: &Input) -> Result<RunSummary, String> {
    let path = repository().join(&input.path);
    let digest = crate::arm::binary_digest(&path)?;
    if input.sha256.as_ref() != Some(&digest) {
        return Err(format!("input SHA-256 differs: {}", input.path.display()));
    }
    crate::read_summary(&path).map_err(|error| error.to_string())
}

fn evaluate(
    summary: &RunSummary,
    original: &ChangeSection,
    baseline: &Input,
    landed: &[Input],
) -> Result<ChangeSection, String> {
    let baseline = read(baseline)?;
    let landed = landed.iter().map(read).collect::<Result<Vec<_>, _>>()?;
    Ok(super::project_report(summary, original.clone(), &baseline, &landed).unwrap_or_else(|why| {
        let mut report = original.clone();
        report.stop = Some(Stop::Incomplete { why });
        report
    }))
}

/// Attach named inputs and retain an incomplete report when any input is refused.
pub fn attach(summary: &mut RunSummary, inputs: &Inputs) -> Result<(), String> {
    let parent = &summary.change.as_ref().ok_or("change section missing")?.parent_commit;
    let original = super::from_summary(summary, parent)?;
    let baseline_run = inputs.baseline_run.as_deref().map(record);
    let landed: Vec<_> = inputs.landed.iter().map(|path| record(path)).collect();
    let mut report = baseline_run
        .as_ref()
        .ok_or_else(|| "baseline run missing".into())
        .and_then(|baseline| evaluate(summary, &original, baseline, &landed))
        .unwrap_or_else(|why| {
            let mut report = original.clone();
            report.stop = Some(Stop::Incomplete { why });
            report
        });
    report.baseline_run = baseline_run;
    report.landed = landed;
    summary.change = Some(report);
    Ok(())
}

/// Refuse changed inputs or a stored section that disagrees with its rows and inputs.
pub fn verify(summary: &RunSummary) -> Result<ChangeSection, String> {
    let stored = summary.change.as_ref().ok_or("change section missing")?;
    let baseline = stored.baseline_run.as_ref().ok_or("baseline run missing")?;
    let original = super::from_summary(summary, &stored.parent_commit)?;
    let mut report = evaluate(summary, &original, baseline, &stored.landed)?;
    report.baseline_run.clone_from(&stored.baseline_run);
    report.landed.clone_from(&stored.landed);
    if stored != &report {
        return Err("committed change section disagrees with rows or inputs".into());
    }
    Ok(report)
}

/// Verify and print a change report, returning the offline command's exit code.
#[must_use]
pub fn command(file: &Path) -> u8 {
    let result = crate::read_summary(file).map_err(|error| error.to_string()).and_then(|summary| verify(&summary));
    match result {
        Ok(report) => {
            for ratio in &report.ratios {
                println!(
                    "{} statistic={} interval=[{}, {}]",
                    ratio.name, ratio.value.statistic, ratio.value.interval.0, ratio.value.interval.1
                );
            }
            println!(
                "O change={} parent={} attempts={} parent_lower={}",
                report.outcome.change_passes,
                report.outcome.parent_passes,
                report.outcome.attempts,
                report.outcome.parent_lower
            );
            for projection in &report.projection {
                println!(
                    "{} projection={} baseline_interval=[{}, {}]",
                    projection.name,
                    projection.statistic,
                    projection.baseline_interval.0,
                    projection.baseline_interval.1
                );
            }
            println!("stop={:?}", report.stop);
            0
        }
        Err(reason) => {
            eprintln!("{reason}");
            3
        }
    }
}
