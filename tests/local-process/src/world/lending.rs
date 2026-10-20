//! Env and key lending stories outside every hosted process.
use super::{Files, Placement, World};
use skein_lib::{Duration, Writer, base64};

/// The access source lent by the local service composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lending {
    /// The shell lends a named variable once.
    Env,
    /// The shell lends a private key on each request.
    Key,
}

pub(crate) fn login_token(expired: bool) -> Box<[u8]> {
    login_at(skein_tls_world::pki::VALID, expired)
}

/// Access-only synthetic login for a real-loop story's clock.
#[must_use]
pub fn login_at(wall: skein_lib::Wall, expired: bool) -> Box<[u8]> {
    let wall = wall.as_nanos();
    let nanos =
        if expired { wall - Duration::from_secs(1).as_nanos() } else { wall + Duration::from_secs(7200).as_nanos() };
    let claims = serde_json::to_vec(&serde_json::json!({
        "exp": nanos / 1_000_000_000,
        "https://api.openai.com/auth": {"chatgpt_account_id": "acc"}
    }))
    .expect("synthetic claims");
    let length =
        base64::encoded_len(base64::Alphabet::Url, u32::try_from(claims.len()).expect("fixture claims length"))
            .expect("fixture encoded length");
    let mut encoded =
        Writer::new(usize::try_from(length).expect("fixture length").checked_add(9).expect("fixture token length"));
    encoded.put(b"e30.").expect("fixture header");
    base64::encode(base64::Alphabet::Url, &claims, &mut encoded).expect("fixture claims");
    encoded.put(b".c2ln").expect("fixture signature");
    encoded.finish()
}

impl World {
    /// Lend one synthetic env token or private key to the run.
    #[must_use]
    pub fn lending(seed: u64, files: &Files, lending: Lending) -> Self {
        let mut world = Self::new(seed, files, Placement::InProcess, b"First answer");
        world.scenario.launch.lending = Some(lending);
        if lending == Lending::Key {
            use std::os::unix::fs::PermissionsExt;
            let key = files.path().join("key");
            std::fs::write(&key, b"key-for-work").expect("scratch key");
            std::fs::set_permissions(key, std::fs::Permissions::from_mode(0o600)).expect("private key");
        }
        world.check_memory();
        world
    }
}
