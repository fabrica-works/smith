//! The product's end, its close reports and shell handoff (shell.md, section 9).
//! The phase retains the last status and termination disposition. It knows no
//! live run policy; its reports come from the components and the shell.

use super::{Service, local};
use skein_lib::Time;
use smith_agent_service::{Close, Step};

/// The product's close reports, including its shell before its stream terminal.
#[derive(Clone, Copy, Debug)]
pub(super) enum Ending {
    /// Local policy still owes its last word.
    Live { shell_closed: bool },
    /// Owned components, the shell, stream or signal reader still owe reports.
    Closing { at: Time, exit: local::ExitStatus, close: Close, step: Step, shell_closed: bool },
    /// Every owned close reported; the shell receives the retained exit.
    Closed { exit: local::ExitStatus, close: Close },
}

impl Service {
    pub(super) fn end_work(&self) -> bool {
        match self.ending {
            Ending::Live { .. } | Ending::Closed { .. } => false,
            Ending::Closing { step: Step::Components, shell_closed, .. } => {
                let streams = match &self.stream {
                    Some(stream) => stream.producer.streams(),
                    None => 0,
                }
                .checked_add(match self.below.inline() {
                    Some(effects) => effects.streams(),
                    None => 0,
                })
                .expect("bounded destination count")
                .checked_add(self.below.process().signal_streams())
                .expect("bounded retained streams");
                let lower_closed = match self.below.inline() {
                    Some(effects) => effects.components_closed(),
                    None => true,
                };
                shell_closed && self.accounts_closed() && lower_closed && self.below.process().settled(streams)
            }
            Ending::Closing { step: Step::Stream, .. } => {
                let lower_closed = match self.below.inline() {
                    Some(effects) => effects.closed(),
                    None => true,
                };
                lower_closed && self.below.process().settled(self.below.process().signal_streams())
            }
            Ending::Closing { step: Step::Signals, .. } => self.below.process().closed() && self.shell.room() > 0,
            Ending::Closing { step: Step::Usage, .. } => unreachable!("usage belongs to the producer"),
        }
    }

    fn exit_status(&self) -> Option<local::ExitStatus> {
        match self.ending {
            Ending::Live { .. } | Ending::Closing { .. } => None,
            Ending::Closed { exit, .. } => Some(exit),
        }
    }

    /// The local domain has said its last word; the shell starts its own closes.
    #[must_use]
    pub fn closing(&self) -> bool {
        match self.ending {
            Ending::Live { .. } => false,
            Ending::Closing { .. } | Ending::Closed { .. } => true,
        }
    }

    /// The shell's terminal, credential owners and launch directory reported closed.
    pub fn shell_closed(&mut self) {
        match &mut self.ending {
            Ending::Closing { shell_closed, .. } => *shell_closed = true,
            Ending::Live { .. } => unreachable!("the shell closes after the local last word"),
            Ending::Closed { .. } => {}
        }
    }

    /// A result file failure changes only the product exit after an accepted answer.
    pub fn result_file_failed(&mut self) -> bool {
        match &mut self.ending {
            Ending::Closing { exit, .. } if *exit == local::ExitStatus::Accepted => {
                *exit = local::ExitStatus::RunFailed;
                true
            }
            Ending::Closing { .. } => false,
            Ending::Live { .. } | Ending::Closed { .. } => unreachable!("result output settles during product closes"),
        }
    }

    /// Whether outstanding graceful closes were interrupted by a termination signal.
    #[must_use]
    pub fn aborted(&self) -> bool {
        match self.ending {
            Ending::Live { .. } => false,
            Ending::Closing { close, .. } | Ending::Closed { close, .. } => close == Close::Aborted,
        }
    }

    pub(super) fn shutdown(&mut self) {
        match &mut self.ending {
            Ending::Live { .. } => {
                self.terminal.interrupt(&mut self.terminal_events);
                return;
            }
            Ending::Closing { close, .. } => *close = Close::Aborted,
            Ending::Closed { .. } => return,
        }
        if let Some(stream) = &mut self.stream {
            stream.abort(self.now);
        }
        match self.ending {
            Ending::Closing { step: Step::Stream, .. } => {
                let terminal_sent = match &self.stream {
                    Some(stream) => stream.exit.is_none(),
                    None => false,
                };
                if terminal_sent {
                    self.below.process_mut().abort_stream(&mut self.stream);
                }
            }
            Ending::Live { .. } | Ending::Closing { .. } | Ending::Closed { .. } => {}
        }
        self.below.process_mut().abort();
        if let Some(effects) = self.below.inline_mut() {
            effects.abort();
        }
    }

    pub(super) fn advance_end(&mut self) {
        match self.ending {
            Ending::Live { .. } | Ending::Closed { .. } => {}
            Ending::Closing { at, exit, close, step: Step::Components, shell_closed } => {
                let streams = match &self.stream {
                    Some(stream) => stream.producer.streams(),
                    None => 0,
                }
                .checked_add(match self.below.inline() {
                    Some(effects) => effects.streams(),
                    None => 0,
                })
                .expect("bounded destination count")
                .checked_add(self.below.process().signal_streams())
                .expect("bounded retained streams");
                let lower_closed = match self.below.inline() {
                    Some(effects) => effects.components_closed(),
                    None => true,
                };
                if lower_closed && self.accounts_closed() && self.below.process().settled(streams) && shell_closed {
                    if let Some(effects) = self.below.inline_mut() {
                        effects.end(crate::stream::exit(exit));
                    }
                    if let Some(stream) = &mut self.stream {
                        stream.exit = Some(crate::stream::exit(exit));
                    }
                    self.ending = Ending::Closing { at, exit, close, step: Step::Stream, shell_closed };
                }
            }
            Ending::Closing { at, exit, close, step: Step::Stream, shell_closed } => {
                let lower_closed = match self.below.inline() {
                    Some(effects) => effects.closed(),
                    None => true,
                };
                if lower_closed && self.below.process().settled(self.below.process().signal_streams()) {
                    self.below.process_mut().close_signals();
                    self.ending = Ending::Closing { at, exit, close, step: Step::Signals, shell_closed };
                }
            }
            Ending::Closing { exit, close, step: Step::Signals, .. } => {
                if self.below.process().closed() && self.shell.room() > 0 {
                    self.ending = Ending::Closed { exit, close };
                    self.shell.push(local::Request::Exit { status: self.exit_status().expect("closed product") });
                }
            }
            Ending::Closing { step: Step::Usage, .. } => unreachable!("usage is owned by the product producer"),
        }
    }
}
