//! Repository range claims preserve measurement gaps (benchmarks.md, 3.1 and 9).

use std::path::PathBuf;

use smith_bench::{Agent, Design, LargeEffect, Measure, RunSummary, large_effect, summarise};

fn repository_summary(candidate: [Measure<u64>; 3], design: Design) -> RunSummary {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json");
    let recorded = smith_bench::read_result(&file).expect("recorded attempt");
    let mut results = Vec::new();
    for (arm, agent, values) in [
        (
            "codex",
            Agent::Codex,
            [Measure::Observed { value: 10 }, Measure::Observed { value: 20 }, Measure::Observed { value: 30 }],
        ),
        ("smith", Agent::Smith, candidate),
    ] {
        for (index, value) in values.into_iter().enumerate() {
            let mut result = recorded.clone();
            result.identity.task = "repository/shell-cwd".into();
            result.identity.attempt = format!("{arm}-{index}");
            result.identity.arm = arm.into();
            result.identity.agent = agent;
            result.timing.task_wall_ms = value;
            results.push(result);
        }
    }
    summarise(&results, "repository", "working", design, 17, None, None).expect("repository summary")
}

fn wall_effect(summary: &RunSummary) -> Option<LargeEffect> {
    summary
        .comparisons
        .iter()
        .find(|comparison| comparison.metric == "task_wall_ms")
        .expect("wall comparison")
        .large_effect
        .clone()
}

#[test]
fn a_repository_summary_marks_disjoint_ranges_as_large_effects() {
    let summary = repository_summary([const { Measure::Observed { value: 40 } }; 3], Design::Single);
    assert_eq!(wall_effect(&summary), Some(LargeEffect::Separated { above: true }));
}

#[test]
fn an_interleaved_repository_summary_still_claims_only_range_separation() {
    let summary = repository_summary([const { Measure::Observed { value: 40 } }; 3], Design::Interleaved);
    let wall =
        summary.comparisons.iter().find(|comparison| comparison.metric == "task_wall_ms").expect("wall comparison");
    assert_eq!(
        wall.result,
        Measure::Unavailable { reason: "repository tasks claim range separation, not significance".into() }
    );
}

#[test]
fn a_repository_summary_marks_overlapping_ranges_with_no_effect() {
    let summary = repository_summary([const { Measure::Observed { value: 20 } }; 3], Design::Single);
    assert_eq!(wall_effect(&summary), Some(LargeEffect::None));
}

#[test]
fn a_repository_lower_bound_keeps_its_reason_without_separation() {
    let summary = repository_summary(
        [
            Measure::Observed { value: 40 },
            Measure::Observed { value: 50 },
            Measure::LowerBound { value: 60, missing: vec!["child timing absent".into()] },
        ],
        Design::Single,
    );
    assert_eq!(
        wall_effect(&summary),
        Some(LargeEffect::Unclaimed { reason: "candidate: lower bound: child timing absent".into() })
    );
}

#[test]
fn candidate_values_below_the_reference_claim_a_downward_effect() {
    assert_eq!(
        large_effect(&[Measure::Observed { value: 10.0 }], &[Measure::Observed { value: 5.0 }]),
        LargeEffect::Separated { above: false }
    );
}

#[test]
fn ranges_that_share_an_endpoint_claim_no_large_effect() {
    assert_eq!(
        large_effect(
            &[Measure::Observed { value: 10.0 }],
            &[Measure::Observed { value: 10.0 }, Measure::Observed { value: 20.0 }]
        ),
        LargeEffect::None
    );
}

#[test]
fn missing_values_on_both_arms_keep_both_reasons() {
    assert_eq!(
        large_effect(
            &[Measure::unavailable("reference spans absent")],
            &[Measure::unavailable("candidate spans absent")]
        ),
        LargeEffect::Unclaimed {
            reason: "reference: reference spans absent; candidate: candidate spans absent".into()
        }
    );
}

#[test]
fn an_empty_reference_range_cannot_claim_a_large_effect() {
    assert_eq!(
        large_effect(&[], &[Measure::Observed { value: 10.0 }]),
        LargeEffect::Unclaimed { reason: "reference: statistics need nonempty finite nonnegative measurements".into() }
    );
}
