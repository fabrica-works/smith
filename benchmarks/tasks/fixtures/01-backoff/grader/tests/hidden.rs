use fixture::RetryPolicy;

#[test]
fn multiplication_overflow_caps() {
    let p = RetryPolicy { base_ms: 1u64 << 63, cap_ms: u64::MAX };
    assert_eq!(p.delay(1), u64::MAX);
    assert_eq!(p.delay(2), u64::MAX);
}

#[test]
fn huge_attempts_do_not_wrap_shift() {
    let p = RetryPolicy { base_ms: 7, cap_ms: 999 };
    for n in [63, 64, 65, 127, 128, u32::MAX] {
        assert_eq!(p.delay(n), 999, "attempt {n}");
    }
}

#[test]
fn zero_base_is_always_zero() {
    for n in [0, 1, 63, 64, u32::MAX] {
        assert_eq!(RetryPolicy { base_ms: 0, cap_ms: u64::MAX }.delay(n), 0);
    }
}

#[test]
fn deadline_saturates() {
    let p = RetryPolicy { base_ms: 9, cap_ms: 50 };
    assert_eq!(p.deadline(u64::MAX - 2, 0), u64::MAX);
    assert_eq!(p.deadline(u64::MAX, 1), u64::MAX);
}

#[test]
fn exhaustive_small_values_match_mathematical_reference() {
    for base in 0..32 {
        for cap in 0..40 {
            for attempt in 0..12 {
                let p = RetryPolicy { base_ms: base, cap_ms: cap };
                assert_eq!(p.delay(attempt), (base * (1 << attempt)).min(cap));
            }
        }
    }
}
