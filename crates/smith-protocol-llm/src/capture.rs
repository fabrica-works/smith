//! Content as translated for the actual provider request and received completion.
//! No replay envelope or credential enters an observation. The caller supplies
//! the bounded conversation cursor; `prompt` copies only the new part, and
//! `completion` preserves provider block order before policy decoding replaces
//! cut calls. Contracts: protocol/events.md, sections 3.5, 3.11 and 5.3.

use alloc::boxed::Box;

use skein_lib::{List, Time, Token};
use skein_llm as shared;
use smith_events::{Block, Completion, Message, Prompt, Role};

use crate::Error;

/// One request's content selection, supplied by its service before translation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Observation {
    /// The first request in the current window includes system and tool names.
    pub first: bool,
    /// Already recorded history messages, including completed assistant messages.
    pub messages: u32,
    /// The owner asks to receive text and refusal deltas as they arrive.
    pub live_text: bool,
}

/// A bounded protocol observation, drained by the service before its terminal.
#[derive(Debug)]
pub enum Measure {
    /// The actual translated request's new part, before the connection consumes it.
    Prompt { at: Time, owner: Token, prompt: Prompt },
    /// The provider's completion before application decoding replaces cut inputs.
    Completion { at: Time, owner: Token, completion: Completion },
    /// A text or refusal piece; the service joins its request's response identity.
    Delta { at: Time, owner: Token, block: u32, text: Box<[u8]> },
}

pub(crate) fn prompt(
    input: &shared::Prompt,
    observation: Observation,
    limits: &shared::client::Limits,
) -> Result<Prompt, Error> {
    let (system, tools) = if observation.first {
        let mut tools = List::with_capacity(u32::try_from(input.tools.len()).or(Err(Error::Limit))?);
        for tool in &input.tools {
            tools.push(tool.name.clone()).expect("one name per admitted tool");
        }
        (Some(input.instructions.clone()), Some(tools))
    } else {
        (None, None)
    };
    let since = usize::try_from(observation.messages).expect("u32 fits usize");
    let appended = input.messages.get(since..).ok_or(Error::Invalid)?;
    let mut messages = List::with_capacity(u32::try_from(appended.len()).or(Err(Error::Limit))?);
    for message in appended {
        let role = match message.role {
            shared::Role::User => Role::User,
            shared::Role::Assistant => Role::Assistant,
        };
        messages.push(Message { role, blocks: blocks(&message.content, limits)? }).expect("admitted new part");
    }
    Ok(Prompt { system, tools, messages })
}

pub(crate) fn completion(input: &shared::Completion, limits: &shared::client::Limits) -> Result<Completion, Error> {
    Ok(Completion { blocks: blocks(&input.content, limits)? })
}

fn blocks(input: &[shared::Block], limits: &shared::client::Limits) -> Result<List<Block>, Error> {
    let mut output = List::with_capacity(u32::try_from(input.len()).or(Err(Error::Limit))?);
    for block in input {
        let observed = match block {
            shared::Block::Text { text, replay: _ } => Block::Text { text: text.clone() },
            shared::Block::Refusal { text, replay: _ } => Block::Refusal { text: text.clone() },
            shared::Block::ToolCall { id, name, arguments, replay: _ } => {
                Block::Call { call: id.clone(), tool: name.clone(), input: arguments.clone() }
            }
            shared::Block::Oversize { id, name, bytes } => {
                Block::Oversized { call: id.clone(), tool: name.clone(), bytes: *bytes }
            }
            shared::Block::Cut { id, name, arguments } => {
                Block::Cut { call: id.clone(), tool: name.clone(), input: arguments.clone() }
            }
            shared::Block::ToolResult { id, text, is_error } => {
                Block::Result { call: id.clone(), error: *is_error, text: text.clone() }
            }
            shared::Block::Reasoning { replay } => {
                let encoded = match replay.to_bytes(&crate::limits::replay_limits(limits)) {
                    Ok(encoded) => encoded,
                    Err(error) => return Err(Error::from_shared(error)),
                };
                Block::Opaque { bytes: u64::try_from(encoded.len()).or(Err(Error::Limit))? }
            }
            shared::Block::Dropped { bytes } => Block::Opaque { bytes: *bytes },
        };
        output.push(observed).expect("one observation per admitted block");
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_macros,
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "programming-model.md section 10.2: deterministic translation harnesses use ordinary Rust"
    )]

    use super::*;

    #[test]
    fn typed_prompt_capture_preserves_text_and_tool_input_without_a_credential() {
        let input = shared::Prompt {
            model: b"model".to_vec().into(),
            instructions: b"system instructions".to_vec().into(),
            tools: Box::new([]),
            choice: shared::ToolChoice::Auto,
            messages: Box::new([shared::Message {
                role: shared::Role::User,
                content: Box::new([
                    shared::Block::Text { text: b"request".to_vec().into(), replay: None },
                    shared::Block::ToolCall {
                        id: b"id".to_vec().into(),
                        name: b"inspect".to_vec().into(),
                        arguments: b"input".to_vec().into(),
                        replay: None,
                    },
                ]),
            }]),
            reasoning_effort: None,
            affinity: None,
            max_output_tokens: Some(10),
        };
        let captured =
            prompt(&input, Observation { first: true, messages: 0, live_text: true }, &skein_llm_world::limits())
                .expect("bounded translated prompt");
        assert_eq!(captured.system.as_deref(), Some(b"system instructions".as_slice()));
        assert_eq!(captured.tools.expect("first request carries its names").len(), 0);
        assert_eq!(captured.messages.get(0).expect("actual task").role, Role::User);
        assert_eq!(
            captured.messages.get(0).expect("actual task").blocks.as_slice(),
            &[
                Block::Text { text: b"request".to_vec().into() },
                Block::Call {
                    call: b"id".to_vec().into(),
                    tool: b"inspect".to_vec().into(),
                    input: b"input".to_vec().into()
                },
            ]
        );
    }

    #[test]
    fn completed_content_preserves_cut_arguments_and_provider_block_order() {
        let input = shared::Completion {
            content: vec![
                shared::Block::Text { text: b"first".to_vec().into(), replay: None },
                shared::Block::Cut {
                    id: b"cut".to_vec().into(),
                    name: b"shell".to_vec().into(),
                    arguments: b"{\"command\":".to_vec().into(),
                },
                shared::Block::Oversize { id: b"big".to_vec().into(), name: b"write".to_vec().into(), bytes: 8193 },
                shared::Block::Dropped { bytes: 65537 },
                shared::Block::Refusal { text: b"last".to_vec().into(), replay: None },
            ]
            .into_boxed_slice(),
            stop: shared::Stop::MaxTokens,
            usage: shared::Usage { input: None, output: None, cache_read: None, cache_write: None, reasoning: None },
        };
        let observed = completion(&input, &skein_llm_world::limits()).unwrap();
        assert_eq!(
            observed.blocks.as_slice(),
            &[
                Block::Text { text: b"first".to_vec().into() },
                Block::Cut {
                    call: b"cut".to_vec().into(),
                    tool: b"shell".to_vec().into(),
                    input: b"{\"command\":".to_vec().into()
                },
                Block::Oversized { call: b"big".to_vec().into(), tool: b"write".to_vec().into(), bytes: 8193 },
                Block::Opaque { bytes: 65537 },
                Block::Refusal { text: b"last".to_vec().into() },
            ]
        );
    }

    #[test]
    fn request_new_parts_omit_previous_completions_and_retries_are_empty() {
        let mut input = shared::Prompt {
            model: b"model".to_vec().into(),
            instructions: b"actual system".to_vec().into(),
            tools: Box::new([]),
            choice: shared::ToolChoice::Auto,
            messages: vec![shared::Message {
                role: shared::Role::User,
                content: vec![shared::Block::Text { text: b"task".to_vec().into(), replay: None }].into(),
            }]
            .into(),
            reasoning_effort: None,
            affinity: None,
            max_output_tokens: None,
        };
        let limits = skein_llm_world::limits();
        let first = prompt(&input, Observation { first: true, messages: 0, live_text: true }, &limits).unwrap();
        assert_eq!(first.system.as_deref(), Some(b"actual system".as_slice()));
        assert_eq!(first.tools.unwrap().len(), 0);
        assert_eq!(first.messages.len(), 1);
        let mut messages = input.messages.to_vec();
        messages.push(shared::Message {
            role: shared::Role::Assistant,
            content: vec![shared::Block::Text { text: b"already in response.completed".to_vec().into(), replay: None }]
                .into(),
        });
        messages.push(shared::Message {
            role: shared::Role::User,
            content: vec![shared::Block::ToolResult {
                id: b"call".to_vec().into(),
                text: b"actual rendered result [cut]".to_vec().into(),
                is_error: true,
            }]
            .into(),
        });
        input.messages = messages.into();
        let next = prompt(&input, Observation { first: false, messages: 2, live_text: true }, &limits).unwrap();
        assert_eq!(next.system, None);
        assert_eq!(next.tools, None);
        assert_eq!(
            next.messages.as_slice(),
            &[Message {
                role: Role::User,
                blocks: {
                    let mut blocks = List::with_capacity(1);
                    blocks
                        .push(Block::Result {
                            call: b"call".to_vec().into(),
                            error: true,
                            text: b"actual rendered result [cut]".to_vec().into(),
                        })
                        .unwrap();
                    blocks
                }
            }]
        );
        let retry = prompt(&input, Observation { first: false, messages: 3, live_text: true }, &limits).unwrap();
        assert!(retry.messages.is_empty());
        assert_eq!(retry.system, None);
        assert_eq!(retry.tools, None);
        assert_eq!(
            prompt(&input, Observation { first: false, messages: 4, live_text: false }, &limits),
            Err(Error::Invalid)
        );
    }
}
