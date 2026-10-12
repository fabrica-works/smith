//! Cross-process bounds from the agent's own limits (protocol/limits.md,
//! section 4). These pure queries know no run, provider activity or host policy.
use crate::Limits;
use skein_lib::Duration;

/// The longest short-operation deadline between progress reports; long operations report their spans.
#[must_use]
pub fn silence_bound(limits: &Limits) -> Option<Duration> {
    let stream = match limits.events {
        Some(events) => events.write_deadline,
        None => Duration::ZERO,
    };
    Some(limits.file_timeout.max(limits.io.close_timeout).max(stream))
}
