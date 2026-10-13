//! A frozen local JSON template is rendered into guarded private paths.
//! The renderer knows native configuration fields, never a running domain's
//! state. Unsupported author overrides refuse before credential conversion
//! (benchmarks.md, sections 5.2, 6.2 and 10; reliability-plan 00.11).

use std::path::Path;

use serde_json::{Value, json};

use crate::agent::Context;
use crate::isolate::{Setup, SetupStep};
use crate::{Task, Variant};

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

fn budget(task: &Task, variant: Option<&Variant>) -> Result<Value, String> {
    let budget = variant.and_then(|variant| variant.budget.as_ref()).unwrap_or(&task.budget);
    let (turns, spend, currency_unit, seconds) = (4096, 1_000_000_000, 1_000_000, 86400);
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
    if requested_seconds <= 1 {
        return Err("native reserve requires room below total time".into());
    }
    value["reserve"] = json!({"turns":2,"seconds":if requested_seconds>300 {300} else {requested_seconds/2}});
    Ok(value)
}

fn overrides(settings: &mut Value, fields: &std::collections::BTreeMap<String, toml::Value>) -> Result<(), String> {
    for (name, value) in fields {
        if name == "directories" {
            continue;
        }
        let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
        if name == "model" {
            if value.as_str().is_none_or(|name| name.trim().is_empty()) {
                return Err("Smith task override model requires a nonempty string".into());
            }
            settings["models"][0]["name"] = value;
            continue;
        }
        if value.as_u64().is_none() {
            return Err(format!("Smith task override {name} requires an unsigned integer"));
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
        settings["effort"] = json!(effort);
        settings["agent"]["trace"] = json!({"path":trace,"capture":"everything"});
        settings["directories"] = super::directories::render(context)?;
        settings["accounts"] = json!([{"number":0,"account_id":account_id}]);
        settings["token_directory"] = json!(tokens);
        settings["budget"] = budget(context.task, context.variant)?;
        let environment: Result<Vec<_>, String> = context
            .environment
            .iter()
            .map(|(name, value)| {
                Ok(json!({"name":name.to_str().ok_or("Smith environment name is not UTF-8")?,
                "value":value.to_str().ok_or("Smith environment value is not UTF-8")?}))
            })
            .collect();
        settings["agent"]["environment"] = json!(environment?);
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

        {
            let model = settings["models"][0].as_object_mut().expect("validated model");
            for rate in ["input_price", "cached_price", "output_price"] {
                let value = model[rate]
                    .as_u64()
                    .and_then(|value| value.checked_mul(1_000_000))
                    .ok_or("notional price cannot be scaled to micro-dollars")?;
                model.insert(rate.into(), json!(value));
            }
        }
        settings["sub_agents"] = json!(true);
        overrides(&mut settings, &context.task.smith)?;
        if let Some(variant) = context.variant {
            overrides(&mut settings, &variant.smith)?;
        }
        Ok(settings)
    })();
    render.map_err(configuration)
}

/// Two sequential native IO close deadlines bound the agent's teardown.
pub fn teardown(settings: &Value) -> Result<skein_lib::Duration, Setup> {
    let close = match settings.pointer("/agent/profile/policy/close") {
        Some(value) => value.as_u64(),
        None => Some(5000),
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

fn seconds(name: &str, value: &Value) -> Result<Value, Setup> {
    let milliseconds = value.as_u64().ok_or_else(|| configuration("pinned duration must be milliseconds"))?;
    if milliseconds % 1000 != 0 {
        return Err(configuration(format!("product duration override {name} requires whole seconds")));
    }
    Ok(json!(milliseconds / 1000))
}

fn dollars(value: &Value) -> Result<Value, Setup> {
    let micro = value.as_u64().ok_or_else(|| configuration("pinned price must be micro-dollars"))?;
    let decimal = format!("{}.{:06}", micro / 1_000_000, micro % 1_000_000);
    let dollars: f64 = decimal.parse().map_err(|_| configuration("pinned price exceeds dollars"))?;
    Ok(json!(dollars))
}

/// Render the pin's instructions, endpoint and task limits as a product user layer.
pub fn product(context: &Context<'_>, base: &Value, login: &Path) -> Result<Value, Setup> {
    let root = context.private.root.join("smith");
    let native = settings(context, base, "unused", &root.join("trace.jsonl"), &root.join("tokens"))?;
    let endpoint = &native["agent"]["endpoints"][0];
    let model = &native["models"][0];
    let endpoint_name = endpoint["name"].as_str().ok_or_else(|| configuration("pinned endpoint name"))?;
    let model_name = model["name"].as_str().ok_or_else(|| configuration("pinned model name"))?;
    let selected = format!("{endpoint_name}/{model_name}");
    let mut declared = model.clone();
    for field in ["endpoint", "name"] {
        declared.as_object_mut().expect("native model").remove(field);
    }
    for field in ["head", "idle"] {
        declared[field] = seconds(field, &model[field])?;
    }
    for field in ["input_price", "cached_price", "output_price"] {
        declared[field] = dollars(&model[field])?;
    }
    let headers: std::collections::BTreeMap<_, _> = endpoint["headers"]
        .as_array()
        .ok_or_else(|| configuration("pinned endpoint headers"))?
        .iter()
        .map(|header| {
            Ok((
                header["name"].as_str().ok_or_else(|| configuration("pinned header name"))?,
                header["value"].as_str().ok_or_else(|| configuration("pinned header value"))?,
            ))
        })
        .collect::<Result<_, Setup>>()?;
    let mut provider = json!({"preset":endpoint_name,"account":endpoint_name,"dialect":endpoint["provider"],
        "address":endpoint["address"],"identity":endpoint["identity"],"headers":headers,
        "connect":seconds("connect", &endpoint["connect"])? ,"handshake":seconds("handshake", &endpoint["handshake"])?});
    if let Some(server_name) = endpoint.get("server_name") {
        provider["server_name"] = server_name.clone();
    }
    let environment: std::collections::BTreeMap<_, _> = context
        .environment
        .iter()
        .map(|(name, value)| {
            Ok((
                name.to_str().ok_or_else(|| configuration("environment name must be UTF-8"))?,
                value.to_str().ok_or_else(|| configuration("environment value must be UTF-8"))?,
            ))
        })
        .collect::<Result<_, Setup>>()?;
    let mut deployment = native["agent"]["profile"]["declared"].clone();
    for (field, value) in native["agent"]["profile"]["policy"].as_object().expect("native policy") {
        // The historical process grace belongs to its local invocation; the product uses its preset.
        if field == "group_stop"
            && !context.task.smith.contains_key(field)
            && context.variant.is_none_or(|variant| !variant.smith.contains_key(field))
        {
            continue;
        }
        deployment[field] = match field.as_str() {
            "max_time" | "max_waiting" | "connection_keep" | "tool_deadline" | "shell_timeout" | "group_stop"
            | "close" | "write_deadline" | "cancel_grace" | "exit_grace" => seconds(field, value)?,
            "max_spend" => dollars(value)?,
            _ => value.clone(),
        };
    }
    let mut budget = native["budget"].clone();
    budget["spend"] = dollars(&budget["spend"])?;
    Ok(json!({"model":selected,"effort":native["effort"],"profile":"standard",
        "instructions":native["instructions"],"sub_agents":native["sub_agents"],"budget":budget,
        "deployment":deployment,"models":{selected:declared},"endpoints":{endpoint_name:provider},
        "accounts":{endpoint_name:{"source":"borrow","file":login,"format":"codex","program":"codex"}},
        "workspace":{"directories":native["directories"],"conventions":native["conventions"],"contract":native["contract"]},
        "environment":{"inherit":[],"set":environment}}))
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
        let refused_input = budget(&task, None).expect("product judges entrance reserve");
        assert_eq!(refused_input["turns"], 1);
        assert_eq!(refused_input["reserve"], json!({"turns":2,"seconds":300}));
    }

    #[test]
    fn native_shapes_keep_task_deadlines_environment_prices_and_named_overrides() {
        one_turn_refusal_reaches_the_product_with_the_default_reserve();
        native_shape_cases();
    }

    #[test]
    fn unknown_model_overrides_are_strings_and_reach_the_provider_declaration() {
        let mut settings = json!({"models":[{"name":"configured-model"}]});
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("model".into(), toml::Value::String("smith-bench-nonexistent-model".into()));
        overrides(&mut settings, &fields).expect("provider owns unknown model rejection");
        assert_eq!(settings["models"][0]["name"], "smith-bench-nonexistent-model");
        for value in [toml::Value::Integer(1), toml::Value::String(" ".into())] {
            fields.insert("model".into(), value);
            assert!(overrides(&mut settings, &fields).expect_err("model type/name").contains("model"));
        }
    }

    fn loopback_template() -> Value {
        let mut base: Value = serde_json::from_str(include_str!("../../../agents/smith/smoke.json")).expect("template");
        let endpoint = base["agent"]["endpoints"][0].as_object_mut().expect("fixture endpoint");
        endpoint.insert("address".into(), json!("127.0.0.1:443"));
        endpoint.insert("transport".into(), json!("plaintext"));
        endpoint.remove("server_name");
        base
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
        let base = loopback_template();
        let render = |task: &Task| {
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
                &base,
                "opaque-account",
                &root.join("trace"),
                &root.join("tokens"),
            )
        };
        task.budget.spend = Some(1.25);
        task.smith.insert("read_window".into(), toml::Value::Integer(4096));
        // This startup control declares its small task's actual payload and
        // call count, so current native InputAnswer/WindowFirst checks apply.
        task.smith.insert("tool_payload".into(), toml::Value::Integer(4096));
        task.smith.insert("calls_per_response".into(), toml::Value::Integer(1));
        task.smith.insert("file_bytes".into(), toml::Value::Integer(1_048_576));
        let current = render(&task).expect("current");
        assert_eq!(current["agent"]["profile"]["declared"]["read_window"], 4096);
        assert_eq!(current["agent"]["profile"]["declared"]["tool_payload"], 4096);
        assert_eq!(current["agent"]["profile"]["declared"]["calls_per_response"], 1);
        assert_eq!(current["models"][0]["input_price"], 10_000_000);
        assert_eq!(current["sub_agents"], true);
        assert_eq!(current["agent"]["environment"][0]["value"], "exact-value");
        assert_eq!(current["agent"]["trace"]["capture"], "everything");
        assert_eq!(current["in_process"], false);
        let local: smith_local_shell::local_settings::Settings =
            serde_json::from_value(current.clone()).expect("shipped local settings");
        let configured = native_agent_configuration(&local);
        native_failure_settings(&current, configured.profile.declared.file_bytes);
        native_parallel_profiles(&directory, &root, &render);
        task.smith.insert("output".into(), toml::Value::Integer(1024));
        let changed = render(&task).expect("declared output override");
        assert_eq!(changed["models"][0]["output"], 1024);
        assert!(changed["agent"]["endpoints"][0].get("models").is_none());
        let local = serde_json::from_value(changed).expect("changed local settings");
        let encoded = smith_local_shell::local_settings::agent_document(&local).expect("changed agent document");
        let agent: Value = serde_json::from_slice(&encoded).expect("generated JSON");
        assert_eq!(agent["endpoints"][0]["models"][0]["output"], 1024);
        assert_eq!(teardown(&current).expect("two close deadlines"), skein_lib::Duration::from_secs(10));
        task.smith.insert("read_window".into(), toml::Value::String("wrong".into()));
        assert!(render(&task).expect_err("wrong field type").reason.contains("read_window"));
        task.smith.clear();
        task.budget.seconds = Some(11);
        assert!(render(&task).is_err());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    fn native_failure_settings(base: &Value, file_bytes: u32) {
        assert_eq!(file_bytes, 1_048_576);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let task = crate::read_task(&root.join("tasks/probes/failures/task.toml")).expect("authored failures");
        for name in ["unknown-model", "derived-limit", "bad-setting"] {
            let task = task.with_variant(name).expect("variant");
            let mut settings = base.clone();
            overrides(&mut settings, &task.smith).expect("render native invalid input");
            let path = std::env::temp_dir().join(format!("smith-failure-settings-{}-{name}.json", std::process::id()));
            assert!(!path.exists(), "fixture path is owned by this test");
            std::fs::write(&path, serde_json::to_vec(&settings).expect("native JSON")).expect("private fixture");
            let local = smith_local_shell::local_settings::read(&path, None);
            std::fs::remove_file(path).expect("remove own input fixture");
            let result = local
                .and_then(|local| smith_local_shell::local_settings::agent_document(&local))
                .and_then(|encoded| smith_agent_shell::config::parse(&encoded));
            if name == "unknown-model" {
                let configured = result.expect("fully declared model reaches provider selection");
                assert_eq!(configured.service.limits.llm.adapter.client.detail_bytes, 1024);
                assert!(task.event.contains(&crate::EventCheck::ProviderFailure {
                    status_at_least: 400,
                    status_at_most: 499,
                    detail_bytes_at_most: configured.service.limits.llm.adapter.client.detail_bytes
                }));
            } else {
                let Err(error) = result else { panic!("native startup accepted the authored invalid setting") };
                assert!(error.contains(if name == "bad-setting" { "output" } else { "read-in-payload" }), "{error}");
            }
        }
    }
    fn native_parallel_profiles(directory: &Path, root: &Path, render: &impl Fn(&Task) -> Result<Value, Setup>) {
        let history = crate::read_task(&directory.join("tasks/probes/history/task.toml")).expect("history task");
        for task in [history.clone(), history.with_variant("small-window").expect("small window")] {
            let rendered = render(&task).expect("history settings");
            let path = root.join("history.json");
            std::fs::write(&path, serde_json::to_vec(&rendered).expect("native JSON")).expect("private fixture");
            let local = smith_local_shell::local_settings::read(&path, None).expect("native settings reader");
            let configured = native_agent_configuration(&local);
            smith_local_shell::local_settings::policy(
                &local,
                &configured.service.channel_endpoints,
                configured.service.limits.domain,
                vec![0; local.models.len()].into(),
            )
            .expect("native policy admits its finishing reserve");
            assert_eq!(configured.profile.declared.calls_per_response, 1);
        }
        for name in ["parallel-read", "parallel-children"] {
            let parallel = crate::read_task(&directory.join(format!("tasks/probes/{name}/task.toml")))
                .expect("authored parallel profile");
            let rendered = render(&parallel).expect("native parallel profile");
            let path = root.join(format!("{name}.json"));
            std::fs::write(&path, serde_json::to_vec(&rendered).expect("native JSON")).expect("private fixture");
            let local = smith_local_shell::local_settings::read(&path, None).expect("shipped public settings reader");
            let configured = native_agent_configuration(&local);
            assert_eq!(configured.profile.declared.calls_per_response, if name == "parallel-read" { 20 } else { 16 });
            assert_eq!(configured.profile.declared.conversations, if name == "parallel-read" { 1 } else { 5 });
        }
    }

    fn native_agent_configuration(
        local: &smith_local_shell::local_settings::Settings,
    ) -> smith_agent_shell::config::Configuration {
        let encoded = smith_local_shell::local_settings::agent_document(local).expect("native agent document");
        smith_agent_shell::config::parse(&encoded).expect("native declarations admitted")
    }
}
