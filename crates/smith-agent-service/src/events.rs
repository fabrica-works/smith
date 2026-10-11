//! Event projection for one agent invocation, before its component hand-offs.
//! The producer keeps admitted scalar metadata, configured endpoint names and
//! bounded joins. It never parses a provider or learns credentials. `fact`
//! precedes `request`; the service supplies the exact `answer` it sends.
//! Contracts: protocol/events.md, sections 3–5; domain/run.md, section 11.

#![expect(clippy::manual_map, clippy::manual_let_else, reason = "bounded production entries use explicit matches without closures")]

use alloc::boxed::Box;
use skein_lib::{Duration, List, Map, Time, Token};
use smith_domain::{self as domain, run};
use smith_events::{Event, Record};
use smith_protocol_channel as channel;
use smith_protocol_llm as llm;

use crate::sinks::{Capture, SinkLimits, Sinks};

/// Startup's bounded event configuration, supplied before any descriptor is adopted.
#[derive(Clone, Copy, Debug)]
pub struct EventLimits {
    pub sinks: SinkLimits,
    pub capture: Capture,
    /// IO's stall deadline for each adopted destination.
    pub write_deadline: Duration,
}

impl EventLimits {
    /// Derive receiving and destination ownership from the admitted source bounds.
    /// `option_bytes` is the shell's measured largest configured effort string.
    pub fn derive(limits: &crate::Limits, capture: Capture, destinations: u32, option_bytes: u32, write_deadline: Duration) -> Option<EventLimits> {
        let charter = limits.channel.charter;
        let string = limits.llm.adapter.client.strings
            .max(charter.llm_endpoint).max(charter.llm_model).max(charter.host_tool_name)
            .max(charter.field_name).max(charter.run_result_label_some).max(option_bytes).max(46);
        let content = u32::try_from(limits.domain.run.outcome_bytes).ok()?;
        let items = charter.run_result_fields.max(limits.domain.run.host_tools).max(18);
        let records = smith_events::Limits { string, content, items };
        let line = crate::sinks::largest_line(&records, capture)?;
        let sinks = SinkLimits {
            destinations,
            output: line.checked_mul(5)?,
            records,
            conversations: limits.domain.run.conversations,
        };
        crate::sinks::sinks_worst_case(&sinks, capture)?;
        Some(EventLimits { sinks, capture, write_deadline })
    }
}

#[derive(Debug)]
struct Start {
    messages: u64,
    llm: run::charter::Llm,
    budget: smith_events::Budget,
    tools: smith_events::Tools,
    contract: List<smith_events::Form>,
}

/// One invocation's typed records, supplied to its bounded destination drain.
#[derive(Debug)]
pub(crate) struct Producer {
    pub(crate) sinks: Sinks,
    names: channel::Endpoints,
    efforts: Map<u32, Box<[u8]>>,
    capture: Capture,
    origin: Option<Time>,
    start: Option<Start>,
    admitted: Option<u64>,
    answer_at: Option<Time>,
    wind_down: bool,
    unread: u64,
    check_at: Option<Time>,
}

impl Producer {
    pub(crate) fn new(limits: &crate::Limits, names: channel::Endpoints, endpoints: &llm::Endpoints, declared: &[run::charter::Endpoint]) -> Producer {
        let events = limits.events.expect("event configuration exists");
        let mut efforts = Map::with_capacity(limits.channel.endpoints);
        for endpoint in declared {
            let options = endpoints.resolve(domain::llm::Endpoint(endpoint.0)).expect("configured domain endpoint resolves");
            if let Some(effort) = &options.reasoning_effort {
                assert!(u32::try_from(effort.len()).expect("configured option length") <= events.sinks.records.string);
                efforts.insert(endpoint.0, effort.clone()).expect("bounded configured endpoints");
            }
        }
        Producer {
            sinks: Sinks::new(&events.sinks, events.capture, limits.io.output), names, efforts,
            capture: events.capture, origin: None, start: None, admitted: None, answer_at: None,
            wind_down: false, unread: 0, check_at: None,
        }
    }

    pub(crate) fn started(&mut self, now: Time, header: smith_events::SessionStarted) {
        assert!(self.origin.is_none(), "one process startup before its first pass");
        self.origin = Some(now);
        self.sinks.record(&Record { t_ms: 0, event: Event::SessionStarted(header) });
        self.notice(now, smith_events::NoticeKind::Unknown(b"not_contained".as_slice().into()), None, None, None);
    }

    pub(crate) fn starting(&mut self, start: &channel::DecodedStart) {
        assert!(self.start.is_none(), "one Start per invocation");
        let charter = &start.charter;
        let mut families = List::with_capacity(4);
        for (allowed, family) in [
            (charter.grants.tools.inspect, smith_events::Family::Inspect),
            (charter.grants.tools.modify, smith_events::Family::Modify),
            (charter.grants.tools.shell, smith_events::Family::Shell),
            (charter.grants.agents, smith_events::Family::SubAgents),
        ] {
            if allowed { families.push(family).expect("four tool families"); }
        }
        let mut host = List::with_capacity(u32::try_from(charter.grants.host_tools.len()).expect("bounded host declarations"));
        for tool in &charter.grants.host_tools { host.push(tool.name.clone()).expect("admitted host names"); }
        let mut contract = List::with_capacity(4);
        if charter.outcome.change.is_some() { contract.push(smith_events::Form::Change).expect("four forms"); }
        if !charter.outcome.verdicts.is_empty() { contract.push(smith_events::Form::Verdict).expect("four forms"); }
        if charter.outcome.report.is_some() { contract.push(smith_events::Form::Report).expect("four forms"); }
        if charter.outcome.failure.is_some() { contract.push(smith_events::Form::Failure).expect("four forms"); }
        self.start = Some(Start {
            messages: u64::try_from(start.messages.len()).expect("bounded messages"),
            llm: charter.llm.clone(),
            budget: smith_events::Budget {
                turns: u64::from(charter.budget.turns), spend: charter.budget.spend,
                time_ms: charter.budget.time.as_nanos() / 1_000_000,
                reserve: smith_events::Reserve { turns: u64::from(charter.reserve.turns),
                    time_ms: charter.reserve.time.as_nanos() / 1_000_000,
                    spend: run::reserve_spend(charter.reserve, &charter.llm).unwrap_or(0) },
            },
            tools: smith_events::Tools { families, wait: charter.grants.wait, deliver: charter.grants.deliver.is_some(), host },
            contract,
        });
    }

    pub(crate) fn answer_at(&self) -> Option<Time> { self.answer_at }

    pub(crate) fn origin(&self) -> Time { self.origin.expect("session.started precedes the first service pass") }

    #[expect(clippy::too_many_lines, reason = "one exhaustive content-free root fact projection")]
    pub(crate) fn fact(&mut self, fact: &domain::Fact) {
        let origin = self.origin();
        let (fact, opening) = match fact {
            domain::Fact::Session { fact } => { self.sinks.session(origin, fact); return; }
            domain::Fact::Run { fact, opening } => (fact, opening),
        };
        let event = match fact.kind {
            run::facts::FactKind::Admitted { run, resumed } => {
                self.admitted = Some(run.raw());
                let start = self.start.take().expect("actual Start retained before its admission");
                Event::RunStarted(smith_events::RunStarted { run: run.raw(), resumed, messages: start.messages,
                    main: self.model(&start.llm), budget: start.budget, tools: start.tools, contract: start.contract })
            }
            run::facts::FactKind::Opened { run, conversation, kind } => {
                let opening = opening.as_ref().expect("actual Run::Open metadata accompanied the fact");
                let (kind, parent, call) = match kind {
                    run::facts::Kind::Main => (smith_events::ConversationKind::Main, None, None),
                    run::facts::Kind::Child { .. } => unreachable!("child call identity wiring follows this checkpoint"),
                };
                self.sinks.conversation(fact.at, origin, smith_events::ConversationOpened {
                    run: run.raw(), conversation: conversation.raw(), kind, parent, call,
                    thread: u64::from(opening.ordinal), model: self.model(&opening.llm), requested: None,
                    share: None, clamped: List::with_capacity(0),
                });
                return;
            }
            run::facts::FactKind::Ended { conversation, end, .. } => {
                self.sinks.conversation_ended(fact.at, conversation, end); return;
            }
            run::facts::FactKind::Budget { standing, .. } => {
                self.wind_down = match standing.stage { run::Stage::Ordinary => false, run::Stage::WindDown => true };
                self.sinks.record(&crate::budget_record(fact, origin).expect("matched budget fact")); return;
            }
            run::facts::FactKind::MessageReceived { run, name, bytes } => Event::MessageReceived(smith_events::MessageReceived {
                run: run.raw(), name: name.raw(), bytes, refused: None, label: None, text: None,
            }),
            run::facts::FactKind::MessageRefused { run, name, bytes, reason } => Event::MessageReceived(smith_events::MessageReceived {
                run: run.raw(), name: name.raw(), bytes, refused: Some(message_refusal(reason)), label: None, text: None,
            }),
            run::facts::FactKind::MessageRead { run, name, turn } => Event::MessageRead(smith_events::MessageRead {
                run: run.raw(), name: name.raw(), turn: u64::from(turn),
            }),
            run::facts::FactKind::MessageUnread { run, name } => {
                self.unread = self.unread.checked_add(1).expect("bounded unread messages");
                Event::MessageUnread(smith_events::MessageUnread { run: run.raw(), name: name.raw() })
            }
            run::facts::FactKind::CheckStarted { run, deadline } => {
                self.check_at = Some(fact.at);
                Event::CheckStarted(smith_events::CheckStarted { run: run.raw(), deadline_ms: Some(deadline.saturating_since(fact.at).as_nanos() / 1_000_000) })
            }
            run::facts::FactKind::CheckFinished { run, exit } => {
                let (exit, passed) = crate::check_exit(exit);
                Event::CheckCompleted(smith_events::CheckCompleted { run: run.raw(), passed,
                    exit,
                    duration_ms: fact.at.saturating_since(self.check_at.take().expect("one check start")).as_nanos() / 1_000_000 })
            }
            run::facts::FactKind::MessageFence { .. } | run::facts::FactKind::Prepared { .. }
            | run::facts::FactKind::Called { .. } | run::facts::FactKind::Returned { .. }
            | run::facts::FactKind::Delivered { .. } | run::facts::FactKind::Answered { .. } => return,
        };
        self.sinks.record(&Record { t_ms: fact.at.saturating_since(origin).as_nanos() / 1_000_000, event });
    }

    fn model(&self, model: &run::charter::Llm) -> smith_events::Model {
        let name = self.names.name_of(model.endpoint.0, model.dialect).expect("admitted endpoint has its configured name");
        let effort = match self.efforts.get(&model.endpoint.0) { Some(effort) => Some(effort.as_ref()), None => None };
        crate::opening_model(model, name, effort)
    }

    pub(crate) fn request(&mut self, now: Time, owner: Token, prompt: &domain::llm::Prompt) {
        self.sinks.request(now, self.origin(), owner, prompt);
    }

    pub(crate) fn answer(&mut self, now: Time, started: Time, answer: &run::Answer, read: Option<Token>) {
        assert!(self.answer_at.is_none(), "one actual answer per invocation");
        self.answer_at = Some(now);
        let record = crate::answer_record(crate::AnswerMeta {
            run: self.admitted.unwrap_or(1), wind_down: self.wind_down,
            read: match read { Some(read) => Some(read.raw()), None => None }, unread: self.unread,
            elapsed: now.saturating_since(started),
        }, answer, self.capture);
        self.sinks.record(&Record { t_ms: now.saturating_since(self.origin()).as_nanos() / 1_000_000,
            event: Event::RunCompleted(record) });
    }

    pub(crate) fn notice(&mut self, now: Time, kind: smith_events::NoticeKind, account: Option<u64>, wait_ms: Option<u64>, bytes: Option<u64>) {
        self.sinks.record(&Record { t_ms: now.saturating_since(self.origin()).as_nanos() / 1_000_000,
            event: Event::Notice(smith_events::Notice { level: smith_events::Level::Warning, kind,
                run: self.admitted, account, wait_ms, bytes }) });
    }
}

fn message_refusal(reason: run::MessageRefusal) -> smith_events::MessageRefusal {
    match reason {
        run::MessageRefusal::TooLarge => smith_events::MessageRefusal::TooLarge,
        run::MessageRefusal::Full => smith_events::MessageRefusal::InboxFull,
        run::MessageRefusal::NameInUse => smith_events::MessageRefusal::NameInUse,
        run::MessageRefusal::Ending => smith_events::MessageRefusal::Ending,
    }
}

pub(crate) fn worst_case(limits: &crate::Limits) -> Option<u64> {
    let events = limits.events?;
    crate::sinks::sinks_worst_case(&events.sinks, events.capture)?
        .checked_add(channel::Endpoints::worst_case(limits.channel.endpoints, limits.channel.charter.llm_endpoint)?)?
        .checked_add(Map::<u32, Box<[u8]>>::worst_case(limits.channel.endpoints)?)?
        .checked_add(u64::from(limits.channel.endpoints).checked_mul(u64::from(events.sinks.records.string))?)?
        .checked_add(List::<smith_events::Family>::worst_case(4)?)?
        .checked_add(List::<Box<[u8]>>::worst_case(limits.domain.run.host_tools)?)?
        .checked_add(List::<smith_events::Form>::worst_case(4)?)?
        .checked_add(u64::from(limits.domain.run.host_tools.checked_add(1)?).checked_mul(u64::from(events.sinks.records.string))?)
}
