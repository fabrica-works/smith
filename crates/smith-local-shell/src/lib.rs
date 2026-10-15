//! Smith's local shell library (protocol/agent.md, section 6; protocol/hosts.md,
//! section 5). Configuration, durable chat and token files, and terminal
//! output are shared by the binary and hosted process worlds. Kernel,
//! simulator, neighbour and fault state belong to the caller.

pub mod check;
pub mod command;
pub mod compose;
pub mod credentials;
pub mod front;
pub mod local_host;
pub mod presets;
pub mod role;
pub mod settings;
pub mod startup;

pub use local_host::{Local, Resources};
pub use presets::{Prices, list_prices};
pub use startup::Failure;
