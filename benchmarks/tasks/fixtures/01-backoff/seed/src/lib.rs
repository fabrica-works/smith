#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub base_ms: u64,
    pub cap_ms: u64,
}

impl RetryPolicy {
    pub fn delay(self, attempt: u32) -> u64 {
        self.base_ms
            .wrapping_mul(1u64.wrapping_shl(attempt))
            .min(self.cap_ms)
    }

    pub fn deadline(self, now: u64, attempt: u32) -> u64 {
        now.wrapping_add(self.delay(attempt))
    }
}
