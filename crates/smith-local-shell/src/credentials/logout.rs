//! Source-specific removal (shell.md, 4.6). Holds no credential values.
use super::{Account, Environment, LoginOutcome, Source};
use crate::settings::Settings;
use skein_shell::Clock;

pub(super) fn run(
    account: &Account,
    source: &Source,
    settings: &Settings,
    registration: super::login::provider::Registration,
    clock: &Clock,
    environment: &Environment,
) -> Result<LoginOutcome, String> {
    let name = &account.name;
    match source {
        Source::SignIn { file, .. } => super::login::provider::run(
            account,
            file,
            settings,
            registration,
            super::login::provider::Operation::SignOut,
            clock,
            environment,
        ),
        Source::Key { file } => {
            remove_key(file)?;
            eprintln!("account {name}: key removed");
            Ok(LoginOutcome::Usable)
        }
        Source::Program { .. } => super::login::program::run(name, settings, true, clock, environment),
        Source::Env { .. } | Source::Nothing => {
            eprintln!("account {name}: smith holds nothing");
            Ok(LoginOutcome::Usable)
        }
    }
}

pub(super) fn remove_key(file: &std::path::Path) -> Result<(), String> {
    match std::fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("cannot remove private key".into()),
    }
}
