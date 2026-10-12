# Overflow-safe capped retry backoff

Fix the retry policy in this Rust crate. `delay(attempt)` must equal
min(cap_ms, base_ms * 2^attempt) mathematically for every u64 base/cap and
u32 attempt. In particular zero base remains zero even at huge attempts.
`deadline(now, attempt)` must saturate at u64::MAX when now + delay
overflows. Neither method may panic on valid inputs, and work must remain
bounded independently of attempt (do not iterate billions of times).
Keep the public API, preserve existing behavior for small values, add
regression tests, and run the suite. There are no external dependencies.
