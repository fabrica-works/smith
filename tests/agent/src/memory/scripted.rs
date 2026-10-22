//! Scripted entries through the same memory driver as the random sweep.

use super::{Asked, Domain, Driver, Duration, Env, Event, Limits, Meter, Queue, Request, Rng, Time, Token, Wall};
use smith_domain::llm::{Asker, Completion, Purpose};
use smith_domain::{Config, max_out, resume, step};

/// An agent domain measured after its requests are drained and garbage reclaimed.
pub struct Memory {
    driver: Driver,
    domain: Domain,
    env: Env<Limits>,
    out: Queue<Request>,
    meter: Meter,
}

impl Memory {
    /// Allocate the driver's containers before measuring the domain.
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        let driver = Driver {
            rng: Rng::new(5),
            asked: Vec::with_capacity(4096),
            runs: Vec::with_capacity(64),
            workers: 0,
            running: None,
            seen: [0; 18],
        };
        let out = Queue::with_capacity(max_out(&limits));
        let meter = Meter::new();
        let domain = Domain::new(
            &limits,
            Config {
                programs: Box::new([]),
                endpoints: Box::new([super::Endpoint(0)]),
                models: Box::new([smith_domain::ConfiguredModel {
                    efforts: Box::new([]),
                    prices: smith_domain::run::Prices {
                        input: 1,
                        cached: 1,
                        cache_write: None,
                        output: 1,
                        unit: 1_000_000,
                    },
                    protocol_allowance: 0,
                    endpoint: super::Endpoint(0),
                    model: b"m".as_slice().into(),
                    window: 272_000,
                    output: 32_000,
                    head: Duration::from_secs(60),
                    idle: Duration::from_secs(300),
                }]),
            },
            5,
        );
        Self { driver, domain, env: Env { now: Time::ZERO, wall: Wall::EPOCH, limits }, out, meter }
    }

    /// Open the scripted main conversation with the profile's budget.
    pub fn start(&mut self) {
        let mut charter = super::charter(1024);
        charter.llm.window = 272_000;
        charter.llm.output = 32_000;
        charter.budget = self.env.limits.run.budget;
        charter.conventions = None;
        charter.reserve.time = self.env.limits.run.check_timeout;
        self.step(Event::Start {
            messages: Box::new([]),
            answered: Box::new([]),
            workspace: Some(super::workspace()),
            grants: Box::new([smith_domain::Grant {
                renewable: true,
                name: smith_domain::GrantName { account: 0, generation: 0 },
                valid: Duration::from_secs(100_000),
            }]),
            reply_to: skein_lib::ReplyTo::new(Token::new(1)),
            host_run: Token::new(1),
            identity: None,
            activation: 1,
            window: smith_domain::Window { turns: u32::MAX, bytes: u64::MAX },
            charter,
            transcript: None,
        });
        self.settle();
    }

    /// Deliver a boundary event, then drain and reclaim without resuming.
    pub fn step(&mut self, event: Event) {
        step(&mut self.domain, &self.env, event, &mut self.out);
        self.reclaim();
    }

    /// Run the ready domain until its next boundary request.
    pub fn settle(&mut self) {
        for _ in 0..1000 {
            if !self.domain.is_ready() {
                let preparation = self
                    .driver
                    .asked
                    .iter()
                    .position(|asked| matches!(asked, Asked::Read { .. } | Asked::Probe { .. }));
                if let Some(at) = preparation {
                    let event = match self.driver.asked.swap_remove(at) {
                        Asked::Read { owner } => Event::Read { owner, read: smith_domain::run::Read::Missing },
                        Asked::Probe { owner } => Event::Probed { owner, executable: false },
                        Asked::Complete { .. }
                        | Asked::Io { .. }
                        | Asked::Check { .. }
                        | Asked::Delivery { .. }
                        | Asked::Job { .. } => {
                            unreachable!("selected preparation")
                        }
                    };
                    self.step(event);
                    continue;
                }
                break;
            }
            resume(&mut self.domain, &self.env, &mut self.out);
            self.reclaim();
        }
        assert!(!self.domain.is_ready(), "script settles without advancing time");
    }

    fn reclaim(&mut self) {
        self.driver.drain(&mut self.out);
        while self.domain.pop_fact().is_some() {}
        while self.domain.pop_content().is_some() {}
        self.domain.reclaim();
    }

    /// Current domain-owned heap, with request payloads dropped.
    #[must_use]
    pub fn held(&self) -> u64 {
        self.meter.held()
    }

    /// Take the one completion of the named conversation and purpose.
    pub fn completion(&mut self, asker: Asker, compact: bool) -> Token {
        let at = self.driver.asked.iter().position(|asked| matches!(asked, Asked::Complete { asker: who, purpose, .. } if *who == asker && matches!(purpose, Purpose::Compaction { .. }) == compact)).expect("the scripted completion was requested");
        let Asked::Complete { owner, .. } = self.driver.asked.swap_remove(at) else {
            unreachable!("selected a completion")
        };
        owner
    }

    /// Take the next concrete IO request.
    pub fn io(&mut self) -> Token {
        let at = self
            .driver
            .asked
            .iter()
            .position(|asked| matches!(asked, Asked::Io { .. }))
            .expect("the scripted IO was requested");
        let Asked::Io { owner, .. } = self.driver.asked.swap_remove(at) else { unreachable!("selected IO") };
        owner
    }

    /// The running job number received by the model in its last prompt.
    pub fn running_job(&mut self) -> smith_domain_tools::JobNumber {
        self.driver.running.take().expect("a running shell result was sent")
    }

    /// Take the next read or stop of a held command.
    pub fn job(&mut self) -> (Token, smith_domain_tools::JobAct) {
        let at = self
            .driver
            .asked
            .iter()
            .position(|asked| matches!(asked, Asked::Job { .. }))
            .expect("a job act was requested");
        let Asked::Job { owner, act } = self.driver.asked.swap_remove(at) else { unreachable!("selected a job act") };
        (owner, act)
    }

    /// Answer a completion and run to its next request.
    pub fn answer(&mut self, owner: Token, completion: Completion) {
        self.step(Event::Completed { owner, completion });
        self.settle();
    }
}
