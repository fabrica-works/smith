//! Concrete session history (domain/session.md, section 3). Encoding these
//! versioned values into bytes belongs to the protocol layer, never the domain.

use crate::{
    Spec,
    llm::{Affinity, Endpoint, Message, Usage},
};
use alloc::boxed::Box;

/// Concrete session transcript version emitted and accepted at the domain boundary.
pub const VERSION: u16 = 6;

/// Session-issued position of an opener call in its concrete logical transcript.
/// This fixed name survives ticket translation; provider ids and callback tokens
/// are not its identity. The host scopes it by the logical run, never a live slab.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Origin {
    /// One-based accepted completion sequence, including a restored prefix.
    /// Checked exhaustion refuses the next completion's effects.
    pub sequence: u32,
    /// Zero-based assistant block position, including preceding non-call blocks.
    /// The receiving session bounds message storage and refuses unrepresentable block counts before effects.
    pub position: u32,
}

/// Charter rates per `unit` tokens; a completion rounds its combined charge once
/// (domain/session.md, section 6). The opener sends them; overflow ends pricing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Prices {
    /// Integer price for fresh input tokens per unit.
    pub input: u64,
    /// Price for cache-read input tokens in the configured unit.
    pub cached: u64,
    /// Separate cache-write rate; absent writes use `input`.
    pub cache_write: Option<u64>,
    /// Integer price for output tokens per unit.
    pub output: u64,
    /// Token denominator for checked, upward-rounded integer pricing; zero is refused.
    pub unit: u32,
}

impl Prices {
    /// The highest input, cached and cache-write rate, reserved before provider work.
    #[must_use]
    pub const fn reserved_input(self) -> u64 {
        let highest = if self.cached > self.input { self.cached } else { self.input };
        match self.cache_write {
            Some(write) => {
                if write > highest {
                    write
                } else {
                    highest
                }
            }
            None => highest,
        }
    }

    /// Charge reported usage once; missing input/output requires the caller's
    /// recorded reservation instead. Missing cache counts add no reported charge.
    #[must_use]
    pub fn price(self, usage: Usage) -> Option<u64> {
        if self.unit == 0 {
            return None;
        }
        let input = u128::from(usage.input_tokens?).checked_mul(u128::from(self.input))?;
        let written = u128::from(usage.cache_write_tokens.unwrap_or(0))
            .checked_mul(u128::from(self.cache_write.unwrap_or(self.input)))?;
        let cached = u128::from(usage.cache_read_tokens.unwrap_or(0)).checked_mul(u128::from(self.cached))?;
        let output = u128::from(usage.output_tokens?).checked_mul(u128::from(self.output))?;
        let sum = input.checked_add(cached)?.checked_add(written)?.checked_add(output)?;
        let unit = u128::from(self.unit);
        let rounded = sum.checked_div(unit)?.checked_add(u128::from(sum.checked_rem(unit)? != 0))?;
        u64::try_from(rounded).ok()
    }
}

/// Parent-owned concrete opening, validated before any tools or provider effect.
/// `transcript: None` starts fresh under the same reservation and terminal rules.
#[derive(PartialEq, Eq, Debug)]
pub struct Opening {
    /// The original session opening, validated before replaying any history.
    pub spec: Spec,
    /// Declared old window model for a model-change compaction, supplied by the root.
    pub window_llm: Option<Llm>,
    /// Provider dialect/schema identity, paired with the configured endpoint.
    pub dialect: u32,
    /// Immutable integer prices used to charge each accepted completion.
    pub prices: Prices,
    /// Allowance supplied at admission, checked before starting more work.
    pub budget: u64,
    /// Optional concrete history admitted before any provider work.
    pub transcript: Option<Transcript>,
    /// Results the host supplied for calls in the last saved turn.
    pub answered: Box<[Answered]>,
}

/// A window's declared model and rates, supplied by the root for its compaction.
#[derive(PartialEq, Eq, Debug)]
pub struct Llm {
    pub effort: Option<Box<[u8]>>,
    pub model: Box<[u8]>,
    pub window: u32,
    pub output: u32,
    pub prices: Prices,
}

/// One post-transcript result, keyed by the saved completion and block position.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Answered {
    /// The call's position in the last saved assistant message.
    pub origin: Origin,
    /// The result rendered by the parent in the same form as a live result.
    pub result: crate::llm::Returned,
}

/// The transport that wrote an opening, preserved by the parent in saved history.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Backend {
    /// The provider connection served the session directly.
    Direct,
    /// The named program served the session's provider traffic.
    Program { name: Box<[u8]> },
}

/// What a window's opening turn records (protocol/transcript.md, sections 2 and 3).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Opener {
    pub affinity: Affinity,
    pub account: Option<u32>,
    pub backend: Backend,
}

/// The session's turn kind, including the affinity of each window opener
/// (protocol/transcript.md, sections 2 and 3).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    /// The conversation's first settled completion, opening its first window.
    First(Box<Opener>),
    /// A settled ordinary completion after the window's opener.
    Ordinary,
    /// A settled compaction, opening another window with the same affinity.
    Window(Box<Opener>),
}

impl Kind {
    /// Whether the host may begin a saved transcript at this turn.
    #[must_use]
    pub const fn opens_window(&self) -> bool {
        match self {
            Kind::First(_) | Kind::Window(_) => true,
            Kind::Ordinary => false,
        }
    }
}

/// A settled completion and its surrounding messages. Assistant call names,
/// provider ids and input bytes are concrete; delegated result bytes are too.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Turn {
    /// Concrete transcript encoding version; unsupported versions are refused.
    pub version: u16,
    pub kind: Kind,
    pub endpoint: Option<Endpoint>,
    pub model: Box<[u8]>,
    pub effort: Option<Box<[u8]>>,
    /// Configured provider dialect identity; replay requires an exact match.
    pub dialect: u32,
    /// Monotonic turn sequence within the concrete transcript.
    pub sequence: u32,
    pub usage: Usage,
    /// Cumulative spend of this activation, children included.
    pub spent: u64,
    /// Highest job number issued across the conversation; zero before its first.
    pub jobs: u32,
    /// Oldest-first conversation messages, with provider call/result pairing preserved.
    pub messages: Box<[Message]>,
}

impl Turn {
    /// Checked ownership of this turn's opener, message envelopes and their payloads.
    #[must_use]
    pub fn owned_bytes(&self) -> Option<u64> {
        let backend = match &self.kind {
            Kind::First(opener) | Kind::Window(opener) => {
                u64::try_from(size_of::<Opener>()).ok()?.checked_add(match &opener.backend {
                    Backend::Direct => 0,
                    Backend::Program { name } => u64::try_from(name.len()).ok()?,
                })?
            }
            Kind::Ordinary => 0,
        };
        messages_bytes(&self.messages)?
            .checked_add(backend)?
            .checked_add(u64::try_from(self.model.len()).ok()?)?
            .checked_add(match &self.effort {
                Some(effort) => u64::try_from(effort.len()).ok()?,
                None => 0,
            })
    }
}

/// Concrete versioned session history; a different provider window is carried over.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Transcript {
    /// Concrete transcript encoding version; unsupported versions are refused.
    pub version: u16,
    pub endpoint: Option<Endpoint>,
    /// Configured provider dialect identity; replay requires an exact match.
    pub dialect: u32,
    /// Settled concrete turns, oldest first, validated within the transcript byte cap.
    pub turns: Box<[Turn]>,
}

/// Reason admission or transcript replay was rejected before unsupported history could run.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Refusal {
    /// The durable format version is unsupported.
    Version,
    /// Input has invalid syntax or violates the codec's structural contract.
    Malformed,
    /// History contains a live ticket rather than concrete replay data.
    Unresolved,
    /// A configured ownership, count or encoded-byte cap would be exceeded.
    TooLarge,
}

impl Transcript {
    /// Complete retained history ownership, including allocated Turn and Message
    /// envelopes separately from block/payload storage. The protocol bounds
    /// input counts; the root checks this count before retaining Start context,
    /// while session performs the semantic admission before any effects.
    /// `None` means checked ownership arithmetic overflow; the caller refuses
    /// before retaining the history or starting effects.
    #[must_use]
    pub fn owned_bytes(&self) -> Option<u64> {
        let mut bytes = u64::try_from(size_of::<Turn>()).ok()?.checked_mul(u64::try_from(self.turns.len()).ok()?)?;
        for turn in &self.turns {
            bytes = bytes.checked_add(turn.owned_bytes()?)?;
        }
        Some(bytes)
    }
}

fn messages_bytes(messages: &[Message]) -> Option<u64> {
    let mut bytes = u64::try_from(size_of::<Message>()).ok()?.checked_mul(u64::try_from(messages.len()).ok()?)?;
    for message in messages {
        bytes = bytes.checked_add(crate::session::content_cost(&message.content)?)?;
    }
    Some(bytes)
}
