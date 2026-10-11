//! What the runs tell whoever watches the agent (domain/run.md, section 11):
//! a fact for each thing that happened, content-free (tokens, counts and
//! classifications, never what an LLM, the charter or the host said), in a
//! bounded queue the parent drains at its own pace.
//!
//! Facts are a separate step output. The parent reserves [`crate::max_facts`]
//! slots before each entrance, holding work while its drain is behind; every
//! observation is preserved. Facts change no decision: the host's watchdog
//! hears of a check's deadline from `Request::Checking`.
//!
//! A start refused at the entrance tells nothing: no run was. Each step of an
//! admitted run says which run it is about; the facts its requests tell are
//! derived from them in one place ([`tell`]), and the facts of what happened
//! to the run without a request (a conversation ending, a call made, checks
//! or a delivery ending) are told where they happen.

use skein_lib::{Duration, Queue, Slab, Time, Token};

use crate::budget::{Granted, Share, Stage};
use crate::charter::Reserve;

use crate::boundary::{Answer, End, Exit, Failure, Refusal, Request, Returned};
use crate::run::{self, Conversation, Run};

/// An observation stamped with the injected time of its emitting step.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Fact {
    /// Emission time, preserved when parents drain this observation later.
    pub at: Time,
    /// The content-free observation.
    pub kind: FactKind,
}

/// Something that happened in the run `run` (the run's token for it, as
/// `Admitted` gives it).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FactKind {
    /// Each host message covered by an actual told main turn.
    MessageRead { run: Token, name: Token, turn: u32 },
    /// The final ordered read fence of one actual told main turn.
    MessageFence { run: Token, turn: u32, read: Option<Token> },
    /// An accepted host message ended unread at the run answer.
    MessageUnread { run: Token, name: Token },
    /// A host message entered the bounded inbox, counted as rendered bytes.
    MessageReceived { run: Token, name: Token, bytes: u64 },
    /// A host message ended refused before retention, counted as rendered bytes.
    MessageRefused { run: Token, name: Token, bytes: u64, reason: crate::MessageRefusal },
    /// The run was admitted.
    Admitted { run: Token, resumed: bool },
    /// It looked in its checkout, and found `guides` guides and `checks`
    /// repositories with checks.
    Prepared {
        run: Token,
        /// Number of repository guides retained during preparation.
        guides: u32,
        /// Number of prepared repository check executables.
        checks: u32,
    },
    /// It opened main or a child with its asked and effective share.
    Opened { run: Token, conversation: Token, kind: Kind },
    /// A completion was charged against the run's shared activation budget.
    Budget { run: Token, standing: Standing },
    /// The conversation ended, or was refused at its entrance.
    Ended {
        run: Token,
        conversation: Token,
        /// Terminal classification after everything started beneath this entity has settled.
        end: End,
    },
    /// A conversation made the call `call` of the run.
    Called {
        run: Token,
        conversation: Token,
        /// Run-issued call token, identifying the observed finish or sub-agent ask.
        call: Token,
        /// Typed run tool ask, validated against the asker's authority and charter.
        ask: Asked,
    },
    /// The call returned.
    Returned {
        run: Token,
        /// Run-issued call token, identifying the observed finish or sub-agent ask.
        call: Token,
        /// The one terminal value for the enclosing call; ownership passes to its receiver.
        result: Return,
    },
    /// Checks started, to be stopped at `deadline` at the latest.
    CheckStarted { run: Token, deadline: Time },
    /// Checks ended.
    CheckFinished {
        run: Token,
        /// Terminal process classification; only a zero exit code passes checks.
        exit: Exit,
    },
    /// An host delivery ended with its content-free classification.
    Delivered { run: Token, status: crate::DeliveryStatus },
    /// The run answered.
    Answered {
        run: Token,
        /// Single terminal value returned to the caller.
        answer: Answered,
    },
}

/// The role and share of the conversation the run opened.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    /// The run's main conversation.
    Main,

    /// A child serves its parent's call under the requested and effective share.
    Child { parent: Token, call: Token, asked: Share, granted: Granted },
}

/// The run's activation budget at a charge (protocol/events.md, section 3.8).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Standing {
    pub stage: Stage,
    pub turns: u32,
    pub turns_limit: u32,
    pub spend: u64,
    pub spend_limit: u64,
    pub time: Duration,
    pub time_limit: Duration,
    pub reserve: Reserve,
    pub reserve_spend: u64,
}

/// What one child used before its settled return, told by the run.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Used {
    pub turns: u32,
    pub time: Duration,
    pub spend: u64,
}

/// What a call asked for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Asked {
    /// Main requested settled waiting.
    Wait,
    /// An opaque declared host-tool call.
    Host,
    /// Main requested separately granted delivery.
    Deliver,
    /// The LLM declared a run result.
    Finish,
    /// The LLM asked its run to open a sub-agent.
    SubAgent,
}

/// How a call returned.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Return {
    /// Main finish returned to queued work without accepting its declared outcome.
    Crossed,

    /// Main wait intent accepted.
    Waited,
    /// The run parked while its wait result remained withheld.
    Withheld,
    /// Actual bounded host text reached the caller.
    HostAnswered,
    /// The host decided, but its answer exceeded the receiving cap.
    HostTooLarge,
    /// Relay settled but no permitted recovery can learn its outcome.
    HostUnknown,
    /// Host declaration/input refused before effects.
    HostRejected,
    /// Actual host delivery evidence.
    Delivered,
    /// No changed directory.
    Nothing,
    /// Named correctable host refusal.
    DeliveryRefused,
    /// The run accepted its declared result.
    Accepted,
    /// The declared result failed the charter's contract.
    Rejected,
    /// Required checks did not pass.
    ChecksFailed,
    /// The host found its delivery context stale.
    Stale,
    /// The host delivery failed and returned bounded feedback.
    DeliveryFailed,
    /// The caller cancelled and the terminal settled.
    Cancelled,
    /// The injected operation deadline won the race.
    TimedOut,
    /// No capacity is currently available; a later call may fit.
    Busy,
    /// The sub-agent answered or the run supplied its terminal, as classified by the enclosing fact.
    Answered {
        used: Used,
        /// Bytes the child's own answer cap omitted before its tool result was rendered.
        cut: u64,
    },
    /// The sub-agent ended without an accepted answer.
    Unanswered { used: Used },
    /// The entrance or operation was refused with the enclosing typed reason.
    Refused,
}

/// How a run answered.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Answered {
    /// Settled idle main parked.
    Parked,
    /// The entrance or operation was refused with the enclosing typed reason.
    Refused(Refusal),
    /// The run accepted its declared result.
    Accepted,
    /// One failed terminal with bounded diagnostics.
    Failed(Failure),
}

/// The facts not yet drained and the run the step
/// being taken is about.
#[derive(Debug)]
pub(crate) struct Facts {
    queue: Queue<Fact>,
    now: Time,
    about: Option<Token>,
}

impl Facts {
    pub(crate) fn with_capacity(capacity: u32) -> Facts {
        Facts { queue: Queue::with_capacity(capacity), now: Time::ZERO, about: None }
    }

    /// Keeps one fact in the room the parent reserved for this entry point.
    pub(crate) fn push(&mut self, kind: FactKind) {
        let fact = Fact { at: self.now, kind };
        self.queue.push(fact);
    }

    pub(crate) fn pop(&mut self) -> Option<Fact> {
        self.queue.pop()
    }

    pub(crate) fn room(&self) -> u32 {
        self.queue.room()
    }

    pub(crate) fn now(&self) -> Time {
        self.now
    }

    /// The step being taken is about the run `run`.
    pub(crate) fn about(&mut self, run: Token) {
        self.about = Some(run);
    }

    /// A step begins, about no run yet.
    pub(crate) fn begin(&mut self, now: Time) {
        self.now = now;
        self.about = None;
    }
}

/// What the step that made the requests in `out` from `mark` on tells, a fact
/// for each but the requests whose outcome is told when it comes (closes,
/// says, cancels, deliveries) or that tell nothing of their own (looks).
pub(crate) fn tell(
    facts: &mut Facts,
    runs: &Slab<Run>,
    conversations: &Slab<Conversation>,
    out: &Queue<Request>,
    mark: u32,
) {
    let Some(run) = facts.about else {
        return;
    };
    let made = usize::try_from(mark).expect("a u32 fits in a usize");
    for request in out.iter().skip(made) {
        let fact = match request {
            Request::Admitted { host_run: _, run } => {
                FactKind::Admitted { run: *run, resumed: run::resumed(runs, *run) }
            }
            Request::Open { conversation, opening: _ } => {
                let kind = run::opened(conversations, *conversation);
                FactKind::Opened { run, conversation: *conversation, kind }
            }
            Request::Return { call, result, .. } => match result_of(result) {
                Some(result) => FactKind::Returned { run, call: *call, result },
                None => continue,
            },
            Request::Check { owner: _, program: _, deadline, tail: _ } => {
                FactKind::CheckStarted { run, deadline: *deadline }
            }
            Request::Answer { to: _, answer, read: _ } => FactKind::Answered { run, answer: answered(answer) },
            Request::CompletionReady { .. }
            | Request::MessageRefused { .. }
            | Request::Waiting { .. }
            | Request::Turn { .. }
            | Request::HostCall { .. }
            | Request::WithdrawHost { .. }
            | Request::Say { .. }
            | Request::Close { .. }
            | Request::Read { .. }
            | Request::Probe { .. }
            | Request::Abort { .. }
            | Request::Checking { .. }
            | Request::ChecksEnded { .. }
            | Request::Deliver { .. } => continue,
        };
        facts.push(fact);
    }
}

/// The two child answers include their settled usage; other returns are told from requests.
pub(crate) fn child_returned(facts: &mut Facts, run: Token, call: Token, result: &Returned, used: Used) {
    let result = match result {
        Returned::Answered { cut, .. } => Return::Answered { used, cut: *cut },
        Returned::Unanswered { .. } => Return::Unanswered { used },
        Returned::Waited { .. }
        | Returned::Withheld
        | Returned::Crossed { .. }
        | Returned::HostAnswered(_)
        | Returned::HostTooLarge { .. }
        | Returned::HostReportedTooLarge
        | Returned::HostUnknown
        | Returned::HostRejected(_)
        | Returned::Delivered(_)
        | Returned::Nothing
        | Returned::DeliveryRefused(_)
        | Returned::Accepted
        | Returned::Rejected { .. }
        | Returned::ChecksFailed { .. }
        | Returned::Stale
        | Returned::DeliveryFailed { .. }
        | Returned::Cancelled
        | Returned::TimedOut
        | Returned::Busy
        | Returned::Refused { .. } => return,
    };
    facts.push(FactKind::Returned { run, call, result });
}

fn result_of(result: &Returned) -> Option<Return> {
    let result = match result {
        Returned::Waited { .. } => Return::Waited,
        Returned::Withheld => Return::Withheld,
        Returned::Crossed { .. } => Return::Crossed,
        Returned::HostAnswered(_) => Return::HostAnswered,
        Returned::HostTooLarge { .. } | Returned::HostReportedTooLarge => Return::HostTooLarge,
        Returned::HostUnknown => Return::HostUnknown,
        Returned::HostRejected(_) => Return::HostRejected,
        Returned::Delivered(_) => Return::Delivered,
        Returned::Nothing => Return::Nothing,
        Returned::DeliveryRefused(_) => Return::DeliveryRefused,
        Returned::Accepted => Return::Accepted,
        Returned::Rejected { .. } => Return::Rejected,
        Returned::ChecksFailed { .. } => Return::ChecksFailed,
        Returned::Stale => Return::Stale,
        Returned::DeliveryFailed { .. } => Return::DeliveryFailed,
        Returned::Cancelled => Return::Cancelled,
        Returned::TimedOut => Return::TimedOut,
        Returned::Busy => Return::Busy,
        Returned::Answered { .. } | Returned::Unanswered { .. } => return None,
        Returned::Refused { .. } => Return::Refused,
    };
    Some(result)
}

fn answered(answer: &Answer) -> Answered {
    match answer {
        Answer::Parked { .. } => Answered::Parked,
        Answer::Refused(refusal) => Answered::Refused(*refusal),
        Answer::Accepted { .. } => Answered::Accepted,
        Answer::Failed { failure, .. } => Answered::Failed(*failure),
    }
}

#[cfg(test)]
mod carrier_tests {
    #![allow(
        clippy::disallowed_macros,
        clippy::disallowed_methods,
        clippy::disallowed_types,
        reason = "programming-model.md section10.2: deterministic source-carrier test uses ordinary Rust"
    )]

    use super::*;
    use crate::boundary::Stop;
    use crate::budget::Budget;

    #[test]
    fn a_child_answer_fact_preserves_the_actual_tool_cap_omission() {
        let used = Used { turns: 2, time: Duration::from_millis(19), spend: 7 };
        let actual = Returned::Answered {
            text: b"kept".as_slice().into(),
            cut: 17,
            stop: Stop::EndTurn,
            used,
            left: Budget { turns: 3, spend: 11, time: Duration::from_secs(1) },
        };
        let mut facts = Facts::with_capacity(1);
        facts.begin(Time::from_nanos(23_000_000));
        child_returned(&mut facts, Token::new(31), Token::new(43), &actual, used);
        let fact = facts.pop().expect("one actual child terminal");
        assert_eq!(fact.at, Time::from_nanos(23_000_000));
        let FactKind::Returned { run, call, result: Return::Answered { used: observed, .. } } = fact.kind else {
            panic!("one child answer")
        };
        assert_eq!((run, call, observed), (Token::new(31), Token::new(43), used));
        // This assertion compiles unchanged on the current parent, where the
        // scalar is lost: its runtime failure is the carrier's parent proof.
        assert!(format!("{:?}", fact.kind).contains("cut: 17"), "actual cut was lost: {:?}", fact.kind);
        assert_eq!(facts.pop(), None);
        let Returned::Answered { text, .. } = actual else { panic!("source answer") };
        assert_eq!(text.as_ref(), b"kept", "the scalar fact leaves source content unchanged");
    }
}
