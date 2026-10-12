//! Endpoint derivation keeps native limits and the shared history minima.
//! It knows only numeric declarations, never credentials or transport addresses.
//! `client` derives one endpoint; `path` composes their receiving maxima.
//! Contract: protocol/limits.md, sections 3.1 and 3.2.

use skein_llm::{self as shared, client};

use super::{Configuration, Endpoint, Layer, LimitName, Profile, Refusal};

/// Native maxima and the smallest history entrance shared by all endpoints.
#[derive(Clone, Copy, Debug)]
pub(super) struct Path {
    pub client: client::Limits,
    pub framing: u64,
}

/// Derive an endpoint from its largest actual model declarations, before transport pieces.
pub fn client(profile: &Profile, endpoint: &Endpoint, history_items: u32) -> Result<client::Limits, Refusal> {
    let mut declared = declarations(profile, endpoint)?;
    declared.history_items = history_items;
    let limits = match shared::Limits::derive(&declared) {
        Ok(limits) => limits,
        Err(violation) => return Err(Refusal::Llm { endpoint: endpoint.number, violation }),
    };
    Ok(limits.dialect(endpoint.provider))
}

pub(super) fn path(profile: &Profile, configuration: &Configuration) -> Result<Path, Refusal> {
    super::validate::configured_models(configuration)?;
    let mut largest = shared::Declared {
        window: 0,
        output: 0,
        reasoning_item: 0,
        tool_payload: call_payload(profile)?,
        calls_per_response: profile.declared.calls_per_response,
        conversations: profile.declared.conversations,
        tools: profile.policy.host_tools.checked_add(10).expect("validated tool count"),
        history_items: 2,
    };
    let mut framing = 0_u64;
    for endpoint in &configuration.endpoints {
        let native = client(profile, endpoint, 2)?;
        let tools = profile
            .policy
            .host_tools
            .checked_add(10)
            .ok_or(Refusal::Derived { limit: LimitName::deployment(Layer::Llm, "prompt_allowance") })?;
        let allowance = smith_protocol_llm::prompt_allowance(endpoint.provider, &native, tools)
            .or(Err(Refusal::Derived { limit: LimitName::deployment(Layer::Llm, "prompt_allowance") }))?;
        framing = framing.max(allowance);
        let declared = declarations(profile, endpoint)?;
        largest.window = largest.window.max(declared.window);
        largest.output = largest.output.max(declared.output);
        largest.reasoning_item = largest.reasoning_item.max(declared.reasoning_item);
    }
    let limits = shared::Limits::derive(&largest)
        .or(Err(Refusal::Derived { limit: LimitName::deployment(Layer::Llm, "receiving_maxima") }))?;
    let mut client = limits.codex;
    // Codex has the larger fixed response head, event records and metadata;
    // Anthropic's smaller tool spelling permits more represented tools.
    client.tools = client.tools.max(limits.anthropic.tools);
    Ok(Path { client, framing })
}

fn declarations(profile: &Profile, endpoint: &Endpoint) -> Result<shared::Declared, Refusal> {
    let mut declared = shared::Declared {
        window: 0,
        output: 0,
        reasoning_item: 0,
        tool_payload: call_payload(profile)?,
        calls_per_response: profile.declared.calls_per_response,
        conversations: profile.declared.conversations,
        tools: profile.policy.host_tools.checked_add(10).expect("validated tool count"),
        history_items: 2,
    };
    for model in &endpoint.models {
        declared.window = declared.window.max(model.window);
        declared.output = declared.output.max(model.output);
        declared.reasoning_item = declared.reasoning_item.max(model.reasoning_item);
    }
    Ok(declared)
}

// The native declaration bounds complete unescaped arguments, whereas the
// deployment declares a content piece. Rule 2 of protocol/limits.md, section 3:
// content and the longest path each expand by ESCAPE; fixed Write keys and
// punctuation need room too. Round their bytes upward into the native declaration
// so its existing checked escape factor covers the entire compact call.
fn call_payload(profile: &Profile) -> Result<u32, Refusal> {
    let refusal = Refusal::Derived { limit: LimitName::deployment(Layer::Llm, "input") };
    let path = super::derive_tools(profile)?.path_bytes;
    let framing = u32::try_from(br#"{"path":"","content":""}"#.len()).or(Err(refusal))?;
    let framing = framing.checked_add(shared::ESCAPE - 1).ok_or(refusal)?.checked_div(shared::ESCAPE).ok_or(refusal)?;
    profile.declared.tool_payload.checked_add(path).ok_or(refusal)?.checked_add(framing).ok_or(refusal)
}
