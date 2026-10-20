//! Validate hidden graders on private seeds and reference overlays, without an
//! agent or credentials (benchmarks.md, sections 3.1, 5.2 and 10).

use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use skein_lib::Duration;
use skein_shell::Clock;

use crate::CatalogueTask;
use crate::grade::{Entry, GradeEnd, Snapshot};
use crate::guard::Guard;
use crate::isolate::{Machine, Private};
use crate::process::Bounds;

/// A task's grader terminals, supplied to the validation command.
#[derive(Debug)]
pub struct Validation {
    pub task: String,
    pub seed: GradeEnd,
    pub reference: GradeEnd,
}

impl Validation {
    /// A useful grader rejects its seed and accepts its reference solution.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.seed == GradeEnd::Failed && self.reference == GradeEnd::Passed
    }
}

/// Check the credentials probe against accepted, unchanged and refused pair controls.
/// No login or provider is used by these controls.
pub fn credentials(tasks: &Path) -> Result<(), String> {
    let file = tasks.join("probes/credentials/task.toml");
    let design = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/design");
    let task = crate::check_task(&file, &design).map_err(|error| error.to_string())?;
    crate::batch::validate_credentials(&task)
}

/// Run named graded tasks, or every graded task, in private command homes.
pub fn validate(tasks: &Path, names: &[String]) -> Result<Vec<Validation>, String> {
    let design = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/design");
    let catalogue = crate::catalogue(tasks, &design).map_err(|error| error.to_string())?;
    let guard = Guard::from_environment()?;
    let machine = Machine::from_environment()?;
    let state = crate::arm::state_root(&guard)?;
    let scratch = state.join("scratch");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(guard.write_path(&scratch)?)
        .map_err(|error| error.to_string())?;
    let root = crate::isolate::create_directory(&guard, &scratch.join(format!("validate-{}", std::process::id())))?;
    let result = validate_catalogue(&catalogue, names, &guard, &machine, &root);
    crate::isolate::remove_owned_tree(&guard, &root)?;
    result
}

fn validate_catalogue(
    tasks: &[CatalogueTask],
    names: &[String],
    guard: &Guard,
    machine: &Machine,
    root: &Path,
) -> Result<Vec<Validation>, String> {
    for name in names {
        if !tasks.iter().any(|entry| entry.name == *name && !entry.task.grade.is_empty()) {
            return Err(format!("unknown graded task {name}"));
        }
    }
    let clock = Clock::new();
    let mut results = Vec::new();
    for (index, entry) in tasks.iter().enumerate() {
        if entry.task.grade.is_empty() || (!names.is_empty() && !names.contains(&entry.name)) {
            continue;
        }
        let seed = candidate(&clock, guard, machine, entry, &root.join(format!("{index}-seed")), false)?;
        let reference = candidate(&clock, guard, machine, entry, &root.join(format!("{index}-reference")), true)?;
        results.push(Validation { task: entry.name.clone(), seed, reference });
    }
    Ok(results)
}

fn overlay(guard: &Guard, source: &Path, destination: &Path) -> Result<(), String> {
    for (relative, entry) in crate::grade::snapshot_all(source)? {
        let target = guard.write_path(&destination.join(&relative))?;
        match entry {
            Entry::Directory => {
                if !target.exists() {
                    crate::isolate::create_directory(guard, &target)?;
                }
            }
            Entry::File { .. } => {
                fs::copy(source.join(relative), target).map_err(|error| error.to_string())?;
            }
            Entry::Link { .. } | Entry::Special => return Err("reference contains a link or special node".into()),
        }
    }
    Ok(())
}

fn candidate(
    clock: &Clock,
    guard: &Guard,
    machine: &Machine,
    entry: &CatalogueTask,
    root: &Path,
    reference: bool,
) -> Result<GradeEnd, String> {
    let task_root = entry.manifest.parent().ok_or("task has no directory")?;
    let bundles = crate::arm::state_root(guard)?.join("bundles");
    let (seed, digest) = crate::seed_source(&entry.task, task_root, &bundles).map_err(|refusal| refusal.reason)?;
    let private: Private = crate::isolate::prepare(guard, &seed, &digest, root).map_err(|error| error.reason)?;
    let before = crate::grade::snapshot(&private.workspace)?;
    if reference {
        let reference = task_root.join("reference");
        if reference.join("solution.patch").is_file() {
            apply_patch(clock, guard, machine, &private, &reference.join("solution.patch"))?;
        } else {
            overlay(guard, &reference, &private.workspace)?;
        }
    }
    let grader = task_root.join("grader");
    if grader.exists() {
        crate::isolate::copy_directory(guard, &grader, &root.join("grader"))?;
    }
    let bounds = Bounds {
        hard: Duration::ZERO,
        teardown: Duration::ZERO,
        grace: Duration::from_secs(1),
        settlement: Duration::from_secs(3),
        stdout: 0,
        stderr: 0,
        force_walk: true,
    };
    let mut end = GradeEnd::Passed;
    for command in &entry.task.setup {
        let setup = crate::Grade {
            command: command.clone(),
            deadline_seconds: entry.task.deadline_seconds,
            max_output_bytes: 4 * 1024 * 1024,
        };
        let environment = &entry.task.environment;
        let result = crate::grade::command(clock, machine, &private, &private.workspace, &setup, bounds, environment)?;
        if result.end != GradeEnd::Passed {
            return Err(format!(
                "validation setup failed: {:?}: {}",
                result.end,
                String::from_utf8_lossy(&result.stderr)
            ));
        }
    }
    for (index, grade) in entry.task.grade.iter().enumerate() {
        let result = crate::grade::command(
            clock,
            machine,
            &private,
            &private.workspace,
            grade,
            bounds,
            &entry.task.environment,
        )?;
        if entry.task.kind == crate::Kind::Repository {
            eprintln!(
                "{} {} grade {index}:\n{}{}",
                entry.name,
                if reference { "reference" } else { "seed" },
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        if reference && result.end != GradeEnd::Passed {
            eprintln!(
                "{} reference grade {index}:\n{}{}",
                entry.name,
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        match result.end {
            GradeEnd::Failed if end == GradeEnd::Passed => end = GradeEnd::Failed,
            GradeEnd::Passed | GradeEnd::Failed => {}
            GradeEnd::Fault { reason } => end = GradeEnd::Fault { reason },
        }
    }
    if reference {
        reference_checks(&entry.task, &private.workspace, &before).map_err(|refusal| refusal.to_string())?;
    }
    Ok(end)
}

fn reference_checks(task: &crate::Task, settled: &Path, before: &Snapshot) -> Result<(), crate::Refusal> {
    let after = crate::grade::snapshot(settled).map_err(|reason| crate::Refusal::new(settled, "reference", reason))?;
    for path in &task.protected {
        if crate::grade::changed(before, &after, path) {
            return Err(crate::Refusal::new(settled, "protected", format!("reference changes {}", path.display())));
        }
    }
    let ended = crate::grade::Ended {
        end: crate::End::Completed,
        exit: crate::Measure::Observed { value: crate::Exit::Code(0) },
        last_line: None,
        answer: None,
        credential_refresh: crate::Measure::Unavailable { reason: "reference runs without credentials".into() },
    };
    for check in crate::grade::checks(settled, before, &after, &task.outcome, &ended) {
        if check.passed != (crate::Measure::Observed { value: true }) {
            return Err(crate::Refusal::new(settled, "outcome", format!("reference fails {}", check.check)));
        }
    }
    Ok(())
}

fn apply_patch(clock: &Clock, guard: &Guard, machine: &Machine, private: &Private, patch: &Path) -> Result<(), String> {
    let target = private.root.join("solution.patch");
    fs::copy(patch, guard.write_path(&target)?).map_err(|error| error.to_string())?;
    let grade = crate::Grade {
        command: vec!["git".into(), "apply".into(), "../solution.patch".into()],
        deadline_seconds: 10,
        max_output_bytes: 1024 * 1024,
    };
    let bounds = Bounds {
        hard: Duration::ZERO,
        teardown: Duration::ZERO,
        grace: Duration::from_secs(1),
        settlement: Duration::from_secs(3),
        stdout: 0,
        stderr: 0,
        force_walk: true,
    };
    let result = crate::grade::command(
        clock,
        machine,
        private,
        &private.workspace,
        &grade,
        bounds,
        &std::collections::BTreeMap::default(),
    )?;
    if result.end != GradeEnd::Passed {
        return Err(format!("reference patch failed: {}", String::from_utf8_lossy(&result.stderr)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str, solved: bool) -> Validation {
        let root = std::env::temp_dir().join(format!("smith-bench-validate-{name}-{}", std::process::id()));
        fs::create_dir(&root).expect("fresh root");
        let task_root = root.join("task");
        fs::create_dir_all(task_root.join("seed")).expect("seed");
        fs::create_dir(task_root.join("reference")).expect("reference");
        if solved {
            fs::write(task_root.join("seed/solved"), "solution").expect("solution marker");
        }
        let mut task =
            crate::read_task(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/tasks/valid/task.toml"))
                .expect("recorded task");
        task.seed_sha256 = crate::seed_digest(&task_root.join("seed")).expect("seed digest");
        task.grade = vec![crate::Grade {
            command: vec!["sh".into(), "-c".into(), "test -f solved".into()],
            deadline_seconds: 1,
            max_output_bytes: 1024,
        }];
        let guard = Guard::new(&root.join("home"), &root.join("config"), &root.join("state"), &[]).expect("guard");
        let machine = Machine { path: "/usr/bin:/bin".into(), rustup_home: None };
        let entry = CatalogueTask { name: name.into(), manifest: task_root.join("task.toml"), task };
        let mut results = validate_catalogue(&[entry], &[], &guard, &machine, &root).expect("validation");
        fs::remove_dir_all(root).expect("remove fixture");
        results.pop().expect("one task")
    }

    #[test]
    fn validate_reports_a_seed_its_grader_passes() {
        let result = fixture("passing-seed", true);
        assert_eq!(result.seed, GradeEnd::Passed);
        assert!(!result.passed());
    }

    #[test]
    fn validate_reports_a_reference_its_grader_fails() {
        let result = fixture("failing-reference", false);
        assert_eq!(result.seed, GradeEnd::Failed);
        assert_eq!(result.reference, GradeEnd::Failed);
        assert!(!result.passed());
    }
}
