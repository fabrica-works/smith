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

#[test]
fn a_change_report_takes_its_parent_by_commit_whatever_the_names() {
    const CHANGE_WALL_MS: u64 = 50_000;
    let mut summary = common::change_summary();
    for row in &mut summary.attempts {
        if row.agent_version == common::CHANGE_COMMIT {
            row.task_wall_ms = Measure::Observed { value: CHANGE_WALL_MS };
        }
    }
    for (old, new) in [("a-smith", "parent"), ("z-reference", "change")] {
        for row in &mut summary.attempts {
            if row.arm == old {
                row.arm = new.into();
            }
        }
        for group in &mut summary.groups {
            if group.arm == old {
                group.arm = new.into();
            }
        }
    }
    let report = parity::change::from_summary(&summary, common::PARENT_COMMIT).expect("change report");
    assert_eq!(report.change_commit, common::CHANGE_COMMIT);
    assert_eq!(report.parent_commit, common::PARENT_COMMIT);
    assert_eq!(report.ratios[0].value.statistic.to_bits(), 0.5_f64.to_bits());
}

#[test]
fn a_change_report_gives_each_criterions_ratio_to_its_parent() {
    const CHANGE_WALL_MS: u64 = 50_000;
    const CHANGE_USD: f64 = 0.25;
    let mut summary = common::change_summary();
    for row in &mut summary.attempts {
        if row.agent_version == common::CHANGE_COMMIT {
            row.task_wall_ms = Measure::Observed { value: CHANGE_WALL_MS };
            row.priced_usd = Measure::Observed { value: CHANGE_USD };
        }
    }
    let report = parity::change::from_summary(&summary, common::PARENT_COMMIT).expect("change report");
    for (ratio, (name, value)) in report.ratios.iter().zip([("FT", 0.5_f64), ("FC", 0.25), ("RT", 0.5), ("RC", 0.25)]) {
        assert_eq!(ratio.name, name);
        assert_eq!(ratio.value.statistic.to_bits(), value.to_bits(), "{name}");
        assert_eq!(ratio.value.interval, (value, value), "{name}");
    }
}

#[test]
fn a_change_report_gives_o_as_each_arms_passes() {
    let mut summary = common::change_summary();
    let row = summary
        .attempts
        .iter_mut()
        .find(|row| row.agent_version == common::CHANGE_COMMIT && row.task == "repository/shell-cwd")
        .expect("change row");
    row.passed = false;
    let report = parity::change::from_summary(&summary, common::PARENT_COMMIT).expect("change report");
    assert_eq!(report.outcome.change_passes, 19);
    assert_eq!(report.outcome.parent_passes, 20);
    assert_eq!(report.outcome.attempts, 20);
    assert_eq!(report.outcome.parent_lower.to_bits(), 0.831_566_529_016_914_7_f64.to_bits());
}

#[test]
fn a_change_report_keeps_responses_and_time_beside() {
    let summary = common::change_summary();
    let report = parity::change::from_summary(&summary, common::PARENT_COMMIT).expect("change report");
    assert_eq!(report.beside.len(), 8);
    for beside in &report.beside {
        assert_eq!(beside.responses, Measure::Observed { value: 2.0 });
        assert_eq!(beside.requests_s, Measure::Observed { value: 1.0 });
        assert_eq!(beside.compaction_s, Measure::Observed { value: 2.0 });
        assert_eq!(beside.tools_s, Measure::Observed { value: 7.0 });
        assert_eq!(beside.fresh_tokens, Measure::Observed { value: 100.0 });
    }
}

#[test]
fn a_lower_bound_on_the_changes_side_is_incomplete() {
    const WALL_MS: u64 = 100_000;
    let mut summary = common::change_summary();
    let row = summary.attempts.iter_mut().find(|row| row.agent_version == common::CHANGE_COMMIT).expect("change row");
    row.task_wall_ms = Measure::LowerBound { value: WALL_MS, missing: vec!["answer absent".into()] };
    let report = parity::change::from_summary(&summary, common::PARENT_COMMIT).expect("change report");
    assert_eq!(report.ratios[0].value.statistic.to_bits(), f64::INFINITY.to_bits());
}

#[test]
fn a_lower_bound_on_the_parents_side_is_taken_as_observed() {
    const WALL_MS: u64 = 100_000;
    let mut summary = common::change_summary();
    let row = summary.attempts.iter_mut().find(|row| row.agent_version == common::PARENT_COMMIT).expect("parent row");
    row.task_wall_ms = Measure::LowerBound { value: WALL_MS, missing: vec!["answer absent".into()] };
    let report = parity::change::from_summary(&summary, common::PARENT_COMMIT).expect("change report");
    assert_eq!(report.ratios[0].value.statistic.to_bits(), 1.0_f64.to_bits());
    assert_eq!(report.reference_lower_bounds, ["09: task_wall_ms"]);
}

#[test]
fn the_parity_section_is_unchanged_by_the_arm_arguments() {
    let summary = common::parity_summary(Rows::equal());
    let report = recompute(&summary);
    for (value, (name, statistic, interval, holds)) in report.criteria.iter().zip([
        ("O", 34.0, (0.897_182_075_740_987_4, 1.0), true),
        ("FT", 1.0, (1.0, 1.0), true),
        ("FC", 1.0, (1.0, 1.0), true),
        ("RT", 1.0, (1.0, 1.0), true),
        ("RC", 1.0, (1.0, 1.0), true),
    ]) {
        assert_eq!((&*value.name, value.statistic, value.interval, value.holds), (name, statistic, interval, holds));
    }
}

#[test]
fn a_projection_just_below_the_baselines_upper_bound_goes_on() {
    const CHANGE_WALL_MS: u64 = 109_999;
    let mut summary = common::change_summary();
    for row in &mut summary.attempts {
        if row.agent_version == common::CHANGE_COMMIT {
            row.task_wall_ms = Measure::Observed { value: CHANGE_WALL_MS };
        }
    }
    let report =
        parity::change::project(&summary, common::PARENT_COMMIT, &common::change_baseline(), &[]).expect("projection");
    assert_eq!(report.projection[0].statistic.to_bits(), 1.099_99_f64.to_bits());
    assert_eq!(report.projection[0].baseline_interval, (0.9, 1.1));
    assert_eq!(report.stop, None);
    assert_eq!(report.code(), 0);
}

#[test]
fn a_projection_just_above_the_baselines_upper_bound_stops() {
    const CHANGE_WALL_MS: u64 = 110_001;
    let mut summary = common::change_summary();
    for row in &mut summary.attempts {
        if row.agent_version == common::CHANGE_COMMIT {
            row.task_wall_ms = Measure::Observed { value: CHANGE_WALL_MS };
        }
    }
    let report =
        parity::change::project(&summary, common::PARENT_COMMIT, &common::change_baseline(), &[]).expect("projection");
    assert_eq!(report.projection[0].statistic.to_bits(), 1.100_01_f64.to_bits());
    assert_eq!(report.stop, Some(parity::change::Stop::Moved { criterion: "FT".into() }));
    assert_eq!(report.code(), 1);
}

#[test]
fn the_projection_multiplies_every_landed_changes_ratio() {
    let summary = common::change_summary();
    let mut landed = common::change_summary();
    let mut report = parity::change::from_summary(&landed, common::PARENT_COMMIT).expect("ratios");
    for ratio in &mut report.ratios {
        ratio.value.statistic = 0.5;
    }
    landed.change = Some(report);
    let report =
        parity::change::project(&summary, common::PARENT_COMMIT, &common::change_baseline(), &[landed.clone(), landed])
            .expect("projection");
    for projection in &report.projection {
        assert_eq!(projection.statistic.to_bits(), 0.25_f64.to_bits(), "{}", projection.name);
    }
    assert_eq!(report.stop, None);
}

#[test]
fn o_just_within_the_parents_bound_goes_on() {
    const FAILURES: usize = 3;
    let mut summary = common::change_summary();
    for row in summary
        .attempts
        .iter_mut()
        .filter(|row| row.agent_version == common::CHANGE_COMMIT && row.task == "repository/shell-cwd")
        .take(FAILURES)
    {
        row.passed = false;
    }
    let report =
        parity::change::project(&summary, common::PARENT_COMMIT, &common::change_baseline(), &[]).expect("projection");
    assert_eq!(report.outcome.change_passes, 17);
    assert_eq!(report.stop, None);
}

#[test]
fn o_just_past_the_parents_bound_stops() {
    const FAILURES: usize = 4;
    let mut summary = common::change_summary();
    for row in summary
        .attempts
        .iter_mut()
        .filter(|row| row.agent_version == common::CHANGE_COMMIT && row.task == "repository/shell-cwd")
        .take(FAILURES)
    {
        row.passed = false;
    }
    let report =
        parity::change::project(&summary, common::PARENT_COMMIT, &common::change_baseline(), &[]).expect("projection");
    assert_eq!(report.outcome.change_passes, 16);
    assert_eq!(report.stop, Some(parity::change::Stop::Moved { criterion: "O".into() }));
}

#[test]
fn an_arm_without_a_pass_on_a_task_is_incomplete() {
    let mut summary = common::change_summary();
    for row in summary
        .attempts
        .iter_mut()
        .filter(|row| row.agent_version == common::CHANGE_COMMIT && row.task == "repository/shell-cwd")
    {
        row.passed = false;
    }
    let report =
        parity::change::project(&summary, common::PARENT_COMMIT, &common::change_baseline(), &[]).expect("projection");
    assert_eq!(
        report.stop,
        Some(parity::change::Stop::Incomplete { why: "criterion unavailable or an arm has no pass on a task".into() })
    );
    assert_eq!(report.code(), 2);
}

#[test]
fn a_landed_input_that_is_not_a_change_summary_is_refused() {
    let summary = common::change_summary();
    assert_eq!(
        parity::change::project(
            &summary,
            common::PARENT_COMMIT,
            &common::change_baseline(),
            &[common::change_baseline()]
        ),
        Err("landed input must be a parity-change summary with a change section".into())
    );
}

#[test]
fn an_incomplete_report_stops_before_a_moved_criterion() {
    const CHANGE_WALL_MS: u64 = 200_000;
    let mut summary = common::change_summary();
    for row in &mut summary.attempts {
        if row.agent_version == common::CHANGE_COMMIT {
            row.task_wall_ms = Measure::Observed { value: CHANGE_WALL_MS };
        }
    }
    summary.attempts.last_mut().expect("parent row").priced_usd = Measure::unavailable("spend absent");
    let report =
        parity::change::project(&summary, common::PARENT_COMMIT, &common::change_baseline(), &[]).expect("projection");
    assert_eq!(report.projection[0].statistic.to_bits(), 2.0_f64.to_bits());
    assert_eq!(
        report.stop,
        Some(parity::change::Stop::Incomplete { why: "criterion unavailable or an arm has no pass on a task".into() })
    );
}

#[test]
fn the_change_command_refuses_a_section_that_disagrees() {
    let (mut summary, directory) = common::named_change("disagrees");
    summary.change.as_mut().expect("section").ratios[0].value.statistic = 2.0;
    let file = directory.join("change.json");
    smith_bench::write_summary(&file, &summary).expect("changed section");
    let code = parity::change::command(&file);
    assert_eq!(code, 3);
    assert_eq!(parity::change::verify(&summary), Err("committed change section disagrees with rows or inputs".into()));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn the_change_command_agrees_with_a_stopped_section() {
    const CHANGE_WALL_MS: u64 = 200_000;
    let (mut summary, directory) = common::named_change("stopped");
    for row in &mut summary.attempts {
        if row.agent_version == common::CHANGE_COMMIT {
            row.task_wall_ms = Measure::Observed { value: CHANGE_WALL_MS };
        }
    }
    let baseline = directory.join("baseline.json");
    parity::change::attach(&mut summary, &parity::change::Inputs { baseline_run: Some(baseline), landed: Vec::new() })
        .expect("updated section");
    let file = directory.join("change.json");
    smith_bench::write_summary(&file, &summary).expect("stopped section");
    let code = parity::change::command(&file);
    assert_eq!(code, 0);
    assert_eq!(summary.change.expect("section").stop, Some(parity::change::Stop::Moved { criterion: "FT".into() }));
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn a_change_command_refuses_an_input_whose_hash_differs() {
    let (summary, directory) = common::named_change("digest");
    let file = directory.join("change.json");
    smith_bench::write_summary(&file, &summary).expect("stored section");
    std::fs::write(directory.join("baseline.json"), b"changed input").expect("changed baseline");
    let code = parity::change::command(&file);
    assert_eq!(code, 3);
    std::fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn a_change_command_refuses_a_missing_input() {
    let (summary, directory) = common::named_change("missing");
    let file = directory.join("change.json");
    smith_bench::write_summary(&file, &summary).expect("stored section");
    std::fs::remove_file(directory.join("baseline.json")).expect("remove baseline");
    let code = parity::change::command(&file);
    assert_eq!(code, 3);
    std::fs::remove_dir_all(directory).expect("cleanup");
}
