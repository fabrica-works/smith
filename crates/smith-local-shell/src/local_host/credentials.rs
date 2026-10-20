//! The shell's source-specific notices contain names and expiry, never access values.
use super::{Local, domain};
pub(crate) mod programs;
use crate::credentials::{Credential, Source, time};

impl Local {
    pub(super) fn credential_notice(&self, notice: &domain::Notice) -> Option<Box<[u8]>> {
        let (account, reason) = match notice {
            domain::Notice::Failure(domain::Failure::Credential { account, reason }) => (*account, reason),
            domain::Notice::Failure(_)
            | domain::Notice::Warning(_)
            | domain::Notice::Waiting
            | domain::Notice::Checking(_)
            | domain::Notice::NotDelivered { .. }
            | domain::Notice::ChatRefused { .. } => return None,
        };
        let location = self.credential_locations.iter().find(|location| location.account == account)?;
        let expires = match reason {
            domain::CredentialFailure::BorrowedExpiring { at } => Some((*at, "expires")),
            domain::CredentialFailure::Missing
            | domain::CredentialFailure::Refresh
            | domain::CredentialFailure::ProgramNotSignedIn
            | domain::CredentialFailure::ProgramSignedInOtherwise { .. }
            | domain::CredentialFailure::ProgramFailed(_)
            | domain::CredentialFailure::ProgramInterrupted => None,
        };
        let mut line = match &location.source {
            Source::Env { variable, .. } => match expires {
                Some((at, verb)) => {
                    format!("account {}: variable {variable} {verb} at {}", location.name, time(at.as_nanos()))
                }
                None => Credential::Unset { variable: variable.clone() }.line(&location.name),
            },
            Source::SignIn { .. } => Credential::NotSignedIn.line(&location.name),
            Source::Key { .. } => match reason {
                domain::CredentialFailure::Missing => Credential::NotSignedIn.line(&location.name),
                domain::CredentialFailure::ProgramNotSignedIn
                | domain::CredentialFailure::ProgramSignedInOtherwise { .. }
                | domain::CredentialFailure::ProgramFailed(_)
                | domain::CredentialFailure::ProgramInterrupted
                | domain::CredentialFailure::Refresh
                | domain::CredentialFailure::BorrowedExpiring { .. } => {
                    Credential::KeyUnreadable { problem: crate::credentials::KeyProblem::Unreadable }
                        .line(&location.name)
                }
            },
            Source::Program { .. } => program_credential(reason)?.line(&location.name),
            Source::Nothing => return None,
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

fn program_credential(reason: &domain::CredentialFailure) -> Option<Credential> {
    use smith_host_domain::ProgramCheck;
    let answer = match reason {
        domain::CredentialFailure::ProgramNotSignedIn => ProgramCheck::NotSignedIn,
        domain::CredentialFailure::ProgramSignedInOtherwise { method } => ProgramCheck::SignedInOtherwise(*method),
        domain::CredentialFailure::ProgramFailed(failure) => ProgramCheck::Failed(failure.clone()),
        domain::CredentialFailure::ProgramInterrupted => {
            return Some(Credential::ProgramFailed {
                program: smith_agent_shell::config::ProgramName::ClaudeCode,
                detail: "the check was interrupted".into(),
            });
        }
        domain::CredentialFailure::Missing
        | domain::CredentialFailure::Refresh
        | domain::CredentialFailure::BorrowedExpiring { .. } => return None,
    };
    Some(crate::credentials::program_check::credential(smith_agent_shell::config::ProgramName::ClaudeCode, answer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use smith_host_domain::{FailureReason, ProgramFailure, ProgramMethod};

    #[test]
    fn program_notices_name_the_account() {
        for (reason, expected) in [
            (
                domain::CredentialFailure::ProgramNotSignedIn,
                "account work: Claude Code is not signed in: run `smith login work`",
            ),
            (
                domain::CredentialFailure::ProgramSignedInOtherwise { method: ProgramMethod::new(b"api").unwrap() },
                "account work: Claude Code is signed in with api, not a subscription: run `smith login work`",
            ),
            (
                domain::CredentialFailure::ProgramFailed(ProgramFailure {
                    reason: FailureReason::Unreadable,
                    detail: b"bad\x1b\n".as_slice().into(),
                }),
                r"account work: Claude Code could not be checked: bad\x1b\n",
            ),
            (
                domain::CredentialFailure::ProgramInterrupted,
                "account work: Claude Code could not be checked: the check was interrupted",
            ),
        ] {
            assert_eq!(program_credential(&reason).unwrap().line("work"), expected);
        }
    }
}
