//! smith's patch set, as data (transcript.md, 12): hand-written in the shape
//! `skein-codegen patch` generates, and empty until the baseline, whose run
//! of the command (`--baseline`) writes `patches/mod.rs` in its place.

use skein_codec::patch::{Build, Halt, Old, Site};
use skein_codec::value::Node;

mod context {
    /// The owner's per-record inputs to written patch functions (codec.md, 11.5).
    #[derive(Debug)]
    pub struct Context {}

    // Default by contract (codec.md, 11.5; transcript.md, 12), and empty: no
    // live state. Written out, since crates/clippy.toml refuses the derive and
    // clippy reports a derive's disallowed macro at the crate alone, where an
    // expectation would cover every item.
    #[expect(clippy::derivable_impls, reason = "the derive is refused crate-wide; see above")]
    impl Default for Context {
        fn default() -> Self {
            Self {}
        }
    }
}

pub use context::Context;

/// The owner's patch texts in number order, empty before the baseline.
pub const PATCHES: &[&str] = &[];

/// Kept families whose texts a host loads before planning patches.
pub mod kept {
    /// The turn's kept versions, absent before the baseline.
    pub mod smith_transcript {
        /// The family's kept schema texts in version order.
        pub const TEXTS: &[&str] = &[];
    }
}

/// Declared refusals of written patch functions, empty with the set.
pub mod refusal {}

/// Answers a host's written stop with no matching site in this empty set.
pub fn written(site: Site, _old: &Old, _new: &mut Build, _at: Node, _cx: &mut Context) -> Result<(), Halt> {
    Err(Halt::NoSite(site))
}
