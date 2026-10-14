//! Raw optional token totals compare only within the same session/run.
//! Missing provider counts remain unknown (benchmarks.md11.1, events.md3.3/3.5).

use super::Observer;
use crate::Measure;
use smith_events::{Event, Usage};
use std::collections::BTreeMap;

fn counts(usage: &Usage) -> [Option<u64>; 5] {
    [usage.input_tokens, usage.cache_read_tokens, usage.cache_write_tokens, usage.output_tokens, usage.reasoning_tokens]
}

pub(super) fn check(observer: &Observer) -> Measure<bool> {
    let mut unknown = false;
    for session in &observer.sessions {
        let mut runs = BTreeMap::new();
        let mut sums = BTreeMap::<u64, [Option<u64>; 5]>::new();
        let mut seen = BTreeMap::new();
        for record in &session.records {
            match &record.event {
                Event::ResponseCompleted(response) => {
                    let values = counts(&response.usage);
                    let identity = (response.conversation, values);
                    match seen.insert((response.run, response.response), identity) {
                        Some(previous) if previous != identity => return Measure::Observed { value: false },
                        Some(_) => continue,
                        None => {}
                    }
                    let sum = sums.entry(response.run).or_insert([Some(0); 5]);
                    for (total, value) in sum.iter_mut().zip(values) {
                        *total = total.and_then(|total| value.and_then(|value| total.checked_add(value)));
                    }
                }
                Event::RunCompleted(run) => {
                    if runs.insert(run.run, counts(&run.usage)).is_some() {
                        return Measure::Observed { value: false };
                    }
                }
                Event::MessageReceived(_)
                | Event::MessageRead(_)
                | Event::MessageUnread(_)
                | Event::Budget(_)
                | Event::SessionStarted(_)
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
                | Event::Notice(_) => {}
            }
        }
        if runs.is_empty() || sums.keys().any(|run| !runs.contains_key(run)) {
            unknown = true;
        }
        for (run, total) in runs {
            let sum = sums.get(&run).copied().unwrap_or([Some(0); 5]);
            for (sum, total) in sum.into_iter().zip(total) {
                match (sum, total) {
                    (Some(sum), Some(total)) if sum != total => return Measure::Observed { value: false },
                    (Some(_), Some(_)) => {}
                    _ => unknown = true,
                }
            }
        }
    }
    if unknown {
        Measure::unavailable("a raw response or run token count is null or overflows; no zero inferred")
    } else {
        Measure::Observed { value: true }
    }
}
