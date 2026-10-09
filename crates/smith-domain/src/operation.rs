//! Issued command notices (domain/host.md, section 4; protocol/channel.md,
//! section 4). The root's bounded map retains IO owners and logical runs.
//! These entry points announce actual Spawn issuance and its actual Done,
//! never a cancellation request or provider/tool syntax.

use crate::{Domain, Operation, Request, run, tools};
use skein_lib::{Queue, Time, Token};

pub(crate) fn command(domain: &mut Domain, owner: Token, op: &tools::Op, deadline: Time, out: &mut Queue<Request>) {
    let spawn = match op {
        tools::Op::Spawn { .. } => true,
        tools::Op::Load { .. } | tools::Op::Scan { .. } | tools::Op::Store { .. } | tools::Op::Search { .. } => false,
    };
    if spawn {
        let conversation = domain.session.io_opener(owner).expect("issued command retains its session");
        let host_run = run::host_name(&domain.run, conversation).expect("issued command retains its run");
        let old = domain.commands.insert(owner, host_run).expect("one command per live tools call");
        assert!(old.is_none(), "the previous operation has actually ended");
        out.push(Request::Long { host_run, operation: Operation::Command, until: deadline });
    }
}

pub(crate) fn command_done(domain: &mut Domain, owner: Token) {
    if let Some(host_run) = domain.commands.remove(&owner) {
        domain.notices.push(Request::LongDone { host_run });
    }
}
