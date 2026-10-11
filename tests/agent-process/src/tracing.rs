//! A Smith-specific observer of the service's native local trace output.
//! Skein owns scheduling, kernel operations, settlement and replay. This
//! adapter adopts Smith's inherited streams and writes only emitted records;
//! it never reads the engine's decisions or manufactures an observation.

use skein_io::kernel::{self, Complete, Fd, Op, Submit};
use skein_lib::{List, Queue, Time, Token, Wall};
use skein_shell::Host;
use smith_agent_service::{self as agent, Service};

pub(super) struct Tracing {
    service: Service,
    roots: std::vec::IntoIter<Fd>,
    unused: Vec<Fd>,
    completions: Queue<Complete>,
    submissions: Queue<Submit>,
    closing: u32,
    result: Option<bool>,
    memory: u64,
    opening: Option<Fd>,
    next: u64,
    started: bool,
    pub(super) channel_loss: u64,
}

impl Tracing {
    pub(super) fn new(mut configuration: agent::Config, inherited: &skein_world::Inherited) -> Self {
        configuration.limits.events = Some(event_limits(&configuration.limits));
        let queue = configuration.limits.queue;
        let memory = agent::worst_case(&configuration.limits, &configuration.llm_endpoints)
            .expect("bounded actual service")
            .checked_add(u64::try_from(size_of::<Self>()).expect("observer cells"))
            .and_then(|bound| bound.checked_add(Queue::<Complete>::worst_case(queue)?))
            .and_then(|bound| bound.checked_add(Queue::<Submit>::worst_case(queue)?))
            .and_then(|bound| bound.checked_add(64))
            .expect("observer ownership");
        let mut service = Service::new(configuration, 7).expect("bounded observed service");
        let pipe = |child| inherited.pipes.iter().find(|(number, _)| *number == child).expect("inherited pipe").1;
        service.adopt_streams(pipe(0), pipe(1), inherited.signal).expect("service streams");
        let root = inherited.roots.iter().find(|(name, _)| name.as_ref() == b"event-trace").expect("trace root").1;
        let mut submissions = Queue::with_capacity(queue);
        submissions.push(Submit {
            op: Token::new(u64::MAX),
            kind: Op::Open {
                root,
                path: Box::from(b"events.jsonl".as_slice()),
                how: kernel::OpenHow::CreateNoFollow { mode: Some(0o600) },
            },
        });
        Self {
            service,
            roots: inherited
                .roots
                .iter()
                .filter(|(name, _)| name.as_ref() != b"event-trace")
                .map(|(_, root)| *root)
                .collect::<Vec<_>>()
                .into_iter(),
            unused: vec![pipe(2)],
            completions: Queue::with_capacity(queue),
            submissions,
            closing: 0,
            result: None,
            memory,
            opening: Some(root),
            next: u64::MAX - 1,
            started: false,
            channel_loss: 0,
        }
    }

    fn close(&mut self, fd: Fd) {
        self.closing += 1;
        self.submissions.push(Submit { op: Token::new(self.next), kind: Op::Close { fd } });
        self.next = self.next.checked_sub(1).expect("finite observer operations");
    }
}

impl Host for Tracing {
    fn iterate(&mut self, now: Time, wall: Wall) {
        while let Some(complete) = self.completions.pop() {
            if complete.op.raw() == u64::MAX {
                let Ok(kernel::Done::Fd(fd)) = complete.result else { panic!("trace file opens beneath its root") };
                self.service
                    .adopt_destination(fd, agent::Destination::Trace, agent::sinks::Delivery::Complete)
                    .expect("trace adopted");
                let root = self.opening.take().expect("one startup trace open");
                self.close(root);
            } else if complete.op.raw() >= u64::MAX - 8 {
                assert!(complete.result.is_ok(), "unused inherited close");
                self.closing -= 1;
            } else {
                self.service.completions().push(complete);
            }
        }
        if self.result.is_some() {
            return;
        }
        if self.opening.is_some() {
            return;
        }
        if !self.started {
            self.started = true;
            self.service.start_session(
                now,
                smith_events::SessionStarted {
                    wall_ms: wall.as_nanos() / 1_000_000,
                    agent: smith_events::Agent {
                        name: b"smith".as_slice().into(),
                        version: b"world".as_slice().into(),
                        build: b"deterministic".as_slice().into(),
                    },
                    mode: smith_events::Mode::Agent,
                    containment: smith_events::Containment::Groups,
                    pid: 101,
                    profile: b"standard".as_slice().into(),
                    capture: smith_events::Capture::Everything,
                    delivery: smith_events::Delivery::Complete,
                    versions: smith_events::Versions { events: 1, channel: 2, charter: 2, transcript: 3 },
                    limits: List::with_capacity(0),
                },
            );
        }
        agent::iterate(&mut self.service, now, wall);
        while self.service.pop_trace_content().is_some() {}
        while self.service.root_to_open().is_some() {
            let path = self.service.root_to_open().expect("pending root");
            let host =
                smith_protocol_channel::host_path(path, self.service.host_path_bytes()).expect("absolute fixture path");
            self.service.root_opened(self.roots.next().ok_or(()), host);
        }
        self.channel_loss = self.service.lost_channel_facts();
        if let Some(answered) = agent::done(&self.service) {
            self.result = Some(answered);
            let mut unused = std::mem::take(&mut self.unused);
            unused.extend(self.roots.by_ref());
            for fd in unused {
                self.close(fd);
            }
        }
        while let Some(submission) = self.service.submissions().pop() {
            self.submissions.push(submission);
        }
    }

    fn completions(&mut self) -> &mut Queue<Complete> {
        &mut self.completions
    }

    fn submissions(&mut self) -> &mut Queue<Submit> {
        &mut self.submissions
    }

    fn work_pending(&self, now: Time) -> bool {
        (self.result.is_none() && agent::work_pending(&self.service, now))
            || !self.completions.is_empty()
            || !self.submissions.is_empty()
    }

    fn next_deadline(&self) -> Option<Time> {
        if self.result.is_none() { agent::next_deadline(&self.service) } else { None }
    }

    fn is_empty(&self) -> bool {
        self.result.is_some() && self.closing == 0 && self.completions.is_empty() && self.submissions.is_empty()
    }

    fn exit(&self) -> Option<kernel::Exit> {
        if self.is_empty() { self.result.map(|answered| kernel::Exit::Code(u8::from(!answered))) } else { None }
    }

    fn worst_case(&self) -> u64 {
        self.memory
    }

    fn operations(&self) -> u32 {
        crate::limits().routes + 8
    }
}

/// Receiving bounds derived from the same actual producer configuration.
pub(super) fn event_limits(limits: &agent::Limits) -> agent::EventLimits {
    agent::EventLimits::derive(limits, agent::sinks::Capture::Everything, 1, 0, skein_lib::Duration::from_secs(1))
        .expect("world event bounds")
}
