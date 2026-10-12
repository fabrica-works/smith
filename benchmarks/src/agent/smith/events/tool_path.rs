//! A forced namespace spelling is joined to its actual call terminal.
//! Only captured JSON input and immutable session/run/conversation/call/tool
//! identity are known. Host aliases and resolved-path metadata are not inferred
//! (domain/tools.md, section 2; protocol/events.md, section 5).

use std::path::{Component, Path};

use smith_events::Event;

use crate::Measure;

use super::{Observer, names};

fn namespace(path: &str) -> Option<Vec<Component<'_>>> {
    let components: Vec<_> = Path::new(path).components().filter(|part| *part != Component::CurDir).collect();
    if components.is_empty() || path.contains('\0') { None } else { Some(components) }
}

pub(super) fn check(observer: &Observer, tool: &str, path: &str, aliases: &[String], verdict: &str) -> Measure<bool> {
    let mut missing = false;
    for session in &observer.sessions {
        for (index, record) in session.records.iter().enumerate() {
            if let Event::ToolStarted(start) = &record.event {
                if start.tool.as_ref() != tool.as_bytes() {
                    continue;
                }
                let Some(input) = &start.input else {
                    missing = true;
                    continue;
                };
                let Ok(input) = serde_json::from_slice::<serde_json::Value>(input) else {
                    continue;
                };
                let Some(requested) = input.get("path").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                if namespace(requested).is_none()
                    || (namespace(requested) != namespace(path)
                        && !aliases.iter().any(|alias| namespace(requested) == namespace(alias)))
                {
                    continue;
                }
                let terminal = session.records[index + 1..].iter().find_map(|record| {
                    if let Event::ToolCompleted(end) = &record.event
                        && end.run == start.run
                        && end.conversation == start.conversation
                        && end.call == start.call
                        && end.tool == start.tool
                    {
                        return Some(end);
                    }
                    None
                });
                if let Some(terminal) = terminal {
                    if names::verdict(&terminal.verdict) == verdict.as_bytes() {
                        return Measure::Observed { value: true };
                    }
                } else {
                    missing = true;
                }
            }
        }
    }
    if missing {
        Measure::unavailable("path input or its exact tool terminal was not captured")
    } else {
        Measure::Observed { value: false }
    }
}
