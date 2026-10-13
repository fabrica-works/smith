//! Synthetic records for process worlds; production owns no fixture file IO.
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub struct Tokens {
    root: PathBuf,
    limits: skein_oauth::Limits,
}
impl Tokens {
    pub fn new(root: &Path, limits: skein_oauth::Limits) -> Result<Self, String> {
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(root).map_err(|why| why.to_string())?;
        Ok(Self { root: root.into(), limits })
    }
    pub fn load(&self, account: u32) -> Result<Option<skein_oauth::SavedToken>, String> {
        match std::fs::read(self.root.join(format!("{account}.json"))) {
            Ok(bytes) => skein_oauth::decode_record(&bytes, &self.limits).map(Some).map_err(|why| format!("{why:?}")),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(why) => Err(why.to_string()),
        }
    }
    pub fn save(&self, account: u32, bytes: &[u8]) -> Result<(), String> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(self.root.join(format!("{account}.json")))
            .map_err(|why| why.to_string())?;
        file.write_all(bytes).map_err(|why| why.to_string())
    }
}
