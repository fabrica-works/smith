//! Summaries keep every terminal, observation count and explicit measurement gap.
//! `summarise` reads only versioned attempt results, samples audits with its seed
//! and compares like task inputs; `baseline` freezes one configuration and tier.
//! Drift asks for another comparison, never declares a regression
//! (benchmarks.md, sections 5.4, 8 and 9).

mod compare;

#[cfg(test)]
mod read_tests;

#[cfg(test)]
std::thread_local! {
    static VALUE_DECODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The one committed summary format understood by this harness.
pub const SUMMARY_VERSION: u32 = 4;

/// The one baseline format understood by this harness.
pub const BASELINE_VERSION: u32 = 1;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    Agent, AttemptResult, Comparison, Convention, Design, End, Identity, Interval, Measure, ModelIdentity, Provider,
    RESULT_VERSION, Refusal, ResourceScope, ScopeKind, SmithFace, SpendBasis, TokenRecord, audit_sample,
    coefficient_of_variation, median, minimum_detectable_effect, pass_interval, read_result,
};

/// Observations of one metric across every end, supplied by the summary builder.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MetricSummary {
    pub observed: u32,
    pub total_attempts: u32,
    pub missing: Vec<String>,
    pub median: Measure<f64>,
    pub range: Measure<Interval>,
    pub cv: Measure<f64>,
    pub minimum_detectable_effect: Measure<f64>,
}

/// Check counts supplied by the builder, with setup and budgets apart from rates.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PassCounts {
    pub passed: u32,
    pub failed: u32,
    pub unavailable: u32,
    pub budget: u32,
    pub setup: u32,
    pub interval_95: Measure<Interval>,
}

/// A frozen task/arm's summary supplied to reports and baselines.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArmSummary {
    pub task: String,
    pub task_version: u32,
    pub seed_sha256: String,
    pub prompt_sha256: String,
    pub arm: String,
    pub agent: Agent,
    pub agent_version: String,
    pub binary_sha256: String,
    pub smith_face: Option<SmithFace>,
    pub configuration_sha256: String,
    pub provider: Provider,
    pub model: ModelIdentity,
    pub attempts: Vec<String>,
    pub counts_by_end: BTreeMap<String, u32>,
    pub passes: PassCounts,
    pub metrics: BTreeMap<String, MetricSummary>,
    /// Canonical cache triplets retain each repetition and the deciding median.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache_triplets: Vec<crate::CacheTriplet>,
}

/// One like-input metric comparison supplied by the seeded bootstrap.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SummaryComparison {
    pub task: String,
    pub reference: String,
    pub candidate: String,
    pub metric: String,
    /// Reference and candidate conventions for a token field across agents.
    pub conventions: Option<(Convention, Convention)>,
    pub result: Measure<Comparison>,
    /// Repository tasks claim only separation of complete observed ranges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub large_effect: Option<crate::LargeEffect>,
}

/// The process boundary attached to the peak retained in an attempt row.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PeakScope {
    /// The agent reports its own process's resident peak.
    Process,
    /// The harness counts the agent's process tree.
    Tree,
}

fn peak(result: &AttemptResult) -> (Measure<u64>, PeakScope) {
    match &result.resources.own_peak_rss_bytes {
        value @ Measure::Observed { .. } => (value.clone(), PeakScope::Process),
        Measure::LowerBound { .. } | Measure::Unavailable { .. } => {
            (result.resources.peak_rss_bytes.clone(), PeakScope::Tree)
        }
    }
}

/// One attempt's evidence, retained by the builder for an offline verdict.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttemptRow {
    pub attempt: String,
    pub task: String,
    pub arm: String,
    pub agent: String,
    pub agent_version: String,
    pub time_record: crate::TimeRecord,
    pub end: String,
    pub passed: bool,
    pub task_wall_ms: Measure<u64>,
    pub priced_usd: Measure<f64>,
    pub responses: Measure<u64>,
    pub peak_rss_bytes: Measure<u64>,
    pub peak_rss_scope: PeakScope,
    pub fresh_tokens: Measure<u64>,
    pub cache_read_tokens: Measure<u64>,
    pub output_tokens: Measure<u64>,
}

fn attempt_row(result: &AttemptResult) -> AttemptRow {
    let total = result.tokens.iter().find(|record| record.role == ScopeKind::Total);
    let (peak_rss_bytes, peak_rss_scope) = peak(result);
    AttemptRow {
        attempt: result.identity.attempt.clone(),
        task: result.identity.task.clone(),
        arm: result.identity.arm.clone(),
        agent: result.identity.agent.directory().into(),
        agent_version: result.identity.agent_version.clone(),
        time_record: result.timing.record.clone(),
        end: end_name(&result.outcome.end).into(),
        passed: crate::judge(result) == crate::Judgment::Pass,
        task_wall_ms: result.timing.task_wall_ms.clone(),
        priced_usd: result.spend.priced.clone(),
        responses: result.counts.responses.clone(),
        peak_rss_bytes,
        peak_rss_scope,
        fresh_tokens: total
            .map_or_else(|| Measure::unavailable("total tokens not reported"), |record| record.usage.fresh.clone()),
        cache_read_tokens: total.map_or_else(
            || Measure::unavailable("total tokens not reported"),
            |record| record.usage.cache_read.clone(),
        ),
        output_tokens: total
            .map_or_else(|| Measure::unavailable("total tokens not reported"), |record| record.usage.output.clone()),
    }
}

/// A committed run summary supplied by the offline summary command.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RunSummary {
    pub version: u32,
    pub result_version: u32,
    pub priced_at: String,
    pub run: String,
    pub suite: String,
    pub tier: String,
    pub design: Design,
    pub seed: u64,
    pub date: String,
    pub attempts: Vec<AttemptRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parity: Option<crate::parity::Parity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<crate::parity::change::ChangeSection>,
    pub groups: Vec<ArmSummary>,
    pub comparisons: Vec<SummaryComparison>,
    pub audits: Vec<String>,
    pub drift_notices: Vec<String>,
}

/// A one-configuration, one-tier drift reference supplied by the baseline author.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub version: u32,
    pub result_version: u32,
    pub suite: String,
    pub tier: String,
    pub date: String,
    pub run: String,
    pub groups: Vec<ArmSummary>,
}

fn observed(value: f64) -> Measure<f64> {
    Measure::Observed { value }
}

pub(crate) fn measure_u64(measure: &Measure<u64>) -> Measure<f64> {
    // Integers above 2^53 are not exactly representable by the statistics.
    let convert = |value: u64| {
        if value <= (1_u64 << 53) {
            Some(
                f64::from(u32::try_from(value >> 32).expect("high half fits u32")) * 4_294_967_296.0
                    + f64::from(u32::try_from(value & 0xffff_ffff).expect("low half fits u32")),
            )
        } else {
            None
        }
    };
    match measure {
        Measure::Observed { value } => convert(*value)
            .map_or_else(|| Measure::unavailable("integer exceeds exact statistical precision"), observed),
        Measure::LowerBound { value, missing } => convert(*value).map_or_else(
            || Measure::unavailable("integer exceeds exact statistical precision"),
            |value| Measure::LowerBound { value, missing: missing.clone() },
        ),
        Measure::Unavailable { reason } => Measure::unavailable(reason.clone()),
    }
}

fn metric_values(result: &AttemptResult) -> BTreeMap<String, Measure<f64>> {
    let mut values = BTreeMap::new();
    values.insert("responses".into(), measure_u64(&result.counts.responses));
    let (peak, scope) = peak(result);
    let scope = match scope {
        PeakScope::Process => "process",
        PeakScope::Tree => "tree",
    };
    values.insert(format!("peak_rss_bytes/{scope}"), measure_u64(&peak));
    for (name, value) in [
        ("process_wall_ms", &result.timing.process_wall_ms),
        ("task_wall_ms", &result.timing.task_wall_ms),
        ("teardown_ms", &result.timing.teardown_ms),
        ("grading_wall_ms", &result.timing.grading_wall_ms),
        ("first_byte_ms", &result.timing.first_byte_ms),
        ("largest_gap_ms", &result.timing.largest_gap_ms),
        ("longest_completion_ms", &result.timing.longest_completion_ms),
        ("time/requests_ms", &result.timing.record.requests_ms),
        ("time/first_byte_ms", &result.timing.record.first_byte_ms),
        ("time/largest_gap_ms", &result.timing.record.largest_gap_ms),
        ("time/compaction_ms", &result.timing.record.compaction_ms),
        ("time/children_ms", &result.timing.record.children_ms),
        ("time/rest_ms", &result.timing.record.rest_ms),
        ("time/commands_outside_calls_ms", &result.timing.record.commands_outside_calls_ms),
    ] {
        values.insert(name.into(), measure_u64(value));
    }
    for (name, value) in &result.timing.record.tools_ms {
        values.insert(format!("time/tools/{name}_ms"), measure_u64(value));
    }
    for (name, scope, metric) in [
        ("cpu_ms", result.resources.cpu_scope, &result.resources.cpu_ms),
        ("peak_rss_bytes", result.resources.peak_memory_scope, &result.resources.peak_rss_bytes),
    ] {
        let scope = match scope {
            ResourceScope::Cgroup => "cgroup",
            ResourceScope::PidfdWalk => "pidfd-walk",
            ResourceScope::ProcessGroup => "process-group",
            ResourceScope::LargestProcess => "largest-process",
            ResourceScope::AgentProcess => "agent-process",
            ResourceScope::Unavailable => "unavailable",
        };
        values.insert(format!("{name}/{scope}"), measure_u64(metric));
    }
    let basis = match result.spend.basis {
        SpendBasis::Notional => "notional",
        SpendBasis::Priced => "priced",
        SpendBasis::Reported => "reported",
    };
    values.insert(format!("spend_usd/{basis}"), result.spend.usd.clone());
    for TokenRecord { role, convention, usage, .. } in &result.tokens {
        if *role != ScopeKind::Total {
            continue;
        }
        let convention = match convention {
            Convention::Codex => "codex",
            Convention::ClaudeCode => "claude-code",
            Convention::SmithEvents => "smith-events",
        };
        for (name, value) in [
            ("fresh", &usage.fresh),
            ("cache_read", &usage.cache_read),
            ("cache_write", &usage.cache_write),
            ("output", &usage.output),
            ("reasoning", &usage.reasoning),
        ] {
            values.insert(format!("tokens/{convention}/{name}"), measure_u64(value));
        }
    }
    values.insert("spend_usd/priced".into(), result.spend.priced.clone());
    values.insert("delegation/turns".into(), measure_u64(&result.delegation.turns));
    values.insert("delegation/parent_blocked_ms".into(), measure_u64(&result.delegation.parent_blocked_ms));
    for (name, budget_only) in [("children", false), ("children_budget", true)] {
        let count = match &result.delegation.children {
            Measure::Observed { value } => measure_u64(&Measure::Observed {
                value: u64::try_from(
                    value.iter().filter(|child| !budget_only || child.stop == crate::ChildStop::Budget).count(),
                )
                .expect("children fit u64"),
            }),
            Measure::LowerBound { missing, .. } => Measure::unavailable(missing.join("; ")),
            Measure::Unavailable { reason } => Measure::unavailable(reason),
        };
        values.insert(format!("delegation/{name}"), count);
    }

    for (name, count) in &result.counts.tool_calls {
        values.insert(format!("tool_calls/{name}"), measure_u64(count));
    }
    if let Some(cache) = &result.cache {
        values.insert(
            "cache/reuse".into(),
            match cache {
                Measure::Observed { value } => observed(value.reuse()),
                Measure::LowerBound { missing, .. } => Measure::unavailable(missing.join("; ")),
                Measure::Unavailable { reason } => Measure::unavailable(reason),
            },
        );
    }
    values
}

fn observations(results: &[&AttemptResult], metric: &str) -> (Vec<f64>, Vec<String>) {
    let mut values = Vec::new();
    let mut missing = Vec::new();
    for result in results {
        match metric_values(result).remove(metric) {
            Some(Measure::Observed { value }) if value.is_finite() && value >= 0.0 => values.push(value),
            Some(Measure::Observed { value: _ }) => missing.push(format!("{}: invalid value", result.identity.attempt)),
            Some(Measure::LowerBound { value: _, missing: reasons }) => {
                missing.push(format!("{}: lower bound ({})", result.identity.attempt, reasons.join("; ")));
            }
            Some(Measure::Unavailable { reason }) => missing.push(format!("{}: {reason}", result.identity.attempt)),
            None => missing.push(format!("{}: metric not reported", result.identity.attempt)),
        }
    }
    (values, missing)
}

fn metric_summary(results: &[&AttemptResult], metric: &str) -> Result<MetricSummary, String> {
    let (values, missing) = observations(results, metric);
    let n = u32::try_from(values.len()).map_err(|_| "too many observations")?;
    let total = u32::try_from(results.len()).map_err(|_| "too many attempts")?;
    let median = median(&values).map_or_else(Measure::unavailable, observed);
    let range = if values.is_empty() {
        Measure::unavailable("no observed values")
    } else {
        Measure::Observed {
            value: Interval {
                lower: values.iter().copied().fold(f64::INFINITY, f64::min),
                upper: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            },
        }
    };
    let cv = coefficient_of_variation(&values).map_or_else(Measure::unavailable, observed);
    let mde = coefficient_of_variation(&values)
        .and_then(|cv| minimum_detectable_effect(cv, n))
        .map_or_else(Measure::unavailable, observed);
    Ok(MetricSummary { observed: n, total_attempts: total, missing, median, range, cv, minimum_detectable_effect: mde })
}

fn checks_passed(result: &AttemptResult) -> Option<bool> {
    let mut unknown = !result.outcome.deciding.iter().any(|name| name != "grade");
    for name in &result.outcome.deciding {
        unknown |= name != "grade" && !result.outcome.checks.iter().any(|check| check.check == *name);
    }
    for check in &result.outcome.checks {
        if check.check == "grade" || !result.outcome.deciding.contains(&check.check) {
            continue;
        }
        match &check.passed {
            Measure::Observed { value: false } => return Some(false),
            Measure::Observed { value: true } => {}
            Measure::LowerBound { .. } | Measure::Unavailable { .. } => unknown = true,
        }
    }
    if unknown { None } else { Some(true) }
}

pub(crate) fn passed(result: &AttemptResult) -> Option<bool> {
    match crate::judge(result) {
        crate::Judgment::Pass => Some(true),
        crate::Judgment::Fail => Some(false),
        crate::Judgment::Incomplete => None,
    }
}

fn end_name(end: &End) -> &'static str {
    match end {
        End::Completed => "completed",
        End::Failed { .. } => "failed",
        End::Budget { .. } => "budget",
        End::Refused { .. } => "refused-setup",
        End::Timeout => "timeout",
        End::HarnessError { .. } => "harness-error",
    }
}

pub(crate) fn same_inputs(left: &Identity, right: &Identity) -> bool {
    left.task_version == right.task_version
        && left.seed_sha256 == right.seed_sha256
        && left.prompt_sha256 == right.prompt_sha256
        && left.agent == right.agent
        && left.agent_version == right.agent_version
        && left.binary_sha256 == right.binary_sha256
        && left.smith_face == right.smith_face
        && left.configuration_sha256 == right.configuration_sha256
        && left.user_configuration_sha256 == right.user_configuration_sha256
        && left.provider == right.provider
        && left.model == right.model
        && left.host == right.host
        && left.cpus == right.cpus
        && left.harness_commit == right.harness_commit
}

fn group_summary(results: &[&AttemptResult]) -> Result<ArmSummary, String> {
    let first = &results[0].identity;
    if results.iter().any(|result| !same_inputs(first, &result.identity)) {
        return Err(format!("{} / {} mixes task versions or arm inputs", first.task, first.arm));
    }
    let mut counts = BTreeMap::new();
    let mut passes = PassCounts {
        passed: 0,
        failed: 0,
        unavailable: 0,
        budget: 0,
        setup: 0,
        interval_95: Measure::unavailable("no eligible checks"),
    };
    let cache_triplets = if results.iter().any(|result| crate::cache::selected(result)) {
        crate::cache::triplets(results)
    } else {
        Vec::new()
    };
    for result in results {
        *counts.entry(end_name(&result.outcome.end).into()).or_insert(0_u32) += 1;
        if !cache_triplets.is_empty() {
            continue;
        }
        match &result.outcome.end {
            End::Budget { .. } => passes.budget += 1,
            End::Refused { .. } => passes.setup += 1,
            End::Completed | End::Failed { .. } | End::Timeout | End::HarnessError { .. } => match passed(result) {
                Some(true) => passes.passed += 1,
                Some(false) => passes.failed += 1,
                None => passes.unavailable += 1,
            },
        }
    }
    for (verdict, _) in &cache_triplets {
        match verdict {
            crate::Judgment::Pass => passes.passed += 1,
            crate::Judgment::Fail => passes.failed += 1,
            crate::Judgment::Incomplete => passes.unavailable += 1,
        }
    }
    if passes.unavailable == 0 {
        passes.interval_95 = pass_interval(passes.passed, passes.passed + passes.failed)
            .map_or_else(Measure::unavailable, |value| Measure::Observed { value });
    } else {
        passes.interval_95 = Measure::unavailable("some eligible attempt checks are unavailable");
    }
    let names: BTreeSet<String> = results.iter().flat_map(|result| metric_values(result).into_keys()).collect();
    let mut metrics = BTreeMap::new();
    for name in names {
        metrics.insert(name.clone(), metric_summary(results, &name)?);
    }
    Ok(ArmSummary {
        task: first.task.clone(),
        task_version: first.task_version,
        seed_sha256: first.seed_sha256.clone(),
        prompt_sha256: first.prompt_sha256.clone(),
        arm: first.arm.clone(),
        agent: first.agent,
        agent_version: first.agent_version.clone(),
        binary_sha256: first.binary_sha256.clone(),
        smith_face: first.smith_face,
        configuration_sha256: first.configuration_sha256.clone(),
        provider: first.provider,
        model: first.model.clone(),
        attempts: results.iter().map(|result| result.identity.attempt.clone()).collect(),
        counts_by_end: counts,
        passes,
        metrics,
        cache_triplets: cache_triplets.into_iter().map(|(_, summary)| summary).collect(),
    })
}

/// Summarise one run with every terminal, seeded comparisons, audits and drift.
pub fn summarise(
    results: &[AttemptResult],
    suite: &str,
    tier: &str,
    design: Design,
    seed: u64,
    reference: Option<&Baseline>,
    reference_arm: Option<&str>,
) -> Result<RunSummary, String> {
    if results.is_empty() || suite.is_empty() || tier.is_empty() || results.len() > u32::MAX as usize {
        return Err("summary needs attempts, suite and tier".into());
    }
    let run = &results[0].identity.run;
    let mut ordered: Vec<&AttemptResult> = results.iter().collect();
    ordered.sort_by(|left, right| left.identity.attempt.cmp(&right.identity.attempt));
    let mut ids = BTreeSet::new();
    let mut grouped: BTreeMap<(String, String), Vec<&AttemptResult>> = BTreeMap::new();
    let mut disagreements = Vec::new();
    for (index, result) in ordered.iter().enumerate() {
        if result.version != RESULT_VERSION
            || &result.identity.run != run
            || result.identity.harness_commit != results[0].identity.harness_commit
            || !ids.insert(&result.identity.attempt)
        {
            return Err("summary mixes runs/versions or duplicate attempts".into());
        }
        if let Some(Measure::Observed { value: grade }) = result.outcome.grade
            && let Some(checks) = checks_passed(result)
            && grade != checks
        {
            disagreements.push(index);
        }
        grouped.entry((result.identity.task.clone(), result.identity.arm.clone())).or_default().push(result);
    }
    let mut groups = Vec::new();
    for group in grouped.values() {
        groups.push(group_summary(group)?);
    }
    let comparisons = compare::comparisons(&groups, &grouped, design, seed, reference_arm)?;
    let audits = audit_sample(ordered.len(), 3, &disagreements, seed)?
        .into_iter()
        .map(|index| ordered[index].identity.attempt.clone())
        .collect();
    let date =
        ordered.iter().map(|result| result.identity.started_utc.as_str()).min().expect("nonempty run").to_owned();
    let drift_notices =
        if let Some(reference) = reference { drift(&groups, suite, tier, reference)? } else { Vec::new() };
    let mut summary = RunSummary {
        version: SUMMARY_VERSION,
        result_version: RESULT_VERSION,
        priced_at: results[0].identity.harness_commit.clone(),
        run: run.clone(),
        suite: suite.into(),
        tier: tier.into(),
        design,
        seed,
        date,
        attempts: results.iter().map(attempt_row).collect(),
        parity: None,
        change: None,
        groups,
        comparisons,
        audits,
        drift_notices,
    };
    if suite == "parity" {
        summary.parity = Some(crate::parity::from_summary(&summary)?);
    } else if suite == "parity-change" {
        let parent = summary
            .groups
            .iter()
            .find(|group| Some(group.arm.as_str()) == reference_arm)
            .ok_or("change needs its parent reference arm")?
            .agent_version
            .clone();
        summary.change = Some(crate::parity::change::from_summary(&summary, &parent)?);
    }
    Ok(summary)
}

fn drift(groups: &[ArmSummary], suite: &str, tier: &str, reference: &Baseline) -> Result<Vec<String>, String> {
    if reference.version != BASELINE_VERSION
        || reference.result_version != RESULT_VERSION
        || reference.suite != suite
        || reference.tier != tier
    {
        return Err("baseline version, suite or tier differs".into());
    }
    let mut notices = Vec::new();
    for group in groups {
        let old = reference.groups.iter().find(|old| old.task == group.task && old.arm == group.arm);
        let Some(old) = old else {
            continue;
        };
        if old.task_version != group.task_version
            || old.seed_sha256 != group.seed_sha256
            || old.prompt_sha256 != group.prompt_sha256
            || old.agent != group.agent
            || old.agent_version != group.agent_version
            || old.configuration_sha256 != group.configuration_sha256
            || old.provider != group.provider
            || old.model != group.model
            || old.smith_face != group.smith_face
        {
            notices.push(format!("{} / {}: baseline inputs changed; retake baseline", group.task, group.arm));
            continue;
        }
        for (name, metric) in &group.metrics {
            let Some(previous) = old.metrics.get(name) else {
                continue;
            };
            if let Measure::Observed { value: current } = metric.median
                && let Measure::Observed { value: previous_median } = previous.median
                && let Measure::Observed { value: effect } = previous.minimum_detectable_effect
                && previous_median > 0.0
                && (current / previous_median - 1.0).abs() > effect
            {
                notices.push(format!(
                    "{} / {} / {name}: drift beyond baseline MDE; compare in one session",
                    group.task, group.arm
                ));
            }
        }
    }
    Ok(notices)
}

/// Freeze a suite for one configuration and tier; comparisons use separate baselines.
pub fn baseline(summary: &RunSummary) -> Result<Baseline, String> {
    let first = summary.groups.first().ok_or("baseline needs a task")?;
    if summary.version != SUMMARY_VERSION
        || summary.result_version != RESULT_VERSION
        || summary.groups.iter().any(|group| {
            group.arm != first.arm
                || group.agent != first.agent
                || group.provider != first.provider
                || group.configuration_sha256 != first.configuration_sha256
                || group.model != first.model
                || group.binary_sha256 != first.binary_sha256
        })
    {
        return Err("baseline needs one supported agent configuration, model tier and binary".into());
    }
    Ok(Baseline {
        version: BASELINE_VERSION,
        result_version: RESULT_VERSION,
        suite: summary.suite.clone(),
        tier: summary.tier.clone(),
        date: summary.date.clone(),
        run: summary.run.clone(),
        groups: summary.groups.clone(),
    })
}

/// Select one agent configuration and model/effort from a whole suite summary.
pub fn baseline_group(
    summary: &RunSummary,
    agent: Agent,
    configuration: &str,
    model: &str,
    effort: &str,
) -> Result<Baseline, String> {
    let mut selected = summary.clone();
    selected.groups.retain(|group| {
        group.agent == agent
            && group.configuration_sha256 == configuration
            && group.model.requested_model == model
            && group.model.requested_effort == effort
    });
    baseline(&selected)
}

/// Read raw result.json files recursively, without following links or other logs.
pub fn read_results(path: &Path) -> Result<Vec<AttemptResult>, Refusal> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| Refusal::new(path, "results", error.to_string()))?;
    if metadata.is_symlink() {
        return Err(Refusal::new(path, "results", "result paths must not be symlinks"));
    }
    if metadata.is_file() {
        return Ok(vec![read_result(path)?]);
    }
    if !metadata.is_dir() {
        return Err(Refusal::new(path, "results", "expected a file or directory"));
    }
    let mut entries = std::fs::read_dir(path)
        .map_err(|error| Refusal::new(path, "results", error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| Refusal::new(path, "results", error.to_string()))?;
    entries.sort_by_key(std::fs::DirEntry::path);
    let mut results = Vec::new();
    for entry in entries {
        let kind = entry.file_type().map_err(|error| Refusal::new(&entry.path(), "results", error.to_string()))?;
        if kind.is_dir() || entry.file_name() == "result.json" {
            results.extend(read_results(&entry.path())?);
        }
    }
    Ok(results)
}

fn read_json<T: serde::de::DeserializeOwned>(file: &Path) -> Result<T, Refusal> {
    let text = std::fs::read_to_string(file).map_err(|error| Refusal::new(file, "document", error.to_string()))?;
    serde_json::from_str(&text).map_err(|error| Refusal::new(file, "document", error.to_string()))
}

/// Read the one summary version and the current result version.
pub fn read_summary(file: &Path) -> Result<RunSummary, Refusal> {
    let text = std::fs::read_to_string(file).map_err(|error| Refusal::new(file, "document", error.to_string()))?;
    decode_summary(file, &text)
}

fn decode_summary(file: &Path, text: &str) -> Result<RunSummary, Refusal> {
    if let Ok(summary) = serde_json::from_str::<RunSummary>(text)
        && summary.version == SUMMARY_VERSION
        && summary.result_version == RESULT_VERSION
    {
        return Ok(summary);
    }
    // Preserve version-first refusals and the value decoder's handling of
    // duplicate fields, without building a value tree for ordinary summaries.
    decode_summary_value(file, text)
}

fn decode_summary_value(file: &Path, text: &str) -> Result<RunSummary, Refusal> {
    #[cfg(test)]
    VALUE_DECODES.with(|count| count.set(count.get() + 1));

    let document: serde_json::Value =
        serde_json::from_str(text).map_err(|error| Refusal::new(file, "document", error.to_string()))?;
    if document["version"].as_u64() != Some(u64::from(SUMMARY_VERSION))
        || document["result_version"].as_u64() != Some(u64::from(RESULT_VERSION))
    {
        return Err(Refusal::new(
            file,
            "version",
            format!(
                "summary version {} or result version {} is unsupported",
                document["version"], document["result_version"]
            ),
        ));
    }
    serde_json::from_value(document).map_err(|error| Refusal::new(file, "document", error.to_string()))
}

/// Read a drift reference only at the supported versions.
pub fn read_baseline(file: &Path) -> Result<Baseline, Refusal> {
    let reference: Baseline = read_json(file)?;
    if reference.version != BASELINE_VERSION || reference.result_version != RESULT_VERSION {
        return Err(Refusal::new(file, "version", "unsupported baseline or result version"));
    }
    Ok(reference)
}

/// Write a committed summary or baseline without overwriting an existing record.
pub fn write_summary<T: Serialize>(file: &Path, document: &T) -> Result<(), String> {
    use std::io::Write;
    let text = serde_json::to_string_pretty(document).map_err(|error| error.to_string())?;
    let mut output =
        std::fs::OpenOptions::new().write(true).create_new(true).open(file).map_err(|error| error.to_string())?;
    output.write_all(format!("{text}\n").as_bytes()).map_err(|error| error.to_string())
}

fn rounded_cost(value: f64) -> Option<u64> {
    if value.is_finite() && (0.0..=9_007_199_254_740_992.0).contains(&value) {
        format!("{:.0}", value.ceil()).parse().ok()
    } else {
        None
    }
}

fn median_cost(group: &ArmSummary) -> Option<crate::Estimate> {
    let prefix = match group.agent {
        Agent::Smith => "smith-events",
        Agent::Codex => "codex",
        Agent::ClaudeCode => "claude-code",
    };
    let names = [
        "task_wall_ms".into(),
        format!("tokens/{prefix}/fresh"),
        format!("tokens/{prefix}/cache_read"),
        format!("tokens/{prefix}/output"),
    ];
    let mut values = Vec::new();
    for name in names {
        if let Some(metric) = group.metrics.get(&name)
            && metric.observed == metric.total_attempts
            && metric.missing.is_empty()
            && let Measure::Observed { value } = metric.median
        {
            values.push(value);
        }
    }
    if values.len() != 4 {
        return None;
    }
    Some(crate::Estimate {
        seconds: rounded_cost(values[0] / 1000.0)?,
        tokens: rounded_cost(values[1] + values[2] + values[3])?,
    })
}

/// Latest matching committed medians, covering every configured agent before
/// replacing an estimate; costs reserve the slowest and most expensive route.
pub fn committed_costs(
    suite: &crate::Suite,
    tasks: &[crate::CatalogueTask],
    summaries: &[RunSummary],
    agents: &Path,
    models: &crate::ModelTiers,
) -> Result<BTreeMap<String, crate::Estimate>, String> {
    use sha2::{Digest, Sha256};
    let mut pins = Vec::new();
    for agent in &suite.agents {
        let file = agents.join(agent.agent.directory()).join(format!("{}.pin.toml", agent.config));
        let pin = crate::read_configuration(&file).map_err(|error| error.to_string())?;
        let choice = models
            .lookup(&agents.join("models.toml"), &suite.tier, agent.provider)
            .map_err(|error| error.to_string())?;
        pins.push((pin, choice, agent));
    }
    let mut latest: BTreeMap<(String, String), (String, crate::Estimate)> = BTreeMap::new();
    for summary in summaries {
        if summary.version != SUMMARY_VERSION || summary.result_version != RESULT_VERSION {
            return Err("committed summary has unsupported summary or result version".into());
        }
        if summary.tier != suite.tier {
            continue;
        }
        for group in &summary.groups {
            let Some((pin, _, _)) = pins.iter().find(|(pin, choice, agent)| {
                (agent.tasks.is_empty() || agent.tasks.contains(&group.task))
                    && pin.pin.agent == group.agent
                    && pin.pin.provider == group.provider
                    && pin.pin.version == group.agent_version
                    && pin.sha256 == group.configuration_sha256
                    && choice.model.as_ref() == Some(&group.model.requested_model)
                    && choice.effort.as_ref() == Some(&group.model.requested_effort)
            }) else {
                continue;
            };
            let Some(task) = tasks
                .iter()
                .filter(|task| task.name == group.task || group.task.starts_with(&format!("{}/", task.name)))
                .max_by_key(|task| task.name.len())
            else {
                continue;
            };
            let prompt = if group.task == task.name {
                &task.task.prompt
            } else {
                let name = &group.task[task.name.len() + 1..];
                let Some(variant) = task.task.variant.iter().find(|variant| variant.name == name) else {
                    continue;
                };
                variant.prompt.as_ref().unwrap_or(&task.task.prompt)
            };
            if task.task.version != group.task_version
                || task.task.seed_sha256 != group.seed_sha256
                || format!("{:x}", Sha256::digest(prompt.as_bytes())) != group.prompt_sha256
            {
                continue;
            }
            let Some(cost) = median_cost(group) else {
                continue;
            };
            let key = (group.task.clone(), pin.sha256.clone());
            if let Some((date, previous)) = latest.get_mut(&key) {
                if &summary.date > date {
                    date.clone_from(&summary.date);
                    *previous = cost;
                } else if &summary.date == date {
                    previous.seconds = previous.seconds.max(cost.seconds);
                    previous.tokens = previous.tokens.max(cost.tokens);
                }
            } else {
                latest.insert(key, (summary.date.clone(), cost));
            }
        }
    }
    let names: BTreeSet<String> = latest.keys().map(|(name, _)| name.clone()).collect();
    let mut costs = BTreeMap::new();
    for name in names {
        let mut estimate = crate::Estimate { seconds: 0, tokens: 0 };
        let mut complete = true;
        for (pin, _, agent) in &pins {
            if !agent.tasks.is_empty() && !agent.tasks.contains(&name) {
                continue;
            }
            if let Some((_, cost)) = latest.get(&(name.clone(), pin.sha256.clone())) {
                estimate.seconds = estimate.seconds.max(cost.seconds);
                estimate.tokens = estimate.tokens.max(cost.tokens);
            } else {
                complete = false;
            }
        }
        if complete {
            costs.insert(name, estimate);
        }
    }
    Ok(costs)
}
