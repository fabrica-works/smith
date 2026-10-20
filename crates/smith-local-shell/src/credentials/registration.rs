//! Smith's registration (shell.md, 6.2, 7.1). Addresses are constants from
//! <https://auth.openai.com/.well-known/openid-configuration> (2026-10-20),
//! never discovery fetched at runtime. Only the addresses may move in tests.
use super::{Account, Registered, sign_in};
use crate::settings::Refusal;
use skein_oauth as oauth;

pub(super) const DISCONNECT: &str = "chatgpt.com/settings";
pub(super) const NOTICE: &str = "You're using your ChatGPT plan. Eligible requests from smith use your plan's included usage; manage it at chatgpt.com/settings/usage.";

/// Validated overrides for the preset's issuer addresses, never its registration.
#[derive(Default)]
pub(super) struct Moved {
    pub issuer: Option<String>,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub keys_url: Option<String>,
    pub revocation_url: Option<String>,
}

impl Moved {
    pub(super) fn of(account: &Account) -> Result<Self, Refusal> {
        let get = |field: &str| -> Result<Option<String>, Refusal> {
            account
                .fields
                .get(field)
                .map(|value| {
                    let uri = value.value.as_str().expect("schema URL");
                    sign_in::issuer(uri).map_err(|why| sign_in::refuse(account, field, why))?;
                    Ok(uri.to_owned())
                })
                .transpose()
        };
        Ok(Self {
            issuer: get("issuer")?,
            authorization_url: get("authorization_url")?,
            token_url: get("token_url")?,
            keys_url: get("keys_url")?,
            revocation_url: get("revocation_url")?,
        })
    }
}

pub(super) fn openai(host_id: &str, redirect_port: Option<u16>, moved: &Moved) -> oauth::Registration {
    let address =
        |uri: &Option<String>, default: &str| -> Box<[u8]> { uri.as_deref().unwrap_or(default).as_bytes().into() };
    oauth::Registration {
        terms: Some(oauth::Terms {
            parameters: Box::new([
                oauth::Parameter {
                    name: b"agent_name_hint".as_slice().into(),
                    value: b"smith".as_slice().into(),
                    sent: oauth::Sent::NewRegistration,
                },
                oauth::Parameter {
                    name: b"ext_agent_host_id".as_slice().into(),
                    value: host_id.as_bytes().into(),
                    sent: oauth::Sent::EverySignIn,
                },
            ]),
            openid: Some(oauth::OpenId {
                issuer: address(&moved.issuer, "https://auth.openai.com"),
                key_set: Some(address(&moved.keys_url, "https://auth.openai.com/.well-known/jwks.json")),
            }),
            issued_client_id: true,
            varying_port: true,
            resource: Some(b"https://api.openai.com/v1".as_slice().into()),
            required_scopes: Some(b"chatgpt.tokens.use.direct".as_slice().into()),
            revocation_endpoint: Some(address(
                &moved.revocation_url,
                "https://auth.openai.com/api/accounts/oauth/revoke",
            )),
            sign_in_again: Box::new([]),
        }),
        client_id: b"dynamic_agent_client".as_slice().into(),
        authorization_url: address(&moved.authorization_url, "https://auth.openai.com/api/accounts/authorize"),
        token_endpoint: address(&moved.token_url, "https://auth.openai.com/api/accounts/oauth/token"),
        redirect_uri: format!("http://127.0.0.1:{}/auth/callback", redirect_port.unwrap_or(0)).into_bytes().into(),
        scope: b"openid profile email offline_access resource.invoke chatgpt.tokens.use.direct".as_slice().into(),
        wire: oauth::WireFormat::Form,
        client_secret: None,
        pkce_for_confidential: false,
        metadata_claim: None,
    }
}

pub(super) fn configured(account: &Account, host_id: &str, workspace: &std::path::Path) -> Result<Registered, Refusal> {
    let port = account
        .fields
        .get("redirect_port")
        .and_then(|value| value.value.as_integer())
        .and_then(|port| u16::try_from(port).ok());
    let registration = openai(host_id, port, &Moved::of(account)?);
    oauth::validate_registration(&registration, &super::product::client_limits(), true)
        .map_err(|_| sign_in::refuse(account, "source", "registration exceeds the client's bounds or syntax"))?;
    let uri = std::str::from_utf8(&registration.token_endpoint).expect("configured URL");
    let endpoint = sign_in::destination(account, uri, workspace)?;
    Ok(Registered { registration, endpoint })
}

#[cfg(test)]
mod tests;
