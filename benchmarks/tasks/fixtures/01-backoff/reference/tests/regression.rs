use fixture::RetryPolicy;

#[test]
fn ordinary_backoff_and_cap() {
    let policy = RetryPolicy {
        base_ms: 10,
        cap_ms: 35,
    };
    assert_eq!(policy.delay(0), 10);
    assert_eq!(policy.delay(1), 20);
    assert_eq!(policy.delay(2), 35);
    assert_eq!(policy.deadline(100, 1), 120);
}

#[test]
fn zero_base_stays_zero_at_the_largest_attempt() {
    let policy = RetryPolicy {
        base_ms: 0,
        cap_ms: u64::MAX,
    };
    assert_eq!(policy.delay(u32::MAX), 0);
    assert_eq!(policy.deadline(42, u32::MAX), 42);
}
