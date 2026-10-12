//! A frozen source ancestry selects the native settings and token codec.
//! The registry adds entries, never edits an earlier binary's invocation.
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

/// A registry entry from which this native face applies.
#[derive(Clone, Debug, Serialize)]
pub struct FaceSince {
    pub commit: String,
    pub shape: super::events::Shape,
    pub receiving: Receiving,
    /// This writer's session terminal carries the teardown-aborted field.
    pub session_ended_aborted: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    commit: String,
    shape: super::events::Shape,
    receiving: Receiving,
    #[serde(default)]
    session_ended_aborted: Option<bool>,
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
        let mut session_ended_aborted = false;
        for entry in registry.face {
            session_ended_aborted = entry.session_ended_aborted.unwrap_or(session_ended_aborted);
            entry.receiving.limits()?;
            faces.push(FaceSince {
                commit: entry.commit,
                shape: entry.shape,
                receiving: entry.receiving,
                session_ended_aborted,
            });
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
        selected.ok_or_else(|| "no native face supports this Smith arm".into())
    })();
    select.map_err(|reason| Setup { step: SetupStep::Configuration, reason })
}
