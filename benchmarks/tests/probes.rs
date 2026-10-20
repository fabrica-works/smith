//! Smoke task manifests and decision boundaries use recorded and typed outside evidence.

use smith_bench::{CheckVerdict, End, Judgment, Measure, judge, judge_suite};
use std::path::PathBuf;

pub mod common;

fn history() -> smith_bench::Task {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    smith_bench::check_task(&root.join("tasks/probes/history/task.toml"), &root.join("../docs/design"))
        .expect("frozen history task")
}

#[test]
fn history_reads_one_file_per_completion_to_its_sum() {
    let task = history();
    let seed = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tasks/probes/history/seed");
    let mut next = "node-93e158a7.txt".to_owned();
    let mut sum = 0_u32;
    let mut visited = std::collections::BTreeSet::new();
    for _ in 0..32 {
        assert!(visited.insert(next.clone()));
        let text = std::fs::read_to_string(seed.join(&next)).expect("next file");
        assert_eq!(text.len(), 8192);
        let mut lines = text.lines();
        sum += lines.next().expect("value").strip_prefix("value: ").expect("field").parse::<u32>().expect("integer");
        next = lines.next().expect("next").strip_prefix("next: ").expect("field").to_owned();
    }
    assert_eq!(next, "END");
    assert_eq!(sum, 528);
    assert_eq!(std::fs::read_dir(seed).expect("seed").count(), 32);
    assert!(task.outcome.contains(&smith_bench::OutcomeCheck::Answer { pattern: "^ALPHA 528$".into() }));
    assert!(task.event.contains(&smith_bench::EventCheck::ToolCall {
        tool: "read".into(),
        count: None,
        input_bytes_at_least: None,
        verdict: Some("read".into()),
        exit: None,
        clamped: None,
    }));
    assert!(
        task.event.contains(&smith_bench::EventCheck::CallCount {
            tool: "read".into(),
            count: smith_bench::Count::Exactly(32),
        })
    );
    assert!(task.event.contains(&smith_bench::EventCheck::CallsPerResponse { tool: "read".into(), at_most: 1 }));
    assert_eq!(task.smith["deployment.calls_per_response"].as_integer(), Some(1));
}

fn history_observer(records: &mut [smith_events::Record]) -> smith_bench::agent::smith::events::Observer {
    for (position, record) in records.iter_mut().enumerate() {
        record.t_ms = u64::try_from(position).expect("small fixture");
    }
    let bytes = common::events::bytes(records);
    let ceiling = u32::try_from(bytes.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("records"))
        .expect("bounded fixture");
    let mut observer = smith_bench::agent::smith::events::Observer::with_ceiling(&common::events::LIMITS, ceiling)
        .expect("reader with the fixture's encoded ceiling");
    observer.feed(&bytes).expect("outside records");
    observer.finish().expect("whole stream");
    observer
}

#[test]
fn small_window_requires_a_compaction_and_no_prompt_past_its_window() {
    const COMPACTION_MATERIAL: u64 = 5_894; // 10_000 - 4_096 - 10: the fixture's prompt is above a line of 10.
    let base = history();
    let task = base.with_variant("small-window").expect("small window");
    assert_eq!(task.smith.len(), base.smith.len());
    assert_eq!(task.smith["models.\"{model}\".window"].as_integer(), Some(65536));
    for (key, value) in &base.smith {
        if key != "models.\"{model}\".window" {
            assert_eq!(&task.smith[key], value);
        }
    }
    assert_eq!(task.prompt, base.prompt);
    assert!(32 * 8192 >= 4 * task.smith["models.\"{model}\".window"].as_integer().expect("window"));
    let compacted = smith_bench::EventCheck::ConversationOpened {
        kind: "compaction".into(),
        count: smith_bench::Count::AtLeast(1),
    };
    assert!(task.event.contains(&compacted));
    assert!(task.event.contains(&smith_bench::EventCheck::PromptWithinWindow));
    let mut records = common::events::balanced();
    if let smith_events::Event::SessionStarted(value) = &mut records[0].event {
        value
            .limits
            .push(smith_events::EffectiveLimit {
                name: b"run.compaction_material".as_slice().into(),
                value: smith_events::LimitValue::Integer(COMPACTION_MATERIAL),
            })
            .expect("material fits fixture");
    }
    assert_eq!(history_observer(&mut records).check(&compacted, "task").passed, Measure::Observed { value: false });
    let mut summary = records[2..6].to_vec();
    for record in &mut summary {
        if let smith_events::Event::ConversationOpened(value) = &mut record.event {
            value.kind = smith_events::ConversationKind::Compaction;
            value.conversation = 1;
            value.parent = Some(0);
        }
        if let smith_events::Event::ResponseStarted(value) = &mut record.event {
            value.conversation = 1;
        }
        if let smith_events::Event::ResponseCompleted(value) = &mut record.event {
            value.conversation = 1;
        }
        if let smith_events::Event::ConversationClosed(value) = &mut record.event {
            value.conversation = 1;
        }
    }
    records.splice(5..5, summary);
    let observer = history_observer(&mut records);
    assert_eq!(observer.check(&compacted, "task").passed, Measure::Observed { value: true });
    assert_eq!(
        observer.check(&smith_bench::EventCheck::PromptWithinWindow, "task").passed,
        Measure::Observed { value: true }
    );
    if let smith_events::Event::ResponseCompleted(value) = &mut records[4].event {
        value.usage.input_tokens = Some(10_000);
    }
    assert_eq!(
        history_observer(&mut records).check(&smith_bench::EventCheck::PromptWithinWindow, "task").passed,
        Measure::Observed { value: false }
    );
}

fn result() -> smith_bench::AttemptResult {
    smith_bench::read_result(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/results/attempt.json"))
        .expect("unchanged recorded result")
}

fn smoke_entry() -> smith_bench::CatalogueTask {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("tasks/probes/smoke-tools/task.toml");
    let task = smith_bench::read_task(&manifest).expect("smoke task");
    smith_bench::CatalogueTask { name: "probes/smoke-tools".into(), manifest, task }
}

#[test]
fn smoke_skips_an_uncalibrated_probe() {
    let entry = smoke_entry();
    let selected =
        smith_bench::probe::gating::smoke_tasks(std::slice::from_ref(&entry), std::slice::from_ref(&entry.name))
            .expect("selection");
    assert_eq!(selected, Vec::<String>::new());
}

#[test]
fn the_smoke_suite_names_the_one_turn_refusal_probe() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite = smith_bench::read_suite(&root.join("suites/smoke.toml")).expect("suite");
    assert_eq!(
        suite.tasks,
        [
            "probes/first-request",
            "probes/smoke-tools",
            "probes/large-write",
            "probes/budget/child-quota",
            "probes/budget/wrap-up",
            "probes/failures/one-turn"
        ]
    );
}

#[test]
fn smoke_keeps_a_probe_with_current_calibration_digests() {
    let mut entry = smoke_entry();
    entry.task.calibration = Some(common::calibration(&entry.task));
    let selected =
        smith_bench::probe::gating::smoke_tasks(std::slice::from_ref(&entry), std::slice::from_ref(&entry.name))
            .expect("selection");
    assert_eq!(selected, ["probes/smoke-tools"]);
}

#[test]
fn smoke_skips_a_probe_whose_calibration_digests_changed() {
    let mut entry = smoke_entry();
    entry.task.calibration = Some(common::calibration(&entry.task));
    entry.task.prompt.push_str("changed prompt");
    let selected =
        smith_bench::probe::gating::smoke_tasks(std::slice::from_ref(&entry), std::slice::from_ref(&entry.name))
            .expect("selection");
    assert_eq!(selected, Vec::<String>::new());
}

fn failed_probe() -> smith_bench::batch::Batch {
    let mut value = result();
    value.identity.task = "probes/smoke-tools".into();
    value.outcome.end = End::Completed;
    value.outcome.deciding = vec!["end".into()];
    value.outcome.checks =
        vec![CheckVerdict { check: "end".into(), passed: Measure::Observed { value: false }, evidence: vec![] }];
    smith_bench::batch::Batch {
        probes: [(format!("{}/{}", value.identity.task, value.identity.arm), smith_bench::ProbeVerdict::Regression)]
            .into(),
        results: vec![value],
        incomplete: false,
    }
}

#[test]
fn an_uncalibrated_probe_failure_is_not_a_regression() {
    let batch = failed_probe();
    let names = ["probes/smoke-tools".into()].into();
    assert_eq!(smith_bench::probe::gating::judgment(&batch, &names), Judgment::Pass);
    assert_eq!(batch.results.len(), 1, "raw attempt stays available");
}

#[test]
fn a_calibrated_probe_failure_still_counts_as_a_regression() {
    assert_eq!(
        smith_bench::probe::gating::judgment(&failed_probe(), &std::collections::BTreeSet::default()),
        Judgment::Fail
    );
}

#[test]
fn an_uncalibrated_harness_error_still_makes_the_suite_incomplete() {
    let mut batch = failed_probe();
    batch.results[0].outcome.end = End::HarnessError { what: "recorded harness error".into() };
    let names = ["probes/smoke-tools".into()].into();
    assert_eq!(smith_bench::probe::gating::judgment(&batch, &names), Judgment::Incomplete);
}

#[test]
fn credentials_requires_the_refresh_pair_with_both_answers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::check_task(&root.join("tasks/probes/credentials/task.toml"), &root.join("../docs/design"))
        .expect("credentials task with its frozen seed");
    assert_eq!(task.deciding, ["end", "answer", "credential-refresh"]);
    assert_eq!(task.smith["deployment.refresh_lead"].as_integer(), Some(34_560_000));
    assert!(task.prompt.contains("CREDENTIALS_REFRESH_OK"));
    assert_eq!(task.version, 2);
}

fn follow_up() -> smith_bench::Task {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    smith_bench::check_task(&root.join("tasks/probes/follow-up/task.toml"), &root.join("../docs/design"))
        .expect("frozen follow-up task")
}

fn exchanges(records: &mut [smith_events::Record]) -> Measure<bool> {
    for (position, record) in records.iter_mut().enumerate() {
        record.t_ms = u64::try_from(position).expect("small fixture");
    }
    let bytes = common::events::bytes(records);
    let ceiling = u32::try_from(bytes.split_inclusive(|byte| *byte == b'\n').map(<[u8]>::len).max().expect("records"))
        .expect("bounded fixture");
    let mut observer = smith_bench::agent::smith::events::Observer::with_ceiling(&common::events::LIMITS, ceiling)
        .expect("reader with semantic limits and the fixture's encoded ceiling");
    observer.feed(&bytes).expect("outside records");
    observer.finish().expect("whole stream");
    observer.check(&smith_bench::EventCheck::CompletionPerExchange, "task").passed
}

#[test]
fn follow_up_sends_its_request_on_finish() {
    let task = follow_up();
    assert_eq!(task.person.len(), 1);
    assert_eq!(task.person[0].when, smith_bench::agent::Trigger::ToolCall { tool: "finish".into() });
    assert!(task.person[0].text.contains("printf 'FOLLOW_UP\\n' > follow-up.txt"));
    assert!(task.event.contains(&smith_bench::EventCheck::HandledOnce));
    assert!(task.event.contains(&smith_bench::EventCheck::ToolCall {
        tool: "shell".into(),
        count: Some(smith_bench::Count::Exactly(1)),
        input_bytes_at_least: None,
        verdict: Some("exited".into()),
        exit: None,
        clamped: None,
    }));
    assert!(task.outcome.contains(&smith_bench::OutcomeCheck::ExitCode { code: 0 }));
    assert!(task.outcome.contains(&smith_bench::OutcomeCheck::FileContent {
        path: "follow-up.txt".into(),
        content: "FOLLOW_UP\n".into(),
    }));
}

#[test]
fn follow_up_eof_ends_input_after_its_second_line() {
    let base = follow_up();
    let task = base.with_variant("eof").expect("eof variant");
    assert_eq!(task.person, base.person);
    assert_eq!(task.input_end, Some(smith_bench::agent::Close::AfterLastCue));
    assert_eq!(task.event, base.event);
    assert_eq!(task.outcome, base.outcome);
    assert_eq!(task.person.len() + 1, 2);
}

#[test]
fn follow_up_chat_judges_one_completion_per_exchange() {
    let task = follow_up().with_variant("chat").expect("chat variant");
    assert_eq!(task.person.len() + 1, 3);
    assert!(task.person.iter().all(|cue| cue.when == smith_bench::agent::Trigger::ToolCall { tool: "wait".into() }));
    assert!(task.event.contains(&smith_bench::EventCheck::CompletionPerExchange));
    assert!(task.event.contains(&smith_bench::EventCheck::HandledOnce));
    let mut records = common::events::balanced();
    let mut received =
        common::events::record(include_str!("../../crates/smith-events/golden/v4/message_received_everything.jsonl"));
    if let smith_events::Event::MessageReceived(value) = &mut received.event {
        value.run = 0;
        value.label = Some(b"person".as_slice().into());
        value.refused = None;
    }
    records.insert(2, received.clone());
    assert_eq!(exchanges(&mut records), Measure::Observed { value: true });
    records.insert(2, received);
    assert_eq!(exchanges(&mut records), Measure::Observed { value: false });
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
    let tasks: Vec<_> = ["first-request", "smoke-tools", "large-write", "budget", "failures"]
        .into_iter()
        .map(|name| {
            let manifest = root.join("tasks/probes").join(name).join("task.toml");
            let task = smith_bench::check_task(&manifest, &design).expect("frozen seed and manifest");
            smith_bench::CatalogueTask { name: format!("probes/{name}"), manifest, task }
        })
        .collect();
    let models = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("model tiers");
    for name in ["smoke", "calibrate"] {
        let path = root.join("suites").join(format!("{name}.toml"));
        let suite = smith_bench::read_suite(&path).expect("suite format");
        assert_eq!(
            smith_bench::validate_suite(&path, &suite, &tasks, &root.join("agents"), &models)
                .expect("pins/selectors")
                .iter()
                .filter(|task| *task != "probes/failures/one-turn")
                .count(),
            5
        );
    }
    let failures = tasks.iter().find(|task| task.name == "probes/failures").expect("authored deferred probe");
    let one_turn = failures.task.with_variant("one-turn").expect("offline only until10.5");
    assert_eq!(one_turn.budget.turns, Some(1));
    assert!(one_turn.deciding.iter().any(|name| name == "no-request"));
    let write = tasks.iter().find(|task| task.name == "probes/large-write").expect("large-write");
    let oversized = write.task.with_variant("oversized").expect("named replacement");
    assert_eq!(write.task.event.len(), 4, "original three assertions plus captured-file comparison");
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
    fix_body_keeps_both_runs_and_distinguishes_effect_from_unjudged();
    decision_names_keep_advisory_unknown_and_false_evidence_without_deciding_the_probe();
    attempt_and_suite_share_failure_precedence_and_preserve_expected_budget_class();
    smoke_listing_is_a_minimum_and_shell_stays_exact();
}

#[test]
fn the_smoke_retains_each_probes_frozen_checks_and_inputs() {
    smoke_catalogue_selects_five_active_probes_and_retains_deferred_one_turn_with_exact_frozen_inputs();
}

fn smoke_listing_is_a_minimum_and_shell_stays_exact() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let task = smith_bench::read_task(&root.join("tasks/probes/smoke-tools/task.toml")).expect("smoke task");
    for task in [task.clone(), task.with_variant("child").expect("child task")] {
        let count = |name: &str| {
            task.event
                .iter()
                .find_map(|check| {
                    if let smith_bench::EventCheck::ToolCall { tool, count, verdict, .. } = check
                        && tool == name
                    {
                        assert_eq!(verdict.as_deref(), Some(if name == "list" { "listed" } else { "exited" }));
                        *count
                    } else {
                        None
                    }
                })
                .expect("authored tool cardinality")
        };
        assert_eq!(count("list"), smith_bench::Count::AtLeast(1), "benchmarks.md 11.1 requires a listing");
        assert!(!count("list").holds(0));
        assert!(count("list").holds(1));
        assert!(count("list").holds(2));
        assert_eq!(count("shell"), smith_bench::Count::Exactly(1));
        assert!(!count("shell").holds(0));
        assert!(count("shell").holds(1));
        assert!(!count("shell").holds(2));
    }
}

fn fix_body_keeps_both_runs_and_distinguishes_effect_from_unjudged() {
    use smith_bench::fix::{ProbeRun, Shown, body, report};
    let before = ProbeRun {
        arm: smith_bench::arm::ArmRef::Installed { program: "/parent".into() },
        run: "red/attempt01".into(),
        verdict: Judgment::Fail,
        setup_refused: false,
    };
    let mut after = ProbeRun {
        arm: smith_bench::arm::ArmRef::Installed { program: "/fix".into() },
        run: "green/attempt01".into(),
        verdict: Judgment::Pass,
        setup_refused: false,
    };
    let value = report("probes/smoke-tools".into(), before.clone(), after.clone());
    assert_eq!(value.shown, Shown::Effect);
    assert_eq!(
        body(&value),
        "Probe: probes/smoke-tools\nBefore: installed /parent run=red/attempt01 Fail\nAfter: installed /fix run=green/attempt01 Pass\nShown: Effect\n"
    );
    after.verdict = Judgment::Incomplete;
    assert_eq!(report("probe".into(), before.clone(), after.clone()).shown, Shown::Unjudged);
    after.verdict = Judgment::Fail;
    assert_eq!(report("probe".into(), before, after).shown, Shown::FailsAfter);
}

#[test]
fn the_probes_suite_names_each_probe_and_variant_once() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tasks: Vec<_> = std::fs::read_dir(root.join("tasks/probes"))
        .expect("probe catalogue")
        .map(|entry| entry.expect("probe directory"))
        .filter(|entry| entry.file_type().expect("catalogue entry type").is_dir())
        .map(|entry| smith_bench::read_task(&entry.path().join("task.toml")).expect("probe manifest"))
        .collect();
    let suite = smith_bench::read_suite(&root.join("suites/probes.toml")).expect("probes suite");
    let mut named: Vec<&str> = suite
        .agents
        .iter()
        .filter(|agent| agent.agent == smith_bench::Agent::Smith)
        .flat_map(|agent| agent.tasks.iter().map(String::as_str))
        .collect();
    named.sort_unstable();
    let mut expected: Vec<String> = tasks
        .iter()
        .flat_map(|task| {
            std::iter::once(format!("probes/{}", task.id))
                .chain(task.variant.iter().map(|variant| format!("probes/{}/{}", task.id, variant.name)))
        })
        .collect();
    // budget/spend waits for its next task version (decisions 13.95).
    expected.retain(|probe| probe != "probes/budget/spend");
    expected.sort_unstable();
    assert_eq!(named, expected.iter().map(String::as_str).collect::<Vec<_>>());
    let mut selected = suite.tasks.clone();
    selected.sort_unstable();
    assert_eq!(selected, expected);
}

#[test]
fn an_agent_task_outside_the_suite_is_refused() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("tasks/probes/smoke-tools/task.toml");
    let tasks = [smith_bench::CatalogueTask {
        name: "probes/smoke-tools".into(),
        task: smith_bench::read_task(&manifest).expect("task"),
        manifest,
    }];
    let file = root.join("suites/probes.toml");
    let mut suite = smith_bench::read_suite(&file).expect("suite");
    suite.tasks = vec!["probes/smoke-tools".into()];
    suite.agents[0].tasks = vec!["fixtures/01-backoff".into()];
    let models = smith_bench::read_model_tiers(&root.join("agents/models.toml")).expect("tiers");
    assert_eq!(
        smith_bench::validate_suite(&file, &suite, &tasks, &root.join("agents"), &models)
            .expect_err("agent task outside suite")
            .key,
        "agents.tasks"
    );
}

#[test]
fn codex_runs_only_its_outcome_halves() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite = smith_bench::read_suite(&root.join("suites/probes.toml")).expect("suite");
    let codex: Vec<_> = suite.agents.iter().filter(|agent| agent.agent == smith_bench::Agent::Codex).collect();
    assert_eq!(codex.len(), 1);
    assert_eq!(codex[0].config, "standard");
    assert_eq!(codex[0].tasks, ["probes/smoke-tools", "probes/smoke-tools/child", "probes/large-write"]);
}
