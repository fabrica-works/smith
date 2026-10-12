//! The event vocabulary's effective names, with configured endpoint and model names.
//! No addresses or credentials are read. Contract: protocol/limits.md, section 9.

use super::super::Configuration;
use super::{Limit, LimitName, Per, Value};
use alloc::boxed::Box;
use skein_lib::{List, Writer};
use smith_events::{EffectiveLimit, LimitValue};

/// Spell the startup list's names, durations in milliseconds and exact fractions.
#[must_use]
pub fn header(
    limits: &[Limit],
    names: &smith_protocol_channel::Endpoints,
    configuration: &Configuration,
) -> Option<List<EffectiveLimit>> {
    let mut result = List::with_capacity(u32::try_from(limits.len()).ok()?);
    for limit in limits {
        let value = match limit.value {
            Value::Bytes(value) | Value::Count(value) | Value::Tokens(value) | Value::Spend(value) => {
                LimitValue::Integer(value)
            }
            Value::Duration(value) => LimitValue::Integer(value.as_nanos().checked_div(1_000_000)?),
            Value::Fraction(value) => {
                if value.denominator == 0 {
                    return None;
                }
                LimitValue::Fraction {
                    numerator: u64::from(value.numerator),
                    denominator: u64::from(value.denominator),
                }
            }
        };
        result.push(EffectiveLimit { name: name(&limit.name, names, configuration)?, value }).ok()?;
    }
    Some(result)
}

fn name(
    limit: &LimitName,
    names: &smith_protocol_channel::Endpoints,
    configuration: &Configuration,
) -> Option<Box<[u8]>> {
    let mut parts: [&[u8]; 8] = [limit.layer.name().as_bytes(), b".", limit.field.as_bytes(), b"", b"", b"", b"", b""];
    match limit.per {
        Per::Deployment => {}
        Per::Endpoint(number) => {
            parts[3] = b"[endpoint=";
            parts[4] = names.name_for(number)?;
            parts[5] = b"]";
        }
        Per::Model { endpoint, model } => {
            parts[3] = b"[endpoint=";
            parts[4] = names.name_for(endpoint)?;
            parts[5] = b",model=";
            parts[6] = model_name(configuration, endpoint, model)?;
            parts[7] = b"]";
        }
    }
    let mut length = 0_usize;
    for part in parts {
        length = length.checked_add(part.len())?;
    }
    let mut writer = Writer::new(length);
    for part in parts {
        writer.put(part).ok()?;
    }
    Some(writer.finish())
}

fn model_name(configuration: &Configuration, endpoint: u32, model: u32) -> Option<&[u8]> {
    for declared in &configuration.endpoints {
        if declared.number == endpoint {
            return Some(&declared.models.get(usize::try_from(model).ok()?)?.name);
        }
    }
    None
}
