//! Startup measures provider punctuation with empty application text.
//! Only a byte count leaves this module; no credential or destination is kept.
//! `prompt_allowance` prices the configured tool inventory and fixed routing
//! fields. Contract: protocol/limits.md, section3.2.

use alloc::boxed::Box;

use skein_lib::List;
use skein_llm::{self as shared, client};

use crate::Error;

/// Measure fixed provider framing, including the largest supported optional routing fields.
/// Actual endpoint option text and identity are counted separately in its own part.
pub fn prompt_allowance(provider: shared::Provider, client: &client::Limits, tools: u32) -> Result<u64, Error> {
    let endpoint = match provider {
        shared::Provider::OpenAi => shared::Endpoint::openai(),
        shared::Provider::OpenAiCodex => shared::Endpoint::codex(),
        shared::Provider::Anthropic => shared::Endpoint::anthropic(),
    };
    let reasoning_effort: Option<Box<[u8]>> = match provider {
        shared::Provider::OpenAi | shared::Provider::OpenAiCodex => Some(Box::new([0; 32])),
        shared::Provider::Anthropic => Some(Box::from(b"medium".as_slice())),
    };
    let mut prompt = shared::Prompt {
        model: Box::from(b"a".as_slice()),
        instructions: Box::new([]),
        tools: offered_tools(tools)?,
        choice: shared::ToolChoice::Auto,
        messages: Box::new([shared::Message {
            role: shared::Role::User,
            content: Box::new([shared::Block::Text { text: Box::new([]), replay: None }]),
        }]),
        reasoning_effort,
        affinity: Some(shared::Affinity { key: [0; 16], thread: u32::MAX }),
        max_output_tokens: None,
    };
    prompt.output_ceiling(provider, client.declared_output_tokens).or(Err(Error::Invalid))?;
    let account_id: Box<[u8]> = match provider {
        shared::Provider::OpenAiCodex => Box::from(b"a".as_slice()),
        shared::Provider::OpenAi | shared::Provider::Anthropic => Box::new([]),
    };
    let credential =
        shared::Credential { value: Box::from(b"a".as_slice()), account_id, scheme: skein_llm::Scheme::Bearer };
    let mut limits = *client;
    // Only these actual fixed records participate in this measurement.
    // Runtime history/tool maxima do not belong to its temporary wrappers.
    limits.tools = tools;
    limits.history_items = 2;
    limits.http.request = skein_tls::client::MAX_PLAINTEXT;
    limits.http.read = 4096;
    limits.http.send = skein_tls::client::MAX_PLAINTEXT;
    limits.sse.chunk = limits.http.read;
    let measured = match client::measure(&prompt, Some(&credential), &endpoint, &limits) {
        Ok(measured) => measured,
        Err(error) => return Err(Error::from_shared(error)),
    };
    Ok(u64::from(measured.body))
}

/// The same fixed empty schema and metadata for each offered tool.
pub(super) fn offered_tools(tools: u32) -> Result<Box<[shared::Tool]>, Error> {
    let schema = shared::Json::from_bytes(b"{}", &shared::DocumentLimits { bytes: 2, strings: 1, tokens: 2, depth: 2 })
        .or(Err(Error::Invalid))?;
    let mut offered = List::with_capacity(tools);
    for _ in 0..tools {
        offered
            .push(shared::Tool { name: Box::from(b"a".as_slice()), description: Box::new([]), schema: schema.clone() })
            .or(Err(Error::Limit))?;
    }
    Ok(offered.into_boxed())
}
