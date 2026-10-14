//! JSON retains infinite bootstrap ratios as a named value, never null.
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum Number {
    Finite(f64),
    Named(String),
}

fn encode(value: f64) -> Number {
    if value.is_finite() { Number::Finite(value) } else { Number::Named("infinite".into()) }
}

fn decode(value: Number) -> Result<f64, String> {
    match value {
        Number::Finite(value) if value.is_finite() => Ok(value),
        Number::Named(value) if value == "infinite" => Ok(f64::INFINITY),
        Number::Finite(_) | Number::Named(_) => Err("expected finite number or infinite".into()),
    }
}

pub(super) fn serialize<S: Serializer>(value: impl std::borrow::Borrow<f64>, serializer: S) -> Result<S::Ok, S::Error> {
    encode(*value.borrow()).serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    decode(Number::deserialize(deserializer)?).map_err(serde::de::Error::custom)
}

pub(super) mod interval {
    use super::{Deserialize, Deserializer, Number, Serialize, Serializer, decode, encode};

    pub(crate) fn serialize<S: Serializer>(value: &(f64, f64), serializer: S) -> Result<S::Ok, S::Error> {
        (encode(value.0), encode(value.1)).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(f64, f64), D::Error> {
        let (lower, upper) = <(Number, Number)>::deserialize(deserializer)?;
        Ok((decode(lower).map_err(serde::de::Error::custom)?, decode(upper).map_err(serde::de::Error::custom)?))
    }
}
