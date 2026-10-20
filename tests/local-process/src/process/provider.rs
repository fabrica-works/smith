//! Smith's own registration against the shared fake issuer (shell.md, 7.1).
use super::*;

pub(super) fn issuer(port: u16, subject: &[u8], plan: bool, again: bool, revocation: Option<bool>) -> Issuer {
    let host = smith_agent_process_world::fake::loopback();
    let origin = format!("http://{host}:{port}");
    let mut document = token_limits();
    document.issued_client_bytes = 1024;
    document.subject_bytes = 1024;
    let limits = fake_oauth::Limits {
        document,
        uri_bytes: 8192,
        request_bytes: 16384,
        codes: 4,
        carried: 16,
        rotations: 4,
        plans: 4,
    };
    let response = skein_fake_peers::oauth::least_response(&limits).unwrap();
    let mut issuer = Issuer::new(
        (host, port).into(),
        skein_fake_peers::Transport::Plaintext,
        smith_agent_process_world::fake::limits(),
        fake_oauth::Config {
            authorization_url: format!("{origin}/authorize").into_bytes().into(),
            token_endpoint: format!("{origin}/token").into_bytes().into(),
            client_id: if again {
                b"oaiapp_for_work".as_slice().into()
            } else {
                b"dynamic_agent_client".as_slice().into()
            },
            client_secret: None,
            terms: Some(fake_oauth::Terms {
                issued: Box::new([b"oaiapp_for_work".as_slice().into()]),
                omit_issued: false,
                revocation_endpoint: Some(format!("{origin}/revoke").into_bytes().into()),
                varying_port: true,
                openid: Some(fake_oauth::OpenId {
                    issuer: origin.clone().into_bytes().into(),
                    key_set_url: format!("{origin}/keys").into_bytes().into(),
                    subject: subject.into(),
                    lifetime: Duration::from_secs(3600),
                    header_kid: true,
                    fault: None,
                }),
            }),
            redirect_uri: b"http://127.0.0.1:0/auth/callback".as_slice().into(),
            refresh_token: b"refresh-for-work".as_slice().into(),
        },
        limits,
        skein_http::server::Limits { head: 16384, headers: 32, body: 16384, read: 1024, response, send: 1024 },
    )
    .expect("provider issuer");
    let (status, body) = match revocation {
        Some(confirmed) => (if confirmed { 200 } else { 500 }, fake_oauth::Body::Raw(Box::new([]))),
        None => (
            200,
            fake_oauth::Body::Token(skein_oauth::TokenResponse {
                scope: Some(if plan {
                    b"openid profile email offline_access resource.invoke chatgpt.tokens.use.direct".as_slice().into()
                } else {
                    b"openid profile email".as_slice().into()
                }),
                id_token: None,
                earliest_refresh_at: None,
                access_token: b"access-for-work".as_slice().into(),
                refresh_token: Some(b"refresh-for-work".as_slice().into()),
                expires_in: 3600,
            }),
        ),
    };
    issuer
        .queue(fake_oauth::Plan { status, body, delay: Duration::ZERO, retry_after: Duration::ZERO })
        .expect("provider plan");
    if revocation.is_none() {
        issuer
            .queue(fake_oauth::Plan {
                status: 200,
                body: fake_oauth::Body::Raw(Box::new([])),
                delay: Duration::ZERO,
                retry_after: Duration::ZERO,
            })
            .expect("rollback revocation plan");
    }
    issuer
}
