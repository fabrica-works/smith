use super::*;

#[test]
fn the_openai_registration_carries_smiths_terms() {
    let registration = openai("urn:uuid:for-tests", None, &Moved::default());
    assert_eq!(registration.client_id.as_ref(), b"dynamic_agent_client");
    assert_eq!(registration.redirect_uri.as_ref(), b"http://127.0.0.1:0/auth/callback");
    assert_eq!(registration.authorization_url.as_ref(), b"https://auth.openai.com/api/accounts/authorize");
    assert_eq!(registration.token_endpoint.as_ref(), b"https://auth.openai.com/api/accounts/oauth/token");
    assert_eq!(
        registration.scope.as_ref(),
        b"openid profile email offline_access resource.invoke chatgpt.tokens.use.direct"
    );
    let terms = registration.terms.unwrap();
    assert!(terms.issued_client_id);
    assert!(terms.varying_port);
    assert_eq!(terms.resource.as_deref(), Some(b"https://api.openai.com/v1".as_slice()));
    assert_eq!(terms.required_scopes.as_deref(), Some(b"chatgpt.tokens.use.direct".as_slice()));
    let openid = terms.openid.unwrap();
    assert_eq!(openid.issuer.as_ref(), b"https://auth.openai.com");
    assert_eq!(openid.key_set.as_deref(), Some(b"https://auth.openai.com/.well-known/jwks.json".as_slice()));
    assert_eq!(
        terms.revocation_endpoint.as_deref(),
        Some(b"https://auth.openai.com/api/accounts/oauth/revoke".as_slice())
    );
}

#[test]
fn the_hints_have_their_declared_lifetimes() {
    let registration = openai("urn:uuid:for-tests", Some(1455), &Moved::default());
    let terms = registration.terms.unwrap();
    assert_eq!(terms.parameters[0].name.as_ref(), b"agent_name_hint");
    assert_eq!(terms.parameters[0].value.as_ref(), b"smith");
    assert_eq!(terms.parameters[0].sent, oauth::Sent::NewRegistration);
    assert_eq!(terms.parameters[1].name.as_ref(), b"ext_agent_host_id");
    assert_eq!(terms.parameters[1].value.as_ref(), b"urn:uuid:for-tests");
    assert_eq!(terms.parameters[1].sent, oauth::Sent::EverySignIn);
}

#[test]
fn moved_addresses_keep_the_registration() {
    let moved = Moved {
        issuer: Some("http://127.0.0.1:444".into()),
        authorization_url: Some("http://127.0.0.1:444/auth".into()),
        token_url: Some("http://127.0.0.1:444/token".into()),
        keys_url: Some("http://127.0.0.1:444/keys".into()),
        revocation_url: Some("http://127.0.0.1:444/revoke".into()),
    };
    let registration = openai("host", Some(1455), &moved);
    assert_eq!(registration.client_id.as_ref(), b"dynamic_agent_client");
    assert_eq!(registration.authorization_url.as_ref(), b"http://127.0.0.1:444/auth");
    assert_eq!(registration.token_endpoint.as_ref(), b"http://127.0.0.1:444/token");
    assert_eq!(registration.redirect_uri.as_ref(), b"http://127.0.0.1:1455/auth/callback");
    let terms = registration.terms.unwrap();
    assert_eq!(terms.openid.unwrap().key_set.as_deref(), Some(b"http://127.0.0.1:444/keys".as_slice()));
    assert_eq!(terms.revocation_endpoint.as_deref(), Some(b"http://127.0.0.1:444/revoke".as_slice()));
}
