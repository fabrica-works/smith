//! Failed-write boundaries within a completion (domain/session.md, section 5).
//! No state beyond the batch's first failed call is retained here.
use super::{Block, Conversation, Decoded, Effect, Outcome, Returned, Slot, Tools, call_id, chosen};
use crate::llm::NotRun;
use smith_domain_tools::{self as tools, Call};

pub(super) fn suppressed(block: &Block) -> bool {
    match block {
        Block::ToolCall { call: Decoded::Owned { call }, .. } => tools::effect(call) == Effect::Write,
        Block::ToolCall { call: Decoded::Delegated { effect, .. }, .. } => *effect == Effect::Write,
        Block::ToolCall { call: Decoded::Invalid { .. } | Decoded::Historical, .. }
        | Block::Text { .. }
        | Block::Refusal { .. }
        | Block::Opaque { .. }
        | Block::ToolResult { .. } => false,
    }
}

pub(super) fn answer_skipped(conversation: &Conversation, block: &Block, index: u32, tools: &mut Tools) -> bool {
    let why = if chosen(&conversation.choice, block) {
        match tools.failed {
            Some(call) if suppressed(block) => NotRun::AfterFailedWrite { call },
            Some(_) | None => return false,
        }
    } else {
        NotRun::OutsideChoice { choice: conversation.choice_names.clone() }
    };
    let result = Block::ToolResult { id: call_id(conversation, index), result: Returned::NotRun { why } };
    tools.slots.push(Slot::Done { result }).expect("one slot per call");
    true
}

pub(super) fn answered_next(tools: &mut Tools, block: &Block, result: &Returned) {
    answered(tools, tools.slots.len(), block, result);
}

pub(super) fn answered(tools: &mut Tools, slot: u32, block: &Block, result: &Returned) {
    let failed = match result {
        Returned::Invalid { .. } => true,
        Returned::Owned { outcome } => {
            file_change(block)
                && match outcome {
                    Outcome::Written { .. } | Outcome::Edited { .. } => false,
                    Outcome::Read { .. }
                    | Outcome::Listed { .. }
                    | Outcome::Found { .. }
                    | Outcome::Exited { .. }
                    | Outcome::Running { .. }
                    | Outcome::Job { .. }
                    | Outcome::NotGranted
                    | Outcome::Outside { .. }
                    | Outcome::ReadOnly { .. }
                    | Outcome::TooLong
                    | Outcome::NotFound { .. }
                    | Outcome::NotFile
                    | Outcome::Linked
                    | Outcome::Protected
                    | Outcome::NotDirectory { .. }
                    | Outcome::TooLarge { .. }
                    | Outcome::NotRead
                    | Outcome::Stale
                    | Outcome::EditFailed { .. }
                    | Outcome::Failed { .. }
                    | Outcome::TimedOut
                    | Outcome::Cancelled
                    | Outcome::Busy
                    | Outcome::NulByte => true,
                }
        }
        Returned::Text { .. } | Returned::Withdrawn | Returned::NotRun { .. } => false,
    };
    if failed {
        let call = u16::try_from(slot.checked_add(1).expect("bounded call count")).expect("bounded completion calls");
        tools.failed = Some(tools.failed.unwrap_or(call).min(call));
    }
}

/// Forecast invalid calls and skip the same writes as advancement does.
pub(super) fn scheduled(block: &Block, failed: &mut bool) -> bool {
    match block {
        Block::ToolCall { call: Decoded::Invalid { .. }, .. } => {
            *failed = true;
            false
        }
        Block::ToolCall { call: Decoded::Owned { .. } | Decoded::Delegated { .. }, .. } => {
            !*failed || !suppressed(block)
        }
        Block::ToolCall { call: Decoded::Historical, .. }
        | Block::Text { .. }
        | Block::Refusal { .. }
        | Block::Opaque { .. }
        | Block::ToolResult { .. } => false,
    }
}

fn file_change(block: &Block) -> bool {
    match block {
        Block::ToolCall { call: Decoded::Owned { call }, .. } => match call {
            Call::Write { .. } | Call::Edit { .. } => true,
            Call::Read { .. } | Call::List { .. } | Call::Search { .. } | Call::Shell { .. } | Call::Job { .. } => {
                false
            }
        },
        Block::ToolCall { call: Decoded::Delegated { .. } | Decoded::Invalid { .. } | Decoded::Historical, .. }
        | Block::Text { .. }
        | Block::Refusal { .. }
        | Block::Opaque { .. }
        | Block::ToolResult { .. } => false,
    }
}
