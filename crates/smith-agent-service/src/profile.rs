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

mod bounds;
mod quantities;

pub use bounds::{silence_bound, teardown_bound};

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
pub use effective::{Layer, Limit, LimitName, Per, Value, effective, header};
pub use words::words;

pub use smith_protocol_llm::OversizedReasoning;

pub use quantities::{Configuration, Declared, Endpoint, Fraction, Model, Name, Policy, Profile, standard};

/// The number of configured destinations the standard profile admits.
pub const ENDPOINTS: u32 = 3;

/// The count of distinct credential accounts the standard profile admits.
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
    let fact =
        u32::try_from(smith_channel::Fact::worst_case_bytes(&bodies).ok_or(derived(Layer::Channel, "output_bytes"))?)
            .or(Err(derived(Layer::Channel, "output_bytes")))?;
    let output_bytes = channel_output(&bodies)?
        .checked_add(fact.checked_add(8).ok_or(derived(Layer::Channel, "output_bytes"))?)
        .ok_or(derived(Layer::Channel, "output_bytes"))?;
    let machine = derive_machine(profile, configuration, &domain)?;
    let file_slots = machine.roots.checked_add(machine.operations).ok_or(derived(Layer::Service, "file_slots"))?;
    let io = io_limits(output_bytes, &policy, declared.conversations, &machine, file_slots)?;
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
    let transcript = transcript(&domain, &client)?;
    let queue = domain::max_out(&domain).max(256);
    let operations = io::operations(&io).ok_or(derived(Layer::Service, "routes"))?;
    let routes = operations.checked_add(64).ok_or(derived(Layer::Service, "routes"))?;
    let limits = service::Limits {
        domain,
        channel: channel_limits(&domain, &policy, bodies, charter, transcript, &io)?,
        llm: component_limits(profile, client, io, &domain, rendered_result)?,
        machine,
        io,
        file_slots,
        file_read: domain.session.tools.tool_payload.max(domain.session.tools.read_bytes),
        file_entries: domain.session.tools.list_entries,
        file_bytes: domain.session.tools.file_bytes,
        file_timeout: domain.session.tools.file_timeout,
        live_interval: policy.live_interval,
        queue,
        routes,
        memory,
        events: None,
    };
    validate::machine(&limits.machine, &limits.domain)?;
    validate::resume_holds_window(u64::from(domain.session.history_messages), u64::from(limits.channel.resume_turns))?;
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
fn transcript(
    domain: &domain::Limits,
    client: &shared::client::Limits,
) -> Result<smith_transcript::v6::Limits, Refusal> {
    let session = domain.session;
    let tools = session.tools;
    let run = domain.run;
    let result_blocks = run.calls.checked_add(1).ok_or(derived(Layer::Transcript, "message_blocks"))?;
    let message_blocks = client.output_items.max(result_blocks);
    validate::calls_in_transcript(message_blocks, smith_transcript::CEILINGS.message_blocks)?;
    let host = u32::try_from(run.host_part).or(Err(derived(Layer::Transcript, "text_text")))?;
    let mut limits = smith_transcript::CEILINGS;
    // A text or call's native metadata carries the durable replay header.
    limits.replay_bytes =
        client.metadata.checked_add(shared::REPLAY_HEADER_BYTES).ok_or(derived(Layer::Transcript, "replay_bytes"))?;
    limits.text_text = client.answer.max(host).max(run.offer_bytes).max(run.answer_bytes);
    // Consecutive reasoning items share one block, which can hold the
    // completion's whole reasoning share (protocol/limits.md, section 3.3).
    limits.opaque_bytes = client
        .output_items
        .checked_mul(client.reasoning)
        .ok_or(derived(Layer::Transcript, "opaque_bytes"))?
        .checked_add(shared::REPLAY_HEADER_BYTES)
        .ok_or(derived(Layer::Transcript, "opaque_bytes"))?;
    limits.call_input = client.input;
    limits.call_name_name = limits.call_name;
    limits.allowed_calls_names =
        run.host_tools.checked_add(4).ok_or(derived(Layer::Transcript, "allowed_calls_names"))?;
    limits.entry_name = tools.path_bytes;
    limits.hit_path = tools.path_bytes;
    limits.hit_text = tools.search_bytes;
    limits.read_content = tools.tool_payload.max(tools.read_bytes);
    limits.listed_entries = tools.list_entries;
    limits.found_hits = tools
        .search_hits
        .checked_mul(12)
        .ok_or(derived(Layer::Transcript, "found_hits"))?
        .checked_add(tools.search_bytes)
        .ok_or(derived(Layer::Transcript, "found_hits"))?;
    limits.command_end_head = tools.shell_head;
    limits.command_end_last = tools.shell_tail;
    limits.command_running_head = tools.shell_head;
    limits.command_running_last = tools.shell_tail;
    limits.job_read_head = tools.shell_head;
    limits.job_read_last = tools.shell_tail;
    limits.ambiguous_lines = tools.match_lines;
    limits.working_path_path = tools.path_bytes;
    limits.other_path_directory = run.directory_name_bytes;
    limits.other_path_path = tools.path_bytes;
    limits.outside_path_path = tools.path_bytes;
    limits.said_text = run
        .host_reply_bytes
        .max(run.message_bytes)
        .max(u32::try_from(session.delegated_result_bytes).or(Err(derived(Layer::Transcript, "said_text")))?);
    limits.message_blocks = message_blocks;
    limits.turn_messages = 4;
    let ceiling = smith_transcript::CEILINGS;
    for (field, required, available) in [
        ("replay_bytes", limits.replay_bytes, ceiling.replay_bytes),
        ("text_text", limits.text_text, ceiling.text_text),
        ("opaque_bytes", limits.opaque_bytes, ceiling.opaque_bytes),
        ("call_input", limits.call_input, ceiling.call_input),
        ("entry_name", limits.entry_name, ceiling.entry_name),
        ("hit_path", limits.hit_path, ceiling.hit_path),
        ("hit_text", limits.hit_text, ceiling.hit_text),
        ("found_hits", limits.found_hits, ceiling.found_hits),
        ("read_content", limits.read_content, ceiling.read_content),
        ("command_end_head", limits.command_end_head, ceiling.command_end_head),
        ("command_end_last", limits.command_end_last, ceiling.command_end_last),
        ("working_path_path", limits.working_path_path, ceiling.working_path_path),
        ("other_path_directory", limits.other_path_directory, ceiling.other_path_directory),
        ("other_path_path", limits.other_path_path, ceiling.other_path_path),
        ("outside_path_path", limits.outside_path_path, ceiling.outside_path_path),
        ("said_text", limits.said_text, ceiling.said_text),
        ("call_name_name", limits.call_name_name, ceiling.call_name_name),
    ] {
        validate::transcript_field(field, Value::Bytes(u64::from(required)), Value::Bytes(u64::from(available)))?;
    }
    for (field, required, available) in [
        ("listed_entries", limits.listed_entries, ceiling.listed_entries),
        ("allowed_calls_names", limits.allowed_calls_names, ceiling.allowed_calls_names),
        ("ambiguous_lines", limits.ambiguous_lines, ceiling.ambiguous_lines),
        ("turn_messages", limits.turn_messages, ceiling.turn_messages),
    ] {
        validate::transcript_field(field, Value::Count(u64::from(required)), Value::Count(u64::from(available)))?;
    }
    let bytes = smith_transcript::Turn::worst_case_bytes(&limits).ok_or(derived(Layer::Transcript, "turn_bytes"))?;
    let available =
        smith_transcript::Turn::worst_case_bytes(&ceiling).ok_or(derived(Layer::Transcript, "turn_bytes"))?;
    validate::turn_in_transcript(bytes, available)?;
    Ok(limits)
}

// Endpoint-derived request heads include both credential ceilings and fit one
// native TLS plaintext piece (skein's llm-connection.md, section 7).
fn tls_limits() -> tls::client::Limits {
    tls::client::Limits { read: 19_000, send: tls::client::MAX_PLAINTEXT, records: tls::client::MAX_RECORD }
}

fn channel_output(bodies: &smith_channel::Limits) -> Result<u32, Refusal> {
    let schema = smith_channel::schema(bodies).or(Err(derived(Layer::Channel, "output_bytes")))?;
    let version = schema.version(5).ok_or(derived(Layer::Channel, "version"))?;
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
    let refusal = derived(Layer::Channel, "turn_body");
    let session = domain.session;
    let results = tools::batch_result_worst_case(&session.tools, run.calls).ok_or(refusal)?;
    let delegated = session.delegated_result_bytes.checked_mul(u64::from(run.calls)).ok_or(refusal)?;
    // One completion, every result it can cause, and its first/window opening
    // share a turn. A window's earlier settled completions are not this turn.
    // Binary lengths fit their owned cells; payload has no escape multiplier.
    let payload = session::completion_reserve(&session)
        .ok_or(refusal)?
        .checked_add(results.max(delegated))
        .ok_or(refusal)?
        .checked_add(run.host_part)
        .ok_or(refusal)?
        .checked_add(u64::from(run.offer_bytes))
        .ok_or(refusal)?
        .checked_add(session.answer_bytes)
        .ok_or(refusal)?
        .checked_add(run.run_bytes)
        .ok_or(refusal)?
        .min(session.history_bytes);
    let turn = payload
        .checked_add(
            4_u64
                .checked_mul(u64::try_from(size_of::<session::llm::Message>()).expect("cell fits u64"))
                .ok_or(derived(Layer::Channel, "turn_body"))?,
        )
        .ok_or(derived(Layer::Channel, "turn_body"))?
        .checked_add(u64::try_from(size_of::<session::record::Turn>()).expect("cell fits u64"))
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
    let lengths =
        u64::from(domain.session.history_messages).checked_mul(4).ok_or(derived(Layer::Channel, "start_transcript"))?;
    let history = domain::max_turn_bytes(domain)
        .ok_or(derived(Layer::Channel, "start_transcript"))?
        .checked_mul(2)
        .ok_or(derived(Layer::Channel, "start_transcript"))?;
    let piece = history.checked_add(lengths).ok_or(derived(Layer::Channel, "start_transcript"))?;
    validate::turn_in_channel(piece, u64::from(bodies.start_transcript))?;
    bodies.start_transcript = u32::try_from(piece).or(Err(derived(Layer::Channel, "start_transcript")))?;
    validate::turn_in_channel(turn, u64::from(smith_channel::CEILINGS.turn_body))?;
    validate::turn_in_channel(charter, u64::from(smith_channel::CEILINGS.start_charter))?;
    validate::turn_in_channel(u64::from(run.message_bytes), u64::from(smith_channel::CEILINGS.message_text))?;
    bodies.accepted_result = u32::try_from(result.min(u64::from(bodies.accepted_result)))
        .or(Err(derived(Layer::Channel, "accepted_result")))?;
    Ok(bodies)
}

// Rule 6: saved turns share one whole window. Scalar and length fields
// fit twice their owning record cells; individual codec bounds stay sealed.
fn channel_start_bytes(bodies: &smith_channel::Limits) -> Result<u32, Refusal> {
    let bytes = smith_channel::Start::worst_case_bytes(bodies).ok_or(derived(Layer::Channel, "start_bytes"))?;
    u32::try_from(bytes).or(Err(derived(Layer::Channel, "start_bytes")))
}

// Rule 5: bounded producers share the byte queue; frame slots cover the
// unacknowledged turns, live calls, notices, long spans, answer and one fact.
fn channel_limits(
    domain: &domain::Limits,
    policy: &Policy,
    bodies: smith_channel::Limits,
    charter: smith_charter::v4::Limits,
    transcript: smith_transcript::v6::Limits,
    io: &io::Limits,
) -> Result<channel::Limits, Refusal> {
    let refusal = derived(Layer::Channel, "output_frames");
    let turns = policy.unacknowledged.min(domain.session.history_messages);
    let long = domain
        .run
        .conversations
        .checked_mul(domain.session.tools.calls.checked_add(1).ok_or(refusal)?)
        .ok_or(refusal)?
        .checked_add(1)
        .ok_or(refusal)?;
    let output_frames = turns
        .checked_add(domain.run.conversations)
        .ok_or(refusal)?
        .checked_add(domain.run.messages.min(bodies.start_messages))
        .ok_or(refusal)?
        .checked_add(long.checked_mul(2).ok_or(refusal)?)
        .ok_or(refusal)?
        .checked_add(3)
        .ok_or(refusal)?;
    let authority = channel_output(&bodies)?;
    let fact = u32::try_from(smith_channel::Fact::worst_case_bytes(&bodies).ok_or(refusal)?)
        .or(Err(refusal))?
        .checked_add(8)
        .ok_or(refusal)?;
    let live = u32::try_from(smith_channel::Live::worst_case_bytes(&bodies).ok_or(refusal)?)
        .or(Err(refusal))?
        .checked_add(8)
        .ok_or(refusal)?;
    let output_bytes = authority
        .checked_add(fact)
        .ok_or(derived(Layer::Channel, "output_bytes"))?
        .checked_add(live)
        .ok_or(derived(Layer::Channel, "output_bytes"))?;
    Ok(channel::Limits {
        bodies,
        start_bytes: channel_start_bytes(&bodies)?,
        charter,
        transcript,
        resume_turns: domain.session.history_messages,
        search_hits: domain.session.tools.search_hits,
        channel: skein_channel::Limits {
            chunk: io.receive,
            credential: 0,
            skip: channel_start_bytes(&bodies)?.max(output_bytes),
            output_bytes,
            output_frames,
            kinds: u32::try_from(smith_channel::V5.len()).or(Err(derived(Layer::Channel, "kinds")))?,
        },
        endpoints: ENDPOINTS,
        calls: domain.run.calls,
        turns,
        fact_reserve_frames: 1,
        fact_reserve_bytes: authority,
        grants: domain.accounts.checked_mul(2).ok_or(derived(Layer::Channel, "grants"))?,
    })
}

// Rule 6: one connection per conversation, the three process streams, file
// slots, and each machine child's process plus its two read-only pipes.
fn io_limits(
    output_bytes: u32,
    policy: &Policy,
    connections: u32,
    machine: &machine::Limits,
    file_slots: u32,
) -> Result<io::Limits, Refusal> {
    let refusal = derived(Layer::Io, "sockets");
    let sockets = connections
        .checked_add(3)
        .ok_or(refusal)?
        .checked_add(file_slots)
        .ok_or(refusal)?
        .checked_add(machine.processes.checked_mul(3).ok_or(refusal)?)
        .ok_or(refusal)?;
    Ok(io::Limits {
        sockets,
        names: 0,
        refusals: 4,
        intake: 19_000,
        receive: 4096,
        output: output_bytes.max(19_000),
        sends: 8,
        accepts: 1,
        backlog: 2,
        close_timeout: policy.close,
        retry: Duration::from_millis(10),
    })
}

fn line_bytes(configuration: &Configuration, material: u64) -> Result<u64, Refusal> {
    let refusal = derived(Layer::Session, "line");
    let mut largest = 0;
    for endpoint in &configuration.endpoints {
        for model in &endpoint.models {
            let bytes = u64::from(model.window)
                .checked_sub(u64::from(model.output))
                .ok_or(refusal)?
                .checked_sub(material)
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
        parallel_tools: policy.parallel_tools,
        jobs: declared.jobs,
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
        job_poll: policy.job_poll,
        job_wait: policy.job_wait,
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
    let parts = room::room(profile, configuration, own, run::summary_bytes())?;
    let schema_costs = llm::schema_costs().or(Err(derived(Layer::Run, "schema_costs")))?;
    let mut run = match run::derive(&run::Derivation {
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
        compaction_material: room::compaction_material(declared.calls_per_response, run::summary_bytes())
            .ok_or(derived(Layer::Run, "compaction_material"))?,
        schema_costs,
    }) {
        Ok(limits) => limits,
        Err(field) => return Err(derived(Layer::Run, field.name())),
    };
    validate::host_part_holds_the_role(profile, configuration, &run)?;
    // The entrance splits what the rendered charter leaves among the start's
    // own directories (protocol/limits.md, section 3.9); this is only the cap.
    run.guide_bytes =
        u32::try_from(parts.host.min(u64::from(declared.guide))).or(Err(derived(Layer::Run, "guide_bytes")))?;
    let delegated = domain::feedback_worst_case(&run).ok_or(derived(Layer::Session, "delegated_result_bytes"))?;
    let parallel = policy.parallel_tools.min(declared.calls_per_response);
    let owned = tools::batch_result_worst_case(&tools, parallel).ok_or(derived(Layer::Session, "batch_results"))?;
    let batch = owned.max(delegated.checked_mul(u64::from(parallel)).ok_or(derived(Layer::Session, "batch_results"))?);
    let line_bytes = line_bytes(configuration, run.compaction_material)?;
    let session = match session::derive(&session::Derivation {
        conversations: declared.conversations,
        calls_per_response: declared.calls_per_response,
        parallel_tools: policy.parallel_tools,
        max_turns: policy.max_turns,
        max_spend: policy.max_spend,
        max_time: policy.max_time,
        line_bytes,
        completion_bytes: completion,
        answer_bytes: u64::from(path.client.answer),
        batch_result_bytes: batch,
        delegated_result_bytes: delegated,
        output_items: path.client.output_items,
        detail: path.client.detail_bytes,
        protocol_allowance: parts.own.checked_add(parts.host).ok_or(derived(Layer::Session, "protocol_allowance"))?,
        threshold: [policy.compaction_threshold.numerator, policy.compaction_threshold.denominator],
        token_bytes: shared::TOKEN_BYTES,
        tools,
    }) {
        Ok(limits) => limits,
        Err(field) => return Err(derived(Layer::Session, field.name())),
    };
    let mut configured_model_bytes = 0_u64;
    let mut model_count = 0_u32;
    for endpoint in &configuration.endpoints {
        let count = u32::try_from(endpoint.models.len()).or(Err(derived(Layer::Run, "models")))?;
        model_count = model_count.checked_add(count).ok_or(derived(Layer::Run, "models"))?;
        for model in &endpoint.models {
            let owned = domain::ConfiguredModel::worst_case(model.name.len(), &model.efforts)
                .ok_or(derived(Layer::Service, "configured_model_bytes"))?;
            configured_model_bytes =
                configured_model_bytes.checked_add(owned).ok_or(derived(Layer::Service, "configured_model_bytes"))?;
        }
    }
    run.models = model_count.checked_sub(1).ok_or(derived(Layer::Run, "models"))?;
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
        llms: domain.run.models.checked_add(1).ok_or(derived(Layer::Llm, "llms"))?,
        accounts: ACCOUNTS,
        // Each grant value the channel admits installs: a signed-in plan's
        // access token is a JWT of some kilobytes.
        grant_value_bytes: smith_channel::CEILINGS.credential_value_bytes,
    })
}

fn derived(layer: Layer, field: &'static str) -> Refusal {
    Refusal::Derived { limit: LimitName::deployment(layer, field) }
}

// protocol/limits.md, section 3.5: each conversation's batch and jobs
// coexist with one check per directory.
fn machine_processes(domain: &domain::Limits) -> Option<u32> {
    let tools = domain.session.tools;
    tools.kits.checked_mul(tools.calls.checked_add(tools.jobs)?)?.checked_add(domain.run.directories)
}

fn derive_machine(
    profile: &Profile,
    configuration: &Configuration,
    domain: &domain::Limits,
) -> Result<machine::Limits, Refusal> {
    let policy = profile.policy;
    match machine::derive(&machine::Derivation {
        operations: domain.run.calls,
        processes: machine_processes(domain).ok_or(derived(Layer::Machine, "processes"))?,
        roots: domain.session.tools.repos,
        path_bytes: domain.run.conflict_path_bytes.max(domain.session.tools.path_bytes),
        file_bytes: domain.session.tools.file_bytes,
        tool_payload: domain.session.tools.tool_payload,
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
