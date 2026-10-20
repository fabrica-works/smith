//! Selected command budgets, kept without file or process state.
//! The kit selects duration and proportional capture before emitting IO;
//! the flight retains only their clamp evidence through the process terminal.
//! Contract: domain/tools.md, sections 3 and 5; programming-model.md, section 6.3.

use crate::{Budget, BytesClamp, Limits, Timeout, TimeoutClamp};
use skein_lib::Duration;

/// A command's retained head and tail with its applied byte clamp.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Kept {
    pub(crate) head: u32,
    pub(crate) tail: u32,
    pub(crate) clamp: BytesClamp,
}

/// A command's effective duration with its applied deadline clamp.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Timed {
    pub(crate) timeout: Duration,
    pub(crate) clamp: TimeoutClamp,
}

pub(crate) fn kept(asked: Budget, limits: &Limits) -> Kept {
    let ceiling = limits.shell_head.checked_add(limits.shell_tail).expect("admitted capture ceiling fits u32");
    let bytes = match asked {
        Budget::Default => limits.shell_output,
        Budget::Asked(bytes) => bytes,
    };
    let effective = bytes.min(ceiling);
    let clamp = if bytes > ceiling { BytesClamp::Clamped { asked: bytes, ceiling } } else { BytesClamp::Within };
    let head = if ceiling == 0 {
        0
    } else {
        let numerator =
            u64::from(effective).checked_mul(u64::from(limits.shell_head)).expect("two u32 quantities fit u64");
        u32::try_from(numerator.checked_div(u64::from(ceiling)).expect("positive capture ceiling"))
            .expect("selected head fits its u32 budget")
    };
    let tail = effective.checked_sub(head).expect("selected head is within the budget");
    Kept { head, tail, clamp }
}

pub(crate) fn timed(asked: Timeout, limits: &Limits) -> Timed {
    let timeout = match asked {
        Timeout::Default => limits.shell_timeout,
        Timeout::Asked(duration) => duration,
    };
    if timeout > limits.tool_deadline {
        Timed {
            timeout: limits.tool_deadline,
            clamp: TimeoutClamp::Clamped { asked: timeout, ceiling: limits.tool_deadline },
        }
    } else {
        Timed { timeout, clamp: TimeoutClamp::Within }
    }
}

/// Select a shell's yield without changing its command deadline.
pub(crate) fn yielded(asked: crate::Yield, limits: &Limits) -> (Option<Duration>, TimeoutClamp) {
    match asked {
        crate::Yield::Never => (None, TimeoutClamp::Within),
        crate::Yield::Asked(duration) => {
            let timed = job_timed(duration, limits);
            (Some(timed.timeout), timed.clamp)
        }
    }
}

/// Select a job read's wait under the same ceiling as a shell's yield.
pub(crate) fn waited(asked: crate::Wait, limits: &Limits) -> Timed {
    match asked {
        crate::Wait::Default => Timed { timeout: limits.job_poll, clamp: TimeoutClamp::Within },
        crate::Wait::Asked(duration) => job_timed(duration, limits),
    }
}

fn job_timed(asked: Duration, limits: &Limits) -> Timed {
    if asked > limits.job_wait {
        Timed { timeout: limits.job_wait, clamp: TimeoutClamp::Clamped { asked, ceiling: limits.job_wait } }
    } else {
        Timed { timeout: asked, clamp: TimeoutClamp::Within }
    }
}
