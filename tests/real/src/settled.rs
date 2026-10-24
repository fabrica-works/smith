//! Expected durable changes for these Smith scratch stories (testing.md,
//! section 6; protocol/hosts.md, sections 5.3–5.4). Only named chat, token and
//! checkout outputs may change; original records and fixture files stay put.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use smith_local_domain::{DeliveryState, Event, Fact};
use smith_local_process_world::world::Scenario;
use smith_local_process_world::{Placement, referee::Seen};

use crate::git;

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    mode: u32,
    bytes: Option<Vec<u8>>,
}

pub struct Before {
    entries: BTreeMap<PathBuf, Entry>,
    head: Vec<u8>,
}

fn entries(root: &Path, directory: &Path, out: &mut BTreeMap<PathBuf, Entry>) {
    for entry in std::fs::read_dir(directory).expect("scratch directory") {
        let path = entry.expect("scratch entry").path();
        let metadata = std::fs::symlink_metadata(&path).expect("scratch metadata");
        assert!(metadata.is_file() || metadata.is_dir(), "scratch has only regular files and directories");
        let bytes = metadata.is_file().then(|| std::fs::read(&path).expect("scratch bytes"));
        out.insert(
            path.strip_prefix(root).expect("inside scratch").to_path_buf(),
            Entry { mode: metadata.permissions().mode() & 0o7777, bytes },
        );
        if metadata.is_dir() {
            entries(root, &path, out);
        }
    }
}

impl Before {
    pub fn new(root: &Path) -> Self {
        let mut snapshot = BTreeMap::new();
        entries(root, root, &mut snapshot);
        Self { entries: snapshot, head: git(&root.join("repo"), &["rev-parse", "HEAD"]) }
    }

    pub(crate) fn assert_expected(&self, scenario: &Scenario, seen: &Seen) {
        self.assert_outputs(scenario, seen, None);
    }

    /// A binary adds its generated configuration and configured trace to the shared scratch contract.
    pub fn assert_binary_expected(
        &self,
        scenario: &Scenario,
        seen: &Seen,
        resolved: &smith_local_shell::settings::resolve::Resolved,
    ) {
        self.assert_outputs(scenario, seen, Some(resolved));
    }

    fn assert_outputs(
        &self,
        scenario: &Scenario,
        seen: &Seen,
        resolved: Option<&smith_local_shell::settings::resolve::Resolved>,
    ) {
        let binary = resolved.is_some();
        let root = &scenario.launch.root_path;
        let mut after = BTreeMap::new();
        entries(root, root, &mut after);
        let (chat, endpoints) = if let Some(resolved) = resolved {
            let workspace = root.canonicalize().unwrap();
            let digest = skein_io::digest::digest(workspace.as_os_str().as_encoded_bytes());
            let key = digest.0.iter().map(|part| format!("{part:016x}")).collect::<String>();
            let chat =
                PathBuf::from("state/smith/chats").join(key).join(std::str::from_utf8(&resolved.local.chat).unwrap());
            (chat, resolved.agent.service.channel_endpoints.clone())
        } else {
            (PathBuf::from("chat"), smith_local_process_world::process::lower_configuration().channel_endpoints)
        };
        let mut expected = BTreeSet::from([chat.join("state")]);
        let marker = chat.join("format");
        let current = smith_local_protocol::format::file();
        assert_eq!(
            after.get(&marker).and_then(|entry| entry.bytes.as_deref()),
            Some(current.bytes.as_ref()),
            "the settled chat records this build's layouts"
        );
        if !self.entries.contains_key(&marker) {
            expected.insert(marker);
        }
        if binary {
            expected.insert(PathBuf::from("agent-trace.jsonl"));
        } else if scenario.launch.placement == Placement::Spawned {
            expected.extend([PathBuf::from("agent.json"), PathBuf::from("launch.json")]);
        }
        let Event::Loaded { state, transcript, deliveries } =
            smith_local_process_world::world::load_chat(&root.join(&chat), &endpoints)
        else {
            unreachable!("chat codecs return Loaded")
        };
        assert!(state.is_some());
        let transcript = transcript.expect("a settled chat has durable turns");
        let old_turns = self
            .entries
            .keys()
            .filter(|path| {
                path.parent() == Some(chat.as_path()) && path.extension().is_some_and(|extension| extension == "turn")
            })
            .count();
        assert_eq!(
            transcript.turns.len(),
            old_turns
                + if binary {
                    seen.queries.len()
                } else {
                    seen.facts.iter().filter(|fact| matches!(fact, Fact::Turn { .. })).count()
                },
            "only this invocation's observed turns were appended"
        );
        let mut returned = BTreeSet::new();
        for fact in &seen.facts {
            if let Fact::DeliveryReturned { name } = fact {
                returned.insert(
                    chat.join(format!("{:020}.{:010}.{:010}", name.activation, name.completion, name.position))
                        .to_str()
                        .unwrap()
                        .to_owned(),
                );
            }
        }
        let mut new_deliveries = BTreeSet::new();
        for turn in &transcript.turns {
            let path = chat.join(format!("{:010}.turn", turn.sequence));
            // A resumed run appends; already durable turns cannot be rewritten.
            if !self.entries.contains_key(&path) {
                expected.insert(path);
            }
        }
        for delivery in &deliveries {
            assert!(matches!(delivery.state, DeliveryState::Answer(_)), "every durable intent has its terminal");
            let name = delivery.name;
            let path = chat.join(format!("{:020}.{:010}.{:010}", name.activation, name.completion, name.position));
            if !self.entries.contains_key(&path) {
                new_deliveries.insert(path.to_str().expect("fixture record name").to_owned());
                expected.insert(path);
            }
        }
        if binary {
            assert_eq!(
                new_deliveries.len(),
                usize::from(scenario.launch.change),
                "only the expected final delivery is persisted"
            );
        } else {
            assert_eq!(new_deliveries, returned, "only observed delivery terminals were persisted");
        }
        if scenario.launch.authenticated {
            expected.insert(PathBuf::from(if binary { "credentials/tokens/fake" } else { "tokens/0.json" }));
        }
        let repo = root.join("repo");
        if scenario.launch.change {
            for name in ["original.txt", "result.txt", "command.txt", "checks-ran.txt"] {
                expected.insert(PathBuf::from("repo").join(name));
            }
            let reference = git(&repo, &["symbolic-ref", "HEAD"]);
            let reference = std::str::from_utf8(reference.trim_ascii_end()).expect("fixture branch");
            for name in ["index", "COMMIT_EDITMSG", "logs/HEAD", reference] {
                expected.insert(PathBuf::from("repo/.git").join(name));
            }
            expected.insert(PathBuf::from("repo/.git/logs").join(reference));
            let previous = format!("^{}", std::str::from_utf8(self.head.trim_ascii_end()).expect("initial object id"));
            let objects = git(&repo, &["rev-list", "--objects", "HEAD", &previous]);
            for object in objects.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()) {
                let hash = object.split(|byte| *byte == b' ').next().expect("object id");
                let hash = std::str::from_utf8(hash).expect("hex object id");
                expected.insert(PathBuf::from(format!("repo/.git/objects/{}/{}", &hash[..2], &hash[2..])));
            }
        }
        let mut directories = BTreeSet::new();
        if scenario.launch.authenticated {
            directories.insert(PathBuf::from(if binary {
                "credentials/tokens/fake.lock"
            } else {
                "tokens/0.json.lock"
            }));
        }
        for path in &expected {
            let mut parent = path.parent();
            while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
                directories.insert(path.to_path_buf());
                parent = path.parent();
            }
        }
        for (path, before) in &self.entries {
            let after = after.get(path).unwrap_or_else(|| panic!("scratch removed {path:?}"));
            if !expected.contains(path) {
                assert_eq!(before, after, "scratch changed only its expected outputs: {path:?}");
            }
        }
        for (path, entry) in &after {
            if !self.entries.contains_key(path) {
                assert!(
                    expected.contains(path) || entry.bytes.is_none() && directories.contains(path),
                    "unexpected scratch addition: {path:?}"
                );
            }
        }
        for path in expected {
            assert!(after.get(&path).is_some_and(|entry| entry.bytes.is_some()), "expected durable file {path:?}");
        }
        assert!(
            git(&repo, &["status", "--porcelain", "--untracked-files=all"]).is_empty(),
            "checkout settled with no uncommitted changes"
        );
    }
}

/// Outside observation after the loop drained: no actual git, rg, sh or check
/// child remains, including a zombie awaiting wait. Hosted agents use the
/// shared world's own process/descriptor ledger and exit assertions.
pub fn assert_no_children() {
    for task in std::fs::read_dir("/proc/self/task").expect("Linux task observations") {
        let path = task.expect("task entry").path().join("children");
        match std::fs::read_to_string(path) {
            Ok(children) => assert!(children.trim().is_empty(), "every actual child was reaped: {children}"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("actual child observation: {error}"),
        }
    }
}
