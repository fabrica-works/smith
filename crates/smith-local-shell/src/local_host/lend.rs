//! The shell's unrefreshed grants (domain/host.md, section 7; shell.md, 7.1).
//! `Lent` keeps startup values and their last lent generations, not key paths.
//! `answer` lends from memory only; it never reads a key renewed on disk.
use skein_lib::{Duration, Map};
use smith_domain as agent;
use smith_protocol_channel::GrantValue;

/// One shell answer, sent to the service or translated to a missing credential.
#[derive(Debug)]
pub(super) enum Answer {
    /// A fresh key or program grant, handed to the service's value map.
    Lend(agent::Grant, GrantValue),
    /// No value to lend, or its generation cannot advance.
    Missing,
}

/// Startup values and per-account generations, kept until this invocation ends.
#[derive(Debug)]
pub(super) struct Lent {
    accounts: Map<u32, (u64, GrantValue)>,
}

impl Lent {
    pub(super) fn new(accounts: Map<u32, (u64, GrantValue)>) -> Self {
        Self { accounts }
    }

    pub(super) fn answer(&mut self, account: u32) -> Answer {
        let Some((generation, value)) = self.accounts.get_mut(&account) else {
            return Answer::Missing;
        };
        let Some(next) = generation.checked_add(1) else {
            return Answer::Missing;
        };
        *generation = next;
        Answer::Lend(
            agent::Grant {
                name: agent::GrantName { account, generation: next },
                valid: Duration::from_nanos(u64::MAX),
                renewable: match value {
                    GrantValue::Key(_) => false,
                    GrantValue::Bearer(_) | GrantValue::Program(_) | GrantValue::Withdrawn { .. } => true,
                },
            },
            value.clone(),
        )
    }
}

#[cfg(test)]
mod tests;
