//! Reading status records (protocol/programs.md, section 3). The bounded
//! JSON walk retains only the two status fields, never any login file.

use skein_json::{Document, Kind, collector as json, tokenizer};
use skein_lib::{Env, Intake, Queue, Time, Wall, stream};

/// Status arguments sent by the host or agent, without argv[0].
pub const STATUS: [&[u8]; 2] = [b"auth", b"status"];

/// Sign-in arguments sent by a terminal caller, without argv[0].
pub const SIGN_IN: [&[u8]; 2] = [b"auth", b"login"];

/// Sign-out arguments sent by the host, without argv[0].
pub const SIGN_OUT: [&[u8]; 2] = [b"auth", b"logout"];

/// The method naming a person's subscription (protocol/programs.md, section 3).
pub const SUBSCRIPTION: &[u8] = b"claude.ai";

/// Maximum status standard output, in bytes.
pub const STATUS_BYTES: u32 = 4096;

/// A program's status record, read by the host or agent (programs.md, section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Exit 0, logged in through the person's subscription.
    Usable,
    /// Exit 1, or exit 0 with loggedIn false.
    NotSignedIn,
    /// Logged in through another printable, bounded authentication method.
    Otherwise(Method),
    /// The output exceeds its bound or does not describe a status.
    Unreadable,
}

/// An authMethod's 1 to 32 printable ASCII bytes, returned by a status.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Method {
    len: u8,
    bytes: [u8; 32],
}

impl Method {
    /// Refuse empty, oversized or nonprintable authentication methods.
    #[must_use]
    pub fn new(method: &[u8]) -> Option<Self> {
        if method.is_empty() || method.len() > 32 {
            return None;
        }
        for byte in method {
            if !(0x21..=0x7e).contains(byte) {
                return None;
            }
        }
        let mut bytes = [0; 32];
        for (target, byte) in bytes.iter_mut().zip(method) {
            *target = *byte;
        }
        Some(Self { len: u8::try_from(method.len()).ok()?, bytes })
    }

    /// The validated method, without its padding.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..usize::from(self.len)).expect("validated method length")
    }
}

impl core::fmt::Debug for Method {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("Method(\"")?;
        for byte in self.as_bytes() {
            let mut character = [0; 1];
            formatter.write_str(char::from(*byte).encode_utf8(&mut character))?;
        }
        formatter.write_str("\")")
    }
}

const FILTER: json::Filter = json::Filter {
    root: json::Keep::Into(&[
        json::Node { key: json::Key::Field(b"loggedIn"), keep: json::Keep::Value },
        json::Node { key: json::Key::Field(b"authMethod"), keep: json::Keep::Text(json::Cap::new(0)) },
    ]),
};

/// Read exit 0 or 1; the caller handles all other exits (programs.md, section 3).
#[must_use]
pub fn status(code: u8, stdout: &[u8]) -> Status {
    if code == 1 {
        return Status::NotSignedIn;
    }
    if code != 0 {
        return Status::Unreadable;
    }
    match read(stdout) {
        Some(status) => status,
        None => Status::Unreadable,
    }
}

fn read(bytes: &[u8]) -> Option<Status> {
    let document = collect(bytes)?;
    if document.token(0)?.kind != Kind::ObjectStart {
        return None;
    }
    let mut logged_in = None;
    let mut method = None;
    for index in 0..document.len() {
        let key = document.token(index)?;
        if key.kind != Kind::Key {
            continue;
        }
        let value = document.token(index.checked_add(1)?)?;
        match document.text(key)? {
            b"loggedIn" => {
                logged_in = match value.kind {
                    Kind::True => Some(true),
                    Kind::False => Some(false),
                    Kind::ArrayStart
                    | Kind::ArrayEnd
                    | Kind::ObjectStart
                    | Kind::ObjectEnd
                    | Kind::Key
                    | Kind::String
                    | Kind::Number
                    | Kind::Null
                    | Kind::Long => return None,
                };
            }
            b"authMethod" => {
                if value.kind != Kind::String {
                    return None;
                }
                method = Method::new(document.text(value)?);
            }
            _ => {}
        }
    }
    let method = method?;
    if !logged_in? {
        Some(Status::NotSignedIn)
    } else if method.as_bytes() == SUBSCRIPTION {
        Some(Status::Usable)
    } else {
        Some(Status::Otherwise(method))
    }
}

fn collect(bytes: &[u8]) -> Option<Document> {
    let length = u32::try_from(bytes.len()).ok()?;
    if length > STATUS_BYTES {
        return None;
    }
    let limits = limits();
    let mut collector = json::Collector::new(FILTER, &limits, &[32]).ok()?;
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut above = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(1);
    let mut intake = Intake::with_capacity(STATUS_BYTES);
    intake.append(bytes).ok()?;
    json::down(&mut collector, &env, json::Request::Collect, &mut above, &mut below);
    for _ in 0..u64::from(length).checked_mul(4)?.checked_add(16)? {
        match above.pop() {
            Some(json::Event::Collected(document)) => return Some(document),
            Some(json::Event::Failed(_) | json::Event::Closed) => return None,
            None => {}
        }
        match below.pop() {
            Some(stream::Down::Demand { read, room: 0 }) => {
                let event = match intake.meet(read) {
                    Some(bytes) => stream::Up::Bytes(bytes),
                    None => stream::Up::End,
                };
                json::up(&mut collector, &env, event, &mut above, &mut below);
            }
            Some(stream::Down::Demand { .. } | stream::Down::Send(_) | stream::Down::Finish) | None => return None,
        }
    }
    None
}

fn limits() -> json::Limits {
    json::Limits {
        tokenizer: tokenizer::Limits {
            depth: STATUS_BYTES,
            string: STATUS_BYTES,
            number: 32,
            chunk: STATUS_BYTES,
            length: STATUS_BYTES,
        },
        tokens: 16,
        text: STATUS_BYTES,
        skip: u64::from(STATUS_BYTES),
    }
}

pub(crate) fn worst_case() -> Option<u64> {
    json::worst_case(&limits(), &[32], &FILTER)?
        .checked_add(Intake::worst_case(STATUS_BYTES)?)?
        .checked_add(Queue::<json::Event>::worst_case(1)?)?
        .checked_add(Queue::<stream::Down>::worst_case(1)?)
}
