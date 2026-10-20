//! Access-only conversion keeps credential values only while preparing a record.
//! The source login is read-only; no refresh field is deserialized or copied.
//! `codex` writes a guarded private record; smith uses its own sign-in record.
//! `Borrowed::finish` settles the credential and checks the source digest.
//! Contracts: benchmarks.md, section 10; reliability-plan Credentials (95a9807).

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skein_lib::{Duration, Wall};

use crate::guard::{Guard, check_credential_record};
use crate::isolate::{Setup, SetupStep};

pub mod logins;

const MAX_AUTH_BYTES: u64 = 128 * 1024;

#[derive(Deserialize)]
struct Source {
    auth_mode: String,
    tokens: Tokens,
}

#[derive(Deserialize, Serialize)]
struct Tokens {
    id_token: String,
    access_token: String,
    account_id: Option<String>,
}

#[derive(Serialize)]
struct AccessOnly<'a> {
    auth_mode: &'a str,
    tokens: AccessTokens<'a>,
    last_refresh: String,
}

#[derive(Serialize)]
struct AccessTokens<'a> {
    id_token: &'a str,
    access_token: &'a str,
    account_id: &'a Option<String>,
    refresh_token: &'static str,
}

#[derive(Deserialize)]
struct Claims {
    exp: u64,
}

/// Nonsecret evidence needed to remove a borrowed file and verify its source.
#[derive(Clone, Debug)]
pub struct Borrowed {
    pub credential: PathBuf,
    pub expires: Wall,
    source: PathBuf,
    digest: String,
}

struct Login {
    source: PathBuf,
    auth: Source,
    digest: String,
    expires: Wall,
}

/// The read-only source and private-file observations retained after settlement.
#[derive(Debug, Serialize)]
#[expect(clippy::struct_excessive_bools, reason = "independent persisted assertions, not execution states")]
pub struct Proof {
    pub source_sha256_before: String,
    pub source_sha256_after: String,
    pub source_unchanged: bool,
    pub expires_unix_seconds: u64,
    pub credential_removed: bool,
    pub private_refresh_empty: bool,
    pub private_refresh_unusable: bool,
}

fn read_source(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| "Codex auth.json is unavailable")?;
    if !metadata.is_file() || metadata.is_symlink() || metadata.len() > MAX_AUTH_BYTES {
        return Err("Codex auth.json must be a bounded ordinary file".into());
    }
    let file = File::open(path).map_err(|_| "cannot read Codex auth.json")?;
    let mut bytes = Vec::new();
    file.take(MAX_AUTH_BYTES + 1).read_to_end(&mut bytes).map_err(|_| "cannot read Codex auth.json")?;
    if u64::try_from(bytes.len()).expect("bounded credential fits u64") > MAX_AUTH_BYTES {
        return Err("Codex auth.json exceeds the credential bound".into());
    }
    Ok(bytes)
}

fn expiry(access: &str) -> Result<Wall, String> {
    let mut segments = access.split('.');
    let header = segments.next().ok_or("access token has no JWT header")?;
    let payload = segments.next().ok_or("access token has no JWT claims")?;
    let signature = segments.next().ok_or("access token has no JWT signature")?;
    if header.is_empty() || signature.is_empty() || segments.next().is_some() {
        return Err("access token has an invalid JWT shape".into());
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| "access token claims are not base64url")?;
    let claims: Claims = serde_json::from_slice(&bytes).map_err(|_| "access token has no valid expiry claim")?;
    let nanos = claims.exp.checked_mul(1_000_000_000).ok_or("access token expiry is outside the wall clock")?;
    Ok(Wall::from_nanos(nanos))
}

fn login(source_home: &Path, now: Wall, lifetime: Duration) -> Result<Login, String> {
    let source = source_home.join("auth.json");
    let bytes = read_source(&source)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let auth: Source = serde_json::from_slice(&bytes).map_err(|_| "Codex auth.json has an invalid shape")?;
    if auth.auth_mode.is_empty() || auth.tokens.access_token.is_empty() {
        return Err("Codex login has no authentication mode or access token".into());
    }
    let expires = expiry(&auth.tokens.access_token)?;
    let end = now.as_nanos().checked_add(lifetime.as_nanos()).ok_or("attempt lifetime exceeds the wall clock")?;
    if expires.as_nanos() <= end {
        return Err("Codex access token expires before the attempt ends".into());
    }
    Ok(Login { source, auth, digest, expires })
}

fn write_record(guard: &Guard, home: &Path, name: &str, encoded: &[u8]) -> Result<PathBuf, String> {
    let home = crate::isolate::create_directory(guard, home)?;
    let credential = guard.write_path(&home.join(name))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&credential)
        .map_err(|_| "cannot create private credential record")?;
    if let Err(error) = file.write_all(encoded).and_then(|()| file.sync_all()) {
        std::fs::remove_file(guard.write_path(&credential)?).map_err(|_| "cannot remove incomplete credential")?;
        return Err(format!("cannot write private credential record: {}", error.kind()));
    }
    if file.metadata().map_err(|_| "cannot inspect private credential record")?.permissions().mode() & 0o777 != 0o600 {
        std::fs::remove_file(guard.write_path(&credential)?)
            .map_err(|_| "cannot remove incorrectly private credential")?;
        return Err("credential record requires mode 0600".into());
    }
    Ok(credential)
}

/// Convert only allowed Codex fields into a new private attempt-owned home.
/// `now` is the shell's wall observation; `lifetime` covers the hard deadline.
pub fn codex(guard: &Guard, source_home: &Path, home: &Path, now: Wall, lifetime: Duration) -> Result<Borrowed, Setup> {
    logins::codex(source_home)?;
    let converted: Result<Borrowed, String> = (|| {
        let Login { source, auth: source_auth, digest, expires } = login(source_home, now, lifetime)?;
        let last_refresh = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(now.as_nanos()))
            .map_err(|_| "credential write time is outside the timestamp range")?
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| "credential write time cannot be formatted")?;
        let record = serde_json::to_value(AccessOnly {
            auth_mode: &source_auth.auth_mode,
            tokens: AccessTokens {
                id_token: &source_auth.tokens.id_token,
                access_token: &source_auth.tokens.access_token,
                account_id: &source_auth.tokens.account_id,
                refresh_token: "",
            },
            last_refresh,
        })
        .map_err(|_| "cannot encode the access-only record")?;
        check_credential_record(&record)?;
        let encoded = serde_json::to_vec(&record).map_err(|_| "cannot encode the access-only record")?;
        let credential = write_record(guard, home, "auth.json", &encoded)?;
        Ok(Borrowed { credential, expires, source, digest })
    })();
    converted.map_err(|reason| Setup {
        step: SetupStep::Credentials,
        reason: format!("{reason}; run `CODEX_HOME={} codex` once before retrying", source_home.display()),
    })
}

impl Borrowed {
    /// Remove the private auth file after settlement and verify the user login.
    pub fn finish(&self, guard: &Guard) -> Result<Proof, String> {
        let checked: Result<(), String> = (|| {
            let bytes = read_source(&self.credential)?;
            let record: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| "private Codex auth.json changed shape")?;
            check_credential_record(&record)?;
            if record.pointer("/tokens/refresh_token").and_then(serde_json::Value::as_str) != Some("") {
                return Err("private Codex refresh field is no longer empty".into());
            }
            Ok(())
        })();
        std::fs::remove_file(guard.write_path(&self.credential)?)
            .map_err(|_| "cannot remove private Codex auth.json")?;
        checked?;
        let after = format!("{:x}", Sha256::digest(read_source(&self.source)?));
        if after != self.digest {
            return Err(
                "user Codex auth.json changed during the isolated attempt; stop before further inference".into()
            );
        }
        Ok(Proof {
            source_sha256_before: self.digest.clone(),
            source_sha256_after: after,
            source_unchanged: true,
            expires_unix_seconds: self.expires.as_secs(),
            credential_removed: true,
            private_refresh_empty: true,
            private_refresh_unusable: true,
        })
    }
}
