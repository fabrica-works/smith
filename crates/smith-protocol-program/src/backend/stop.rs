//! One interrupt/result/input-close sequence (protocol/programs.md 5.7).
//! Keeps whether a result is owed; knows no run budget. Cancel, close and the
//! wind-down enter here; the backend's existing grace escalates to termination.

use super::{Backend, Below, Phase, tail, turns};
use skein_io::{self as io, kernel};
use skein_lib::{Env, List, Queue, Time};
use smith_domain::llm;
use smith_protocol_llm as protocol;
use smith_protocol_llm::{FromDomain, ToDomain};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stop {
    None,
    Closing,
    WindDown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WindDown {
    Ordinary,
    Finishing,
}

impl Backend {
    pub(super) fn active_turn(&self) -> bool {
        if self.turns.result {
            return false;
        }
        if self.request.is_some() || self.owner.is_some() || self.turns.message.is_some() || !self.calls.is_empty() {
            return true;
        }
        for call in &self.ledger {
            if !call.answered {
                return true;
            }
        }
        false
    }

    pub(super) fn wind_down(
        &mut self,
        env: &Env<protocol::ComponentLimits>,
        mut request: FromDomain,
        up: &mut Queue<ToDomain>,
        below: &mut Queue<Below>,
    ) -> Option<FromDomain> {
        let (prompt, capture) = match &mut request {
            FromDomain::Complete { prompt, observation, .. } => (prompt, observation.is_some()),
            FromDomain::Cancel { .. } => unreachable!("completion route"),
        };
        match &prompt.choice {
            llm::ToolChoice::Only(_) if self.winding == WindDown::Ordinary => {}
            llm::ToolChoice::Only(_) | llm::ToolChoice::Auto | llm::ToolChoice::None => return Some(request),
        }
        self.winding = WindDown::Finishing;
        let Some(rendered) =
            tail::render_observed(prompt, &self.ledger, &self.settings.adapter, self.settings.input_bytes, capture)
        else {
            self.fail(env, llm::Failure::Invalid, b"smith: wind-down input exceeds its bound", up, below);
            return None;
        };
        self.outgoing = Some(rendered.line);
        let active = self.active_turn();
        self.request = Some(request);
        if let Some(text) = rendered.text {
            self.prompt_text_measure(env.now, text);
        }
        self.stop(env.now, active, true);
        if self.stop == Stop::None {
            self.completion_deadline(env, true);
        }
        self.flush(below);
        None
    }

    pub(super) fn stop(&mut self, now: Time, active: bool, keeping: bool) {
        self.flush_deltas(now);
        let waiting = self.stop != Stop::None;
        self.stop = if keeping { Stop::WindDown } else { Stop::Closing };
        self.calls = List::with_capacity(u32::from(self.settings.mcp.calls));
        if active && let Some(child) = self.child {
            if !waiting {
                self.io.push(io::Request::Signal {
                    child,
                    to: kernel::Target::Group,
                    signal: kernel::Signal::Interrupt,
                });
            }
        } else if !waiting {
            self.finish_stop();
        }
        if !waiting {
            self.deadline = if self.child.is_some() || self.spawning {
                Some(now.saturating_add(self.settings.grace))
            } else {
                None
            };
        }
    }

    pub(super) fn stopped_result(&mut self, env: &Env<protocol::ComponentLimits>) {
        self.finish_stop();
        if self.phase == Phase::Running && !self.switching {
            self.completion_deadline(env, true);
        }
    }

    fn finish_stop(&mut self) {
        self.calls = List::with_capacity(u32::from(self.settings.mcp.calls));
        for index in 0..self.ledger.len() {
            let call = self.ledger.get_mut(index).expect("ledger cell");
            call.rpc = None;
            call.answer = None;
            call.answered = true;
        }
        match self.stop {
            Stop::Closing => {
                if let Some(input) = self.pipes[0] {
                    self.io.push(io::Request::Close { entity: input });
                }
            }
            Stop::WindDown => {
                let closed = self.turns.closed;
                self.turns = turns::Turns::new(&self.settings.receiving, self.settings.adapter.client.answer);
                self.turns.closed = closed;
                self.send_input();
            }
            Stop::None => {}
        }
        self.stop = Stop::None;
    }
}
