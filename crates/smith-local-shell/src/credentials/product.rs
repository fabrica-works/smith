//! Startup source descriptors and the product's exact credential notice names.
//! Values are handed to the driver once; locations contain no credential bytes.
use super::{Credential, Refusal, Source, environment, inspect, source};
use crate::settings::resolve::Resolved;
use skein_lib::{Duration, List, Queue, Wall};
use smith_local_service as service;

/// Names used by the shell when its driver reports a borrowed login's expiry.
#[derive(Debug)]
pub struct Location {
    pub account: u32,
    pub name: String,
    pub source: Source,
}

/// Startup's selected handed-in sources and bounded notice metadata.
pub struct Prepared {
    pub sources: List<service::Source>,
    pub locations: List<Location>,
    pub limits: skein_oauth_accounts::Limits,
}

impl Prepared {
    /// Price the driver, its translated outputs and source-specific rendering.
    #[must_use]
    pub fn worst_case(&self) -> Option<u64> {
        let mut bytes = service::accounts_worst_case(&self.limits)?
            .checked_add(List::<Location>::worst_case(self.locations.capacity())?)?
            .checked_add(Queue::<smith_local_service::CredentialRequest>::worst_case(service::ACCOUNTS_MAX_OUT)?)?
            .checked_add(
                u64::from(self.limits.client.document.token_bytes).checked_mul(u64::from(service::ACCOUNTS_MAX_OUT))?,
            )?;
        for location in &self.locations {
            let source = match &location.source {
                Source::Borrow { file, program, .. } => {
                    file.as_os_str().as_encoded_bytes().len().checked_add(program.len())?
                }
                Source::Env { variable, .. } => variable.len(),
                Source::SignIn { file } => file.as_os_str().as_encoded_bytes().len(),
            };
            let length = location.name.len().checked_add(source)?.checked_add(256)?;
            bytes = bytes.checked_add(u64::try_from(length).ok()?.checked_mul(4)?)?;
        }
        Some(bytes)
    }
}

pub(crate) fn codex(resolved: &Resolved, account: u32) -> bool {
    for model in &resolved.local.models {
        if model.account == Some(account) {
            let endpoint = resolved
                .agent
                .service
                .llm_endpoints
                .resolve(smith_domain::llm::Endpoint(model.endpoint.0))
                .expect("resolved endpoint");
            match endpoint.provider {
                skein_llm::Provider::OpenAiCodex => return true,
                skein_llm::Provider::OpenAi | skein_llm::Provider::Anthropic => {}
            }
        }
    }
    false
}

/// Inspect selected accounts, report startup warnings once, and open read-only roots.
pub fn prepare(resolved: &Resolved, wall: Wall, interactive: bool) -> Result<Prepared, Refusal> {
    for warning in crate::check::startup_warnings(&resolved.settings).map_err(Refusal::Configuration)? {
        eprintln!("smith: {}", crate::check::warnings::line(&warning));
    }
    for warning in super::unused_warnings(resolved, wall) {
        eprintln!("smith: {}", crate::check::warnings::line(&warning));
    }
    let count = u32::try_from(resolved.accounts.len()).map_err(|_| {
        Refusal::Configuration(crate::settings::Refusal {
            origin: crate::settings::Origin::BuiltIn,
            key: "accounts".into(),
            problem: crate::settings::Problem::Input("too many account sources".into()),
        })
    })?;
    let mut sources = List::with_capacity(count);
    let mut locations = List::with_capacity(count);
    for account in &resolved.accounts {
        let descriptor =
            source(&account.fields, &resolved.settings.workspace, codex(resolved, account.number), &account.name);
        locations
            .push(Location { account: account.number, name: account.name.clone(), source: descriptor.clone() })
            .expect("selected locations fit their account bound");
        let handed = match descriptor {
            Source::Borrow { file, format, program } => {
                let credential =
                    inspect(&Source::Borrow { file: file.clone(), format, program: program.clone() }, wall);
                if !credential.usable() {
                    return Err(unavailable(&account.name, credential));
                }
                let parent = file
                    .parent()
                    .ok_or_else(|| unavailable(&account.name, Credential::Unreadable { file: file.clone() }))?;
                let path = file
                    .file_name()
                    .ok_or_else(|| unavailable(&account.name, Credential::Unreadable { file: file.clone() }))?;
                let root = skein_shell::open_root(parent)
                    .map_err(|_| unavailable(&account.name, Credential::Unreadable { file: file.clone() }))?;
                let path = path.as_encoded_bytes().into();
                service::Source::Borrow { account: account.number, root, path, format }
            }
            Source::Env { variable, codex } => {
                let borrowed = environment(&variable, codex, wall)
                    .ok_or_else(|| unavailable(&account.name, Credential::Unset { variable: variable.clone() }))?;
                if borrowed.record.remaining(wall) == Duration::ZERO {
                    return Err(unavailable(
                        &account.name,
                        Credential::ExpiredVariable { at: borrowed.record.expires_at.as_nanos(), variable },
                    ));
                }
                service::Source::Env { account: account.number, borrowed }
            }
            Source::SignIn { file } => {
                let credential = super::sign_in::inspect(&file, wall);
                if !interactive && !credential.usable() {
                    return Err(unavailable(&account.name, credential));
                }
                super::sign_in::private_source(
                    account.number,
                    &file,
                    super::registration(account, codex(resolved, account.number), &resolved.settings.workspace)
                        .map_err(Refusal::Configuration)?,
                    interactive,
                )
                .map_err(|_| unavailable(&account.name, Credential::Unreadable { file }))?
            }
        };
        assert!(sources.push(handed).is_ok(), "selected sources fit their account bound");
    }
    Ok(Prepared { sources, locations, limits: limits(resolved.agent.service.limits.io, count.max(1)) })
}

fn unavailable(account: &str, credential: Credential) -> Refusal {
    Refusal::Credentials { account: account.into(), credential }
}

/// Receiving bounds for handed-in accounts; no exchange or public listener is opened.
#[must_use]
pub fn limits(mut io: skein_io::Limits, accounts: u32) -> skein_oauth_accounts::Limits {
    // One exchange, its callback and its listener need three socket slots.
    io.sockets = 3;
    skein_oauth_accounts::Limits {
        accounts,
        file_stall: Duration::from_secs(30),
        lock_wait: Duration::from_secs(1),
        exchanges: 1,
        listeners: 1,
        server: skein_http::server::Limits {
            head: 1024,
            headers: 8,
            body: 1024,
            read: 1024,
            response: 1024,
            send: 1024,
        },
        refresh_lead: Duration::from_secs(60),
        client: client_limits(),
        http: skein_http::client::Limits { request: 16_384, head: 16_384, headers: 128, read: 16_384, send: 16_384 },
        tls: skein_tls::client::Limits { read: 16_384, send: 16_384, records: 1_048_576 },
        io,
    }
}

/// Native exchange bounds shared by startup inspection and private sign-in.
#[must_use]
pub fn client_limits() -> skein_oauth::ClientLimits {
    skein_oauth::ClientLimits {
        parameter_bytes: 0,
        resource_bytes: 0,
        nonce_bytes: 0,
        metadata_bytes: 0,
        key_set_bytes: 0,
        keys: 0,
        modulus_bits: 2048,
        skew: Duration::ZERO,
        document: crate::local_host::token_limits(),
        uri_bytes: 4096,
        scope_bytes: 1024,
        state_bytes: 64,
        code_bytes: 4096,
        url_bytes: 8192,
        request_bytes: 16_384,
        sign_in_time: Duration::from_secs(120),
        request_time: Duration::from_secs(20),
        backoff_base: Duration::from_secs(1),
        backoff_ceiling: Duration::from_secs(8),
        max_attempts: 3,
    }
}
