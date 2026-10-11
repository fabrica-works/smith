//! Accepted calls wait below the session, with one terminal and no failed admission.
//! Contract: protocol/limits.md, section 6; protocol/llm.md, section 6.

use skein_lib::Token;
use smith_domain::llm;
use smith_protocol_llm::ToDomain;
use smith_protocol_llm_world::pool_peer::Peer;

#[test]
fn three_calls_to_one_endpoint_beyond_its_connections_each_complete() {
    let mut peer = Peer::configured(3, 1);
    for owner in 1..=3 {
        peer.start(owner, 60, 60, 300);
    }
    assert_eq!(peer.connections(), 1, "two accepted calls wait behind one actual socket");
    assert!(peer.terminals.is_empty());
    for owner in 1..=3 {
        peer.complete(100);
        assert_eq!(peer.connections(), 1, "the endpoint reuses its one physical binding");
        assert_eq!(peer.terminals.len(), usize::try_from(owner).expect("three calls"));
        assert!(matches!(peer.terminals.last(), Some(ToDomain::Completed { owner: observed, completion })
            if *observed == Token::new(owner)
                && completion.content.iter().any(|block| matches!(block, llm::Said::Text { text, .. } if text.as_ref() == b"done"))));
    }
    peer.advance(301);
    assert_eq!(peer.terminals.len(), 3, "late timers never repeat a terminal");
    assert_eq!(peer.connections(), 0);
}

#[test]
fn a_waiting_call_arms_only_whole_and_cancels_without_disturbing_the_next() {
    let mut peer = Peer::configured(3, 1);
    peer.start(1, 60, 60, 300);
    peer.start(2, 1, 1, 300);
    peer.start(3, 1, 1, 300);
    peer.advance(10);
    assert!(peer.terminals.is_empty(), "queued calls have no head or idle phase yet");
    peer.cancel(2);
    peer.cancel(2);
    assert!(matches!(peer.terminals.as_slice(), [ToDomain::Cancelled { owner }] if *owner == Token::new(2)));
    assert!(matches!(peer.measures.as_slice(), [smith_protocol_llm::Measure::Terminal { owner, measures }]
        if *owner == Token::new(2) && *measures == smith_protocol_llm::Measures::none()));
    peer.complete(100);
    peer.complete(100);
    assert!(matches!(&peer.terminals[1], ToDomain::Completed { owner, .. } if *owner == Token::new(1)));
    assert!(matches!(&peer.terminals[2], ToDomain::Completed { owner, .. } if *owner == Token::new(3)));
    peer.advance(301);
    assert_eq!(peer.terminals.len(), 3);
}

#[test]
fn a_waiting_whole_expiry_is_unsent_and_leaves_the_running_call_live() {
    let mut peer = Peer::configured(2, 1);
    peer.start(1, 60, 60, 300);
    peer.start(2, 1, 1, 10);
    peer.advance(10);
    assert!(matches!(peer.terminals.as_slice(), [ToDomain::Failed {
        owner, failure: llm::Failure::TimedOut { phase: llm::Phase::Whole }, evidence: llm::Evidence::Unsent, ..
    }] if *owner == Token::new(2)));
    assert!(matches!(peer.measures.as_slice(), [smith_protocol_llm::Measure::Terminal { owner, measures }]
        if *owner == Token::new(2) && *measures == smith_protocol_llm::Measures::none()));
    assert_eq!(peer.connections(), 1);
    peer.complete(100);
    assert!(matches!(&peer.terminals[1], ToDomain::Completed { owner, .. } if *owner == Token::new(1)));
    peer.advance(301);
    assert_eq!(peer.terminals.len(), 2);
}
