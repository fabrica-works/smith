//! Independent write-holder oracle over actual run requests and returns.
//! Skein owns referee plumbing. Contract: domain/run.md, sections 5.3, 8.1 and 13.

use skein_lib::{Time, Token};
use skein_world::domain::{Expectations, Judge, Referee, Verdict};
use std::collections::BTreeMap;

/// One actual opening, operation or terminal at the run boundary.
#[derive(Clone, Copy, Debug)]
pub enum Seen {
    /// A shell answer names its conversation's new job.
    JobStarted { conversation: Token, job: u32 },
    /// An answer or close reports that job's one end.
    JobTerminal { conversation: Token, job: u32 },
    /// Each job terminal precedes its conversation's close.
    ConversationClosed { conversation: Token },
    /// An admitted child opens with its effective authority.
    ChildOpened { call: Token, writing: bool },
    /// The child's terminal returns its calling session's call.
    ChildReturned { call: Token },
    /// The first check or submission starts an exclusive workspace operation.
    WorkspaceOpened { owner: Token },
    /// A discovered check runs under this operation's holder.
    Check { owner: Token },
    /// The checked snapshot is submitted under the same holder.
    Delivery { owner: Token },
    /// The workspace operation has returned to its calling session.
    WorkspaceReturned { owner: Token },
    /// The admitted run gives its final answer.
    Answer,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Holder {
    Child(Token),
    Workspace(Token),
}

/// Effective child authorities and the one external holder for an admitted run.
#[derive(Default)]
pub struct Meeting {
    jobs: BTreeMap<(Token, u32), bool>,
    children: BTreeMap<Token, bool>,
    holder: Option<Holder>,
    answered: bool,
}

impl Expectations for Meeting {
    type Seen = Seen;
    type Name = &'static str;
    type Stimulus = ();

    fn observe(&mut self, seen: Seen, judge: &mut Judge<Self::Name, Self::Stimulus>) {
        judge.check(!self.answered, "no lease observation follows the run's answer");
        match seen {
            Seen::JobStarted { conversation, job } => {
                judge.check(self.jobs.insert((conversation, job), false).is_none(), "a job number names one start");
            }
            Seen::JobTerminal { conversation, job } => {
                let ended = self.jobs.get_mut(&(conversation, job)).expect("job terminal follows its start");
                judge.check(!*ended, "one terminal per job");
                *ended = true;
            }
            Seen::ConversationClosed { conversation } => {
                judge.check(
                    self.jobs.iter().all(|((owner, _), ended)| *owner != conversation || *ended),
                    "each job's terminal precedes conversation close",
                );
            }
            Seen::ChildOpened { call, writing } => {
                judge.check(self.holder.is_none(), "a child opens only without another write holder");
                judge.check(!writing || self.children.is_empty(), "a writing child runs alone");
                judge.check(self.children.insert(call, writing).is_none(), "a child's call opens once");
                if writing {
                    self.holder = Some(Holder::Child(call));
                }
            }
            Seen::ChildReturned { call } => {
                let writing = self.children.remove(&call);
                judge.check(writing.is_some(), "a child's return follows its opening");
                if writing == Some(true) {
                    judge.check(
                        self.holder == Some(Holder::Child(call)),
                        "the writing child's own return ends its lease",
                    );
                    self.holder = None;
                }
            }
            Seen::WorkspaceOpened { owner } => {
                judge.check(self.holder.is_none() && self.children.is_empty(), "the whole workspace has one holder");
                self.holder = Some(Holder::Workspace(owner));
            }
            Seen::Check { owner } | Seen::Delivery { owner } => {
                judge.check(
                    self.holder == Some(Holder::Workspace(owner)),
                    "checks and delivery hold the whole workspace",
                );
                judge.check(self.children.is_empty(), "no child overlaps checks or delivery");
            }
            Seen::WorkspaceReturned { owner } => {
                judge.check(
                    self.holder == Some(Holder::Workspace(owner)),
                    "the workspace holder stays through its own return",
                );
                self.holder = None;
            }
            Seen::Answer => {
                judge.check(self.holder.is_none() && self.children.is_empty(), "no holder or child outlives the run");
                self.answered = true;
            }
        }
    }
}

/// Judge ordered actual observations separately for each admitted run.
#[must_use]
pub fn judge(seen: &[(Time, Token, Seen)]) -> Vec<Verdict> {
    let mut meetings = BTreeMap::new();
    for (at, run, seen) in seen {
        let meeting = meetings.entry(*run).or_insert_with(|| Referee::new(Meeting::default()));
        meeting.observe(*at, *seen, &mut Vec::new());
    }
    meetings.values().map(Referee::verdict).collect()
}
