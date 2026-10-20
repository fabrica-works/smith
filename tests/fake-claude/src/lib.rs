//! Scripted sign-in for Claude Code (protocol/programs.md, section 8;
//! testing.md, section 4). The fake keeps one command, its story and queued
//! I/O, and knows no person's home or credentials. [`Fake`] is the step
//! machine, [`hosted`] its world factory and [`main`] its kernel entry point.

#![forbid(unsafe_code)]

mod commands;
mod faces;
mod fake;
mod output;
mod story;

pub use faces::{config_root, hosted, main, observation_append};
pub use fake::{Faces, Fake};
pub use story::{Login, Logout, Observation, Status, Story, observations};

/// The environment variable naming this invocation's configuration directory.
pub const CONFIG_DIR: &str = "CLAUDE_CONFIG_DIR";

/// The configuration-directory file holding the signed-in authentication method.
pub const LOGIN_FILE: &str = "fake-claude.login";

/// The configuration-directory file holding the test's encoded story.
pub const STORY_FILE: &str = "fake-claude.story";

/// The configuration-directory file receiving appended invocation observations.
pub const OBSERVATIONS_FILE: &str = "fake-claude.observations";
