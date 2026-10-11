//! A frozen source ancestry selects the native settings and token codec.
//! The registry adds entries, never edits an earlier binary's invocation.
//! `face` reads only Git's ancestry, not an agent's private state
//! (benchmarks.md, section 6.2; reliability-plan 00, Interface sketches).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::arm::BuiltArm;
use crate::credential::TokenShape;
use crate::isolate::{Setup, SetupStep};

use super::legacy::{FactShape, Shape};

/// A binary's outside observation vocabulary, selected by its source ancestry.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "face", content = "shape", rename_all = "kebab-case")]
pub enum Face {
    /// Structured Debug records from the local host's spawned agent.
    Legacy(Shape),
    /// Versioned records from the binary's selected event front end.
    Events(super::events::Shape),
}

/// Immutable receiving bounds pinned with the face that writes the stream.
/// The caller supplies semantic bounds; overflow refuses before inference.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receiving {
    pub string: u32,
    pub content: u32,
    pub items: u32,
}

impl Receiving {
    /// Preserve each bound exactly and check the codec's complete line capacity.
    pub fn limits(self) -> Result<smith_events::Limits, String> {
        let limits = smith_events::Limits { string: self.string, content: self.content, items: self.items };
        if self.string == 0 || self.content == 0 || self.items == 0 || smith_events::largest_record(&limits).is_none() {
            return Err("event receiving bounds do not fit the codec's complete line capacity".into());
        }
        Ok(limits)
    }
}

/// How this arm receives its credential without touching the source login.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "credential", content = "record", rename_all = "kebab-case")]
pub enum CredentialShape {
    /// A private converted record in this arm's frozen saved-token codec.
    Converted(TokenShape),
}

/// An immutable registry entry from which this native face applies.
#[derive(Clone, Debug, Serialize)]
pub struct FaceSince {
    pub commit: String,
    pub face: Face,
    pub credential: CredentialShape,

    pub facts: FactShape,
    pub receiving: Option<Receiving>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FaceKind {
    Legacy,
    Events,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EntryShape {
    Legacy(Shape),
    Events(super::events::Shape),
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CredentialKind {
    Converted,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    commit: String,
    face: FaceKind,
    shape: EntryShape,
    credential: CredentialKind,
    record: TokenShape,
    #[serde(default)]
    facts: FactShape,
    #[serde(default)]
    receiving: Option<Receiving>,
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
            let face = match entry.face {
                FaceKind::Legacy => match entry.shape {
                    EntryShape::Legacy(shape) => {
                        if entry.receiving.is_some() {
                            return Err("legacy face cannot declare event receiving bounds".into());
                        }
                        Face::Legacy(shape)
                    }
                    EntryShape::Events(_) => return Err("legacy face requires a legacy invocation shape".into()),
                },
                FaceKind::Events => match entry.shape {
                    EntryShape::Events(shape) => {
                        entry.receiving.ok_or("events face requires pinned receiving bounds")?.limits()?;
                        Face::Events(shape)
                    }
                    EntryShape::Legacy(_) => return Err("events face requires an events invocation shape".into()),
                },
            };
            faces.push(FaceSince {
                commit: entry.commit,
                face,
                credential: match entry.credential {
                    CredentialKind::Converted => CredentialShape::Converted(entry.record),
                },
                facts: entry.facts,
                receiving: entry.receiving,
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
        for entry in faces {
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
                Some(0) => selected = Some(entry.clone()),
                Some(1) => {}
                Some(_) | None => return Err("cannot establish Smith arm source ancestry".into()),
            }
        }
        selected.ok_or_else(|| "no native face supports this Smith arm".into())
    })();
    select.map_err(|reason| Setup { step: SetupStep::Configuration, reason })
}
