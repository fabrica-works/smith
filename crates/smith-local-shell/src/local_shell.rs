//! Local host command and shell effects (protocol/hosts.md, sections 5.1–5.6).
//! The shell owns paths, file descriptors and child launch. The local service
//! owns chat and run decisions. One Skein kernel loop drives terminal input,
//! signals, the hosted agent and their deadlines.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use skein_io::kernel::{Exit, Fd};
use skein_shell::{Clock, Config as KernelConfig, Host, Kernel, drive};
use smith_host_domain as host;
use smith_host_protocol as protocol;
use smith_local_domain as local;
use smith_local_service as service;

use smith_agent_shell::config;

use crate::{
    local_host::{Local, Resources},
    local_settings,
};

pub fn run(settings_path: &Path, state_root: &Path, workspace_settings: Option<&Path>) -> Result<Exit, String> {
    let settings = local_settings::read(settings_path, workspace_settings)?;
    if settings.delivery_environment.len() > 64
        || settings.delivery_environment.iter().map(String::len).sum::<usize>() > 4096
        || settings.delivery_environment.iter().any(|entry| !entry.contains('=') || entry.contains('\0'))
    {
        return Err("delivery environment exceeds its bounds or has invalid entries".into());
    }
    let mut delivery_roots = Vec::with_capacity(settings.directories.len());
    for directory in &settings.directories {
        delivery_roots.push(
            skein_shell::open_root(Path::new(&directory.path))
                .map_err(|error| format!("delivery root {}: {error}", directory.path))?,
        );
    }
    let agent_bytes = local_settings::agent_document(&settings)?;
    let agent_config = config::parse(&agent_bytes)?;
    let endpoints = agent_config.service.channel_endpoints.clone();
    let mut prepared = policy(&settings, &agent_config.service)?;
    fs::create_dir_all(state_root).map_err(|error| format!("state directory: {error}"))?;
    let agent_path = state_root.join("agent.json");
    save_agent_config(&agent_path, &agent_bytes)?;
    let chat_path = state_root.join(&settings.chat);
    let token_path = token_directory(settings.token_directory.as_deref())?;
    let parent = token_path.parent().ok_or("token directory has no parent")?;
    fs::create_dir_all(parent).map_err(|error| format!("token directory parent: {error}"))?;
    let mut effect_roots = Vec::new();
    if settings.in_process {
        for directory in &settings.directories {
            effect_roots.push(
                skein_shell::open_root(Path::new(&directory.path))
                    .map_err(|error| format!("agent root {}: {error}", directory.path))?,
            );
        }
    }
    let launch_root = skein_shell::open_root(Path::new(".")).map_err(|error| format!("launch root: {error}"))?;
    let executable = std::env::current_exe().map_err(|error| format!("agent program path: {error}"))?;
    let largest_turn =
        smith_domain::max_turn_bytes(&agent_config.service.limits.domain).ok_or("agent turn bound overflows")?;
    let host_limits = service::profile::host_limits(
        &agent_config.profile,
        largest_turn,
        agent_config.service.limits.domain.run.message_bytes,
    );
    if !settings.in_process {
        prepared.limits.agent = local::AgentLimits::Spawned(host_limits);
    }
    let mut limits = service_limits(prepared.limits, host_limits, &agent_config.service)?;
    if settings.in_process {
        limits.inline = Some(service::InlineLimits::new(&agent_config.service).ok_or_else(|| {
            profile_refusal(smith_agent_service::profile::Refusal::Derived {
                limit: smith_agent_service::profile::LimitName::deployment(
                    smith_agent_service::profile::Layer::Service,
                    "local_worst_case",
                ),
            })
        })?);
    }
    service::profile::memory(&limits, agent_config.memory).map_err(profile_refusal)?;
    let seed = skein_shell::seed().map_err(|error| format!("local seed failed (errno {error})"))?;
    let local_config = service::Config {
        agent: service::AgentConfig::Spawned,
        local: prepared.config,
        limits,
        endpoints,
        paths: prepared.paths,
        launch: service::Launch {
            program: executable.as_os_str().as_bytes().into(),
            arguments: Box::new([Box::from(&b"agent"[..]), agent_path.as_os_str().as_bytes().into()]),
            environment: Box::new([]),
            root: launch_root,
            directory: Box::from(&b"."[..]),
        },
    };
    let signals =
        skein_shell::open_termination_signals().map_err(|error| format!("local signals failed (errno {error})"))?;
    let oauth_entropy = oauth_entropy()?;
    let lower = settings.in_process.then_some(agent_config.service);
    let mut local_service = Local::within_memory(
        local_config,
        lower,
        Resources {
            state_directory: chat_path,
            token_directory: token_path,
            input: Fd::new(0),
            output: Fd::new(1),
            signals,
            delivery_roots: delivery_roots.into(),
            effect_roots: effect_roots.into(),
            delivery_environment: settings.delivery_environment.iter().map(|entry| entry.as_bytes().into()).collect(),
            accounts: settings.accounts.into(),
            seed,
            oauth_entropy,
        },
        Box::new(std::io::stderr()),
        agent_config.memory,
    )?;
    let mut kernel = Kernel::open(KernelConfig { operations: local_service.operations() })
        .map_err(|error| format!("local kernel: {error}"))?;
    Ok(drive(&mut kernel, &Clock::new(), &mut local_service))
}

/// Collect the fixed startup entropy required by the OAuth composition.
fn oauth_entropy() -> Result<[u8; 32], String> {
    let mut entropy = [0_u8; 32];
    for part in entropy.chunks_exact_mut(8) {
        let seed = skein_shell::seed().map_err(|error| format!("OAuth entropy failed (errno {error})"))?;
        part.copy_from_slice(&seed.to_be_bytes());
    }
    Ok(entropy)
}

/// Carry each actual model's startup framing measurement into local admission.
fn policy(
    settings: &local_settings::Settings,
    agent: &smith_agent_service::Config,
) -> Result<local_settings::Prepared, String> {
    let endpoints = &agent.channel_endpoints;
    let mut protocol_allowances = Vec::with_capacity(settings.models.len());
    for model in &settings.models {
        let endpoint = endpoints.resolve(model.endpoint.as_bytes()).ok_or("local model endpoint is absent")?;
        let configured = agent
            .domain
            .models
            .iter()
            .find(|configured| {
                configured.endpoint.0 == endpoint.number && configured.model.as_ref() == model.name.as_bytes()
            })
            .ok_or("local model declaration is absent")?;
        protocol_allowances.push(configured.protocol_allowance);
    }
    local_settings::policy(settings, &agent.channel_endpoints, agent.limits.domain, protocol_allowances.into())
}

/// Bound the common local IO and the selected child before constructing either.
fn service_limits(
    local: local::Limits,
    host: host::Limits,
    agent: &smith_agent_service::Config,
) -> Result<service::Limits, String> {
    let queue = local::max_out(&local).max(host::max_out(&host)).max(256);
    let mut process = service::ProcessLimits {
        io: agent.limits.io,
        channel: protocol::Limits {
            bodies: agent.limits.channel.bodies,
            charter: agent.limits.channel.charter,
            transcript: agent.limits.channel.transcript,
            endpoints: agent.limits.channel.endpoints,
            channel: agent.limits.channel.channel,
            start_bytes: agent.limits.channel.start_bytes,
            calls: host.calls,
        },
        detail_bytes: host.detail_bytes,
        queue,
    };
    process.channel.channel.output_bytes = protocol::output_bytes(&process.channel).ok_or_else(|| {
        profile_refusal(smith_agent_service::profile::Refusal::Derived {
            limit: smith_agent_service::profile::LimitName::deployment(
                smith_agent_service::profile::Layer::Channel,
                "host_output_bytes",
            ),
        })
    })?;
    Ok(service::Limits { local, inline: None, envelope_bytes: host.answer_bytes, process, queue })
}

#[expect(
    clippy::disallowed_types,
    reason = "10-product composes the inline agent and writes no agent configuration file"
)]
fn save_agent_config(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("agent configuration has no directory")?;
    let temporary = parent.join(".agent.json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|error| format!("agent configuration open: {error}"))?;
    file.write_all(bytes).map_err(|error| format!("agent configuration write: {error}"))?;
    file.sync_all().map_err(|error| format!("agent configuration sync: {error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("agent configuration rename: {error}"))?;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("agent configuration directory sync: {error}"))
}

fn token_directory(configured: Option<&str>) -> Result<std::path::PathBuf, String> {
    if let Some(configured) = configured {
        return Ok(configured.into());
    }
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(base) => std::path::PathBuf::from(base),
        None => std::path::PathBuf::from(std::env::var_os("HOME").ok_or("HOME is absent; configure token_directory")?)
            .join(".config"),
    };
    Ok(base.join("smith/tokens"))
}

fn profile_refusal(refusal: smith_agent_service::profile::Refusal) -> String {
    String::from_utf8(smith_agent_service::profile::words(&refusal).into_vec()).expect("refusal words are UTF-8")
}

#[cfg(test)]
#[test]
fn product_host_startup_and_complete_outer_memory_fit() {
    let mut environment = Vec::new();
    for index in 0..10 {
        environment.push(serde_json::json!({"name":format!("X{index}"),"value":if index == 0 { "x".repeat(853) } else { String::new() }}));
    }
    let settings: local_settings::Settings = serde_json::from_value(serde_json::json!({
      "agent":{
        "profile":{"name":"standard","declared":{
          "conversations":6,"tool_payload":40960,"file_bytes":1_048_576,"calls_per_response":16,
          "read_window":32768,"guide":16384,"shell_output":16384,"shell_head":16384,"shell_tail":49152,
          "search_hits":100,"search_bytes":16384,"list_entries":512,"llm_pool":134_217_728,"memory":2_147_483_648_u64},
          "policy":{"group_stop":250}},
        "endpoints":[{"name":"codex","number":1,"dialect":1,"account":0,"provider":"codex",
          "address":"127.0.0.1:443","transport":"tls","server_name":"fixture.invalid",
          "connect":10000,"handshake":10000,"reasoning_effort":"low"}],
        "environment":environment,"trace":{"path":"/unused-product-trace","capture":"everything"}},
      "chat":"fixture","instructions":"x".repeat(170),
      "models":[{"endpoint":"codex","name":"gpt-6-luna","window":272_000,"output":32000,
        "reasoning_item":65536,"head":60000,"idle":300_000,"input_price":1_250_000,
        "cached_price":125_000,"output_price":10_000_000,"price_unit":1_000_000}],
      "budget":{"turns":4096,"spend":1_000_000_000_u64,"seconds":300,"reserve":{"turns":2,"seconds":150}},
      "waiting_seconds":30,"contract":{"form":"report","max":4096},"sub_agents":true,
      "directories":[{"name":"work","path":"/synthetic/work","writable":true,"git":false}],
      "accounts":[{"number":0,"account_id":"synthetic"}],"in_process":false
    }))
    .expect("synthetic product settings");
    let agent_bytes = local_settings::agent_document(&settings).expect("agent DTO");
    let agent = config::parse(&agent_bytes).expect("every actual agent startup comparison");
    assert_eq!(agent.service.limits.machine.env_bytes, 883);
    let mut prepared = policy(&settings, &agent.service).expect("product local admission");
    let largest = smith_domain::max_turn_bytes(&agent.service.limits.domain).expect("turn bytes");
    let host = service::profile::host_limits(&agent.profile, largest, agent.service.limits.domain.run.message_bytes);
    prepared.limits.agent = local::AgentLimits::Spawned(host);
    let limits = service_limits(prepared.limits, host, &agent.service).expect("host directional derivation");
    service::profile::memory(&limits, agent.memory).expect("all local service startup comparisons");
    let schema =
        smith_protocol_channel::schema_for_start(&limits.process.channel.bodies, limits.process.channel.start_bytes)
            .expect("joint Start schema");
    schema.check(skein_channel::Role::Initiator, &limits.process.channel.channel).expect("host side admits its Start");
    let raw =
        smith_agent_service::worst_case(&agent.service.limits, &agent.service.llm_endpoints).expect("agent owners");
    let agent_limits = &agent.service.limits;
    assert!(agent_limits.file_bytes >= agent_limits.domain.session.tools.file_bytes);
    let roots = agent_limits.domain.run.directories;
    let shell_queue = agent_limits.queue.checked_add(roots).and_then(|value| value.checked_add(2)).unwrap();
    let agent_shell = skein_lib::Queue::<skein_io::kernel::Complete>::worst_case(shell_queue).unwrap()
        + skein_lib::Queue::<skein_io::kernel::Submit>::worst_case(shell_queue).unwrap()
        + u64::try_from(size_of::<smith_agent_shell::Agent>()).unwrap()
        + u64::from(roots) * u64::try_from(size_of::<Fd>()).unwrap();
    // The IO writer is already in raw. Startup retains four header strings;
    // build.rs emits forty commit bytes, plus six only for a dirty build.
    let startup_header =
        u64::try_from(b"smith".len() + env!("CARGO_PKG_VERSION").len() + 46 + b"standard".len()).unwrap();
    let agent_complete = raw + startup_header + agent_shell;
    assert!(agent_complete <= agent.memory, "complete agent product bound {agent_complete}");
    println!(
        "PRODUCT_AGENT_ACTUAL raw={raw} startup_header_bound={startup_header} shell={agent_shell} complete={agent_complete} file_io={:?} whole={} env=883",
        skein_io::file_layer::FileIo::worst_case(
            agent_limits.file_slots,
            agent_limits.file_read,
            agent_limits.file_entries,
            agent_limits.file_bytes
        ),
        agent_limits.file_bytes
    );
    let local_service = service::worst_case(&limits).expect("local service owners");
    let complete = Local::required_memory(&limits, 1, 0).expect("every actual local outer owner");
    assert!(complete <= 2_147_483_648, "complete local product bound {complete}");
    println!(
        "PRODUCT_LOCAL agent_raw={raw} service={local_service} complete={complete} limit=2147483648 host_output={} start={} env=883 accounts=1",
        limits.process.channel.channel.output_bytes, limits.process.channel.start_bytes
    );
    let mut short = limits;
    short.process.channel.channel.output_bytes -= 1;
    assert!(service::worst_case(&short).is_none(), "one byte short rejects before adopting streams");
    assert!(Local::required_memory(&short, 1, 0).is_none());
}
