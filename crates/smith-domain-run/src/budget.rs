//! Scalar host-unit ceilings and exact accounting (domain/run.md,
//! sections 9 and 10). Sessions price usage; the run never reprices it.

use crate::charter::{Llm, Reserve};
use skein_lib::Duration;

/// A part of a run's budget supplied by its host (domain/run.md, section 9).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Part {
    /// Its completions.
    Turns,
    /// Its spend in the host's unit.
    Spend,
    /// Its monotonic wall time.
    Time,
}

/// The most of a part the agent admits, returned with an entrance refusal.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Ceiling {
    /// The most completions.
    Turns(u32),
    /// The most spend in the host's unit.
    Spend(u64),
    /// The most monotonic wall time.
    Time(Duration),
}

/// Why the agent refuses a budget before effects (domain/run.md, sections 4 and 9).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BudgetRefusal {
    /// A part asks for more than the agent admits.
    Ceiling(Ceiling),
    /// A part asks for nothing.
    Empty(Part),
    /// The budget cannot hold its finishing reserve.
    Reserve(Part),
    /// A change contract needs more check time than its finishing reserve holds.
    Checks { reserve: Duration, checks: Duration },
    /// The spend cannot pay for one of main's most costly completions.
    Completion { most: u64 },
}

/// A model rate that the host must price above zero (domain/run.md, section 9).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Rate {
    /// New input.
    Input,
    /// Input read from the cache.
    Cached,
    /// Input written to the cache when priced apart.
    CacheWrite,
    /// Output, including reasoning.
    Output,
}

/// Host-supplied activation ceiling shared by every conversation. Zero turns,
/// spend or time refuses admission before effects.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Budget {
    /// Maximum completions; spend for each is reserved before provider work.
    pub turns: u32,

    /// Host-unit ceiling shared by spent charges and live reservations.
    pub spend: u64,

    /// Monotonic activation time; expiry immediately settles cancellation.
    pub time: Duration,
}

/// The share a `sub_agent` call asks for: each part optional, spend never asked.
/// Contract: domain/run.md, section 5.3.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Share {
    pub turns: Option<u32>,

    pub time: Option<Duration>,
}

impl Share {
    /// Neither part asked for: the child receives the ordinary pool.
    pub const UNASKED: Self = Self { turns: None, time: None };
}

/// A child's effective share and how each part was set (domain/run.md, section 5.3).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Granted {
    pub turns: u32,

    pub time: Duration,

    /// All spend left above the finishing reserve when asked, never shown to the model.
    pub spend: u64,

    pub turns_set: Set,

    pub time_set: Set,
}

/// How the run set one part of a child's share.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Set {
    /// As asked, within what was left above the reserve.
    Asked,

    /// Not asked: all of what was left above the reserve.
    Pool,

    /// Asked for more than was left above the reserve, and lowered to it.
    Reserve,

    /// Lowered to the asker's call deadline; time only.
    Deadline,
}

/// The run's budget stage, recorded for its watchers (domain/run.md, section 9.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Stage {
    /// Work draws on the ordinary pool.
    Ordinary,

    /// The ordinary pool is spent and main finishes on the reserve.
    WindDown,
}

/// Host-supplied model rates, interpreted only by Session. The price is the
/// ceiling of ((fresh input + cache writes)*input + cache reads*cached +
/// output*output)/unit, checked before conversion to u64. The run refuses zero
/// rates or a zero unit before discovery; Session keeps its own charging policy.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Prices {
    /// Fresh input rate.
    pub input: u64,

    /// Cache-read rate.
    pub cached: u64,

    /// Separate cache-write rate; absent writes use `input`.
    pub cache_write: Option<u64>,

    /// Output rate.
    pub output: u64,

    /// Positive rate denominator.
    pub unit: u32,
}

/// Exact charged activation accounting, independent of the budget. A failed
/// addition leaves every field unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Spend {
    /// Actual completion count.
    pub turns: u32,

    /// Actual fresh input tokens.
    pub input: Option<u64>,

    /// Actual output tokens.
    pub output: Option<u64>,

    /// Actual cache-read tokens.
    pub cache_read: Option<u64>,

    /// Actual cache-write tokens.
    pub cache_write: Option<u64>,

    /// Reported reasoning tokens, already within output and never charged again.
    pub reasoning: Option<u64>,

    /// Own completion units summed once across all conversations.
    pub units: u64,
}

/// Scalar or time ceiling that prevents a subsequent completion.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Exhausted {
    /// Completion ceiling reached.
    Turns,

    /// Host-unit ceiling reached.
    Spend,

    /// Monotonic deadline reached.
    Time,
    /// A session exceeded its receiving token ceiling for one kind.
    Tokens(ReceivingLimit),
    /// Checked priced or raw usage arithmetic could not represent the total.
    Overflow(Overflow),
}

/// Which checked budget arithmetic overflowed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Overflow {
    /// Priced spend could not be represented.
    Spend,
    /// Raw completion usage could not be represented.
    Usage,
}

/// Session receiving token cap, distinct from the run's scalar ceiling.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReceivingLimit {
    /// Fresh input cap.
    Input,

    /// Output cap.
    Output,

    /// Cache-read cap.
    CacheRead,

    /// Cache-write cap.
    CacheWrite,
}

impl Spend {
    /// Empty activation accounting.
    pub const ZERO: Spend =
        Spend { turns: 0, input: None, output: None, cache_read: None, cache_write: None, reasoning: None, units: 0 };

    /// Add only a fully representable increment. No part of a failed addition
    /// is charged.
    #[must_use]
    pub fn accumulate(self, increment: Spend) -> Option<Spend> {
        Some(Spend {
            turns: self.turns.checked_add(increment.turns)?,
            input: add_count(self.input, increment.input)?,
            output: add_count(self.output, increment.output)?,
            cache_read: add_count(self.cache_read, increment.cache_read)?,
            cache_write: add_count(self.cache_write, increment.cache_write)?,
            reasoning: add_count(self.reasoning, increment.reasoning)?,
            units: self.units.checked_add(increment.units)?,
        })
    }

    /// Exact terminal raw residual. Units are delivered separately by Priced.
    #[must_use]
    pub fn unreported(self, reported: Spend) -> Spend {
        Spend {
            turns: self.turns.checked_sub(reported.turns).expect("reported completions belong to session"),
            input: residual_count(self.input, reported.input),
            output: residual_count(self.output, reported.output),
            cache_read: residual_count(self.cache_read, reported.cache_read),
            cache_write: residual_count(self.cache_write, reported.cache_write),
            reasoning: residual_count(self.reasoning, reported.reasoning),
            ..Spend::ZERO
        }
    }
}

fn add_count(previous: Option<u64>, received: Option<u64>) -> Option<Option<u64>> {
    match previous {
        Some(previous) => match received {
            Some(received) => Some(Some(previous.checked_add(received)?)),
            None => Some(Some(previous)),
        },
        None => Some(received),
    }
}

fn residual_count(total: Option<u64>, reported: Option<u64>) -> Option<u64> {
    match total {
        Some(total) => match reported {
            Some(reported) => Some(total.checked_sub(reported).expect("reported usage belongs to session")),
            None => Some(total),
        },
        None => {
            assert!(reported.is_none(), "reported count belongs to the session total");
            None
        }
    }
}

impl Prices {
    /// The highest input, cached and cache-write rate, reserved before provider work.
    #[must_use]
    pub const fn reserved_input(self) -> u64 {
        let highest = if self.cached > self.input { self.cached } else { self.input };
        match self.cache_write {
            Some(write) => {
                if write > highest {
                    write
                } else {
                    highest
                }
            }
            None => highest,
        }
    }

    /// Refuse the first unpriced rate at the run's entrance; sessions remain generic.
    pub const fn check(self) -> Result<(), Rate> {
        if self.input == 0 {
            return Err(Rate::Input);
        }
        if self.cached == 0 {
            return Err(Rate::Cached);
        }
        if let Some(0) = self.cache_write {
            return Err(Rate::CacheWrite);
        }
        if self.output == 0 {
            return Err(Rate::Output);
        }
        Ok(())
    }
}

impl Budget {
    /// Check scalar parts against the agent's ceilings before any effects.
    pub fn check(&self, limit: &Budget) -> Result<(), BudgetRefusal> {
        if self.turns == 0 {
            return Err(BudgetRefusal::Empty(Part::Turns));
        }
        if self.spend == 0 {
            return Err(BudgetRefusal::Empty(Part::Spend));
        }
        if self.time == Duration::ZERO {
            return Err(BudgetRefusal::Empty(Part::Time));
        }
        if self.turns > limit.turns {
            return Err(BudgetRefusal::Ceiling(Ceiling::Turns(limit.turns)));
        }
        if self.spend > limit.spend {
            return Err(BudgetRefusal::Ceiling(Ceiling::Spend(limit.spend)));
        }
        if self.time > limit.time {
            return Err(BudgetRefusal::Ceiling(Ceiling::Time(limit.time)));
        }
        Ok(())
    }

    pub(crate) fn is_workable(&self) -> bool {
        self.turns > 0 && self.spend > 0 && self.time > Duration::ZERO
    }

    pub(crate) fn exhausted(&self, spent: Spend) -> Option<Exhausted> {
        if spent.turns >= self.turns {
            Some(Exhausted::Turns)
        } else if spent.units >= self.spend {
            Some(Exhausted::Spend)
        } else {
            None
        }
    }

    pub(crate) fn remainder(&self, spent: Spend, time: Duration) -> Budget {
        Budget { turns: self.turns.saturating_sub(spent.turns), spend: self.spend.saturating_sub(spent.units), time }
    }
}

/// Whether a run may start, shared by the entrance, local host and `smith check`.
/// A change contract supplies its configured check deadline; other contracts
/// supply none (domain/run.md, sections 4 and 9.1).
pub fn admit(
    budget: &Budget,
    reserve: Reserve,
    main: &Llm,
    ceilings: &Budget,
    checks: Option<Duration>,
) -> Result<(), BudgetRefusal> {
    budget.check(ceilings)?;
    if reserve.turns >= budget.turns {
        return Err(BudgetRefusal::Reserve(Part::Turns));
    }
    if reserve.time >= budget.time {
        return Err(BudgetRefusal::Reserve(Part::Time));
    }
    let reserved = reserve_spend(reserve, main).ok_or(BudgetRefusal::Reserve(Part::Spend))?;
    if reserved >= budget.spend {
        return Err(BudgetRefusal::Reserve(Part::Spend));
    }
    if let Some(checks) = checks
        && reserve.time < checks
    {
        return Err(BudgetRefusal::Checks { reserve: reserve.time, checks });
    }
    Ok(())
}

/// Finishing spend is zero until increment 07.12 derives it from main's window.
#[must_use]
pub const fn reserve_spend(_reserve: Reserve, _main: &Llm) -> Option<u64> {
    Some(0)
}

/// Remaining ordinary credit, above main's finishing reserve in each part.
pub(crate) fn ordinary(budget: &Budget, reserve: Reserve, reserve_spend: u64, spent: Spend, left: Duration) -> Budget {
    Budget {
        turns: budget.turns.saturating_sub(spent.turns).saturating_sub(reserve.turns),
        spend: budget.spend.saturating_sub(spent.units).saturating_sub(reserve_spend),
        time: Duration::from_nanos(left.as_nanos().saturating_sub(reserve.time.as_nanos())),
    }
}

/// Pure root admission decision before any Client effect. Closing has its
/// existing terminal right and must never be reported as scalar exhaustion.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CompletionPermit {
    /// Live conversation with remaining global allowance.
    Allowed,

    /// The main awaits durable acknowledgement credit before its unsent completion.
    Held,

    /// Live conversation prevented by a scalar ceiling.
    Denied(
        /// Exact scalar dimension.
        Exhausted,
    ),

    /// Stale or closing conversation; settle its unsent completion as Closed.
    Closing,
}
