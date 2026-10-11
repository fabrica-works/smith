//! Bounded provider call joins for event starts and terminals.
//! The book keeps the session-issued owner, actual provider identity, public
//! conversation scope and start time until the terminal fact. It never owns
//! tool input, result content or a rendered result. Workspace caps arrive from
//! the tool's terminal; delegated content remains outside this projection.
//! Contracts: protocol/events.md, sections 3.1, 3.6 and 5; domain/run.md, section 11.

#![expect(clippy::manual_let_else, reason = "bounded production entries keep explicit Option terminals")]

use skein_lib::{List, Map, Time, Token};
use smith_domain::{session, tools};
use smith_events::{Event, Record, ToolCompleted, ToolStarted};

use crate::responses::Scope;
use crate::sinks::Sinks;

#[derive(Debug)]
struct Call {
    metadata: session::ToolCall,
    scope: Scope,
    at: Time,
}

/// Actual session batch capacities and aggregate provider metadata ownership.
#[derive(Clone, Copy)]
struct Bounds {
    slots: u32,
    bytes: u64,
}

impl Bounds {
    fn derive(limits: &session::Limits) -> Option<Bounds> {
        // The source session run slab holds two batches per session;
        // identities share the completion's aggregate ownership, not one
        // maximum provider string for every possible call.
        Some(Bounds {
            slots: limits.sessions.checked_mul(limits.parallel_tools)?.checked_mul(2)?,
            bytes: u64::from(limits.sessions).checked_mul(2)?.checked_mul(limits.completion_bytes)?,
        })
    }
}

/// One bounded set of actual provider calls which still owe a terminal.
#[derive(Debug)]
pub(crate) struct Book {
    calls: Map<Token, Call>,
    bytes: u64,
    bound: u64,
}

impl Book {
    pub(crate) fn new(limits: &session::Limits) -> Book {
        let bounds = Bounds::derive(limits).expect("admitted session batch bounds fit");
        Book { calls: Map::with_capacity(bounds.slots), bytes: 0, bound: bounds.bytes }
    }

    /// Project only a fact with actual provider metadata; kit lifecycle is not a call.
    pub(crate) fn fact(&mut self, origin: Time, fact: &session::Fact, sinks: &mut Sinks) {
        let opener = match fact.kind {
            session::FactKind::Tools {
                opener,
                fact: tools::FactKind::Started { .. } | tools::FactKind::Answered { .. },
            }
            | session::FactKind::DelegateStarted { opener, .. }
            | session::FactKind::DelegateAnswered { opener, .. }
            | session::FactKind::DelegateCancelled { opener }
            | session::FactKind::DelegateWithheld { opener } => opener,
            session::FactKind::Opened { .. }
            | session::FactKind::CompletionStarted { .. }
            | session::FactKind::CompletionAnswered { .. }
            | session::FactKind::CompletionFailed { .. }
            | session::FactKind::CompletionCancelled { .. }
            | session::FactKind::CompletionRetried { .. }
            | session::FactKind::ReasoningDropped { .. }
            | session::FactKind::Yielded { .. }
            | session::FactKind::Used { .. }
            | session::FactKind::Ended { .. }
            | session::FactKind::Tools {
                fact:
                    tools::FactKind::Opened { .. }
                    | tools::FactKind::Refused { .. }
                    | tools::FactKind::Closing { .. }
                    | tools::FactKind::Closed { .. },
                ..
            } => return,
        };
        let metadata = match &fact.call {
            Some(metadata) => metadata,
            None => return,
        };
        let scope = sinks.scope(opener).expect("actual conversation opening precedes its provider call");
        match fact.kind {
            session::FactKind::Tools { fact: tools::FactKind::Started { .. }, .. }
            | session::FactKind::DelegateStarted { .. } => {
                sinks.record(&self.started(origin, fact.at, scope, metadata));
            }
            session::FactKind::Tools {
                fact: tools::FactKind::Answered { verdict, bytes, omitted_bytes, clamped, .. },
                ..
            } => {
                // An entrance refusal settles the session's actual call in
                // the same step, without a lower work-start fact or IO.
                if !self.calls.contains_key(&metadata.owner) {
                    sinks.record(&self.started(origin, fact.at, scope, metadata));
                }
                let terminal = Terminal {
                    verdict: verdict_kind(verdict),
                    exit: command_exit(verdict),
                    bytes,
                    omitted_bytes,
                    clamped,
                };
                sinks.record(&self.completed(origin, fact.at, metadata, terminal));
            }
            session::FactKind::DelegateAnswered { bytes, error, .. } => {
                let verdict = if error { smith_events::Verdict::Error } else { smith_events::Verdict::Result };
                sinks.record(&self.completed(
                    origin,
                    fact.at,
                    metadata,
                    Terminal { verdict, exit: None, bytes, omitted_bytes: 0, clamped: false },
                ));
            }
            session::FactKind::DelegateCancelled { .. } | session::FactKind::DelegateWithheld { .. } => {
                sinks.record(&self.completed(
                    origin,
                    fact.at,
                    metadata,
                    Terminal {
                        verdict: smith_events::Verdict::Withdrawn,
                        exit: None,
                        bytes: 0,
                        omitted_bytes: 0,
                        clamped: false,
                    },
                ));
            }
            session::FactKind::Opened { .. }
            | session::FactKind::CompletionStarted { .. }
            | session::FactKind::CompletionAnswered { .. }
            | session::FactKind::CompletionFailed { .. }
            | session::FactKind::CompletionCancelled { .. }
            | session::FactKind::CompletionRetried { .. }
            | session::FactKind::ReasoningDropped { .. }
            | session::FactKind::Yielded { .. }
            | session::FactKind::Used { .. }
            | session::FactKind::Ended { .. }
            | session::FactKind::Tools {
                fact:
                    tools::FactKind::Opened { .. }
                    | tools::FactKind::Refused { .. }
                    | tools::FactKind::Closing { .. }
                    | tools::FactKind::Closed { .. },
                ..
            } => unreachable!("only selected actual call facts reach projection"),
        }
    }

    fn started(&mut self, origin: Time, at: Time, scope: Scope, metadata: &session::ToolCall) -> Record {
        let bytes = metadata_bytes(metadata);
        self.bytes = self.bytes.checked_add(bytes).expect("bounded call identity sum fits");
        assert!(self.bytes <= self.bound, "actual source batches bound copied provider identity");
        let old = self
            .calls
            .insert(metadata.owner, Call { metadata: metadata.clone(), scope, at })
            .expect("source session run slab bounds active event calls");
        assert!(old.is_none(), "one provider start per actual session call owner");
        let source = match metadata.source {
            session::ToolSource::Workspace => smith_events::Source::Workspace,
            session::ToolSource::Run => smith_events::Source::Run,
            session::ToolSource::Host => smith_events::Source::Host,
        };
        let effect = match metadata.effect {
            tools::Effect::Read => smith_events::Effect::Read,
            tools::Effect::Write => smith_events::Effect::Write,
        };
        Record {
            t_ms: millis(at, origin),
            event: Event::ToolStarted(ToolStarted {
                run: scope.run,
                conversation: scope.conversation,
                call: metadata.id.clone(),
                tool: metadata.name.clone(),
                source,
                effect,
                deadline_ms: Some(millis(metadata.deadline, at)),
                input_bytes: metadata.input_bytes,
                input: None,
            }),
        }
    }

    fn completed(&mut self, origin: Time, at: Time, metadata: &session::ToolCall, terminal: Terminal) -> Record {
        let call = self.calls.remove(&metadata.owner).expect("actual terminal follows its one start");
        assert_eq!(call.metadata, *metadata, "terminal keeps its exact parent call metadata");
        self.bytes = self.bytes.checked_sub(metadata_bytes(&call.metadata)).expect("identity was retained at start");
        Record {
            t_ms: millis(at, origin),
            event: Event::ToolCompleted(ToolCompleted {
                run: call.scope.run,
                conversation: call.scope.conversation,
                call: call.metadata.id,
                tool: call.metadata.name,
                verdict: terminal.verdict,
                exit: terminal.exit,
                bytes: terminal.bytes,
                omitted_bytes: terminal.omitted_bytes,
                clamped: terminal.clamped,
                duration_ms: millis(at, call.at),
                child: None,
                delivery: None,
                result: None,
            }),
        }
    }
}

struct Terminal {
    verdict: smith_events::Verdict,
    exit: Option<smith_events::CommandExit>,
    bytes: u64,
    omitted_bytes: u64,
    clamped: bool,
}

fn millis(at: Time, since: Time) -> u64 {
    at.saturating_since(since).as_nanos() / 1_000_000
}

fn metadata_bytes(metadata: &session::ToolCall) -> u64 {
    let id = u64::try_from(metadata.id.len()).expect("bounded provider id fits u64");
    id.checked_add(u64::try_from(metadata.name.len()).expect("bounded tool name fits u64"))
        .expect("bounded provider identity fits u64")
}

fn command_exit(verdict: tools::Verdict) -> Option<smith_events::CommandExit> {
    match verdict {
        tools::Verdict::Exited { exit } => match exit {
            tools::Exit::Code { code } => Some(smith_events::CommandExit::Code { code: i64::from(code) }),
            tools::Exit::Signal { signal } => Some(smith_events::CommandExit::Signal { signal: u64::from(signal) }),
            tools::Exit::TimedOut => None,
        },
        tools::Verdict::Read
        | tools::Verdict::Listed
        | tools::Verdict::Found
        | tools::Verdict::Written { .. }
        | tools::Verdict::Edited { .. }
        | tools::Verdict::NotGranted
        | tools::Verdict::Outside
        | tools::Verdict::ReadOnly
        | tools::Verdict::TooLong
        | tools::Verdict::NotFound
        | tools::Verdict::NotFile
        | tools::Verdict::Linked
        | tools::Verdict::Protected
        | tools::Verdict::NotDirectory
        | tools::Verdict::TooLarge
        | tools::Verdict::NotRead
        | tools::Verdict::Stale
        | tools::Verdict::NoMatch { .. }
        | tools::Verdict::Ambiguous { .. }
        | tools::Verdict::Unchanged { .. }
        | tools::Verdict::Failed { .. }
        | tools::Verdict::TimedOut
        | tools::Verdict::Cancelled
        | tools::Verdict::Busy
        | tools::Verdict::NulByte => None,
    }
}

fn verdict_kind(verdict: tools::Verdict) -> smith_events::Verdict {
    match verdict {
        tools::Verdict::Read => smith_events::Verdict::Read,
        tools::Verdict::Listed => smith_events::Verdict::Listed,
        tools::Verdict::Found => smith_events::Verdict::Found,
        tools::Verdict::Written { .. } => smith_events::Verdict::Written,
        tools::Verdict::Edited { .. } => smith_events::Verdict::Edited,
        tools::Verdict::Exited { .. } => smith_events::Verdict::Exited,
        tools::Verdict::NotGranted => smith_events::Verdict::NotGranted,
        tools::Verdict::Outside => smith_events::Verdict::Outside,
        tools::Verdict::ReadOnly => smith_events::Verdict::ReadOnly,
        tools::Verdict::TooLong => smith_events::Verdict::TooLong,
        tools::Verdict::NotFound => smith_events::Verdict::NotFound,
        tools::Verdict::NotFile => smith_events::Verdict::NotFile,
        tools::Verdict::Linked => smith_events::Verdict::Linked,
        tools::Verdict::Protected => smith_events::Verdict::Protected,
        tools::Verdict::NotDirectory => smith_events::Verdict::NotDirectory,
        tools::Verdict::TooLarge => smith_events::Verdict::TooLarge,
        tools::Verdict::NotRead => smith_events::Verdict::NotRead,
        tools::Verdict::Stale => smith_events::Verdict::Stale,
        tools::Verdict::NoMatch { .. } => smith_events::Verdict::NoMatch,
        tools::Verdict::Ambiguous { .. } => smith_events::Verdict::Ambiguous,
        tools::Verdict::Unchanged { .. } => smith_events::Verdict::Unchanged,
        tools::Verdict::Failed { .. } => smith_events::Verdict::Failed,
        tools::Verdict::TimedOut => smith_events::Verdict::TimedOut,
        tools::Verdict::Cancelled => smith_events::Verdict::Cancelled,
        tools::Verdict::Busy => smith_events::Verdict::Busy,
        tools::Verdict::NulByte => smith_events::Verdict::NulByte,
    }
}

pub(crate) fn worst_case(limits: &session::Limits, string: u32) -> Option<u64> {
    let bounds = Bounds::derive(limits)?;
    Map::<Token, Call>::worst_case(bounds.slots)?
        .checked_add(bounds.bytes)?
        .checked_add(List::<Record>::worst_case(2)?)?
        .checked_add(u64::from(string).checked_mul(4)?)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_macros,
        clippy::disallowed_methods,
        clippy::disallowed_types,
        reason = "programming-model.md section10.2: deterministic test harnesses use ordinary Rust"
    )]

    use super::*;
    use crate::sinks::tests::{drain, drain_ready, grant};
    use crate::sinks::{Capture, Delivery, Destination, Ended};
    use skein_lib::Duration;

    fn fixture(capture: Capture) -> (Book, Sinks, Token, Token, session::ToolCall) {
        let mut profile = crate::profile::standard();
        profile.declared.memory = Some(1_u64 << 40_u32);
        let mut source = crate::profile::derive(&profile, &crate::tests::declarations(0)).unwrap().domain.session;
        source.sessions = 1;
        source.parallel_tools = 2;
        source.completion_bytes = 1024;
        let book = Book::new(&source);
        let limits = crate::sinks::tests::limits(1);
        let mut sinks = Sinks::new(&limits, capture, 7);
        let destination = Token::new(999);
        sinks.add(Destination::Trace, destination, Delivery::Complete).unwrap();
        grant(&mut sinks, destination);
        let opener = Token::new(91);
        sinks.conversation(
            Time::ZERO,
            Time::ZERO,
            opener,
            smith_events::ConversationOpened {
                run: 77,
                conversation: 0,
                kind: smith_events::ConversationKind::Main,
                parent: None,
                call: None,
                thread: 0,
                model: smith_events::Model {
                    endpoint: b"actual-endpoint".as_slice().into(),
                    model: b"actual-model".as_slice().into(),
                    window_tokens: 4096,
                    output_tokens: 32,
                    effort: None,
                },
                requested: None,
                share: None,
                clamped: List::with_capacity(0),
            },
        );
        let opening = drain_ready(&mut sinks, destination);
        assert_eq!(opening.len(), 1);
        let metadata = session::ToolCall {
            owner: Token::new(43),
            id: b"provider-\"id".as_slice().into(),
            name: b"actual-tool".as_slice().into(),
            source: session::ToolSource::Workspace,
            effect: tools::Effect::Read,
            deadline: Time::from_nanos(103_000_000),
            input_bytes: 1234,
        };
        (book, sinks, destination, opener, metadata)
    }

    #[test]
    fn actual_workspace_terminals_keep_scalar_measurements_and_provider_identity_under_every_capture() {
        for capture in [Capture::None, Capture::Calls, Capture::Everything] {
            for (verdict, expected, exit, omitted, clamped) in [
                (tools::Verdict::Read, smith_events::Verdict::Read, None, 19, true),
                (tools::Verdict::Read, smith_events::Verdict::Read, None, 0, false),
                (
                    tools::Verdict::Exited { exit: tools::Exit::Code { code: 0 } },
                    smith_events::Verdict::Exited,
                    Some(smith_events::CommandExit::Code { code: 0 }),
                    21,
                    true,
                ),
                (
                    tools::Verdict::Exited { exit: tools::Exit::Signal { signal: 15 } },
                    smith_events::Verdict::Exited,
                    Some(smith_events::CommandExit::Signal { signal: 15 }),
                    0,
                    false,
                ),
                (tools::Verdict::Exited { exit: tools::Exit::TimedOut }, smith_events::Verdict::Exited, None, 9, true),
                (tools::Verdict::TimedOut, smith_events::Verdict::TimedOut, None, 0, true),
                (tools::Verdict::Cancelled, smith_events::Verdict::Cancelled, None, 0, true),
            ] {
                let (mut book, mut sinks, destination, opener, metadata) = fixture(capture);
                book.fact(
                    Time::ZERO,
                    &session::Fact {
                        at: Time::from_nanos(3_000_000),
                        kind: session::FactKind::Tools {
                            opener,
                            fact: tools::FactKind::Started { session: Token::new(88), tool: tools::Tool::Read },
                        },
                        call: Some(metadata.clone()),
                        response: None,
                    },
                    &mut sinks,
                );
                book.fact(
                    Time::ZERO,
                    &session::Fact {
                        at: Time::from_nanos(14_000_000),
                        kind: session::FactKind::Tools {
                            opener,
                            fact: tools::FactKind::Answered {
                                session: Token::new(88),
                                tool: tools::Tool::Read,
                                verdict,
                                bytes: 13,
                                omitted_bytes: omitted,
                                clamped,
                            },
                        },
                        call: Some(metadata.clone()),
                        response: None,
                    },
                    &mut sinks,
                );
                assert!(book.calls.is_empty());
                assert_eq!(book.bytes, 0);
                sinks.ended(
                    Time::from_nanos(20_000_000),
                    Time::ZERO,
                    &Ended {
                        exit: 0,
                        answered: true,
                        teardown: Some(Duration::from_millis(6)),
                        usage: skein_io::Measured::Unread,
                        channel: None,
                    },
                );
                let records = drain(&mut sinks, destination);
                let [started, completed, _] = records.as_slice() else {
                    panic!("one actual start and one completed record")
                };
                let Event::ToolStarted(started) = &started.event else { panic!("start") };
                let Event::ToolCompleted(completed) = &completed.event else { panic!("completed") };
                assert_eq!((started.run, started.conversation, completed.run, completed.conversation), (77, 0, 77, 0));
                assert_eq!(started.call, metadata.id);
                assert_eq!(completed.call, metadata.id);
                assert_eq!(started.tool, metadata.name);
                assert_eq!(completed.tool, metadata.name);
                assert_eq!(started.input_bytes, 1234);
                assert_eq!(started.deadline_ms, Some(100));
                assert_eq!(started.input, None);
                assert_eq!(completed.result, None);
                assert_eq!(completed.verdict, expected);
                assert_eq!(completed.exit, exit);
                assert_eq!(
                    (completed.bytes, completed.omitted_bytes, completed.clamped, completed.duration_ms),
                    (13, omitted, clamped, 11)
                );
            }
        }
    }
}
