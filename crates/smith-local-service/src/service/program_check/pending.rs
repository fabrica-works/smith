//! Cancel only the credential prefix preceding Interrupt or Drop (domain/host.md,
//! section 8). Keeps counts, never credential values; service routing consumes them.
use super::{Queue, local};
use crate::Service;

impl Service {
    pub(crate) fn interrupt_program_requests(&mut self) {
        let mut credentials = 0_u32;
        for request in &self.local_requests {
            match request {
                local::Request::Credential { .. } => {
                    credentials = credentials.checked_add(1).expect("bounded request queue");
                }
                local::Request::Input { .. }
                | local::Request::Show { .. }
                | local::Request::Notice { .. }
                | local::Request::Load
                | local::Request::SelectChat { .. }
                | local::Request::SaveState { .. }
                | local::Request::SaveTurn { .. }
                | local::Request::SaveDelivery { .. }
                | local::Request::Git { .. }
                | local::Request::PlainStatus { .. }
                | local::Request::Lower { .. }
                | local::Request::Process(_)
                | local::Request::Answered { .. }
                | local::Request::Unanswered { .. }
                | local::Request::Exit { .. } => {}
            }
        }
        self.interrupted_credentials = credentials;
        self.below.process_mut().cancel_programs();
    }

    pub(crate) fn interrupted_program_request(&mut self, account: u32) -> bool {
        if self.interrupted_credentials == 0 {
            return false;
        }
        self.interrupted_credentials = self.interrupted_credentials.checked_sub(1).expect("interrupted prefix");
        if self.below.process_mut().program_account(account) {
            self.local_events
                .push(local::Event::NoCredential { account, reason: local::CredentialFailure::ProgramInterrupted });
            return true;
        }
        false
    }

    pub(crate) fn drain_program_interruptions(&mut self) -> bool {
        let notify = !self.closing();
        self.below.process_mut().drain_program_interruptions(&mut self.local_events, notify)
    }
}

impl crate::process::ProcessAdapter {
    fn program_account(&self, account: u32) -> bool {
        match &self.programs {
            Some(checks) => checks.account_index(account).is_some(),
            None => false,
        }
    }

    fn drain_program_interruptions(&mut self, events: &mut Queue<local::Event>, notify: bool) -> bool {
        match &mut self.programs {
            Some(checks) => checks.drain_interrupted(events, notify),
            None => true,
        }
    }
}
