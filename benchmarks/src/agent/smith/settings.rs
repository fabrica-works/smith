//! A frozen local JSON template is rendered into guarded private paths.
//! The renderer knows native configuration fields, never a running domain's
//! state. Unsupported author overrides refuse before credential conversion
//! (benchmarks.md, sections 5.2, 6.2 and 10; reliability-plan 00.11).

use std::path::Path;

use serde_json::{Value, json};

use crate::agent::Context;
use crate::isolate::{Setup, SetupStep};
use crate::{Task, Variant};

use super::legacy::Shape;

fn configuration(reason: impl Into<String>) -> Setup {
    Setup { step: SetupStep::Configuration, reason: reason.into() }
}

fn currency(spend: f64, unit: u64) -> Result<u64, String> {
    let unit = u32::try_from(unit).map_err(|_| "currency unit exceeds its integer range")?;
    let scaled = spend * f64::from(unit);
    if !scaled.is_finite() || scaled <= 0.0 || scaled.fract() != 0.0 {
        return Err("task spend cannot be represented in this arm's integer currency".into());
    }
    format!("{scaled:.0}").parse().map_err(|_| "task spend exceeds the currency range".into())
}

fn budget(shape: Shape, task: &Task, variant: Option<&Variant>) -> Result<Value, String> {
    let budget = variant.and_then(|variant| variant.budget.as_ref()).unwrap_or(&task.budget);
    let (turns, spend, currency_unit, seconds) = match shape {
        Shape::LocalJson | Shape::LocalDeclared => (64_u64, 1_u64, 1_u64, 3600_u64),
        Shape::LocalPriced | Shape::LocalDelegating | Shape::LocalReserved => (4096, 1_000_000_000, 1_000_000, 86400),
    };
    let requested_turns = budget.turns.unwrap_or(turns);
    let requested_spend = match budget.spend {
        Some(value) => currency(value, currency_unit)?,
        None => spend,
    };
    let requested_seconds = budget.seconds.unwrap_or(task.deadline_seconds);
    if requested_turns == 0 || requested_turns > turns {
        return Err(format!("task budget.turns exceeds this arm's positive ceiling {turns}"));
    }
    if requested_spend == 0 || requested_spend > spend {
        return Err(format!("task budget.spend exceeds this arm's positive ceiling {spend}"));
    }
    if requested_seconds == 0 || requested_seconds > task.deadline_seconds || requested_seconds > seconds {
        return Err("task budget.seconds must fit the attempt deadline".into());
    }
    let mut value = json!({"turns":requested_turns,"spend":requested_spend,"seconds":requested_seconds});
    if shape == Shape::LocalReserved {
        if requested_seconds <= 1 {
            return Err("native reserve requires room below total time".into());
        }
        value["reserve"] = json!({"turns":2,"seconds":if requested_seconds>300 {300} else {requested_seconds/2}});
    }
    Ok(value)
}

fn overrides(
    shape: Shape,
    settings: &mut Value,
    fields: &std::collections::BTreeMap<String, toml::Value>,
) -> Result<(), String> {
    for (name, value) in fields {
        let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
        if value.as_u64().is_none() {
            return Err(format!("Smith task override {name} requires an unsigned integer"));
        }
        if shape == Shape::LocalJson {
            let path = match name.as_str() {
                "memory_bytes" => "/agent/memory_bytes",
                "grace_ms" => "/agent/grace_ms",
                "waiting_seconds" => "/waiting_seconds",
                "max_tokens" => "/models/0/max_tokens",
                _ => return Err(format!("the baseline cannot take task override {name}")),
            };
            *settings.pointer_mut(path).ok_or("baseline pin is missing its native field")? = value;
            continue;
        }
        let (path, declared) = match name.as_str() {
            "window" | "output" | "reasoning_item" | "head" | "idle" => (format!("/models/0/{name}"), false),
            "connect" | "handshake" => (format!("/agent/endpoints/0/{name}"), false),
            "conversations" | "tool_payload" | "file_bytes" | "calls_per_response" | "read_window" | "shell_output"
            | "shell_head" | "shell_tail" | "search_hits" | "search_bytes" | "list_entries" | "guide" | "llm_pool"
            | "memory" => (format!("/agent/profile/declared/{name}"), true),
            "max_turns" | "max_spend" | "max_time" | "max_waiting" | "sections" | "host_tools" | "verdicts"
            | "items" | "fields" | "inbox" | "unacknowledged" | "connection_keep" | "tool_deadline"
            | "shell_timeout" | "group_stop" | "close" | "write_deadline" | "cancel_grace" | "exit_grace" => {
                (format!("/agent/profile/policy/{name}"), true)
            }
            _ => return Err(format!("this native Smith face cannot take task override {name}")),
        };
        if declared {
            let (parent, field) = path.rsplit_once('/').expect("generated field path");
            let object = settings
                .pointer_mut(parent)
                .and_then(Value::as_object_mut)
                .ok_or("profile template is missing its named overrides")?;
            object.insert(field.into(), value);
        } else {
            let field = settings.pointer_mut(&path).ok_or("native template is missing its field")?;
            *field = value;
        }
    }
    Ok(())
}

/// Render this arm's native settings and refuse unsupported limits by name.
pub fn settings(
    context: &Context<'_>,
    shape: Shape,
    base: &Value,
    account_id: &str,
    trace: &Path,
    tokens: &Path,
) -> Result<Value, Setup> {
    let render: Result<Value, String> = (|| {
        let mut settings = base.clone();
        if !settings.is_object()
            || !settings["agent"].is_object()
            || settings["models"].as_array().map(Vec::len) != Some(1)
            || !settings["models"][0].is_object()
            || settings["agent"]["endpoints"].as_array().map(Vec::len) != Some(1)
            || !settings["agent"]["endpoints"][0].is_object()
        {
            return Err("Smith pin requires one native endpoint and priced model".into());
        }
        let model =
            context.model.model.as_ref().filter(|model| !model.is_empty()).ok_or("Smith model must be selected")?;
        let effort =
            context.model.effort.as_ref().filter(|effort| !effort.is_empty()).ok_or("Smith effort must be selected")?;
        settings["models"][0]["name"] = json!(model);
        settings["agent"]["endpoints"][0]["reasoning_effort"] = json!(effort);
        settings["agent"]["trace"] = json!({"path":trace,"capture":"everything"});
        settings["directories"] = json!([{"name":"repo","path":context.private.workspace,"writable":true,"git":true}]);
        settings["accounts"] = json!([{"number":0,"account_id":account_id}]);
        settings["token_directory"] = json!(tokens);
        settings["budget"] = budget(shape, context.task, context.variant)?;
        let environment: Result<Vec<_>, String> = context
            .environment
            .iter()
            .map(|(name, value)| {
                Ok(json!({"name":name.to_str().ok_or("Smith environment name is not UTF-8")?,
                "value":value.to_str().ok_or("Smith environment value is not UTF-8")?}))
            })
            .collect();
        settings["agent"]["environment"] = json!(environment?);
        match shape {
            Shape::LocalJson => {}
            Shape::LocalDeclared | Shape::LocalPriced | Shape::LocalDelegating | Shape::LocalReserved => {
                let agent = settings["agent"].as_object_mut().expect("validated native template");
                let memory = agent.remove("memory_bytes").ok_or("baseline template has no memory bound")?;
                let group_stop = agent.remove("grace_ms").ok_or("baseline template has no group grace")?;
                agent.insert(
                    "profile".into(),
                    json!({"name":"standard","declared":{"memory":memory},
                    "policy":{"group_stop":group_stop}}),
                );
                agent["endpoints"][0]["connect"] = json!(30_000);
                agent["endpoints"][0]["handshake"] = json!(30_000);
                let model = settings["models"][0].as_object_mut().expect("validated model");
                let output = model.remove("max_tokens").ok_or("baseline template has no output bound")?;
                model.insert("window".into(), json!(131_072));
                model.insert("output".into(), output);
                model.insert("reasoning_item".into(), json!(65_536));
                model.insert("head".into(), json!(60_000));
                model.insert("idle".into(), json!(120_000));
            }
        }
        if matches!(shape, Shape::LocalPriced | Shape::LocalDelegating | Shape::LocalReserved) {
            let model = settings["models"][0].as_object_mut().expect("validated model");
            for rate in ["input_price", "cached_price", "output_price"] {
                let value = model[rate]
                    .as_u64()
                    .and_then(|value| value.checked_mul(1_000_000))
                    .ok_or("notional price cannot be scaled to micro-dollars")?;
                model.insert(rate.into(), json!(value));
            }
        }
        if matches!(shape, Shape::LocalDelegating | Shape::LocalReserved) {
            settings["sub_agents"] = json!(true);
        }
        overrides(shape, &mut settings, &context.task.smith)?;
        if let Some(variant) = context.variant {
            overrides(shape, &mut settings, &variant.smith)?;
        }
        if shape == Shape::LocalJson
            && !settings["models"][0]["max_tokens"].as_u64().is_some_and(|tokens| (1..=4096).contains(&tokens))
        {
            return Err("baseline max_tokens must fit its standard ceiling 4096".into());
        }
        Ok(settings)
    })();
    render.map_err(configuration)
}

/// Two sequential native IO close deadlines bound the agent's teardown.
pub fn teardown(shape: Shape, settings: &Value) -> Result<skein_lib::Duration, Setup> {
    let close = match shape {
        Shape::LocalJson => Some(5000),
        Shape::LocalDeclared | Shape::LocalPriced | Shape::LocalDelegating | Shape::LocalReserved => {
            match settings.pointer("/agent/profile/policy/close") {
                Some(value) => value.as_u64(),
                None => Some(5000),
            }
        }
    }
    .ok_or_else(|| configuration("native IO close must be positive milliseconds"))?;
    let milliseconds = close
        .checked_mul(2)
        .filter(|value| *value > 0)
        .ok_or_else(|| configuration("native teardown exceeds its duration range"))?;
    let nanos = milliseconds
        .checked_mul(1_000_000)
        .ok_or_else(|| configuration("native teardown exceeds its duration range"))?;
    Ok(skein_lib::Duration::from_nanos(nanos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Context;
    use crate::arm::ArmRef;

    fn one_turn_refusal_reaches_the_product_with_the_default_reserve() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut task = crate::read_task(&directory.join("tests/recorded/tasks/valid/task.toml")).expect("task");
        task.budget.turns = Some(1);
        task.budget.seconds = Some(600);
        task.deadline_seconds = 630;
        let refused_input = budget(Shape::LocalReserved, &task, None).expect("product judges entrance reserve");
        assert_eq!(refused_input["turns"], 1);
        assert_eq!(refused_input["reserve"], json!({"turns":2,"seconds":300}));
    }

    #[test]
    fn native_shapes_keep_task_deadlines_environment_prices_and_named_overrides() {
        one_turn_refusal_reaches_the_product_with_the_default_reserve();
        native_shape_cases();
    }

    fn native_shape_cases() {
        let root = std::env::temp_dir().join(format!("smith-settings-{}", std::process::id()));
        std::fs::create_dir(&root).expect("fixture parent");
        let guard = crate::guard::Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[])
            .expect("guard");
        let private = crate::isolate::empty(&guard, &root.join("scratch")).expect("scratch");
        let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let pin_file = directory.join("agents/smith/standard.pin.toml");
        let pin = crate::read_configuration(&pin_file).expect("pin");
        let mut task = crate::read_task(&directory.join("tests/recorded/tasks/valid/task.toml")).expect("task");
        let model = crate::ModelChoice { model: Some("selected-model".into()), effort: Some("low".into()) };
        let arm = ArmRef::Installed { program: "/unused".into() };
        let environment = vec![("NAMED".into(), "exact-value".into())];
        let base: Value = serde_json::from_str(include_str!("../../../agents/smith/legacy.json")).expect("template");
        let render = |task: &Task, shape| {
            settings(
                &Context {
                    guard: &guard,
                    private: &private,
                    pin: &pin,
                    pin_file: &pin_file,
                    task,
                    variant: None,
                    model: &model,
                    arm: &arm,
                    installed_version: "unused",
                    home: &root.join("user/.codex"),
                    wall: skein_lib::Wall::EPOCH,
                    lifetime: skein_lib::Duration::from_secs(20),
                    environment: &environment,
                },
                shape,
                &base,
                "opaque-account",
                &root.join("trace"),
                &root.join("tokens"),
            )
        };
        let baseline = render(&task, Shape::LocalJson).expect("baseline");
        assert_eq!(baseline["models"][0]["max_tokens"], 4096);
        assert_eq!(baseline["budget"]["turns"], 64);
        assert_eq!(baseline["budget"]["seconds"], 10);
        assert_eq!(baseline["models"][0]["input_price"], 10);
        assert_eq!(baseline["agent"]["environment"][0]["value"], "exact-value");
        assert_eq!(baseline["agent"]["trace"]["capture"], "everything");
        assert_eq!(baseline["in_process"], false);
        let declared = render(&task, Shape::LocalDeclared).expect("pre-pricing declarations");
        assert_eq!(declared["budget"]["turns"], 64);
        assert_eq!(declared["budget"]["spend"], 1);
        assert_eq!(declared["models"][0]["input_price"], 10);
        assert_eq!(declared["models"][0]["output"], 4096);
        task.budget.turns = Some(65);
        assert!(render(&task, Shape::LocalDeclared).expect_err("old turn ceiling").reason.contains("turns"));
        task.budget.spend = Some(1.25);
        let priced = render(&task, Shape::LocalPriced).expect("new priced policy");
        assert_eq!(priced["budget"]["turns"], 65);
        assert_eq!(priced["budget"]["spend"], 1_250_000);
        assert_eq!(priced["models"][0]["input_price"], 10_000_000);
        task.budget.turns = None;
        assert!(render(&task, Shape::LocalDeclared).expect_err("old integer currency").reason.contains("currency"));
        task.budget.spend = None;
        task.smith.insert("max_tokens".into(), toml::Value::Integer(8192));
        assert!(render(&task, Shape::LocalJson).expect_err("standard ceiling").reason.contains("max_tokens"));
        task.smith.clear();
        task.smith.insert("read_window".into(), toml::Value::Integer(4096));
        assert!(
            render(&task, Shape::LocalJson).expect_err("unsupported baseline field").reason.contains("read_window")
        );
        // This startup control declares its small task's actual payload and
        // call count, so current native InputAnswer/WindowFirst checks apply.
        task.smith.insert("tool_payload".into(), toml::Value::Integer(4096));
        task.smith.insert("calls_per_response".into(), toml::Value::Integer(1));
        task.smith.insert("file_bytes".into(), toml::Value::Integer(1_048_576));
        let current = render(&task, Shape::LocalDelegating).expect("current");
        assert_eq!(current["agent"]["profile"]["declared"]["read_window"], 4096);
        assert_eq!(current["agent"]["profile"]["declared"]["tool_payload"], 4096);
        assert_eq!(current["agent"]["profile"]["declared"]["calls_per_response"], 1);
        assert_eq!(current["models"][0]["input_price"], 10_000_000);
        assert_eq!(current["sub_agents"], true);
        let local: smith_local_shell::local_settings::Settings =
            serde_json::from_value(current.clone()).expect("shipped local settings");
        let encoded = smith_local_shell::local_settings::agent_document(&local).expect("local generates its models");
        let configured =
            smith_agent_shell::config::parse(&encoded).expect("shipped agent accepts the local-generated document");
        assert_eq!(configured.profile.declared.file_bytes, 1_048_576);
        task.smith.insert("output".into(), toml::Value::Integer(1024));
        let changed = render(&task, Shape::LocalDeclared).expect("declared output override");
        assert_eq!(changed["models"][0]["output"], 1024);
        assert!(changed["agent"]["endpoints"][0].get("models").is_none());
        let local = serde_json::from_value(changed).expect("changed local settings");
        let encoded = smith_local_shell::local_settings::agent_document(&local).expect("changed agent document");
        let agent: Value = serde_json::from_slice(&encoded).expect("generated JSON");
        assert_eq!(agent["endpoints"][0]["models"][0]["output"], 1024);
        assert_eq!(
            teardown(Shape::LocalDelegating, &current).expect("two close deadlines"),
            skein_lib::Duration::from_secs(10)
        );
        task.smith.insert("read_window".into(), toml::Value::String("wrong".into()));
        assert!(render(&task, Shape::LocalDelegating).expect_err("wrong field type").reason.contains("read_window"));
        task.smith.clear();
        task.budget.seconds = Some(11);
        assert!(render(&task, Shape::LocalJson).is_err());
        let suite_path = crate::fix::calibration_suite("probes/first-request");
        let suite = crate::read_suite(&suite_path).expect("first-request suite");
        let selected_pin = directory.join("agents/smith").join(format!("{}.pin.toml", suite.agents[0].config));
        let selected = crate::read_configuration(&selected_pin).expect("selected first-request pin");
        let selected_base: Value = serde_json::from_slice(
            &std::fs::read(selected_pin.parent().expect("pin directory").join(&selected.pin.configuration))
                .expect("selected template"),
        )
        .expect("selected JSON");
        let first_task = crate::read_task(&directory.join("tasks/probes/first-request/task.toml"))
            .expect("unchanged first-request task");
        let context = Context {
            guard: &guard,
            private: &private,
            pin: &selected,
            pin_file: &selected_pin,
            task: &first_task,
            variant: None,
            model: &model,
            arm: &arm,
            installed_version: "unused",
            home: &root.join("user/.codex"),
            wall: skein_lib::Wall::EPOCH,
            lifetime: skein_lib::Duration::from_secs(20),
            environment: &environment,
        };
        let first_settings = settings(
            &context,
            Shape::LocalReserved,
            &selected_base,
            "opaque-account",
            &root.join("trace"),
            &root.join("tokens"),
        )
        .expect("actual first-request native render");
        assert_eq!(first_settings["instructions"], "", "empty-brief probe must not demand AGENTS/read calls");
        let mut expected = settings(
            &context,
            Shape::LocalReserved,
            &base,
            "opaque-account",
            &root.join("trace"),
            &root.join("tokens"),
        )
        .expect("old native render");
        expected["instructions"] = json!("");
        assert_eq!(first_settings, expected, "all other native fields remain identical");
        assert_eq!(suite.agents[0].config, "first-request");
        assert_eq!(crate::fix::calibration_suite("probes/smoke-tools"), directory.join("suites/calibrate.toml"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
