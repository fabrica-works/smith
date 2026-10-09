//! Attempt preparation keeps private paths and an exact environment, no login.
//! `prepare` verifies and copies the frozen seed before inference;
//! `environment` gives commands only machine tools, private homes and explicit
//! task variables (benchmarks.md, sections 6.1 and 10). Setup and measurement
//! belong to the runner; this module reads no clock or agent configuration.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::guard::Guard;

/// The machine's tool paths, supplied once to attempt preparation.
#[derive(Clone, Debug)]
pub struct Machine {
    pub path: OsString,
    pub rustup_home: Option<PathBuf>,
}

impl Machine {
    /// Read only the allow-listed directory choices, never an inherited login.
    pub fn from_environment() -> Result<Self, String> {
        let path = std::env::var_os("PATH").filter(|value| !value.is_empty()).ok_or("PATH is required")?;
        let rustup_home = std::env::var_os("RUSTUP_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".rustup")));
        Ok(Self { path, rustup_home })
    }
}

/// The writable paths supplied to one agent and its commands.
#[derive(Clone, Debug)]
pub struct Private {
    pub root: PathBuf,
    pub workspace: PathBuf,
    pub home: PathBuf,
    pub temporary: PathBuf,
    pub cargo_home: PathBuf,
    pub target: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
}

impl Private {
    fn paths(root: &Path) -> Self {
        Self {
            root: root.into(),
            workspace: root.join("workspace"),
            home: root.join("home"),
            temporary: root.join("tmp"),
            cargo_home: root.join("cargo"),
            target: root.join("target"),
            config: root.join("config"),
            state: root.join("state"),
        }
    }
}

fn directory(guard: &Guard, path: &Path) -> Result<(), String> {
    let destination = guard.write_path(path)?;
    fs::DirBuilder::new().mode(0o700).create(destination).map_err(|error| format!("private directory: {error}"))
}

fn copy_seed(guard: &Guard, source: &Path, destination: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(source)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
        let destination = guard.write_path(&destination.join(entry.file_name()))?;
        if metadata.is_dir() {
            directory(guard, &destination)?;
            copy_seed(guard, &entry.path(), &destination)?;
        } else if metadata.is_file() && !metadata.is_symlink() {
            fs::copy(entry.path(), &destination).map_err(|error| error.to_string())?;
            let mode = if metadata.permissions().mode() & 0o111 == 0 { 0o600 } else { 0o700 };
            fs::set_permissions(&destination, fs::Permissions::from_mode(mode)).map_err(|error| error.to_string())?;
        } else {
            return Err("seed contains a link or special file".into());
        }
    }
    Ok(())
}

/// Materialise a fresh guarded attempt after verifying the frozen seed digest.
/// The caller supplies an existing private scratch parent outside repositories.
pub fn prepare(guard: &Guard, seed: &Path, sha256: &str, root: &Path) -> Result<Private, String> {
    if crate::seed_digest(seed).map_err(|error| error.to_string())? != sha256 {
        return Err("seed digest differs; setup refused before materialisation".into());
    }
    let paths = Private::paths(&guard.write_path(root)?);
    directory(guard, &paths.root)?;
    for path in
        [&paths.workspace, &paths.home, &paths.temporary, &paths.cargo_home, &paths.target, &paths.config, &paths.state]
    {
        directory(guard, path)?;
    }
    copy_seed(guard, seed, &paths.workspace)?;
    if crate::seed_digest(&paths.workspace).map_err(|error| error.to_string())? != sha256 {
        return Err("materialised seed digest differs; setup refused".into());
    }
    Ok(paths)
}

/// Commit the materialised seed in its own repository, without user git policy.
/// The seed digest must already have been verified before git metadata exists.
pub fn commit_seed(guard: &Guard, machine: &Machine, private: &Private) -> Result<(), String> {
    guard.write_path(&private.workspace)?;
    guard.write_path(&private.workspace.join(".git"))?;
    if private.workspace.join(".git").exists() {
        return Err("attempt repository already exists".into());
    }
    let variables = environment(machine, private, &BTreeMap::new())?;
    for arguments in [
        vec!["init", "--quiet"],
        vec!["add", "--", "."],
        vec!["commit", "--quiet", "--allow-empty", "-m", "Frozen task seed"],
    ] {
        let status = Command::new("git")
            .current_dir(&private.workspace)
            .env_clear()
            .envs(variables.clone())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.autocrlf=false",
                "-c",
                "gc.auto=0",
                "-c",
                "maintenance.auto=false",
                "-c",
                "user.name=Smith benchmark",
                "-c",
                "user.email=smith-bench@example.invalid",
            ])
            .args(arguments)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|error| format!("seed git setup: {error}"))?;
        if !status.success() {
            return Err(format!("seed git setup failed: {status}"));
        }
    }
    Ok(())
}

/// Construct an exact allow-listed environment, refusing private-path overrides.
pub fn environment(
    machine: &Machine,
    private: &Private,
    task: &BTreeMap<String, String>,
) -> Result<Vec<(OsString, OsString)>, String> {
    let mut values = BTreeMap::new();
    values.insert(OsString::from("PATH"), machine.path.clone());
    values.insert("LANG".into(), "C.UTF-8".into());
    values.insert("LC_ALL".into(), "C.UTF-8".into());
    for (name, path) in [
        ("HOME", &private.home),
        ("TMPDIR", &private.temporary),
        ("CARGO_HOME", &private.cargo_home),
        ("CARGO_TARGET_DIR", &private.target),
        ("XDG_CONFIG_HOME", &private.config),
        ("XDG_STATE_HOME", &private.state),
    ] {
        values.insert(name.into(), path.as_os_str().into());
    }
    if let Some(path) = &machine.rustup_home {
        values.insert("RUSTUP_HOME".into(), path.as_os_str().into());
    }
    for (name, value) in task {
        if name.is_empty() || name.contains(['=', '\0']) || value.contains('\0') {
            return Err("task environment names/values must be valid process entries".into());
        }
        if values.contains_key(&OsString::from(name))
            || matches!(name.as_str(), "CODEX_HOME" | "CLAUDE_CONFIG_DIR" | "RUSTUP_HOME")
            || name.starts_with("GIT_CONFIG_")
        {
            return Err(format!("task variable {name} would override harness isolation"));
        }
        values.insert(name.into(), value.into());
    }
    Ok(values.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_environment_has_private_caches_and_only_explicit_task_variables() {
        let machine = Machine { path: "/usr/bin:/bin".into(), rustup_home: Some("/tools/rustup".into()) };
        let private = Private::paths(Path::new("/attempt"));
        let task = BTreeMap::from([("BENCH_MARKER".into(), "exact".into())]);
        let values: BTreeMap<_, _> = environment(&machine, &private, &task).expect("allow-list").into_iter().collect();
        assert_eq!(values[&OsString::from("HOME")], OsString::from("/attempt/home"));
        assert_eq!(values[&OsString::from("CARGO_TARGET_DIR")], OsString::from("/attempt/target"));
        assert_eq!(values[&OsString::from("BENCH_MARKER")], OsString::from("exact"));
        assert!(!values.contains_key(&OsString::from("CODEX_HOME")));
        for name in ["HOME", "PATH", "TMPDIR", "CODEX_HOME", "CARGO_HOME", "GIT_CONFIG_GLOBAL"] {
            assert!(environment(&machine, &private, &BTreeMap::from([(name.into(), "/outside".into())])).is_err());
        }
    }

    #[test]
    fn seed_digest_mismatch_is_refused_before_writing_any_attempt() {
        let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/tasks/valid/seed");
        let fake_home = std::env::temp_dir().join(format!("smith-bench-isolation-{}", std::process::id()));
        let guard = Guard::new(&fake_home, &fake_home.join("config"), &fake_home.join("state"), &[]).expect("guard");
        let attempt = fake_home.join("attempt");
        assert!(prepare(&guard, &seed, &"0".repeat(64), &attempt).expect_err("digest").contains("digest"));
        assert!(!fake_home.exists());
    }
}

/// Copy a settled directory into a fresh guarded destination, without links.
/// Grading and cache copies are private writable files, never hard links.
pub fn copy_directory(guard: &Guard, source: &Path, destination: &Path) -> Result<(), String> {
    directory(guard, destination)?;
    copy_seed(guard, source, destination)
}

/// Create a new private archive or scratch parent behind the common guard.
pub fn create_directory(guard: &Guard, path: &Path) -> Result<PathBuf, String> {
    let path = guard.write_path(path)?;
    directory(guard, &path)?;
    Ok(path)
}
