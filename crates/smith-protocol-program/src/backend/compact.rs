//! Tool-free summaries of portable windows (protocol/programs.md 5.8).
//! Rendering and argv reuse main's settled storage; no second child is live.

use super::{Backend, calls, start, stream};
use alloc::boxed::Box;
use skein_lib::List;
use smith_domain::llm;
use smith_protocol_llm as protocol;

pub(super) fn is_compaction(prompt: &llm::Prompt) -> bool {
    match prompt.purpose {
        llm::Purpose::Compaction { .. } => true,
        llm::Purpose::Ordinary { .. } => false,
    }
}

pub(super) fn arguments(prompt: &llm::Prompt, instructions: &[u8], session: &[u8]) -> Option<Box<[Box<[u8]>]>> {
    let ordinary = start::arguments(&prompt.model, prompt.effort.as_deref(), instructions, b"", b"", 1, session, b"")?;
    let mut arguments = List::with_capacity(36);
    let mut skip = false;
    for argument in ordinary {
        if skip {
            skip = false;
            continue;
        }
        if [b"--mcp-config".as_slice(), b"--permission-prompt-tool", b"--json-schema"].contains(&argument.as_ref()) {
            skip = true;
        } else {
            arguments.push(argument).ok()?;
        }
    }
    Some(arguments.into_boxed())
}

fn put(output: &mut List<u8>, bytes: &[u8]) -> Option<()> {
    for byte in bytes {
        output.push(*byte).ok()?;
    }
    Some(())
}

pub(super) fn render(prompt: &mut llm::Prompt, limits: &protocol::Limits, maximum: u32) -> Option<Box<[u8]>> {
    let messages = core::mem::replace(&mut prompt.messages, Box::new([]));
    let mut output = List::with_capacity(maximum);
    for message in messages {
        put(
            &mut output,
            match message.role {
                llm::Role::User => b"user:\n",
                llm::Role::Assistant => b"assistant:\n",
            },
        )?;
        for block in message.content {
            match block {
                llm::Block::Text { text, .. } | llm::Block::Refusal { text, .. } => put(&mut output, &text)?,
                llm::Block::ToolCall { name, input, .. } => {
                    put(&mut output, &name)?;
                    put(&mut output, b": ")?;
                    put(&mut output, &input)?;
                }
                llm::Block::ToolResult { id, result } => {
                    put(&mut output, b"result of ")?;
                    put(&mut output, &id)?;
                    put(&mut output, b": ")?;
                    match calls::render(result, prompt, limits)? {
                        crate::mcp::Answer::Text(text) | crate::mcp::Answer::Error(text) => put(&mut output, &text)?,
                    }
                }
                llm::Block::Opaque { .. } => {}
            }
            put(&mut output, b"\n")?;
        }
    }
    Some(output.into_boxed())
}

impl Backend {
    pub(super) fn compaction(&self) -> bool {
        match &self.request {
            Some(protocol::FromDomain::Complete { prompt, .. }) => is_compaction(prompt),
            Some(protocol::FromDomain::Cancel { .. }) | None => false,
        }
    }

    pub(super) fn compaction_input(&mut self) -> Option<()> {
        let (prompt, observed) = match self.request.as_mut()? {
            protocol::FromDomain::Complete { prompt, observation, .. } => (prompt, observation.is_some()),
            protocol::FromDomain::Cancel { .. } => return None,
        };
        let text = render(prompt, &self.settings.adapter, self.settings.input_bytes)?;
        self.outgoing = Some(stream::text_message(&text, self.settings.input_bytes)?);
        if observed {
            self.prompt_text_measure(skein_lib::Time::ZERO, text);
        }
        Some(())
    }
}

pub(super) fn checked_init(bytes: &[u8], depth: u32, tokens: u32) -> Option<()> {
    let document = stream::collect(bytes, depth, tokens)?;
    let json_type = crate::mcp::json::text_in(&document, crate::mcp::json::field(&document, 0, b"type"))?;
    let subtype = crate::mcp::json::text_in(&document, crate::mcp::json::field(&document, 0, b"subtype"))?;
    if json_type != b"system" || subtype != b"init" {
        return None;
    }
    for name in [b"tools".as_slice(), b"mcp_servers"] {
        let index = crate::mcp::json::field(&document, 0, name)?;
        if document.token(index)?.kind != skein_json::Kind::ArrayStart
            || document.token(index.checked_add(1)?)?.kind != skein_json::Kind::ArrayEnd
        {
            return None;
        }
    }
    Some(())
}
