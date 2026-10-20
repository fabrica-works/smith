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
    // Inspect both levels without creating or following a source directory.
    for path in [directory.programs(), directory.program(account)] {
        match smith_agent_shell::program::private_directory(&path) {
            Ok(false) => return Credential::ProgramNotSignedIn { program },
            Ok(true) => {}
            Err(why) => return failed(&program_source::detail(why.to_string().as_bytes())),
        }
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
        Ok(SourceAnswer::Check(answer)) => credential(program, answer),
        Err(why) => Credential::ProgramFailed { program, detail: program_source::detail(why.as_bytes()) },
        Ok(SourceAnswer::SignOut(_)) => unreachable!("check answers check"),
    }
}

pub(crate) fn credential(program: ProgramName, answer: ProgramCheck) -> Credential {
    match answer {
        ProgramCheck::Usable => Credential::Usable { until: None },
        ProgramCheck::NotSignedIn => Credential::ProgramNotSignedIn { program },
        ProgramCheck::SignedInOtherwise(method) => {
            Credential::ProgramSignedInOtherwise { program, method: program_source::detail(method.as_bytes()) }
        }
        ProgramCheck::Failed(failure) => {
            Credential::ProgramFailed { program, detail: program_source::detail(&failure.detail) }
        }
    }
}

pub(crate) const fn key(program: ProgramName) -> &'static str {
    match program {
        ProgramName::ClaudeCode => "claude_code",
    }
}

pub(super) const fn shown(program: ProgramName) -> &'static str {
    match program {
        ProgramName::ClaudeCode => "Claude Code",
    }
}

pub(super) fn serialize<S: serde::Serializer>(
    program: impl std::borrow::Borrow<ProgramName>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(key(*program.borrow()))
}

#[cfg(test)]
mod tests;
