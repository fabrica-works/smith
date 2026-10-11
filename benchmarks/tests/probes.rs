//! Smoke task manifests and decision boundaries use recorded and typed outside evidence.

use smith_bench::{CheckVerdict, End, Judgment, Measure, judge, judge_suite};
use std::path::PathBuf;

fn result() -> smith_bench::AttemptResult {
    smith_bench::read_result(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json"))
        .expect("unchanged recorded result")
}

fn decision_names_keep_advisory_unknown_and_false_evidence_without_deciding_the_probe() {
    let mut value = result();
    value.outcome.end = End::Completed;
    value.outcome.grade = None;
    value.outcome.deciding = vec!["answer".into(), "paired".into()];
    value.outcome.checks = vec![
        CheckVerdict {
            check: "answer".into(),
            passed: Measure::Observed { value: true },
            evidence: vec!["accepted ALPHA".into()],
        },
        CheckVerdict { check: "paired".into(), passed: Measure::Observed { value: true }, evidence: vec![] },
        CheckVerdict {
            check: "headroom/peak-rss".into(),
            passed: Measure::unavailable("not yet: effective projection"),
            evidence: vec![],
        },
        CheckVerdict { check: "advisory".into(), passed: Measure::Observed { value: false }, evidence: vec![] },
    ];
    assert_eq!(judge(&value).code(), 0);
    assert_eq!(value.outcome.checks.len(), 4, "raw advisory evidence survives");
    value.outcome.checks[1].passed = Measure::unavailable("terminal not observed");
    assert_eq!(judge(&value).code(), 2);
    value.outcome.checks[0].passed = Measure::Observed { value: false };
    assert_eq!(judge(&value).code(), 1, "a false decides despite another unavailable");
    value.outcome.checks[0].passed = Measure::Observed { value: true };
    value.outcome.checks.remove(1);
    assert_eq!(judge(&value), Judgment::Incomplete, "absent deciding check is never vacuous truth");
    value.outcome.deciding = vec!["answer".into()];
    value.outcome.end = End::Timeout;
    assert_eq!(judge(&value), Judgment::Fail, "true answer cannot turn timeout into a pass");
}

fn attempt_and_suite_share_failure_precedence_and_preserve_expected_budget_class() {
    let mut value = result();
    value.outcome.end = End::Completed;
    value.outcome.grade = None;
    value.outcome.deciding = vec!["end".into()];
    value.outcome.checks =
        vec![CheckVerdict { check: "end".into(), passed: Measure::Observed { value: true }, evidence: vec![] }];
    assert_eq!(judge_suite(&[value.clone()], false), Judgment::Pass);
    assert_eq!(judge_suite(&[value.clone()], true), Judgment::Incomplete);
    assert_eq!(judge_suite(&[], false), Judgment::Incomplete);
    let mut missing = value.clone();
    missing.outcome.checks[0].passed = Measure::unavailable("unread terminal");
    assert_eq!(judge_suite(&[missing.clone(), value.clone()], false).code(), 2);
    value.outcome.checks[0].passed = Measure::Observed { value: false };
    assert_eq!(judge_suite(&[missing, value.clone()], true).code(), 1);
    value.outcome.end = End::Budget { which: smith_bench::BudgetLimit::Turns };
    assert_eq!(judge(&value), Judgment::Fail);
    value.outcome.checks[0].passed = Measure::Observed { value: true };
    assert_eq!(judge(&value), Judgment::Pass, "an explicitly expected budget diagnostic is judged by its checks");
    value.outcome.end = End::Refused { setup: "budget reserve".into() };
    assert_eq!(judge(&value), Judgment::Pass, "expected product refusal is distinct from harness verdict");
    value.outcome.end = End::HarnessError { what: "cleanup failed".into() };
    assert_eq!(judge(&value), Judgment::Fail);
}

fn smoke_catalogue_selects_five_active_probes_and_retains_deferred_one_turn_with_exact_frozen_inputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let design = root.join("../docs/design");
    let tasks = smith_bench::catalogue(&root.join("tasks"), &design).expect("frozen seeds and manifests");
    let models = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("model tiers");
    for name in ["smoke", "calibrate"] {
        let path = root.join("suites").join(format!("{name}.toml"));
        let suite = smith_bench::read_suite(&path).expect("suite format");
        assert_eq!(
            smith_bench::validate_suite(&path, &suite, &tasks, &root.join("agents"), &models)
                .expect("pins/selectors")
                .len(),
            5
        );
    }
    let failures = tasks.iter().find(|task| task.name == "probes/failures").expect("authored deferred probe");
    let one_turn = failures.task.with_variant("one-turn").expect("offline only until10.5");
    assert_eq!(one_turn.budget.turns, Some(1));
    assert!(one_turn.deciding.iter().any(|name| name == "no-request"));
    let write = tasks.iter().find(|task| task.name == "probes/large-write").expect("large-write");
    let oversized = write.task.with_variant("oversized").expect("named replacement");
    assert_eq!(write.task.event.len(), 3, "normal one-call assertions retained");
    assert!(matches!(
        write.task.event[1],
        smith_bench::EventCheck::ToolCall { count: Some(smith_bench::Count::Exactly(1)), .. }
    ));
    assert!(oversized.event.iter().any(|check| matches!(
        check,
        smith_bench::EventCheck::Blocks { oversized: Some(smith_bench::Count::Exactly(1)), .. }
    )));
    assert!(
        !oversized.event.iter().any(|check| matches!(
            check,
            smith_bench::EventCheck::ToolCall { count: Some(smith_bench::Count::Exactly(1)), .. }
        )),
        "piecewise repair does not inherit the one-call predicate"
    );
    let smoke = tasks.iter().find(|task| task.name == "probes/smoke-tools").expect("smoke-tools");
    let child = smoke.task.with_variant("child").expect("historical additive behavior");
    assert!(smoke.task.event.iter().all(|check| child.event.contains(check)));
    let directory = std::env::temp_dir().join(format!("smith-probe-name-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("isolated author typo");
    let manifest = directory.join("task.toml");
    let source = std::fs::read_to_string(&smoke.manifest).expect("authored manifest");
    std::fs::write(&manifest, source.replace("tree-ended", "tree-edned")).expect("one changed check name");
    assert!(
        smith_bench::check_task(&manifest, &design)
            .expect_err("misspelled deciding check refused")
            .key
            .starts_with("deciding")
    );
    std::fs::remove_dir_all(directory).expect("offline cleanup");
    let serialized = smith_bench::render_result(&result()).expect("historical result remains valid");
    assert_eq!(smith_bench::parse_result(&root, &serialized).expect("historical round trip"), result());
}

#[test]
fn smoke_probe_documents_and_pass_fail_incomplete_boundary() {
    decision_names_keep_advisory_unknown_and_false_evidence_without_deciding_the_probe();
    attempt_and_suite_share_failure_precedence_and_preserve_expected_budget_class();
    smoke_catalogue_selects_five_active_probes_and_retains_deferred_one_turn_with_exact_frozen_inputs();
}
