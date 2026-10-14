//! Spend checks compare native integer charges with the prepared currency,
//! without rounding the ceiling or charging aggregate usage a second time
//! (benchmarks.md, sections6.2/11.1; protocol/events.md, sections3.3/3.5).

use std::collections::BTreeMap;

use smith_events::Event;

use crate::Measure;

use super::Observer;

fn within(spent: u128, currency: u64, ceiling: f64) -> bool {
    let bits = ceiling.to_bits();
    let exponent = u32::try_from((bits >> 52) & 0x7ff).expect("float exponent fits");
    let fraction = bits & ((1_u64 << 52) - 1);
    let (mantissa, shift) = if exponent == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, i32::try_from(exponent).expect("float exponent fits") - 1075)
    };
    let coefficient = u128::from(mantissa) * u128::from(currency);
    if shift >= 0 {
        let bound = u32::try_from(shift)
            .ok()
            .and_then(|shift| 1_u128.checked_shl(shift))
            .and_then(|scale| coefficient.checked_mul(scale));
        bound.is_none_or(|bound| spent <= bound)
    } else {
        if spent == 0 {
            return true;
        }
        let scaled = u32::try_from(-shift)
            .ok()
            .and_then(|shift| 1_u128.checked_shl(shift))
            .and_then(|scale| spent.checked_mul(scale));
        scaled.is_some_and(|scaled| scaled <= coefficient)
    }
}

pub(super) fn check(observer: &Observer, currency: u64, ceiling: f64) -> Measure<bool> {
    if currency == 0 || !ceiling.is_finite() || ceiling <= 0.0 {
        return Measure::unavailable("spend needs a positive exact currency and finite dollar ceiling");
    }
    let mut measured = false;
    let mut response_spend = 0_u128;
    let mut terminal_spend = 0_u128;
    for session in &observer.sessions {
        let mut responses = BTreeMap::new();
        let mut terminals = BTreeMap::new();
        for record in &session.records {
            let spent = match &record.event {
                Event::ResponseCompleted(value) => {
                    let identity = (value.run, value.response);
                    let charge = (value.conversation, value.spent);
                    if let Some(previous) = responses.insert(identity, charge) {
                        if previous != charge {
                            return Measure::unavailable("repeated response changed its charge or scope");
                        }
                        continue;
                    }
                    let Some(total) = response_spend.checked_add(u128::from(value.spent)) else {
                        return Measure::unavailable("response spend sum overflow");
                    };
                    response_spend = total;
                    total
                }
                Event::RunCompleted(value) => {
                    if let Some(previous) = terminals.insert(value.run, value.spent) {
                        if previous != value.spent {
                            return Measure::unavailable("repeated run terminal changed its spend");
                        }
                        continue;
                    }
                    let Some(total) = terminal_spend.checked_add(u128::from(value.spent)) else {
                        return Measure::unavailable("terminal spend sum overflow");
                    };
                    terminal_spend = total;
                    total
                }
                Event::Budget(value) => u128::from(value.spend.used),
                Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunStarted(_)
                | Event::ConversationOpened(_)
                | Event::ConversationClosed(_)
                | Event::ResponseStarted(_)
                | Event::TextDelta(_)
                | Event::ToolStarted(_)
                | Event::JobEnded(_)
                | Event::ToolCompleted(_)
                | Event::CheckStarted(_)
                | Event::CheckCompleted(_)
                | Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Notice(_) => continue,
            };
            measured = true;
            if !within(spent, currency, ceiling) {
                return Measure::Observed { value: false };
            }
        }
    }
    if measured { Measure::Observed { value: true } } else { Measure::unavailable("no run charge was reported") }
}
