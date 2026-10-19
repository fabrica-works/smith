//! Operator lines retain a refusal's relationship, names and units within the
//! vocabulary's 512-byte bound. No configuration or provider text is retained.
//! `words` meets the shell. Contract: protocol/agent.md, section 6;
//! protocol/limits.md, sections 4 and 8.

use alloc::boxed::Box;
use skein_lib::List;
use skein_llm::{Cap, Error};
use skein_llm_connection::EndpointError;

use super::{Limit, LimitName, Per, Refusal, Value};

/// Render one startup refusal for the shell, bounded to 512 UTF-8 bytes.
#[must_use]
pub fn words(refusal: &Refusal) -> Box<[u8]> {
    let mut output = List::with_capacity(512);
    match refusal {
        Refusal::Field { limit } => {
            text(&mut output, "field: ");
            name(&mut output, limit);
            text(&mut output, " must not be empty");
        }
        Refusal::Broken { check, left, right } => {
            text(&mut output, check.name());
            text(&mut output, ": ");
            limit(&mut output, left);
            text(&mut output, "; ");
            limit(&mut output, right);
        }
        Refusal::Derived { limit } => {
            text(&mut output, "derived: ");
            name(&mut output, limit);
            text(&mut output, " does not fit its type");
        }
        Refusal::Environment { index } => {
            text(&mut output, "environment: entry ");
            number(&mut output, u64::from(*index));
            text(&mut output, " has an invalid or repeated name");
        }
        Refusal::Connection(error) => connection(&mut output, *error),
        Refusal::Llm { endpoint, violation } => native(&mut output, *endpoint, *violation),
    }
    output.into_boxed()
}

fn name(output: &mut List<u8>, name: &LimitName) {
    text(output, name.layer.name());
    text(output, ".");
    text(output, name.field);
    match name.per {
        Per::Deployment => {}
        Per::Endpoint(endpoint) => {
            text(output, "[endpoint=");
            number(output, u64::from(endpoint));
            text(output, "]");
        }
        Per::Model { endpoint, model } => {
            text(output, "[endpoint=");
            number(output, u64::from(endpoint));
            text(output, ",model=");
            number(output, u64::from(model));
            text(output, "]");
        }
    }
}

fn limit(output: &mut List<u8>, limit: &Limit) {
    name(output, &limit.name);
    text(output, "=");
    match limit.value {
        Value::Bytes(value) => {
            number(output, value);
            text(output, " bytes");
        }
        Value::Count(value) => {
            number(output, value);
            text(output, " items");
        }
        Value::Tokens(value) => {
            number(output, value);
            text(output, " tokens");
        }
        Value::Spend(value) => {
            number(output, value);
            text(output, " host units");
        }
        Value::Duration(value) => {
            number(output, value.as_nanos());
            text(output, " ns");
        }
        Value::Fraction(value) => {
            number(output, u64::from(value.numerator));
            text(output, "/");
            number(output, u64::from(value.denominator));
            text(output, " share");
        }
    }
}

fn connection(output: &mut List<u8>, error: EndpointError) {
    match error {
        EndpointError::TooMany => text(output, "connection-endpoints: too many"),
        EndpointError::Limits => text(output, "connection-limits: inconsistent or unrepresentable"),
        EndpointError::ConnectionsCalls { connections, calls } => {
            text(output, "connections-calls: connections=");
            number(output, u64::from(connections));
            text(output, " items; calls=");
            number(output, u64::from(calls));
            text(output, " items");
        }
        EndpointError::Client(error) => client(output, error),
        EndpointError::HttpReadTlsRead { demand, cap } => pair(output, "http-read-tls-read", demand, cap),
        EndpointError::HttpSendTlsSend { demand, cap } => pair(output, "http-send-tls-send", demand, cap),
        EndpointError::TlsReadIoIntake { demand, cap } => pair(output, "tls-read-io-intake", demand, cap),
        EndpointError::TlsSendIoOutput { demand, cap } => pair(output, "tls-send-io-output", demand, cap),
        EndpointError::HttpReadIoIntake { demand, cap } => pair(output, "http-read-io-intake", demand, cap),
        EndpointError::HttpSendIoOutput { demand, cap } => pair(output, "http-send-io-output", demand, cap),
        EndpointError::SseChunkHttpRead { demand, cap } => pair(output, "sse-chunk-http-read", demand, cap),
        EndpointError::TokenizerDemandSseChunk { demand, cap } => {
            pair(output, "tokenizer-demand-sse-chunk", demand, cap);
        }
        EndpointError::MemoryLargestCall { memory, call } => {
            text(output, "memory-largest-call: memory=");
            number(output, memory);
            text(output, " bytes; call=");
            number(output, call);
            text(output, " bytes");
        }
        EndpointError::PlaintextAddress => text(output, "plaintext-address: must be loopback"),
    }
}

fn pair(output: &mut List<u8>, check: &str, demand: u32, cap: u32) {
    text(output, check);
    text(output, ": demand=");
    number(output, u64::from(demand));
    text(output, " bytes; cap=");
    number(output, u64::from(cap));
    text(output, " bytes");
}

fn client(output: &mut List<u8>, error: Error) {
    match error {
        Error::Invalid => text(output, "request-head: invalid endpoint or reserved header"),
        Error::Unsupported => text(output, "request-head: unsupported endpoint"),
        Error::Limit { which, bound } => {
            text(output, "request-head: ");
            text(output, cap_name(which));
            text(output, " bound=");
            number(output, bound);
        }
    }
}

fn cap_name(cap: Cap) -> &'static str {
    match cap {
        Cap::Request => "request",
        Cap::RequestHead => "request-head",
        Cap::ResponseHead => "response-head",
        Cap::ResponseFields => "response-fields",
        Cap::Skip => "skip",
        Cap::Field => "field",
        Cap::Retained => "retained",
        Cap::Tokens => "tokens",
        Cap::Depth => "depth",
        Cap::Strings => "strings",
        Cap::Number => "number",
        Cap::Reasoning => "reasoning",
        Cap::OutputItems => "output-items",
        Cap::Metadata => "metadata",
        Cap::Tools => "tools",
        Cap::HistoryItems => "history-items",
        Cap::Output => "output",
        Cap::Answer => "answer",
        Cap::ErrorBody => "error-body",
        Cap::Value => "credential-value",
        Cap::AccountId => "account-id",
    }
}

fn text(output: &mut List<u8>, part: &str) {
    for character in part.chars() {
        let mut encoded = [0_u8; 4];
        let bytes = character.encode_utf8(&mut encoded).as_bytes();
        if bytes.len() > usize::try_from(output.room()).expect("bounded room fits usize") {
            break;
        }
        for byte in bytes {
            output.push(*byte).expect("character fits remaining room");
        }
    }
}

fn number(output: &mut List<u8>, mut value: u64) {
    let mut digits = [0_u8; 20];
    let mut count = 0_usize;
    for _ in 0_u8..20 {
        *digits.get_mut(count).expect("twenty decimal digits fit") =
            b'0'.checked_add(u8::try_from(value % 10).expect("one digit")).expect("ascii digit");
        count = count.checked_add(1).expect("twenty digits fit");
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for digit in digits.get(..count).expect("twenty digits fit").iter().rev() {
        if output.room() == 0 {
            break;
        }
        output.push(*digit).expect("room checked");
    }
}

fn native(output: &mut List<u8>, endpoint: u32, violation: skein_llm::Violation) {
    use skein_llm::Violation;
    let (relationship, left, left_value, right, right_value) = match violation {
        Violation::Overflow { rule } => {
            text(output, "derived: ");
            name(output, &LimitName { layer: super::Layer::Llm, field: rule_name(rule), per: Per::Endpoint(endpoint) });
            text(output, " does not fit its type");
            return;
        }
        Violation::InputAnswer { input, answer } => ("input-answer", "input", input, "answer", answer),
        Violation::OutputItemsCalls { output_items, calls } => {
            ("output-items-calls", "output_items", output_items, "calls_per_response", calls)
        }
        Violation::StringsInput { strings, input } => ("strings-input", "strings", strings, "input", input),
        Violation::StringsReasoning { strings, reasoning } => {
            ("strings-reasoning", "strings", strings, "reasoning", reasoning)
        }
    };
    text(output, relationship);
    text(output, ": ");
    name(output, &LimitName { layer: super::Layer::Llm, field: left, per: Per::Endpoint(endpoint) });
    text(output, "=");
    number(output, u64::from(left_value));
    text(output, "; ");
    name(output, &LimitName { layer: super::Layer::Llm, field: right, per: Per::Endpoint(endpoint) });
    text(output, "=");
    number(output, u64::from(right_value));
}

const fn rule_name(rule: skein_llm::Rule) -> &'static str {
    use skein_llm::Rule;
    match rule {
        Rule::Request => "request",
        Rule::Answer => "answer",
        Rule::Input => "input",
        Rule::Reasoning => "reasoning",
        Rule::OutputItems => "output_items",
        Rule::Strings => "strings",
        Rule::Tokens => "tokens",
        Rule::Retained => "retained",
        Rule::Receiving => "receiving",
        Rule::Skip => "skip",
        Rule::Tools => "tools",
        Rule::HistoryItems => "history_items",
    }
}
