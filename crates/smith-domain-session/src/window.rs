//! Admission before ordinary completions (domain/session.md, section 8.1).
//! A window retains reported prompt/output tokens, the byte bound appended
//! after that output and whether it has made an ordinary completion. It knows
//! no provider grammar. `check` treats every arithmetic overflow as compaction.

use crate::llm::Usage;

/// A token count; one retained byte is conservatively one token.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tokens(pub(crate) u64);

/// Whether this window has made an ordinary completion.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Stage {
    /// No ordinary answer has yet been accepted in this window.
    Fresh,
    /// An ordinary answer supplies the retained usage baseline.
    Used,
}

/// Reported usage and appended bytes of the current conversation window.
#[derive(Debug)]
pub(crate) struct Window {
    pub(crate) number: u32,
    pub(crate) last_prompt: Option<Tokens>,
    pub(crate) last_output: Tokens,
    pub(crate) appended: Tokens,
    pub(crate) stage: Stage,
    /// First message after the last ordinary completion.
    pub(crate) following: u32,
    /// Summary bytes replaced by its reported output count in a fresh check.
    pub(crate) summary_bytes: u64,
}

/// Whether an ordinary completion fits the current window's threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Admission {
    Fits,
    Compact,
}

impl Window {
    pub(crate) const fn fresh(number: u32, output: u64) -> Self {
        Self {
            number,
            last_prompt: None,
            last_output: Tokens(output),
            appended: Tokens(0),
            stage: Stage::Fresh,
            following: 0,
            summary_bytes: 0,
        }
    }

    pub(crate) fn completed(&mut self, usage: Usage, output: u32, following: u32) {
        self.last_prompt = prompt_tokens(usage);
        self.last_output = Tokens(usage.output_tokens.unwrap_or(u64::from(output)));
        self.appended = Tokens(0);
        self.stage = Stage::Used;
        self.following = following;
        self.summary_bytes = 0;
    }
}

pub(crate) fn check(
    window: &Window,
    prompt_bytes: u64,
    allowance: u64,
    output: u32,
    model: u32,
    threshold: [u32; 2],
) -> Admission {
    let Some(bound) = bound(window, prompt_bytes, allowance, output) else { return Admission::Compact };
    let Some(limit) = u64::from(model).checked_mul(u64::from(threshold[0])) else { return Admission::Compact };
    let Some(limit) = limit.checked_div(u64::from(threshold[1])) else { return Admission::Compact };
    if bound <= limit { Admission::Fits } else { Admission::Compact }
}

fn prompt_tokens(usage: Usage) -> Option<Tokens> {
    let mut tokens = usage.input_tokens?;
    for extra in [usage.cache_read_tokens.unwrap_or(0), usage.cache_write_tokens.unwrap_or(0)] {
        let Some(sum) = tokens.checked_add(extra) else { return Some(Tokens(u64::MAX)) };
        tokens = sum;
    }
    Some(Tokens(tokens))
}

fn bound(window: &Window, bytes: u64, allowance: u64, output: u32) -> Option<u64> {
    let prompt = match window.last_prompt {
        Some(prompt) => prompt.0.checked_add(window.last_output.0)?.checked_add(window.appended.0)?,
        None => match window.stage {
            Stage::Used => bytes.checked_add(allowance)?,
            Stage::Fresh => {
                bytes.checked_sub(window.summary_bytes)?.checked_add(window.last_output.0)?.checked_add(allowance)?
            }
        },
    };
    prompt.checked_add(u64::from(output))
}
