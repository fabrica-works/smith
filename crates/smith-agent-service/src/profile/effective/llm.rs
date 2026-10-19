//! Named projections of admitted layer limits, without retained state.
//! Contract: protocol/limits.md, section 9.

use super::list::Inventory;
use super::{Layer, Value};

pub(super) fn client(inventory: &mut Inventory, limits: &skein_llm::client::Limits) -> Option<()> {
    let skein_llm::client::Limits {
        http: _,
        sse: _,
        request,
        answer,
        input,
        reasoning,
        metadata,
        output_items,
        strings,
        tokens,
        retained,
        receiving,
        skip,
        tools,
        history_items,
        depth,
        detail_bytes,
        error_bytes,
        drop_reasoning: _,
        declared_output_tokens,
    } = *limits;
    inventory.layer = Layer::Llm;
    inventory.value("request", Value::Bytes(u64::from(request)))?;
    inventory.value("answer", Value::Bytes(u64::from(answer)))?;
    inventory.value("input", Value::Bytes(u64::from(input)))?;
    inventory.value("reasoning", Value::Bytes(u64::from(reasoning)))?;
    inventory.value("metadata", Value::Bytes(u64::from(metadata)))?;
    inventory.value("output_items", Value::Count(u64::from(output_items)))?;
    inventory.value("strings", Value::Bytes(u64::from(strings)))?;
    inventory.value("tokens", Value::Count(u64::from(tokens)))?;
    inventory.value("retained", Value::Bytes(u64::from(retained)))?;
    inventory.value("receiving", Value::Bytes(u64::from(receiving)))?;
    inventory.value("skip", Value::Bytes(u64::from(skip)))?;
    inventory.value("tools", Value::Count(u64::from(tools)))?;
    inventory.value("history_items", Value::Count(u64::from(history_items)))?;
    inventory.value("depth", Value::Count(u64::from(depth)))?;
    inventory.value("detail_bytes", Value::Bytes(u64::from(detail_bytes)))?;
    inventory.value("error_bytes", Value::Bytes(u64::from(error_bytes)))?;
    inventory.value("declared_output_tokens", Value::Tokens(u64::from(declared_output_tokens)))?;
    Some(())
}

pub(super) fn http(inventory: &mut Inventory, limits: &skein_http::client::Limits) -> Option<()> {
    let skein_http::client::Limits { request, head, headers, read, send } = *limits;
    inventory.layer = Layer::Llm;
    inventory.value("http.request", Value::Bytes(u64::from(request)))?;
    inventory.value("http.head", Value::Bytes(u64::from(head)))?;
    inventory.value("http.headers", Value::Count(u64::from(headers)))?;
    inventory.value("http.read", Value::Bytes(u64::from(read)))?;
    inventory.value("http.send", Value::Bytes(u64::from(send)))?;
    Some(())
}

pub(super) fn sse(inventory: &mut Inventory, limits: &skein_http::sse::Limits) -> Option<()> {
    let skein_http::sse::Limits { line, event, field, chunk } = *limits;
    inventory.layer = Layer::Llm;
    inventory.value("sse.line", Value::Bytes(u64::from(line)))?;
    inventory.value("sse.event", Value::Bytes(u64::from(event)))?;
    inventory.value("sse.field", Value::Bytes(u64::from(field)))?;
    inventory.value("sse.chunk", Value::Bytes(u64::from(chunk)))?;
    Some(())
}

pub(super) fn connection(inventory: &mut Inventory, limits: &skein_llm_connection::Limits) -> Option<()> {
    let skein_llm_connection::Limits { endpoints, connections, calls, per_endpoint, memory, idle_keep, io: _, tls: _ } =
        *limits;
    inventory.layer = Layer::Llm;
    inventory.value("connection.endpoints", Value::Count(u64::from(endpoints)))?;
    inventory.value("connection.connections", Value::Count(u64::from(connections)))?;
    inventory.value("connection.calls", Value::Count(u64::from(calls)))?;
    inventory.value("connection.per_endpoint", Value::Count(u64::from(per_endpoint)))?;
    inventory.value("connection.memory", Value::Bytes(memory))?;
    inventory.value("connection.idle_keep", Value::Duration(idle_keep))?;
    Some(())
}

pub(super) fn tls(inventory: &mut Inventory, limits: &skein_tls::client::Limits) -> Option<()> {
    let skein_tls::client::Limits { read, send, records } = *limits;
    inventory.layer = Layer::Llm;
    inventory.value("connection.tls.read", Value::Bytes(u64::from(read)))?;
    inventory.value("connection.tls.send", Value::Bytes(u64::from(send)))?;
    inventory.value("connection.tls.records", Value::Bytes(u64::from(records)))?;
    Some(())
}

pub(super) fn component(inventory: &mut Inventory, limits: &smith_protocol_llm::ComponentLimits) -> Option<()> {
    let smith_protocol_llm::ComponentLimits {
        adapter: _,
        connection: _,
        receiving: _,
        contract_bytes,
        llms,
        accounts,
        grant_value_bytes,
    } = *limits;
    inventory.layer = Layer::Llm;
    inventory.value("component.contract_bytes", Value::Bytes(contract_bytes))?;
    inventory.value("component.llms", Value::Count(u64::from(llms)))?;
    inventory.value("component.accounts", Value::Count(u64::from(accounts)))?;
    inventory.value("component.grant_value_bytes", Value::Bytes(u64::from(grant_value_bytes)))?;
    Some(())
}

pub(super) fn adapter(inventory: &mut Inventory, limits: &smith_protocol_llm::Limits) -> Option<()> {
    let smith_protocol_llm::Limits { client: _, tool_bytes, history_bytes, rendered_result: _, stated: _ } = *limits;
    inventory.layer = Layer::Llm;
    inventory.value("adapter.tool_bytes", Value::Bytes(tool_bytes))?;
    inventory.value("adapter.history_bytes", Value::Bytes(history_bytes))?;
    Some(())
}

pub(super) fn receiving(inventory: &mut Inventory, limits: &smith_protocol_llm::Receiving) -> Option<()> {
    let smith_protocol_llm::Receiving {
        max_completion_bytes,
        max_completion_blocks,
        decoded_call_bytes,
        max_failure_bytes,
    } = *limits;
    inventory.layer = Layer::Llm;
    inventory.value("receiving.max_completion_bytes", Value::Bytes(max_completion_bytes))?;
    inventory.value("receiving.max_completion_blocks", Value::Count(u64::from(max_completion_blocks)))?;
    inventory.value("receiving.decoded_call_bytes", Value::Bytes(decoded_call_bytes))?;
    inventory.value("receiving.max_failure_bytes", Value::Bytes(u64::from(max_failure_bytes)))?;
    Some(())
}
