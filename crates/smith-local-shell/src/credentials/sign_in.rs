//! Explicit public-client registration (shell.md, section 7).
//! Keys come from the account's settings; no provider registration is implied.
mod scopes;

use super::Account;
use crate::settings::{Origin, Problem, Refusal};
use skein_oauth as oauth;
use skein_oauth_accounts as accounts;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

/// The account's registration and the token URL's resolved transport.
#[expect(missing_debug_implementations, reason = "OAuth registrations may contain selected claims")]
pub struct Registered {
    pub registration: oauth::Registration,
    pub endpoint: accounts::Endpoint,
}

/// Read and validate an explicitly configured public client before driving it.
pub fn registration(account: &Account, codex: bool, workspace: &std::path::Path) -> Result<Registered, Refusal> {
    if account.fields.get("provider").is_some_and(|value| value.value.as_str() == Some("openai")) {
        return super::registration::configured(account, "", workspace);
    }
    let registration = configured(account, codex)?;
    let endpoint = destination(account, text(account, "token_url")?, workspace)?;
    Ok(Registered { registration, endpoint })
}

fn configured(account: &Account, codex: bool) -> Result<oauth::Registration, Refusal> {
    let registration = oauth::Registration {
        terms: None,
        client_id: text(account, "client_id")?.as_bytes().into(),
        authorization_url: text(account, "authorization_url")?.as_bytes().into(),
        token_endpoint: text(account, "token_url")?.as_bytes().into(),
        redirect_uri: text(account, "redirect_uri")?.as_bytes().into(),
        scope: text(account, "scope")?.as_bytes().into(),
        wire: match text(account, "token_format")? {
            "json" => oauth::WireFormat::Json,
            "form" => oauth::WireFormat::Form,
            _ => return Err(refuse(account, "token_format", "expected json or form")),
        },
        client_secret: None,
        pkce_for_confidential: false,
        metadata_claim: codex.then(|| oauth::ClaimSelector {
            object: b"https://api.openai.com/auth".as_slice().into(),
            field: b"chatgpt_account_id".as_slice().into(),
        }),
    };
    issuer(text(account, "authorization_url")?).map_err(|why| refuse(account, "authorization_url", why))?;
    issuer(text(account, "token_url")?).map_err(|why| refuse(account, "token_url", why))?;
    oauth::redirect_address(&registration.redirect_uri)
        .map_err(|_| refuse(account, "redirect_uri", "expected loopback, an explicit port and path"))?;
    oauth::validate_registration(&registration, &super::product::client_limits(), true)
        .map_err(|_| refuse(account, "client_id", "registration exceeds the client's bounds or syntax"))?;
    Ok(registration)
}

fn text<'a>(account: &'a Account, field: &str) -> Result<&'a str, Refusal> {
    account
        .fields
        .get(field)
        .and_then(|value| value.value.as_str())
        .ok_or_else(|| refuse(account, field, "required registration value is absent"))
}

pub(super) fn refuse(account: &Account, field: &str, problem: &str) -> Refusal {
    let origin = account
        .fields
        .get(field)
        .or_else(|| account.fields.get("source"))
        .map_or(Origin::BuiltIn, |value| value.origin.clone());
    Refusal {
        origin,
        key: format!("accounts.{}.{}", crate::settings::name(&account.name), field),
        problem: Problem::Input(problem.into()),
    }
}

pub(crate) fn issuer(uri: &str) -> Result<(&str, bool), &'static str> {
    let (rest, plaintext) = if let Some(rest) = uri.strip_prefix("https://") {
        (rest, false)
    } else if let Some(rest) = uri.strip_prefix("http://") {
        (rest, true)
    } else {
        return Err("expected https, or http on loopback");
    };
    let authority = rest.split(['/', '?']).next().ok_or("missing issuer authority")?;
    if authority.is_empty() || authority.contains('@') || uri.contains(['\r', '\n', '#']) {
        return Err("invalid issuer URL");
    }
    if plaintext && authority != "localhost" && !authority.starts_with("localhost:") {
        let host = authority
            .parse::<SocketAddr>()
            .map(|address| address.ip())
            .or_else(|_| authority.trim_matches(['[', ']']).parse::<IpAddr>())
            .map_err(|_| "http must name a loopback address")?;
        if !host.is_loopback() {
            return Err("http must name a loopback address");
        }
    }
    Ok((authority, plaintext))
}

pub(super) fn destination(
    account: &Account,
    uri: &str,
    workspace: &std::path::Path,
) -> Result<accounts::Endpoint, Refusal> {
    let (authority, plaintext) = issuer(uri).map_err(|why| refuse(account, "token_url", why))?;
    let address = if let Ok(address) = authority.parse::<SocketAddr>() {
        address
    } else {
        let address = if authority.contains(':') && !authority.ends_with(']') {
            authority.to_owned()
        } else {
            format!("{authority}:{}", if plaintext { 80 } else { 443 })
        };
        address
            .to_socket_addrs()
            .map_err(|_| refuse(account, "token_url", "issuer name did not resolve"))?
            .next()
            .ok_or_else(|| refuse(account, "token_url", "issuer name resolved to no address"))?
    };
    if address.port() == 0 || (plaintext && !address.ip().is_loopback()) {
        return Err(refuse(account, "token_url", "invalid token endpoint address"));
    }
    let transport = if plaintext {
        accounts::Transport::Plaintext
    } else {
        let host = if authority.starts_with('[') {
            authority.split(']').next().expect("bracketed authority").trim_start_matches('[')
        } else {
            authority.split(':').next().expect("issuer authority")
        };
        let server_name = skein_tls::Name::new(host).ok_or_else(|| refuse(account, "token_url", "invalid TLS name"))?;
        accounts::Transport::Tls { server_name, trust: trust(account, workspace)? }
    };
    Ok(accounts::Endpoint { address: skein_io::kernel::Addr::from(address), transport })
}

fn trust(account: &Account, workspace: &std::path::Path) -> Result<skein_tls::Config, Refusal> {
    let mut sources = Vec::new();
    for format in ["der", "pem"] {
        if let Some(paths) = account.fields.get(&format!("trust.{format}")).and_then(|value| value.value.as_array()) {
            for path in paths {
                let path = workspace.join(path.as_str().expect("decoded trust paths"));
                sources.push(if format == "der" {
                    skein_shell::Trust::Der(path)
                } else {
                    skein_shell::Trust::Machine { ssl_cert_file: Some(path) }
                });
            }
        }
    }
    if sources.is_empty() {
        sources.push(skein_shell::Trust::Machine { ssl_cert_file: None });
    }
    smith_agent_shell::config::configured_trust(&sources).map_err(|why| refuse(account, "trust", &why))
}

pub(super) fn document_limits() -> oauth::Limits {
    let mut limits = crate::local_host::token_limits();
    limits.issued_client_bytes = 1024;
    limits.subject_bytes = 1024;
    limits
}

pub(super) fn record(file: &std::path::Path) -> Result<Option<oauth::SavedToken>, String> {
    match skein_shell::read_private_file(file, document_limits().record_bytes, skein_shell::effective_user()) {
        Ok(bytes) => oauth::decode_record(bytes.expose(), &document_limits())
            .map(Some)
            .map_err(|_| "unreadable sign-in record".into()),
        Err(skein_shell::Unread::Absent) => Ok(None),
        Err(_) => Err("unreadable sign-in record".into()),
    }
}

pub(super) fn inspect(
    file: &std::path::Path,
    wall: skein_lib::Wall,
    required_scopes: Option<&[u8]>,
) -> super::Credential {
    if !file.is_absolute() {
        return super::Credential::NotSignedIn;
    }
    match skein_shell::read_private_file(file, document_limits().record_bytes, skein_shell::effective_user()) {
        Ok(bytes) => match oauth::decode_record(bytes.expose(), &document_limits()).ok() {
            Some(record) => {
                if let Some(required) = required_scopes {
                    let missing = scopes::missing(required, &record);
                    if !missing.is_empty() {
                        return super::Credential::WithoutScope {
                            scope: String::from_utf8_lossy(&missing).into_owned(),
                        };
                    }
                }
                if record.remaining(wall) > skein_lib::Duration::ZERO || record.refresh_token.is_some() {
                    super::Credential::Usable { until: Some(record.expires_at.as_nanos()) }
                } else {
                    super::Credential::SignInAgain
                }
            }
            None => super::Credential::NotSignedIn,
        },
        Err(skein_shell::Unread::Absent) => super::Credential::NotSignedIn,
        Err(_) => super::Credential::Unreadable { file: file.into() },
    }
}

pub(super) fn private_source(
    number: u32,
    file: &std::path::Path,
    registered: Registered,
    interactive: bool,
) -> Result<smith_local_service::Source, String> {
    private_source_at(number, file, registered, interactive, None)
}

pub(super) fn private_source_at(
    number: u32,
    file: &std::path::Path,
    registered: Registered,
    interactive: bool,
    inherited: Option<skein_io::kernel::Fd>,
) -> Result<smith_local_service::Source, String> {
    use std::os::unix::fs::DirBuilderExt;
    if !file.is_absolute() || file.as_os_str().as_encoded_bytes().len() > 4095 {
        return Err("HOME or an absolute XDG_STATE_HOME is required for bounded private tokens".into());
    }
    let directory = file.parent().ok_or("private record has no directory")?;
    let base = directory.parent().ok_or("private record has no base")?;
    let root = if let Some(root) = inherited {
        root
    } else {
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(base).map_err(|why| why.to_string())?;
        skein_shell::open_root(base).map_err(|why| format!("private token root: {why:?}"))?
    };
    Ok(smith_local_service::Source::SignIn {
        account: number,
        root,
        user: skein_shell::effective_user(),
        directory: directory.file_name().ok_or("private directory has no name")?.as_encoded_bytes().into(),
        file: file.file_name().ok_or("private record has no name")?.as_encoded_bytes().into(),
        registration: registered.registration,
        endpoint: registered.endpoint,
        account_id: Box::new([]),
        interactive,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Layer, Sourced};
    fn account() -> Account {
        let fields = [
            ("source", "sign_in"),
            ("client_id", "client"),
            ("authorization_url", "http://127.0.0.1:444/authorize"),
            ("token_url", "http://127.0.0.1:444/token"),
            ("redirect_uri", "http://localhost:2345/callback"),
            ("scope", "read"),
            ("token_format", "form"),
        ]
        .into_iter()
        .map(|(key, value)| {
            (
                key.into(),
                Sourced {
                    value: toml::Value::String(value.into()),
                    origin: Origin::File { layer: Layer::User, path: "fixture.toml".into(), line: 8 },
                },
            )
        })
        .collect();
        Account { name: "fixture".into(), number: 0, fields }
    }
    #[test]
    fn every_missing_registration_key_names_its_key_and_file() {
        for key in ["client_id", "authorization_url", "token_url", "redirect_uri", "scope", "token_format"] {
            let mut account = account();
            account.fields.remove(key);
            let refusal = registration(&account, false, std::path::Path::new("/unused")).err().expect("missing key");
            assert_eq!(refusal.key, format!("accounts.fixture.{key}"));
            assert!(refusal.line().contains("fixture.toml:8"));
        }
    }
    #[test]
    fn a_public_registration_retains_the_exact_localhost_redirect_and_has_no_secret() {
        let registered = registration(&account(), false, std::path::Path::new("/unused")).expect("public fixture");
        assert_eq!(registered.registration.redirect_uri.as_ref(), b"http://localhost:2345/callback");
        assert!(registered.registration.client_secret.is_none());
        assert!(matches!(registered.endpoint.transport, accounts::Transport::Plaintext));
    }
    #[test]
    fn an_http_issuer_outside_loopback_is_refused() {
        let mut account = account();
        account.fields.get_mut("authorization_url").expect("fixture").value =
            toml::Value::String("http://203.0.113.1/authorize".into());
        assert_eq!(
            registration(&account, false, std::path::Path::new("/unused")).err().expect("remote http").key,
            "accounts.fixture.authorization_url"
        );
    }
}
