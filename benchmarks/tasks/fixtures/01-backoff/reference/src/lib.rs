#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub base_ms: u64,
    pub cap_ms: u64,
}

impl RetryPolicy {
    pub fn delay(self, attempt: u32) -> u64 {
        if self.base_ms == 0 {
            return 0;
        }
        let factor = 1u64.checked_shl(attempt);
        factor
            .and_then(|factor| self.base_ms.checked_mul(factor))
            .unwrap_or(u64::MAX)
            .min(self.cap_ms)
    }

    pub fn deadline(self, now: u64, attempt: u32) -> u64 {
        now.saturating_add(self.delay(attempt))
    }
}
