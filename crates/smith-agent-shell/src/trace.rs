//! Startup's append destination and its service-owned delivery policy.
//! No file writer, thread or credential lives here; the shell opens the path
//! beneath its parent root and the service owns its IO lifecycle.
//! Contract: protocol/events.md, sections 5.1–5.3; protocol/agent.md, section 5.

use std::path::PathBuf;

pub use smith_agent_service::sinks::{Capture, Delivery};

/// A configured append destination; the service closes it after its session end.
pub struct TraceConfig {
    pub path: PathBuf,
    pub capture: Capture,
    pub delivery: Delivery,
}
