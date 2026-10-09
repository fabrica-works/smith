//! The run's retained write holder (domain/run.md, sections 5.3 and 8.1).
//! It follows from the existing call schedule: acquisition asserts the
//! schedule, never waits, and the call's terminal releases it. It knows no
//! paths or locks. A later document changes who may hold it (section 15).

use crate::call::Call;
use skein_lib::{Id, Token};

/// Write authority over the run's overlapping directories, owned until terminal.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Lease {
    /// No child or check holds it: main writes in its own turns.
    Free,
    /// A child granted modify or shell holds it for its call's life.
    Child { call: Id<Call> },
    /// Checks, then their delivery, hold it over the whole workspace.
    Workspace { owner: Token },
}

impl Lease {
    pub(crate) fn assert_free(&self) {
        match self {
            Lease::Free => {}
            Lease::Child { .. } | Lease::Workspace { .. } => {
                unreachable!("one write holder over overlapping directories")
            }
        }
    }

    pub(crate) fn child(&mut self, call: Id<Call>) {
        self.assert_free();
        *self = Lease::Child { call };
    }

    pub(crate) fn child_returned(&mut self, returned: Id<Call>) {
        match self {
            Lease::Child { call } => assert!(*call == returned, "the writing child's own terminal releases its lease"),
            Lease::Free | Lease::Workspace { .. } => unreachable!("a writing child retains its lease until it ends"),
        }
        *self = Lease::Free;
    }

    pub(crate) fn workspace(&mut self, owner: Token) {
        self.assert_free();
        *self = Lease::Workspace { owner };
    }

    pub(crate) fn assert_workspace(&self, expected: Token) {
        match self {
            Lease::Workspace { owner } => assert!(*owner == expected, "checks and delivery share one workspace holder"),
            Lease::Free | Lease::Child { .. } => unreachable!("checks and delivery hold the whole workspace"),
        }
    }

    pub(crate) fn workspace_returned(&mut self, owner: Token) {
        self.assert_workspace(owner);
        *self = Lease::Free;
    }
}
