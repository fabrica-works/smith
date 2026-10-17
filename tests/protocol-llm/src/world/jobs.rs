//! Timed command writes and held-job acts at the agent world's IO boundary.
use super::{Delivery, Duration, Event, Family, Flight, Key, Time, Token, World, io, tools};
use tools::{Done, Op};
use tools::{JobAct, JobEnd};

#[derive(Debug)]
pub(super) struct Held {
    started: io::Started,
    since: Time,
    deadline: Time,
    cursor: usize,
    end: Option<JobEnd>,
    response: Response,
    event: Option<Key>,
    caps: (u32, u32),
}

#[derive(Debug)]
enum Response {
    Yield { at: Time },
    Held,
    Read { at: Time },
}

impl World {
    pub(super) fn start_job(&mut self, owner: Token, op: &Op, deadline: Time) -> bool {
        let Op::Spawn { cwd, command, head, tail, after: Some(after) } = op else { return false };
        self.boundaries.push((self.now, super::Boundary::Io { op: op.clone() }));
        self.flights.open(
            (Family::Io, owner),
            Flight { key: None, cancelled: false, completion_message: None, completion_index: None },
        );
        let started = io::spawn(
            &self.disk,
            cwd,
            command,
            &[(b"PATH".to_vec(), b"/usr/bin:/bin".to_vec()), (b"HOME".to_vec(), b"/home/agent".to_vec())],
            (*head, *tail),
        );
        match started {
            Ok(started) => {
                let job = Held {
                    started,
                    since: self.now,
                    deadline,
                    cursor: 0,
                    end: None,
                    response: Response::Yield { at: self.now.saturating_add(*after) },
                    event: None,
                    caps: (*head, *tail),
                };
                self.keep_job(owner, job);
            }
            Err(done) => self.terminal(Family::Io, owner, Event::Done { owner, done }),
        }
        true
    }

    pub(super) fn job_act(&mut self, owner: Token, act: JobAct) {
        let mut job = self.jobs.remove(&owner).expect("an act names a held owner");
        if act == JobAct::Hasten {
            if matches!(job.response, Response::Read { .. }) {
                job.response = Response::Read { at: self.now };
            }
            if let Some(event) = job.event.take() {
                self.schedule.withdraw(event);
            }
            self.keep_job(owner, job);
            return;
        }
        assert!(matches!(job.response, Response::Held), "one act at a time per job");
        let (head, tail, wait) = match act {
            JobAct::Hasten => unreachable!("hastening retains the pending read"),
            JobAct::Read { head, tail, wait } => (head, tail, wait),
            JobAct::Stop { head, tail } => {
                if job.end.is_none() {
                    job.end = Some(JobEnd::Stopped);
                }
                (head, tail, Duration::ZERO)
            }
        };
        job.caps = (head, tail);
        job.response = Response::Read { at: self.now.saturating_add(wait) };
        if job.end.is_some() {
            job.response = Response::Read { at: self.now };
        }
        if let Some(event) = job.event.take() {
            self.schedule.withdraw(event);
        }
        self.keep_job(owner, job);
    }

    pub(super) fn cancel_job(&mut self, owner: Token) -> bool {
        let Some(mut job) = self.jobs.remove(&owner) else { return false };
        if let Some(event) = job.event.take() {
            self.schedule.withdraw(event);
        }
        if job.end.is_none() {
            job.end = Some(JobEnd::Cancelled);
        }
        match job.response {
            Response::Yield { .. } => self.terminal(Family::Io, owner, Event::Done { owner, done: Done::Cancelled }),
            Response::Held | Response::Read { .. } => {
                job.response = Response::Read { at: self.now };
                self.keep_job(owner, job);
            }
        }
        true
    }

    pub(super) fn job_tick(&mut self, owner: Token) {
        let mut job = self.jobs.remove(&owner).expect("only a live command has deliveries");
        job.event = None;
        let ends = job.since.saturating_add(Duration::from_nanos(
            u64::try_from(job.started.process.program.duration.as_nanos()).expect("finite fixture"),
        ));
        if job.end.is_none() && self.now >= ends.min(job.deadline) {
            if ends <= job.deadline {
                io::finish(&mut self.disk, &job.started.process);
                let exit = match job.started.process.program.exit {
                    skein_fake_checkout::Exit::Code(code) => tools::Exit::Code { code },
                    skein_fake_checkout::Exit::Signal(signal) => tools::Exit::Signal { signal },
                };
                job.end = Some(JobEnd::Ended(exit));
            } else {
                job.end = Some(JobEnd::Ended(tools::Exit::TimedOut));
            }
        }
        let answer = match job.response {
            Response::Yield { at } | Response::Read { at } => self.now >= at || job.end.is_some(),
            Response::Held => false,
        };
        if answer {
            let written = job.written(self.now);
            let done = io::exited(
                Some(skein_fake_checkout::Exit::Code(0)),
                &job.started.process.program.output[job.cursor..written],
                job.caps.0,
                job.caps.1,
            );
            job.cursor = written;
            let Done::Exited { head, tail, dropped, .. } = done else { unreachable!("capture returns an exit") };
            let done = match job.response {
                Response::Yield { .. } => match job.end {
                    None => Done::Held { head, tail, dropped },
                    Some(JobEnd::Ended(exit)) => Done::Exited { head, tail, dropped, exit },
                    Some(JobEnd::Cancelled) => Done::Cancelled,
                    Some(JobEnd::Running | JobEnd::Stopped) => unreachable!("a yield has no stop"),
                },
                Response::Read { .. } => Done::Job { head, tail, dropped, end: job.end.unwrap_or(JobEnd::Running) },
                Response::Held => unreachable!("held output needs a read"),
            };
            if job.end.is_some() {
                self.terminal(Family::Io, owner, Event::Done { owner, done });
            } else {
                self.stage.push(Event::Done { owner, done });
            }
            if job.end.is_some() {
                return;
            }
            job.response = Response::Held;
        }
        self.keep_job(owner, job);
    }

    fn keep_job(&mut self, owner: Token, mut job: Held) {
        let process = if job.end.is_none() {
            Some(job.deadline.min(job.since.saturating_add(Duration::from_nanos(
                u64::try_from(job.started.process.program.duration.as_nanos()).expect("finite fixture"),
            ))))
        } else {
            None
        };
        let answer = match job.response {
            Response::Yield { at } | Response::Read { at } => Some(at),
            Response::Held => None,
        };
        let next = match (process, answer) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        job.event = next.map(|at| self.schedule.send(at, Delivery::Job { owner }));
        assert!(self.jobs.insert(owner, job).is_none(), "one process per original token");
    }
}

impl Held {
    fn written(&self, now: Time) -> usize {
        let ran = now.min(self.deadline).saturating_since(self.since);
        let total = self.started.process.program.output.len();
        usize::try_from(
            u128::from(ran.as_nanos()) * total as u128 / self.started.process.program.duration.as_nanos().max(1),
        )
        .expect("finite output")
        .min(total)
    }
}
