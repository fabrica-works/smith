//! Bounded joins between logical session facts and actual LLM hand-offs.
//! Each live conversation keeps its opening model, current intent and at most
//! one provider attempt. Intents can be revised before hand-off; only `request`
//! assigns a run-wide response number. No prompt or transport measurement is
//! retained. Contracts: protocol/events.md, sections 3.4–3.5 and 4.

#![expect(
    clippy::manual_let_else,
    clippy::manual_map,
    reason = "the strict subset keeps bounded entries explicit and uses no closures"
)]

use skein_lib::{List, Map, Time, Token};
use smith_domain::{llm, run, session};
use smith_events::{
    CompletionFailure, ConversationClosed, ConversationEnd, ConversationOpened, Event, Model, Outcome, Record,
    ResponseCompleted, ResponseStarted, Stop, Usage,
};

/// Public stream identity of one actual internal conversation owner.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scope {
    pub(crate) run: u64,
    pub(crate) conversation: u64,
}

/// A bounded response join table, owned by the event sink.
#[derive(Debug)]
pub(crate) struct Book {
    conversations: Map<Token, Conversation>,
    next: u64,
}

#[derive(Debug)]
struct Conversation {
    run: u64,
    number: u64,
    thread: u64,
    model: Model,
    opened: Time,
    intent: Option<Intent>,
    attempt: Attempt,
    spent: u64,
    admitted: bool,
    root_end: Option<(Time, run::End)>,
}

#[derive(Clone, Copy, Debug)]
struct Intent {
    turn: u64,
    retry: u32,
}

#[derive(Clone, Copy, Debug)]
struct Started {
    number: u64,
    at: Time,
    owner: Token,
    received: Option<Received>,
}

#[derive(Clone, Copy, Debug)]
struct Received {
    usage: llm::Usage,
    oversized: u64,
    cut: u64,
}

/// The actual hand-off's one terminal, completed only after accepted accounting.
#[derive(Debug)]
enum Attempt {
    /// No provider request was handed off, or its terminal was already recorded.
    Idle,
    /// A request was handed to the LLM component and owes one terminal.
    Started(Started),
    /// The completion answered; its accepted Used or end fact still follows.
    Answered { started: Started, at: Time, stop: Stop, blocks: u64, calls: u64, invalid: u64 },
    /// The failure's retry decision or session end still follows.
    Failed { started: Started, at: Time, failure: CompletionFailure },
}

impl Book {
    pub(crate) fn new(conversations: u32) -> Book {
        Book { conversations: Map::with_capacity(conversations), next: 1 }
    }

    pub(crate) fn opened(&mut self, at: Time, owner: Token, opening: &ConversationOpened) {
        let old = self
            .conversations
            .insert(
                owner,
                Conversation {
                    run: opening.run,
                    number: opening.conversation,
                    thread: opening.thread,
                    model: opening.model.clone(),
                    opened: at,
                    intent: None,
                    attempt: Attempt::Idle,
                    spent: 0,
                    admitted: false,
                    root_end: None,
                },
            )
            .expect("one join row per bounded live conversation");
        assert!(old.is_none(), "each conversation opens once");
    }

    pub(crate) fn scope(&self, owner: Token) -> Option<Scope> {
        let conversation = self.conversations.get(&owner)?;
        Some(Scope { run: conversation.run, conversation: conversation.number })
    }

    pub(crate) fn request(&mut self, now: Time, origin: Time, request_owner: Token, prompt: &llm::Prompt) -> Record {
        let mut owner = None;
        for (name, conversation) in &self.conversations {
            if conversation.thread == u64::from(prompt.affinity.thread) {
                assert!(owner.is_none(), "one live conversation per actual affinity ordinal");
                owner = Some(*name);
            }
        }
        let owner = owner.expect("the opening fact is drained before its actual request");
        let conversation = self.conversations.get_mut(&owner).expect("found above");
        assert_eq!(
            conversation.model.model.as_ref(),
            prompt.model.as_ref(),
            "the actual request uses its opening's model"
        );
        match &conversation.attempt {
            Attempt::Idle => {}
            Attempt::Started(_) | Attempt::Answered { .. } | Attempt::Failed { .. } => {
                unreachable!("a conversation hands off one provider request at a time")
            }
        }
        let intent = conversation.intent.take().expect("each actual request has a surviving logical intent");
        let number = self.next;
        self.next = self.next.checked_add(1).expect("a process cannot exhaust response names");
        conversation.attempt = Attempt::Started(Started { number, at: now, owner: request_owner, received: None });
        Record {
            t_ms: millis(now, origin),
            event: Event::ResponseStarted(ResponseStarted {
                run: conversation.run,
                conversation: conversation.number,
                response: number,
                turn: intent.turn,
                attempt: u64::from(intent.retry),
                // Compaction (08.6) is not merged: each actual opening has its first window.
                window: 1,
                model: conversation.model.clone(),
                messages: u64::try_from(prompt.messages.len()).expect("admitted actual prompt messages"),
                output_tokens: u64::from(prompt.max_tokens),
                tool_choice: crate::request_tool_choice(&prompt.choice),
                prompt: None,
            }),
        }
    }

    pub(crate) fn received(&mut self, owner: Token, completion: &llm::Completion) {
        let mut found = None;
        for (name, conversation) in &self.conversations {
            match &conversation.attempt {
                Attempt::Started(started) if started.owner == owner => {
                    found = Some(*name);
                    break;
                }
                Attempt::Idle | Attempt::Started(_) | Attempt::Answered { .. } | Attempt::Failed { .. } => {}
            }
        }
        let conversation = self
            .conversations
            .get_mut(&found.expect("typed answer retains its hand-off identity"))
            .expect("found above");
        match &mut conversation.attempt {
            Attempt::Started(started) => {
                let (oversized, cut) = rejected_calls(completion);
                assert!(started.received.is_none(), "one typed component answer per actual hand-off");
                started.received = Some(Received { usage: completion.usage, oversized, cut });
            }
            Attempt::Idle | Attempt::Answered { .. } | Attempt::Failed { .. } => {
                unreachable!("found active hand-off above")
            }
        }
    }

    pub(crate) fn root_ended(&mut self, at: Time, owner: Token, end: run::End) {
        // SessionEnded can have already closed a child in a previous drain.
        if let Some(conversation) = self.conversations.get_mut(&owner) {
            conversation.root_end = Some((at, end));
        }
    }

    /// Called only after the complete gathered fact batch has been drained.
    /// A root refusal before session admission still closes its opening once.
    pub(crate) fn facts_drained(&mut self, origin: Time) -> Option<Record> {
        let mut refused = None;
        for (owner, conversation) in &self.conversations {
            if !conversation.admitted && conversation.root_end.is_some() {
                refused = Some(*owner);
                break;
            }
        }
        let owner = refused?;
        let conversation = self.conversations.remove(&owner).expect("selected above");
        let (at, end) = conversation.root_end.expect("selected above");
        let end = match end {
            run::End::Busy | run::End::Invalid => ConversationEnd::Refused,
            run::End::TranscriptRefused { .. } => ConversationEnd::Transcript,
            run::End::PriceOverflow
            | run::End::UsageOverflow
            | run::End::Receiving(_)
            | run::End::Closed
            | run::End::Fault(_)
            | run::End::Budget(_) => {
                unreachable!("other conversation ends follow a session admission or its Ended fact")
            }
        };
        Some(Record {
            t_ms: millis(at, origin),
            event: Event::ConversationClosed(ConversationClosed {
                run: conversation.run,
                conversation: conversation.number,
                end,
                which: None,
                failure: None,
                turns: 0,
                usage: crate::reported_usage(llm::Usage::ZERO),
                spent: 0,
                duration_ms: millis(at, conversation.opened),
            }),
        })
    }

    /// At most two records: an unanswered completion's terminal, then its conversation end.
    #[expect(
        clippy::too_many_lines,
        reason = "one exhaustive session-fact join; each entrance keeps at most two records"
    )]
    pub(crate) fn fact(&mut self, origin: Time, fact: &session::Fact) -> List<Record> {
        let mut records = List::with_capacity(2);
        match fact.kind {
            session::FactKind::Opened { opener } => {
                self.conversations.get_mut(&opener).expect("session admission follows actual opening").admitted = true;
            }
            session::FactKind::CompletionStarted { opener, attempt, .. } => {
                let turn = fact.response.expect("intent carries its actual session turn").number;
                let conversation = self.conversations.get_mut(&opener).expect("intent follows opening");
                conversation.intent = Some(Intent { turn, retry: attempt });
            }
            session::FactKind::CompletionAnswered { opener, stop, blocks, calls, invalid } => {
                let conversation = self.conversations.get_mut(&opener).expect("answer follows opening");
                let started = take_started(&mut conversation.attempt);
                match started {
                    Some(started) => {
                        conversation.attempt = Attempt::Answered {
                            started,
                            at: fact.at,
                            stop: stop_kind(stop),
                            blocks: u64::from(blocks),
                            calls: u64::from(calls),
                            invalid: u64::from(invalid),
                        }
                    }
                    None => unreachable!("a provider answer follows an actual hand-off"),
                }
            }
            session::FactKind::Used { opener, usage } => {
                let conversation = self.conversations.get_mut(&opener).expect("charge follows opening");
                let spent =
                    fact.response.expect("charge has response identity").spent.expect("Used has the accepted charge");
                conversation.spent = conversation
                    .spent
                    .checked_add(spent)
                    .expect("session accepted this representable cumulative spend");
                let record = terminal(conversation, origin, crate::reported_usage(usage), spent, None)
                    .expect("an accepted charge follows its pending answer");
                records.push(record).expect("one response terminal");
            }
            session::FactKind::CompletionFailed { opener, failure, evidence } => {
                let conversation = self.conversations.get_mut(&opener).expect("failure follows opening");
                let started = take_started(&mut conversation.attempt);
                match started {
                    Some(started) => {
                        conversation.attempt =
                            Attempt::Failed { started, at: fact.at, failure: completion_failure(failure, evidence) }
                    }
                    None => unreachable!("a component failure follows an actual hand-off"),
                }
            }
            session::FactKind::CompletionRetried { opener, delay, .. } => {
                let conversation = self.conversations.get_mut(&opener).expect("retry follows opening");
                let record = terminal(conversation, origin, unknown_usage(), 0, Some(delay.as_nanos() / 1_000_000))
                    .expect("a retry follows one pending actual failure");
                records.push(record).expect("one response terminal");
            }
            session::FactKind::CompletionCancelled { opener } => {
                let conversation = self.conversations.get_mut(&opener).expect("cancellation follows opening");
                let started = take_started(&mut conversation.attempt);
                // The domain can abandon an unsent logical intent; that is not a provider attempt.
                if let Some(started) = started {
                    records.push(cancelled(conversation, origin, started, fact.at)).expect("one cancellation");
                }
            }
            session::FactKind::Ended { opener, end, turns, usage } => {
                let mut conversation = match self.conversations.remove(&opener) {
                    Some(conversation) => conversation,
                    None => return records,
                };
                if let Some(record) = terminal(&mut conversation, origin, unknown_usage(), 0, None) {
                    records.push(record).expect("one pending terminal before end");
                }
                let (end, which, failure) = conversation_end(end);
                records
                    .push(Record {
                        t_ms: millis(fact.at, origin),
                        event: Event::ConversationClosed(ConversationClosed {
                            run: conversation.run,
                            conversation: conversation.number,
                            end,
                            which: match which {
                                Some(which) => Some(which.into()),
                                None => None,
                            },
                            failure,
                            turns: u64::from(turns),
                            usage: crate::reported_usage(usage),
                            spent: conversation.spent,
                            duration_ms: millis(fact.at, conversation.opened),
                        }),
                    })
                    .expect("one conversation terminal");
            }
            session::FactKind::ReasoningDropped { .. }
            | session::FactKind::Tools { .. }
            | session::FactKind::DelegateStarted { .. }
            | session::FactKind::DelegateAnswered { .. }
            | session::FactKind::DelegateCancelled { .. }
            | session::FactKind::DelegateWithheld { .. }
            | session::FactKind::Yielded { .. } => {}
        }
        records
    }
}

fn rejected_calls(completion: &llm::Completion) -> (u64, u64) {
    let mut oversized = 0_u64;
    let mut cut = 0_u64;
    for block in &completion.content {
        match block {
            llm::Said::ToolCall { call, .. } => match call {
                llm::Decoded::Invalid { problem } => match problem {
                    llm::Problem::Oversize { .. } => oversized = oversized.checked_add(1).expect("bounded blocks"),
                    llm::Problem::CutOff { .. } => cut = cut.checked_add(1).expect("bounded blocks"),
                    llm::Problem::Nesting
                    | llm::Problem::UnknownTool
                    | llm::Problem::NotAnObject
                    | llm::Problem::Missing { .. }
                    | llm::Problem::WrongType { .. }
                    | llm::Problem::BadValue { .. }
                    | llm::Problem::TooLarge => {}
                },
                llm::Decoded::Owned { .. } | llm::Decoded::Served { .. } => {}
            },
            llm::Said::Opaque { .. } | llm::Said::Refusal { .. } | llm::Said::Text { .. } => {}
        }
    }
    (oversized, cut)
}

fn take_started(attempt: &mut Attempt) -> Option<Started> {
    match core::mem::replace(attempt, Attempt::Idle) {
        Attempt::Started(started) => Some(started),
        Attempt::Idle => None,
        Attempt::Answered { .. } | Attempt::Failed { .. } => unreachable!("one terminal per actual provider attempt"),
    }
}

fn terminal(
    conversation: &mut Conversation,
    origin: Time,
    usage: Usage,
    spent: u64,
    retry_ms: Option<u64>,
) -> Option<Record> {
    let (started, at, outcome, stop, blocks, calls, invalid, failure) =
        match core::mem::replace(&mut conversation.attempt, Attempt::Idle) {
            Attempt::Idle => return None,
            Attempt::Started(_) => unreachable!("a session end follows its actual provider terminal"),
            Attempt::Answered { started, at, stop, blocks, calls, invalid } => {
                (started, at, Outcome::Completed, Some(stop), blocks, calls, invalid, None)
            }
            Attempt::Failed { started, at, failure } => (started, at, Outcome::Failed, None, 0, 0, 0, Some(failure)),
        };
    let (usage, oversized, cut) = match started.received {
        Some(received) => (crate::reported_usage(received.usage), received.oversized, received.cut),
        None => (usage, 0, 0),
    };
    Some(Record {
        t_ms: millis(at, origin),
        event: Event::ResponseCompleted(ResponseCompleted {
            run: conversation.run,
            conversation: conversation.number,
            response: started.number,
            outcome,
            stop,
            blocks,
            calls,
            invalid,
            oversized,
            cut,
            usage,
            spent,
            request_bytes: None,
            first_byte_ms: None,
            largest_gap_ms: None,
            duration_ms: millis(at, started.at),
            failure,
            retry_ms,
            completion: None,
        }),
    })
}

fn cancelled(conversation: &Conversation, origin: Time, started: Started, at: Time) -> Record {
    Record {
        t_ms: millis(at, origin),
        event: Event::ResponseCompleted(ResponseCompleted {
            run: conversation.run,
            conversation: conversation.number,
            response: started.number,
            outcome: Outcome::Cancelled,
            stop: None,
            blocks: 0,
            calls: 0,
            invalid: 0,
            oversized: 0,
            cut: 0,
            usage: unknown_usage(),
            spent: 0,
            request_bytes: None,
            first_byte_ms: None,
            largest_gap_ms: None,
            duration_ms: millis(at, started.at),
            failure: None,
            retry_ms: None,
            completion: None,
        }),
    }
}

pub(crate) const fn unknown_usage() -> Usage {
    Usage {
        input_tokens: None,
        cache_read_tokens: None,
        cache_write_tokens: None,
        output_tokens: None,
        reasoning_tokens: None,
    }
}

fn millis(at: Time, origin: Time) -> u64 {
    at.saturating_since(origin).as_nanos() / 1_000_000
}

fn stop_kind(stop: llm::Stop) -> Stop {
    match stop {
        llm::Stop::EndTurn => Stop::End,
        llm::Stop::ToolUse => Stop::Tools,
        llm::Stop::MaxTokens => Stop::MaxTokens,
        llm::Stop::Refusal => Stop::Refusal,
    }
}

pub(crate) fn completion_failure(failure: llm::Failure, evidence: llm::Evidence) -> CompletionFailure {
    let (class, retry_after_ms) = match failure {
        llm::Failure::Limit { .. } | llm::Failure::ApplicationLimit => (smith_events::FailureClass::Limit, None),
        llm::Failure::Protocol => (smith_events::FailureClass::Protocol, None),
        llm::Failure::Cancelled => (smith_events::FailureClass::Cancelled, None),
        llm::Failure::Overloaded => (smith_events::FailureClass::Overloaded, None),
        llm::Failure::Unavailable => (smith_events::FailureClass::Unavailable, None),
        llm::Failure::TimedOut { .. } => (smith_events::FailureClass::TimedOut, None),
        llm::Failure::ContextTooLong => (smith_events::FailureClass::ContextTooLong, None),
        llm::Failure::Invalid => (smith_events::FailureClass::Invalid, None),
        llm::Failure::Unauthorized => (smith_events::FailureClass::Unauthorized, None),
        llm::Failure::RateLimited { retry_after } => {
            (smith_events::FailureClass::RateLimited, Some(retry_after.as_nanos() / 1_000_000))
        }
        llm::Failure::Exhausted { retry_after } => {
            (smith_events::FailureClass::Exhausted, Some(retry_after.as_nanos() / 1_000_000))
        }
    };
    let evidence = match evidence {
        llm::Evidence::Unsent => smith_events::Evidence::Unsent,
        llm::Evidence::Unknown => smith_events::Evidence::MaybeSent,
        llm::Evidence::Response => smith_events::Evidence::Response,
    };
    CompletionFailure { class, evidence, retry_after_ms }
}

fn conversation_end(end: session::End) -> (ConversationEnd, Option<&'static [u8]>, Option<CompletionFailure>) {
    match end {
        session::End::Closed => (ConversationEnd::Closed, None, None),
        session::End::Busy | session::End::Invalid => (ConversationEnd::Refused, None, None),
        session::End::Failed { failure, evidence } => {
            (ConversationEnd::Failed, None, Some(completion_failure(failure, evidence)))
        }
        session::End::Budget { spent } => {
            let which: &[u8] = match spent {
                session::Dimension::Turns => b"turns",
                session::Dimension::Input => b"input_tokens",
                session::Dimension::Output => b"output_tokens",
                session::Dimension::CacheRead => b"cache_read_tokens",
                session::Dimension::CacheWrite => b"cache_write_tokens",
                session::Dimension::Time => b"time",
                session::Dimension::Unit => b"spend",
            };
            (ConversationEnd::Budget, Some(which), None)
        }
        session::End::TranscriptFull | session::End::TranscriptRefused { .. } => {
            (ConversationEnd::Transcript, None, None)
        }
        session::End::PriceOverflow | session::End::UsageOverflow => (ConversationEnd::Overflow, None, None),
    }
}

pub(crate) fn worst_case(conversations: u32, string: u32) -> Option<u64> {
    List::<Record>::worst_case(2)?
        .checked_add(Map::<Token, Conversation>::worst_case(conversations)?)?
        .checked_add(u64::from(conversations).checked_mul(u64::from(string).checked_mul(3)?)?)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_macros,
        clippy::disallowed_types,
        clippy::disallowed_methods,
        reason = "programming-model.md section 10.2: deterministic unit harnesses use ordinary Rust"
    )]

    use super::*;
    use alloc::boxed::Box;
    use skein_lib::Duration;

    fn at(millis: u64) -> Time {
        Time::ZERO.checked_add(Duration::from_millis(millis)).unwrap()
    }

    fn opened(owner: u64, thread: u64) -> ConversationOpened {
        ConversationOpened {
            run: 7,
            conversation: owner,
            thread,
            kind: smith_events::ConversationKind::Main,
            parent: None,
            call: None,
            requested: None,
            share: None,
            clamped: List::with_capacity(0),
            model: Model {
                endpoint: b"actual-endpoint".to_vec().into(),
                model: b"actual-model".to_vec().into(),
                window_tokens: 123,
                output_tokens: 7,
                effort: Some(b"medium".to_vec().into()),
            },
        }
    }

    fn prompt(thread: u32) -> llm::Prompt {
        llm::Prompt {
            endpoint: llm::Endpoint(0),
            affinity: llm::Affinity { key: [0; 16], thread },
            asker: llm::Asker::Main,
            model: b"actual-model".to_vec().into(),
            system: Box::new([]),
            tools: smith_domain::tools::Grants { inspect: false, modify: false, shell: false },
            namespace: smith_domain::tools::Namespace::NoWorkspace,
            served: Box::new([]),
            choice: llm::ToolChoice::Only(Box::new([])),
            messages: Box::new([]),
            max_tokens: 7,
        }
    }

    fn fact(millis: u64, kind: session::FactKind, turn: u64, spent: Option<u64>) -> session::Fact {
        session::Fact {
            at: at(millis),
            kind,
            call: None,
            response: Some(session::ResponseInfo { number: turn, spent }),
        }
    }

    fn intent(book: &mut Book, owner: Token, choice: session::Choice, retry: u32) {
        let records = book.fact(
            Time::ZERO,
            &fact(
                2,
                session::FactKind::CompletionStarted {
                    opener: owner,
                    attempt: retry,
                    messages: 0,
                    max_tokens: 7,
                    choice,
                },
                1,
                None,
            ),
        );
        assert!(records.is_empty());
    }

    #[test]
    fn revised_unsent_intent_and_retry_join_once_to_actual_hand_offs() {
        let mut book = Book::new(2);
        let owner = Token::new(11);
        book.opened(at(1), Token::new(11), &opened(11, 0));
        intent(&mut book, owner, session::Choice::Auto, 0);
        intent(&mut book, owner, session::Choice::Only, 0);
        let first = book.request(at(3), Time::ZERO, Token::new(99), &prompt(0));
        let Event::ResponseStarted(first) = first.event else {
            panic!("actual hand-off");
        };
        assert_eq!((first.response, first.turn, first.attempt), (1, 1, 0));
        assert_eq!(first.tool_choice, smith_events::ToolChoice::Only);
        assert!(first.prompt.is_none());
        let failed = fact(
            5,
            session::FactKind::CompletionFailed {
                opener: owner,
                failure: llm::Failure::RateLimited { retry_after: Duration::from_millis(29) },
                evidence: llm::Evidence::Response,
            },
            1,
            None,
        );
        assert!(book.fact(Time::ZERO, &failed).is_empty());
        let records = book.fact(
            Time::ZERO,
            &fact(
                6,
                session::FactKind::CompletionRetried { opener: owner, attempt: 1, delay: Duration::from_millis(31) },
                1,
                None,
            ),
        );
        let Event::ResponseCompleted(failed) = &records.get(0).unwrap().event else {
            panic!("one failure terminal");
        };
        assert_eq!((failed.response, failed.duration_ms, failed.retry_ms), (1, 2, Some(31)));
        assert_eq!(failed.failure.as_ref().unwrap().retry_after_ms, Some(29));
        assert_eq!(failed.usage, unknown_usage());
        intent(&mut book, owner, session::Choice::Only, 1);
        let second = book.request(at(37), Time::ZERO, Token::new(100), &prompt(0));
        let Event::ResponseStarted(second) = second.event else {
            panic!("actual retry hand-off");
        };
        assert_eq!((second.response, second.turn, second.attempt), (2, 1, 1));
        book.opened(at(38), Token::new(12), &opened(12, 1));
        intent(&mut book, Token::new(12), session::Choice::Only, 0);
        let child = book.request(at(39), Time::ZERO, Token::new(101), &prompt(1));
        let Event::ResponseStarted(child) = child.event else {
            panic!("actual second conversation");
        };
        assert_eq!(child.response, 3);
    }

    #[test]
    fn internal_owner_tokens_do_not_replace_public_activation_and_conversation_numbers() {
        let mut book = Book::new(1);
        let owner = Token::new(91);
        let mut opening = opened(0, 0);
        opening.run = 77;
        book.opened(at(1), owner, &opening);
        let scope = book.scope(owner).expect("actual owner joined");
        assert_eq!((scope.run, scope.conversation), (77, 0));
        assert!(book.scope(Token::new(0)).is_none(), "public ordinal is not the internal owner");
        intent(&mut book, owner, session::Choice::Auto, 0);
        let record = book.request(at(3), Time::ZERO, Token::new(99), &prompt(0));
        let Event::ResponseStarted(record) = record.event else { panic!("actual hand-off") };
        assert_eq!((record.run, record.conversation), (77, 0));
        let completion = actual_completion();
        book.received(Token::new(99), &completion);
        drop(book.fact(
            Time::ZERO,
            &fact(
                5,
                session::FactKind::CompletionAnswered {
                    opener: owner,
                    stop: completion.stop,
                    blocks: 2,
                    calls: 2,
                    invalid: 2,
                },
                1,
                None,
            ),
        ));
        let records = book
            .fact(Time::ZERO, &fact(7, session::FactKind::Used { opener: owner, usage: completion.usage }, 1, Some(3)));
        let Event::ResponseCompleted(record) = &records.as_slice()[0].event else { panic!("actual terminal") };
        assert_eq!((record.run, record.conversation), (77, 0));
        let records = book.fact(
            Time::ZERO,
            &fact(
                11,
                session::FactKind::Ended {
                    opener: owner,
                    end: session::End::Closed,
                    turns: 1,
                    usage: completion.usage,
                },
                1,
                None,
            ),
        );
        let Event::ConversationClosed(record) = &records.as_slice()[0].event else { panic!("actual conversation end") };
        assert_eq!((record.run, record.conversation), (77, 0));
        assert!(book.scope(owner).is_none());
    }

    fn actual_completion() -> llm::Completion {
        llm::Completion {
            reasoning_dropped: Box::new([]),
            content: Box::new([
                llm::Said::ToolCall {
                    id: b"oversized-call".to_vec().into(),
                    name: b"tool".to_vec().into(),
                    input: b"{}".to_vec().into(),
                    call: llm::Decoded::Invalid { problem: llm::Problem::Oversize { bytes: 55, bound: 32 } },
                    replay: None,
                },
                llm::Said::ToolCall {
                    id: b"cut-call".to_vec().into(),
                    name: b"tool".to_vec().into(),
                    input: b"{}".to_vec().into(),
                    call: llm::Decoded::Invalid { problem: llm::Problem::CutOff { bytes: 13 } },
                    replay: None,
                },
            ]),
            stop: llm::Stop::ToolUse,
            usage: llm::Usage {
                input_tokens: Some(17),
                output_tokens: Some(19),
                cache_read_tokens: None,
                cache_write_tokens: Some(0),
                reasoning_tokens: Some(3),
            },
        }
    }

    #[test]
    fn raw_usage_and_rejection_counts_survive_accounting_refusal() {
        for accepted in [false, true] {
            let mut book = Book::new(1);
            let owner = Token::new(11);
            book.opened(at(1), Token::new(11), &opened(11, 0));
            book.fact(Time::ZERO, &fact(2, session::FactKind::Opened { opener: owner }, 1, None));
            intent(&mut book, owner, session::Choice::Only, 0);
            book.request(at(3), Time::ZERO, Token::new(99), &prompt(0));
            let completion = actual_completion();
            book.received(Token::new(99), &completion);
            book.fact(
                Time::ZERO,
                &fact(
                    5,
                    session::FactKind::CompletionAnswered {
                        opener: owner,
                        stop: completion.stop,
                        blocks: 2,
                        calls: 2,
                        invalid: 2,
                    },
                    1,
                    None,
                ),
            );
            let records = if accepted {
                book.fact(
                    Time::ZERO,
                    &fact(5, session::FactKind::Used { opener: owner, usage: completion.usage }, 1, Some(23)),
                )
            } else {
                book.fact(
                    Time::ZERO,
                    &fact(
                        5,
                        session::FactKind::Ended {
                            opener: owner,
                            end: session::End::PriceOverflow,
                            turns: 0,
                            usage: llm::Usage::ZERO,
                        },
                        1,
                        None,
                    ),
                )
            };
            let Event::ResponseCompleted(ended) = &records.get(0).unwrap().event else {
                panic!("one typed response terminal");
            };
            assert_eq!(ended.usage, crate::reported_usage(completion.usage));
            assert_eq!((ended.oversized, ended.cut, ended.invalid), (1, 1, 2));
            assert_eq!(ended.spent, if accepted { 23 } else { 0 });
            assert!(ended.completion.is_none());
            assert!(ended.request_bytes.is_none() && ended.first_byte_ms.is_none() && ended.largest_gap_ms.is_none());
            if !accepted {
                assert_eq!(records.len(), 2);
                let Event::ConversationClosed(closed) = &records.get(1).unwrap().event else {
                    panic!("then one conversation end");
                };
                assert_eq!((closed.turns, closed.spent), (0, 0));
                assert_eq!(closed.end, ConversationEnd::Overflow);
            }
        }
    }

    #[test]
    fn cancellations_and_pre_session_refusals_have_one_terminal_right() {
        for sent in [false, true] {
            let mut book = Book::new(1);
            let owner = Token::new(11);
            book.opened(at(1), Token::new(11), &opened(11, 0));
            intent(&mut book, owner, session::Choice::Only, 0);
            if sent {
                book.request(at(3), Time::ZERO, Token::new(99), &prompt(0));
            }
            let cancelled =
                book.fact(Time::ZERO, &fact(5, session::FactKind::CompletionCancelled { opener: owner }, 1, None));
            assert_eq!(cancelled.len(), u32::from(sent));
            if sent {
                let Event::ResponseCompleted(cancelled) = &cancelled.get(0).unwrap().event else {
                    panic!("one cancellation terminal");
                };
                assert_eq!(cancelled.outcome, Outcome::Cancelled);
                assert_eq!(cancelled.usage, unknown_usage());
            }
        }
        let mut book = Book::new(1);
        let owner = Token::new(11);
        book.opened(at(1), Token::new(11), &opened(11, 0));
        book.root_ended(at(5), owner, run::End::Invalid);
        let Event::ConversationClosed(refused) = book.facts_drained(Time::ZERO).unwrap().event else {
            panic!("refusal closes opening");
        };
        assert_eq!(refused.end, ConversationEnd::Refused);
        assert_eq!((refused.turns, refused.spent), (0, 0));
        assert!(book.facts_drained(Time::ZERO).is_none());
        assert!(
            book.fact(
                Time::ZERO,
                &fact(
                    5,
                    session::FactKind::Ended {
                        opener: owner,
                        end: session::End::Invalid,
                        turns: 0,
                        usage: llm::Usage::ZERO
                    },
                    1,
                    None
                )
            )
            .is_empty()
        );
        // The same capacity can now admit another refused opening.
        book.opened(at(6), Token::new(12), &opened(12, 1));
        book.root_ended(at(7), Token::new(12), run::End::Busy);
        assert!(book.facts_drained(Time::ZERO).is_some());
    }
}
