//! A host's loop over the fixture owner's set as data (transcript.md, 12).

use skein_codec::patch::{Capacity, Halt, Patcher, Step};
use smith_transcript_patches::patches;

/// Both kept fixture versions are loaded before planning.
const VERSIONS: u32 = 2;
/// The fixture history contains one conversion plan.
const PLANS: u32 = 1;
/// Each fixture schema and patch fits within this text bound.
const TEXT_BYTES: u32 = 256;
/// Version 1: u16 version + two u32 lengths + two 64-byte contents.
pub const RECORD_BYTES: u32 = 138;
/// One record node and its two leaf fields in either version.
pub const NODES: u32 = 3;

pub fn fixture_patcher() -> Patcher {
    let mut patcher = Patcher::new(Capacity {
        versions: VERSIONS,
        patches: PLANS,
        text: TEXT_BYTES,
        bytes: RECORD_BYTES,
        nodes: NODES,
    });
    for text in patches::kept::fixture_turn::TEXTS {
        patcher.keep(text.as_bytes()).expect("kept fixture schema");
    }
    for text in patches::PATCHES {
        patcher.plan(text.as_bytes()).expect("fixture patch plans");
    }
    patcher
}

/// Patches a kept turn to the current version: each plan from its version on,
/// the root found once by `ROOT` and carried to each next version by its
/// counterpart (codec.md, 11.6).
pub fn patch_to_current(patcher: &mut Patcher, record: Box<[u8]>) -> Result<Box<[u8]>, Halt> {
    let version = skein_codec::version_of(&record).expect("a kept turn starts with its version");
    let Some(mut plan) = patcher.plan_from(version) else {
        return Ok(record);
    };
    let schema = patcher.schema(version).expect("the record's kept schema");
    let mut root = schema.find(smith_transcript::ROOT.as_bytes()).expect("the root by its spelling");
    let mut context = patches::Context::default();
    patcher.start(plan, root, record)?;
    loop {
        match patcher.next()? {
            Step::Written { site, at } => {
                let (old, new) = patcher.parts();
                patches::written(site, old, new, at, &mut context)?;
            }
            Step::Held { .. } => panic!("the fixture turn holds no record"),
            Step::Done => match patcher.plan_from(patcher.versions(plan).to) {
                Some(next) => {
                    patcher.finish()?;
                    root = patcher.counterpart(plan, root).expect("the root's counterpart");
                    plan = next;
                    patcher.next_pass(plan, root)?;
                }
                None => return Ok(patcher.finish()?.into()),
            },
        }
    }
}
