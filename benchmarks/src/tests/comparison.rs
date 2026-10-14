//! Cross-agent summary basis (benchmarks.md, section 9).

use super::recorded_result;

fn cross_agent_results() -> Vec<crate::AttemptResult> {
    let mut results = Vec::new();
    for (arm, agent, convention, fresh) in [
        ("codex", crate::Agent::Codex, crate::Convention::Codex, 80),
        ("smith", crate::Agent::Smith, crate::Convention::SmithEvents, 160),
    ] {
        for index in 0..5 {
            let mut result = recorded_result();
            result.identity.attempt = format!("{arm}-{index}");
            result.identity.arm = arm.into();
            result.identity.agent = agent;
            result.timing.process_wall_ms = crate::Measure::unavailable("not needed by token test");
            result.timing.task_wall_ms = crate::Measure::unavailable("not needed by token test");
            for record in &mut result.tokens {
                record.convention = convention;
                record.usage.fresh = crate::Measure::Observed { value: fresh + index };
                record.usage.cache_read = crate::Measure::unavailable("not needed by fresh test");
                record.usage.output = crate::Measure::unavailable("not needed by fresh test");
            }
            results.push(result);
        }
    }
    results
}

pub(super) fn cross_agent_token_fields_pair_across_conventions_and_name_both() {
    let summary = crate::summarise(&cross_agent_results(), "compare", "working", crate::Design::Interleaved, 17, None)
        .expect("interleaved summary");
    let fresh = summary.comparisons.iter().find(|item| item.metric == "tokens/fresh").expect("compared");
    assert_eq!(fresh.conventions, Some((crate::Convention::Codex, crate::Convention::SmithEvents)));
    assert!(matches!(&fresh.result, crate::Measure::Observed { value } if (value.ratio - 162.0 / 82.0).abs() < 1e-9));
}

pub(super) fn cross_agent_spend_is_priced_from_one_table() {
    let mut results = cross_agent_results();
    for result in &mut results {
        result.spend.usd = crate::Measure::Observed { value: 999.0 };
        result.identity.model.resolved_window = crate::Measure::Observed { value: 272_000 };
        result.identity.model.resolved_model = crate::Measure::Observed { value: "gpt-6-luna".into() };
        for record in &mut result.tokens {
            record.usage.cache_read = crate::Measure::Observed { value: 20 };
            record.usage.cache_write = crate::Measure::Observed { value: 0 };
            record.usage.output = crate::Measure::Observed { value: 10 };
        }
        result.spend.priced = super::super::result::priced_spend(result);
    }
    let summary = crate::summarise(&results, "compare", "working", crate::Design::Single, 17, None).expect("summary");
    assert!(matches!(summary.groups[0].metrics["spend_usd/priced"].median,
        crate::Measure::Observed { value } if (value - 0.000_013_4).abs() < 1e-12));
    assert!(matches!(summary.groups[1].metrics["spend_usd/priced"].median,
        crate::Measure::Observed { value } if (value - 0.000_021_4).abs() < 1e-12));
    assert_eq!(results[0].spend.usd, crate::Measure::Observed { value: 999.0 });
}

pub(super) fn a_model_missing_from_the_price_table_leaves_priced_spend_unavailable() {
    let mut results = cross_agent_results();
    for result in &mut results {
        result.identity.model.resolved_window = crate::Measure::Observed { value: 272_000 };
        result.identity.model.resolved_model = crate::Measure::Observed { value: "unlisted-model".into() };
        result.spend.priced = super::super::result::priced_spend(result);
    }
    let summary = crate::summarise(&results, "compare", "working", crate::Design::Single, 17, None).expect("summary");
    let metric = &summary.groups[0].metrics["spend_usd/priced"];
    assert_eq!(metric.observed, 0);
    assert!(
        metric.missing.iter().all(|reason| reason.contains("unlisted-model") && reason.contains("product presets"))
    );
    assert!(matches!(metric.median, crate::Measure::Unavailable { .. }));
}

pub(super) fn teardown_resources_and_tool_calls_are_never_compared_across_agents() {
    let summary = crate::summarise(&cross_agent_results(), "compare", "working", crate::Design::Interleaved, 17, None)
        .expect("summary");
    assert!(
        summary
            .comparisons
            .iter()
            .all(|item| matches!(item.metric.as_str(), "task_wall_ms" | "process_wall_ms" | "spend_usd/priced")
                || item.metric.starts_with("tokens/"))
    );
}

pub(super) fn tool_calls_by_name_are_summarised_per_arm() {
    let summary = crate::summarise(&cross_agent_results(), "compare", "working", crate::Design::Single, 17, None)
        .expect("summary");
    for group in summary.groups {
        assert_eq!(group.metrics["tool_calls/command_execution"].median, crate::Measure::Observed { value: 1.0 });
    }
}

fn preset_priced_result() -> crate::AttemptResult {
    let mut result = recorded_result();
    result.identity.model.resolved_window = crate::Measure::Observed { value: 272_000 };
    result.identity.model.resolved_model = crate::Measure::Observed { value: "gpt-6-astra".into() };
    result.spend.basis = crate::SpendBasis::Notional;
    result.spend.usd = crate::Measure::Observed { value: 999.0 };
    for record in &mut result.tokens {
        record.usage.fresh = crate::Measure::Observed { value: 1_000_000 };
        record.usage.cache_read = crate::Measure::Observed { value: 1_000_000 };
        record.usage.cache_write = crate::Measure::Observed { value: 100_000 };
        record.usage.output = crate::Measure::Observed { value: 1_000_000 };
    }
    result.spend.priced = super::super::result::priced_spend(&result);
    result
}

#[test]
fn the_summary_prices_tokens_from_the_product_presets() {
    let result = preset_priced_result();
    let summary = crate::summarise(&[result], "sample", "working", crate::Design::Single, 17, None).expect("summary");
    assert_eq!(summary.groups[0].metrics["spend_usd/priced"].median, crate::Measure::Observed { value: 61.25 });
}

#[test]
fn the_priced_basis_names_its_catalog_commit() {
    let result = preset_priced_result();
    let commit = result.identity.harness_commit.clone();
    let summary = crate::summarise(&[result], "sample", "working", crate::Design::Single, 17, None).expect("summary");
    assert_eq!(summary.priced_at, commit);
    assert_eq!(summary.version, 3);
}

#[test]
fn an_agents_own_figure_is_kept_beside_the_priced_one() {
    let result = preset_priced_result();
    assert_eq!(result.spend.usd, crate::Measure::Observed { value: 999.0 });
    assert_eq!(result.spend.priced, crate::Measure::Observed { value: 61.25 });
    let summary = crate::summarise(&[result], "sample", "working", crate::Design::Single, 17, None).expect("summary");
    assert_eq!(summary.groups[0].metrics["spend_usd/notional"].median, crate::Measure::Observed { value: 999.0 });
}

#[test]
fn an_unavailable_window_leaves_the_priced_spend_unavailable() {
    let mut result = preset_priced_result();
    result.identity.model.resolved_window = crate::Measure::unavailable("stream omits context window");
    assert_eq!(
        super::super::result::priced_spend(&result),
        crate::Measure::unavailable("declared model window not observed for pricing")
    );
}
