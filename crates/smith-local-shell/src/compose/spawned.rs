//! The generated spawned configuration, written and closed before its launch.
//! Contract: shell.md, section 5.4; protocol/events.md, section 2.
use super::{AgentKind, Refused, ResultFile, root};
use crate::command::Usage;
use skein_lib::Duration;
use skein_shell::{Clock, Kernel};
use std::path::{Path, PathBuf};

pub(super) struct Startup<'a> {
    pub document: Option<Box<[u8]>>,
    pub kind: &'a AgentKind,
    pub chat: &'a Path,
    pub workspace: &'a Path,
    pub trace: Option<&'a smith_agent_shell::trace::TraceConfig>,
    pub exit_grace: Duration,
    pub timeout: Duration,
    pub seed: u64,
}

impl Startup<'_> {
    pub(super) fn launch(self, kernel: &mut Kernel, clock: &Clock) -> Result<smith_local_service::Launch, Refused> {
        let path = self.chat.with_extension("agent.json");
        if let Some(document) = self.document {
            let document = generated(&document, self.trace, self.exit_grace)?;
            ResultFile::write_startup(&path, document, self.timeout, self.seed, kernel, clock)?;
        }
        Ok(self.kind.launch(root(self.workspace)?, &path))
    }
}

/// Add only the spawned child's derived grace and separate trace destination.
fn generated(
    bytes: &[u8],
    trace: Option<&smith_agent_shell::trace::TraceConfig>,
    exit_grace: Duration,
) -> Result<Box<[u8]>, Refused> {
    let mut document: serde_json::Value = serde_json::from_slice(bytes).expect("startup generated JSON");
    document
        .get_mut("profile")
        .expect("generated profile")
        .get_mut("policy")
        .expect("generated policy")
        .as_object_mut()
        .expect("policy object")
        .insert("exit_grace".into(), serde_json::json!(exit_grace.as_nanos() / 1_000_000));
    if let Some(trace) = trace {
        let mut path = trace.path.as_os_str().to_os_string();
        path.push(".agent");
        document
            .get_mut("trace")
            .expect("generated trace")
            .as_object_mut()
            .expect("trace object")
            .insert("path".into(), serde_json::json!(PathBuf::from(path)));
    }
    let bytes = serde_json::to_vec(&document).expect("generated spawned JSON");
    if bytes.len() > 1_048_576 {
        return Err(Refused::Usage(Usage("generated agent configuration exceeds 1 MiB".into())));
    }
    Ok(bytes.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::{Path, generated};

    #[test]
    fn the_generated_child_uses_its_hosts_exit_grace_and_trace_suffix() {
        let source = crate::settings::Source {
            layer: crate::settings::Layer::User, path: "fixture.toml".into(),
            text: "model='codex/gpt-6-luna'\n[accounts.codex]\nsource='env'\n[endpoints.codex]\naddress='127.0.0.1:443'\n[trace]\npath='product.jsonl'\ncapture='calls'\n".into(),
        };
        let settings =
            crate::settings::from_sources(&crate::command::Shared::default(), "/synthetic".into(), &[source]).unwrap();
        let (resolved, bytes) = crate::settings::resolve::resolve_startup(&settings).unwrap();
        let host =
            smith_local_service::profile::host_limits(&resolved.agent.profile, &resolved.agent.service.limits).unwrap();
        let bytes = generated(&bytes, resolved.agent.trace.as_ref(), host.exit_grace)
            .unwrap_or_else(|why| panic!("{}", why.line()));
        let child = smith_agent_shell::config::from_value(serde_json::from_slice(&bytes).unwrap()).unwrap();
        assert_eq!(child.profile.policy.exit_grace, host.exit_grace);
        let trace = child.trace.unwrap();
        assert_eq!(trace.path, Path::new("product.jsonl.agent"));
        assert!(trace.capture == smith_agent_service::sinks::Capture::Calls);
    }
}
