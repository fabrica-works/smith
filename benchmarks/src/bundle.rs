//! Repository seeds name frozen, vendored directories kept outside the tree.
//! The manifest binds their source pins and digest; an absent bundle is said and
//! skipped offline, and refuses an attempt (benchmarks.md, sections 5.2 and 10).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::isolate::{Setup, SetupStep};
use crate::{Kind, Refusal, Task};

/// A repository seed's bundle contents and digest, supplied by the task author.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BundleManifest {
    /// Directory under `$XDG_STATE_HOME/smith-bench/bundles/`.
    pub name: String,
    /// Frozen source commit.
    pub source: String,
    pub pins: Vec<Pin>,
    /// `seed_digest` of the complete bundle directory.
    pub sha256: String,
}

/// One vendored git dependency's frozen commit, supplied by the task author.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub name: String,
    pub commit: String,
}

/// Offline bundle checking either accepts the directory or reports its absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleCheck {
    /// The present bundle matches its manifest.
    Matches,
    /// The machine has no bundle: the offline check is skipped and said so.
    Absent,
}

/// Read a strict bundle manifest with directory names, git pins and SHA-256 checked.
pub fn read_bundle_manifest(file: &Path) -> Result<BundleManifest, Refusal> {
    let manifest: BundleManifest = crate::formats::read_document(file)?;
    crate::check::check_path(file, "name", Path::new(&manifest.name))?;
    if manifest.name.contains('/')
        || !hex(&manifest.source, 40)
        || !hex(&manifest.sha256, 64)
        || manifest.pins.iter().any(|pin| pin.name.is_empty() || !hex(&pin.commit, 40))
    {
        return Err(Refusal::new(file, "bundle", "one directory name, exact git commits and SHA-256 required"));
    }
    Ok(manifest)
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Accept a present matching bundle; absence is skipped, and any changed bytes refuse.
pub fn check_bundle(manifest: &BundleManifest, bundles: &Path) -> Result<BundleCheck, Refusal> {
    let directory = bundles.join(&manifest.name);
    if !directory.try_exists().map_err(|error| Refusal::new(&directory, "bundle", error.to_string()))? {
        return Ok(BundleCheck::Absent);
    }
    let digest = crate::seed_digest(&directory)?;
    if digest != manifest.sha256 {
        return Err(Refusal::new(&directory, "sha256", "bundle digest differs"));
    }
    Ok(BundleCheck::Matches)
}

/// The copy source and its digest: an ordinary seed, or a repository's checked bundle.
pub fn seed_source(task: &Task, task_directory: &Path, bundles: &Path) -> Result<(PathBuf, String), Setup> {
    let resolve = || {
        let seed = task_directory.join("seed");
        if task.kind != Kind::Repository {
            return Ok((seed, task.seed_sha256.clone()));
        }
        let manifest = read_bundle_manifest(&seed.join("bundle.toml")).map_err(|error| error.to_string())?;
        match check_bundle(&manifest, bundles).map_err(|error| error.to_string())? {
            BundleCheck::Matches => Ok((bundles.join(manifest.name), manifest.sha256)),
            BundleCheck::Absent => Err(format!("repository bundle {} is absent", manifest.name)),
        }
    };
    resolve().map_err(|reason| Setup { step: SetupStep::Seed, reason })
}

pub(crate) fn check_repository_bundles(tasks: &[crate::CatalogueTask], bundles: &Path) -> Result<(), Refusal> {
    for entry in tasks {
        if entry.task.kind != Kind::Repository {
            continue;
        }
        let file = entry.manifest.parent().expect("catalogue manifest parent").join("seed/bundle.toml");
        let manifest = read_bundle_manifest(&file)?;
        if check_bundle(&manifest, bundles)? == BundleCheck::Absent {
            eprintln!("{} bundle {} absent: skipped", entry.name, manifest.name);
        }
    }
    Ok(())
}
