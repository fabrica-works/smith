//! Shared actual Smith adapter and HTTP/SSE pool peer, with caller-owned virtual time.
//! It keeps physical bindings and independent terminal observations, never retry policy.
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
        stated: crate::adapter::stated(),
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

/// The production LLM component, actual physical bindings and observed terminals.
pub struct Peer {
    component: Component,
    env: Env<ComponentLimits>,
    above: Queue<ToDomain>,
    below: Queue<IoRequest>,
    bindings: BTreeMap<Token, Binding>,
    /// Accepted calls have one independently observed terminal, in arrival order.
    pub terminals: Vec<ToDomain>,
}

impl Peer {
    /// Build a peer with declared calls and a lowered endpoint share.
    #[must_use]
    pub fn configured(calls: u32, per_endpoint: u32) -> Self {
        let mut limits = limits();
        limits.connection.calls = calls;
        limits.connection.connections = calls;
        limits.connection.per_endpoint = per_endpoint;
        limits.connection.io.sockets = calls + 1;
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

    /// Admit one actual call with caller-selected model and whole deadlines.
    pub fn start(&mut self, owner: u64, head: u64, idle: u64, whole: u64) {
        self.component.from_domain(
            &self.env,
            FromDomain::Complete {
                owner: Token::new(owner),
                grant: GrantName { account: 0, generation: 1 },
                prompt: llm::Prompt {
                    choice: smith_domain::llm::ToolChoice::Auto,
                    namespace: smith_domain::tools::Namespace::One,
                    affinity: smith_domain::llm::Affinity { key: [0; 16], thread: 0 },
                    asker: llm::Asker::Main,
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

    /// Count actual open socket bindings.
    #[must_use]
    pub fn connections(&self) -> usize {
        self.bindings.len()
    }

    /// Drive bounded lower settlement until no immediate work remains.
    pub fn pump(&mut self) {
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
                    | IoRequest::Signal { .. }
                    | IoRequest::Usage { .. }) => panic!("unexpected request: {other:?}"),
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

    /// Advance the deterministic clock, including every waiting whole deadline.
    pub fn advance(&mut self, seconds: u64) {
        self.env.now = Time::ZERO.saturating_add(Duration::from_secs(seconds));
        self.pump();
    }

    /// Deliver actual HTTP/SSE bytes to a named physical socket.
    pub fn bytes(&mut self, socket: u64, bytes: &[u8]) {
        assert!(
            self.bindings.contains_key(&Token::new(socket)),
            "live binding {socket}, terminals={:?}",
            self.terminals
        );
        self.bindings
            .get_mut(&Token::new(socket))
            .expect("live binding")
            .response
            .append(bytes)
            .expect("bounded response intake");
        self.pump();
    }

    /// Complete a native SSE response on one physical connection; later calls reuse it.
    pub fn complete(&mut self, socket: u64) {
        let body = concat!(
            "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"item\",\"role\":\"assistant\",\"content\":[]}}\n\n",
            "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"item\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"done\"}]}}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n"
        );
        let head =
            format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n", body.len());
        self.bytes(socket, &[head.as_bytes(), body.as_bytes()].concat());
    }

    /// Cancel an accepted call through the production adapter.
    pub fn cancel(&mut self, owner: u64) {
        self.component.from_domain(
            &self.env,
            FromDomain::Cancel { owner: Token::new(owner) },
            &mut self.above,
            &mut self.below,
        );
        self.pump();
    }
}
