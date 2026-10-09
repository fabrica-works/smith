//! Deterministic declared pool sizes through the actual adapter and native byte client.
//! Contract: protocol/limits.md, section 6; testing-strategy.md, section 7.

use skein_lib::{Rng, Token};
use smith_protocol_llm::ToDomain;
use smith_protocol_llm_world::pool_peer::Peer;

#[test]
fn random_pool_sizes_complete_in_order_without_capacity_failures() {
    for seed in [941_u64, 947, 953, 967] {
        let mut rng = Rng::new(seed);
        for _ in 0..32 {
            let calls = u32::try_from(rng.next_u64() % 6 + 1).expect("bounded pool");
            for per_endpoint in [1, calls] {
                let mut peer = Peer::configured(calls, per_endpoint);
                for owner in 1..=u64::from(calls) {
                    peer.start(owner, 60, 60, 300);
                }
                assert_eq!(peer.connections(), usize::try_from(per_endpoint).expect("small pool"));
                assert!(peer.terminals.is_empty());
                for index in 0..calls {
                    let socket = if per_endpoint == 1 { 100 } else { 100 + u64::from(index) };
                    peer.complete(socket);
                    assert!(
                        matches!(peer.terminals.last(), Some(ToDomain::Completed { owner, .. })
                        if *owner == Token::new(u64::from(index) + 1)),
                        "seed {seed}, calls {calls}"
                    );
                }
                peer.advance(301);
                assert_eq!(peer.terminals.len(), usize::try_from(calls).expect("small pool"));
                assert_eq!(peer.connections(), 0);
            }
        }
    }
}
