//! Scripted input waits for outside cues, never agent state or text heuristics.
//! One ordered cue fires once; its bounded line is written before input closes
//! (benchmarks.md, section 6; reliability-plan 00, Interface sketches).

use std::collections::VecDeque;

use serde::Deserialize;

use super::Seen;

/// A person's ordered lines and its input-close policy.
#[derive(Clone, Debug)]
pub struct Script {
    pub cues: Vec<Cue>,
    pub close: Close,
}

/// One complete line typed when its outside trigger is observed.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cue {
    pub when: Trigger,
    pub text: String,
}

/// The outside evidence that permits one scripted line.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Trigger {
    /// Type the initial prompt before any response.
    Start,
    /// Type after this named native tool call.
    ToolCall { tool: String },
    /// Type after the accepted main answer.
    Answer,
}

/// When the person closes its input, after queued bytes have been written.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Close {
    /// Close after the initial prompt, for a noninteractive command.
    AfterPrompt,
    /// Close after main's accepted answer, for the local terminal.
    AfterAnswer,
    /// Close after every ordered cue has fired.
    AfterLastCue,
}

#[derive(Debug)]
pub(crate) struct Person {
    script: Script,
    next: usize,
    lines: VecDeque<Box<[u8]>>,
    answered: bool,
}

impl Person {
    pub(crate) fn new(script: Script) -> Result<Self, String> {
        if script.cues.len() > 64 || script.close == Close::AfterPrompt && script.cues.len() > 1 {
            return Err("script exceeds its cue bound or closes before a later cue".into());
        }
        for (index, cue) in script.cues.iter().enumerate() {
            let bytes = cue.text.as_bytes();
            if bytes.is_empty()
                || bytes.len() > 4088
                || !bytes.ends_with(b"\n")
                || bytes[..bytes.len() - 1].contains(&b'\n')
            {
                return Err("script needs complete bounded one-line cues".into());
            }
            if (index == 0) != (cue.when == Trigger::Start) {
                return Err("only the first script cue may be Start".into());
            }
        }
        let mut person = Self { script, next: 0, lines: VecDeque::new(), answered: false };
        if !person.script.cues.is_empty() {
            person.fire();
        }
        Ok(person)
    }

    fn fire(&mut self) {
        self.lines.push_back(self.script.cues[self.next].text.as_bytes().to_vec().into_boxed_slice());
        self.next += 1;
    }

    pub(crate) fn see(&mut self, seen: &Seen) {
        if *seen == Seen::Answer {
            self.answered = true;
        }
        if let Some(cue) = self.script.cues.get(self.next) {
            let matches = match &cue.when {
                Trigger::Start => false,
                Trigger::ToolCall { tool } => matches!(seen,Seen::ToolCall {tool:actual} if actual==tool),
                Trigger::Answer => *seen == Seen::Answer,
            };
            if matches {
                self.fire();
            }
        }
    }

    pub(crate) fn line(&mut self) -> Option<Box<[u8]>> {
        self.lines.pop_front()
    }

    pub(crate) fn ready(&self) -> bool {
        !self.lines.is_empty()
    }

    pub(crate) fn all_cues(&self) -> bool {
        self.next == self.script.cues.len() && !self.ready()
    }

    pub(crate) fn closing(&self) -> bool {
        if self.ready() {
            return false;
        }
        match self.script.close {
            Close::AfterPrompt => true,
            Close::AfterAnswer => self.answered,
            Close::AfterLastCue => self.next == self.script.cues.len(),
        }
    }
}
