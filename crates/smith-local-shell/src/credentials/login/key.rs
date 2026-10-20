//! Read a private key from piped input (shell.md, 4.5, 7.2).
//! Values enter the private store only; every diagnostic names the account.
use super::LoginOutcome;
use std::{
    io::{IsTerminal, Read},
    path::Path,
};

pub(super) fn run(name: &str, file: &Path) -> Result<LoginOutcome, String> {
    keep(file, &mut std::io::stdin().lock(), std::io::stdin().is_terminal())?;
    eprintln!("account {name}: key kept");
    Ok(LoginOutcome::Usable)
}

pub(super) fn keep(file: &Path, input: &mut dyn Read, terminal: bool) -> Result<(), String> {
    if terminal {
        return Err("key input must be piped; a terminal echoes credentials".into());
    }
    let bound = crate::local_host::token_limits().token_bytes;
    let mut bytes = Vec::new();
    input
        .take(u64::from(bound).checked_add(2).expect("key input bound"))
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read key input")?;
    let bytes = super::super::key::value(&bytes).map_err(|_| "empty or invalid key input")?;
    let entropy = crate::startup::oauth_entropy()?;
    super::super::private_file::store(file, bytes, skein_io::file::Expect::Any, &entropy)?;
    Ok(())
}

#[cfg(test)]
mod tests;
