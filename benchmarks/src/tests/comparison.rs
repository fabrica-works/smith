//! Cross-agent summary basis (benchmarks.md, section 9).

use super::recorded_result;

fn response(result: &crate::AttemptResult) -> crate::ResponseTokens {
    let usage = &result.tokens.iter().find(|record| record.role == crate::ScopeKind::Total).expect("total").usage;
    crate::ResponseTokens {
        scope: "response-1".into(),
        model: match &result.identity.model.resolved_model {
            crate::Measure::Observed { value } => value.clone(),
            crate::Measure::LowerBound { .. } | crate::Measure::Unavailable { .. } => String::new(),
        },
        fresh: crate::tokens::reported(&usage.fresh).unwrap_or(0),
        cache_read: crate::tokens::reported(&usage.cache_read).unwrap_or(0),
        cache_write: crate::tokens::reported(&usage.cache_write).unwrap_or(0),
        output: crate::tokens::reported(&usage.output).unwrap_or(0),
    }
}

fn pricing_result() -> crate::AttemptResult {
    let mut result = recorded_result();
    for record in &mut result.tokens {
        record.usage.cache_write = crate::Measure::Observed { value: 0 };
    }
    result
}

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
    let summary =
        crate::summarise(&cross_agent_results(), "compare", "working", crate::Design::Interleaved, 17, None, None)
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
        result.spend.priced = super::super::result::priced_spend(result, &[response(result)]);
    }
    let summary =
        crate::summarise(&results, "compare", "working", crate::Design::Single, 17, None, None).expect("summary");
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
        for record in &mut result.tokens {
            record.usage.cache_read = crate::Measure::Observed { value: 20 };
            record.usage.cache_write = crate::Measure::Observed { value: 0 };
            record.usage.output = crate::Measure::Observed { value: 10 };
        }
        result.spend.priced = super::super::result::priced_spend(result, &[response(result)]);
    }
    let summary =
        crate::summarise(&results, "compare", "working", crate::Design::Single, 17, None, None).expect("summary");
    let metric = &summary.groups[0].metrics["spend_usd/priced"];
    assert_eq!(metric.observed, 0);
    assert!(
        metric.missing.iter().all(|reason| reason.contains("unlisted-model") && reason.contains("product presets"))
    );
    assert!(matches!(metric.median, crate::Measure::Unavailable { .. }));
}

pub(super) fn teardown_resources_and_tool_calls_are_never_compared_across_agents() {
    let summary =
        crate::summarise(&cross_agent_results(), "compare", "working", crate::Design::Interleaved, 17, None, None)
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
    let summary = crate::summarise(&cross_agent_results(), "compare", "working", crate::Design::Single, 17, None, None)
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
    let mut record = response(&result);
    record.fresh = 100_000;
    record.cache_read = 100_000;
    record.cache_write = 10_000;
    record.output = 100_000;
    result.spend.priced = super::super::result::priced_spend(&result, &vec![record; 10]);
    result
}

#[test]
fn the_summary_prices_tokens_from_the_product_presets() {
    let result = preset_priced_result();
    let summary =
        crate::summarise(&[result], "sample", "working", crate::Design::Single, 17, None, None).expect("summary");
    assert_eq!(summary.groups[0].metrics["spend_usd/priced"].median, crate::Measure::Observed { value: 61.25 });
}

#[test]
fn the_priced_basis_names_its_catalog_commit() {
    let result = preset_priced_result();
    let commit = result.identity.harness_commit.clone();
    let summary =
        crate::summarise(&[result], "sample", "working", crate::Design::Single, 17, None, None).expect("summary");
    assert_eq!(summary.priced_at, commit);
    assert_eq!(summary.version, 4);
}

#[test]
fn an_agents_own_figure_is_kept_beside_the_priced_one() {
    let result = preset_priced_result();
    assert_eq!(result.spend.usd, crate::Measure::Observed { value: 999.0 });
    assert_eq!(result.spend.priced, crate::Measure::Observed { value: 61.25 });
    let summary =
        crate::summarise(&[result], "sample", "working", crate::Design::Single, 17, None, None).expect("summary");
    assert_eq!(summary.groups[0].metrics["spend_usd/notional"].median, crate::Measure::Observed { value: 999.0 });
}

#[test]
fn a_response_without_usage_leaves_the_spend_unavailable() {
    let mut result = preset_priced_result();
    let total = result.tokens.iter_mut().find(|record| record.role == crate::ScopeKind::Total).expect("total");
    total.usage.output = crate::Measure::unavailable("response-2: output_tokens is null");
    assert_eq!(
        super::super::result::priced_spend(&result, &[response(&result)]),
        crate::Measure::unavailable("total output tokens: response-2: output_tokens is null")
    );
}

#[test]
fn each_response_is_priced_at_its_own_prompts_tier() {
    let result = pricing_result();
    let responses = [100_000, 300_000].map(|fresh| crate::ResponseTokens {
        scope: format!("response-{fresh}"),
        model: "gpt-6.1-sol".into(),
        fresh,
        cache_read: 0,
        cache_write: 0,
        output: 1000,
    });
    assert_eq!(super::super::result::priced_spend(&result, &responses), crate::Measure::Observed { value: 1.425 });
}

#[test]
fn a_declared_window_past_the_tier_prices_short_prompts_at_base_rates() {
    let mut result = pricing_result();
    result.identity.model.resolved_window = crate::Measure::Observed { value: 475_000 };
    let response = crate::ResponseTokens {
        scope: "response-1".into(),
        model: "gpt-6.1-sol".into(),
        fresh: 100_000,
        cache_read: 0,
        cache_write: 0,
        output: 1000,
    };
    assert_eq!(super::super::result::priced_spend(&result, &[response]), crate::Measure::Observed { value: 0.21 });
}

#[test]
fn a_child_on_another_model_is_priced_at_its_own() {
    let result = pricing_result();
    let responses = ["gpt-6.1-sol", "gpt-6-luna"].map(|model| crate::ResponseTokens {
        scope: model.into(),
        model: model.into(),
        fresh: 100_000,
        cache_read: 0,
        cache_write: 0,
        output: 1000,
    });
    assert_eq!(super::super::result::priced_spend(&result, &responses), crate::Measure::Observed { value: 0.2205 });
}

fn unsent_result(retried: bool) -> crate::AttemptResult {
    let mut result = pricing_result();
    let limits = smith_events::Limits { string: 524_288, content: 4096, items: 512 };
    let mut observer = crate::agent::smith::events::Observer::with_ceiling(&limits, 16_384).expect("response bounds");
    let records = include_str!("../../tests/recorded/retried-failure-17d63afd-02.jsonl")
        .replace("\"evidence\":\"response\"", "\"evidence\":\"unsent\"");
    let mut records: Vec<serde_json::Value> =
        records.lines().map(|line| serde_json::from_str(line).expect("event")).collect();
    if !retried {
        for record in &mut records {
            if record["type"] == "response.started" && record["attempt"].as_u64().is_some_and(|attempt| attempt > 0) {
                record["turn"] = serde_json::json!(999);
            }
        }
    }
    for record in records {
        observer.feed(format!("{record}\n").as_bytes()).expect("recorded event");
    }
    observer.finish().expect("whole lines");
    result.tokens = observer.tokens().expect("tokens");
    result.spend.priced = super::super::result::priced_spend(&result, &observer.response_tokens().expect("responses"));
    result
}

#[test]
fn a_request_refused_unsent_is_priced_at_nothing() {
    assert_eq!(unsent_result(false).spend.priced, crate::Measure::Observed { value: 0.036_580_4 });
}

#[test]
fn a_retried_unsent_failure_leaves_no_gap() {
    assert_eq!(unsent_result(true).spend.priced, crate::Measure::Observed { value: 0.036_580_4 });
}

#[test]
fn a_summary_keeps_each_attempts_row() {
    let mut results = vec![recorded_result(), recorded_result()];
    for (index, result) in results.iter_mut().enumerate() {
        result.identity.attempt = ["02", "01"][index].into();
        result.outcome.grade = Some(crate::Measure::Observed { value: true });
        result.spend.priced = crate::Measure::Observed { value: 0.25 };
    }
    let summary = crate::summarise(&results, "sample", "small", crate::Design::Single, 17, None, None).expect("rows");
    let expected: Vec<_> = ["02", "01"]
        .map(|attempt| crate::AttemptRow {
            attempt: attempt.into(),
            task: "probes/recorded".into(),
            arm: "codex-standard".into(),
            agent: "codex".into(),
            agent_version: recorded_result().identity.agent_version,
            time_record: recorded_result().timing.record,
            end: "completed".into(),
            passed: true,
            task_wall_ms: crate::Measure::Observed { value: 30 },
            priced_usd: crate::Measure::Observed { value: 0.25 },
            responses: crate::Measure::Observed { value: 1 },
            peak_rss_bytes: crate::Measure::Observed { value: 4096 },
            peak_rss_scope: crate::PeakScope::Tree,
            fresh_tokens: crate::Measure::Observed { value: 80 },
            cache_read_tokens: crate::Measure::Observed { value: 20 },
            output_tokens: crate::Measure::Observed { value: 10 },
        })
        .into();
    assert_eq!(summary.attempts, expected);
}

#[test]
fn the_reference_arm_is_the_codex_arm_whatever_the_names() {
    for (codex, smith) in [("a-reference", "z-candidate"), ("z-reference", "a-candidate")] {
        let mut results: Vec<_> = cross_agent_results().into_iter().step_by(5).collect();
        for result in &mut results {
            result.identity.arm = if result.identity.agent == crate::Agent::Codex { codex } else { smith }.into();
        }
        let summary = crate::summarise(&results, "sample", "small", crate::Design::Interleaved, 17, None, None)
            .expect("Codex reference");
        assert!(!summary.comparisons.is_empty());
        assert!(summary.comparisons.iter().all(|row| row.reference == codex && row.candidate == smith));
    }
}

#[test]
fn a_suite_without_codex_compares_against_its_named_reference() {
    let mut suite = super::sample_suite();
    suite.design = crate::Design::Interleaved;
    suite.agents[0].agent = crate::Agent::Smith;
    suite.agents[0].config = "everything".into();
    suite.arms = ["smith", "codex"]
        .map(|name| crate::Arm::Override { name: name.into(), smith: std::collections::BTreeMap::new() })
        .into();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let models = crate::read_model_tiers(&root.join("agents/models.toml")).expect("models");
    let tasks = super::sample_catalogue();
    let missing =
        crate::validate_suite(std::path::Path::new("suite.toml"), &suite, &tasks, &root.join("agents"), &models)
            .expect_err("missing named reference");
    assert_eq!(missing.key, "reference");
    suite.reference = Some("smith".into());
    crate::validate_suite(std::path::Path::new("suite.toml"), &suite, &tasks, &root.join("agents"), &models)
        .expect("reference");
    let mut results: Vec<_> = cross_agent_results().into_iter().step_by(5).collect();
    for result in &mut results {
        result.identity.agent = crate::Agent::Smith;
    }
    let summary =
        crate::summarise(&results, "sample", "small", crate::Design::Interleaved, 17, None, suite.reference.as_deref())
            .expect("named reference");
    assert!(!summary.comparisons.is_empty());
    assert!(summary.comparisons.iter().all(|row| row.reference == "smith" && row.candidate == "codex"));
}

#[test]
fn a_summary_of_another_version_is_refused_by_name() {
    let summary = crate::summarise(&[recorded_result()], "sample", "small", crate::Design::Single, 17, None, None)
        .expect("summary");
    let mut document = serde_json::to_value(summary).expect("document");
    document["version"] = 2.into();
    document.as_object_mut().expect("object").remove("attempts");
    let file = std::env::temp_dir().join(format!("smith-summary-version-{}.json", std::process::id()));
    std::fs::write(&file, document.to_string()).expect("fixture");
    let refusal = crate::read_summary(&file).expect_err("version two");
    std::fs::remove_file(&file).expect("cleanup");
    assert_eq!(refusal.file, file);
    assert_eq!(refusal.key, "version");
    assert!(refusal.reason.contains("summary version 2"));
}
