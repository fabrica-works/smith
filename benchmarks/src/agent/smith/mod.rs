//! smith's faces keep outside observations, never the running agent's state.
//! Event records are read through smith-events (benchmarks.md, section 6).

pub mod events;

mod faces;

mod settings;

mod directories;

mod prepare;

mod observe;

pub use faces::{FaceSince, Receiving, face, read as read_faces};

pub use prepare::{Credential, Prepared, prepare, teardown_from_check};

pub use observe::Observing;
