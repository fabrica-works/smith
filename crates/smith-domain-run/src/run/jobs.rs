//! Writers wait for session jobs to settle (domain/run.md, sections 5.3 and 8.1).
//! The pending child owns its bounded opening; a withdrawal retains its cause
//! until the parent's cancellation acknowledgement. No child is opened then.

use super::{Conversation, Phase, agent, child_ready, may_finish, mem, retire_call};
use crate::agent::Child;
use crate::call::{Withdrawal, Work};
use crate::{Domain, Limits, Request, land};
use skein_lib::{Env, Id, Queue, Slab, Token};

pub(super) fn peer(conversations: &Slab<Conversation>, conversation: Id<Conversation>) -> Token {
    match conversations.get(conversation).expect("live writer").phase {
        Phase::Running { peer } => peer,
        Phase::Pending | Phase::Opening | Phase::Unwanted | Phase::Closing | Phase::Closed => {
            unreachable!("only a running session asks for a writer")
        }
    }
}

pub(crate) fn cancelled(domain: &mut Domain, env: &Env<Limits>, conversation: Token, out: &mut Queue<Request>) {
    let conversation = Id::<Conversation>::from_token(conversation);
    let Some(id) = domain.calls.exclusive(conversation) else { return };
    let call = domain.calls.get_mut(id).expect("exclusive call waits for jobs");
    domain.facts.about(call.run.token());
    let run = domain.runs.get_mut(call.run).expect("writer retains its run");
    match &mut call.work {
        Work::Landing(landing) => {
            if !may_finish(&run.state) {
                land::withdraw(landing, id, Withdrawal::Withdrawn, out);
            }
            let settled = land::jobs_cancelled(landing, id, call.owner, run, env, out);
            super::settle(domain, id, settled, out);
        }
        Work::Child(state) => {
            let (child, opening, deadline, why) = match mem::replace(state, Child::Closed) {
                Child::Cancelling { child, opening, deadline, why } => (child, opening, deadline, why),
                Child::Working { .. } | Child::Answering { .. } | Child::Closing { .. } | Child::Closed => {
                    unreachable!("job settlement precedes the writing child's open")
                }
            };
            let waiting = domain.conversations.get_mut(child).expect("pending child retains its slot");
            let unwanted = match waiting.phase {
                Phase::Opening => false,
                Phase::Unwanted => true,
                Phase::Pending | Phase::Running { .. } | Phase::Closing | Phase::Closed => {
                    unreachable!("child has not opened before jobs settle")
                }
            };
            let unwanted = unwanted || !may_finish(&run.state);
            let why = if env.now >= deadline { why.or(Some(Withdrawal::Expired)) } else { why };
            match why {
                None if !unwanted => {
                    waiting.opened_at = env.now;
                    *state = agent::working(child);
                    out.push(Request::Open { conversation: child.token(), opening: *opening });
                }
                None | Some(_) => {
                    let why = why.unwrap_or(Withdrawal::Withdrawn);
                    out.push(Request::Return { spent: 0, call: call.owner, result: crate::call::stopped(why) });
                    assert!(run.children.remove(&child), "pending child belongs to its run");
                    run.conversations = run.conversations.checked_sub(1).expect("pending child was counted");
                    run.lease.child_returned(id);
                    let finishing = match &mut run.wind_down {
                        Some(phase) => phase.child_ended(),
                        None => false,
                    };
                    waiting.phase = Phase::Closed;
                    domain.conversations.retire(child);
                    retire_call(&mut domain.calls, &mut domain.alarms, &mut domain.conversations, id);
                    if finishing {
                        child_ready(run, out);
                    }
                }
            }
        }
        Work::Host(_) => unreachable!("a host call does not cancel session jobs"),
    }
}
