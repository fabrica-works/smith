//! Cancel and terminal settlement (domain/host.md, section 9.2).
//! The slot keeps submitted rights after its answer and requests their cleanup
//! once. Gone requires every provider/parent terminal; a drop releases held
//! turns unacknowledged. Answered roots still hear their owed terminals;
//! dropped roots hear nothing. The injected clock bounds cancellation grace.

#![expect(clippy::manual_let_else, reason = "checked slot lookup remains exhaustive")]

use alloc::boxed::Box;
use skein_lib::{Env, Id, List, Queue, Time};
use smith_domain as smith;
use smith_host_domain::{End, parent};

use crate::domain::{Domain, Slot, observe, root_step};
use crate::{FactKind, Limits, Lower, Output};

pub(crate) fn cancel(agent: &mut Domain, env: &Env<Limits>, id: Id<Slot>, out: &mut Queue<Output>) {
    let slot = match agent.slots.get_mut(id) {
        Some(slot) => slot,
        None => return,
    };
    match slot.state {
        Stage::Live => slot.state = Stage::Cancelling { grace: env.now.saturating_add(env.limits.cancel_grace) },
        Stage::Cancelling { .. } | Stage::Settling(_) | Stage::Gone => return,
    }
    let run = slot.run;
    let client = slot.client;
    observe(agent, env.now, FactKind::Stopped { client });
    if let Some(run) = run {
        root_step(agent, env, id, smith::Event::Cancel { run }, out);
    }
}

pub(crate) fn finish(agent: &mut Domain, now: Time, id: Id<Slot>, out: &mut Queue<Output>) {
    let slot = agent.slots.get_mut(id).expect("answer belongs to a retained slot");
    match slot.state {
        Stage::Live | Stage::Cancelling { .. } => slot.state = Stage::Settling(Settling::Answered),
        Stage::Settling(_) | Stage::Gone => unreachable!("a root answers exactly once"),
    }
    cleanup(agent, now, id, out);
}

pub(crate) fn drop_run(agent: &mut Domain, now: Time, id: Id<Slot>, out: &mut Queue<Output>) {
    let slot = match agent.slots.get_mut(id) {
        Some(slot) => slot,
        None => return,
    };
    let unanswered = match slot.state {
        Stage::Live | Stage::Cancelling { .. } => true,
        Stage::Settling(Settling::Answered) => false,
        Stage::Settling(Settling::Dropped) | Stage::Gone => return,
    };
    if unanswered {
        slot.state = Stage::Settling(Settling::Dropped);
    }
    slot.turns = skein_lib::Map::with_capacity(slot.turns.capacity());
    let client = slot.client;
    if unanswered {
        if let Some(names) = slot.inbox.report() {
            out.push(Output::Parent(parent::Request::Unread { client, names }));
        }
        out.push(Output::Parent(parent::Request::Faulted { client, fault: smith_host_domain::Fault::Exited }));
    }
    if unanswered {
        observe(agent, now, FactKind::Dropped { client });
    }
    cleanup(agent, now, id, out);
}

fn cleanup(agent: &mut Domain, now: Time, id: Id<Slot>, out: &mut Queue<Output>) {
    let slot = agent.slots.get_mut(id).expect("cleanup belongs to a retained slot");
    let mut cancel = List::with_capacity(slot.completions.capacity());
    for (owner, cancelled) in &slot.completions {
        if !cancelled {
            cancel.push(*owner).expect("one owner per completion");
        }
    }
    for owner in &cancel {
        *slot.completions.get_mut(owner).expect("retained provider right") = true;
        out.push(Output::Lower { agent: id.token(), request: Lower::Cancel { owner: *owner } });
    }
    crate::workspace::cancel(slot, id.token(), out);
    let mut withdraw = List::with_capacity(slot.relays.capacity());
    for (owner, relay) in &slot.relays {
        if !relay.withdrawn {
            withdraw.push(*owner).expect("one owner per relay");
        }
    }
    for owner in &withdraw {
        slot.relays.get_mut(owner).expect("retained host right").withdrawn = true;
        out.push(Output::Parent(parent::Request::Withdrawn { client: slot.client, call: *owner }));
    }
    settle(agent, now, id, out);
}

pub(crate) fn settle(agent: &mut Domain, now: Time, id: Id<Slot>, out: &mut Queue<Output>) {
    let slot = agent.slots.get_mut(id).expect("settlement belongs to a retained slot");
    match slot.state {
        Stage::Settling(_) => {
            if !slot.completions.is_empty()
                || !slot.io.is_empty()
                || !slot.machine.is_empty()
                || !slot.relays.is_empty()
                || !slot.turns.is_empty()
            {
                return;
            }
            slot.state = Stage::Gone;
            let client = slot.client;
            if let Some(names) = slot.inbox.report() {
                out.push(Output::Parent(parent::Request::Unread { client, names }));
            }
            out.push(Output::Parent(parent::Request::Gone { client, end: End::Settled, detail: Box::default() }));
            observe(agent, now, FactKind::Gone { client, end: End::Settled });
        }
        Stage::Live | Stage::Cancelling { .. } | Stage::Gone => {}
    }
}

/// A retained slot's end, owned by cancellation and settlement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Stage {
    /// The root runs and receives its inputs.
    Live,
    /// The root heard cancellation and has until the host's grace to answer.
    Cancelling { grace: Time },
    /// Every retained lower right is ending for this reason.
    Settling(Settling),
    /// Every terminal settled; native observations still keep the slot until drained.
    Gone,
}

/// Whether the root may hear its remaining owed terminals (domain/host.md, section 9.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Settling {
    /// An answered root continues receiving the terminals it owns.
    Answered,
    /// A dropped root receives nothing; the slot settles each terminal itself.
    Dropped,
}
