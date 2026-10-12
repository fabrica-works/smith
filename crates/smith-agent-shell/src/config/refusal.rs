//! Startup's typed refusals, retained until the product adds value origins.
//! Legacy JSON callers use `detail`; no setting or credential is retained here.
//! Contract: shell.md, section 5.3; protocol/limits.md, sections 4 and 8.
use smith_agent_service::profile;

/// A configuration input failure or a named relationship with both values.
#[derive(Debug)]
pub enum ConfigurationRefusal {
    /// An input or destination could not be decoded; carries its diagnostic.
    Input(String),
    /// Derivation refused a named field or relationship, retaining its values.
    Profile(profile::Refusal),
}

impl ConfigurationRefusal {
    /// Preserve the legacy JSON entrance's diagnostic until that face is removed.
    #[must_use]
    pub fn detail(self) -> String {
        match self {
            Self::Input(message) => message,
            Self::Profile(why) => super::profile_refusal(why),
        }
    }
}

impl From<String> for ConfigurationRefusal {
    fn from(message: String) -> Self {
        Self::Input(message)
    }
}

impl From<&str> for ConfigurationRefusal {
    fn from(message: &str) -> Self {
        Self::Input(message.into())
    }
}
