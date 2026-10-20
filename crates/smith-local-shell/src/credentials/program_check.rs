//! Program checks for check and startup (shell.md, 4.4 and 5.3;
//! protocol/programs.md, 3). Keeps redacted states, never a program's login.
//! `check` asks sequentially, with the runner's deadline and service environment.
use super::{Credential, Directory, program_source};
use crate::settings::resolve::Resolved;
use skein_shell::Clock;
use smith_agent_shell::config::ProgramName;
use smith_host_domain::ProgramCheck;
use smith_host_protocol::{SourceAnswer, SourceAsk};
use std::collections::BTreeMap;

pub(crate) type ProgramStates = BTreeMap<String, Credential>;

pub(crate) fn check<'a>(resolved: &Resolved, accounts: impl Iterator<Item = &'a str>, clock: &Clock) -> ProgramStates {
    accounts.map(|account| (account.to_owned(), inspect(resolved, account, clock))).collect()
}

pub(crate) fn needed(resolved: &Resolved, clock: &Clock) -> ProgramStates {
    check(
        resolved,
        resolved.accounts.iter().filter(|account| account.source() == "program").map(|account| account.name.as_str()),
        clock,
    )
}

fn inspect(resolved: &Resolved, account: &str, clock: &Clock) -> Credential {
    let program = ProgramName::ClaudeCode;
    let failed = |detail: &str| Credential::ProgramFailed { program, detail: detail.into() };
    let Some(directory) = Directory::of(&resolved.settings) else {
        return failed("an absolute credentials directory is required");
    };
    // Nothing is kept to ask about; check must not create this directory.
    match std::fs::symlink_metadata(directory.program(account)) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Credential::ProgramNotSignedIn { program },
        Err(_) | Ok(_) => {}
    }
    let settings = &resolved.settings;
    let configured = settings.values.get("programs.claude_code").and_then(|field| field.value.as_str());
    let Ok(executable) = crate::settings::resolve::programs::locate(configured, settings.paths.path.as_deref()) else {
        return failed("program is missing");
    };
    state(
        program,
        program_source::run(
            SourceAsk::Check,
            &executable,
            account,
            &directory,
            clock,
            &resolved.agent.service.environment.command_entries(),
        ),
    )
}

fn state(program: ProgramName, answer: Result<SourceAnswer, String>) -> Credential {
    match answer {
        Ok(SourceAnswer::Check(ProgramCheck::Usable)) => Credential::Usable { until: None },
        Ok(SourceAnswer::Check(ProgramCheck::NotSignedIn)) => Credential::ProgramNotSignedIn { program },
        Ok(SourceAnswer::Check(ProgramCheck::SignedInOtherwise(method))) => {
            Credential::ProgramSignedInOtherwise { program, method: program_source::detail(method.as_bytes()) }
        }
        Ok(SourceAnswer::Check(ProgramCheck::Failed(failure))) => {
            Credential::ProgramFailed { program, detail: program_source::detail(&failure.detail) }
        }
        Err(why) => Credential::ProgramFailed { program, detail: program_source::detail(why.as_bytes()) },
        Ok(SourceAnswer::SignOut(_)) => unreachable!("check answers check"),
    }
}

pub(crate) const fn names(program: ProgramName) -> (&'static str, &'static str) {
    match program {
        ProgramName::ClaudeCode => ("claude_code", "Claude Code"),
    }
}

pub(super) fn serialize<S: serde::Serializer>(
    program: impl std::borrow::Borrow<ProgramName>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(names(*program.borrow()).0)
}

#[cfg(test)]
mod tests;
