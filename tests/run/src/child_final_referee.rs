//! Independent own-turn-share terminal rule over actual run facts.
//! It keeps only effective child grants, never session state or production triggers.
//! Contract: domain/run.md, sections 5.3 and 13; testing-strategy.md, section 7.

use std::collections::BTreeMap;

use skein_lib::Token;
use skein_world::domain::{Expectations, Judge, Referee, Verdict};
use smith_domain_run::{
    Exhausted,
    facts::{EndKind, Fact, FactKind, Kind},
};

/// Effective child turn shares observed at their actual openings.
#[derive(Default)]
pub struct Meeting {
    children: BTreeMap<Token, u32>,
}

impl Expectations for Meeting {
    type Seen = FactKind;
    type Name = &'static str;
    type Stimulus = ();

    fn observe(&mut self, seen: FactKind, judge: &mut Judge<Self::Name, Self::Stimulus>) {
        match seen {
            FactKind::Opened { conversation, kind: Kind::Child { granted, .. }, .. } => {
                self.children.insert(conversation, granted.turns);
            }
            FactKind::Ended { conversation, end: EndKind::Budget(Exhausted::Turns), .. } => {
                judge.check(
                    self.children.get(&conversation).is_none_or(|turns| *turns < 2),
                    "a child with at least two turns answers before its own turn share ends",
                );
            }
            FactKind::MessageRead { .. }
            | FactKind::MessageFence { .. }
            | FactKind::MessageUnread { .. }
            | FactKind::MessageReceived { .. }
            | FactKind::MessageRefused { .. }
            | FactKind::Admitted { .. }
            | FactKind::Prepared { .. }
            | FactKind::Opened { .. }
            | FactKind::Budget { .. }
            | FactKind::Ended { .. }
            | FactKind::Called { .. }
            | FactKind::Returned { .. }
            | FactKind::CheckStarted { .. }
            | FactKind::CheckFinished { .. }
            | FactKind::Delivered { .. }
            | FactKind::Answered { .. } => {}
        }
    }
}

/// Judge effective openings and actual terminals with Skein's referee.
#[must_use]
pub fn judge(facts: &[Fact]) -> Verdict {
    let mut referee = Referee::new(Meeting::default());
    for fact in facts {
        referee.observe(fact.at, fact.kind, &mut Vec::new());
    }
    referee.verdict()
}
