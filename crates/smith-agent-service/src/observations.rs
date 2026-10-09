//! Pure scalar fact projections for the bounded event producer. No provider,
//! native call text, credential or sink state is known here; the caller supplies
//! the process clock origin. Budget records enter through [`budget_record`].
//! Contract: protocol/events.md, sections 3.8 and 4.

use skein_lib::Time;
use smith_domain::run;
use smith_events::{BudgetStage, BudgetStanding, Event, Quantity, Record, Reserve};

/// A run's actual charge observation, stamped from the process origin for its event sink.
#[must_use]
pub fn budget_record(fact: &run::facts::Fact, started_at: Time) -> Option<Record> {
    let (run, standing) = match fact.kind {
        run::facts::FactKind::Budget { run, standing } => (run, standing),
        run::facts::FactKind::MessageRead { .. }
        | run::facts::FactKind::MessageFence { .. }
        | run::facts::FactKind::MessageUnread { .. }
        | run::facts::FactKind::MessageReceived { .. }
        | run::facts::FactKind::MessageRefused { .. }
        | run::facts::FactKind::Admitted { .. }
        | run::facts::FactKind::Prepared { .. }
        | run::facts::FactKind::Opened { .. }
        | run::facts::FactKind::Ended { .. }
        | run::facts::FactKind::Called { .. }
        | run::facts::FactKind::Returned { .. }
        | run::facts::FactKind::CheckStarted { .. }
        | run::facts::FactKind::CheckFinished { .. }
        | run::facts::FactKind::Delivered { .. }
        | run::facts::FactKind::Answered { .. } => return None,
    };
    let stage = match standing.stage {
        run::Stage::Ordinary => BudgetStage::Ordinary,
        run::Stage::WindDown => BudgetStage::WindDown,
    };
    Some(Record {
        t_ms: fact.at.saturating_since(started_at).as_nanos() / 1_000_000,
        event: Event::Budget(BudgetStanding {
            run: run.raw(),
            stage,
            turns: Quantity { used: u64::from(standing.turns), limit: u64::from(standing.turns_limit) },
            spend: Quantity { used: standing.spend, limit: standing.spend_limit },
            time_ms: Quantity {
                used: standing.time.as_nanos() / 1_000_000,
                limit: standing.time_limit.as_nanos() / 1_000_000,
            },
            reserve: Reserve {
                turns: u64::from(standing.reserve.turns),
                time_ms: standing.reserve.time.as_nanos() / 1_000_000,
                spend: standing.reserve_spend,
            },
        }),
    })
}

/// A child's scalar opening metadata, joined by the event producer to its model and call id.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ChildShare {
    pub requested: smith_events::Requested,
    pub share: smith_events::Share,
    pub clamped: skein_lib::List<smith_events::Clamp>,
}

/// Project an actual child's opening, keeping an absent ask distinct from a grant.
#[must_use]
#[expect(clippy::manual_map, reason = "the strict subset requires explicit matches")]
pub fn child_share(kind: run::facts::Kind) -> Option<ChildShare> {
    let (asked, granted) = match kind {
        run::facts::Kind::Main => return None,
        run::facts::Kind::Child { asked, granted, .. } => (asked, granted),
    };
    let mut clamped = skein_lib::List::with_capacity(2);
    clamp(&mut clamped, smith_events::SharePart::Turns, granted.turns_set);
    clamp(&mut clamped, smith_events::SharePart::Time, granted.time_set);
    Some(ChildShare {
        requested: smith_events::Requested {
            turns: match asked.turns {
                Some(turns) => Some(u64::from(turns)),
                None => None,
            },
            seconds: match asked.time {
                Some(time) => Some(time.as_nanos() / 1_000_000_000),
                None => None,
            },
        },
        share: smith_events::Share {
            turns: u64::from(granted.turns),
            time_ms: granted.time.as_nanos() / 1_000_000,
            spend: granted.spend,
        },
        clamped,
    })
}

fn clamp(clamped: &mut skein_lib::List<smith_events::Clamp>, part: smith_events::SharePart, set: run::Set) {
    let by = match set {
        run::Set::Asked | run::Set::Pool => return,
        run::Set::Reserve => smith_events::ShareBound::Reserve,
        run::Set::Deadline => smith_events::ShareBound::Deadline,
    };
    clamped.push(smith_events::Clamp { part, by }).expect("at most two share parts");
}

/// Join the run's settled usage to the child's actual session terminal and token totals.
/// The caller supplies the stop and nullable provider totals; they are never inferred from a return.
#[must_use]
pub fn child_completed(
    conversation: u64,
    stop: smith_events::ChildStop,
    used: run::facts::Used,
    usage: smith_events::Usage,
) -> smith_events::ChildCompleted {
    smith_events::ChildCompleted {
        conversation,
        stop,
        turns: u64::from(used.turns),
        time_ms: used.time.as_nanos() / 1_000_000,
        spent: used.spend,
        usage,
    }
}
