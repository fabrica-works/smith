//! Derived history ownership at its boundary, through an actual shared Client.
//! Contract: protocol/limits.md, section 3.2; domain/session.md, section 4.

use skein_fake_llm_domain::api::{Finish, Line, Script, Turn};
use skein_lib::{Duration, Queue, Token};
use smith_domain::{Event, llm, session};
use smith_protocol_llm_world::{
    adapter,
    wire::{self, Wire},
};
use smith_session_world::recorded::{World, opening};

#[test]
fn a_derived_history_at_its_bound_is_sent_and_one_byte_more_is_refused_typed() {
    for provider in [skein_llm::Provider::OpenAiCodex, skein_llm::Provider::Anthropic] {
        history_edge(provider);
    }
}

struct Fixture {
    declared: skein_llm::Declared,
    client: skein_llm::client::Limits,
    limits: session::Limits,
    decoded: u64,
    allowance: u64,
}

fn fixture(provider: skein_llm::Provider) -> Fixture {
    // (512 window - 16 output - 50 material) * 8 bytes per token.
    const LINE_BYTES: u64 = 3_568;
    let declared = skein_llm::Declared {
        tools: 16,
        history_items: 256,
        window: 512,
        output: 16,
        reasoning_item: 1,
        tool_payload: 16,
        calls_per_response: 1,
        conversations: 1,
    };
    let mut client = skein_llm::Limits::derive(&declared).expect("tiny actual model").dialect(provider);
    client.http.request = 4096;
    client.http.read = 256;
    client.http.send = 256;
    client.sse.chunk = 256;
    let tools = smith_protocol_llm_world::LIMITS.session.tools;
    let allowance = smith_protocol_llm::prompt_allowance(provider, &client, 0).expect("measured native framing");
    let mut limits = session::derive(&session::Derivation {
        conversations: 1,
        calls_per_response: 1,
        parallel_tools: 4,
        max_turns: 4,
        max_spend: u64::MAX,
        max_time: Duration::from_secs(60),
        tools,
        line_bytes: LINE_BYTES,
        completion_bytes: 4096,
        answer_bytes: u64::from(client.answer),
        batch_result_bytes: 4096,
        delegated_result_bytes: 4096,
        output_items: client.output_items,
        detail: client.detail_bytes,
        protocol_allowance: allowance,
        threshold: [4, 5],
        token_bytes: skein_llm::TOKEN_BYTES,
    })
    .expect("native history derives");
    let decoded = u64::from(client.input)
        + u64::from(client.output_items) * u64::try_from(size_of::<llm::Decoded>()).expect("decoded cell");
    limits.completion_bytes = adapter::completion_worst_case(&client, decoded).expect("native completion ownership");
    Fixture { declared, client, limits, decoded, allowance }
}

fn history_edge(provider: skein_llm::Provider) {
    let Fixture { declared, client, mut limits, decoded, allowance } = fixture(provider);
    let tools = smith_protocol_llm_world::LIMITS.session.tools;
    let configuration = match provider {
        skein_llm::Provider::OpenAi => panic!("fixture exercises the withdrawn routes"),
        skein_llm::Provider::OpenAiCodex => 0,
        skein_llm::Provider::Anthropic => 1,
    };
    let system = b"@history-window";
    let payload = payload_at_edge(&limits, system);
    limits.history_bytes += u64::try_from(size_of::<session::llm::Block>()).expect("summary cell")
        + u64::from(limits.tools.path_bytes) * u64::from(limits.completion_blocks);
    for extra in [0, 1] {
        let mut world = World::new(4800 + extra, 256);
        world.env.limits = limits;
        world.domain = session::Domain::new(&limits, 4800 + extra);
        world.out = Queue::with_capacity(session::max_out(&limits));
        let mut start = opening(None, u64::MAX);
        start.spec.model = b"a".as_slice().into();
        start.spec.system = system.as_slice().into();
        start.spec.prompt = vec![b'p'; usize::try_from(payload + extra).expect("small history")].into();
        start.spec.delegated = Box::new([]);
        start.spec.budget = limits.budget;
        start.spec.output = declared.output;
        start.spec.protocol_allowance = allowance;
        world.open(start);
        if extra == 1 {
            assert!(world.prompts.is_empty(), "past the history bound never sends a provider request");
            assert_eq!(world.end, Some(history_failure(limits.history_bytes)));
            world.domain.reclaim();
            assert_eq!(world.domain.sessions(), 0);
            assert_eq!(world.domain.kits(), 0);
            continue;
        }
        let sent = world.prompts.last().expect("exact receiving boundary starts its completion");
        let prompt = llm::Prompt {
            effort: None,
            purpose: llm::Purpose::Ordinary { guard: None },
            choice: llm::ToolChoice::Auto,
            namespace: sent.namespace.clone(),
            affinity: sent.affinity,
            asker: llm::Asker::Main,
            endpoint: llm::Endpoint(sent.endpoint.0),
            model: sent.model.clone(),
            system: sent.system.clone(),
            tools: sent.tools,
            served: Box::new([]),
            max_tokens: sent.max_tokens,
            messages: Box::new([llm::Message {
                role: llm::Role::User,
                content: Box::new([llm::Block::Text {
                    text: vec![b'p'; usize::try_from(payload).expect("history")].into(),
                    replay: None,
                }]),
            }]),
        };
        let adapter_limits = adapter::Limits {
            client,
            tool_bytes: 1024,
            history_bytes: u64::from(client.request),
            rendered_result: 1024,
            stated: adapter::Stated::from_tools(&tools, client.input).expect("effective numeric descriptions"),
        };
        let receiving = adapter::Receiving {
            max_completion_bytes: limits.completion_bytes,
            max_completion_blocks: limits.completion_blocks,
            decoded_call_bytes: decoded,
            max_failure_bytes: limits.failure_bytes,
        };
        let mut connection = Wire::prepare(
            Token::new(48),
            prompt,
            receiving,
            &wire::configurations()[configuration],
            &adapter_limits,
            scripts(system),
        )
        .expect("admitted history prepares the actual Client");
        let mut terminals = connection.start();
        for _ in 0..100_000 {
            let (progress, returned) = connection.tick();
            terminals.extend(returned);
            if !progress {
                break;
            }
        }
        assert_eq!(connection.peer.queries.len(), 1, "outside peer receives the exact admitted history");
        let [Event::Completed { completion, .. }] = terminals.as_slice() else {
            panic!("actual completion expected: {terminals:?}");
        };
        let [llm::Said::Text { text, .. }] = completion.content.as_ref() else { panic!("one actual text") };
        world.complete(
            Box::new([session::llm::Block::Text { text: text.clone(), replay: None }]),
            session::llm::Stop::EndTurn,
            completion.usage,
        );
        world.close();
        assert!(connection.close().is_empty());
        assert!(connection.settle().is_empty());
    }
}

fn scripts(system: &[u8]) -> Box<[Script]> {
    Box::new([Script {
        cue: system.into(),
        turns: Box::new([Turn {
            lines: Box::new([Line::Text { text: b"done".as_slice().into() }]),
            finish: Finish::Stop,
            tokens: 1,
        }]),
    }])
}

fn history_failure(bound: u64) -> session::End {
    session::End::Failed {
        detail: Box::default(),
        failure: session::llm::Failure::Limit { which: session::llm::Limit::History, bound },
        evidence: session::llm::Evidence::Unsent,
        attempts: 0,
    }
}

fn payload_at_edge(limits: &session::Limits, system: &[u8]) -> u64 {
    let reserve = session::completion_reserve(limits).expect("whole receiving credit");
    limits
        .history_bytes
        .checked_sub(reserve)
        .and_then(|bytes| {
            bytes.checked_sub(
                u64::try_from(system.len() + 1 + size_of::<session::llm::Block>()).expect("fixed ownership"),
            )
        })
        .and_then(|bytes| bytes.checked_sub(26)) // The fixture's retained summary instruction.
        .and_then(|bytes| bytes.checked_sub(23)) // The retained preamble is beside the task.
        .and_then(|bytes| bytes.checked_div(2)) // The task is retained beside its history block.
        .expect("model holds one maximum history and receiving credit")
}
