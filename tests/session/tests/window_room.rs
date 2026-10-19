//! Startup-derived fresh windows through the real session boundary.
//! Contract: protocol/limits.md, sections 3.2, 3.9 and 4.

use skein_lib::{Duration, Queue};
use smith_agent_service::profile::{self, Configuration, Endpoint, Model, OversizedReasoning};
use smith_domain_session::{self as session, llm};
use smith_session_world::recorded::{World, opening};

#[test]
fn a_window_opened_at_its_largest_makes_its_first_completion() {
    for window in [65_536, 131_072] {
        let mut profile = profile::standard();
        profile.declared.memory = 2_147_483_648;
        profile.declared.conversations = 1;
        profile.declared.calls_per_response = 2;
        profile.declared.tool_payload = 256;
        profile.declared.read_window = 256;
        let declarations = Configuration {
            role_bound: None,
            environment_bytes: 0,
            endpoints: Box::new([Endpoint {
                number: 7,
                provider: skein_llm::Provider::OpenAiCodex,
                option_bytes: 0,
                connect: Duration::from_secs(1),
                handshake: Duration::from_secs(1),
                models: Box::new([Model {
                    efforts: Box::new([]),
                    name: b"model".as_slice().into(),
                    window,
                    output: 512,
                    reasoning_item: 64,
                    oversized_reasoning: OversizedReasoning::Fail,
                    head: Duration::from_secs(1),
                    idle: Duration::from_secs(1),
                }]),
            }]),
        };
        let limits = profile::derive(&profile, &declarations).expect("small real model derives");
        let allowance = profile::endpoint_allowance(&profile, &declarations.endpoints[0], &limits.domain.session.tools)
            .expect("actual schemas and provider framing");
        let client =
            profile::endpoint_client(&profile, &declarations.endpoints[0], limits.domain.session.history_blocks)
                .expect("native client");
        let inventory = profile.policy.host_tools.checked_add(10).expect("bounded inventory");
        let framing = smith_protocol_llm::prompt_allowance(declarations.endpoints[0].provider, &client, inventory)
            .expect("provider framing");
        let own = profile::room::own_part(&limits.domain.session.tools, client.input, framing)
            .expect("measured fixed own part");
        let parts = profile::room::room(&profile, &declarations, own, 0).expect("fresh room");
        let mut world = World::new(4606 + u64::from(window), 256);
        world.env.limits = limits.domain.session;
        world.domain = session::Domain::new(&world.env.limits, 4606 + u64::from(window));
        world.out = Queue::with_capacity(session::max_out(&world.env.limits));
        let mut start = opening(None, world.env.limits.spend);
        start.spec.delegated = Box::new([]);
        start.spec.budget = world.env.limits.budget;
        start.spec.output = 512;
        start.spec.window = window;
        start.spec.protocol_allowance = allowance;
        // The entire host part and fixed text are present. The initial task
        // holds both offers' bytes, covering the summary and held-back offer
        // reservation before compaction's own instruction is introduced.
        start.spec.system =
            vec![b's'; usize::try_from(parts.own + parts.host - allowance).expect("small system")].into();
        start.spec.prompt = vec![b'p'; usize::try_from(parts.offer * 2).expect("two offers")].into();
        world.open(start);
        assert!(world.completing.is_some(), "largest fresh window admitted: {:?}", world.end);
        let sent = world.prompts.last().expect("the provider receives its first request");
        assert!(sent.input_bytes().expect("bounded input") < world.env.limits.history_bytes);
        world.complete(
            Box::new([llm::Block::Text { text: b"done".as_slice().into(), replay: None }]),
            llm::Stop::EndTurn,
            llm::Usage::ZERO,
        );
        assert_eq!(world.turns.len(), 1, "the completion produces a real transcript turn");
        world.close();

        let mut final_world = World::new(4611 + u64::from(window), 256);
        final_world.env.limits = limits.domain.session;
        final_world.domain = session::Domain::new(&final_world.env.limits, 4611 + u64::from(window));
        final_world.out = Queue::with_capacity(session::max_out(&final_world.env.limits));
        let mut final_start = opening(None, final_world.env.limits.spend);
        final_start.spec.budget = final_world.env.limits.budget;
        final_start.spec.budget.turns = 1;
        final_start.spec.window = window;
        final_start.spec.final_turn =
            Some(session::FinalTurn { note: b"Answer now.".as_slice().into(), allowance: Duration::from_secs(1) });
        final_world.open(final_start);
        final_world.complete(smith_session_world::recorded::called(), llm::Stop::ToolUse, llm::Usage::ZERO);
        final_world.resume();
        assert!(final_world.delegated.is_empty(), "final-turn calls start no effects");
        let turn = final_world.turns.last().expect("the actual final completion produces a durable turn");
        let mut names = skein_lib::List::with_capacity(1);
        names
            .push(smith_protocol_channel::Endpoint {
                name: b"model-endpoint".as_slice().into(),
                number: turn.endpoint.expect("named endpoint").0,
                dialect: turn.dialect,
            })
            .expect("one declared endpoint");
        let names = smith_protocol_channel::Endpoints::new(names);
        smith_protocol_channel::encode_turn(turn, &limits.channel.transcript, &names)
            .expect("derived transcript receiving holds opening, final note, assistant and settled results");
        let mut short = limits.channel.transcript;
        short.turn_messages = u32::try_from(turn.messages.len()).expect("bounded turn") - 1;
        assert!(smith_protocol_channel::encode_turn(turn, &short, &names).is_err(), "one missing message refuses");
        final_world.close();
    }
}
