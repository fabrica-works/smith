//! Visibility retains the last child-usage values actually told before a report.
//! Model reports compare that text, independently of later budget charges.
//! Smith's comparison joins each returned call to its writing-step Budget.
//! (benchmarks.md, section 11.1; domain/run.md, section 9.2).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use smith_events::{Block, ConversationKind, Event, Status};

use crate::Measure;

use super::Observer;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Remaining {
    turns: u64,
    seconds: u64,
}

fn told(text: &[u8]) -> Result<Option<Remaining>, &'static str> {
    static PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?s)\n\nChild usage:.*; ordinary left ([0-9]+) turns and ([^.]+)\.$")
            .expect("fixed native usage suffix")
    });
    let text = std::str::from_utf8(text).map_err(|_| "child usage text is not UTF-8")?;
    let Some(captures) = PATTERN.captures(text) else { return Ok(None) };
    let turns = captures[1].parse::<u64>().ok();
    let time = &captures[2];
    let seconds = match time {
        "less than a second" => Some(0),
        "1 second" => Some(1),
        _ => time.strip_suffix(" seconds").and_then(|seconds| seconds.parse::<u64>().ok()),
    };
    Ok(turns.zip(seconds).map(|(turns, seconds)| Remaining { turns, seconds }))
}

#[derive(PartialEq, Eq)]
struct Parent {
    conversation: u64,
    call: Box<[u8]>,
}

#[derive(Default)]
enum Report {
    #[default]
    Absent,
    Missing,
    Agrees,
    Differs,
}

#[derive(Default)]
struct State {
    main: BTreeSet<(u64, u64)>,
    children: BTreeMap<(u64, u64), Parent>,
    pending: BTreeSet<(u64, u64, Box<[u8]>)>,
    budgets: BTreeMap<(u64, Box<[u8]>), Remaining>,
    told: BTreeMap<u64, Remaining>,
    measured: bool,
    wrong: bool,
    report: Report,
}

impl State {
    fn record(&mut self, event: &Event, require_writing_budget: bool) -> Result<(), &'static str> {
        match event {
            Event::ConversationOpened(value) => match &value.kind {
                ConversationKind::Main => {
                    self.main.insert((value.run, value.conversation));
                }
                ConversationKind::Child => {
                    let parent = value.parent.ok_or("child lacks its actual parent")?;
                    let call = value.call.clone().ok_or("child lacks its actual parent call")?;
                    self.children.insert((value.run, value.conversation), Parent { conversation: parent, call });
                }
                ConversationKind::Compaction => {}
                ConversationKind::Unknown(_) => return Err("unknown conversation kind"),
            },
            Event::ToolCompleted(value) => {
                if let Some(child) = &value.child {
                    let opening =
                        self.children.get(&(value.run, child.conversation)).ok_or("child return lacks opening")?;
                    if opening.conversation != value.conversation || opening.call != value.call {
                        return Err("child return does not join its actual parent call");
                    }
                    self.pending.insert((value.run, value.conversation, value.call.clone()));
                }
            }
            Event::ResponseStarted(value) if self.main.contains(&(value.run, value.conversation)) => {
                let prompt = value.prompt.as_ref().ok_or("main's report request was not captured")?;
                for message in &prompt.messages {
                    for block in &message.blocks {
                        if let Block::Result { call, text, .. } = block {
                            let identity = (value.run, value.conversation, call.clone());
                            if self.pending.remove(&identity) {
                                self.measured = true;
                                match told(text)? {
                                    Some(actual) => {
                                        if require_writing_budget {
                                            let expected = self
                                                .budgets
                                                .get(&(value.run, call.clone()))
                                                .ok_or("child usage lacks its identified writing-step Budget")?;
                                            self.wrong |= actual != *expected;
                                        }
                                        self.told.insert(value.run, actual);
                                    }
                                    None => return Err("actual returned child usage values were not captured"),
                                }
                            }
                        }
                    }
                }
            }
            Event::RunCompleted(value) if value.status == Status::Accepted => {
                match self.told.get(&value.run).zip(value.result.as_ref().and_then(|result| result.text.as_ref())) {
                    Some((expected, text)) => {
                        if text.as_ref() != format!("ALPHA {} {}", expected.turns, expected.seconds).as_bytes() {
                            self.report = Report::Differs;
                        } else if matches!(self.report, Report::Absent) {
                            self.report = Report::Agrees;
                        }
                    }
                    None if !matches!(self.report, Report::Differs) => self.report = Report::Missing,
                    None => {}
                }
            }
            Event::Budget(value) if require_writing_budget => {
                if let Some(call) = &value.call {
                    let ordinary = Remaining {
                        turns: value.turns.limit.saturating_sub(value.turns.used).saturating_sub(value.reserve.turns),
                        seconds: value
                            .time_ms
                            .limit
                            .saturating_sub(value.time_ms.used)
                            .saturating_sub(value.reserve.time_ms)
                            / 1000,
                    };
                    self.budgets.insert((value.run, call.clone()), ordinary);
                }
            }
            Event::Budget(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::RunStarted(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::RunCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::JobEnded(_)
            | Event::Notice(_) => {}
        }
        Ok(())
    }
}

pub(super) fn check(observer: &Observer, report: bool) -> Measure<bool> {
    let mut measured = false;
    for session in &observer.sessions {
        let mut state = State::default();
        for record in &session.records {
            if let Err(reason) = state.record(&record.event, !report) {
                return Measure::unavailable(reason);
            }
        }
        if !state.measured || !state.pending.is_empty() {
            return Measure::unavailable("visibility lacks actual returned usage or captured report");
        }
        measured = true;
        let wrong = if report {
            match state.report {
                Report::Absent | Report::Missing => return Measure::unavailable("main report was not captured"),
                Report::Agrees => false,
                Report::Differs => true,
            }
        } else {
            state.wrong
        };
        if wrong {
            return Measure::Observed { value: false };
        }
    }
    if measured { Measure::Observed { value: true } } else { Measure::unavailable("no visibility evidence") }
}
