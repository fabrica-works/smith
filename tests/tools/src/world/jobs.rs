//! Timed command writes and held-job acts at the tools world's IO boundary.
use super::{Delivery, Done, Duration, Key, Op, Time, Token, World, tools, translate};
use tools::{JobAct, JobEnd};

pub(super) struct Job {
    started: translate::Started,
    since: Time,
    deadline: Time,
    output: Vec<(Duration, Vec<u8>)>,
    cursor: usize,
    end: Option<JobEnd>,
    response: Response,
    event: Option<Key>,
    caps: (u32, u32),
}

enum Response {
    Yield { at: Time },
    Held,
    Read { at: Time },
}

impl World {
    /// Script a command's writes; their concatenation must equal its program output.
    pub fn job_output(&mut self, command: &[u8], writes: &[(Duration, &[u8])]) {
        self.job_outputs.insert(command.to_vec(), writes.iter().map(|(at, bytes)| (*at, bytes.to_vec())).collect());
    }

    pub(super) fn start_yielded(&mut self, owner: Token, op: &Op, deadline: Time) -> bool {
        let Op::Spawn { cwd, command, head, tail, after: Some(after) } = op else { return false };
        self.stats.commands += 1;
        let started = translate::spawn(
            &self.checkout,
            cwd,
            command,
            &[(b"PATH".to_vec(), b"/usr/bin:/bin".to_vec()), (b"HOME".to_vec(), b"/home/agent".to_vec())],
            (*head, *tail),
        );
        match started {
            Ok(started) => {
                let output = self.job_outputs.get(command.as_ref()).cloned().unwrap_or_default();
                if !output.is_empty() {
                    assert_eq!(
                        output.iter().flat_map(|(_, bytes)| bytes.iter().copied()).collect::<Vec<_>>(),
                        started.process.program.output
                    );
                }
                let job = Job {
                    started,
                    since: self.now,
                    deadline,
                    output,
                    cursor: 0,
                    end: None,
                    response: Response::Yield { at: self.now.saturating_add(*after) },
                    event: None,
                    caps: (*head, *tail),
                };
                self.keep_job(owner, job);
            }
            Err(done) => self.stage.push(tools::Event::Done { owner, done }),
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
                self.wire.withdraw(event);
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
            self.wire.withdraw(event);
        }
        self.keep_job(owner, job);
    }

    pub(super) fn cancel_job(&mut self, owner: Token) -> bool {
        let Some(mut job) = self.jobs.remove(&owner) else { return false };
        if let Some(event) = job.event.take() {
            self.wire.withdraw(event);
        }
        if job.end.is_none() {
            job.end = Some(JobEnd::Cancelled);
        }
        match job.response {
            Response::Yield { .. } => self.stage.push(tools::Event::Done { owner, done: Done::Cancelled }),
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
                self.perform_command(&job.started.process);
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
            let done = translate::exited(
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
            self.stage.push(tools::Event::Done { owner, done });
            if job.end.is_some() {
                return;
            }
            job.response = Response::Held;
        }
        self.keep_job(owner, job);
    }

    fn keep_job(&mut self, owner: Token, mut job: Job) {
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
        job.event = next.map(|at| self.wire.send(at, Delivery::Job { owner }));
        assert!(self.jobs.insert(owner, job).is_none(), "one process per original token");
    }
}

impl Job {
    fn written(&self, now: Time) -> usize {
        let ran = now.min(self.deadline).saturating_since(self.since);
        if self.output.is_empty() {
            let total = self.started.process.program.output.len();
            usize::try_from(
                u128::from(ran.as_nanos()) * total as u128 / self.started.process.program.duration.as_nanos().max(1),
            )
            .expect("finite output")
            .min(total)
        } else {
            self.output.iter().filter(|(at, _)| *at <= ran).map(|(_, bytes)| bytes.len()).sum()
        }
    }
}

impl World {
    /// Content-free job terminals, in the order each session received them.
    #[must_use]
    pub fn job_facts(&self, session: u64) -> Vec<tools::Fact> {
        self.facts
            .get(&session)
            .into_iter()
            .flatten()
            .filter(|fact| matches!(fact.kind, tools::FactKind::JobEnded { .. }))
            .copied()
            .collect()
    }

    pub(super) fn observe_job_answer(&mut self, session: u64, result: &Result<tools::Outcome, tools::JobProblem>) {
        match result {
            Ok(tools::Outcome::Running { job, .. }) => {
                assert!(self.job_numbers.insert((session, job.0), false).is_none(), "job numbers never repeat");
            }
            Ok(tools::Outcome::Job { job, end, .. }) => {
                let ended = self.job_numbers.get_mut(&(session, job.0)).expect("a job read names a running answer");
                assert!(!*ended, "a terminal is told once");
                if *end != JobEnd::Running {
                    *ended = true;
                }
            }
            _ => {}
        }
        let running = self.job_numbers.iter().filter(|((whose, _), ended)| *whose == session && !**ended).count();
        assert!(running <= self.settings.tools.jobs as usize, "at most jobs commands run per kit");
    }

    pub(super) fn observe_job_fact(&mut self, fact: &tools::Fact) {
        match fact.kind {
            tools::FactKind::JobEnded { session, job, end, .. } => {
                assert_ne!(end, JobEnd::Running, "a close reports a terminal");
                let ended =
                    self.job_numbers.get_mut(&(session.raw(), job.0)).expect("a terminal names a running answer");
                assert!(!*ended, "a job terminal is told once by answer or fact");
                *ended = true;
            }
            tools::FactKind::Closed { session } => {
                assert!(
                    self.job_numbers.iter().all(|((whose, _), ended)| *whose != session.raw() || *ended),
                    "every job ends before closed"
                );
            }
            tools::FactKind::Opened { .. }
            | tools::FactKind::Refused { .. }
            | tools::FactKind::Started { .. }
            | tools::FactKind::Answered { .. }
            | tools::FactKind::Closing { .. } => {}
        }
    }
}
