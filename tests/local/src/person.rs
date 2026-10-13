//! A person reacts only to visible turns, answers and typed notices (testing.md, section 5).

use std::collections::VecDeque;

use skein_lib::Rng;
use smith_local_domain::Notice;

/// What the person waits for before acting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// A displayed turn.
    Turn,
    /// A displayed answer.
    Answer,
    /// A notice of this kind.
    Notice(Kind),
    /// This many further world iterations.
    Steps(u32),
}

/// A notice class visible to the person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The run waits for input.
    Waiting,
    /// The run's checking span changed.
    Checking,
    /// A line was not delivered.
    NotDelivered,
    /// A host operation failed.
    Failure,
}

/// One input action by the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Type a complete line.
    Type(Vec<u8>),
    /// Ask the live run to stop.
    Interrupt,
    /// End input.
    Close,
}

/// An action and the visible event that releases it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
    pub on: Trigger,
    pub act: Act,
}

/// What the terminal showed, without rendering bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// A displayed turn.
    Turn,
    /// A displayed answer.
    Answer,
    /// A displayed host notice.
    Notice(Notice),
}

/// An ordered, finite script with no access to domain state.
#[derive(Debug)]
pub struct Person {
    cues: VecDeque<Cue>,
}

impl Person {
    /// Keep the person's ordered observations and actions.
    #[must_use]
    pub fn new(cues: Vec<Cue>) -> Person {
        Self { cues: cues.into() }
    }

    /// Draw one to three lines, their triggers and where input ends from a seed.
    #[must_use]
    pub fn drawn(seed: u64) -> Person {
        let mut rng = Rng::new(seed);
        let count = rng.between(1, 3);
        let close = rng.below(count + 1);
        let mut cues = Vec::new();
        for line in 0..count {
            if line == close {
                cues.push(Cue {
                    on: Trigger::Steps(u32::try_from(rng.below(8)).expect("bounded step count")),
                    act: Act::Close,
                });
                return Self::new(cues);
            }
            let on = if line == 0 {
                Trigger::Steps(u32::try_from(rng.below(8)).expect("bounded step count"))
            } else {
                match rng.below(4) {
                    0 => Trigger::Turn,
                    1 if line == 1 => Trigger::Notice(Kind::Waiting),
                    1 | 2 => Trigger::Answer,
                    _ => Trigger::Steps(u32::try_from(rng.below(8)).expect("bounded step count")),
                }
            };
            cues.push(Cue { on, act: Act::Type(format!("Person line {} from seed {seed}", line + 1).into_bytes()) });
        }
        cues.push(Cue {
            on: Trigger::Steps(u32::try_from(rng.below(8)).expect("bounded step count")),
            act: Act::Close,
        });
        Self::new(cues)
    }

    /// Return the next action only when this observation releases it.
    pub fn saw(&mut self, shown: Shown) -> Option<Act> {
        let on = match shown {
            Shown::Turn => Trigger::Turn,
            Shown::Answer => Trigger::Answer,
            Shown::Notice(notice) => Trigger::Notice(match notice {
                Notice::Waiting => Kind::Waiting,
                Notice::Checking(_) => Kind::Checking,
                Notice::NotDelivered { .. } => Kind::NotDelivered,
                Notice::Failure(_) | Notice::Warning(_) | Notice::ChatRefused { .. } => Kind::Failure,
            }),
        };
        if self.cues.front().is_some_and(|cue| cue.on == on) { self.cues.pop_front().map(|cue| cue.act) } else { None }
    }

    /// Advance a pending step trigger once per world iteration.
    pub fn step(&mut self) -> Option<Act> {
        let cue = self.cues.front_mut()?;
        if let Trigger::Steps(left) = &mut cue.on {
            if *left == 0 {
                return self.cues.pop_front().map(|cue| cue.act);
            }
            *left -= 1;
        }
        None
    }
}
