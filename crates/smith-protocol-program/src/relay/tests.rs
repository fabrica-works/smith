#![expect(clippy::match_like_matches_macro, reason = "I/O record filters use explicit matches in the test harness")]
#![expect(clippy::wildcard_enum_match_arm, reason = "test record filters ignore unrelated I/O effects")]
#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "unit test harnesses use ordinary Rust per programming-model.md, section 10.2"
)]

extern crate std;
use super::{ACK, Exit, Relay, Target, Usage, parse};
use alloc::{boxed::Box, format, vec, vec::Vec};
use skein_io::{self as io};
use skein_lib::{
    Queue, Secret, Token,
    stream::{Down, Fault, OutputDown, OutputOutcome, OutputUp, Read, Up},
};

const INPUT: Token = Token::new(1);
const OUTPUT: Token = Token::new(2);
const OWNER: Token = Token::new(3);
const SOCKET: Token = Token::new(100);
fn arguments() -> Vec<Box<[u8]>> {
    vec![b"--mcp-relay".as_slice().into(), b"127.0.0.1:47100".as_slice().into()]
}

fn environment() -> Vec<Box<[u8]>> {
    vec![format!("SMITH_MCP_SECRET={}", "07".repeat(32)).into_bytes().into_boxed_slice()]
}

fn drain(below: &mut Queue<io::Request>) -> Vec<io::Request> {
    let mut records = Vec::new();
    while let Some(record) = below.pop() {
        records.push(record);
    }
    records
}

struct Harness {
    relay: Relay,
    below: Queue<io::Request>,
}

impl Harness {
    fn new() -> Self {
        let mut below = Queue::with_capacity(64);
        let target = Target {
            addr: "127.0.0.1:47100".parse().expect("address"),
            secret: Secret::new(Box::new(*b"0707070707070707070707070707070707070707070707070707070707070707")),
        };
        let relay = Relay::new(target, INPUT, OUTPUT, OWNER, &mut below);
        Self { relay, below }
    }
    fn event(&mut self, event: io::Event) {
        self.relay.from_io(event, &mut self.below);
    }
    fn connect(&mut self) {
        drain(&mut self.below);
        self.event(io::Event::Connecting { owner: OWNER, socket: SOCKET });
        self.event(io::Event::Connected { owner: OWNER });
    }
    fn grant(&mut self, stream: Token) -> Vec<u8> {
        let records = drain(&mut self.below);
        let right = records
            .into_iter()
            .find_map(|request| match request {
                io::Request::Output { stream: target, down: OutputDown::Room { right, .. } } if target == stream => {
                    Some(right)
                }
                _ => None,
            })
            .expect("output room request");
        let owner = if stream == SOCKET { OWNER } else { stream };
        self.event(io::Event::Output { owner, up: OutputUp::Settled { right, outcome: OutputOutcome::Granted } });
        let records = drain(&mut self.below);
        records
            .into_iter()
            .find_map(|request| match request {
                io::Request::Output { stream: target, down: OutputDown::Send { bytes, .. } } if target == stream => {
                    Some(bytes.into_vec())
                }
                _ => None,
            })
            .expect("output send")
    }
    fn handshake(&mut self) {
        self.connect();
        assert_eq!(self.grant(SOCKET), format!("{}\n", "07".repeat(32)).as_bytes());
        self.event(io::Event::Stream { owner: OWNER, up: Up::Bytes(ACK.into()) });
        drain(&mut self.below);
    }
    fn settle(&mut self) {
        for owner in [INPUT, OUTPUT, OWNER] {
            self.event(io::Event::Closed { owner });
        }
    }
}

#[test]
fn the_relay_parses_its_flag_with_its_secret() {
    let target = parse(&arguments(), &environment()).expect("valid target");
    assert!(target.addr.ip().is_loopback());
    assert_eq!(target.addr.port(), 47100);
    assert_eq!(target.secret.len(), 64);
    let mut args = arguments();
    *args.last_mut().expect("address") = b"[::1]:47100".as_slice().into();
    let _target = parse(&args, &environment()).expect("IPv6 loopback target");
}

#[test]
fn a_relay_without_its_secret_is_refused() {
    assert_eq!(parse(&arguments(), &[]).expect_err("no secret"), Usage::Secret);
    for secret in ["A".repeat(64), "a".repeat(63), "a".repeat(65), "g".repeat(64)] {
        let env = vec![format!("SMITH_MCP_SECRET={secret}").into_bytes().into_boxed_slice()];
        assert_eq!(parse(&arguments(), &env).expect_err("invalid secret"), Usage::Secret);
    }
}

#[test]
fn a_relay_to_a_non_loopback_address_is_refused() {
    for addr in [b"192.0.2.1:9".as_slice(), b"localhost:9", b"127.0.0.1:0", b"[::]:9"] {
        let args = vec![b"--mcp-relay".as_slice().into(), addr.into()];
        assert_eq!(parse(&args, &environment()).expect_err("invalid address"), Usage::Address);
    }
}

#[test]
fn a_relay_with_extra_arguments_is_refused() {
    let mut args = arguments();
    args.push(b"extra".as_slice().into());
    assert_eq!(parse(&args, &environment()).expect_err("extra"), Usage::Arguments);
    assert_eq!(parse(&[], &environment()).expect_err("missing"), Usage::Arguments);
}

#[test]
fn the_relay_sends_its_secret_before_any_message() {
    let mut h = Harness::new();
    h.connect();
    let records = drain(&mut h.below);
    assert!(!records.iter().any(|request| match request {
        io::Request::Stream { stream: INPUT, .. } => true,
        _ => false,
    }));
    let right = records
        .into_iter()
        .find_map(|r| match r {
            io::Request::Output { down: OutputDown::Room { right, bytes: 65 }, .. } => Some(right),
            _ => None,
        })
        .expect("secret room");
    h.event(io::Event::Output { owner: OWNER, up: OutputUp::Settled { right, outcome: OutputOutcome::Granted } });
    let records = drain(&mut h.below);
    assert!(records.iter().any(|r| match r {
        io::Request::Output { down: OutputDown::Send { bytes, .. }, .. }
            if bytes.as_ref() == format!("{}\n", "07".repeat(32)).as_bytes() =>
            true,
        _ => false,
    }));
    assert!(!records.iter().any(|r| match r {
        io::Request::Stream { stream: INPUT, .. } => true,
        _ => false,
    }));
}

#[test]
fn the_relay_copies_each_way_under_room() {
    let mut h = Harness::new();
    h.handshake();
    h.event(io::Event::Stream { owner: INPUT, up: Up::Bytes(b"request\n".as_slice().into()) });
    let records = drain(&mut h.below);
    assert!(!records.iter().any(|r| match r {
        io::Request::Stream { stream: INPUT, .. } => true,
        _ => false,
    }));
    let right = records
        .into_iter()
        .find_map(|r| match r {
            io::Request::Output { down: OutputDown::Room { right, .. }, .. } => Some(right),
            _ => None,
        })
        .expect("socket room");
    h.event(io::Event::Output { owner: OWNER, up: OutputUp::Settled { right, outcome: OutputOutcome::Granted } });
    assert!(drain(&mut h.below).iter().any(|r| match r {
        io::Request::Output { down: OutputDown::Send { bytes, .. }, .. } if bytes.as_ref() == b"request\n" => true,
        _ => false,
    }));
    h.event(io::Event::Stream { owner: OWNER, up: Up::Bytes(b"answer\n".as_slice().into()) });
    assert_eq!(h.grant(OUTPUT), b"answer\n");
}

#[test]
fn the_input_s_end_finishes_the_socket() {
    let mut h = Harness::new();
    h.handshake();
    h.event(io::Event::Stream { owner: INPUT, up: Up::End });
    assert!(drain(&mut h.below).contains(&io::Request::Stream { stream: SOCKET, down: Down::Finish }));
    assert_eq!(h.relay.ended(), None);
}

#[test]
fn the_socket_s_end_ends_the_relay_with_0() {
    let mut h = Harness::new();
    h.handshake();
    h.event(io::Event::Stream { owner: INPUT, up: Up::End });
    h.event(io::Event::Stream { owner: OWNER, up: Up::End });
    assert!(drain(&mut h.below).contains(&io::Request::Stream { stream: OUTPUT, down: Down::Finish }));
    assert_eq!(h.relay.ended(), None);
    h.settle();
    assert_eq!(h.relay.ended(), Some(Exit::Done));
    assert_eq!(Exit::Done.code(), 0);
}

#[test]
fn an_unreachable_agent_ends_the_relay_with_1_without_its_secret() {
    let mut h = Harness::new();
    h.event(io::Event::Failed { owner: OWNER, error: io::Error::Refused });
    h.settle();
    assert_eq!(h.relay.ended(), Some(Exit::Unreachable));
    assert_eq!(Exit::Unreachable.code(), 1);
    assert_eq!(h.relay.detail(), Some(b"smith-agent: the relay's agent is unreachable\n".as_slice()));
    assert!(!format!("{:?}", h.relay).contains(&"07".repeat(32)));
}

#[test]
fn a_reset_ends_the_relay_with_1() {
    let mut h = Harness::new();
    h.handshake();
    h.event(io::Event::Stream { owner: OWNER, up: Up::Failed(Fault::Reset) });
    h.settle();
    assert_eq!(h.relay.ended(), Some(Exit::Failed));
    assert_eq!(Exit::Failed.code(), 1);
    assert_eq!(h.relay.detail(), Some(b"smith-agent: the relay's connection failed\n".as_slice()));
}

#[test]
fn a_termination_ends_the_relay_with_143() {
    let mut h = Harness::new();
    h.connect();
    h.relay.terminate(&mut h.below);
    assert!(drain(&mut h.below).contains(&io::Request::Abort { entity: SOCKET }));
    h.settle();
    assert_eq!(h.relay.ended(), Some(Exit::Terminated));
    assert_eq!(Exit::Terminated.code(), 143);
}

#[test]
fn the_relay_waits_for_its_acknowledgement_before_reading_input() {
    let mut h = Harness::new();
    h.connect();
    let records = drain(&mut h.below);
    assert!(!records.iter().any(|r| match r {
        io::Request::Stream { .. } => true,
        _ => false,
    }));
    let right = records
        .into_iter()
        .find_map(|record| match record {
            io::Request::Output { down: OutputDown::Room { right, .. }, .. } => Some(right),
            _ => None,
        })
        .expect("secret room");
    h.event(io::Event::Output { owner: OWNER, up: OutputUp::Settled { right, outcome: OutputOutcome::Granted } });
    let records = drain(&mut h.below);
    assert!(!records.iter().any(|r| match r {
        io::Request::Stream { stream: INPUT, .. } => true,
        _ => false,
    }));
    assert!(records.iter().any(|r| match r {
        io::Request::Stream { stream: SOCKET, down: Down::Demand { read: Read::Line { max: 13 }, room: 0 } } => true,
        _ => false,
    }));
    h.event(io::Event::Stream { owner: OWNER, up: Up::Bytes(ACK.into()) });
    let records = drain(&mut h.below);
    assert!(records.iter().any(|r| match r {
        io::Request::Stream { stream: INPUT, down: Down::Demand { read: Read::Arrived { max: 4096 }, room: 0 } } =>
            true,
        _ => false,
    }));
    assert!(!records.iter().any(|r| match r {
        io::Request::Output { stream: OUTPUT, .. } => true,
        _ => false,
    }));
}

#[test]
fn an_end_before_the_acknowledgement_is_refused() {
    for up in [Up::End, Up::Failed(Fault::Reset)] {
        let mut h = Harness::new();
        h.connect();
        h.grant(SOCKET);
        h.event(io::Event::Stream { owner: OWNER, up });
        h.settle();
        assert_eq!(h.relay.ended(), Some(Exit::Refused));
        assert_eq!(Exit::Refused.code(), 1);
        assert_eq!(h.relay.detail(), Some(b"smith-agent: the relay's agent refused it\n".as_slice()));
    }
}

#[test]
fn a_wrong_acknowledgement_is_refused() {
    for bytes in [b"smith-mcp no\n".as_slice(), b"smith-mcp ok\r", b"smith-mcp ok ", b"\n"] {
        let mut h = Harness::new();
        h.connect();
        h.grant(SOCKET);
        h.event(io::Event::Stream { owner: OWNER, up: Up::Bytes(bytes.into()) });
        h.settle();
        assert_eq!(h.relay.ended(), Some(Exit::Refused));
    }
}
