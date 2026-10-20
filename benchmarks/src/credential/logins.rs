//! Harness login layout and existence-only admission (benchmarks.md, section 10).
//! The developer creates these paths; the harness never writes them.

use std::path::Path;

use crate::isolate::{Setup, SetupStep};

/// Shared account declarations for the attempt layer and documented setup.
pub const ACCOUNTS: &str = include_str!("../../agents/smith/accounts.toml");

fn present(path: &Path, directory: bool) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|metadata| !metadata.is_symlink() && if directory { metadata.is_dir() } else { metadata.is_file() })
}

/// Require only the selected smith account's setup, without opening its record.
pub fn smith(directory: &Path, account: &str) -> Result<(), Setup> {
    let required = match account {
        "chatgpt" => vec![("config/smith/config.toml", false), ("host-id", false), ("tokens/chatgpt", false)],
        "claude" => vec![("config/smith/config.toml", false), ("programs/claude", true)],
        "anthropic-key" => {
            return if std::env::var_os("ANTHROPIC_API_KEY").is_some_and(|value| !value.is_empty()) {
                Ok(())
            } else {
                Err(Setup { step: SetupStep::Credentials, reason: "`ANTHROPIC_API_KEY` is not set".into() })
            };
        }
        _ => return Err(Setup { step: SetupStep::Credentials, reason: "unknown harness account".into() }),
    };
    if required.iter().all(|(path, directory_kind)| present(&directory.join(path), *directory_kind)) {
        return Ok(());
    }
    Err(Setup {
        step: SetupStep::Credentials,
        reason: format!(
            "no smith login for the harness: run `XDG_CONFIG_HOME={} smith login {account}`",
            directory.join("config").display()
        ),
    })
}

/// Require the harness's Codex record before reading its access fields.
pub fn codex(directory: &Path) -> Result<(), Setup> {
    if present(&directory.join("auth.json"), false) {
        return Ok(());
    }
    Err(Setup {
        step: SetupStep::Credentials,
        reason: format!("no Codex login for the harness: run `CODEX_HOME={} codex login`", directory.display()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;

    #[test]
    fn documented_accounts_match_the_rendered_declarations() {
        assert!(include_str!("../../README.md").contains("cat benchmarks/agents/smith/accounts.toml >>"));
    }

    #[test]
    fn missing_login_pieces_name_the_setup_command() {
        let root = std::env::temp_dir().join(format!("smith-bench-logins-{}", std::process::id()));
        std::fs::DirBuilder::new().mode(0o700).create(&root).expect("private synthetic root");
        for (account, missing) in [
            ("chatgpt", "config/smith/config.toml"),
            ("chatgpt", "host-id"),
            ("chatgpt", "tokens/chatgpt"),
            ("claude", "config/smith/config.toml"),
            ("claude", "programs/claude"),
        ] {
            let directory = root.join("smith");
            for file in ["config/smith/config.toml", "host-id", "tokens/chatgpt"] {
                std::fs::create_dir_all(directory.join(file).parent().expect("parent")).expect("synthetic parent");
                std::fs::write(directory.join(file), []).expect("synthetic marker");
            }
            std::fs::create_dir_all(directory.join("programs/claude")).expect("synthetic program login");
            let path = directory.join(missing);
            if path.is_dir() {
                std::fs::remove_dir(&path).expect("missing directory");
            } else {
                std::fs::remove_file(&path).expect("missing file");
            }
            assert_eq!(
                smith(&directory, account).expect_err(missing).reason,
                format!(
                    "no smith login for the harness: run `XDG_CONFIG_HOME={} smith login {account}`",
                    directory.join("config").display()
                )
            );
            std::fs::remove_dir_all(directory).expect("reset synthetic setup");
        }
        let directory = root.join("codex");
        assert_eq!(
            codex(&directory).expect_err("missing Codex").reason,
            format!("no Codex login for the harness: run `CODEX_HOME={} codex login`", directory.display())
        );
        std::fs::remove_dir_all(root).expect("synthetic cleanup");
    }
}
