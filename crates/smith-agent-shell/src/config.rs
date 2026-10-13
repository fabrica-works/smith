//! Generated configuration values or JSON read once before the agent opens its channel.
//! It retains endpoint names and destinations, limits, command environment,
//! and future trace settings; no credential is read here.
//! Durations in JSON are milliseconds. Contract: protocol/agent.md, sections 4 and 6.

use std::net::{SocketAddr, ToSocketAddrs};
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

mod refusal;
mod trace;
pub use refusal::ConfigurationRefusal;

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
    check_timeout: Option<u64>,
    wrap_up: Option<u64>,
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
    prices: PriceDocument,
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
struct PriceDocument {
    input: u64,
    cached: u64,
    #[serde(default)]
    cache_write: Option<u64>,
    output: u64,
    unit: u32,
}

impl PriceDocument {
    fn rates(&self) -> run::Prices {
        run::Prices {
            input: self.input,
            cached: self.cached,
            cache_write: self.cache_write,
            output: self.output,
            unit: self.unit,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceDocument {
    path: String,
    #[serde(default = "default_capture")]
    capture: String,
    #[serde(default = "default_delivery")]
    delivery: String,
}

fn default_capture() -> String {
    "none".into()
}

fn default_delivery() -> String {
    "complete".into()
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
    trust: Option<TrustDocument>,
    #[serde(default)]
    headers: Vec<HeaderField>,
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
struct TrustDocument {
    #[serde(default = "system_trust")]
    system: bool,
    #[serde(default)]
    der: Vec<String>,
    #[serde(default)]
    pem: Vec<String>,
}

const fn system_trust() -> bool {
    true
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
    let bytes = skein_shell::read_file(path, u32::try_from(CONFIG_BYTES).expect("configuration bound fits u32"))
        .map_err(|why| match why {
            skein_shell::Unread::Absent => format!("configuration metadata: {}: file absent", path.display()),
            skein_shell::Unread::NotAFile => format!("configuration {}: not a regular file", path.display()),
            skein_shell::Unread::TooLarge { max } => {
                format!("configuration {} is larger than {max} bytes", path.display())
            }
            skein_shell::Unread::Unsafe(reason) => format!("configuration read {}: {reason:?}", path.display()),
            skein_shell::Unread::Kernel(error) => format!("configuration read {}: {error:?}", path.display()),
        })?;
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
fn environment_size_refusal() -> ConfigurationRefusal {
    ConfigurationRefusal::Profile(Refusal::Derived { limit: LimitName::deployment(Layer::Machine, "env_bytes") })
}

fn environment_refusal(error: machine::Unencodable) -> ConfigurationRefusal {
    match error {
        machine::Unencodable::Variable { index } | machine::Unencodable::Repeated { index } => {
            ConfigurationRefusal::Profile(Refusal::Environment { index })
        }
        // This configuration's exact count derives the receiving bound; a
        // smaller bound would mean the derived field cannot hold that count.
        machine::Unencodable::TooLarge { .. } => environment_size_refusal(),
    }
}

fn event_limits(
    limits: &service::Limits,
    _endpoints: &llm::Endpoints,
    _declarations: &service::profile::Configuration,
    capture: Capture,
    header: &[smith_events::EffectiveLimit],
    write_deadline: Duration,
) -> Result<service::EventLimits, ConfigurationRefusal> {
    let events = service::EventLimits::derive(
        limits,
        capture,
        1,
        smith_charter::CEILINGS.llm_effort_some,
        header,
        write_deadline,
    )
    .ok_or("event ownership overflow")?;
    Ok(events)
}

fn effective_header(
    profile: &service::profile::Profile,
    declarations: &service::profile::Configuration,
    limits: &service::Limits,
    names: &channel::Endpoints,
    endpoints: &llm::Endpoints,
) -> Result<List<smith_events::EffectiveLimit>, ConfigurationRefusal> {
    let inventory =
        service::profile::effective(profile, declarations, limits, endpoints).ok_or("effective limits overflow")?;
    let header = service::profile::header(&inventory, names, declarations).ok_or("effective names overflow")?;
    Ok(header)
}

fn build(document: Document) -> Result<Configuration, String> {
    build_parts(document).map_err(ConfigurationRefusal::detail)
}

/// Build the agent from generated settings values, without a JSON file or byte round trip.
pub fn from_value(value: serde_json::Value) -> Result<Configuration, ConfigurationRefusal> {
    let document = serde_json::from_value(value)
        .map_err(|why| ConfigurationRefusal::Input(format!("configuration JSON: {why}")))?;
    build_parts(document)
}

/// Resolve profile overrides before destinations or credentials are consulted.
pub fn profile_from_value(value: serde_json::Value) -> Result<service::profile::Profile, ConfigurationRefusal> {
    let document =
        serde_json::from_value(value).map_err(|why| ConfigurationRefusal::Input(format!("profile JSON: {why}")))?;
    build_profile(&document)
}

fn build_parts(document: Document) -> Result<Configuration, ConfigurationRefusal> {
    let profile = build_profile(&document.profile)?;
    let memory = profile.declared.memory;
    if document.endpoints.is_empty() {
        return Err(ConfigurationRefusal::Profile(Refusal::Field {
            limit: LimitName::deployment(Layer::Declared, "endpoints"),
        }));
    }
    if document.endpoints.len() > usize::try_from(ENDPOINTS).expect("endpoint count fits usize") {
        return Err("too many endpoints".into());
    }
    let trace = trace::build(document.trace)?;
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
    let mut limits = service::profile::derive(&profile, &declarations).map_err(ConfigurationRefusal::Profile)?;
    let environment = machine::Environment::encode(&environment, &limits.machine).map_err(environment_refusal)?;
    let prices: Vec<_> = document
        .endpoints
        .iter()
        .flat_map(|endpoint| endpoint.models.iter().map(|model| model.prices.rates()))
        .collect();
    let endpoints = build_endpoints(document.endpoints, &limits.llm, &profile)?;
    let llm_endpoints =
        llm::Endpoints::new(endpoints.llm, ENDPOINTS, ACCOUNTS).map_err(|error| format!("LLM endpoints: {error:?}"))?;
    if let Some(trace) = &trace {
        // First admit the sink, then include its own limits in the receiving header.
        for _ in 0..2 {
            let header = effective_header(&profile, &declarations, &limits, &endpoints.channel, &llm_endpoints)?;
            limits.events = Some(event_limits(
                &limits,
                &llm_endpoints,
                &declarations,
                trace.capture,
                header.as_slice(),
                profile.policy.write_deadline,
            )?);
        }
    }
    let mut models = Vec::new();
    for endpoint in &declarations.endpoints {
        for model in &endpoint.models {
            models.push(domain::ConfiguredModel {
                prices: *prices.get(models.len()).expect("one resolved price per declared model"),
                endpoint: run::charter::Endpoint(endpoint.number),
                model: model.name.clone(),
                window: model.window,
                output: model.output,
                protocol_allowance: service::profile::endpoint_allowance(
                    &profile,
                    endpoint,
                    &limits.domain.session.tools,
                )
                .map_err(ConfigurationRefusal::Profile)?,
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
    };
    check_memory(&config, memory)?;
    Ok(Configuration { service: config, memory, trace, profile, declarations })
}

fn check_memory(config: &service::Config, memory: u64) -> Result<(), ConfigurationRefusal> {
    let complete = service::worst_case(&config.limits, &config.llm_endpoints).ok_or_else(|| {
        ConfigurationRefusal::Profile(Refusal::Derived { limit: LimitName::deployment(Layer::Service, "worst_case") })
    })?;
    service::profile::validate::memory(complete, memory).map_err(ConfigurationRefusal::Profile)
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
) -> Result<PreparedEndpoints, ConfigurationRefusal> {
    let mut domain_endpoints = Vec::with_capacity(endpoints.len());
    let mut channel_endpoints = List::with_capacity(ENDPOINTS);
    let mut llm_endpoints = Vec::with_capacity(endpoints.len());
    let mut names = std::collections::BTreeSet::new();
    let mut numbers = std::collections::BTreeSet::new();
    for endpoint in endpoints {
        let declared = build_models(&endpoint)?;
        let mut client = service::profile::endpoint_client(profile, &declared, limits.adapter.client.history_items)
            .map_err(ConfigurationRefusal::Profile)?;
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
        let head = shared::client::request_head(&llm_endpoint, &credential, &client).map_err(|error| {
            ConfigurationRefusal::Profile(Refusal::Connection(connection::EndpointError::Client(error)))
        })?;
        service::profile::validate::request_head(endpoint.number, head, tls::client::MAX_PLAINTEXT)
            .map_err(ConfigurationRefusal::Profile)?;
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

fn endpoint_transport(endpoint: &Endpoint, address: SocketAddr) -> Result<connection::Transport, ConfigurationRefusal> {
    match endpoint.transport.as_str() {
        "tls" => Ok(connection::Transport::Tls {
            server_name: tls::Name::new(endpoint.server_name.as_deref().ok_or("TLS endpoint requires server_name")?)
                .ok_or("invalid TLS server name")?,
            trust: endpoint_trust(endpoint)?,
        }),
        "plaintext" => {
            if !address.ip().is_loopback() {
                return Err("plaintext endpoint address must be loopback".into());
            }
            if endpoint.server_name.is_some() || endpoint.trust_der.is_some() || endpoint.trust.is_some() {
                return Err("plaintext endpoint cannot name TLS server_name or trust_der".into());
            }
            Ok(connection::Transport::Plaintext)
        }
        _ => Err("endpoint transport must be tls or plaintext".into()),
    }
}

fn endpoint_models(models: &[ModelDocument]) -> Result<Box<[llm::ConfiguredModel]>, ConfigurationRefusal> {
    let mut configured = Vec::with_capacity(models.len());
    for model in models {
        configured.push(llm::ConfiguredModel {
            name: model.name.as_bytes().into(),
            oversized_reasoning: reasoning_policy(model.oversized_reasoning.as_deref())?,
        });
    }
    Ok(configured.into_boxed_slice())
}

fn build_profile(document: &ProfileDocument) -> Result<service::profile::Profile, ConfigurationRefusal> {
    if document.name != "standard" {
        return Err("unknown profile name".into());
    }
    let mut profile = service::profile::standard();
    apply_declared(&mut profile, &document.declared)?;
    apply_counts(&mut profile, &document.policy)?;
    apply_durations(&mut profile, &document.policy)?;
    Ok(profile)
}

fn apply_declared(
    profile: &mut service::profile::Profile,
    overrides: &DeclaredOverrides,
) -> Result<(), ConfigurationRefusal> {
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
        profile.declared.memory = value;
    }
    Ok(())
}

fn apply_counts(
    profile: &mut service::profile::Profile,
    overrides: &PolicyOverrides,
) -> Result<(), ConfigurationRefusal> {
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

fn apply_durations(
    profile: &mut service::profile::Profile,
    overrides: &PolicyOverrides,
) -> Result<(), ConfigurationRefusal> {
    if let Some(value) = overrides.check_timeout {
        profile.policy.check_timeout = duration(value, LimitName::deployment(Layer::Policy, "check_timeout"))?;
    }
    if let Some(value) = overrides.wrap_up {
        profile.policy.wrap_up = duration(value, LimitName::deployment(Layer::Policy, "wrap_up"))?;
    }
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

fn duration(millis: u64, name: LimitName) -> Result<Duration, ConfigurationRefusal> {
    let nanos = millis.checked_mul(1_000_000).ok_or(ConfigurationRefusal::Profile(Refusal::Derived { limit: name }))?;
    Ok(Duration::from_nanos(nanos))
}

fn endpoint_duration(millis: u64, endpoint: u32, field: &'static str) -> Result<Duration, ConfigurationRefusal> {
    duration(millis, LimitName { layer: Layer::Policy, field, per: Per::Endpoint(endpoint) })
}

fn endpoint_identity(value: Option<&str>) -> Result<llm::IdentityProfile, ConfigurationRefusal> {
    match value {
        None | Some("plain") => Ok(llm::IdentityProfile::Plain),
        Some("claude-code") => Ok(llm::IdentityProfile::ClaudeCode),
        Some(_) => Err("endpoint identity must be plain or claude-code".into()),
    }
}

fn reasoning_policy(value: Option<&str>) -> Result<llm::OversizedReasoning, ConfigurationRefusal> {
    match value {
        None | Some("fail") => Ok(llm::OversizedReasoning::Fail),
        Some("drop") => Ok(llm::OversizedReasoning::Drop),
        Some(_) => Err("model oversized_reasoning must be fail or drop".into()),
    }
}

fn build_models(endpoint: &Endpoint) -> Result<service::profile::Endpoint, ConfigurationRefusal> {
    let number = endpoint.number;
    let models = &endpoint.models;
    let provider = match endpoint.provider.as_str() {
        "codex" => shared::Provider::OpenAiCodex,
        "anthropic" => shared::Provider::Anthropic,
        _ => return Err("endpoint provider must be codex or anthropic".into()),
    };
    let identity = endpoint_identity(endpoint.identity.as_deref())?;
    let option_bytes = llm::endpoint_own(identity).map_err(|_| {
        ConfigurationRefusal::Profile(Refusal::Derived {
            limit: LimitName { layer: Layer::Service, field: "endpoint_own", per: Per::Endpoint(number) },
        })
    })?;
    if models.is_empty() {
        return Err(ConfigurationRefusal::Profile(Refusal::Field {
            limit: LimitName { layer: Layer::Declared, field: "models", per: Per::Endpoint(number) },
        }));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut prepared = Vec::with_capacity(models.len());
    for (index, model) in models.iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| {
            ConfigurationRefusal::Profile(Refusal::Derived {
                limit: LimitName::deployment(Layer::Service, "configured_models"),
            })
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
        model.prices.rates().check().map_err(|_| "model prices must be positive")?;
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

fn resolve(address: &str) -> Result<SocketAddr, ConfigurationRefusal> {
    let mut resolved = address.to_socket_addrs().map_err(|error| format!("endpoint address {address:?}: {error}"))?;
    resolved.next().ok_or_else(|| ConfigurationRefusal::Input(format!("endpoint address {address:?} has no result")))
}

// Startup trust material and decoded configuration are bounded together.
/// Bound on startup trust material and decoded configuration, in bytes.
pub const TRUST_BYTES: u64 = 16_777_216;

/// Load the bounded startup trust roots used by agent and local endpoints.
pub fn trust(der: Option<&str>) -> Result<tls::Config, String> {
    let source = match der {
        Some(path) => skein_shell::Trust::Der(path.into()),
        None => skein_shell::Trust::Machine { ssl_cert_file: std::env::var_os("SSL_CERT_FILE").map(Into::into) },
    };
    trust_source(&source)
}

/// Load account trust arrays through the existing bounded root owner.
pub fn configured_trust(sources: &[skein_shell::Trust]) -> Result<tls::Config, String> {
    trust_sources(sources).map_err(ConfigurationRefusal::detail)
}

fn trust_source(source: &skein_shell::Trust) -> Result<tls::Config, String> {
    trust_sources(std::slice::from_ref(source)).map_err(ConfigurationRefusal::detail)
}

fn endpoint_trust(endpoint: &Endpoint) -> Result<tls::Config, ConfigurationRefusal> {
    let Some(document) = &endpoint.trust else {
        return trust(endpoint.trust_der.as_deref()).map_err(ConfigurationRefusal::Input);
    };
    if endpoint.trust_der.is_some() {
        return Err("endpoint cannot name both trust_der and trust".into());
    }
    let mut sources = Vec::new();
    if document.system {
        sources.push(skein_shell::Trust::Machine { ssl_cert_file: std::env::var_os("SSL_CERT_FILE").map(Into::into) });
    }
    sources.extend(document.der.iter().map(|path| skein_shell::Trust::Der(path.into())));
    sources.extend(document.pem.iter().map(|path| skein_shell::Trust::Machine { ssl_cert_file: Some(path.into()) }));
    trust_sources(&sources)
}

fn trust_sources(sources: &[skein_shell::Trust]) -> Result<tls::Config, ConfigurationRefusal> {
    let mut certificates = Vec::new();
    let mut bytes = 0_u64;
    let mut malformed = 0_u32;
    let mut other_blocks = 0_u32;
    let mut paths = Vec::new();
    for source in sources {
        let bounds = skein_shell::RootBounds {
            bundle_bytes: u32::try_from(TRUST_BYTES).expect("trust bound fits u32"),
            certificates: 512,
            certificate_bytes: match source {
                skein_shell::Trust::Der(_) => 65_536,
                skein_shell::Trust::Machine { .. } => 8192,
            },
        };
        let roots = skein_shell::read_trust_roots(source, &bounds).map_err(|why| {
            ConfigurationRefusal::Input(match why {
                skein_shell::RootsRefusal::NoBundle => "machine trust roots: no certificate bundle found".into(),
                skein_shell::RootsRefusal::Unread { path, why } => {
                    format!("trust certificate {}: {why:?}", path.display())
                }
                skein_shell::RootsRefusal::TooMany { bound } => format!("trust roots exceed {bound} certificates"),
                skein_shell::RootsRefusal::TooLong { index, bound } => {
                    format!("trust certificate {index} exceeds {bound} bytes")
                }
            })
        })?;
        for certificate in roots.certificates {
            bytes = bytes
                .checked_add(u64::try_from(certificate.len()).expect("bounded certificate"))
                .ok_or("trust roots exceed their byte bound")?;
            if bytes > TRUST_BYTES || certificates.len() >= 512 {
                return Err("trust roots exceed their combined bound".into());
            }
            certificates.push(certificate);
        }
        malformed = malformed.checked_add(roots.malformed).ok_or("trust block count overflow")?;
        other_blocks = other_blocks.checked_add(roots.other_blocks).ok_or("trust block count overflow")?;
        paths.push(roots.path.display().to_string());
    }
    let (config, parsed) =
        tls::Config::from_der(&certificates, &[]).map_err(|why| format!("TLS trust {}: {why:?}", paths.join(", ")))?;
    eprintln!(
        "smith: trust roots: {} taken, {} skipped, {} malformed blocks, {} other blocks",
        parsed.taken, parsed.skipped, malformed, other_blocks
    );
    Ok(config)
}

pub(crate) fn profile_refusal(error: Refusal) -> String {
    String::from_utf8(service::profile::words(&error).into_vec()).expect("refusal words are UTF-8")
}

#[cfg(test)]
mod tests {
    mod native;
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
    fn a_trust_file_that_is_not_a_regular_file_is_refused_naming_it() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"));
        let error = trust(path.to_str()).expect_err("directory is not a certificate file");
        assert!(error.contains(path.to_str().expect("fixture path")), "refusal names the path");
        assert!(error.contains("NotAFile"), "refusal names the non-file");
    }

    #[test]
    fn a_machine_bundle_past_its_certificate_bound_is_refused_naming_the_bound() {
        let path = std::env::temp_dir().join(format!("smith-trust-bundle-{}.pem", std::process::id()));
        std::fs::write(&path, b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n".repeat(513))
            .expect("bounded PEM fixture");
        let error = trust_source(&skein_shell::Trust::Machine { ssl_cert_file: Some(path.clone()) })
            .expect_err("too many certificates");
        std::fs::remove_file(path).expect("remove fixture");
        assert_eq!(error, "trust roots exceed 512 certificates");
    }

    #[test]
    fn named_endpoint_resolves_and_its_tls_trust_builds_before_start() {
        let mut document = document();
        document.endpoints.clear();
        document.endpoints.push(Endpoint {
            connect: 10_000,
            handshake: 10_000,
            models: vec![ModelDocument {
                prices: PriceDocument { input: 1, cached: 1, cache_write: None, output: 1, unit: 1 },
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
            trust: None,
            headers: vec![],
            identity: None,
        });
        let configuration = build(document).expect("resolved endpoint");
        service::Service::new(configuration.service, 1).expect("LLM and channel endpoints");
    }

    #[test]
    fn affinity_header_names_and_static_cache_keys_are_refused_in_configuration() {
        for name in ["session-id", "SESSION-ID", "thread-id", "ThReAd-Id"] {
            let json = format!(
                r#"{{"profile":{{"name":"standard","declared":{{"memory":1099511627776,"tool_payload":256,"read_window":256}},"policy":{{"group_stop":10}}}},"endpoints":[{{"name":"local","number":1,"dialect":1,"account":0,"provider":"codex","address":"127.0.0.1:8080","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"prices":{{"input":1,"cached":1,"output":1,"unit":1}},"head":60000,"idle":30000}}],"transport":"plaintext","headers":[{{"name":"{name}","value":"host override"}}]}}],"environment":[]}}"#
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
                "address":"{address}","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"prices":{{"input":1,"cached":1,"output":1,"unit":1}},"head":60000,"idle":30000}}],"transport":"plaintext"}}],"environment":[]}}"#
            );
            let configuration = parse(json.as_bytes()).expect("loopback plaintext configuration");
            service::Service::new(configuration.service, 1).expect("loopback plaintext service");
        }
        for address in ["192.0.2.1:8080", "[2001:db8::1]:8080"] {
            let json = format!(
                r#"{{"profile":{{"name":"standard","declared":{{"memory":1099511627776,"tool_payload":256,"read_window":256}},"policy":{{"group_stop":10}}}},
                "endpoints":[{{"name":"local","number":1,"dialect":1,"account":0,"provider":"codex",
                "address":"{address}","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"prices":{{"input":1,"cached":1,"output":1,"unit":1}},"head":60000,"idle":30000}}],"transport":"plaintext"}}],"environment":[]}}"#
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
                "address":"127.0.0.1:8080","connect":10000,"handshake":10000,"models":[{{"name":"test","window":65536,"output":512,"reasoning_item":64,"prices":{{"input":1,"cached":1,"output":1,"unit":1}},"head":60000,"idle":30000}}],{extra}}}],"environment":[]}}"#
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
            duration(u64::MAX, endpoint).err().map(ConfigurationRefusal::detail).as_deref(),
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
            "profile":{"name":"standard","declared":{"memory":2_147_483_648_u64,"tool_payload":256,"read_window":256}},
            "endpoints":[{"name":"local","number":0,"dialect":0,"account":0,"provider":"codex",
                "address":"127.0.0.1:8080","transport":"plaintext","connect":10000,"handshake":10000,
                "models":[{"name":"test","window":65536,"output":512,"reasoning_item":64,"prices":{"input":1,"cached":1,"output":1,"unit":1},"head":60000,"idle":30000}]}],
            "environment":[]
        })
    }

    fn parse_value(value: &serde_json::Value) -> Result<Configuration, String> {
        parse(&serde_json::to_vec(value).expect("configuration JSON"))
    }

    #[test]
    fn omitted_memory_uses_the_standard_process_declaration() {
        let mut document = shaped_document();
        document["profile"]["declared"].as_object_mut().expect("declarations").remove("memory");
        let configuration = parse_value(&document).expect("default memory fits");
        assert_eq!(configuration.memory, 2_147_483_648);
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

    fn product_document() -> serde_json::Value {
        serde_json::json!({
          "profile":{"name":"standard"},
          "endpoints":[{"name":"codex","number":1,"dialect":1,"account":0,"provider":"codex",
            "address":"127.0.0.1:443","transport":"tls","server_name":"chatgpt.com",
            "connect":10000,"handshake":10000,
            "models":[{"name":"gpt-6-luna","window":272_000,"output":32000,"reasoning_item":65536,"prices":{"input":1,"cached":1,"output":1,"unit":1},"head":60000,"idle":300_000}]}],
          "environment":[{"name":"X","value":"x".repeat(833)}],
          "trace":{"path":"/tmp/unused-diagnostic-trace","capture":"everything"}
        })
    }

    fn all_product_models(mut document: serde_json::Value) -> serde_json::Value {
        let mut astra = document["endpoints"][0]["models"][0].clone();
        astra["name"] = "gpt-6-astra".into();
        document["endpoints"][0]["models"].as_array_mut().unwrap().push(astra);
        let mut anthropic = document["endpoints"][0].clone();
        anthropic["name"] = "anthropic".into();
        anthropic["number"] = 2.into();
        anthropic["dialect"] = 2.into();
        anthropic["provider"] = "anthropic".into();
        anthropic["server_name"] = "api.anthropic.com".into();
        anthropic["identity"] = "claude-code".into();
        let mut models = Vec::new();
        for name in ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"] {
            let mut model = document["endpoints"][0]["models"][0].clone();
            model["name"] = name.into();
            model["window"] = 200_000.into();
            model["idle"] = 120_000.into();
            models.push(model);
        }
        anthropic["models"] = models.into();
        document["endpoints"].as_array_mut().unwrap().push(anthropic);
        let trust = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/protocol-llm/fixtures/root.der");
        for endpoint in document["endpoints"].as_array_mut().unwrap() {
            endpoint["trust_der"] = trust.to_str().expect("fixture trust path").into();
        }
        document
    }

    fn assert_native_prompt_line(line: &[u8], native: &shared::Prompt, text: &[u8], credential: &shared::Credential) {
        // Native conformance already feeds real append bytes through Reader.
        // At the largest writer bound use a separate JSON implementation;
        // this does not charge or re-test the consumer's tokenizer machinery.
        let decoded: serde_json::Value = serde_json::from_slice(line).expect("independent whole-line JSON");
        let prompt = &decoded["prompt"];
        assert_eq!(prompt["system"], "native system", "the actual native system is emitted");
        let messages = prompt["messages"].as_array().expect("ordered message array");
        assert_eq!(messages.len(), native.messages.len(), "every native source message survives");
        let expected = std::str::from_utf8(text).expect("native admitted UTF8");
        for message in messages {
            assert_eq!(message["role"], "user", "native role survives");
            let blocks = message["blocks"].as_array().expect("ordered block array");
            assert_eq!(blocks.len(), 1, "one actual native block per message");
            assert_eq!(blocks[0]["type"], "text", "actual native text kind");
            assert_eq!(blocks[0]["text"], expected, "every escaped native payload survives");
        }
        assert!(
            !std::str::from_utf8(line)
                .expect("JSON UTF8")
                .contains(std::str::from_utf8(&credential.access_token).expect("fixture access UTF8")),
            "grant access bytes never enter content"
        );
        assert!(
            !std::str::from_utf8(line)
                .expect("JSON UTF8")
                .contains(std::str::from_utf8(&credential.account_id).expect("fixture account UTF8")),
            "grant account bytes never enter content"
        );
    }

    #[test]
    fn native_validated_dense_prompts_fit_the_actual_joint_receiving_ceiling() {
        let configured = parse_value(&product_document()).expect("actual product startup");
        let client = configured.service.limits.llm.adapter.client;
        let events = configured.service.limits.events.expect("actual product destination");
        let ceiling = events.sinks.line_ceiling(Capture::Everything).expect("joint ceiling");
        let credential = shared::Credential {
            access_token: b"secret-prompt-bound-token".as_slice().into(),
            account_id: b"secret-prompt-bound-account".as_slice().into(),
        };
        for dense in [true, false] {
            let count = if dense { client.history_items } else { 8 };
            let text = if dense { b"x".as_slice().into() } else { vec![0; 256_000].into_boxed_slice() };
            let native = shared::Prompt {
                model: b"gpt-6-luna".as_slice().into(),
                instructions: b"native system".as_slice().into(),
                tools: Box::new([]),
                choice: shared::ToolChoice::None,
                messages: (0..count)
                    .map(|_| shared::Message {
                        role: shared::Role::User,
                        content: Box::new([shared::Block::Text { text: text.clone(), replay: None }]),
                    })
                    .collect(),
                reasoning_effort: None,
                affinity: None,
                max_output_tokens: None,
            };
            let measured = shared::client::measure(&native, &credential, &shared::Endpoint::codex(), &client)
                .expect("the actual native writer admits the complete source payload");
            assert!(measured.body <= client.request, "native body stays within its owning cap");
            let mut messages = List::with_capacity(count);
            for _ in 0..count {
                let mut blocks = List::with_capacity(1);
                blocks.push(smith_events::Block::Text { text: text.clone() }).expect("one actual block");
                messages
                    .push(smith_events::Message { role: smith_events::Role::User, blocks })
                    .expect("actual admitted history");
            }
            let record = smith_events::Record {
                t_ms: u64::MAX,
                event: smith_events::Event::ResponseStarted(smith_events::ResponseStarted {
                    run: u64::MAX,
                    conversation: u64::MAX,
                    response: u64::MAX,
                    turn: u64::MAX,
                    attempt: u64::MAX,
                    window: u64::MAX,
                    model: smith_events::Model {
                        endpoint: b"codex".as_slice().into(),
                        model: native.model.clone(),
                        window_tokens: 272_000,
                        output_tokens: 32_000,
                        effort: Some(b"low".as_slice().into()),
                    },
                    messages: u64::from(count),
                    output_tokens: 32_000,
                    tool_choice: smith_events::ToolChoice::None,
                    prompt: Some(smith_events::Prompt {
                        system: Some(native.instructions.clone()),
                        tools: Some(List::with_capacity(0)),
                        messages,
                    }),
                }),
            };
            let line = smith_events::write_with_ceiling(
                &record,
                &smith_events::Capture::Everything,
                &events.sinks.records,
                ceiling,
            )
            .expect("source-admitted prompt fits both semantic and joint caps")
            .expect("actual response start is emitted");
            assert!(
                line.len() > if dense { 1_000_000 } else { 12_000_000 },
                "exercise dense framing and escaped payload maxima"
            );
            assert_native_prompt_line(&line, &native, &text, &credential);
        }
    }

    #[test]
    #[expect(clippy::too_many_lines, reason = "one startup-bound fixture records every concrete owning layer")]
    fn product_values_pass_every_startup_check_and_fit_complete_memory() {
        use skein_io::kernel::{Complete, Fd, Submit};
        use skein_lib::Queue;

        let document = all_product_models(product_document());
        // Record each owner and the complete checked process bound before
        // checking the concrete parser's startup relationships.
        let dto: Document = serde_json::from_value(document.clone()).expect("product DTO");
        let selected = build_profile(&dto.profile).expect("product profile");
        let declarations = service::profile::Configuration {
            environment_bytes: 835,
            endpoints: dto.endpoints.iter().map(|endpoint| build_models(endpoint).unwrap()).collect(),
        };
        let mut derived =
            service::profile::derive(&selected, &declarations).expect("all product startup relationships");
        let effort = smith_charter::CEILINGS.llm_effort_some;
        let configured = build_endpoints(dto.endpoints, &derived.llm, &selected).expect("product endpoints");
        let destinations = llm::Endpoints::new(configured.llm, ENDPOINTS, ACCOUNTS).expect("numeric destinations");
        let header = effective_header(&selected, &declarations, &derived, &configured.channel, &destinations)
            .expect("product header");
        let events = service::EventLimits::derive(
            &derived,
            Capture::Everything,
            1,
            effort,
            header.as_slice(),
            selected.policy.write_deadline,
        )
        .expect("product event ownership");
        println!(
            "PRODUCT_EVENTS string={} content={} items={} encoded_line={} metadata_string={}",
            events.sinks.records.string,
            events.sinks.records.content,
            events.sinks.records.items,
            events.sinks.line_ceiling(Capture::Everything).unwrap(),
            events.sinks.metadata_string
        );
        derived.events = Some(events);
        let run = run::worst_case(&derived.domain.run).expect("run bound");
        let session = domain::session::worst_case(&derived.domain.session).expect("session bound");
        let tools = domain::tools::worst_case(&derived.domain.session.tools).expect("tools bound");
        println!("PRODUCT_DOMAIN run={run} session_including_tools={session} tools={tools}");
        for endpoint in destinations.destinations() {
            let client = endpoint.limits;
            let native = client.native();
            println!(
                "PRODUCT_NATIVE fixed={:?} encoding_scratch={:?} openai_receiving={:?} anthropic_receiving={:?} request={} answer={} reasoning={} tokens={} output_items={} history_items={} receiving={} skip={}",
                skein_llm::client::worst_case(&client),
                skein_llm::client::encoding_worst_case(&client),
                skein_llm::openai::worst_case(&native),
                skein_llm::anthropic::worst_case(&native),
                client.request,
                client.answer,
                client.reasoning,
                client.tokens,
                client.output_items,
                client.history_items,
                client.receiving,
                client.skip
            );
        }
        println!(
            "PRODUCT_LLM connection={:?} adapter_entrance={:?} tool_bytes={} contract_bytes={}",
            skein_llm_connection::worst_case(&derived.llm.connection, destinations.destinations()),
            llm::worst_case(&derived.llm.adapter, &derived.llm.receiving),
            derived.llm.adapter.tool_bytes,
            derived.llm.contract_bytes
        );
        println!(
            "PRODUCT_PARTIAL domain={:?} channel={:?} llm={:?} machine={:?} io={:?} start={} output={} history={} history_items={} completion={}",
            domain::worst_case(&derived.domain),
            channel::worst_case(&derived.channel),
            llm::component_worst_case(&derived.llm, &destinations),
            machine::worst_case(&derived.machine),
            skein_io::worst_case(&derived.io),
            derived.channel.start_bytes,
            derived.channel.channel.output_bytes,
            derived.domain.session.history_bytes,
            derived.domain.session.history_blocks,
            derived.domain.session.completion_bytes
        );
        let limits = &derived;
        assert!(limits.file_bytes >= limits.domain.session.tools.file_bytes);
        println!(
            "PRODUCT_FILE_IO whole={} tools={} machine={} bound={}",
            limits.file_bytes,
            limits.domain.session.tools.file_bytes,
            limits.machine.file_bytes,
            skein_io::file_layer::FileIo::worst_case(
                limits.file_slots,
                limits.file_read,
                limits.file_entries,
                limits.file_bytes
            )
            .expect("whole-file driver bound")
        );
        let service = service::worst_case(limits, &destinations).expect("service bound");
        let roots = limits.domain.run.directories;
        let queue =
            limits.queue.checked_add(roots).and_then(|n| n.checked_add(2)).expect("shell queues and trace root");
        let shell = Queue::<Complete>::worst_case(queue).expect("complete queue")
            + Queue::<Submit>::worst_case(queue).expect("submit queue")
            + u64::try_from(size_of::<crate::Agent>()).expect("agent cell")
            + u64::from(roots) * u64::try_from(size_of::<Fd>()).expect("root cells");
        let header_limits = effective_header(&selected, &declarations, limits, &configured.channel, &destinations)
            .expect("complete header");
        let header_names: u64 = header_limits.iter().map(|entry| u64::try_from(entry.name.len()).unwrap()).sum();
        let header_inventory =
            List::<smith_events::EffectiveLimit>::worst_case(header_limits.len()).unwrap() + header_names;
        let header = header_inventory
            + u64::try_from(
                b"smith".len() + env!("CARGO_PKG_VERSION").len() + env!("SMITH_BUILD").len() + b"standard".len(),
            )
            .unwrap();
        let complete = service + header + shell;
        println!(
            "PRODUCT_COMPLETE service={service} startup_header={header} shell={shell} complete={complete} memory={}",
            limits.memory
        );
        parse_value(&document).expect("product declarations pass every startup check");
        assert!(complete <= limits.memory, "complete {complete} > declared {}", limits.memory);
    }
    #[test]
    fn drawn_declarations_are_named_or_fit_complete_process_memory() {
        use service::profile::{Check, validate};
        use skein_io::kernel::{Complete, Fd, Submit};
        use skein_lib::{Queue, Rng};

        let mut rng = Rng::new(4_170_005);
        let (mut admitted, mut relationship_refusals, mut memory_refusals) = (0, 0, 0);
        for sample in 0..96 {
            let mut document = product_document();
            // All 96 draws exercise the real bounded DER loader. Loading the
            // machine's complete certificate store twice for each draw is
            // unrelated to the drawn memory and quantity relationships.
            document["endpoints"][0]["trust_der"] = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/protocol-llm/fixtures/root.der")
                .to_str()
                .expect("fixture path")
                .into();
            let payload = rng.next_u64() % 40_960 + 1;
            let declared = &mut document["profile"]["declared"];
            declared["conversations"] = (rng.next_u64() % 6 + 1).into();
            declared["tool_payload"] = payload.into();
            declared["file_bytes"] = (rng.next_u64() % 1_048_576 + payload).into();
            declared["calls_per_response"] = (rng.next_u64() % 16 + 1).into();
            declared["read_window"] =
                if sample % 6 == 0 { (payload + 1).into() } else { (rng.next_u64() % payload + 1).into() };
            // Draw both the declared memory ceiling and the quantities which
            // determine the owned bytes. Some otherwise valid draws must fail.
            declared["memory"] = if sample % 6 == 1 { 1_u64.into() } else { 2_147_483_648_u64.into() };
            let model = &mut document["endpoints"][0]["models"][0];
            model["window"] = (rng.next_u64() % 272_000 + 272_000).into();
            model["output"] = (rng.next_u64() % 32_000 + 32_000).into();
            let dto: Document = serde_json::from_value(document.clone()).expect("drawn DTO");
            let profile = build_profile(&dto.profile).expect("positive drawn declarations");
            let declarations = service::profile::Configuration {
                environment_bytes: 835,
                endpoints: dto.endpoints.iter().map(|endpoint| build_models(endpoint).unwrap()).collect(),
            };
            let mut limits = match service::profile::derive(&profile, &declarations) {
                Ok(limits) => limits,
                Err(refusal) => {
                    let check = refusal.check().expect("draw names a startup relationship");
                    let words = profile_refusal(refusal);
                    assert!(words.starts_with(check.name()), "{words}");
                    assert_eq!(parse_value(&document).err().as_deref(), Some(words.as_str()));
                    relationship_refusals += 1;
                    continue;
                }
            };
            validate::declarations(&profile, &declarations).expect("all drawn declaration comparisons");
            validate::window_first(&profile, &declarations, &limits.domain.session).expect("whole window and batch");
            validate::machine(&limits.machine, &limits.domain.session.tools).expect("machine holds tools");
            assert!(limits.file_bytes >= limits.domain.session.tools.file_bytes);
            let effort = smith_charter::CEILINGS.llm_effort_some;
            let events = service::EventLimits::derive(
                &limits,
                Capture::Everything,
                1,
                effort,
                &[],
                profile.policy.write_deadline,
            )
            .expect("drawn event ownership");
            limits.events = Some(events);
            let configured = build_endpoints(dto.endpoints, &limits.llm, &profile).expect("drawn endpoints");
            let destinations =
                llm::Endpoints::new(configured.llm, ENDPOINTS, ACCOUNTS).expect("drawn native admission");
            let service_bytes = service::worst_case(&limits, &destinations).expect("drawn service bound");
            // The shell may hold every inherited root, its error stream and
            // the append file's temporary root before its first pass.
            let roots = limits.domain.run.directories;
            let queue = limits.queue.checked_add(roots).and_then(|value| value.checked_add(2)).unwrap();
            let header = u64::try_from(
                b"smith".len() + env!("CARGO_PKG_VERSION").len() + env!("SMITH_BUILD").len() + b"standard".len(),
            )
            .unwrap();
            let complete = service_bytes
                .checked_add(header)
                .and_then(|value| value.checked_add(Queue::<Complete>::worst_case(queue)?))
                .and_then(|value| value.checked_add(Queue::<Submit>::worst_case(queue)?))
                .and_then(|value| value.checked_add(u64::try_from(size_of::<crate::Agent>()).ok()?))
                .and_then(|value| value.checked_add(u64::from(roots) * u64::try_from(size_of::<Fd>()).ok()?))
                .expect("drawn complete bound");
            match validate::memory(complete, limits.memory) {
                Ok(()) => {
                    assert!(complete <= profile.declared.memory);
                    parse_value(&document).expect("all concrete startup comparisons");
                    admitted += 1;
                }
                Err(refusal) => {
                    assert_eq!(refusal.check(), Some(Check::Memory));
                    let words = profile_refusal(refusal);
                    assert!(words.contains(&format!("service.worst_case={complete} bytes")), "{words}");
                    assert!(words.contains(&format!("declared.memory={} bytes", limits.memory)), "{words}");
                    assert!(complete > limits.memory);
                    memory_refusals += 1;
                }
            }
        }
        assert!(admitted > 0 && relationship_refusals > 0 && memory_refusals > 0);
        println!(
            "DRAWN_STARTUP samples=96 admitted={admitted} relationship_refusals={relationship_refusals} memory_refusals={memory_refusals}"
        );
    }
}
