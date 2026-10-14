//! Attempt preparation keeps private paths and an exact environment, no login.
//! `prepare` verifies and copies the frozen seed before inference;
//! `environment` gives commands only machine tools, private homes and explicit
//! task variables (benchmarks.md, sections 6.1 and 10). Setup and measurement
//! belong to the runner; this module reads no clock or agent configuration.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use crate::guard::Guard;

/// One user-owned temporary entry, without following links or reading contents.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TemporaryEntry {
    kind: fs::FileType,
    bytes: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
}

/// List the user's entries directly under /tmp, outside this attempt's private root.
pub(crate) fn temporary_entries(private: &Path) -> Result<BTreeMap<PathBuf, TemporaryEntry>, String> {
    let owner = fs::metadata("/proc/self").map_err(|error| error.to_string())?.uid();
    let mut entries = BTreeMap::new();
    for entry in fs::read_dir("/tmp").map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.starts_with(private) {
            continue;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        if metadata.uid() == owner {
            entries.insert(
                path,
                TemporaryEntry {
                    kind: metadata.file_type(),
                    bytes: metadata.len(),
                    modified_seconds: metadata.mtime(),
                    modified_nanoseconds: metadata.mtime_nsec(),
                },
            );
        }
    }
    Ok(entries)
}

/// Retain the path and both metadata records for each new or changed user entry.
pub(crate) fn temporary_changes(
    before: Result<BTreeMap<PathBuf, TemporaryEntry>, String>,
    private: &Path,
) -> Result<Vec<(PathBuf, String)>, String> {
    let before = before?;
    let after = temporary_entries(private)?;
    Ok(after
        .into_iter()
        .filter(|(path, entry)| before.get(path) != Some(entry))
        .map(|(path, entry)| {
            let evidence = format!("{}: before {:?}, after {entry:?}", path.display(), before.get(&path));
            (path, evidence)
        })
        .collect())
}

/// The preparation step that refused an attempt before inference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupStep {
    /// The fresh private scratch paths could not be made.
    Scratch,
    /// The frozen seed could not be validated or materialised.
    Seed,
    /// The author's warm-up or private cache copy failed.
    Warm,
    /// Available disk space could not satisfy the attempt's estimate.
    Space,
    /// The pinned invocation or its exact environment was refused.
    Configuration,
    /// The adapter found an absent or lapsed existing login.
    Credentials,
}

/// A preparation refusal, rendered by the runner into the result's setup end.
#[derive(Debug)]
pub struct Setup {
    pub step: SetupStep,
    pub reason: String,
}

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

/// Resolve an author's command through the exact machine PATH before spawn.
/// Skein executes the supplied path directly; no inherited PATH is consulted.
pub fn program(machine: &Machine, working_directory: &Path, name: &Path) -> Result<PathBuf, String> {
    let candidates = if name.components().count() > 1 || name.is_absolute() {
        vec![working_directory.join(name)]
    } else {
        std::env::split_paths(&machine.path).map(|directory| working_directory.join(directory).join(name)).collect()
    };
    for path in candidates {
        if path.is_file() {
            // Tool proxies such as rustup choose their command from argv[0].
            return Ok(path);
        }
    }
    Err(format!("command not found in the exact PATH: {}", name.display()))
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
    pub warm_cargo: PathBuf,
    pub warm_target: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
}

/// Resolve an authored seed-relative working directory before credentials.
pub fn working_directory(guard: &Guard, private: &Private, task: &crate::Task) -> Result<PathBuf, String> {
    let Some(path) = &task.working_directory else {
        return Ok(private.workspace.clone());
    };
    if path.as_os_str().is_empty() || path.components().any(|part| !matches!(part, Component::Normal(_))) {
        return Err("task working_directory must be a seed-relative path without traversal".into());
    }
    let workspace = private.workspace.canonicalize().map_err(|error| error.to_string())?;
    let directory = guard.write_path(&workspace.join(path))?.canonicalize().map_err(|error| error.to_string())?;
    if !directory.is_dir() || !directory.starts_with(&workspace) {
        return Err("task working_directory must be an existing directory inside the private seed".into());
    }
    Ok(directory)
}

/// Delete settled owned scratch, restoring read-only directory modes without following links.
pub fn remove_owned_tree(guard: &Guard, root: &Path) -> Result<(), String> {
    fn writable(guard: &Guard, directory: &Path) -> Result<(), String> {
        let metadata = fs::symlink_metadata(directory).map_err(|error| error.to_string())?;
        if !metadata.is_dir() || metadata.is_symlink() {
            return Err("owned scratch root must be an ordinary directory".into());
        }
        guard.write_path(directory)?;
        fs::set_permissions(directory, fs::Permissions::from_mode(metadata.permissions().mode() | 0o700))
            .map_err(|error| error.to_string())?;
        for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            if entry.file_type().map_err(|error| error.to_string())?.is_dir() {
                writable(guard, &entry.path())?;
            }
        }
        Ok(())
    }
    writable(guard, root)?;
    guard.write_path(root)?;
    fs::remove_dir_all(root).map_err(|error| error.to_string())
}

impl Private {
    pub(crate) fn paths(root: &Path) -> Self {
        Self {
            root: root.into(),
            workspace: root.join("workspace"),
            home: root.join("home"),
            temporary: root.join("tmp"),
            cargo_home: root.join("cargo"),
            target: root.join("target"),
            warm_cargo: root.join("warm-cargo"),
            warm_target: root.join("warm-target"),
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
        if entry.file_name() == ".git" {
            continue;
        }
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
pub fn prepare(guard: &Guard, seed: &Path, sha256: &str, root: &Path) -> Result<Private, Setup> {
    let mut step = SetupStep::Seed;
    let materialised = (|| {
        if crate::seed_digest(seed).map_err(|error| error.to_string())? != sha256 {
            return Err("seed digest differs; setup refused before materialisation".into());
        }
        step = SetupStep::Scratch;
        let paths = empty(guard, root)?;
        step = SetupStep::Seed;
        copy_seed(guard, seed, &paths.workspace)?;
        if crate::seed_digest(&paths.workspace).map_err(|error| error.to_string())? != sha256 {
            return Err("materialised seed digest differs; setup refused".into());
        }
        Ok(paths)
    })();
    materialised.map_err(|reason| Setup { step, reason })
}

/// Create fresh private command paths with an initially empty workspace.
pub fn empty(guard: &Guard, root: &Path) -> Result<Private, String> {
    let paths = Private::paths(&guard.write_path(root)?);
    directory(guard, &paths.root)?;
    for path in [
        &paths.workspace,
        &paths.home,
        &paths.temporary,
        &paths.cargo_home,
        &paths.target,
        &paths.warm_cargo,
        &paths.warm_target,
        &paths.config,
        &paths.state,
    ] {
        directory(guard, path)?;
    }
    Ok(paths)
}

/// Populate a caller-created private workspace without following links.
pub fn copy_into(guard: &Guard, source: &Path, destination: &Path) -> Result<(), String> {
    guard.write_path(destination)?;
    if fs::read_dir(destination).map_err(|error| error.to_string())?.next().is_some() {
        return Err("copy destination must be empty".into());
    }
    copy_seed(guard, source, destination)
}

/// Restore the machine PATH when a login shell reads this private home.
/// No inherited profile or instruction file is copied (benchmarks.md, section 6.1).
pub(crate) fn login_profile(guard: &Guard, machine: &Machine, private: &Private) -> Result<(), String> {
    let path = machine.path.as_bytes();
    if path.contains(&0) {
        return Err("machine PATH contains a NUL".into());
    }
    let mut contents = b"export PATH='".to_vec();
    for byte in path {
        if *byte == b'\'' {
            contents.extend_from_slice(b"'\\''");
        } else {
            contents.push(*byte);
        }
    }
    contents.extend_from_slice(b"'\n");
    let destination = guard.write_path(&private.home.join(".profile"))?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)
        .and_then(|mut file| file.write_all(&contents))
        .map_err(|error| format!("private login profile: {error}"))
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

/// Make a settled writable copy private, rejecting links before changing modes.
pub fn restrict_tree(guard: &Guard, path: &Path) -> Result<(), String> {
    let path = guard.write_path(path)?;
    let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
    let mode = if metadata.is_symlink() {
        return Err("private copy contains a link".into());
    } else if metadata.is_dir() {
        0o700
    } else if metadata.is_file() {
        if metadata.permissions().mode() & 0o111 == 0 { 0o600 } else { 0o700 }
    } else {
        return Err("private copy contains a special node".into());
    };
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).map_err(|error| error.to_string())?;
    if metadata.is_dir() {
        for child in fs::read_dir(&path).map_err(|error| error.to_string())? {
            restrict_tree(guard, &child.map_err(|error| error.to_string())?.path())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_keeps_the_name_of_its_tool_proxy() {
        let root = std::env::temp_dir().join(format!("smith-bench-proxy-{}", std::process::id()));
        fs::create_dir(&root).expect("fresh proxy directory");
        std::os::unix::fs::symlink("/bin/sh", root.join("cargo")).expect("tool proxy");
        let machine = Machine { path: root.as_os_str().into(), rustup_home: None };
        assert_eq!(program(&machine, &root, Path::new("cargo")).expect("proxy"), root.join("cargo"));
        fs::remove_dir_all(root).expect("remove proxy directory");
    }

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
        let refused = prepare(&guard, &seed, &"0".repeat(64), &attempt).expect_err("digest");
        assert_eq!(refused.step, SetupStep::Seed);
        assert!(refused.reason.contains("digest"));
        assert!(!fake_home.exists());
    }
}

#[cfg(test)]
#[test]
fn readonly_owned_cleanup_restores_directory_permissions_and_never_follows_links() {
    let root = std::env::temp_dir().join(format!("smith-readonly-cleanup-{}", std::process::id()));
    let guard = Guard::new(&root.join("user"), &root.join("config"), &root.join("state"), &[]).expect("fixture guard");
    let owned = root.join("owned");
    let readonly = owned.join("spec");
    let outside = root.join("outside");
    fs::create_dir_all(&readonly).expect("owned layout");
    fs::create_dir_all(&outside).expect("owned outside control");
    fs::write(readonly.join("rules.txt"), "frozen").expect("readonly file");
    fs::write(outside.join("sentinel.txt"), "unchanged").expect("outside sentinel");
    std::os::unix::fs::symlink(&outside, owned.join("link")).expect("outside link");
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o500)).expect("outside mode control");
    fs::set_permissions(&readonly, fs::Permissions::from_mode(0o500)).expect("readonly mode");
    assert!(fs::remove_file(readonly.join("rules.txt")).is_err(), "ordinary unlink cannot delete readonly layout");
    remove_owned_tree(&guard, &owned).expect("settled cleanup");
    assert!(!owned.exists());
    assert_eq!(fs::metadata(&outside).expect("outside unchanged").permissions().mode() & 0o777, 0o500);
    assert_eq!(fs::read_to_string(outside.join("sentinel.txt")).expect("outside unchanged"), "unchanged");
    let root_link = root.join("root-link");
    std::os::unix::fs::symlink(&outside, &root_link).expect("nonordinary root");
    assert!(remove_owned_tree(&guard, &root_link).is_err());
    assert_eq!(fs::read_to_string(outside.join("sentinel.txt")).expect("outside still unchanged"), "unchanged");
    fs::remove_file(root_link).expect("unlink fixture root link");
    remove_owned_tree(&guard, &root).expect("owned fixture cleanup");
}
