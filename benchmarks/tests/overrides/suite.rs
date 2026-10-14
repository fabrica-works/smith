use super::*;

fn fixture() -> (PathBuf, crate::CatalogueTask, Suite, ArmRef) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("tasks/probes/history/task.toml");
    let task = crate::read_task(&manifest).expect("history").with_variant("small-window").expect("variant");
    let suite = toml::from_str("name='compaction'\ntier='small'\ndesign='interleaved'\nrepetitions=5\nrerun_failed=0\nmax_wall_seconds=3600\nmax_tokens=12000000\ntasks=['probes/history/small-window']\n[[agents]]\nagent='smith'\nprovider='codex'\nconfig='cache-reuse'\n[[arm]]\nsource='override'\nname='80-percent'\n[arm.smith]\n'deployment.compaction_threshold.numerator'=4\n'deployment.compaction_threshold.denominator'=5\n[[arm]]\nsource='override'\nname='60-percent'\n[arm.smith]\n'deployment.compaction_threshold.numerator'=3\n'deployment.compaction_threshold.denominator'=5\n").expect("suite");
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
        &crate::PriceTable::default(),
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
