//! Hand the terminal to program sign-in, then ask the host half (shell.md, 4.5–4.6).
use super::super::{Directory, program_source};
use super::{Environment, LoginOutcome};
use crate::settings::Settings;
use program_source::detail;
use skein_shell::Clock;
use smith_host_domain::{ProgramCheck, ProgramSignOut};
use smith_host_protocol::{SourceAnswer, SourceAsk};

pub(in crate::credentials) fn run(
    name: &str,
    settings: &Settings,
    logout: bool,
    clock: &Clock,
    environment: &Environment,
) -> Result<LoginOutcome, String> {
    let directory = Directory::of(settings).ok_or("an absolute credentials directory is required")?;
    let configured = settings.values.get("programs.claude_code").and_then(|value| value.value.as_str());
    let Ok(program) = crate::settings::resolve::programs::locate(configured, settings.paths.path.as_deref()) else {
        eprintln!("account {name}: program is missing");
        return Ok(LoginOutcome::Unavailable);
    };
    let configuration =
        smith_agent_shell::program::prepare(&directory.programs(), name).map_err(|why| why.to_string())?;
    if !logout {
        let result =
            smith_agent_shell::program::sign_in(&program, &configuration, environment.variables.clone()).status();
        if !result.is_ok_and(|status| status.success()) {
            eprintln!("account {name}: program sign-in failed");
            return Ok(LoginOutcome::Unavailable);
        }
    }
    let base: Vec<Box<[u8]>> = environment
        .variables
        .iter()
        .map(|(name, value)| {
            let mut entry = name.as_encoded_bytes().to_vec();
            entry.push(b'=');
            entry.extend_from_slice(value.as_encoded_bytes());
            entry.into()
        })
        .collect();
    if logout {
        match program_source::run(SourceAsk::SignOut, &program, name, &directory, clock, &base)? {
            SourceAnswer::SignOut(ProgramSignOut::SignedOut) => {}
            SourceAnswer::SignOut(ProgramSignOut::Failed(failure)) => {
                eprintln!("account {name}: program sign-out failed: {}", detail(&failure.detail));
            }
            SourceAnswer::Check(_) => unreachable!("sign-out answers sign-out"),
        }
    }
    match program_source::run(SourceAsk::Check, &program, name, &directory, clock, &base)? {
        SourceAnswer::Check(ProgramCheck::Usable) => {
            eprintln!("account {name}: usable");
            Ok(if logout { LoginOutcome::Unavailable } else { LoginOutcome::Usable })
        }
        SourceAnswer::Check(ProgramCheck::NotSignedIn) => {
            eprintln!("account {name}: not signed in");
            Ok(if logout { LoginOutcome::Usable } else { LoginOutcome::Unavailable })
        }
        SourceAnswer::Check(ProgramCheck::SignedInOtherwise(method)) => {
            eprintln!("account {name}: signed in otherwise ({})", String::from_utf8_lossy(method.as_bytes()));
            Ok(LoginOutcome::Unavailable)
        }
        SourceAnswer::Check(ProgramCheck::Failed(failure)) => {
            eprintln!("account {name}: program check failed: {}", detail(&failure.detail));
            Ok(LoginOutcome::Unavailable)
        }
        SourceAnswer::SignOut(_) => unreachable!("check answers check"),
    }
}
