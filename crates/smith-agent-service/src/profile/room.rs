//! A declared model's room and the host and message parts derived from it.
//! Only numeric bounds are retained; no charter, provider destination or
//! transcript is known. `room` checks every declared model before returning
//! one set of receiving bounds. Contract: protocol/limits.md, sections 3.9 and 4.

#[cfg(test)]
mod tests;

use super::{Check, Configuration, Layer, LimitName, Per, Profile, Refusal, Value};

/// A window's receiving parts, sent by startup to the run and local host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    /// The smallest threshold less two reserved outputs, counting a byte as a token.
    pub room: u64,
    /// Largest measured Smith part across endpoints, including their actual options.
    pub own: u64,
    /// The host's rendered contribution, including directory guides, in bytes.
    pub host: u64,
    /// Two offers' rendered bytes.
    pub messages: u64,
    /// One offer's rendered bytes.
    pub offer: u64,
    /// One admitted message's bytes, also within the tool payload.
    pub message: u32,
}

/// Measure fixed run text, workspace schemas and the prompt framing allowance.
/// The summary instruction is added by its owner when compaction is available.
#[must_use]
pub fn own_part(tools: &smith_domain::tools::Limits, call_input_bytes: u32, allowance: u64) -> Option<u64> {
    let stated = smith_protocol_llm::Stated::from_tools(tools, call_input_bytes)?;
    smith_domain::run::fixed_prompt_bytes()?
        .checked_add(smith_protocol_llm::fixed_schema_bytes(&stated)?)?
        .checked_add(allowance)
}

/// Bytes above the line for the run's summary instruction and owed-result markers.
#[must_use]
pub fn compaction_material(calls_per_response: u32, summary_bytes: u64) -> Option<u64> {
    u64::from(calls_per_response)
        .checked_mul(u64::from(smith_protocol_llm::CUT_MARKER_BYTES))?
        .checked_add(summary_bytes)
}

/// Check all declared models and split their smallest room after Smith's own part.
/// `summary_bytes` is the measured instruction that compaction adds above the
/// threshold; it is already included in `own`. A byte counts as a token.
pub fn room(profile: &Profile, configuration: &Configuration, own: u64, summary_bytes: u64) -> Result<Room, Refusal> {
    super::validate::configured_models(configuration)?;
    let threshold = profile.policy.compaction_threshold;
    if threshold.numerator == 0 || threshold.numerator >= threshold.denominator {
        return Err(Refusal::Broken {
            check: Check::FreshWindow,
            left: LimitName::deployment(Layer::Policy, "compaction_threshold").value(Value::Fraction(threshold)),
            right: LimitName::deployment(Layer::Policy, "compaction_threshold_maximum")
                .value(Value::Fraction(super::Fraction { numerator: 1, denominator: 1 })),
        });
    }
    let mut smallest = None;
    let mut smallest_remaining = None;
    let mut largest_own = own;
    for endpoint in &configuration.endpoints {
        let endpoint_own = own.checked_add(endpoint.option_bytes).ok_or(derived("fresh_window"))?;
        largest_own = largest_own.max(endpoint_own);
        for (index, model) in endpoint.models.iter().enumerate() {
            let per = Per::Model {
                endpoint: endpoint.number,
                model: u32::try_from(index).or(Err(derived("configured_models")))?,
            };
            let window = u64::from(model.window);
            let output = u64::from(model.output);
            let within = window
                .checked_mul(u64::from(threshold.numerator))
                .ok_or(derived("window_at_threshold"))?
                .checked_div(u64::from(threshold.denominator))
                .ok_or(derived("window_at_threshold"))?;
            let reserved = output.checked_mul(2).ok_or(derived("fresh_window"))?;
            let required = reserved.checked_add(endpoint_own).ok_or(derived("fresh_window"))?;
            if required >= within {
                return Err(Refusal::Broken {
                    check: Check::FreshWindow,
                    left: model_value(Layer::Session, "fresh_window", per, required),
                    right: model_value(Layer::Declared, "window_at_threshold", per, within),
                });
            }
            let headroom = window.checked_sub(within).expect("proper threshold within the window");
            let material = compaction_material(profile.declared.calls_per_response, summary_bytes)
                .ok_or(derived("compaction_headroom"))?;
            let required = output.checked_add(material).ok_or(derived("compaction_headroom"))?;
            if required > headroom {
                return Err(Refusal::Broken {
                    check: Check::CompactionHeadroom,
                    left: model_value(Layer::Session, "compaction_headroom", per, required),
                    right: model_value(Layer::Declared, "window_above_threshold", per, headroom),
                });
            }
            let room = within.checked_sub(reserved).expect("fresh window holds both outputs");
            let remaining = room.checked_sub(endpoint_own).expect("fresh window leaves room after actual own part");
            smallest_remaining = Some(match smallest_remaining {
                Some(current) => remaining.min(current),
                None => remaining,
            });
            smallest = Some(match smallest {
                Some(current) => room.min(current),
                None => room,
            });
        }
    }
    let room = smallest.expect("validated configuration declares at least one model");
    let remaining = smallest_remaining.expect("validated models leave measured room");
    let host = remaining.checked_div(2).expect("nonzero fixed divisor");
    let messages = remaining.checked_sub(host).expect("host takes at most half the remainder");
    let offer = messages.checked_div(2).expect("nonzero fixed divisor");
    let message =
        u32::try_from(offer.min(u64::from(profile.declared.tool_payload))).or(Err(derived("message_bytes")))?;
    Ok(Room { room, own: largest_own, host, messages, offer, message })
}

fn model_value(layer: Layer, field: &'static str, per: Per, value: u64) -> super::Limit {
    LimitName { layer, field, per }.value(Value::Tokens(value))
}

fn derived(field: &'static str) -> Refusal {
    Refusal::Derived { limit: LimitName::deployment(Layer::Service, field) }
}
