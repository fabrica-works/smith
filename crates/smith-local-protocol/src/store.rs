//! Generated local chat codecs and serial durable-file requests
//! (domain/host.md, section 8; protocol/hosts.md, section 5.3).
//! Store keeps its selected root, checked bit, versions and bounded loading
//! buffers, never conversation policy or another build's record layout.
//! `Store::request` and `from_io` drive IO; `save_state`, `save_turn` and
//! the decoders translate concrete records without interpreting a conversation.

mod boundary;
mod machine;

pub use boundary::{Action, History, RecordName, StoreFault, StoreLimits, StoreRequest};
pub use machine::Store;

/// The chat store version named by the product startup record.
pub const CHAT_STORE_VERSION: u64 = 1;

use alloc::boxed::Box;
use skein_lib::{List, Reader, Token, Writer};
use smith_domain::Transcript;
use smith_local_domain::ChatState;
use smith_protocol_channel::{self as channel, Endpoints};
use smith_transcript as transcript;

/// One file replacement; the service acknowledges it only after a durable store terminal.
#[derive(Debug, PartialEq, Eq)]
pub struct File {
    /// Path relative to the selected chat directory.
    pub name: Box<[u8]>,
    /// Whole bounded replacement bytes.
    pub bytes: Box<[u8]>,
}

/// Why a retained chat record cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreError {
    /// The record is malformed, truncated, or from a different version.
    Malformed,
    /// A typed turn cannot be encoded under these configured endpoints.
    Turn,
    /// The record exceeds the configured file bound.
    TooLarge,
}

/// The metadata file after the caller has durably stored it.
#[must_use]
#[expect(clippy::manual_map, reason = "strict subset keeps optional boundary conversion as an explicit match")]
pub fn save_state(state: ChatState) -> File {
    let read = match state.read {
        Some(read) => Some(read.raw()),
        None => None,
    };
    let record = crate::local_store_v1::State::new(
        &crate::local_store_v1::CEILINGS,
        crate::local_store_v1::StateParts {
            identity: state.identity.bytes(),
            activation: state.activation,
            next_message: state.next_message,
            read,
        },
    )
    .expect("metadata has no variable bounds");
    let mut writer = Writer::new(usize::try_from(record.measure()).expect("measured bytes fit usize"));
    record.encode(&mut writer).expect("measured metadata");
    File { name: b"state".as_slice().into(), bytes: writer.finish() }
}

/// Read a complete generated local metadata file.
#[expect(clippy::manual_map, reason = "strict subset keeps optional boundary conversion as an explicit match")]
pub fn decode_state(bytes: &[u8]) -> Result<ChatState, StoreError> {
    let record = crate::local_store_v1::State::decode(&crate::local_store_v1::CEILINGS, &mut Reader::new(bytes))
        .or(Err(StoreError::Malformed))?
        .into_parts();
    let read = match record.read {
        Some(read) => Some(Token::new(read)),
        None => None,
    };
    Ok(ChatState {
        identity: smith_domain::run::Identity::new(record.identity),
        activation: record.activation,
        next_message: record.next_message,
        read,
    })
}

/// Fixed-width numeric path for one turn, sortable in transcript order.
#[must_use]
pub fn turn_name(place: u32) -> Box<[u8]> {
    let mut digits = [b'0'; 10];
    let mut number = place;
    for index in (0..10).rev() {
        let digit = number.checked_rem(10).expect("nonzero base");
        let cell = digits.get_mut(index).expect("decimal position within ten digits");
        *cell = b'0'.checked_add(u8::try_from(digit).expect("decimal digit fits")).expect("digit fits");
        number = number.checked_div(10).expect("nonzero base");
    }
    let mut writer = Writer::new(15);
    writer.put(&digits).expect("ten digits fit");
    writer.put(b".turn").expect("turn suffix fits");
    writer.finish()
}

/// Encode one concrete turn for a durable numbered file.
#[expect(clippy::manual_map, reason = "strict subset keeps optional boundary conversion as an explicit match")]
pub fn save_turn(
    place: u32,
    read: Option<Token>,
    opens_window: bool,
    turn: &smith_domain::Turn,
    limits: &transcript::v6::Limits,
    endpoints: &Endpoints,
) -> Result<File, StoreError> {
    if place == 0 || place != turn.sequence {
        return Err(StoreError::Malformed);
    }
    let Ok(bytes) = channel::encode_turn(turn, limits, endpoints) else { return Err(StoreError::Turn) };
    let read = match read {
        Some(read) => Some(read.raw()),
        None => None,
    };
    let record = crate::local_store_v1::Turn::new(
        &crate::local_store_v1::CEILINGS,
        crate::local_store_v1::TurnParts { read, opens_window, body: bytes },
    )
    .or(Err(StoreError::TooLarge))?;
    let mut writer = Writer::new(usize::try_from(record.measure()).expect("measured bytes fit usize"));
    record.encode(&mut writer).expect("measured turn envelope");
    Ok(File { name: turn_name(place), bytes: writer.finish() })
}

/// Read the whole generated envelope without interpreting the transcript body.
pub fn turn_opens_window(bytes: &[u8]) -> Result<bool, StoreError> {
    turn_opens_window_bounded(bytes, crate::local_store_v1::CEILINGS.turn_body)
}

pub(super) fn turn_opens_window_bounded(bytes: &[u8], file_bytes: u32) -> Result<bool, StoreError> {
    let limits = envelope_limits(file_bytes);
    let record =
        crate::local_store_v1::Turn::decode(&limits, &mut Reader::new(bytes)).or(Err(StoreError::Malformed))?;
    Ok(record.opens_window())
}

/// Decode only the last window's ordered files before returning Loaded.
pub fn decode_turns(
    files: &[Box<[u8]>],
    limits: &transcript::v6::Limits,
    endpoints: &Endpoints,
) -> Result<(Option<Transcript>, Option<Token>), StoreError> {
    decode_turns_bounded(files, limits, endpoints, crate::local_store_v1::CEILINGS.turn_body)
}

#[expect(clippy::manual_map, reason = "strict subset keeps optional boundary conversion as an explicit match")]
pub(super) fn decode_turns_bounded(
    files: &[Box<[u8]>],
    limits: &transcript::v6::Limits,
    endpoints: &Endpoints,
    file_bytes: u32,
) -> Result<(Option<Transcript>, Option<Token>), StoreError> {
    let envelope = envelope_limits(file_bytes);
    let mut start = None;
    for (index, file) in files.iter().enumerate() {
        if turn_opens_window_bounded(file, file_bytes)? {
            start = Some(index);
        }
    }
    let files = match start {
        Some(start) => files.get(start..).expect("window lies within files"),
        None if files.is_empty() => files,
        None => return Err(StoreError::Malformed),
    };
    let Ok(count) = u32::try_from(files.len()) else { return Err(StoreError::TooLarge) };
    let mut bodies = List::with_capacity(count);
    let mut last_read = None;
    for file in files {
        let record = crate::local_store_v1::Turn::decode(&envelope, &mut Reader::new(file))
            .or(Err(StoreError::Malformed))?
            .into_parts();
        last_read = match record.read {
            Some(read) => Some(Token::new(read)),
            None => None,
        };
        bodies.push(record.body).expect("bounded turn count");
    }
    let count = bodies.len();
    let piece = channel::transcript_piece(bodies.as_slice()).ok_or(StoreError::Malformed)?;
    // The aggregate piece now owns the bodies' bytes. Release those copies
    // before typed transcript decoding; whole-envelope reads have their own copy.
    drop(bodies);
    match channel::decode_transcript(&piece, count, u32::MAX, limits, endpoints) {
        Ok(transcript) => Ok((transcript, last_read)),
        Err(_) => Err(StoreError::Malformed),
    }
}

fn envelope_limits(file_bytes: u32) -> crate::local_store_v1::Limits {
    crate::local_store_v1::Limits {
        turn_body: file_bytes.min(crate::local_store_v1::CEILINGS.turn_body),
        ..crate::local_store_v1::CEILINGS
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn saved_metadata_round_trips_with_read_fence() {
        let state = ChatState {
            identity: smith_domain::run::Identity::new([0x42; 16]),
            activation: 42,
            next_message: 9,
            read: Some(Token::new(8)),
        };
        let file = save_state(state);
        assert_eq!(file.name.as_ref(), b"state");
        assert_eq!(decode_state(&file.bytes), Ok(state));
        let truncated = file.bytes.get(..file.bytes.len().saturating_sub(1)).expect("state has bytes");
        assert_eq!(decode_state(truncated), Err(StoreError::Malformed));
    }

    #[test]
    fn numeric_turn_names_sort_in_place_order() {
        assert_eq!(turn_name(1).as_ref(), b"0000000001.turn");
        assert_eq!(turn_name(42).as_ref(), b"0000000042.turn");
        assert!(turn_name(9) < turn_name(10));
    }

    pub(crate) fn fixture() -> (Endpoints, smith_domain::Turn) {
        let mut entries = List::with_capacity(1);
        entries.push(channel::Endpoint { name: Box::from(&b"main"[..]), number: 2, dialect: 1 }).expect("one endpoint");
        let endpoints = Endpoints::new(entries);
        let turn = smith_domain::Turn {
            model: b"model".as_slice().into(),
            effort: None,
            kind: smith_domain::session::record::Kind::First(Box::new(smith_domain::session::record::Opener {
                affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
                account: None,
                backend: smith_domain::session::record::Backend::Direct,
            })),
            version: smith_domain::session::record::VERSION,
            endpoint: Some(smith_domain::session::llm::Endpoint(2)),
            dialect: 1,
            sequence: 1,
            usage: smith_domain::session::llm::Usage::ZERO,
            spent: 0,
            jobs: 0,
            messages: Box::new([]),
        };
        (endpoints, turn)
    }

    #[test]
    fn one_saved_turn_reloads_as_concrete_history() {
        let (endpoints, turn) = fixture();
        let file = save_turn(1, Some(Token::new(9)), true, &turn, &transcript::CEILINGS, &endpoints)
            .expect("bounded saved turn");
        assert_eq!(file.name.as_ref(), b"0000000001.turn");
        let (loaded, read) = decode_turns(&[file.bytes], &transcript::CEILINGS, &endpoints).expect("valid transcript");
        let loaded = loaded.expect("one turn");
        assert_eq!(read, Some(Token::new(9)));
        assert_eq!(loaded.turns.as_ref(), &[turn]);
    }

    #[test]
    fn a_load_starts_at_the_last_turn_that_opens_a_window() {
        let (endpoints, first) = fixture();
        let window = smith_domain::Turn {
            model: b"model".as_slice().into(),
            effort: None,
            kind: smith_domain::session::record::Kind::Window(Box::new(smith_domain::session::record::Opener {
                affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
                account: None,
                backend: smith_domain::session::record::Backend::Direct,
            })),
            sequence: 3,
            ..first.clone()
        };
        let earlier = save_turn(1, None, true, &first, &transcript::CEILINGS, &endpoints).expect("first file");
        let mut envelope =
            crate::local_store_v1::Turn::decode(&crate::local_store_v1::CEILINGS, &mut Reader::new(&earlier.bytes))
                .unwrap()
                .into_parts();
        envelope.body = b"not a transcript body".as_slice().into();
        let envelope = crate::local_store_v1::Turn::new(&crate::local_store_v1::CEILINGS, envelope).unwrap();
        let mut unread_body = Writer::new(usize::try_from(envelope.measure()).expect("measured bytes fit usize"));
        envelope.encode(&mut unread_body).unwrap();
        let window_file = save_turn(3, None, true, &window, &transcript::CEILINGS, &endpoints).expect("window file");
        let ordinary = smith_domain::Turn {
            model: b"model".as_slice().into(),
            effort: None,
            kind: smith_domain::session::record::Kind::Ordinary,
            sequence: 4,
            ..first
        };
        let last = save_turn(4, Some(Token::new(9)), false, &ordinary, &transcript::CEILINGS, &endpoints)
            .expect("ordinary file");
        let (loaded, read) =
            decode_turns(&[unread_body.finish(), window_file.bytes, last.bytes], &transcript::CEILINGS, &endpoints)
                .expect("only the current window is decoded");
        assert_eq!(loaded.expect("history").turns.as_ref(), &[window, ordinary]);
        assert_eq!(read, Some(Token::new(9)));
    }

    #[test]
    fn files_without_a_window_flag_are_malformed() {
        let (endpoints, first) = fixture();
        let file = save_turn(1, None, false, &first, &transcript::CEILINGS, &endpoints).expect("unflagged file");
        assert_eq!(decode_turns(&[file.bytes], &transcript::CEILINGS, &endpoints), Err(StoreError::Malformed));
    }
}
