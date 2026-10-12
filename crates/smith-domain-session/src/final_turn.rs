//! A child's final-answer trigger (domain/session.md, section 7). This keeps
//! the opener's note and allowance until the last request, then its final
//! status through retries and results. It knows no run policy or provider syntax.

use alloc::boxed::Box;
use core::mem;

use skein_lib::Duration;

use crate::{Dimension, FinalTurn};

/// The opener's final note, or the final answer already being requested.
#[derive(Debug)]
pub(crate) enum Final {
    /// Main has no session-owned final turn.
    Absent,
    /// A child has not reached its final completion.
    Pending(FinalTurn),
    /// Retries preserve one appended note; the answer yields for budget.
    Answering { spent: Dimension },
}

impl Final {
    pub(crate) fn new(final_turn: Option<FinalTurn>) -> Final {
        match final_turn {
            Some(final_turn) => Final::Pending(final_turn),
            None => Final::Absent,
        }
    }

    pub(crate) fn note_bytes(&self) -> Option<u64> {
        match self {
            Final::Pending(final_turn) => u64::try_from(final_turn.note.len()).ok(),
            Final::Absent | Final::Answering { .. } => Some(0),
        }
    }

    pub(crate) const fn answering(&self) -> bool {
        match self {
            Final::Answering { .. } => true,
            Final::Absent | Final::Pending(_) => false,
        }
    }

    pub(crate) const fn spent(&self) -> Option<Dimension> {
        match self {
            Final::Answering { spent } => Some(*spent),
            Final::Absent | Final::Pending(_) => None,
        }
    }

    pub(crate) fn take_note(&mut self, turns: u32, time: Duration, compaction: bool) -> Option<Box<[u8]>> {
        let starts = match self {
            Final::Pending(final_turn) => final_turn.due(turns, time, compaction),
            Final::Absent | Final::Answering { .. } => false,
        };
        if !starts {
            return None;
        }
        let spent = if turns <= if compaction { 2 } else { 1 } { Dimension::Turns } else { Dimension::Time };
        match mem::replace(self, Final::Answering { spent }) {
            Final::Pending(final_turn) => Some(final_turn.note),
            Final::Absent | Final::Answering { .. } => unreachable!("only a pending final turn supplies its note"),
        }
    }
}
