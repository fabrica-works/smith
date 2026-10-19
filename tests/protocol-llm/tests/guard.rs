//! The actual component bounds encoded additions before its byte peer sees them.
//! Contract: protocol/llm.md, sections 6 and 10.

use skein_llm::Provider;
use smith_domain::{llm, tools};
use smith_protocol_llm::ToDomain;
use smith_protocol_llm_world::pool_peer::Peer;

const LINE: u64 = 1_000;
const CONTROL_BYTES: usize = 16;
// Two message envelopes and separators, plus "answer" and "go on".
const CODEX_MEASURE: u64 = 171;
const ANTHROPIC_MEASURE: u64 = 124;

fn prompt(guard: Option<llm::Guard>, text: &[u8]) -> llm::Prompt {
    llm::Prompt {
        effort: None,
        purpose: llm::Purpose::Ordinary { guard },
        choice: llm::ToolChoice::Auto,
        namespace: tools::Namespace::One,
        affinity: llm::Affinity { key: [0; 16], thread: 0 },
        asker: llm::Asker::Main,
        endpoint: llm::Endpoint(0),
        model: b"fixture".as_slice().into(),
        system: b"guard world".as_slice().into(),
        tools: tools::Grants { inspect: false, modify: false, shell: false },
        served: Box::new([]),
        messages: Box::new([
            message(llm::Role::User, b"begin"),
            message(llm::Role::Assistant, text),
            message(llm::Role::User, b"go on"),
        ]),
        max_tokens: 128,
    }
}

fn message(role: llm::Role, text: &[u8]) -> llm::Message {
    llm::Message { role, content: Box::new([llm::Block::Text { text: text.into(), replay: None }]) }
}

fn attempt(provider: Provider, guard: Option<llm::Guard>, text: &[u8]) -> Peer {
    let mut peer = Peer::for_provider(1, 1, provider);
    peer.start_prompt(7, 2, 3, 10, prompt(guard, text), None);
    peer
}

fn measure(provider: Provider) -> u64 {
    match provider {
        skein_llm::Provider::OpenAi => panic!("fixture exercises the withdrawn routes"),
        Provider::OpenAiCodex => CODEX_MEASURE,
        Provider::Anthropic => ANTHROPIC_MEASURE,
    }
}

fn refusal(peer: &Peer) -> &[u8] {
    assert_eq!(peer.request_bytes, 0);
    let [ToDomain::Failed { failure, evidence, detail, .. }] = peer.terminals.as_slice() else {
        panic!("one unsent failure: {:?}", peer.terminals);
    };
    assert_eq!(failure.clone(), llm::Failure::ContextTooLong);
    assert_eq!(*evidence, llm::Evidence::Unsent);
    detail
}

#[test]
fn a_request_within_the_line_is_sent() {
    for provider in [Provider::OpenAiCodex, Provider::Anthropic] {
        let guard = llm::Guard { line: LINE, last_prompt: LINE - measure(provider), from: 1 };
        let peer = attempt(provider, Some(guard), b"answer");
        assert!(peer.request_bytes > 0, "{provider:?}");
        assert!(peer.terminals.is_empty());
    }
}

#[test]
fn a_request_one_byte_past_the_line_fails_unsent() {
    for provider in [Provider::OpenAiCodex, Provider::Anthropic] {
        let guard = llm::Guard { line: LINE, last_prompt: LINE - measure(provider) + 1, from: 1 };
        let peer = attempt(provider, Some(guard), b"answer");
        refusal(&peer);
        assert_eq!(peer.connections(), 0);
    }
}

#[test]
fn the_guards_failure_names_the_line_and_the_measure() {
    for (provider, expected) in [
        (Provider::OpenAiCodex, b"past the line: last prompt 830 + new part 171 > line 1000".as_slice()),
        (Provider::Anthropic, b"past the line: last prompt 877 + new part 124 > line 1000".as_slice()),
    ] {
        let guard = llm::Guard { line: LINE, last_prompt: LINE - measure(provider) + 1, from: 1 };
        assert_eq!(refusal(&attempt(provider, Some(guard), b"answer")), expected);
    }
}

#[test]
fn short_lines_are_measured_as_they_render() {
    for (provider, expected) in [
        // Six payload bytes, with three newlines taking two encoded bytes each.
        (Provider::OpenAiCodex, b"past the line: last prompt 830 + new part 174 > line 1000".as_slice()),
        (Provider::Anthropic, b"past the line: last prompt 877 + new part 127 > line 1000".as_slice()),
    ] {
        let guard = llm::Guard { line: LINE, last_prompt: LINE - measure(provider) + 1, from: 1 };
        assert_eq!(refusal(&attempt(provider, Some(guard), b"1\n2\n3\n")), expected);
    }
}

#[test]
fn escaped_bytes_are_measured_as_they_render() {
    for (provider, expected) in [
        // CONTROL_BYTES * 6 replaces six plain answer bytes: an addition of 90.
        (Provider::OpenAiCodex, b"past the line: last prompt 830 + new part 261 > line 1000".as_slice()),
        (Provider::Anthropic, b"past the line: last prompt 877 + new part 214 > line 1000".as_slice()),
    ] {
        let guard = llm::Guard { line: LINE, last_prompt: LINE - measure(provider) + 1, from: 1 };
        assert_eq!(refusal(&attempt(provider, Some(guard), &[1; CONTROL_BYTES])), expected);
    }
}

#[test]
fn a_request_without_a_guard_is_sent_unmeasured() {
    for provider in [Provider::OpenAiCodex, Provider::Anthropic] {
        let peer = attempt(provider, None, &[1; CONTROL_BYTES]);
        assert!(peer.request_bytes > 0);
        assert!(peer.terminals.is_empty());
    }
}
