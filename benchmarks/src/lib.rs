//! Task manifests and their offline validation; no agent or credentials are read.
//! The harness keeps task specifications, never an agent's internal state.
//! `read_task` parses strictly and `check_task` validates the frozen inputs,
//! following benchmarks.md, sections 5.2 and 14.

#![forbid(unsafe_code)]

mod bundle;
mod check;
mod formats;
mod judgment;
mod models;
mod passive;
mod result;
mod stats;
mod suite;
mod summary;
mod task;
mod tokens;

pub mod agent;

pub mod arm;

pub mod guard;

pub mod credential;

pub mod isolate;

pub mod process;

pub mod grade;

pub use agent::{Agent, Configuration, PinnedConfiguration, Provider, read_configuration};
pub use models::{ModelChoice, ModelTiers, read_model_tiers};
pub use result::{
    Artifact, AttemptResult, BudgetLimit, CheckVerdict, Child, ChildStop, Clamp, Classification, Counts, Delegation,
    End, Exit, FailureReason, Forced, Headroom, Health, Identity, Measure, ModelIdentity, Outcome, RESULT_VERSION,
    Requested, ResourceScope, Resources, Share, ShareBound, SharePart, SmithFace, Spend, SpendBasis, TimeRecord,
    Timing, classify_smith_exit, parse_result, read_result, render_result,
};
pub use stats::{
    Comparison, Interval, LargeEffect, ProbeVerdict, audit_sample, bootstrap_ratio, coefficient_of_variation,
    interleave, large_effect, median, minimum_detectable_effect, pass_interval, probe_verdict,
};
pub use suite::{
    Arm, CatalogueTask, Design, GuardSelection, Selection, Suite, SuiteAgent, catalogue, choose_guards,
    choose_guards_with_costs, include, read_suite, validate_suite,
};
pub use summary::{
    ArmSummary, AttemptRow, BASELINE_VERSION, Baseline, MetricSummary, PassCounts, PeakScope, RunSummary,
    SUMMARY_VERSION, SummaryComparison, baseline, baseline_group, committed_costs, read_baseline, read_results,
    read_summary, summarise, write_summary,
};
pub use tokens::{Convention, ResponseTokens, ScopeKind, TokenCounts, TokenLedger, TokenRecord, Usage};

pub use judgment::{Judgment, judge, judge_suite};

pub use passive::{HeadroomOf, Passive};

pub use bundle::{BundleCheck, BundleManifest, Pin, check_bundle, read_bundle_manifest, seed_source};
pub use check::{
    ManifestCounts, check_benchmark_tree, check_benchmark_tree_with_bundles, check_task, check_tree, seed_digest,
};
pub use task::{
    Before, Budget, Calibration, Count, Estimate, EventCheck, Grade, Kind, OutcomeCheck, Refusal, Task, Variant,
    read_task,
};

#[cfg(test)]
mod tests;

pub mod run;

pub mod batch;

pub mod fix;

mod cache;

mod time_record;

pub use cache::{CacheObservation, CacheTriplet};

pub mod probe;

pub mod parity;

pub mod validate;
