//! Program source commands (protocol/programs.md, sections 3, 4.1 and 4.2).
//! Keeps a command's bounded output, deadline and child/pipe ownership; knows
//! no login contents or host policy. `configuration_directory`, `environment`
//! and `status` build and read source records; `Command` drives their children.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod backend;
mod command;
pub mod mcp;
pub mod relay;
mod source;
mod status;

#[cfg(test)]
mod tests;

pub use command::{Command, Ended, Launch, command_worst_case};
pub use source::{
    ENVIRONMENT_BYTES, ENVIRONMENT_ENTRIES, Error, Programs, configuration_directory, environment, keeps,
};
pub use status::{Method, SIGN_IN, SIGN_OUT, STATUS, STATUS_BYTES, SUBSCRIPTION, Status, status};
