use super::*;
use crate::CredentialRequest;
use skein_lib::Duration;

fn fixture(name: &str) -> Box<[u8]> {
    let path = std::path::Path::new("../smith-local-protocol/src/login-fixtures").join(name);
    std::fs::read(path).expect("synthetic login fixture").into_boxed_slice()
}

fn limits() -> oauth::Limits {
    oauth::Limits {
        accounts: 2,
        file_stall: Duration::from_secs(30),
        lock_wait: Duration::from_secs(1),
        exchanges: 1,
        listeners: 0,
        server: skein_http::server::Limits { head: 1024, headers: 8, body: 1024, read: 256, response: 1024, send: 256 },
        http: skein_http::client::Limits { request: 1024, head: 1024, headers: 8, read: 256, send: 256 },
        tls: skein_tls::client::Limits { read: 1024, send: 1024, records: 131_072 },
        io: skein_io::Limits {
            sockets: 1,
            names: 0,
            refusals: 1,
            intake: 32_768,
            receive: 1024,
            output: 32_768,
            sends: 1,
            accepts: 1,
            backlog: 1,
            close_timeout: Duration::from_secs(1),
            retry: Duration::from_millis(1),
        },
        refresh_lead: Duration::from_secs(10),
        client: skein_oauth::ClientLimits {
            parameter_bytes: 0,
            resource_bytes: 0,
            nonce_bytes: 0,
            metadata_bytes: 0,
            key_set_bytes: 0,
            keys: 0,
            modulus_bits: 2048,
            skew: Duration::ZERO,
            document: skein_oauth::Limits {
                issued_client_bytes: 0,
                granted_bytes: 64,
                subject_bytes: 0,
                id_token_bytes: 256,
                document_bytes: 1024,
                string_bytes: 512,
                token_bytes: 256,
                client_bytes: 64,
                detail_bytes: 64,
                record_bytes: 1024,
                depth: 8,
                tokens: 64,
            },
            uri_bytes: 256,
            scope_bytes: 64,
            state_bytes: 64,
            code_bytes: 64,
            url_bytes: 1024,
            request_bytes: 1024,
            sign_in_time: Duration::from_secs(120),
            request_time: Duration::from_secs(20),
            backoff_base: Duration::from_secs(1),
            backoff_ceiling: Duration::from_secs(8),
            max_attempts: 3,
        },
    }
}

fn wall() -> Wall {
    Wall::from_nanos(100_000_000_000)
}
fn source() -> Source {
    Source::Borrow {
        account: 7,
        root: kernel::Fd::new(70),
        path: b"auth.json".as_slice().into(),
        format: LoginFormat::Codex,
    }
}
fn accounts(source: Source) -> Accounts {
    let mut sources = List::with_capacity(1);
    assert!(sources.push(source).is_ok());
    Accounts::new(sources, limits(), [7; 32]).expect("handed-in owner")
}
fn answer(mut submit: kernel::Submit, bytes: &[u8], closed: &mut List<kernel::Fd>) -> kernel::Complete {
    let result = if let kernel::Op::Open { path, how, .. } = &submit.kind {
        assert_eq!(path.as_ref(), b"auth.json");
        assert_eq!(*how, kernel::OpenHow::ReadNoFollow);
        kernel::Done::Fd(kernel::Fd::new(71))
    } else if let kernel::Op::Stat { .. } = &submit.kind {
        kernel::Done::Stat(kernel::Stat {
            kind: kernel::Kind::File,
            size: u64::try_from(bytes.len()).expect("fixture length"),
            mode: 0o600,
            owner: 7,
            links: 1,
        })
    } else if let kernel::Op::Read { buf, at, .. } = &mut submit.kind {
        let start = usize::try_from(*at).expect("fixture offset");
        let count = bytes.len().saturating_sub(start).min(buf.len());
        for (index, value) in buf.iter_mut().enumerate().take(count) {
            *value = bytes[start.checked_add(index).expect("fixture position")];
        }
        kernel::Done::Count(u32::try_from(count).expect("fixture count"))
    } else if let kernel::Op::Close { fd } = &submit.kind {
        closed.push(*fd).expect("fixture closes");
        kernel::Done::Nothing
    } else {
        panic!("only read-only file operations");
    };
    kernel::Complete { op: submit.op, kind: submit.kind, result: Ok(result) }
}
fn settle(
    owner: &mut Accounts,
    bytes: &[u8],
    now: Time,
    out: &mut Queue<CredentialRequest>,
    closed: &mut List<kernel::Fd>,
) {
    for _ in 0..32_u32 {
        owner.advance(now, wall(), out);
        if let Some(submit) = owner.submissions().pop() {
            owner.complete(answer(submit, bytes, closed));
        }
        if !owner.work_pending(now) && owner.files.takes() {
            return;
        }
    }
    panic!("bounded account transitions settle");
}
fn ready(
    out: &mut Queue<CredentialRequest>,
    generation: u64,
    seconds: u64,
    value: &[u8],
    expected: smith_host_domain::Form,
) {
    let Some(CredentialRequest::Ready { event: local::Event::Credential { grant, form }, value: token }) = out.pop()
    else {
        panic!("one ready grant")
    };
    assert_eq!(form, expected);
    assert_eq!(grant.name, smith_domain::GrantName { account: 7, generation });
    assert_eq!(grant.valid, Duration::from_secs(seconds));
    assert_eq!(token.as_ref(), value);
}

#[test]
fn a_borrowed_login_is_loaded_at_startup_and_its_file_is_never_written() {
    let mut owner = accounts(source());
    let mut out = Queue::with_capacity(ACCOUNTS_MAX_OUT);
    let mut closed = List::with_capacity(8);
    settle(&mut owner, &fixture("valid.json"), Time::ZERO, &mut out, &mut closed);
    assert!(out.is_empty());
    assert!(owner.request(7));
    settle(&mut owner, &fixture("valid.json"), Time::ZERO, &mut out, &mut closed);
    ready(&mut out, 2, 20, b"header.eyJleHAiOjEyMH0.signature", smith_host_domain::Form::Withdrawn);
    assert_eq!(owner.account_id(7), Some(b"acc".as_slice()));
    owner.close(smith_agent_service::Close::Graceful, Time::ZERO, wall());
    settle(&mut owner, &fixture("valid.json"), Time::ZERO, &mut out, &mut closed);
    assert!(owner.closed());
    let Some(CredentialRequest::Closed) = out.pop() else {
        panic!("one close terminal");
    };
    owner.advance(Time::ZERO, wall(), &mut out);
    assert!(out.is_empty());
    assert_eq!(closed.as_slice(), &[kernel::Fd::new(71), kernel::Fd::new(71), kernel::Fd::new(70)]);
}

#[test]
fn a_login_changed_by_its_program_is_loaded_for_the_next_grant() {
    let mut owner = accounts(source());
    let mut out = Queue::with_capacity(ACCOUNTS_MAX_OUT);
    let mut closed = List::with_capacity(8);
    settle(&mut owner, &fixture("expired.json"), Time::ZERO, &mut out, &mut closed);
    assert!(owner.request(7));
    settle(&mut owner, &fixture("valid.json"), Time::ZERO, &mut out, &mut closed);
    ready(&mut out, 2, 20, b"header.eyJleHAiOjEyMH0.signature", smith_host_domain::Form::Withdrawn);
    assert!(out.is_empty());
}

#[test]
fn an_env_account_lends_the_handed_in_value_without_a_file_operation() {
    for (identifier, form) in [
        (b"acc".as_slice(), smith_host_domain::Form::Withdrawn),
        (b"".as_slice(), smith_host_domain::Form::Credential(smith_host_domain::Scheme::Bearer)),
    ] {
        let mut borrowed = decode_login(LoginFormat::Codex, &fixture("valid.json"), &limits().client.document)
            .expect("fixture record");
        borrowed.account_id = Box::from(identifier);
        let mut owner = accounts(Source::Env { account: 7, borrowed });
        let mut out = Queue::with_capacity(ACCOUNTS_MAX_OUT);
        let mut closed = List::with_capacity(8);
        assert!(owner.request(7));
        settle(&mut owner, b"", Time::ZERO, &mut out, &mut closed);
        ready(&mut out, 1, 20, b"header.eyJleHAiOjEyMH0.signature", form);
        assert!(owner.submissions().is_empty());
        assert!(closed.is_empty());
    }
}

#[test]
fn an_expired_borrowed_record_is_not_lent() {
    let mut owner = accounts(source());
    let mut out = Queue::with_capacity(12);
    let mut closed = List::with_capacity(8);
    assert!(owner.request(7));
    settle(&mut owner, &fixture("expired.json"), Time::ZERO, &mut out, &mut closed);
    let Some(CredentialRequest::Failed { event: local::Event::NoCredential { account, reason } }) = out.pop() else {
        panic!("expired grant ends without a credential")
    };
    assert_eq!(account, 7);
    assert_eq!(reason, local::CredentialFailure::BorrowedExpired { at: Wall::from_nanos(90_000_000_000) });
    assert!(out.is_empty());
}

#[test]
fn a_borrowed_grant_announces_its_lead_and_ends_at_expiry() {
    let mut owner = accounts(source());
    let mut out = Queue::with_capacity(ACCOUNTS_MAX_OUT);
    let mut closed = List::with_capacity(8);
    assert!(owner.request(7));
    settle(&mut owner, &fixture("valid.json"), Time::ZERO, &mut out, &mut closed);
    ready(&mut out, 1, 20, b"header.eyJleHAiOjEyMH0.signature", smith_host_domain::Form::Withdrawn);
    owner.advance(Time::from_nanos(10_000_000_000), wall(), &mut out);
    let Some(CredentialRequest::Notice { account: 7, reason: local::CredentialFailure::BorrowedExpiring { .. } }) =
        out.pop()
    else {
        panic!("expiry lead notice");
    };
    owner.advance(Time::from_nanos(20_000_000_000), wall(), &mut out);
    let Some(CredentialRequest::Failed {
        event: local::Event::NoCredential { account: 7, reason: local::CredentialFailure::BorrowedExpired { .. } },
    }) = out.pop()
    else {
        panic!("expiry ends the held grant");
    };
}

#[test]
fn releasing_a_finished_runs_grants_prevents_a_later_grant_failure() {
    let mut owner = accounts(source());
    let mut out = Queue::with_capacity(ACCOUNTS_MAX_OUT);
    let mut closed = List::with_capacity(8);
    assert!(owner.request(7));
    settle(&mut owner, &fixture("valid.json"), Time::ZERO, &mut out, &mut closed);
    ready(&mut out, 1, 20, b"header.eyJleHAiOjEyMH0.signature", smith_host_domain::Form::Withdrawn);
    owner.release_all(Time::ZERO, wall());
    owner.advance(Time::from_nanos(20_000_000_000), wall(), &mut out);
    assert!(out.is_empty());
    assert!(owner.next_deadline().is_none());
}

#[test]
fn a_handed_in_refresh_token_is_refused_before_the_owner_starts() {
    let mut borrowed =
        decode_login(LoginFormat::Codex, &fixture("valid.json"), &limits().client.document).expect("fixture");
    borrowed.record.refresh_token = Some(b"refresh".as_slice().into());
    let mut sources = List::with_capacity(1);
    assert!(sources.push(Source::Env { account: 7, borrowed }).is_ok());
    assert!(Accounts::new(sources, limits(), [7; 32]).is_err());
}

#[test]
fn zero_file_stall_is_refused_before_a_file_owner_is_constructed() {
    let mut sources = List::with_capacity(1);
    assert!(sources.push(source()).is_ok());
    let mut bounds = limits();
    bounds.file_stall = Duration::ZERO;
    assert!(Accounts::new(sources, bounds, [7; 32]).is_err());
}

#[test]
fn closing_cancels_a_login_read_before_closing_its_root() {
    let mut owner = accounts(source());
    let mut out = Queue::with_capacity(12);
    let mut closed = List::with_capacity(8);
    assert!(owner.request(7));
    owner.advance(Time::ZERO, wall(), &mut out);
    let opened = owner.submissions().pop().expect("login open");
    owner.close(smith_agent_service::Close::Graceful, Time::ZERO, wall());
    let cancel = owner.submissions().pop().expect("read cancellation");
    assert_eq!(cancel.kind, kernel::Op::Cancel { target: opened.op });
    owner.complete(kernel::Complete { op: cancel.op, kind: cancel.kind, result: Ok(kernel::Done::Nothing) });
    owner.complete(kernel::Complete { op: opened.op, kind: opened.kind, result: Err(kernel::Error::Cancelled) });
    settle(&mut owner, &fixture("valid.json"), Time::ZERO, &mut out, &mut closed);
    let Some(CredentialRequest::Failed {
        event: local::Event::NoCredential { account: 7, reason: local::CredentialFailure::Missing },
    }) = out.pop()
    else {
        panic!("cancelled grant request ends once");
    };
    let Some(CredentialRequest::Closed) = out.pop() else {
        panic!("root close ends the owner");
    };
    assert!(out.is_empty());
    assert!(owner.closed());
    assert_eq!(closed.as_slice(), &[kernel::Fd::new(70)]);
}

mod private;
