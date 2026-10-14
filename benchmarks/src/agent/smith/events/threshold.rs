//! Compaction is owed only once a request reaches the window's policy share.
//! Retained requests join disjoint usage to their output reservation; missing
//! counts stay unknown (benchmarks.md, sections 3.2 and 11.1).

use std::collections::BTreeMap;

use smith_events::{ConversationKind, Event, LimitValue, Outcome};

use crate::Measure;

use super::Observer;

pub(super) fn reached(observer: &Observer) -> Measure<bool> {
    if observer.sessions.iter().flat_map(|session| &session.records).any(|record| {
        matches!(&record.event, Event::ConversationOpened(value) if value.kind == ConversationKind::Compaction)
    }) {
        return Measure::Observed { value: true };
    }
    for session in &observer.sessions {
        let Some(threshold) =
            session.started.limits.iter().find(|limit| limit.name.as_ref() == b"policy.compaction_threshold")
        else {
            return Measure::unavailable("session.started lacks policy.compaction_threshold");
        };
        let LimitValue::Fraction { numerator, denominator } = threshold.value else {
            return Measure::unavailable("policy.compaction_threshold is not a fraction");
        };
        let mut models = BTreeMap::new();
        let mut requests = BTreeMap::new();
        for (position, record) in session.records.iter().enumerate() {
            if let Event::ConversationOpened(value) = &record.event {
                models.insert((value.run, value.conversation), &value.model);
            }
            if let Event::ResponseStarted(value) = &record.event {
                if models.get(&(value.run, value.conversation)) != Some(&&value.model) {
                    return Measure::unavailable("request model does not match its actual conversation opening");
                }
                requests.insert((value.run, value.conversation, value.response), value);
            }
            if let Event::ResponseCompleted(value) = &record.event {
                let Some(request) = requests.get(&(value.run, value.conversation, value.response)) else {
                    return Measure::unavailable("response lacks its request's actual model window");
                };
                if value.outcome == Outcome::Failed && value.blocks == 0
                    && session.records[position + 1..].iter().any(|record| matches!(&record.event,
                        Event::ResponseStarted(retry) if retry.run == value.run && retry.conversation == value.conversation
                            && retry.turn == request.turn && retry.attempt > request.attempt))
                {
                    continue;
                }
                let Some(input) = value.usage.input_tokens else {
                    return Measure::unavailable("provider did not report fresh prompt input");
                };
                let Some(read) = value.usage.cache_read_tokens else {
                    return Measure::unavailable("provider did not report cached prompt input");
                };
                let Some(write) = value.usage.cache_write_tokens else {
                    return Measure::unavailable("provider did not report written prompt input");
                };
                let reserved =
                    u128::from(input) + u128::from(read) + u128::from(write) + u128::from(request.output_tokens);
                let Some(scaled) = reserved.checked_mul(u128::from(denominator)) else {
                    return Measure::unavailable(
                        "prompt and output reservation exceed the exact threshold counter range",
                    );
                };
                if scaled > u128::from(request.model.window_tokens) * u128::from(numerator) {
                    return Measure::Observed { value: true };
                }
            }
        }
    }
    Measure::Observed { value: false }
}
