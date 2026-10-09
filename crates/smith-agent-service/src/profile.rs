//! The agent profile keeps declared deployment quantities and policy ceilings and
//! computes the service limits from them. It never knows endpoint destinations,
//! credentials or a run's charter. `derive` returns typed refusals.
//! Contract: shell.md, section 2.1; protocol/agent.md, section 4.

use skein_http as http;
use skein_io as io;
use skein_lib::Duration;
use skein_llm::{self as shared, openai};
use skein_llm_connection as connection;
use skein_tls as tls;
use smith_domain::{self as domain, run, session, tools};
use smith_protocol_channel as channel;
use smith_protocol_llm as llm;
use smith_protocol_machine as machine;

use crate as service;

mod quantities;

mod checks;

mod effective;

mod words;

#[cfg(test)]
mod tests;

pub mod validate;

pub mod room;

pub use checks::{Check, Refusal};
pub use effective::{Layer, Limit, LimitName, Per, Value};
pub use words::words;

pub use smith_protocol_llm::OversizedReasoning;

pub use quantities::{Configuration, Declared, Endpoint, Fraction, Model, Name, Policy, Profile, standard};

/// The number of configured destinations the standard profile admits.
pub const ENDPOINTS: u32 = 3;

/// The number of credential accounts the standard profile admits.
pub const ACCOUNTS: u32 = 4;

/// Compose layer limits from the selected profile and endpoint declarations.
pub fn derive(profile: &Profile, configuration: &Configuration) -> Result<service::Limits, Refusal> {
    validate::declarations(profile, configuration)?;
    let declared = profile.declared;
    let policy = profile.policy;
    let memory = declared.memory.ok_or(derived(Layer::Declared, "memory"))?;
    let client = client_limits();
    let decoded_call_bytes = 4096;
    let completion =
        llm::completion_worst_case(&client, decoded_call_bytes).ok_or(derived(Layer::Session, "completion_bytes"))?;
    let bodies = smith_channel::CEILINGS;
    let output_bytes = channel_output(&bodies)?;
    let io = io::Limits {
        sockets: 32,
        refusals: 4,
        intake: 19_000,
        receive: 4096,
        output: output_bytes.max(19_000),
        sends: 8,
        accepts: 1,
        backlog: 2,
        close_timeout: policy.close,
        retry: Duration::from_millis(10),
    };
    let mut domain = derive_domain(profile, configuration)?;
    domain.session.completion_bytes = completion;
    // Before skein's derivation removes its sent-text cap, rendering takes the
    // smaller cap and marks oversized outcomes rather than refusing the call.
    let rendered_result = llm::render_worst_case(
        &domain.session.tools,
        domain.run.host_reply_bytes.max(domain.run.message_bytes),
        domain.session.delegated_result_bytes,
    )
    .ok_or(derived(Layer::Render, "result"))?
    .min(client.dialect.string_bytes);
    let mut charter = smith_charter::CEILINGS;
    charter.charter_brief = policy.sections;
    charter.tools_host = policy.host_tools;
    charter.contract_verdicts = policy.verdicts;
    charter.run_result_items = policy.items;
    charter.run_result_fields = policy.fields;
    charter.item_fields = policy.fields;
    charter.text_rule_fields = policy.fields;
    charter.item_kind_fields = policy.fields;
    charter.verdict_rule_fields = policy.fields;
    charter.change_rule_fields = policy.fields;
    let queue = domain::max_out(&domain).max(256);
    let operations = io::operations(&io).ok_or(derived(Layer::Service, "routes"))?;
    let routes = operations.checked_add(64).ok_or(derived(Layer::Service, "routes"))?;
    let limits = service::Limits {
        domain,
        channel: channel::Limits {
            bodies,
            charter,
            transcript: smith_transcript::CEILINGS,
            channel: skein_channel::Limits {
                chunk: 4096,
                credential: 0,
                skip: 4096,
                output_bytes,
                output_frames: 4,
                kinds: 18,
            },
            endpoints: ENDPOINTS,
            calls: declared.calls_per_response,
            turns: policy.unacknowledged,
            fact_reserve_frames: 1,
            fact_reserve_bytes: 128,
            grants: 8,
        },
        llm: component_limits(profile, client, io, &domain, rendered_result)?,
        machine: derive_machine(profile, configuration, &domain)?,
        io,
        file_slots: 32,
        file_read: 65_536,
        file_entries: 64,
        file_bytes: 65_536,
        file_timeout: Duration::from_secs(30),
        queue,
        routes,
        memory,
    };
    validate::machine(&limits.machine, &limits.domain.session.tools)?;
    Ok(limits)
}

// Endpoint-derived request heads include both credential ceilings and fit one
// native TLS plaintext piece (skein's llm-connection.md, section 7).
fn tls_limits() -> tls::client::Limits {
    tls::client::Limits { read: 4096, send: tls::client::MAX_PLAINTEXT, records: tls::client::MAX_RECORD }
}

fn client_limits() -> shared::client::Limits {
    shared::client::Limits {
        http: http::client::Limits { request: 4096, head: 4096, headers: 32, read: 256, send: 31 },
        sse: http::sse::Limits { line: 4096, event: 8192, field: 128, chunk: 128 },
        dialect: openai::Limits {
            request_bytes: 8192,
            document_bytes: 8192,
            string_bytes: 4096,
            depth: 32,
            tokens: 1024,
            parts: 16,
            input_bytes: 2048,
            opaque_bytes: 2048,
            answer_bytes: 8192,
            detail_bytes: 256,
        },
        error_bytes: 4096,
        drop_reasoning: false,
        declared_output_tokens: 4096,
    }
}

fn channel_output(bodies: &smith_channel::Limits) -> Result<u32, Refusal> {
    let schema = smith_channel::schema(bodies).or(Err(derived(Layer::Channel, "output_bytes")))?;
    let version = schema.version(2).ok_or(derived(Layer::Channel, "version"))?;
    let mut largest = 0;
    for kind in &version.kinds {
        largest = largest.max(kind.largest);
    }
    largest.checked_add(8).ok_or(derived(Layer::Channel, "frame_bytes"))
}

fn derive_domain(profile: &Profile, configuration: &Configuration) -> Result<domain::Limits, Refusal> {
    let declared = profile.declared;
    let policy = profile.policy;
    let tools = match tools::derive(&tools::Derivation {
        conversations: declared.conversations,
        calls_per_response: declared.calls_per_response,
        tool_payload: declared.tool_payload,
        file_bytes: declared.file_bytes,
        read_window: declared.read_window,
        shell_head: declared.shell_head,
        shell_tail: declared.shell_tail,
        search_hits: declared.search_hits,
        search_bytes: declared.search_bytes,
        list_entries: declared.list_entries,
        tool_deadline: policy.tool_deadline,
        shell_timeout: policy.shell_timeout,
    }) {
        Ok(limits) => limits,
        Err(field) => return Err(derived(Layer::Tools, field.name())),
    };
    let own =
        room::own_part(&tools, client_limits().dialect.input_bytes, 0).ok_or(derived(Layer::Service, "own_part"))?;
    let parts = room::room(profile, configuration, own, 0)?;
    let schema_costs = llm::schema_costs().or(Err(derived(Layer::Run, "schema_costs")))?;
    let run = match run::derive(&run::Derivation {
        conversations: declared.conversations,
        calls_per_response: declared.calls_per_response,
        inbox: policy.inbox,
        max_waiting: policy.max_waiting,
        max_turns: policy.max_turns,
        max_spend: policy.max_spend,
        max_time: policy.max_time,
        sections: policy.sections,
        check_timeout: policy.check_timeout,
        wrap_up: policy.wrap_up,
        host_tools: policy.host_tools,
        verdicts: policy.verdicts,
        guide: declared.guide,
        shell_tail: declared.shell_tail,
        tool_payload: declared.tool_payload,
        tool_deadline: policy.tool_deadline,
        host: parts.host.min(u64::from(client_limits().dialect.string_bytes)),
        offer: parts.offer.min(u64::from(client_limits().dialect.string_bytes)),
        message: parts.message.min(client_limits().dialect.string_bytes),
        room: parts.room,
        compaction_threshold: [policy.compaction_threshold.numerator, policy.compaction_threshold.denominator],
        schema_costs,
    }) {
        Ok(limits) => limits,
        Err(field) => return Err(derived(Layer::Run, field.name())),
    };
    let session = match session::derive(&session::Derivation {
        conversations: declared.conversations,
        calls_per_response: declared.calls_per_response,
        max_turns: policy.max_turns,
        max_spend: policy.max_spend,
        max_time: policy.max_time,
        tool_deadline: policy.tool_deadline,
        tools,
    }) {
        Ok(limits) => limits,
        Err(field) => return Err(derived(Layer::Session, field.name())),
    };
    let mut configured_model_bytes = 0_u64;
    for endpoint in &configuration.endpoints {
        for model in &endpoint.models {
            let owned = domain::ConfiguredModel::worst_case(model.name.len())
                .ok_or(derived(Layer::Service, "configured_model_bytes"))?;
            configured_model_bytes =
                configured_model_bytes.checked_add(owned).ok_or(derived(Layer::Service, "configured_model_bytes"))?;
        }
    }
    Ok(domain::Limits {
        accounts: ACCOUNTS,
        endpoints: ENDPOINTS,
        configured_model_bytes,
        decoded_call_bytes: 4096,
        skew: Duration::ZERO,
        run,
        session,
    })
}

fn component_limits(
    profile: &Profile,
    client: shared::client::Limits,
    io: io::Limits,
    domain: &domain::Limits,
    rendered_result: u32,
) -> Result<llm::ComponentLimits, Refusal> {
    let mut stated = llm::Stated::from_tools(&domain.session.tools, client.dialect.input_bytes)
        .ok_or(derived(Layer::Llm, "output_ceiling"))?;
    stated.host_part = domain.run.host_part;
    Ok(llm::ComponentLimits {
        adapter: llm::Limits { client, tool_bytes: 32_768, rendered_result, stated },
        connection: connection::Limits {
            endpoints: ENDPOINTS,
            connections: profile.declared.conversations,
            calls: profile.declared.conversations,
            per_endpoint: profile.declared.conversations,
            idle_keep: profile.policy.connection_keep,
            io,
            tls: tls_limits(),
        },
        receiving: llm::Receiving {
            max_completion_bytes: domain.session.completion_bytes,
            max_completion_blocks: client.dialect.parts,
            decoded_call_bytes: domain.decoded_call_bytes,
            max_failure_bytes: domain.session.failure_bytes,
        },
        contract_bytes: 4096,
        accounts: ACCOUNTS,
        grant_value_bytes: 2048,
    })
}

fn derived(layer: Layer, field: &'static str) -> Refusal {
    Refusal::Derived { limit: LimitName::deployment(layer, field) }
}

fn derive_machine(
    profile: &Profile,
    configuration: &Configuration,
    domain: &domain::Limits,
) -> Result<machine::Limits, Refusal> {
    let policy = profile.policy;
    match machine::derive(&machine::Derivation {
        operations: domain.run.calls,
        roots: domain.session.tools.repos,
        path_bytes: domain.run.conflict_path_bytes.max(domain.session.tools.path_bytes),
        file_bytes: domain.session.tools.file_bytes,
        entries: domain.session.tools.list_entries,
        entry_bytes: domain.session.tools.list_bytes,
        shell_head: domain.session.tools.shell_head,
        shell_tail: domain.session.tools.shell_tail,
        search_hits: domain.session.tools.search_hits,
        search_bytes: domain.session.tools.search_bytes,
        environment_bytes: configuration.environment_bytes,
        group_stop: policy.group_stop,
    }) {
        Ok(limits) => Ok(limits),
        Err(field) => Err(derived(Layer::Machine, field.name())),
    }
}
