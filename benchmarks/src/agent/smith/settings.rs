//! Product user layers hold the account and trace; task settings travel as flags.
//! Only attempt-owned paths and the selected model are substituted. No preset
//! value is derived here (benchmarks.md, sections 5.4, 6.1 and 10).

use std::path::Path;

use serde_json::Value;

use crate::agent::Context;
use crate::isolate::{Setup, SetupStep};

fn configuration(reason: impl Into<String>) -> Setup {
    Setup { step: SetupStep::Configuration, reason: reason.into() }
}

/// Render the pin's trace and shared account declarations with its credentials directory.
pub fn product(context: &Context<'_>, base: &Value, login: &Path) -> Result<Value, Setup> {
    let mut layer = base.clone();
    if !layer.is_object() || !layer["trace"].is_object() {
        return Err(configuration("Smith pin requires a product account and trace layer"));
    }
    let shared = declarations()?;
    layer["accounts"] = shared["accounts"].clone();
    let endpoint = endpoint(context.pin.pin.provider);
    layer["endpoints"] = serde_json::json!({endpoint: shared["endpoints"][endpoint]});
    layer["credentials"] = serde_json::json!(login);
    layer["invocation"] = serde_json::json!(arguments(context)?);
    Ok(layer)
}

fn declarations() -> Result<Value, Setup> {
    let shared: toml::Value =
        toml::from_str(crate::credential::logins::ACCOUNTS).map_err(|error| configuration(error.to_string()))?;
    serde_json::to_value(shared).map_err(|error| configuration(error.to_string()))
}

fn endpoint(provider: crate::Provider) -> &'static str {
    match provider {
        crate::Provider::Codex => "openai",
        crate::Provider::Anthropic => "anthropic",
    }
}

/// The selected endpoint's harness account comes from the shared declarations.
pub fn account(context: &Context<'_>) -> Result<String, Setup> {
    declarations()?["endpoints"][endpoint(context.pin.pin.provider)]["account"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| configuration("shared endpoint needs an account"))
}

/// The selected account's source decides whether channel facts live in a child trace.
pub fn program(context: &Context<'_>) -> Result<bool, Setup> {
    let account = account(context)?;
    let shared = declarations()?;
    let source = shared
        .get("accounts")
        .and_then(|accounts| accounts.get(&account))
        .and_then(|account| account.get("source"))
        .and_then(Value::as_str)
        .ok_or_else(|| configuration(format!("shared account {account} needs a source")))?;
    Ok(source == "program")
}

fn setting(arguments: &mut Vec<String>, key: &str, value: &toml::Value) {
    arguments.extend(["-c".into(), format!("{key}={value}")]);
}

/// Pass product settings directly, resolving the tier model and private paths.
pub fn arguments(context: &Context<'_>) -> Result<Vec<String>, Setup> {
    let model = context
        .model
        .model
        .as_ref()
        .filter(|model| !model.is_empty())
        .ok_or_else(|| configuration("Smith model must be selected"))?;
    let effort = context
        .model
        .effort
        .as_ref()
        .filter(|effort| !effort.is_empty())
        .ok_or_else(|| configuration("Smith effort must be selected"))?;
    let selected = context
        .variant
        .and_then(|variant| variant.smith.get("model"))
        .or_else(|| context.task.smith.get("model"))
        .and_then(toml::Value::as_str)
        .map_or_else(|| format!("{}/{model}", endpoint(context.pin.pin.provider)), str::to_owned);
    let working =
        crate::isolate::working_directory(context.guard, context.private, context.task).map_err(configuration)?;
    let mut arguments = vec![
        "-C".into(),
        working.to_str().ok_or_else(|| configuration("Smith workspace is not UTF-8"))?.into(),
        "-m".into(),
        selected.clone(),
        "--effort".into(),
        effort.clone(),
    ];
    for fields in std::iter::once(&context.task.smith).chain(context.variant.map(|variant| &variant.smith)) {
        for (key, value) in fields {
            let key = key.replace("{model}", &selected);
            let value = if key == "workspace.directories" {
                let directories = super::directories::render(context).map_err(configuration)?;
                serde_json::from_value(directories).map_err(|error| configuration(error.to_string()))?
            } else {
                value.clone()
            };
            setting(&mut arguments, &key, &value);
        }
    }
    let budget = context.variant.and_then(|variant| variant.budget.as_ref()).unwrap_or(&context.task.budget);
    for (key, value) in [("budget.turns", budget.turns), ("budget.seconds", budget.seconds)] {
        if let Some(value) = value {
            let value = i64::try_from(value).map_err(|_| configuration("Smith budget exceeds its integer range"))?;
            setting(&mut arguments, key, &toml::Value::Integer(value));
        }
    }
    if let Some(value) = budget.spend {
        setting(&mut arguments, "budget.spend", &toml::Value::Float(value));
    }
    for (name, value) in context.environment {
        let name = name.to_str().ok_or_else(|| configuration("Smith environment name is not UTF-8"))?;
        let value = value.to_str().ok_or_else(|| configuration("Smith environment value is not UTF-8"))?;
        setting(
            &mut arguments,
            &format!("environment.set.{}", serde_json::json!(name)),
            &toml::Value::String(value.into()),
        );
    }
    Ok(arguments)
}
