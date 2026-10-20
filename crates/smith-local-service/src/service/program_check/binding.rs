//! Check admission and pricing (protocol/programs.md, sections 3 and 4.2).
use super::{Account, Checks};
use crate::{Error, Service};
use alloc::boxed::Box;
use skein_io::kernel;
use skein_lib::{Duration, List, Queue};
use smith_host_domain::SourceName;
use smith_protocol_program::{ENVIRONMENT_BYTES, ENVIRONMENT_ENTRIES, Programs};

/// One serial check, bounded names, environment and queued requests, without I/O.
/// Paths use the program configuration's 4096-byte bound (protocol/agent.md, section 4).
#[must_use]
pub fn program_checks_worst_case(accounts: u32, detail_bytes: u32) -> Option<u64> {
    smith_host_protocol::program_source_worst_case(detail_bytes)?
        .checked_add(u64::try_from(size_of::<Checks>()).ok()?)?
        .checked_add(List::<Account>::worst_case(accounts)?)?
        .checked_add(Queue::<u32>::worst_case(accounts)?)?
        .checked_add(u64::try_from(ENVIRONMENT_BYTES).ok()?.checked_mul(3)?)?
        .checked_add(List::<Box<[u8]>>::worst_case(ENVIRONMENT_ENTRIES)?.checked_mul(3)?)?
        .checked_add(4096_u64.checked_mul(4)?)?
        .checked_add(List::<Box<[u8]>>::worst_case(2)?)?
        .checked_add(11)?
        .checked_add(u64::try_from(size_of::<kernel::Pipe>()).ok()?.checked_mul(2)?)?
        .checked_add(u64::from(accounts).checked_mul(u64::from(detail_bytes))?.checked_mul(8)?)
}

impl Service {
    /// Install checked program accounts and transfer their root before the first pass.
    /// Startup covers the first request; later checks obey domain/host.md, section 8.
    pub fn handed_programs(
        &mut self,
        sources: List<(u32, SourceName)>,
        programs: Programs,
        base: Box<[Box<[u8]>]>,
        root: kernel::Fd,
        detail_bytes: u32,
        duration: Duration,
    ) -> Result<(), Error> {
        if sources.is_empty()
            || sources.len() > self.limits.local.smith.accounts
            || programs.directory.len() > 4096
            || programs.claude.len() > 4096
        {
            return Err(Error::Memory);
        }
        let mut accounts = List::with_capacity(sources.len());
        for (number, source) in &sources {
            let Ok(configuration) = smith_protocol_program::configuration_directory(&programs, source.as_bytes())
            else {
                return Err(Error::Process);
            };
            match smith_protocol_program::environment(&base, &configuration) {
                Ok(_) => {}
                Err(_) => return Err(Error::Process),
            }
            accounts.push(Account { number: *number, source: *source, covered: true }).expect("admitted account count");
        }
        self.below.process_mut().handed_programs(Checks {
            accounts,
            programs,
            base,
            root: Some(root),
            root_closed: false,
            detail_bytes,
            duration,
            // One live request per account; stale requests are refused before enqueueing.
            // Cancelled waiters drain before routing new requests. The active check
            // may still be stale and settling, so all N accounts can be waiting.
            waiting: Queue::with_capacity(self.limits.local.smith.accounts),
            next_owner: 1_u64 << 60,
            active: None,
            interrupted: 0,
        });
        Ok(())
    }
}
