//! A sample host service supervising the real Smith agent service over Skein's
//! simulated child pipes (protocol/hosts.md, sections 3 and 7). The parent
//! keeps domain and process state; Skein's harness owns scheduling and child
//! lifetime, beside the scenario's fake machine and independent LLM peer. The host routing below is the part another host copies.

use skein_io::{self as io, kernel};
use skein_lib::{Duration, Env, Queue, Time, Token, Wall};
use smith_agent_process_world as agent_fixture;
use smith_host_domain as host;
use smith_host_protocol as protocol;
use std::os::unix::ffi::OsStrExt;

pub mod program_world;
pub mod referee;

mod world;

pub use world::World;

/// Which child program the scripted machine supplies for the host's spawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program {
    /// A second simulated process running the actual agent service.
    Service,
    /// The machine refuses to start the requested agent.
    Refused,
    /// A child writes an error and never finishes its channel opening.
    ErrorTail,
    /// A child starts but never speaks the channel opening.
    Silent,
}

/// Boundary observations retained by the sample host's parent.
#[derive(Debug, Default)]
pub struct Seen {
    pub started: bool,
    pub admitted: bool,
    pub answered: Option<host::Answer>,
    pub gone: Option<host::End>,
    pub detail: Option<Box<[u8]>>,
    pub turns: u32,
    /// Owner's pieces with the number of domain turns already told at arrival.
    pub pieces: Vec<(u32, smith_channel::Live)>,
    pub calls: u32,
    pub exits: u32,
    pub reaps: u32,
    pub signals: Vec<host::Signal>,
}

/// Boundary observations for an independent host-process referee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    /// The host domain accepted a run start.
    Started,
    /// The channel's last answer reached the parent.
    Answered,
    /// A named host call reached the parent.
    Called(Token),
    /// The parent supplied one terminal for a host call.
    CallAnswered(Token),
    /// The child process exited.
    Exited,
    /// The child and its pipes were reaped.
    Reaped,
    /// Spawn or opening failed and the process resources settled.
    Unspawned,
    /// The domain released the run slot.
    Gone,
}

/// The copied host composition: domain, process adapter, io and their queues.
#[expect(missing_debug_implementations, reason = "world service holds a non-Debug simulated kernel queue")]
#[expect(clippy::struct_excessive_bools, reason = "independent referee requests and startup-root close flags")]
pub struct HostService {
    domain: host::Domain,
    domain_env: Env<host::Limits>,
    io: io::Io,
    io_env: Env<io::Limits>,
    process: Option<protocol::Process>,
    domain_owner: Option<Token>,
    process_limits: protocol::Limits,
    root: kernel::Fd,
    launch: Option<smith_local_service::Launch>,
    domain_events: Queue<host::Input>,
    domain_requests: Queue<host::Output>,
    process_events: Queue<protocol::ProcessEvent>,
    io_events: Queue<io::Event>,
    io_requests: Queue<io::Request>,
    submissions: Queue<kernel::Submit>,
    completions: Queue<kernel::Complete>,
    seen: Seen,
    observations: Vec<Observation>,
    root_closing: bool,
    root_closed: bool,
    stop_requested: bool,
    crash_requested: bool,
    crashed: bool,
}

fn host_limits() -> host::Limits {
    let mut profile = smith_agent_service::profile::standard();
    profile.declared.conversations = 2;
    profile.declared.calls_per_response = 8;
    profile.declared.memory = 2_147_483_648;
    profile.policy.inbox = 8;
    profile.policy.unacknowledged = 8;
    let mut limits =
        smith_local_service::profile::host_limits(&profile, &agent_fixture::limits()).expect("host limits");
    limits.accounts = 1;
    limits.charter_bytes = 65_536;
    limits.transcript_bytes = 65_536;
    limits.answered_bytes = 65_536;
    limits.turns = 8;
    limits.turn_bytes = 65_536;
    limits.unacknowledged_bytes = 1_000_000_000;
    limits.outcome_bytes = 4096;
    limits.spawn_timeout = Duration::from_secs(2);
    limits.no_progress = Duration::from_secs(10);
    limits.cancel_grace = Duration::from_secs(5);
    limits.kill_after = Duration::from_secs(1);
    limits
}

fn io_limits() -> io::Limits {
    io::Limits {
        sockets: 8,
        names: 0,
        refusals: 1,
        intake: 4096,
        receive: 4096,
        output: 1_000_000,
        sends: 8,
        accepts: 1,
        backlog: 2,
        close_timeout: Duration::from_secs(1),
        retry: Duration::from_millis(1),
    }
}

fn start(live: Option<host::channel::LiveAsk>) -> host::Start {
    host::Start {
        live,
        messages: Box::default(),
        logical_run: Token::new(1),
        identity: None,
        activation: 1,
        workspace: None,
        charter: host::Charter::new(
            smith_protocol_channel::decode_charter(
                &agent_fixture::charter(),
                &smith_charter::CEILINGS,
                &agent_fixture::configuration().channel_endpoints,
            )
            .expect("typed fixture charter"),
            host_limits().charter_bytes,
        )
        .expect("host charter bound"),
        transcript: None,
        answered: Box::new([]),
        directories: Box::new([]),
        grants: Box::new([host::Grant {
            account: 0,
            generation: 1,
            valid: Duration::from_secs(7200),
            form: smith_host_domain::Form::Withdrawn,
        }]),
    }
}

impl HostService {
    /// Construct one host slot and queue a parent Spawn.
    #[must_use]
    pub fn new(
        launch: smith_local_service::Launch,
        now: Time,
        wall: Wall,
        detail_bytes: u32,
        live: Option<host::channel::LiveAsk>,
    ) -> HostService {
        let mut domain_limits = host_limits();
        domain_limits.detail_bytes = detail_bytes;
        let io_limits = io_limits();
        let agent_limits = agent_fixture::limits();
        let mut service = HostService {
            domain: host::Domain::new(&domain_limits),
            domain_env: Env { now, wall, limits: domain_limits },
            io: io::Io::new(&io_limits),
            io_env: Env { now, wall, limits: io_limits },
            process: None,
            domain_owner: None,
            process_limits: protocol::Limits {
                bodies: agent_limits.channel.bodies,
                charter: agent_limits.channel.charter,
                transcript: agent_limits.channel.transcript,
                endpoints: agent_limits.channel.endpoints,
                channel: agent_limits.channel.channel,
                start_bytes: agent_limits.channel.start_bytes,
                calls: domain_limits.calls,
            },
            root: launch.root,
            launch: Some(launch),
            domain_events: Queue::with_capacity(256),
            domain_requests: Queue::with_capacity(256),
            process_events: Queue::with_capacity(256),
            io_events: Queue::with_capacity(256),
            io_requests: Queue::with_capacity(256),
            submissions: Queue::with_capacity(256),
            completions: Queue::with_capacity(256),
            seen: Seen::default(),
            observations: Vec::with_capacity(256),
            root_closing: false,
            root_closed: false,
            stop_requested: false,
            crash_requested: false,
            crashed: false,
        };
        service.process_limits.channel.output_bytes =
            protocol::output_bytes(&service.process_limits).expect("host directional frame");
        service
            .domain_events
            .push(host::Input::Parent(host::parent::Event::Spawn { client: Token::new(1), start: start(live) }));
        service
    }

    /// One up pass and one down pass; the world supplies the injected clocks.
    pub fn iterate(&mut self, now: Time, wall: Wall) {
        if self.stop_requested {
            self.stop_requested = false;
            self.stop();
        }
        if self.crash_requested {
            self.crash_requested = false;
            self.crashed = true;
            self.process.as_ref().expect("started child").signal(kernel::Signal::Kill, &mut self.io_requests);
        }
        self.domain_env.now = now;
        self.domain_env.wall = wall;
        self.io_env.now = now;
        self.io_env.wall = wall;
        for _ in 0..self.completions.capacity() {
            let Some(complete) = self.completions.pop() else { break };
            if complete.op == Token::new(u64::MAX) {
                assert!(complete.result.is_ok(), "parent root closes");
                self.root_closed = true;
            } else {
                io::up(&mut self.io, &self.io_env, complete, &mut self.io_events, &mut self.submissions);
            }
        }
        for _ in 0..self.io_env.limits.sockets {
            if self.io.is_ready() {
                io::resume(&mut self.io, &self.io_env, &mut self.io_events, &mut self.submissions);
            }
            if self.io.is_due(now) {
                io::fire(&mut self.io, &self.io_env, &mut self.io_events, &mut self.submissions);
            }
        }
        for _ in 0..self.io_events.capacity() {
            let Some(event) = self.io_events.pop() else { break };
            self.process.as_mut().expect("spawn request preceded io event").from_io(
                now,
                event,
                &mut self.process_events,
                &mut self.io_requests,
            );
        }
        if let Some(process) = &mut self.process {
            process.fire(now, &mut self.process_events, &mut self.io_requests);
        }
        for _ in 0..self.process_events.capacity() {
            let Some(event) = self.process_events.pop() else { break };
            self.route_process(event);
        }
        for _ in 0..self.domain_env.limits.agents.saturating_mul(3) {
            if self.domain.is_due(now) {
                host::fire(&mut self.domain, &self.domain_env, &mut self.domain_requests);
            }
        }
        for _ in 0..self.domain_events.capacity() {
            let Some(event) = self.domain_events.pop() else { break };
            host::step(&mut self.domain, &self.domain_env, event, &mut self.domain_requests);
        }
        for _ in 0..self.domain_requests.capacity() {
            let Some(request) = self.domain_requests.pop() else { break };
            self.route_domain(request);
        }
        if let Some(process) = &mut self.process {
            for _ in 0..16_u32 {
                let Some(request) = process.next_channel_down() else { break };
                process.channel_down(request, &mut self.io_requests);
            }
        }
        for _ in 0..self.io_requests.capacity() {
            let Some(request) = self.io_requests.pop() else { break };
            io::down(&mut self.io, &self.io_env, request, &mut self.submissions);
        }
        if self.seen.gone.is_some() && !self.root_closing {
            self.root_closing = true;
            self.submissions
                .push(kernel::Submit { op: Token::new(u64::MAX), kind: kernel::Op::Close { fd: self.root } });
        }
        self.io.reclaim();
        self.domain.reclaim();
    }

    fn route_process(&mut self, event: protocol::ProcessEvent) {
        match event {
            protocol::ProcessEvent::Spawned { agent, process } => {
                self.domain_events.push(host::Input::Process(host::process::Event::Spawned { owner: agent, process }));
            }
            protocol::ProcessEvent::Unspawned { agent, detail } => {
                self.observations.push(Observation::Unspawned);
                self.domain_events.push(host::Input::Process(host::process::Event::Unspawned { owner: agent, detail }));
            }
            protocol::ProcessEvent::Exited { agent, exit } => {
                self.observations.push(Observation::Exited);
                self.seen.exits = self.seen.exits.checked_add(1).expect("bounded observed exits");
                self.domain_events.push(host::Input::Process(host::process::Event::Exited {
                    owner: agent,
                    exit: smith_local_service::to_host_exit(exit),
                }));
            }
            protocol::ProcessEvent::Reaped { agent, detail } => {
                self.observations.push(Observation::Reaped);
                self.seen.reaps = self.seen.reaps.checked_add(1).expect("bounded observed reaps");
                self.domain_events.push(host::Input::Process(host::process::Event::Reaped { owner: agent, detail }));
            }
            protocol::ProcessEvent::Channel { agent, event } => {
                self.route_channel(agent, *event);
            }
        }
    }

    fn route_channel(&mut self, agent: Token, event: protocol::OpenEvent) {
        use protocol::OpenEvent;
        let event = match event {
            OpenEvent::MessageRefused { name, reason } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::MessageRefused { name, reason },
            }),
            // Opened is transport-only. A failed write already settles its
            // send via Unsent; the independent read still owns Hangup.
            OpenEvent::Opened { .. } | OpenEvent::WriteFailed => return,
            OpenEvent::Hangup { .. } => host::Input::Process(host::process::Event::Hangup { owner: agent }),
            OpenEvent::Sent { .. } => host::Input::Process(host::process::Event::Sent { owner: agent }),
            OpenEvent::Unsent { .. } => host::Input::Process(host::process::Event::Unsent { owner: agent }),
            OpenEvent::Answer { answer, .. } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Answer { answer },
            }),
            OpenEvent::Admitted => {
                host::Input::Process(host::process::Event::Received { owner: agent, message: host::Up::Admitted })
            }
            OpenEvent::Waiting { read } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Waiting { read },
            }),
            OpenEvent::Long { operation, span } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Long { operation, span },
            }),
            OpenEvent::LongDone => {
                host::Input::Process(host::process::Event::Received { owner: agent, message: host::Up::LongDone })
            }
            OpenEvent::Turn { turn } => {
                host::Input::Process(host::process::Event::Received { owner: agent, message: host::Up::Turn { turn } })
            }
            OpenEvent::Live { turn, retry, kind, offset, bytes, call } => {
                let piece = smith_channel::Live::new(
                    &smith_channel::CEILINGS,
                    smith_channel::LiveParts { turn, retry, kind, offset, bytes, call },
                )
                .expect("process already validated the live record");
                self.seen.pieces.push((self.seen.turns, piece));
                host::Input::Process(host::process::Event::Progress { owner: agent })
            }
            OpenEvent::Fact { body } => {
                host::Input::Process(host::process::Event::Received { owner: agent, message: host::Up::Fact { body } })
            }
            OpenEvent::Unauthorized { account, generation } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Unauthorized { account, generation },
            }),
            OpenEvent::Limited { account, reset } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Limited { account, reset },
            }),
            OpenEvent::Refused { account, code, reason } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Refused { account, code, reason },
            }),
            OpenEvent::Call { call, name, deadline, ask } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Call { call, name, deadline, ask },
            }),
            OpenEvent::Withdraw { call } => host::Input::Process(host::process::Event::Received {
                owner: agent,
                message: host::Up::Withdraw { call },
            }),
        };
        self.domain_events.push(event);
    }

    fn route_domain(&mut self, request: host::Output) {
        match request {
            host::Output::Process(host::process::Request::Spawn { owner, deadline, .. }) => {
                self.domain_owner = Some(owner);
                let launch = self.launch.take().expect("one agent launch");
                let document = std::path::Path::new(std::ffi::OsStr::from_bytes(
                    launch.arguments.first().expect("generated configuration argument"),
                ));
                let endpoints = smith_agent_shell::config::read(document)
                    .expect("generated agent configuration")
                    .service
                    .channel_endpoints;
                let mut process =
                    protocol::Process::new(owner, &self.process_limits, self.domain_env.limits.detail_bytes, endpoints)
                        .expect("bounded host process");
                process.spawn(
                    protocol::Launch {
                        program: launch.program,
                        arguments: launch.arguments,
                        environment: launch.environment,
                        root: launch.root,
                        directory: launch.directory,
                    },
                    deadline,
                    &mut self.io_requests,
                );
                self.process = Some(process);
            }
            host::Output::Parent(host::parent::Request::Started { .. }) => {
                self.seen.started = true;
                self.observations.push(Observation::Started);
            }
            host::Output::Parent(host::parent::Request::Admitted { .. }) => self.seen.admitted = true,
            host::Output::Parent(host::parent::Request::Answered { answer, .. }) => {
                self.seen.answered = Some(answer);
                self.observations.push(Observation::Answered);
            }
            host::Output::Parent(host::parent::Request::Gone { end, detail, .. }) => {
                self.seen.gone = Some(end);
                self.seen.detail = Some(detail);
                self.observations.push(Observation::Gone);
            }
            host::Output::Parent(host::parent::Request::Turn { turn, .. }) => {
                self.seen.turns = self.seen.turns.checked_add(1).expect("bounded observed turns");
                self.domain_events.push(host::Input::Parent(host::parent::Event::Acknowledge {
                    agent: self.agent_owner(),
                    turn: turn.number,
                }));
            }
            host::Output::Parent(host::parent::Request::Called { call, .. }) => {
                self.seen.calls = self.seen.calls.checked_add(1).expect("bounded observed calls");
                self.observations.push(Observation::Called(call));
                self.domain_events.push(host::Input::Parent(host::parent::Event::Answer {
                    agent: self.agent_owner(),
                    call,
                    reply: host::Reply::Unavailable,
                }));
            }
            host::Output::Process(host::process::Request::Send { message, .. }) => self.send(message),
            host::Output::Process(host::process::Request::Signal { signal, .. }) => {
                self.seen.signals.push(signal);
                let signal_below = match signal {
                    host::Signal::Terminate => kernel::Signal::Terminate,
                    host::Signal::Kill => kernel::Signal::Kill,
                };
                self.process.as_ref().expect("spawned child").signal(signal_below, &mut self.io_requests);
                self.domain_events
                    .push(host::Input::Process(host::process::Event::Signalled { owner: self.agent_owner() }));
            }
            host::Output::Process(host::process::Request::Read { .. }) => {
                self.process.as_mut().expect("spawned child").read(&mut self.process_events, &mut self.io_requests);
            }
            host::Output::Process(host::process::Request::Wait { .. } | host::process::Request::Reap { .. })
            | host::Output::Parent(
                host::parent::Request::Waiting { .. }
                | host::parent::Request::Long { .. }
                | host::parent::Request::LongDone { .. }
                | host::parent::Request::Unread { .. }
                | host::parent::Request::Withdrawn { .. }
                | host::parent::Request::Unauthorized { .. }
                | host::parent::Request::Refused { .. }
                | host::parent::Request::Limited { .. }
                | host::parent::Request::Told { .. }
                | host::parent::Request::Faulted { .. }
                | host::parent::Request::MessageRefused { .. },
            ) => {}
        }
    }

    fn send(&mut self, message: host::Down) {
        let process = self.process.as_mut().expect("spawned child");
        let token = Token::new(99);
        let answering = match &message {
            host::Down::Answer { call, .. } => Some(*call),
            host::Down::Start { .. }
            | host::Down::Message { .. }
            | host::Down::Acknowledge { .. }
            | host::Down::Grant { .. }
            | host::Down::Cancel => None,
        };
        let sent = match message {
            host::Down::Start { start, window } => process.send_start(
                start,
                window,
                protocol::Values {
                    paths: Box::new([]),
                    credentials: Box::new([smith_protocol_channel::GrantValue::Withdrawn {
                        account_id: Box::from(&b"acc"[..]),
                        bearer: Box::from(&b"token"[..]),
                    }]),
                },
                token,
                &mut self.process_events,
                &mut self.io_requests,
            ),
            host::Down::Message { finish_requested, name, label, text } => process.send_message(
                host::Message { finish_requested, name, label, text },
                token,
                &mut self.process_events,
                &mut self.io_requests,
            ),
            host::Down::Answer { call, reply } => {
                process.send_reply(call, reply, token, &mut self.process_events, &mut self.io_requests)
            }
            host::Down::Acknowledge { turn } => {
                process.send_acknowledge(turn, token, &mut self.process_events, &mut self.io_requests)
            }
            host::Down::Grant { grant } => process.send_grant(
                grant,
                smith_protocol_channel::GrantValue::Withdrawn {
                    account_id: Box::from(&b"acc"[..]),
                    bearer: Box::from(&b"token"[..]),
                },
                token,
                &mut self.process_events,
                &mut self.io_requests,
            ),
            host::Down::Cancel => process.send_cancel(token, &mut self.process_events, &mut self.io_requests),
        };
        if sent.is_err() {
            self.domain_events.push(host::Input::Process(host::process::Event::Unsent { owner: self.agent_owner() }));
        } else if let Some(call) = answering {
            self.observations.push(Observation::CallAnswered(call));
        }
    }

    fn agent_owner(&self) -> Token {
        self.domain_owner.expect("host domain admitted one agent")
    }

    #[must_use]
    pub fn next_deadline(&self) -> Option<Time> {
        [
            self.domain.next_deadline(),
            self.io.next_deadline(),
            self.process.as_ref().and_then(protocol::Process::next_deadline),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// The domain's and the process protocol's deadlines, excluding io's.
    #[must_use]
    pub fn next_policy_deadline(&self) -> Option<Time> {
        [self.domain.next_deadline(), self.process.as_ref().and_then(protocol::Process::next_deadline)]
            .into_iter()
            .flatten()
            .min()
    }

    #[must_use]
    pub fn work_pending(&self, now: Time) -> bool {
        self.io.is_ready()
            || self.io.is_due(now)
            || self.domain.is_due(now)
            || !self.io_events.is_empty()
            || !self.domain_events.is_empty()
            || !self.domain_requests.is_empty()
            || !self.process_events.is_empty()
            || !self.io_requests.is_empty()
            || !self.completions.is_empty()
    }

    pub fn completions(&mut self) -> &mut Queue<kernel::Complete> {
        &mut self.completions
    }

    /// The sample parent requests a run stop through the host domain.
    pub fn stop(&mut self) {
        self.domain_events.push(host::Input::Parent(host::parent::Event::Stop { agent: self.agent_owner() }));
    }

    pub fn submissions(&mut self) -> &mut Queue<kernel::Submit> {
        &mut self.submissions
    }

    #[must_use]
    pub fn seen(&self) -> &Seen {
        &self.seen
    }

    /// Boundary events in their order at the host-process interface.
    #[must_use]
    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }

    #[must_use]
    pub fn io_empty(&self) -> bool {
        self.io.is_empty()
    }
}
