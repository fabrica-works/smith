//! Anonymous overlapping spans (domain/host.md, section 4;
//! protocol/channel.md, section 4). State keeps the open count and latest
//! permitted end. It never knows which operation a `LongDone` ended.

use skein_lib::Time;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Long {
    pub(crate) open: u32,
    until: Time,
}

impl Long {
    pub(crate) const fn new(now: Time) -> Long {
        Long { open: 0, until: now }
    }

    pub(crate) fn begin(&mut self, until: Time) {
        self.until = if self.open == 0 { until } else { self.until.max(until) };
        self.open = self.open.checked_add(1).expect("checked against long_open before entry");
    }

    pub(crate) fn end(&mut self) {
        self.open = self.open.checked_sub(1).expect("LongDone owns an open span");
    }

    pub(crate) fn stretch(&self, progress: Time) -> Time {
        if self.open == 0 { progress } else { progress.max(self.until) }
    }
}

#[cfg(test)]
mod tests {
    use super::Long;
    use skein_lib::{Duration, Time};

    fn second(value: u64) -> Time {
        Time::ZERO.saturating_add(Duration::from_secs(value))
    }

    #[test]
    fn either_terminal_keeps_the_latest_extension_until_both_are_done() {
        for ends in [[10, 40], [40, 10]] {
            let mut long = Long::new(Time::ZERO);
            long.begin(second(ends[0]));
            long.begin(second(ends[1]));
            assert_eq!(long.stretch(second(2)), second(40));
            long.end();
            assert_eq!(long.stretch(second(3)), second(40));
            long.end();
            assert_eq!(long.stretch(second(4)), second(4));
            long.begin(second(8));
            assert_eq!(long.stretch(second(5)), second(8), "a later batch forgets old deadlines");
        }
    }
}
