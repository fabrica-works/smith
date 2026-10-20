//! Counted effective inventory of the agent's own layers, with no retained state.
//! Contract: protocol/limits.md, sections 2 and 9.

use super::super::{Configuration, Profile};
use super::{Layer, Limit, LimitName, Per, Value};
use alloc::boxed::Box;
use skein_lib::List;

/// List every declared, policy and derived agent value under its owning name.
#[must_use]
pub fn effective(
    profile: &Profile,
    configuration: &Configuration,
    limits: &crate::Limits,
    endpoints: &smith_protocol_llm::Endpoints,
) -> Option<Box<[Limit]>> {
    let mut inventory = Inventory { count: 0, list: None, layer: Layer::Declared, per: Per::Deployment };
    project(&mut inventory, profile, configuration, limits, endpoints)?;
    inventory.list = Some(List::with_capacity(inventory.count));
    inventory.count = 0;
    project(&mut inventory, profile, configuration, limits, endpoints)?;
    Some(inventory.list?.into_boxed())
}

pub(super) struct Inventory {
    count: u32,
    list: Option<List<Limit>>,
    pub(super) layer: Layer,
    pub(super) per: Per,
}

impl Inventory {
    pub(super) fn value(&mut self, field: &'static str, value: Value) -> Option<()> {
        self.count = self.count.checked_add(1)?;
        match &mut self.list {
            Some(list) => list.push(Limit { name: LimitName { layer: self.layer, field, per: self.per }, value }).ok(),
            None => Some(()),
        }
    }
}

fn project(
    inventory: &mut Inventory,
    profile: &Profile,
    configuration: &Configuration,
    limits: &crate::Limits,
    endpoints: &smith_protocol_llm::Endpoints,
) -> Option<()> {
    declared(inventory, &profile.declared)?;
    policy(inventory, &profile.policy)?;
    models(inventory, profile, configuration)?;
    domain(inventory, &limits.domain)?;
    run(inventory, &limits.domain.run)?;
    session(inventory, &limits.domain.session)?;
    tools(inventory, &limits.domain.session.tools)?;
    inventory.layer = Layer::Render;
    inventory.value("result", Value::Bytes(u64::from(limits.llm.adapter.rendered_result)))?;
    service(inventory, limits)?;
    for endpoint in &configuration.endpoints {
        inventory.per = Per::Endpoint(endpoint.number);
        let client = &endpoints.resolve(smith_domain::llm::Endpoint(endpoint.number))?.client;
        super::llm::client(inventory, client)?;
        super::llm::http(inventory, &client.http)?;
        super::llm::sse(inventory, &client.sse)?;
    }
    inventory.per = Per::Deployment;
    super::llm::connection(inventory, &limits.llm.connection)?;
    super::llm::tls(inventory, &limits.llm.connection.tls)?;
    super::llm::component(inventory, &limits.llm)?;
    super::llm::adapter(inventory, &limits.llm.adapter)?;
    super::llm::receiving(inventory, &limits.llm.receiving)?;
    super::boundaries::machine(inventory, &limits.machine)?;
    super::channel::transcript(inventory, &limits.channel.transcript)?;
    super::channel::bodies(inventory, &limits.channel.bodies)?;
    super::channel::channel(inventory, &limits.channel)?;
    super::channel::framing(inventory, &limits.channel.channel)?;
    super::boundaries::io(inventory, &limits.io)?;
    if let Some(events) = &limits.events {
        super::boundaries::events(inventory, events)?;
        super::boundaries::sinks(inventory, &events.sinks)?;
        super::boundaries::records(inventory, &events.sinks.records)?;
    }
    Some(())
}

fn declared(inventory: &mut Inventory, limits: &super::super::Declared) -> Option<()> {
    let super::super::Declared {
        conversations,
        tool_payload,
        file_bytes,
        calls_per_response,
        jobs,
        read_window,
        shell_output,
        shell_head,
        shell_tail,
        search_hits,
        search_bytes,
        list_entries,
        guide,
        llm_pool,
        memory,
    } = *limits;
    inventory.layer = Layer::Declared;
    inventory.value("conversations", Value::Count(u64::from(conversations)))?;
    inventory.value("tool_payload", Value::Bytes(u64::from(tool_payload)))?;
    inventory.value("file_bytes", Value::Bytes(u64::from(file_bytes)))?;
    inventory.value("calls_per_response", Value::Count(u64::from(calls_per_response)))?;
    inventory.value("jobs", Value::Count(u64::from(jobs)))?;
    inventory.value("read_window", Value::Bytes(u64::from(read_window)))?;
    inventory.value("shell_output", Value::Bytes(u64::from(shell_output)))?;
    inventory.value("shell_head", Value::Bytes(u64::from(shell_head)))?;
    inventory.value("shell_tail", Value::Bytes(u64::from(shell_tail)))?;
    inventory.value("search_hits", Value::Count(u64::from(search_hits)))?;
    inventory.value("search_bytes", Value::Bytes(u64::from(search_bytes)))?;
    inventory.value("list_entries", Value::Count(u64::from(list_entries)))?;
    inventory.value("guide", Value::Bytes(u64::from(guide)))?;
    inventory.value("llm_pool", Value::Bytes(llm_pool))?;
    inventory.value("memory", Value::Bytes(memory))?;
    Some(())
}

fn policy(inventory: &mut Inventory, limits: &super::super::Policy) -> Option<()> {
    let super::super::Policy {
        max_turns,
        max_spend,
        max_time,
        max_waiting,
        sections,
        host_tools,
        verdicts,
        items,
        fields,
        inbox,
        unacknowledged,
        compaction_threshold,
        parallel_tools,
        connection_keep: _,
        tool_deadline,
        shell_timeout,
        job_poll,
        job_wait,
        check_timeout,
        wrap_up,
        group_stop,
        close,
        write_deadline,
        cancel_grace,
        exit_grace,
        refresh_lead,
        live_interval,
    } = *limits;
    inventory.layer = Layer::Policy;
    inventory.value("max_turns", Value::Count(u64::from(max_turns)))?;
    inventory.value("max_spend", Value::Spend(max_spend))?;
    inventory.value("max_time", Value::Duration(max_time))?;
    inventory.value("max_waiting", Value::Duration(max_waiting))?;
    inventory.value("sections", Value::Count(u64::from(sections)))?;
    inventory.value("host_tools", Value::Count(u64::from(host_tools)))?;
    inventory.value("verdicts", Value::Count(u64::from(verdicts)))?;
    inventory.value("items", Value::Count(u64::from(items)))?;
    inventory.value("fields", Value::Count(u64::from(fields)))?;
    inventory.value("inbox", Value::Count(u64::from(inbox)))?;
    inventory.value("unacknowledged", Value::Count(u64::from(unacknowledged)))?;
    inventory.value("compaction_threshold", Value::Fraction(compaction_threshold))?;
    inventory.value("parallel_tools", Value::Count(u64::from(parallel_tools)))?;
    inventory.value("tool_deadline", Value::Duration(tool_deadline))?;
    inventory.value("shell_timeout", Value::Duration(shell_timeout))?;
    inventory.value("job_poll", Value::Duration(job_poll))?;
    inventory.value("job_wait", Value::Duration(job_wait))?;
    inventory.value("check_timeout", Value::Duration(check_timeout))?;
    inventory.value("wrap_up", Value::Duration(wrap_up))?;
    inventory.value("group_stop", Value::Duration(group_stop))?;
    inventory.value("close", Value::Duration(close))?;
    inventory.value("write_deadline", Value::Duration(write_deadline))?;
    inventory.value("cancel_grace", Value::Duration(cancel_grace))?;
    inventory.value("exit_grace", Value::Duration(exit_grace))?;
    inventory.value("refresh_lead", Value::Duration(refresh_lead))?;
    inventory.value("live_interval", Value::Duration(live_interval))?;
    Some(())
}

fn domain(inventory: &mut Inventory, limits: &smith_domain::Limits) -> Option<()> {
    let smith_domain::Limits {
        accounts,
        endpoints,
        configured_model_bytes,
        decoded_call_bytes,
        skew,
        run: _,
        session: _,
    } = *limits;
    inventory.layer = Layer::Domain;
    inventory.value("accounts", Value::Count(u64::from(accounts)))?;
    inventory.value("endpoints", Value::Count(u64::from(endpoints)))?;
    inventory.value("configured_model_bytes", Value::Bytes(configured_model_bytes))?;
    inventory.value("decoded_call_bytes", Value::Bytes(decoded_call_bytes))?;
    inventory.value("skew", Value::Duration(skew))?;
    Some(())
}

fn run(inventory: &mut Inventory, limits: &smith_domain::run::Limits) -> Option<()> {
    let smith_domain::run::Limits {
        messages,
        message_bytes,
        offer_messages,
        offer_bytes,
        waiting,
        delivery_timeout,
        runs,
        conversations,
        run_bytes,
        brief_sections,
        host_part,
        room,
        compaction_threshold,
        compaction_material,
        schema_costs: _,
        directories,
        directory_name_bytes,
        host_path_bytes,
        conflicts,
        conflict_path_bytes,
        host_tools,
        host_input_bytes,
        host_reply_bytes,
        answered_calls,
        answered_bytes,
        host_timeout,
        host_timeout_max,
        host_backoff,
        verdicts,
        calls,
        budget: _,
        models,
        run_conversations,
        answer_bytes,
        nudges,
        guide_bytes,
        io_timeout,
        outcome_bytes,
        check_timeout,
        wrap_up,
        check_tail,
        facts,
    } = *limits;
    inventory.layer = Layer::Run;
    inventory.value("messages", Value::Count(u64::from(messages)))?;
    inventory.value("message_bytes", Value::Bytes(u64::from(message_bytes)))?;
    inventory.value("offer_messages", Value::Count(u64::from(offer_messages)))?;
    inventory.value("offer_bytes", Value::Bytes(u64::from(offer_bytes)))?;
    inventory.value("waiting", Value::Duration(waiting))?;
    inventory.value("delivery_timeout", Value::Duration(delivery_timeout))?;
    inventory.value("runs", Value::Count(u64::from(runs)))?;
    inventory.value("conversations", Value::Count(u64::from(conversations)))?;
    inventory.value("run_bytes", Value::Bytes(run_bytes))?;
    inventory.value("brief_sections", Value::Count(u64::from(brief_sections)))?;
    inventory.value("host_part", Value::Bytes(host_part))?;
    inventory.value("room", Value::Tokens(room))?;
    inventory.value("compaction_material", Value::Tokens(compaction_material))?;
    inventory.value(
        "compaction_threshold",
        Value::Fraction(super::super::Fraction {
            numerator: compaction_threshold[0],
            denominator: compaction_threshold[1],
        }),
    )?;
    inventory.value("directories", Value::Count(u64::from(directories)))?;
    inventory.value("directory_name_bytes", Value::Bytes(u64::from(directory_name_bytes)))?;
    inventory.value("host_path_bytes", Value::Bytes(u64::from(host_path_bytes)))?;
    inventory.value("conflicts", Value::Count(u64::from(conflicts)))?;
    inventory.value("conflict_path_bytes", Value::Bytes(u64::from(conflict_path_bytes)))?;
    inventory.value("host_tools", Value::Count(u64::from(host_tools)))?;
    inventory.value("host_input_bytes", Value::Bytes(u64::from(host_input_bytes)))?;
    inventory.value("host_reply_bytes", Value::Bytes(u64::from(host_reply_bytes)))?;
    inventory.value("answered_calls", Value::Count(u64::from(answered_calls)))?;
    inventory.value("answered_bytes", Value::Bytes(u64::from(answered_bytes)))?;
    inventory.value("host_timeout", Value::Duration(host_timeout))?;
    inventory.value("host_timeout_max", Value::Duration(host_timeout_max))?;
    inventory.value("host_backoff", Value::Duration(host_backoff))?;
    inventory.value("verdicts", Value::Count(u64::from(verdicts)))?;
    inventory.value("calls", Value::Count(u64::from(calls)))?;
    inventory.value("models", Value::Count(u64::from(models)))?;
    inventory.value("run_conversations", Value::Count(u64::from(run_conversations)))?;
    inventory.value("answer_bytes", Value::Bytes(u64::from(answer_bytes)))?;
    inventory.value("nudges", Value::Count(u64::from(nudges)))?;
    inventory.value("guide_bytes", Value::Bytes(u64::from(guide_bytes)))?;
    inventory.value("io_timeout", Value::Duration(io_timeout))?;
    inventory.value("outcome_bytes", Value::Bytes(outcome_bytes))?;
    inventory.value("check_timeout", Value::Duration(check_timeout))?;
    inventory.value("wrap_up", Value::Duration(wrap_up))?;
    inventory.value("check_tail", Value::Bytes(u64::from(check_tail)))?;
    inventory.value("facts", Value::Count(u64::from(facts)))?;
    let smith_domain::run::Budget { turns, spend, time } = limits.budget;
    inventory.value("budget.turns", Value::Count(u64::from(turns)))?;
    inventory.value("budget.spend", Value::Spend(spend))?;
    inventory.value("budget.time", Value::Duration(time))?;
    Some(())
}

fn session(inventory: &mut Inventory, limits: &smith_domain::session::Limits) -> Option<()> {
    let smith_domain::session::Limits {
        threshold: _,
        token_bytes: _,
        sessions,
        spend,
        protocol_allowance,
        history_messages,
        history_blocks,
        history_bytes,
        completion_bytes,
        answer_bytes,
        completion_blocks,
        failure_bytes,
        delegated_result_bytes,
        budget: _,
        parallel_tools,
        retries,
        backoff_base,
        backoff_max,
        facts,
        tools: _,
    } = *limits;
    inventory.layer = Layer::Session;
    inventory.value("sessions", Value::Count(u64::from(sessions)))?;
    inventory.value("spend", Value::Spend(spend))?;
    inventory.value("protocol_allowance", Value::Tokens(protocol_allowance))?;
    inventory.value("history_messages", Value::Count(u64::from(history_messages)))?;
    inventory.value("history_blocks", Value::Count(u64::from(history_blocks)))?;
    inventory.value("history_bytes", Value::Bytes(history_bytes))?;
    inventory.value("completion_bytes", Value::Bytes(completion_bytes))?;
    inventory.value("answer_bytes", Value::Bytes(answer_bytes))?;
    inventory.value("completion_blocks", Value::Count(u64::from(completion_blocks)))?;
    inventory.value("failure_bytes", Value::Bytes(u64::from(failure_bytes)))?;
    inventory.value("delegated_result_bytes", Value::Bytes(delegated_result_bytes))?;
    inventory.value("parallel_tools", Value::Count(u64::from(parallel_tools)))?;
    inventory.value("retries", Value::Count(u64::from(retries)))?;
    inventory.value("backoff_base", Value::Duration(backoff_base))?;
    inventory.value("backoff_max", Value::Duration(backoff_max))?;
    inventory.value("facts", Value::Count(u64::from(facts)))?;
    let smith_domain::session::Budget { turns, input, output, cache_read, cache_write, time } = limits.budget;
    inventory.value("budget.turns", Value::Count(u64::from(turns)))?;
    inventory.value("budget.input", Value::Tokens(input))?;
    inventory.value("budget.output", Value::Tokens(output))?;
    inventory.value("budget.cache_read", Value::Tokens(cache_read))?;
    inventory.value("budget.cache_write", Value::Tokens(cache_write))?;
    inventory.value("budget.time", Value::Duration(time))?;
    Some(())
}

fn tools(inventory: &mut Inventory, limits: &smith_domain::tools::Limits) -> Option<()> {
    let smith_domain::tools::Limits {
        kits,
        calls,
        jobs,
        repos,
        path_bytes,
        host_path_bytes,
        known_files,
        file_bytes,
        tool_payload,
        read_bytes,
        list_entries,
        list_bytes,
        match_lines,
        file_timeout,
        shell_timeout,
        job_poll,
        job_wait,
        tool_deadline,
        shell_output,
        shell_head,
        shell_tail,
        search_hits,
        search_bytes,
        search_timeout,
        facts,
    } = *limits;
    inventory.layer = Layer::Tools;
    inventory.value("kits", Value::Count(u64::from(kits)))?;
    inventory.value("calls", Value::Count(u64::from(calls)))?;
    inventory.value("jobs", Value::Count(u64::from(jobs)))?;
    inventory.value("repos", Value::Count(u64::from(repos)))?;
    inventory.value("path_bytes", Value::Bytes(u64::from(path_bytes)))?;
    inventory.value("host_path_bytes", Value::Bytes(u64::from(host_path_bytes)))?;
    inventory.value("known_files", Value::Count(u64::from(known_files)))?;
    inventory.value("file_bytes", Value::Bytes(u64::from(file_bytes)))?;
    inventory.value("tool_payload", Value::Bytes(u64::from(tool_payload)))?;
    inventory.value("read_bytes", Value::Bytes(u64::from(read_bytes)))?;
    inventory.value("list_entries", Value::Count(u64::from(list_entries)))?;
    inventory.value("list_bytes", Value::Bytes(list_bytes))?;
    inventory.value("match_lines", Value::Count(u64::from(match_lines)))?;
    inventory.value("file_timeout", Value::Duration(file_timeout))?;
    inventory.value("shell_timeout", Value::Duration(shell_timeout))?;
    inventory.value("job_poll", Value::Duration(job_poll))?;
    inventory.value("job_wait", Value::Duration(job_wait))?;
    inventory.value("tool_deadline", Value::Duration(tool_deadline))?;
    inventory.value("shell_output", Value::Bytes(u64::from(shell_output)))?;
    inventory.value("shell_head", Value::Bytes(u64::from(shell_head)))?;
    inventory.value("shell_tail", Value::Bytes(u64::from(shell_tail)))?;
    inventory.value("search_hits", Value::Count(u64::from(search_hits)))?;
    inventory.value("search_bytes", Value::Bytes(u64::from(search_bytes)))?;
    inventory.value("search_timeout", Value::Duration(search_timeout))?;
    inventory.value("facts", Value::Count(u64::from(facts)))?;
    Some(())
}

fn service(inventory: &mut Inventory, limits: &crate::Limits) -> Option<()> {
    let crate::Limits {
        domain: _,
        channel: _,
        llm: _,
        machine: _,
        io: _,
        file_slots,
        file_read,
        file_entries,
        file_bytes,
        file_timeout,
        queue,
        routes,
        memory,
        events: _,
        live_interval: _,
    } = *limits;
    inventory.layer = Layer::Service;
    inventory.value("teardown", Value::Duration(super::super::teardown_bound(limits)?))?;
    inventory.value("file_slots", Value::Count(u64::from(file_slots)))?;
    inventory.value("file_read", Value::Bytes(u64::from(file_read)))?;
    inventory.value("file_entries", Value::Count(u64::from(file_entries)))?;
    inventory.value("file_bytes", Value::Bytes(u64::from(file_bytes)))?;
    inventory.value("file_timeout", Value::Duration(file_timeout))?;
    inventory.value("queue", Value::Count(u64::from(queue)))?;
    inventory.value("routes", Value::Count(u64::from(routes)))?;
    inventory.value("memory", Value::Bytes(memory))?;
    Some(())
}

fn models(inventory: &mut Inventory, profile: &Profile, configuration: &Configuration) -> Option<()> {
    for endpoint in &configuration.endpoints {
        let super::super::Endpoint { number, provider: _, option_bytes: _, connect, handshake, models } = endpoint;
        inventory.per = Per::Endpoint(*number);
        inventory.layer = Layer::Policy;
        inventory.value("connect", Value::Duration(*connect))?;
        inventory.value("handshake", Value::Duration(*handshake))?;
        inventory.value("connection_keep", Value::Duration(profile.policy.connection_keep))?;
        inventory.layer = Layer::Declared;
        for (index, model) in models.iter().enumerate() {
            let super::super::Model {
                name: _,
                efforts: _,
                window,
                output,
                reasoning_item,
                oversized_reasoning: _,
                head,
                idle,
            } = model;
            inventory.per = Per::Model { endpoint: *number, model: u32::try_from(index).ok()? };
            inventory.value("window", Value::Tokens(u64::from(*window)))?;
            inventory.value("output", Value::Tokens(u64::from(*output)))?;
            inventory.value("reasoning_item", Value::Bytes(u64::from(*reasoning_item)))?;
            inventory.value("head", Value::Duration(*head))?;
            inventory.value("idle", Value::Duration(*idle))?;
        }
    }
    inventory.per = Per::Deployment;
    Some(())
}
