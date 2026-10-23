//! Both real domains meet across the channel halves, with the process and
//! provider kept as scripted neighbours (protocol/channel.md, section 10).

use skein_channel::StreamMode;
use skein_lib::{Duration, Env, Queue, Reader, ReplyTo, Time, Token, Wall, Writer};
use smith_channel::CEILINGS;
use smith_channel_world::{Observation, World, initial_grant};
use smith_domain::{self as agent, Event as AgentEvent, Request as AgentRequest};
use smith_host_domain::{self as host, Down, Up};

fn synthetic_grant() -> smith_protocol_channel::GrantValue {
    smith_protocol_channel::GrantValue::Bearer(Box::from(b"synthetic".as_slice()))
}

#[path = "composed/long.rs"]
mod long;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one composed story follows both domains and both wire halves to final cleanup"
)]
#[expect(clippy::wildcard_enum_match_arm, reason = "the story rejects every request outside its scripted path")]
fn the_host_start_enters_the_agent_domain_and_its_answer_returns_to_the_host_domain() {
    let Composed { mut host_world, mut wire, mut domain, mut env, mut out } = composed_start();
    let mut run = None;
    let mut completion = None;
    let mut completion_span = None;
    while let Some(request) = out.pop() {
        match request {
            AgentRequest::Admitted { host_run, run: admitted } => {
                assert_eq!(host_run, Token::new(7));
                run = Some(admitted);
                wire.agent_admits();
            }
            AgentRequest::Long { host_run, operation: agent::Operation::Completion, until } => {
                assert_eq!(host_run, Token::new(7));
                completion_span = Some(until.saturating_since(env.now));
                wire.agent_starts_long(agent::Operation::Completion, until.saturating_since(env.now));
            }
            AgentRequest::Complete { owner, .. } => completion = Some(owner),
            other => panic!("unexpected initial agent request: {other:?}"),
        }
    }
    let run = run.expect("real agent domain admitted the decoded Start");
    let completion = completion.expect("scripted provider owes a terminal");
    wire.settle();
    assert!(wire.observations().contains(&Observation::HostAdmitted));
    host_world.up(Up::Admitted);
    assert!(host_world.seen.admitted);
    let span = completion_span.expect("actual issued completion announces its deadline");
    assert!(wire.observations().contains(&Observation::HostLong { operation: agent::Operation::Completion, span }));
    host_world.up(Up::Long { operation: agent::Operation::Completion, span });
    host_world.at(11);
    assert_eq!(host_world.seen.fault, None, "completion outlasts the ten-second no-progress bound");
    env.now = Time::ZERO.saturating_add(Duration::from_secs(11));

    // Send beyond the kit's ingress directly through the host channel half.
    // Every hostile-but-decodable input reaches the real root/run ingress.
    for (name, text, expected) in [
        (10, b"123456".as_slice(), None),
        (11, b"1234567".as_slice(), Some(agent::run::MessageRefusal::TooLarge)),
        (10, b"x".as_slice(), Some(agent::run::MessageRefusal::NameInUse)),
        (12, b"x".as_slice(), Some(agent::run::MessageRefusal::Full)),
    ] {
        let name = Token::new(name);
        wire.send_message(name, Box::from(*b"peer"), Box::from(text), false);
        wire.settle();
        let message = wire
            .observations()
            .iter()
            .rev()
            .find_map(|seen| match seen {
                Observation::AgentMessage { name: seen, label, text, .. } if *seen == name => {
                    Some((label.clone(), text.clone()))
                }
                _ => None,
            })
            .expect("channel decoded labelled message");
        while domain.pop_fact().is_some() {}
        agent::step(
            &mut domain,
            &env,
            AgentEvent::Message { finish_requested: false, run, name, label: message.0, text: message.1 },
            &mut out,
        );
        match expected {
            None => assert!(out.pop().is_none(), "exact-bound message admitted"),
            Some(expected) => {
                let AgentRequest::MessageRefused { name: refused, reason, .. } =
                    out.pop().expect("typed ingress terminal")
                else {
                    panic!("expected typed refusal")
                };
                assert_eq!((refused, reason), (name, expected));
                assert!(out.pop().is_none());
                wire.agent_refuses_message(name, reason);
                wire.settle();
                let reason = match reason {
                    agent::run::MessageRefusal::TooLarge => host::MessageRefusal::TooLarge,
                    agent::run::MessageRefusal::Full => host::MessageRefusal::Full,
                    agent::run::MessageRefusal::NameInUse => host::MessageRefusal::NameInUse,
                    agent::run::MessageRefusal::Ending => host::MessageRefusal::Ending,
                };
                assert!(wire.observations().contains(&Observation::HostMessageRefused { name, reason }));
            }
        }
    }
    host_world.event(host::Input::Parent(host::parent::Event::Stop { agent: host_world.agent() }));
    let down = host_world.seen.down.pop().expect("host domain cancelled live run");
    assert_eq!(down, Down::Cancel);
    wire.host_cancels();
    wire.settle();
    host_world.sent();
    assert!(wire.observations().contains(&Observation::AgentCancel));
    while domain.pop_fact().is_some() {}
    agent::step(&mut domain, &env, AgentEvent::Cancel { run }, &mut out);
    assert!(out.pop().is_none(), "cancel first enters the child handoff");
    domain.reclaim();
    assert!(domain.is_ready());
    while domain.pop_fact().is_some() {}
    agent::resume(&mut domain, &env, &mut out);
    assert_eq!(out.pop(), Some(AgentRequest::Cancel { owner: completion }));
    assert!(out.pop().is_none());
    while domain.pop_fact().is_some() {}
    agent::step(&mut domain, &env, AgentEvent::Cancelled { owner: completion }, &mut out);
    assert_eq!(out.pop(), Some(AgentRequest::LongDone { host_run: Token::new(7) }));
    wire.agent_ends_long();
    wire.settle();
    assert!(wire.observations().contains(&Observation::HostLongDone));
    host_world.up(Up::LongDone);
    let answer = match out.pop().expect("real agent domain answers after provider settlement") {
        AgentRequest::Answer { to, answer, read: _, detail: _ } => {
            assert_eq!(to, ReplyTo::new(Token::new(1)));
            answer
        }
        other => panic!("expected domain answer, got {other:?}"),
    };
    assert!(out.pop().is_none());
    assert!(matches!(answer, agent::run::Answer::Failed { failure: agent::run::Failure::Cancelled, turns: 0, .. }));
    wire.agent_answers(answer);
    wire.settle();
    let translated = wire.take_host_answer().expect("host protocol decoded final answer");
    assert!(matches!(translated.result, host::RunResult::Failed { failure: host::RunFailure::Cancelled }));
    host_world.up(Up::Answer { answer: translated });
    host_world.cleanup(Some(smith_host_domain::Exit::Code(0)));
    host_world.settled();
    assert!(matches!(
        host_world.seen.answer.as_ref().map(|answer| &answer.result),
        Some(host::RunResult::Failed { failure: host::RunFailure::Cancelled })
    ));
    assert_eq!(
        host_world.seen.gone,
        Some(host::End::Exited { exit: Some(smith_host_domain::Exit::Code(0)), forced: smith_host_domain::Forced::No })
    );
}

fn composed_charter() -> host::Charter {
    let smallest = include_bytes!("../../../crates/smith-charter/golden/v4/record_charter_smallest.bin");
    let source =
        smith_charter::Charter::decode(&smith_charter::CEILINGS, &mut Reader::new(smallest)).expect("golden charter");
    let mut parts = source.into_parts();
    parts.budget = smith_charter::Budget::new(
        &smith_charter::CEILINGS,
        smith_charter::BudgetParts {
            turns: 1,
            spend: 1,
            time: Duration::from_secs(60),
            reserve: smith_charter::Reserve::new(
                &smith_charter::CEILINGS,
                smith_charter::ReserveParts { turns: 0, time: Duration::ZERO },
            )
            .expect("bounded reserve"),
        },
    )
    .expect("workable budget");
    parts.contract = smith_charter::Contract::new(
        &smith_charter::CEILINGS,
        smith_charter::ContractParts {
            report: Some(
                smith_charter::TextRule::new(
                    &smith_charter::CEILINGS,
                    smith_charter::TextRuleParts { max: 128, fields: skein_lib::List::with_capacity(0) },
                )
                .expect("report rule"),
            ),
            verdicts: skein_lib::List::with_capacity(0),
            change: None,
            failure: None,
        },
    )
    .expect("outcome contract");
    let mut llm = parts.main.into_parts();
    llm.account = Some(0);
    llm.model = Box::from(*b"fake");
    llm.window = 8192;
    llm.output = 1024;
    llm.prices = smith_charter::Prices::new(
        &smith_charter::CEILINGS,
        smith_charter::PricesParts { input: 1, cached: 1, cache_write: None, output: 1, unit: 1_000_000 },
    )
    .expect("prices");
    parts.main = smith_charter::Llm::new(&smith_charter::CEILINGS, llm).expect("model");
    let record = smith_charter::Charter::new(&smith_charter::CEILINGS, parts).expect("valid charter record");
    let mut writer = Writer::new(usize::try_from(record.measure()).expect("small charter"));
    record.encode(&mut writer).expect("measured charter");
    smith_channel_world::typed_charter(&writer.finish())
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "the composed story selects its one provider boundary request")]
fn carried_start_messages_cross_the_wire_in_order_and_an_overfull_start_is_refused_atomically() {
    for count in [2_u64, 3] {
        let mut start = smith_host_world::start();
        start.charter = composed_charter();
        start.transcript = None;
        start.answered = Box::default();
        start.grants = Box::new([host::Grant {
            account: 0,
            generation: 1,
            valid: Duration::from_secs(7200),
            form: smith_host_domain::Form::Credential(smith_host_domain::Scheme::Bearer),
        }]);
        start.messages = (0..count)
            .map(|name| host::Message {
                finish_requested: name == 0,
                name: Token::new(99 - name),
                label: b"person".as_slice().into(),
                text: if name == 0 { b"first".as_slice().into() } else { b"second".as_slice().into() },
            })
            .collect();
        let mut wire = World::new(CEILINGS, CEILINGS, StreamMode::Two);
        wire.settle();
        wire.send_domain_start(
            start,
            host::channel::Window { turns: 100, bytes: 1_000_000_000 },
            smith_host_protocol::Values { paths: Box::default(), credentials: Box::new([synthetic_grant()]) },
        );
        wire.settle();
        let decoded = wire.take_agent_start().expect("carried Start decoded");
        assert_eq!(decoded.messages.len(), usize::try_from(count).expect("two or three messages"));
        assert_eq!(decoded.messages[0].name, Token::new(99));
        assert_eq!(decoded.messages[1].name, Token::new(98));
        assert!(decoded.messages[0].finish_requested);
        assert!(!decoded.messages[1].finish_requested);
        let mut limits = smith_agent_world::LIMITS;
        limits.run.messages = 2;
        let mut domain = agent::Domain::new(&limits, configuration(), 7);
        let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
        let mut out = Queue::with_capacity(agent::max_out(&limits));
        while domain.pop_fact().is_some() {}
        agent::step(
            &mut domain,
            &env,
            AgentEvent::Start {
                messages: decoded.messages,
                reply_to: ReplyTo::new(Token::new(1)),
                host_run: Token::new(7),
                identity: decoded.identity,
                activation: decoded.activation,
                window: decoded.window,
                charter: decoded.charter,
                workspace: None,
                transcript: decoded.transcript,
                answered: decoded.answered,
                grants: decoded
                    .grants
                    .iter()
                    .map(|grant| agent::Grant { renewable: true, name: grant.name, valid: grant.valid })
                    .collect(),
            },
            &mut out,
        );
        let requests: Vec<_> = core::iter::from_fn(|| out.pop()).collect();
        if count == 2 {
            assert!(requests.iter().any(|request| matches!(request, AgentRequest::Admitted { .. })));
            let prompt = requests
                .iter()
                .find_map(|request| match request {
                    AgentRequest::Complete { prompt, .. } => Some(prompt),
                    _ => None,
                })
                .expect("one real provider request");
            let texts: Vec<_> = prompt
                .messages
                .iter()
                .flat_map(|message| &message.content)
                .filter_map(|block| match block {
                    agent::llm::Block::Text { text, .. } => Some(text.as_ref()),
                    agent::llm::Block::Opaque { .. }
                    | agent::llm::Block::Refusal { .. }
                    | agent::llm::Block::ToolCall { .. }
                    | agent::llm::Block::ToolResult { .. } => None,
                })
                .collect();
            assert_eq!(texts, [b"person: first\n\nperson: second".as_slice()]);
        } else {
            assert!(matches!(
                requests.as_slice(),
                [AgentRequest::Answer {
                    answer: agent::run::Answer::Refused(agent::run::Refusal::Invalid(agent::run::Invalid::Messages)),
                    read: None,
                    ..
                }]
            ));
        }
    }
}

fn configuration() -> agent::Config {
    agent::Config {
        programs: Box::new([]),
        endpoints: Box::new([agent::run::charter::Endpoint(0)]),
        models: Box::new([smith_domain::ConfiguredModel {
            efforts: Box::new([]),
            prices: smith_domain::run::Prices { input: 1, cached: 1, cache_write: None, output: 1, unit: 1_000_000 },
            protocol_allowance: 0,
            endpoint: smith_domain::run::charter::Endpoint(0),
            model: Box::from(&b"fake"[..]),
            window: 8192,
            output: 4096,
            head: Duration::from_secs(60),
            idle: Duration::from_secs(30),
        }]),
    }
}

#[test]
#[expect(clippy::wildcard_enum_match_arm, reason = "the story rejects every request outside the awaiting path")]
fn an_empty_start_waits_without_a_fence_and_the_first_wire_message_opens_main() {
    let mut charter = composed_charter().into_value();
    charter.grants.wait = true;
    charter.waiting = Duration::from_secs(10);
    let mut start = smith_host_world::start();
    start.charter = host::Charter::new(charter, 1_000_000).expect("bounded typed charter");
    start.transcript = None;
    start.answered = Box::default();
    start.grants = Box::new([host::Grant {
        account: 0,
        generation: 1,
        valid: Duration::from_secs(7200),
        form: smith_host_domain::Form::Credential(smith_host_domain::Scheme::Bearer),
    }]);
    let mut wire = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    wire.settle();
    wire.send_domain_start(
        start,
        host::channel::Window { turns: 100, bytes: 1_000_000_000 },
        smith_host_protocol::Values { paths: Box::default(), credentials: Box::new([synthetic_grant()]) },
    );
    wire.settle();
    let decoded = wire.take_agent_start().expect("wire Start");
    let limits = smith_agent_world::LIMITS;
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut domain = agent::Domain::new(&limits, configuration(), 7);
    let mut out = Queue::with_capacity(agent::max_out(&limits));
    while domain.pop_fact().is_some() {}
    agent::step(
        &mut domain,
        &env,
        AgentEvent::Start {
            messages: decoded.messages,
            reply_to: ReplyTo::new(Token::new(1)),
            host_run: Token::new(7),
            identity: decoded.identity,
            activation: decoded.activation,
            window: decoded.window,
            charter: decoded.charter,
            workspace: None,
            transcript: decoded.transcript,
            answered: decoded.answered,
            grants: decoded
                .grants
                .iter()
                .map(|grant| agent::Grant { renewable: true, name: grant.name, valid: grant.valid })
                .collect(),
        },
        &mut out,
    );
    let AgentRequest::Admitted { run, .. } = out.pop().expect("admission") else { panic!("admission first") };
    wire.agent_admits();
    assert!(matches!(out.pop(), Some(AgentRequest::Waiting { read: None, .. })));
    assert!(out.pop().is_none(), "no provider request before work");
    wire.agent_waits(None);
    wire.settle();
    assert!(wire.observations().contains(&Observation::HostWaiting { read: None }));
    wire.send_message(Token::new(0), b"person".as_slice().into(), b"first".as_slice().into(), false);
    wire.settle();
    let (label, text) = wire
        .observations()
        .iter()
        .rev()
        .find_map(|seen| match seen {
            Observation::AgentMessage { name, label, text, .. } if *name == Token::new(0) => {
                Some((label.clone(), text.clone()))
            }
            _ => None,
        })
        .expect("actual decoded message");
    while domain.pop_fact().is_some() {}
    agent::step(
        &mut domain,
        &env,
        AgentEvent::Message { finish_requested: false, run, name: Token::new(0), label, text },
        &mut out,
    );
    let Some(AgentRequest::Long { operation: agent::Operation::Completion, .. }) = out.pop() else {
        panic!("first wire work announces actual provider issuance");
    };
    assert!(matches!(out.pop(), Some(AgentRequest::Complete { .. })), "first wire work opens the provider");
    assert!(out.pop().is_none());
}

fn composed_host() -> (smith_host_world::World, agent::Limits) {
    let charter = composed_charter();
    let mut limits = smith_agent_world::LIMITS;
    limits.run.message_bytes = 12;
    limits.run.messages = 1;
    let largest_turn = agent::max_turn_bytes(&limits).expect("bounded concrete turn");
    let mut host_limits = smith_host_world::limits();
    host_limits.charter_bytes = charter.owned_bytes();
    host_limits.turn_bytes = largest_turn;
    host_limits.unacknowledged_bytes = largest_turn;
    let mut host_world = smith_host_world::World::new(1, host_limits);
    let mut start = smith_host_world::start();
    start.charter = charter;
    start.transcript = None;
    start.answered = Box::default();
    start.grants = Box::from([host::Grant {
        account: 0,
        generation: 1,
        valid: Duration::from_secs(7200),
        form: smith_host_domain::Form::Credential(smith_host_domain::Scheme::Bearer),
    }]);
    host_world.spawn(start);
    host_world.spawned();
    (host_world, limits)
}

struct Composed {
    host_world: smith_host_world::World,
    wire: World,
    domain: agent::Domain,
    env: Env<agent::Limits>,
    out: Queue<AgentRequest>,
}

fn composed_start() -> Composed {
    let (mut host_world, limits) = composed_host();
    let first = host_world.seen.down.pop().expect("host domain first Send");
    let Down::Start { start, window } = first else { panic!("host first word must be Start") };
    let mut wire = World::new(CEILINGS, CEILINGS, StreamMode::Two);
    wire.settle();
    wire.send_domain_start(
        start,
        window,
        smith_host_protocol::Values { paths: Box::default(), credentials: Box::from([initial_grant()]) },
    );
    wire.settle();
    host_world.sent();
    let decoded = wire.take_agent_start().expect("translated Start reached agent side");
    let mut domain = agent::Domain::new(&limits, configuration(), 7);
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut out = Queue::with_capacity(agent::max_out(&limits));
    while domain.pop_fact().is_some() {}
    agent::step(
        &mut domain,
        &env,
        AgentEvent::Start {
            messages: decoded.messages,
            reply_to: ReplyTo::new(Token::new(1)),
            host_run: Token::new(7),
            identity: decoded.identity,
            activation: decoded.activation,
            window: decoded.window,
            charter: decoded.charter,
            workspace: None,
            transcript: decoded.transcript,
            answered: decoded.answered,
            grants: decoded
                .grants
                .iter()
                .map(|grant| agent::Grant { renewable: true, name: grant.name, valid: grant.valid })
                .collect(),
        },
        &mut out,
    );
    Composed { host_world, wire, domain, env, out }
}

#[test]
fn account_notices_reach_the_host_domain_without_values() {
    for NoticeCase { failure, report, detail, expected } in notice_cases() {
        let Composed { mut host_world, mut wire, mut domain, env, mut out } = composed_start();
        let owner = admit_composed(&mut wire, &mut host_world, &mut out);
        while domain.pop_fact().is_some() {}
        agent::step(
            &mut domain,
            &env,
            AgentEvent::Failed { owner, failure, evidence: agent::llm::Evidence::Unknown, detail, report },
            &mut out,
        );
        while let Some(request) = out.pop() {
            send_account_notice(&mut wire, request);
        }
        wire.settle();
        let decoded = decoded_notice(&wire);
        host_world.up(decoded);
        assert_eq!(host_world.seen.account_notices, [expected]);
    }
}

fn admit_composed(wire: &mut World, host_world: &mut smith_host_world::World, out: &mut Queue<AgentRequest>) -> Token {
    let mut owner = None;
    while let Some(request) = out.pop() {
        if matches!(request, AgentRequest::Admitted { .. }) {
            wire.agent_admits();
        } else if let AgentRequest::Complete { owner: issued, .. } = request {
            owner = Some(issued);
        } else if !matches!(request, AgentRequest::Long { .. }) {
            panic!("unexpected initial request: {request:?}");
        }
    }
    wire.settle();
    host_world.up(Up::Admitted);
    owner.expect("provider request issued")
}

fn send_account_notice(wire: &mut World, request: AgentRequest) {
    if let AgentRequest::Unauthorized { grant } = request {
        wire.agent_reports_unauthorized(grant.account, grant.generation);
    } else if let AgentRequest::Limited { account, reset } = request {
        wire.agent_reports_limited(account, reset);
    } else if let AgentRequest::Refused { account, code, reason } = request {
        wire.agent_reports_refused(account, &code, &reason);
    } else if !matches!(request, AgentRequest::Answer { .. } | AgentRequest::LongDone { .. }) {
        panic!("unexpected failure request: {request:?}");
    }
}

fn decoded_notice(wire: &World) -> Up {
    wire.observations()
        .iter()
        .find_map(|notice| {
            if let Observation::HostUnauthorized { account, generation } = notice {
                Some(Up::Unauthorized { account: *account, generation: *generation })
            } else if let Observation::HostLimited { account, reset } = notice {
                Some(Up::Limited { account: *account, reset: *reset })
            } else if let Observation::HostRefused { account, code, reason } = notice {
                Some(Up::Refused { account: *account, code: code.clone(), reason: reason.clone() })
            } else {
                None
            }
        })
        .expect("real host protocol decoded the account notice")
}

struct NoticeCase {
    failure: agent::llm::Failure,
    report: agent::llm::Report,
    detail: Box<[u8]>,
    expected: Up,
}

fn notice_cases() -> [NoticeCase; 3] {
    [
        NoticeCase {
            failure: agent::llm::Failure::Unauthorized,
            report: agent::llm::Report::none(),
            detail: Box::default(),
            expected: Up::Unauthorized { account: 0, generation: 1 },
        },
        NoticeCase {
            failure: agent::llm::Failure::Exhausted { retry_after: Some(Duration::from_secs(30)) },
            report: agent::llm::Report::none(),
            detail: Box::default(),
            expected: Up::Limited { account: 0, reset: Some(Duration::from_secs(30)) },
        },
        NoticeCase {
            failure: agent::llm::Failure::Refused,
            report: agent::llm::Report {
                code: Box::from(b"not-eligible".as_slice()),
                request: Box::default(),
                field: Box::default(),
            },
            detail: Box::from(b"provider reason".as_slice()),
            expected: Up::Refused {
                account: 0,
                code: Box::from(b"not-eligible".as_slice()),
                reason: Box::from(b"provider reason".as_slice()),
            },
        },
    ]
}
