//! A kit's numbered commands and the reads awaiting their unread output.
//! Original spawn tokens stay live; same-job reads reach the machine one at a
//! time. Provider ids and output bytes never enter this table.
//! Contract: domain/tools.md, section 5; programming-model.md, sections 5.2 and 6.3.

use crate::command::{Kept, Timed};
use crate::domain::Domain;
use crate::facts::{self, FactKind};
use crate::flight::{self, Flight};
use crate::kit::{self, Kit};
use crate::{CallInfo, Done, JobAct, JobEnd, JobNumber, JobProblem, Limits, Outcome, Request};
use skein_lib::{Env, Id, Map, Queue, ReplyTo, Time};

#[derive(Debug)]
pub(crate) struct Jobs {
    held: Map<u32, Job>,
    reads: Map<u64, Read>,
    highest: u32,
    next_read: u64,
}

#[derive(Debug)]
pub(crate) struct Job {
    owner: Id<Flight>,
    reader: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct Read {
    job: JobNumber,
    reply_to: ReplyTo,
    observation: Option<CallInfo>,
    timed: Timed,
    kept: Kept,
    deadline: Time,
    stop: bool,
}

impl Jobs {
    pub(crate) fn new(limits: &Limits) -> Self {
        Self {
            held: Map::with_capacity(limits.jobs),
            reads: Map::with_capacity(limits.calls),
            highest: 0,
            next_read: 0,
        }
    }

    pub(crate) fn full(&self) -> bool {
        self.held.len() >= self.held.capacity()
    }

    pub(crate) fn empty(&self) -> bool {
        self.held.is_empty()
    }

    pub(crate) fn calls(&self) -> u32 {
        self.reads.len()
    }

    pub(crate) fn held(&mut self, owner: Id<Flight>) -> JobNumber {
        self.highest = self.highest.checked_add(1).expect("a conversation's job numbers fit u32");
        let number = JobNumber(self.highest);
        self.held.insert(number.0, Job { owner, reader: None }).expect("the session serializes yielding shell writes");
        number
    }

    pub(crate) fn close(&self, out: &mut Queue<Request>) {
        for (_, job) in &self.held {
            out.push(Request::CancelIo { owner: job.owner.token() });
        }
    }
}

/// Enqueue a read; another read of this number must consume its answer first.
pub(crate) fn call(domain: &mut Domain, env: &Env<Limits>, id: Id<Kit>, read: Read, out: &mut Queue<Request>) {
    let kit = domain.kits.get_mut(id).expect("the session owns its kit");
    let job = read.job;
    let problem = if kit.jobs.held.contains_key(&job.0) { None } else { Some(JobProblem::Unknown { job }) };
    if let Some(problem) = problem {
        out.push(Request::Invalid { to: read.reply_to, problem });
        return;
    }
    if let Err(outcome) = kit::ready(kit, read.deadline, env) {
        domain.facts.push_call(
            facts::answered(
                kit::session(kit),
                crate::Tool::Job,
                &outcome,
                facts::Observation { omitted_bytes: 0, clamped: false },
            ),
            read.observation,
        );
        out.push(Request::Answer { to: read.reply_to, outcome });
        return;
    }
    domain.facts.push_call(FactKind::Started { session: kit::session(kit), tool: crate::Tool::Job }, read.observation);
    kit.jobs.next_read = kit.jobs.next_read.checked_add(1).expect("bounded activation calls fit u64");
    let key = kit.jobs.next_read;
    kit.jobs.reads.insert(key, read).expect("the common call admission reserves a read cell");
    advance(&mut kit.jobs, job, env.now, out);
}

pub(crate) fn read(
    job: JobNumber,
    reply_to: ReplyTo,
    observation: Option<CallInfo>,
    timed: Timed,
    kept: Kept,
    deadline: Time,
    stop: bool,
) -> Read {
    Read { job, reply_to, observation, timed, kept, deadline, stop }
}

fn advance(jobs: &mut Jobs, number: JobNumber, now: Time, out: &mut Queue<Request>) {
    let job = jobs.held.get_mut(&number.0).expect("a queued read names a held job");
    if job.reader.is_some() {
        return;
    }
    for (key, read) in &jobs.reads {
        if read.job == number {
            job.reader = Some(*key);
            let wait = read.timed.timeout.min(read.deadline.saturating_since(now));
            out.push(Request::Job {
                owner: job.owner.token(),
                act: if read.stop {
                    JobAct::Stop { head: read.kept.head, tail: read.kept.tail }
                } else {
                    JobAct::Read { head: read.kept.head, tail: read.kept.tail, wait }
                },
            });
            return;
        }
    }
}

/// A held owner's next terminal settles its active read, keeping the owner if
/// still running. Close also receives a terminal when no read was outstanding.
pub(crate) fn done(
    domain: &mut Domain,
    env: &Env<Limits>,
    owner: Id<Flight>,
    number: JobNumber,
    done: Done,
    out: &mut Queue<Request>,
) {
    let kit_id = flight::kit(domain.flights.get(owner).expect("a held owner lives until its end"));
    let kit = domain.kits.get_mut(kit_id).expect("a kit owns every held command");
    let (head, tail, dropped, end) = match done {
        Done::Job { head, tail, dropped, end } => (head, tail, dropped, end),
        Done::Loaded { .. }
        | Done::Listed { .. }
        | Done::Stored { .. }
        | Done::Conflict { .. }
        | Done::Exited { .. }
        | Done::Held { .. }
        | Done::Found { .. }
        | Done::Missing
        | Done::NotFile
        | Done::Linked
        | Done::NotDirectory
        | Done::TooLarge { .. }
        | Done::Escapes
        | Done::Failed { .. }
        | Done::TimedOut
        | Done::Cancelled => unreachable!("a held command answers a job act"),
    };
    let job = kit.jobs.held.get_mut(&number.0).expect("the original owner has its job cell");
    let ended = match end {
        JobEnd::Running => false,
        JobEnd::Ended(_) | JobEnd::Stopped | JobEnd::Cancelled => true,
    };
    if let Some(reader) = job.reader.take() {
        let read = kit.jobs.reads.remove(&reader).expect("an active read owns its reply right");
        let outcome =
            Outcome::Job { job: number, head, tail, dropped, end, wait: read.timed.clamp, output: read.kept.clamp };
        domain.facts.push_call(
            facts::answered(
                kit::session(kit),
                crate::Tool::Job,
                &outcome,
                facts::Observation { omitted_bytes: 0, clamped: false },
            ),
            read.observation,
        );
        out.push(Request::Answer { to: read.reply_to, outcome });
    } else {
        assert!(kit::closing(kit) && ended, "only close cancels an unpolled job");
    }
    if ended {
        kit.jobs.held.remove(&number.0).expect("the job ends once");
        // Calls queued behind an end now name a job whose end was answered.
        let mut queued = skein_lib::List::with_capacity(env.limits.calls);
        for (key, read) in &kit.jobs.reads {
            if read.job == number {
                queued.push(*key).expect("at most calls reads await output");
            }
        }
        for key in &queued {
            let read = kit.jobs.reads.remove(key).expect("a queued right ends once");
            out.push(Request::Invalid { to: read.reply_to, problem: JobProblem::Unknown { job: number } });
        }
        domain.flights.retire(owner);
        kit::settled(&mut domain.kits, &mut domain.facts, kit_id, out);
    } else {
        advance(&mut kit.jobs, number, env.now, out);
    }
}

pub(crate) fn worst_case(limits: &Limits) -> Option<u64> {
    Map::<u32, Job>::worst_case(limits.jobs)?
        .checked_add(Map::<u64, Read>::worst_case(limits.calls)?)?
        .checked_add(skein_lib::List::<u64>::worst_case(limits.calls)?)
}
