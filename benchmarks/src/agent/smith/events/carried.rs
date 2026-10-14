//! Compaction joins its parent's owed calls, captured results and next window.
//! Only retained prompts and completions establish what crossed the window;
//! absent capture stays unavailable (benchmarks.md 3.2; domain/session.md 8.2).

use smith_events::{Block, ConversationKind, ConversationOpened, Event, Outcome, Record};

use crate::{CheckVerdict, Measure};

use super::Observer;

pub(super) fn check(observer: &Observer) -> CheckVerdict {
    for session in &observer.sessions {
        for (position, record) in session.records.iter().enumerate() {
            if let Event::ConversationOpened(opening) = &record.event
                && opening.kind == ConversationKind::Compaction
            {
                let identity = format!("run={} compaction={}", opening.run, opening.conversation);
                match carried(&session.records, position, opening) {
                    Ok(None) => {}
                    Ok(Some(reason)) => {
                        return CheckVerdict {
                            check: "compaction-carried".into(),
                            passed: Measure::Observed { value: false },
                            evidence: vec![format!("{identity}: {reason}")],
                        };
                    }
                    Err(reason) => {
                        return CheckVerdict {
                            check: "compaction-carried".into(),
                            passed: Measure::unavailable(format!("{identity}: {reason}")),
                            evidence: vec![],
                        };
                    }
                }
            }
        }
    }
    CheckVerdict { check: "compaction-carried".into(), passed: Measure::Observed { value: true }, evidence: vec![] }
}

fn carried(records: &[Record], position: usize, opening: &ConversationOpened) -> Result<Option<String>, String> {
    let parent = opening.parent.ok_or("compaction parent was not retained")?;
    let previous = records[..position]
        .iter()
        .rev()
        .find_map(|record| {
            if let Event::ResponseCompleted(value) = &record.event
                && value.run == opening.run
                && value.conversation == parent
                && value.outcome == Outcome::Completed
            {
                Some(value)
            } else {
                None
            }
        })
        .ok_or("parent's completed response was not retained")?;
    let completion = previous.completion.as_ref().ok_or("parent completion was not captured")?;
    let following = &records[position + 1..];
    let request = following
        .iter()
        .find_map(|record| {
            if let Event::ResponseStarted(value) = &record.event
                && value.run == opening.run
                && value.conversation == opening.conversation
            {
                Some(value)
            } else {
                None
            }
        })
        .ok_or("compaction request was not retained")?;
    let prompt = request.prompt.as_ref().ok_or("compaction prompt was not captured")?;
    for block in &completion.blocks {
        if let Block::Call { call, .. } = block
            && !prompt
                .messages
                .iter()
                .flat_map(|message| message.blocks.iter())
                .any(|block| matches!(block, Block::Result { call: result, .. } if result == call))
        {
            return Ok(Some(format!("missing result for call {}", String::from_utf8_lossy(call))));
        }
    }
    let Some(closed) = following.iter().position(|record| {
        matches!(&record.event,
        Event::ConversationClosed(value) if value.run == opening.run && value.conversation == opening.conversation)
    }) else {
        return Ok(None);
    };
    let Some(next) = following[closed + 1..].iter().find_map(|record| {
        if let Event::ResponseStarted(value) = &record.event
            && value.run == opening.run
            && value.conversation == parent
        {
            Some(value)
        } else {
            None
        }
    }) else {
        return Ok(None);
    };
    let prompt = next.prompt.as_ref().ok_or("next window prompt was not captured")?;
    let summary = following[..closed]
        .iter()
        .rev()
        .find_map(|record| {
            if let Event::ResponseCompleted(value) = &record.event
                && value.run == opening.run
                && value.conversation == opening.conversation
                && value.outcome == Outcome::Completed
            {
                Some(value)
            } else {
                None
            }
        })
        .ok_or("compaction completion was not retained")?;
    let summary = summary.completion.as_ref().ok_or("compaction completion was not captured")?;
    let text = summary
        .blocks
        .iter()
        .filter_map(|block| if let Block::Text { text } = block { Some(text.as_ref()) } else { None })
        .collect::<Vec<_>>()
        .join(&b"\n\n"[..]);
    if !prompt.messages.iter().next().is_some_and(|message| {
        message.blocks.iter().any(|block| matches!(block, Block::Text { text: actual } if actual.as_ref() == text))
    }) {
        return Ok(Some("missing summary in next window's first message".into()));
    }
    Ok(None)
}
