//! Local-host settings, read before a chat opens (protocol/hosts.md, section
//! 5.2). The shell keeps the operator's JSON and an optional workspace
//! override. Neither file is a transcript or a credential store. The merged
//! agent object is written as the child agent's configuration at startup.

use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;
#[cfg(test)]
use skein_lib::List;
use skein_lib::{Duration, Token};
#[cfg(test)]
use smith_charter as wire;
use smith_domain::{self as domain, run};
use smith_local_domain as local;
use smith_protocol_channel as channel;

const SETTINGS_BYTES: u64 = 1 << 20;

/// One priced model from a named endpoint in the generated agent config.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub endpoint: String,
    pub name: String,
    pub window: u32,
    pub output: u32,
    pub reasoning_item: u32,
    #[serde(default)]
    pub oversized_reasoning: Option<String>,
    /// Progress durations in milliseconds.
    pub head: u64,
    pub idle: u64,
    pub input_price: u64,
    pub cached_price: u64,
    /// Separate positive cache-write rate; omitted writes use `input_price`.
    #[serde(default)]
    pub cache_write_price: Option<u64>,
    pub output_price: u64,
    pub price_unit: u32,
}

/// Host-selected command and result bounds for one activation.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub turns: u32,
    pub spend: u64,
    pub seconds: u64,
    #[serde(default = "default_reserve")]
    pub reserve: Reserve,
}

/// Finishing allowance in local settings; the host supplies the product default.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reserve {
    pub turns: u32,
    pub seconds: u64,
}

fn default_reserve() -> Reserve {
    Reserve { turns: 2, seconds: 300 }
}

/// One configured directory mounted into a local run.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Directory {
    pub name: String,
    pub path: String,
    pub writable: bool,
    pub git: bool,
}

/// A required field in a change result or mid-run delivery.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub max: u32,
}

/// Configured final result form; report is the default when omitted.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "form", rename_all = "snake_case", deny_unknown_fields)]
pub enum Contract {
    Report { max: u32 },
    Change { checks_must_pass: bool, fields: Vec<Field> },
}

/// Paths used by guide and check operations in the agent workspace.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conventions {
    pub guide: String,
    pub checks: String,
}

/// Host-authored context sent in order with every activation.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub title: String,
    pub text: String,
}

/// Local settings after the optional workspace override is merged.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub agent: Value,
    pub chat: String,
    pub instructions: String,
    pub models: Vec<Model>,
    /// Whether main may delegate; omitted settings keep children disabled.
    #[serde(default)]
    pub sub_agents: bool,
    pub budget: Budget,
    pub waiting_seconds: u64,
    #[serde(default)]
    pub brief: Vec<Section>,
    #[serde(default)]
    pub directories: Vec<Directory>,
    #[serde(default)]
    pub conventions: Option<Conventions>,
    #[serde(default)]
    pub contract: Option<Contract>,
    #[serde(default)]
    pub deliver: Option<Vec<Field>>,
    #[serde(default = "default_title")]
    pub title_field: String,
    #[serde(default)]
    pub delivery_environment: Vec<String>,
    #[serde(default)]
    pub in_process: bool,
    #[serde(default)]
    pub token_directory: Option<String>,
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub push: Option<Vec<Option<Push>>>,
}

/// A configured OAuth account's opaque identifier for the agent envelope.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub number: u32,
    pub account_id: String,
    #[serde(default)]
    pub oauth: Option<OAuth>,
}

/// Public-client registration and the token endpoint's startup destination.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OAuth {
    pub authorization_url: String,
    pub token_endpoint: String,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scope: String,
    pub address: String,
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub trust_der: Option<String>,
    #[serde(default)]
    pub json: bool,
}

/// One explicitly configured push destination for local git deliveries.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Push {
    pub remote: String,
    pub branch: String,
}

/// Build bounded local policy and parallel workspace paths from settings.
pub struct Prepared {
    pub config: local::Config,
    pub limits: local::Limits,
    pub paths: Box<[Box<[u8]>]>,
}

#[expect(
    clippy::too_many_lines,
    reason = "10-product replaces JSON local policy construction with resolved TOML settings"
)]
pub fn policy(
    settings: &Settings,
    endpoints: &channel::Endpoints,
    agent_limits: domain::Limits,
) -> Result<Prepared, String> {
    run::Budget {
        turns: settings.budget.turns,
        spend: settings.budget.spend,
        time: Duration::from_secs(settings.budget.seconds),
    }
    .check(&agent_limits.run.budget)
    .map_err(budget_refusal)?;
    if Duration::from_secs(settings.waiting_seconds) > agent_limits.run.waiting {
        return Err(format!(
            "waiting_seconds exceeds the agent's ceiling of {} seconds",
            agent_limits.run.waiting.as_nanos() / 1_000_000_000
        ));
    }
    check_prices(settings)?;
    let mut models = Vec::with_capacity(settings.models.len());
    let mut accounts = Vec::new();
    for model in &settings.models {
        let endpoint = endpoints.resolve(model.endpoint.as_bytes()).ok_or("local model endpoint is absent")?;
        if !accounts.contains(&endpoint.account) {
            accounts.push(endpoint.account);
        }
        models.push(run::charter::Llm {
            prices: run::Prices {
                input: model.input_price,
                cached: model.cached_price,
                cache_write: model.cache_write_price,
                output: model.output_price,
                unit: model.price_unit,
            },
            dialect: endpoint.dialect,
            account: endpoint.account,
            endpoint: run::charter::Endpoint(endpoint.number),
            model: model.name.as_bytes().into(),
            window: model.window,
            output: model.output,
        });
    }
    let mut paths = Vec::with_capacity(settings.directories.len());
    let mut directories = Vec::with_capacity(settings.directories.len());
    for (index, directory) in settings.directories.iter().enumerate() {
        let number = u64::try_from(index)
            .map_err(|_| "too many workspace directories")?
            .checked_add(1)
            .ok_or("directory token overflow")?;
        let absolute =
            std::path::absolute(Path::new(&directory.path)).map_err(|_| "workspace host path is unavailable")?;
        let host = smith_protocol_channel::host_path(
            absolute.as_os_str().as_encoded_bytes(),
            agent_limits.run.host_path_bytes,
        )
        .ok_or("workspace host path exceeds its bound")?;
        paths.push(host.clone());
        directories.push(run::Directory {
            host,
            name: directory.name.as_bytes().into(),
            root: Token::new(number),
            writable: directory.writable,
            git: directory.git,
            conflicts: Box::new([]),
        });
    }
    let workspace =
        if directories.is_empty() { None } else { Some(run::Workspace { directories: directories.into() }) };
    let mut brief = Vec::with_capacity(settings.brief.len());
    for section in &settings.brief {
        brief.push(run::charter::Section {
            title: section.title.as_bytes().into(),
            text: section.text.as_bytes().into(),
        });
    }
    let contract = match &settings.contract {
        Some(Contract::Change { checks_must_pass, fields }) => local::Contract::Change(run::outcome::ChangeSpec {
            checks_must_pass: *checks_must_pass,
            fields: fields_of(fields),
        }),
        Some(Contract::Report { max }) => {
            local::Contract::Report(run::outcome::TextSpec { max: *max, fields: Box::new([]) })
        }
        None => local::Contract::Report(run::outcome::TextSpec { max: 4096, fields: Box::new([]) }),
    };
    let deliver = settings
        .deliver
        .as_ref()
        .map(|fields| run::outcome::ChangeSpec { checks_must_pass: true, fields: fields_of(fields) });
    let policy = local::Config {
        chat: settings.chat.as_bytes().into(),
        instructions: settings.instructions.as_bytes().into(),
        brief: run::charter::Brief { sections: brief.into() },
        models: models.into(),
        sub_agents: settings.sub_agents,
        progress: settings
            .models
            .iter()
            .map(|model| smith_domain::session::Progress {
                head: Duration::from_millis(model.head),
                idle: Duration::from_millis(model.idle),
            })
            .collect(),
        budget: run::Budget {
            turns: settings.budget.turns,
            spend: settings.budget.spend,
            time: Duration::from_secs(settings.budget.seconds),
        },
        reserve: run::charter::Reserve {
            turns: settings.budget.reserve.turns,
            time: Duration::from_secs(settings.budget.reserve.seconds),
        },
        conventions: settings.conventions.as_ref().map(|value| run::Conventions {
            guide: value.guide.as_bytes().into(),
            checks: value.checks.as_bytes().into(),
        }),
        contract,
        deliver,
        title_field: settings.title_field.as_bytes().into(),
        waiting: Duration::from_secs(settings.waiting_seconds),
        resume: true,
        accounts: accounts.into(),
        workspace,
        push: settings.push.as_ref().map(|targets| {
            targets
                .iter()
                .map(|target| {
                    target.as_ref().map(|target| local::PushTarget {
                        remote: target.remote.as_bytes().into(),
                        branch: target.branch.as_bytes().into(),
                    })
                })
                .collect()
        }),
    };
    let mut configured_endpoints = Vec::with_capacity(settings.models.len());
    for model in &settings.models {
        let endpoint = endpoints.resolve(model.endpoint.as_bytes()).expect("checked endpoint");
        let name = run::charter::Endpoint(endpoint.number);
        if !configured_endpoints.contains(&name) {
            configured_endpoints.push(name);
        }
    }
    let limits = local::Limits {
        endpoints: configured_endpoints.into(),
        smith: agent_limits,
        agent: local::AgentLimits::inline(agent_limits, 2, policy.budget.time).ok_or("local inline bounds overflow")?,
        chat_bytes: 256,
        text_bytes: 1 << 16,
        models: 3,
        line_bytes: local::max_line_bytes(agent_limits.run.message_bytes)
            .ok_or("local message bound cannot hold its label")?,
        show_bytes: 8192,
        lines: 8,
        unsaved: 2,
        facts: 1024,
    };
    policy.validate(&limits).map_err(|error| match error {
        local::Invalid::Budget(reason) => budget_refusal(reason),
        other @ (local::Invalid::Price(_)
        | local::Invalid::Chat
        | local::Invalid::Text
        | local::Invalid::Models
        | local::Invalid::Accounts
        | local::Invalid::Endpoint
        | local::Invalid::Limits
        | local::Invalid::Contract
        | local::Invalid::Push) => format!("local policy: {other:?}"),
    })?;
    Ok(Prepared { config: policy, limits, paths: paths.into() })
}

fn check_prices(settings: &Settings) -> Result<(), String> {
    for (index, model) in settings.models.iter().enumerate() {
        for (key, value) in [
            ("input_price", model.input_price),
            ("cached_price", model.cached_price),
            ("cache_write_price", model.cache_write_price.unwrap_or(model.input_price)),
            ("output_price", model.output_price),
        ] {
            if value == 0 {
                return Err(format!("models[{index}].{key} must be positive"));
            }
        }
    }
    Ok(())
}

fn budget_refusal(reason: run::BudgetRefusal) -> String {
    match reason {
        run::BudgetRefusal::Ceiling(ceiling) => match ceiling {
            run::Ceiling::Turns(most) => format!("budget.turns exceeds the agent's ceiling of {most}"),
            run::Ceiling::Spend(most) => format!("budget.spend exceeds the agent's ceiling of {most} micro-dollars"),
            run::Ceiling::Time(most) => {
                format!("budget.seconds exceeds the agent's ceiling of {} seconds", most.as_nanos() / 1_000_000_000)
            }
        },
        run::BudgetRefusal::Empty(part) => format!("budget.{} must be positive", budget_key(part)),
        run::BudgetRefusal::Reserve(part) => format!("budget.{} cannot hold the finishing reserve", budget_key(part)),
        run::BudgetRefusal::Completion { most } => {
            format!("budget.spend cannot pay for a completion costing {most} micro-dollars")
        }
        run::BudgetRefusal::Checks { reserve, checks } => format!(
            "budget.reserve.seconds is {} seconds, below check_timeout of {} seconds",
            reserve.as_nanos() / 1_000_000_000,
            checks.as_nanos() / 1_000_000_000
        ),
    }
}

fn budget_key(part: run::Part) -> &'static str {
    match part {
        run::Part::Turns => "turns",
        run::Part::Spend => "spend",
        run::Part::Time => "seconds",
    }
}

fn fields_of(source: &[Field]) -> Box<[run::outcome::FieldRule]> {
    source.iter().map(|field| run::outcome::FieldRule { name: field.name.as_bytes().into(), max: field.max }).collect()
}

fn default_title() -> String {
    "title".into()
}

/// Read bounded global settings and a workspace override, replacing only
/// keys named by the latter. Objects merge recursively; lists replace whole.
pub fn read(global: &Path, workspace: Option<&Path>) -> Result<Settings, String> {
    let mut document = read_document(global)?;
    if let Some(path) = workspace {
        let override_document = read_document(path)?;
        merge(&mut document, override_document);
    }
    let settings: Settings = serde_json::from_value(document).map_err(|error| format!("local settings: {error}"))?;
    check_prices(&settings)?;
    if settings.chat.is_empty()
        || settings.chat == "."
        || settings.chat == ".."
        || settings.chat.contains(['/', '\\', '\0'])
        || settings.models.is_empty()
        || settings.models.iter().any(|model| {
            model.endpoint.is_empty()
                || model.name.is_empty()
                || model.price_unit == 0
                || model.window == 0
                || model.output == 0
                || model.reasoning_item == 0
                || !matches!(model.oversized_reasoning.as_deref(), None | Some("fail" | "drop"))
                || model.head == 0
                || model.idle == 0
        })
        || settings.budget.turns == 0
        || settings.budget.seconds == 0
        || settings.waiting_seconds == 0
        || settings.title_field.is_empty()
    {
        return Err("local settings contain an empty or zero required value".into());
    }
    if !settings.agent.is_object() {
        return Err("local agent configuration must be an object".into());
    }
    Ok(settings)
}

/// Generate every served model declaration from the local host's priced models.
/// Progress durations are milliseconds in this generated JSON.
pub fn agent_document(settings: &Settings) -> Result<Vec<u8>, String> {
    let mut agent = settings.agent.clone();
    let endpoints = agent.get_mut("endpoints").and_then(Value::as_array_mut).ok_or("agent endpoints must be a list")?;
    let mut declared = std::collections::BTreeSet::new();
    for endpoint in endpoints {
        let name = endpoint.get("name").and_then(Value::as_str).ok_or("agent endpoint name is required")?.to_owned();
        let mut models = Vec::new();
        for model in &settings.models {
            if model.endpoint == name {
                if !declared.insert((model.endpoint.clone(), model.name.clone())) {
                    return Err("duplicate local model declaration".into());
                }
                models.push(serde_json::json!({"name":model.name,"window":model.window,"output":model.output,
                    "reasoning_item":model.reasoning_item,"oversized_reasoning":model.oversized_reasoning,"head":model.head,"idle":model.idle}));
            }
        }
        if models.is_empty() {
            return Err(format!("endpoint {name:?} has no declared local model"));
        }
        let object = endpoint.as_object_mut().ok_or("agent endpoint must be an object")?;
        if object.contains_key("models") {
            return Err("local agent endpoint models come from settings.models".into());
        }
        object.insert("models".into(), Value::Array(models));
    }
    if declared.len() != settings.models.len() {
        return Err("local model names an undeclared endpoint".into());
    }
    serde_json::to_vec(&agent).map_err(|error| format!("generated agent configuration JSON: {error}"))
}

fn read_document(path: &Path) -> Result<Value, String> {
    let size = fs::metadata(path).map_err(|error| format!("settings metadata {}: {error}", path.display()))?.len();
    if size > SETTINGS_BYTES {
        return Err(format!("settings {} exceed {SETTINGS_BYTES} bytes", path.display()));
    }
    let bytes = fs::read(path).map_err(|error| format!("settings read {}: {error}", path.display()))?;
    if u64::try_from(bytes.len()).expect("length fits u64") > SETTINGS_BYTES {
        return Err(format!("settings {} exceed {SETTINGS_BYTES} bytes", path.display()));
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("settings JSON {}: {error}", path.display()))
}

fn merge(base: &mut Value, override_value: Value) {
    match (base, override_value) {
        (Value::Object(base), Value::Object(overrides)) => {
            for (name, replacement) in overrides {
                match base.get_mut(&name) {
                    Some(current) => merge(current, replacement),
                    None => {
                        base.insert(name, replacement);
                    }
                }
            }
        }
        (base, replacement) => *base = replacement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_replaces_only_its_named_settings_and_agent_fields() {
        let mut document = serde_json::json!({
            "agent": {"profile":{"name":"standard", "declared":{"memory":42}}, "environment":[]},
            "chat":"one", "instructions":"global", "models":[{"endpoint":"main","name":"small","window":8192,"output":32,"reasoning_item":2048,"head":60000,"idle":30000,
                "input_price":1,"cached_price":1,"output_price":1,"price_unit":1}],
            "budget":{"turns":8,"spend":100,"seconds":60}, "waiting_seconds":30
        });
        merge(
            &mut document,
            serde_json::json!({"instructions":"workspace", "agent":{"profile":{"declared":{"memory":99}}}}),
        );
        let settings: Settings = serde_json::from_value(document.clone()).expect("merged settings");
        assert!(!settings.sub_agents, "omitted settings do not grant sub-agents");
        assert_eq!(settings.budget.reserve.turns, 2);
        assert_eq!(settings.budget.reserve.seconds, 300);
        assert_eq!(settings.instructions, "workspace");
        assert_eq!(settings.agent["profile"]["name"], "standard");
        assert_eq!(settings.agent["profile"]["declared"]["memory"], 99);
        assert_eq!(settings.models[0].name, "small");
        merge(&mut document, serde_json::json!({"sub_agents":true}));
        let enabled: Settings = serde_json::from_value(document).expect("enabled sub-agent setting");
        assert!(enabled.sub_agents);
    }

    #[test]
    fn invalid_model_and_zero_budget_are_refused_before_a_chat_opens() {
        let path = std::env::temp_dir().join(format!("smith-local-settings-{}", std::process::id()));
        let document = serde_json::json!({"agent":{},"chat":"one","instructions":"x",
            "models":[{"endpoint":"","name":"small","window":8192,"output":32,"reasoning_item":2048,"head":60000,"idle":30000,"input_price":1,"cached_price":1,
                "output_price":1,"price_unit":1}],"budget":{"turns":0,"spend":1,"seconds":60},"waiting_seconds":30});
        fs::write(&path, serde_json::to_vec(&document).expect("JSON")).expect("settings file");
        assert!(read(&path, None).is_err());
        fs::remove_file(path).expect("remove settings file");
    }

    #[test]
    fn a_zero_model_rate_names_its_settings_key_before_agent_preparation() {
        for key in ["input_price", "cached_price", "cache_write_price", "output_price"] {
            let path = std::env::temp_dir().join(format!("smith-zero-price-{}-{key}", std::process::id()));
            let mut document = serde_json::json!({"agent":{},"chat":"one","instructions":"Assist",
                "models":[{"endpoint":"main","name":"gpt-5","window":8192,"output":32,"reasoning_item":2048,
                    "head":60000,"idle":30000,"input_price":1_250_000,"cached_price":125_000,
                    "output_price":10_000_000,"price_unit":1_000_000}],
                "budget":{"turns":512,"spend":25_000_000,"seconds":3600},"waiting_seconds":30});
            document["models"][0][key] = serde_json::json!(0);
            fs::write(&path, serde_json::to_vec(&document).expect("JSON")).expect("settings file");
            assert_eq!(read(&path, None).expect_err("zero rate"), format!("models[0].{key} must be positive"));
            fs::remove_file(path).expect("remove settings file");
        }
    }

    fn admission_limits(settings: &Settings) -> domain::Limits {
        smith_agent_service::profile::derive(
            &{
                let mut profile = smith_agent_service::profile::standard();
                profile.declared.memory = Some(u64::MAX);
                profile
            },
            &smith_agent_service::profile::Configuration {
                environment_bytes: 0,
                endpoints: Box::new([smith_agent_service::profile::Endpoint {
                    number: 7,
                    connect: Duration::from_secs(10),
                    handshake: Duration::from_secs(10),
                    models: settings
                        .models
                        .iter()
                        .map(|model| smith_agent_service::profile::Model {
                            name: model.name.as_bytes().into(),
                            window: model.window,
                            output: model.output,
                            reasoning_item: model.reasoning_item,
                            oversized_reasoning: smith_agent_service::profile::OversizedReasoning::Fail,
                            head: Duration::from_millis(model.head),
                            idle: Duration::from_millis(model.idle),
                        })
                        .collect(),
                }]),
            },
        )
        .expect("profile derives")
        .domain
    }

    fn assert_sub_agent_grant(
        settings: &Settings,
        endpoints: &channel::Endpoints,
        limits: &domain::Limits,
        previous: &run::charter::Charter,
    ) {
        let enabled = policy(settings, endpoints, *limits).expect("enabled sub-agent policy");
        let encoded = channel::encode_charter(&local::charter(&enabled.config), &wire::CEILINGS, endpoints)
            .expect("enabled charter");
        let decoded = channel::decode_charter(&encoded, &wire::CEILINGS, endpoints).expect("enabled wire grant");
        assert_eq!(decoded.reserve, previous.reserve);
        assert!(decoded.grants.agents);
        assert_eq!(decoded.llm, previous.llm);
        assert!(decoded.models.is_empty(), "an unnamed child uses main's model");
    }

    #[test]
    fn priced_local_policy_round_trips_through_the_child_charter() {
        let mut entries = List::with_capacity(1);
        entries
            .push(channel::Endpoint { name: Box::from(&b"main"[..]), number: 7, dialect: 1, account: 0 })
            .expect("one endpoint");
        let endpoints = channel::Endpoints::new(entries);
        let mut settings = Settings {
            agent: serde_json::json!({}),
            chat: "one".into(),
            instructions: "Assist".into(),
            models: vec![Model {
                endpoint: "main".into(),
                name: "small".into(),
                window: 8192,
                output: 32,
                reasoning_item: 2048,
                oversized_reasoning: None,
                head: 60_000,
                idle: 30_000,
                input_price: 1_250_000,
                cached_price: 125_000,
                cache_write_price: Some(2_500_000),
                output_price: 10_000_000,
                price_unit: 1_000_000,
            }],
            sub_agents: false,
            budget: Budget { turns: 512, spend: 25_000_000, seconds: 3600, reserve: default_reserve() },
            waiting_seconds: 30,
            brief: vec![],
            directories: vec![],
            conventions: None,
            contract: None,
            deliver: None,
            title_field: "title".into(),
            delivery_environment: vec![],
            in_process: false,
            token_directory: None,
            accounts: vec![],
            push: None,
        };
        let agent_limits = admission_limits(&settings);
        for (key, most) in [
            ("turns", u64::from(agent_limits.run.budget.turns)),
            ("spend", agent_limits.run.budget.spend),
            ("seconds", agent_limits.run.budget.time.as_nanos() / 1_000_000_000),
        ] {
            let mut exceeded = settings.clone();
            match key {
                "turns" => exceeded.budget.turns = u32::try_from(most + 1).expect("bounded turns"),
                "spend" => exceeded.budget.spend = most + 1,
                "seconds" => exceeded.budget.seconds = most + 1,
                _ => unreachable!("test keys"),
            }
            let refused = policy(&exceeded, &endpoints, agent_limits).err().expect("past ceiling");
            assert!(refused.contains(&format!("budget.{key}")) && refused.contains(&most.to_string()), "{refused}");
        }
        for key in ["input_price", "cached_price", "cache_write_price", "output_price"] {
            let mut unpriced = settings.clone();
            match key {
                "input_price" => unpriced.models[0].input_price = 0,
                "cached_price" => unpriced.models[0].cached_price = 0,
                "cache_write_price" => unpriced.models[0].cache_write_price = Some(0),
                "output_price" => unpriced.models[0].output_price = 0,
                _ => unreachable!("test keys"),
            }
            let refused = policy(&unpriced, &endpoints, agent_limits).err().expect("zero rate");
            assert_eq!(refused, format!("models[0].{key} must be positive"));
        }
        let prepared = policy(&settings, &endpoints, agent_limits).expect("bounded policy");
        assert!(prepared.paths.is_empty());
        assert_eq!(prepared.limits.endpoints.as_ref(), &[run::charter::Endpoint(7)]);
        let encoded = channel::encode_charter(&local::charter(&prepared.config), &wire::CEILINGS, &endpoints)
            .expect("matching charter");
        assert!(!encoded.is_empty());
        let decoded = channel::decode_charter(&encoded, &wire::CEILINGS, &endpoints).expect("version two charter");
        assert_eq!((decoded.llm.window, decoded.llm.output), (8192, 32));
        assert_eq!(decoded.llm.prices.cache_write, settings.models[0].cache_write_price);
        assert!(!decoded.grants.agents);
        settings.sub_agents = true;
        assert_eq!(decoded.reserve, run::charter::Reserve { turns: 2, time: Duration::from_secs(300) });
        assert_sub_agent_grant(&settings, &endpoints, &agent_limits, &decoded);
        settings.directories = vec![Directory { name: "work".into(), path: ".".into(), writable: true, git: false }];
        let prepared = policy(&settings, &endpoints, agent_limits).expect("relative host path remains usable");
        let mounted = prepared.config.workspace.as_ref().expect("workspace");
        assert_eq!(mounted.directories[0].host, prepared.paths[0]);
        assert_eq!(mounted.directories[0].host.first(), Some(&b'/'));
    }
}
