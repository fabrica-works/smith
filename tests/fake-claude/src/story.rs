//! The test's scripts and recorded invocation names (protocol/programs.md,
//! section 8; testing.md, section 4); no environment values are retained.

use serde::{Deserialize, Serialize};

/// What a test scripts; defaults to a truthful subscription login and logout.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Story {
    pub status: Status,
    pub login: Login,
    pub logout: Logout,
}

/// A status answer selected by the story, ending on output or a signal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Reports the login file's method, or not signed in when absent.
    #[default]
    Truthful,
    /// Reports bytes that do not decode as JSON, then exits successfully.
    Unreadable,
    /// Waits without output until a termination signal ends the invocation.
    Silent,
}

/// A sign-in flow selected by the story, ending on a saved login or refusal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Login {
    /// Saves the specified method without asking for input.
    Immediate(String),
    /// Saves the method after a nonempty pasted line, refusing empty input or EOF.
    Code(String),
    /// Refuses sign-in and keeps the previous login.
    Refused,
}

impl Default for Login {
    fn default() -> Self {
        Self::Immediate("claude.ai".into())
    }
}

/// A sign-out answer selected by the story, ending with removal or failure.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Logout {
    /// Removes a login, accepting an already absent file.
    #[default]
    Succeeds,
    /// Refuses removal and keeps the login.
    Fails,
}

impl Story {
    /// The bytes a test writes to `fake-claude.story`.
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("story contains only JSON data")
    }
}

/// One invocation as the fake saw it, recorded without environment values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// The invocation's arguments, excluding the program name.
    pub arguments: Vec<String>,
    /// The invocation's environment variable names, without their values.
    pub environment: Vec<String>,
}

/// Reads the newline-separated records in `fake-claude.observations`.
/// Malformed records fail the test that reads them.
pub fn observations(bytes: &[u8]) -> Vec<Observation> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("fake observation record"))
        .collect()
}
