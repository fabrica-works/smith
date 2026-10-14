//! Named projections of admitted layer limits, without retained state.
//! Contract: protocol/limits.md, section 9.

use super::list::Inventory;
use super::{Layer, Value};

pub(super) fn machine(inventory: &mut Inventory, limits: &smith_protocol_machine::Limits) -> Option<()> {
    let smith_protocol_machine::Limits {
        operations,
        roots,
        path_bytes,
        file_bytes,
        tool_payload,
        entries,
        entry_bytes,
        processes,
        output_bytes,
        search_hits,
        search_bytes,
        search_line_bytes,
        env_bytes,
        group_stop,
    } = *limits;
    inventory.layer = Layer::Machine;
    inventory.value("operations", Value::Count(u64::from(operations)))?;
    inventory.value("roots", Value::Count(u64::from(roots)))?;
    inventory.value("path_bytes", Value::Bytes(u64::from(path_bytes)))?;
    inventory.value("file_bytes", Value::Bytes(u64::from(file_bytes)))?;
    inventory.value("tool_payload", Value::Bytes(u64::from(tool_payload)))?;
    inventory.value("entries", Value::Count(u64::from(entries)))?;
    inventory.value("entry_bytes", Value::Bytes(entry_bytes))?;
    inventory.value("processes", Value::Count(u64::from(processes)))?;
    inventory.value("output_bytes", Value::Bytes(u64::from(output_bytes)))?;
    inventory.value("search_hits", Value::Count(u64::from(search_hits)))?;
    inventory.value("search_bytes", Value::Bytes(u64::from(search_bytes)))?;
    inventory.value("search_line_bytes", Value::Bytes(u64::from(search_line_bytes)))?;
    inventory.value("env_bytes", Value::Bytes(u64::from(env_bytes)))?;
    inventory.value("group_stop", Value::Duration(group_stop))?;
    Some(())
}

pub(super) fn io(inventory: &mut Inventory, limits: &skein_io::Limits) -> Option<()> {
    let skein_io::Limits {
        sockets,
        names: _,
        refusals,
        intake,
        receive,
        output,
        sends,
        accepts,
        backlog,
        close_timeout,
        retry,
    } = *limits;
    inventory.layer = Layer::Io;
    inventory.value("sockets", Value::Count(u64::from(sockets)))?;
    inventory.value("refusals", Value::Count(u64::from(refusals)))?;
    inventory.value("intake", Value::Bytes(u64::from(intake)))?;
    inventory.value("receive", Value::Bytes(u64::from(receive)))?;
    inventory.value("output", Value::Bytes(u64::from(output)))?;
    inventory.value("sends", Value::Count(u64::from(sends)))?;
    inventory.value("accepts", Value::Count(u64::from(accepts)))?;
    inventory.value("backlog", Value::Count(u64::from(backlog)))?;
    inventory.value("close_timeout", Value::Duration(close_timeout))?;
    inventory.value("retry", Value::Duration(retry))?;
    Some(())
}

pub(super) fn events(inventory: &mut Inventory, limits: &crate::EventLimits) -> Option<()> {
    let crate::EventLimits { sinks: _, capture: _, front: _, write_deadline } = *limits;
    inventory.layer = Layer::Events;
    inventory.value("write_deadline", Value::Duration(write_deadline))?;
    Some(())
}

pub(super) fn sinks(inventory: &mut Inventory, limits: &crate::sinks::SinkLimits) -> Option<()> {
    let crate::sinks::SinkLimits {
        destinations,
        output,
        records: _,
        answer_bytes,
        metadata_string,
        metadata_items,
        scalar_content,
        observation_line,
        fact_extra,
        conversations,
    } = *limits;
    inventory.layer = Layer::Events;
    inventory.value("destinations", Value::Count(u64::from(destinations)))?;
    inventory.value("output", Value::Bytes(u64::from(output)))?;
    inventory.value("answer_bytes", Value::Bytes(u64::from(answer_bytes)))?;
    inventory.value("metadata_string", Value::Bytes(u64::from(metadata_string)))?;
    inventory.value("metadata_items", Value::Count(u64::from(metadata_items)))?;
    inventory.value("scalar_content", Value::Bytes(u64::from(scalar_content)))?;
    inventory.value("observation_line", Value::Bytes(u64::from(observation_line)))?;
    inventory.value("fact_extra", Value::Bytes(u64::from(fact_extra)))?;
    inventory.value("conversations", Value::Count(u64::from(conversations)))?;
    Some(())
}

pub(super) fn records(inventory: &mut Inventory, limits: &smith_events::Limits) -> Option<()> {
    let smith_events::Limits { string, content, items } = *limits;
    inventory.layer = Layer::Events;
    inventory.value("records.string", Value::Bytes(u64::from(string)))?;
    inventory.value("records.content", Value::Bytes(u64::from(content)))?;
    inventory.value("records.items", Value::Count(u64::from(items)))?;
    Some(())
}
