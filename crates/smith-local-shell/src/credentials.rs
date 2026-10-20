//! Credential source inspection for startup and check (shell.md, section 7).
//! Reports retain names, locations and expiry only. Login bytes and variable
//! values leave this module only for the service's credential storage.
use crate::settings::{self, Sourced};
use skein_lib::Wall;
use smith_local_protocol::{Borrowed, LoginFormat, decode_login};
use std::{collections::BTreeMap, path::PathBuf};

mod directory;
pub(crate) mod key;
mod login;
mod product;
mod sign_in;
pub use crate::settings::resolve::Account;
pub use directory::Directory;
pub use key::KeyProblem;
pub use login::{LoginOutcome, command, credential_words};
pub(crate) use product::codex as product_codex;
pub use product::{Location, Prepared, limits, prepare};
pub use sign_in::{Registered, registration};

/// An account's state reported without its credential value.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Credential {
    /// An access credential that can be lent until its optional expiry.
    Usable { until: Option<u64> },
    /// A smith-owned account has no saved login.
    NotSignedIn,
    /// The other tool must renew this expired login.
    Expired { at: u64, file: PathBuf, program: String },
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
            | Self::Expired { .. }
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
            Self::NotSignedIn => format!("account {account}: not signed in; run `smith login {account}`"),
            Self::Expired { at, file, program } => format!(
                "the {account} login borrowed from {} expired at {}. Run `{program}` once to refresh it, then try again.",
                file.display(),
                time(*at)
            ),
            Self::Unreadable { file } => format!("account {account}: unreadable login at {}", file.display()),
            Self::KeyUnreadable { problem } => format!("account {account}: key {}", problem.words()),
            Self::Unset { variable } => format!("account {account}: variable {variable} is unset"),
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
    /// The shell opens this directory once; the service loads its file read-only.
    Borrow { file: PathBuf, format: LoginFormat, program: String },
    /// The named variable is read once before the loop.
    Env { variable: String, codex: bool },
    /// The native driver keeps this account in its private state file.
    SignIn { file: PathBuf },
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
            Self::Borrow { .. } => "Source::Borrow",
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
    settings: &settings::Settings,
    codex: bool,
    name: &str,
    directory: Option<&Directory>,
) -> Source {
    let text = |key: &str| fields.get(key).and_then(|field| field.value.as_str()).unwrap_or_default();
    match text("source") {
        "borrow" => {
            let file = text("file");
            let file = if let Some(relative) = file.strip_prefix("~/") {
                settings.paths.home.as_ref().map(PathBuf::from).unwrap_or_default().join(relative)
            } else {
                settings.workspace.join(file)
            };
            Source::Borrow { file, format: LoginFormat::Codex, program: text("program").into() }
        }
        "env" => Source::Env { variable: text("variable").into(), codex },
        "key" => Source::Key {
            file: directory.map_or_else(|| PathBuf::from("missing-key"), |directory| directory.key(name)),
        },
        "program" => Source::Program { name: name.into() },
        "none" => Source::Nothing,
        _ => Source::SignIn {
            file: directory.map_or_else(|| PathBuf::from("missing-token"), |directory| directory.token(name)),
        },
    }
}

/// Read the startup credential once, with clock data supplied by the shell.
#[must_use]
pub fn inspect(source: &Source, wall: Wall) -> Credential {
    match source {
        Source::Borrow { file, format, program } => {
            match skein_shell::read_file(file, crate::local_host::token_limits().document_bytes)
                .ok()
                .and_then(|bytes| decode_login(*format, &bytes, &crate::local_host::token_limits()).ok())
            {
                Some(borrowed) if borrowed.account_id.len() > smith_local_service::Source::account_id_bytes() => {
                    Credential::Unreadable { file: file.clone() }
                }
                Some(borrowed) if borrowed.record.remaining(wall) > skein_lib::Duration::ZERO => {
                    Credential::Usable { until: Some(borrowed.record.expires_at.as_nanos()) }
                }
                Some(borrowed) => Credential::Expired {
                    at: borrowed.record.expires_at.as_nanos(),
                    file: file.clone(),
                    program: program.clone(),
                },
                None => Credential::Unreadable { file: file.clone() },
            }
        }
        Source::Env { variable, codex } => match environment(variable, *codex, wall) {
            Some(value) if value.record.remaining(wall) > skein_lib::Duration::ZERO => {
                Credential::Usable { until: (*codex).then_some(value.record.expires_at.as_nanos()) }
            }
            Some(value) => {
                Credential::ExpiredVariable { at: value.record.expires_at.as_nanos(), variable: variable.clone() }
            }
            None => Credential::Unset { variable: variable.clone() },
        },
        Source::SignIn { file } => sign_in::inspect(file, wall),
        Source::Key { file } => key::inspect(file),
        Source::Program { .. } | Source::Nothing => Credential::Usable { until: None },
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

/// Inspect every catalog account and mark only the selected accounts as required.
#[must_use]
pub fn report(resolved: &settings::resolve::Resolved, wall: Wall) -> BTreeMap<String, AccountReport> {
    catalog(resolved, wall, true)
}

pub(crate) fn unused_warnings(resolved: &settings::resolve::Resolved, wall: Wall) -> Vec<crate::check::Warning> {
    catalog(resolved, wall, false)
        .into_iter()
        .filter_map(|(name, account)| {
            (!account.credential.usable())
                .then_some(crate::check::Warning::Credential { account: name, credential: account.credential })
        })
        .collect()
}

fn catalog(
    resolved: &settings::resolve::Resolved,
    wall: Wall,
    include_required: bool,
) -> BTreeMap<String, AccountReport> {
    let settings = &resolved.settings;
    let directory = Directory::of(settings);
    let mut fields = BTreeMap::<String, BTreeMap<String, Sourced>>::new();
    for (key, value) in &settings.values {
        if let Some(rest) = key.strip_prefix("accounts.") {
            let (account, field) = settings::named_field(rest).expect("validated account key");
            fields.entry(account).or_default().insert(field.into(), value.clone());
        }
    }
    fields
        .into_iter()
        .filter_map(|(account, fields)| {
            let needed = resolved.accounts.iter().find(|needed| needed.name == account);
            if needed.is_some() && !include_required {
                return None;
            }
            let codex = needed.is_some_and(|needed| product::codex(resolved, needed.number));
            let credential = inspect(&source(&fields, settings, codex, &account, directory.as_ref()), wall);
            Some((account, AccountReport { required: needed.is_some(), credential }))
        })
        .collect()
}

/// A catalog account's redacted credential state and the resolver's requirement.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AccountReport {
    pub required: bool,
    #[serde(flatten)]
    pub credential: Credential,
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
