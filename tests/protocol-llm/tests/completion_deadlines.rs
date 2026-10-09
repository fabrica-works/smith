//! Model progress and session expiry through the actual adapter and shared HTTP/SSE
//! pool, with virtual clocks. Session-world stories retain outer cancellation.
//! Contract: domain/session.md, sections 4, 6 and 10; protocol/llm.md, section 6.

use std::collections::BTreeMap;

use skein_io::{Event as IoEvent, Request as IoRequest};
use skein_lib::stream::{Down, Read, Up};
use skein_lib::{Duration, Env, Intake, Queue, Time, Token, Wall};
use smith_domain::{GrantName, llm, run, tools};
use smith_protocol_llm::{
    self as adapter, Component, ComponentLimits, ConfiguredEndpoint, Endpoints, FromDomain, Receiving, ToDomain,
};

fn limits() -> ComponentLimits {
    let client = skein_llm_world::limits();
    let adapter = adapter::Limits {
        client,
        tool_bytes: 32_768,
        rendered_result: client.dialect.string_bytes,
        shell_default: skein_lib::Duration::from_secs(120),
        shell_maximum: skein_lib::Duration::from_secs(1200),
    };
    let decoded_call_bytes = 4096;
    ComponentLimits {
        adapter,
        connection: skein_llm_connection::Limits {
            endpoints: 1,
            connections: 2,
            calls: 2,
            per_endpoint: 2,
            idle_keep: Duration::from_secs(1),
            io: skein_io::Limits {
                sockets: 3,
                refusals: 1,
                intake: 19_000,
                receive: 1024,
                output: 19_000,
                sends: 2,
                accepts: 1,
                backlog: 2,
                close_timeout: Duration::from_secs(1),
                retry: Duration::from_millis(10),
            },
            tls: skein_tls::client::Limits { read: 4096, send: 4096, records: skein_tls::client::MAX_RECORD },
        },
        receiving: Receiving {
            max_completion_bytes: adapter::completion_worst_case(&client, decoded_call_bytes).expect("receiving bound"),
            max_completion_blocks: client.dialect.parts,
            decoded_call_bytes,
            max_failure_bytes: client.dialect.detail_bytes,
        },
        contract_bytes: 4096,
        accounts: 1,
        grant_value_bytes: 128,
    }
}

struct Binding {
    owner: Token,
    read: Option<Read>,
    response: Intake,
}

struct Peer {
    component: Component,
    env: Env<ComponentLimits>,
    above: Queue<ToDomain>,
    below: Queue<IoRequest>,
    bindings: BTreeMap<Token, Binding>,
    terminals: Vec<ToDomain>,
}

impl Peer {
    fn new() -> Self {
        let limits = limits();
        let endpoints = Endpoints::new(
            Box::new([ConfiguredEndpoint {
                connect: Duration::from_secs(2),
                handshake: Duration::from_secs(2),
                name: llm::Endpoint(0),
                destination: skein_llm_connection::Endpoint {
                    limits: limits.adapter.client,
                    credential: skein_llm::client::CredentialLimits { access_token: 128, account_id: 128 },
                    address: "127.0.0.1:443".parse().expect("fixture address"),
                    transport: skein_llm_connection::Transport::Plaintext,
                    llm: skein_llm::Endpoint::codex(),
                },
                account: 0,
                reasoning_effort: None,
                cache_key: None,
                identity: smith_protocol_llm::IdentityProfile::Plain,
                models: Box::new([]),
            }]),
            limits.connection.endpoints,
            limits.connection.connections,
        )
        .expect("fixture endpoint");
        let mut component = Component::new(&limits, endpoints).expect("production component");
        component
            .grant(
                GrantName { account: 0, generation: 1 },
                skein_llm::Credential {
                    access_token: b"fake".as_slice().into(),
                    account_id: b"fake".as_slice().into(),
                },
                Time::ZERO.saturating_add(Duration::from_secs(1000)),
            )
            .expect("fixture grant");
        Self {
            component,
            env: Env { now: Time::ZERO, wall: Wall::EPOCH, limits },
            above: Queue::with_capacity(smith_protocol_llm::MAX_OUT.above),
            below: Queue::with_capacity(smith_protocol_llm::MAX_OUT.below),
            bindings: BTreeMap::new(),
            terminals: Vec::new(),
        }
    }

    fn start(&mut self, owner: u64, head: u64, idle: u64, whole: u64) {
        self.component.from_domain(
            &self.env,
            FromDomain::Complete {
                owner: Token::new(owner),
                grant: GrantName { account: 0, generation: 1 },
                prompt: llm::Prompt {
                    endpoint: llm::Endpoint(0),
                    model: b"fixture".as_slice().into(),
                    system: b"timed reasoning".as_slice().into(),
                    tools: tools::Grants { inspect: true, modify: false, shell: false },
                    served: Box::new([]),
                    messages: Box::new([llm::Message {
                        role: llm::Role::User,
                        content: Box::new([llm::Block::Text { text: b"begin".as_slice().into(), replay: None }]),
                    }]),
                    max_tokens: 4096,
                },
                deadlines: smith_domain::session::Deadlines {
                    head: Duration::from_secs(head),
                    idle: Duration::from_secs(idle),
                    whole: Time::ZERO.saturating_add(Duration::from_secs(whole)),
                },
                bounds: self.env.limits.receiving,
                outcome: run::outcome::OutcomeSpec {
                    change: None,
                    verdicts: Box::new([]),
                    report: None,
                    failure: None,
                },
                deliver: None,
            },
            &mut self.above,
            &mut self.below,
        );
        self.pump();
    }

    fn pump(&mut self) {
        for _ in 0..10_000 {
            while let Some(request) = self.below.pop() {
                match request {
                    IoRequest::Connect { owner, .. } => {
                        let socket = Token::new(100 + u64::try_from(self.bindings.len()).expect("few bindings"));
                        self.bindings
                            .insert(socket, Binding { owner, read: None, response: Intake::with_capacity(16_384) });
                        self.component.from_below(
                            &self.env,
                            IoEvent::Connecting { owner, socket },
                            &mut self.above,
                            &mut self.below,
                        );
                        self.component.from_below(
                            &self.env,
                            IoEvent::Connected { owner },
                            &mut self.above,
                            &mut self.below,
                        );
                    }
                    IoRequest::Stream { stream, down } => {
                        let binding = self.bindings.get_mut(&stream).expect("known stream");
                        match down {
                            Down::Demand { read, room } => {
                                binding.read = if room == 0 { Some(read) } else { None };
                                if room > 0 {
                                    self.component.from_below(
                                        &self.env,
                                        IoEvent::Stream { owner: binding.owner, up: Up::Room },
                                        &mut self.above,
                                        &mut self.below,
                                    );
                                }
                            }
                            Down::Send(_) | Down::Finish => {}
                        }
                    }
                    IoRequest::Close { entity } | IoRequest::Abort { entity } => {
                        let binding = self.bindings.remove(&entity).expect("one physical close");
                        self.component.from_below(
                            &self.env,
                            IoEvent::Closed { owner: binding.owner },
                            &mut self.above,
                            &mut self.below,
                        );
                    }
                    other @ (IoRequest::Listen { .. }
                    | IoRequest::Bind { .. }
                    | IoRequest::Reject { .. }
                    | IoRequest::Output { .. }
                    | IoRequest::Spawn { .. }
                    | IoRequest::Signal { .. }) => panic!("unexpected request: {other:?}"),
                }
            }
            let mut delivered = false;
            for binding in self.bindings.values_mut() {
                if let Some(read) = binding.read
                    && let Some(bytes) = binding.response.meet(read)
                {
                    binding.read = None;
                    delivered = true;
                    self.component.from_below(
                        &self.env,
                        IoEvent::Stream { owner: binding.owner, up: Up::Bytes(bytes) },
                        &mut self.above,
                        &mut self.below,
                    );
                }
            }
            while let Some(event) = self.above.pop() {
                if !matches!(event, ToDomain::Text { .. }) {
                    self.terminals.push(event);
                }
            }
            self.component.fire(&self.env, &mut self.above, &mut self.below);
            self.component.reclaim();
            if !delivered && !self.component.has_work() && self.above.is_empty() && self.below.is_empty() {
                return;
            }
        }
        panic!("bounded fixture did not settle");
    }

    fn advance(&mut self, seconds: u64) {
        self.env.now = Time::ZERO.saturating_add(Duration::from_secs(seconds));
        self.pump();
    }

    fn bytes(&mut self, socket: u64, bytes: Vec<u8>) {
        assert!(
            self.bindings.contains_key(&Token::new(socket)),
            "live binding {socket}, terminals={:?}",
            self.terminals
        );
        self.bindings
            .get_mut(&Token::new(socket))
            .expect("live binding")
            .response
            .append(&bytes)
            .expect("bounded response intake");
        self.pump();
    }

    fn cancel(&mut self, owner: u64) {
        self.component.from_domain(
            &self.env,
            FromDomain::Cancel { owner: Token::new(owner) },
            &mut self.above,
            &mut self.below,
        );
        self.pump();
    }
}

fn body() -> Vec<u8> {
    [
        r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"item","role":"assistant","content":[]}}"#,
        r#"{"type":"response.output_item.done","output_index":0,"item":{"type":"message","id":"item","role":"assistant","content":[{"type":"output_text","text":"done"}]}}"#,
        r#"{"type":"response.completed","response":{"status":"completed","output":[],"usage":{"input_tokens":1,"output_tokens":1}}}"#,
    ]
    .iter()
    .flat_map(|event| format!("data: {event}\n\n").into_bytes())
    .collect()
}

fn head_for(bytes: usize) -> Vec<u8> {
    format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {bytes}\r\n\r\n").into_bytes()
}

const HEARTBEAT: &[u8] = b": progress\n\n";

#[test]
fn a_response_streaming_past_any_fixed_bound_completes() {
    let mut peer = Peer::new();
    peer.start(1, 60, 60, 900);
    peer.bytes(100, head_for(body().len() + 9 * HEARTBEAT.len()));
    for seconds in (50..=450).step_by(50) {
        peer.advance(seconds);
        assert!(peer.terminals.is_empty(), "progress remains within the declared idle duration");
        peer.bytes(100, HEARTBEAT.to_vec());
    }
    peer.advance(451);
    peer.bytes(100, body());
    assert!(matches!(peer.terminals.as_slice(), [ToDomain::Completed { owner, completion }] if *owner == Token::new(1)
        && completion.content.iter().any(|block| matches!(block, llm::Said::Text { text, .. } if text.as_ref() == b"done"))));
    peer.advance(901);
    assert_eq!(peer.terminals.len(), 1, "former phase and whole timers never repeat a terminal");
}

#[test]
fn model_head_and_idle_deadlines_fail_only_the_stalled_call() {
    let mut peer = Peer::new();
    peer.start(1, 5, 8, 100);
    peer.start(2, 20, 5, 100);
    peer.bytes(101, head_for(body().len()));
    peer.advance(6);
    assert_eq!(peer.terminals.len(), 2);
    assert!(
        peer.terminals.iter().all(|event| matches!(event, ToDomain::Failed { failure: llm::Failure::TimedOut, .. }))
    );
    peer.advance(101);
    assert_eq!(peer.terminals.len(), 2, "expired calls settle once");
}

#[test]
fn session_whole_expiry_and_cancel_leave_the_sibling_live() {
    let mut peer = Peer::new();
    peer.start(1, 30, 30, 90);
    peer.start(2, 120, 120, 300);
    peer.bytes(100, head_for(body().len() + 4 * HEARTBEAT.len()));
    peer.bytes(101, head_for(body().len()));
    for seconds in [20, 40, 60, 80] {
        peer.advance(seconds);
        peer.bytes(100, HEARTBEAT.to_vec());
    }
    assert!(peer.terminals.is_empty());
    peer.advance(90);
    assert!(
        matches!(peer.terminals.as_slice(), [ToDomain::Failed { owner, failure: llm::Failure::TimedOut, evidence: llm::Evidence::Response, .. }] if *owner == Token::new(1))
    );
    peer.bytes(101, body());
    assert!(matches!(&peer.terminals[1], ToDomain::Completed { owner, .. } if *owner == Token::new(2)));
    peer.advance(301);
    assert_eq!(peer.terminals.len(), 2);

    let mut peer = Peer::new();
    peer.start(1, 60, 60, 300);
    peer.start(2, 60, 60, 300);
    peer.bytes(100, head_for(body().len()));
    peer.bytes(101, head_for(body().len()));
    peer.cancel(1);
    peer.cancel(1);
    peer.bytes(101, body());
    assert!(matches!(&peer.terminals[0], ToDomain::Cancelled { owner } if *owner == Token::new(1)));
    assert!(matches!(&peer.terminals[1], ToDomain::Completed { owner, .. } if *owner == Token::new(2)));
    peer.advance(301);
    assert_eq!(peer.terminals.len(), 2);
}
