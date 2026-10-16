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
    const TURNS: u64 = 7;
    const RESULT_BYTES: usize = 32_768;
    for turn in 0..TURNS {
        let owner = memory.completion(Asker::Main, false);
        memory.answer(owner, read(turn * u64::try_from(RESULT_BYTES).expect("result bytes")));
        let owner = memory.io();
        memory.step(Event::Done {
            owner,
            done: Done::Loaded {
                content: vec![b'x'; RESULT_BYTES].into_boxed_slice(),
                version: Version::new([1, 0, 0, 0]),
            },
        });
        if turn + 1 < TURNS {
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

fn tool(call: Decoded, name: &[u8]) -> Completion {
    Completion {
        reasoning_dropped: Box::new([]),
        content: Box::new([Said::ToolCall {
            id: b"c".as_slice().into(),
            name: name.into(),
            input: b"{}".as_slice().into(),
            call,
            replay: None,
        }]),
        stop: Stop::ToolUse,
        usage: Usage { input_tokens: Some(1000), output_tokens: Some(0), ..Usage::NONE },
    }
}

fn read_job_to_its_end(memory: &mut Memory) {
    use smith_domain_tools::{Budget, Exit, JobAct, JobEnd, Timeout, Wait, Yield};
    let owner = memory.completion(Asker::Main, false);
    memory.answer(
        owner,
        tool(
            Decoded::Owned {
                call: Call::Shell {
                    command: b"work".as_slice().into(),
                    timeout: Timeout::Default,
                    output: Budget::Default,
                    after: Yield::Asked(Duration::from_secs(1)),
                },
            },
            b"shell",
        ),
    );
    let owner = memory.io();
    memory.step(Event::Done {
        owner,
        done: Done::Held {
            head: vec![b'h'; 4096].into_boxed_slice(),
            tail: vec![b't'; 4096].into_boxed_slice(),
            dropped: 0,
        },
    });
    memory.settle();
    let job = memory.running_job();
    let owner = memory.completion(Asker::Main, false);
    memory.answer(
        owner,
        tool(
            Decoded::Owned { call: Call::Job { job, wait: Wait::Default, output: Budget::Default, stop: false } },
            b"job",
        ),
    );
    let (owner, act) = memory.job();
    assert_eq!(act, JobAct::Read { head: 16_384, tail: 49_152, wait: Duration::from_secs(30) });
    memory.step(Event::Done {
        owner,
        done: Done::Job {
            head: vec![b'h'; 8192].into_boxed_slice(),
            tail: vec![b't'; 8192].into_boxed_slice(),
            dropped: 0,
            end: JobEnd::Ended(Exit::Code { code: 0 }),
        },
    });
    memory.settle();
}

fn open_and_close_child(memory: &mut Memory) {
    use smith_domain::run::charter::{Families, Tools};
    use smith_domain::run::{Ask, Share};
    let owner = memory.completion(Asker::Main, false);
    memory.answer(
        owner,
        tool(
            Decoded::Served {
                ask: Ask::SubAgent {
                    brief: vec![b'b'; 1024].into_boxed_slice(),
                    families: Families { tools: Tools { inspect: true, modify: false, shell: false }, agents: false },
                    llm: None,
                    share: Share::UNASKED,
                },
            },
            b"sub_agent",
        ),
    );
    let owner = memory.completion(Asker::Child, false);
    memory.answer(
        owner,
        Completion {
            reasoning_dropped: Box::new([]),
            content: Box::new([Said::Text { text: vec![b'c'; 1024].into_boxed_slice(), replay: None }]),
            stop: Stop::EndTurn,
            usage: Usage { input_tokens: Some(1000), output_tokens: Some(256), ..Usage::NONE },
        },
    );
}

#[test]
fn repeated_compactions_jobs_and_children_hold_no_more_heap() {
    let mut memory = Memory::new(limits());
    memory.start();
    let mut held = [0; 3];
    for bytes in &mut held {
        fill_window(&mut memory);
        memory.settle();
        let owner = memory.completion(Asker::Main, true);
        memory.answer(owner, summary());
        read_job_to_its_end(&mut memory);
        open_and_close_child(&mut memory);
        *bytes = memory.held();
    }
    assert!(held[1] <= held[0]);
    assert!(held[2] <= held[0]);
}
