//! The shell's source-specific notices contain names and expiry, never access values.
use super::{Local, domain};
use crate::credentials::{Credential, Source, time};

impl Local {
    pub(super) fn credential_notice(&self, notice: &domain::Notice) -> Option<Box<[u8]>> {
        let (account, reason) = match notice {
            domain::Notice::Failure(domain::Failure::Credential { account, reason }) => (*account, *reason),
            domain::Notice::Failure(_)
            | domain::Notice::Warning(_)
            | domain::Notice::Waiting
            | domain::Notice::Checking(_)
            | domain::Notice::NotDelivered { .. }
            | domain::Notice::ChatRefused { .. } => return None,
        };
        let location = self.credential_locations.iter().find(|location| location.account == account)?;
        let expires = match reason {
            domain::CredentialFailure::BorrowedExpiring { at } => Some((at, "expires")),
            domain::CredentialFailure::BorrowedExpired { at } => Some((at, "expired")),
            domain::CredentialFailure::Missing | domain::CredentialFailure::Refresh => None,
        };
        let mut line = match &location.source {
            Source::Borrow { file, program, .. } => match expires {
                Some((at, verb)) => format!(
                    "the {} login borrowed from {} {verb} at {}. Run `{program}` once to refresh it, then try again.",
                    location.name,
                    file.display(),
                    time(at.as_nanos())
                ),
                None => Credential::Unreadable { file: file.clone() }.line(&location.name),
            },
            Source::Env { variable, .. } => match expires {
                Some((at, verb)) => {
                    format!("account {}: variable {variable} {verb} at {}", location.name, time(at.as_nanos()))
                }
                None => Credential::Unset { variable: variable.clone() }.line(&location.name),
            },
            Source::SignIn { .. } => Credential::NotSignedIn.line(&location.name),
        };
        let cap = usize::try_from(self.service.show_bytes().checked_sub(1)?).ok()?;
        if line.len() > cap {
            let end = line.char_indices().map(|(index, _)| index).take_while(|index| *index <= cap).last().unwrap_or(0);
            line.truncate(end);
        }
        line.push('\n');
        Some(line.into_bytes().into())
    }
}
