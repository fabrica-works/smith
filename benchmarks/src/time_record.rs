//! Interval unions retain overlap on the main thread (benchmarks.md, section 8.1).
//! No intervals are inferred from missing records; adapters supply matched spans.

use crate::{Measure, TimeRecord};

pub(crate) type Span = (u64, u64);

pub(crate) fn union(spans: &[Span]) -> Option<u64> {
    let mut spans = spans.to_vec();
    spans.sort_unstable();
    let mut end = 0;
    let mut total = 0_u64;
    for (start, next) in spans {
        next.checked_sub(start)?;
        let duration = next.checked_sub(start.max(end).min(next))?;
        total = total.checked_add(duration)?;
        end = end.max(next);
    }
    Some(total)
}

pub(crate) fn observed(value: Option<u64>) -> Measure<u64> {
    value.map_or_else(
        || Measure::unavailable("duration absent or outside its range"),
        |value| Measure::Observed { value },
    )
}

#[derive(Default)]
pub(crate) struct Breakdown {
    pub requests: Vec<Span>,
    pub tools: Vec<(String, Vec<Span>)>,
    pub compaction: Vec<Span>,
    pub children: Vec<Span>,
    pub commands: Vec<Span>,
}

impl Breakdown {
    pub fn tool(&mut self, name: &str, span: Span) {
        if let Some((_, spans)) = self.tools.iter_mut().find(|(tool, _)| tool == name) {
            spans.push(span);
        } else {
            self.tools.push((name.into(), vec![span]));
        }
    }

    pub fn finish(self, wall: Span) -> TimeRecord {
        let calls: Vec<_> = self.tools.iter().flat_map(|(_, spans)| spans.iter().copied()).collect();
        let occupied: Vec<_> =
            self.requests.iter().chain(&calls).chain(&self.compaction).chain(&self.children).copied().collect();
        let intersection: Vec<_> = self
            .commands
            .iter()
            .flat_map(|command| {
                calls.iter().filter_map(|call| {
                    let start = command.0.max(call.0);
                    let end = command.1.min(call.1);
                    (start <= end).then_some((start, end))
                })
            })
            .collect();
        let inside = occupied.iter().all(|span| span.0 >= wall.0 && span.1 <= wall.1 && span.0 <= span.1);
        TimeRecord {
            requests_ms: observed(union(&self.requests)),
            first_byte_ms: Measure::unavailable("first-byte durations not reported"),
            largest_gap_ms: Measure::unavailable("largest-gap durations not reported"),
            tools_ms: self.tools.into_iter().map(|(name, spans)| (name, observed(union(&spans)))).collect(),
            compaction_ms: observed(union(&self.compaction)),
            children_ms: observed(union(&self.children)),
            rest_ms: observed(if inside {
                wall.1.checked_sub(wall.0).and_then(|wall| wall.checked_sub(union(&occupied)?))
            } else {
                None
            }),
            commands_outside_calls_ms: observed(
                union(&self.commands).and_then(|commands| commands.checked_sub(union(&intersection)?)),
            ),
        }
    }
}
