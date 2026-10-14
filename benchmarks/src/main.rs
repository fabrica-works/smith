//! Offline checks, frozen arm builds and prepared attempts under shared supervision.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
        [command, request] if command == "attempt" => attempt_command(Path::new(request)),
        [command, rest @ ..] if matches!(command.as_str(), "probe" | "calibrate" | "fix" | "suite") => {
            match smith_bench::probe::command(command, rest) {
                Ok(verdict) => ExitCode::from(verdict.code()),
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(2)
                }
            }
        }
        [command, commit] if command == "build" => build_command(commit, None),
        [command, commit, flag, repository] if command == "build" && flag == "--repository" => {
            build_command(commit, Some(Path::new(repository)))
        }
        [command] if command == "check" => check_all(),
        [command, directory] if command == "check" => check(Path::new(directory)),
        [command, summary, flag, commit] if command == "parity" && flag == "--commit" => {
            parity_command(Path::new(summary), commit)
        }
        [command, names @ ..] if command == "validate" => validate_command(names),
        [command, results, suite, tier, design, seed, reference @ ..]
            if command == "summarise" && reference.len() <= 1 =>
        {
            summarize_command(Path::new(results), suite, tier, design, seed, reference.first().map(Path::new))
        }
        [command, summary, name] if command == "baseline" => baseline_command(Path::new(summary), name, None),
        [command, summary, name, flag, agent, configuration, model, effort]
            if command == "baseline" && flag == "--group" =>
        {
            baseline_command(Path::new(summary), name, Some((agent, configuration, model, effort)))
        }
        [command, flag, suite, tasks_flag, directory, sections @ ..]
            if command == "guards" && flag == "--suite" && tasks_flag == "--tasks" && !sections.is_empty() =>
        {
            guards(Path::new(suite), Some(Path::new(directory)), sections)
        }
        [command, flag, suite, sections @ ..] if command == "guards" && flag == "--suite" && !sections.is_empty() => {
            guards(Path::new(suite), None, sections)
        }
        _ => {
            eprintln!(
                "usage: smith-bench check [TASKS_DIRECTORY]\n       smith-bench validate [TASK]...\n       smith-bench guards --suite SUITE [--tasks TASKS_DIRECTORY] SECTION...\n       smith-bench summarise RESULTS SUITE TIER DESIGN SEED [BASELINE]\n       smith-bench parity SUMMARY --commit SHA\n       smith-bench baseline SUMMARY NAME [--group AGENT CONFIGURATION_SHA256 MODEL EFFORT]\n       smith-bench build COMMIT [--repository REPOSITORY]\n       smith-bench attempt PREPARED_REQUEST\n       smith-bench probe PROBE [--agent smith|codex] [--attempts N] [--commit COMMIT | --arm DIRECTORY]\n       smith-bench calibrate PROBE [--attempts N] [--passes N] [--control COMMIT] [--commit COMMIT | --arm DIRECTORY]\n       smith-bench fix PROBE --parent COMMIT [--commit COMMIT]\n       smith-bench suite NAME [--seed N] [--commit COMMIT | --arm DIRECTORY]"
            );
            ExitCode::from(2)
        }
    }
}

fn build_command(commit: &str, repository: Option<&Path>) -> ExitCode {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("benchmark workspace");
    report_output(smith_bench::arm::build(repository.unwrap_or(source), commit).map(|built| {
        println!("built {} sha256 {}", built.arm.commit, built.arm.sha256);
        built.directory
    }))
}

fn parity_command(file: &Path, commit: &str) -> ExitCode {
    let result = smith_bench::read_summary(file)
        .map_err(|error| error.to_string())
        .and_then(|summary| smith_bench::parity::verify(&summary, commit).map(|parity| (summary, parity)));
    match result {
        Ok((summary, parity)) => {
            for criterion in parity.criteria {
                let lower_bound = summary.attempts.iter().any(|row| {
                    let repository = row.task == "repository/shell-cwd";
                    row.agent == "codex"
                        && match criterion.name.as_str() {
                            "FT" | "RT" => {
                                repository == (criterion.name == "RT")
                                    && matches!(row.task_wall_ms, smith_bench::Measure::LowerBound { .. })
                            }
                            "FC" | "RC" => {
                                repository == (criterion.name == "RC")
                                    && matches!(row.priced_usd, smith_bench::Measure::LowerBound { .. })
                            }
                            _ => false,
                        }
                });
                println!(
                    "{} statistic={} interval=[{}, {}] margin={} holds={} codex_lower_bound={}",
                    criterion.name,
                    criterion.statistic,
                    criterion.interval.0,
                    criterion.interval.1,
                    criterion.margin,
                    criterion.holds,
                    lower_bound
                );
            }
            ExitCode::from(parity.verdict.code())
        }
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::from(2)
        }
    }
}

fn committed_output(directory: &str, name: &str) -> Result<PathBuf, String> {
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')) {
        return Err("committed output name must use letters, digits, '-' or '_'".into());
    }
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(directory);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join(format!("{name}.json")))
}

fn report_output(result: Result<PathBuf, String>) -> ExitCode {
    match result {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn summarize_command(
    results: &Path,
    suite: &str,
    tier: &str,
    design: &str,
    seed: &str,
    reference: Option<&Path>,
) -> ExitCode {
    report_output((|| {
        let design = match design {
            "single" => smith_bench::Design::Single,
            "interleaved" => smith_bench::Design::Interleaved,
            _ => return Err("design must be single or interleaved".into()),
        };
        let seed = seed.parse::<u64>().map_err(|error| error.to_string())?;
        let results = smith_bench::read_results(results).map_err(|error| error.to_string())?;
        let reference = reference.map(smith_bench::read_baseline).transpose().map_err(|error| error.to_string())?;
        let reference_arm = if design == smith_bench::Design::Interleaved {
            let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("suites").join(format!("{suite}.toml"));
            smith_bench::read_suite(&file).map_err(|error| error.to_string())?.reference
        } else {
            None
        };
        let summary =
            smith_bench::summarise(&results, suite, tier, design, seed, reference.as_ref(), reference_arm.as_deref())?;
        let output = committed_output("summaries", &summary.run)?;
        smith_bench::write_summary(&output, &summary)?;
        Ok(output)
    })())
}

fn baseline_command(summary: &Path, name: &str, group: Option<(&str, &str, &str, &str)>) -> ExitCode {
    report_output((|| {
        let summary = smith_bench::read_summary(summary).map_err(|error| error.to_string())?;
        let reference = match group {
            None => smith_bench::baseline(&summary)?,
            Some((agent, configuration, model, effort)) => {
                let agent = serde_json::from_value(serde_json::Value::String(agent.into()))
                    .map_err(|error| error.to_string())?;
                smith_bench::baseline_group(&summary, agent, configuration, model, effort)?
            }
        };
        let output = committed_output("baselines", name)?;
        smith_bench::write_summary(&output, &reference)?;
        Ok(output)
    })())
}

fn check_all() -> ExitCode {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match smith_bench::check_benchmark_tree(&root, &root.join("../docs/design")) {
        Ok(counts) => {
            println!(
                "checked {} tasks, {} suites, {} configuration pins, {} summaries, {} baselines",
                counts.tasks, counts.suites, counts.configurations, counts.summaries, counts.baselines
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn guards(file: &Path, tasks: Option<&Path>, sections: &[String]) -> ExitCode {
    match guarded_tasks(file, tasks, sections) {
        Ok(selection) => {
            for name in selection.selected {
                println!("{name}");
            }
            for (name, reason) in selection.omitted {
                println!("omitted {name}: {reason}");
            }
            println!("estimated {} seconds, {} tokens", selection.estimated_seconds, selection.estimated_tokens);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn guarded_tasks(
    file: &Path,
    tasks_directory: Option<&Path>,
    sections: &[String],
) -> Result<smith_bench::GuardSelection, smith_bench::Refusal> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite = smith_bench::read_suite(file)?;
    let default_tasks = root.join("tasks");
    let tasks = smith_bench::catalogue(tasks_directory.unwrap_or(&default_tasks), &root.join("../docs/design"))?;
    let agents = root.join("agents");
    let models = smith_bench::read_model_tiers(&agents.join("models.toml"))?;
    smith_bench::validate_suite(file, &suite, &tasks, &agents, &models)?;
    let summaries_directory = root.join("summaries");
    let mut summaries = Vec::new();
    if summaries_directory.exists() {
        for entry in std::fs::read_dir(&summaries_directory)
            .map_err(|error| smith_bench::Refusal::new(&summaries_directory, "summaries", error.to_string()))?
        {
            let entry = entry
                .map_err(|error| smith_bench::Refusal::new(&summaries_directory, "summaries", error.to_string()))?;
            if entry.path().extension().is_some_and(|extension| extension == "json") {
                summaries.push(smith_bench::read_summary(&entry.path())?);
            }
        }
    }
    let costs = smith_bench::committed_costs(&suite, &tasks, &summaries, &agents, &models)
        .map_err(|error| smith_bench::Refusal::new(file, "summaries", error))?;
    Ok(smith_bench::choose_guards_with_costs(&suite, &tasks, sections, &costs))
}

fn check(directory: &Path) -> ExitCode {
    let design = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/design");
    match smith_bench::check_tree(directory, &design) {
        Ok(count) => {
            println!("checked {count} tasks");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn attempt_command(request: &Path) -> ExitCode {
    match smith_bench::run::attempt_file(request) {
        Ok(recorded) => {
            println!("{}", recorded.directory.join("result.json").display());
            ExitCode::from(smith_bench::judge(&recorded.result).code())
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn validate_command(names: &[String]) -> ExitCode {
    let tasks = Path::new(env!("CARGO_MANIFEST_DIR")).join("tasks");
    match smith_bench::validate::validate(&tasks, names) {
        Ok(results) => {
            for result in &results {
                println!("{} seed={} reference={}", result.task, grade_end(&result.seed), grade_end(&result.reference));
            }
            ExitCode::from(u8::from(!results.iter().all(smith_bench::validate::Validation::passed)))
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn grade_end(end: &smith_bench::grade::GradeEnd) -> String {
    match end {
        smith_bench::grade::GradeEnd::Passed => "passed".into(),
        smith_bench::grade::GradeEnd::Failed => "failed".into(),
        smith_bench::grade::GradeEnd::Fault { reason } => format!("fault:{reason}"),
    }
}
