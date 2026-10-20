//! Check deadlines and root closure (domain/host.md, section 8).
use super::{Box, Checks, Queue, Time, kernel, local};

impl Checks {
    pub(crate) fn next_deadline(&self) -> Option<Time> {
        match &self.active {
            Some(active) => active.source.next_deadline(),
            None => None,
        }
    }

    pub(crate) fn work_pending(&self, now: Time, closing: bool) -> bool {
        self.interrupted > 0
            || (self.active.is_none() && !self.waiting.is_empty())
            || match self.next_deadline() {
                Some(at) => at <= now,
                None => false,
            }
            || (closing && self.active.is_none() && self.root.is_some())
    }

    pub(crate) fn closing_root(&mut self) -> Option<kernel::Fd> {
        if self.active.is_none() && self.waiting.is_empty() { self.root.take() } else { None }
    }

    pub(crate) fn root_closed(&mut self) {
        self.root_closed = true;
    }

    pub(crate) fn closed(&self) -> bool {
        self.root_closed && self.active.is_none() && self.waiting.is_empty()
    }
}

impl crate::process::ProcessAdapter {
    pub(crate) fn program_owns(&self, owner: Option<super::Token>) -> bool {
        match &self.programs {
            Some(checks) => match owner {
                Some(owner) => checks.owns(owner),
                None => false,
            },
            None => false,
        }
    }

    pub(crate) fn handed_programs(&mut self, checks: Checks) {
        assert!(self.programs.is_none(), "program bindings are handed once");
        self.programs = Some(Box::new(checks));
    }

    pub(crate) fn program_request(&mut self, account: u32, now: Time, shell: &mut Queue<local::Request>) -> bool {
        match &mut self.programs {
            Some(checks) => checks.request(account, now, &mut self.io_requests, shell),
            None => false,
        }
    }

    pub(crate) fn cancel_programs(&mut self) {
        if let Some(checks) = &mut self.programs {
            checks.cancel(&mut self.io_requests);
        }
    }

    pub(crate) fn programs_pass(
        &mut self,
        now: Time,
        events: &mut Queue<local::Event>,
        shell: &mut Queue<local::Request>,
        notify: bool,
    ) {
        if let Some(checks) = &mut self.programs {
            checks.advance(now, &mut self.io_requests, events, shell, notify);
        }
    }
}
