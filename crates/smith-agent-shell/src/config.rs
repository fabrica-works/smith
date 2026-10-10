//! JSON configuration read once before the agent opens its channel.
//! It retains endpoint names and destinations, limits, command environment,
//! and future trace settings; no credential is read here.
//! Durations in JSON are milliseconds. Contract: protocol/agent.md, sections 4 and 6.

use std::fs;
use std::io::Read;
use std::net::{SocketAddr, ToSocketAddrs};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use serde::Deserialize;
use skein_channel::StreamMode;
use skein_http::Header;
use skein_lib::{Duration, List};
use skein_llm as shared;
use skein_llm_connection as connection;
use skein_tls as tls;
use smith_agent_service as service;
use smith_domain::{self as domain, run};
use smith_protocol_channel as channel;
use smith_protocol_llm as llm;
use smith_protocol_machine as machine;

use crate::trace::{Capture, TraceConfig};
use smith_agent_service::profile::{ACCOUNTS, ENDPOINTS, Layer, LimitName, Per, Refusal};

const CONFIG_BYTES: u64 = 1 << 20;

/// Parsed startup configuration and the service it can build.
pub struct Configuration {
    pub service: service::Config,
    pub memory: u64,
    pub trace: Option<TraceConfig>,
    pub profile: service::profile::Profile,
    pub declarations: service::profile::Configuration,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    profile: ProfileDocument,
    endpoints: Vec<Endpoint>,
    environment: Vec<Variable>,
    #[serde(default)]
    trace: Option<TraceDocument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileDocument {
    name: String,
    #[serde(default)]
    declared: DeclaredOverrides,
    #[serde(default)]
    policy: PolicyOverrides,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclaredOverrides {
    conversations: Option<u32>,
    tool_payload: Option<u32>,
    file_bytes: Option<u32>,
    calls_per_response: Option<u32>,
    read_window: Option<u32>,
    shell_output: Option<u32>,
    shell_head: Option<u32>,
    shell_tail: Option<u32>,
    search_hits: Option<u32>,
    search_bytes: Option<u32>,
    list_entries: Option<u32>,
    guide: Option<u32>,
    llm_pool: Option<u64>,
    memory: Option<u64>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyOverrides {
    max_turns: Option<u32>,
    max_spend: Option<u64>,
    max_time: Option<u64>,
    max_waiting: Option<u64>,
    sections: Option<u32>,
    host_tools: Option<u32>,
    verdicts: Option<u32>,
    items: Option<u32>,
    fields: Option<u32>,
    inbox: Option<u32>,
    unacknowledged: Option<u32>,
    compaction_threshold: Option<FractionDocument>,
    connection_keep: Option<u64>,
    tool_deadline: Option<u64>,
    shell_timeout: Option<u64>,
    group_stop: Option<u64>,
    close: Option<u64>,
    write_deadline: Option<u64>,
    cancel_grace: Option<u64>,
    exit_grace: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FractionDocument {
    numerator: u32,
    denominator: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelDocument {
    name: String,
    window: u32,
    output: u32,
    reasoning_item: u32,
    #[serde(default)]
    oversized_reasoning: Option<String>,
    head: u64,
    idle: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceDocument {
    path: String,
    capture: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    connect: u64,
    handshake: u64,
    models: Vec<ModelDocument>,
    name: String,
    number: u32,
    dialect: u32,
    account: u32,
    provider: String,
    address: String,
    #[serde(default = "tls_transport")]
    transport: String,
    #[serde(default)]
    server_name: Option<String>,
    #[serde(default)]
    authority: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    trust_der: Option<String>,
    #[serde(default)]
    headers: Vec<HeaderField>,
    #[serde(default)]
    reasoning_effort: Option<String>,
    #[serde(default)]
    identity: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeaderField {
    name: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Variable {
    name: String,
    value: String,
}

/// Read one JSON document and reject unknown keys, oversized input and
/// invalid transport settings before any channel byte is read.
pub fn read(path: &Path) -> Result<Configuration, String> {
    let size = fs::metadata(path).map_err(|error| format!("configuration metadata: {error}"))?.len();
    if size > CONFIG_BYTES {
        return Err(format!("configuration is larger than {CONFIG_BYTES} bytes"));
    }
    let bytes = fs::read(path).map_err(|error| format!("configuration read: {error}"))?;
    if u64::try_from(bytes.len()).expect("byte length fits u64") > CONFIG_BYTES {
        return Err(format!("configuration is larger than {CONFIG_BYTES} bytes"));
    }
    parse(&bytes)
}

/// Validate generated child configuration before the local host spawns it.
pub fn parse(bytes: &[u8]) -> Result<Configuration, String> {
    if u64::try_from(bytes.len()).expect("byte length fits u64") > CONFIG_BYTES {
        return Err(format!("configuration is larger than {CONFIG_BYTES} bytes"));
    }
    let document: Document = serde_json::from_slice(bytes).map_err(|error| format!("configuration JSON: {error}"))?;
    build(document)
}

// The exact encoded count supplies machine.env_bytes before the shared encoder
// validates names. Contract: protocol/limits.md, sections 3.5 and 4.
fn environment_size_refusal() -> String {
    profile_refusal(Refusal::Derived { limit: LimitName::deployment(Layer::Machine, "env_bytes") })
}

fn environment_refusal(error: machine::Unencodable) -> String {
    match error {
        machine::Unencodable::Variable { index } | machine::Unencodable::Repeated { index } => {
            profile_refusal(Refusal::Environment { index })
        }
        // This configuration's exact count derives the receiving bound; a
        // smaller bound would mean the derived field cannot hold that count.
        machine::Unencodable::TooLarge { .. } => environment_size_refusal(),
    }
}

fn build(document: Document) -> Result<Configuration, String> {
    let profile = build_profile(&document.profile)?;
    let memory = profile.declared.memory.ok_or("profile.declared.memory is required")?;
    if document.endpoints.is_empty() {
        return Err(profile_refusal(Refusal::Field { limit: LimitName::deployment(Layer::Declared, "endpoints") }));
    }
    if document.endpoints.len() > usize::try_from(ENDPOINTS).expect("endpoint count fits usize") {
        return Err("too many endpoints".into());
    }
    let trace = match document.trace {
        Some(trace) => {
            if trace.path.is_empty() {
                return Err("trace path is empty".into());
            }
            let capture = match trace.capture.as_str() {
                "none" => Capture::None,
                "calls" => Capture::Calls,
                "everything" => Capture::Everything,
                _ => return Err("trace capture must be none, calls or everything".into()),
            };
            Some(TraceConfig { path: trace.path.into(), capture })
        }
        None => None,
    };
    let mut environment = Vec::with_capacity(document.environment.len());
    let mut environment_bytes = 0_u64;
    for variable in document.environment {
        let entry_bytes = variable
            .name
            .len()
            .checked_add(variable.value.len())
            .and_then(|bytes| bytes.checked_add(1))
            .ok_or_else(environment_size_refusal)?;
        environment_bytes = environment_bytes
            .checked_add(u64::try_from(entry_bytes).map_err(|_| environment_size_refusal())?)
            .ok_or_else(environment_size_refusal)?;
        environment.push(machine::Variable {
            name: variable.name.into_bytes().into(),
            value: variable.value.into_bytes().into(),
        });
    }
    let mut declarations = Vec::with_capacity(document.endpoints.len());
    for endpoint in &document.endpoints {
        declarations.push(build_models(endpoint)?);
    }
    let declarations = service::profile::Configuration {
        environment_bytes: u32::try_from(environment_bytes).map_err(|_| environment_size_refusal())?,
        endpoints: declarations.into_boxed_slice(),
    };
    let limits = service::profile::derive(&profile, &declarations).map_err(profile_refusal)?;
    let environment = machine::Environment::encode(&environment, &limits.machine).map_err(environment_refusal)?;
    let endpoints = build_endpoints(document.endpoints, &limits.llm, &profile)?;
    let llm_endpoints =
        llm::Endpoints::new(endpoints.llm, ENDPOINTS, ACCOUNTS).map_err(|error| format!("LLM endpoints: {error:?}"))?;
    let mut models = Vec::new();
    for endpoint in &declarations.endpoints {
        for model in &endpoint.models {
            models.push(domain::ConfiguredModel {
                endpoint: run::charter::Endpoint(endpoint.number),
                model: model.name.clone(),
                window: model.window,
                output: model.output,
                protocol_allowance: service::profile::endpoint_allowance(
                    &profile,
                    endpoint,
                    &limits.domain.session.tools,
                )
                .map_err(profile_refusal)?,
                head: model.head,
                idle: model.idle,
            });
        }
    }
    let config = service::Config {
        limits,
        domain: domain::Config { endpoints: endpoints.domain, models: models.into_boxed_slice() },
        channel_endpoints: endpoints.channel,
        llm_endpoints,
        environment,
        stream_mode: StreamMode::Two,
        capture_prompts: match &trace {
            Some(trace) => trace.capture == Capture::Everything,
            None => false,
        },
    };
    let reserve = if trace.is_some() { crate::trace::MEMORY_RESERVE } else { 0 };
    let complete = service::worst_case(&config.limits, &config.llm_endpoints)
        .and_then(|bytes| bytes.checked_add(reserve))
        .ok_or_else(|| {
            profile_refusal(Refusal::Derived { limit: LimitName::deployment(Layer::Service, "worst_case") })
        })?;
    service::profile::validate::memory(complete, memory).map_err(profile_refusal)?;
    Ok(Configuration { service: config, memory, trace, profile, declarations })
}

struct PreparedEndpoints {
    domain: Box<[run::charter::Endpoint]>,
    channel: channel::Endpoints,
    llm: Box<[llm::ConfiguredEndpoint]>,
}

fn build_endpoints(
    endpoints: Vec<Endpoint>,
    limits: &llm::ComponentLimits,
    profile: &service::profile::Profile,
) -> Result<PreparedEndpoints, String> {
    let mut domain_endpoints = Vec::with_capacity(endpoints.len());
    let mut channel_endpoints = List::with_capacity(ENDPOINTS);
    let mut llm_endpoints = Vec::with_capacity(endpoints.len());
    let mut names = std::collections::BTreeSet::new();
    let mut numbers = std::collections::BTreeSet::new();
    for endpoint in endpoints {
        let declared = build_models(&endpoint)?;
        let mut client = service::profile::endpoint_client(profile, &declared).map_err(profile_refusal)?;
        if endpoint.name.is_empty()
            || endpoint.name.len()
                > usize::try_from(smith_charter::CEILINGS.llm_endpoint).expect("endpoint name bound fits usize")
        {
            return Err("endpoint name is empty or too long".into());
        }
        if endpoint.account >= ACCOUNTS {
            return Err("endpoint account is outside the profile".into());
        }
        if !names.insert(endpoint.name.clone()) || !numbers.insert(endpoint.number) {
            return Err("duplicate endpoint name or number".into());
        }
        let address = resolve(&endpoint.address)?;
        let transport = endpoint_transport(&endpoint, address)?;
        let mut llm_endpoint = match endpoint.provider.as_str() {
            "codex" => shared::Endpoint::codex(),
            "anthropic" => shared::Endpoint::anthropic(),
            _ => return Err("endpoint provider must be codex or anthropic".into()),
        };
        if let Some(authority) = endpoint.authority {
            if authority.is_empty() || authority.contains(['\r', '\n']) {
                return Err("invalid endpoint HTTP authority".into());
            }
            llm_endpoint.authority = authority.into_bytes().into();
        }
        if let Some(target) = endpoint.target {
            if !target.starts_with('/') || target.contains(['\r', '\n']) {
                return Err("invalid endpoint HTTP target".into());
            }
            llm_endpoint.target = target.into_bytes().into();
        }
        let mut headers = Vec::with_capacity(endpoint.headers.len());
        for header in endpoint.headers {
            if header.name.eq_ignore_ascii_case("session-id")
                || header.name.eq_ignore_ascii_case("thread-id")
                || header.name.is_empty()
                || header.name.contains(['\r', '\n', ':'])
                || header.value.contains(['\r', '\n'])
            {
                return Err("invalid endpoint HTTP header".into());
            }
            headers.push(Header { name: header.name.into_bytes().into(), value: header.value.into_bytes().into() });
        }
        llm_endpoint.headers = headers.into_boxed_slice();
        let identity = endpoint_identity(endpoint.identity.as_deref())?;
        domain_endpoints.push(run::charter::Endpoint(endpoint.number));
        channel_endpoints
            .push(channel::Endpoint {
                name: endpoint.name.into_bytes().into(),
                number: endpoint.number,
                dialect: endpoint.dialect,
                account: endpoint.account,
            })
            .map_err(|_| "too many channel endpoints")?;
        let credential = credential_limits(limits);
        let head = shared::client::request_head(&llm_endpoint, &credential, &client)
            .map_err(|error| profile_refusal(Refusal::Connection(connection::EndpointError::Client(error))))?;
        service::profile::validate::request_head(endpoint.number, head, tls::client::MAX_PLAINTEXT)
            .map_err(profile_refusal)?;
        client.http.request = head;
        let mut destination =
            connection::Endpoint { address, transport, llm: llm_endpoint, limits: client, credential };
        destination.pieces(&limits.connection);
        llm_endpoints.push(llm::ConfiguredEndpoint {
            name: domain::llm::Endpoint(endpoint.number),
            destination,
            account: endpoint.account,
            connect: endpoint_duration(endpoint.connect, endpoint.number, "connect")?,
            handshake: endpoint_duration(endpoint.handshake, endpoint.number, "handshake")?,
            reasoning_effort: endpoint.reasoning_effort.map(|value| value.into_bytes().into()),
            identity,
            models: endpoint_models(&endpoint.models)?,
        });
    }
    Ok(PreparedEndpoints {
        domain: domain_endpoints.into_boxed_slice(),
        channel: channel::Endpoints::new(channel_endpoints),
        llm: llm_endpoints.into_boxed_slice(),
    })
}

fn endpoint_transport(endpoint: &Endpoint, address: SocketAddr) -> Result<connection::Transport, String> {
    match endpoint.transport.as_str() {
        "tls" => Ok(connection::Transport::Tls {
            server_name: tls::Name::new(endpoint.server_name.as_deref().ok_or("TLS endpoint requires server_name")?)
                .ok_or("invalid TLS server name")?,
            trust: trust(endpoint.trust_der.as_deref())?,
        }),
        "plaintext" => {
            if !address.ip().is_loopback() {
                return Err("plaintext endpoint address must be loopback".into());
            }
            if endpoint.server_name.is_some() || endpoint.trust_der.is_some() {
                return Err("plaintext endpoint cannot name TLS server_name or trust_der".into());
            }
            Ok(connection::Transport::Plaintext)
        }
        _ => Err("endpoint transport must be tls or plaintext".into()),
    }
}

fn endpoint_models(models: &[ModelDocument]) -> Result<Box<[llm::ConfiguredModel]>, String> {
    let mut configured = Vec::with_capacity(models.len());
    for model in models {
        configured.push(llm::ConfiguredModel {
            name: model.name.as_bytes().into(),
            oversized_reasoning: reasoning_policy(model.oversized_reasoning.as_deref())?,
        });
    }
    Ok(configured.into_boxed_slice())
}

fn build_profile(document: &ProfileDocument) -> Result<service::profile::Profile, String> {
    if document.name != "standard" {
        return Err("unknown profile name".into());
    }
    let mut profile = service::profile::standard();
    apply_declared(&mut profile, &document.declared)?;
    apply_counts(&mut profile, &document.policy)?;
    apply_durations(&mut profile, &document.policy)?;
    Ok(profile)
}

fn apply_declared(profile: &mut service::profile::Profile, overrides: &DeclaredOverrides) -> Result<(), String> {
    if let Some(value) = overrides.conversations {
        if value == 0 {
            return Err("profile.declared.conversations must be positive".into());
        }
        profile.declared.conversations = value;
    }
    if let Some(value) = overrides.tool_payload {
        if value == 0 {
            return Err("profile.declared.tool_payload must be positive".into());
        }
        profile.declared.tool_payload = value;
    }
    if let Some(value) = overrides.file_bytes {
        if value == 0 {
            return Err("profile.declared.file_bytes must be positive".into());
        }
        profile.declared.file_bytes = value;
    }
    if let Some(value) = overrides.calls_per_response {
        if value == 0 {
            return Err("profile.declared.calls_per_response must be positive".into());
        }
        profile.declared.calls_per_response = value;
    }
    if let Some(value) = overrides.read_window {
        if value == 0 {
            return Err("profile.declared.read_window must be positive".into());
        }
        profile.declared.read_window = value;
    }
    if let Some(value) = overrides.shell_output {
        if value == 0 {
            return Err("profile.declared.shell_output must be positive".into());
        }
        profile.declared.shell_output = value;
    }
    if let Some(value) = overrides.shell_head {
        if value == 0 {
            return Err("profile.declared.shell_head must be positive".into());
        }
        profile.declared.shell_head = value;
    }
    if let Some(value) = overrides.shell_tail {
        if value == 0 {
            return Err("profile.declared.shell_tail must be positive".into());
        }
        profile.declared.shell_tail = value;
    }
    if let Some(value) = overrides.search_hits {
        if value == 0 {
            return Err("profile.declared.search_hits must be positive".into());
        }
        profile.declared.search_hits = value;
    }
    if let Some(value) = overrides.search_bytes {
        if value == 0 {
            return Err("profile.declared.search_bytes must be positive".into());
        }
        profile.declared.search_bytes = value;
    }
    if let Some(value) = overrides.list_entries {
        if value == 0 {
            return Err("profile.declared.list_entries must be positive".into());
        }
        profile.declared.list_entries = value;
    }
    if let Some(value) = overrides.guide {
        if value == 0 {
            return Err("profile.declared.guide must be positive".into());
        }
        profile.declared.guide = value;
    }
    if let Some(value) = overrides.llm_pool {
        if value == 0 {
            return Err("profile.declared.llm_pool must be positive".into());
        }
        profile.declared.llm_pool = value;
    }
    if let Some(value) = overrides.memory {
        if value == 0 {
            return Err("profile.declared.memory must be positive".into());
        }
        profile.declared.memory = Some(value);
    }
    Ok(())
}

fn apply_counts(profile: &mut service::profile::Profile, overrides: &PolicyOverrides) -> Result<(), String> {
    if let Some(value) = overrides.max_turns {
        if value == 0 {
            return Err("profile.policy.max_turns must be positive".into());
        }
        profile.policy.max_turns = value;
    }
    if let Some(value) = overrides.max_spend {
        if value == 0 {
            return Err("profile.policy.max_spend must be positive".into());
        }
        profile.policy.max_spend = value;
    }
    if let Some(value) = overrides.max_time {
        profile.policy.max_time = duration(value, LimitName::deployment(Layer::Policy, "max_time"))?;
    }
    if let Some(value) = overrides.max_waiting {
        profile.policy.max_waiting = duration(value, LimitName::deployment(Layer::Policy, "max_waiting"))?;
    }
    if let Some(value) = overrides.sections {
        if value == 0 {
            return Err("profile.policy.sections must be positive".into());
        }
        profile.policy.sections = value;
    }
    if let Some(value) = overrides.host_tools {
        if value == 0 {
            return Err("profile.policy.host_tools must be positive".into());
        }
        profile.policy.host_tools = value;
    }
    if let Some(value) = overrides.verdicts {
        if value == 0 {
            return Err("profile.policy.verdicts must be positive".into());
        }
        profile.policy.verdicts = value;
    }
    if let Some(value) = overrides.items {
        if value == 0 {
            return Err("profile.policy.items must be positive".into());
        }
        profile.policy.items = value;
    }
    if let Some(value) = overrides.fields {
        if value == 0 {
            return Err("profile.policy.fields must be positive".into());
        }
        profile.policy.fields = value;
    }
    if let Some(value) = overrides.inbox {
        if value == 0 {
            return Err("profile.policy.inbox must be positive".into());
        }
        profile.policy.inbox = value;
    }
    if let Some(value) = overrides.unacknowledged {
        if value == 0 {
            return Err("profile.policy.unacknowledged must be positive".into());
        }
        profile.policy.unacknowledged = value;
    }
    if let Some(value) = &overrides.compaction_threshold {
        if value.numerator == 0 || value.numerator >= value.denominator {
            return Err("profile.policy.compaction_threshold must be between zero and one".into());
        }
        profile.policy.compaction_threshold =
            service::profile::Fraction { numerator: value.numerator, denominator: value.denominator };
    }
    Ok(())
}

fn apply_durations(profile: &mut service::profile::Profile, overrides: &PolicyOverrides) -> Result<(), String> {
    if let Some(value) = overrides.connection_keep {
        profile.policy.connection_keep = duration(value, LimitName::deployment(Layer::Policy, "connection_keep"))?;
    }
    if let Some(value) = overrides.tool_deadline {
        profile.policy.tool_deadline = duration(value, LimitName::deployment(Layer::Policy, "tool_deadline"))?;
    }
    if let Some(value) = overrides.shell_timeout {
        profile.policy.shell_timeout = duration(value, LimitName::deployment(Layer::Policy, "shell_timeout"))?;
    }
    if let Some(value) = overrides.group_stop {
        profile.policy.group_stop = duration(value, LimitName::deployment(Layer::Policy, "group_stop"))?;
    }
    if let Some(value) = overrides.close {
        profile.policy.close = duration(value, LimitName::deployment(Layer::Policy, "close"))?;
    }
    if let Some(value) = overrides.write_deadline {
        profile.policy.write_deadline = duration(value, LimitName::deployment(Layer::Policy, "write_deadline"))?;
    }
    if let Some(value) = overrides.cancel_grace {
        profile.policy.cancel_grace = duration(value, LimitName::deployment(Layer::Policy, "cancel_grace"))?;
    }
    if let Some(value) = overrides.exit_grace {
        profile.policy.exit_grace = duration(value, LimitName::deployment(Layer::Policy, "exit_grace"))?;
    }
    Ok(())
}

fn duration(millis: u64, name: LimitName) -> Result<Duration, String> {
    let nanos = millis.checked_mul(1_000_000).ok_or_else(|| profile_refusal(Refusal::Derived { limit: name }))?;
    Ok(Duration::from_nanos(nanos))
}

fn endpoint_duration(millis: u64, endpoint: u32, field: &'static str) -> Result<Duration, String> {
    duration(millis, LimitName { layer: Layer::Policy, field, per: Per::Endpoint(endpoint) })
}

fn endpoint_identity(value: Option<&str>) -> Result<llm::IdentityProfile, String> {
    match value {
        None | Some("plain") => Ok(llm::IdentityProfile::Plain),
        Some("claude-code") => Ok(llm::IdentityProfile::ClaudeCode),
        Some(_) => Err("endpoint identity must be plain or claude-code".into()),
    }
}

fn reasoning_policy(value: Option<&str>) -> Result<llm::OversizedReasoning, String> {
    match value {
        None | Some("fail") => Ok(llm::OversizedReasoning::Fail),
        Some("drop") => Ok(llm::OversizedReasoning::Drop),
        Some(_) => Err("model oversized_reasoning must be fail or drop".into()),
    }
}

fn build_models(endpoint: &Endpoint) -> Result<service::profile::Endpoint, String> {
    let number = endpoint.number;
    let models = &endpoint.models;
    let provider = match endpoint.provider.as_str() {
        "codex" => shared::Provider::OpenAiCodex,
        "anthropic" => shared::Provider::Anthropic,
        _ => return Err("endpoint provider must be codex or anthropic".into()),
    };
    let identity = endpoint_identity(endpoint.identity.as_deref())?;
    let option_bytes =
        llm::endpoint_own(endpoint.reasoning_effort.as_deref().map(str::as_bytes), identity).map_err(|_| {
            profile_refusal(Refusal::Derived {
                limit: LimitName { layer: Layer::Service, field: "endpoint_own", per: Per::Endpoint(number) },
            })
        })?;
    if models.is_empty() {
        return Err(profile_refusal(Refusal::Field {
            limit: LimitName { layer: Layer::Declared, field: "models", per: Per::Endpoint(number) },
        }));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut prepared = Vec::with_capacity(models.len());
    for (index, model) in models.iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| {
            profile_refusal(Refusal::Derived { limit: LimitName::deployment(Layer::Service, "configured_models") })
        })?;
        let per = Per::Model { endpoint: number, model: index };
        if model.name.is_empty()
            || model.name.len()
                > usize::try_from(smith_charter::CEILINGS.llm_model).expect("model name bound fits usize")
            || model.window == 0
            || model.output == 0
            || model.reasoning_item == 0
        {
            return Err("model declaration contains an empty or zero quantity".into());
        }
        if !names.insert(model.name.clone()) {
            return Err("duplicate declared model".into());
        }
        prepared.push(service::profile::Model {
            name: model.name.as_bytes().into(),
            window: model.window,
            output: model.output,
            reasoning_item: model.reasoning_item,
            oversized_reasoning: reasoning_policy(model.oversized_reasoning.as_deref())?,
            head: duration(model.head, LimitName { layer: Layer::Declared, field: "head", per })?,
            idle: duration(model.idle, LimitName { layer: Layer::Declared, field: "idle", per })?,
        });
    }
    Ok(service::profile::Endpoint {
        number,
        provider,
        option_bytes,
        connect: endpoint_duration(endpoint.connect, number, "connect")?,
        handshake: endpoint_duration(endpoint.handshake, number, "handshake")?,
        models: prepared.into_boxed_slice(),
    })
}

fn credential_limits(limits: &llm::ComponentLimits) -> shared::client::CredentialLimits {
    shared::client::CredentialLimits { access_token: limits.grant_value_bytes, account_id: limits.grant_value_bytes }
}

fn tls_transport() -> String {
    "tls".into()
}

fn resolve(address: &str) -> Result<SocketAddr, String> {
    let mut resolved = address.to_socket_addrs().map_err(|error| format!("endpoint address {address:?}: {error}"))?;
    resolved.next().ok_or_else(|| format!("endpoint address {address:?} has no result"))
}

// Startup trust material and decoded configuration are bounded together.
/// Bound on startup trust material and decoded configuration, in bytes.
pub const TRUST_BYTES: u64 = 16_777_216;

/// Load the bounded startup trust roots used by agent and local endpoints.
pub fn trust(der: Option<&str>) -> Result<tls::Config, String> {
    let mut roots = tls::RootCertStore::empty();
    if let Some(path) = der {
        #[expect(clippy::disallowed_types, reason = "10-product uses skein startup reads and trust roots")]
        let file: fs::File = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .map_err(|error| format!("trust certificate {path:?}: {error}"))?;
        let metadata = file.metadata().map_err(|error| format!("trust certificate metadata: {error}"))?;
        if !metadata.is_file() || metadata.len() > 65_536 {
            return Err("trust certificate must be a regular file of at most 65536 bytes".into());
        }
        let mut bytes = Vec::new();
        file.take(65_537).read_to_end(&mut bytes).map_err(|error| format!("trust certificate read: {error}"))?;
        if bytes.len() > 65_536 {
            return Err("trust certificate exceeds 65536 bytes".into());
        }
        roots.add(tls::CertificateDer::from(bytes)).map_err(|error| format!("trust certificate: {error}"))?;
    } else {
        let native = rustls_native_certs::load_native_certs();
        if native.certs.len() > 512 || native.certs.iter().any(|cert| cert.len() > 8192) {
            return Err("native trust store exceeds its certificate bounds".into());
        }
        for cert in native.certs {
            roots.add(cert).map_err(|error| format!("native trust certificate: {error}"))?;
        }
    }
    tls::Config::new(roots, &[]).map_err(|error| format!("TLS trust: {error:?}"))
}

pub(crate) fn profile_refusal(error: Refusal) -> String {
    String::from_utf8(service::profile::words(&error).into_vec()).expect("refusal words are UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> Document {
        Document {
            profile: ProfileDocument {
                name: "standard".into(),
                declared: DeclaredOverrides {
                    memory: Some(u64::MAX),
                    tool_payload: Some(256),
                    read_window: Some(256),
                    ..DeclaredOverrides::default()
                },
                policy: PolicyOverrides::default(),
            },
            endpoints: serde_json::from_value(shaped_document()["endpoints"].clone()).expect("tiny endpoint"),
            environment: vec![],
            trace: None,
        }
    }

    #[test]
    fn empty_endpoint_and_model_lists_name_the_missing_field() {
        let mut empty = document();
        empty.endpoints.clear();
        assert_eq!(build(empty).err().as_deref(), Some("field: declared.endpoints must not be empty"));
        let mut empty = document();
        empty.endpoints[0].models.clear();
        assert_eq!(build(empty).err().as_deref(), Some("field: declared.models[endpoint=0] must not be empty"));
    }

    #[test]
    fn shared_environment_validation_names_the_invalid_or_repeated_position() {
        for (name, value) in [("", "v"), ("A=B", "v"), ("A\0B", "v"), ("A", "v\0")] {
            let mut document = document();
            document.environment = vec![Variable { name: name.into(), value: value.into() }];
            assert_eq!(build(document).err().as_deref(), Some("environment: entry 0 has an invalid or repeated name"));
        }
        let mut document = document();
        document.environment = vec![
            Variable { name: "A".into(), value: "first".into() },
            Variable { name: "A".into(), value: "second".into() },
        ];
        assert_eq!(build(document).err().as_deref(), Some("environment: entry 1 has an invalid or repeated name"));
    }

    #[test]
    fn empty_and_equals_values_reach_the_shared_environment_encoder() {
        let mut document = document();
        document.environment = vec![
            Variable { name: "EMPTY".into(), value: String::new() },
            Variable { name: "MULTI".into(), value: "a=b".into() },
        ];
        let configuration = build(document).expect("valid environment");
        assert_eq!(configuration.declarations.environment_bytes, 15);
        assert_eq!(configuration.service.limits.machine.env_bytes, 15);
    }

    #[test]
    fn named_endpoint_resolves_and_its_tls_trust_builds_before_start() {
        let mut document = document();
        document.endpoints.clear();
        document.endpoints.push(Endpoint {
            connect: 10_000,
            handshake: 10_000,
            models: vec![ModelDocument {
                name: "test".into(),
                window: 65_536,
                output: 512,
                reasoning_item: 64,
                oversized_reasoning: None,
                head: 60_000,
                idle: 30_000,
            }],
            name: "test".into(),
            number: 42,
            dialect: 1,
            account: 0,
            provider: "codex".into(),
            address: "127.0.0.1:443".into(),
            transport: "tls".into(),
            server_name: Some("example.test".into()),
            authority: None,
            target: None,
            trust_der: Some(format!("{}/../../tests/protocol-llm/fixtures/root.der", env!("CARGO_MANIFEST_DIR"))),
            headers: vec![],
            reasoning_effort: None,
            identity: None,
        });
        let configuration = build(document).expect("resolved endpoint");
        service::Service::new(configuration.service, 1).expect("LLM and channel endpoints");
    }

    #[test]
    fn affinity_header_names_and_static_cache_keys_are_refused_in_configuration() {
        for name in ["session-id", "SESSION-ID", "thread-id", "ThReAd-Id"] {
            let json = format!(
                r#"{{"profile":{{"name":"standard","declared":{{"memory":1099511627776,"tool_payload":256,"read_window":256}},"policy":{{"group_stop":10}}}},"endpoints":[{{"name":"local","number":1,"dialect":1,"account":0,"provider":"codex","address":"127.0.0.1:8080","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"head":60000,"idle":30000}}],"transport":"plaintext","headers":[{{"name":"{name}","value":"host override"}}]}}],"environment":[]}}"#
            );
            assert_eq!(parse(json.as_bytes()).err().as_deref(), Some("invalid endpoint HTTP header"));
        }
        let error = serde_json::from_str::<Endpoint>(r#"{"name":"old","number":1,"dialect":1,"account":0,"provider":"codex","address":"127.0.0.1:8080","transport":"plaintext","cache_key":"static"}"#).err().expect("removed field is rejected");
        assert!(error.to_string().contains("unknown field `cache_key`"));
    }

    #[test]
    fn plaintext_configuration_names_its_transport_and_admits_only_loopback() {
        for address in ["127.0.0.1:8080", "[::1]:8080"] {
            let json = format!(
                r#"{{"profile":{{"name":"standard","declared":{{"memory":1099511627776,"tool_payload":256,"read_window":256}},"policy":{{"group_stop":10}}}},
                "endpoints":[{{"name":"local","number":1,"dialect":1,"account":0,"provider":"codex",
                "address":"{address}","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"head":60000,"idle":30000}}],"transport":"plaintext"}}],"environment":[]}}"#
            );
            let configuration = parse(json.as_bytes()).expect("loopback plaintext configuration");
            service::Service::new(configuration.service, 1).expect("loopback plaintext service");
        }
        for address in ["192.0.2.1:8080", "[2001:db8::1]:8080"] {
            let json = format!(
                r#"{{"profile":{{"name":"standard","declared":{{"memory":1099511627776,"tool_payload":256,"read_window":256}},"policy":{{"group_stop":10}}}},
                "endpoints":[{{"name":"local","number":1,"dialect":1,"account":0,"provider":"codex",
                "address":"{address}","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"head":60000,"idle":30000}}],"transport":"plaintext"}}],"environment":[]}}"#
            );
            assert_eq!(parse(json.as_bytes()).err().as_deref(), Some("plaintext endpoint address must be loopback"));
        }
    }

    #[test]
    fn ambiguous_or_unknown_endpoint_transports_refuse_at_configuration() {
        for extra in [
            r#""transport":"plaintext","server_name":"example.test""#,
            r#""transport":"plaintext","trust_der":"missing.der""#,
            r#""transport":"unknown""#,
            r#""transport":"tls""#,
        ] {
            let json = format!(
                r#"{{"profile":{{"name":"standard","declared":{{"memory":1099511627776,"tool_payload":256,"read_window":256}},"policy":{{"group_stop":10}}}},
                "endpoints":[{{"name":"local","number":1,"dialect":1,"account":0,"provider":"codex",
                "address":"127.0.0.1:8080","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"head":60000,"idle":30000}}],{extra}}}],"environment":[]}}"#
            );
            assert!(parse(json.as_bytes()).is_err(), "invalid transport {extra}");
        }
    }

    #[test]
    fn overflowing_declared_durations_name_the_first_field_and_its_scope() {
        let mut policy = document();
        policy.profile.policy.max_time = Some(u64::MAX);
        assert_eq!(build(policy).err().as_deref(), Some("derived: policy.max_time does not fit its type"));
        let endpoint = LimitName { layer: Layer::Declared, field: "idle", per: Per::Model { endpoint: 7, model: 3 } };
        assert_eq!(
            duration(u64::MAX, endpoint).err().as_deref(),
            Some("derived: declared.idle[endpoint=7,model=3] does not fit its type")
        );
    }

    #[test]
    fn bad_profile_and_zero_grace_refuse_before_a_channel_is_opened() {
        let mut bad = document();
        bad.profile.name = "unknown".into();
        assert!(build(bad).is_err());
        let mut bad = document();
        bad.profile.policy.group_stop = Some(0);
        assert!(build(bad).is_err());
    }

    #[test]
    fn malformed_json_refuses_missing_required_configuration_fields() {
        assert!(serde_json::from_slice::<Document>(br#"{"profile":"standard"}"#).is_err());
        assert!(serde_json::from_slice::<Document>(br#"{"profile":"standard","memory_bytes":1,"grace_ms":1,"endpoints":[],"environment":[],"credential":"bad"}"#).is_err());
    }

    fn shaped_document() -> serde_json::Value {
        serde_json::json!({
            "profile":{"name":"standard","declared":{"memory":1_099_511_627_776_u64,"tool_payload":256,"read_window":256}},
            "endpoints":[{"name":"local","number":0,"dialect":0,"account":0,"provider":"codex",
                "address":"127.0.0.1:8080","transport":"plaintext","connect":10000,"handshake":10000,
                "models":[{"name":"test","window":65536,"output":512,"reasoning_item":64,"head":60000,"idle":30000}]}],
            "environment":[]
        })
    }

    fn parse_value(value: &serde_json::Value) -> Result<Configuration, String> {
        parse(&serde_json::to_vec(value).expect("configuration JSON"))
    }

    #[test]
    fn quantities_override_one_value_and_derive_through_the_service() {
        let mut document = shaped_document();
        document["profile"]["declared"]["conversations"] = 2.into();
        document["profile"]["declared"]["shell_tail"] = 64.into();
        document["profile"]["policy"] = serde_json::json!({"inbox":3,"group_stop":7,"max_turns":9,"unacknowledged":2});
        let configured = parse_value(&document).expect("quantity overrides");
        let limits = configured.service.limits;
        assert_eq!(limits.domain.run.conversations, 2);
        assert_eq!(limits.domain.session.sessions, 2);
        assert_eq!(limits.domain.session.tools.kits, 2);
        assert_eq!(limits.domain.run.messages, 3);
        assert_eq!(limits.domain.run.budget.turns, 9);
        assert_eq!(limits.channel.turns, 2);
        assert_eq!(limits.machine.group_stop, Duration::from_millis(7));
        assert_eq!(limits.domain.session.tools.shell_tail, 64);
        assert_eq!(limits.domain.run.check_tail, 64);
        assert_eq!(limits.domain.session.tools.tool_deadline, configured.profile.policy.tool_deadline);
        assert_eq!(limits.domain.session.tools.shell_output, configured.profile.declared.shell_output);
        assert_eq!(limits.domain.run.host_timeout_max, configured.profile.policy.tool_deadline);
        assert_eq!(limits.llm.adapter.stated.tool_deadline, configured.profile.policy.tool_deadline);
        assert_eq!(configured.profile.declared.shell_head, service::profile::standard().declared.shell_head);
        assert_eq!(configured.declarations.endpoints[0].models[0].window, 65536);
    }

    #[test]
    fn configuration_refuses_old_shape_raw_limits_duplicates_and_zero_quantities() {
        assert!(
            parse(
                br#"{"profile":"standard","memory_bytes":1099511627776,"grace_ms":10,"endpoints":[],"environment":[]}"#
            )
            .is_err()
        );
        for field in ["request_bytes", "unexpected"] {
            let mut document = shaped_document();
            document["profile"]["declared"][field] = 100.into();
            assert!(parse_value(&document).is_err(), "raw or unknown field {field}");
        }
        let mut document = shaped_document();
        let duplicate = document["endpoints"][0]["models"][0].clone();
        document["endpoints"][0]["models"].as_array_mut().expect("models").push(duplicate);
        assert_eq!(parse_value(&document).err().as_deref(), Some("duplicate declared model"));
        for field in ["window", "output", "reasoning_item", "head", "idle"] {
            let mut document = shaped_document();
            document["endpoints"][0]["models"][0][field] = 0.into();
            assert!(parse_value(&document).is_err(), "zero model quantity {field}");
        }
        for field in ["conversations", "tool_payload", "file_bytes", "calls_per_response", "memory"] {
            let mut document = shaped_document();
            document["profile"]["declared"][field] = 0.into();
            assert!(parse_value(&document).is_err(), "zero deployment quantity {field}");
        }
    }
    #[test]
    fn the_whole_file_declaration_is_independent_and_payload_checked() {
        let mut document = shaped_document();
        document["profile"]["declared"]["file_bytes"] = 1_048_576.into();
        let configured = parse_value(&document).expect("explicit whole file ceiling");
        assert_eq!(configured.profile.declared.file_bytes, 1_048_576);
        document["profile"]["declared"]["file_bytes"] = 1.into();
        let reason = parse_value(&document).err().expect("payload does not fit");
        assert!(reason.starts_with("payload-in-file:"), "{reason}");
    }
    #[test]
    fn oversized_reasoning_defaults_to_fail_and_drop_requires_explicit_selection() {
        let mut document = shaped_document();
        let configured = parse_value(&document).expect("default policy");
        assert_eq!(
            configured.declarations.endpoints[0].models[0].oversized_reasoning,
            service::profile::OversizedReasoning::Fail
        );
        document["endpoints"][0]["models"][0]["oversized_reasoning"] = "drop".into();
        let configured = parse_value(&document).expect("explicit drop");
        assert_eq!(
            configured.declarations.endpoints[0].models[0].oversized_reasoning,
            service::profile::OversizedReasoning::Drop
        );
        document["endpoints"][0]["models"][0]["oversized_reasoning"] = "fallback".into();
        assert_eq!(parse_value(&document).err().as_deref(), Some("model oversized_reasoning must be fail or drop"));
    }
}
