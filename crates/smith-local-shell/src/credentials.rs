//! Credential source inspection for startup and check (shell.md, section 7).
//! Reports retain names, locations and expiry only. Login bytes and variable
//! values leave this module only for the service's credential storage.
use crate::settings::{self, Sourced};
use skein_lib::Wall;
use smith_local_protocol::Borrowed;
use std::{collections::BTreeMap, path::PathBuf};

pub(crate) mod account_report;
mod directory;
mod host_id;
pub(crate) mod key;
mod login;
mod logout;
mod private_file;
mod product;
pub(crate) mod program_check;
pub(crate) mod program_source;
mod registration;
pub(crate) mod sign_in;
pub use crate::settings::resolve::Account;
pub(crate) use account_report::unused_warnings;
pub use account_report::{AccountReport, Kept, report};
pub use directory::Directory;
pub use key::KeyProblem;
pub use login::{Environment, LoginOutcome, command, credential_words};
pub use product::{Location, Prepared, limits, prepare, record_generation};
pub(crate) use product::{codex as product_codex, status_words};
use program_check::ProgramStates;
pub use sign_in::{Registered, registration};
use smith_agent_shell::config::ProgramName;

/// An account's state reported without its credential value.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Credential {
    /// An access credential that can be lent until its optional expiry.
    Usable { until: Option<u64> },
    /// A smith-owned account has no saved login.
    NotSignedIn,
    /// Startup and check require the account to sign in again before a run.
    SignInAgain,
    /// Startup and check found a saved sign-in without a required scope.
    WithoutScope { scope: String },
    /// The program told the shell its source is not signed in.
    ProgramNotSignedIn {
        #[serde(serialize_with = "program_check::serialize")]
        program: ProgramName,
    },
    /// The program told the shell its source uses another payment method.
    ProgramSignedInOtherwise {
        #[serde(serialize_with = "program_check::serialize")]
        program: ProgramName,
        method: String,
    },
    /// The shell could not establish the program source's state.
    ProgramFailed {
        #[serde(serialize_with = "program_check::serialize")]
        program: ProgramName,
        detail: String,
    },
    /// The named login cannot be read or decoded.
    Unreadable { file: PathBuf },
    /// A private key failed inspection; startup reports only its fixed problem class.
    KeyUnreadable { problem: KeyProblem },
    /// The named environment variable supplies no credential.
    Unset { variable: String },
    /// A named variable holds an access token past its stated expiry.
    ExpiredVariable { at: u64, variable: String },
}

impl Credential {
    /// Whether this account can serve a run at the check's injected wall time.
    #[must_use]
    pub const fn usable(&self) -> bool {
        match self {
            Self::Usable { .. } => true,
            Self::NotSignedIn
            | Self::SignInAgain
            | Self::WithoutScope { .. }
            | Self::ProgramNotSignedIn { .. }
            | Self::ProgramSignedInOtherwise { .. }
            | Self::ProgramFailed { .. }
            | Self::Unreadable { .. }
            | Self::KeyUnreadable { .. }
            | Self::Unset { .. }
            | Self::ExpiredVariable { .. } => false,
        }
    }

    /// The person's next action, without any part of the access value.
    #[must_use]
    pub fn line(&self, account: &str) -> String {
        match self {
            Self::Usable { until: Some(until) } => format!("account {account}: usable until {until}"),
            Self::Usable { until: None } => format!("account {account}: usable"),
            Self::NotSignedIn => format!("account {account} has no credential: run `smith login {account}`"),
            Self::SignInAgain => format!("account {account} must sign in again: run `smith login {account}`"),
            Self::WithoutScope { scope } => format!(
                "account {account} was signed in without {scope}: run `smith login {account}` again and allow it, or use a key"
            ),
            Self::ProgramNotSignedIn { program } => format!(
                "account {account}: {} is not signed in: run `smith login {account}`",
                program_check::shown(*program)
            ),
            Self::ProgramSignedInOtherwise { program, method } => format!(
                "account {account}: {} is signed in with {method}, not a subscription: run `smith login {account}`",
                program_check::shown(*program)
            ),
            Self::ProgramFailed { program, detail } => {
                format!("account {account}: {} could not be checked: {detail}", program_check::shown(*program))
            }
            Self::Unreadable { file } => format!("account {account}: unreadable login at {}", file.display()),
            Self::KeyUnreadable { problem } => format!("account {account}: key {}", problem.words()),
            Self::Unset { variable } => format!("account {account}: {variable} is not set"),
            Self::ExpiredVariable { at, variable } => {
                format!("account {account}: variable {variable} expired at {}", time(*at))
            }
        }
    }
}

pub(crate) fn time(nanos: u64) -> String {
    let minutes = nanos.checked_div(60_000_000_000).expect("minute divisor");
    let hours = minutes.checked_div(60).expect("hour divisor").checked_rem(24).expect("day divisor");
    format!("{hours:02}:{:02}", minutes.checked_rem(60).expect("hour divisor"))
}

/// Account descriptors for a selected run; this preparation reads no sign-in source.
#[derive(Clone)]
pub enum Source {
    /// The named variable is read once before the loop.
    Env { variable: String, codex: bool },
    /// The native driver keeps this account in its private state file.
    SignIn {
        file: PathBuf,
        /// Scopes required by the selected registration, absent for an explicit client.
        required_scopes: Option<Box<[u8]>>,
    },
    /// A private key lent once by the shell, never refreshed (domain/host.md, 7).
    Key { file: PathBuf },
    /// The program owns this account's login; the shell lends its name (domain/host.md, 7).
    Program { name: String },
    /// This account lends nothing; its models name no account (domain/host.md, 7).
    Nothing,
}

impl std::fmt::Debug for Source {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Env { .. } => "Source::Env",
            Self::SignIn { .. } => "Source::SignIn",
            Self::Key { .. } => "Source::Key",
            Self::Program { .. } => "Source::Program",
            Self::Nothing => "Source::Nothing",
        })
    }
}

/// Resolve one account descriptor without exposing a variable's value.
#[must_use]
pub fn source(
    fields: &BTreeMap<String, Sourced>,
    _settings: &settings::Settings,
    codex: bool,
    name: &str,
    directory: Option<&Directory>,
) -> Source {
    let text = |key: &str| fields.get(key).and_then(|field| field.value.as_str()).unwrap_or_default();
    match text("source") {
        "env" => Source::Env { variable: text("variable").into(), codex },
        "key" => Source::Key {
            file: directory.map_or_else(|| PathBuf::from("missing-key"), |directory| directory.key(name)),
        },
        "program" => Source::Program { name: name.into() },
        "none" => Source::Nothing,
        _ => {
            let required_scopes = if text("provider") == "openai" {
                registration::openai("", None, &registration::Moved::default())
                    .terms
                    .and_then(|terms| terms.required_scopes)
            } else {
                None
            };
            Source::SignIn {
                required_scopes,
                file: directory.map_or_else(|| PathBuf::from("missing-token"), |directory| directory.token(name)),
            }
        }
    }
}

/// Read the startup credential once, with clock data supplied by the shell.
#[must_use]
pub fn inspect(source: &Source, wall: Wall, programs: &ProgramStates) -> Credential {
    match source {
        Source::Env { variable, codex } => match environment(variable, *codex, wall) {
            Some(value) if value.record.remaining(wall) > skein_lib::Duration::ZERO => {
                Credential::Usable { until: (*codex).then_some(value.record.expires_at.as_nanos()) }
            }
            Some(value) => {
                Credential::ExpiredVariable { at: value.record.expires_at.as_nanos(), variable: variable.clone() }
            }
            None => Credential::Unset { variable: variable.clone() },
        },
        Source::SignIn { file, required_scopes } => sign_in::inspect(file, wall, required_scopes.as_deref()),
        Source::Key { file } => key::inspect(file),
        Source::Program { name } => programs.get(name).cloned().unwrap_or_else(|| Credential::ProgramFailed {
            program: ProgramName::ClaudeCode,
            detail: "not checked".into(),
        }),
        Source::Nothing => Credential::Usable { until: None },
    }
}

/// Read a named variable into one access-only record; no variable is revisited.
#[must_use]
pub fn environment(variable: &str, codex: bool, wall: Wall) -> Option<Borrowed> {
    let value = std::env::var_os(variable)?;
    env_record(value.as_encoded_bytes(), codex, wall)
}

/// Decode one handed-in variable value, with the endpoint's dialect supplied by resolution.
#[must_use]
pub fn env_record(value: &[u8], codex: bool, wall: Wall) -> Option<Borrowed> {
    if !valid_key(value) {
        return None;
    }
    let (account_id, expires_at) = if codex {
        let selector = skein_oauth::ClaimSelector {
            object: b"https://api.openai.com/auth".as_slice().into(),
            field: b"chatgpt_account_id".as_slice().into(),
        };
        let claims = skein_oauth::read_claims(value, &selector, wall, &crate::local_host::token_limits()).ok()?;
        if claims.metadata.is_empty() || claims.metadata.len() > smith_local_service::Source::account_id_bytes() {
            return None;
        }
        (claims.metadata, claims.expires_at.unwrap_or(Wall::from_nanos(u64::MAX)))
    } else {
        let empty: Box<[u8]> = Box::new([]);
        (empty, Wall::from_nanos(u64::MAX))
    };
    Some(Borrowed {
        record: skein_oauth::SavedToken {
            issued_client_id: None,
            granted_scopes: None,
            subject: None,
            id_token: None,
            earliest_refresh_at: None,
            key: 0,
            generation: 1,
            access_token: value.into(),
            refresh_token: None,
            metadata: None,
            expires_at,
        },
        account_id,
    })
}

pub(crate) fn valid_key(value: &[u8]) -> bool {
    !value.is_empty()
        && u64::try_from(value.len()).expect("key length fits u64")
            <= u64::from(crate::local_host::token_limits().token_bytes)
        && value.iter().all(|byte| (0x21..=0x7e).contains(byte))
}

/// Why credential preparation cannot compose the selected run.
#[derive(Debug)]
pub enum Refusal {
    /// A startup configuration check refused its named setting.
    Configuration(settings::Refusal),
    /// A selected account cannot supply a usable access credential.
    Credentials { account: String, credential: Credential },
}

impl Refusal {
    /// Configuration and credentials keep their separate startup exit codes.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Configuration(_) => 2,
            Self::Credentials { .. } => 3,
        }
    }

    /// The source's next action, without any access value.
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            Self::Configuration(why) => why.line(),
            Self::Credentials { account, credential } => format!("smith: {}", credential.line(account)),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod state_tests;
