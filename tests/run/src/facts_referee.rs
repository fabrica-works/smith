//! Independent cumulative budget-fact oracle, using host ceilings and real callbacks.
//! Contract: domain/run.md, sections 9.1, 11 and 13.

use skein_lib::{Duration, Time};
use skein_world::domain::{Expectations, Judge, Referee, Verdict};
use smith_domain_run::{Budget, Spend, charter::Reserve, facts::Standing};

/// A host allowance and cumulative actual callback accounting, observed outside the domain.
#[derive(Clone, Copy, Debug)]
pub struct Charged {
    pub spent: Spend,
    pub budget: Budget,
    pub reserve: Reserve,
    pub reserved_spend: u64,
    pub time: Duration,
}

/// A real budget fact paired with its independent callback observation.
#[derive(Clone, Copy, Debug)]
pub struct Seen {
    pub charged: Charged,
    pub standing: Standing,
}

/// No domain state; each fact must reproduce the external charges and host limits.
#[derive(Default)]
pub struct Meeting;

impl Expectations for Meeting {
    type Seen = Seen;
    type Name = &'static str;
    type Stimulus = ();

    fn observe(&mut self, seen: Seen, judge: &mut Judge<Self::Name, Self::Stimulus>) {
        let got = seen.standing;
        let outside = seen.charged;
        judge.check(
            got.turns == outside.spent.turns
                && got.spend == outside.spent.units
                && got.time.as_nanos().is_multiple_of(1_000_000)
                && got.time >= outside.time
                && got.time.as_nanos().saturating_sub(outside.time.as_nanos()) < 1_000_000,
            "budget facts reproduce actual completion callbacks and the injected clock",
        );
        judge.check(
            got.turns_limit == outside.budget.turns
                && got.spend_limit == outside.budget.spend
                && got.time_limit == outside.budget.time
                && got.reserve == outside.reserve,
            "budget facts reproduce the admitted host ceiling and reserve",
        );
        judge.check(
            got.reserve_spend == outside.reserved_spend,
            "reserve spend follows the admitted model rates and bounds",
        );
    }
}

/// Judge actual fact/callback pairs using Skein's shared referee.
#[must_use]
pub fn judge(seen: &[(Time, Seen)]) -> Verdict {
    let mut referee = Referee::new(Meeting);
    for (at, value) in seen {
        referee.observe(*at, *value, &mut Vec::new());
    }
    referee.verdict()
}
