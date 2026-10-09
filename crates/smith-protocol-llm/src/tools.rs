//! Schemas and descriptions for Smith's tools.
//! This module retains no state. The prompt translation offers these declarations
//! under the domain's grants; host declarations pass through unchanged.
//! Contract: protocol/llm.md, section 3; domain/tools.md, section 3.

use alloc::boxed::Box;

use skein_lib::List;
use smith_domain::run;
use smith_domain::{llm, tools};

use crate::{Error, ToolKind, ToolSchema};

/// The bounded inventory of Smith tools offered for this prompt.
/// Host tools are supplied separately by the charter and retain their bytes.
#[must_use]
pub fn schemas(prompt: &llm::Prompt, stated: &crate::Stated) -> Box<[ToolSchema]> {
    let mut descriptors = List::with_capacity(10);
    if prompt.tools.inspect {
        add(&mut descriptors, owned(tools::Tool::Read, b"read", stated, &prompt.namespace));
        add(&mut descriptors, owned(tools::Tool::List, b"list", stated, &prompt.namespace));
        add(&mut descriptors, owned(tools::Tool::Search, b"search", stated, &prompt.namespace));
    }
    if prompt.tools.modify {
        add(&mut descriptors, owned(tools::Tool::Write, b"write", stated, &prompt.namespace));
        add(&mut descriptors, owned(tools::Tool::Edit, b"edit", stated, &prompt.namespace));
    }
    if prompt.tools.shell {
        add(&mut descriptors, owned(tools::Tool::Shell, b"shell", stated, &prompt.namespace));
    }
    for offered in &prompt.served {
        match offered {
            llm::Served::Host(_) => {}
            llm::Served::Wait => add(&mut descriptors, waiting()),
            llm::Served::SubAgent => add(&mut descriptors, sub_agent()),
            llm::Served::Deliver => add(&mut descriptors, descriptor(
                ToolKind::Deliver,
                b"deliver",
                b"Offer the checked change to the host under the change contract.",
                br#"{"type":"object","properties":{"ticket":{"type":"string"}},"required":["ticket"]}"#,
            )),
            llm::Served::Finish => add(&mut descriptors, descriptor(
                ToolKind::Finish,
                b"finish",
                b"Declare the final result under the host's result contract.",
                br#"{"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"},"report":{"type":"string"},"source":{"type":"string"},"failure":{"type":"string"},"cause":{"type":"string"},"verdict":{"type":"string"},"children":{"type":"array","items":{"type":"object","properties":{"kind":{"type":"string"},"path":{"type":"string"}},"required":["kind","path"]}}},"anyOf":[{"required":["title","body"]},{"required":["report"]},{"required":["failure"]},{"required":["verdict","body"]}]}"#,
            )),
        }
    }
    descriptors.into_boxed()
}

/// The offered inventory with finish and deliver derived from the charter's
/// admitted rules. The caller supplies a separate deliver rule only when that
/// mid-run tool was granted.
pub fn schemas_for_contract(
    prompt: &llm::Prompt,
    stated: &crate::Stated,
    outcome: &run::outcome::OutcomeSpec,
    deliver: Option<&run::outcome::ChangeSpec>,
    maximum: u32,
) -> Result<Box<[ToolSchema]>, Error> {
    let mut inventory = List::with_capacity(10);
    for mut descriptor in schemas(prompt, stated) {
        match descriptor.kind {
            ToolKind::Finish => descriptor.schema = crate::contract::finish_schema(outcome, maximum)?,
            ToolKind::Deliver => {
                let change = deliver.ok_or(Error::Invalid)?;
                descriptor.schema = crate::contract::deliver_schema(change, maximum)?;
            }
            ToolKind::Owned(_) | ToolKind::SubAgent | ToolKind::Wait => {}
        }
        inventory.push(descriptor).or(Err(Error::Limit))?;
    }
    Ok(inventory.into_boxed())
}

/// Measure Smith's fixed workspace schemas and fixed wait/sub-agent declarations.
/// Contract-generated finish and deliver declarations belong to the host's part.
#[must_use]
pub fn fixed_schema_bytes(stated: &crate::Stated) -> Option<u64> {
    let mut bytes = 0_u64;
    for schema in [
        owned(tools::Tool::Read, b"read", stated, &tools::Namespace::One),
        owned(tools::Tool::List, b"list", stated, &tools::Namespace::One),
        owned(tools::Tool::Search, b"search", stated, &tools::Namespace::One),
        owned(tools::Tool::Write, b"write", stated, &tools::Namespace::One),
        owned(tools::Tool::Edit, b"edit", stated, &tools::Namespace::One),
        owned(tools::Tool::Shell, b"shell", stated, &tools::Namespace::One),
        waiting(),
        sub_agent(),
    ] {
        for field in [&schema.name, &schema.description, &schema.schema] {
            bytes = bytes.checked_add(u64::try_from(field.len()).ok()?)?;
        }
    }
    Some(bytes)
}

fn waiting() -> ToolSchema {
    descriptor(
        ToolKind::Wait,
        b"wait",
        b"Wait for the next message or outstanding result.",
        br#"{"type":"object","properties":{}}"#,
    )
}

fn sub_agent() -> ToolSchema {
    descriptor(ToolKind::SubAgent, b"sub_agent",
        b"Ask a sub-agent to work under the granted tool families and optional model. Optional max_turns and max_seconds ask for a share, clamped to what is left above the finishing reserve. The child is told its effective share.",
        br#"{"type":"object","properties":{"brief":{"type":"string"},"tools":{"type":"array","items":{"type":"string","enum":["inspect","modify","shell"]}},"llm":{"type":"string"},"max_turns":{"type":"integer","minimum":1},"max_seconds":{"type":"integer","minimum":1}},"required":["brief","tools"]}"#)
}

fn add(descriptors: &mut List<ToolSchema>, schema: ToolSchema) {
    descriptors.push(schema).expect("ten fixed tool slots");
}

fn descriptor(kind: ToolKind, name: &[u8], description: &[u8], schema: &[u8]) -> ToolSchema {
    ToolSchema { kind, name: name.into(), description: description.into(), schema: schema.into() }
}

fn owned(tool: tools::Tool, name: &[u8], stated: &crate::Stated, namespace: &tools::Namespace) -> ToolSchema {
    ToolSchema {
        kind: ToolKind::Owned(tool),
        name: name.into(),
        description: crate::descriptions::description(tool, stated),
        schema: crate::descriptions::schema(tool, stated, namespace),
    }
}
