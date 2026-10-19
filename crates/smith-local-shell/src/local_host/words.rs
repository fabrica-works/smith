//! Operator translation uses the failure's own account and the host's answer clock.
//! Display names come from the existing credential locations and endpoint table;
//! earlier notices, provider addresses and event records are never consulted.
use super::{Local, domain};
use smith_host_domain::{ModelFault, RunFailure};

impl Local {
    pub(super) fn run_notice(&self, notice: &domain::Notice) -> Option<Box<[u8]>> {
        if let Some(text) = self.service.effort_notice(notice) {
            return Some(text);
        }
        let (failure, detail) = match notice {
            domain::Notice::Failure(domain::Failure::Run { failure, detail }) => (failure, detail),
            domain::Notice::Failure(_)
            | domain::Notice::Warning(_)
            | domain::Notice::Waiting
            | domain::Notice::Checking(_)
            | domain::Notice::NotDelivered { .. }
            | domain::Notice::ChatRefused { .. } => return None,
        };
        let model = match failure {
            RunFailure::Model(ModelFault::Completion { model, .. }) => {
                self.service.models().get(usize::try_from(*model).expect("bounded model index"))
            }
            RunFailure::Model(_)
            | RunFailure::Budget { .. }
            | RunFailure::Policy(_)
            | RunFailure::Transcript(_)
            | RunFailure::Cancelled
            | RunFailure::Stale => None,
        };
        let account = match failure {
            RunFailure::Model(
                ModelFault::Limited { account, .. }
                | ModelFault::Refused { account, .. }
                | ModelFault::Unauthorized { account },
            ) => Some(*account),
            RunFailure::Model(_)
            | RunFailure::Budget { .. }
            | RunFailure::Policy(_)
            | RunFailure::Transcript(_)
            | RunFailure::Cancelled
            | RunFailure::Stale => model.and_then(|model| model.account),
        };
        let name = account.and_then(|account| {
            self.credential_locations
                .iter()
                .find(|location| location.account == account)
                .map(|location| location.name.as_bytes())
        });
        Some(smith_local_protocol::run_failure_text(
            failure,
            detail,
            self.service.models(),
            self.service.show_bytes(),
            name,
            model.and_then(|model| self.service.endpoint_name(model.endpoint.0)),
            self.service.answer_wall(),
        ))
    }
}
