//! Operator failure words, never channel or prompt content (protocol/agent.md,
//! section 6; shell.md, section 4.3). No process state or credential is known.
use crate::{AnswerClass, AnswerFailure, FailureClass};
use alloc::boxed::Box;
use skein_lib::{Decimal, List};

/// The vocabulary's byte ceiling on an operator's detail.
pub const DETAIL_BYTES: usize = 512;

/// Keep the first line within the byte ceiling and at a UTF-8 character boundary.
#[must_use]
#[expect(clippy::disallowed_methods, reason = "the operator codec validates its UTF-8 prefix before rendering")]
pub fn clip(detail: &[u8]) -> &[u8] {
    let prefix = detail.get(..detail.len().min(DETAIL_BYTES)).expect("bounded prefix");
    let mut end = prefix.len();
    for (index, byte) in prefix.iter().enumerate() {
        if *byte == b'\n' || *byte == b'\r' {
            end = index;
            break;
        }
    }
    let line = prefix.get(..end).expect("first line within prefix");
    match core::str::from_utf8(line) {
        Ok(_) => line,
        Err(error) => line.get(..error.valid_up_to()).expect("valid UTF-8 prefix"),
    }
}

/// A typed answer failure in operator words, including unknown vocabulary values.
#[must_use]
pub fn words(failure: &AnswerFailure) -> Box<[u8]> {
    let mut line = List::with_capacity(4096);
    match &failure.class {
        AnswerClass::Model => model(&mut line, failure),
        AnswerClass::Budget => named(&mut line, b"out of ", failure.which.as_deref(), b"budget"),
        AnswerClass::Policy => named(&mut line, b"policy: ", failure.reason.as_deref(), b"unfinished result"),
        AnswerClass::Cancelled => put(&mut line, b"the run was cancelled"),
        AnswerClass::Stale => named(&mut line, b"stale: ", failure.reason.as_deref(), b"workspace changed"),
        AnswerClass::Transcript => {
            named(&mut line, b"chat cannot be resumed: ", failure.reason.as_deref(), b"transcript refused");
        }
        AnswerClass::Agent => put(&mut line, b"the agent ended without answering"),
        AnswerClass::Host => named(&mut line, b"host: ", failure.reason.as_deref(), b"operation failed"),
        AnswerClass::Busy => put(&mut line, b"busy"),
        AnswerClass::Invalid => named(&mut line, b"invalid: ", failure.which.as_deref(), b"start refused"),
        AnswerClass::Unknown(value) => named(&mut line, b"unknown failure: ", Some(value), b"unknown"),
    }
    line.into_boxed()
}

fn model(line: &mut List<u8>, failure: &AnswerFailure) {
    if let Some(endpoint) = &failure.endpoint {
        put(line, clip(endpoint));
        named(line, b"/", failure.model.as_deref(), b"unknown model");
        put(line, b" ");
    }
    match &failure.completion {
        Some(completion) => {
            put(line, class(&completion.class));
            if let Some(phase) = &completion.phase {
                put(line, b" during ");
                put(
                    line,
                    match phase {
                        crate::FailurePhase::Connect => b"connect",
                        crate::FailurePhase::Handshake => b"handshake",
                        crate::FailurePhase::Head => b"response head",
                        crate::FailurePhase::Idle => b"response idle",
                        crate::FailurePhase::Whole => b"whole request",
                        crate::FailurePhase::Unknown(value) => clip(value),
                    },
                );
            }
            if let Some(limit) = &completion.limit {
                named(line, b": ", Some(&limit.which), b"limit");
                put(line, b" reached its bound of ");
                put(line, Decimal::of(limit.bound).as_bytes());
            }
            if let Some(status) = completion.status {
                put(line, b" (HTTP ");
                put(line, Decimal::of(status).as_bytes());
                put(line, b")");
            }
        }
        None => named(line, b"", failure.which.as_deref(), b"model failed"),
    }
    if let Some(attempts) = failure.attempts {
        put(line, b" after ");
        put(line, Decimal::of(attempts).as_bytes());
        put(line, b" attempts");
    }
}

fn class(class: &FailureClass) -> &[u8] {
    match class {
        FailureClass::Overloaded => b"overloaded",
        FailureClass::RateLimited => b"rate limited",
        FailureClass::Exhausted => b"exhausted",
        FailureClass::Unavailable => b"unavailable",
        FailureClass::TimedOut => b"timed out",
        FailureClass::ContextTooLong => b"context too long",
        FailureClass::Invalid => b"invalid",
        FailureClass::Unauthorized => b"unauthorized",
        FailureClass::Limit => b"limit",
        FailureClass::Protocol => b"protocol failed",
        FailureClass::Cancelled => b"cancelled",
        FailureClass::Unknown(value) => clip(value),
    }
}

fn named(line: &mut List<u8>, prefix: &[u8], value: Option<&[u8]>, absent: &[u8]) {
    put(line, prefix);
    put(
        line,
        match value {
            Some(value) => clip(value),
            None => absent,
        },
    );
}

fn put(line: &mut List<u8>, text: &[u8]) {
    for byte in text {
        line.push(*byte).expect("fixed words and clipped names fit the operator word bound");
    }
}
