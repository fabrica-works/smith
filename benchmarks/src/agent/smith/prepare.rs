//! Preparation writes guarded settings and selects access-only credential handling.
//! The arm and append-only face registry choose the codec; user login
//! bytes are read only by credential.rs (benchmarks.md, sections 6.2 and 10).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::agent::{Context, Invocation};
use crate::isolate::{Setup, SetupStep};

use super::{FaceSince, faces, settings};

/// The private native invocation and its immutable outside face.
#[derive(Debug)]
pub struct Prepared {
    pub face: super::events::Shape,
    pub settings: PathBuf,
    pub trace: Option<PathBuf>,
    pub registry: FaceSince,
    pub rendered: serde_json::Value,
    pub program: PathBuf,
    pub state: PathBuf,
    pub teardown: skein_lib::Duration,
    /// Held through settlement for attempts sharing the Claude Code login.
    pub serial: Option<std::fs::File>,
}

fn configuration(reason: impl Into<String>) -> Setup {
    Setup { step: SetupStep::Configuration, reason: reason.into() }
}

/// Read check's version 1 or 2 teardown bound (benchmarks.md, section 10).
/// Both versions use the same effective limits shape.
pub fn teardown_from_check(bytes: &[u8]) -> Result<skein_lib::Duration, Setup> {
    let report: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| configuration(format!("Smith check JSON: {error}")))?;
    if !matches!(report["version"].as_u64(), Some(1 | 2)) {
        return Err(configuration("Smith check document must have version 1 or 2"));
    }
    let limits = report["limits"].as_array().ok_or_else(|| configuration("Smith check needs effective limits"))?;
    let mut teardown = None;
    for limit in limits {
        if limit["name"].as_str() != Some("service.teardown") {
            continue;
        }
        if teardown.is_some() {
            return Err(configuration("Smith check repeats service.teardown"));
        }
        if limit["unit"].as_str() != Some("ms") {
            return Err(configuration("Smith check service.teardown must use milliseconds"));
        }
        let nanos = limit["value"]
            .as_u64()
            .filter(|value| *value > 0)
            .and_then(|value| value.checked_mul(1_000_000))
            .ok_or_else(|| configuration("Smith check service.teardown exceeds its positive duration range"))?;
        teardown = Some(skein_lib::Duration::from_nanos(nanos));
    }
    teardown.ok_or_else(|| configuration("Smith check lacks service.teardown"))
}

fn write(context: &Context<'_>, path: &Path, bytes: &[u8]) -> Result<(), Setup> {
    let result: Result<(), String> = (|| {
        let path = context.guard.write_path(path)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).and_then(|()| file.sync_all()).map_err(|error| error.to_string())
    })();
    result.map_err(configuration)
}

/// Prepare the selected arm, refusing a nonzero product startup check.
pub fn prepare(context: &Context<'_>) -> Result<Prepared, Setup> {
    let (prepared, receipt) = prepare_checked(context, &[])?;
    if let Some(receipt) = receipt {
        return Err(configuration(format!("Smith check refused setup with {:?}", receipt.exit)));
    }
    Ok(prepared)
}

/// Retain a refused check's process receipt for the runner's explicit startup stories.
pub fn prepare_checked(
    context: &Context<'_>,
    overrides: &[String],
) -> Result<(Prepared, Option<crate::process::ProcessResult>), Setup> {
    if context.pin.pin.agent != crate::Agent::Smith || context.pin.pin.version != "built-arm" {
        return Err(configuration("Smith requires a built-arm configuration pin"));
    }
    let build = match context.arm {
        crate::arm::ArmRef::Built(build) => build,
        crate::arm::ArmRef::Installed { .. } => return Err(configuration("Smith requires a verified built arm")),
    };
    let program = build.directory.join("smith");
    let verified = crate::arm::read_build(&program).map_err(configuration)?;
    if verified != *build || context.installed_version != build.arm.commit {
        return Err(configuration("Smith arm differs from its planned full source identity"));
    }
    let pin = crate::read_configuration(context.pin_file).map_err(|error| configuration(error.to_string()))?;
    if pin != *context.pin {
        return Err(configuration("Smith configuration changed since planning"));
    }
    let directory = context.pin_file.parent().ok_or_else(|| configuration("Smith pin needs a directory"))?;
    let repository = directory
        .ancestors()
        .find(|path| path.join(".git").exists())
        .ok_or_else(|| configuration("Smith pin needs its source repository for arm ancestry"))?;
    if let Some(minimum) = &context.pin.pin.minimum_commit {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(["merge-base", "--is-ancestor", minimum, &build.arm.commit])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .map_err(|_| configuration("cannot check the pin's minimum source commit"))?;
        if !status.success() {
            return Err(configuration("Smith arm predates the pin's minimum source commit"));
        }
    }
    let mut registry = faces::face(&build.arm, &faces::read(&directory.join("faces.toml"))?, repository)?;
    if registry.shape == super::events::Shape::Chat && context.variant.is_some_and(|variant| variant.name == "headless")
    {
        registry.shape = super::events::Shape::Exec;
    }
    let bytes = fs::read(directory.join(&pin.pin.configuration)).map_err(|error| configuration(error.to_string()))?;
    if bytes.len() > 65_536 {
        return Err(configuration("Smith settings layer exceeds its bound"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| configuration(error.to_string()))?;
    let base: toml::Value = toml::from_str(text).map_err(|error| configuration(error.to_string()))?;
    let base = serde_json::to_value(base).map_err(|error| configuration(error.to_string()))?;
    let root = context.private.root.join("smith");
    crate::run::private_parent(context.guard, &root).map_err(configuration)?;
    exec(context, &base, program, registry, overrides)
}

fn exec(
    context: &Context<'_>,
    base: &serde_json::Value,
    program: PathBuf,
    registry: FaceSince,
    overrides: &[String],
) -> Result<(Prepared, Option<crate::process::ProcessResult>), Setup> {
    let account = settings::account(context)?;
    crate::credential::logins::smith(context.home, &account)?;
    if account == "claude" {
        let working =
            crate::isolate::working_directory(context.guard, context.private, context.task).map_err(configuration)?;
        crate::credential::logins::claude(context.environment, &working)?;
    }
    let serial =
        if account == "claude" { Some(crate::credential::logins::serial(context.guard, context.home)?) } else { None };
    let login = context.home;
    let mut rendered = settings::product(context, base, login)?;
    let trace = if registry.shape == super::events::Shape::Chat || settings::program(context)? {
        let trace = context.private.root.join("smith/trace.jsonl");
        write(context, &trace, &[])?;
        Some(trace)
    } else {
        None
    };
    chat_trace(&mut rendered, trace.as_deref());
    let directory = context.private.config.join("smith");
    crate::run::private_parent(context.guard, &directory).map_err(configuration)?;
    let settings = directory.join("config.toml");
    let mut layer = rendered.clone();
    layer.as_object_mut().expect("product layer").remove("invocation");
    let document: toml::Value = serde_json::from_value(layer).map_err(|error| configuration(error.to_string()))?;
    write(
        context,
        &settings,
        toml::to_string(&document).map_err(|error| configuration(error.to_string()))?.as_bytes(),
    )?;
    let mut check_arguments = check_arguments(registry.shape, &rendered);
    crate::batch::settings_arguments(&mut check_arguments, overrides);
    let receipt = crate::process::execute(
        &skein_shell::Clock::new(),
        skein_world::end_to_end::Command {
            program: program.clone(),
            arguments: check_arguments,
            environment: context.environment.to_vec(),
            directory: context.private.workspace.clone(),
        },
        Box::default(),
        crate::process::Capture::Raw,
        crate::process::Bounds {
            hard: skein_lib::Duration::from_secs(15),
            teardown: skein_lib::Duration::ZERO,
            grace: skein_lib::Duration::from_secs(1),
            settlement: skein_lib::Duration::from_secs(5),
            stdout: 1_048_576,
            stderr: 65_536,
            force_walk: true,
        },
    )
    .map_err(configuration)?;
    write(context, &directory.join("check.json"), &receipt.stdout)?;
    write(context, &directory.join("check-stderr.log"), &receipt.stderr)?;
    if receipt.error.is_some() || receipt.forced != crate::Forced::No {
        return Err(configuration("Smith check did not settle within its process bounds"));
    }
    let refused = receipt.exit != crate::Exit::Code(0);
    let teardown = if refused { skein_lib::Duration::ZERO } else { teardown_from_check(&receipt.stdout)? };
    let prepared = Prepared {
        face: registry.shape,
        registry,
        rendered,
        program,
        settings,
        state: context.private.state.clone(),
        serial,
        trace,
        teardown,
    };
    Ok((prepared, refused.then_some(receipt)))
}

fn chat_trace(rendered: &mut serde_json::Value, path: Option<&Path>) {
    if let Some(path) = path {
        rendered["trace"]["path"] = serde_json::json!(path);
    }
}

fn product_arguments(rendered: &serde_json::Value) -> Vec<std::ffi::OsString> {
    rendered["invocation"]
        .as_array()
        .expect("prepared invocation")
        .iter()
        .map(|value| value.as_str().expect("prepared product argument").into())
        .collect()
}

fn check_arguments(face: super::events::Shape, rendered: &serde_json::Value) -> Vec<std::ffi::OsString> {
    let mut arguments = vec!["check".into(), "--json".into()];
    match face {
        super::events::Shape::Exec => {
            arguments.extend(["exec".into(), "--json".into()]);
            arguments.extend(product_arguments(rendered));
            arguments.push("-".into());
        }
        super::events::Shape::Chat => arguments.extend(product_arguments(rendered)),
    }
    arguments
}

fn exec_arguments(rendered: &serde_json::Value) -> Vec<std::ffi::OsString> {
    let mut arguments = vec!["exec".into(), "--json".into()];
    arguments.extend(product_arguments(rendered));
    arguments.push("-".into());
    arguments
}

impl Prepared {
    /// Program accounts emit channel facts to a separate agent trace.
    #[must_use]
    pub fn program_source(&self) -> bool {
        self.rendered["endpoints"].as_object().is_some_and(|endpoints| {
            endpoints.values().any(|endpoint| {
                endpoint["account"]
                    .as_str()
                    .is_some_and(|account| self.rendered["accounts"][account]["source"] == "program")
            })
        })
    }

    /// The product check's retained setup document for the selected invocation.
    #[must_use]
    pub fn check_document(&self) -> Option<PathBuf> {
        Some(self.settings.with_file_name("check.json"))
    }

    /// Invoke the frozen native local front end with its own private state.
    #[must_use]
    pub fn command(&self) -> Invocation {
        if self.face == super::events::Shape::Chat {
            let mut arguments = product_arguments(&self.rendered);
            arguments.extend(["--trace".into(), self.trace.as_ref().expect("Chat trace").as_os_str().into()]);
            return Invocation {
                program: self.program.clone(),
                arguments,
                environment: Vec::new(),
                mode: skein_world::end_to_end::Mode::Terminal,
                teardown: self.teardown,
            };
        }
        Invocation {
            program: self.program.clone(),
            arguments: exec_arguments(&self.rendered),
            environment: Vec::new(),
            mode: skein_world::end_to_end::Mode::Pipes,
            teardown: self.teardown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Prepared, chat_trace, check_arguments, teardown_from_check};
    use crate::agent::smith::{FaceSince, Receiving, events::Shape};
    use crate::isolate::SetupStep;
    use serde_json::json;

    fn report(limits: &serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&json!({"version":1,"limits":limits})).expect("check fixture")
    }

    fn invocation(face: Shape) -> Prepared {
        let mut prepared = Prepared {
            face,
            settings: "/scratch/config/smith/config.toml".into(),
            trace: (face == Shape::Chat).then(|| "/scratch/smith/trace.jsonl".into()),
            registry: FaceSince {
                commit: "0".repeat(40),
                shape: face,
                receiving: Receiving { string: 64, content: 4096, items: 32, encoded_line: 65536 },
            },
            rendered: json!({"trace":{"capture":"everything"},"invocation":["-C","/scratch/workspace","-m","codex/gpt-6-luna","--effort","low"]}),
            program: "/arm/smith".into(),
            state: "/scratch/state".into(),
            serial: None,
            teardown: skein_lib::Duration::from_secs(10),
        };
        chat_trace(&mut prepared.rendered, prepared.trace.as_deref());
        prepared
    }

    #[test]
    fn the_chat_face_types_at_a_terminal_and_reads_its_private_trace() {
        let prepared = invocation(Shape::Chat);
        assert_eq!(prepared.rendered["trace"]["capture"], "everything");
        assert_eq!(prepared.rendered["trace"]["path"], "/scratch/smith/trace.jsonl");
        let command = prepared.command();
        assert!(matches!(command.mode, skein_world::end_to_end::Mode::Terminal));
        let arguments: Vec<_> =
            command.arguments.iter().map(|argument| argument.to_str().expect("UTF-8 fixture")).collect();
        assert_eq!(
            arguments,
            [
                "-C",
                "/scratch/workspace",
                "-m",
                "codex/gpt-6-luna",
                "--effort",
                "low",
                "--trace",
                "/scratch/smith/trace.jsonl"
            ]
        );
        assert_eq!(prepared.check_document().expect("product check"), prepared.settings.with_file_name("check.json"));
        assert_eq!(command.teardown, prepared.teardown);
    }

    #[test]
    fn the_headless_face_keeps_exec_json_on_pipes() {
        let prepared = invocation(Shape::Exec);
        let command = prepared.command();
        assert!(matches!(command.mode, skein_world::end_to_end::Mode::Pipes));
        let arguments: Vec<_> =
            command.arguments.iter().map(|argument| argument.to_str().expect("UTF-8 fixture")).collect();
        assert_eq!(
            arguments,
            ["exec", "--json", "-C", "/scratch/workspace", "-m", "codex/gpt-6-luna", "--effort", "low", "-"]
        );
        assert!(prepared.trace.is_none());
        assert!(prepared.rendered["trace"].get("path").is_none());
    }

    #[test]
    fn check_names_the_invocation_each_face_starts() {
        for face in [Shape::Exec, Shape::Chat] {
            let prepared = invocation(face);
            let command = smith_local_shell::command::parse(check_arguments(face, &prepared.rendered))
                .expect("product check flags");
            let smith_local_shell::command::Command::Check { json, execution, .. } = command else {
                panic!("product startup check");
            };
            assert!(json, "{face:?}");
            match face {
                Shape::Exec => assert!(execution.expect("headless check's exec invocation").json),
                Shape::Chat => assert!(execution.is_none(), "interactive check has no exec invocation"),
            }
        }
    }

    #[test]
    fn check_teardown_reads_the_named_bound_in_milliseconds() {
        let bytes = report(&json!([
            {"name":"policy.close","value":999,"unit":"ms"},
            {"name":"service.teardown","value":12345,"unit":"ms"}
        ]));
        assert_eq!(teardown_from_check(&bytes).expect("effective teardown").as_nanos(), 12_345_000_000);
    }

    #[test]
    fn a_version_2_document_gives_its_teardown() {
        let bytes = br#"{"version":2,"limits":[{"name":"service.teardown","value":12345,"unit":"ms"}]}"#;
        assert_eq!(
            teardown_from_check(bytes).expect("version 2 teardown").as_nanos(),
            12_345_000_000,
            "version 2 retains the milliseconds bound"
        );
    }

    #[test]
    fn a_version_3_document_is_refused() {
        let bytes = br#"{"version":3,"limits":[{"name":"service.teardown","value":12345,"unit":"ms"}]}"#;
        assert!(teardown_from_check(bytes).is_err(), "an unknown version cannot supply a bound");
    }

    #[test]
    fn check_teardown_refuses_a_missing_bound() {
        let refused = teardown_from_check(&report(&json!([]))).expect_err("bound required");
        assert_eq!(refused.step, SetupStep::Configuration);
    }

    #[test]
    fn check_teardown_refuses_a_repeated_bound() {
        let bound = json!({"name":"service.teardown","value":12345,"unit":"ms"});
        assert!(teardown_from_check(&report(&json!([bound, bound]))).is_err());
    }

    #[test]
    fn check_teardown_refuses_another_unit() {
        let bound = json!({"name":"service.teardown","value":12,"unit":"seconds"});
        assert!(teardown_from_check(&report(&json!([bound]))).is_err());
    }

    #[test]
    fn check_teardown_refuses_values_outside_its_positive_duration_range() {
        for value in [json!(0), json!(-1), json!(1.5), json!("12"), json!(u64::MAX)] {
            let bound = json!({"name":"service.teardown","value":value,"unit":"ms"});
            assert!(teardown_from_check(&report(&json!([bound]))).is_err());
        }
    }

    #[test]
    fn check_teardown_refuses_another_version_or_a_malformed_document() {
        for bytes in [b"{}".as_slice(), b"{", br#"{"version":3,"limits":[]}"#, br#"{"version":1}"#] {
            assert!(teardown_from_check(bytes).is_err());
        }
    }
}
