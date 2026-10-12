//! Cross-agent summary basis (benchmarks.md, section 9).

use std::path::PathBuf;

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
    let summary = crate::summarise(
        &cross_agent_results(),
        "compare",
        "working",
        crate::Design::Interleaved,
        17,
        None,
        &crate::PriceTable::default(),
    )
    .expect("interleaved summary");
    let fresh = summary.comparisons.iter().find(|item| item.metric == "tokens/fresh").expect("compared");
    assert_eq!(fresh.conventions, Some((crate::Convention::Codex, crate::Convention::SmithEvents)));
    assert!(matches!(&fresh.result, crate::Measure::Observed { value } if (value.ratio - 162.0 / 82.0).abs() < 1e-9));
}

pub(super) fn cross_agent_spend_is_priced_from_one_table() {
    let table =
        crate::read_prices(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("agents/prices.toml")).expect("prices");
    let mut results = cross_agent_results();
    for result in &mut results {
        result.spend.usd = crate::Measure::Observed { value: 999.0 };
        for record in &mut result.tokens {
            record.usage.cache_read = crate::Measure::Observed { value: 20 };
            record.usage.output = crate::Measure::Observed { value: 10 };
        }
    }
    let summary =
        crate::summarise(&results, "compare", "working", crate::Design::Single, 17, None, &table).expect("summary");
    assert!(matches!(summary.groups[0].metrics["spend_usd/priced"].median,
        crate::Measure::Observed { value } if (value - 0.000_205).abs() < 1e-12));
    assert!(matches!(summary.groups[1].metrics["spend_usd/priced"].median,
        crate::Measure::Observed { value } if (value - 0.000_305).abs() < 1e-12));
    assert_eq!(results[0].spend.usd, crate::Measure::Observed { value: 999.0 });
}

pub(super) fn a_model_missing_from_the_price_table_leaves_priced_spend_unavailable() {
    let results = cross_agent_results();
    let summary = crate::summarise(
        &results,
        "compare",
        "working",
        crate::Design::Single,
        17,
        None,
        &crate::PriceTable::default(),
    )
    .expect("summary");
    let metric = &summary.groups[0].metrics["spend_usd/priced"];
    assert_eq!(metric.observed, 0);
    assert!(metric.missing.iter().all(|reason| reason.contains("gpt-6-luna") && reason.contains("price table")));
    assert!(matches!(metric.median, crate::Measure::Unavailable { .. }));
}

pub(super) fn teardown_resources_and_tool_calls_are_never_compared_across_agents() {
    let summary = crate::summarise(
        &cross_agent_results(),
        "compare",
        "working",
        crate::Design::Interleaved,
        17,
        None,
        &crate::PriceTable::default(),
    )
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
    let summary = crate::summarise(
        &cross_agent_results(),
        "compare",
        "working",
        crate::Design::Single,
        17,
        None,
        &crate::PriceTable::default(),
    )
    .expect("summary");
    for group in summary.groups {
        assert_eq!(group.metrics["tool_calls/command_execution"].median, crate::Measure::Observed { value: 1.0 });
    }
}
