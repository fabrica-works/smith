//! Real root command effects cross both framed halves into the real watchdog.
use super::*;

struct LongWorld {
    domain: agent::Domain,
    env: Env<agent::Limits>,
    out: Queue<AgentRequest>,
    wire: World,
    host: smith_host_world::World,
    run: Option<Token>,
    completion: Option<Token>,
    command: Option<Token>,
    cancelled: bool,
    notices: Vec<Option<agent::Operation>>,
}

impl LongWorld {
    fn new() -> Self {
        let limits = smith_agent_world::LIMITS;
        let mut bounds = smith_host_world::limits();
        bounds.turn_bytes = agent::max_turn_bytes(&limits).expect("declared turn envelope");
        bounds.unacknowledged_bytes = bounds.turn_bytes;
        let mut host = smith_host_world::World::new(97, bounds);
        host.live();
        let mut wire = World::new(CEILINGS, CEILINGS, StreamMode::Two);
        wire.settle();
        wire.send_start(Box::from(
            &include_bytes!("../../../../crates/smith-charter/golden/v4/record_charter_smallest.bin")[..],
        ));
        wire.settle();
        Self {
            domain: agent::Domain::new(&limits, configuration(), 97),
            env: Env { now: Time::ZERO, wall: Wall::EPOCH, limits },
            out: Queue::with_capacity(agent::max_out(&limits).checked_mul(4).expect("fixture output room")),
            wire,
            host,
            run: None,
            completion: None,
            command: None,
            cancelled: false,
            notices: Vec::new(),
        }
    }

    fn step(&mut self, event: AgentEvent) {
        while self.domain.pop_fact().is_some() {}
        agent::step(&mut self.domain, &self.env, event, &mut self.out);
        self.outputs();
    }

    fn ready(&mut self) {
        for _ in 0..100 {
            self.domain.reclaim();
            if !self.domain.is_ready() {
                return;
            }
            while self.domain.pop_fact().is_some() {}
            agent::resume(&mut self.domain, &self.env, &mut self.out);
            self.outputs();
        }
        panic!("bounded ready handoffs settle");
    }

    #[expect(clippy::wildcard_enum_match_arm, reason = "the composed script rejects every unexpected root effect")]
    fn outputs(&mut self) {
        while let Some(request) = self.out.pop() {
            match request {
                AgentRequest::Admitted { run, .. } => {
                    self.run = Some(run);
                    self.wire.agent_admits();
                    self.wire.settle();
                }
                AgentRequest::Read { owner, .. } => {
                    self.step(AgentEvent::Read { owner, read: agent::run::Read::Missing });
                }
                AgentRequest::Probe { owner, .. } => {
                    self.step(AgentEvent::Probed { owner, executable: false });
                }
                AgentRequest::Long { operation, until, .. } => {
                    let span = until.saturating_since(self.env.now);
                    self.notices.push(Some(operation));
                    self.wire.agent_starts_long(operation, span);
                    self.wire.settle();
                    assert!(self.wire.observations().contains(&Observation::HostLong { operation, span }));
                    self.host.up(Up::Long { operation, span });
                }
                AgentRequest::LongDone { .. } => {
                    self.notices.push(None);
                    self.wire.agent_ends_long();
                    self.wire.settle();
                    self.host.up(Up::LongDone);
                }
                AgentRequest::Complete { owner, .. } => self.completion = Some(owner),
                AgentRequest::Io { owner, op: agent::tools::Op::Spawn { .. }, .. } => self.command = Some(owner),
                AgentRequest::Job { .. } => unreachable!("the tools issue no job act yet"),
                AgentRequest::CancelIo { owner } => {
                    assert_eq!(Some(owner), self.command);
                    self.cancelled = true;
                }
                AgentRequest::Turn { number, turn, read, spent, opens_window, .. } => {
                    self.wire.agent_tells_turn(number, read, &turn);
                    self.wire.settle();
                    self.host.up(Up::Turn {
                        turn: host::Turn {
                            number,
                            read,
                            spent: spent.units,
                            opens_window,
                            body: host::TurnValue::new(turn, self.host.stage.env.limits.turn_bytes)
                                .expect("actual turn fits declared credit"),
                        },
                    });
                    self.host.event(host::Input::Parent(host::parent::Event::Acknowledge {
                        agent: self.host.agent(),
                        turn: number,
                    }));
                    self.host.sent();
                    self.wire.host_acknowledges(number);
                    self.wire.settle();
                    self.step(AgentEvent::Acknowledge { run: self.run.expect("admitted root"), turn: number });
                }
                AgentRequest::Answer { answer, read, .. } => {
                    self.wire.agent_answers_with_fence(answer, read);
                    self.wire.settle();
                    let answer = self.wire.take_host_answer().expect("actual answer crosses the framed halves");
                    self.host.up(Up::Answer { answer });
                }
                other => panic!("unexpected command story request: {other:?}"),
            }
        }
    }
}

#[test]
fn an_actual_command_crosses_the_channel_and_outlasts_no_progress_until_its_terminal() {
    let mut world = LongWorld::new();
    let mut charter = composed_charter().into_value();
    charter.grants.tools.shell = true;
    world.step(AgentEvent::Start {
        messages: Box::default(),
        identity: None,
        activation: 1,
        host_run: Token::new(7),
        reply_to: ReplyTo::new(Token::new(1)),
        window: agent::Window { turns: 100, bytes: u64::MAX },
        charter,
        transcript: None,
        answered: Box::default(),
        workspace: Some(agent::run::Workspace {
            directories: Box::from([agent::run::Directory {
                host: Box::from(b"/host".as_slice()),
                name: b"work".as_slice().into(),
                root: Token::new(900),
                writable: true,
                git: true,
                conflicts: Box::default(),
            }]),
        }),
        grants: Box::from([agent::Grant {
            name: agent::GrantName { account: 0, generation: 1 },
            renewable: true,
            valid: Duration::from_secs(7200),
        }]),
    });
    let owner = world.completion.take().expect("actual provider request");
    world.step(AgentEvent::Completed {
        owner,
        completion: agent::llm::Completion {
            reasoning_dropped: Box::default(),
            stop: agent::llm::Stop::ToolUse,
            usage: agent::llm::Usage::NONE,
            content: Box::from([agent::llm::Said::ToolCall {
                id: b"command".as_slice().into(),
                name: b"shell".as_slice().into(),
                input: br#"{"command":"sleep 12"}"#.as_slice().into(),
                replay: None,
                call: agent::llm::Decoded::Owned {
                    call: agent::tools::Call::Shell {
                        after: agent::tools::Yield::Never,
                        command: b"sleep 12".as_slice().into(),
                        timeout: agent::tools::Timeout::Asked(Duration::from_secs(30)),
                        output: agent::tools::Budget::Default,
                    },
                },
            }]),
        },
    });
    let command = world.command.expect("actual command IO was issued");
    assert_eq!(world.notices, [Some(agent::Operation::Completion), None, Some(agent::Operation::Command)]);
    world.env.now = Time::ZERO.saturating_add(Duration::from_secs(11));
    world.host.at(11);
    assert_eq!(world.host.seen.fault, None, "actual command stretches the ten-second no-progress watchdog");
    world.step(AgentEvent::Cancel { run: world.run.expect("actual admitted run") });
    world.ready();
    assert!(world.cancelled);
    assert_eq!(world.notices.len(), 3, "a cancel request is not the command terminal");
    world.step(AgentEvent::Done { owner: command, done: agent::tools::Done::Cancelled });
    world.ready();
    assert_eq!(world.notices, [Some(agent::Operation::Completion), None, Some(agent::Operation::Command), None]);
    assert!(matches!(
        world.host.seen.answer.as_ref().map(|answer| &answer.result),
        Some(host::RunResult::Failed { failure: host::RunFailure::Cancelled })
    ));
    world.host.cleanup(Some(smith_host_domain::Exit::Code(0)));
    world.host.settled();
    world.domain.reclaim();
    assert_eq!((world.domain.peers(), world.domain.flights(), world.domain.tickets()), (0, 0, 0));
}
