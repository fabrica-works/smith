//! Scalar fact projections and bounded raw-report totals for the event producer. No provider,
//! native call text, credential or sink state is known here; the caller supplies
//! the process clock origin. Budget records enter through [`budget_record`].
//! Contracts: protocol/events.md, sections 3.8, 3.10 and 4.

use alloc::boxed::Box;
use skein_lib::Time;
use smith_domain::run;
use smith_events::{BudgetStage, BudgetStanding, Event, Quantity, Record, Reserve};

/// Reported totals kept independently of accepted accounting, including sticky overflow.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reported {
    counts: [Count; 5],
}

#[derive(Clone, Copy, Debug)]
enum Count {
    Absent,
    Value(u64),
    Overflow,
}

impl Reported {
    pub(crate) const NONE: Self = Self { counts: [Count::Absent; 5] };

    pub(crate) fn add(&mut self, usage: smith_domain::llm::Usage) {
        let received = [
            usage.input_tokens,
            usage.cache_read_tokens,
            usage.cache_write_tokens,
            usage.output_tokens,
            usage.reasoning_tokens,
        ];
        for (index, received) in received.into_iter().enumerate() {
            let count = self.counts.get_mut(index).expect("five reported counters");
            *count = match *count {
                Count::Overflow => Count::Overflow,
                Count::Absent => match received {
                    Some(value) => Count::Value(value),
                    None => Count::Absent,
                },
                Count::Value(previous) => match received {
                    Some(value) => match previous.checked_add(value) {
                        Some(total) => Count::Value(total),
                        None => Count::Overflow,
                    },
                    None => Count::Value(previous),
                },
            };
        }
    }

    pub(crate) const fn usage(self) -> smith_events::Usage {
        smith_events::Usage {
            input_tokens: count_value(self.counts[0]),
            cache_read_tokens: count_value(self.counts[1]),
            cache_write_tokens: count_value(self.counts[2]),
            output_tokens: count_value(self.counts[3]),
            reasoning_tokens: count_value(self.counts[4]),
        }
    }
}

const fn count_value(count: Count) -> Option<u64> {
    match count {
        Count::Absent | Count::Overflow => None,
        Count::Value(value) => Some(value),
    }
}

/// The actual settled checks' exit, with a passing result only for code zero.
/// The run keeps cancellation and deadline expiry distinct after its tree stops.
#[must_use]
pub fn check_exit(exit: run::Exit) -> (smith_events::CheckExit, bool) {
    match exit {
        run::Exit::Code { code } => (smith_events::CheckExit::Code { code: i64::from(code) }, code == 0),
        run::Exit::Signalled { signal } => (smith_events::CheckExit::Signal { signal: u64::from(signal) }, false),
        run::Exit::TimedOut => (smith_events::CheckExit::TimedOut, false),
        run::Exit::Unstarted => (smith_events::CheckExit::Unstarted, false),
        run::Exit::Cancelled => (smith_events::CheckExit::Cancelled, false),
    }
}

/// The actual opening's model, joined to the configured endpoint name and effort.
/// The caller supplies configured metadata; no endpoint or model is inferred.
#[must_use]
#[expect(clippy::manual_map, reason = "the strict subset requires an explicit match instead of a closure")]
pub fn opening_model(
    llm: &run::charter::Llm,
    endpoint: &[u8],
    effort: Option<&[u8]>,
    backend: smith_events::Backend,
) -> smith_events::Model {
    smith_events::Model {
        account: llm.account,
        backend,
        endpoint: endpoint.into(),
        model: llm.model.clone(),
        window_tokens: u64::from(llm.window),
        output_tokens: u64::from(llm.output),
        effort: match effort {
            Some(effort) => Some(effort.into()),
            None => None,
        },
    }
}

/// Exact provider-reported counters, keeping each absent counter unknown.
#[must_use]
pub const fn reported_usage(usage: smith_domain::llm::Usage) -> smith_events::Usage {
    smith_events::Usage {
        input_tokens: usage.input_tokens,
        cache_read_tokens: usage.cache_read_tokens,
        cache_write_tokens: usage.cache_write_tokens,
        output_tokens: usage.output_tokens,
        reasoning_tokens: usage.reasoning_tokens,
    }
}

/// Tool selection of the actual surviving request handed to the LLM component.
/// Logical completion-intent facts do not establish a provider request start.
#[must_use]
pub const fn request_tool_choice(choice: &smith_domain::llm::ToolChoice) -> smith_events::ToolChoice {
    match choice {
        smith_domain::llm::ToolChoice::Auto => smith_events::ToolChoice::Auto,
        smith_domain::llm::ToolChoice::None => smith_events::ToolChoice::None,
        smith_domain::llm::ToolChoice::Only(_) => smith_events::ToolChoice::Only,
    }
}

/// A run's actual charge observation, joined to its Start activation and stamped from the process origin.
#[must_use]
pub fn budget_record(
    fact: &run::facts::Fact,
    started_at: Time,
    activation: u64,
    call: Option<Box<[u8]>>,
) -> Option<Record> {
    let standing = match fact.kind {
        run::facts::FactKind::Budget { standing, .. } => standing,
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
            run: activation,
            call,
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
