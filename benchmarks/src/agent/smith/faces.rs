//! Source ancestry selects the shipped product invocation and receiving bounds.
//! The registry supports only binaries descended from the first product face.
//! `face` reads only Git's ancestry, not an agent's private state
//! (benchmarks.md, section 6.2; reliability-plan 00, Interface sketches).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::arm::BuiltArm;
use crate::isolate::{Setup, SetupStep};

/// Receiving bounds pinned with the face that writes the stream.
/// The arm supplies semantic caps and the writer-derived aggregate line ceiling.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receiving {
    pub string: u32,
    pub content: u32,
    pub items: u32,
    /// The writer's complete encoded line in bytes, including its newline.
    pub encoded_line: u32,
}

impl Receiving {
    /// Preserve semantic caps exactly; the separate aggregate ceiling must be nonzero.
    pub fn limits(self) -> Result<smith_events::Limits, String> {
        let limits = smith_events::Limits { string: self.string, content: self.content, items: self.items };
        if self.string == 0 || self.content == 0 || self.items == 0 || self.encoded_line == 0 {
            return Err("event receiving semantic caps and encoded-line ceiling must be positive".into());
        }
        Ok(limits)
    }
}

/// A registry entry from which this product face applies.
#[derive(Clone, Debug, Serialize)]
pub struct FaceSince {
    pub commit: String,
    pub shape: super::events::Shape,
    pub receiving: Receiving,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    commit: String,
    shape: super::events::Shape,
    receiving: Receiving,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    face: Vec<Entry>,
}

/// Read the bounded, closed registry beside the configuration pin.
pub fn read(path: &Path) -> Result<Vec<FaceSince>, Setup> {
    let load: Result<Vec<FaceSince>, String> = (|| {
        let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
        if bytes.len() > 65_536 {
            return Err("Smith face registry exceeds its bound".into());
        }
        let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
        let registry: Registry = toml::from_str(text).map_err(|error| error.to_string())?;
        if registry.face.is_empty() || registry.face.len() > 64 {
            return Err("Smith face registry has an invalid inventory".into());
        }
        for entry in &registry.face {
            if entry.commit.len() != 40
                || !entry.commit.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err("Smith face registry requires full source commits".into());
            }
        }
        let mut faces = Vec::new();
        for entry in registry.face {
            entry.receiving.limits()?;
            faces.push(FaceSince { commit: entry.commit, shape: entry.shape, receiving: entry.receiving });
        }
        Ok(faces)
    })();
    load.map_err(|reason| Setup { step: SetupStep::Configuration, reason })
}

/// Choose the newest entry that is an ancestor of this verified built arm.
pub fn face(arm: &BuiltArm, faces: &[FaceSince], repository: &Path) -> Result<FaceSince, Setup> {
    let select: Result<FaceSince, String> = (|| {
        let mut selected = None;
        for entry in faces.iter().rev() {
            let result = std::process::Command::new("git")
                .args(["-c", "core.hooksPath=/dev/null", "-C"])
                .arg(repository)
                .args(["merge-base", "--is-ancestor", &entry.commit, &arm.commit])
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .env_remove("GIT_OBJECT_DIRECTORY")
                .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
                .stdin(std::process::Stdio::null())
                .output()
                .map_err(|error| error.to_string())?;
            match result.status.code() {
                Some(0) => {
                    selected = Some(entry.clone());
                    break;
                }
                Some(1) => {}
                Some(_) | None => return Err("cannot establish Smith arm source ancestry".into()),
            }
        }
        selected.ok_or_else(|| format!("Smith arm {} predates the first supported product face", arm.commit))
    })();
    select.map_err(|reason| Setup { step: SetupStep::Configuration, reason })
}
