//! Typed outside observations retain process streams and their exact records.
//! The reader knows only reported facts, never domain state or missing metadata.
//! `feed` accepts chunks; `finish` checks framing, and measures retain loss gaps
//! (protocol/events.md, sections2/6; benchmarks.md, sections6.2/8/11.2).

use serde::{Deserialize, Serialize};
use smith_events::{Event, Reader, Record, SessionEnded, SessionStarted};

use crate::agent::{HarnessFault, Seen};
use crate::{Classification, Counts, Exit, Forced, TokenRecord};

mod delegation;
mod time;
mod usage;
mod usage_sums;

mod loss;

mod passive;

mod pairs;

mod classify;

mod checks;

mod tool_path;

mod names;

mod messages;

mod scopes;

mod final_turn;

mod carried;
mod compaction_choice;
mod threshold;

mod budget;
mod budget_report;

mod headroom;

mod cache;
mod cache_shape;

mod spend;

mod written_file;

mod file_pieces;
mod jobs;
mod slow;

/// A product invocation selected by the source face registry.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    /// The checked headless front end writes JSON records to stdout.
    Exec,
    /// The interactive front end writes records to a named trace.
    Chat,
}

/// One process stream, including its authoritative boundaries in `records`.
#[derive(Clone, Debug)]
pub struct Session {
    pub started: SessionStarted,
    pub records: Vec<Record>,
    pub ended: Option<SessionEnded>,
    /// A future record was skipped by the codec, so its type cannot be compared.
    pub unknown: bool,
}

/// An incremental outside reader paired with its caller's receiving bounds.
#[derive(Debug)]
pub struct Observer {
    reader: Reader,
    sessions: Vec<Session>,
    fault: Option<String>,
    unknown: bool,
    finished: bool,
}

fn fault(error: impl Into<String>) -> HarnessFault {
    HarnessFault { what: error.into() }
}

fn codec(error: smith_events::Error) -> HarnessFault {
    match error {
        smith_events::Error::Version(version) => {
            fault(format!("events version {version} is not {}", smith_events::VERSION))
        }
        smith_events::Error::Malformed => fault("events record is malformed or not newline-terminated"),
        smith_events::Error::Shape => fault("events record has an invalid known field"),
        smith_events::Error::Limits => fault("events record or receiving limits exceed the codec bound"),
        smith_events::Error::Text => fault("events content is not UTF-8"),
    }
}

impl Observer {
    /// Resolve the main conversation's first declared model and window.
    pub fn resolve_model(&self, identity: &mut crate::ModelIdentity) {
        identity.resolved_model = crate::Measure::unavailable("main conversation model not reported");
        identity.resolved_window = crate::Measure::unavailable("main conversation window not reported");
        for session in &self.sessions {
            for record in &session.records {
                if let Event::ConversationOpened(opened) = &record.event
                    && opened.kind == smith_events::ConversationKind::Main
                {
                    identity.resolved_model = std::str::from_utf8(&opened.model.model).map_or_else(
                        |error| crate::Measure::unavailable(error.to_string()),
                        |model| crate::Measure::Observed { value: model.into() },
                    );
                    identity.resolved_window = crate::Measure::Observed { value: opened.model.window_tokens };
                    return;
                }
            }
        }
    }

    /// Reserve the codec's bounded line before reading; invalid limits refuse.
    pub fn new(limits: &smith_events::Limits) -> Result<Self, HarnessFault> {
        Ok(Self {
            reader: Reader::new(limits).map_err(codec)?,
            sessions: Vec::new(),
            fault: None,
            unknown: false,
            finished: false,
        })
    }

    /// Reserve the arm's writer-derived byte ceiling, including the final newline.
    /// Semantic field and list caps remain checked by the codec independently.
    pub fn with_ceiling(limits: &smith_events::Limits, encoded_line: u32) -> Result<Self, HarnessFault> {
        Ok(Self {
            reader: Reader::with_ceiling(limits, encoded_line).map_err(codec)?,
            sessions: Vec::new(),
            fault: None,
            unknown: false,
            finished: false,
        })
    }

    /// Consume complete lines in order, retaining a partial line across chunks.
    /// Unknown records and fields remain subject to the codec's version check.
    pub fn feed(&mut self, mut bytes: &[u8]) -> Result<Vec<Seen>, HarnessFault> {
        self.finished = false;
        if let Some(error) = &self.fault {
            return Err(fault(error.clone()));
        }
        let mut seen = Vec::new();
        while !bytes.is_empty() {
            let result = self.reader.feed(bytes).map_err(codec).and_then(|line| {
                bytes = &bytes[line.consumed..];
                if let Some(record) = line.record {
                    self.record(record)
                } else {
                    if line.complete {
                        self.unknown = true;
                        if let Some(session) = self.sessions.last_mut() {
                            session.unknown = true;
                        }
                    }
                    Ok(Vec::new())
                }
            });
            match result {
                Ok(cues) => seen.extend(cues),
                Err(error) => {
                    self.fault = Some(error.what.clone());
                    return Err(error);
                }
            }
        }
        Ok(seen)
    }

    /// End an append stream only after a complete newline-terminated line.
    pub fn finish(&mut self) -> Result<(), HarnessFault> {
        if let Some(error) = &self.fault {
            return Err(fault(error.clone()));
        }
        self.reader.finish().map_err(|error| {
            let error = codec(error);
            self.fault = Some(error.what.clone());
            error
        })?;
        self.finished = true;
        Ok(())
    }

    /// A settled capture contains no known, unknown or partial startup record.
    #[must_use]
    pub fn ended_without_run(&self) -> bool {
        self.finished && self.sessions.is_empty() && !self.unknown && self.fault.is_none()
    }

    /// Retain exact typed records for checks and recorded-output audits.
    #[must_use]
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    fn record(&mut self, record: Record) -> Result<Vec<Seen>, HarnessFault> {
        if let Event::SessionStarted(started) = &record.event {
            self.sessions.push(Session {
                started: started.clone(),
                records: vec![record],
                ended: None,
                unknown: false,
            });
            return Ok(Vec::new());
        }
        let session = self.sessions.last_mut().ok_or_else(|| fault("event precedes session.started"))?;
        if session.ended.is_some() {
            return Err(fault("event follows session.ended without a new session.started"));
        }
        if session.records.last().is_some_and(|previous| previous.t_ms > record.t_ms) {
            return Err(fault("events timestamps moved backwards within a process stream"));
        }
        let mut seen = Vec::new();
        match &record.event {
            Event::SessionEnded(ended) => session.ended = Some(ended.clone()),
            Event::ResponseCompleted(response) => {
                if !session.records.iter().any(|previous| {
                    matches!(&previous.event,
                    Event::ResponseCompleted(value) if value.run == response.run && value.response == response.response)
                }) {
                    seen.push(Seen::Response);
                }
            }
            Event::ToolStarted(tool) => {
                seen.push(Seen::ToolCall { tool: String::from_utf8_lossy(&tool.tool).into_owned() });
            }
            Event::ConversationOpened(conversation) => {
                if conversation.kind == smith_events::ConversationKind::Child {
                    seen.push(Seen::Child);
                }
            }
            // A terminal, including a failure, lets the scripted person close.
            Event::RunCompleted(run) => {
                if !session.records.iter().any(|previous| {
                    matches!(&previous.event,
                    Event::RunCompleted(value) if value.run == run.run)
                }) {
                    seen.push(Seen::Answer);
                }
            }
            Event::SessionStarted(_)
            | Event::RunStarted(_)
            | Event::ConversationClosed(_)
            | Event::ResponseStarted(_)
            | Event::TextDelta(_)
            | Event::JobEnded(_)
            | Event::ToolCompleted(_)
            | Event::CheckStarted(_)
            | Event::CheckCompleted(_)
            | Event::MessageReceived(_)
            | Event::MessageRead(_)
            | Event::MessageUnread(_)
            | Event::Budget(_)
            | Event::Notice(_) => {}
        }
        session.records.push(record);
        Ok(seen)
    }

    /// Count per-response usage once within each session and run's actual scope.
    pub fn tokens(&self) -> Result<Vec<TokenRecord>, HarnessFault> {
        usage::tokens(&self.sessions, &mut Vec::new()).map_err(fault)
    }

    /// Retain each complete response's usage and conversation model for pricing.
    pub fn response_tokens(&self) -> Result<Vec<crate::ResponseTokens>, HarnessFault> {
        let mut responses = Vec::new();
        usage::tokens(&self.sessions, &mut responses).map_err(fault)?;
        Ok(responses)
    }

    /// Retain reported child shares, stops, turns and the union of main call spans.
    #[must_use]
    pub fn delegation(&self) -> crate::Delegation {
        delegation::delegation(self)
    }

    /// Read the agent process's peak from its authoritative session ends.
    #[must_use]
    pub fn own_peak_rss_bytes(&self) -> crate::Measure<u64> {
        passive::own_peak_rss_bytes(self)
    }

    /// Counts are exact only where the session explicitly reports no event loss.
    #[must_use]
    pub fn counts(&self) -> Counts {
        usage::counts(&self.sessions)
    }

    /// Hold genuine exit and force against the last authoritative run answer.
    #[must_use]
    pub fn classify(&self, exit: Exit, forced: Forced) -> Classification {
        classify::classify(self, exit, forced)
    }

    /// Evaluate an author predicate from typed records and the exact task text.
    #[must_use]
    pub fn check(&self, check: &crate::EventCheck, task: &str) -> crate::CheckVerdict {
        checks::evaluate(self, check, task, None)
    }

    /// Judge a spend predicate in the immutable prepared arm's native units.
    #[must_use]
    pub fn check_with_currency(&self, check: &crate::EventCheck, task: &str, currency: u64) -> crate::CheckVerdict {
        checks::evaluate(self, check, task, Some(currency))
    }

    /// Compare a captured successful write with its settled regular file.
    #[must_use]
    pub fn written_file(
        &self,
        path: &std::path::Path,
        minimum: u64,
        workspace: &std::path::Path,
    ) -> crate::CheckVerdict {
        written_file::check(self, path, minimum, workspace)
    }

    /// Judge one exact paired oversized write before any repair.
    #[must_use]
    pub fn oversized_write(
        &self,
        path: &std::path::Path,
        above: u64,
        protocol_input_bytes_above: Option<u64>,
        workspace: &std::path::Path,
    ) -> crate::CheckVerdict {
        file_pieces::oversized_check(self, path, above, protocol_input_bytes_above, workspace)
    }

    /// Apply successful captured pieces in order against the declared file bound.
    #[must_use]
    pub fn file_pieces(
        &self,
        path: &std::path::Path,
        above: u64,
        file_bytes: u64,
        workspace: &std::path::Path,
    ) -> crate::CheckVerdict {
        file_pieces::check(self, path, above, file_bytes, workspace)
    }

    /// Judge accepted pieces even when the protocol trigger was not exercised.
    #[must_use]
    pub fn file_pieces_for_protocol_probe(
        &self,
        path: &std::path::Path,
        above: u64,
        file_bytes: u64,
        protocol_bound: u64,
        workspace: &std::path::Path,
    ) -> crate::CheckVerdict {
        file_pieces::check_protocol(self, path, above, file_bytes, protocol_bound, workspace)
    }

    /// Report typed stream integrity, pairing, loss and spend observations.
    #[must_use]
    pub fn health(&self, exit: Exit, forced: Forced, teardown_bound_ms: u64) -> crate::Health {
        passive::health(self, exit, forced, teardown_bound_ms)
    }
}

fn run(event: &Event) -> Option<u64> {
    match event {
        Event::Budget(value) => Some(value.run),
        Event::RunStarted(value) => Some(value.run),
        Event::RunCompleted(value) => Some(value.run),
        Event::ConversationOpened(value) => Some(value.run),
        Event::ConversationClosed(value) => Some(value.run),
        Event::ResponseStarted(value) => Some(value.run),
        Event::ResponseCompleted(value) => Some(value.run),
        Event::TextDelta(value) => Some(value.run),
        Event::ToolStarted(value) => Some(value.run),
        Event::JobEnded(value) => Some(value.run),
        Event::ToolCompleted(value) => Some(value.run),
        Event::CheckStarted(value) => Some(value.run),
        Event::CheckCompleted(value) => Some(value.run),
        Event::MessageReceived(value) => Some(value.run),
        Event::MessageRead(value) => Some(value.run),
        Event::MessageUnread(value) => Some(value.run),
        Event::Notice(value) => value.run,
        Event::SessionStarted(_) | Event::SessionEnded(_) => None,
    }
}

pub(super) fn reported_spend(observer: &Observer, currency: u64) -> Result<crate::Spend, String> {
    if currency == 0 {
        return Err("notional currency unit must be positive".into());
    }
    let mut total = 0_u128;
    let mut saw_spend = false;
    let mut missing = Vec::new();
    for session in &observer.sessions {
        let mut runs = std::collections::BTreeSet::new();
        for record in &session.records {
            if let Event::RunCompleted(value) = &record.event {
                if !runs.insert(value.run) {
                    return Err("duplicate authoritative run spend".into());
                }
                saw_spend = true;
                total = total.checked_add(u128::from(value.spent)).ok_or("reported spend overflow")?;
            }
        }
        if let Some(reason) = usage::incomplete(session) {
            missing.push(reason);
        }
    }
    let value = total.to_string().parse::<f64>().map_err(|error| error.to_string())?
        / currency.to_string().parse::<f64>().map_err(|error| error.to_string())?;
    let usd = if !saw_spend {
        crate::Measure::unavailable("no authoritative run spend was reported")
    } else if missing.is_empty() {
        crate::Measure::Observed { value }
    } else {
        crate::Measure::LowerBound { value, missing }
    };
    Ok(crate::Spend {
        priced: crate::Measure::unavailable("common price basis not applied yet"),
        basis: crate::SpendBasis::Notional,
        usd,
    })
}

impl Observer {
    /// One repetition's exact cache observations; thresholds bind its triplet.
    #[must_use]
    pub fn cache_reuse(
        &self,
        median_at_least: f64,
        full_misses_per_ten_at_most: u32,
    ) -> crate::Measure<crate::CacheObservation> {
        if self.fault.is_some()
            || self.unknown
            || self.sessions.is_empty()
            || self.sessions.iter().any(|session| usage::incomplete(session).is_some())
        {
            crate::Measure::unavailable("incomplete event stream prevents exact cache observations")
        } else {
            cache::observation(self, median_at_least, full_misses_per_ten_at_most)
        }
    }
}
