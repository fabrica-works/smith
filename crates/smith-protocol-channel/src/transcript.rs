//! Saved turn bytes become concrete session history (protocol/transcript.md,
//! sections 2–4 and 8; domain/session.md, section 3).
//!
//! This pure translation keeps no state. It knows configured endpoint names
//! and dialect identities, never provider addresses or credential values.
//! `decode_transcript` checks the codec and sequence before the domain admits
//! the history; the domain still checks call/result pairing and ownership.

use alloc::boxed::Box;
use skein_lib::{List, Reader};
use smith_domain_session::{llm, record};
use smith_domain_tools as tools;
use smith_transcript as wire;

use crate::Endpoints;

/// Decode the host's ordered saved turn bodies into the domain's concrete history.
pub fn decode_transcript(
    bytes: &[Box<[u8]>],
    limits: &wire::v3::Limits,
    endpoints: &Endpoints,
) -> Result<Option<record::Transcript>, record::Refusal> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let Ok(count) = u32::try_from(bytes.len()) else {
        return Err(record::Refusal::TooLarge);
    };
    let mut turns = List::with_capacity(count);
    let mut first_endpoint: Option<llm::Endpoint> = None;
    let mut first_dialect: Option<u32> = None;
    let mut previous: Option<u32> = None;
    for body in bytes {
        if body.get(..2) != Some(&[0, 3][..]) {
            return Err(record::Refusal::Version);
        }
        let place = match previous {
            Some(place) => Some(place.checked_add(1).ok_or(record::Refusal::Malformed)?),
            None => None,
        };
        let turn = saved_turn(body, place, limits, endpoints)?;
        if turns.is_empty() {
            match turn.kind {
                record::Kind::First { .. } | record::Kind::Window { .. } => {}
                record::Kind::Ordinary => return Err(record::Refusal::Malformed),
            }
        } else {
            match turn.kind {
                record::Kind::First { .. } | record::Kind::Window { .. } => return Err(record::Refusal::Malformed),
                record::Kind::Ordinary => {}
            }
        }
        previous = Some(turn.sequence);
        match first_endpoint {
            Some(first) if first != turn.endpoint => return Err(record::Refusal::Endpoint),
            Some(_) => {}
            None => first_endpoint = Some(turn.endpoint),
        }
        match first_dialect {
            Some(first) if first != turn.dialect => return Err(record::Refusal::Dialect),
            Some(_) => {}
            None => first_dialect = Some(turn.dialect),
        }
        if turns.push(turn).is_err() {
            return Err(record::Refusal::TooLarge);
        }
    }
    Ok(Some(record::Transcript {
        version: record::VERSION,
        endpoint: first_endpoint.ok_or(record::Refusal::Malformed)?,
        dialect: first_dialect.ok_or(record::Refusal::Malformed)?,
        turns: turns.into_boxed(),
    }))
}

/// Decode one live numbered turn without requiring its earlier saved prefix.
pub fn decode_turn(
    bytes: &[u8],
    number: u32,
    limits: &wire::v3::Limits,
    endpoints: &Endpoints,
) -> Result<record::Turn, record::Refusal> {
    if number == 0 {
        return Err(record::Refusal::Malformed);
    }
    saved_turn(bytes, Some(number), limits, endpoints)
}

fn saved_turn(
    bytes: &[u8],
    number: Option<u32>,
    limits: &wire::v3::Limits,
    endpoints: &Endpoints,
) -> Result<record::Turn, record::Refusal> {
    if bytes.get(..2) != Some(&[0, 3][..]) {
        return Err(record::Refusal::Version);
    }
    let Ok(source) = wire::Turn::decode(limits, &mut Reader::new(bytes)) else {
        return Err(record::Refusal::Malformed);
    };
    if source.place() == 0 {
        return Err(record::Refusal::Malformed);
    }
    match number {
        Some(number) if source.place() != number => return Err(record::Refusal::Malformed),
        Some(_) | None => {}
    }
    let endpoint = endpoints.resolve(source.endpoint()).ok_or(record::Refusal::Endpoint)?;
    if source.dialect() != dialect_name(endpoint.dialect).as_ref() {
        return Err(record::Refusal::Dialect);
    }
    translate_turn(source, endpoint.number, endpoint.dialect)
}

#[expect(clippy::manual_map, reason = "step code translates optional affinity through an exhaustive match")]
fn translate_turn(source: wire::Turn, endpoint: u32, dialect: u32) -> Result<record::Turn, record::Refusal> {
    let affinity = match source.affinity() {
        Some(value) => Some(llm::Affinity { key: *value.key(), thread: value.thread() }),
        None => None,
    };
    let kind = match source.kind() {
        wire::TurnKind::Ordinary => match affinity {
            Some(affinity) if source.place() == 1 => record::Kind::First { affinity },
            None if source.place() > 1 => record::Kind::Ordinary,
            Some(_) | None => return Err(record::Refusal::Malformed),
        },
        wire::TurnKind::Window => match affinity {
            Some(affinity) => record::Kind::Window { affinity },
            None => return Err(record::Refusal::Malformed),
        },
    };
    let mut messages = List::with_capacity(source.messages().len());
    for message in source.messages() {
        let role = match message.role() {
            wire::Role::User => llm::Role::User,
            wire::Role::Assistant => llm::Role::Assistant,
        };
        let mut content = List::with_capacity(message.blocks().len());
        for block in message.blocks() {
            let value = decode_block(block, dialect)?;
            if content.push(value).is_err() {
                return Err(record::Refusal::TooLarge);
            }
        }
        if messages.push(llm::Message { role, content: content.into_boxed() }).is_err() {
            return Err(record::Refusal::TooLarge);
        }
    }
    Ok(record::Turn {
        version: record::VERSION,
        kind,
        endpoint: llm::Endpoint(endpoint),
        dialect,
        sequence: source.place(),
        usage: llm::Usage {
            input_tokens: *source.usage().input(),
            output_tokens: *source.usage().output(),
            cache_read_tokens: *source.usage().cache_read(),
            cache_write_tokens: *source.usage().cache_write(),
            reasoning_tokens: *source.usage().reasoning(),
        },
        spent: source.spent(),
        messages: messages.into_boxed(),
    })
}

fn decode_block(source: &wire::Block, dialect: u32) -> Result<llm::Block, record::Refusal> {
    let value = match source {
        wire::Block::Text(text) => {
            llm::Block::Text { text: Box::from(text.text()), replay: replay(text.replay().as_ref(), dialect)? }
        }
        wire::Block::Refusal(text) => {
            llm::Block::Refusal { text: Box::from(text.text()), replay: replay(text.replay().as_ref(), dialect)? }
        }
        wire::Block::Call(call) => llm::Block::ToolCall {
            id: Box::from(call.id()),
            name: Box::from(call.name()),
            input: Box::from(call.input()),
            call: llm::Decoded::Historical,
            replay: replay(call.replay().as_ref(), dialect)?,
        },
        wire::Block::Result(result) => {
            llm::Block::ToolResult { id: Box::from(result.id()), result: returned(result.returned())? }
        }
        wire::Block::Opaque(opaque) => {
            if opaque.dialect() != dialect_name(dialect).as_ref() {
                return Err(record::Refusal::Dialect);
            }
            llm::Block::Opaque { bytes: Box::from(opaque.bytes()) }
        }
    };
    Ok(value)
}

fn replay(source: Option<&wire::Replay>, dialect: u32) -> Result<Option<llm::Replay>, record::Refusal> {
    match source {
        Some(value) => {
            if value.dialect() != dialect_name(dialect).as_ref() {
                return Err(record::Refusal::Dialect);
            }
            Ok(Some(llm::Replay { bytes: Box::from(value.bytes()) }))
        }
        None => Ok(None),
    }
}

fn returned(source: &wire::Returned) -> Result<llm::Returned, record::Refusal> {
    let value = match source {
        wire::Returned::Outcome(value) => llm::Returned::Owned { outcome: outcome(value.value())? },
        wire::Returned::Said(value) => {
            llm::Returned::Text { text: Box::from(value.text()), error: value.error(), replay: None }
        }
        wire::Returned::Invalid(value) => llm::Returned::Invalid { problem: problem(value.problem()) },
        wire::Returned::NotRun => llm::Returned::NotRun { why: llm::NotRun::Stopped },
        wire::Returned::Withdrawn => llm::Returned::Withdrawn,
    };
    Ok(value)
}

fn problem(source: &wire::CallProblem) -> llm::Problem {
    match source {
        wire::CallProblem::UnknownTool => llm::Problem::UnknownTool,
        wire::CallProblem::Nesting => llm::Problem::Nesting,
        wire::CallProblem::NotAnObject => llm::Problem::NotAnObject,
        wire::CallProblem::Missing(field) => llm::Problem::Missing { field: Box::from(field.field()) },
        wire::CallProblem::WrongType(field) => llm::Problem::WrongType { field: Box::from(field.field()) },
        wire::CallProblem::BadValue(field) => llm::Problem::BadValue { field: Box::from(field.field()) },
        wire::CallProblem::TooLarge => llm::Problem::TooLarge,
        wire::CallProblem::Oversize(problem) => {
            llm::Problem::Oversize { bytes: problem.bytes(), bound: problem.bound() }
        }
        wire::CallProblem::CutOff(problem) => llm::Problem::CutOff { bytes: problem.bytes() },
    }
}

fn outcome(source: &wire::Outcome) -> Result<tools::Outcome, record::Refusal> {
    let value = match source {
        wire::Outcome::Read(read) => tools::Outcome::Read {
            content: Box::from(read.content()),
            skipped: read.skipped(),
            lines: read.lines(),
            total: read.total(),
            cut: read.cut(),
        },
        wire::Outcome::Listed(listed) => {
            let mut entries = List::with_capacity(listed.entries().len());
            for entry in listed.entries() {
                normalized(entry.name())?;
                if entry.name().is_empty() {
                    return Err(record::Refusal::Malformed);
                }
                let name = Box::from(entry.name());
                let kind = match entry.kind() {
                    wire::Kind::File => tools::Kind::File,
                    wire::Kind::Directory => tools::Kind::Directory,
                    wire::Kind::Link => tools::Kind::Link,
                    wire::Kind::Other => tools::Kind::Other,
                };
                if entries.push(tools::Entry { name, kind }).is_err() {
                    return Err(record::Refusal::TooLarge);
                }
            }
            tools::Outcome::Listed { entries: entries.into_boxed(), more: listed.more() }
        }
        wire::Outcome::Found(found) => {
            let mut hits = List::with_capacity(found.hits().len());
            for hit in found.hits() {
                let item = tools::Hit { path: Box::from(hit.path()), line: hit.line(), text: Box::from(hit.text()) };
                if hits.push(item).is_err() {
                    return Err(record::Refusal::TooLarge);
                }
            }
            tools::Outcome::Found { hits: hits.into_boxed(), more: found.more(), timed_out: found.timed_out() }
        }
        wire::Outcome::Written(written) => tools::Outcome::Written { created: written.created() },
        wire::Outcome::Edited(edited) => tools::Outcome::Edited { replaced: edited.replaced() },
        wire::Outcome::Exited(command) => {
            let exit = match command.exit() {
                wire::Exit::Code(code) => tools::Exit::Code { code: code.code() },
                wire::Exit::Signal(signal) => tools::Exit::Signal { signal: signal.signal() },
                wire::Exit::TimedOut => tools::Exit::TimedOut,
            };
            tools::Outcome::Exited {
                exit,
                head: Box::from(command.head()),
                tail: Box::from(command.last()),
                dropped: command.dropped(),
            }
        }
        wire::Outcome::NotGranted => tools::Outcome::NotGranted,
        wire::Outcome::Outside(failure) => tools::Outcome::Outside { path: resolved(failure.path())? },
        wire::Outcome::ReadOnly(failure) => tools::Outcome::ReadOnly { path: resolved(failure.path())? },
        wire::Outcome::TooLong => tools::Outcome::TooLong,
        wire::Outcome::NotFound(failure) => tools::Outcome::NotFound { path: resolved(failure.path())? },
        wire::Outcome::NotFile => tools::Outcome::NotFile,
        wire::Outcome::Linked => tools::Outcome::Linked,
        wire::Outcome::Protected => tools::Outcome::Protected,
        wire::Outcome::NotDirectory(failure) => tools::Outcome::NotDirectory { path: resolved(failure.path())? },
        wire::Outcome::TooLarge(size) => tools::Outcome::TooLarge { size: size.size() },
        wire::Outcome::NotRead => tools::Outcome::NotRead,
        wire::Outcome::Stale => tools::Outcome::Stale,
        wire::Outcome::EditFailed(failed) => {
            if failed.edit() == 0 {
                return Err(record::Refusal::Malformed);
            }
            tools::Outcome::EditFailed { edit: failed.edit(), why: mismatch(failed.why())? }
        }
        wire::Outcome::Failed(failed) => {
            let fault = match failed.fault() {
                wire::Fault::Denied => tools::Fault::Denied,
                wire::Fault::NoSpace => tools::Fault::NoSpace,
                wire::Fault::Other => tools::Fault::Other,
            };
            tools::Outcome::Failed { fault }
        }
        wire::Outcome::TimedOut => tools::Outcome::TimedOut,
        wire::Outcome::Cancelled => tools::Outcome::Cancelled,
        wire::Outcome::Busy => tools::Outcome::Busy,
        wire::Outcome::NulByte => tools::Outcome::NulByte,
    };
    Ok(value)
}

fn dialect_name(dialect: u32) -> [u8; 8] {
    let [a, b, c, d] = dialect.to_be_bytes();
    [hex(a >> 4), hex(a & 15), hex(b >> 4), hex(b & 15), hex(c >> 4), hex(c & 15), hex(d >> 4), hex(d & 15)]
}

fn hex(value: u8) -> u8 {
    if value < 10 {
        b'0'.checked_add(value).expect("hex digit")
    } else {
        b'a'.checked_add(value.checked_sub(10).expect("hex letter")).expect("hex digit")
    }
}

/// Restore a normalized descriptor, refusing a malformed directory name or path.
fn resolved(source: &wire::Resolved) -> Result<tools::Resolved, record::Refusal> {
    let value = match source {
        wire::Resolved::Working(working) => {
            normalized(working.path())?;
            tools::Resolved::Working { path: Box::from(working.path()) }
        }
        wire::Resolved::Other(other) => {
            normalized(other.path())?;
            let directory = tools::Name::new(Box::from(other.directory())).ok_or(record::Refusal::Malformed)?;
            tools::Resolved::Other { directory, path: Box::from(other.path()) }
        }
        wire::Resolved::Outside(outside) => {
            normalized(outside.path())?;
            tools::Resolved::Outside { path: Box::from(outside.path()) }
        }
    };
    Ok(value)
}

fn normalized(path: &[u8]) -> Result<(), record::Refusal> {
    if path.is_empty() {
        return Ok(());
    }
    let mut start = 0_usize;
    for (index, byte) in path.iter().enumerate() {
        if *byte == 0 {
            return Err(record::Refusal::Malformed);
        }
        if *byte == b'/' {
            let name = path.get(start..index).expect("component within path");
            component(name)?;
            start = index.checked_add(1).expect("path index");
        }
    }
    component(path.get(start..).expect("remaining component"))
}

fn component(name: &[u8]) -> Result<(), record::Refusal> {
    if name.is_empty() || name == b"." || name == b".." {
        return Err(record::Refusal::Malformed);
    }
    Ok(())
}

fn mismatch(source: &wire::Mismatch) -> Result<tools::Mismatch, record::Refusal> {
    let why = match source {
        wire::Mismatch::NoMatch => tools::Mismatch::NoMatch,
        wire::Mismatch::Unchanged => tools::Mismatch::Unchanged,
        wire::Mismatch::Ambiguous(ambiguous) => {
            let mut lines = List::with_capacity(ambiguous.lines().len());
            for line in ambiguous.lines() {
                if lines.push(line.number()).is_err() {
                    return Err(record::Refusal::TooLarge);
                }
            }
            tools::Mismatch::Ambiguous { count: ambiguous.count(), lines: lines.into_boxed() }
        }
    };
    Ok(why)
}

#[cfg(test)]
mod edit_tests {
    use super::{outcome, record, tools, wire};

    #[test]
    fn an_edit_failure_index_counts_from_one_at_transcript_admission() {
        let zero = wire::Outcome::EditFailed(
            wire::EditFailed::new(&wire::CEILINGS, wire::EditFailedParts { edit: 0, why: wire::Mismatch::NoMatch })
                .expect("codec-shaped value"),
        );
        assert_eq!(outcome(&zero), Err(record::Refusal::Malformed));
        let two = wire::Outcome::EditFailed(
            wire::EditFailed::new(&wire::CEILINGS, wire::EditFailedParts { edit: 2, why: wire::Mismatch::NoMatch })
                .expect("bounded value"),
        );
        assert_eq!(outcome(&two), Ok(tools::Outcome::EditFailed { edit: 2, why: tools::Mismatch::NoMatch }));
    }
}
