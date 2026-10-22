//! Successive machine requests in one real activation (protocol/agent.md, 2).
//! It keeps only the fixture's outcome after every child and root has settled.

use super::{Application, Job, Terminal, World, copy_terminal, job_terminals};
use skein_lib::{Duration, Time};
use skein_world::Expectations;
use smith_protocol_machine::{FromDomain, ToDomain};

impl World {
    /// Exercise successive job acts in one activation on the real kernel.
    pub fn jobs(&mut self, requests: Vec<FromDomain>) -> Vec<ToDomain> {
        self.jobs_after(requests, Vec::new())
    }

    /// Exercise successive job acts in one activation on the real kernel.
    pub fn jobs_after(&mut self, mut requests: Vec<FromDomain>, delays: Vec<Duration>) -> Vec<ToDomain> {
        let owner = Job::Request(requests.first().expect("one spawn").clone(), false).owner();
        let path = std::env::temp_dir().join(format!("smith-machine-jobs-{}-{}", std::process::id(), self.seed));
        std::fs::create_dir(&path).expect("unique job workspace");
        let root = skein_shell::open_root(&path).expect("open job workspace");
        let clock = skein_shell::Clock::new();
        let base = clock.now().now;
        for request in &mut requests {
            match request {
                FromDomain::Op { deadline, .. }
                | FromDomain::Read { deadline, .. }
                | FromDomain::Probe { deadline, .. }
                | FromDomain::Check { deadline, .. } => {
                    *deadline = base.saturating_add(Duration::from_nanos(deadline.as_nanos()));
                }
                FromDomain::Job { .. } | FromDomain::Cancel { .. } | FromDomain::Abort { .. } => {}
            }
        }
        let referee = Expectations::new(
            self.seed,
            vec![Terminal {
                owner,
                expected: job_terminals(&requests),
                deadline: base.saturating_add(Duration::from_secs(4)),
            }],
        );
        let mut world = skein_world::real::World::new(referee);
        world.spawn_with_fds(vec![root], || {
            Application::new(
                self.seed,
                self.writable,
                Time::ZERO,
                root,
                Job::Sequence { requests, delays },
                self.limits,
                &self.environment,
            )
        });
        let outcome = world.run(&clock, Duration::from_secs(4));
        assert!(outcome.killed.is_empty(), "job activation settles without harness cleanup");
        assert!(outcome.procs[0].component.closed(), "machine reports closed after its retained children settle");
        self.now = Time::from_nanos(outcome.end.as_nanos().saturating_sub(outcome.start.as_nanos()));
        let answers = outcome.procs[0].terminals.iter().map(copy_terminal).collect();
        drop(outcome);
        std::fs::remove_dir(path).expect("job workspace is empty");
        answers
    }
}
