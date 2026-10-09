//! The budget's finishing phase and deferred child closures (domain/run.md,
//! section 9.1). This keeps the exhausted ordinary part, final requests made,
//! and bounded ready runs. It never knows provider syntax or child results.

use skein_lib::{Id, Set};

use crate::Exhausted;
use crate::run::Run;

/// Where a working run is after its ordinary pool is spent.
#[derive(Debug)]
pub(crate) enum WindDown {
    /// Child terminals settle before main's final completion.
    Settling { children: u32, exhausted: Exhausted },
    /// Main requests at most the reserve's turns; its first gate carries a note.
    Finishing { used: u32, noted: bool, exhausted: Exhausted },
}

impl WindDown {
    pub(crate) const fn new(children: u32, exhausted: Exhausted) -> WindDown {
        if children == 0 {
            WindDown::Finishing { used: 0, noted: false, exhausted }
        } else {
            WindDown::Settling { children, exhausted }
        }
    }

    pub(crate) const fn exhausted(&self) -> Exhausted {
        match self {
            WindDown::Settling { exhausted, .. } | WindDown::Finishing { exhausted, .. } => *exhausted,
        }
    }

    pub(crate) fn child_ended(&mut self) -> bool {
        match self {
            WindDown::Settling { children, exhausted } => {
                *children = children.checked_sub(1).expect("one terminal per retained child");
                if *children == 0 {
                    *self = WindDown::Finishing { used: 0, noted: false, exhausted: *exhausted };
                    true
                } else {
                    false
                }
            }
            WindDown::Finishing { .. } => false,
        }
    }
}

/// Runs whose child closures wait until the next reclaim point.
#[derive(Debug)]
pub(crate) struct Ready {
    now: Set<Id<Run>>,
    next: Set<Id<Run>>,
}

impl Ready {
    pub(crate) const fn with_capacity(runs: u32) -> Ready {
        Ready { now: Set::with_capacity(runs), next: Set::with_capacity(runs) }
    }

    pub(crate) fn worst_case(runs: u32) -> Option<u64> {
        Set::<Id<Run>>::worst_case(runs)?.checked_mul(2)
    }

    pub(crate) fn is_ready(&self) -> bool {
        !self.now.is_empty()
    }

    pub(crate) fn pop(&mut self) -> Option<Id<Run>> {
        self.now.pop_first()
    }

    pub(crate) fn keep(&mut self, run: Id<Run>) {
        self.next.insert(run).expect("one ready entry per run");
    }

    pub(crate) fn remove(&mut self, run: Id<Run>) {
        self.now.remove(&run);
        self.next.remove(&run);
    }

    pub(crate) fn promote(&mut self) {
        for _ in 0..self.next.capacity() {
            let Some(run) = self.next.pop_first() else {
                break;
            };
            self.now.insert(run).expect("one ready entry per run");
        }
    }
}
