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
