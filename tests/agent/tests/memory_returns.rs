//! Heap returns after a window's use ends (protocol/limits.md, sections 5 and 11).

use skein_lib::Duration;
use skein_world::domain::heap;
use smith_agent_service::profile::{self, Configuration, Endpoint, Model, OversizedReasoning};
use smith_agent_world::memory::Memory;
use smith_domain::llm::{Asker, Completion, Decoded, Said, Stop, Usage};
use smith_domain::tools::{Call, Done, Name, Part, Path, Version};
use smith_domain::{Event, Limits};

#[global_allocator]
static HEAP: heap::Counting = heap::Counting;

fn limits() -> Limits {
    let declarations = Configuration {
        role_bound: None,
        environment_bytes: 0,
        endpoints: Box::new([Endpoint {
            number: 0,
            provider: skein_llm::Provider::OpenAiCodex,
            option_bytes: 0,
            connect: Duration::from_secs(30),
            handshake: Duration::from_secs(30),
            models: Box::new([Model {
                efforts: Box::new([]),
                name: b"m".as_slice().into(),
                window: 272_000,
                output: 32_000,
                reasoning_item: 65_536,
                oversized_reasoning: OversizedReasoning::Fail,
                head: Duration::from_secs(60),
                idle: Duration::from_secs(300),
            }]),
        }]),
    };
    profile::derive(&profile::standard(), &declarations).expect("standard deployment derives").domain
}

fn read(input_tokens: u64) -> Completion {
    Completion {
        reasoning_dropped: Box::new([]),
        content: Box::new([
            Said::Text { text: vec![b'a'; 1024].into_boxed_slice(), replay: None },
            Said::ToolCall {
                id: b"c".as_slice().into(),
                name: b"read".as_slice().into(),
                input: b"{}".as_slice().into(),
                replay: None,
                call: Decoded::Owned {
                    call: Call::Read {
                        path: Path {
                            absolute: false,
                            parts: Box::new([Part::Name {
                                name: Name::new(b"data".as_slice().into()).expect("file name"),
                            }]),
                        },
                        offset: 1,
                        window: smith_domain_tools::Budget::Default,
                    },
                },
            },
        ]),
        stop: Stop::ToolUse,
        usage: Usage { input_tokens: Some(input_tokens), output_tokens: Some(0), ..Usage::NONE },
    }
}

fn fill_window(memory: &mut Memory) {
    for turn in 0..6 {
        let owner = memory.completion(Asker::Main, false);
        memory.answer(owner, read(turn * 32_768));
        let owner = memory.io();
        memory.step(Event::Done {
            owner,
            done: Done::Loaded { content: vec![b'x'; 32_768].into_boxed_slice(), version: Version::new([1, 0, 0, 0]) },
        });
        if turn < 5 {
            memory.settle();
        }
    }
}

fn summary() -> Completion {
    Completion {
        reasoning_dropped: Box::new([]),
        content: Box::new([Said::Text { text: vec![b's'; 64].into_boxed_slice(), replay: None }]),
        stop: Stop::EndTurn,
        usage: Usage { output_tokens: Some(64), ..Usage::NONE },
    }
}

#[test]
fn a_compaction_releases_the_compacted_historys_bytes() {
    // Six answers: 1024 text + 1 id + 4 name + 2 input + 1 result id + 32768 result.
    // The task survives; the replacement adds the 94-byte preamble and 64-byte summary.
    const EXPECTED_FALL: u64 = 202_642;
    let mut memory = Memory::new(limits());
    memory.start();
    fill_window(&mut memory);
    let before = memory.held();
    memory.settle();
    let owner = memory.completion(Asker::Main, true);
    memory.answer(owner, summary());
    let after = memory.held();
    assert!(before - after >= EXPECTED_FALL);
}
