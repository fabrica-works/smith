//! The shell's unrefreshed grants (domain/host.md, section 7; shell.md, 7.1).
//! `Lent` remembers account numbers only; `answer` reads key bytes on demand.
use crate::credentials::{Location, Source};
use skein_lib::{Duration, List};
use smith_domain as agent;
use smith_protocol_channel::GrantValue;

/// One shell answer, sent to the service or translated to a missing credential.
#[derive(Debug)]
pub(super) enum Answer {
    /// The first key or program grant, handed to the service's value map.
    Lend(agent::Grant, GrantValue),
    /// No value to lend, including an account already lent once.
    Missing,
}

/// Numbers already lent by the shell, kept until this invocation ends.
pub(super) struct Lent {
    accounts: List<u32>,
}

impl Lent {
    pub(super) fn with_capacity(accounts: u32) -> Self {
        Self { accounts: List::with_capacity(accounts) }
    }

    pub(super) fn answer(&mut self, locations: &List<Location>, account: u32) -> Answer {
        if self.accounts.iter().any(|lent| *lent == account) {
            return Answer::Missing;
        }
        let Some(location) = locations.iter().find(|location| location.account == account) else {
            return Answer::Missing;
        };
        let value = match &location.source {
            Source::Key { file } => match crate::credentials::key::read(file) {
                Ok(bytes) => GrantValue::Key(bytes),
                Err(_) => return Answer::Missing,
            },
            Source::Program { name } => GrantValue::Program(name.as_bytes().into()),
            Source::Borrow { .. } | Source::Env { .. } | Source::SignIn { .. } | Source::Nothing => {
                return Answer::Missing;
            }
        };
        self.accounts.push(account).expect("one lent entry per selected account");
        Answer::Lend(
            agent::Grant { name: agent::GrantName { account, generation: 1 }, valid: Duration::from_nanos(u64::MAX) },
            value,
        )
    }
}

#[cfg(test)]
mod tests;
