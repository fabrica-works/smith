//! Parity's offline verdict uses attempt rows and committed source identities.
#[path = "common/parity.rs"]
mod common;

use common::Rows;
use smith_bench::parity::{self, Parity, Verdict};
use smith_bench::{Measure, RunSummary};

fn recompute(summary: &RunSummary) -> Parity {
    parity::from_summary(summary).expect("verdict")
}

fn failing(parity: &Parity) -> Vec<&str> {
    parity.criteria.iter().filter(|criterion| !criterion.holds).map(|criterion| criterion.name.as_str()).collect()
}

#[test]
fn parity_passes_when_every_criterion_is_below_its_margin() {
    let summary = common::parity_summary(Rows::equal());
    let verdict = parity::verify(&summary, &"0".repeat(40)).expect("committed section");
    assert_eq!(verdict.verdict, Verdict::OnPar);
    assert_eq!(verdict.verdict.code(), 0);
    assert_eq!(failing(&verdict), Vec::<&str>::new());
    assert_eq!(
        verdict.criteria.iter().map(|criterion| criterion.name.as_str()).collect::<Vec<_>>(),
        ["O", "FT", "FC", "RT", "RC"]
    );
    assert_eq!(verdict.criteria[0].margin.to_bits(), 31.0_f64.to_bits());
}

#[test]
fn a_smith_spend_missing_only_retried_failures_counts_as_reported() {
    let mut summary = common::parity_summary(Rows::equal());
    let row = summary.attempts.iter_mut().find(|row| row.agent == "smith").expect("smith row");
    row.priced_usd = Measure::LowerBound { value: 1.0, missing: vec!["root: response-3: retried failure".into()] };
    assert_eq!(recompute(&summary).verdict, Verdict::OnPar);
    assert_eq!(
        summary.attempts.iter().find(|row| row.agent == "smith").expect("smith row").priced_usd,
        Measure::LowerBound { value: 1.0, missing: vec!["root: response-3: retried failure".into()] }
    );
}

#[test]
fn a_summary_with_a_retried_failure_row_verifies_against_its_own_section() {
    const SMITH_FIXTURE_WALL_MS: u64 = 1_739_248;
    const CODEX_FIXTURE_WALL_MS: u64 = 1_748_166;
    let mut summary = common::parity_summary(Rows {
        smith_wall_ms: SMITH_FIXTURE_WALL_MS,
        codex_wall_ms: CODEX_FIXTURE_WALL_MS,
        ..Rows::equal()
    });
    let limits = smith_events::Limits { string: 524_288, content: 4096, items: 512 };
    let mut observer =
        smith_bench::agent::smith::events::Observer::with_ceiling(&limits, 16_384).expect("response bounds");
    observer.feed(include_bytes!("recorded/retried-failure-17d63afd-02.jsonl")).expect("response excerpt");
    observer.finish().expect("whole lines");
    let tokens = observer.tokens().expect("recorded tokens");
    let total = tokens.iter().find(|record| record.role == smith_bench::ScopeKind::Total).expect("total");
    let row = summary.attempts.iter_mut().find(|row| row.agent == "smith").expect("smith row");
    row.fresh_tokens = total.usage.fresh.clone();
    row.cache_read_tokens = total.usage.cache_read.clone();
    row.output_tokens = total.usage.output.clone();
    row.priced_usd = Measure::LowerBound {
        value: 1.0,
        missing: vec!["session-0/run-1/conversation-0: session-0/run-1/response-3: retried failure".into()],
    };
    summary.parity = Some(recompute(&summary));
    let file = std::env::temp_dir().join(format!("smith-bench-parity-roundtrip-{}.json", std::process::id()));
    smith_bench::write_summary(&file, &summary).expect("write summary");
    let read_back = smith_bench::read_summary(&file).expect("read summary");
    std::fs::remove_file(file).expect("remove summary");
    assert_eq!(parity::verify(&read_back, &"0".repeat(40)).expect("section agrees").verdict, Verdict::OnPar);
}

#[test]
fn parity_names_the_failing_criterion_and_exits_one() {
    let summary = common::parity_summary(Rows { smith_wall_ms: 130_000, ..Rows::equal() });
    let verdict = recompute(&summary);
    assert_eq!(verdict.verdict, Verdict::NotOnPar);
    assert_eq!(failing(&verdict), ["FT", "RT"]);
    assert_eq!(verdict.verdict.code(), 1);
}

#[test]
fn parity_reference_is_the_codex_arm_whatever_the_arm_names() {
    let mut summary = common::parity_summary(Rows::equal());
    for row in &mut summary.attempts {
        row.arm = if row.agent == "codex" { "last".into() } else { "first".into() };
    }
    for group in &mut summary.groups {
        group.arm = if group.agent == smith_bench::Agent::Codex { "last".into() } else { "first".into() };
    }
    assert_eq!(recompute(&summary).reference_arm, "last");
}

#[test]
fn parity_refuses_a_summary_of_another_smith_commit() {
    let summary = common::parity_summary(Rows::equal());
    assert_eq!(
        parity::verify(&summary, &"1".repeat(40)),
        Err("summary smith arm was built from another commit".into())
    );
}

#[test]
fn an_unavailable_task_wall_makes_the_verdict_incomplete() {
    let mut summary = common::parity_summary(Rows::equal());
    summary.attempts[0].task_wall_ms = Measure::unavailable("answer boundary absent");
    let verdict = recompute(&summary);
    assert_eq!(verdict.verdict, Verdict::Incomplete);
    assert_eq!(verdict.verdict.code(), 2);
    let json = serde_json::to_string(&verdict).expect("incomplete section");
    assert_eq!(serde_json::from_str::<Parity>(&json).expect("round trip"), verdict);
}

#[test]
fn a_setup_refusal_or_harness_error_makes_the_verdict_incomplete() {
    let mut summary = common::parity_summary(Rows::equal());
    for end in ["refused-setup", "harness-error"] {
        summary.attempts[0].end = end.into();
        assert_eq!(recompute(&summary).verdict, Verdict::Incomplete, "{end}");
    }
}

#[test]
fn timeouts_and_budget_ends_count_as_unsolved_with_their_wall() {
    let mut summary = common::parity_summary(Rows::equal());
    for end in ["failed", "timeout", "budget"] {
        for row in &mut summary.attempts {
            if row.agent == "smith" && row.task.starts_with("fixtures/") {
                row.end = end.into();
            }
        }
        let verdict = recompute(&summary);
        assert_eq!(verdict.verdict, Verdict::NotOnPar, "{end}");
        assert_eq!(verdict.criteria[0].statistic.to_bits(), 10.0_f64.to_bits());
        assert_eq!(verdict.criteria[1].statistic.to_bits(), f64::INFINITY.to_bits());
    }
}

#[test]
fn a_ratio_just_below_its_margin_holds_and_just_above_fails() {
    for (wall, usd, failed) in [
        (100_000, 0.0, vec![]),
        (119_000, 1.19, vec![]),
        (121_000, 1.21, vec!["FT", "FC", "RT"]),
        (100_000, 1.49, vec!["FC"]),
        (100_000, 1.51, vec!["FC", "RC"]),
    ] {
        let summary = common::parity_summary(Rows { smith_wall_ms: wall, smith_usd: usd, ..Rows::equal() });
        assert_eq!(failing(&recompute(&summary)), failed, "{wall}, {usd}");
    }
}

#[test]
fn parity_refuses_a_section_that_disagrees_with_attempts() {
    let mut summary = common::parity_summary(Rows::equal());
    summary.parity.as_mut().expect("section").criteria[0].holds = false;
    assert_eq!(
        parity::verify(&summary, &"0".repeat(40)),
        Err("committed parity section disagrees with attempt rows".into())
    );
}

#[test]
fn missing_reference_passes_and_smith_lower_bounds_are_incomplete() {
    for agent in ["smith", "codex"] {
        let mut summary = common::parity_summary(Rows::equal());
        for row in &mut summary.attempts {
            if row.agent == agent {
                row.passed = false;
            }
        }
        assert_eq!(
            recompute(&summary).verdict,
            if agent == "smith" { Verdict::NotOnPar } else { Verdict::Incomplete },
            "{agent}"
        );
    }
    for (agent, expected) in [("smith", Verdict::Incomplete), ("codex", Verdict::OnPar)] {
        let mut summary = common::parity_summary(Rows::equal());
        let row = summary.attempts.iter_mut().find(|row| row.agent == agent).expect("arm");
        row.task_wall_ms = Measure::LowerBound { value: 100_000, missing: vec!["timing boundary".into()] };
        assert_eq!(recompute(&summary).verdict, expected, "{agent}");
    }
}

#[test]
fn the_stratified_bootstrap_keeps_each_tasks_size_in_every_draw() {
    let mut summary = common::parity_summary(Rows::equal());
    for row in &mut summary.attempts {
        if row.task == "fixtures/02-line-decoder" {
            row.task_wall_ms = Measure::Observed { value: 1_000_000_000 };
        }
        if row.task == "fixtures/03-planner" {
            row.task_wall_ms = Measure::Observed { value: 1 };
        }
    }
    let constant = recompute(&summary);
    assert_eq!(constant.criteria[1].statistic.to_bits(), 1.0_f64.to_bits());
    assert_eq!(constant.criteria[1].interval, (1.0, 1.0));
    for row in &mut summary.attempts {
        if row.task == "fixtures/01-backoff" {
            let index = row.attempt.parse::<u32>().expect("id");
            row.task_wall_ms = Measure::Observed { value: if index.is_multiple_of(2) { 110_000 } else { 90_000 } };
        }
    }
    let varied = recompute(&summary);
    assert_eq!(varied.criteria[1].statistic.to_bits(), 1.0_f64.to_bits());
    assert!(varied.criteria[1].interval.0 >= 0.999);
    assert!(varied.criteria[1].interval.1 <= 1.001);
    assert_eq!(recompute(&summary), varied);
}

#[test]
fn o_holds_at_thirty_one_when_codex_passes_thirty_four() {
    const FAILURES: usize = 3;
    let mut summary = common::parity_summary(Rows::equal());
    for row in summary.attempts.iter_mut().filter(|row| row.agent == "smith").take(FAILURES) {
        row.passed = false;
        row.end = "refused".into();
    }
    let verdict = recompute(&summary);
    assert_eq!(verdict.criteria[0].statistic.to_bits(), 31.0_f64.to_bits());
    assert!(verdict.criteria[0].holds);
}

#[test]
fn o_fails_at_thirty_when_codex_passes_thirty_four() {
    const FAILURES: usize = 4;
    let mut summary = common::parity_summary(Rows::equal());
    for row in summary.attempts.iter_mut().filter(|row| row.agent == "smith").take(FAILURES) {
        row.passed = false;
        row.end = "refused".into();
    }
    let verdict = recompute(&summary);
    assert_eq!(verdict.criteria[0].statistic.to_bits(), 30.0_f64.to_bits());
    assert!(!verdict.criteria[0].holds);
}

#[test]
fn a_parity_run_of_five_shell_cwd_attempts_is_incomplete() {
    const ATTEMPTS: usize = 5;
    let mut summary = common::parity_summary(Rows::equal());
    let mut counts = std::collections::BTreeMap::new();
    summary.attempts.retain(|row| {
        let count = counts.entry(row.agent.clone()).or_insert(0);
        if row.task != "repository/shell-cwd" {
            return true;
        }
        *count += 1;
        *count <= ATTEMPTS
    });
    assert_eq!(recompute(&summary).verdict, Verdict::Incomplete);
}
