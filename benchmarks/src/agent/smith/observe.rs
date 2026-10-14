//! Native observations retain the event codec's scopes, terminals and usage.
//! Receiving bounds follow the selected product writer;
//! no running domain state is known (benchmarks.md, sections 6 and 8).

use crate::agent::{HarnessFault, Seen};
use crate::{Counts, Spend, TokenRecord};

use super::Prepared;

fn response_rejected(value: &smith_events::ResponseCompleted) -> bool {
    value.failure.as_ref().is_some_and(|failure| {
        failure.evidence == smith_events::Evidence::Response
            && failure.status.is_some_and(|status| (400..500).contains(&status))
    }) && value.outcome == smith_events::Outcome::Failed
        && value.blocks == 0
        && value.calls == 0
        && value.invalid == 0
        && value.oversized == 0
        && value.cut == 0
        && value.completion.is_none()
        && value.usage.output_tokens.is_none_or(|tokens| tokens == 0)
        && value.usage.reasoning_tokens.is_none_or(|tokens| tokens == 0)
}

fn rejected_responses(sessions: &[super::events::Session], expected: u64) -> bool {
    use smith_events::{Capture, Event};
    let mut started = std::collections::BTreeSet::new();
    let mut rejected = std::collections::BTreeSet::new();
    for (index, session) in sessions.iter().enumerate() {
        if session.started.capture != Capture::Everything {
            return false;
        }
        for record in &session.records {
            match &record.event {
                Event::ResponseStarted(value) => {
                    if !started.insert((index, value.run, value.conversation, value.response)) {
                        return false;
                    }
                }
                Event::ResponseCompleted(value) => {
                    if !response_rejected(value)
                        || !rejected.insert((index, value.run, value.conversation, value.response))
                    {
                        return false;
                    }
                }
                Event::TextDelta(_) => return false,
                Event::SessionStarted(_)
                | Event::SessionEnded(_)
                | Event::RunStarted(_)
                | Event::RunCompleted(_)
                | Event::ConversationOpened(_)
                | Event::ConversationClosed(_)
                | Event::ToolStarted(_)
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
        }
    }
    expected > 0 && u64::try_from(started.len()).ok() == Some(expected) && started == rejected
}

/// An event observer paired with its private invocation and writer bounds.
#[derive(Debug)]
pub struct Observing {
    pub prepared: Prepared,
    pub observer: super::events::Observer,
}

impl Observing {
    /// Retain an outside observer with no knowledge of the running domains.
    pub fn new(prepared: Prepared) -> Result<Self, HarnessFault> {
        let receiving = prepared.registry.receiving;
        let limits = receiving.limits().map_err(|what| HarnessFault { what })?;
        let observer = super::events::Observer::with_ceiling(&limits, receiving.encoded_line)?;
        Ok(Self { prepared, observer })
    }

    /// Consume one complete event line and report its script cues.
    pub fn feed_line(&mut self, line: &str) -> Result<Vec<Seen>, HarnessFault> {
        self.observer.feed(format!("{line}\n").as_bytes())
    }

    /// Preserve the stream's passive evidence and name its missing fields.
    #[must_use]
    pub fn health(&self, exit: crate::Exit, forced: crate::Forced) -> Option<crate::Health> {
        let mut health = self.observer.health(exit, forced, self.prepared.teardown.as_nanos() / 1_000_000);
        let whole = ["stream-whole", "nothing-lost", "paired"].iter().all(|name| {
            health
                .passive_checks
                .iter()
                .any(|check| check.check == *name && check.passed == crate::Measure::Observed { value: true })
        });
        if whole
            && let crate::Measure::Observed { value: attempts } = self.observer.counts().provider_attempts
            && rejected_responses(self.observer.sessions(), attempts)
        {
            health.passive_checks.push(crate::CheckVerdict {
                check: "provider-rejected-before-output".into(),
                passed: crate::Measure::Observed { value: true },
                evidence: vec![format!("all {attempts} observed requests ended with 4xx before output")],
            });
        }
        Some(health)
    }

    /// Author checks read the event stream in its micro-dollar currency.
    #[must_use]
    pub fn checks(&self, checks: &[crate::EventCheck], task: &str) -> Vec<crate::CheckVerdict> {
        checks.iter().map(|check| self.observer.check_with_currency(check, task, 1_000_000)).collect()
    }

    /// File-dependent predicates run after the process tree settles.
    #[must_use]
    pub fn checks_in_workspace(
        &self,
        checks: &[crate::EventCheck],
        task: &str,
        workspace: &std::path::Path,
    ) -> Vec<crate::CheckVerdict> {
        checks
            .iter()
            .map(|check| match check {
                crate::EventCheck::WrittenFile { path, content_bytes_at_least } => {
                    self.observer.written_file(path, *content_bytes_at_least, workspace)
                }
                crate::EventCheck::OversizedWrite { path, content_bytes_above, protocol_input_bytes_above } => {
                    self.observer.oversized_write(path, *content_bytes_above, *protocol_input_bytes_above, workspace)
                }
                crate::EventCheck::FilePieces { path, bytes_above, file_bytes } => {
                    let protocol_bound = checks.iter().find_map(|check| {
                        if let crate::EventCheck::OversizedWrite { protocol_input_bytes_above, .. } = check {
                            *protocol_input_bytes_above
                        } else {
                            None
                        }
                    });
                    match protocol_bound {
                        Some(bound) => self.observer.file_pieces_for_protocol_probe(
                            path,
                            *bytes_above,
                            *file_bytes,
                            bound,
                            workspace,
                        ),
                        None => self.observer.file_pieces(path, *bytes_above, *file_bytes, workspace),
                    }
                }
                crate::EventCheck::CallCount { .. }
                | crate::EventCheck::ToolCall { .. }
                | crate::EventCheck::UsageSums
                | crate::EventCheck::NoVerdict { .. }
                | crate::EventCheck::Completions { .. }
                | crate::EventCheck::FirstRequestHoldsTask
                | crate::EventCheck::NothingBeforeTask
                | crate::EventCheck::HandledOnce
                | crate::EventCheck::CompletionPerExchange
                | crate::EventCheck::NoStop { .. }
                | crate::EventCheck::Blocks { .. }
                | crate::EventCheck::ConversationOpened { .. }
                | crate::EventCheck::PromptWithinWindow
                | crate::EventCheck::CacheReuse { .. }
                | crate::EventCheck::MainAfterChild
                | crate::EventCheck::FinalTurn
                | crate::EventCheck::CompactionChoice
                | crate::EventCheck::CompactionCarried
                | crate::EventCheck::ChildShare { .. }
                | crate::EventCheck::BudgetVisible
                | crate::EventCheck::SpendWithin { .. }
                | crate::EventCheck::CallsInOneResponse { .. }
                | crate::EventCheck::ChildrenOverlap { .. }
                | crate::EventCheck::NoFailure { .. }
                | crate::EventCheck::RunCompleted { .. }
                | crate::EventCheck::ProviderFailure { .. }
                | crate::EventCheck::NoRequest
                | crate::EventCheck::ToolPath { .. }
                | crate::EventCheck::ToolResultBytes { .. }
                | crate::EventCheck::ToolTimeout { .. }
                | crate::EventCheck::SlowCompletion { .. }
                | crate::EventCheck::BudgetTold
                | crate::EventCheck::BudgetReport
                | crate::EventCheck::CallsPerResponse { .. }
                | crate::EventCheck::ChildrenInTurn { .. }
                | crate::EventCheck::NoRun => self.checks(std::slice::from_ref(check), task).remove(0),
            })
            .collect()
    }

    /// Count the typed events observed in the process stream.
    #[must_use]
    pub fn counts(&self) -> Counts {
        self.observer.counts()
    }

    /// Retain each child's observed share and terminal.
    #[must_use]
    pub fn delegation(&self) -> crate::Delegation {
        self.observer.delegation()
    }

    /// Count scoped response usage once.
    pub fn tokens(&self) -> Result<Vec<TokenRecord>, String> {
        self.observer.tokens().map_err(|error| error.what)
    }

    /// Read the producer's notional spend in micro-dollars.
    pub fn spend(&self, _records: &[TokenRecord]) -> Result<Spend, String> {
        super::events::reported_spend(&self.observer, 1_000_000)
    }
}

#[cfg(test)]
#[path = "../../../tests/common/events.rs"]
mod test_events;

#[cfg(test)]
mod tests {
    use super::{rejected_responses, test_events};
    use smith_events::{CompletionFailure, Event, Evidence, FailureClass, Outcome, Usage};

    fn rejection() -> super::super::events::Session {
        let mut observer = super::super::events::Observer::new(&test_events::LIMITS).expect("limits");
        observer.feed(&test_events::bytes(&test_events::balanced())).expect("records");
        observer.finish().expect("complete");
        let mut session = observer.sessions()[0].clone();
        let Event::ResponseCompleted(value) = &mut session.records[4].event else { panic!("response terminal") };
        value.outcome = Outcome::Failed;
        value.blocks = 0;
        value.calls = 0;
        value.invalid = 0;
        value.oversized = 0;
        value.cut = 0;
        value.completion = None;
        value.usage = Usage {
            input_tokens: None,
            cache_read_tokens: None,
            cache_write_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
        };
        value.failure = Some(CompletionFailure {
            class: FailureClass::Invalid,
            evidence: Evidence::Response,
            status: Some(400),
            retry_after_ms: None,
            phase: None,
            limit: None,
            detail: None,
        });
        session
    }

    #[test]
    fn zero_generation_requires_every_request_to_end_in_a_4xx_without_output() {
        let mut session = rejection();
        assert!(rejected_responses(std::slice::from_ref(&session), 1));
        assert!(rejected_responses(&[session.clone(), session.clone()], 2));
        assert!(!rejected_responses(std::slice::from_ref(&session), 0));
        assert!(!rejected_responses(std::slice::from_ref(&session), 2));
        let started = session.records[3].clone();
        session.records.push(started);
        assert!(!rejected_responses(std::slice::from_ref(&session), 1));
        session.records.pop();
        let Event::ResponseCompleted(value) = &mut session.records[4].event else { panic!("response terminal") };
        value.failure.as_mut().expect("failure").status = Some(503);
        assert!(!rejected_responses(std::slice::from_ref(&session), 1));
        assert!(!rejected_responses(&[rejection(), session], 2));
        let mut session = rejection();
        let Event::ResponseCompleted(value) = &mut session.records[4].event else { panic!("response terminal") };
        value.usage.output_tokens = Some(1);
        assert!(!rejected_responses(std::slice::from_ref(&session), 1));
        let mut session = rejection();
        let Event::ResponseCompleted(value) = &mut session.records[4].event else { panic!("response terminal") };
        value.blocks = 1;
        assert!(!rejected_responses(std::slice::from_ref(&session), 1));
        let mut session = rejection();
        session.records.push(test_events::record(include_str!(
            "../../../../crates/smith-events/golden/v2/text_delta_everything.jsonl"
        )));
        assert!(!rejected_responses(std::slice::from_ref(&session), 1));
        session.records.pop();
        session.records.remove(4);
        assert!(!rejected_responses(std::slice::from_ref(&session), 1));
    }
}
