//! Independent final-request allowance at the actual run/provider boundaries.
//! It retains admitted quotas and actual reservations, never run state or facts.
//! Contract: domain/run.md, sections 9.1 and 13.

use skein_lib::{Time, Token};
use skein_world::domain::{Expectations, Judge, Referee, Verdict};
use std::collections::BTreeMap;

/// An external admission, actual provider reservation or authoritative answer.
#[derive(Clone, Copy, Debug)]
pub enum Seen {
    /// The host's admitted final-completion allowance.
    Admitted { reserve: u32 },
    /// A final provider request actually reserved credit.
    Final { main: bool, live_children: u32 },
    /// The actual answer names a turns failure or a full wall-time failure.
    Answer { turns_failure: bool, time_failure: bool, time_expired: bool },
}

/// Count accepted final provider reservations independently of completion usage.
#[derive(Default)]
pub struct Meeting {
    reserve: u32,
    finals: u32,
}

impl Expectations for Meeting {
    type Seen = Seen;
    type Name = &'static str;
    type Stimulus = ();

    fn observe(&mut self, seen: Seen, judge: &mut Judge<Self::Name, Self::Stimulus>) {
        match seen {
            Seen::Admitted { reserve } => self.reserve = reserve,
            Seen::Final { main, live_children } => {
                judge.check(main && live_children == 0, "only main starts final work after every child has ended");
                self.finals += 1;
                judge.check(self.finals <= self.reserve, "final requests never exceed the admitted reserve");
            }
            Seen::Answer { turns_failure, time_failure, time_expired } => {
                judge.check(
                    !turns_failure || self.finals >= self.reserve,
                    "no turns failure before main's last final request",
                );
                judge.check(!time_failure || time_expired, "wind-down time failure waits for the full run deadline");
            }
        }
    }
}

/// Skein judges each admitted run's ordered external evidence.
#[must_use]
pub fn judge(seen: &[(Time, Token, Seen)]) -> Vec<Verdict> {
    let mut meetings = BTreeMap::new();
    for (at, run, seen) in seen {
        let meeting = meetings.entry(*run).or_insert_with(|| Referee::new(Meeting::default()));
        meeting.observe(*at, *seen, &mut Vec::new());
    }
    meetings.values().map(Referee::verdict).collect()
}
