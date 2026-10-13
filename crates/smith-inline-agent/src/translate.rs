//! Total parent/root translations (domain/host.md, section 9.4).
//! No state, codec, policy interpretation or credential values live here.
//! Values move into the root; its own admission checks decide their semantics.

#![expect(clippy::manual_let_else, reason = "exhaustive bounded request draining")]
#![expect(clippy::manual_map, reason = "no application closures or map in the strict subset")]

use alloc::boxed::Box;
use skein_lib::{Env, Id, List, Queue, ReplyTo, Token};
use smith_domain::{self as smith, run};
use smith_host_domain::{self as host, parent};

use crate::domain::{Domain, Relay, Slot};
use crate::{Limits, Lower, Output, stop};

pub(crate) fn start(client: Token, source: host::Start, limits: &Limits) -> Result<smith::Event, host::Invalid> {
    let count = checked_count(source.messages.len())?;
    if count > limits.smith.run.messages {
        return Err(host::Invalid::Messages);
    }
    let mut messages = List::with_capacity(count);
    for message in source.messages {
        messages
            .push(run::Message { name: message.name, label: message.label, text: message.text })
            .expect("checked count");
    }
    let count = checked_count(source.grants.len())?;
    if count > limits.smith.accounts {
        return Err(host::Invalid::Grants);
    }
    let mut grants = List::with_capacity(count);
    for item in source.grants {
        grants.push(grant(item)).expect("checked count");
    }
    let count = checked_count(source.answered.len())?;
    if count > limits.smith.run.answered_calls {
        return Err(host::Invalid::Answered);
    }
    let mut answered = List::with_capacity(count);
    for call in source.answered {
        let answer = match call.reply {
            host::SavedReply::Host { error, body } => {
                smith::Answered::Host(run::HostAnswer::new(body, error).ok_or(host::Invalid::Answered)?)
            }
            host::SavedReply::Delivery(value) => {
                smith::Answered::Delivery(Box::new(delivery(*value).ok_or(host::Invalid::Answered)?))
            }
            host::SavedReply::TooLarge => smith::Answered::TooLarge,
        };
        answered
            .push(smith::AnsweredCall {
                name: run::CallName {
                    activation: call.name.activation,
                    completion: call.name.completion,
                    position: call.name.position,
                },
                tool: call.tool,
                answer,
            })
            .expect("checked count");
    }
    let transcript = match source.transcript {
        Some(value) => Some(value.into_value()),
        None => None,
    };
    Ok(smith::Event::Start {
        reply_to: ReplyTo::new(client),
        host_run: source.logical_run,
        activation: source.activation,
        identity: source.identity,
        window: limits.window,
        messages: messages.into_boxed(),
        charter: source.charter.into_value(),
        workspace: crate::workspace::start(source.workspace, source.directories, limits)?,
        transcript,
        answered: answered.into_boxed(),
        grants: grants.into_boxed(),
    })
}

fn checked_count(length: usize) -> Result<u32, host::Invalid> {
    match u32::try_from(length) {
        Ok(count) => Ok(count),
        Err(_) => Err(host::Invalid::Limits),
    }
}

pub(crate) const fn grant(value: host::Grant) -> smith::Grant {
    smith::Grant { name: smith::GrantName { account: value.account, generation: value.generation }, valid: value.valid }
}

pub(crate) fn host_reply(value: host::Reply) -> Option<run::HostReply> {
    match value {
        host::Reply::Host { error, body } => match run::HostAnswer::new(body, error) {
            Some(answer) => Some(run::HostReply::Answered(answer)),
            None => Some(run::HostReply::TooLarge),
        },
        host::Reply::Busy => Some(run::HostReply::Busy),
        host::Reply::Unavailable => Some(run::HostReply::Unanswered(run::Unanswered::Lost)),
        host::Reply::Withdrawn => Some(run::HostReply::Withdrawn),
        host::Reply::TooLarge => Some(run::HostReply::TooLarge),
        host::Reply::Delivery(_) => None,
    }
}

pub(crate) fn answer(value: run::Answer, read: Option<Token>, now: skein_lib::Time) -> host::Answer {
    match value {
        run::Answer::Parked { spent, turns } => {
            host::Answer { read, turns, spent: spent.units, result: host::RunResult::Parked }
        }
        run::Answer::Refused(value) => {
            host::Answer { read, turns: 0, spent: 0, result: host::RunResult::Refused { refusal: refusal(value) } }
        }
        run::Answer::Accepted { outcome, spent, turns } => host::Answer {
            read,
            turns,
            spent: spent.units,
            result: host::RunResult::Accepted {
                outcome: host::Declared::new(outcome, u64::MAX).expect("checked root outcome"),
            },
        },
        run::Answer::Failed { failure: value, spent, turns } => host::Answer {
            read,
            turns,
            spent: spent.units,
            result: host::RunResult::Failed { failure: failure(value, now) },
        },
    }
}

const fn refusal(value: run::Refusal) -> host::Refusal {
    match value {
        run::Refusal::Busy => host::Refusal::Busy,
        run::Refusal::Invalid(value) => host::Refusal::Invalid(invalid(value)),
    }
}

const fn failure(value: run::Failure, now: skein_lib::Time) -> host::RunFailure {
    match value {
        run::Failure::Transcript(value) => host::RunFailure::Transcript(transcript_refusal(value)),
        run::Failure::Model(value) => host::RunFailure::Model(model_fault(value, now)),
        run::Failure::Budget { exhausted: value, used, bound } => {
            host::RunFailure::Budget { exhausted: exhausted(value), used, bound }
        }
        run::Failure::Policy(run::Policy::Unfinished { nudges, rejected }) => {
            host::RunFailure::Policy(host::Policy::Unfinished { nudges, rejected })
        }
        run::Failure::Cancelled => host::RunFailure::Cancelled,
        run::Failure::Stale => host::RunFailure::Stale,
    }
}

const fn model_fault(value: run::Fault, now: skein_lib::Time) -> host::ModelFault {
    match value {
        run::Fault::Completion { failure, evidence, model, attempts } => host::ModelFault::Completion {
            failure: completion_failure(failure),
            evidence: completion_evidence(evidence),
            model,
            attempts,
        },
        run::Fault::Exhausted { account, retry_at } => host::ModelFault::Exhausted {
            account,
            retry_after_ms: match retry_at {
                Some(at) => Some(at.saturating_since(now).as_nanos() / 1_000_000),
                None => None,
            },
        },
        run::Fault::Provider => host::ModelFault::Provider,
        run::Fault::Refused => host::ModelFault::Refused,
        run::Fault::Truncated => host::ModelFault::Truncated,
        run::Fault::Malformed => host::ModelFault::Malformed,
    }
}

const fn invalid(value: run::Invalid) -> host::RunInvalid {
    match value {
        run::Invalid::Messages => host::RunInvalid::Messages,
        run::Invalid::CharterVersion => host::RunInvalid::CharterVersion,
        run::Invalid::MalformedCharter => host::RunInvalid::MalformedCharter,
        run::Invalid::Endpoint => host::RunInvalid::Endpoint,
        run::Invalid::Activation => host::RunInvalid::Activation,
        run::Invalid::Window => host::RunInvalid::Window,
        run::Invalid::Conventions => host::RunInvalid::Conventions,
        run::Invalid::TooLarge => host::RunInvalid::TooLarge,
        run::Invalid::Workspace => host::RunInvalid::Workspace,
        run::Invalid::Grants => host::RunInvalid::Grants,
        run::Invalid::Outcome => host::RunInvalid::Outcome,
        run::Invalid::Budget(refusal) => host::RunInvalid::Budget(budget_refusal(refusal)),
        run::Invalid::Price(rate) => host::RunInvalid::Price(price_rate(rate)),
        run::Invalid::Llm => host::RunInvalid::Llm,
        run::Invalid::Effort => host::RunInvalid::Effort,
        run::Invalid::Conversation => host::RunInvalid::Conversation,
        run::Invalid::HostPart { bound } => host::RunInvalid::HostPart { bound },
        run::Invalid::Room { bound } => host::RunInvalid::Room { bound },
    }
}

const fn transcript_refusal(value: run::TranscriptRefusal) -> host::TranscriptRefusal {
    match value {
        run::TranscriptRefusal::Version => host::TranscriptRefusal::Version,
        run::TranscriptRefusal::Endpoint => host::TranscriptRefusal::Endpoint,
        run::TranscriptRefusal::Dialect => host::TranscriptRefusal::Dialect,
        run::TranscriptRefusal::Malformed => host::TranscriptRefusal::Malformed,
        run::TranscriptRefusal::Unresolved => host::TranscriptRefusal::Unresolved,
        run::TranscriptRefusal::TooLarge => host::TranscriptRefusal::TooLarge,
    }
}

const fn completion_evidence(value: run::CompletionEvidence) -> host::CompletionEvidence {
    match value {
        run::CompletionEvidence::Unsent => host::CompletionEvidence::Unsent,
        run::CompletionEvidence::Unknown => host::CompletionEvidence::Unknown,
        run::CompletionEvidence::Response { status } => host::CompletionEvidence::Response { status },
    }
}

const fn receiving_limit(value: run::ReceivingLimit) -> host::ReceivingLimit {
    match value {
        run::ReceivingLimit::Input => host::ReceivingLimit::Input,
        run::ReceivingLimit::Output => host::ReceivingLimit::Output,
        run::ReceivingLimit::CacheRead => host::ReceivingLimit::CacheRead,
        run::ReceivingLimit::CacheWrite => host::ReceivingLimit::CacheWrite,
    }
}

const fn overflow(value: run::Overflow) -> host::Overflow {
    match value {
        run::Overflow::Spend => host::Overflow::Spend,
        run::Overflow::Usage => host::Overflow::Usage,
    }
}

pub(crate) const fn effect(value: run::HostEffect) -> host::Effect {
    match value {
        run::HostEffect::Read => host::Effect::Read,
        run::HostEffect::Write => host::Effect::Write,
    }
}

pub(crate) const fn message_refusal(value: run::MessageRefusal) -> host::MessageRefusal {
    match value {
        run::MessageRefusal::TooLarge => host::MessageRefusal::TooLarge,
        run::MessageRefusal::Full => host::MessageRefusal::Full,
        run::MessageRefusal::Ending => host::MessageRefusal::Ending,
        run::MessageRefusal::NameInUse => host::MessageRefusal::NameInUse,
    }
}

const fn completion_failure(value: run::CompletionFailure) -> host::CompletionFailure {
    match value {
        run::CompletionFailure::Limit { which, bound } => {
            host::CompletionFailure::Limit { which: completion_limit(which), bound }
        }
        run::CompletionFailure::ApplicationLimit => host::CompletionFailure::ApplicationLimit,
        run::CompletionFailure::Protocol => host::CompletionFailure::Protocol,
        run::CompletionFailure::Cancelled => host::CompletionFailure::Cancelled,
        run::CompletionFailure::Overloaded => host::CompletionFailure::Overloaded,
        run::CompletionFailure::Unavailable => host::CompletionFailure::Unavailable,
        run::CompletionFailure::TimedOut { phase } => {
            host::CompletionFailure::TimedOut { phase: completion_phase(phase) }
        }
        run::CompletionFailure::ContextTooLong => host::CompletionFailure::ContextTooLong,
        run::CompletionFailure::Invalid => host::CompletionFailure::Invalid,
        run::CompletionFailure::Unauthorized => host::CompletionFailure::Unauthorized,
        run::CompletionFailure::RateLimited { retry_after } => host::CompletionFailure::RateLimited { retry_after },
        run::CompletionFailure::Exhausted { retry_after } => host::CompletionFailure::Exhausted { retry_after },
    }
}

const fn exhausted(value: run::Exhausted) -> host::Exhausted {
    match value {
        run::Exhausted::Turns => host::Exhausted::Turns,
        run::Exhausted::Spend => host::Exhausted::Spend,
        run::Exhausted::Time => host::Exhausted::Time,
        run::Exhausted::Tokens(value) => host::Exhausted::Tokens(receiving_limit(value)),
        run::Exhausted::Overflow(value) => host::Exhausted::Overflow(overflow(value)),
    }
}

pub(crate) fn delivery(value: host::Delivery) -> Option<run::Delivery> {
    match value {
        host::Delivery::Delivered(value) => {
            let mut receipts = List::with_capacity(u32::try_from(value.receipts().len()).ok()?);
            for receipt in value.receipts() {
                receipts
                    .push(run::Receipt::new(receipt.directory(), Box::from(receipt.text()))?)
                    .expect("sealed count");
            }
            Some(run::Delivery::Delivered(run::Delivered::new(receipts.into_boxed())?))
        }
        host::Delivery::Nothing => Some(run::Delivery::Nothing),
        host::Delivery::Refused(value) => {
            let marker = match value.marker() {
                Some(marker) => Some(run::Marker::new(marker.directory(), Box::from(marker.path()))?),
                None => None,
            };
            Some(run::Delivery::Refused(run::DeliveryRefusal::new(marker, Box::from(value.explanation()))?))
        }
        host::Delivery::Failed(value) => Some(run::Delivery::Failed(run::DeliveryFailure {
            directory: value.directory,
            reason: delivery_reason(value.reason),
            diagnostic: run::Diagnostic::new(value.diagnostic.output(), value.diagnostic.cut()),
        })),
        host::Delivery::Stale => Some(run::Delivery::Stale),
    }
}

const fn delivery_reason(value: host::DeliveryReason) -> run::DeliveryReason {
    match value {
        host::DeliveryReason::Unreachable => run::DeliveryReason::Unreachable,
        host::DeliveryReason::RefusedByTarget => run::DeliveryReason::RefusedByTarget,
        host::DeliveryReason::TimedOut => run::DeliveryReason::TimedOut,
        host::DeliveryReason::Broken => run::DeliveryReason::Broken,
        host::DeliveryReason::TooLarge => run::DeliveryReason::TooLarge,
        host::DeliveryReason::Missing => run::DeliveryReason::Missing,
        host::DeliveryReason::Busy => run::DeliveryReason::Busy,
        host::DeliveryReason::Unavailable => run::DeliveryReason::Unavailable,
        host::DeliveryReason::Cancelled => run::DeliveryReason::Cancelled,
        host::DeliveryReason::Unknown => run::DeliveryReason::Unknown,
    }
}

#[expect(clippy::too_many_lines, reason = "every root request has one typed parent or lower route")]
pub(crate) fn route(agent: &mut Domain, env: &Env<Limits>, id: Id<Slot>, out: &mut Queue<Output>) {
    for _ in 0..smith::max_out(&env.limits.smith) {
        let request = match agent.lower.pop() {
            Some(request) => request,
            None => break,
        };
        let slot = agent.slots.get_mut(id).expect("root output belongs to its slot");
        let client = slot.client;
        match request {
            smith::Request::Admitted { host_run, run } => {
                assert_eq!(slot.logical_run, host_run, "root output preserves its scoped identity and single right");
                slot.run = Some(run);
                out.push(Output::Parent(parent::Request::Admitted { client }));
            }
            smith::Request::MessageRefused { host_run, name, reason } => {
                assert_eq!(slot.logical_run, host_run, "root output preserves its scoped identity and single right");
                if slot.inbox.refused(name) {
                    out.push(Output::Parent(parent::Request::MessageRefused {
                        client,
                        name,
                        reason: message_refusal(reason),
                    }));
                }
            }
            smith::Request::Turn { host_run, number, spent, read, turn, opens_window, position: _ } => {
                assert_eq!(slot.logical_run, host_run, "root output preserves its scoped identity and single right");
                slot.inbox.fence(read);
                let body = host::TurnValue::new(turn, u64::MAX).expect("root concrete turns have checked ownership");
                assert!(
                    slot.turns
                        .insert(number, body.owned_bytes())
                        .expect("root respects the acknowledgement window")
                        .is_none(),
                    "root output preserves its scoped identity and single right"
                );
                out.push(Output::Parent(parent::Request::Turn {
                    client,
                    turn: host::Turn { number, spent: spent.units, read, opens_window, body },
                }));
            }
            smith::Request::Waiting { host_run, read } => {
                assert_eq!(slot.logical_run, host_run, "root output preserves its scoped identity and single right");
                slot.inbox.fence(read);
                out.push(Output::Parent(parent::Request::Waiting { client, read }));
            }
            smith::Request::Answer { to, answer, read, detail } => {
                assert_eq!(to.into_token(), client, "root output preserves its scoped identity and single right");
                slot.inbox.fence(read);
                out.push(Output::Parent(parent::Request::Answered {
                    client,
                    answer: self::answer(answer, read, env.now),
                    detail,
                }));
                if let Some(names) = slot.inbox.report() {
                    out.push(Output::Parent(parent::Request::Unread { client, names }));
                }
                stop::finish(agent, env.now, id, out);
            }
            smith::Request::Complete {
                owner,
                name,
                grant,
                prompt,
                deadlines,
                max_completion_bytes,
                max_completion_blocks,
                max_failure_bytes,
                decoded_call_bytes,
            } => {
                assert!(
                    slot.completions.insert(owner, false).expect("root bounds live completions").is_none(),
                    "root output preserves its scoped identity and single right"
                );
                out.push(Output::Lower {
                    agent: id.token(),
                    request: Lower::Complete {
                        owner,
                        name,
                        grant,
                        prompt,
                        deadlines,
                        max_completion_bytes,
                        max_completion_blocks,
                        max_failure_bytes,
                        decoded_call_bytes,
                    },
                });
            }
            smith::Request::Cancel { owner } => match slot.completions.get_mut(&owner) {
                Some(cancelled) => {
                    if !*cancelled {
                        *cancelled = true;
                        out.push(Output::Lower { agent: id.token(), request: Lower::Cancel { owner } });
                    }
                }
                None => unreachable!("cancel retains a live provider right"),
            },
            smith::Request::HostCall { host_run, relay, name, tool, effect, input, deadline } => {
                assert_eq!(slot.logical_run, host_run, "root output preserves its scoped identity and single right");
                let call = crate::workspace::call(slot);
                assert!(
                    slot.relays
                        .insert(call, Relay { name: crate::workspace::Call::Host(relay), withdrawn: false })
                        .expect("root bounds its host rights")
                        .is_none(),
                    "root output preserves its scoped identity and single right"
                );
                out.push(Output::Parent(parent::Request::Called {
                    client,
                    logical_run: host_run,
                    call,
                    name: host::CallName {
                        activation: name.activation,
                        completion: name.completion,
                        position: name.position,
                    },
                    deadline,
                    ask: host::Ask::Host { tool, effect: self::effect(effect), body: Box::from(input.bytes()) },
                }));
            }
            smith::Request::WithdrawHost { relay } => {
                let mut found = None;
                for (call, pending) in &slot.relays {
                    match pending.name {
                        crate::workspace::Call::Host(name) => {
                            if name == relay {
                                found = Some(*call);
                                break;
                            }
                        }
                        crate::workspace::Call::Delivery(_) => {}
                    }
                }
                if let Some(call) = found {
                    let pending = slot.relays.get_mut(&call).expect("retained relay name");
                    if !pending.withdrawn {
                        pending.withdrawn = true;
                        out.push(Output::Parent(parent::Request::Withdrawn { client, call }));
                    }
                }
            }
            smith::Request::Rejected { grant } => out.push(Output::Parent(parent::Request::Rejected {
                client,
                account: grant.account,
                generation: grant.generation,
            })),
            smith::Request::Exhausted { account, retry_after } => {
                out.push(Output::Parent(parent::Request::Exhausted { client, account, retry_after }));
            }
            smith::Request::Long { host_run, operation, until } => {
                assert_eq!(slot.logical_run, host_run, "root workspace requests preserve logical scope");
                out.push(Output::Parent(parent::Request::Long {
                    client,
                    operation,
                    span: until.saturating_since(env.now),
                }));
            }
            smith::Request::LongDone { host_run } => {
                assert_eq!(slot.logical_run, host_run, "root workspace requests preserve logical scope");
                out.push(Output::Parent(parent::Request::LongDone { client }));
            }
            smith::Request::Deliver { host_run, owner, change, name, deadline } => {
                assert_eq!(slot.logical_run, host_run, "root workspace requests preserve logical scope");
                let call = crate::workspace::call(slot);
                assert!(
                    slot.relays
                        .insert(call, Relay { name: crate::workspace::Call::Delivery(owner), withdrawn: false })
                        .expect("bounded parent rights")
                        .is_none(),
                    "each delivery owns one unique parent call"
                );
                out.push(Output::Parent(parent::Request::Called {
                    client,
                    logical_run: host_run,
                    call,
                    name: host::CallName {
                        activation: name.activation,
                        completion: name.completion,
                        position: name.position,
                    },
                    deadline,
                    ask: host::Ask::Deliver {
                        fields: host::Fields::new(change, u64::MAX).expect("root bounds checked fields"),
                    },
                }));
            }
            smith::Request::Io { owner, op, deadline } => {
                crate::workspace::io(slot, id.token(), owner, op, deadline, out);
            }
            smith::Request::CancelIo { owner } => crate::workspace::cancel_io(slot, id.token(), owner, out),
            smith::Request::Read { owner, at, max, deadline } => {
                crate::workspace::read(slot, id.token(), owner, at, max, deadline, out);
            }
            smith::Request::Probe { owner, at, deadline } => {
                crate::workspace::probe(slot, id.token(), owner, at, deadline, out);
            }
            smith::Request::Check { owner, program, deadline, tail } => {
                crate::workspace::check(slot, id.token(), owner, program, deadline, tail, out);
            }
            smith::Request::Abort { owner } => crate::workspace::abort(slot, id.token(), owner, out),
        }
    }
}

const fn price_rate(rate: run::Rate) -> host::Rate {
    match rate {
        run::Rate::Input => host::Rate::Input,
        run::Rate::Cached => host::Rate::Cached,
        run::Rate::CacheWrite => host::Rate::CacheWrite,
        run::Rate::Output => host::Rate::Output,
    }
}

const fn budget_part(part: run::Part) -> host::Part {
    match part {
        run::Part::Turns => host::Part::Turns,
        run::Part::Spend => host::Part::Spend,
        run::Part::Time => host::Part::Time,
    }
}

const fn budget_refusal(refusal: run::BudgetRefusal) -> host::BudgetRefusal {
    match refusal {
        run::BudgetRefusal::Ceiling(ceiling) => host::BudgetRefusal::Ceiling(match ceiling {
            run::Ceiling::Turns(turns) => host::Ceiling::Turns(turns),
            run::Ceiling::Spend(spend) => host::Ceiling::Spend(spend),
            run::Ceiling::Time(time) => host::Ceiling::Time(time),
        }),
        run::BudgetRefusal::Empty(part) => host::BudgetRefusal::Empty(budget_part(part)),
        run::BudgetRefusal::Reserve(part) => host::BudgetRefusal::Reserve(budget_part(part)),
        run::BudgetRefusal::Completion { most } => host::BudgetRefusal::Completion { most },
        run::BudgetRefusal::Checks { reserve, checks } => host::BudgetRefusal::Checks { reserve, checks },
    }
}

const fn completion_limit(which: run::Limit) -> host::Limit {
    match which {
        run::Limit::Pool { reservation } => host::Limit::Pool { reservation },
        run::Limit::Path(cap) => host::Limit::Path(completion_cap(cap)),
        run::Limit::History => host::Limit::History,
        run::Limit::Window => host::Limit::Window,
        run::Limit::Call => host::Limit::Call,
        run::Limit::Message => host::Limit::Message,
    }
}

const fn completion_cap(cap: run::Cap) -> host::Cap {
    match cap {
        run::Cap::Request => host::Cap::Request,
        run::Cap::RequestHead => host::Cap::RequestHead,
        run::Cap::ResponseHead => host::Cap::ResponseHead,
        run::Cap::ResponseFields => host::Cap::ResponseFields,
        run::Cap::Skip => host::Cap::Skip,
        run::Cap::Field => host::Cap::Field,
        run::Cap::Retained => host::Cap::Retained,
        run::Cap::Tokens => host::Cap::Tokens,
        run::Cap::Depth => host::Cap::Depth,
        run::Cap::Strings => host::Cap::Strings,
        run::Cap::Number => host::Cap::Number,
        run::Cap::Reasoning => host::Cap::Reasoning,
        run::Cap::OutputItems => host::Cap::OutputItems,
        run::Cap::Metadata => host::Cap::Metadata,
        run::Cap::Tools => host::Cap::Tools,
        run::Cap::HistoryItems => host::Cap::HistoryItems,
        run::Cap::Output => host::Cap::Output,
        run::Cap::Answer => host::Cap::Answer,
        run::Cap::ErrorBody => host::Cap::ErrorBody,
        run::Cap::AccessToken => host::Cap::AccessToken,
        run::Cap::AccountId => host::Cap::AccountId,
    }
}

const fn completion_phase(phase: run::Phase) -> host::Phase {
    match phase {
        run::Phase::Connect => host::Phase::Connect,
        run::Phase::Handshake => host::Phase::Handshake,
        run::Phase::Head => host::Phase::Head,
        run::Phase::Idle => host::Phase::Idle,
        run::Phase::Whole => host::Phase::Whole,
    }
}
