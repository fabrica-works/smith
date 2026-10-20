//! Source-specific credential commands (shell.md, sections 4.5–4.6).
//! The edge supplies the environment; dispatch keeps names and redacted outcomes.
use super::{Account, Credential, Directory, Source};
use crate::{command::Command, settings};
use skein_shell::Clock;

mod key;
pub(super) mod program;
pub(super) mod provider;

/// The environment snapshot supplied by main, never rendered with its values.
#[derive(Clone, Default)]
pub struct Environment {
    pub variables: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Environment")
    }
}

/// The named source's terminal after one credential command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginOutcome {
    /// The requested credential state is ready.
    Usable,
    /// The requested credential state could not be reached.
    Unavailable,
    /// A declaration or account identity was refused.
    Refused,
}

impl LoginOutcome {
    /// The product's credential command exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Usable => 0,
            Self::Unavailable => 3,
            Self::Refused => 2,
        }
    }
}

/// Render the account's next action without its access value.
#[must_use]
pub fn credential_words(account: &str, credential: &Credential) -> String {
    credential.line(account)
}

/// Handle login or logout before the run and check front ends.
#[must_use]
pub fn command(arguments: &[std::ffi::OsString], clock: &Clock, environment: &Environment) -> Option<u8> {
    let (shared, name, logout) = match crate::command::parse(arguments.iter().skip(1).cloned().collect()) {
        Ok(Command::Login { shared, account }) => (shared, account, false),
        Ok(Command::Logout { shared, account }) => (shared, account, true),
        Ok(Command::Interactive { .. } | Command::Check { .. } | Command::Exec(_)) | Err(_) => return None,
    };
    Some(match run(&shared, &name, logout, clock, environment) {
        Ok(outcome) => outcome.code(),
        Err(why) => {
            eprintln!("smith: {why}");
            2
        }
    })
}

fn run(
    shared: &crate::command::Shared,
    name: &str,
    logout: bool,
    clock: &Clock,
    environment: &Environment,
) -> Result<LoginOutcome, String> {
    let settings = settings::read_in(shared, &environment.variables).map_err(|why| why.line())?;
    let settings = settings::resolve::checked(&settings).map_err(|why| why.line())?;
    let prefix = format!("accounts.{}.", settings::name(name));
    let fields = settings
        .values
        .iter()
        .filter_map(|(key, value)| key.strip_prefix(&prefix).map(|field| (field.into(), value.clone())))
        .collect();
    let account = Account { name: name.into(), number: 0, fields };
    if !account.fields.contains_key("source") {
        return Err(format!("unknown account {name}"));
    }
    let codex = settings.values.iter().any(|(key, value)| {
        key.starts_with("endpoints.")
            && key.ends_with(".account")
            && value.value.as_str() == Some(name)
            && key.strip_suffix("account").is_some_and(|prefix| {
                ["dialect", "preset"].iter().any(|field| {
                    settings.values.get(&format!("{prefix}{field}")).and_then(|value| value.value.as_str())
                        == Some("codex")
                })
            })
    });
    let directory = Directory::of(&settings);
    let source = super::source(&account.fields, &settings, codex, name, directory.as_ref());
    let registration = if codex { provider::Registration::Codex } else { provider::Registration::Configured };
    if logout {
        return super::logout::run(&account, &source, &settings, registration, clock, environment);
    }
    match source {
        Source::SignIn { file, .. } => {
            provider::run(&account, &file, &settings, registration, provider::Operation::SignIn, clock, environment)
        }
        Source::Key { file } => key::run(name, &file),
        Source::Program { .. } => program::run(name, &settings, false, clock, environment),
        Source::Nothing => {
            eprintln!("account {name}: needs nothing");
            Ok(LoginOutcome::Usable)
        }
        Source::Env { variable, codex } => {
            let wall = clock.now().wall;
            let borrowed = environment
                .variables
                .iter()
                .find(|(key, _)| key.as_os_str() == std::ffi::OsStr::new(&variable))
                .and_then(|(_, value)| super::env_record(value.as_encoded_bytes(), codex, wall));
            let credential = match borrowed {
                Some(value) if value.record.remaining(wall) > skein_lib::Duration::ZERO => {
                    Credential::Usable { until: codex.then_some(value.record.expires_at.as_nanos()) }
                }
                Some(value) => Credential::ExpiredVariable { at: value.record.expires_at.as_nanos(), variable },
                None => Credential::Unset { variable },
            };
            eprintln!("{}", credential_words(name, &credential));
            Ok(if credential.usable() { LoginOutcome::Usable } else { LoginOutcome::Unavailable })
        }
    }
}
