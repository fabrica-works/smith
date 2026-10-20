//! Scope-only inspection of saved sign-ins (shell.md, 7.1; protocol/hosts.md, 5.2).
//! Keeps no issuer or transport; the registration below only adapts supplied
//! scopes to skein's `missing_scopes` API and never starts an exchange.
use skein_oauth as oauth;

pub(super) fn missing(required: &[u8], record: &oauth::SavedToken) -> Box<[u8]> {
    let registration = oauth::Registration {
        terms: Some(oauth::Terms {
            parameters: Box::new([]),
            openid: None,
            issued_client_id: false,
            varying_port: false,
            resource: None,
            required_scopes: Some(required.into()),
            revocation_endpoint: None,
            sign_in_again: Box::new([]),
        }),
        authorization_url: Box::new([]),
        token_endpoint: Box::new([]),
        client_id: Box::new([]),
        redirect_uri: Box::new([]),
        // An omitted grant scope means the requested scopes were granted.
        scope: required.into(),
        wire: oauth::WireFormat::Form,
        client_secret: None,
        pkce_for_confidential: false,
        metadata_claim: None,
    };
    oauth::missing_scopes(&registration, record)
}
