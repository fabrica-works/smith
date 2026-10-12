use fixture::RetryPolicy;

#[test]
fn ordinary_backoff_and_cap() {
    let policy = RetryPolicy { base_ms: 10, cap_ms: 35 };
    assert_eq!(policy.delay(0), 10);
    assert_eq!(policy.delay(1), 20);
    assert_eq!(policy.delay(2), 35);
    assert_eq!(policy.deadline(100, 1), 120);
}
