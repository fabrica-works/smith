//! Comparisons pair only the cross-agent basis in benchmarks.md, section 9.

use std::collections::BTreeMap;

use crate::{ArmSummary, AttemptResult, Convention, Measure, SummaryComparison, bootstrap_ratio};

pub(crate) fn comparisons(
    groups: &[ArmSummary],
    grouped: &BTreeMap<(String, String), Vec<&AttemptResult>>,
    design: crate::Design,
    seed: u64,
    reference_arm: Option<&str>,
) -> Result<Vec<SummaryComparison>, String> {
    let mut comparisons = Vec::new();
    let tasks: std::collections::BTreeSet<_> = groups.iter().map(|group| group.task.as_str()).collect();
    for task in tasks {
        let repository = task.starts_with("repository/");
        if !repository && design != crate::Design::Interleaved {
            continue;
        }
        let arms: Vec<_> = groups.iter().filter(|group| group.task == task).collect();
        if arms.len() < 2 {
            continue;
        }
        let left = arms
            .iter()
            .find(|group| group.agent == crate::Agent::Codex)
            .or_else(|| arms.iter().find(|group| Some(group.arm.as_str()) == reference_arm))
            .ok_or_else(|| format!("{task}: comparison without Codex needs its named reference arm"))?;
        for right in &arms {
            if right.arm == left.arm {
                continue;
            }
            if left.task_version != right.task_version
                || left.seed_sha256 != right.seed_sha256
                || left.prompt_sha256 != right.prompt_sha256
            {
                return Err("comparison mixes task versions or frozen inputs".into());
            }
            for metric in left.metrics.keys() {
                let Some((candidate_metric, conventions)) = counterpart(metric, left, right) else { continue };
                let (reference_values, reference_missing) =
                    super::observations(&grouped[&(left.task.clone(), left.arm.clone())], metric);
                let (candidate_values, candidate_missing) =
                    super::observations(&grouped[&(right.task.clone(), right.arm.clone())], &candidate_metric);
                let large_effect = repository.then(|| {
                    let measures = |arm: &str, metric: &str| {
                        grouped[&(task.into(), arm.into())]
                            .iter()
                            .map(|result| {
                                super::metric_values(result)
                                    .remove(metric)
                                    .unwrap_or_else(|| Measure::unavailable("metric not reported"))
                            })
                            .collect::<Vec<_>>()
                    };
                    crate::large_effect(&measures(&left.arm, metric), &measures(&right.arm, &candidate_metric))
                });
                let result = if repository {
                    Measure::unavailable("repository tasks claim range separation, not significance")
                } else if reference_missing.is_empty() && candidate_missing.is_empty() {
                    bootstrap_ratio(&reference_values, &candidate_values, seed, 10_000)
                        .map_or_else(Measure::unavailable, |value| Measure::Observed { value })
                } else {
                    Measure::unavailable("comparison needs observed values for every attempt, regardless of end")
                };
                comparisons.push(SummaryComparison {
                    task: left.task.clone(),
                    reference: left.arm.clone(),
                    candidate: right.arm.clone(),
                    metric: if conventions.is_some() {
                        format!("tokens/{}", metric.rsplit('/').next().expect("token field"))
                    } else {
                        metric.clone()
                    },
                    conventions,
                    result,
                    large_effect,
                });
            }
        }
    }
    Ok(comparisons)
}

fn convention(name: &str) -> Option<Convention> {
    match name {
        "codex" => Some(Convention::Codex),
        "smith-events" => Some(Convention::SmithEvents),
        "claude-code" => Some(Convention::ClaudeCode),
        _ => None,
    }
}

fn counterpart(
    metric: &str,
    reference: &ArmSummary,
    candidate: &ArmSummary,
) -> Option<(String, Option<(Convention, Convention)>)> {
    if reference.agent == candidate.agent {
        return candidate.metrics.contains_key(metric).then(|| (metric.into(), None));
    }
    if matches!(metric, "task_wall_ms" | "process_wall_ms" | "spend_usd/priced") {
        return candidate.metrics.contains_key(metric).then(|| (metric.into(), None));
    }
    let tokens = metric.strip_prefix("tokens/")?;
    let (name, field) = tokens.split_once('/')?;
    let reference_convention = convention(name)?;
    for key in candidate.metrics.keys() {
        if let Some((name, candidate_field)) = key.strip_prefix("tokens/").and_then(|key| key.split_once('/'))
            && candidate_field == field
        {
            return Some((key.clone(), Some((reference_convention, convention(name)?))));
        }
    }
    None
}
