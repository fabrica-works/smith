use super::*;

fn fixture() -> (PathBuf, crate::CatalogueTask, Suite, ArmRef) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("tasks/probes/history/task.toml");
    let task = crate::read_task(&manifest).expect("history").with_variant("small-window").expect("variant");
    let suite = toml::from_str("name='compaction'\ntier='small'\ndesign='interleaved'\nreference='80-percent'\nrepetitions=5\nrerun_failed=0\nmax_wall_seconds=3600\nmax_tokens=12000000\ntasks=['probes/history/small-window']\n[[agents]]\nagent='smith'\nprovider='codex'\nconfig='everything'\n[[arm]]\nsource='override'\nname='80-percent'\n[arm.smith]\n'deployment.compaction_threshold.numerator'=4\n'deployment.compaction_threshold.denominator'=5\n[[arm]]\nsource='override'\nname='60-percent'\n[arm.smith]\n'deployment.compaction_threshold.numerator'=3\n'deployment.compaction_threshold.denominator'=5\n").expect("suite");
    let arm = ArmRef::Built(crate::arm::Build {
        directory: "/unexecuted-arm".into(),
        arm: crate::arm::BuiltArm {
            version: 1,
            commit: "0".repeat(40),
            sha256: "1".repeat(64),
            toolchain: "unexecuted".into(),
            binaries: Vec::new(),
        },
    });
    (root, crate::CatalogueTask { name: "probes/history/small-window".into(), manifest, task }, suite, arm)
}

fn requests(root: &Path, entry: &crate::CatalogueTask, suite: &Suite, arm: &ArmRef) -> Vec<crate::run::Prepared> {
    let mut requests = Vec::new();
    task_requests(
        root.parent().expect("repository"),
        entry,
        suite,
        (arm, &[]),
        ("offline-overrides", "2026-10-14T00:00:00Z", Some(crate::SmithFace::Events), &"0".repeat(40)),
        5,
        &mut requests,
    )
    .expect("prepared requests");
    requests
}

#[test]
fn a_suite_with_two_override_arms_prepares_ten_interleaved_requests() {
    let (root, entry, suite, arm) = fixture();
    let prepared = requests(&root, &entry, &suite, &arm);
    assert_eq!(prepared.len(), 10);
    let ordered = crate::batch::ordered(&suite, prepared, 17).expect("interleaved requests");
    for block in ordered.chunks_exact(2) {
        assert_ne!(block[0].identity.arm, block[1].identity.arm);
    }
    let mut results = Vec::new();
    for request in &ordered {
        assert_eq!(request.program, Path::new("/unexecuted-arm/smith"));
        assert_eq!(request.identity.task_version, entry.task.version);
        assert_eq!(request.identity.model.requested_model, "gpt-6-luna");
        let encoded = serde_json::to_vec(request).expect("persisted request");
        let decoded: crate::run::Prepared = serde_json::from_slice(&encoded).expect("request identity");
        let mut result =
            crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("recorded result");
        result.identity = decoded.identity;
        results.push(result);
    }
    let summary = crate::summarise(
        &results,
        "compaction",
        "small",
        crate::Design::Interleaved,
        17,
        None,
        suite.reference.as_deref(),
    )
    .expect("grouped summary");
    assert_eq!(summary.groups.len(), 2);
    assert_eq!(summary.groups.iter().map(|group| group.arm.as_str()).collect::<Vec<_>>(), ["60-percent", "80-percent"]);
    assert!(summary.groups.iter().all(|group| group.attempts.len() == 5));
}

#[test]
fn an_override_arms_settings_reach_its_command_after_the_tasks() {
    let (root, entry, suite, arm) = fixture();
    let prepared = requests(&root, &entry, &suite, &arm);
    for request in prepared {
        let mut command = ["exec", "-c", "deployment.read_window=8192", "-"].map(Into::into).to_vec();
        crate::batch::settings_arguments(&mut command, &request.arguments);
        let expected_numerator = if request.identity.arm == "80-percent" { "4" } else { "3" };
        let expected = [
            "exec",
            "-c",
            "deployment.read_window=8192",
            "-c",
            "deployment.compaction_threshold.denominator=5",
            "-c",
            &format!("deployment.compaction_threshold.numerator={expected_numerator}"),
            "-",
        ];
        assert_eq!(command.iter().map(|argument| argument.to_str().expect("argument")).collect::<Vec<_>>(), expected);
        let mut check = vec!["check".into(), "--json".into()];
        crate::batch::settings_arguments(&mut check, &request.arguments);
        assert_eq!(&check[2..], &command[3..command.len() - 1]);
    }
}

#[test]
fn a_suite_gives_one_task_its_own_repetitions() {
    let (_, _, mut suite, _) = fixture();
    suite.repetitions_by_task.insert("repository/shell-cwd".into(), 3);
    for (name, authored, logical, expected) in [
        ("repository/shell-cwd", None, false, 3),
        ("repository/shell-cwd", Some(2), false, 3),
        ("fixtures/01-backoff", None, false, 5),
        ("fixtures/01-backoff", Some(2), false, 2),
        ("repository/shell-cwd", Some(2), true, 6),
    ] {
        assert_eq!(suite.task_repetitions(name, authored, logical).expect("count"), expected, "{name}");
    }
    let (root, entry, _, arm) = fixture();
    suite.repetitions_by_task.insert(entry.name.clone(), 3);
    let mut prepared = Vec::new();
    let count = suite.task_repetitions(&entry.name, entry.task.repetitions, false).expect("count");
    task_requests(
        root.parent().expect("repository"),
        &entry,
        &suite,
        (&arm, &[]),
        ("offline-overrides", "2026-10-14T00:00:00Z", Some(crate::SmithFace::Events), &"0".repeat(40)),
        count,
        &mut prepared,
    )
    .expect("prepared requests");
    assert_eq!(crate::batch::ordered(&suite, prepared, 17).expect("ordered").len(), 6);
}

#[test]
fn the_parity_suite_runs_58_attempts_interleaved() {
    let (root, entry, base, arm) = fixture();
    let suite = crate::read_suite(&root.join("suites/parity.toml")).expect("parity suite");
    assert_eq!(suite.design, crate::Design::Interleaved);
    assert_eq!(suite.tier, "working");
    assert_eq!(suite.rerun_failed, 0);
    assert_eq!(suite.max_wall_seconds, 28_800);
    assert_eq!(suite.max_tokens, 150_000_000);
    assert_eq!(suite.agents.iter().map(|agent| agent.agent).collect::<Vec<_>>(), [Agent::Smith, Agent::Codex]);
    assert!(suite.agents.iter().all(|agent| agent.config == "standard" && agent.provider == crate::Provider::Codex));
    let prepared = parity_requests(&root, &suite, &requests(&root, &entry, &base, &arm)[0]);
    let ordered = crate::batch::ordered(&suite, prepared.clone(), 17).expect("interleaved");
    assert_eq!(ordered.len(), 68);
    let repeated = crate::batch::ordered(&suite, prepared, 17).expect("repeated order");
    assert_eq!(
        repeated.iter().map(|request| &request.identity).collect::<Vec<_>>(),
        ordered.iter().map(|request| &request.identity).collect::<Vec<_>>()
    );
    for (name, expected) in [
        ("fixtures/01-backoff", 16),
        ("fixtures/02-line-decoder", 16),
        ("fixtures/03-planner", 16),
        ("repository/shell-cwd", 20),
    ] {
        assert_eq!(ordered.iter().filter(|request| request.identity.task == name).count(), expected, "{name}");
    }
    for block in ordered.chunks_exact(2) {
        assert_eq!(block[0].identity.task, block[1].identity.task);
        assert_ne!(block[0].identity.agent, block[1].identity.agent);
    }
}

fn parity_requests(root: &Path, suite: &Suite, template: &crate::run::Prepared) -> Vec<crate::run::Prepared> {
    let mut prepared = Vec::new();
    for name in &suite.tasks {
        let task = root.join("tasks").join(name).join("task.toml");
        let authored = crate::read_task(&task).expect("task").repetitions;
        for agent in &suite.agents {
            for _ in 0..suite.task_repetitions(name, authored, false).expect("repetitions") {
                let mut request = template.clone();
                request.task.clone_from(&task);
                request.identity.task.clone_from(name);
                request.identity.agent = agent.agent;
                request.identity.arm = agent.agent.directory().into();
                request.identity.attempt = format!("{:02}", prepared.len() + 1);
                prepared.push(request);
            }
        }
    }
    prepared
}

fn binary_fixture() -> (PathBuf, Suite, Vec<(String, ArmRef)>) {
    let (root, _, mut suite, arm) = fixture();
    suite.name = "role".into();
    suite.reference = Some("before".into());
    suite.tasks = ["fixtures/01-backoff", "fixtures/02-line-decoder", "fixtures/03-planner"].map(Into::into).to_vec();
    suite.arms = vec![
        crate::Arm::Binary { name: "before".into(), commit: "0".repeat(40) },
        crate::Arm::Binary { name: "after".into(), commit: "2".repeat(40) },
    ];
    let ArmRef::Built(mut after) = arm.clone() else { panic!("built fixture") };
    after.directory = "/after-arm".into();
    after.arm.commit = "2".repeat(40);
    after.arm.sha256 = "3".repeat(64);
    (root, suite, vec![("before".into(), arm), ("after".into(), ArmRef::Built(after))])
}

fn binary_requests(root: &Path, suite: &Suite, binaries: &[(String, ArmRef)]) -> Vec<crate::run::Prepared> {
    const REPETITIONS_PER_ARM: u32 = 5;
    let mut prepared = Vec::new();
    for name in &suite.tasks {
        let manifest = root.join("tasks").join(name).join("task.toml");
        let entry =
            crate::CatalogueTask { name: name.clone(), task: crate::read_task(&manifest).expect("task"), manifest };
        task_requests(
            root.parent().expect("repository"),
            &entry,
            suite,
            (&binaries[0].1, binaries),
            ("offline-binaries", "2026-10-15T00:00:00Z", Some(crate::SmithFace::Events), &"0".repeat(40)),
            REPETITIONS_PER_ARM,
            &mut prepared,
        )
        .expect("prepared requests");
    }
    prepared
}

#[test]
fn a_suite_with_two_binary_arms_prepares_each_arm_by_its_name() {
    let (root, suite, binaries) = binary_fixture();
    let prepared = binary_requests(&root, &suite, &binaries);
    assert_eq!(prepared.len(), 30);
    assert_eq!(prepared.iter().filter(|request| request.identity.arm == "before").count(), 15);
    assert_eq!(prepared.iter().filter(|request| request.identity.arm == "after").count(), 15);
    for task in prepared.chunks_exact(10) {
        assert_eq!(
            task.iter().map(|request| request.identity.arm.as_str()).collect::<Vec<_>>(),
            ["before", "before", "before", "before", "before", "after", "after", "after", "after", "after"]
        );
    }
    let ordered = crate::batch::ordered(&suite, prepared, 17).expect("interleaved requests");
    for request in &ordered {
        let (program, commit, sha256) = match request.identity.arm.as_str() {
            "before" => ("/unexecuted-arm/smith", "0".repeat(40), "1".repeat(64)),
            "after" => ("/after-arm/smith", "2".repeat(40), "3".repeat(64)),
            name => panic!("unexpected arm {name}"),
        };
        assert_eq!(request.program, Path::new(program));
        assert_eq!(request.identity.agent_version, commit);
        assert_eq!(request.identity.binary_sha256, sha256);
    }
    for block in ordered.chunks_exact(2) {
        assert_ne!(block[0].identity.arm, block[1].identity.arm);
    }
}

fn change_fixture() -> (PathBuf, Suite, Vec<(String, ArmRef)>) {
    let (root, _, binaries) = binary_fixture();
    let suite = crate::read_suite(&root.join("suites/parity-change.toml")).expect("change suite");
    let binaries = binaries.into_iter().zip(["change", "parent"]).map(|((_, arm), name)| (name.into(), arm)).collect();
    (root, suite, binaries)
}

#[test]
fn a_change_suite_runs_a_change_and_its_parent() {
    let (root, suite, binaries) = change_fixture();
    let prepared = binary_requests(&root, &suite, &binaries);
    let ordered = crate::batch::ordered(&suite, prepared, 17).expect("interleaved change arms");
    assert_eq!(ordered.len(), 40);
    for block in ordered.chunks_exact(2) {
        assert_eq!(block[0].identity.task, block[1].identity.task);
        assert_ne!(block[0].identity.arm, block[1].identity.arm);
    }
    for request in ordered {
        assert_eq!(request.identity.agent, Agent::Smith);
        let commit = match request.identity.arm.as_str() {
            "change" => "0".repeat(40),
            "parent" => "2".repeat(40),
            name => panic!("unexpected arm {name}"),
        };
        assert_eq!(request.identity.agent_version, commit);
    }
}

#[test]
fn a_change_suite_without_a_parent_is_refused() {
    let arguments = ["parity-change", "--commit", "HEAD"].map(str::to_owned);
    assert_eq!(
        command("suite", &arguments).expect_err("missing parent"),
        "change suite requires --parent or --parent-arm"
    );
}

#[test]
fn a_parent_given_to_another_suite_is_refused() {
    for option in ["--parent", "--parent-arm"] {
        let arguments = ["parity", option, "unused"].map(str::to_owned);
        assert_eq!(command("suite", &arguments).expect_err("unexpected parent"), "only change suites accept a parent");
    }
}

#[test]
fn a_change_suite_declaring_arms_is_refused() {
    let (root, entry, _, _) = fixture();
    let (_, mut suite, _) = change_fixture();
    suite.tasks = vec![entry.name.clone()];
    suite.arms.push(crate::Arm::Binary { name: "extra".into(), commit: "0".repeat(40) });
    let models = crate::read_model_tiers(&root.join("agents/models.toml")).expect("models");
    let error =
        crate::validate_suite(&root.join("suites/parity-change.toml"), &suite, &[entry], &root.join("agents"), &models)
            .expect_err("declared arms");
    assert_eq!(error.key, "change");
}

#[test]
fn the_change_suites_reference_is_its_parent() {
    let (_, suite, _) = change_fixture();
    assert_eq!(suite.reference_arm(), Some("parent"));
}

#[test]
fn a_suite_run_below_its_attempts_is_summarised_as_small() {
    const ATTEMPTS: u32 = 1;
    let (root, entry, base, arm) = fixture();
    let mut value: toml::Value = crate::formats::read_document(&root.join("suites/parity.toml")).expect("suite table");
    suite_attempts(&mut value, ATTEMPTS).expect("attempt override");
    let suite: Suite = value.try_into().expect("overridden suite");
    let requests = parity_requests(&root, &suite, &requests(&root, &entry, &base, &arm)[0]);
    assert_eq!(crate::batch::ordered(&suite, requests, 17).expect("small order").len(), 8);
    let mut result = crate::read_result(&root.join("tests/recorded/results/attempt.json")).expect("recorded");
    result.identity.agent = Agent::Smith;
    result.identity.agent_version = "0".repeat(40);
    result.identity.run = "small-parity".into();
    result.identity.task = "fixtures/01-backoff".into();
    let batch =
        crate::batch::Batch { results: vec![result], incomplete: false, probes: std::collections::BTreeMap::new() };
    let directory = std::env::temp_dir().join(format!("smith-small-suite-{}", std::process::id()));
    assert_eq!(small_summary(&batch, &suite, 17, &directory).expect("small summary"), Judgment::Incomplete);
    let summary = crate::read_summary(&directory.join("summary.json")).expect("run-local summary");
    assert_eq!(summary.parity.expect("section").verdict, crate::parity::Verdict::Incomplete);
    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn a_change_run_below_its_attempts_exits_two_whatever_its_stop() {
    const FIXTURE_ATTEMPTS: usize = 1;
    const REPOSITORY_ATTEMPTS: usize = 5;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut recorded = crate::read_summary(&root.join("tests/recorded/parity/change.json")).expect("change rows");
    let kept: std::collections::BTreeSet<_> = recorded
        .groups
        .iter()
        .flat_map(|group| {
            group
                .attempts
                .iter()
                .take(if group.task == "repository/shell-cwd" { REPOSITORY_ATTEMPTS } else { FIXTURE_ATTEMPTS })
                .cloned()
        })
        .collect();
    recorded.attempts.retain(|row| kept.contains(&row.attempt));
    let directory = std::env::temp_dir().join(format!("smith-small-change-{}", std::process::id()));
    fs::create_dir(&directory).expect("run directory");
    let mut baseline = crate::read_summary(&root.join("tests/recorded/parity/on-par.json")).expect("baseline");
    baseline.attempts.clear();
    baseline.groups.clear();
    for (index, upper) in [1.1, 0.9].into_iter().enumerate() {
        let run = directory.join(index.to_string());
        fs::create_dir(&run).expect("run directory");
        for criterion in &mut baseline.parity.as_mut().expect("section").criteria {
            criterion.interval = (0.8, upper);
        }
        let file = run.join("baseline.json");
        crate::write_summary(&file, &baseline).expect("baseline input");
        let inputs = crate::parity::change::Inputs { baseline_run: Some(file), landed: Vec::new() };
        assert_eq!(settle_change(recorded.clone(), &run, &inputs, true).expect("small run"), Judgment::Incomplete);
        assert_eq!(crate::read_summary(&run.join("summary.json")).expect("run-local summary").suite, "parity-change");
    }
    fs::remove_dir_all(directory).expect("cleanup");
}
