//! Credential paths under one base (protocol/hosts.md, 5.4; shell.md, 7.2).
//! `of` and `default_in` take the caller's environment; no files are opened.
use crate::settings::Settings;
use std::{ffi::OsStr, path::PathBuf};

/// The shell's absolute credentials base, without any credential bytes.
#[derive(Clone)]
pub struct Directory {
    base: PathBuf,
}

impl std::fmt::Debug for Directory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Directory")
    }
}

impl Directory {
    /// Resolve the setting before the handed-in state and home defaults.
    #[must_use]
    pub fn of(settings: &Settings) -> Option<Self> {
        match settings.values.get("credentials") {
            Some(value) => Self::from_base(PathBuf::from(value.value.as_str()?)),
            None => Self::default_in(settings.paths.state_home.as_deref(), settings.paths.home.as_deref()),
        }
    }

    /// Prefer an absolute state home, then an absolute home's local state.
    #[must_use]
    pub fn default_in(state_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<Self> {
        state_home
            .and_then(|base| Self::from_base(PathBuf::from(base).join("smith")))
            .or_else(|| home.and_then(|base| Self::from_base(PathBuf::from(base).join(".local/state/smith"))))
    }

    /// Admit only an absolute base.
    #[must_use]
    pub fn from_base(base: PathBuf) -> Option<Self> {
        base.is_absolute().then_some(Self { base })
    }

    /// The private key file for a validated account name.
    #[must_use]
    pub fn key(&self, account: &str) -> PathBuf {
        self.base.join("keys").join(account)
    }

    /// The current private sign-in record name.
    #[must_use]
    pub fn token(&self, account: &str) -> PathBuf {
        self.base.join("tokens").join(account)
    }

    /// The permanent directory whose exclusive lock guards the account record.
    #[must_use]
    pub fn lock(&self, account: &str) -> PathBuf {
        self.base.join("tokens").join(format!("{account}.lock"))
    }

    /// The installation identifier created only by a provider sign-in.
    #[must_use]
    pub fn host_id(&self) -> PathBuf {
        self.base.join("host-id")
    }

    /// The directory containing program source configurations.
    #[must_use]
    pub fn programs(&self) -> PathBuf {
        self.base.join("programs")
    }

    /// One validated account's program configuration directory.
    #[must_use]
    pub fn program(&self, account: &str) -> PathBuf {
        self.programs().join(account)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod layout_tests;
