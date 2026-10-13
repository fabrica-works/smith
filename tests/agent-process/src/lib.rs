//! A whole agent service on Skein's simulated kernel, with a scripted host
//! speaking the real channel schema. The fake machine owns any filesystem and
//! child-process effects. Contract: protocol/agent.md, sections 4–6.

use skein_channel::{Role, StreamMode, frame_writer};
use skein_fake_channel::ScriptedPeer;
use skein_io::kernel;
use skein_lib::{Duration, List, Writer};
use smith_agent_service as agent;
use smith_protocol_channel as channel;
use smith_protocol_llm as llm;

pub mod fake;

pub mod events;
mod tracing;

/// The bounded diagnostic writer observed outside each hosted agent.
#[derive(Clone, Default)]
pub struct AgentErrors {
    observed: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
    pipe: Option<skein_world::PipeWriter>,
}

impl AgentErrors {
    /// Observe the bytes accepted by skein's shared diagnostic pipe writer.
    #[must_use]
    pub fn pipe(fd: kernel::Fd) -> (Self, skein_world::PipeCapture) {
        let (writer, capture) = skein_world::PipeWriter::new(fd, 4096);
        (Self { observed: std::rc::Rc::default(), pipe: Some(writer) }, capture)
    }

    /// Snapshot the agent's words without reading its service state.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        self.observed.borrow().clone()
    }
}

impl std::io::Write for AgentErrors {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = match &mut self.pipe {
            Some(pipe) => std::io::Write::write(pipe, bytes)?,
            None => bytes.len(),
        };
        let mut output = self.observed.borrow_mut();
        assert!(output.len().checked_add(count).is_some_and(|size| size <= 4096), "finite agent diagnostics");
        output.extend_from_slice(bytes.get(..count).expect("accepted diagnostic prefix"));
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.pipe {
            Some(pipe) => std::io::Write::flush(pipe),
            None => Ok(()),
        }
    }
}

/// One real tiny model declaration for startup controls and process worlds.
#[must_use]
pub fn declarations(environment_bytes: u32) -> agent::profile::Configuration {
    use agent::profile::{Configuration, Endpoint, Model, OversizedReasoning};
    Configuration {
        environment_bytes,
        endpoints: Box::new([Endpoint {
            number: 0,
            provider: skein_llm::Provider::OpenAiCodex,
            option_bytes: 0,
            connect: skein_lib::Duration::from_secs(1),
            handshake: skein_lib::Duration::from_secs(1),
            models: Box::new([Model {
                name: Box::from(b"fake".as_slice()),
                window: 1_048_576,
                output: 512,
                reasoning_item: 64,
                oversized_reasoning: OversizedReasoning::Fail,
                head: skein_lib::Duration::from_secs(1),
                idle: skein_lib::Duration::from_secs(1),
            }]),
        }]),
    }
}

/// Adopt scenario-specific service settings and mounts in Start order.
#[must_use]
pub fn configured(
    configuration: agent::Config,
    seed: u64,
    inherited: &skein_world::Inherited,
    errors: AgentErrors,
) -> smith_agent_shell::Agent {
    let environment_bytes = configuration.limits.machine.env_bytes;
    smith_agent_shell::Agent::new(
        smith_agent_shell::config::Configuration {
            service: configuration,
            memory: profile().declared.memory,
            trace: None,
            profile: profile(),
            declarations: declarations(environment_bytes),
        },
        smith_agent_shell::Resources {
            input: inherited.pipes.iter().find(|(child, _)| *child == 0).expect("stdin").1,
            output: inherited.pipes.iter().find(|(child, _)| *child == 1).expect("stdout").1,
            error: Some(inherited.pipes.iter().find(|(child, _)| *child == 2).expect("stderr").1),
            signals: inherited.signal,
            seed,
            pid: 101,
            trace: None,
            roots: Some(inherited.roots.iter().map(|(_, fd)| *fd).collect()),
        },
        Box::new(errors),
    )
    .expect("configured shipped agent")
}

/// Tiny deployment declarations shared by direct and hosted process worlds.
#[must_use]
pub fn profile() -> agent::profile::Profile {
    let mut profile = agent::profile::standard();
    profile.declared.conversations = smith_agent_world::LIMITS.run.conversations;
    profile.declared.calls_per_response = smith_agent_world::LIMITS.run.calls;
    profile.declared.tool_payload = 256;
    profile.declared.file_bytes = 65_536;
    profile.declared.shell_output = 512;
    profile.declared.read_window = 16;
    profile.declared.shell_head = 64;
    profile.declared.shell_tail = 64;
    profile.declared.search_hits = 8;
    profile.declared.search_bytes = 256;
    profile.declared.list_entries = 2;
    profile.declared.guide = 128;
    profile.declared.memory = 2_147_483_648;
    profile.declared.llm_pool = agent::profile::standard().declared.llm_pool;
    profile.policy.max_turns = 512;
    profile.policy.max_time = Duration::from_secs(3600);
    profile.policy.inbox = 8;
    profile.policy.sections = 4;
    profile.policy.host_tools = 2;
    profile.policy.verdicts = 2;
    profile.policy.check_timeout = Duration::from_secs(60);
    profile.policy.unacknowledged = 8;
    profile.policy.group_stop = Duration::from_millis(10);
    profile
}

/// Bounded agent-process settings derived from the tiny deployment and model.
#[must_use]
pub fn limits() -> agent::Limits {
    let profile = profile();
    let configuration = declarations(0);
    let mut limits = agent::profile::derive(&profile, &configuration).expect("tiny profile derives");
    // Worlds may lower or specialize a derived limit (protocol/limits.md2.2).
    limits.domain.run.models = 2;
    limits.domain.configured_model_bytes = 4096;
    limits.llm.connection.endpoints = 1;
    limits.llm.connection.connections = 2;
    limits.llm.connection.calls = 2;
    limits.llm.connection.per_endpoint = 2;
    limits.llm.accounts = 1;
    limits.llm.grant_value_bytes = 128;
    limits.machine.file_bytes = 16;
    limits.file_timeout = Duration::from_secs(1);
    limits.queue = smith_domain::max_out(&limits.domain).max(256);
    limits
}

/// Build the same agent service for a direct or spawned process world.
#[must_use]
pub fn service(seed: u64) -> agent::Service {
    agent::Service::new(configuration(), seed).expect("bounded agent")
}

/// The shared lower configuration for direct, spawned and colocated process worlds.
#[must_use]
pub fn configuration() -> agent::Config {
    configuration_for(false)
}

fn configuration_for(anthropic: bool) -> agent::Config {
    configuration_with_limits(anthropic, &limits())
}

fn configuration_with_limits(anthropic: bool, limits: &agent::Limits) -> agent::Config {
    let mut endpoints = List::with_capacity(1);
    endpoints
        .push(channel::Endpoint { name: Box::default(), number: 0, dialect: 0, account: 0 })
        .expect("one endpoint");
    let llm_endpoints = llm::Endpoints::new(
        Box::new([llm::ConfiguredEndpoint {
            connect: skein_lib::Duration::from_secs(1),
            handshake: skein_lib::Duration::from_secs(1),
            name: smith_domain::llm::Endpoint(0),
            destination: skein_llm_connection::Endpoint {
                limits: limits.llm.adapter.client,
                credential: skein_llm::client::CredentialLimits {
                    access_token: limits.llm.grant_value_bytes,
                    account_id: limits.llm.grant_value_bytes,
                },
                address: kernel::Addr::from((std::net::Ipv4Addr::LOCALHOST, 443)),
                transport: skein_llm_connection::Transport::Plaintext,
                llm: if anthropic { skein_llm::Endpoint::anthropic() } else { skein_llm_world::call(7).endpoint },
            },
            account: 0,
            identity: llm::IdentityProfile::Plain,
            models: Box::new([]),
        }]),
        1,
        1,
    )
    .expect("one LLM endpoint");
    agent::Config {
        limits: *limits,
        domain: smith_domain::Config {
            endpoints: Box::new([smith_domain::run::charter::Endpoint(0)]),
            models: Box::new([smith_domain::ConfiguredModel {
                prices: smith_domain::run::Prices {
                    input: 1,
                    cached: 1,
                    cache_write: None,
                    output: 1,
                    unit: 1_000_000,
                },
                protocol_allowance: agent::profile::endpoint_allowance(
                    &profile(),
                    &declarations(0).endpoints[0],
                    &limits.domain.session.tools,
                )
                .expect("derived endpoint framing"),
                endpoint: smith_domain::run::charter::Endpoint(0),
                model: Box::from(&b"fake"[..]),
                window: 1_048_576,
                output: 512,
                head: Duration::from_secs(2),
                idle: Duration::from_secs(3),
            }]),
        },
        channel_endpoints: channel::Endpoints::new(endpoints),
        llm_endpoints,
        environment: smith_protocol_machine::Environment::encode(&[], &limits.machine).expect("empty environment fits"),
        stream_mode: StreamMode::Two,
    }
}

/// The same process with the live reasoning-item entrance for the stopped-turn story.
fn reasoning_configuration() -> agent::Config {
    let mut declaration = declarations(0);
    declaration.endpoints[0].models[0].window = 272_000;
    declaration.endpoints[0].models[0].output = 16_384;
    declaration.endpoints[0].models[0].reasoning_item = 65_536;
    let limits = agent::profile::derive(&profile(), &declaration).expect("reasoning profile derives");
    let mut configuration = configuration_with_limits(false, &limits);
    configuration.domain.models[0].window = 272_000;
    configuration.domain.models[0].output = 16_384;
    configuration
}

/// The scripted channel initiator used by agent startup stories.
#[must_use]
pub fn host() -> ScriptedPeer {
    host_with_limits(limits().channel)
}

fn host_with_limits(mut limits: channel::Limits) -> ScriptedPeer {
    let schema = channel::schema_for_start(&limits.bodies, limits.start_bytes).expect("joint Start schema");
    // The scripted host sends the initiator's kinds, with the same joint
    // Start as the agent receives (protocol/limits.md, section 3, rule 6).
    let mut largest = 0;
    for version in &schema.versions {
        for kind in &version.kinds {
            match kind.direction {
                skein_channel::Direction::FromInitiator => largest = largest.max(kind.largest),
                skein_channel::Direction::FromResponder => (),
            }
        }
    }
    limits.channel.output_bytes = largest.checked_add(8).expect("host frame");
    ScriptedPeer::new(schema, Role::Initiator, limits.channel, 2, Box::new([])).expect("scripted host")
}

/// One activation Start sent by the scripted host, ending with an Answer.
#[must_use]
pub fn start(charter: &[u8]) -> skein_channel::Frame {
    start_window(charter, 8)
}

pub(crate) fn start_window(charter: &[u8], turns: u32) -> skein_channel::Frame {
    start_workspace(charter, turns, false)
}

pub(crate) fn start_workspace(charter: &[u8], turns: u32, workspace: bool) -> skein_channel::Frame {
    start_workspace_message(charter, turns, workspace, false, None, &scenario_credential(false, false))
}

pub(crate) fn start_workspace_message(
    charter: &[u8],
    turns: u32,
    workspace: bool,
    absolute: bool,
    message: Option<&smith_channel::Message>,
    credential: &skein_llm::Credential,
) -> skein_channel::Frame {
    let limits = smith_channel::CEILINGS;
    let window = smith_channel::Window::new(&limits, smith_channel::WindowParts { turns, bytes: 1_000_000_000 })
        .expect("window");
    let mut grants = List::with_capacity(1);
    grants
        .push(
            smith_channel::Grant::new(
                &limits,
                smith_channel::GrantParts {
                    account: 0,
                    generation: 1,
                    valid: Duration::from_secs(7200),
                    value: smith_channel::GrantValue::new(
                        &limits,
                        smith_channel::GrantValueParts { credential: grant_bytes(credential) },
                    )
                    .expect("credential envelope"),
                },
            )
            .expect("grant"),
        )
        .expect("one grant");
    let mut messages = List::with_capacity(u32::from(message.is_some()));
    if let Some(message) = message {
        messages.push(message.clone()).expect("one selected host message");
    }
    let record = smith_channel::Start::new(
        &limits,
        smith_channel::StartParts {
            messages,
            identity: None,
            activation: 1,
            charter: Box::from(charter),
            workspace: if workspace {
                let mut directories = List::with_capacity(2);
                for (name, path) in
                    [(b"repo".as_slice(), b"unused-a".as_slice()), (b"other".as_slice(), b"unused-b".as_slice())]
                {
                    let path = if absolute { [b"/".as_slice(), path].concat() } else { path.into() };
                    directories
                        .push(
                            smith_channel::Directory::new(
                                &limits,
                                smith_channel::DirectoryParts {
                                    name: name.into(),
                                    path: path.into(),
                                    writable: false,
                                    git: false,
                                    conflicts: List::with_capacity(0),
                                },
                            )
                            .expect("configured test directory"),
                        )
                        .expect("two directories");
                }
                Some(
                    smith_channel::Workspace::new(&limits, smith_channel::WorkspaceParts { directories })
                        .expect("two roots"),
                )
            } else {
                None
            },
            transcript: Box::default(),
            answered: List::with_capacity(0),
            grants,
            window,
        },
    )
    .expect("Start record");
    let mut body = Writer::new(usize::try_from(record.measure()).expect("body size"));
    record.encode(&mut body).expect("measured body");
    let body = body.finish();
    let mut frame = frame_writer(0x0100, u32::try_from(body.len()).expect("body length")).expect("Start frame");
    frame.put(&body).expect("measured frame");
    frame.finish().expect("full frame")
}

fn scenario_credential(anthropic: bool, private_grant: bool) -> skein_llm::Credential {
    skein_llm::Credential {
        access_token: Box::from(if private_grant {
            b"native-private-bearer-02.6".as_slice()
        } else {
            b"token".as_slice()
        }),
        account_id: if anthropic {
            Box::new([])
        } else {
            Box::from(if private_grant { b"native-private-account-02.6".as_slice() } else { b"acc".as_slice() })
        },
    }
}

fn grant_bytes(credential: &skein_llm::Credential) -> Box<[u8]> {
    let mut bytes = u16::try_from(credential.account_id.len()).expect("fixture credential").to_be_bytes().to_vec();
    bytes.extend_from_slice(&credential.account_id);
    bytes.extend_from_slice(&credential.access_token);
    bytes.into_boxed_slice()
}

/// A concrete charter within the process limits, with one report contract.
#[must_use]
pub fn charter() -> Box<[u8]> {
    let smallest = include_bytes!("../../../crates/smith-charter/golden/v2/record_charter_smallest.bin");
    let source = smith_charter::Charter::decode(&smith_charter::CEILINGS, &mut skein_lib::Reader::new(smallest))
        .expect("golden charter");
    let mut parts = source.into_parts();
    parts.budget = smith_charter::Budget::new(
        &smith_charter::CEILINGS,
        smith_charter::BudgetParts {
            turns: 1,
            spend: 2,
            time: Duration::from_secs(60),
            reserve: smith_charter::Reserve::new(
                &smith_charter::CEILINGS,
                smith_charter::ReserveParts { turns: 0, time: Duration::ZERO },
            )
            .expect("bounded reserve"),
        },
    )
    .expect("workable budget");
    parts.contract = smith_charter::Contract::new(
        &smith_charter::CEILINGS,
        smith_charter::ContractParts {
            report: Some(
                smith_charter::TextRule::new(
                    &smith_charter::CEILINGS,
                    smith_charter::TextRuleParts { max: 128, fields: List::with_capacity(0) },
                )
                .expect("report rule"),
            ),
            verdicts: List::with_capacity(0),
            change: None,
            failure: None,
        },
    )
    .expect("outcome contract");
    let mut llm = parts.main.into_parts();
    llm.model = Box::from(*b"fake");
    llm.window = declarations(0).endpoints[0].models[0].window;
    llm.output = declarations(0).endpoints[0].models[0].output;
    llm.prices = smith_charter::Prices::new(
        &smith_charter::CEILINGS,
        smith_charter::PricesParts { input: 1, cached: 1, cache_write: None, output: 1, unit: 1_000_000 },
    )
    .expect("prices");
    parts.main = smith_charter::Llm::new(&smith_charter::CEILINGS, llm).expect("model");
    let record = smith_charter::Charter::new(&smith_charter::CEILINGS, parts).expect("charter");
    let mut writer = Writer::new(usize::try_from(record.measure()).expect("charter size"));
    record.encode(&mut writer).expect("measured charter");
    writer.finish()
}

pub use world::{TraceCase, World};

pub use events::check as assert_event_conformance;

mod world;

fn acknowledge(number: u32) -> skein_channel::Frame {
    let record = smith_channel::Acknowledge::new(&smith_channel::CEILINGS, smith_channel::AcknowledgeParts { number })
        .expect("durable prefix acknowledgement");
    let mut body = Writer::new(usize::try_from(record.measure()).expect("ACK body size"));
    record.encode(&mut body).expect("ACK body");
    let mut frame = frame_writer(0x0103, record.measure()).expect("ACK frame");
    frame.put(&body.finish()).expect("ACK body fits frame");
    frame.finish().expect("ACK frame filled")
}

#[cfg(test)]
mod memory {
    use skein_world::domain::heap::{Counting, Meter};

    use super::{agent, limits, service};

    #[global_allocator]
    static HEAP: Counting = Counting;

    #[test]
    fn composed_service_start_fits_the_checked_bound() {
        let bound = agent::worst_case(&limits(), &super::configuration().llm_endpoints).expect("checked service bound");
        let meter = Meter::new();
        meter.start();
        let agent = service(7);
        let peak = meter.check(meter.end(), bound, &"service construction");
        assert!(peak > 0, "the counting allocator saw live service memory");
        drop(agent);
    }
}
