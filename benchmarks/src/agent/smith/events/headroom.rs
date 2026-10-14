//! Independent headroom uses each request's actual model and admitted run
//! allowances. Startup-key names, ordinal joins and unavailable bounds are never
//! guessed. Integer comparisons decide violations and half warnings; only the
//! displayed fraction is approximate (protocol/events.md, sections3.3/3.5/3.8/3.10;
//! benchmarks.md, section11.2).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use smith_events::{Event, Model};

use crate::{CheckVerdict, Headroom, HeadroomOf, Health, Measure, Passive};

use super::{Observer, Session};

/// One joined usage and native allowance, in the same units.
struct Reading {
    used: u128,
    bound: u64,
    scope: String,
}

impl Reading {
    fn compare(&self, other: &Self) -> Ordering {
        let divisor = u128::from(self.bound);
        let other_divisor = u128::from(other.bound);
        // Compare integral parts first. Remainder products fit u128 even
        // when a prompt's three disjoint u64 counts do not fit u64.
        (self.used / divisor)
            .cmp(&(other.used / other_divisor))
            .then_with(|| ((self.used % divisor) * other_divisor).cmp(&((other.used % other_divisor) * divisor)))
    }

    #[expect(clippy::cast_precision_loss, reason = "display only; violation, maximum and warning use exact integers")]
    fn fraction(&self) -> f64 {
        self.used as f64 / self.bound as f64
    }

    fn evidence(&self) -> String {
        format!("{}: used={}, bound={}", self.scope, self.used, self.bound)
    }
}

/// The largest joined ratio plus missing observations, across process scopes.
#[derive(Default)]
struct Peak {
    largest: Option<Reading>,
    missing: Vec<String>,
    violation: bool,
    warning: bool,
}

impl Peak {
    fn push(&mut self, reading: Reading) {
        self.violation |= reading.used > u128::from(reading.bound);
        self.warning |= reading.bound > 0 && reading.used * 2 >= u128::from(reading.bound);
        if reading.bound == 0 {
            self.missing.push(format!("{}; zero bound has no defined fraction", reading.evidence()));
            return;
        }
        if self.largest.as_ref().is_none_or(|previous| reading.compare(previous) == Ordering::Greater) {
            self.largest = Some(reading);
        }
    }

    fn finish(self, quantity: HeadroomOf) -> (Headroom, CheckVerdict) {
        let mut missing = self.missing;
        if self.largest.is_none() {
            missing.push("no joined native usage and allowance".into());
        }
        let complete = missing.is_empty();
        let fraction_used = match &self.largest {
            Some(reading) if complete => Measure::Observed { value: reading.fraction() },
            Some(reading) => Measure::LowerBound { value: reading.fraction(), missing: missing.clone() },
            None => Measure::unavailable(missing.join("; ")),
        };
        let passed = if self.violation {
            Measure::Observed { value: false }
        } else if complete {
            Measure::Observed { value: true }
        } else {
            Measure::unavailable(missing.join("; "))
        };
        let mut evidence: Vec<_> = self.largest.iter().map(Reading::evidence).collect();
        evidence.extend(missing);
        (
            Headroom { fraction_used, warning: self.warning },
            CheckVerdict { check: Passive::Headroom(quantity).name().into(), passed, evidence },
        )
    }
}

#[derive(Default)]
struct Readings {
    prompt: Peak,
    turns: Peak,
    spend: Peak,
}

fn prompt(readings: &mut Peak, value: &smith_events::ResponseCompleted, window: Option<&u64>, scope: String) {
    let Some(bound) = window else {
        readings.missing.push(format!("{scope}: response lacks its request's actual model window"));
        return;
    };
    match (value.usage.input_tokens, value.usage.cache_read_tokens, value.usage.cache_write_tokens) {
        (Some(input), Some(read), Some(write)) => readings.push(Reading {
            used: u128::from(input) + u128::from(read) + u128::from(write),
            bound: *bound,
            scope,
        }),
        _ => readings.missing.push(format!("{scope}: fresh/cache-read/cache-write prompt counts are not all reported")),
    }
}

fn snapshot(
    totals: &mut Readings,
    value: &smith_events::BudgetStanding,
    admitted: Option<&&smith_events::Budget>,
    index: usize,
) {
    let Some(budget) = admitted else {
        totals.turns.missing.push(format!("session-{index}/run-{}: budget lacks its admission", value.run));
        totals.spend.missing.push(format!("session-{index}/run-{}: budget lacks its admission", value.run));
        return;
    };
    let scope = format!("session-{index}/run-{}", value.run);
    if value.turns.limit == budget.turns {
        totals.turns.push(Reading { used: u128::from(value.turns.used), bound: budget.turns, scope: scope.clone() });
    } else {
        totals.turns.missing.push(format!("{scope}: snapshot turn allowance differs from admission"));
    }
    if value.spend.limit == budget.spend {
        totals.spend.push(Reading { used: u128::from(value.spend.used), bound: budget.spend, scope });
    } else {
        totals.spend.missing.push(format!("{scope}: snapshot spend allowance differs from admission"));
    }
}

fn collect(totals: &mut Readings, session: &Session, index: usize) {
    if super::usage::incomplete(session).is_some() {
        for reading in [&mut totals.prompt, &mut totals.turns, &mut totals.spend] {
            reading.missing.push(format!("session-{index}: incomplete/lossy stream prevents proving the maximum"));
        }
    }
    let mut models = BTreeMap::<(u64, u64), &Model>::new();
    let mut requests = BTreeMap::new();
    let mut budgets = BTreeMap::new();
    let mut charged = BTreeSet::new();
    let mut ended = BTreeSet::new();
    let mut answered = BTreeSet::new();
    for record in &session.records {
        match &record.event {
            Event::RunStarted(value) => {
                budgets.insert(value.run, &value.budget);
            }
            Event::ConversationOpened(value) => {
                models.insert((value.run, value.conversation), &value.model);
            }
            Event::ResponseStarted(value) => {
                if models.get(&(value.run, value.conversation)) == Some(&&value.model) {
                    requests.insert((value.run, value.conversation, value.response), value.model.window_tokens);
                } else {
                    totals.prompt.missing.push(format!(
                        "session-{index}/run-{}/response-{}: request does not join its opening model",
                        value.run, value.response
                    ));
                }
            }
            Event::ResponseCompleted(value) => {
                let key = (value.run, value.conversation, value.response);
                answered.insert(key);
                prompt(
                    &mut totals.prompt,
                    value,
                    requests.get(&key),
                    format!(
                        "session-{index}/run-{}/conversation-{}/response-{}",
                        value.run, value.conversation, value.response
                    ),
                );
            }
            Event::Budget(value) => {
                charged.insert(value.run);
                snapshot(totals, value, budgets.get(&value.run), index);
            }
            Event::RunCompleted(value) => {
                ended.insert(value.run);
                if let Some(budget) = budgets.get(&value.run) {
                    totals.spend.push(Reading {
                        used: u128::from(value.spent),
                        bound: budget.spend,
                        scope: format!("session-{index}/run-{}", value.run),
                    });
                } else if value.status != smith_events::Status::Refused {
                    totals
                        .spend
                        .missing
                        .push(format!("session-{index}/run-{}: answer lacks its actual admission", value.run));
                }
                // RunCompleted.turns counts durable activation turns, not
                // completion-budget charges (domain/run.md, section9.1).
            }
            Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::ConversationClosed(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Notice(_) => {}
        }
    }
    for run in budgets.keys() {
        if !charged.contains(run) {
            totals
                .turns
                .missing
                .push(format!("session-{index}/run-{run}: no actual completion-budget charges reported"));
        }
        if !ended.contains(run) {
            for reading in [&mut totals.turns, &mut totals.spend] {
                reading
                    .missing
                    .push(format!("session-{index}/run-{run}: missing run answer prevents proving the maximum"));
            }
        }
    }
    for key in requests.keys() {
        if !answered.contains(key) {
            totals.prompt.missing.push(format!(
                "session-{index}/run-{}/conversation-{}/response-{}: request lacks its usage terminal",
                key.0, key.1, key.2
            ));
        }
    }
}

pub(super) fn apply(observer: &Observer, health: &mut Health) {
    let mut totals = Readings::default();
    for (index, session) in observer.sessions.iter().enumerate() {
        collect(&mut totals, session, index);
    }
    if observer.fault.is_some() {
        for reading in [&mut totals.prompt, &mut totals.turns, &mut totals.spend] {
            reading.missing.push("outside reader fault prevents proving the maximum".into());
        }
    }
    for (quantity, peak) in [
        (HeadroomOf::PromptTokens, totals.prompt),
        (HeadroomOf::Turns, totals.turns),
        (HeadroomOf::Spend, totals.spend),
    ] {
        let (headroom, check) = peak.finish(quantity);
        let name = check.check.trim_start_matches("headroom/");
        health.headroom.insert(name.into(), headroom);
        let previous = health
            .passive_checks
            .iter_mut()
            .find(|previous| previous.check == check.check)
            .expect("named headroom initialized before projection");
        *previous = check;
    }
}
