//! Generate the agent's existing configuration values (shell.md, 5.4).
//! Profile, environment and endpoint values are built without opening credentials.
use super::super::{Refusal, Settings};
use super::{input, models::Model};
use serde_json::{Value, json};

pub(crate) fn profile(settings: &Settings) -> Result<Value, Refusal> {
    let mut document = json!({"name":input::text(settings,"profile")?,"declared":{},"policy":{}});
    let declared = [
        "conversations",
        "tool_payload",
        "file_bytes",
        "calls_per_response",
        "jobs",
        "read_window",
        "shell_output",
        "shell_head",
        "shell_tail",
        "search_hits",
        "search_bytes",
        "list_entries",
        "guide",
        "llm_pool",
        "memory",
    ];
    let durations = [
        "max_time",
        "max_waiting",
        "connection_keep",
        "tool_deadline",
        "shell_timeout",
        "job_poll",
        "job_wait",
        "check_timeout",
        "wrap_up",
        "group_stop",
        "close",
        "write_deadline",
        "cancel_grace",
        "exit_grace",
        "refresh_lead",
    ];
    for (key, sourced) in &settings.values {
        let Some(field) = key.strip_prefix("deployment.") else { continue };
        if field.starts_with("compaction_threshold.") {
            continue;
        }
        let part = if declared.contains(&field) { "declared" } else { "policy" };
        let value = if durations.contains(&field) {
            json!(input::millis(settings, key)?)
        } else if field == "max_spend" {
            json!(input::money(settings, key)?)
        } else {
            serde_json::to_value(&sourced.value).expect("schema value")
        };
        document
            .get_mut(part)
            .expect("profile part")
            .as_object_mut()
            .expect("profile object")
            .insert(field.into(), value);
    }
    let numerator = "deployment.compaction_threshold.numerator";
    let denominator = "deployment.compaction_threshold.denominator";
    if settings.values.contains_key(numerator) || settings.values.contains_key(denominator) {
        let default = smith_agent_service::profile::standard().policy.compaction_threshold;
        document.get_mut("policy").expect("policy").as_object_mut().expect("policy object").insert("compaction_threshold".into(), json!({
            "numerator":if settings.values.contains_key(numerator) {input::count(settings,numerator)?} else {default.numerator},
            "denominator":if settings.values.contains_key(denominator) {input::count(settings,denominator)?} else {default.denominator}}));
    }
    Ok(document)
}

pub(super) fn environment(settings: &Settings) -> Result<Vec<Value>, Refusal> {
    let values = super::super::environment::values(settings)?;
    Ok(values.into_iter().map(|(name, value)| json!({"name":name,"value":value})).collect())
}

pub(in crate::settings) fn endpoints(settings: &Settings, models: &[Model]) -> Result<Vec<Value>, Refusal> {
    let names: std::collections::BTreeSet<_> = models.iter().map(|model| model.endpoint.as_str()).collect();
    let mut endpoints = Vec::new();
    for (index, name) in names.into_iter().enumerate() {
        let prefix = format!("endpoints.{}.", super::super::name(name));
        let provider = input::text(settings, &format!("{prefix}dialect"))?;
        let dialect = match provider {
            "openai" => 3,
            "codex" => 1,
            "anthropic" => 2,
            _ => return Err(input::refuse(settings, &format!("{prefix}dialect"), "unsupported dialect")),
        };
        let headers: Vec<_> = settings.values.iter().filter_map(|(key,value)| {
            key.strip_prefix(&format!("{prefix}headers.")).map(|name| json!({"name":serde_json::from_str::<String>(name).unwrap_or_else(|_|name.into()),"value":value.value.as_str().expect("schema header")}))
        }).collect();
        let declared: Vec<_> = models.iter().filter(|model| model.endpoint == name).map(Model::document).collect();
        let mut endpoint = json!({"name":name,"number":u32::try_from(index).expect("bounded endpoints"),
            "dialect":dialect,"provider":provider,"transport":"tls",
            "address":input::text(settings,&format!("{prefix}address"))?,"server_name":input::text(settings,&format!("{prefix}server_name"))?,
            "connect":input::millis(settings,&format!("{prefix}connect"))?,"handshake":input::millis(settings,&format!("{prefix}handshake"))?,
            "models":declared,"headers":headers});
        for (setting, field) in [(&format!("{prefix}identity"), "identity"), (&format!("{prefix}program"), "program")] {
            if let Some(value) = settings.values.get(setting) {
                endpoint
                    .as_object_mut()
                    .expect("endpoint object")
                    .insert(field.into(), json!(value.value.as_str().expect("schema option")));
            }
        }
        let mut trust = json!({});
        for field in ["system", "der", "pem"] {
            if let Some(value) = settings.values.get(&format!("{prefix}trust.{field}")) {
                trust.as_object_mut().expect("trust object").insert(
                    field.into(),
                    if field == "system" {
                        json!(value.value.as_bool().expect("schema trust switch"))
                    } else {
                        let paths: Result<Vec<_>, Refusal> = value
                            .value
                            .as_array()
                            .expect("schema paths")
                            .iter()
                            .map(|path| {
                                settings
                                    .workspace
                                    .join(path.as_str().expect("schema path"))
                                    .to_str()
                                    .map(str::to_owned)
                                    .ok_or_else(|| {
                                        input::refuse(
                                            settings,
                                            &format!("{prefix}trust.{field}"),
                                            "expected UTF-8 path",
                                        )
                                    })
                            })
                            .collect();
                        json!(paths?)
                    },
                );
            }
        }
        if !trust.as_object().expect("trust object").is_empty() {
            endpoint.as_object_mut().expect("endpoint object").insert("trust".into(), trust);
        }
        endpoints.push(endpoint);
    }
    Ok(endpoints)
}
