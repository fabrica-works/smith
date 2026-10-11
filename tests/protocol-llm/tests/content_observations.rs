//! Actual content observations through Smith's component and the native HTTP/SSE client.
//! Contracts: protocol/events.md, sections 3.5 and 3.11.

use skein_lib::Token;
use smith_events::Block;
use smith_protocol_llm::Measure;
use smith_protocol_llm_world::pool_peer::Peer;

#[test]
fn actual_codex_text_deltas_join_to_the_observed_completion() {
    let body = [
        r#"{"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"item","role":"assistant","content":[]}}"#,
        r#"{"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"one "}"#,
        r#"{"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"two"}"#,
        r#"{"type":"response.output_item.done","output_index":0,"item":{"type":"message","id":"item","role":"assistant","content":[{"type":"output_text","text":"one two"}]}}"#,
        r#"{"type":"response.completed","response":{"status":"completed","output":[],"usage":{"input_tokens":1,"output_tokens":2}}}"#,
    ].iter().flat_map(|event| format!("data: {event}\n\n").into_bytes()).collect::<Vec<_>>();
    let head = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n", body.len());
    let mut peer = Peer::configured(1, 1);
    peer.start_observed(7, 60, 60, 300);
    peer.bytes(100, head.as_bytes());
    for bytes in body.chunks(17) {
        peer.bytes(100, bytes);
    }
    assert_eq!(peer.terminals.len(), 1);
    let mut text = Vec::new();
    let mut prompt_count = 0;
    let mut completed = 0;
    let mut terminals = 0;
    for measure in peer.measures {
        match measure {
            Measure::Prompt { at: _, owner, prompt } => {
                assert_eq!(owner, Token::new(7));
                assert_eq!(prompt.system.as_deref(), Some(b"timed reasoning".as_slice()));
                assert_eq!(prompt.messages.len(), 1);
                prompt_count += 1;
            }
            Measure::Delta { at: _, owner, block, text: delta } => {
                assert_eq!(owner, Token::new(7));
                assert_eq!(block, 0);
                text.extend_from_slice(&delta);
            }
            Measure::Completion { at: _, owner, completion } => {
                assert_eq!(owner, Token::new(7));
                assert_eq!(text, b"one two");
                assert_eq!(completion.blocks.as_slice(), &[Block::Text { text: b"one two".to_vec().into() }]);
                completed += 1;
            }
            Measure::Terminal { owner, measures } => {
                assert_eq!(owner, Token::new(7));
                assert_eq!(measures.request_bytes, Some(peer.request_bytes));
                assert!(peer.request_bytes > 0);
                assert_eq!(measures.first_byte, Some(skein_lib::Duration::ZERO));
                assert_eq!(measures.largest_gap, Some(skein_lib::Duration::ZERO));
                terminals += 1;
            }
        }
    }
    assert_eq!((prompt_count, completed), (1, 1));
    assert_eq!(terminals, 1);
}

#[test]
fn actual_transport_latency_and_event_gap_use_the_peer_clock_without_content_capture() {
    let first = "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"item\",\"role\":\"assistant\",\"content\":[]}}\n\n";
    let last = concat!(
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"item\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"done\"}]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n"
    );
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n",
        first.len() + last.len()
    );
    let mut peer = Peer::configured(1, 1);
    peer.start(7, 60, 60, 300);
    peer.advance(5);
    peer.bytes(100, &[head.as_bytes(), first.as_bytes()].concat());
    peer.advance(8);
    peer.bytes(100, last.as_bytes());
    assert_eq!(peer.terminals.len(), 1);
    assert_eq!(peer.measures.len(), 1, "disabled capture retains only the actual terminal measures");
    let Measure::Terminal { owner, measures } = peer.measures[0] else { panic!("actual terminal measures") };
    assert_eq!(owner, Token::new(7));
    assert_eq!(measures.request_bytes, Some(peer.request_bytes));
    assert_eq!(measures.first_byte, Some(skein_lib::Duration::from_secs(5)));
    assert_eq!(measures.largest_gap, Some(skein_lib::Duration::from_secs(3)));
}
