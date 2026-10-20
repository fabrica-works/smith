//! The test's scripts and recorded invocation names (protocol/programs.md,
//! sections 5 and 8; testing.md, section 4); no environment values are retained.

use serde::{Deserialize, Serialize};

/// What a test scripts; defaults to a truthful subscription login and logout.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Story {
    pub status: Status,
    pub login: Login,
    pub logout: Logout,
    #[serde(default)]
    pub init: Init,
    #[serde(default)]
    pub turns: Vec<Turn>,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default)]
    pub resume: Resume,
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
    /// Refuses removal with the supplied stderr detail.
    Detail(String),
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
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).expect("fake observation record"))
        .filter(|value| value.get("seen").is_none())
        .map(|value| serde_json::from_value(value).expect("invocation observation"))
        .collect()
}

/// Additional init claims sent by the scripted program to its parent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Init {
    #[serde(default)]
    pub extra_tools: Vec<String>,
    #[serde(default)]
    pub extra_servers: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// The n-th user message runs this turn; beyond the story it says "Done.".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub end: End,
}

impl Default for Turn {
    fn default() -> Self {
        Self { steps: vec![Step::Text("Done.".into())], end: End::default() }
    }
}

/// A scripted action sent as stream messages or held until a signal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step {
    /// Sends text and, when requested, its partial messages.
    Text(String),
    /// Reports a rejected usage limit with an optional Unix-seconds reset.
    RateLimit { resets_at: Option<u64> },
    /// Reports the program's own retry to its parent.
    Retry { attempt: u32, status: u16 },
    /// Removes the login before the next turn checks it.
    LoseLogin,
    /// Waits until an interrupt or termination.
    Hang,
    /// Emits one assistant message, then runs its Call or Builtin steps in order.
    Together(Vec<Step>),
    /// Calls the named server tool, returning its answer to the model.
    Call { tool: String, input: serde_json::Value },
    /// Calls a server tool and checks its reply against scripted text.
    CallChecked { tool: String, input: serde_json::Value, contains: String },
    /// Asks permission before returning this built-in tool's scripted result.
    Builtin { tool: String, input: serde_json::Value, result: String },
}

/// A turn's terminal message, or its process exit without a result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum End {
    /// Completes successfully, optionally supplying the requested outcome.
    Success { structured_output: Option<serde_json::Value> },
    /// Reports the model's error with its available evidence.
    Error {
        api_error_status: Option<u16>,
        terminal_reason: Option<String>,
        errors: Vec<String>,
        /// Optional Unix-seconds account reset, as in the rate event.
        #[serde(default)]
        resets_at: Option<u64>,
    },
    /// Exits with this code without sending a result.
    Exit(u8),
    /// Sends an undecodable line and exits one.
    Undecodable,
}

impl Default for End {
    fn default() -> Self {
        Self::Success { structured_output: None }
    }
}

/// Token counts per assistant message; the parent receives their turn sum.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default = "default_input")]
    pub input_tokens: u64,
    #[serde(default = "default_output")]
    pub output_tokens: u64,
}

fn default_input() -> u64 {
    10
}

fn default_output() -> u64 {
    5
}

impl Default for Usage {
    fn default() -> Self {
        Self { input_tokens: 10, output_tokens: 5 }
    }
}

/// Whether the scripted program can resume the parent's named session.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resume {
    /// Resumes the conversation and accepts user messages.
    #[default]
    Found,
    /// Reports an absent conversation and exits one.
    Missing,
}

/// Settings names and permission rules recorded without other settings values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Seen {
    /// The settings read at startup, sent only to the observation file.
    Settings { allow: Vec<String>, ask: Vec<String>, deny: Vec<String>, keys: Vec<String> },
    /// The settings could not be read or decoded.
    UnreadableSettings,
    /// A server tool call and whether its result failed, without its input.
    Called { tool: String, error: bool },
    /// A built-in tool's permission decision, without its input.
    Prompted { tool: String, decision: Decision },
}

/// Reads only the appended observations, skipping invocation records.
pub fn seen(bytes: &[u8]) -> Vec<Seen> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).expect("observation JSON"))
        .filter_map(|value| value.get("seen").cloned())
        .map(|value| serde_json::from_value(value).expect("seen record"))
        .collect()
}

/// A permission decision observed by a test, without the tool's input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// The prompt allowed the call; records whether the input stayed unchanged.
    Allow { unchanged: bool },
    /// The prompt denied the call with this explanation.
    Deny(String),
    /// The prompt did not return a readable permission decision.
    Unreadable,
}
