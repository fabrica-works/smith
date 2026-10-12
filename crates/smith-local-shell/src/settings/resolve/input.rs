//! Required catalog fields and their source-preserving defaults (shell.md, 5.2).
use super::super::{Origin, Problem, Refusal, Settings, Sourced};
use toml::Value;

pub(super) fn refuse(settings: &Settings, key: &str, problem: &str) -> Refusal {
    Refusal {
        origin: settings.values.get(key).map_or(Origin::BuiltIn, |value| value.origin.clone()),
        key: key.into(),
        problem: Problem::Input(problem.into()),
    }
}

pub(super) fn sourced<'a>(settings: &'a Settings, key: &str) -> Result<&'a Sourced, Refusal> {
    settings.values.get(key).ok_or_else(|| refuse(settings, key, "required value is absent"))
}

pub(super) fn text<'a>(settings: &'a Settings, key: &str) -> Result<&'a str, Refusal> {
    sourced(settings, key)?.value.as_str().ok_or_else(|| refuse(settings, key, "expected a string"))
}

pub(super) fn model_key(model: &str, field: &str) -> String {
    let name = super::super::name(model);
    format!("models.{name}.{field}")
}

pub(super) fn default(settings: &mut Settings, key: String, value: Value) {
    if settings.values.get(&key).is_none_or(|prior| prior.origin == Origin::BuiltIn) {
        settings.values.insert(key, Sourced { value, origin: Origin::BuiltIn });
    }
}

pub(super) fn number(settings: &Settings, key: &str) -> Result<u64, Refusal> {
    sourced(settings, key)?
        .value
        .as_integer()
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| refuse(settings, key, "expected a nonnegative integer"))
}

pub(super) fn count(settings: &Settings, key: &str) -> Result<u32, Refusal> {
    u32::try_from(number(settings, key)?).map_err(|_| refuse(settings, key, "count exceeds its record width"))
}

pub(super) fn millis(settings: &Settings, key: &str) -> Result<u64, Refusal> {
    number(settings, key)?.checked_mul(1000).ok_or_else(|| refuse(settings, key, "duration does not fit"))
}

pub(super) fn money(settings: &Settings, key: &str) -> Result<u64, Refusal> {
    let value = &sourced(settings, key)?.value;
    let dollars = value
        .as_float()
        .or_else(|| value.as_integer().and_then(|value| value.to_string().parse::<f64>().ok()))
        .ok_or_else(|| refuse(settings, key, "expected dollars"))?;
    let scaled = dollars * 1_000_000.0;
    if !scaled.is_finite() || scaled < 0.0 || scaled.fract() != 0.0 {
        return Err(refuse(settings, key, "dollars must be representable in whole micro-dollars"));
    }
    format!("{scaled:.0}").parse().map_err(|_| refuse(settings, key, "dollars exceed the host's unit range"))
}
