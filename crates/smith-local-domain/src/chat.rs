//! One chat's durable activation and transient lifecycle (domain/host.md,
//! sections 2, 8 and 9). The metadata is saved before a new activation or
//! message uses its name. The transient phase is never put in the store.

use crate::ChatState;
use skein_lib::Rng;
use smith_domain::run::Identity;

/// One invocation's chat phase.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    /// Waiting for the store's typed transcript.
    Loading,
    /// Ready for a person line or exit.
    Idle,
    /// Saving names and obtaining grants before the child starts.
    Starting,
    /// Child run is live and can receive named messages.
    Running,
    /// A cancellation was sent; the child's answer is still owed.
    Ending,
    /// The answer arrived; saves and the child end may still be owed.
    Answered { gone: Gone },
    /// Final exit was emitted.
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Gone {
    Owed,
    Heard,
}

/// Durable metadata and one invocation's phase.
#[derive(Debug)]
#[expect(clippy::struct_excessive_bools, reason = "independent terminal rights coexist with each lifecycle phase")]
pub(crate) struct Chat {
    pub(crate) state: ChatState,
    pub(crate) phase: Phase,
    pub(crate) load_requested: bool,
    pub(crate) state_pending: bool,
    pub(crate) start_saved: bool,
    pub(crate) closed: bool,
}

impl Chat {
    pub(crate) fn new(seed: u64) -> Chat {
        let mut random = Rng::new(seed);
        let low = u128::from(random.next_u64());
        let high = u128::from(random.next_u64()).checked_shl(64).expect("fixed upper half shift");
        let identity = Identity::new((low | high).to_le_bytes());
        Chat {
            state: ChatState { identity, activation: 0, next_message: 0, read: None },
            phase: Phase::Loading,
            load_requested: false,
            state_pending: false,
            start_saved: false,
            closed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Chat;

    #[test]
    fn a_new_chat_mints_one_replayable_identity_from_its_own_seed() {
        let first = Chat::new(8084);
        assert_eq!(first.state.identity, Chat::new(8084).state.identity);
        assert_ne!(first.state.identity, Chat::new(8085).state.identity);
    }
}
