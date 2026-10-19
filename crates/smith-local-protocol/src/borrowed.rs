//! Read-only login decoding (shell.md, section 7; protocol/hosts.md, 5.4).
//! Selective collection keeps access and account fields; refresh tokens are
//! scanned without being retained. No signature or login ownership is inferred.
use alloc::boxed::Box;
use skein_json::{Document, Kind, collector as json, tokenizer};
use skein_lib::{
    Env, Intake, Queue, Time, Wall, base64, bytes as octets,
    stream::{Down, Up},
};

/// The external tool's login document format, selected by the shell.
#[derive(Clone, Copy, Debug)]
pub enum LoginFormat {
    /// Codex's access token and account identifier in `tokens`.
    Codex,
}

/// An access-only record handed to the account driver, never kept or refreshed.
#[expect(missing_debug_implementations, reason = "the record contains a credential")]
pub struct Borrowed {
    pub record: skein_oauth::SavedToken,
    pub account_id: Box<[u8]>,
}

/// Why a bounded login document cannot supply a grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginRefusal {
    /// A document or selected value exceeded its configured bound.
    TooLarge,
    /// The login or token claims do not have their stated shape.
    Malformed,
    /// No access token was present.
    NoAccessToken,
    /// No account identifier was present.
    NoAccountId,
    /// The access token carries no expiry claim.
    NoExpiry,
}

const LOGIN: json::Filter = json::Filter {
    root: json::Keep::Into(&[json::Node {
        key: json::Key::Field(b"tokens"),
        keep: json::Keep::Into(&[
            json::Node { key: json::Key::Field(b"access_token"), keep: json::Keep::Text(json::Cap::new(0)) },
            json::Node { key: json::Key::Field(b"account_id"), keep: json::Keep::Text(json::Cap::new(0)) },
        ]),
    }]),
};
const CLAIMS: json::Filter =
    json::Filter { root: json::Keep::Into(&[json::Node { key: json::Key::Field(b"exp"), keep: json::Keep::Value }]) };

/// Both collected documents, their bounded walk, and the decoded claims and record.
#[must_use]
pub fn login_worst_case(bounds: &skein_oauth::Limits) -> Option<u64> {
    let limits = collector_limits(bounds);
    json::worst_case(&limits, &[bounds.token_bytes], &LOGIN)?
        .checked_add(json::worst_case(&limits, &[bounds.token_bytes], &CLAIMS)?)?
        .checked_add(Intake::worst_case(bounds.document_bytes)?)?
        .checked_add(Queue::<json::Event>::worst_case(1)?)?
        .checked_add(Queue::<Down>::worst_case(1)?)?
        .checked_add(u64::from(bounds.document_bytes).checked_mul(2)?)?
        .checked_add(u64::try_from(size_of::<Borrowed>()).ok()?)
}

/// Decode a tool's login without ever selecting its refresh token.
pub fn decode_login(format: LoginFormat, bytes: &[u8], limits: &skein_oauth::Limits) -> Result<Borrowed, LoginRefusal> {
    match format {
        LoginFormat::Codex => {}
    }
    let login = collect(bytes, LOGIN, limits)?;
    for index in 0..login.len() {
        if login.token(index).ok_or(LoginRefusal::Malformed)?.kind == Kind::Long {
            return Err(LoginRefusal::TooLarge);
        }
    }
    let access = field(&login, b"access_token", Kind::String).ok_or(LoginRefusal::NoAccessToken)?;
    let account = field(&login, b"account_id", Kind::String).ok_or(LoginRefusal::NoAccountId)?;
    if access.is_empty() || account.is_empty() {
        return Err(LoginRefusal::Malformed);
    }
    for byte in account {
        if !(0x21..=0x7e).contains(byte) {
            return Err(LoginRefusal::Malformed);
        }
    }
    for byte in access {
        if !(0x21..=0x7e).contains(byte) {
            return Err(LoginRefusal::Malformed);
        }
    }
    let first = octets::find(access, b".").ok_or(LoginRefusal::Malformed)?;
    let rest = access.get(first.checked_add(1).ok_or(LoginRefusal::TooLarge)?..).ok_or(LoginRefusal::Malformed)?;
    let second = octets::find(rest, b".").ok_or(LoginRefusal::Malformed)?;
    let payload = rest.get(..second).ok_or(LoginRefusal::Malformed)?;
    let signature = rest.get(second.checked_add(1).ok_or(LoginRefusal::TooLarge)?..).ok_or(LoginRefusal::Malformed)?;
    if first == 0 || signature.is_empty() || octets::find(signature, b".").is_some() {
        return Err(LoginRefusal::Malformed);
    }
    let Ok(decoded) = base64::decode(base64::Alphabet::Url, payload, limits.document_bytes) else {
        return Err(LoginRefusal::Malformed);
    };
    let claims = collect(&decoded, CLAIMS, limits)?;
    let expires = field(&claims, b"exp", Kind::Number).ok_or(LoginRefusal::NoExpiry)?;
    let mut seconds = 0_u64;
    for byte in expires {
        let digit = byte.checked_sub(b'0').ok_or(LoginRefusal::Malformed)?;
        if digit > 9 {
            return Err(LoginRefusal::Malformed);
        }
        seconds = seconds
            .checked_mul(10)
            .ok_or(LoginRefusal::TooLarge)?
            .checked_add(u64::from(digit))
            .ok_or(LoginRefusal::TooLarge)?;
    }
    let nanos = seconds.checked_mul(1_000_000_000).ok_or(LoginRefusal::TooLarge)?;
    Ok(Borrowed {
        record: skein_oauth::SavedToken {
            key: 0,
            generation: 1,
            access_token: Box::from(access),
            refresh_token: None,
            issued_client_id: None,
            granted_scopes: None,
            subject: None,
            id_token: None,
            earliest_refresh_at: None,
            metadata: None,
            expires_at: Wall::from_nanos(nanos),
        },
        account_id: Box::from(account),
    })
}

fn field<'a>(document: &'a Document, name: &[u8], kind: Kind) -> Option<&'a [u8]> {
    for index in 0..document.len() {
        let token = document.token(index)?;
        if token.kind == Kind::Key && document.text(token)? == name {
            let value = document.token(index.checked_add(1)?)?;
            return if value.kind == kind { document.text(value) } else { None };
        }
    }
    None
}

fn collect(bytes: &[u8], filter: json::Filter, bounds: &skein_oauth::Limits) -> Result<Document, LoginRefusal> {
    let Ok(length) = u32::try_from(bytes.len()) else {
        return Err(LoginRefusal::TooLarge);
    };
    if length > bounds.document_bytes {
        return Err(LoginRefusal::TooLarge);
    }
    let limits = collector_limits(bounds);
    let Ok(mut collector) = json::Collector::new(filter, &limits, &[bounds.token_bytes]) else {
        return Err(LoginRefusal::Malformed);
    };
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut above = Queue::with_capacity(1);
    let mut below = Queue::with_capacity(1);
    let mut intake = Intake::with_capacity(bounds.document_bytes);
    if intake.append(bytes).is_err() {
        return Err(LoginRefusal::TooLarge);
    }
    json::down(&mut collector, &env, json::Request::Collect, &mut above, &mut below);
    for _ in 0..u64::from(length)
        .checked_mul(4)
        .ok_or(LoginRefusal::TooLarge)?
        .checked_add(16)
        .ok_or(LoginRefusal::TooLarge)?
    {
        match above.pop() {
            Some(json::Event::Collected(document)) => return Ok(document),
            Some(json::Event::Failed(_) | json::Event::Closed) => return Err(LoginRefusal::Malformed),
            None => {}
        }
        match below.pop() {
            Some(Down::Demand { read, room: 0 }) => {
                let event = match intake.meet(read) {
                    Some(bytes) => Up::Bytes(bytes),
                    None => Up::End,
                };
                json::up(&mut collector, &env, event, &mut above, &mut below);
            }
            Some(Down::Demand { .. } | Down::Send(_) | Down::Finish) | None => return Err(LoginRefusal::Malformed),
        }
    }
    Err(LoginRefusal::Malformed)
}

fn collector_limits(bounds: &skein_oauth::Limits) -> json::Limits {
    json::Limits {
        tokenizer: tokenizer::Limits {
            depth: bounds.depth,
            string: bounds.string_bytes,
            number: 32,
            chunk: bounds.document_bytes,
            length: bounds.document_bytes,
        },
        tokens: bounds.tokens,
        text: bounds.document_bytes,
        skip: u64::from(bounds.document_bytes),
    }
}
