//! The frozen phase-one control must be refused before effects or service four turns.
//! Contract: protocol/limits.md, sections 3.2 and 4; domain/session.md, section 5.

use skein_lib::{Duration, Queue, Token};
use smith_agent_service::profile::{self, Check, Configuration, Endpoint, Model, OversizedReasoning, Refusal, Value};
use smith_domain_session::{self as session, llm};
use smith_session_world::recorded::{World, opening};

#[test]
fn the_frozen_control_is_refused_at_startup_or_services_finish_after_four_turns() {
    let mut profile = profile::standard();
    profile.declared.memory = 2_147_483_648;
    profile.declared.tool_payload = 256;
    profile.declared.read_window = 256;
    profile.declared.calls_per_response = 1;
    profile.policy.group_stop = Duration::from_millis(250);
    let configuration = Configuration {
        role_bound: None,
        environment_bytes: 835,
        endpoints: Box::new([Endpoint {
            number: 1,
            provider: skein_llm::Provider::OpenAiCodex,
            option_bytes: 3,
            connect: Duration::from_secs(30),
            handshake: Duration::from_secs(30),
            models: Box::new([Model {
                efforts: Box::new([]),
                name: b"gpt-6-luna".as_slice().into(),
                window: 131_072,
                output: 4096,
                reasoning_item: 65_536,
                oversized_reasoning: OversizedReasoning::Fail,
                head: Duration::from_secs(60),
                idle: Duration::from_secs(120),
            }]),
        }]),
    };
    let limits = match profile::derive(&profile, &configuration) {
        Ok(limits) => limits,
        Err(Refusal::Broken { check: Check::WindowFirst, left, right }) => {
            assert_eq!(left.name.field, "line");
            assert_eq!(right.name.field, "history_bytes");
            let (Value::Bytes(required), Value::Bytes(available)) = (left.value, right.value) else {
                panic!("both window-first sides must retain byte units");
            };
            assert!(required > available);
            assert!(
                profile::words(&Refusal::Broken { check: Check::WindowFirst, left, right })
                    .starts_with(b"window-first: session.line[endpoint=1,model=0]=")
            );
            return;
        }
        Err(refusal) => panic!("the frozen control needs its actual receiving refusal: {refusal:?}"),
    };
    assert!(limits.domain.session.history_messages > 4, "the defect is not six-message room");
    let allowance = profile::endpoint_allowance(&profile, &configuration.endpoints[0], &limits.domain.session.tools)
        .expect("real fixed schemas and framing");
    let mut world = World::new(415, 256);
    world.env.limits = limits.domain.session;
    world.domain = session::Domain::new(&world.env.limits, 415);
    world.out = Queue::with_capacity(session::max_out(&world.env.limits));
    let mut start = opening(None, 1_000_000_000);
    start.spec.endpoint = llm::Endpoint(1);
    start.spec.model = b"gpt-6-luna".as_slice().into();
    start.spec.output = 4096;
    start.spec.protocol_allowance = allowance;
    start.spec.budget = world.env.limits.budget;
    start.spec.budget.turns = 4096;
    start.spec.budget.time = Duration::from_secs(300);
    world.open(start);
    for turn in 0..3 {
        assert!(world.completing.is_some(), "provider turn {} admitted: {:?}", turn + 1, world.end);
        world.complete(
            Box::new([llm::Block::Text { text: b"read complete".as_slice().into(), replay: None }]),
            llm::Stop::EndTurn,
            llm::Usage::ZERO,
        );
        world.step(session::Event::Continue {
            session: world.session.expect("an admitted conversation remains open"),
            content: b"continue to finish".as_slice().into(),
        });
    }
    world.complete(
        Box::new([llm::Block::ToolCall {
            id: b"finish-four".as_slice().into(),
            name: b"finish".as_slice().into(),
            input: br#"{"form":"report","text":"ALPHA"}"#.as_slice().into(),
            call: llm::Decoded::Delegated {
                descriptor: Token::new(19),
                source: session::ToolSource::Run,
                ticket: Token::new(991),
                effect: smith_domain_tools::Effect::Write,
            },
            replay: None,
        }]),
        llm::Stop::ToolUse,
        llm::Usage::ZERO,
    );
    assert_eq!(world.delegated.len(), 1, "fourth-turn finish must be served: {:?}", world.end);
    world.step(session::Event::Answered {
        owner: world.delegated[0],
        text: b"ALPHA".as_slice().into(),
        error: false,
        spent: 0,
    });
    assert!(world.delegated.is_empty(), "actual finish receiving right has its terminal");
    world.close();
}
