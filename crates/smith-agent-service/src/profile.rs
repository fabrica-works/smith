//! The agent profile keeps declared deployment quantities and policy ceilings and
//! computes the service limits from them. It never knows endpoint destinations,
//! credentials or a run's charter. `derive` returns typed refusals.
//! Contract: shell.md, section 2.1; protocol/agent.md, section 4.

use skein_io as io;
use skein_lib::Duration;
use skein_llm as shared;
use skein_llm_connection as connection;
use skein_tls as tls;
use smith_domain::{self as domain, run, session, tools};
use smith_protocol_channel as channel;
use smith_protocol_llm as llm;
use smith_protocol_machine as machine;

use crate as service;

mod quantities;

mod path;

pub use path::client as endpoint_client;

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
    let memory = declared.memory;
    let path = path::path(profile, configuration)?;
    let mut client = path.client;
    // Receiving maxima cover both configured transports; each destination
    // takes its own pieces before entering the actual native connection pool.
    client.http.request = tls::client::MAX_PLAINTEXT;
    client.http.read = 19_000;
    client.http.send = tls::client::MAX_PLAINTEXT;
    client.sse.chunk = client.http.read;
    let decoded_call_bytes = u64::from(declared.calls_per_response)
        .checked_mul(u64::from(declared.tool_payload))
        .ok_or(derived(Layer::Session, "decoded_call_bytes"))?
        .min(u64::from(client.answer))
        .checked_add(
            u64::from(client.output_items)
                .checked_mul(u64::try_from(size_of::<domain::llm::Decoded>()).expect("cell fits u64"))
                .ok_or(derived(Layer::Session, "decoded_call_bytes"))?,
        )
        .ok_or(derived(Layer::Session, "decoded_call_bytes"))?;
    let completion =
        llm::completion_worst_case(&client, decoded_call_bytes).ok_or(derived(Layer::Session, "completion_bytes"))?;
    let domain = derive_domain(profile, configuration, &path, completion, decoded_call_bytes)?;
    client.history_items = domain
        .session
        .history_blocks
        .checked_add(policy.host_tools)
        .ok_or(derived(Layer::Llm, "history_items"))?
        .checked_add(11)
        .ok_or(derived(Layer::Llm, "history_items"))?;
    validate::window_first(profile, configuration, &domain.session)?;
    // The result bound holds whole capped outcomes; native outgoing text has
    // no receiving-string cap. Cutting remains a safeguard at this layer.
    let rendered_result = llm::render_worst_case(
        &domain.session.tools,
        domain.run.host_reply_bytes.max(domain.run.message_bytes),
        domain.session.delegated_result_bytes,
    )
    .ok_or(derived(Layer::Render, "result"))?;
    validate::render_in_result(rendered_result, rendered_result)?;
    let bodies = channel_bodies(&domain, &policy)?;
    let start_bytes = channel_start_bytes(&bodies, &domain)?;
    let output_bytes = channel_output(&bodies)?;
    let io = io_limits(output_bytes, &policy);
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
    let transcript = transcript(declared.calls_per_response, client.output_items)?;
    let queue = domain::max_out(&domain).max(256);
    let operations = io::operations(&io).ok_or(derived(Layer::Service, "routes"))?;
    let routes = operations.checked_add(64).ok_or(derived(Layer::Service, "routes"))?;
    let limits = service::Limits {
        domain,
        channel: channel::Limits {
            bodies,
            start_bytes,
            charter,
            transcript,
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
        file_bytes: domain.session.tools.file_bytes,
        file_timeout: Duration::from_secs(30),
        queue,
        routes,
        memory,
        events: None,
    };
    validate::machine(&limits.machine, &limits.domain.session.tools)?;
    Ok(limits)
}

/// Measure one endpoint's fixed schema, framing and configured option contribution.
/// The root adds the admitted contract's exact generated-schema bytes to each opening.
pub fn endpoint_allowance(profile: &Profile, endpoint: &Endpoint, tools: &tools::Limits) -> Result<u64, Refusal> {
    let client = endpoint_client(profile, endpoint, 2)?;
    let inventory = profile.policy.host_tools.checked_add(10).ok_or(derived(Layer::Llm, "prompt_allowance"))?;
    let framing = llm::prompt_allowance(endpoint.provider, &client, inventory)
        .or(Err(derived(Layer::Llm, "prompt_allowance")))?;
    let stated = llm::Stated::from_tools(tools, client.input).ok_or(derived(Layer::Llm, "fixed_schema_bytes"))?;
    llm::fixed_schema_bytes(&stated)
        .ok_or(derived(Layer::Llm, "fixed_schema_bytes"))?
        .checked_add(framing)
        .ok_or(derived(Layer::Llm, "prompt_allowance"))?
        .checked_add(endpoint.option_bytes)
        .ok_or(derived(Layer::Llm, "endpoint_own"))
}

// One turn surrounds one completion; a first final turn also includes its
// opening and final note before the assistant and settled results. A window
// turn instead has one opening message.
fn transcript(calls: u32, completion: u32) -> Result<smith_transcript::v3::Limits, Refusal> {
    let result_blocks = calls.checked_add(1).ok_or(derived(Layer::Transcript, "message_blocks"))?;
    let message_blocks = completion.max(result_blocks);
    validate::calls_in_transcript(message_blocks, smith_transcript::CEILINGS.message_blocks)?;
    let mut limits = smith_transcript::CEILINGS;
    limits.message_blocks = message_blocks;
    limits.turn_messages = 4;
    Ok(limits)
}

// Endpoint-derived request heads include both credential ceilings and fit one
// native TLS plaintext piece (skein's llm-connection.md, section 7).
fn tls_limits() -> tls::client::Limits {
    tls::client::Limits { read: 19_000, send: tls::client::MAX_PLAINTEXT, records: tls::client::MAX_RECORD }
}

fn channel_output(bodies: &smith_channel::Limits) -> Result<u32, Refusal> {
    let schema = smith_channel::schema(bodies).or(Err(derived(Layer::Channel, "output_bytes")))?;
    let version = schema.version(2).ok_or(derived(Layer::Channel, "version"))?;
    let mut largest = 0;
    for kind in &version.kinds {
        match kind.direction {
            skein_channel::Direction::FromResponder => largest = largest.max(kind.largest),
            skein_channel::Direction::FromInitiator => {}
        }
    }
    largest.checked_add(8).ok_or(derived(Layer::Channel, "frame_bytes"))
}

// Rule 6: active field bounds follow their source domain's admission. Sealed
// vocabulary maxima remain upper bounds, never an independent reservation.
fn channel_bodies(domain: &domain::Limits, policy: &Policy) -> Result<smith_channel::Limits, Refusal> {
    let run = domain.run;
    let turn = domain::max_turn_bytes(domain)
        .ok_or(derived(Layer::Channel, "turn_body"))?
        .checked_mul(2)
        .ok_or(derived(Layer::Channel, "turn_body"))?;
    let charter = run.run_bytes.checked_mul(2).ok_or(derived(Layer::Channel, "start_charter"))?;
    let result = run.outcome_bytes.checked_mul(2).ok_or(derived(Layer::Channel, "accepted_result"))?;
    let mut bodies = smith_channel::CEILINGS;
    bodies.directory_name = run.directory_name_bytes;
    bodies.directory_path = run.host_path_bytes;
    bodies.directory_conflicts = run.conflicts;
    bodies.directory_conflicts_item = run.conflict_path_bytes;
    bodies.workspace_directories = run.directories;
    bodies.host_reply_text = run.host_reply_bytes;
    bodies.message_text = run.message_bytes;
    bodies.start_charter = u32::try_from(charter.min(u64::from(bodies.start_charter)))
        .or(Err(derived(Layer::Channel, "start_charter")))?;
    bodies.start_answered = run.answered_calls;
    bodies.start_grants = ACCOUNTS.checked_mul(2).ok_or(derived(Layer::Channel, "start_grants"))?;
    bodies.start_messages = run.messages.min(bodies.start_messages);
    bodies.host_ask_input = run.host_input_bytes;
    bodies.field_text = u32::try_from(run.outcome_bytes.min(u64::from(bodies.field_text)))
        .or(Err(derived(Layer::Channel, "field_text")))?;
    bodies.deliver_ask_fields = policy.fields;
    bodies.turn_body =
        u32::try_from(turn.min(u64::from(bodies.turn_body))).or(Err(derived(Layer::Channel, "turn_body")))?;
    bodies.start_transcript_item = bodies.turn_body;
    bodies.accepted_result = u32::try_from(result.min(u64::from(bodies.accepted_result)))
        .or(Err(derived(Layer::Channel, "accepted_result")))?;
    Ok(bodies)
}

// Rule 6: saved turns share one whole window. Scalar and length fields
// fit twice their owning record cells; individual codec bounds stay sealed.
fn channel_start_bytes(bodies: &smith_channel::Limits, domain: &domain::Limits) -> Result<u32, Refusal> {
    let refusal = derived(Layer::Channel, "start_bytes");
    let mut empty_transcript = *bodies;
    empty_transcript.start_transcript = 0;
    let lengths = u64::from(bodies.start_transcript).checked_mul(4).ok_or(refusal)?;
    let history = domain::max_turn_bytes(domain).ok_or(refusal)?.checked_mul(2).ok_or(refusal)?;
    let bytes = smith_channel::Start::worst_case_bytes(&empty_transcript)
        .ok_or(refusal)?
        .checked_add(lengths)
        .ok_or(refusal)?
        .checked_add(history)
        .ok_or(refusal)?;
    u32::try_from(bytes).or(Err(refusal))
}

fn io_limits(output_bytes: u32, policy: &Policy) -> io::Limits {
    io::Limits {
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
    }
}

fn threshold_bytes(policy: &Policy, configuration: &Configuration) -> Result<u64, Refusal> {
    let refusal = derived(Layer::Session, "window_threshold");
    let mut largest = 0;
    for endpoint in &configuration.endpoints {
        for model in &endpoint.models {
            let bytes = u64::from(model.window)
                .checked_mul(u64::from(policy.compaction_threshold.numerator))
                .ok_or(refusal)?
                .checked_div(u64::from(policy.compaction_threshold.denominator))
                .ok_or(refusal)?
                .checked_mul(u64::from(shared::TOKEN_BYTES))
                .ok_or(refusal)?;
            largest = largest.max(bytes);
        }
    }
    Ok(largest)
}

fn derive_tools(profile: &Profile) -> Result<tools::Limits, Refusal> {
    let declared = profile.declared;
    let policy = profile.policy;
    match tools::derive(&tools::Derivation {
        conversations: declared.conversations,
        calls_per_response: declared.calls_per_response,
        tool_payload: declared.tool_payload,
        file_bytes: declared.file_bytes,
        read_window: declared.read_window,
        shell_output: declared.shell_output,
        shell_head: declared.shell_head,
        shell_tail: declared.shell_tail,
        search_hits: declared.search_hits,
        search_bytes: declared.search_bytes,
        list_entries: declared.list_entries,
        tool_deadline: policy.tool_deadline,
        shell_timeout: policy.shell_timeout,
    }) {
        Ok(limits) => Ok(limits),
        Err(field) => Err(derived(Layer::Tools, field.name())),
    }
}

fn derive_domain(
    profile: &Profile,
    configuration: &Configuration,
    path: &path::Path,
    completion: u64,
    decoded_call_bytes: u64,
) -> Result<domain::Limits, Refusal> {
    let declared = profile.declared;
    let policy = profile.policy;
    let tools = derive_tools(profile)?;
    let own = room::own_part(&tools, path.client.input, path.framing).ok_or(derived(Layer::Service, "own_part"))?;
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
        host: parts.host,
        offer: parts.offer,
        message: parts.message,
        room: parts.room,
        compaction_threshold: [policy.compaction_threshold.numerator, policy.compaction_threshold.denominator],
        schema_costs,
    }) {
        Ok(limits) => limits,
        Err(field) => return Err(derived(Layer::Run, field.name())),
    };
    let delegated = domain::feedback_worst_case(&run).ok_or(derived(Layer::Session, "delegated_result_bytes"))?;
    let parallel = declared.calls_per_response.min(4);
    let owned = tools::batch_result_worst_case(&tools, parallel).ok_or(derived(Layer::Session, "batch_results"))?;
    let batch = owned.max(delegated.checked_mul(u64::from(parallel)).ok_or(derived(Layer::Session, "batch_results"))?);
    let threshold_bytes = threshold_bytes(&policy, configuration)?;
    let session = match session::derive(&session::Derivation {
        conversations: declared.conversations,
        calls_per_response: declared.calls_per_response,
        max_turns: policy.max_turns,
        max_spend: policy.max_spend,
        max_time: policy.max_time,
        threshold_bytes,
        completion_bytes: completion,
        answer_bytes: u64::from(path.client.answer),
        batch_result_bytes: batch,
        delegated_result_bytes: delegated,
        output_items: path.client.output_items,
        detail: path.client.detail_bytes,
        protocol_allowance: parts.own.checked_add(parts.host).ok_or(derived(Layer::Session, "protocol_allowance"))?,
        threshold: [policy.compaction_threshold.numerator, policy.compaction_threshold.denominator],
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
        decoded_call_bytes,
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
    let mut stated =
        llm::Stated::from_tools(&domain.session.tools, client.input).ok_or(derived(Layer::Llm, "output_ceiling"))?;
    stated.host_part = domain.run.host_part;
    Ok(llm::ComponentLimits {
        adapter: llm::Limits {
            client,
            history_bytes: domain.session.history_bytes,
            tool_bytes: domain
                .session
                .protocol_allowance
                .checked_add(
                    u64::from(profile.policy.host_tools.checked_add(10).ok_or(derived(Layer::Llm, "tool_bytes"))?)
                        .checked_mul(u64::try_from(size_of::<domain::llm::Served>()).expect("cell fits u64"))
                        .ok_or(derived(Layer::Llm, "tool_bytes"))?,
                )
                .ok_or(derived(Layer::Llm, "tool_bytes"))?,
            rendered_result,
            stated,
        },
        connection: connection::Limits {
            endpoints: ENDPOINTS,
            connections: profile.declared.conversations,
            calls: profile.declared.conversations,
            per_endpoint: profile.declared.conversations,
            idle_keep: profile.policy.connection_keep,
            memory: profile.declared.llm_pool,
            io,
            tls: tls_limits(),
        },
        receiving: llm::Receiving {
            max_completion_bytes: domain.session.completion_bytes,
            max_completion_blocks: client.output_items,
            decoded_call_bytes: domain.decoded_call_bytes,
            max_failure_bytes: domain.session.failure_bytes,
        },
        // Outcome and its separately retained delivery contract both come
        // from the run's admitted aggregate declaration ownership.
        contract_bytes: domain.run.run_bytes.checked_mul(2).ok_or(derived(Layer::Llm, "contract_bytes"))?,
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
