//! Whole-window replacement (domain/session.md, section 8.2). The session
//! keeps opener text aside while the provider summarises the old history.
//! This module knows no provider wire form or run policy. Admission, `start`,
//! `open_window` and the content-free facts are the session's call sites.

use alloc::boxed::Box;
use core::mem::{self, size_of};
use skein_lib::{List, Queue, Writer};

use super::{Conversation, End, Limits, Request, content_cost, payload_cost, release_provider, spec_cost};
use crate::llm::{Block, Cap, Evidence, Failure, Limit, Message, Role, Stop, Usage};
use crate::window::{self, Admission, Stage, Tokens, Window};

/// Opener text waits beside the old history until its new window opens.
#[derive(Debug)]
pub(super) enum Compaction {
    Idle,
    /// A restored foreign window awaits compaction on the charter's LLM.
    CarryOver,
    Asking {
        held: List<Box<[u8]>>,
        announced: bool,
    },
}

impl Compaction {
    pub(super) const fn asking(&self) -> bool {
        match self {
            Self::Idle | Self::CarryOver => false,
            Self::Asking { .. } => true,
        }
    }
}

/// Retain the charter model while the old window is compacted at its own rates.
pub(super) fn change_model(conversation: &mut Conversation, llm: crate::record::Llm, limits: &Limits) -> Option<End> {
    let effort_bytes = match &llm.effort {
        Some(effort) => effort.len(),
        None => 0,
    };
    if llm.model.is_empty()
        || llm.model.len() > 128
        || effort_bytes > 32
        || llm.window == 0
        || llm.output == 0
        || llm.prices.unit == 0
    {
        return Some(End::TranscriptRefused { reason: crate::record::Refusal::Malformed });
    }
    let cost = match u64::try_from(llm.model.len()) {
        Ok(bytes) => bytes.checked_add(match &llm.effort {
            Some(effort) => u64::try_from(effort.len()).expect("bounded effort"),
            None => 0,
        }),
        Err(_) => None,
    };
    if !super::charge(conversation, cost, limits) {
        return Some(End::TranscriptRefused { reason: crate::record::Refusal::TooLarge });
    }
    conversation.next_llm = Some(crate::record::Llm {
        effort: mem::replace(&mut conversation.effort, llm.effort),
        model: mem::replace(&mut conversation.model, llm.model),
        window: mem::replace(&mut conversation.model_window, llm.window),
        output: mem::replace(&mut conversation.output, llm.output),
        prices: mem::replace(&mut conversation.recording.prices, llm.prices),
    });
    conversation.window.stage = Stage::Used;
    conversation.window.following = conversation.transcript.len().saturating_sub(1);
    None
}

pub(super) fn due(conversation: &mut Conversation, limits: &Limits) -> bool {
    let carried = match conversation.compaction {
        Compaction::CarryOver => true,
        Compaction::Idle | Compaction::Asking { .. } => false,
    };
    if conversation.next_llm.is_some() || carried {
        return true;
    }
    conversation.window.appended = Tokens(appended_bytes(conversation).unwrap_or(u64::MAX));
    match prompt_bytes(conversation) {
        Some(bytes) => {
            window::check(
                &conversation.window,
                bytes,
                conversation.protocol_allowance,
                limits.token_bytes,
                conversation.model_window,
                limits.threshold,
            ) == Admission::Compact
        }
        None => true,
    }
}

fn appended_bytes(conversation: &Conversation) -> Option<u64> {
    let mut bytes = 0_u64;
    for message in conversation.transcript.iter().skip(usize::try_from(conversation.window.following).ok()?) {
        for block in &message.content {
            bytes = bytes.checked_add(payload_cost(block)?)?;
        }
    }
    Some(bytes)
}

/// Count the same owned prompt form as the session's emitted request without copying it.
pub(super) fn prompt_bytes(conversation: &Conversation) -> Option<u64> {
    prompt_prefix_bytes(conversation, usize::try_from(conversation.transcript.len()).ok()?)
}

pub(super) fn prompt_prefix_bytes(conversation: &Conversation, messages: usize) -> Option<u64> {
    let mut bytes = u64::try_from(conversation.model.len())
        .ok()?
        .checked_add(u64::try_from(conversation.system.len()).ok()?)?
        .checked_add(conversation.namespace.owned_bytes()?)?;
    if let Some(effort) = &conversation.effort {
        bytes = bytes.checked_add(u64::try_from(effort.len()).ok()?)?;
    }
    for message in conversation.transcript.iter().take(messages) {
        for block in &message.content {
            bytes = bytes.checked_add(payload_cost(block)?)?;
        }
    }
    Some(bytes)
}

pub(super) fn instruction_bytes(conversation: &Conversation) -> usize {
    if !conversation.compaction.asking() {
        return conversation.summary.len();
    }
    match conversation.transcript.last().expect("summary message").content.first().expect("summary block") {
        Block::Text { text, .. } => text.len(),
        Block::Refusal { .. } | Block::ToolCall { .. } | Block::ToolResult { .. } | Block::Opaque { .. } => {
            unreachable!("summary instruction is text")
        }
    }
}

pub(super) fn window_limit(conversation: &Conversation, evidence: Evidence, attempts: u32) -> End {
    let failure = Failure::Limit { which: Limit::Window, bound: u64::from(conversation.model_window) };
    End::Failed { failure, evidence, attempts }
}

/// Move only opener text appended after the last ordinary completion out of the request.
pub(super) fn start(
    conversation: &mut Conversation,
    limits: &Limits,
    evidence: Evidence,
    attempts: u32,
) -> Option<End> {
    match conversation.window.stage {
        Stage::Fresh => return Some(window_limit(conversation, evidence, attempts)),
        Stage::Used if conversation.compaction.asking() => return Some(window_limit(conversation, evidence, attempts)),
        Stage::Used => {}
    }
    let mut blocks = 0_u32;
    for message in &conversation.transcript {
        blocks = blocks.checked_add(super::count(message.content.len())).expect("validated window block bound");
    }
    let mut held = List::with_capacity(blocks.checked_add(1).expect("held offers fit u32"));
    let previous = mem::replace(&mut conversation.transcript, List::with_capacity(limits.history_messages));
    for (index, message) in previous.into_boxed().into_iter().enumerate() {
        if index < usize::try_from(conversation.window.following).expect("u32 fits") || message.role == Role::Assistant
        {
            conversation.transcript.push(message).expect("one old message per slot");
            continue;
        }
        let mut kept = List::with_capacity(u32::try_from(message.content.len()).expect("bounded blocks"));
        for block in message.content {
            match block {
                Block::Text { text, .. } => {
                    conversation.bytes = conversation
                        .bytes
                        .checked_sub(u64::try_from(size_of::<Block>()).expect("block cell fits"))
                        .expect("held text releases its history block");
                    held.push(text).expect("one offer fits its history");
                }
                block @ (Block::ToolResult { .. }
                | Block::ToolCall { .. }
                | Block::Refusal { .. }
                | Block::Opaque { .. }) => {
                    kept.push(block).expect("one kept slot per old block");
                }
            }
        }
        if !kept.is_empty() {
            conversation
                .transcript
                .push(Message { role: message.role, content: kept.into_boxed() })
                .expect("one old message per slot");
        }
    }
    // The instruction moves from the spec into history; its bytes were already charged.
    let cell = u64::try_from(size_of::<Block>()).expect("summary cell fits");
    assert!(conversation.transcript.room() > 0, "history admission leaves the summary a message");
    assert!(super::charge(conversation, Some(cell), limits), "history admission leaves the summary a block");
    let text = mem::replace(&mut conversation.summary, Box::new([]));
    let content: Box<[Block]> = Box::new([Block::Text { text, replay: None }]);
    conversation.transcript.push(Message { role: Role::User, content }).expect("summary message slot checked");
    conversation.compaction = Compaction::Asking { held, announced: false };
    None
}

/// Open the replacement from an accepted text summary; its terminal classification is the caller's.
pub(super) fn open_window(
    conversation: &mut Conversation,
    text: Box<[u8]>,
    usage: Usage,
    limits: &Limits,
    out: &mut Queue<Request>,
) -> Option<End> {
    release_provider(conversation);
    let Some(number) = conversation.window.number.checked_add(1) else {
        return Some(super::history::sequence_limit());
    };
    let asking = Compaction::Asking { held: List::with_capacity(0), announced: true };
    let held = match mem::replace(&mut conversation.compaction, asking) {
        Compaction::Asking { held, .. } => held,
        Compaction::Idle | Compaction::CarryOver => unreachable!("a summary answers a compaction"),
    };
    let count = held.len().checked_add(3).expect("held offer plus task, preamble and summary fits u32");
    let summary_bytes = u64::try_from(text.len()).expect("bounded summary bytes");
    let mut content = List::with_capacity(count);
    content.push(Block::Text { text: conversation.task.clone(), replay: None }).expect("task slot");
    content.push(Block::Text { text: conversation.preamble.clone(), replay: None }).expect("preamble slot");
    content.push(Block::Text { text, replay: None }).expect("summary slot");
    for text in held.into_boxed() {
        content.push(Block::Text { text, replay: None }).expect("one held message per slot");
    }
    let content = content.into_boxed();
    let last = conversation.transcript.len().checked_sub(1).expect("summary instruction is last");
    let message = conversation.transcript.get_mut(last).expect("summary instruction is retained");
    conversation.summary = match message.content.first_mut().expect("one summary text") {
        Block::Text { text, .. } => mem::replace(text, Box::new([])),
        Block::Refusal { .. } | Block::ToolCall { .. } | Block::ToolResult { .. } | Block::Opaque { .. } => {
            unreachable!("a compaction asks with the summary text last")
        }
    };
    if let Some(llm) = conversation.next_llm.take() {
        conversation.model = llm.model;
        conversation.effort = llm.effort;
        conversation.model_window = llm.window;
        conversation.output = llm.output;
        conversation.recording.prices = llm.prices;
    }
    let Some(bytes) = new_bytes(conversation, &content) else {
        return Some(super::history::limit(limits.history_bytes));
    };
    if super::count(content.len()) > limits.history_blocks {
        return Some(super::history::limit(u64::from(limits.history_blocks)));
    }
    if bytes > limits.history_bytes {
        return Some(super::history::limit(limits.history_bytes));
    }
    conversation.compaction = Compaction::Idle;
    conversation.window_turn = true;
    conversation.transcript = List::with_capacity(limits.history_messages);
    conversation.transcript.push(Message { role: Role::User, content }).expect("a fresh history has one slot");
    conversation.bytes = bytes;
    conversation.window = Window::fresh(number, usage.output_tokens.unwrap_or(u64::from(conversation.output)));
    conversation.window.summary_bytes = summary_bytes;
    conversation.recording.told = 0;
    super::tell_turn(conversation, out)
}

fn new_bytes(conversation: &Conversation, content: &[Block]) -> Option<u64> {
    spec_cost(&conversation.model, &conversation.system, &conversation.delegated, content)?
        .checked_add(conversation.namespace.owned_bytes()?)?
        .checked_add(super::ticket_bytes(&conversation.choice))?
        .checked_add(super::choice_bytes(conversation.choice_names.as_deref()))?
        .checked_add(conversation.final_turn.note_bytes()?)?
        .checked_add(u64::try_from(conversation.task.len()).ok()?)?
        .checked_add(u64::try_from(conversation.summary.len()).ok()?)?
        .checked_add(u64::try_from(conversation.preamble.len()).ok()?)?
        .checked_add(match &conversation.effort {
            Some(effort) => u64::try_from(effort.len()).ok()?,
            None => 0,
        })
}

pub(super) fn context_failure(failure: Failure) -> bool {
    match failure {
        Failure::ContextTooLong | Failure::Limit { which: Limit::Path(Cap::Request), .. } => true,
        Failure::Limit { .. }
        | Failure::ApplicationLimit
        | Failure::Protocol
        | Failure::Cancelled
        | Failure::Overloaded
        | Failure::RateLimited { .. }
        | Failure::Unavailable
        | Failure::TimedOut { .. }
        | Failure::Refused
        | Failure::Unsupported
        | Failure::Invalid
        | Failure::Unauthorized
        | Failure::Exhausted { .. } => false,
    }
}

pub(super) fn answered(
    conversation: &mut Conversation,
    id: skein_lib::Id<super::Session>,
    completion: crate::llm::Completion,
    env: &skein_lib::Env<Limits>,
    out: &mut Queue<Request>,
) -> super::State {
    match accept(conversation, completion, &env.limits, out) {
        Some(end) => super::finish(end),
        None => super::call(conversation, id, 0, env, out),
    }
}

pub(super) fn accept(
    conversation: &mut Conversation,
    completion: crate::llm::Completion,
    limits: &Limits,
    out: &mut Queue<Request>,
) -> Option<End> {
    match content_cost(&completion.content) {
        Some(bytes)
            if bytes <= limits.completion_bytes
                && super::count(completion.content.len()) <= limits.completion_blocks => {}
        Some(_) | None => {
            release_provider(conversation);
            return Some(super::history::completion_limit(conversation, &completion.content, limits));
        }
    }
    let mut refused = completion.stop == Stop::Refusal;
    let mut text = false;
    for block in &completion.content {
        match block {
            Block::Refusal { .. } => refused = true,
            Block::Text { text: bytes, .. } => {
                for byte in bytes {
                    if !byte.is_ascii_whitespace() {
                        text = true;
                    }
                }
            }
            Block::ToolCall { .. } | Block::ToolResult { .. } | Block::Opaque { .. } => {}
        }
    }
    let fault = if refused {
        Some(crate::NoSummary::Refused)
    } else if !text {
        Some(match completion.stop {
            Stop::MaxTokens => crate::NoSummary::Truncated,
            Stop::EndTurn | Stop::ToolUse | Stop::Refusal => crate::NoSummary::Malformed,
        })
    } else {
        None
    };
    if let Some(fault) = fault {
        release_provider(conversation);
        super::clear_pending(conversation);
        return Some(End::NoSummary { fault });
    }
    let text = summary_text(&completion.content);
    open_window(conversation, text, completion.usage, limits, out)
}

/// Join only summary text blocks, leaving other answer text policies unchanged.
fn summary_text(content: &[Block]) -> Box<[u8]> {
    let mut length = 0_usize;
    let mut parts = 0_usize;
    for block in content {
        match block {
            Block::Text { text, .. } => {
                length = length.checked_add(text.len()).expect("bounded summary text");
                parts = parts.checked_add(1).expect("bounded summary blocks");
            }
            Block::Refusal { .. } | Block::ToolCall { .. } | Block::ToolResult { .. } | Block::Opaque { .. } => {}
        }
    }
    let separators = parts.saturating_sub(1).checked_mul(2).expect("bounded summary separators");
    let mut written = Writer::new(length.checked_add(separators).expect("bounded joined summary"));
    let mut first = true;
    for block in content {
        match block {
            Block::Text { text, .. } => {
                if !first {
                    written.put(b"\n\n").expect("measured separator");
                }
                written.put(text).expect("measured text");
                first = false;
            }
            Block::Refusal { .. } | Block::ToolCall { .. } | Block::ToolResult { .. } | Block::Opaque { .. } => {}
        }
    }
    written.finish()
}

pub(super) fn tell(conversation: &mut Conversation, facts: &mut crate::facts::Facts, request: &Request) {
    let opener = conversation.opener;
    match request {
        Request::Complete { .. } => match &mut conversation.compaction {
            Compaction::Asking { announced, .. } if !*announced => {
                *announced = true;
                facts.push(crate::FactKind::compaction_started(
                    opener,
                    conversation.window.number,
                    &conversation.model,
                    conversation.model_window,
                    conversation.output,
                    conversation.effort.as_deref(),
                ));
            }
            Compaction::Asking { .. } | Compaction::Idle | Compaction::CarryOver => {}
        },
        Request::Turn { turn, .. } => match turn.kind {
            crate::record::Kind::Window(_) => facts.push(crate::FactKind::CompactionEnded {
                opener,
                window: conversation.window.number,
                end: crate::Compacted::Opened,
            }),
            crate::record::Kind::First(_) | crate::record::Kind::Ordinary => {}
        },
        Request::Ended { .. } if conversation.compaction.asking() => {
            facts.push(crate::FactKind::CompactionEnded {
                opener,
                window: conversation.window.number,
                end: crate::Compacted::Failed,
            });
            conversation.compaction = Compaction::Idle;
        }
        Request::Ended { .. }
        | Request::JobsCancelled { .. }
        | Request::Opened { .. }
        | Request::Yielded { .. }
        | Request::Used { .. }
        | Request::Priced { .. }
        | Request::Cancel { .. }
        | Request::Io { .. }
        | Request::Job { .. }
        | Request::CancelIo { .. }
        | Request::Delegate { .. }
        | Request::Withdraw { .. } => {}
    }
}

pub(super) fn restore(
    conversation: &mut Conversation,
    transcript: &crate::record::Transcript,
    bytes: &mut u64,
) -> Result<(), crate::record::Refusal> {
    let first = transcript.turns.first().expect("validated nonempty transcript");
    let mut messages = 0_u32;
    let mut following = 0_u32;
    for turn in &transcript.turns {
        for message in &turn.messages {
            messages = messages.checked_add(1).ok_or(crate::record::Refusal::TooLarge)?;
            if message.role == Role::Assistant {
                following = messages;
            }
        }
    }
    for message in &first.messages {
        if message.role == Role::User {
            match message.content.first() {
                Some(Block::Text { text, .. }) => {
                    *bytes = bytes
                        .checked_sub(u64::try_from(conversation.task.len()).expect("bounded task"))
                        .ok_or(crate::record::Refusal::TooLarge)?
                        .checked_add(u64::try_from(text.len()).expect("bounded restored task"))
                        .ok_or(crate::record::Refusal::TooLarge)?;
                    conversation.task.clone_from(text);
                }
                Some(
                    Block::Refusal { .. } | Block::Opaque { .. } | Block::ToolCall { .. } | Block::ToolResult { .. },
                )
                | None => {}
            }
            break;
        }
    }
    match first.kind {
        crate::record::Kind::Window(_) if transcript.turns.len() == 1 => {
            conversation.window = Window::fresh(1, first.usage.output_tokens.unwrap_or(u64::from(conversation.output)));
            match first.messages.first().expect("one window message").content.get(2) {
                Some(Block::Text { text, .. }) => {
                    conversation.window.summary_bytes = u64::try_from(text.len()).expect("bounded summary");
                }
                Some(
                    Block::Refusal { .. } | Block::Opaque { .. } | Block::ToolCall { .. } | Block::ToolResult { .. },
                )
                | None => {}
            }
        }
        crate::record::Kind::First(_) | crate::record::Kind::Window(_) | crate::record::Kind::Ordinary => {
            conversation.window.completed(
                transcript.turns.last().expect("nonempty").usage,
                conversation.output,
                following,
                0,
            );
        }
    }
    Ok(())
}

/// Carry a foreign window onto the charter's LLM, pricing only its portable history.
pub(super) fn carry_over(
    conversation: &mut Conversation,
    transcript: &mut crate::record::Transcript,
) -> Result<u64, crate::record::Refusal> {
    use crate::record::{Backend, Kind, Refusal};
    let first = transcript.turns.first().expect("validated nonempty history");
    let same_opening = match &first.kind {
        Kind::First(opener) | Kind::Window(opener) => {
            opener.account == conversation.account && opener.backend == Backend::Direct
        }
        Kind::Ordinary => unreachable!("validated opening kind"),
    };
    if transcript.endpoint == Some(conversation.endpoint)
        && transcript.dialect == conversation.recording.dialect
        && same_opening
    {
        return Ok(0);
    }
    let mut released = 0_u64;
    let mut messages = 0_u32;
    for turn in &mut transcript.turns {
        for message in &mut turn.messages {
            messages = messages.checked_add(1).ok_or(Refusal::TooLarge)?;
            let before = content_cost(&message.content).ok_or(Refusal::TooLarge)?;
            let content = mem::replace(&mut message.content, Box::new([]));
            let mut kept = List::with_capacity(super::count(content.len()));
            for block in content {
                if let Some(block) = portable(block) {
                    kept.push(block).expect("one retained slot per old block");
                }
            }
            message.content = kept.into_boxed();
            let after = content_cost(&message.content).ok_or(Refusal::TooLarge)?;
            released =
                released.checked_add(before.checked_sub(after).ok_or(Refusal::TooLarge)?).ok_or(Refusal::TooLarge)?;
        }
    }
    conversation.window.stage = Stage::Used;
    conversation.window.following = messages;
    conversation.compaction = Compaction::CarryOver;
    Ok(released)
}

fn portable(block: Block) -> Option<Block> {
    match block {
        Block::Opaque { .. } => None,
        Block::Text { text, .. } => Some(Block::Text { text, replay: None }),
        Block::Refusal { text, .. } => Some(Block::Refusal { text, replay: None }),
        Block::ToolCall { id, name, input, call, .. } => Some(Block::ToolCall { id, name, input, call, replay: None }),
        Block::ToolResult { id, result } => {
            let result = match result {
                crate::llm::Returned::Text { text, error, .. } => {
                    crate::llm::Returned::Text { text, error, replay: None }
                }
                value @ (crate::llm::Returned::Owned { .. }
                | crate::llm::Returned::Invalid { .. }
                | crate::llm::Returned::NotRun { .. }
                | crate::llm::Returned::Withdrawn) => value,
            };
            Some(Block::ToolResult { id, result })
        }
    }
}
