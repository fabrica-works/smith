//! Session read results pass the real protocol guard (domain/session.md, 8.1).

use alloc::boxed::Box;
use skein_lib::{Duration, Env, Queue, Time, Token, Wall};
use smith_domain::{llm, run, session, tools};
use smith_protocol_llm as adapter;

// Full results add at least 8192 bytes, beyond LINE - PROMPT = 7192.
const FILE_BYTES: usize = 4096;
const PROMPT: u64 = 1000;
const LINE: u64 = 8192;
const ROOT: Token = Token::new(70);
const OWNER: Token = Token::new(1);

fn named(text: &[u8]) -> tools::Name {
    tools::Name::new(text.into()).unwrap()
}

fn read_prompt(calls: u8, line: u64, usage: llm::Usage) -> (llm::Prompt, Vec<usize>) {
    let mut limits = super::standard_limits().unwrap().domain.session;
    limits.parallel_tools = 2;
    limits.tools.read_bytes = u32::try_from(FILE_BYTES).unwrap();
    limits.tools.tool_payload = u32::try_from(FILE_BYTES).unwrap();
    limits.tools.file_bytes = u32::try_from(FILE_BYTES).unwrap();
    limits.tools.path_bytes = 64;
    let mut domain = session::Domain::new(&limits, 1);
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut out = Queue::with_capacity(session::max_out(&limits));
    let opening = opening(line, limits.budget);
    session::step(&mut domain, &env, session::Event::Open { opener: OWNER, opening: Box::new(opening) }, &mut out);
    let mut session_owner = None;
    while let Some(request) = out.pop() {
        if let session::Request::Opened { session, .. } = request {
            session_owner = Some(session);
        }
    }
    session::reserve_completion(&mut domain, session_owner.unwrap(), 0, 0, 1024).unwrap();
    let content = (0..calls)
        .map(|index| session::llm::Block::ToolCall {
            id: Box::new([b'a'.checked_add(index).unwrap()]),
            name: b"read".as_slice().into(),
            input: br#"{"path":"file"}"#.as_slice().into(),
            call: session::llm::Decoded::Owned {
                call: tools::Call::Read {
                    path: tools::Path {
                        absolute: false,
                        parts: Box::new([tools::Part::Name { name: named(b"file") }]),
                    },
                    offset: 1,
                    window: tools::Budget::Default,
                },
            },
            replay: None,
        })
        .collect();
    session::step(
        &mut domain,
        &env,
        session::Event::Completed {
            owner: session_owner.unwrap(),
            completion: session::llm::Completion {
                reasoning_dropped: Box::new([]),
                content,
                stop: llm::Stop::ToolUse,
                usage,
            },
        },
        &mut out,
    );
    // Requests are the only observations; results are supplied at each tool's declared IO cap.
    for _ in 0..calls {
        let mut runs = Vec::new();
        while let Some(request) = out.pop() {
            match request {
                session::Request::Io { owner, .. } => runs.push(owner),
                session::Request::Complete { prompt, .. } => return resolved(prompt),
                session::Request::Opened { .. }
                | session::Request::JobsCancelled { .. }
                | session::Request::Turn { .. }
                | session::Request::Priced { .. }
                | session::Request::Yielded { .. }
                | session::Request::Used { .. }
                | session::Request::Ended { .. }
                | session::Request::Cancel { .. }
                | session::Request::CancelIo { .. }
                | session::Request::Job { .. }
                | session::Request::Delegate { .. }
                | session::Request::Withdraw { .. } => {}
            }
        }
        for owner in runs {
            session::step(
                &mut domain,
                &env,
                session::Event::Done {
                    owner,
                    done: tools::Done::Loaded {
                        content: vec![b'x'; FILE_BYTES].into(),
                        version: tools::Version::new([1, 0, 0, 0]),
                    },
                },
                &mut out,
            );
        }
    }
    panic!("reads issue a next completion")
}

fn resolved(prompt: session::llm::Prompt) -> (llm::Prompt, Vec<usize>) {
    let mut caps = Vec::new();
    let messages = prompt
        .messages
        .into_iter()
        .map(|message| llm::Message {
            role: message.role,
            content: message
                .content
                .into_iter()
                .map(|block| match block {
                    session::llm::Block::Text { text, replay } => llm::Block::Text { text, replay },
                    session::llm::Block::ToolCall { id, name, input, replay, .. } => {
                        llm::Block::ToolCall { id, name, input, replay }
                    }
                    session::llm::Block::ToolResult { id, result: session::llm::Returned::Owned { outcome } } => {
                        let tools::Outcome::Read { content, .. } = &outcome else { panic!("read result") };
                        caps.push(content.len());
                        llm::Block::ToolResult { id, result: llm::Returned::Owned { outcome } }
                    }
                    session::llm::Block::Opaque { .. }
                    | session::llm::Block::Refusal { .. }
                    | session::llm::Block::ToolResult { .. } => panic!("read-only fixture"),
                })
                .collect(),
        })
        .collect();
    (
        llm::Prompt {
            purpose: prompt.purpose,
            asker: llm::Asker::Main,
            endpoint: prompt.endpoint,
            affinity: prompt.affinity,
            model: prompt.model,
            effort: prompt.effort,
            system: prompt.system,
            tools: prompt.tools,
            namespace: prompt.namespace,
            served: Box::new([]),
            choice: llm::ToolChoice::Auto,
            messages,
            max_tokens: prompt.max_tokens,
        },
        caps,
    )
}

fn submit(prompt: llm::Prompt, provider: skein_llm::Provider) -> (Vec<adapter::ToDomain>, bool) {
    let limits = super::standard_limits().unwrap().llm;
    let endpoints = endpoints(&limits, provider);
    let mut component = adapter::Component::new(&limits, endpoints).unwrap();
    let grant = smith_domain::GrantName { account: 0, generation: 1 };
    component
        .grant(
            grant,
            match provider {
                skein_llm::Provider::OpenAiCodex => skein_llm::Credential {
                    value: b"fake".as_slice().into(),
                    account_id: b"fake".as_slice().into(),
                    scheme: skein_llm::Scheme::Bearer,
                },
                skein_llm::Provider::Anthropic => skein_llm::Credential::anthropic(b"fake".as_slice().into()),
                skein_llm::Provider::OpenAi => panic!("withdrawn route"),
            },
            Time::ZERO.saturating_add(Duration::from_secs(60)),
        )
        .unwrap();
    let env = Env { now: Time::ZERO, wall: Wall::EPOCH, limits };
    let mut above = Queue::with_capacity(adapter::MAX_OUT.above);
    let mut below = Queue::with_capacity(adapter::MAX_OUT.below);
    component.from_domain(
        &env,
        adapter::FromDomain::Complete {
            owner: OWNER,
            grant: Some(grant),
            prompt,
            deadlines: session::Deadlines {
                head: Duration::from_secs(1),
                idle: Duration::from_secs(1),
                whole: Time::ZERO.saturating_add(Duration::from_secs(10)),
            },
            bounds: limits.receiving,
            outcome: run::outcome::OutcomeSpec { change: None, verdicts: Box::new([]), failure: None, report: None },
            deliver: None,
            observation: None,
            deltas: false,
        },
        &mut above,
        &mut below,
    );
    let mut terminals = Vec::new();
    while let Some(terminal) = above.pop() {
        terminals.push(terminal);
    }
    let mut sent = false;
    let mut connection_owner = None;
    while let Some(request) = below.pop() {
        match request {
            skein_io::Request::Connect { owner, .. } => {
                connection_owner = Some(owner);
                component.from_below(&env, skein_io::Event::Connecting { owner, socket: ROOT }, &mut above, &mut below);
                component.from_below(&env, skein_io::Event::Connected { owner }, &mut above, &mut below);
            }
            skein_io::Request::Stream { down: skein_lib::stream::Down::Send(bytes), .. } => {
                sent |= !bytes.is_empty();
            }
            skein_io::Request::Stream { down: skein_lib::stream::Down::Demand { room, .. }, .. } if room > 0 => {
                component.from_below(
                    &env,
                    skein_io::Event::Stream { owner: connection_owner.unwrap(), up: skein_lib::stream::Up::Room },
                    &mut above,
                    &mut below,
                );
            }
            skein_io::Request::Listen { .. }
            | skein_io::Request::Bind { .. }
            | skein_io::Request::Reject { .. }
            | skein_io::Request::Stream { .. }
            | skein_io::Request::Output { .. }
            | skein_io::Request::Spawn { .. }
            | skein_io::Request::Signal { .. }
            | skein_io::Request::Usage { .. }
            | skein_io::Request::Window { .. }
            | skein_io::Request::Close { .. }
            | skein_io::Request::Abort { .. } => {}
        }
    }
    while let Some(terminal) = above.pop() {
        terminals.push(terminal);
    }
    (terminals, sent)
}

fn reported() -> llm::Usage {
    llm::Usage { input_tokens: Some(PROMPT), output_tokens: Some(5), ..llm::Usage::NONE }
}

#[test]
fn first_and_later_read_batches_pass_the_real_guard() {
    for (calls, provider) in [
        (2, skein_llm::Provider::OpenAiCodex),
        (4, skein_llm::Provider::OpenAiCodex),
        (2, skein_llm::Provider::Anthropic),
        (4, skein_llm::Provider::Anthropic),
    ] {
        let (prompt, caps) = read_prompt(calls, LINE, reported());
        assert_eq!(caps.len(), usize::from(calls));
        assert!(caps.iter().all(|cap| *cap < FILE_BYTES));
        let (terminals, sent) = submit(prompt, provider);
        assert!(sent, "{calls} calls: {terminals:?}");
        assert!(terminals.is_empty());
    }
}

#[test]
fn absent_usage_keeps_full_reads_and_sends_without_a_guard() {
    let (prompt, caps) = read_prompt(2, LINE, llm::Usage::NONE);
    assert_eq!(caps, [FILE_BYTES; 2]);
    let (terminals, sent) = submit(prompt, skein_llm::Provider::OpenAiCodex);
    assert!(sent, "{terminals:?}");
    assert!(terminals.is_empty());
}

#[test]
fn insufficient_minima_keep_full_reads_and_the_guard_refuses() {
    let (prompt, caps) = read_prompt(2, PROMPT + 1, reported());
    assert_eq!(caps, [FILE_BYTES; 2]);
    let (terminals, sent) = submit(prompt, skein_llm::Provider::OpenAiCodex);
    assert!(!sent);
    let [adapter::ToDomain::Failed { failure, evidence, .. }] = terminals.as_slice() else {
        panic!("one refused request")
    };
    assert_eq!(*failure, llm::Failure::ContextTooLong);
    assert_eq!(*evidence, llm::Evidence::Unsent);
}

fn opening(line: u64, budget: session::Budget) -> session::record::Opening {
    let spec = session::Spec {
        account: None,
        final_turn: None,
        endpoint: llm::Endpoint(0),
        affinity: llm::Affinity { key: [0; 16], thread: 0 },
        protocol_allowance: 0,
        progress: session::Progress { head: Duration::from_secs(30), idle: Duration::from_secs(30) },
        model: b"fake".as_slice().into(),
        effort: None,
        system: b"read files".as_slice().into(),
        authority: tools::Authority {
            cwd: Box::new([named(b"work")]),
            repos: Box::new([tools::Repo {
                host: Box::new([named(b"host")]),
                mount: Box::new([named(b"work")]),
                root: ROOT,
                writable: false,
            }]),
            grants: tools::Grants { inspect: true, modify: false, shell: false },
        },
        delegated: Box::new([]),
        prompt: b"read the files".as_slice().into(),
        window: 65_536,
        line,
        summary: b"summarize".as_slice().into(),
        preamble: b"summary".as_slice().into(),
        output: 1024,
        budget,
    };
    session::record::Opening {
        window_llm: None,
        spec,
        dialect: 1,
        prices: session::record::Prices { input: 0, cached: 0, cache_write: None, output: 0, unit: 1 },
        budget: 100,
        transcript: None,
        answered: Box::new([]),
    }
}

fn endpoints(limits: &adapter::ComponentLimits, provider: skein_llm::Provider) -> adapter::Endpoints {
    adapter::Endpoints::new(
        Box::new([adapter::ConfiguredEndpoint {
            name: llm::Endpoint(0),
            destination: skein_llm_connection::Endpoint {
                limits: limits.adapter.client,
                credential: skein_llm::client::CredentialLimits { value: 128, account_id: 128 },
                address: "127.0.0.1:443".parse().unwrap(),
                transport: skein_llm_connection::Transport::Plaintext,
                llm: match provider {
                    skein_llm::Provider::OpenAiCodex => skein_llm::Endpoint::codex(),
                    skein_llm::Provider::Anthropic => skein_llm::Endpoint::anthropic(),
                    skein_llm::Provider::OpenAi => panic!("withdrawn route"),
                },
            },
            connect: Duration::from_secs(1),
            handshake: Duration::from_secs(1),
            identity: adapter::IdentityProfile::Plain,
            models: Box::new([]),
        }]),
        limits.connection.endpoints,
    )
    .unwrap()
}
