//! Per-block rendered bytes and token ratios, retained while a window extends.
//! Contract: domain/session.md, section 10.

use skein_fake_llm_domain::api::{Part, Query, Usage};
use skein_lib::Rng;
use smith_domain_session::llm::{Guard, Purpose};

/// One block's independently drawn rendering and token count.
#[derive(Clone, Copy, Debug)]
pub struct MeasuredBlock {
    pub message: Option<usize>,
    pub counted: u64,
    pub rendered: u64,
    pub ratio: u64,
}

/// The bytes and tokens seen by the world's protocol effect.
#[derive(Clone, Debug)]
pub struct MeasuredPrompt {
    pub blocks: Vec<MeasuredBlock>,
    pub input: u64,
    pub new_part: u64,
    pub guard: Option<Guard>,
    pub sent: bool,
    pub ordinary: bool,
}

#[derive(Debug)]
pub(crate) struct Counting {
    rng: Rng,
    previous: Vec<(Option<usize>, Part, MeasuredBlock)>,
}

impl Counting {
    pub(crate) fn new(seed: u64) -> Self {
        Self { rng: Rng::new(seed), previous: Vec::new() }
    }

    pub(crate) fn measure(&mut self, query: &Query, purpose: Purpose) -> MeasuredPrompt {
        let parts = std::iter::once((None, Part::Text { text: query.system.clone() })).chain(
            query
                .messages
                .iter()
                .enumerate()
                .flat_map(|(message, value)| value.parts.iter().cloned().map(move |part| (Some(message), part))),
        );
        let mut current = Vec::new();
        for (index, (message, part)) in parts.enumerate() {
            let measured = match self.previous.get(index) {
                Some((old_message, old, measured)) if *old_message == message && *old == part => *measured,
                _ => {
                    let counted = counted(&part);
                    MeasuredBlock {
                        message,
                        counted,
                        rendered: self.rng.between(counted, counted * 6),
                        ratio: self.rng.between(1, u64::from(skein_llm::TOKEN_BYTES)),
                    }
                }
            };
            current.push((message, part, measured));
        }
        self.previous = current;
        let blocks: Vec<_> = self.previous.iter().map(|(_, _, measured)| *measured).collect();
        let input = blocks.iter().map(|block| block.rendered.div_ceil(block.ratio)).sum();
        let guard = match purpose {
            Purpose::Ordinary { guard } => guard,
            Purpose::Compaction { .. } => None,
        };
        let new_part = guard.map_or(0, |guard| {
            blocks
                .iter()
                .filter(|block| block.message.is_some_and(|index| index >= guard.from))
                .map(|block| block.rendered)
                .sum()
        });
        let sent = guard.is_none_or(|guard| guard.last_prompt.saturating_add(new_part) <= guard.line);
        MeasuredPrompt { blocks, input, new_part, guard, sent, ordinary: matches!(purpose, Purpose::Ordinary { .. }) }
    }
}

fn counted(part: &Part) -> u64 {
    let bytes = match part {
        Part::Text { text } => text.len(),
        Part::Opaque { bytes } => bytes.len(),
        Part::ToolCall { id, name, arguments } => id.len() + name.len() + arguments.len(),
        Part::ToolOutput { id, output, .. } => id.len() + output.len(),
    };
    u64::try_from(bytes).expect("bounded block")
}

/// Keep the fake's cache split, replacing its total with the drawn block count.
pub(crate) fn report(usage: &mut Usage, input: u64) {
    if usage.input.is_none() && usage.cache_read.is_none() && usage.cache_write.is_none() {
        return;
    }
    let previous = usage.input.unwrap_or(0) + usage.cache_read.unwrap_or(0) + usage.cache_write.unwrap_or(0);
    let read = input * usage.cache_read.unwrap_or(0) / previous.max(1);
    let write = input * usage.cache_write.unwrap_or(0) / previous.max(1);
    usage.input = Some(input - read - write);
    usage.cache_read = usage.cache_read.map(|_| read);
    usage.cache_write = usage.cache_write.map(|_| write);
}
