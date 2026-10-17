//! A command's yield and successive acts on its held output (protocol/agent.md, 2).
//! The process keeps its original deadline; only an outstanding answer has a wait.

use super::{Process, Stop, Why, tool_exit};
use crate::{Below, ToDomain};
use skein_lib::{Duration, Queue, Time, Token};
use smith_domain::tools;

#[derive(Debug)]
pub(super) enum Response {
    Final,
    Yield { at: Time, head: u32, tail: u32 },
    Held { head: u32, tail: u32 },
    Read { until: Time, head: u32, tail: u32 },
    Closing,
}

pub(crate) enum Advance {
    Waiting,
    Answer { terminal: ToDomain, retire: bool },
    Finished,
    Closed,
}

impl Process {
    pub(crate) fn yielding(&mut self, now: Time, after: Option<Duration>, head: u32, tail: u32, kept: u32) {
        if after.is_some() {
            self.output = super::Capture::new(kept, kept);
        }
        self.response = match after {
            Some(after) => Response::Yield { at: now.saturating_add(after), head, tail },
            None => Response::Final,
        };
    }

    pub(crate) fn job(&mut self, now: Time, grace: Duration, act: tools::JobAct, below: &mut Queue<Below>) {
        match act {
            tools::JobAct::Hasten => {
                match &mut self.response {
                    Response::Read { until, .. } => *until = now,
                    Response::Held { .. } => {}
                    Response::Final | Response::Yield { .. } | Response::Closing => {
                        unreachable!("a hastened call has started its read")
                    }
                }
                return;
            }
            tools::JobAct::Read { .. } | tools::JobAct::Stop { .. } => {}
        }
        match self.response {
            Response::Held { .. } => {}
            Response::Final | Response::Yield { .. } | Response::Read { .. } | Response::Closing => {
                unreachable!("one act at a time on a held job")
            }
        }
        let (head, tail, until) = match act {
            tools::JobAct::Hasten => unreachable!("handled before a new read"),
            tools::JobAct::Read { head, tail, wait } => (head, tail, now.saturating_add(wait)),
            tools::JobAct::Stop { head, tail } => {
                self.stop(now, grace, Why::Stop, below);
                (head, tail, Time::from_nanos(u64::MAX))
            }
        };
        self.response = Response::Read { until, head, tail };
    }

    pub(crate) fn cancel_job(&mut self) {
        match self.response {
            Response::Held { head, tail } => {
                self.response = Response::Read { until: Time::from_nanos(u64::MAX), head, tail };
            }
            Response::Final | Response::Yield { .. } | Response::Read { .. } | Response::Closing => {}
        }
    }

    pub(crate) fn close_job(&mut self, below: &mut Queue<Below>) {
        match self.response {
            Response::Held { .. } => {
                self.response = Response::Closing;
                self.stop = Stop::Closing { why: Why::Cancel };
                if let Some(child) = self.child
                    && !self.child_closed
                {
                    below.push(Below::Process(skein_io::Request::Close { entity: child }));
                }
            }
            Response::Final | Response::Yield { .. } | Response::Read { .. } | Response::Closing => {}
        }
    }

    pub(crate) fn advance(&mut self, now: Time, owner: Token) -> Advance {
        match self.response {
            Response::Final => {
                if self.ready() {
                    Advance::Finished
                } else {
                    Advance::Waiting
                }
            }
            Response::Yield { at, head, tail } => {
                if self.ready() {
                    let (head, tail, dropped) = self.output.take(head, tail);
                    let done = match self.job_end() {
                        tools::JobEnd::Ended(exit) => {
                            if self.failed {
                                tools::Done::Failed { fault: tools::Fault::Other }
                            } else {
                                tools::Done::Exited { head, tail, dropped, exit }
                            }
                        }
                        tools::JobEnd::Cancelled => tools::Done::Cancelled,
                        tools::JobEnd::Stopped | tools::JobEnd::Running => unreachable!("yield has no job act"),
                    };
                    Advance::Answer { terminal: ToDomain::Done { owner, done }, retire: true }
                } else if now >= at {
                    self.response = Response::Held { head, tail };
                    let (head, tail, dropped) = self.output.take(head, tail);
                    Advance::Answer {
                        terminal: ToDomain::Done { owner, done: tools::Done::Held { head, tail, dropped } },
                        retire: false,
                    }
                } else {
                    Advance::Waiting
                }
            }
            Response::Held { .. } => Advance::Waiting,
            Response::Read { until, head, tail } => {
                let ended = self.ready();
                if ended || now >= until {
                    let end = if ended { self.job_end() } else { tools::JobEnd::Running };
                    self.response = Response::Held { head, tail };
                    let (head, tail, dropped) = self.output.take(head, tail);
                    Advance::Answer {
                        terminal: ToDomain::Done { owner, done: tools::Done::Job { head, tail, dropped, end } },
                        retire: ended,
                    }
                } else {
                    Advance::Waiting
                }
            }
            Response::Closing => {
                if self.ready() {
                    Advance::Closed
                } else {
                    Advance::Waiting
                }
            }
        }
    }

    pub(crate) fn closing_ready(&self) -> bool {
        match self.response {
            Response::Closing => self.ready(),
            Response::Final | Response::Yield { .. } | Response::Held { .. } | Response::Read { .. } => false,
        }
    }

    pub(super) fn answer_deadline(&self) -> Option<Time> {
        match self.response {
            Response::Yield { at, .. } => Some(at),
            Response::Read { until, .. } => Some(until),
            Response::Final | Response::Held { .. } | Response::Closing => None,
        }
    }

    fn job_end(&self) -> tools::JobEnd {
        match self.stop {
            Stop::Running => tools::JobEnd::Ended(tool_exit(self.exit, false)),
            Stop::Terminating { why, .. } | Stop::Killing { why, .. } | Stop::Closing { why } => match why {
                Why::Deadline => tools::JobEnd::Ended(tools::Exit::TimedOut),
                Why::Stop => tools::JobEnd::Stopped,
                Why::Cancel => tools::JobEnd::Cancelled,
            },
        }
    }
}
