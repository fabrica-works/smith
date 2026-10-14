//! Budget prose joins its immutable opening and the latest run-scoped charge.
//! Child returns join rendered results in their actual parent's next request;
//! their usage retains the return-time budget. Missing capture or links remain
//! unavailable. Millisecond projections retain their submillisecond uncertainty,
//! never guessed nanoseconds (domain/run.md, sections5.3/9.2; protocol/events.md,
//! sections3.4/3.6/3.8; benchmarks.md, section11.1).

use std::collections::{BTreeMap, BTreeSet};

use smith_events::{
    Budget, BudgetStage, BudgetStanding, ChildStop, ConversationKind, ConversationOpened, Event, Record, RunStarted,
    ToolChoice, ToolCompleted,
};

use crate::Measure;

use super::{Observer, Session};

type Checked = Result<bool, &'static str>;

const AGENTS: &str = "Sub-agents share only what is above the reserve. Ask for focused turn or time shares; leaving a part unasked gives the child all of that part left.\n";

/// The range of native durations whose floor is the recorded milliseconds.
#[derive(Clone, Copy)]
struct Duration {
    least: u128,
    most: u128,
}

impl Duration {
    fn recorded(milliseconds: u64) -> Self {
        let least = u128::from(milliseconds) * 1_000_000;
        Self { least, most: least + 999_999 }
    }

    fn elapsed(previous: u64, now: u64) -> Self {
        let middle = u128::from(now.saturating_sub(previous)) * 1_000_000;
        Self { least: middle.saturating_sub(999_999), most: middle + 999_999 }
    }

    fn subtract(self, other: Self) -> Self {
        Self { least: self.least.saturating_sub(other.most), most: self.most.saturating_sub(other.least) }
    }

    fn prose(self) -> BTreeSet<String> {
        let mut prose = BTreeSet::from([duration(self.least), duration(self.most)]);
        if self.least == 0 && self.most > 0 {
            prose.insert(duration(1));
        }
        prose
    }
}

fn duration(nanoseconds: u128) -> String {
    match nanoseconds / 1_000_000_000 {
        0 if nanoseconds > 0 => "less than a second".into(),
        1 => "1 second".into(),
        seconds => format!("{seconds} seconds"),
    }
}

fn ordinary_time(value: &BudgetStanding, at: u64, now: u64) -> Duration {
    Duration::recorded(value.time_ms.limit)
        .subtract(Duration::recorded(value.time_ms.used))
        .subtract(Duration::elapsed(at, now))
        .subtract(Duration::recorded(value.reserve.time_ms))
}

fn main_system(text: &str, budget: &Budget, opened: Duration) -> bool {
    opened.prose().iter().any(|time| {
        Duration::recorded(budget.reserve.time_ms).prose().iter().any(|reserve| {
            let prose = format!("\n## Budget\n\nAt open, you have {} turns and {time} of wall time. Spend is bounded too.\nKeep {} turns and {reserve} to finish. When the ordinary budget is spent, live children close and you finish on that reserve.\n", budget.turns, budget.reserve.turns);
            text.ends_with(&prose) || text.strip_suffix(AGENTS).is_some_and(|text| text.ends_with(&prose))
        })
    })
}

fn child_system(text: &str, opening: &ConversationOpened) -> Checked {
    let share = opening.share.as_ref().ok_or("child opening lacks its effective share")?;
    Ok(Duration::recorded(share.time_ms).prose().iter().any(|time| {
        let prefix = format!(
            "\n## Your share\n\nAt open, your share is {} turns and {time}. Your answer may hold at most ",
            share.turns
        );
        let Some((_, tail)) = text.rsplit_once(&prefix) else { return false };
        let Some(cap) = tail.strip_suffix(
            " bytes; anything past it is cut with a marker. Your last turn has no tools: answer with what you have.\n",
        ) else {
            return false;
        };
        // This predicate observes the share, not an unreported answer-byte cap.
        cap.parse::<u32>().is_ok_and(|value| value > 0 && cap == value.to_string())
    }))
}

fn snapshot(start: &RunStarted, value: &BudgetStanding) -> Checked {
    if value.turns.limit != start.budget.turns
        || value.spend.limit != start.budget.spend
        || value.time_ms.limit != start.budget.time_ms
        || value.reserve != start.budget.reserve
    {
        return Ok(false);
    }
    match &value.stage {
        BudgetStage::Ordinary | BudgetStage::WindDown => Ok(true),
        BudgetStage::Unknown(_) => Err("budget stage is unknown"),
    }
}

fn share(opening: &ConversationOpened, value: &BudgetStanding, at: u64, now: u64) -> Checked {
    let share = opening.share.as_ref().ok_or("child opening lacks its effective share")?;
    match &value.stage {
        BudgetStage::Ordinary => {}
        BudgetStage::WindDown => return Ok(false),
        BudgetStage::Unknown(_) => return Err("child opening has an unknown budget stage"),
    }
    let turns = value.turns.limit.saturating_sub(value.turns.used).saturating_sub(value.reserve.turns);
    let spend = value.spend.limit.saturating_sub(value.spend.used).saturating_sub(value.reserve.spend);
    let time = ordinary_time(value, at, now);
    if share.turns == 0
        || share.turns > turns
        || share.spend == 0
        || share.spend > spend
        || Duration::recorded(share.time_ms).least > time.most
    {
        return Ok(false);
    }
    if Duration::recorded(share.time_ms).most > time.least {
        return Err("child time share touches the reserve within submillisecond record precision");
    }
    Ok(true)
}

fn usage(tool: &ToolCompleted, text: &[u8], value: &BudgetStanding, at: u64, now: u64) -> Checked {
    let child = tool.child.as_ref().expect("child return selected");
    let stopped = match &child.stop {
        ChildStop::Budget => "yes",
        ChildStop::End | ChildStop::Failed | ChildStop::Closed => "no",
        ChildStop::Unknown(_) => return Err("returned child stop is unknown"),
    };
    let text = std::str::from_utf8(text).map_err(|_| "returned child usage text is not UTF-8")?;
    let left = value.turns.limit.saturating_sub(value.turns.used).saturating_sub(value.reserve.turns);
    Ok(Duration::recorded(child.time_ms).prose().iter().any(|used| {
        ordinary_time(value, at, now).prose().iter().any(|time| {
            text.ends_with(&format!("\n\nChild usage: used {} turns and {used}; stopped for budget: {stopped}; ordinary left {left} turns and {time}.", child.turns))
        })
    }))
}

fn wind_down(prompt: &smith_events::Prompt, reserve: &smith_events::Reserve) -> bool {
    Duration::recorded(reserve.time_ms).prose().iter().any(|time| {
        let expected = format!("The ordinary budget is spent. Finish now with what you have. You have at most {} final completions and {time} to finish. Use only finish, or deliver when offered.", reserve.turns);
        prompt.messages.iter().any(|message| message.blocks.iter().any(|block| match block {
            smith_events::Block::Text { text } => text.windows(expected.len()).any(|text| text == expected.as_bytes()),
            smith_events::Block::Refusal { .. } | smith_events::Block::Unknown { .. }
            | smith_events::Block::Call { .. } | smith_events::Block::Oversized { .. }
            | smith_events::Block::Cut { .. } | smith_events::Block::Result { .. }
            | smith_events::Block::Opaque { .. } => false,
        }))
    })
}

/// References retain exact identities within one process stream only.
#[derive(Default)]
struct State<'a> {
    runs: BTreeMap<u64, (&'a RunStarted, u64)>,
    budgets: BTreeMap<u64, (&'a BudgetStanding, u64)>,
    openings: BTreeMap<(u64, u64), (&'a ConversationOpened, u64)>,
    systems: BTreeMap<(u64, u64), Box<[u8]>>,
    pending: BTreeMap<(u64, u64, Box<[u8]>), Returned<'a>>,
    returned: BTreeSet<(u64, u64)>,
    winding: BTreeSet<u64>,
    winding_note: BTreeSet<u64>,
}

/// A child terminal and the budget used when its feedback was made.
struct Returned<'a> {
    tool: &'a ToolCompleted,
    budget: &'a BudgetStanding,
    budget_at: u64,
    at: u64,
}

impl<'a> State<'a> {
    fn opening(&mut self, value: &'a ConversationOpened, now: u64) -> Checked {
        if !self.runs.contains_key(&value.run) {
            return Err("conversation opening lacks its actual run admission");
        }
        if value.kind == ConversationKind::Child {
            let parent = value.parent.ok_or("child opening lacks its actual parent")?;
            if !self.openings.contains_key(&(value.run, parent)) || value.call.is_none() {
                return Err("child opening lacks its actual parent call");
            }
            let (budget, at) = self.budgets.get(&value.run).ok_or("child opening lacks a preceding run budget")?;
            if !share(value, budget, *at, now)? {
                return Ok(false);
            }
        }
        if self.openings.insert((value.run, value.conversation), (value, now)).is_some() {
            return Ok(false);
        }
        Ok(true)
    }

    fn request(&mut self, value: &smith_events::ResponseStarted) -> Checked {
        let identity = (value.run, value.conversation);
        let (opening, at) = self.openings.get(&identity).ok_or("request lacks its actual opening")?;
        if opening.kind == ConversationKind::Compaction {
            return Ok(true);
        }
        if matches!(&opening.kind, ConversationKind::Unknown(_)) {
            return Err("opening kind is unknown");
        }
        let prompt = value.prompt.as_ref().ok_or("budget request prompt was not captured")?;
        for message in &prompt.messages {
            for block in &message.blocks {
                if let smith_events::Block::Result { call, text, .. } = block {
                    let identity = (value.run, value.conversation, call.clone());
                    if let Some(returned) = self.pending.remove(&identity) {
                        if !usage(returned.tool, text, returned.budget, returned.budget_at, returned.at)? {
                            return Ok(false);
                        }
                        let child = returned.tool.child.as_ref().expect("retained child return");
                        self.returned.insert((value.run, child.conversation));
                    }
                }
            }
        }
        if let Some(system) = &prompt.system {
            if let Some(previous) = self.systems.get(&identity) {
                if previous != system {
                    return Ok(false);
                }
            } else {
                let text = std::str::from_utf8(system).map_err(|_| "budget system text is not UTF-8")?;
                let agrees = match &opening.kind {
                    ConversationKind::Main => {
                        let (run, started) = self.runs.get(&value.run).expect("opening checked admission");
                        let time = Duration::recorded(run.budget.time_ms).subtract(Duration::elapsed(*started, *at));
                        main_system(text, &run.budget, time)
                    }
                    ConversationKind::Child => child_system(text, opening)?,
                    ConversationKind::Compaction | ConversationKind::Unknown(_) => unreachable!("classified opening"),
                };
                if !agrees {
                    return Ok(false);
                }
                self.systems.insert(identity, system.clone());
            }
        } else if !self.systems.contains_key(&identity) {
            return Err("opening budget system text was not captured");
        }
        if opening.kind == ConversationKind::Main && self.winding.contains(&value.run) {
            match &value.tool_choice {
                ToolChoice::Only => {}
                ToolChoice::Auto | ToolChoice::None => return Ok(false),
                ToolChoice::Unknown(_) => return Err("wind-down request tool choice is unknown"),
            }
            if !self.winding_note.contains(&value.run) {
                let (run, _) = self.runs.get(&value.run).expect("opening checked admission");
                if !wind_down(prompt, &run.budget.reserve) {
                    return Ok(false);
                }
                self.winding_note.insert(value.run);
            }
        }
        Ok(true)
    }

    fn returned(&mut self, value: &'a ToolCompleted, now: u64) -> Checked {
        let child = value.child.as_ref().expect("child return selected");
        let identity = (value.run, child.conversation);
        let (opening, _) = self.openings.get(&identity).ok_or("child return lacks its actual opening")?;
        if opening.parent != Some(value.conversation) || opening.call.as_ref() != Some(&value.call) {
            return Err("child return does not join its actual parent call");
        }
        let (budget, at) = self.budgets.get(&value.run).ok_or("child usage lacks a preceding run budget")?;
        let identity = (value.run, value.conversation, value.call.clone());
        let returned = Returned { tool: value, budget, budget_at: *at, at: now };
        if self.pending.insert(identity, returned).is_some() {
            return Ok(false);
        }
        Ok(true)
    }

    fn record(&mut self, record: &'a Record) -> Checked {
        match &record.event {
            Event::RunStarted(value) => {
                if self.runs.insert(value.run, (value, record.t_ms)).is_some() {
                    return Ok(false);
                }
            }
            Event::Budget(value) => {
                let (run, _) = self.runs.get(&value.run).ok_or("budget snapshot lacks its run admission")?;
                if !snapshot(run, value)? {
                    return Ok(false);
                }
                if value.stage == BudgetStage::WindDown {
                    self.winding.insert(value.run);
                }
                if value.stage == BudgetStage::Ordinary && self.winding.contains(&value.run) {
                    return Ok(false);
                }
                self.budgets.insert(value.run, (value, record.t_ms));
            }
            Event::ConversationOpened(value) => return self.opening(value, record.t_ms),
            Event::ResponseStarted(value) => return self.request(value),
            Event::ToolCompleted(value) if value.child.is_some() => return self.returned(value, record.t_ms),
            Event::RunCompleted(value) => {
                if value.wind_down != self.winding.contains(&value.run) {
                    return Ok(false);
                }
            }
            Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::SessionStarted(_)
            | Event::SessionEnded(_)
            | Event::ConversationClosed(_)
            | Event::ResponseCompleted(_)
            | Event::TextDelta(_)
            | Event::ToolStarted(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::Notice(_) => {}
        }
        Ok(true)
    }
}

fn session(session: &Session) -> Checked {
    let mut state = State::default();
    for record in &session.records {
        if !state.record(record)? {
            return Ok(false);
        }
    }
    for (identity, (opening, _)) in &state.openings {
        if !state.budgets.contains_key(&opening.run) {
            return Err("conversation lacks its actual run budget snapshot");
        }
        if opening.kind == ConversationKind::Child && !state.returned.contains(identity) {
            return Err("child lacks captured returned usage");
        }
        if matches!(opening.kind, ConversationKind::Main | ConversationKind::Child)
            && !state.systems.contains_key(identity)
        {
            return Err("conversation lacks its captured opening budget prose");
        }
    }
    if state.budgets.is_empty() || state.systems.is_empty() {
        return Err("budget visibility lacks a captured report and actual budget snapshot");
    }
    Ok(true)
}

pub(super) fn check(observer: &Observer) -> Measure<bool> {
    for value in &observer.sessions {
        match session(value) {
            Ok(true) => {}
            Ok(false) => return Measure::Observed { value: false },
            Err(reason) => return Measure::unavailable(reason),
        }
    }
    Measure::Observed { value: true }
}
