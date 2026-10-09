//! Local check notices (domain/host.md, sections 4 and 8). State counts all
//! anonymous outstanding spans and remembers whether checks were announced.
//! It never knows which overlapping operation a terminal ended; only the
//! last terminal proves every check finished. The parent face admits bounds.

use smith_host_domain::Operation;

#[derive(Debug)]
pub(crate) struct Long {
    open: u32,
    checks_pending: bool,
}

impl Long {
    pub(crate) const fn new() -> Long {
        Long { open: 0, checks_pending: false }
    }

    pub(crate) fn begin(&mut self, operation: Operation) -> bool {
        self.open = self.open.checked_add(1).expect("bounded parent long operations");
        match operation {
            Operation::Checks => {
                self.checks_pending = true;
                true
            }
            Operation::Completion | Operation::Command => false,
        }
    }

    pub(crate) fn end(&mut self) -> bool {
        self.open = self.open.checked_sub(1).expect("parent terminal owns an open span");
        if self.open == 0 && self.checks_pending {
            self.checks_pending = false;
            true
        } else {
            false
        }
    }
}
