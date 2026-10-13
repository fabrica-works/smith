//! The retained JSON face prepares the same native account owner once.
//! OAuth records stay in private keepers; access-only fixtures are handed in.
use super::{Prepared, Registered};
use crate::local_settings::Account;
use skein_lib::List;
use smith_local_service::Source;

/// Prepare JSON-face sources for the existing native driver, adopting named roots.
pub fn prepare(
    accounts: &[Account],
    directory: &std::path::Path,
    io: skein_io::Limits,
    roots: Option<&[skein_io::kernel::Fd]>,
) -> Result<Prepared, String> {
    let count = u32::try_from(accounts.len()).map_err(|_| "too many accounts")?;
    let mut sources = List::with_capacity(count);
    for (index, account) in accounts.iter().enumerate() {
        let file = directory.join(format!("{}.json", account.number));
        let source = if let Some(config) = &account.oauth {
            super::sign_in::issuer(&config.authorization_url).map_err(str::to_string)?;
            super::sign_in::issuer(&config.token_endpoint).map_err(str::to_string)?;
            let address: std::net::SocketAddr = config.address.parse().map_err(|_| "invalid OAuth address")?;
            let transport = if config.token_endpoint.starts_with("http://") {
                skein_oauth_accounts::Transport::Plaintext
            } else {
                skein_oauth_accounts::Transport::Tls {
                    server_name: skein_tls::Name::new(&config.server_name).ok_or("invalid OAuth TLS name")?,
                    trust: smith_agent_shell::config::trust(config.trust_der.as_deref())?,
                }
            };
            let registered = Registered {
                registration: skein_oauth::Registration {
                    authorization_url: config.authorization_url.as_bytes().into(),
                    token_endpoint: config.token_endpoint.as_bytes().into(),
                    client_id: config.client_id.as_bytes().into(),
                    redirect_uri: config.redirect_uri.as_bytes().into(),
                    scope: config.scope.as_bytes().into(),
                    wire: if config.json { skein_oauth::WireFormat::Json } else { skein_oauth::WireFormat::Form },
                    client_secret: None,
                    pkce_for_confidential: false,
                    metadata_claim: None,
                },
                endpoint: skein_oauth_accounts::Endpoint { address, transport },
            };
            let mut source = super::sign_in::private_source_at(
                account.number,
                &file,
                registered,
                true,
                roots.and_then(|roots| roots.get(index)).copied(),
            )?;
            match &mut source {
                Source::SignIn { account_id, .. } => *account_id = account.account_id.as_bytes().into(),
                Source::Borrow { .. } | Source::Env { .. } => unreachable!("prepared private source"),
            }
            source
        } else {
            let credential = super::sign_in::inspect(&file, skein_lib::Wall::EPOCH);
            if !credential.usable() {
                return Err("the access-only account has no private record".into());
            }
            let bytes = skein_shell::read_private_file(
                &file,
                crate::local_host::token_limits().record_bytes,
                skein_shell::effective_user(),
            )
            .map_err(|why| format!("access-only record: {why:?}"))?;
            let mut record = skein_oauth::decode_record(bytes.expose(), &crate::local_host::token_limits())
                .map_err(|_| "invalid access-only record")?;
            record.refresh_token = None;
            Source::Env {
                account: account.number,
                borrowed: smith_local_protocol::Borrowed { record, account_id: account.account_id.as_bytes().into() },
            }
        };
        assert!(sources.push(source).is_ok(), "configured sources fit their count");
    }
    Ok(Prepared { sources, locations: List::with_capacity(0), limits: super::limits(io, count.max(1)) })
}
