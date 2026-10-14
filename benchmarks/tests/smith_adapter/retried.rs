use smith_bench::{Measure, ScopeKind, TokenRecord};

fn retried_tokens(evidence: &str) -> Vec<TokenRecord> {
    let limits = smith_events::Limits { string: 524_288, content: 4096, items: 512 };
    let mut observer =
        smith_bench::agent::smith::events::Observer::with_ceiling(&limits, 16_384).expect("recorded response bounds");
    let bytes = include_str!("../recorded/retried-failure-17d63afd-02.jsonl")
        .replace("\"evidence\":\"response\"", &format!("\"evidence\":\"{evidence}\""));
    observer.feed(bytes.as_bytes()).expect("recorded response excerpt");
    observer.finish().expect("whole recorded lines");
    observer.tokens().expect("recorded tokens")
}

#[test]
fn a_retried_failure_leaves_a_lower_bound_that_names_it() {
    let tokens = retried_tokens("response");
    let total = tokens.iter().find(|record| record.role == ScopeKind::Total).expect("total");
    assert_eq!(
        total.usage.fresh,
        Measure::LowerBound {
            value: 8783,
            missing: vec!["session-0/run-1/conversation-0: session-0/run-1/response-3: retried failure".into()]
        }
    );
}

#[test]
fn a_failure_the_provider_never_answered_is_not_a_retried_failure() {
    let tokens = retried_tokens("maybe_sent");
    let total = tokens.iter().find(|record| record.role == ScopeKind::Total).expect("total");
    assert_eq!(
        total.usage.fresh,
        Measure::LowerBound {
            value: 8783,
            missing: vec![
                "session-0/run-1/conversation-0: session-0/run-1/response-3: smith cache_write_tokens is null; smith input_tokens is null".into()
            ]
        }
    );
}
