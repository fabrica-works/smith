//! Operator failure words, never channel or prompt content (protocol/agent.md,
//! section 6; shell.md, section 4.3). No process state or credential is known.
use crate::{AccountState, AnswerClass, AnswerFailure, FailureClass};
use alloc::boxed::Box;
use skein_lib::{Decimal, List, Wall};

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

/// Name an effort refused by its model and the model's declared alternatives.
#[must_use]
pub fn effort_words(effort: &[u8], choices: &[Box<[u8]>], cap: u32, dropped: bool) -> Box<[u8]> {
    let mut line = List::with_capacity(cap);
    bounded_put(&mut line, b"Effort ");
    bounded_put(&mut line, clip(effort));
    if dropped {
        bounded_put(&mut line, b" dropped for the provider default; accepted: ");
    } else {
        bounded_put(&mut line, b" is not declared; accepted: ");
    }
    if choices.is_empty() {
        bounded_put(&mut line, b"any effort within 32 bytes");
    }
    for (index, choice) in choices.iter().enumerate() {
        if line.room() < u32::try_from(choice.len()).expect("effort fits u32").saturating_add(5) {
            bounded_put(&mut line, b"...");
            break;
        }
        if index > 0 {
            put(&mut line, b", ");
        }
        put(&mut line, choice);
    }
    line.into_boxed()
}

fn bounded_put(line: &mut List<u8>, bytes: &[u8]) {
    for (index, byte) in bytes.iter().enumerate() {
        if byte & 0xc0 == 0x80 {
            continue;
        }
        let width = if *byte < 0x80 {
            1
        } else if *byte < 0xe0 {
            2
        } else if *byte < 0xf0 {
            3
        } else {
            4
        };
        if line.room() < width {
            return;
        }
        let end = index.checked_add(usize::try_from(width).expect("UTF-8 width fits usize")).expect("bounded text");
        let Some(character) = bytes.get(index..end) else { return };
        for byte in character {
            line.push(*byte).expect("complete character fits");
        }
    }
}

/// Render one failure, using its named account and the host clock at its answer when known.
#[must_use]
pub fn words(failure: &AnswerFailure, account_name: Option<&[u8]>, answered_at: Option<Wall>) -> Box<[u8]> {
    let mut line = List::with_capacity(4096);
    match &failure.class {
        AnswerClass::Model => model(&mut line, failure, account_name, answered_at),
        AnswerClass::Budget => {
            named(&mut line, b"out of ", failure.which.as_deref(), b"budget");
            if let Some(used) = failure.used
                && let Some(bound) = failure.bound
            {
                put(&mut line, b": used ");
                put(&mut line, Decimal::of(used).as_bytes());
                put(&mut line, b" of ");
                put(&mut line, Decimal::of(bound).as_bytes());
            }
        }
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

fn model(line: &mut List<u8>, failure: &AnswerFailure, account_name: Option<&[u8]>, answered_at: Option<Wall>) {
    if let Some(program) = &failure.program {
        let name = match program.as_ref() {
            b"claude_code" => b"claude".as_slice(),
            name => name,
        };
        put(line, clip(name));
        put(line, if failure.exit.is_some() { b" exited before its answer: " } else { b" could not run: " });
        if let Some(detail) = &failure.detail {
            program_detail(line, detail);
        }
        return;
    }
    if let Some(number) = failure.account {
        match &failure.state {
            Some(AccountState::Limited) => {
                account(line, number, account_name);
                put(line, b" reached its usage limit");
                if let Some(wait) = failure.reset_ms
                    && let Some(wall) = answered_at
                {
                    put(line, b", usable again at ");
                    put(line, &clock(Wall::from_nanos(wall.as_nanos().saturating_add(wait.saturating_mul(1_000_000)))));
                }
                return;
            }
            Some(AccountState::Refused) => {
                account(line, number, account_name);
                put(line, b": the provider refused it");
                if let Some(code) = &failure.code {
                    named(line, b": ", Some(code), b"unknown");
                }
                return;
            }
            Some(AccountState::Unauthorized) => {
                account(line, number, account_name);
                put(line, b" unauthorized");
                return;
            }
            Some(AccountState::Unknown(_)) | None => {}
        }
    }
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

fn program_detail(line: &mut List<u8>, detail: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    // Clipping first bounds expansion at four times DETAIL_BYTES, inside
    // the 4096-byte line even with its clipped name and fixed vocabulary.
    for byte in clip(detail) {
        if *byte < 0x20 || *byte == 0x7f {
            put(line, b"\\x");
            put(
                line,
                &[
                    *HEX.get(usize::from(byte >> 4_u32)).expect("high hex digit"),
                    *HEX.get(usize::from(byte & 15)).expect("low hex digit"),
                ],
            );
        } else {
            put(line, &[*byte]);
        }
    }
}

fn account(line: &mut List<u8>, number: u32, name: Option<&[u8]>) {
    put(line, b"account ");
    match name {
        Some(name) => put(line, clip(name)),
        None => put(line, Decimal::of(u64::from(number)).as_bytes()),
    }
}

/// The wall clock's hour and minute in UTC, as `HH:MM`.
#[must_use]
pub fn clock(at: Wall) -> [u8; 5] {
    let minutes = at.as_nanos() / 60_000_000_000;
    let hours = u8::try_from((minutes / 60) % 24).expect("hour fits u8");
    let minutes = u8::try_from(minutes % 60).expect("minute fits u8");
    [
        b'0'.checked_add(hours / 10).expect("ASCII digit"),
        b'0'.checked_add(hours % 10).expect("ASCII digit"),
        b':',
        b'0'.checked_add(minutes / 10).expect("ASCII digit"),
        b'0'.checked_add(minutes % 10).expect("ASCII digit"),
    ]
}

fn class(class: &FailureClass) -> &[u8] {
    match class {
        FailureClass::Overloaded => b"overloaded",
        FailureClass::RateLimited => b"rate limited",
        FailureClass::Exhausted => b"limited",
        FailureClass::Refused => b"refused",
        FailureClass::Unsupported => b"unsupported",
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

#[cfg(test)]
mod tests {
    #[test]
    fn effort_words_keep_whole_characters_at_the_host_bound() {
        assert_eq!(super::effort_words("中文".as_bytes(), &[], 10, false).as_ref(), "Effort 中".as_bytes());
    }
}
