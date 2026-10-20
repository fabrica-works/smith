//! Product placement from the main account; no credentials or child state are kept.
//! `program_account` inspects the selection; the binary lookup opens no program (shell.md, section 9).
use super::{AgentKind, Refused};
use crate::{settings, startup};
use std::path::Path;

pub(crate) fn program_account(accounts: &[settings::resolve::Account]) -> bool {
    accounts.first().expect("main account is selected first").source() == "program"
}

/// Price the child's validated teardown before selecting the host's grace.
/// Profile admission includes a final event write even without a destination
/// (protocol/limits.md, sections 3.6 and 3.8; shell.md, section 9).
pub(crate) fn host_limits(
    agent: &smith_agent_shell::config::Configuration,
    spawned: bool,
) -> Result<smith_host_domain::Limits, smith_agent_service::profile::Refusal> {
    let mut child = agent.service.limits;
    if spawned {
        child.events = Some(
            smith_agent_service::EventLimits::derive(
                &child,
                smith_agent_service::sinks::Capture::None,
                1,
                0,
                &[],
                agent.profile.policy.write_deadline,
            )
            .ok_or(smith_agent_service::profile::Refusal::Derived {
                limit: smith_agent_service::profile::LimitName::deployment(
                    smith_agent_service::profile::Layer::Service,
                    "teardown",
                ),
            })?,
        );
    }
    smith_local_service::profile::host_limits(&agent.profile, &child)
}

/// Bound the local host's IO separately from the spawned agent's native effects.
/// One agent uses four entities, one git child three, input and signals two,
/// and trace and JSON two. A further slot covers the shell writer.
/// Contract: shell.md, section 9; domain/host.md, section 8.
pub(crate) fn service_limits(
    local: smith_local_domain::Limits,
    host: smith_host_domain::Limits,
    agent: &smith_agent_service::Config,
) -> Result<smith_local_service::Limits, String> {
    const HOST_ENTITIES: u32 = 12;
    let mut limits = startup::service_limits(local, host, agent)?;
    match limits.local.agent {
        smith_local_domain::AgentLimits::Inline(_) => {}
        smith_local_domain::AgentLimits::Spawned(_) => limits.process.io.sockets = HOST_ENTITIES,
    }
    Ok(limits)
}

pub(super) fn beside_current_exe() -> Result<AgentKind, Refused> {
    let executable = std::env::current_exe()
        .map_err(|why| missing(Path::new("<current_exe>/../smith-agent"), Some(&why.to_string())))?;
    beside(executable.parent().unwrap_or(Path::new("")))
}

fn beside(directory: &Path) -> Result<AgentKind, Refused> {
    let program = directory.join("smith-agent");
    if !program.is_file() {
        return Err(missing(&program, None));
    }
    Ok(AgentKind::Spawned { program })
}

fn missing(path: &Path, reason: Option<&str>) -> Refused {
    let mut detail = format!("smith-agent not found beside smith: {}", path.display());
    if let Some(reason) = reason {
        detail.push_str(" (");
        detail.push_str(reason);
        detail.push(')');
    }
    Refused::Local(startup::Failure::Host { detail })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_main_accounts_source_selects_placement() {
        for (source, spawned) in
            [("program", true), ("key", false), ("env", false), ("sign_in", false), ("none", false)]
        {
            let settings = settings::merge(
                &crate::command::Shared::default(),
                "/workspace".into(),
                &[settings::Source {
                    layer: settings::Layer::User,
                    path: "placement.toml".into(),
                    text: format!("model='anthropic/claude-opus-5-5'\n[accounts.anthropic]\nsource='{source}'\n[endpoints.anthropic]\nprogram='claude_code'\n"),
                }],
            )
            .unwrap_or_else(|why| panic!("{}", why.line()));
            assert_eq!(
                program_account(
                    &settings::resolve::select(&settings).unwrap_or_else(|why| panic!("{}", why.line())).accounts
                ),
                spawned,
                "source {source}"
            );
        }
    }

    #[test]
    fn a_missing_agent_names_the_sibling_path() {
        let scratch = smith_real_world::Scratch::new();
        let refusal = beside(scratch.path()).err().expect("no sibling binary");
        assert_eq!(refusal.code(), 1);
        assert_eq!(
            refusal.line(),
            format!("smith: smith-agent not found beside smith: {}", scratch.path().join("smith-agent").display())
        );
    }
}
