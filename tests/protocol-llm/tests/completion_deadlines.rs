//! Model progress and session expiry through the actual Smith adapter and shared HTTP/SSE.
//! Contract: domain/session.md, sections 4, 6 and 10; protocol/llm.md, section 6.

use skein_lib::Token;
use smith_domain::llm;
use smith_protocol_llm::ToDomain;
use smith_protocol_llm_world::pool_peer::Peer;

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

// Real provider progress crosses native read demand and dispatches an SSE
// message. Comments alone do not refresh the response-event idle deadline.
fn progress_event() -> Vec<u8> {
    let mut bytes = br#"data: {"type":"response.in_progress","response":{"status":"in_progress"},"extra":""#.to_vec();
    bytes.extend_from_slice(&[b' '; 513]);
    bytes.extend_from_slice(b"\"}\n\n");
    bytes
}

#[test]
fn a_response_streaming_past_any_fixed_bound_completes() {
    let mut peer = Peer::configured(2, 2);
    peer.start(1, 60, 60, 900);
    peer.bytes(100, &head_for(body().len() + 9 * progress_event().len()));
    for seconds in (50..=450).step_by(50) {
        peer.advance(seconds);
        assert!(peer.terminals.is_empty(), "progress remains within the declared idle duration: {:?}", peer.terminals);
        peer.bytes(100, &progress_event());
    }
    peer.advance(451);
    peer.bytes(100, &body());
    assert!(matches!(peer.terminals.as_slice(), [ToDomain::Completed { owner, completion }] if *owner == Token::new(1)
        && completion.content.iter().any(|block| matches!(block, llm::Said::Text { text, .. } if text.as_ref() == b"done"))));
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
    assert!(
        peer.terminals.iter().all(|event| matches!(event, ToDomain::Failed { failure: llm::Failure::TimedOut, .. }))
    );
    peer.advance(101);
    assert_eq!(peer.terminals.len(), 2, "expired calls settle once");
}

#[test]
fn session_whole_expiry_and_cancel_leave_the_sibling_live() {
    let mut peer = Peer::configured(2, 2);
    peer.start(1, 30, 30, 90);
    peer.start(2, 120, 120, 300);
    peer.bytes(100, &head_for(body().len() + 4 * progress_event().len()));
    peer.bytes(101, &head_for(body().len()));
    for seconds in [20, 40, 60, 80] {
        peer.advance(seconds);
        peer.bytes(100, &progress_event());
    }
    assert!(peer.terminals.is_empty());
    peer.advance(90);
    assert!(
        matches!(peer.terminals.as_slice(), [ToDomain::Failed { owner, failure: llm::Failure::TimedOut, evidence: llm::Evidence::Response, .. }] if *owner == Token::new(1))
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
