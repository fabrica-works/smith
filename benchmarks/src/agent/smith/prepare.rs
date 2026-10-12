//! Preparation writes guarded native settings and a converted access-only record.
//! The verified arm and append-only face registry choose the codec; user login
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
    pub credential: Credential,
    pub trace: Option<PathBuf>,
    pub registry: FaceSince,
    pub rendered: serde_json::Value,
    pub program: PathBuf,
    pub state: PathBuf,
    pub teardown: skein_lib::Duration,
}

/// The attempt's credential, removed only after the process tree settles.
#[derive(Debug)]
pub enum Credential {
    /// A private converted record with no usable refresh grant.
    Record(Option<crate::credential::Borrowed>),
}

fn configuration(reason: impl Into<String>) -> Setup {
    Setup { step: SetupStep::Configuration, reason: reason.into() }
}

/// Read the effective teardown bound from check's version 1 document (benchmarks.md, section 10).
pub fn teardown_from_check(bytes: &[u8]) -> Result<skein_lib::Duration, Setup> {
    let report: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| configuration(format!("Smith check JSON: {error}")))?;
    if report["version"].as_u64() != Some(1) {
        return Err(configuration("Smith check document must have version 1"));
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

/// Render the selected native arm before borrowing its existing source login.
pub fn prepare(context: &Context<'_>) -> Result<Prepared, Setup> {
    if context.pin.pin.agent != crate::Agent::Smith
        || context.pin.pin.provider != crate::Provider::Codex
        || context.pin.pin.version != "built-arm"
    {
        return Err(configuration("Smith requires the codex built-arm configuration pin"));
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
    let registry = faces::face(&build.arm, &faces::read(&directory.join("faces.toml"))?, repository)?;
    match registry.shape {
        super::events::Shape::LocalTrace | super::events::Shape::LocalAgentTrace => {}
        super::events::Shape::Exec | super::events::Shape::Chat => {
            return Err(configuration("this event front end awaits its shipped product invocation"));
        }
    }
    let bytes = fs::read(directory.join(&pin.pin.configuration)).map_err(|error| configuration(error.to_string()))?;
    if bytes.len() > 65_536 {
        return Err(configuration("Smith settings template exceeds its bound"));
    }
    let base = serde_json::from_slice(&bytes).map_err(|error| configuration(error.to_string()))?;
    let root = context.private.root.join("smith");
    crate::run::private_parent(context.guard, &root).map_err(configuration)?;
    let tokens = root.join("tokens");
    let trace = root.join("trace.jsonl");
    let settings_path = root.join("settings.json");
    let dry = settings::settings(context, &base, "prepared-account", &trace, &tokens)?;
    let teardown = settings::teardown(&dry)?;
    let lifetime =
        context.lifetime.checked_add(teardown).ok_or_else(|| configuration("credential lifetime overflow"))?;
    let converted = crate::credential::smith(context.guard, context.home, &tokens, context.wall, lifetime)?;
    let rendered = settings::settings(context, &base, &converted.account_id, &trace, &tokens)?;
    let encoded = serde_json::to_vec_pretty(&rendered).map_err(|error| configuration(error.to_string()))?;
    write(context, &settings_path, &encoded)?;
    write(context, &trace, &[])?;
    let state = root.join("state");
    crate::run::private_parent(context.guard, &state).map_err(configuration)?;
    Ok(Prepared {
        face: registry.shape,
        settings: settings_path,
        credential: Credential::Record(Some(converted.borrowed)),
        trace: Some(trace),
        registry,
        rendered,
        program,
        state,
        teardown,
    })
}

impl Prepared {
    /// Invoke the frozen native local front end with its own private state.
    #[must_use]
    pub fn command(&self) -> Invocation {
        Invocation {
            program: self.program.clone(),
            arguments: vec![
                "local".into(),
                self.settings.clone().into_os_string(),
                self.state.clone().into_os_string(),
            ],
            environment: Vec::new(),
            mode: skein_world::end_to_end::Mode::Terminal,
            teardown: self.teardown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::teardown_from_check;
    use crate::isolate::SetupStep;
    use serde_json::json;

    fn report(limits: &serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&json!({"version":1,"limits":limits})).expect("check fixture")
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
        for bytes in [b"{}".as_slice(), b"{", br#"{"version":2,"limits":[]}"#, br#"{"version":1}"#] {
            assert!(teardown_from_check(bytes).is_err());
        }
    }
}
