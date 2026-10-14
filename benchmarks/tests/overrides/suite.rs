use super::*;

fn fixture() -> (PathBuf, crate::CatalogueTask, Suite, ArmRef) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("tasks/probes/history/task.toml");
    let task = crate::read_task(&manifest).expect("history").with_variant("small-window").expect("variant");
    let suite = toml::from_str("name='compaction'\ntier='small'\ndesign='interleaved'\nreference='80-percent'\nrepetitions=5\nrerun_failed=0\nmax_wall_seconds=3600\nmax_tokens=12000000\ntasks=['probes/history/small-window']\n[[agents]]\nagent='smith'\nprovider='codex'\nconfig='cache-reuse'\n[[arm]]\nsource='override'\nname='80-percent'\n[arm.smith]\n'deployment.compaction_threshold.numerator'=4\n'deployment.compaction_threshold.denominator'=5\n[[arm]]\nsource='override'\nname='60-percent'\n[arm.smith]\n'deployment.compaction_threshold.numerator'=3\n'deployment.compaction_threshold.denominator'=5\n").expect("suite");
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
        arm,
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
        &arm,
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
    assert_eq!(ordered.len(), 58);
    let repeated = crate::batch::ordered(&suite, prepared, 17).expect("repeated order");
    assert_eq!(
        repeated.iter().map(|request| &request.identity).collect::<Vec<_>>(),
        ordered.iter().map(|request| &request.identity).collect::<Vec<_>>()
    );
    for (name, expected) in [
        ("fixtures/01-backoff", 16),
        ("fixtures/02-line-decoder", 16),
        ("fixtures/03-planner", 16),
        ("repository/shell-cwd", 10),
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
