//! Program source commands (protocol/programs.md, sections 3, 4.1 and 4.2).
//! Keeps a command's bounded output, deadline and child/pipe ownership; knows
//! no login contents or host policy. `configuration_directory`, `environment`
//! and `status` build and read source records; `Command` drives their children.
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod command;
mod source;
mod status;

#[cfg(test)]
mod tests;

pub use command::{Command, Ended, Launch, command_worst_case};
pub use source::{Error, Programs, configuration_directory, environment, keeps};
pub use status::{Method, SIGN_OUT, STATUS, STATUS_BYTES, SUBSCRIPTION, Status, status};
