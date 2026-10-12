//! Trace settings decoded before startup, with no sink or credential opened.
//! Contract: protocol/events.md, section 2.
use super::{ConfigurationRefusal, TraceDocument};
use crate::trace::{Capture, Delivery, TraceConfig};

pub(super) fn build(document: Option<TraceDocument>) -> Result<Option<TraceConfig>, ConfigurationRefusal> {
    let Some(trace) = document else { return Ok(None) };
    if trace.path.is_empty() {
        return Err("trace path is empty".into());
    }
    let capture = match trace.capture.as_str() {
        "none" => Capture::None,
        "calls" => Capture::Calls,
        "everything" => Capture::Everything,
        _ => return Err("trace capture must be none, calls or everything".into()),
    };
    let delivery = match trace.delivery.as_str() {
        "complete" => Delivery::Complete,
        "best_effort" => Delivery::BestEffort,
        _ => return Err("trace delivery must be complete or best_effort".into()),
    };
    Ok(Some(TraceConfig { path: trace.path.into(), capture, delivery }))
}
