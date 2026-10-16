//! Compaction is owed only once a request's prompt passes its conversation's
//! line. Missing usage or compaction material stays unknown (benchmarks.md,
//! sections 3.2 and 11.1).

use std::collections::BTreeMap;

use smith_events::{Event, LimitValue, Outcome};

use crate::Measure;

use super::Observer;

pub(super) fn passed_line(observer: &Observer) -> Measure<bool> {
    for session in &observer.sessions {
        let Some(material) =
            session.started.limits.iter().find(|limit| limit.name.as_ref() == b"run.compaction_material")
        else {
            return Measure::unavailable("not exercised: session.started lacks run.compaction_material");
        };
        let LimitValue::Integer(material) = material.value else {
            return Measure::unavailable("run.compaction_material is not an integer");
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
                let Some(line) = request
                    .model
                    .window_tokens
                    .checked_sub(request.model.output_tokens)
                    .and_then(|room| room.checked_sub(material))
                else {
                    return Measure::unavailable("conversation's window cannot hold output and compaction material");
                };
                let prompt = u128::from(input) + u128::from(read) + u128::from(write);
                if prompt > u128::from(line) {
                    return Measure::Observed { value: true };
                }
            }
        }
    }
    Measure::Observed { value: false }
}
