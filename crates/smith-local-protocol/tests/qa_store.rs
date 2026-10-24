//! Reserved for the independent design-only QA author; no adversarial cases
//! are authored by the implementation agent. The ordinary test binary exists
//! so nextest can resolve the QA profile and exclusions before that pass.
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "this independent QA adapter is an ordinary Rust test harness (programming-model.md, section 10.2)"
)]
#[cfg(test)]
#[path = "support/disk.rs"]
pub mod disk;
