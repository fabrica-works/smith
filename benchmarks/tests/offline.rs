//! Recorded task acceptance and each independent manifest refusal.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use smith_bench::{check_task, check_tree};

fn recorded(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/tasks").join(name).join("task.toml")
}

fn design() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/design")
}

#[test]
fn fixtures_freeze_their_seeds_and_keep_graders_outside_them() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tasks/fixtures");
    for name in ["01-backoff", "02-line-decoder"] {
        let directory = root.join(name);
        let task = check_task(&directory.join("task.toml"), &design()).expect("frozen fixture");
        assert_eq!(task.kind, smith_bench::Kind::Fixture);
        assert_eq!(task.tier, "working");
        assert_eq!(task.setup, [vec!["cargo", "build", "--offline", "--tests"]]);
        assert_eq!(task.protected, ["tests", "Cargo.toml", "Cargo.lock", "AGENTS.md"].map(PathBuf::from));
        assert!(directory.join("grader/tests/hidden.rs").is_file());
        assert!(directory.join("grader/tests/regression.rs").is_file());
        assert!(directory.join("reference/src/lib.rs").is_file());
        assert!(!directory.join("seed/tests/hidden.rs").exists());
        assert!(!directory.join("seed/grader").exists());
        assert!(!directory.join("seed/reference").exists());
    }
    let hidden = std::fs::read_to_string(root.join("02-line-decoder/grader/tests/hidden.rs")).expect("hidden test");
    assert!(hidden.contains("1u32 << 18"));
    assert!(!hidden.contains("i < 16"));
}

#[test]
fn the_compare_suite_names_both_fixtures_and_both_agents() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite = smith_bench::read_suite(&root.join("suites/compare.toml")).expect("comparison");
    assert_eq!(suite.tasks, ["fixtures/01-backoff", "fixtures/02-line-decoder"]);
    assert_eq!(suite.tier, "working");
    assert_eq!(suite.design, smith_bench::Design::Interleaved);
    assert_eq!(suite.repetitions, 5);
    assert_eq!(
        suite.agents.iter().map(|agent| agent.agent).collect::<Vec<_>>(),
        [smith_bench::Agent::Smith, smith_bench::Agent::Codex]
    );
    assert_eq!(suite.agents.iter().map(|agent| agent.config.as_str()).collect::<Vec<_>>(), ["compare", "standard"]);
    let pin = smith_bench::read_configuration(&root.join("agents/smith/compare.pin.toml")).expect("comparison pin");
    assert_eq!(pin.pin.configuration, PathBuf::from("first-request.json"));
}

fn refuses(name: &str, key: &str, reason: &str) {
    let error = check_task(&recorded(name), &design()).expect_err("recorded task must refuse");
    assert!(error.key.contains(key), "{error}");
    assert!(error.reason.contains(reason), "{error}");
}

fn every_catalogue_manifest_is_checked_offline() {
    let tasks = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tasks");
    check_tree(&tasks, &design()).expect("all committed manifests validate");
    let planner = tasks.join("fixtures/03-planner");
    for path in ["grader/tests/hidden.rs", "grader/tests/regression.rs", "reference/src/planner.rs"] {
        assert!(planner.join(path).is_file());
    }
    assert!(!planner.join("seed/tests/hidden.rs").exists());
}

fn every_committed_suite_and_configuration_pin_is_checked_offline() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let counts = smith_bench::check_benchmark_tree_with_bundles(&root, &design(), &root.join("tests/recorded/bundles"))
        .expect("all benchmark manifests validate");
    assert!(counts.configurations > 0, "committed pins are actually checked");
}

fn the_recorded_suite_resolves_its_frozen_task_and_pin() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite_file = root.join("tests/recorded/suites/sample.toml");
    let suite = smith_bench::read_suite(&suite_file).expect("recorded suite");
    let tasks =
        smith_bench::catalogue(&root.join("tests/recorded/tasks/valid"), &design()).expect("recorded catalogue");
    let models = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("committed models");
    assert_eq!(
        smith_bench::validate_suite(&suite_file, &suite, &tasks, &root.join("agents"), &models)
            .expect("references resolve"),
        ["probes/recorded"]
    );
}

fn a_complete_recorded_task_is_accepted() {
    check_task(&recorded("valid"), &design()).expect("valid frozen task");
}

fn a_prompt_without_its_final_newline_is_refused() {
    refuses("missing-newline", "prompt", "end in a newline");
}

fn a_prompt_with_two_paragraphs_is_refused() {
    refuses("multiple-paragraphs", "prompt", "one nonempty paragraph");
}

fn a_prompt_at_the_terminal_line_bound_is_refused() {
    refuses("oversize-prompt", "prompt", "4,088 bytes");
}

fn changed_seed_bytes_are_refused() {
    refuses("wrong-digest", "seed_sha256", "seed digest differs");
}

fn a_missing_design_section_is_refused() {
    refuses("missing-guard", "guards[0]", "does not exist");
}

fn a_benchmark_tree_in_a_seed_is_refused() {
    refuses("benchmark-seed", "seed", "stay outside");
}

fn a_grader_file_in_a_seed_is_refused() {
    refuses("grader-seed", "seed", "stay outside");
}

fn a_reference_file_in_a_seed_is_refused() {
    refuses("reference-seed", "seed", "stay outside");
}

fn an_unknown_outcome_check_is_refused() {
    refuses("unknown-check", "outcome", "unknown variant");
}

fn an_unknown_nested_key_is_refused() {
    refuses("unknown-key", "budget.turnz", "unknown field");
}

fn a_check_cannot_escape_the_workspace() {
    refuses("outside-path", "outcome[0].path", "without traversal");
}

fn duplicate_variant_names_are_refused() {
    refuses("duplicate-variant", "variant[1].name", "unique");
}

fn a_recorded_attempt_round_trips_without_erasing_measurement_gaps() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json");
    let result = smith_bench::read_result(&file).expect("recorded result");
    let rendered = smith_bench::render_result(&result).expect("versioned JSON");
    assert_eq!(smith_bench::parse_result(&file, &rendered).expect("round-trip"), result);
    assert!(matches!(result.spend.usd, smith_bench::Measure::Unavailable { .. }));
    assert!(matches!(result.resources.cpu_ms, smith_bench::Measure::LowerBound { .. }));
    assert_eq!(result.outcome.end, smith_bench::End::Completed);
    assert_eq!(result.outcome.forced, smith_bench::Forced::Killed);
}

fn a_summary_and_baseline_from_recorded_results_round_trip_offline() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json");
    let results = smith_bench::read_results(&file).expect("recorded attempts");
    let summary = smith_bench::summarise(
        &results,
        "recorded",
        "small",
        smith_bench::Design::Single,
        123,
        None,
        &smith_bench::PriceTable::default(),
    )
    .expect("summary");
    assert_eq!(summary.groups[0].counts_by_end["completed"], 1);
    assert_eq!(summary.groups[0].metrics["task_wall_ms"].observed, 1);
    let baseline = smith_bench::baseline(&summary).expect("single configuration baseline");
    let root = std::env::temp_dir().join(format!("smith-bench-summary-{}", std::process::id()));
    std::fs::create_dir(&root).expect("fresh test root");
    let summary_file = root.join("summary.json");
    let baseline_file = root.join("baseline.json");
    smith_bench::write_summary(&summary_file, &summary).expect("summary output");
    smith_bench::write_summary(&baseline_file, &baseline).expect("baseline output");
    assert_eq!(smith_bench::read_summary(&summary_file).expect("round-trip"), summary);
    assert_eq!(smith_bench::read_baseline(&baseline_file).expect("round-trip"), baseline);
    assert!(smith_bench::write_summary(&summary_file, &summary).is_err(), "committed evidence is not overwritten");
    std::fs::remove_dir_all(root).expect("test root removed");
}

// Related offline assertions share a test process to keep startup overhead
// inside the one-second harness budget; each named story keeps its assertions.

#[test]
fn recorded_catalogues_and_results() {
    an_absent_bundle_is_skipped_and_said();
    a_bundle_whose_digest_differs_is_refused();
    a_present_bundle_matches_its_manifest();
    a_repository_seed_comes_from_its_bundle();
    every_catalogue_manifest_is_checked_offline();
    every_committed_suite_and_configuration_pin_is_checked_offline();
    the_recorded_suite_resolves_its_frozen_task_and_pin();
    a_complete_recorded_task_is_accepted();
    a_recorded_attempt_round_trips_without_erasing_measurement_gaps();
    a_summary_and_baseline_from_recorded_results_round_trip_offline();
}

fn an_absent_bundle_is_skipped_and_said() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded");
    let manifest = smith_bench::read_bundle_manifest(&root.join("tasks/repository-seed/seed/bundle.toml"))
        .expect("recorded manifest");
    assert_eq!(smith_bench::check_bundle(&manifest, &root.join("bundles/none")), Ok(smith_bench::BundleCheck::Absent));
}

fn a_bundle_whose_digest_differs_is_refused() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded");
    let mut manifest = smith_bench::read_bundle_manifest(&root.join("tasks/repository-seed/seed/bundle.toml"))
        .expect("recorded manifest");
    manifest.sha256 = "0".repeat(64);
    assert!(
        smith_bench::check_bundle(&manifest, &root.join("bundles"))
            .expect_err("digest differs")
            .reason
            .contains("digest")
    );
}

fn a_present_bundle_matches_its_manifest() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded");
    let manifest = smith_bench::read_bundle_manifest(&root.join("tasks/repository-seed/seed/bundle.toml"))
        .expect("recorded manifest");
    assert_eq!(smith_bench::check_bundle(&manifest, &root.join("bundles")), Ok(smith_bench::BundleCheck::Matches));
}

fn a_repository_seed_comes_from_its_bundle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded");
    let task_directory = root.join("tasks/repository-seed");
    let task = check_task(&task_directory.join("task.toml"), &design()).expect("recorded task");
    let (source, digest) =
        smith_bench::seed_source(&task, &task_directory, &root.join("bundles")).expect("present repository seed");
    assert_eq!(source, root.join("bundles/tiny"));
    assert_eq!(digest, smith_bench::seed_digest(&source).expect("recorded bytes"));
    assert_ne!(digest, task.seed_sha256);
    assert!(
        smith_bench::seed_source(&task, &task_directory, &root.join("bundles/none"))
            .expect_err("absent bundles refuse attempts")
            .reason
            .contains("absent")
    );
}

#[test]
fn invalid_recorded_manifests() {
    a_prompt_without_its_final_newline_is_refused();
    a_prompt_with_two_paragraphs_is_refused();
    a_prompt_at_the_terminal_line_bound_is_refused();
    changed_seed_bytes_are_refused();
    a_missing_design_section_is_refused();
    a_benchmark_tree_in_a_seed_is_refused();
    a_grader_file_in_a_seed_is_refused();
    a_reference_file_in_a_seed_is_refused();
    an_unknown_outcome_check_is_refused();
    an_unknown_nested_key_is_refused();
    a_check_cannot_escape_the_workspace();
    duplicate_variant_names_are_refused();
}

#[test]
fn committed_summaries_and_baselines_parse_at_result_version_two() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(smith_bench::RESULT_VERSION, 2);
    // The catalogue test checks the full inventory; sample each versioned reader here.
    for entry in std::fs::read_dir(root.join("summaries")).expect("summaries") {
        let path = entry.expect("summary entry").path();
        if path.extension().is_some_and(|extension| extension == "json") {
            assert_eq!(smith_bench::read_summary(&path).expect("committed summary").result_version, 2);
            break;
        }
    }
    for entry in std::fs::read_dir(root.join("baselines")).expect("baselines") {
        let path = entry.expect("baseline entry").path();
        if path.extension().is_some_and(|extension| extension == "json") {
            assert_eq!(smith_bench::read_baseline(&path).expect("committed baseline").result_version, 2);
            break;
        }
    }
}

fn multi_configuration_summary() -> smith_bench::RunSummary {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json");
    let results = smith_bench::read_results(&file).expect("recorded attempts");
    let mut summary = smith_bench::summarise(
        &results,
        "recorded",
        "small",
        smith_bench::Design::Single,
        123,
        None,
        &smith_bench::PriceTable::default(),
    )
    .expect("whole summary");
    let first = summary.groups[0].clone();
    for change in 0..4 {
        let mut other = first.clone();
        match change {
            0 => other.agent = smith_bench::Agent::Smith,
            1 => other.configuration_sha256 = "other-configuration".into(),
            2 => other.model.requested_model = "other-model".into(),
            _ => other.model.requested_effort = "other-effort".into(),
        }
        summary.groups.push(other);
    }
    summary
}

#[test]
fn a_baseline_selector_keeps_one_configuration_model_and_effort() {
    let summary = multi_configuration_summary();
    let first = summary.groups[0].clone();
    let selected = smith_bench::baseline_group(
        &summary,
        first.agent,
        &first.configuration_sha256,
        &first.model.requested_model,
        &first.model.requested_effort,
    )
    .expect("one selected configuration");
    assert_eq!(selected.groups.as_slice(), std::slice::from_ref(&first));
    assert_eq!(summary.groups.len(), 5, "the whole suite summary stays intact");
    assert!(
        smith_bench::baseline_group(
            &summary,
            first.agent,
            "missing",
            &first.model.requested_model,
            &first.model.requested_effort,
        )
        .is_err()
    );
}
