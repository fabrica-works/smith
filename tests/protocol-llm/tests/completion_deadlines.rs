//! Model progress and session expiry through the actual Smith adapter and shared HTTP/SSE.
//! Contract: domain/session.md, sections 4, 6 and 10; protocol/llm.md, section 6.

use skein_lib::Token;
use skein_lib::{Duration, Time};
use smith_domain::llm;
use smith_domain::session;
use smith_protocol_llm::ToDomain;
use smith_protocol_llm_world::pool_peer::Peer;
use smith_session_world::recorded::{World, opening};

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

// A text delta carries the model's output, so it re-arms idle; its padding
// crosses native read demand and dispatches an SSE message. Pings and status
// events alone do not (skein's llm-connection.md, section 6).
fn output_event() -> Vec<u8> {
    let mut bytes = br#"data: {"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"done","obfuscation":""#.to_vec();
    bytes.extend_from_slice(&[b' '; 513]);
    bytes.extend_from_slice(b"\"}\n\n");
    bytes
}

fn streamed_body(deltas: usize) -> (Vec<u8>, Vec<u8>) {
    let added = br#"data: {"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"item","role":"assistant","content":[]}}"#;
    let text = "done".repeat(deltas);
    let rest = [
        format!(r#"{{"type":"response.output_item.done","output_index":0,"item":{{"type":"message","id":"item","role":"assistant","content":[{{"type":"output_text","text":"{text}"}}]}}}}"#),
        r#"{"type":"response.completed","response":{"status":"completed","output":[],"usage":{"input_tokens":1,"output_tokens":1}}}"#.to_owned(),
    ]
    .iter()
    .flat_map(|event| format!("data: {event}\n\n").into_bytes())
    .collect();
    let mut first = added.to_vec();
    first.extend_from_slice(b"\n\n");
    (first, rest)
}

#[test]
fn a_response_streaming_past_any_fixed_bound_completes() {
    let mut peer = Peer::configured(2, 2);
    peer.start(1, 60, 60, 900);
    let (first, rest) = streamed_body(9);
    peer.bytes(100, &head_for(first.len() + 9 * output_event().len() + rest.len()));
    peer.bytes(100, &first);
    for seconds in (50..=450).step_by(50) {
        peer.advance(seconds);
        assert!(peer.terminals.is_empty(), "output remains within the declared idle duration: {:?}", peer.terminals);
        peer.bytes(100, &output_event());
    }
    peer.advance(451);
    peer.bytes(100, &rest);
    let text = "done".repeat(9);
    assert!(matches!(peer.terminals.as_slice(), [ToDomain::Completed { owner, completion }] if *owner == Token::new(1)
        && completion.content.iter().any(|block| matches!(block, llm::Said::Text { text: said, .. } if said.as_ref() == text.as_bytes()))));
    peer.advance(901);
    assert_eq!(peer.terminals.len(), 1, "former phase and whole timers never repeat a terminal");
}

#[test]
fn model_head_and_idle_deadlines_fail_only_the_stalled_call() {
    let mut peer = Peer::configured(2, 2);
    peer.start(1, 5, 8, 100);
    peer.start(2, 20, 5, 100);
    peer.bytes(101, &head_for(body().len()));
    peer.advance(6);
    assert_eq!(peer.terminals.len(), 2);
    for (owner, phase) in [(Token::new(1), llm::Phase::Head), (Token::new(2), llm::Phase::Idle)] {
        assert!(peer.terminals.iter().any(|event| matches!(event, ToDomain::Failed { owner: actual, failure: llm::Failure::TimedOut { phase: actual_phase }, .. } if *actual == owner && *actual_phase == phase)));
    }
    peer.advance(101);
    assert_eq!(peer.terminals.len(), 2, "expired calls settle once");
}

#[test]
fn session_whole_expiry_and_cancel_leave_the_sibling_live() {
    let mut peer = Peer::configured(2, 2);
    peer.start(1, 30, 30, 90);
    peer.start(2, 120, 120, 300);
    let (first, rest) = streamed_body(4);
    peer.bytes(100, &head_for(first.len() + 4 * output_event().len() + rest.len()));
    peer.bytes(100, &first);
    peer.bytes(101, &head_for(body().len()));
    for seconds in [20, 40, 60, 80] {
        peer.advance(seconds);
        peer.bytes(100, &output_event());
    }
    assert!(peer.terminals.is_empty());
    peer.advance(90);
    assert!(
        matches!(peer.terminals.as_slice(), [ToDomain::Failed { owner, failure: llm::Failure::TimedOut { phase: llm::Phase::Whole }, evidence: llm::Evidence::Response { status: Some(200) }, .. }] if *owner == Token::new(1))
    );
    peer.bytes(101, &body());
    assert!(matches!(&peer.terminals[1], ToDomain::Completed { owner, .. } if *owner == Token::new(2)));
    peer.advance(301);
    assert_eq!(peer.terminals.len(), 2);

    let mut peer = Peer::configured(2, 2);
    peer.start(1, 60, 60, 300);
    peer.start(2, 60, 60, 300);
    peer.bytes(100, &head_for(body().len()));
    peer.bytes(101, &head_for(body().len()));
    peer.cancel(1);
    peer.cancel(1);
    peer.bytes(101, &body());
    assert!(matches!(&peer.terminals[0], ToDomain::Cancelled { owner } if *owner == Token::new(1)));
    assert!(matches!(&peer.terminals[1], ToDomain::Completed { owner, .. } if *owner == Token::new(2)));
    peer.advance(301);
    assert_eq!(peer.terminals.len(), 2);
}

fn session_with_time(seconds: u64) -> World {
    let mut world = World::new(411, 256);
    let mut start = opening(None, u64::MAX);
    start.spec.budget.time = Duration::from_secs(seconds);
    start.spec.progress = session::Progress { head: Duration::from_secs(20), idle: Duration::from_secs(5) };
    world.open(start);
    world
}

fn deliver_failure(peer: &mut Peer, world: &mut World) {
    let ToDomain::Failed { owner, failure, evidence, detail, .. } = peer.terminals.remove(0) else {
        panic!("one native failed attempt")
    };
    assert_eq!(world.completing, Some(owner));
    let status = match evidence {
        llm::Evidence::Response { status } => status,
        llm::Evidence::Unsent | llm::Evidence::Unknown => None,
    };
    assert_eq!(
        status,
        match evidence {
            llm::Evidence::Response { .. } => Some(200),
            llm::Evidence::Unsent | llm::Evidence::Unknown => None,
        },
        "status follows the actual independent response head, never inferred from its diagnostic"
    );
    world.step(session::Event::Failed { owner, failure, evidence, detail, retry_ok: true });
}

#[test]
fn a_stall_times_out_idle_and_is_retried() {
    let mut world = session_with_time(40);
    let owner = world.completing.expect("actual session call");
    let mut peer = Peer::configured(1, 1);
    // The fixture executes each emitted attempt through the actual adapter;
    // its literal prompt is independent of the timeout and retry decisions.
    peer.start(owner.raw(), 20, 5, 40);
    peer.bytes(100, &head_for(body().len()));
    peer.advance(6);
    assert!(matches!(
        peer.terminals.as_slice(),
        [ToDomain::Failed { failure: llm::Failure::TimedOut { phase: llm::Phase::Idle }, .. }]
    ));
    world.env.now = Time::ZERO.saturating_add(Duration::from_secs(6));
    deliver_failure(&mut peer, &mut world);
    assert_eq!(world.prompts.len(), 1, "no request is sent during backoff");
    world.env.now = world.domain.next_deadline().expect("actual retry alarm");
    assert!(world.env.now < Time::ZERO.saturating_add(Duration::from_secs(40)));
    session::fire(&mut world.domain, &world.env, &mut world.out);
    world.resume();
    assert_eq!(world.prompts.len(), 2);
    assert_eq!(world.prompts[0], world.prompts[1], "retry keeps the admitted conversation");
    peer.advance(7);
    peer.start(owner.raw(), 20, 5, 40);
    peer.bytes(100, &head_for(body().len()));
    peer.bytes(100, &body());
    let ToDomain::Completed { owner: actual, completion } = peer.terminals.remove(0) else {
        panic!("native retry completes")
    };
    assert_eq!(actual, owner);
    let [llm::Said::Text { text, .. }] = completion.content.as_ref() else { panic!("actual answer") };
    world.complete(
        Box::new([session::llm::Block::Text { text: text.clone(), replay: None }]),
        session::llm::Stop::EndTurn,
        completion.usage,
    );
    assert_eq!(world.turns.len(), 1, "the failed attempt commits no turn");
    world.close();
    peer.advance(41);
    assert!(peer.terminals.is_empty(), "settled attempts never repeat a terminal");
    assert_eq!(peer.connections(), 0);
}

#[test]
fn the_sessions_time_ending_fails_once_without_a_retry() {
    let mut world = session_with_time(3);
    let owner = world.completing.expect("actual session call");
    let mut peer = Peer::configured(1, 1);
    peer.start(owner.raw(), 20, 5, 3);
    peer.bytes(100, &head_for(body().len()));
    peer.advance(3);
    assert!(matches!(
        peer.terminals.as_slice(),
        [ToDomain::Failed { failure: llm::Failure::TimedOut { phase: llm::Phase::Whole }, .. }]
    ));
    world.env.now = Time::ZERO.saturating_add(Duration::from_secs(3));
    deliver_failure(&mut peer, &mut world);
    assert_eq!(world.end, Some(session::End::Budget { spent: session::Dimension::Time }));
    assert_eq!(world.prompts.len(), 1);
    assert_eq!(world.domain.next_deadline(), None);
    assert_eq!(world.turns.len(), 0);
    world.close();
    peer.advance(41);
    assert!(peer.terminals.is_empty());
    assert_eq!(peer.connections(), 0);
}
