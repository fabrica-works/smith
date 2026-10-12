//! smith's faces keep outside observations, never the running agent's state.
//! The legacy reader consumes pre-events records (benchmarks.md, section 6.2).

pub mod legacy;

pub mod events;

mod faces;

mod settings;

mod directories;

mod prepare;

mod observe;

pub use faces::{CredentialShape, Face, FaceSince, Receiving, face};

pub use prepare::{Credential, Prepared, prepare};

pub use observe::{Observer, Observing};
