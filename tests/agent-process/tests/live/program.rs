//! Opt-in checks of the real program's interrupt and resume contracts (programs.md 5.7, 8).
use std::os::unix::fs::PermissionsExt;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct Probe {
    home: std::path::PathBuf,
    session: String,
}

impl Probe {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let number = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        assert!(number <= 0xffff_ffff_ffff, "probe sequence fits the session UUID");
        let process = std::process::id();
        let home = std::env::temp_dir().join(format!("smith-live-{process}-{number}"));
        std::fs::create_dir(&home).expect("private test home");
        std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o700)).expect("private test home permissions");
        Self { home, session: format!("{process:08x}-d94e-4ada-98a0-{number:012x}") }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.home).expect("remove private test home");
    }
}

struct Live {
    child: Child,
    probe: Probe,
    messages: mpsc::Receiver<serde_json::Value>,
    configuration: std::path::PathBuf,
}

impl Live {
    fn start(text: &str) -> Self {
        let configuration = std::env::var_os("CLAUDE_CONFIG_DIR")
            .expect("a Claude Code sign-in directory kept for live tests (smith-14 settles where), never ~/.claude or smith's own directories");
        let personal_home = std::path::PathBuf::from(std::env::var_os("HOME").expect("HOME for directory exclusion"));
        let config_home =
            std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| personal_home.join(".config"), std::path::PathBuf::from);
        let state_home = std::env::var_os("XDG_STATE_HOME")
            .map_or_else(|| personal_home.join(".local/state"), std::path::PathBuf::from);
        let configuration = test_configuration(std::path::Path::new(&configuration), &personal_home, &config_home, &state_home)
            .expect("a Claude Code sign-in directory kept for live tests (smith-14 settles where), never ~/.claude or smith's own directories");
        let probe = Probe::new();
        let (child, messages) = Self::spawn(&probe.home, &configuration, &["--session-id", &probe.session], text);
        Self { child, probe, messages, configuration }
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "live protocol probe bounds blocking stdout; departure from testing-strategy.md 2.9"
    )]
    fn spawn(
        home: &std::path::Path,
        configuration: &std::path::Path,
        arguments: &[&str],
        text: &str,
    ) -> (Child, mpsc::Receiver<serde_json::Value>) {
        let mut child = Command::new("claude")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").expect("PATH for claude"))
            .env("HOME", home)
            .env("CLAUDE_CONFIG_DIR", configuration)
            .current_dir(home)
            .args([
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                "--verbose",
                "--include-partial-messages",
                "--setting-sources",
                "user",
                "--strict-mcp-config",
                "--mcp-config",
                r#"{"mcpServers":{}}"#,
                "--tools",
                "",
                "--max-turns",
                "1",
            ])
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("claude executable");
        writeln!(
            child.stdin.as_mut().expect("stdin"),
            "{}",
            serde_json::json!({"type":"user","message":{
                "role":"user","content":text
            }})
        )
        .expect("user message");
        let stdout = child.stdout.take().expect("stdout");
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = line.expect("program stdout");
                if let Ok(value) = serde_json::from_str(&line)
                    && sender.send(value).is_err()
                {
                    return;
                }
            }
        });
        (child, messages)
    }

    fn message(&self) -> serde_json::Value {
        self.messages.recv_timeout(Duration::from_mins(1)).expect("program message within the live deadline")
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
#[ignore = "a Claude Code sign-in directory kept for live tests (smith-14 settles where), never ~/.claude or smith's own directories"]
fn a_live_interrupt_ends_the_turn() {
    let mut live = Live::start("Count from one to ten thousand, writing every number.");
    for _ in 0..1000 {
        let message = live.message();
        assert_ne!(message["type"], "result", "the turn must still run when interrupted");
        if message["event"]["type"] == "message_start" {
            break;
        }
    }
    assert!(Command::new("/bin/kill").args(["-INT", &live.child.id().to_string()]).status().expect("SIGINT").success());
    for _ in 0..1000 {
        let message = live.message();
        if message["type"] == "result" {
            assert_eq!(message["terminal_reason"], "interrupted");
            drop(live.child.stdin.take());
            return;
        }
    }
    panic!("interrupt did not end the program turn");
}

// Canonical paths catch directory aliases without inspecting any login content.
fn test_configuration(
    configuration: &std::path::Path,
    home: &std::path::Path,
    config_home: &std::path::Path,
    state_home: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    if !configuration.is_absolute() {
        return Err(std::io::Error::other("CLAUDE_CONFIG_DIR must be absolute"));
    }
    let configuration = configuration.canonicalize()?;
    for forbidden in [home.join(".claude"), home.join(".codex"), config_home.join("smith"), state_home.join("smith")] {
        let forbidden = match forbidden.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        if configuration.starts_with(forbidden) {
            return Err(std::io::Error::other("personal or smith directory"));
        }
    }
    Ok(configuration)
}

#[test]
fn live_configuration_refuses_personal_or_smith_paths() {
    let root = std::env::temp_dir().join(format!("smith-live-paths-{}", std::process::id()));
    std::fs::create_dir(&root).expect("private test directory");
    let home = root.join("home");
    let defaults = [home.join(".config"), home.join(".local/state")];
    let overrides = [root.join("xdg-config"), root.join("xdg-state")];
    for [config, state] in [&defaults, &overrides] {
        for forbidden in [home.join(".claude"), home.join(".codex"), config.join("smith"), state.join("smith")] {
            let nested = forbidden.join("source");
            std::fs::create_dir_all(&nested).expect("private live-directory fixture");
            assert!(test_configuration(&forbidden, &home, config, state).is_err());
            assert!(test_configuration(&nested, &home, config, state).is_err());
            let alias = root.join("alias");
            std::os::unix::fs::symlink(&nested, &alias).expect("private live-directory fixture");
            assert!(test_configuration(&alias, &home, config, state).is_err());
            std::fs::remove_file(alias).expect("private live-directory fixture");
        }
    }
    let kept = root.join("live-tests");
    std::fs::create_dir(&kept).expect("private live-directory fixture");
    let alias = root.join("allowed-alias");
    std::os::unix::fs::symlink(&kept, &alias).expect("private live-directory fixture");
    assert_eq!(
        test_configuration(&alias, &home, &overrides[0], &overrides[1]).expect("private live-directory fixture"),
        kept.canonicalize().expect("private live-directory fixture")
    );
    assert!(test_configuration(std::path::Path::new("relative"), &home, &overrides[0], &overrides[1]).is_err());
    std::fs::remove_dir_all(root).expect("private live-directory fixture");
}

#[test]
#[ignore = "a Claude Code sign-in directory kept for live tests (smith-14 settles where), never ~/.claude or smith's own directories"]
fn a_live_resume_continues() {
    let mut live = Live::start("Remember the word persimmon. Reply with OK.");
    let id = live.probe.session.clone();
    let mut finished = false;
    for _ in 0..1000 {
        let message = live.message();
        if message["type"] == "result" {
            assert_eq!(message["is_error"], false);
            finished = true;
            break;
        }
    }
    assert!(finished, "first turn result");
    drop(live.child.stdin.take());
    assert!(live.child.wait().expect("first exit").success());
    (live.child, live.messages) = Live::spawn(
        &live.probe.home,
        &live.configuration,
        &["--resume", &id],
        "Which word did I ask you to remember? Reply with that word only.",
    );
    let mut text = String::new();
    for _ in 0..1000 {
        let message = live.message();
        if message["type"] == "assistant" {
            for block in message["message"]["content"].as_array().expect("assistant blocks") {
                if let Some(part) = block["text"].as_str() {
                    text.push_str(part);
                }
            }
        }
        if message["type"] == "result" {
            assert_eq!(message["is_error"], false);
            assert!(text.to_lowercase().contains("persimmon"), "{text}");
            drop(live.child.stdin.take());
            return;
        }
    }
    panic!("resumed turn did not finish");
}

#[test]
fn live_probes_have_distinct_private_state() {
    let first = Probe::new();
    let second = Probe::new();
    assert_ne!(first.home, second.home);
    assert_ne!(first.session, second.session);
    assert_eq!(first.session.len(), 36);
    assert_eq!(second.session.len(), 36);
    for home in [&first.home, &second.home] {
        assert!(home.is_dir());
        assert_eq!(std::fs::metadata(home).expect("private home").permissions().mode() & 0o777, 0o700);
    }
    let removed = first.home.clone();
    drop(first);
    assert!(!removed.exists());
    assert!(second.home.is_dir(), "one probe does not remove another's home");
}
