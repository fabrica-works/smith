//! Grant-specific inputs shared by process scenarios.
use super::World;
impl World {
    /// Select an exact tagged value for the Start.
    pub fn granting(&mut self, value: smith_channel::GrantValue) {
        self.grant = Some(value);
    }

    /// Select the public `OpenAI` endpoint for entrance checks.
    pub fn openai(&mut self) {
        self.openai = true;
    }

    /// Send a later tagged grant after the host observes admission.
    pub fn refreshing(&mut self, value: smith_channel::GrantValue) {
        self.refreshing = Some(value);
    }

    pub(super) fn grant_value(&self) -> smith_channel::GrantValue {
        self.grant.clone().unwrap_or_else(|| {
            let value = if self.private_grant { crate::PRIVATE_GRANT } else { b"token".as_slice() };
            if self.anthropic {
                crate::bearer(value)
            } else {
                let id = if self.private_grant { b"native-private-account-02.6".as_slice() } else { b"acc".as_slice() };
                crate::withdrawn(id, value)
            }
        })
    }

    pub(super) fn credential(&self) -> skein_llm::Credential {
        match &self.grant {
            Some(smith_channel::GrantValue::Credential(value)) => match value.scheme() {
                smith_channel::Scheme::Bearer => skein_llm::Credential::bearer(Box::from(value.bytes().expose())),
                smith_channel::Scheme::Key => skein_llm::Credential::key(Box::from(value.bytes().expose())),
            },
            Some(smith_channel::GrantValue::Withdrawn(value)) => {
                skein_llm::Credential::codex(Box::from(value.bearer().expose()), Box::from(value.id()))
            }
            Some(smith_channel::GrantValue::Program(_)) | None => {
                crate::scenario_credential(self.anthropic, self.private_grant)
            }
        }
    }
}
