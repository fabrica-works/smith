//! Independent effective-share oracle at actual child openings.
//! The world reconstructs ordinary credit from host budgets, time and usage.
//! Contract: domain/run.md, sections 5.3, 9.1 and 13.

use skein_lib::{Duration, Time};
use skein_world::domain::{Expectations, Judge, Referee, Verdict};
use smith_domain_run::Budget;

/// An actual child share and independently observed ordinary credit when asked.
#[derive(Clone, Copy, Debug)]
pub struct Seen {
    pub ordinary: Budget,
    pub effective: Budget,
    pub asked: smith_domain_run::Share,
    pub call_left: Duration,
}

/// No domain state: each actual opening must fit its observed ordinary credit.
#[derive(Default)]
pub struct Meeting;

impl Expectations for Meeting {
    type Seen = Seen;
    type Name = &'static str;
    type Stimulus = ();

    fn observe(&mut self, seen: Seen, judge: &mut Judge<Self::Name, Self::Stimulus>) {
        judge.check(
            seen.effective.turns <= seen.ordinary.turns
                && seen.effective.spend <= seen.ordinary.spend
                && seen.effective.time <= seen.ordinary.time,
            "a child's effective share stays above the finishing reserve",
        );
        let turns = seen.asked.turns.unwrap_or(seen.ordinary.turns).min(seen.ordinary.turns);
        let time = seen.asked.time.unwrap_or(seen.ordinary.time).min(seen.ordinary.time).min(seen.call_left);
        judge.check(
            seen.effective.turns == turns && seen.effective.time == time && seen.effective.spend == seen.ordinary.spend,
            "a child's turns and time equal the asked parts clamped to ordinary credit and the call deadline",
        );
    }
}

/// Judge the actual opening observations with Skein's referee.
#[must_use]
pub fn judge(seen: &[(Time, Seen)]) -> Verdict {
    let mut referee = Referee::new(Meeting);
    for (at, opening) in seen {
        referee.observe(*at, *opening, &mut Vec::new());
    }
    referee.verdict()
}
